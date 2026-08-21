---
title: 'Implement clean-buffer auto-reload + re-index on external write'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: 'b8c1399dfc9a4bcaec167fc974d246fdb5b8aeae'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-5-context.md']
github_issue: 50
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 5.1 (watcher/`WatcherFacade`/`FileChanged`) and 5.3 (`ConflictState`/`ResolveConflict` seam) each ship half of FR-16, but nothing yet CONSUMES a debounced `FileChanged`: when an external tool (VS Code, git) writes an open `.org` file whose in-memory buffer is CLEAN, Orgsidian must auto-reload the buffer from disk, incrementally re-sync that one file into the SQLite index, and preserve the cursor if its source line is unchanged (else reset to top) — silently, no dialog. Today the user must reload by hand and the index goes stale.

**Approach:** Wire the watcher output to the vault + index inside `orgsidian-core` — the ONLY crate the LEAF graph rule (`deny.toml`) lets wrap all three leaves (watcher, vault, index). Add a pure, I/O-free reload model to `orgsidian-vault` (`reload.rs`: cursor-preservation decision + a `BufferReload` plan, mirroring `conflict.rs`/`dirty_buffer.rs` conventions). Add a single-file incremental re-sync helper to the core index façade (`resync_file`, reusing the scan's read→parse→map→`upsert_file`/`quarantine`/`delete` path). Tie them together in a new `orgsidian-core::reconcile` dispatcher: `reconcile_external_write` checks `DirtyBufferManager::is_dirty` under one guard (fail-safe to DIRTY on lock poison) and, on the CLEAN branch, reads disk → re-syncs the index → returns a `CleanReload { new_content, cursor, notice }`. The DIRTY branch is a documented SEAM for Story 5.5 (block-save). The "file reloaded from disk" 3s non-modal notice text + duration are Rust constants carried on the outcome; the Tauri event emission + React `aria-live` notice + CodeMirror cursor application are the frontend seam (documented, not wired — see Design Notes).

## Boundaries & Constraints

**Always:**
- Reload happens ONLY when `DirtyBufferManager::is_dirty(path) == false`. Read the dirtiness under ONE read guard; a poisoned lock fails **safe to DIRTY** (never auto-reload over possibly-unsaved work) — the exact rule `dirty_buffer.rs` documents.
- Cursor rule: preserve `(line, column)` iff the line at the cursor's zero-based line index has identical text in the old buffer and the new disk content; otherwise reset to top `(0, 0)`. Column is clamped to the preserved line's char length.
- Incremental re-sync is per-file and transactional: reuse `orgsidian-index`'s single-file `upsert_file` (the "`sync::incremental`" the AC names) via the core index façade's read→parse→map path; a parse failure quarantines (LD-41), a vanished file `delete_file`s — never a full re-scan.
- New cross-crate edge `orgsidian-core → orgsidian-watcher` only (permitted: watcher's `wrappers = ["orgsidian-core"]`). Vault reload types stay pure (no I/O, no `Result`), with redacting `Debug` (buffer/disk content is user notes — never into a log/panic/`{:?}`).
- Match surrounding module-doc / comment density, LD/FR trace headers, and naming.

**Ask First:**
- Any change to a fourth crate beyond `orgsidian-vault`, `orgsidian-core`, and the workspace `Cargo.toml` (e.g. wiring the Tauri event/command surface in `orgsidian-shell-app`).
- Adding any new external dependency (offline — none available beyond the warmed lockfile).

**Never:**
- No dirty-buffer block-save, `ConflictDetected` event, or conflict banner (Story 5.5) — the DIRTY branch is a returned marker/SEAM only, it must not block or write.
- No three-pane Merge Dialog (Epic 9). No writer-ID suppress-token logic (save-cycle wiring, later).
- No real frontend build: do NOT hand-wire CodeMirror cursor application or the React notice component here (offline, untestable) — ship the Rust contract + record the exact hookup in the story + `deferred-work.md`.
- Do NOT touch `sprint-status.yaml`, `deny.toml`, `tests/anchor.rs`, or the inherited 5.1/5.3 commits.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Clean reload, line unchanged | `is_dirty==false`; cursor on a line whose text is identical old vs disk | buffer = disk content; `CursorOutcome::Preserved` at the same `(line, col)`; index re-synced (`Upserted`) | N/A |
| Clean reload, line changed/deleted | `is_dirty==false`; cursor line text differs or line index now out of range | buffer = disk content; `CursorOutcome::ResetToTop`; index re-synced | N/A |
| Clean reload, column past new line end | preserved line shorter than old column | `Preserved` with column clamped to new line char length | N/A |
| Dirty buffer | `is_dirty==true` (or lock poisoned) | `ExternalWriteOutcome::DirtyConflict { path }` — SEAM for 5.5; NOTHING written, NOT reloaded, index unchanged | fail-safe to DIRTY |
| External delete of a clean file | clean; file removed on disk | index `delete_file`d (`ResyncOutcome::Deleted`); buffer refreshed to empty | `NotFound` read → empty buffer, not an error |
| Unreadable/unparseable external write | clean; disk content cannot become a `Document` | index `quarantine`d for that file (`ResyncOutcome::Quarantined`, LD-41) | quarantine, not error |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-vault/src/reload.rs` -- NEW. Pure, I/O-free reload model (mirrors `conflict.rs`): `CursorPosition { line, column }` (zero-based, `Copy`), `CursorOutcome { Preserved(CursorPosition) | ResetToTop }` (`#[non_exhaustive]`, `resolved()` helper → `(0,0)` for reset), `decide_cursor(old, new, cursor) -> CursorOutcome` (same-index line-text equality + column clamp), `BufferReload { new_content, cursor }` + `plan(old, disk, cursor)`, redacting `Debug` on `BufferReload`. Colocated `#[cfg(test)]` tests.
- `crates/orgsidian-vault/src/lib.rs` -- MODIFY. `pub mod reload;` + `pub use reload::{...}`; present-tense Story 5.4 crate-doc sentence.
- `crates/orgsidian-core/src/index/resync.rs` -- NEW. `resync_file(index, abs_path) -> Result<ResyncOutcome, OrgError>`: `to_rel_path` + `file_stat`; if the file is gone → `delete_file`; else read+parse off `spawn_blocking` → `upsert_file` (map via `map::document_to_input`) or `quarantine_file` on parse failure. `ResyncOutcome { Upserted | Quarantined | Deleted }`. Submitted through `IndexHandle::writer().execute(...)`.
- `crates/orgsidian-core/src/index/scan.rs` -- MODIFY. Promote `read_and_parse`, `file_stat`, `ReadParse` to `pub(super)` so `resync.rs` reuses them (no duplication).
- `crates/orgsidian-core/src/index/mod.rs` -- MODIFY. `mod resync;` + `pub use resync::{resync_file, ResyncOutcome};` and re-expose the writer path to `resync` (already `pub(crate)`).
- `crates/orgsidian-core/src/reconcile.rs` -- NEW. `//! Implements FR-16 clean-buffer auto-reload dispatch (LD-7/LD-9)`. `BufferSnapshot { content, cursor }` (the frontend's current clean-buffer state), `CleanReload { new_content, cursor: CursorOutcome, resync: ResyncOutcome, notice, notice_duration }`, `ExternalWriteOutcome { CleanReload(..) | DirtyConflict { path } }`, `RELOAD_NOTICE = "file reloaded from disk"`, `RELOAD_NOTICE_DURATION = 3s`, and `async fn reconcile_external_write(index, buffers: &SharedDirtyBuffers, change: &FileChanged, snapshot) -> Result<ExternalWriteOutcome, OrgError>`.
- `crates/orgsidian-core/src/lib.rs` -- MODIFY. `pub mod reconcile;` + re-export the reconcile + reload surface for the shell/tests.
- `crates/orgsidian-core/Cargo.toml` + workspace `Cargo.toml` -- MODIFY. Add `orgsidian-watcher = { workspace = true }` (main dep on core) and a `[workspace.dependencies] orgsidian-watcher` entry (mirrors the vault/index entries). Add `tokio` `fs` feature to core for the async disk read.
- `crates/orgsidian-core/tests/external_write_clean.rs` -- NEW. Drives a real `WatcherFacade` (public `EventSource` fake + a local `orgsidian_watcher::Clock` adapter, no real sleeps) to emit a `FileChanged` after a synthetic external write, then `reconcile_external_write` on a temp vault + temp index DB; asserts buffer=disk, index re-synced (raw SQL via `orgsidian_index::open`), and cursor Preserved-vs-ResetToTop. Notes that the SHARED Story 5.2 golden traces stack onto the same `EventSource` seam when 5.2 merges.
- `crates/orgsidian-vault/src/dirty_buffer.rs` -- READ-ONLY. `is_dirty`/`get_buffer` contract + the one-guard + fail-safe-to-DIRTY rules.
- `crates/orgsidian-index/src/sync.rs` -- READ-ONLY. `upsert_file`/`delete_file`/`quarantine_file` (the single-file incremental path) + `SyncOp`.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-vault/src/reload.rs` + `src/lib.rs` -- pure `CursorPosition`/`CursorOutcome`/`decide_cursor`/`BufferReload`/`plan` with redacting `Debug`; colocated unit tests for every cursor row (unchanged / changed / deleted line / column clamp).
- [x] `crates/orgsidian-core/src/index/resync.rs` + `scan.rs` + `index/mod.rs` -- single-file incremental re-sync (`upsert`/`quarantine`/`delete`), reusing scan's read+parse+map helpers.
- [x] `crates/orgsidian-core/src/reconcile.rs` + `src/lib.rs` -- `reconcile_external_write` dispatcher (CLEAN branch wired, DIRTY branch = SEAM), notice constants, `BufferSnapshot`/`CleanReload`/`ExternalWriteOutcome`.
- [x] `crates/orgsidian-core/Cargo.toml` + workspace `Cargo.toml` -- add the `orgsidian-core → orgsidian-watcher` edge + `tokio` `fs`.
- [x] `crates/orgsidian-core/tests/external_write_clean.rs` -- watcher→reconcile end-to-end: buffer + index + cursor invariants (both cursor branches).

**Acceptance Criteria:**
- Given Stories 5.1 + 5.3, when the watcher detects an external write on a file with `DirtyBufferManager::is_dirty(path) == false`, then `reconcile_external_write` returns a `CleanReload` whose `new_content` equals the on-disk bytes — the in-memory buffer refreshed from disk.
- And the SQLite index is incrementally re-synced for that one file via `orgsidian-index`'s single-file `upsert_file` (the `sync::incremental` path) — asserted by querying the DB for the post-write headline.
- And the cursor is `Preserved` iff its source line is unchanged, else `ResetToTop` — asserted for both branches.
- And a non-modal status notice `"file reloaded from disk"` with a 3-second duration is carried on the `CleanReload` outcome (Rust contract; frontend `aria-live` render + CM6 cursor application recorded as the documented seam).
- And `tests/external_write_clean.rs` drives the watcher (via the `EventSource`/`Clock` seams, synthetic external-write event standing in for the not-yet-merged Story 5.2 golden traces) and asserts the buffer + index + cursor invariants.

## Design Notes

- **Why the orchestration lives in `orgsidian-core`, not the watcher.** `orgsidian-watcher`, `-vault`, `-index` are LEAF crates; `deny.toml`'s `wrappers = ["orgsidian-core"]` forbids a watcher→vault or watcher→index edge. Core is the sanctioned hub (it already wraps vault+parser+index for the scan). So "wire the watcher to the vault reload path" = add the core→watcher edge and dispatch there. This is the faithful realization of the epic's "watcher state machine consumes the reload path" given the dependency slice.
- **Cursor decision is pure + same-index line comparison.** `decide_cursor(old, new, cursor)` compares the line at `cursor.line` in both texts; equal → preserve (clamp column), else reset to top. A line inserted above shifts the same text to a new index and reads as "changed" → reset, which is the conservative, deterministic, testable reading of "source line unchanged / reset if the line was deleted". A fuzzy content-following cursor is out of scope. Uses `str::lines()` (no trailing-newline empty element) consistently for old and new.
- **`BufferSnapshot` is the old buffer.** A CLEAN file's buffer equals the last-saved disk content (pre-external-write), which the editor still holds; the frontend supplies it as `BufferSnapshot { content, cursor }`. The reconciler reads the NEW disk content itself and diffs the two. This keeps `decide_cursor` a pure, Rust-testable authority the frontend can call over IPC rather than re-implement.
- **Frontend seam (honest split).** Fully wired + tested in Rust: dirtiness routing, disk read, incremental re-index, cursor decision, notice payload. NOT wired (recorded in `deferred-work.md`): the `orgsidian-shell-app` Tauri event that pushes `CleanReload` to the window, the React `aria-live="polite"` 3s notice ("never `assertive`" — epic UX), and CodeMirror applying `new_content` + the cursor. The event-consumption LOOP (`Receiver<FileChanged>` drain calling `reconcile_external_write` per event with a per-file `BufferSnapshot` round-trip) is left to the shell wiring story alongside the 5.5 DIRTY branch.
- **Golden-trace forward-compat.** Story 5.2's shared JSON traces are absent from this branch (built in parallel). The test drives the SAME `orgsidian_watcher::EventSource` seam with a minimal synthetic external-write burst; when 5.2 merges, replaying its golden traces stacks by swapping the fake source's scripted events — no reconcile-side change.

## Verification

**Commands:**
- `cargo build -p orgsidian-vault -p orgsidian-core --offline` -- expected: clean.
- `cargo test -p orgsidian-vault -p orgsidian-watcher -p orgsidian-index -p orgsidian-core --offline` -- expected: all green, incl. the new `reload` unit tests + `tests/external_write_clean.rs`; no 5.1/5.3 regressions.
- `cargo clippy -p orgsidian-vault -p orgsidian-core --all-targets --offline -- -D warnings` -- expected: 0 warnings from touched crates.
- `cargo fmt -p orgsidian-vault -p orgsidian-core -- --check` -- expected: clean.

**Result (2026-08-21):** `cargo test -p orgsidian-vault -p orgsidian-watcher -p orgsidian-index -p orgsidian-core --offline` GREEN — 27 test binaries, 0 failed, incl. 8 new `reload` unit tests + 6 `tests/external_write_clean.rs` integration tests (clean/Preserved, clean/ResetToTop, dirty-SEAM, poisoned-lock fail-safe, external-delete, unreadable-quarantine). No 5.1/5.3 regressions. `cargo clippy … -D warnings` (vault + core, core with `test-support`) and `cargo fmt --check` clean. `Cargo.lock` delta is exactly one line (the `orgsidian-core → orgsidian-watcher` edge); zero new crate entries.

## Spec Change Log

- 2026-08-21 — Implemented. `orgsidian-vault/src/reload.rs` (pure cursor decision + `BufferReload`), `orgsidian-core/src/index/resync.rs` (single-file incremental re-sync), `orgsidian-core/src/reconcile.rs` (`reconcile_external_write` dispatcher), the `core → watcher` workspace edge, and `tests/external_write_clean.rs`. All ACs satisfied; all gates green offline. Status → in-review.
- 2026-08-21 — Code review (3 layers: Blind Hunter, Edge Case Hunter, Verification Gap). No intent_gap / bad_spec (no loopback). Patches applied + defers recorded (below). Re-ran all gates GREEN.

## Review Findings

Three-layer adversarial review (Blind Hunter, Edge Case Hunter, Verification Gap). No intent_gap or bad_spec findings — the design-tradeoff findings fall inside the spec's documented frontend / Story-5.5 seam. Patches applied; races/edges deferred with rationale.

**Applied (patch):**
- [x] [VerificationGap][Blind] The documented poison-lock → DIRTY fail-safe (`map_or(true, …)`) had no test. Added `poisoned_lock_fails_safe_to_conflict_and_leaves_index_untouched` — poisons the `SharedDirtyBuffers` lock, asserts `DirtyConflict` and an untouched index. **The strongest finding** (a safety-critical NFR-16 branch was uncovered).
- [x] [Blind] `CleanReload` carried no file path, though the frontend seam must route `new_content`+cursor to the right editor. Added `CleanReload::path()` (populated from `change.path`), symmetric with `DirtyConflict`; asserted in the main test.
- [x] [Blind] `BufferReload::into_new_content` (the zero-clone hand-off) was unreachable through `CleanReload`. Added `CleanReload::into_new_content(self)`.
- [x] [Blind] `notice`/`notice_duration` were dead per-instance fields always set to the two consts. Dropped them; `notice()`/`notice_duration()` now return `RELOAD_NOTICE`/`RELOAD_NOTICE_DURATION` directly.
- [x] [EdgeCase][Blind] `resync_file` TOCTOU: a file deleted between `exists()` and `file_stat` returned `Err(Io)` instead of `Deleted`. Now a `NotFound` from `file_stat` maps to the delete branch (`ResyncOutcome::Deleted`).
- [x] [Blind] Missing `#[must_use]` on the pure reload API. Added to `decide_cursor`, both `plan`s, and the value getters (`new_content`/`into_new_content`/`cursor`/`resolved`/`is_preserved`) — matching the Story 5.3 precedent.
- [x] [Blind] Test pinned the notice via a raw literal. Now asserts against the exported `RELOAD_NOTICE`/`RELOAD_NOTICE_DURATION` constants (tracks changes) plus one literal check to pin the wire value.

**Deferred** (recorded in `deferred-work.md` §code review of story-5-4): reconcile races (two-instant reads, mid-flight dirty turn, delete-between-resync-and-read — enforced at frontend apply time / the event loop); clean-buffer reconcile of an unreadable write returning `Err(Io)` after a successful quarantine (untested inconsistent edge, matrix-scoped to the re-sync boundary); cursor line-splitting edges (trailing empty line + CRLF-vs-LF via `str::lines()` — safe reset-to-top worst case).

**Dismissed as noise or by-design:** `libc` "missing" dev-dep (already declared under `[target.'cfg(unix)'.dev-dependencies]`; the test compiles + passes); watcher event kind "ignored" (`FileChanged` carries only `path` by Story 5.1 AC — no kind exists to consult); DIRTY branch "drops the snapshot 5.5 needs" (Story 5.5 sources the buffer from `DirtyBufferManager::get_buffer`, not the snapshot — per 5.3); i18n hook on `RELOAD_NOTICE` (frontend lingui concern); delete-fixture burst realism (kinds coalesce identically by 5.1 design, so the burst shape cannot affect the outcome).

## Suggested Review Order

**Reconciliation dispatch (entry point)**

- Start here: the CLEAN/DIRTY routing that ties watcher → vault → index.
  [`reconcile.rs:198`](../../crates/orgsidian-core/src/reconcile.rs#L198)
- The safety-critical dirtiness read — one guard, fail-safe to DIRTY on poison.
  [`reconcile.rs:210`](../../crates/orgsidian-core/src/reconcile.rs#L210)
- The CLEAN outcome (path + reload plan + resync) and the notice constants.
  [`reconcile.rs:110`](../../crates/orgsidian-core/src/reconcile.rs#L110)

**Incremental re-index (the `sync::incremental` path)**

- Single-file upsert/quarantine/delete, reusing the scan's read→parse→map.
  [`resync.rs:44`](../../crates/orgsidian-core/src/index/resync.rs#L44)

**Pure cursor decision (vault)**

- The "preserve iff source line unchanged, else reset to top" rule + column clamp.
  [`reload.rs:103`](../../crates/orgsidian-vault/src/reload.rs#L103)
- The reload plan carrying the new content + cursor outcome (redacting `Debug`).
  [`reload.rs:145`](../../crates/orgsidian-vault/src/reload.rs#L145)

**Tests + wiring (peripherals)**

- End-to-end watcher→reconcile invariants (buffer + index + cursor).
  [`external_write_clean.rs:178`](../../crates/orgsidian-core/tests/external_write_clean.rs#L178)
- The poison-lock fail-safe regression guard.
  [`external_write_clean.rs:328`](../../crates/orgsidian-core/tests/external_write_clean.rs#L328)
- The `orgsidian-core → orgsidian-watcher` LEAF edge.
  [`Cargo.toml`](../../crates/orgsidian-core/Cargo.toml)
