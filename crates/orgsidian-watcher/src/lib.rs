//! orgsidian-watcher: filesystem watcher (notify-rs) + debounce + external-edits reconciliation (FR-10).
//!
//! Story 1.9 ships the anchor-smoke surface only — `detect_first_write_event`
//! polls `metadata().modified()` mtime via a 10ms sleep loop and gates the
//! timeout decision on the injected `Clock`. Story 5.1 swaps the polling body
//! for `notify-rs` event subscription + the vim/VS Code/Emacs debounce
//! calibration; the public signature
//! `detect_first_write_event(path, clock, deadline) -> Result<DetectedEvent, DetectError>`
//! is preserved across that swap (anchor sentinel discipline — see
//! `tests/anchor.rs`).

use std::path::Path;
use std::time::{Duration, SystemTime};

use thiserror::Error;

/// Production-visible `Clock` facade.
///
/// Declared independently of `orgsidian_core::test_support::clock::Clock`
/// because the core trait is gated behind `cfg(any(test, feature = "test-support"))`
/// and is therefore unavailable in release builds. Shape (`Send + Sync + 'static` +
/// `fn now(&self) -> Instant`) matches the LD-9 trait exactly; consumers that already
/// implement the core trait bridge into this one via a tiny newtype adapter
/// (see `tests/anchor.rs` for the pattern). Story 5.1 revisits the duplication when
/// notify-rs lands and the watcher needs a stable timeout discipline beyond tests.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> std::time::Instant;
}

#[derive(Debug)]
pub struct DetectedEvent {
    pub mtime: SystemTime,
}

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("watcher timeout after {0:?}")]
    Timeout(Duration),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Poll `path` for the first mtime change vs. its initial reading. Returns
/// `Ok(DetectedEvent)` on detection, or `Err(DetectError::Timeout)` if
/// `clock.now()` advances past `start + deadline` first.
///
/// The 10ms inter-poll `thread::sleep` is wall-clock (OS scheduler controls
/// poll cadence); the deadline check is `Clock`-driven (consumer can inject a
/// `FakeClock` that advances past the deadline without real seconds passing).
/// Story 5.1 replaces the polling body with a `notify-rs` event subscription;
/// the `Clock`-driven timeout discipline survives that swap unchanged.
pub fn detect_first_write_event(
    path: &Path,
    clock: &dyn Clock,
    deadline: Duration,
) -> Result<DetectedEvent, DetectError> {
    let initial_mtime = std::fs::metadata(path)?.modified()?;
    let start = clock.now();

    loop {
        let mtime = std::fs::metadata(path)?.modified()?;
        if mtime != initial_mtime {
            return Ok(DetectedEvent { mtime });
        }
        if clock.now().saturating_duration_since(start) > deadline {
            return Err(DetectError::Timeout(deadline));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
