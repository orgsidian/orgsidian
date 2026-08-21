//! Story 5.4 (FR-16): clean-buffer auto-reload + incremental re-index on an
//! external write, driven end-to-end through the watcher.
//!
//! Each test builds a real `TempDir` vault + a `TempDir` index DB (hermetic:
//! `open_index` takes the DB path directly, so no OS data dir / `global.toml`),
//! runs the initial scan, performs an external write on disk, drives a real
//! `orgsidian_watcher::WatcherFacade` (via the public `EventSource`/`Clock`
//! seams, with an injected fake clock — no real sleeps) to emit the debounced
//! `FileChanged`, then reconciles it and asserts the buffer + index + cursor
//! invariants.
//!
//! The Story 5.2 golden traces (recorded vim/VS Code/Emacs save sequences) are
//! built in parallel and absent from this branch. This test drives the SAME
//! `EventSource` seam with a minimal synthetic external-write burst; when 5.2
//! merges, replaying its golden traces stacks by swapping the fake source's
//! scripted events — no reconcile-side change.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use orgsidian_core::{
    open_index, reconcile_external_write, resync_file, scan_vault, BufferSnapshot, CursorOutcome,
    CursorPosition, ExternalWriteOutcome, IndexHandle, ResyncOutcome, RELOAD_NOTICE,
    RELOAD_NOTICE_DURATION,
};
use orgsidian_vault::dirty_buffer::{DirtyBufferManager, SharedDirtyBuffers};
use orgsidian_watcher::{
    Clock, EventSource, FileChanged, PumpStatus, RawEvent, RawKind, RecvOutcome, WatcherFacade,
    DEBOUNCE_WINDOW,
};
use tempfile::TempDir;

// ---- fixtures -------------------------------------------------------------

/// A vault dir + an index-DB path, each in its own `TempDir`.
struct Fixture {
    _vault: TempDir,
    _index: TempDir,
    vault_root: PathBuf,
    db_path: PathBuf,
}

fn fixture() -> Fixture {
    let vault = TempDir::new().expect("vault tempdir");
    let index = TempDir::new().expect("index tempdir");
    let vault_root = vault.path().to_path_buf();
    let db_path = index.path().join("index.sqlite3");
    Fixture {
        _vault: vault,
        _index: index,
        vault_root,
        db_path,
    }
}

/// Open + initial-scan a fixture, returning the live handle. Uses the handle's
/// *canonical* vault root for every subsequent path so `to_rel_path` /
/// `is_dirty` / the disk read all key on the same bytes (macOS resolves
/// `/var`→`/private/var`).
async fn open_and_scan(fx: &Fixture) -> IndexHandle {
    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");
    let cancel = std::sync::atomic::AtomicBool::new(false);
    scan_vault(&handle, &cancel, |_| {}).await.expect("scan");
    handle
}

/// One integer aggregate from a fresh read connection (WAL readers see the
/// writer's committed frames while the handle's writer is still live).
fn count(db_path: &Path, sql: &str) -> i64 {
    let conn = orgsidian_index::open(db_path).expect("open index for read");
    conn.query_row(sql, (), |row| row.get::<_, i64>(0))
        .expect("count query")
}

// ---- watcher seam fakes (public traits; no real sleeps) -------------------

/// A scriptable `EventSource`: yields its queued raw events in order, then
/// reports `Timeout`. Ignores the real timeout so no wall-clock time passes.
struct FakeSource {
    queue: VecDeque<RawEvent>,
}

impl EventSource for FakeSource {
    fn recv_timeout(&mut self, _timeout: Duration) -> RecvOutcome {
        match self.queue.pop_front() {
            Some(event) => RecvOutcome::Event(event),
            None => RecvOutcome::Timeout,
        }
    }
}

/// A `FakeClock` bridged to the watcher's `Clock` facade: reads whatever instant
/// the test has advanced it to (same role as `orgsidian-core`'s `FakeClock`,
/// kept local so the test needs no `test-support` feature gate).
#[derive(Clone)]
struct FakeClock(Arc<Mutex<Instant>>);

