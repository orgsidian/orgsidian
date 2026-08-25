//! Implements FR-16 (v0.1 fallback strategy)
//!
//! Story 5.5 completes the DIRTY branch of the FR-16 Single Writer Rule (LD-7 /
//! NFR-16): when an external write lands on a file whose in-memory buffer holds
//! unsaved edits, Orgsidian must NOT silently overwrite. The v0.1 Alpha fallback
//! blocks the save and surfaces a calm conflict banner (the full three-pane
//! Merge Dialog is Epic 9).
//!
//! This module is the `orgsidian-core` hub that ties the Story 5.3 conflict
//! model to the Story 3.2 Dirty Buffer and the Story 3.1 atomic write — the only
//! crate the LEAF graph rule lets wrap `orgsidian-vault`. It provides three
//! pieces the shell's Tauri commands and the (deferred) watcher event loop
//! consume:
//!
//! - [`resolve_dirty_conflict`] — builds a [`ConflictState`] from the Dirty
//!   Buffer + fresh disk content, runs the injected strategy
//!   ([`orgsidian_vault::conflict::resolve_with`]), and on
//!   [`Resolution::Block`](orgsidian_vault::Resolution::Block) records the block
//!   in [`SharedPendingConflicts`] and returns a redaction-safe
//!   [`ConflictNotice`] for the `ConflictDetected` event payload.
//! - [`save_buffer`] — the save gate: blocked path →
//!   [`OrgError::Vault`] carrying [`VaultError::ExternalConflict`]; otherwise an
//!   atomic write, then the buffer is marked clean.
//! - [`discard_external_changes`] — clears the block ("Discard external
//!   changes"), so the next [`save_buffer`] proceeds.
//!
//! # Redaction
//!
//! [`ConflictState`] carries the user's unsaved buffer and the external disk
//! content. Neither ever crosses the IPC boundary: [`resolve_dirty_conflict`]
//! projects only path + content *byte-lengths* + the ancestor hash into
//! [`ConflictNotice`], and the shell serializes that — never the content. This
//! is the deliberate closure of the Story 5.3 deferred note ("a serde impl for
//! `ConflictState` must NOT serialize the note content"): we add no serde to the
//! conflict types at all and hand out a redacted projection instead.

use std::path::{Path, PathBuf};

use orgsidian_vault::conflict::{resolve_with, ConflictState, Resolution, ResolveConflict};
use orgsidian_vault::{atomic_write, Sha256Hash, SharedDirtyBuffers, VaultError};

use crate::error::OrgError;
use crate::index::vault_err;

pub use orgsidian_vault::pending::{PendingConflicts, SharedPendingConflicts};

/// A redaction-safe projection of a blocked [`ConflictState`] — everything the
/// `ConflictDetected { path, state }` Tauri event needs and NOTHING the user's
/// note content would leak.
///
/// The external (on-disk) and buffer contents are represented only by their
/// byte *lengths* (like the conflict types' redacting `Debug`); the ancestor
/// [`Sha256Hash`] is a digest, not content, so it is carried in full. The
/// banner itself renders only [`path`](Self::path); the sizes + hash are
/// diagnostic metadata the shell projects into the event `state`.
#[derive(Debug, Clone)]
pub struct ConflictNotice {
    path: PathBuf,
    ancestor_hash: Sha256Hash,
    external_len: usize,
    buffer_len: usize,
}

impl ConflictNotice {
    /// The conflicted file (verbatim, as the Dirty Buffer keys it) — the id the
    /// banner shows and the "Discard"/"View file" actions round-trip.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// SHA-256 of the ancestor content (see [`ConflictState`]). A digest, safe
    /// to surface.
    #[must_use]
    pub fn ancestor_hash(&self) -> Sha256Hash {
        self.ancestor_hash
    }

    /// Byte length of the external (on-disk) content — never the content.
    #[must_use]
    pub fn external_len(&self) -> usize {
        self.external_len
    }

    /// Byte length of the unsaved buffer content — never the content.
    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.buffer_len
    }
}

