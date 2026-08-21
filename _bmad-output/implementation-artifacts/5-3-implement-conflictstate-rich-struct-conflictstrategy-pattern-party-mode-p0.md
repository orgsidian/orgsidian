---
title: 'Implement ConflictState rich struct + ConflictStrategy pattern (Party Mode P0)'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '0838dcd'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/3-2-scaffold-dirty-buffer-manager.md']
github_issue: 49
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 5 ships the v0.1 safe fallback (dirty buffer + external write → *block save with warning*), but Epic 9 replaces that fallback with a full three-pane Merge Dialog. Party Mode P0 (Winston + Murat consensus) mandates the conflict be modeled **rich from day-1** — a struct carrying `ancestor_hash`/`external_content`/`buffer_content`, not a boolean — and the resolution behavior be a **strategy pattern** injected as `&dyn ResolveConflict`, so Epic 9 swaps the variant WITHOUT rewriting the watcher state machine (the "Epic 9 watcher-rewrite trap").

**Approach:** Add a pure, I/O-free `conflict` model to `orgsidian-vault` (the crate that already owns the Dirty Buffer — Story 3.2): a rich `ConflictState`, a `Resolution` outcome enum (`Block | WriteMerged | Cancel`), a `ResolveConflict` trait, two concrete strategies (`BlockWithWarning`, `ThreePaneMergeDialog`), and a `ConflictStrategy` selector enum enumerating them. Expose the exact injection seam the watcher will call — `resolve_with(&dyn ResolveConflict, ConflictState) -> Resolution` — so swapping the active strategy is a one-line change at startup. `Sha256Hash` has no home in the workspace yet, so add a minimal newtype (backed by the `sha2` crate already resolved in `Cargo.lock`).

## Boundaries & Constraints

**Always:** Pure in-memory types — no filesystem, no `Result`, no `tracing`. Match the Story 3.2 `dirty_buffer` conventions: module-doc header naming LD/FR traces, redacting `Debug` (user note content must never reach a log/panic/`{:?}`), no `unwrap`/`expect`/`panic!` in non-test code. Strategy behavior reachable through `&dyn ResolveConflict` (object-safe trait). `sha2` added as a workspace dependency + consumed via `{ workspace = true }` (single-source-of-truth, `feedback_version_policy`).

**Never:** Do NOT edit `orgsidian-watcher` or any crate other than `orgsidian-vault` (the watcher wiring itself is Story 5.4 — Stories 5.1/5.2 are built in parallel and absent here; this story ships the SEAM only). No `notify-rs`, no debounce, no Tauri command / event / `Serialize` (IPC payload shaping is Story 5.5 / Epic 9). No real diff/merge algorithm (Epic 9). No new external crate beyond `sha2` (offline — only `Cargo.lock` crates). Do not touch `src/atomic.rs`, `src/error.rs`, `src/path.rs`, `tests/anchor.rs`, `deny.toml`, `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| BlockWithWarning resolves | any `ConflictState` | `Resolution::Block { path }` carrying the conflicted path | N/A (infallible) |
| ThreePaneMergeDialog accept | dialog `decision = Accept { merged_content }` | `Resolution::WriteMerged { path, merged_content }` | N/A |
| ThreePaneMergeDialog cancel | dialog `decision = Cancel` | `Resolution::Cancel` (dirty buffer preserved — nothing written) | N/A |
| Injection via `&dyn` | `resolve_with(strategy, state)` for each strategy | delegates to the strategy's `resolve`; swapping the trait object swaps the outcome | N/A |
| `Sha256Hash::of` determinism | same bytes hashed twice | equal hashes; different bytes → different hash | N/A |
| Redacting `Debug` | `ConflictState`/`Resolution::WriteMerged` holding note text | `{:?}` prints path + content byte-lengths + hash, never the note text | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-vault/src/conflict.rs` -- NEW. `//! Implements FR-16 conflict model (Party Mode P0 rich form).` Declares `ConflictState` (private fields per AC, redacting `Debug`, `new` + getters), `Resolution` enum, `ResolveConflict` trait, `BlockWithWarning`, `ThreePaneMergeDialog { MergeDecision }`, `ConflictStrategy` selector enum, and `resolve_with` injection seam. Colocated `#[cfg(test)]` unit tests.
- `crates/orgsidian-vault/src/hash.rs` -- NEW. Minimal `Sha256Hash([u8; 32])` newtype (backed by `sha2`); `of(&[u8])`, `from_bytes`, `as_bytes`, hex `Display`/`Debug`, `Clone/Copy/PartialEq/Eq/Hash`. Not conflict-specific — a general primitive a future index content-hash consumer may promote.
- `crates/orgsidian-vault/src/lib.rs` -- MODIFIED (re-export site only, mirrors 3.2): `pub mod conflict; pub mod hash;` + `pub use` the new public items; crate-doc gains a present-tense Story 5.3 sentence.
- `crates/orgsidian-vault/tests/conflict_strategy.rs` -- NEW. Parameterized suite over `Vec<(&str, Box<dyn ResolveConflict>)>` asserting the three contract invariants (`Block`, `WriteMerged`, `Cancel`) reachable through the `&dyn ResolveConflict` seam.
- `crates/orgsidian-vault/Cargo.toml` -- MODIFIED: `sha2 = { workspace = true }`.
- `Cargo.toml` (workspace) -- MODIFIED: `sha2` in `[workspace.dependencies]` (already in `Cargo.lock` transitively via Tauri; adds an edge, zero new crates).
- `crates/orgsidian-vault/src/dirty_buffer.rs` -- READ-ONLY reference: `get_buffer` sources `buffer_content`; convention model (redacting `Debug`, module-doc traces).

