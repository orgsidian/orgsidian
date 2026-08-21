//! Implements FR-16 clean-buffer auto-reload dispatch (LD-7 Single Writer Rule
//! / LD-9 external-edits).
//!
//! This is the Story 5.4 wiring the Story 5.3 conflict seam anticipated: the
//! point where a debounced [`orgsidian_watcher::FileChanged`] is consumed and
//! routed by Dirty-Buffer state. `orgsidian-watcher`, `-vault`, and `-index` are
//! all LEAF crates that only `orgsidian-core` may wrap (deny.toml LEAF graph
//! rule), so the reconciler that touches all three lives here — the same hub
//! role the [`crate::index`] scan orchestrator already plays.
//!
//! # Routing
//!
//! [`reconcile_external_write`] reads [`DirtyBufferManager::is_dirty`] under one
//! guard (fail-safe to DIRTY on lock poison — never auto-reload over possibly
//! unsaved work) and branches:
//!
//! - **CLEAN** (`is_dirty == false`): incrementally re-sync the index for that
//!   one file ([`crate::index::resync_file`]), read the fresh disk content, and
//!   decide the cursor ([`orgsidian_vault::reload`]) — returning a
//!   [`CleanReload`] carrying the new buffer content, the cursor outcome, and
//!   the non-modal status notice.
//! - **DIRTY** (`is_dirty == true`): a [`ExternalWriteOutcome::DirtyConflict`]
//!   marker. This is the **SEAM for Story 5.5**: building the
//!   [`orgsidian_vault::conflict::ConflictState`] and running the injected
//!   `BlockWithWarning` strategy from here is 5.5's job — nothing is written or
//!   reloaded on this branch.
//!
//! # Frontend seam (not wired here)
//!
//! The Rust contract is complete and tested: routing, disk read, incremental
//! re-index, cursor decision, and the notice payload. NOT wired (recorded in
//! `deferred-work.md`): the `orgsidian-shell-app` Tauri event pushing a
//! [`CleanReload`] to the window, the React `aria-live="polite"` 3-second notice
//! render, CodeMirror applying `new_content` + the cursor, and the
//! event-consumption loop that drains the watcher's `Receiver<FileChanged>` and
//! calls [`reconcile_external_write`] per event with a per-file
//! [`BufferSnapshot`] round-trip.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use orgsidian_vault::reload::{self, BufferReload, CursorOutcome, CursorPosition};
use orgsidian_vault::SharedDirtyBuffers;
use orgsidian_watcher::FileChanged;

use crate::error::OrgError;
use crate::index::{resync_file, IndexHandle, ResyncOutcome};

/// The non-modal status notice shown after a clean auto-reload.
///
/// Epic 5 UX: announced via `aria-live="polite"` — never `assertive` — because
/// the reload is silent and automatic, not an alert.
pub const RELOAD_NOTICE: &str = "file reloaded from disk";

/// How long the [`RELOAD_NOTICE`] stays visible (FR-16 AC: 3 seconds).
pub const RELOAD_NOTICE_DURATION: Duration = Duration::from_secs(3);

/// The editor's current CLEAN-buffer state for the changed file — the OLD
/// buffer plus the cursor. For a clean file the buffer equals the last-saved
/// on-disk content (before the external write), which the editor still holds; it
/// is the `old_content` side of the cursor decision. The reconciler reads the
/// NEW disk content itself.
///
/// Content is user notes, so [`BufferSnapshot`] carries a redacting `Debug`.
#[derive(Clone)]
pub struct BufferSnapshot {
    content: String,
    cursor: CursorPosition,
}

/// Redacting `Debug`: prints the cursor and the buffer *byte-length*, never the
/// note text (same guarantee as the vault's conflict/dirty-buffer types).
impl fmt::Debug for BufferSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferSnapshot")
            .field("content_len", &self.content.len())
            .field("cursor", &self.cursor)
            .finish()
    }
}

impl BufferSnapshot {
    /// Build a snapshot from the current buffer content and cursor.
    pub fn new(content: impl Into<String>, cursor: CursorPosition) -> Self {
        Self {
            content: content.into(),
            cursor,
        }
    }

    /// The current (pre-reload) buffer content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The current cursor position.
    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }
}

/// The result of auto-reloading a CLEAN buffer: the refreshed buffer plan (new
/// content + cursor outcome), what the index re-sync did, and the status notice
/// to surface.
///
/// The inner [`BufferReload`]'s redacting `Debug` keeps the reloaded note text
/// out of any `{:?}` on this struct.
#[derive(Debug)]
pub struct CleanReload {
    path: PathBuf,
    reload: BufferReload,
    resync: ResyncOutcome,
}