/// Build the [`ConflictState`] for an external write onto a DIRTY buffer, run
/// the injected `strategy`, and — when it blocks (the v0.1
/// [`BlockWithWarning`](orgsidian_vault::BlockWithWarning) fallback) — record
/// the block so a later [`save_buffer`] is refused.
///
/// Returns `Ok(Some(notice))` with the redaction-safe event payload when the
/// save is now blocked, or `Ok(None)` when there is nothing to block (the buffer
/// turned clean before we looked — a save/reload beat this event).
///
/// This is the consumer of the [`ExternalWriteOutcome::DirtyConflict`] marker
/// (`crate::reconcile`) that Story 5.4 left as the SEAM. The (still-deferred)
/// watcher event loop calls this per DIRTY event, then emits `ConflictDetected`
/// from the returned notice.
///
/// [`ExternalWriteOutcome::DirtyConflict`]: crate::ExternalWriteOutcome::DirtyConflict
///
/// # Errors
///
/// [`OrgError::Io`] if the external file cannot be read (other than a benign
/// `NotFound`, treated as empty content — a just-deleted file).
pub async fn resolve_dirty_conflict(
    strategy: &dyn ResolveConflict,
    pending: &SharedPendingConflicts,
    buffers: &SharedDirtyBuffers,
    path: &Path,
) -> Result<Option<ConflictNotice>, OrgError> {
    // Read the external (on-disk) content FIRST, before taking any lock, so the
    // `!Send` dirty-buffer read guard is never held across this `.await`.
    let external_content = match tokio::fs::read_to_string(path).await {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(OrgError::Io {
                reason: format!("failed to read {} for conflict: {err}", path.display()),
            })
        }
    };

    // Snapshot the buffer under ONE read guard (the "read atomically" rule): if
    // it is no longer dirty, there is nothing to block. Clone the content out so
    // the guard drops before we touch anything else. A poisoned lock fails safe
    // to DIRTY (never lose a conflict) — same rule as `reconcile`.
    let buffer_content = match buffers.read() {
        Ok(guard) => match guard.get_buffer(path) {
            Some(content) => content.to_owned(),
            None => return Ok(None), // clean now — a save/reload won the race
        },
        // Poisoned: treat as dirty/unknown and proceed to block, using an empty
        // buffer stand-in (we cannot read it, but must not drop the conflict).
        Err(_) => String::new(),
    };

    // The ancestor is the common content the buffer and disk diverged from.
    // Recovering it needs the index/history (Epic 9); `BlockWithWarning` never
    // reads `ancestor_hash`, so any deterministic stand-in is correct for v0.1 —
    // we stamp the external content's hash (the version the user reconciles
    // against). Epic 9's merge algorithm sources the true ancestor.
    let ancestor_hash = Sha256Hash::of(external_content.as_bytes());

    // Capture the redaction-safe projection BEFORE `resolve` consumes the state.
    let notice = ConflictNotice {
        path: path.to_path_buf(),
        ancestor_hash,
        external_len: external_content.len(),
        buffer_len: buffer_content.len(),
    };

    let state = ConflictState::new(ancestor_hash, external_content, buffer_content, path);

    match resolve_with(strategy, state) {
        Resolution::Block { path } => {
            mark_conflict(pending, path);
            Ok(Some(notice))
        }
        // Epic 9 strategies (`ThreePaneMergeDialog`) resolve to `WriteMerged` /
        // `Cancel`; the v0.1 injected strategy is always `BlockWithWarning` and
        // swapping it is out of scope here, so these arms are unreachable in
        // v0.1. Blocking is the safe default until the merge flow lands.
        _ => {
            if let Ok(mut guard) = pending.write() {
                guard.mark(path.to_path_buf());
            } else {
                // Poison: nothing we can record, but the save gate also fails
                // safe to blocked, so the file stays protected.
            }
            Ok(Some(notice))
        }
    }
}

/// Save `content` to `path` — the v0.1 save gate (FR-16 Single Writer Rule).
///
/// If `path` has an unresolved external conflict recorded in `pending`, the save
/// is REFUSED with [`OrgError::Vault`] carrying
/// [`VaultError::ExternalConflict`] — the "block the save" AC. A poisoned
/// `pending` lock fails safe to blocked (never overwrite when the block state is
/// unknowable). Otherwise the content is written via the Story 3.1
/// [`atomic_write`] (temp-file + rename, AV-retry) and the buffer is marked
/// clean — Orgsidian is the sole writer, so after a successful save the file is
/// no longer dirty.
///
/// # Errors
///
/// [`OrgError::Vault`] with [`VaultError::ExternalConflict`] when the save is
/// blocked, or with the mapped [`VaultError`] when the atomic write fails.
pub async fn save_buffer(
    pending: &SharedPendingConflicts,
    buffers: &SharedDirtyBuffers,
    path: &Path,
    content: &str,
) -> Result<(), OrgError> {
    // Fail safe to BLOCKED on a poisoned lock: an unknowable block state must
    // never let a save clobber possibly-unsaved external work.
    let blocked = pending
        .read()
        .map_or(true, |guard| guard.is_conflicted(path));
    if blocked {
        return Err(vault_err(VaultError::ExternalConflict {
            path: path.to_path_buf(),
        }));
    }

    atomic_write(path, content.as_bytes()).map_err(vault_err)?;

    // Sole-writer: the just-saved content IS the on-disk content, so the buffer
    // is clean. A poisoned lock is a no-op (the entry simply lingers) rather
    // than a panic out of a command.
    if let Ok(mut guard) = buffers.write() {
        guard.mark_clean(path);
    }
    Ok(())
}

/// Clear the external-conflict block on `path` ("Discard external changes"), so
/// the next [`save_buffer`] overwrites the external write via the normal atomic
/// path. A no-op when `path` is not blocked; a poisoned lock is tolerated (the
/// block would then still be read as blocked by the fail-safe gate, which the
/// user can retry — never a panic out of a command).
pub fn discard_external_changes(pending: &SharedPendingConflicts, path: &Path) {
    if let Ok(mut guard) = pending.write() {
        guard.clear(path);
    }
}