impl FakeClock {
    fn new() -> Self {
        FakeClock(Arc::new(Mutex::new(Instant::now())))
    }
    fn advance(&self, delta: Duration) {
        let mut now = self.0.lock().unwrap();
        *now += delta;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}

/// Drive a real `WatcherFacade` over a synthetic external-write burst on `path`
/// and return the single debounced `FileChanged` it emits — the exact type the
/// reconciler consumes. Fully deterministic: the burst is scripted through the
/// `EventSource` seam and the debounce window closes via the injected clock.
fn watch_one_external_write(path: &Path) -> FileChanged {
    let clock = FakeClock::new();
    // A minimal atomic-save-shaped burst (remove+create+modify) on the path.
    let source = FakeSource {
        queue: VecDeque::from(vec![
            RawEvent {
                paths: vec![path.to_path_buf()],
                kind: RawKind::Remove,
            },
            RawEvent {
                paths: vec![path.to_path_buf()],
                kind: RawKind::Create,
            },
            RawEvent {
                paths: vec![path.to_path_buf()],
                kind: RawKind::Modify,
            },
        ]),
    };
    let (tx, rx): (Sender<FileChanged>, _) = channel();
    let mut facade = WatcherFacade::new(source, clock.clone(), DEBOUNCE_WINDOW, tx);

    // Drain the burst while the clock is frozen: nothing settles yet.
    for _ in 0..3 {
        assert_eq!(facade.pump_once(), PumpStatus::Continue);
    }
    assert!(
        rx.try_recv().is_err(),
        "no emission before the window elapses"
    );

    // Advance past the debounce window and pump once more: exactly one change.
    clock.advance(DEBOUNCE_WINDOW);
    assert_eq!(facade.pump_once(), PumpStatus::Continue);
    let change = rx.try_recv().expect("one debounced FileChanged");
    assert!(rx.try_recv().is_err(), "burst coalesced to a single event");
    assert_eq!(change.path, path);
    change
}

fn clean_buffers() -> SharedDirtyBuffers {
    Arc::new(std::sync::RwLock::new(DirtyBufferManager::new()))
}

// v1 / v2 differ only on the last headline. Line index 1 ("shared body line")
// is unchanged; line index 2 (the Beta headline) changes.
const V1: &str = "* Alpha\nshared body line\n* Beta v1\n";
const V2: &str = "* Alpha\nshared body line\n* Beta v2 CHANGED\n";

// ---- tests ----------------------------------------------------------------

/// Matrix: clean reload, line unchanged. Buffer refreshed to disk, cursor
/// Preserved, index incrementally re-synced (`Upserted`).
#[tokio::test(flavor = "multi_thread")]
async fn clean_write_reloads_buffer_reindexes_and_preserves_unchanged_line() {
    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    // Initial index reflects v1.
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v1'"
        ),
        1
    );

    // External write: v2 lands on disk (buffer is CLEAN — never marked dirty).
    fs::write(&notes, V2).expect("external write v2");
    let change = watch_one_external_write(&notes);

    // Cursor on line index 1 ("shared body line") — unchanged between v1 and v2.
    let snapshot = BufferSnapshot::new(V1, CursorPosition::new(1, 4));
    let outcome = reconcile_external_write(&handle, &clean_buffers(), &change, snapshot)
        .await
        .expect("reconcile");

    let reload = match outcome {
        ExternalWriteOutcome::CleanReload(reload) => reload,
        other => panic!("expected CleanReload, got {other:?}"),
    };

    // Buffer refreshed from disk, tagged with the file it belongs to.
    assert_eq!(reload.new_content(), V2);
    assert_eq!(reload.path(), notes);
    // Index incrementally re-synced for this one file.
    assert_eq!(reload.resync(), ResyncOutcome::Upserted);
    // Cursor preserved at the unchanged line.
    assert_eq!(
        reload.cursor(),
        CursorOutcome::Preserved(CursorPosition::new(1, 4))
    );
    // The 3-second "file reloaded from disk" notice is carried on the outcome.
    assert_eq!(reload.notice(), RELOAD_NOTICE);
    assert_eq!(reload.notice_duration(), RELOAD_NOTICE_DURATION);
    // Pin the exact wire values the frontend/aria-live surface depends on.
    assert_eq!(RELOAD_NOTICE, "file reloaded from disk");
    assert_eq!(RELOAD_NOTICE_DURATION, Duration::from_secs(3));

    // Index now reflects v2: the new headline is present, the old one gone.
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v2 CHANGED'"
        ),
        1
    );
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v1'"
        ),
        0
    );
    // Still exactly one file row (upsert-in-place, not a duplicate insert).
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 1);

    handle.shutdown().await;
}

/// Matrix: clean reload, line changed. Cursor resets to top; buffer still
/// refreshed.
#[tokio::test(flavor = "multi_thread")]
async fn clean_write_resets_cursor_when_line_changed() {
    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    fs::write(&notes, V2).expect("external write v2");
    let change = watch_one_external_write(&notes);

    // Cursor on line index 2 (the Beta headline) — its text changed v1→v2.
    let snapshot = BufferSnapshot::new(V1, CursorPosition::new(2, 3));
    let outcome = reconcile_external_write(&handle, &clean_buffers(), &change, snapshot)
        .await
        .expect("reconcile");

    let reload = match outcome {
        ExternalWriteOutcome::CleanReload(reload) => reload,
        other => panic!("expected CleanReload, got {other:?}"),
    };
    assert_eq!(reload.new_content(), V2);
    assert_eq!(reload.cursor(), CursorOutcome::ResetToTop);
    assert_eq!(reload.cursor().resolved(), CursorPosition::TOP);

    handle.shutdown().await;
}

