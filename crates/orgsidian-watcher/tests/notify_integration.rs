//! Story 5.1 — real `notify-rs` end-to-end detection (LD-9 NFR).
//!
//! Proves the production `NotifyEventSource` + `WatcherFacade` path actually
//! observes an external write and surfaces a debounced `FileChanged` within the
//! 5-second budget on the host platform (the CI matrix runs macOS/Linux/Windows).
//! Debounce *timing* is asserted deterministically elsewhere with a `FakeClock`
//! (`src/watcher.rs` unit tests); this test works over real OS events.
//!
//! Scope boundary: this verifies 5.1's contract — timely detection plus the
//! *per-path* 250ms coalescing at the real filesystem boundary. It deliberately
//! does NOT assert "exactly one `FileChanged` per editor save": on macOS FSEvents
//! reports both the file and its parent directory for one write, and a temp+rename
//! atomic save touches two paths — collapsing those into one logical save is
//! trace-calibration work owned by Story 5.2 (golden traces).

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use orgsidian_watcher::{
    FileChanged, NotifyEventSource, SystemClock, WatchError, WatcherFacade, DEBOUNCE_WINDOW,
};

// Drive the facade over real OS events, collecting every `FileChanged` until the
// stream goes quiet (`quiet` with no new emission) or the overall budget expires.
fn collect_until_quiet(
    facade: &mut WatcherFacade<NotifyEventSource, SystemClock>,
    rx: &std::sync::mpsc::Receiver<FileChanged>,
    budget: Duration,
    quiet: Duration,
) -> Vec<PathBuf> {
    let mut seen = Vec::new();
    let start = Instant::now();
    let mut last_change = Instant::now();
    loop {
        facade.pump_once();
        let mut got_one = false;
        while let Ok(change) = rx.try_recv() {
            seen.push(change.path);
            got_one = true;
        }
        if got_one {
            last_change = Instant::now();
        }
        if !seen.is_empty() && last_change.elapsed() >= quiet {
            break;
        }
        if start.elapsed() >= budget {
            break;
        }
    }
    seen
}

#[test]
fn external_write_burst_is_detected_within_5s_and_coalesced_per_path() {
    let dir = tempfile::TempDir::new().expect("TempDir must succeed");
    let target = dir.path().join("watched.org");
    fs::write(&target, b"initial\n").expect("initial write must succeed");

    // OS backends report canonicalized paths (e.g. macOS FSEvents resolves the
    // `/var` → `/private/var` symlink).
    let canonical_root = fs::canonicalize(dir.path()).expect("canonicalize watch root");

    let source = NotifyEventSource::watch(dir.path()).expect("watch must arm");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut facade = WatcherFacade::new(source, SystemClock, DEBOUNCE_WINDOW, tx);

    // A rapid in-place write burst (an atomic save emits 3–12 raw events); every
    // write lands well within one debounce window.
    for i in 0..5 {
        fs::write(&target, format!("change {i}\n")).expect("burst write must succeed");
    }

    let paths = collect_until_quiet(
        &mut facade,
        &rx,
        Duration::from_secs(5),
        Duration::from_millis(600),
    );

    // Detected within the 5s NFR budget.
    assert!(
        !paths.is_empty(),
        "the external write burst must be detected within 5s"
    );
    // Every reported path lies within the watched tree.
    for path in &paths {
        assert!(
            path.starts_with(&canonical_root),
            "reported path {path:?} must lie within the watched tree {canonical_root:?}"
        );
    }
    // Per-path coalescing at the real boundary: the whole burst settles to a
    // single emission per path — no path is emitted more than once.
    let unique: HashSet<&PathBuf> = paths.iter().collect();
    assert_eq!(
        unique.len(),
        paths.len(),
        "each path must coalesce to exactly one FileChanged; got {paths:?}"
    );
}

#[test]
fn watching_a_missing_path_returns_a_typed_error() {
    let dir = tempfile::TempDir::new().expect("TempDir must succeed");
    let missing = dir.path().join("does-not-exist");
    match NotifyEventSource::watch(&missing) {
        Err(WatchError::Notify(_)) => {}
        Ok(_) => panic!("watching a nonexistent path must fail"),
    }
}
