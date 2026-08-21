---
title: 'Implement Dirty-Buffer block-save fallback with conflict warning UI'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '9d5ee48'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-5-context.md']
github_issue: 51
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 5.3 (`ConflictState`/`ResolveConflict`/`BlockWithWarning` model) and 5.4 (`reconcile_external_write` with the CLEAN reload path + the `ExternalWriteOutcome::DirtyConflict { path }` SEAM) leave the DIRTY branch of FR-16 unfinished: when an external tool writes an open `.org` file whose in-memory buffer holds unsaved edits, Orgsidian must NOT silently overwrite. The v0.1 Alpha safe fallback (NFR-16 Single Writer Rule) is to BLOCK the save and surface a calm, non-modal conflict banner — the full three-pane Merge Dialog is Epic 9. Today nothing runs the `BlockWithWarning` strategy, no save gate exists, and there is no banner.

**Approach:** Complete the seam in three layers without touching the 5.4 reconcile signature (so its tests stay green). (1) `orgsidian-vault`: a `VaultError::ExternalConflict { path }` variant and a pure `PendingConflicts` presence-set registry (mirrors `DirtyBufferManager` — `Arc<RwLock<_>>`, fail-safe-to-BLOCKED on poison) tracking files whose save is blocked. (2) `orgsidian-core` (the only crate the LEAF graph rule lets wrap the vault): a `conflict` module carrying `//! Implements FR-16 (v0.1 fallback strategy)` with `resolve_dirty_conflict` (reads disk + Dirty Buffer, builds `ConflictState`, runs the injected strategy via `resolve_with`, records the block, returns a redaction-safe `ConflictNotice`), `save_buffer` (the save gate: blocked → `ExternalConflict`; else atomic-write + mark clean), and `discard_external_changes` (clear the block). (3) `orgsidian-shell-app`: `AppState` holds the two shared registries; `save_file`/`discard_external_changes`/`open_in_default_editor` commands; a `ConflictDetected { path, state }` specta event with a redaction-safe `ConflictSummary` projection + `emit_conflict_detected` helper. Frontend: a `ConflictBanner` React component (subscribed to `conflict-detected`, `role="status"`/`aria-live="polite"`, the two actions) rendered inside the `Editor` surface. The watcher event loop that ties `reconcile_external_write` → `resolve_dirty_conflict` → `emit_conflict_detected` is the SAME loop Story 5.4 deferred for the CLEAN branch (documented residual).

## Boundaries & Constraints

**Always:**
- Never silently overwrite unsaved work (NFR-16). The save gate and both registries **fail safe to BLOCKED/DIRTY** on a poisoned lock — an unknowable state blocks the save, never lets it through.
- Redaction: the user's buffer + external content NEVER cross IPC. Project only path + content byte-*lengths* + ancestor hash into `ConflictNotice`/`ConflictSummary`. Add no `serde` to the conflict types (closes the Story 5.3 deferred redaction note).
- Path identity: registries key on the caller's path verbatim (same contract as `DirtyBufferManager`/`open_file`); normalization is deferred with `open_file`'s hardening.
- The active strategy stays `BlockWithWarning`, injected as `&dyn ResolveConflict` — `resolve_dirty_conflict` runs it through `resolve_with`, never hard-codes the outcome.
- Banner is calm, non-modal, inline in the editor surface: no modal, no warning colors, no exclamation marks; `role="status"`/`aria-live="polite"` (never assertive); native keyboard-operable `<button>`s with a visible `--org-border-focus` ring; `--org-*` token vocabulary.
- Match surrounding module-doc/comment density, LD/FR trace headers, and the sibling component patterns (ModeSwitcher/IndexScanProgress).

**Ask First:**
- Any change to the 5.4 `reconcile_external_write` signature or the `ExternalWriteOutcome::DirtyConflict { path }` shape (would break inherited 5.4 tests).
- Adding any new external dependency (offline — none beyond the warmed lockfile).

