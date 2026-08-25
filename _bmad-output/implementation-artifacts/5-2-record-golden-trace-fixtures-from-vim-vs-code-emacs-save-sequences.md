---
title: 'Record golden-trace fixtures from vim / VS Code / Emacs save sequences'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '45cb7baf6f36301f4742e8aeea129b4b14e7cb52'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-5-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 5.1's [`Debouncer`](../../crates/orgsidian-watcher/src/watcher.rs) coalesces a raw-event burst *per path* into one `FileChanged`, but deliberately deferred the "exactly ONE `FileChanged` per editor save" calibration (see 5.1's `tests/notify_integration.rs` scope note). A real save is multi-*path*, not just multi-event: vim renames the target to a `~` backup, drops a `4913` writability probe and a `.swp` swap file; VS Code writes a temp file and atomically renames it onto the target; Emacs leaves a `~` backup, a `#…#` autosave, and a `.#…` lock file. Left unfiltered, each artifact path debounces into its own spurious `FileChanged`, tripping the Single-Writer / merge state machines Epic 5 protects. OD-3 debounce calibration must be *data-driven* (golden traces), and Epic 9's Merge Dialog tests must be able to replay the same traces.

**Approach:** (1) Add a pure, path-only calibration seam `orgsidian-watcher/src/calibration.rs` — `is_editor_artifact` / `is_save_target` / `save_targets` — that keeps only genuine `.org` save targets and filters swap/backup/autosave/lock/temp/probe artifacts (rules derived from documented vim/VS Code/Emacs save mechanics). (2) Wire it into `WatcherFacade::pump_once` so one logical save arms the debouncer for exactly one target path, which the existing per-path debounce then coalesces to one `FileChanged`. (3) Record three hand-authored golden traces `tests/golden_traces/{vim,vscode,emacs}.json` — timestamped raw events (`offset_ms`, `kind`, `paths`). (4) Add `tests/debounce.rs`, which replays each trace through the real `WatcherFacade` over the public `EventSource`/`Clock` seams (a `FakeClock` advancing per each event's offset, **no real sleeps**) and asserts exactly one `FileChanged` for the save target per logical save. (5) Declare fixture ownership in `fixtures/fixtures.toml` as `owner = "epic-5"` (Murat P1).

## Boundaries & Constraints

**Always:**
- Calibration is pure and path-only (no I/O), so the golden-trace replay is fully `FakeClock`-deterministic (no real sleeps), matching Story 5.1's discipline.
- One logical editor save → exactly one `FileChanged { path }` for the target; artifact-only events (swap flush, lock release, temp churn) arm nothing.
- Golden traces are hand-authored representative traces (offline env — no live capture) modeled on documented editor save mechanics, and are labeled as such in every fixture's `description`/`provenance`.
- Reuse Story 5.1's `EventSource` / `Clock` / `WatcherFacade` seams and the `tests/anchor.rs` `FakeClock` adapter pattern; keep all Story 5.1 tests green (no regression).
- Match surrounding module-doc / comment density and naming.

**Ask First:**
- Any change to another workspace crate. This story is scoped to `orgsidian-watcher`, `fixtures/fixtures.toml`, and the story file.
- Adding any external dependency (offline). `serde`/`serde_json` are **test-only** dev-deps already in the warmed lockfile; the production watcher stays serde-free.

**Never:**
- No `ConflictState` / dirty-buffer / reload / merge logic (Stories 5.3–5.5); no Tauri events / IPC; no writer-ID suppression tokens.
- No real `thread::sleep` in the replay — coalescing is `FakeClock`-deterministic.
- Do not change the `WatcherFacade::new` signature (Story 5.1's tests + integration test construct it with four args).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| vim save | swap+`4913`+`~` backup+rename+in-place modify on `notes.org` | exactly one `FileChanged { notes.org }` | N/A |
| VS Code save | temp create/modify then atomic rename `[temp, notes.org]` | exactly one `FileChanged { notes.org }`; temp filtered | N/A |
| Emacs save | `#…#` autosave + `~` backup + rename + in-place modify + `.#…` lock release | exactly one `FileChanged { notes.org }`; lock (ends in `.org`!) filtered | N/A |
| Emacs lock file | `.#notes.org` (extension is `org`) | classified artifact by `.#` prefix, never a target | N/A |
| Atomic rename event | one event lists `[temp, target]` | keeps only `target` | N/A |
| Artifact-only event | swap flush / lock release only | arms nothing → no `FileChanged` | N/A |
| Non-`.org` sibling / parent dir | `.txt`, `.png`, `/vault` (macOS FSEvents parent) | not a target → ignored | N/A |
| Two saves, one path | trace, quiet gap > 250ms, trace again | two `FileChanged` for the target (per-save, not one-forever) | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-watcher/src/calibration.rs` -- NEW. `//!`-doc OD-3 calibration seam. `pub fn is_editor_artifact(&Path) -> bool` (swap/backup/autosave/lock/temp/probe rules), `pub fn is_save_target(&Path) -> bool` (`.org` ∧ ¬artifact), `pub fn save_targets(&[PathBuf]) -> Vec<PathBuf>`. Inline `#[cfg(test)]` unit tests per artifact family incl. the Emacs-lock-ends-in-`.org` tripwire.
- `crates/orgsidian-watcher/src/watcher.rs` -- MODIFY. `use crate::calibration;`; `pump_once` filters each `RawEvent` through `calibration::save_targets` before arming the debouncer (artifact-only events arm nothing). No signature/API change; Story 5.1 unit tests (all `.org` paths) unaffected.
- `crates/orgsidian-watcher/src/lib.rs` -- MODIFY. `pub mod calibration;` + re-export `is_editor_artifact`/`is_save_target`/`save_targets`; module-doc paragraph for the Story 5.2 seam.
- `crates/orgsidian-watcher/Cargo.toml` -- MODIFY. Add `serde`/`serde_json` **dev-deps** (workspace-pinned, lockfile-present) for the trace parser; production deps unchanged.
- `crates/orgsidian-watcher/tests/golden_traces/{vim,vscode,emacs}.json` -- NEW. Hand-authored save traces: `{editor, target, expected_file_changes, provenance, description, events:[{offset_ms, kind, paths}]}`.
- `crates/orgsidian-watcher/tests/debounce.rs` -- NEW. Serde trace schema + `FakeSource`/`ClockAdapter` over the public seams; `replay()` advances a `FakeClock` per offset and pumps; per-editor tests assert exactly one target `FileChanged`; a two-save test asserts per-save semantics.
- `fixtures/fixtures.toml` -- MODIFY. `[traces.editor-saves]` with `owner = "epic-5"`, `ld_reference = "LD-9"`, hand-authored + Epic-9-reuse note (Murat P1).

## Tasks & Acceptance

**Execution:**
- [x] `src/calibration.rs` -- pure artifact classifier + `save_targets`; unit tests per artifact family.
- [x] `src/watcher.rs` / `src/lib.rs` -- wire calibration into `pump_once`; module decl + re-exports + docstrings.
- [x] `Cargo.toml` -- test-only `serde`/`serde_json` dev-deps.
- [x] `tests/golden_traces/{vim,vscode,emacs}.json` -- record the three traces.
- [x] `tests/debounce.rs` -- `FakeClock`/seam replay harness; one-`FileChanged`-per-save assertions + two-save semantics.
- [x] `fixtures/fixtures.toml` -- `owner = "epic-5"` ownership entry.

**Acceptance Criteria:**
- Given Story 5.1, when the fixtures are recorded, then `crates/orgsidian-watcher/tests/golden_traces/{vim,vscode,emacs}.json` contain timestamped event sequences. ✅ (three JSON traces; `offset_ms`/`kind`/`paths`).
- And `tests/debounce.rs` replays each trace and asserts the watcher emits exactly one `FileChanged` event per save. ✅ (`vim/vscode/emacs_save_emits_exactly_one_file_changed` + `two_consecutive_saves_emit_two_file_changed`, all `FakeClock`-deterministic).
- And `fixtures/fixtures.toml` declares ownership of these traces as `owner = "epic-5"` per Murat P1. ✅ (`[traces.editor-saves]`).

### Review Findings

Adversarial code review (blind-spot, edge-case, verification-gap, acceptance-auditor layers). Actionable findings fixed; rest triaged with rationale.

- [x] [Review][Patch] Unused `FileChanged` import in `tests/debounce.rs` (clippy `-D warnings`) — removed.
- [x] [Review][Patch] `rustfmt` diffs in `debounce.rs` (import wrap + `assert_eq!` layout) — `cargo fmt` applied.

## Design Notes

- **Why a path filter (not deeper coalescing):** in every editor's save, the real `.org` target *itself* receives at least one raw event (in-place modify, or remove-then-create via the backup rename). Every *other* touched path is a transient artifact with a distinguishing name. So keeping only genuine `.org` targets, then letting Story 5.1's per-path debouncer coalesce the target's own sub-burst, yields exactly one `FileChanged` per save without any new timing logic. The filter also resolves the macOS-FSEvents parent-directory report noted in 5.1's integration test (a directory is not a `.org` target).
- **The Emacs lock-file tripwire:** `.#notes.org` ends in `.org`, so an extension-only shortcut would wrongly keep it. `is_editor_artifact` matches the `.#` prefix *before* the extension check; the `emacs.json` trace and a dedicated unit test pin this.
- **Deterministic replay:** `replay()` mirrors Story 5.1's facade unit tests — preload the `FakeSource`, advance the `FakeClock` to each event's `offset_ms`, `pump_once`, drain the sink; a final `DEBOUNCE_WINDOW` advance flushes the tail. Zero wall-clock waits.
- **Hand-authored, not captured:** the offline env cannot spawn real editors, so traces are representative sequences modeled on documented save mechanics (`:h backup`/`:h swap-file`; VS Code atomic write-then-rename; Emacs backup/autosave/lock). Every fixture carries `provenance: "hand-authored"` and a mechanics description; `fixtures.toml` records the same. Epic 9 reuses them ~85% unchanged (only the outcome assertion flips).

## Verification

**Commands:**
- `cargo test -p orgsidian-watcher --offline` -- expected: 16 unit (7 calibration + 9 Story 5.1) + 1 anchor + 4 debounce + 2 notify_integration, all pass; Story 5.1 tests unregressed.
- `cargo clippy -p orgsidian-watcher --offline --all-targets -- -D warnings` -- expected: clean (the C `-Wsign-compare` lines are the vendored tree-sitter grammar).
- `cargo fmt -p orgsidian-watcher -- --check` -- expected: clean.

Note: this worktree is stacked on Story 5.1 (branch `feat/5-2-golden-trace-fixtures`, base `45cb7ba` / PR #175).
