//! FR-16 external-edits detection layer (LD-9).
//!
//! Wraps `notify-rs` and coalesces the 3–12 raw filesystem events an atomic
//! save emits (vim swap+rename, VS Code temp+rename, Emacs backup+save — each a
//! delete+create+modify burst) into a single logical [`FileChanged`] per path.
//! Downstream consumers (Epic 5 Single-Writer reconciliation, Epic 8 Capture)
//! therefore see one settled change per save instead of a burst that would
//! spuriously trip merge state machines.
//!
//! Three trait-based seams keep the debounce logic testable without a real
//! filesystem or wall-clock sleeps:
//!
//! - [`Debouncer`] — a pure per-path coalescing engine driven only by injected
//!   [`Instant`]s. Every raw event re-arms a `now + window` deadline; a path
//!   flushes to one [`FileChanged`] once its window elapses quietly.
//! - [`EventSource`] — the raw-event stream. Production wires
//!   [`NotifyEventSource`] (a `notify` `RecommendedWatcher`); tests script a
//!   fake.
//! - [`crate::Clock`] — the time source. Production wires [`SystemClock`];
//!   tests inject `orgsidian_core`'s `FakeClock` (via a tiny adapter — see
//!   `tests/anchor.rs`).
//!
//! [`WatcherFacade`] wires the three together behind a single-step
//! [`pump_once`](WatcherFacade::pump_once) so [`run`](WatcherFacade::run) can
//! loop it in production while tests advance a `FakeClock` between pumps and
//! assert emissions deterministically.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;

use crate::Clock;

/// Debounce window that coalesces one atomic-save event burst into a single
/// [`FileChanged`] (LD-7 / OD-3: 250ms for vim/VS Code/Emacs save sequences).
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(250);

/// Upper bound on how long an idle [`WatcherFacade::pump_once`] blocks waiting
/// for the next raw event. Caps the idle wakeup cadence; it never adds latency
/// to a real event (the source returns as soon as one arrives).
const IDLE_POLL: Duration = Duration::from_secs(1);

/// The single settled change the watcher emits per path after its debounce
/// window closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChanged {
    pub path: PathBuf,
}

/// Coarse classification of a raw filesystem event. Retained for trace fidelity
/// and debugging; the [`Debouncer`] coalesces every kind identically (an
/// atomic save is precisely a delete+create+modify mix that must collapse to
/// one change). `#[non_exhaustive]` so a future backend distinction (e.g. an
/// inotify-overflow rescan signal) can be added without breaking consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RawKind {
    Create,
    Modify,
    Remove,
    Other,
}

impl From<EventKind> for RawKind {
    fn from(kind: EventKind) -> Self {
        match kind {
            EventKind::Create(_) => RawKind::Create,
            EventKind::Modify(_) => RawKind::Modify,
            EventKind::Remove(_) => RawKind::Remove,
            _ => RawKind::Other,
        }
    }
}

/// A normalized raw filesystem event, decoupled from `notify`'s types so the
/// [`EventSource`] seam can be faked without depending on `notify` in tests.
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub paths: Vec<PathBuf>,
    pub kind: RawKind,
}

impl From<Event> for RawEvent {
    fn from(event: Event) -> Self {
        RawEvent {
            kind: RawKind::from(event.kind),
            paths: event.paths,
        }
    }
}

/// Outcome of a single blocking read from an [`EventSource`].
#[derive(Debug)]
pub enum RecvOutcome {
    Event(RawEvent),
    Timeout,
    Disconnected,
}

/// The raw-event stream seam. Production wraps `notify`; tests script events.
pub trait EventSource: Send {
    /// Block up to `timeout` for the next raw event.
    fn recv_timeout(&mut self, timeout: Duration) -> RecvOutcome;
}

