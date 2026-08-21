//! Story 5.2 — golden-trace debounce calibration.
//!
//! Replays the hand-authored editor save traces under `tests/golden_traces/`
//! (vim swap+rename, VS Code temp+rename, Emacs backup+autosave+save) through
//! the real `WatcherFacade` — over the public `EventSource`/`Clock` seams, with
//! a `FakeClock` advancing per each event's recorded offset and **no real
//! sleeps** — and asserts the watcher emits exactly one `FileChanged` for the
//! save target per logical save (OD-3). This is the data-driven calibration of
//! the artifact filter in `orgsidian_watcher::calibration`; Epic 9's Merge
//! Dialog tests replay the same traces (fixtures.toml `owner = "epic-5"`).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use orgsidian_core::test_support::clock::FakeClock;
use orgsidian_watcher::{
    Clock, EventSource, RawEvent, RawKind, RecvOutcome, WatcherFacade, DEBOUNCE_WINDOW,
};
use serde::Deserialize;

// ---- Golden-trace schema -------------------------------------------------

#[derive(Debug, Deserialize)]
struct Trace {
    editor: String,
    target: String,
    expected_file_changes: usize,
    events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Deserialize)]
struct TraceEvent {
    /// Milliseconds from the start of the save burst.
    offset_ms: u64,
    kind: String,
    paths: Vec<String>,
}

impl TraceEvent {
    fn raw(&self) -> RawEvent {
        RawEvent {
            paths: self.paths.iter().map(PathBuf::from).collect(),
            kind: match self.kind.as_str() {
                "create" => RawKind::Create,
                "modify" => RawKind::Modify,
                "remove" => RawKind::Remove,
                "other" => RawKind::Other,
                other => panic!("unknown raw event kind {other:?} in golden trace"),
            },
        }
    }
}

fn load_trace(editor: &str) -> Trace {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden_traces")
        .join(format!("{editor}.json"));
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden trace {path:?} must be readable: {e}"));
    let trace: Trace = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("golden trace {path:?} must parse: {e}"));
    assert_eq!(
        trace.editor, editor,
        "trace `editor` must match its file name"
    );
    trace
}

// ---- Test seams (public API only) ---------------------------------------

// Bridges core's `FakeClock` into the watcher's public `Clock` facade (the
// `tests/anchor.rs` adapter pattern).
struct ClockAdapter(FakeClock);

impl Clock for ClockAdapter {
    fn now(&self) -> std::time::Instant {
        use orgsidian_core::test_support::clock::Clock as _;
        self.0.now()
    }
}

// Scriptable `EventSource`: yields preloaded events in order, then `Timeout`.
// Ignores the real timeout so no wall-clock time passes (the `FakeClock` is the
// only clock the replay advances).
struct FakeSource(VecDeque<RawEvent>);

impl EventSource for FakeSource {
    fn recv_timeout(&mut self, _timeout: Duration) -> RecvOutcome {
        match self.0.pop_front() {
            Some(event) => RecvOutcome::Event(event),
            None => RecvOutcome::Timeout,
        }
    }
}

/// Drive `events` (offsets in non-decreasing ms order) through the facade,
/// advancing the `FakeClock` to each event's offset before pumping, then flush
/// the tail with one final `DEBOUNCE_WINDOW` advance. Returns every emitted
/// `FileChanged` path, in emission order.
fn replay(events: &[TraceEvent]) -> Vec<PathBuf> {
    let fake = FakeClock::new();
    let clock = ClockAdapter(fake.clone());
    let source = FakeSource(events.iter().map(TraceEvent::raw).collect());
    let (tx, rx) = channel();
    let mut facade = WatcherFacade::new(source, clock, DEBOUNCE_WINDOW, tx);

    let mut seen = Vec::new();
    let mut elapsed = Duration::ZERO;
    for event in events {
        let offset = Duration::from_millis(event.offset_ms);
        fake.advance(offset - elapsed);
        elapsed = offset;
        facade.pump_once();
        while let Ok(change) = rx.try_recv() {
            seen.push(change.path);
        }
    }
    // Quiet gap: the last-armed path's window closes and flushes.
    fake.advance(DEBOUNCE_WINDOW);
    facade.pump_once();
    while let Ok(change) = rx.try_recv() {
        seen.push(change.path);
    }
    seen
}

fn assert_one_change_per_save(editor: &str) {
    let trace = load_trace(editor);
    let seen = replay(&trace.events);
    let target = PathBuf::from(&trace.target);

    assert_eq!(
        seen.len(),
        trace.expected_file_changes,
        "{editor}: one logical save must yield exactly {} FileChanged, got {seen:?}",
        trace.expected_file_changes,
    );
    for path in &seen {
        assert_eq!(
            path, &target,
            "{editor}: the only emitted change must be the save target {target:?}, not the \
             swap/backup/autosave/lock/temp artifacts in the trace"
        );
    }
}

// ---- Per-editor golden-trace assertions ---------------------------------

#[test]
fn vim_save_emits_exactly_one_file_changed() {
    assert_one_change_per_save("vim");
}

#[test]
fn vscode_save_emits_exactly_one_file_changed() {
    assert_one_change_per_save("vscode");
}

#[test]
fn emacs_save_emits_exactly_one_file_changed() {
    assert_one_change_per_save("emacs");
}

// ---- "per save" semantics: two saves → two FileChanged ------------------

#[test]
fn two_consecutive_saves_emit_two_file_changed() {
    // Replaying a save, waiting out a quiet gap well past the debounce window,
    // then replaying it again must produce two distinct FileChanged — proving
    // the calibration coalesces *per save*, not "one forever".
    let trace = load_trace("vim");
    let target = PathBuf::from(&trace.target);

    let last = trace.events.last().expect("trace has events").offset_ms;
    let gap = DEBOUNCE_WINDOW.as_millis() as u64 + 100; // > window: distinct save
    let shift = last + gap;

    let mut doubled = trace.events.clone();
    for event in &trace.events {
        let mut shifted = event.clone();
        shifted.offset_ms += shift;
        doubled.push(shifted);
    }

    let seen = replay(&doubled);
    assert_eq!(
        seen,
        vec![target.clone(), target],
        "two saves separated by a quiet gap must emit two FileChanged for the target"
    );
}
