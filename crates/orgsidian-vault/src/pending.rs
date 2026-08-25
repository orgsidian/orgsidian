//! Implements FR-16 (v0.1 fallback strategy)
//!
//! Pending-conflict registry: the set of open files whose save is currently
//! BLOCKED because an external write landed on a Dirty Buffer (LD-7 Single
//! Writer Rule, NFR-16). This is the v0.1 Alpha half of FR-16 — the safe
//! fallback that refuses to overwrite unsaved work — while the full three-pane
//! Merge Dialog is Epic 9.
//!
//! # Where this sits in the flow
//!
//! When the watcher detects an external write on a file with
//! [`crate::dirty_buffer::DirtyBufferManager::is_dirty`] `== true`, the injected
//! [`crate::conflict::BlockWithWarning`] strategy resolves the conflict to
//! [`crate::conflict::Resolution::Block`]. The reconciler records that path
//! HERE, and the save command consults [`PendingConflicts::is_conflicted`] to
//! refuse the write (returning [`crate::VaultError::ExternalConflict`]). The
//! "Discard external changes" action calls [`PendingConflicts::clear`], after
//! which the next save proceeds through the normal atomic-write path.
//!
//! This registry is a *presence set*, deliberately not richer: the v0.1
//! `BlockWithWarning` fallback needs only "is this file blocked?". The rich
//! three-way material a merge needs already lives in
//! [`crate::conflict::ConflictState`] (built per-event); persisting it per
//! blocked path for a live Merge Dialog is Epic 9's concern, not this scaffold's.
//!
//! # Sharing + poisoning (mirrors [`crate::dirty_buffer`])
//!
//! [`PendingConflicts`] is a plain struct; thread-safety comes from the caller's
//! shared handle [`SharedPendingConflicts`] (`Arc<RwLock<_>>`). Because this
//! registry gates whether an external write may overwrite unsaved work, a
//! poisoned lock must **fail safe to BLOCKED**, never to writable — the same
//! rule the Dirty Buffer follows for `is_dirty`. Read
//! [`is_conflicted`](PendingConflicts::is_conflicted) with
//! `map_or(true, …)` so an unknowable registry state blocks the save rather than
//! risking a silent clobber.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Shared handle for the pending-conflict registry across threads. The app
/// registers one when a Vault is opened, alongside the
/// [`crate::dirty_buffer::SharedDirtyBuffers`] handle.
///
/// Locking — and the fail-safe-to-BLOCKED rule on a poisoned lock — is the
/// holder's responsibility (see the module docs).
pub type SharedPendingConflicts = Arc<RwLock<PendingConflicts>>;

/// The set of files whose save is currently blocked by an unresolved external
/// conflict (FR-16 v0.1 `BlockWithWarning`).
///
/// Keys are the [`PathBuf`]s the caller supplies, **verbatim** — the exact same
/// path-identity contract [`crate::dirty_buffer::DirtyBufferManager`] documents:
/// no canonicalization, case-folding, or symlink resolution here (that policy is
/// the Vault-open layer's job). The path that resolves the conflict must be
/// spelled the same as the path that recorded it.
#[derive(Debug, Default)]
pub struct PendingConflicts {
    // Just the paths — no content. `PathBuf` is not user note text, so (unlike
    // the Dirty Buffer / ConflictState) a derived `Debug` leaks nothing.
    blocked: HashSet<PathBuf>,
}

impl PendingConflicts {
    /// Empty registry — no file is blocked.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `path`'s save is blocked by an unresolved external conflict.
    ///
    /// Idempotent: marking an already-blocked path is a no-op (a set, not a
    /// counter — one external write and ten produce the same "blocked" state).
    pub fn mark(&mut self, path: impl Into<PathBuf>) {
        self.blocked.insert(path.into());
    }

    /// Clear the block on `path` ("Discard external changes"), so the next save
    /// proceeds. A no-op when `path` is not blocked — the caller need not check
    /// first.
    pub fn clear(&mut self, path: &Path) {
        self.blocked.remove(path);
    }

    /// Whether `path`'s save is currently blocked by an unresolved conflict.
    ///
    /// The save command's gate. Callers reading through a poisoned
    /// [`SharedPendingConflicts`] must treat the poison as **blocked** (see the
    /// module docs) — never as writable.
    #[must_use]
    pub fn is_conflicted(&self, path: &Path) -> bool {
        self.blocked.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn new_registry_blocks_nothing() {
        let reg = PendingConflicts::new();
        assert!(!reg.is_conflicted(Path::new("notes.org")));
    }

    #[test]
    fn mark_then_clear_lifecycle() {
        let mut reg = PendingConflicts::new();
        let p = path("notes.org");

        reg.mark(p.clone());
        assert!(reg.is_conflicted(&p), "marked path is blocked");

        reg.clear(&p);
        assert!(!reg.is_conflicted(&p), "cleared path saves again");
    }

    #[test]
    fn mark_is_idempotent() {
        let mut reg = PendingConflicts::new();
        let p = path("notes.org");
        reg.mark(p.clone());
        reg.mark(p.clone());
        assert!(reg.is_conflicted(&p));
        // One clear fully unblocks (set semantics, not a counter).
        reg.clear(&p);
        assert!(!reg.is_conflicted(&p));
    }

    #[test]
    fn clear_untracked_is_noop() {
        let mut reg = PendingConflicts::new();
        reg.mark(path("a.org"));
        reg.clear(Path::new("b.org")); // must not panic or disturb a.org
        assert!(reg.is_conflicted(Path::new("a.org")));
    }

    #[test]
    fn paths_are_distinct_verbatim_keys() {
        let mut reg = PendingConflicts::new();
        reg.mark(path("a.org"));
        assert!(reg.is_conflicted(Path::new("a.org")));
        assert!(!reg.is_conflicted(Path::new("./a.org")), "no normalization");
        assert!(!reg.is_conflicted(Path::new("A.ORG")), "no case folding");
    }

    #[test]
    fn distinct_paths_blocked_independently() {
        let mut reg = PendingConflicts::new();
        let a = path("a.org");
        let b = path("b.org");
        reg.mark(a.clone());
        reg.mark(b.clone());
        reg.clear(&a);
        assert!(!reg.is_conflicted(&a));
        assert!(
            reg.is_conflicted(&b),
            "clearing one must not unblock the other"
        );
    }

    /// Compile-time witness the shared handle can cross the watcher thread.
    fn _assert_shareable<T: Send + Sync + 'static>() {}

    #[test]
    fn shared_handle_is_send_sync() {
        _assert_shareable::<SharedPendingConflicts>();
    }

    /// The documented fail-safe: a poisoned lock read with `map_or(true, …)`
    /// answers BLOCKED, so a save can never slip through over unsaved work when
    /// the registry state is unknowable.
    #[test]
    fn poisoned_lock_reads_as_blocked() {
        let reg: SharedPendingConflicts = Arc::new(RwLock::new(PendingConflicts::new()));
        {
            let poisoner = Arc::clone(&reg);
            let _ = std::thread::spawn(move || {
                let _guard = poisoner.write().unwrap();
                panic!("poison the pending-conflict lock");
            })
            .join();
        }
        assert!(reg.read().is_err(), "lock is poisoned");

        let blocked = reg
            .read()
            .map_or(true, |guard| guard.is_conflicted(Path::new("notes.org")));
        assert!(blocked, "poison must fail safe to BLOCKED, never writable");
    }
}