**Never:**
- No three-pane Merge Dialog and no swapping the active strategy away from `BlockWithWarning` (Epic 9).
- No writer-ID suppress-token logic (save-cycle wiring, deferred with the watcher loop).
- Do NOT touch `sprint-status.yaml`, `deny.toml`, `tests/anchor.rs`, or the inherited 5.1/5.3/5.4 commits.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| External write on a DIRTY buffer, `BlockWithWarning` | `is_dirty==true`; fresh disk content differs | `resolve_dirty_conflict` records the block in `PendingConflicts`, returns `Some(ConflictNotice)`; the (deferred loop) emits `ConflictDetected { path, state }` | N/A |
| Buffer turned clean before the event is handled | `get_buffer(path) == None` | `Ok(None)` — no block recorded (a save/reload won the race) | N/A |
| Save while blocked | `pending.is_conflicted(path)` | `save_file` → `Err(OrgError::Vault)` (from `VaultError::ExternalConflict`); disk untouched | fail-safe to BLOCKED on lock poison |
| Discard then save | block cleared, then save | `discard_external_changes` clears the block; next `save_buffer` atomic-writes the buffer over the external write + marks clean | N/A |
| Save while unblocked | no pending conflict | atomic-write via Story 3.1 `atomic_write`, buffer marked clean | mapped `VaultError` on write failure |
| Poisoned pending lock | lock poisoned | `save_buffer` returns `ExternalConflict` (fail-safe) | never a panic out of a command |
| External file unreadable during resolve | non-`NotFound` read error | `resolve_dirty_conflict` → `Err(OrgError::Io)` | `NotFound` → empty external content |
| Banner: event for a different file | `payload.path != filePath` | banner stays hidden | N/A |
| Banner: discard action | click "Discard external changes" | `commands.discardExternalChanges(path)` then dismiss banner (a failed clear leaves it up) | catch → keep banner |
| Banner: view action | click "View file in default editor" | `commands.openInDefaultEditor(path)`; banner stays (view does not resolve) | best-effort catch |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-vault/src/error.rs` -- MODIFY. Add `VaultError::ExternalConflict { path }` (no `io::Error` source — a deliberate block, not a failure); `into_io` maps it via `io::Error::other`.
- `crates/orgsidian-vault/src/pending.rs` -- NEW. `//! Implements FR-16 (v0.1 fallback strategy)`. `PendingConflicts` (a `HashSet<PathBuf>` presence set) + `SharedPendingConflicts = Arc<RwLock<_>>`; `mark`/`clear`/`is_conflicted`; fail-safe-to-BLOCKED poison rule documented + tested. Colocated unit tests.
- `crates/orgsidian-vault/src/lib.rs` -- MODIFY. `pub mod pending;` + `pub use`; Story 5.5 crate-doc sentence.
- `crates/orgsidian-core/src/conflict.rs` -- NEW. `//! Implements FR-16 (v0.1 fallback strategy)`. `ConflictNotice` (redaction-safe projection: path + ancestor `Sha256Hash` + external/buffer byte-lengths), `resolve_dirty_conflict`, `save_buffer`, `discard_external_changes`. Colocated unit tests (block, clean-race, blocked→discard→save, unblocked save, poisoned-lock fail-safe).
- `crates/orgsidian-core/src/lib.rs` -- MODIFY. `pub mod conflict;` + re-export the conflict surface + the vault `BlockWithWarning`/`ConflictStrategy`/`ResolveConflict`/`SharedDirtyBuffers`/`Sha256Hash` (LEAF façade: the shell reaches the vault only through core).
- `crates/orgsidian-core/src/reconcile.rs` -- MODIFY (doc only). Point the `DirtyConflict` variant at the now-landed `crate::conflict::resolve_dirty_conflict` consumer.
- `crates/orgsidian-shell-app/src/lib.rs` -- MODIFY. `AppState` gains `dirty_buffers`/`pending_conflicts` (both `Default`); `ConflictSummary` (`#[serde(rename_all="camelCase")]`, per the `OrgError` precedent) + `ConflictDetected` event + `from_notice`/`emit_conflict_detected`; `save_file`/`discard_external_changes`/`open_in_default_editor` commands; register all in `build_specta`. Colocated redaction test for the event projection.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- MODIFY. Add anchors for the 3 new commands + the `conflict-detected` event.
- `shell-ui/src/components/editor/ConflictBanner.tsx` -- NEW. Self-contained banner subscribed to `events.conflictDetected` for its `filePath`; `role="status"`/`aria-live="polite"`; "Discard external changes" (`commands.discardExternalChanges`) + "View file in default editor" (`commands.openInDefaultEditor`); `--org-*` tokens.
- `shell-ui/src/components/editor/Editor.tsx` -- MODIFY. Render `<ConflictBanner filePath={filePath} />` at the top of the editor surface.
- `shell-ui/src/components/editor/ConflictBanner.test.tsx` -- NEW. Renders-nothing / on-event / a11y / ignore-other-file / discard-clears-and-dismisses / view-opens.
- `shell-ui/src/components/editor/Editor.test.tsx` + `ModeSwitcher.test.tsx` -- MODIFY (test mocks). Extend the `@/lib/tauri` mock with the no-op `events.conflictDetected.listen` + the two commands the mounted `ConflictBanner` child needs.

## Tasks & Acceptance

**Execution:**
- [x] `orgsidian-vault`: `VaultError::ExternalConflict` + `pending.rs` registry + lib wiring.
- [x] `orgsidian-core`: `conflict.rs` (`resolve_dirty_conflict`/`save_buffer`/`discard_external_changes`/`ConflictNotice`) + re-exports + reconcile doc pointer.
- [x] `orgsidian-shell-app`: `AppState` registries, `ConflictDetected`/`ConflictSummary`/`emit_conflict_detected`, the 3 commands, `build_specta` + `export_bindings` anchors.
- [x] Frontend: `ConflictBanner` + Editor wiring + tests + sibling-test mock updates.

