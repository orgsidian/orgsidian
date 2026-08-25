---
title: 'Implement notify-rs filesystem watcher with debounce'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '0838dcde186787507884f0f0cee80bd623c8b222'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-5-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The `orgsidian-watcher` crate ships only the Story 1.9 anchor-smoke surface (`detect_first_write_event`, an mtime poll loop). Epic 5 (and Epic 8 Capture) need a real filesystem watcher that wraps `notify-rs`, coalesces the 3–12 raw events an atomic save emits (vim swap+rename, VS Code temp+rename, Emacs backup+save — delete+create+modify bursts) into a single logical change, and detects external writes within 5s on all three platforms — so downstream consumers never spuriously trip the Single Writer / merge state machines (LD-9).

**Approach:** Add `src/watcher.rs` built around three trait-based seams so the debounce logic is unit-testable without real filesystems or wall-clock sleeps: (1) a `Debouncer` — a pure per-path coalescing engine driven only by injected `Instant`s (each raw event re-arms a `now + 250ms` deadline; a path flushes to one `FileChanged { path }` once its window elapses quietly); (2) an `EventSource` trait abstracting the raw-event stream, with a production `NotifyEventSource` wrapping `RecommendedWatcher` and a scriptable fake for tests; (3) the existing `Clock` seam (production `SystemClock`, `FakeClock` in tests). `WatcherFacade` ties source + clock + debouncer + an output `Sender<FileChanged>` together via a single-step `pump_once()` (production `run()` loops it), so tests advance a `FakeClock` between pumps and assert emissions deterministically. `detect_first_write_event`'s body is swapped onto `NotifyEventSource` wakeups (mtime stays the authoritative confirmation) to honor the Story 1.9 sentinel contract.

## Boundaries & Constraints

**Always:**
- The debounce window is 250ms (`DEBOUNCE_WINDOW` constant, per LD-7/OD-3); a burst of raw events on one path coalesces to exactly one `FileChanged { path }`; distinct paths debounce independently.
- Every timing decision (debounce deadlines, facade timeout budget) reads the injected `Clock` — never `Instant::now()` directly inside the debouncer or facade stepping logic — so `FakeClock` fully determines emission timing in tests.
- `FileChanged` emission order within a single flush is deterministic (sort by path) despite `HashMap` storage.
- Reuse the crate's existing `Clock` trait facade and `thiserror`-based error style; the anchor `tests/anchor.rs` must stay green (sentinel discipline).
- Match surrounding module-doc / comment density and naming.

**Ask First:**
- Any change to another workspace crate (e.g. promoting `Clock` into `orgsidian-core`, or touching `orgsidian-vault`). This story is scoped to `orgsidian-watcher` + `docs/architecture/resilience.md`.
- Adding any new external dependency (offline — none available beyond the warmed lockfile).