impl CleanReload {
    /// The file that was reloaded — the identifier the frontend seam needs to
    /// route `new_content` + the cursor to the correct editor tab/window
    /// (symmetric with [`ExternalWriteOutcome::DirtyConflict`]'s `path`).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The refreshed buffer content (the on-disk bytes to load into the editor).
    #[must_use]
    pub fn new_content(&self) -> &str {
        self.reload.new_content()
    }

    /// Consume the outcome, yielding the owned new content — the zero-clone path
    /// for handing the reloaded buffer straight to the frontend.
    #[must_use]
    pub fn into_new_content(self) -> String {
        self.reload.into_new_content()
    }

    /// Where the cursor lands after the reload.
    #[must_use]
    pub fn cursor(&self) -> CursorOutcome {
        self.reload.cursor()
    }

    /// What the incremental index re-sync did for this file.
    #[must_use]
    pub fn resync(&self) -> ResyncOutcome {
        self.resync
    }

    /// The non-modal status notice text ([`RELOAD_NOTICE`]).
    #[must_use]
    pub fn notice(&self) -> &'static str {
        RELOAD_NOTICE
    }

    /// How long the notice stays visible ([`RELOAD_NOTICE_DURATION`]).
    #[must_use]
    pub fn notice_duration(&self) -> Duration {
        RELOAD_NOTICE_DURATION
    }

    /// Borrow the underlying buffer-reload plan (new content + cursor outcome).
    #[must_use]
    pub fn buffer_reload(&self) -> &BufferReload {
        &self.reload
    }
}

/// The outcome of reconciling one external write against the Dirty-Buffer state.
///
/// `#[non_exhaustive]`: Story 5.5 fills the DIRTY branch with the block-save
/// resolution flow, and further outcomes may follow (Epic 9), so downstream
/// `match`es stay honest.
#[derive(Debug)]
#[non_exhaustive]
pub enum ExternalWriteOutcome {
    /// The buffer was clean — it was reloaded from disk and the index re-synced.
    CleanReload(CleanReload),
    /// The buffer was dirty (or its lock was poisoned). Nothing was written or
    /// reloaded and the index is untouched. The Story 5.5 consumer —
    /// [`crate::conflict::resolve_dirty_conflict`] — takes this `path`, builds
    /// the `ConflictState` from the Dirty Buffer + fresh disk content, runs the
    /// injected `BlockWithWarning` strategy, records the block, and returns the
    /// `ConflictDetected` notice. Wiring that call in sits on the (deferred)
    /// watcher event-consumption loop that also drives the CLEAN branch.
    DirtyConflict { path: PathBuf },
}

/// Reconcile one debounced external write (`change`) against the Dirty-Buffer
/// registry and, on the CLEAN branch, auto-reload the buffer + re-index the file.
///
/// `snapshot` is the editor's current clean-buffer state for the file (used as
/// the cursor decision's `old_content`); it is ignored on the DIRTY branch.
///
/// # Errors
///
/// [`OrgError::Index`] if the incremental re-sync's writer op fails;
/// [`OrgError::Io`] if the file cannot be re-read for the reload (other than a
/// benign `NotFound`, which yields an empty buffer for a just-deleted file) or
/// its vault-relative path is not representable.
pub async fn reconcile_external_write(
    index: &IndexHandle,
    buffers: &SharedDirtyBuffers,
    change: &FileChanged,
    snapshot: BufferSnapshot,
) -> Result<ExternalWriteOutcome, OrgError> {
    let path = change.path.as_path();

    // Read dirtiness under ONE guard. A poisoned lock is treated as DIRTY
    // (fail-safe — never auto-reload over possibly-unsaved work), exactly the
    // rule `dirty_buffer.rs` documents. The `!Send` read guard is dropped here,
    // before any `.await`.
    let is_dirty = buffers.read().map_or(true, |guard| guard.is_dirty(path));
    if is_dirty {
        return Ok(ExternalWriteOutcome::DirtyConflict {
            path: change.path.clone(),
        });
    }

    // CLEAN branch: incrementally re-sync the index for this one file, then read
    // the fresh disk content and decide the cursor.
    let resync = resync_file(index, path).await?;

    let disk_content = match tokio::fs::read_to_string(path).await {
        Ok(content) => content,
        // A just-deleted file reloads to an empty buffer (its index rows were
        // already removed by the re-sync's delete branch).
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(OrgError::Io {
                reason: format!("failed to read {} for reload: {err}", path.display()),
            })
        }
    };

    let reload = reload::plan(snapshot.content(), disk_content, snapshot.cursor());

    Ok(ExternalWriteOutcome::CleanReload(CleanReload {
        path: change.path.clone(),
        reload,
        resync,
    }))
}