/// Matrix: dirty buffer. The reconciler returns the Story-5.5 conflict SEAM,
/// writes nothing, and leaves the index untouched.
#[tokio::test(flavor = "multi_thread")]
async fn dirty_buffer_yields_conflict_seam_and_leaves_index_untouched() {
    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    // The buffer holds unsaved edits: mark it dirty under the SAME path key the
    // watcher will report.
    let buffers = clean_buffers();
    buffers
        .write()
        .unwrap()
        .mark_dirty(notes.clone(), "* Alpha\nlocal unsaved edit\n");

    fs::write(&notes, V2).expect("external write v2");
    let change = watch_one_external_write(&notes);

    let snapshot = BufferSnapshot::new(V1, CursorPosition::new(0, 0));
    let outcome = reconcile_external_write(&handle, &buffers, &change, snapshot)
        .await
        .expect("reconcile");

    match outcome {
        ExternalWriteOutcome::DirtyConflict { path } => assert_eq!(path, notes),
        other => panic!("expected DirtyConflict, got {other:?}"),
    }

    // Index untouched — still v1 (never auto-reloaded over unsaved work).
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v1'"
        ),
        1
    );
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v2 CHANGED'"
        ),
        0
    );

    handle.shutdown().await;
}

/// Safety-critical fail-safe: a POISONED Dirty-Buffer lock is treated as DIRTY,
/// so auto-reload never runs over possibly-unsaved work even when the registry's
/// state is unknowable (the `map_or(true, …)` branch in `reconcile_external_write`).
#[tokio::test(flavor = "multi_thread")]
async fn poisoned_lock_fails_safe_to_conflict_and_leaves_index_untouched() {
    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    // Poison the lock: panic while holding the write guard on another thread.
    let buffers = clean_buffers();
    {
        let poisoner = Arc::clone(&buffers);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.write().unwrap();
            panic!("intentional panic to poison the dirty-buffer lock");
        })
        .join();
    }
    assert!(buffers.read().is_err(), "lock is poisoned");

    fs::write(&notes, V2).expect("external write v2");
    let change = watch_one_external_write(&notes);

    let snapshot = BufferSnapshot::new(V1, CursorPosition::new(1, 0));
    let outcome = reconcile_external_write(&handle, &buffers, &change, snapshot)
        .await
        .expect("reconcile");

    match outcome {
        ExternalWriteOutcome::DirtyConflict { path } => assert_eq!(path, notes),
        other => panic!("poisoned lock must fail safe to DirtyConflict, got {other:?}"),
    }
    // The index was NOT re-synced — the CLEAN branch never ran.
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v1'"
        ),
        1
    );
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE title = 'Beta v2 CHANGED'"
        ),
        0
    );

    handle.shutdown().await;
}

/// Matrix: external delete of a clean file. The index rows are removed
/// (`Deleted`) and the buffer refreshes to empty.
#[tokio::test(flavor = "multi_thread")]
async fn external_delete_of_clean_file_removes_index_rows_and_empties_buffer() {
    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 1);

    // External delete.
    fs::remove_file(&notes).expect("external delete");
    let change = watch_one_external_write(&notes);

    let snapshot = BufferSnapshot::new(V1, CursorPosition::new(1, 0));
    let outcome = reconcile_external_write(&handle, &clean_buffers(), &change, snapshot)
        .await
        .expect("reconcile");

    let reload = match outcome {
        ExternalWriteOutcome::CleanReload(reload) => reload,
        other => panic!("expected CleanReload, got {other:?}"),
    };
    assert_eq!(reload.resync(), ResyncOutcome::Deleted);
    assert_eq!(reload.new_content(), "", "a deleted file reloads to empty");
    // The whole file row (and its headlines) is gone from the index.
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 0);

    handle.shutdown().await;
}

/// Matrix: unreadable/unparseable external write. The single-file re-sync
/// quarantines the file (LD-41) rather than erroring. Unix-only (needs a
/// genuinely unreadable file); skipped under root, which bypasses mode bits.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn unreadable_external_write_quarantines() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        eprintln!("[skip] root bypasses mode bits; chmod 000 cannot make a file unreadable");
        return;
    }

    let fx = fixture();
    fs::write(fx.vault_root.join("notes.org"), V1).expect("write v1");
    let handle = open_and_scan(&fx).await;
    let notes = handle.vault_root().join("notes.org");

    // Make the file unreadable so the read+parse fails (LD-41 read-failure path).
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o000)).expect("chmod 000");

    // Drive the re-sync directly: a clean-buffer reconcile would fail on the
    // disk read for the reload (an unreadable file cannot refresh the buffer),
    // so the matrix row's index outcome is asserted at the re-sync boundary.
    let outcome = resync_file(&handle, &notes).await.expect("resync");
    assert_eq!(outcome, ResyncOutcome::Quarantined);
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM files WHERE quarantined = 1"
        ),
        1
    );

    // Restore permissions so the TempDir cleanup can remove the file.
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o644)).expect("restore perms");
    handle.shutdown().await;
}