## Tasks & Acceptance

**Execution:**
- [x] `Cargo.toml` (workspace) + `crates/orgsidian-vault/Cargo.toml` — add `sha2` (workspace dep + `{ workspace = true }`), `default-features = false` (minimal, offline).
- [x] `src/hash.rs` — `Sha256Hash` newtype + `of`/`from_bytes`/`as_bytes` + hex `Display`/`Debug` + derives; colocated tests.
- [x] `src/conflict.rs` — `ConflictState` (rich, private fields, redacting `Debug`, `new`+getters), `Resolution`, `ResolveConflict`, `BlockWithWarning`, `ThreePaneMergeDialog`+`MergeDecision`, `ConflictStrategy`+`as_resolver`, `resolve_with` seam; colocated tests.
- [x] `src/lib.rs` — module decls + re-exports + present-tense crate-doc sentence.
- [x] `tests/conflict_strategy.rs` — single parameterized suite over both strategies; assert `Block`/`WriteMerged`/`Cancel` via `&dyn ResolveConflict`.

**Acceptance Criteria:**
- Given Stories 5.1+5.2+3.2, when the strategy pattern is implemented, then `conflict.rs` declares `pub struct ConflictState { ancestor_hash: Sha256Hash, external_content: String, buffer_content: String, file_path: PathBuf }` — verified by compilation + field/getter tests.
- And `pub enum ConflictStrategy` declares variants `BlockWithWarning | ThreePaneMergeDialog` and `pub trait ResolveConflict { fn resolve(&self, state: ConflictState) -> Resolution }` is implemented by each concrete strategy (and delegated by the selector enum) — verified.
- And the injection seam consumes `&dyn ResolveConflict` (`resolve_with`) so the active strategy is chosen at startup — verified by the parameterized `&dyn` suite; the watcher call-site wiring lands in Story 5.4 (documented in-code + here).
- And `tests/conflict_strategy.rs` parameterizes a single suite over both strategies and asserts `Resolution::Block`, `Resolution::WriteMerged`, `Resolution::Cancel` — verified.
- And Epic 9 swaps the active strategy without modifying the state machine (inject a different `&dyn ResolveConflict`) — verified by the seam design.

## Design Notes

- **Why the watcher AC is a seam, not wiring.** Stories 5.1/5.2 (the `orgsidian-watcher` state machine) are built in parallel and are ABSENT from this worktree; the LEAF graph rule (`deny.toml`) also forbids a vault→watcher edge. So this story ships the *contract* the watcher consumes — `resolve_with(strategy: &dyn ResolveConflict, state: ConflictState) -> Resolution`, the exact call the DIRTY branch makes — and Story 5.4 (which stacks watcher+vault) wires the state machine to call it. This is the faithful realization of "the watcher state machine consumes `&dyn ResolveConflict`" given the dependency slice available.
- **ThreePaneMergeDialog is a testable placeholder.** A real 3-pane merge is Epic 9 and needs UI interaction, which a pure `resolve(state) -> Resolution` cannot conjure. So the day-1 strategy is driven by an injected `MergeDecision` (`Accept { merged_content } | Cancel`) — the deterministic stand-in for the eventual user decision, and precisely the Epic 9 replacement point. This lets one parameterized suite reach `WriteMerged` and `Cancel` without a fake UI.
- **Two strategy types + a selector enum.** `BlockWithWarning`/`ThreePaneMergeDialog` are concrete types each `impl ResolveConflict` ("implemented by each"); `ConflictStrategy { BlockWithWarning(..) | ThreePaneMergeDialog(..) }` is the AC-named selector enum that wraps them, exposing `as_resolver(&self) -> &dyn ResolveConflict` (and itself `impl ResolveConflict`) so startup holds one value and the watcher gets its trait object.
- **Redacting `Debug` (Story 3.2 precedent).** `external_content`/`buffer_content` and `WriteMerged.merged_content` are the user's unsaved notes; a derived `Debug` would spill them into any enclosing `{:?}`/panic/log. Manual `Debug` prints path + byte-lengths + hash only; the ancestor hash is a digest, not secret, so it prints in full.
- **`Sha256Hash` via `sha2`.** No such type exists yet; `sha2 0.10.9` is already in `Cargo.lock` (transitive via Tauri), so `Sha256Hash::of` adds a dependency *edge* and zero new crates. `default-features = false` keeps it minimal; the digest API is `no_std`-core.