**Never:**
- No `ConflictState` / dirty-buffer / reload / merge logic (Stories 5.3–5.5), no Tauri event emission or IPC wiring, no golden-trace fixture recording (Story 5.2).
- No real `thread::sleep`-based waits in debounce assertions — coalescing tests are `FakeClock`-deterministic.
- No writer-ID suppression token logic (lands with the save-cycle wiring in later stories).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Atomic-save burst | remove+create+modify on one path, all within the 250ms window | exactly one `FileChanged { path }` after the window elapses quietly | N/A |
| Independent paths | events on path A and path B interleaved | one `FileChanged` per path, each on its own window | N/A |
| Two saves, same path | burst, 250ms quiet, second burst | two `FileChanged` emissions (window reset by the quiet gap) | N/A |
| Sustained re-arming | events keep arriving < 250ms apart | no emission until a quiet 250ms gap (debounce, not throttle) | N/A |
| Real external write | fs write under a watched dir via `NotifyEventSource` + `WatcherFacade` | a `FileChanged { path }` reaches the sink within 5s | detection asserted, not event count (real timing) |
| Source disconnected | notify channel closed / sink receiver dropped | `pump_once` reports `Disconnected`; `run` returns cleanly | no panic |
| Watch setup failure | `notify::watch` errors (missing path) | `NotifyEventSource::watch` returns `Err(WatchError)` | propagated, typed |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-watcher/src/watcher.rs` -- NEW. `//!`-doc for FR-16 detection layer (LD-9). Types: `FileChanged { path }`, `RawEvent { paths, kind }` + `RawKind` (Create/Modify/Remove/Other, normalized from `notify::EventKind`), `EventSource` trait + `RecvOutcome { Event | Timeout | Disconnected }`, `Debouncer` (`new`/`on_event`/`flush_ready`/`next_deadline`/`is_empty`), `WatcherFacade<S,C>` (`new`/`pump_once`→`PumpStatus`/`run`), `NotifyEventSource` (wraps `RecommendedWatcher` + `mpsc::Receiver<RawEvent>`), `SystemClock`, `WatchError` (thiserror, `#[from] notify::Error`), `DEBOUNCE_WINDOW = 250ms`. Inline `#[cfg(test)]` unit tests for `Debouncer` + `WatcherFacade` using a scripted fake `EventSource` + `FakeClock` adapter.
- `crates/orgsidian-watcher/src/lib.rs` -- MODIFY. `pub mod watcher;` + re-export the public surface; swap `detect_first_write_event`'s body onto `NotifyEventSource` wakeups (mtime remains authoritative), keeping its signature; extend `DetectError` with a `Watch(#[from] WatchError)` variant; refresh the now-stale Story-5.1 docstrings.
- `crates/orgsidian-watcher/tests/anchor.rs` -- UNCHANGED (green sentinel; adapter pattern reused by new tests).
- `crates/orgsidian-watcher/tests/notify_integration.rs` -- NEW. Real `NotifyEventSource` + `WatcherFacade` over a `tempfile::TempDir`: asserts a `FileChanged { path }` arrives within 5s (LD-9 NFR evidence, all platforms).
- `crates/orgsidian-core/src/test_support/clock.rs` -- READ-ONLY reference. `FakeClock` (dev-dep via `test-support`) is `now()/advance()`; new tests bridge it to the crate's local `Clock` via the `tests/anchor.rs` adapter pattern.
- `docs/architecture/resilience.md` -- NEW. LD-9 unsupported-configuration note: network mounts and case-folding filesystems are v0.1-unsupported for the watcher.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-watcher/src/watcher.rs` -- implement `Debouncer` (pure 250ms coalescer), `EventSource`/`RecvOutcome` seam, `WatcherFacade` (clock-driven `pump_once`/`run`), `NotifyEventSource` + `SystemClock` + `WatchError`; add `#[cfg(test)]` `Debouncer`/facade unit tests covering every I/O Matrix debounce row via `FakeClock` (no real sleeps).
- [x] `crates/orgsidian-watcher/src/lib.rs` -- wire `pub mod watcher`, re-exports, `DetectError::Watch`, `detect_first_write_event` swap onto notify wakeups, docstring refresh.
- [x] `crates/orgsidian-watcher/tests/notify_integration.rs` -- real-fs end-to-end detection-within-5s test.
- [x] `docs/architecture/resilience.md` -- document network-mount + case-folding filesystems as v0.1 unsupported.

**Acceptance Criteria:**
- Given Epic 3 closed, when `crates/orgsidian-watcher/src/watcher.rs` wraps `notify-rs`, then the watcher emits a single `FileChanged { path }` after a 250ms debounce window coalesces atomic-save delete+create+modify sequences — verified by the `Debouncer`/facade `FakeClock` unit tests.
- And the `WatcherFacade` abstraction allows deterministic fakes for unit tests (fake `EventSource` + `FakeClock`) — verified by those same tests running with no real sleeps.
- And external writes are detected within 5s on macOS/Linux/Windows (LD-9 NFR) — verified by `tests/notify_integration.rs` (real `notify-rs`), platform-agnostic.
- And network mounts and case-folding filesystems are documented as v0.1 unsupported in `docs/architecture/resilience.md`.

### Review Findings

Four parallel review layers (blind-hunter, edge-case-hunter, verification-gap, acceptance-auditor). Actionable findings were fixed; the rest triaged with rationale.