/// Pure per-path debounce engine. Holds no clock and does no I/O: callers pass
/// the current [`Instant`], so `FakeClock`-driven tests are fully
/// deterministic.
///
/// This is a debounce, not a throttle: it has no max-wait ceiling, so a path
/// under a *sustained* sub-window event stream defers indefinitely. That is the
/// correct behavior for the atomic-save bursts it targets (each burst is
/// bounded and settles within one window); the only source of unbounded streams
/// — cloud-sync overlays — is a documented v0.1-unsupported configuration
/// (`docs/architecture/resilience.md`).
pub struct Debouncer {
    window: Duration,
    /// path → the instant at which the path becomes ready to flush.
    pending: HashMap<PathBuf, Instant>,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Debouncer {
            window,
            pending: HashMap::new(),
        }
    }

    /// (Re-)arm the debounce deadline for every path the event touches. Each
    /// event within a burst pushes the deadline to `now + window`, so a path
    /// only settles after a quiet `window` — coalescing the whole burst.
    pub fn on_event(&mut self, event: &RawEvent, now: Instant) {
        let deadline = now + self.window;
        for path in &event.paths {
            self.pending.insert(path.clone(), deadline);
        }
    }

    /// Emit one [`FileChanged`] for every path whose window has elapsed at
    /// `now`, removing it from the pending set. Output is sorted by path for
    /// deterministic ordering despite `HashMap` storage.
    pub fn flush_ready(&mut self, now: Instant) -> Vec<FileChanged> {
        let mut ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, &deadline)| deadline <= now)
            .map(|(path, _)| path.clone())
            .collect();
        ready.sort();
        for path in &ready {
            self.pending.remove(path);
        }
        ready.into_iter().map(|path| FileChanged { path }).collect()
    }

    /// The earliest pending deadline, if any — used to size the facade's next
    /// blocking read so it wakes exactly when a path is due to flush.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.pending.values().copied().min()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Result of one [`WatcherFacade::pump_once`] iteration.
#[derive(Debug, PartialEq, Eq)]
pub enum PumpStatus {
    /// The facade is live; keep pumping.
    Continue,
    /// The event source closed or the sink receiver was dropped; stop.
    Disconnected,
}

/// Ties an [`EventSource`], a [`Clock`], a [`Debouncer`], and an output sink
/// together. Stepping is factored into [`pump_once`](Self::pump_once) so
/// production loops it via [`run`](Self::run) while tests drive it one pump at a
/// time against a `FakeClock`.
pub struct WatcherFacade<S: EventSource, C: Clock> {
    source: S,
    clock: C,
    debouncer: Debouncer,
    sink: Sender<FileChanged>,
}

impl<S: EventSource, C: Clock> WatcherFacade<S, C> {
    pub fn new(source: S, clock: C, window: Duration, sink: Sender<FileChanged>) -> Self {
        WatcherFacade {
            source,
            clock,
            debouncer: Debouncer::new(window),
            sink,
        }
    }

    /// Run one step: size the blocking read from the nearest pending deadline,
    /// fold in any event that arrives, then flush every path whose window has
    /// closed to the sink. All timing reads the injected [`Clock`], so a
    /// `FakeClock` fully determines when paths flush.
    pub fn pump_once(&mut self) -> PumpStatus {
        let now = self.clock.now();
        let timeout = match self.debouncer.next_deadline() {
            Some(deadline) => deadline.saturating_duration_since(now).min(IDLE_POLL),
            None => IDLE_POLL,
        };

        match self.source.recv_timeout(timeout) {
            RecvOutcome::Event(event) => self.debouncer.on_event(&event, self.clock.now()),
            RecvOutcome::Timeout => {}
            RecvOutcome::Disconnected => return PumpStatus::Disconnected,
        }

        for change in self.debouncer.flush_ready(self.clock.now()) {
            if self.sink.send(change).is_err() {
                return PumpStatus::Disconnected;
            }
        }
        PumpStatus::Continue
    }

    /// Pump until the source closes or the sink is dropped.
    pub fn run(mut self) {
        while self.pump_once() == PumpStatus::Continue {}
    }
}