/// Record a block on `path`, tolerating a poisoned lock (best-effort — the save
/// gate independently fails safe to blocked).
fn mark_conflict(pending: &SharedPendingConflicts, path: impl Into<PathBuf>) {
    if let Ok(mut guard) = pending.write() {
        guard.mark(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    use orgsidian_vault::{BlockWithWarning, DirtyBufferManager};

    fn pending() -> SharedPendingConflicts {
        Arc::new(RwLock::new(PendingConflicts::new()))
    }
    fn buffers() -> SharedDirtyBuffers {
        Arc::new(RwLock::new(DirtyBufferManager::new()))
    }

    /// A DIRTY external write resolves through `BlockWithWarning` → records the
    /// block and returns a redacted notice (path + lengths + hash, no content).
    #[tokio::test]
    async fn dirty_conflict_blocks_and_returns_redacted_notice() {
        // Sentinel words chosen NOT to collide with the field names
        // (`external_len` / `buffer_len`) so the redaction check is honest.
        let disk = "* DISKWORD entry\n";
        let unsaved = "* SECRETNOTE\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.org");
        std::fs::write(&path, disk).unwrap();

        let buffers = buffers();
        buffers.write().unwrap().mark_dirty(path.clone(), unsaved);
        let pending = pending();

        let notice = resolve_dirty_conflict(&BlockWithWarning, &pending, &buffers, &path)
            .await
            .expect("resolve")
            .expect("a dirty write is blocked");

        assert_eq!(notice.path(), path);
        assert_eq!(notice.external_len(), disk.len());
        assert_eq!(notice.buffer_len(), unsaved.len());
        assert_eq!(notice.ancestor_hash(), Sha256Hash::of(disk.as_bytes()));
        // The block is now recorded.
        assert!(pending.read().unwrap().is_conflicted(&path));

        // The notice must not carry note content anywhere in its Debug.
        let rendered = format!("{notice:?}");
        assert!(!rendered.contains("SECRETNOTE"), "{rendered}");
        assert!(!rendered.contains("DISKWORD"), "{rendered}");
    }

    /// A buffer that turned clean before the event is looked at yields no block.
    #[tokio::test]
    async fn clean_buffer_records_no_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.org");
        std::fs::write(&path, "* external\n").unwrap();

        let pending = pending();
        let out = resolve_dirty_conflict(&BlockWithWarning, &pending, &buffers(), &path)
            .await
            .expect("resolve");
        assert!(out.is_none(), "nothing dirty → no block");
        assert!(!pending.read().unwrap().is_conflicted(&path));
    }

    /// A blocked save is refused with `ExternalConflict`; after discard it
    /// proceeds via atomic-write and marks the buffer clean (the AC end-to-end).
    #[tokio::test]
    async fn save_blocked_then_discard_allows_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.org");
        std::fs::write(&path, "external write\n").unwrap();

        let buffers = buffers();
        buffers
            .write()
            .unwrap()
            .mark_dirty(path.clone(), "local unsaved\n");
        let pending = pending();

        // External write lands → block recorded.
        resolve_dirty_conflict(&BlockWithWarning, &pending, &buffers, &path)
            .await
            .unwrap();

        // Save is BLOCKED.
        let err = save_buffer(&pending, &buffers, &path, "local unsaved\n")
            .await
            .expect_err("blocked save must error");
        assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
        // The on-disk content is untouched — never clobbered.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external write\n");

        // Discard external changes → block cleared.
        discard_external_changes(&pending, &path);
        assert!(!pending.read().unwrap().is_conflicted(&path));

        // Subsequent save now overwrites atomically and marks the buffer clean.
        save_buffer(&pending, &buffers, &path, "local unsaved\n")
            .await
            .expect("save after discard should succeed");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "local unsaved\n");
        assert!(
            !buffers.read().unwrap().is_dirty(&path),
            "a successful save marks the buffer clean"
        );
    }

    /// A clean, unblocked file saves straight through.
    #[tokio::test]
    async fn unblocked_save_writes_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.org");

        save_buffer(&pending(), &buffers(), &path, "* new\n")
            .await
            .expect("unblocked save");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "* new\n");
    }

    /// Safety-critical: a poisoned `pending` lock fails the save safe to BLOCKED.
    #[tokio::test]
    async fn poisoned_pending_lock_blocks_the_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.org");
        std::fs::write(&path, "disk\n").unwrap();

        let pending = pending();
        {
            let poisoner = Arc::clone(&pending);
            let _ = std::thread::spawn(move || {
                let _g = poisoner.write().unwrap();
                panic!("poison pending lock");
            })
            .join();
        }
        assert!(pending.read().is_err());

        let err = save_buffer(&pending, &buffers(), &path, "new\n")
            .await
            .expect_err("poisoned block state must fail safe to blocked");
        assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
        // Disk untouched.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "disk\n");
    }
}