## Verification

**Commands:**
- `cargo build -p orgsidian-vault --offline` -- expected: clean.
- `cargo test -p orgsidian-vault --offline` -- expected: all green (lib unit + `conflict_strategy` integration + doctests).
- `cargo clippy -p orgsidian-vault --offline --all-targets -- -D warnings` -- expected: 0 warnings.
- `cargo fmt -p orgsidian-vault -- --check` -- expected: clean.

**Result (2026-08-21):** `cargo test -p orgsidian-vault --offline` GREEN — 30 lib unit + 4 `conflict_strategy` integration + 1 anchor + 5 atomic + 6 orphan_cleanup + 7 path + 2 doctests, 0 failed. `cargo build`, `cargo clippy … -D warnings`, `cargo fmt --check` all clean. `Cargo.lock` delta is exactly one line (the `sha2` edge on `orgsidian-vault`); zero new crate entries.

## Spec Change Log

- 2026-08-21 — Implemented. `src/conflict.rs` + `src/hash.rs` + `lib.rs` re-exports + `tests/conflict_strategy.rs` + `sha2` workspace edge. All 5 ACs satisfied; all gates green offline. Status → review.
- 2026-08-21 — Code review (4 layers: Blind Hunter, Edge Case Hunter, Verification Gap, Acceptance Auditor). Acceptance Auditor: all 5 ACs PASS. Verification Gap: none. Edge Case Hunter: none. Blind Hunter: 14 findings triaged (below).

## Review Findings

Four-layer adversarial review. **Acceptance Auditor** — all 5 ACs PASS (the tuple-wrapped selector-enum variants and the injection-seam realization of the watcher AC are spec-sanctioned, judged faithful). **Verification Gap** — no gaps (every behavioral surface has a running assertion that observes it). **Edge Case Hunter** — no unhandled paths. **Blind Hunter** — 14 findings, triaged:

**Applied:**
- [x] [Blind] Derived `Debug` on `MergeDecision` transitively leaked merged note text through `ThreePaneMergeDialog`/`ConflictStrategy` (`{:?}` on the startup strategy value). Replaced with a redacting `Debug` on `MergeDecision` (prints `merged_content_len`); the derived `Debug`s above it now delegate through it and are safe. New test `debug_redacts_strategy_merge_content`. **This was the one real defect** — a redaction hole against the module's stated privacy guarantee.
- [x] [Blind] No `Send + Sync` on the injected `&dyn ResolveConflict` — the watcher runs the conflict path off a filesystem-event thread. Added `Send + Sync` supertrait to `ResolveConflict` (every strategy already satisfies it) + compile-time witness `strategies_are_send_sync_for_the_watcher_thread`.
- [x] [Blind] No `#[must_use]` — a dropped `Resolution` silently discards the conflict decision. Added `#[must_use]` to `Resolution` and to `Sha256Hash::of`/`from_bytes`/`as_bytes`.
- [x] [Blind] By-value trait doc overclaimed (only `BlockWithWarning` moves; `ThreePaneMergeDialog` clones). Corrected the doc; also documented the sync/infallible-by-design tension.
- [x] [Blind] `debug_redacts_conflict_content` used brittle `contains("20")`. Strengthened to assert `external_content_len: 20`, `buffer_content_len: 18`, and the ancestor hash's presence.
- [x] [Blind] Story attribution reconciled (filesystem-event reaction = Story 5.4; `ConflictDetected` IPC payload + serde = Story 5.5 / Epic 9).
- [x] [Blind] Softened the `ancestor_hash` doc so it no longer implies a shipped divergence detector; clarified `Cancel` carries no payload because the caller retains the path.

**Deferred** (recorded in `deferred-work.md` §code review of story-5-3): async/fallible `resolve` signature evolution (Epic 9); `serde` derives for the IPC payload (Story 5.5); genuine-conflict divergence detection + take-local/take-remote first-class outcomes (Epic 9); `Sha256Hash` hex `FromStr` / slice `TryFrom` / streaming constructor (YAGNI until a consumer needs them).

**Dismissed as unrequested surface** (Story 3.2 "no unrequested surface" discipline): `Sha256Hash` `Ord`/`PartialOrd`, `TryFrom<&[u8]>`, incremental hashing — no consumer needs them; adding them now widens the public surface ahead of a real need. `Resolution::Cancel` payload — the watcher's DIRTY branch retains the `path` it built the `ConflictState` from, so a payload-free cancel loses nothing (kept `Cancel` as a unit variant, matching the AC).
