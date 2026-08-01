//! Dirty Buffer manager: in-memory registry of open files with unsaved buffer
//! content, keyed by path (LD-7 + FR-16 + NFR-16).
//!
//! LD-7 makes the Single Writer Rule an integrity contract: Orgsidian must know
//! at all times which open `.org` files hold unsaved edits, so that an external
//! write can be routed correctly (FR-16: clean → auto-reload, dirty → Merge
//! Dialog) instead of silently clobbering the user's work. NFR-16 makes that
//! routing a reliability requirement, not a nicety.
//!
//! This module is the *scaffold*: the data structure and its API surface. It
//! performs no I/O, emits no events, and knows nothing about watchers or merge
//! UI — the Epic 5 watcher calls [`DirtyBufferManager::is_dirty`] on the
//! debounced-event path to pick the CLEAN/DIRTY branch, Epic 5's `ConflictState`
//! sources its buffer text from [`DirtyBufferManager::get_buffer`], and Epic 9's
//! Merge Dialog calls [`DirtyBufferManager::mark_clean`] after it atomic-writes
//! a resolution.
//!
//! # Sharing across threads
//!
//! [`DirtyBufferManager`] is a *plain* struct: `&self` for reads, `&mut self`
//! for mutations. Thread-safety is delivered by the caller's shared handle,
//! [`SharedDirtyBuffers`] (`Arc<RwLock<DirtyBufferManager>>`):
//!
//! ```
//! use std::path::Path;
//! use std::sync::{Arc, RwLock};
//! use orgsidian_vault::dirty_buffer::{DirtyBufferManager, SharedDirtyBuffers};
//!
//! let buffers: SharedDirtyBuffers = Arc::new(RwLock::new(DirtyBufferManager::new()));
//!
//! buffers.write().unwrap().mark_dirty("notes.org", "* TODO unsaved edit\n");
//! assert!(buffers.read().unwrap().is_dirty(Path::new("notes.org")));
//! ```
//!
//! `RwLock` rather than `Mutex` because the access pattern is strongly
//! read-skewed: the watcher hammers `is_dirty` on every debounced filesystem
//! event, while mutations happen only on keystroke-batch and save boundaries.
//! Keeping the lock *outside* the struct is what lets [`get_buffer`] return a
//! borrowed `Option<&str>` per the Epic 3 API contract — an interior lock would
//! force an owned `Option<String>` clone, since the borrow cannot outlive the
//! guard that produced it.
//!
//! [`get_buffer`]: DirtyBufferManager::get_buffer

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Shared handle for the Dirty Buffer registry across threads (see the module
/// docs). Epic 5/6 registers one of these when a Vault is opened.
pub type SharedDirtyBuffers = Arc<RwLock<DirtyBufferManager>>;

/// Tracks which open files have unsaved buffer content (LD-7 Single Writer
/// Rule, FR-16 external-write routing).
///
/// Pure in-memory registry: no I/O, no fallible operations, no `Result`. Keys
/// are the [`PathBuf`]s the caller supplies, verbatim — see [`Self::mark_dirty`]
/// for the path-identity contract.
#[derive(Debug, Default)]
pub struct DirtyBufferManager {
    buffers: HashMap<PathBuf, String>,
}

impl DirtyBufferManager {
    /// Empty registry — no file is dirty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `content` as the unsaved buffer for `path`, marking it dirty.
    ///
    /// Re-marking an already-dirty path **replaces** its stored content: the
    /// registry holds current buffer state, not an edit history.
    ///
    /// # Path identity
    ///
    /// `path` is used verbatim as the key — deliberately *not* canonicalized,
    /// lowercased, or symlink-resolved. Canonicalization is fallible I/O, which
    /// would drag a `Result` into an infallible type, and path-identity policy
    /// (case-folding on macOS/Windows, symlinks, relative-vs-absolute) is a
    /// cross-cutting Vault concern owned by the Vault-open layer. The contract
    /// here is exactly "same `PathBuf` in → same entry"; callers are
    /// responsible for handing in consistent paths.
    pub fn mark_dirty(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.buffers.insert(path.into(), content.into());
    }

    /// Drop any unsaved buffer for `path`, marking it clean.
    ///
    /// A no-op when `path` is not tracked — the caller does not need to check
    /// first. Epic 9's Merge Dialog depends on this: *accept* atomic-writes and
    /// then calls `mark_clean`, while *cancel* simply never calls it, leaving
    /// the buffer intact.
    pub fn mark_clean(&mut self, path: &Path) {
        self.buffers.remove(path);
    }

    /// Whether `path` holds unsaved edits.
    ///
    /// The hot read path: Epic 5's watcher calls this on every debounced
    /// filesystem event to choose auto-reload (clean) vs Merge Dialog (dirty).
    pub fn is_dirty(&self, path: &Path) -> bool {
        self.buffers.contains_key(path)
    }

    /// The currently-buffered content for a dirty `path`, or `None` when it is
    /// clean or untracked.
    ///
    /// Borrows rather than clones — consumers that need ownership (Epic 5
    /// building a `ConflictState`) clone at their call site, so read-only
    /// callers pay nothing.
    pub fn get_buffer(&self, path: &Path) -> Option<&str> {
        self.buffers.get(path).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// AC3: compile-time witness that the manager can cross thread boundaries
    /// and be shared — Epic 5 relies on this when it parks the manager in
    /// `tauri::State`. Cheaper and stronger than any runtime assertion.
    fn _assert_send_sync<T: Send + Sync>() {}

    fn path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn send_sync_contract() {
        _assert_send_sync::<DirtyBufferManager>();
    }

    #[test]
    fn lifecycle_clean_dirty_save_clean() {
        let mut mgr = DirtyBufferManager::new();
        let p = path("notes.org");

        // Fresh path: nothing tracked.
        assert!(!mgr.is_dirty(&p));
        assert_eq!(mgr.get_buffer(&p), None);

        // Edit arrives.
        mgr.mark_dirty(p.clone(), "* TODO unsaved\n");
        assert!(mgr.is_dirty(&p));
        assert_eq!(mgr.get_buffer(&p), Some("* TODO unsaved\n"));

        // Save completes.
        mgr.mark_clean(&p);
        assert!(!mgr.is_dirty(&p));
        assert_eq!(mgr.get_buffer(&p), None);
    }

    #[test]
    fn get_buffer_none_for_untracked_path() {
        let mgr = DirtyBufferManager::new();
        assert_eq!(mgr.get_buffer(Path::new("never-opened.org")), None);
    }

    #[test]
    fn re_mark_dirty_replaces_content() {
        let mut mgr = DirtyBufferManager::new();
        let p = path("draft.org");

        mgr.mark_dirty(p.clone(), "first\n");
        mgr.mark_dirty(p.clone(), "second\n");

        // The later buffer wins — the manager holds current state, not history.
        assert_eq!(mgr.get_buffer(&p), Some("second\n"));
        assert!(mgr.is_dirty(&p));
    }

    #[test]
    fn mark_clean_on_untracked_path_is_noop() {
        let mut mgr = DirtyBufferManager::new();
        let tracked = path("tracked.org");
        mgr.mark_dirty(tracked.clone(), "edit\n");

        // Must not panic, and must not disturb unrelated entries. Epic 9's
        // Merge-Dialog cancel path relies on clear-if-present semantics.
        mgr.mark_clean(Path::new("untracked.org"));

        assert!(mgr.is_dirty(&tracked));
        assert_eq!(mgr.get_buffer(&tracked), Some("edit\n"));
    }

    #[test]
    fn distinct_paths_tracked_independently() {
        let mut mgr = DirtyBufferManager::new();
        let a = path("a.org");
        let b = path("b.org");

        mgr.mark_dirty(a.clone(), "buffer a\n");
        mgr.mark_dirty(b.clone(), "buffer b\n");
        assert!(mgr.is_dirty(&a));
        assert!(mgr.is_dirty(&b));

        mgr.mark_clean(&a);
        assert!(!mgr.is_dirty(&a));
        assert!(
            mgr.is_dirty(&b),
            "cleaning one path must not clear the other"
        );
        assert_eq!(mgr.get_buffer(&b), Some("buffer b\n"));
    }

    /// Concurrency smoke test through the documented shared handle. Asserts on
    /// *final state after join* — never on timing or interleaving order, so it
    /// is deterministic on every platform and scheduler.
    #[test]
    fn shared_handle_is_race_free() {
        const THREADS: usize = 8;
        const EDITS_PER_THREAD: usize = 50;

        let buffers: SharedDirtyBuffers = Arc::new(RwLock::new(DirtyBufferManager::new()));

        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                let buffers = Arc::clone(&buffers);
                thread::spawn(move || {
                    let own = PathBuf::from(format!("thread-{t}.org"));
                    for edit in 0..EDITS_PER_THREAD {
                        buffers
                            .write()
                            .unwrap()
                            .mark_dirty(own.clone(), format!("edit {edit}\n"));
                        // Interleave reads on the hot path the watcher uses.
                        let _ = buffers.read().unwrap().is_dirty(&own);
                    }
                    // Half the threads finish by saving; half leave the buffer
                    // dirty, so the final assertion covers both outcomes.
                    if t % 2 == 0 {
                        buffers.write().unwrap().mark_clean(&own);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("worker thread panicked");
        }

        let mgr = buffers.read().unwrap();
        for t in 0..THREADS {
            let p = PathBuf::from(format!("thread-{t}.org"));
            if t % 2 == 0 {
                assert!(!mgr.is_dirty(&p), "thread {t} saved; must be clean");
                assert_eq!(mgr.get_buffer(&p), None);
            } else {
                assert!(mgr.is_dirty(&p), "thread {t} left edits; must be dirty");
                let expected = format!("edit {}\n", EDITS_PER_THREAD - 1);
                assert_eq!(
                    mgr.get_buffer(&p),
                    Some(expected.as_str()),
                    "last write for thread {t} must win"
                );
            }
        }
    }
}
