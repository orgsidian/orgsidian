---
title: 'Hardcoded coaching balloons for UJ-4 v0.1 first-run'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: '1d5d428'
review_loop_iteration: 0
github_issue: 57
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

> **Renegotiated post-review (2026-09-05):** two reviewer-directed fixes are folded into this frozen section — (1) `UJ4_CAPTURE_INTRO` render-gated until Story 8.1 (the capture hotkey isn't wired yet), full detail in Design Notes below; (2) `read_dismissed_coaching` self-heals a malformed `coaching-dismissed.json` instead of erroring, reflected in the I/O matrix row below. Neither changes the AC intent — see the Spec Change Log entry at the bottom.

## Intent

**Problem:** Stories 6.1 + 6.2 + 6.3 land the Starter Vault picker and a basic Today Agenda, but a first-time user landing on `/today` with example content sees only a bare list — nothing tells them what it is, that clicking a task opens its source file, or that Quick Capture exists. UJ-4's "first five minutes" promise needs SOME coaching now, without waiting for the full FR-21 registry-driven `CoachingSlot` API (Epic 11, v0.5 Beta).

**Approach:** Ship two hardcoded, non-modal coaching balloons as an explicitly disposable v0.1 stand-in. (1) `orgsidian-core`: a new `coaching` module (`src/coaching.rs`) — deliberately separate from the locked `VaultSettings.dismissed_coaching` TOML field (that field is Story 11.5's home for the real mechanism) — persisting a plain JSON array of dismissed coaching ids at `<Vault>/.orgsidian/coaching-dismissed.json` via the Story 3.1 atomic-write path, plus the two hardcoded ID constants (`UJ4_TODAY_INTRO`, `UJ4_CAPTURE_INTRO`). (2) `orgsidian-shell-app`: `get_dismissed_coaching` / `dismiss_coaching` commands, Vault-scoped like `agenda_today`. (3) Frontend: a self-contained `CoachingBalloon` component (mirrors the Story 5.5 `ConflictBanner` calm/non-modal idiom — `role="status"`/`aria-live="polite"`, native keyboard-operable dismiss button, `--org-*` tokens, no new tokens) mounted twice — once inside `AgendaToday` pointing at the first Agenda item, once at the top of the `/today` route for the Quick Capture nudge (see Design Notes for why that one isn't anchored to an "Inbox preview" section).

## Boundaries & Constraints

**Always:**
- Balloons are non-modal, calm: no modal, no warning colors, no exclamation marks; `role="status"`/`aria-live="polite"` (never assertive); native `<button>` dismiss control with a visible `--org-border-focus` ring; `--org-*` token vocabulary only (no new tokens).
- Dismissal persists at `<Vault>/.orgsidian/coaching-dismissed.json`, keyed by the hardcoded coaching IDs `UJ4_TODAY_INTRO` / `UJ4_CAPTURE_INTRO` — exact strings, since Story 11.4 imports them verbatim.
- The coaching-dismissed store is a SEPARATE file/module from `VaultSettings.dismissed_coaching` (Story 11.5's field) — this whole mechanism must be deletable in one PR without touching the locked settings schema.
- Match surrounding module-doc/comment density, LD/FR trace headers (`//! Implements FR-21 (partial) / FR-18 / UJ-4`), and the sibling component patterns (`ConflictBanner`, `AgendaToday`).
- Reuse the existing `current_vault_root` seam (`AppState::current_vault_root`) and the Story 3.1 `atomic_write` path — no new writer discipline invented.

**Ask First:**
- Any change to the `AgendaToday`/`agenda_today` Story 6.3 contract or the `StarterVaultPicker` Story 6.2 onboarding-gate flow.
- Adding any new external dependency (offline — none beyond the warmed lockfile; `serde_json` moves from optional/test-support-gated to an unconditional `orgsidian-core` dependency, but it is already in the workspace lockfile).

**Never:**
- No fabricated Inbox preview section — Epic 7 owns that surface. `UJ4_CAPTURE_INTRO` must not anchor to UI that doesn't exist yet.
- No coupling of the hardcoded coaching IDs/store to the FR-21 registry (`VaultSettings.dismissed_coaching`) — that field stays untouched, reserved for Story 11.5.
- Do NOT touch `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| First `/today` visit, no Vault dismissal file yet | `coaching-dismissed.json` absent | `read_dismissed_coaching` → empty set; both balloons eligible to render | N/A (default-on-missing, not an error) |
| Agenda has items, `UJ4_TODAY_INTRO` not dismissed | items.length > 0 | Balloon renders above the first (grouped) item | N/A |
| Agenda is empty | items.length === 0 | No balloon (nothing to point at) | N/A |
| `UJ4_TODAY_INTRO` already dismissed | id present in the read-back set | Balloon does not render | N/A |
| Dismiss button clicked | user clicks X | `commands.dismissCoaching(id)`; balloon hides immediately (optimistic) | Persist failure caught + swallowed — balloon stays hidden regardless (never re-shown mid-session for a non-critical hint) |
| Two ids dismissed over time | `dismiss_coaching` called for id A then id B | Both persist; the on-disk set is additive, never overwritten wholesale | N/A |
| Same id dismissed twice | repeat `dismiss_coaching(id)` | Idempotent — set size unchanged | N/A |
| `coaching-dismissed.json` malformed | file exists, invalid JSON | Post-review fix (2026-09-05, renegotiated): `read_dismissed_coaching` self-heals — treats a JSON *parse* failure as an empty set (`Ok(BTreeSet::new())`) rather than erroring, so a corrupt file can't permanently trap a balloon in "always resurfaces, can never be dismissed" limbo. A subsequent `dismiss_coaching` overwrites the file with valid content. Non-parse I/O errors (permissions, etc.) are unaffected — still `Err(OrgError::Io)`. | N/A for parse failures (self-healed, not an error); IO errors (permissions, etc.) still surface as `Err`, and the frontend still fails safe to hidden on those |
| No active Vault | `current_vault_root()` is `None` | `get_dismissed_coaching`/`dismiss_coaching` → `Err(OrgError::Vault)` | Frontend catches → balloon fails safe to hidden |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-core/Cargo.toml` -- MODIFY. `serde_json` moved off the `test-support` feature's activation list to an unconditional dependency (Story 6.6's production JSON I/O needs it outside `test-support`); doc comments updated to match.
- `crates/orgsidian-core/src/coaching.rs` -- NEW. `//! Implements FR-21 (partial) / FR-18 / UJ-4`. `UJ4_TODAY_INTRO` / `UJ4_CAPTURE_INTRO` constants, `coaching_dismissed_path`, `read_dismissed_coaching` (default-empty-on-missing), `dismiss_coaching` (additive, idempotent, atomic-write). Colocated unit tests.
- `crates/orgsidian-core/src/lib.rs` -- MODIFY. `pub mod coaching;` + re-exports.
- `crates/orgsidian-shell-app/src/lib.rs` -- MODIFY. `get_dismissed_coaching` / `dismiss_coaching` commands (Vault-scoped via `current_vault_root`/`no_active_vault`, same pattern as `agenda_today`); registered in `build_specta`'s `collect_commands!`.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- MODIFY. Anchors for the 2 new commands.
- `shell-ui/src/components/coaching/coachingIds.ts` -- NEW. The two coaching-id string constants (frontend mirror of the Rust constants — no specta enum exists for arbitrary IDs, so this is the one place they're re-stated).
- `shell-ui/src/components/coaching/CoachingBalloon.tsx` -- NEW. Self-contained balloon: on mount calls `commands.getDismissedCoaching()`, renders nothing while loading / once dismissed / on error (fail-safe hidden); `role="status"`/`aria-live="polite"`; dismiss button calls `commands.dismissCoaching(id)` optimistically.
- `shell-ui/src/components/coaching/CoachingBalloon.test.tsx` -- NEW. Loading / dismissed / not-dismissed / error-fail-safe / keyboard-operable / dismiss-persists / dismiss-survives-persist-failure.
- `shell-ui/src/components/agenda/AgendaToday.tsx` -- MODIFY. Mounts `<CoachingBalloon id={UJ4_TODAY_INTRO}>` above the grouped item list, only when `items.length > 0`.
- `shell-ui/src/components/agenda/AgendaToday.test.tsx` -- MODIFY (test mocks + new tests). Extends the `@/lib/tauri` mock with `getDismissedCoaching`/`dismissCoaching` (defaulted to "already dismissed" so the Story 6.3 assertions stay decoupled); adds a `describe` block covering render/no-render/dismiss for the mounted balloon.
- `shell-ui/src/routes/_layout/today.tsx` -- MODIFY. Mounts `<CoachingBalloon id={UJ4_CAPTURE_INTRO}>` at the top of the route content, above `<AgendaToday />` (see Design Notes for the anchor decision).
- `docs/microcopy-registry.md` -- NEW. Balloon copy + design tokens + the `UJ4_CAPTURE_INTRO` anchor decision, status `[draft]`.

## Tasks & Acceptance

**Execution:**
- [x] `orgsidian-core`: `coaching.rs` (constants + read/dismiss + tests) + `Cargo.toml`/`lib.rs` wiring.
- [x] `orgsidian-shell-app`: the 2 commands + `build_specta`/`export_bindings` anchors.
- [x] Frontend: `CoachingBalloon` + `coachingIds.ts` + tests; `AgendaToday` + `/today` route wiring + sibling-test mock updates.
- [x] `docs/microcopy-registry.md`.

**Acceptance Criteria:**
- Given Stories 6.1 + 6.2 + 6.3, when the user finishes Starter Vault selection and lands on `/today`, then a non-modal balloon renders pointing at the first Agenda item with "**This is your day.** Click any task to open the source file." *(`CoachingBalloon` mounted in `AgendaToday`, tested.)*
- And a second balloon renders with "**Anything on your mind?** Press Cmd/Ctrl+Shift+Space to capture from anywhere." *(`CoachingBalloon` mounted at the top of `/today`; anchored to a calm top-of-route placement rather than a not-yet-built Inbox preview section — see Design Notes; the id/copy/dismissal are exactly per AC.)*
- And dismissing either balloon (X button) persists the dismissal at `<Vault>/.orgsidian/coaching-dismissed.json` keyed by `UJ4_TODAY_INTRO`/`UJ4_CAPTURE_INTRO`. *(`dismiss_coaching` Rust command + `coaching.rs`, tested end-to-end; `CoachingBalloon`'s dismiss button, tested.)*
- And the hardcoded balloons are directly removable by Story 11.4 without touching `VaultSettings`. *(Separate file/module by construction — no code path couples `coaching.rs` to `settings::schema::VaultSettings`.)*
- And the balloon text and design tokens are recorded in `docs/microcopy-registry.md` with status `[draft]`. *(Done — includes the anchor-decision note.)*

## Design Notes

- **Why a separate JSON file, not `VaultSettings.dismissed_coaching`.** That TOML field already exists in `settings/schema.rs`, reserved with the comment "Story 11.5 lands the persist" — it's the FR-21 registry-driven mechanism's home. Coupling Story 6.6's hardcoded, explicitly-disposable balloons to that locked schema would mean Story 11.4's "remove wholesale" instruction turns into "carefully unpick from the settings schema." A standalone `coaching-dismissed.json`, written by a standalone `coaching.rs` module, is deletable in one PR — which is the actual requirement.
- **`UJ4_CAPTURE_INTRO` anchor — locked decision (2026-09-05).** The epic AC anchors this balloon to "the Inbox preview section" of the Today Dashboard. That section is Epic 7 scope (FR-6) and does not exist on `/today` yet — Story 6.3 shipped only the Scheduled/Deadline Agenda list. Building a placeholder Inbox preview just to host a coaching balloon would ship UI nothing else motivates, so instead the balloon renders at a calm top-of-route placement above the Agenda. The id, copy, dismissal persistence, and component are otherwise identical to the AC — a future Epic 7 story (or Story 11.4's registry cutover) re-anchors the same `<CoachingBalloon id={UJ4_CAPTURE_INTRO}>` element to the real Inbox preview without touching the persistence seam this story ships.
- **`serde_json` becomes unconditional in `orgsidian-core`.** Previously gated behind the `test-support` feature (Story 1.12's perf-baseline JSON I/O, `optional = true` + `dep:serde_json` activator) to keep it out of production builds. Story 6.6's `coaching.rs` needs real JSON I/O in production, so the dependency is now a normal (non-optional) `orgsidian-core` dependency — already present in the workspace `Cargo.lock` (used elsewhere, e.g. `orgsidian-shell-app`), so this is not a new external dependency, just a new consumer of an existing one.
- **Optimistic dismiss.** `CoachingBalloon` hides itself immediately on the X click rather than waiting for `commands.dismissCoaching` to resolve (contrast `ConflictBanner`'s discard flow, which waits because a failed clear leaves a real data-safety block in place). A coaching hint has no such stakes: a failed persist just means the balloon may resurface on a later launch, which is an acceptable degrade — never worth re-showing it mid-session or making the dismiss click feel unresponsive.
- **Independent per-balloon dismissed-check.** Both `CoachingBalloon` mounts call `commands.getDismissedCoaching()` independently rather than sharing a Zustand store or context provider. At two hardcoded balloons this is simplest, and the whole mechanism is slated for removal in Story 11.4 — not worth building shared infrastructure for.
- **`UJ4_CAPTURE_INTRO` is implemented but render-gated until Story 8.1 (post-review fix, 2026-09-05).** The capture hotkey (Cmd/Ctrl+Shift+Space) is not wired until Story 8.1 (Epic 8, after v0.1 Alpha) — shipping this balloon on first run would coach a user toward a shortcut that does nothing yet, which is worse than not coaching at all. The fix keeps the component code, the `UJ4_CAPTURE_INTRO` id, its copy, and its dismissal persistence fully intact (so Story 8.1/11.4 inherit them unchanged) and gates only the render site: `shell-ui/src/routes/_layout/today.tsx` wraps the `<CoachingBalloon id={UJ4_CAPTURE_INTRO}>` mount in `{CAPTURE_FEATURE_AVAILABLE && (...)}`, a single `const CAPTURE_FEATURE_AVAILABLE = false` declared at the top of the route file with a `// TODO(Story 8.1): render once the capture hotkey is wired` comment. Story 8.1 flips that one constant to `true` (or wires it to a real "capture hotkey configured" signal, if one exists by then) — no other change needed. `UJ4_TODAY_INTRO` is unaffected: it works today and stays rendered unconditionally. `docs/microcopy-registry.md` records the same gated status.

## Verification

**Commands:**
- `cargo test -p orgsidian-core -p orgsidian-shell-app --offline` -- expected: all green, no Story 6.1/6.2/6.3/6.7 regressions.
- `cargo clippy -p orgsidian-core --all-targets --features test-support --offline -- -D warnings` and `cargo clippy -p orgsidian-shell-app --all-targets --offline -- -D warnings` -- expected: 0 warnings from touched crates (parser C-compiler warnings are pre-existing).
- `cargo fmt -p orgsidian-core -p orgsidian-shell-app -- --check` -- expected: clean.
- `pnpm --filter shell-ui test` (vitest) -- expected: all green incl. new `CoachingBalloon.test.tsx` + the new `AgendaToday.test.tsx` describe block.
- `pnpm --filter shell-ui build` -- expected: `tsc` + `vite build` clean (regenerates `tauri.ts` via the `prebuild` step's `export_bindings` test).

**Result (2026-09-05):** Rust suite GREEN across both crates (0 failed; incl. 6 new `coaching` unit tests) with the `export_bindings` regen picking up `getDismissedCoaching`/`dismissCoaching`. `cargo clippy` (core with `test-support`; shell-app) and `cargo fmt --check` clean on touched crates. `vitest run` GREEN: 28 files, 311 tests (8 new `CoachingBalloon` tests + 4 new `AgendaToday` balloon-integration tests; `AgendaToday.test.tsx`'s existing Story 6.3 assertions unaffected — the mock defaults the balloon to "already dismissed"). `pnpm --filter shell-ui build` (`tsc && vite build`) clean. `Cargo.lock` unchanged (no new crates — `serde_json` was already present, only its `orgsidian-core` gating changed).

## Spec Change Log

- 2026-09-05 — Implemented. `orgsidian-core/src/coaching.rs` (hardcoded dismissal store + IDs), `orgsidian-shell-app` (2 commands), `CoachingBalloon` + `coachingIds.ts` frontend, `AgendaToday`/`/today` wiring, `docs/microcopy-registry.md`. `UJ4_CAPTURE_INTRO` anchored to a calm top-of-route placement (Epic 7 Inbox preview does not exist yet) — locked decision recorded above and in the microcopy registry. All gates green offline. Status → review.
- 2026-09-05 — Post-review fixes: (1) `UJ4_CAPTURE_INTRO` render-gated behind `CAPTURE_FEATURE_AVAILABLE = false` in `today.tsx` until Story 8.1 wires the capture hotkey — component/id/copy/dismissal untouched, `-today.test.tsx` flipped to assert it does not render while gated; `docs/microcopy-registry.md` and this file's Design Notes/I-O matrix updated to record the gated-but-frozen status. (2) `read_dismissed_coaching` self-heals a malformed `coaching-dismissed.json` (parse failure → empty set) instead of erroring, so a corrupt file can no longer trap a balloon in permanent resurface-with-no-way-to-dismiss limbo; non-parse IO errors still propagate. Colocated `coaching.rs` tests updated.