**Acceptance Criteria:**
- Given Stories 5.3 + 5.4, when the active `ConflictStrategy` is `BlockWithWarning` and an external write is detected on a file with a Dirty Buffer, then a `ConflictDetected { path, state }` Tauri event is produced. *(Event type, redaction-safe payload projection, registration, and `emit_conflict_detected` shipped + unit-tested; the `.emit()` call fires from the watcher event loop — the shared deferred Epic-5 seam.)*
- And the frontend renders a banner in the editor surface: "{path} was changed externally — save blocked. [Discard external changes] [View file in default editor]". *(`ConflictBanner`, tested.)*
- And save attempts via `commands.saveFile(path, content)` return `Err(OrgError::Vault(VaultError::ExternalConflict { path }))`. *(`save_file` → `save_buffer` maps `VaultError::ExternalConflict` via the established `vault_err` → `OrgError::Vault`; tested.)*
- And clicking "Discard external changes" allows a subsequent save to overwrite (still atomic-write). *(`discard_external_changes` clears the block; next `save_buffer` atomic-writes; tested end-to-end.)*
- And the module carries `//! Implements FR-16 (v0.1 fallback strategy)`. *(On `orgsidian-core/src/conflict.rs` and `orgsidian-vault/src/pending.rs`.)*

## Design Notes

- **Why the 5.4 reconcile signature is untouched.** `reconcile_external_write` returns `ExternalWriteOutcome::DirtyConflict { path }`; the inherited 5.4 integration tests match that variant by exact fields, so adding fields or params would break them. Story 5.5 therefore consumes the marker in a *separate* core function, `resolve_dirty_conflict`, that the (deferred) watcher loop calls after reconcile — the honest realization of "wire the DIRTY branch through the strategy" given the frozen 5.4 contract.
- **Redaction across IPC.** `ConflictState` holds the user's unsaved buffer + external content. `resolve_dirty_conflict` captures only a redaction-safe `ConflictNotice` (path + byte-lengths + ancestor hash) BEFORE `resolve_with` consumes the state, and the shell serializes only that. No `serde` is added to the conflict types — closing the Story 5.3 deferred redaction concern by construction.
- **`ConflictSummary` camelCase.** The architecture mandates a camelCase IPC wire via a project-wide specta rename, which is unavailable in the pinned `tauri-specta =2.0.0-rc.25` (why `OrgError` carries `#[serde(rename_all="camelCase")]`). `ConflictSummary` is the first multi-word event struct, so it follows that precedent; recorded in `deferred-work.md` for the eventual global-rename cleanup.
- **Frontend seam (honest split).** Fully wired + tested: the block gate, discard flow, atomic re-write, the banner + both actions, and the event payload projection. The residual (recorded in `deferred-work.md`): the per-vault `WatcherFacade` event loop that calls `reconcile_external_write` → `resolve_dirty_conflict` → `emit_conflict_detected` — the same loop 5.4 deferred for the CLEAN branch, needing a live `AppHandle` untestable offline.

## Verification

**Commands:**
- `cargo test -p orgsidian-watcher -p orgsidian-vault -p orgsidian-index -p orgsidian-core -p orgsidian-shell-app --offline` -- expected: all green, no 5.1/5.3/5.4 regressions.
- `cargo clippy -p orgsidian-core --all-targets --features test-support --offline -- -D warnings` and `cargo clippy -p orgsidian-vault -p orgsidian-shell-app --all-targets --offline -- -D warnings` -- expected: 0 warnings from touched crates (parser C-compiler warnings are pre-existing).
- `cargo fmt -p orgsidian-vault -p orgsidian-core -p orgsidian-shell-app -- --check` -- expected: clean.
- `node_modules/.bin/vitest run` (from `shell-ui/`) -- expected: all green incl. new `ConflictBanner.test.tsx`.

**Result (2026-08-21):** Rust suite GREEN across all 5 crates (0 failed; incl. 6 new `pending` unit tests, 5 new `conflict` unit tests, 1 new shell-app redaction test, and the `export_bindings` regen with the 3 new command + event anchors). `cargo clippy` (core with `test-support`; vault + shell-app) and `cargo fmt --check` clean on touched crates. `vitest run` GREEN: 21 files, 246 tests (6 new `ConflictBanner` tests; Editor + ModeSwitcher mocks extended for the new child). `Cargo.lock` unchanged (no new crates).

## Spec Change Log

- 2026-08-21 — Implemented. `orgsidian-vault` (`VaultError::ExternalConflict` + `pending.rs`), `orgsidian-core/src/conflict.rs` (`resolve_dirty_conflict`/`save_buffer`/`discard_external_changes`/`ConflictNotice`), `orgsidian-shell-app` (`ConflictDetected` event + 3 commands), and the `ConflictBanner` frontend. All AC wired except the `.emit()` from the deferred watcher loop; all gates green offline. Status → review.
