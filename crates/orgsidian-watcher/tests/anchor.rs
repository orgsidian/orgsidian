//! Story 1.9 — watcher anchor smoke (anti-placebo-green per Party Mode P2).
//!
//! Detects one real filesystem write within a `Clock`-deadline budget using a
//! deterministic `FakeClock`. Must keep passing across the Story 5.1 swap that
//! replaces the polling body with `notify-rs` event subscription.

use std::thread;
use std::time::Duration;

use orgsidian_core::test_support::clock::FakeClock;
use orgsidian_watcher::{detect_first_write_event, Clock};

// Local adapter: the watcher's `detect_first_write_event` takes the local
// `Clock` facade; `FakeClock` implements the core trait. This newtype lets the
// test inject the fake into the watcher API without exposing core's trait via
// the watcher's public surface.
struct ClockAdapter(FakeClock);

impl Clock for ClockAdapter {
    fn now(&self) -> std::time::Instant {
        use orgsidian_core::test_support::clock::Clock as _;
        self.0.now()
    }
}

#[test]
fn watcher_detects_first_write_within_clock_budget() {
    let dir = tempfile::TempDir::new().expect("anchor TempDir must succeed");
    let target = dir.path().join("watched.org");
    std::fs::write(&target, b"initial\n").expect("initial write must succeed");

    let initial_mtime = std::fs::metadata(&target)
        .expect("initial metadata must succeed")
        .modified()
        .expect("initial mtime must succeed");

    let writer_target = target.clone();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        std::fs::write(&writer_target, b"changed\n").expect("second write must succeed");
    });

    let fake = FakeClock::new();
    fake.advance(Duration::from_secs(0));
    let clock = ClockAdapter(fake);

    let result = detect_first_write_event(&target, &clock, Duration::from_secs(5));
    let event = result.expect("anchor detection must succeed within 5s budget");
    assert!(
        event.mtime > initial_mtime,
        "detected mtime must be strictly greater than initial mtime"
    );

    writer.join().expect("writer thread must join cleanly");
}
