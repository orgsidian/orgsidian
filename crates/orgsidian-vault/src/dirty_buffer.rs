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
//! UI. The consumers it is shaped for do not exist yet — stated here as intent,
//! not as description of code that ships today: the Epic 5 watcher is expected
//! to call [`DirtyBufferManager::is_dirty`] on the debounced-event path to pick
//! the CLEAN/DIRTY branch, Epic 5's `ConflictState` to source its buffer text
//! from [`DirtyBufferManager::get_buffer`], and Epic 9's Merge Dialog to call
//! [`DirtyBufferManager::mark_clean`] after it atomic-writes a resolution.
//! Nothing here enforces those contracts; when a consumer lands and diverges,
//! fix this paragraph rather than trusting it.
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
//! let notes = Path::new("notes.org");
//!
//! buffers
//!     .write()
//!     .expect("dirty buffers lock poisoned")
//!     .mark_dirty(notes, "* TODO unsaved edit\n");
//!
//! // Branch and read the buffer under ONE guard — see "Reading atomically".
//! let guard = buffers.read().expect("dirty buffers lock poisoned");
//! if guard.is_dirty(notes) {
//!     assert_eq!(guard.get_buffer(notes), Some("* TODO unsaved edit\n"));
//! }
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
//! Note the cost of that borrow: a [`std::sync::RwLockReadGuard`] is `!Send`, so
//! the `&str` cannot be held across an `.await`. An async consumer (reading the
//! on-disk file to diff against the buffer, awaiting a Merge Dialog result) must
//! `to_owned()` it before yielding — recorded in `deferred-work.md` so Epic 5 is
//! not surprised.
//!
//! # Reading atomically
//!
//! [`is_dirty`] and [`get_buffer`] are separate calls. Acquiring a *separate*
//! read guard for each opens a TOCTOU window: a `mark_clean` landing between
//! them (a save completing, a Merge Dialog accepting) makes `is_dirty` answer
//! `true` and the following `get_buffer` answer `None`, leaving the caller in
//! the DIRTY branch with no buffer to show. **Hold one guard across both calls**
//! — as the example above does. The same rule covers any multi-path check, such
//! as LD-57 Refile's both-clean precondition: one guard, both lookups.
//!
//! # Lock poisoning
//!
//! [`std::sync::RwLock`] poisons permanently once a thread panics while holding
//! a guard, and every later `read()`/`write()` returns `Err`. Because this
//! registry is the oracle that decides whether an external write may overwrite
//! unsaved work, callers must **fail safe to DIRTY**, never to clean:
//!
//! ```
//! # use std::path::Path;
//! # use std::sync::{Arc, RwLock};
//! # use orgsidian_vault::dirty_buffer::{DirtyBufferManager, SharedDirtyBuffers};
//! fn is_dirty_or_unknown(buffers: &SharedDirtyBuffers, path: &Path) -> bool {
//!     // A poisoned lock means "state unknown" — treat it as dirty so the
//!     // conflict path runs. `unwrap_or(false)` here would auto-reload over
//!     // the user's edits, the exact FR-16 failure this module exists to stop.
//!     buffers.read().map_or(true, |guard| guard.is_dirty(path))
//! }
//! # let b: SharedDirtyBuffers = Arc::new(RwLock::new(DirtyBufferManager::new()));
//! # assert!(!is_dirty_or_unknown(&b, Path::new("notes.org")));
//! ```
//!
//! The `.expect(…)` calls in the first example are doctest-local brevity, not a
//! recommendation for production wiring.
//!
//! [`get_buffer`]: DirtyBufferManager::get_buffer
//! [`is_dirty`]: DirtyBufferManager::is_dirty

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Shared handle for the Dirty Buffer registry across threads (see the module
/// docs). Epic 5/6 registers one of these when a Vault is opened.
///
/// Locking is the holder's responsibility, including the two rules the module
/// docs spell out: hold **one** guard across an [`DirtyBufferManager::is_dirty`]
/// → [`DirtyBufferManager::get_buffer`] pair, and treat a poisoned lock as
/// **dirty**, never as clean.
pub type SharedDirtyBuffers = Arc<RwLock<DirtyBufferManager>>;

/// Tracks which open files have unsaved buffer content (LD-7 Single Writer
/// Rule, FR-16 external-write routing).
///
/// Pure in-memory registry: no I/O, no fallible operations, no `Result`. Keys
/// are the [`PathBuf`]s the caller supplies, verbatim — see [`Self::mark_dirty`]
/// for the path-identity contract.
#[derive(Default)]
pub struct DirtyBufferManager {
    buffers: HashMap<PathBuf, String>,
}

/// Redacting `Debug`: prints tracked paths and buffer *sizes*, never buffer
/// content.
///
/// The stored `String`s are the user's unsaved notes. A derived `Debug` would
/// spill them verbatim into any `tracing` line, `.expect()` message, or panic
/// backtrace that formats enclosing application state — a privacy leak into
/// files the user never consented to write. Sizes are enough to debug the
/// registry itself.
impl fmt::Debug for DirtyBufferManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirtyBufferManager")
            .field(
                "buffers",
                &self
                    .buffers
                    .iter()
                    .map(|(path, content)| (path, content.len()))
                    .collect::<HashMap<_, _>>(),
            )
            .finish()
    }
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
    ///
    /// # Content contract
    ///
    /// `content` must be a **lossless UTF-8 round-trip** of the file's bytes. A
    /// legacy non-UTF-8 `.org` file read through `String::from_utf8_lossy` would
    /// arrive here with `U+FFFD` where the original bytes were, and a later
    /// merge-accept would write those replacement characters back over content
    /// the user never edited. Files that do not round-trip must not be routed
    /// through the buffer path at all.
    ///
    /// Empty `content` is a legitimate dirty state (select-all, delete, unsaved)
    /// — see [`Self::is_dirty`] for why emptiness is never treated as clean.
    pub fn mark_dirty(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.buffers.insert(path.into(), content.into());
    }

    /// Drop any unsaved buffer for `path`, marking it clean.
    ///
    /// A no-op when `path` is not tracked — the caller does not need to check
    /// first. Epic 9's Merge Dialog is expected to rely on this: *accept*
    /// atomic-writes and then calls `mark_clean`, while *cancel* simply never
    /// calls it, leaving the buffer intact.
    ///
    /// The dropped buffer is not returned (the signature is pinned by the Epic 3
    /// contract), so a caller cannot tell a clean that removed a buffer from one
    /// that matched nothing, and has no undo material if it cleaned the wrong
    /// path. Both consequences are recorded in `deferred-work.md`.
    pub fn mark_clean(&mut self, path: &Path) {
        self.buffers.remove(path);
    }

    /// Whether `path` holds unsaved edits.
    ///
    /// The hot read path: the Epic 5 watcher is expected to call this on every
    /// debounced filesystem event to choose auto-reload (clean) vs Merge Dialog
    /// (dirty).
    ///
    /// Dirtiness is **key presence**, nothing else. It is not content emptiness
    /// (`mark_dirty(p, "")` is dirty — the user emptied the file and has not
    /// saved) and it is not disk inequality (this type never reads the disk).
    /// Consumers crossing the Tauri IPC boundary must branch on this method, not
    /// on the truthiness of the buffer string: `Some("")` serializes to a
    /// JS-falsy `""` and a frontend `if (buffer)` would read a genuinely dirty
    /// file as clean.
    ///
    /// # `false` means clean **or** not open
    ///
    /// A saved file and a never-opened file are indistinguishable here — both
    /// are simply absent from the map. A consumer that needs "is this file
    /// open?" (deciding whether to react to an event at all, or the LD-41
    /// external-delete-with-dirty-buffer branch) cannot get it from this type
    /// and must track openness itself.
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

    /// AC3: compile-time witness that the registry can cross thread boundaries
    /// and be shared. Cheaper and stronger than any runtime assertion.
    fn _assert_send_sync<T: Send + Sync>() {}

    /// The bound `tauri::State` actually demands of the value parked in it.
    /// Asserting it on the inner type alone would leave the alias — the thing
    /// Epic 5/6 registers — unwitnessed.
    fn _assert_shareable<T: Send + Sync + 'static>() {}

    fn path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn send_sync_contract() {
        _assert_send_sync::<DirtyBufferManager>();
        _assert_shareable::<SharedDirtyBuffers>();
    }

    /// The keying contract is "same `PathBuf` in → same entry" and nothing more
    /// (AC2 + Dev Note 3). Pinning it matters *because* it is the sharp edge: a
    /// well-meant `to_lowercase()` or `canonicalize()` inside `mark_dirty` would
    /// otherwise pass the whole suite while silently making two distinct files
    /// on a case-sensitive volume share one buffer.
    #[test]
    fn path_spellings_are_distinct_keys() {
        let mut mgr = DirtyBufferManager::new();
        mgr.mark_dirty(path("a.org"), "edit\n");

        assert!(mgr.is_dirty(Path::new("a.org")));
        assert!(!mgr.is_dirty(Path::new("./a.org")), "no path normalization");
        assert!(!mgr.is_dirty(Path::new("/vault/a.org")), "no absolutizing");
        assert!(!mgr.is_dirty(Path::new("A.ORG")), "no case folding");
        assert_eq!(mgr.get_buffer(Path::new("./a.org")), None);
    }

    /// An emptied-but-unsaved file is dirty. Guards the boundary a frontend
    /// `if (buffer)` would collapse: `Some("")` is not `None`.
    #[test]
    fn empty_buffer_is_still_dirty() {
        let mut mgr = DirtyBufferManager::new();
        let p = path("emptied.org");

        mgr.mark_dirty(p.clone(), "");

        assert!(mgr.is_dirty(&p), "dirtiness is key presence, not content");
        assert_eq!(mgr.get_buffer(&p), Some(""));

        mgr.mark_clean(&p);
        assert_eq!(mgr.get_buffer(&p), None, "clean is None, not Some(\"\")");
    }

    /// `Debug` must never leak buffer content — it is the user's unsaved notes,
    /// and any `{:?}` on enclosing app state would write it to a log file.
    #[test]
    fn debug_redacts_buffer_content() {
        let mut mgr = DirtyBufferManager::new();
        mgr.mark_dirty(path("secret.org"), "* private diary entry\n");

        let rendered = format!("{mgr:?}");

        assert!(!rendered.contains("private diary entry"), "{rendered}");
        assert!(rendered.contains("secret.org"), "paths stay visible");
        assert!(rendered.contains("22"), "byte length stays visible");
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

    /// The test above gives every thread a private key, so nothing ever
    /// contends. This one puts all threads on **one** path — writers racing
    /// `mark_dirty` against `mark_clean` while readers observe — and asserts
    /// the invariant that must hold at every observation, not just at the end:
    /// `is_dirty(p)` and `get_buffer(p).is_some()` never disagree.
    ///
    /// Each read takes a single guard, which is also the documented
    /// "Reading atomically" rule the Epic 5 watcher must follow — taking two
    /// guards here is exactly what would make this test flaky.
    #[test]
    fn contended_path_never_observes_split_state() {
        const THREADS: usize = 8;
        const CYCLES: usize = 200;

        let buffers: SharedDirtyBuffers = Arc::new(RwLock::new(DirtyBufferManager::new()));
        let contended = PathBuf::from("contended.org");

        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                let buffers = Arc::clone(&buffers);
                let p = contended.clone();
                thread::spawn(move || {
                    for cycle in 0..CYCLES {
                        match t % 3 {
                            0 => buffers
                                .write()
                                .expect("lock poisoned")
                                .mark_dirty(p.clone(), format!("edit {cycle} from {t}\n")),
                            1 => buffers.write().expect("lock poisoned").mark_clean(&p),
                            _ => {
                                // One guard for both calls — the split-state
                                // check is only meaningful if it is atomic.
                                let guard = buffers.read().expect("lock poisoned");
                                assert_eq!(
                                    guard.is_dirty(&p),
                                    guard.get_buffer(&p).is_some(),
                                    "is_dirty and get_buffer disagreed under contention"
                                );
                            }
                        }
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("worker thread panicked");
        }

        // Final state is scheduler-dependent by construction — assert only the
        // invariant, never which thread won.
        let mgr = buffers.read().expect("lock poisoned");
        assert_eq!(
            mgr.is_dirty(&contended),
            mgr.get_buffer(&contended).is_some()
        );
    }
}
