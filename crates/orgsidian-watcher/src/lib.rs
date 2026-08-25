//! orgsidian-watcher: filesystem watcher (notify-rs) + debounce + external-edits reconciliation (FR-16).
//!
//! Story 5.1 lands the real watcher in [`watcher`]: a `notify-rs` wrapper whose
//! [`watcher::Debouncer`] coalesces atomic-save event bursts into a single
//! [`watcher::FileChanged`] per path, exposed through [`watcher::WatcherFacade`]
//! over three trait-based seams ([`watcher::EventSource`], the [`Clock`] facade,
//! and an output sink) so the debounce is unit-testable with deterministic
//! fakes. See the [`watcher`] module docs for the design.
//!
//! [`detect_first_write_event`] remains the Story 1.9 anchor-smoke surface
//! (signature preserved — anchor sentinel discipline, see `tests/anchor.rs`).
//! Its body now blocks on the same `notify-rs` subscription for wakeups instead
//! of a fixed mtime spin, while `metadata().modified()` stays the authoritative
//! change confirmation and the injected [`Clock`] still gates the timeout.

pub mod watcher;

pub use watcher::{
    Debouncer, EventSource, FileChanged, NotifyEventSource, PumpStatus, RawEvent, RawKind,
    RecvOutcome, SystemClock, WatchError, WatcherFacade, DEBOUNCE_WINDOW,
};

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
/// (see `tests/anchor.rs` for the pattern). The production [`SystemClock`] and the
/// [`WatcherFacade`] both consume this facade.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> std::time::Instant;
}

#[derive(Debug)]
pub struct DetectedEvent {
    pub mtime: SystemTime,
}

/// Errors from [`detect_first_write_event`].
///
/// Story 5.1 widened this beyond the Story 1.9 `Timeout`/`Io` surface: arming
/// the `notify-rs` subscription can now fail before any polling begins, surfaced
/// as [`DetectError::Watch`]. The function *signature* is preserved (anchor
/// sentinel discipline), but callers that exhaustively matched the old two
/// variants must handle the new one — hence `#[non_exhaustive]`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DetectError {
    #[error("watcher timeout after {0:?}")]
    Timeout(Duration),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Watch(#[from] WatchError),
}

/// How long a single blocking read from the notify subscription waits before
/// looping back to re-check mtime and the deadline. Small enough that a frozen
/// `FakeClock` anchor test still detects the write promptly via the mtime
/// backstop; the notify wakeup returns sooner whenever a real event lands.
const WAKEUP_SLICE: Duration = Duration::from_millis(50);

/// Detect the first external write to `path`. Returns `Ok(DetectedEvent)` once
/// `path`'s mtime changes vs. its initial reading, or `Err(DetectError::Timeout)`
/// if `clock.now()` advances past `start + deadline` first.
///
/// Story 5.1 swapped the Story 1.9 mtime spin for a `notify-rs` subscription:
/// the loop blocks on [`NotifyEventSource`] wakeups (so it reacts as soon as the
/// OS reports activity) but confirms the change through `metadata().modified()`,
/// which is authoritative and non-flaky. The deadline check is `Clock`-driven —
/// a `FakeClock` can advance past the deadline without real seconds passing, and
/// the mtime backstop guarantees a frozen `FakeClock` still detects a real write.
pub fn detect_first_write_event(
    path: &Path,
    clock: &dyn Clock,
    deadline: Duration,
) -> Result<DetectedEvent, DetectError> {
    let initial_mtime = std::fs::metadata(path)?.modified()?;
    // notify watches directories; watch the file's parent so atomic renames
    // (which touch the parent) are observed. `parent()` yields `Some("")` for a
    // bare relative filename — treat that empty path like `None` and fall back
    // to the current directory rather than watching an empty path (which errors
    // at the OS layer).
    let watch_root = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut source = NotifyEventSource::watch(watch_root)?;
    let start = clock.now();

    loop {
        // Block for a wakeup: a real event returns immediately; otherwise the
        // slice elapses and we re-check mtime + the deadline below.
        if let RecvOutcome::Disconnected = source.recv_timeout(WAKEUP_SLICE) {
            return Err(DetectError::Timeout(deadline));
        }

        // Re-read the target's mtime. During an atomic delete+create the file is
        // transiently absent — treat `NotFound` as "keep waiting for the write"
        // rather than aborting detection with an error.
        match std::fs::metadata(path).and_then(|meta| meta.modified()) {
            Ok(mtime) if mtime != initial_mtime => return Ok(DetectedEvent { mtime }),
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(DetectError::Io(err)),
        }
        if clock.now().saturating_duration_since(start) > deadline {
            return Err(DetectError::Timeout(deadline));
        }
    }
}