- [x] [Review][Patch] Bare-filename `parent()` returns `Some("")` — watch on empty path fails [src/lib.rs] — fixed: empty-parent guard falls back to `.`.
- [x] [Review][Patch] `metadata()?` aborts detection if the file is transiently absent mid atomic delete+create [src/lib.rs] — fixed: `NotFound` now continues waiting.
- [x] [Review][Patch] `resilience.md` self-contradiction — "verified on macOS/Windows" vs default APFS/NTFS listed as unsupported case-folding [docs/architecture/resilience.md] — fixed: separated the detection-latency axis (all platforms) from the path-identity axis (case-folding).
- [x] [Review][Patch] Real-boundary coalescing weakly verified — integration test broke on first receipt without counting [tests/notify_integration.rs] — fixed: burst + drain-to-quiescence, asserting per-path uniqueness (each path coalesces to one emission); documented the exactly-one-per-save boundary as Story 5.2's scope.
- [x] [Review][Patch] "Two saves, same path" test reached into a private field instead of the seam [src/watcher.rs] — fixed: `FakeSource` now shares its queue behind an `Arc` so both bursts arrive through `pump_once`.
- [x] [Review][Patch] `WatcherFacade::run()` clean-termination unverified [src/watcher.rs] — fixed: added `facade_run_terminates_when_source_closes`.
- [x] [Review][Patch] Public enums not future-proofed [src/watcher.rs, src/lib.rs] — fixed: `#[non_exhaustive]` on `RawKind` and `DetectError` (the widened error surface); documented.
- [x] [Review][Defer] inotify-overflow / rescan (empty-paths) events are silently dropped — resync belongs to the index-rebuild path (LD-13), out of scope for 5.1. Current behavior is a safe no-op (no spurious emission). Logged in `deferred-work.md`.
- [x] [Review][Dismiss] `FileChanged` carries only `path` (no mtime/kind) — the AC mandates exactly `FileChanged { path }`; consumer-specific fields are later-story scope.
- [x] [Review][Dismiss] No max-wait ceiling on the debounce — pure debounce is the AC-specified behavior; unbounded streams (cloud-sync) are a documented unsupported config. Tradeoff noted in the `Debouncer` doc.
- [x] [Review][Dismiss] Misc (idle 1s wakeup, ~50ms timeout overshoot, unconditional recursive watch, `flush_ready` clones, no explicit cancel signal) — cosmetic or acceptable for a 5s budget; drop-sink/close-source is the documented shutdown contract.

## Design Notes

- **Why a pure `Debouncer` + a `pump_once` seam:** a debounce is intrinsically timing-based, but the coalescing *decision* is pure given `now`. `Debouncer::on_event(ev, now)` sets `pending[path] = now + 250ms` (re-arm), `flush_ready(now)` drains every path with `deadline <= now`. `WatcherFacade::pump_once` reads `clock.now()`, sizes the source `recv_timeout` from `next_deadline`, feeds events, then flushes to the sink. Tests preload a fake `EventSource` with a burst, pump (nothing emitted while the `FakeClock` is frozen), `advance(250ms)`, pump once more → exactly one `FileChanged`. Fully deterministic, zero wall-clock waits.
- **`detect_first_write_event` swap:** now blocks on `NotifyEventSource::recv_timeout` for wakeups instead of a fixed 10ms mtime spin, but still confirms via `metadata().modified()` (authoritative, non-flaky) and still gates the timeout on the injected `Clock` — so the frozen-`FakeClock` anchor test detects the real write via the mtime backstop and never hangs.
- **Real-fs integration assertion:** OS backends report canonicalized paths (macOS FSEvents resolves `/var`→`/private/var` and may report the watched directory itself), so `tests/notify_integration.rs` asserts the detected change lies within the canonicalized watched tree, not an exact path equality. Exact per-path coalescing is pinned by the `FakeClock` unit tests.

## Verification

**Commands:**
- `cargo build -p orgsidian-watcher --offline` -- expected: success.
- `cargo clippy -p orgsidian-watcher --all-targets --offline -- -D warnings` -- expected: no warnings from `orgsidian-watcher` (the C `-Wsign-compare` lines are the vendored tree-sitter grammar).
- `cargo test -p orgsidian-watcher --offline` -- expected: 9 unit + 1 anchor (`tests/anchor.rs`) + 2 integration (`tests/notify_integration.rs`) all pass; the real-fs coalescing test re-run 10× with no flakiness.
- `cargo fmt -p orgsidian-watcher -- --check` -- expected: clean.

Note: this worktree required `git submodule update --init crates/orgsidian-parser/grammar` (offline, from the shared object store) so the `orgsidian-core` test-support dev-dep chain builds.
