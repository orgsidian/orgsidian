//! Deterministic clock abstraction (LD-9).
//!
//! `Clock` is the time-reading surface consumed by the watcher's timeout discipline
//! and the perf-snapshot macros (Story 1.12). `FakeClock` is the test-only fake that
//! lets consumers advance time without wall-clock sleeps.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Instant;
}

#[derive(Clone)]
pub struct FakeClock {
    inner: Arc<Mutex<Instant>>,
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn advance(&self, dur: Duration) {
        let mut guard = self.inner.lock().expect("FakeClock mutex poisoned");
        *guard += dur;
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.inner.lock().expect("FakeClock mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_clock_advances_deterministically() {
        let clock = FakeClock::new();
        let t0 = clock.now();
        clock.advance(Duration::from_secs(10));
        let t1 = clock.now();
        assert_eq!(t1.duration_since(t0), Duration::from_secs(10));
    }

    #[test]
    fn fake_clock_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<FakeClock>();
    }
}