/// Errors from establishing a [`NotifyEventSource`] watch.
#[derive(Debug, Error)]
pub enum WatchError {
    #[error(transparent)]
    Notify(#[from] notify::Error),
}

/// Production [`EventSource`]: a `notify` `RecommendedWatcher` whose callback
/// feeds normalized [`RawEvent`]s into an `mpsc` channel this source drains.
pub struct NotifyEventSource {
    // Held to keep the OS watch alive; dropping it stops the watch.
    _watcher: RecommendedWatcher,
    rx: Receiver<RawEvent>,
}

impl NotifyEventSource {
    /// Recursively watch `root` for filesystem events.
    pub fn watch(root: &Path) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                // A closed receiver just means the facade stopped; drop silently.
                let _ = tx.send(RawEvent::from(event));
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(NotifyEventSource {
            _watcher: watcher,
            rx,
        })
    }
}

impl EventSource for NotifyEventSource {
    fn recv_timeout(&mut self, timeout: Duration) -> RecvOutcome {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => RecvOutcome::Event(event),
            Err(RecvTimeoutError::Timeout) => RecvOutcome::Timeout,
            Err(RecvTimeoutError::Disconnected) => RecvOutcome::Disconnected,
        }
    }
}

/// Production [`Clock`]: reads the real monotonic clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};

    // A `FakeClock`-backed clock: reads whatever instant the test has advanced
    // the shared `FakeClock` to. Bridges core's test-support `Clock` into this
    // crate's local `Clock` facade (same pattern as `tests/anchor.rs`).
    #[derive(Clone)]
    struct TestClock(orgsidian_core::test_support::clock::FakeClock);

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            use orgsidian_core::test_support::clock::Clock as _;
            self.0.now()
        }
    }

    // Scriptable `EventSource`: yields queued events in order, then reports
    // `Timeout` (queue momentarily empty) or, once closed, `Disconnected`.
    // Ignores the real `timeout` so no wall-clock time passes. Its queue is
    // shared behind an `Arc` so a test can enqueue a *second* burst — through
    // the seam — after the facade has already been handed the source (a real
    // "burst / quiet gap / burst" script without reaching into facade internals).
    #[derive(Clone)]
    struct FakeSource {
        queue: Arc<Mutex<VecDeque<RawEvent>>>,
        closed: Arc<AtomicBool>,
    }

    impl FakeSource {
        fn new(events: Vec<RawEvent>) -> Self {
            FakeSource {
                queue: Arc::new(Mutex::new(events.into_iter().collect())),
                closed: Arc::new(AtomicBool::new(false)),
            }
        }

        // A control handle over the same shared queue, taken before the source
        // is moved into the facade.
        fn handle(&self) -> FakeSource {
            self.clone()
        }

        fn push(&self, events: Vec<RawEvent>) {
            self.queue.lock().unwrap().extend(events);
        }

        fn close(&self) {
            self.closed.store(true, Ordering::SeqCst);
        }
    }

    impl EventSource for FakeSource {
        fn recv_timeout(&mut self, _timeout: Duration) -> RecvOutcome {
            match self.queue.lock().unwrap().pop_front() {
                Some(event) => RecvOutcome::Event(event),
                None if self.closed.load(Ordering::SeqCst) => RecvOutcome::Disconnected,
                None => RecvOutcome::Timeout,
            }
        }
    }

    fn ev(kind: RawKind, path: &str) -> RawEvent {
        RawEvent {
            paths: vec![PathBuf::from(path)],
            kind,
        }
    }

    // ---- Debouncer (pure, Instant-driven) ------------------------------

    #[test]
    fn atomic_save_burst_coalesces_to_one_event() {
        let mut d = Debouncer::new(DEBOUNCE_WINDOW);
        let t0 = Instant::now();
        // remove+create+modify — the atomic-save signature — all within window.
        d.on_event(&ev(RawKind::Remove, "/vault/a.org"), t0);
        d.on_event(&ev(RawKind::Create, "/vault/a.org"), t0);
        d.on_event(&ev(RawKind::Modify, "/vault/a.org"), t0);

        // Window not yet elapsed: nothing flushes.
        assert!(d.flush_ready(t0 + Duration::from_millis(249)).is_empty());

        // Window elapsed quietly: exactly one FileChanged.
        let out = d.flush_ready(t0 + DEBOUNCE_WINDOW);
        assert_eq!(
            out,
            vec![FileChanged {
                path: PathBuf::from("/vault/a.org")
            }]
        );
        assert!(d.is_empty());
    }

    #[test]
    fn distinct_paths_debounce_independently() {
        let mut d = Debouncer::new(DEBOUNCE_WINDOW);
        let t0 = Instant::now();
        d.on_event(&ev(RawKind::Modify, "/vault/a.org"), t0);
        d.on_event(
            &ev(RawKind::Modify, "/vault/b.org"),
            t0 + Duration::from_millis(100),
        );

        // At t0+250 only a.org is due; b.org's window closes 100ms later.
        let first = d.flush_ready(t0 + DEBOUNCE_WINDOW);
        assert_eq!(
            first,
            vec![FileChanged {
                path: PathBuf::from("/vault/a.org")
            }]
        );

        let second = d.flush_ready(t0 + Duration::from_millis(350));
        assert_eq!(
            second,
            vec![FileChanged {
                path: PathBuf::from("/vault/b.org")
            }]
        );
    }

    #[test]
    fn sustained_rearming_defers_emission_until_quiet_gap() {
        let mut d = Debouncer::new(DEBOUNCE_WINDOW);
        let t0 = Instant::now();
        // Events every 100ms keep pushing the deadline (debounce, not throttle).
        for i in 0..5 {
            d.on_event(
                &ev(RawKind::Modify, "/vault/a.org"),
                t0 + Duration::from_millis(i * 100),
            );
            assert!(
                d.flush_ready(t0 + Duration::from_millis(i * 100 + 50))
                    .is_empty(),
                "must not emit while events keep arriving < window apart"
            );
        }
        // Last event was at t0+400 → deadline t0+650.
        assert!(d.flush_ready(t0 + Duration::from_millis(600)).is_empty());
        assert_eq!(
            d.flush_ready(t0 + Duration::from_millis(650)),
            vec![FileChanged {
                path: PathBuf::from("/vault/a.org")
            }]
        );
    }

    #[test]
    fn flush_output_is_sorted_by_path() {
        let mut d = Debouncer::new(DEBOUNCE_WINDOW);
        let t0 = Instant::now();
        for name in ["/z.org", "/a.org", "/m.org"] {
            d.on_event(&ev(RawKind::Modify, name), t0);
        }
        let out: Vec<PathBuf> = d
            .flush_ready(t0 + DEBOUNCE_WINDOW)
            .into_iter()
            .map(|c| c.path)
            .collect();
        assert_eq!(
            out,
            vec![
                PathBuf::from("/a.org"),
                PathBuf::from("/m.org"),
                PathBuf::from("/z.org")
            ]
        );
    }

    // ---- WatcherFacade (fake source + FakeClock, no real sleeps) --------

    #[test]
    fn facade_coalesces_burst_deterministically_via_fake_clock() {
        let fake = orgsidian_core::test_support::clock::FakeClock::new();
        let clock = TestClock(fake.clone());
        let source = FakeSource::new(vec![
            ev(RawKind::Remove, "/vault/a.org"),
            ev(RawKind::Create, "/vault/a.org"),
            ev(RawKind::Modify, "/vault/a.org"),
        ]);
        let (tx, rx) = channel();
        let mut facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);

        // Drain the burst while the clock is frozen: nothing settles yet.
        for _ in 0..3 {
            assert_eq!(facade.pump_once(), PumpStatus::Continue);
        }
        assert!(
            rx.try_recv().is_err(),
            "no emission before the window elapses"
        );

        // Advance past the window and pump once more: exactly one FileChanged.
        fake.advance(DEBOUNCE_WINDOW);
        assert_eq!(facade.pump_once(), PumpStatus::Continue);
        assert_eq!(
            rx.try_recv(),
            Ok(FileChanged {
                path: PathBuf::from("/vault/a.org")
            })
        );
        assert!(rx.try_recv().is_err(), "burst coalesced to a single event");
    }

    #[test]
    fn facade_emits_two_events_for_two_saves_separated_by_a_quiet_gap() {
        let fake = orgsidian_core::test_support::clock::FakeClock::new();
        let clock = TestClock(fake.clone());
        let (tx, rx) = channel();
        // Both bursts flow through the `EventSource` seam: the source starts
        // empty, the handle enqueues each burst, and the quiet gap is the
        // `FakeClock` advance between them.
        let source = FakeSource::new(vec![]);
        let handle = source.handle();
        let mut facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);

        // First save burst on the path.
        handle.push(vec![
            ev(RawKind::Modify, "/vault/a.org"),
            ev(RawKind::Modify, "/vault/a.org"),
        ]);
        assert_eq!(facade.pump_once(), PumpStatus::Continue); // first event
        assert_eq!(facade.pump_once(), PumpStatus::Continue); // second event (re-arm)
        fake.advance(DEBOUNCE_WINDOW);
        assert_eq!(facade.pump_once(), PumpStatus::Continue); // quiet → flush #1
        assert_eq!(
            rx.try_recv(),
            Ok(FileChanged {
                path: PathBuf::from("/vault/a.org")
            })
        );

        // A later, separate save on the same path — arriving through the seam
        // after the quiet gap — is a distinct FileChanged.
        handle.push(vec![
            ev(RawKind::Modify, "/vault/a.org"),
            ev(RawKind::Modify, "/vault/a.org"),
        ]);
        assert_eq!(facade.pump_once(), PumpStatus::Continue);
        assert_eq!(facade.pump_once(), PumpStatus::Continue);
        fake.advance(DEBOUNCE_WINDOW);
        assert_eq!(facade.pump_once(), PumpStatus::Continue); // flush #2
        assert_eq!(
            rx.try_recv(),
            Ok(FileChanged {
                path: PathBuf::from("/vault/a.org")
            })
        );
    }

    #[test]
    fn facade_reports_disconnected_when_source_closes() {
        let fake = orgsidian_core::test_support::clock::FakeClock::new();
        let clock = TestClock(fake);
        let source = FakeSource::new(vec![]);
        source.close();
        let (tx, _rx) = channel();
        let mut facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);
        assert_eq!(facade.pump_once(), PumpStatus::Disconnected);
    }

    #[test]
    fn facade_run_terminates_when_source_closes() {
        let fake = orgsidian_core::test_support::clock::FakeClock::new();
        let clock = TestClock(fake);
        let source = FakeSource::new(vec![]);
        source.close();
        let (tx, _rx) = channel();
        let facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);
        // Returns cleanly (does not hang) once the source reports Disconnected.
        facade.run();
    }

    #[test]
    fn facade_reports_disconnected_when_sink_dropped() {
        let fake = orgsidian_core::test_support::clock::FakeClock::new();
        let clock = TestClock(fake.clone());
        let source = FakeSource::new(vec![ev(RawKind::Modify, "/vault/a.org")]);
        let (tx, rx) = channel();
        let mut facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);
        drop(rx); // sink receiver gone
        assert_eq!(facade.pump_once(), PumpStatus::Continue); // event armed, nothing to send yet
        fake.advance(DEBOUNCE_WINDOW);
        assert_eq!(facade.pump_once(), PumpStatus::Disconnected); // flush fails to send
    }
}
