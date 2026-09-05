---
title: 'Implement Starter Vault picker UI on first launch'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: '4d6f105'
review_loop_iteration: 0
github_issue: 53
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 6.1 shipped a pure-Rust generator (`orgsidian_core::generate_starter_vault` /
`StarterVaultKind`) that nothing calls yet — there is no first-launch UI, no
Tauri command wiring the generator to a user-chosen folder, and no way to
detect "no configured Vault" so the onboarding surface even knows to show
itself. A first-time user today has no path onto a populated Vault at all;
the `/today` placeholder route unconditionally shows the Story 3.6 manual
`VaultPicker` (a bare folder-choose for an *existing* `.org` folder), which
neither offers a Starter Vault nor gates on Vault state.

**Approach:** Ship `shell-ui/src/components/onboarding/StarterVaultPicker.tsx`
— three primary cards (Personal GTD, Student, Freelancer) plus a secondary
"Use my own folder" link — and wire it into `/today` behind a new
`hasConfiguredVault` onboarding gate. Picking Personal GTD or Student prompts
for a target folder (`tauri-plugin-dialog`) then calls a new
`generate_starter_vault` command, which runs the Story 6.1 generator and then
designates + scans the freshly-populated folder through the SAME body
`designate_vault` uses (factored into a shared `designate_vault_impl`, so the
two commands can never drift on the serialize/previous-handle-shutdown/
cancel-flag discipline). "Use my own folder" reveals the existing Story 3.6
`VaultPicker` inline (extended with an optional `onDesignated` callback)
rather than duplicating its folder-choose + `designateVault` + scan-progress
logic.

**Scope decision (locked 2026-09-05, per the orchestrator brief):** Story 6.1
shipped **Personal GTD + Student only** — the **Freelancer** generator is
deferred (needs Story 8.7's BacklinksPanel for its ≥1-backlink AC; see
`_bmad-output/implementation-artifacts/deferred-work.md`). The Freelancer
card renders per the AC (three primary options) but is **disabled** with a
"Coming soon" affordance and is never wired to `generateStarterVault` — no
Freelancer generator is invented here.

## Boundaries & Constraints

**Always:**
- The picker's primary cards send only `StarterVaultKind` values the Rust
  generator actually has (`"personalGtd" | "student"`) — the wire enum has no
  `Freelancer` variant, so a disabled card cannot even type-check its way
  into a call.
- `today` for `generateStarterVault` is resolved on the FRONTEND as the
  user's local calendar day (`localTodayIso()`, the same helper/convention
  `commands.setScheduled` already uses) and sent as `YYYY-MM-DD` — the
  backend never reads the wall clock for Starter Vault content, mirroring
  Story 6.1's dependency-injected-`today` design.
- `designate_vault` and `generate_starter_vault` share one designate-then-scan
  body (`designate_vault_impl`) — no second implementation of the
  serialization lock / previous-handle-shutdown / cancel-flag sequencing.
- `has_configured_vault` treats "configured" as: a Vault already designated
  THIS session, OR (LD-40) `GlobalSettings.recent_vaults` non-empty from a
  prior launch. This is a first-launch gate, not a "re-ask every relaunch"
  gate — a returning user with a recorded Vault path never sees the picker,
  even though no story yet auto-reopens that Vault's content on launch (a
  gap this story does not attempt to close — see Design Notes).
- `--org-*` CSS token vocabulary throughout; native `<button>`s (keyboard-
  operable, individually tab-reachable) for every card and the own-folder
  link, matching the `ConflictBanner`/`IndexScanProgress` a11y precedent.
- Match surrounding module-doc/comment density and the colocated-Vitest-test
  pattern used by `VaultPicker`/`IndexScanProgress`/`ConflictBanner`.

**Ask First:**
- Any change to the Story 6.1 `generate_starter_vault(kind, vault_root,
  today)` signature or the `StarterVaultKind` core enum (would ripple into
  the 6.1 inherited tests).
- Adding any external dependency (none needed — `tauri-plugin-dialog` and
  `@tanstack/react-router` are already in the lockfile).

**Never:**
- No Freelancer generator, no wiring the Freelancer card to any command.
- No Empty Starter card (out of scope — Story 11.1, v0.5 Beta, per the epic's
  own note); the "Use my own folder" link is the explicit v0.1 stand-in.
- Do NOT touch `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `/today` mount, no Vault ever configured | `recent_vaults` empty, no session Vault | `hasConfiguredVault()` → `false`; `StarterVaultPicker` renders | a failed query falls back to `true` (never traps a returning user behind onboarding) |
| `/today` mount, a Vault was configured previously | `recent_vaults` non-empty | `hasConfiguredVault()` → `true`; normal `/today` content renders, picker never mounts | N/A |
| Personal GTD / Student card clicked | user picks a folder | `commands.generateStarterVault(kind, path, today)` runs the Story 6.1 generator then designates + scans; `onVaultConfigured()` fires on success | a rejected promise surfaces `role="alert"` text via the shared `errorMessage` extractor; card returns to clickable |
| Folder dialog dismissed (Starter card or own-folder) | dialog returns `null` | no command call; picker/VaultPicker stays as-is | N/A |
| Freelancer card clicked | — | no-op — the `<button>` is `disabled` | N/A |
| "Use my own folder" clicked | — | embeds `<VaultPicker onDesignated={onVaultConfigured} />` inline | VaultPicker's own error path (unchanged) |
| `VaultPicker` designation succeeds (complete OR cancelled) | `designateVault` resolves | `onDesignated?.()` fires — both a completed and a user-cancelled scan leave a valid designated Vault per the LD-42 "cancellable + partial retained" design | N/A |
| `VaultPicker` designation fails | `designateVault` rejects | error shown as before; `onDesignated` is NOT called | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-shell-app/src/lib.rs` -- MODIFY. `designate_vault` command body factored into `designate_vault_impl(path, app, state)`; new `StarterVaultKind` wire enum (camelCase, no `Freelancer`); `resolve_today` helper (mirrors `resolve_planned`'s date-parse + `bad_timestamp` mapping); `generate_starter_vault` command (generate via `orgsidian_core::generate_starter_vault`, then `designate_vault_impl`); `has_configured_vault` command (session vault OR non-empty `GlobalSettings.recent_vaults`); both registered in `build_specta`. Colocated unit tests for `resolve_today`.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- MODIFY. New anchors: `generateStarterVault`, `hasConfiguredVault`, `StarterVaultKind`.
- `shell-ui/src/components/settings/VaultPicker.tsx` -- MODIFY. Export `errorMessage` (was file-local) and add an optional `onDesignated?: () => void` prop, called after a successful `designateVault` (complete or cancelled) — the hook `StarterVaultPicker`'s "Use my own folder" flow needs to notify its parent.
- `shell-ui/src/components/settings/VaultPicker.test.tsx` -- NEW. Colocated tests for the baseline folder-choose/designate/error flow plus the new `onDesignated` callback and the exported `errorMessage` helper (the component had no test file before this story).
- `shell-ui/src/components/onboarding/StarterVaultPicker.tsx` -- NEW. Three primary cards (Personal GTD, Student, disabled Freelancer) + the "Use my own folder" link that reveals the embedded `VaultPicker`. `today` resolved via `localTodayIso()` (imported from `shell-ui/src/components/editor/schedule.ts`, Story 4.8's existing helper).
- `shell-ui/src/components/onboarding/StarterVaultPicker.test.tsx` -- NEW. Cards render + Freelancer disabled/unreachable + dialog-dismissed no-op + successful generate calls `onVaultConfigured` + failed generate surfaces an error + own-folder reveal wires `VaultPicker.onDesignated` through.
- `shell-ui/src/routes/_layout/today.tsx` -- MODIFY. Onboarding gate: on mount, `commands.hasConfiguredVault()` decides between rendering `StarterVaultPicker` (dismissed via `onVaultConfigured`) and the route's existing placeholder content (`VaultPicker` retained there as the post-onboarding manual re-designate entry point).

## Tasks & Acceptance

**Execution:**
- [x] `orgsidian-shell-app`: `designate_vault_impl` extraction, `StarterVaultKind` wire enum, `generate_starter_vault` + `has_configured_vault` commands, `build_specta` + `export_bindings` anchors, colocated `resolve_today` tests.
- [x] `VaultPicker`: exported `errorMessage`, `onDesignated` callback, new colocated test file.
- [x] `StarterVaultPicker.tsx` + colocated tests.
- [x] `/today` onboarding gate wiring.

**Acceptance Criteria:**
- Given Story 6.1, when Orgsidian launches with no configured Vault, then `shell-ui/src/components/onboarding/StarterVaultPicker.tsx` renders three primary options (Personal GTD, Student, Freelancer) plus a secondary "Use my own folder" link that routes to Story 3.6's `designateVault` flow. *(Implemented; Freelancer rendered disabled per the locked scope decision — see below.)*
- And selecting Personal GTD, Student, or Freelancer prompts for a target folder via `tauri-plugin-dialog`, then invokes the generator from Story 6.1. *(Personal GTD + Student: implemented + tested — `commands.generateStarterVault` runs the Story 6.1 generator then designates. Freelancer: NOT wired — disabled, "Coming soon", per the locked scope decision; no generator exists to invoke.)*
- And the "Use my own folder" link prompts for an existing `.org` folder to designate via Story 3.6's `designateVault` flow. *(Implemented — embeds `VaultPicker`, which already owns that flow.)*
- And the picker is dismissed once a Vault is configured and the user lands on the `/today` route. *(Implemented — `onVaultConfigured` flips the `/today` route's onboarding gate; `hasConfiguredVault()` also short-circuits the gate on a later mount if a Vault was configured in a previous session.)*

**Deferred (explicitly out of this story, per the locked scope decision):**
- Freelancer starter generator — blocked on Story 8.7 (BacklinksPanel). Recorded in `deferred-work.md`.
- Auto-reopening a previously-configured Vault's actual content on a fresh app launch — no story yet owns this; `hasConfiguredVault` only prevents the *picker* from re-appearing (see Design Notes).

## Design Notes

- **Why `designate_vault_impl` is factored out.** `generate_starter_vault`'s
  compound "generate then designate" action needs the identical
  designate-then-scan sequencing `designate_vault` already has (the
  serialization lock, the previous-handle shutdown, the cancel-flag
  lifecycle) — duplicating that body would let the two commands silently
  drift. Both now call one shared `async fn designate_vault_impl(path, app,
  state)`.
- **Why `StarterVaultKind` is redeclared in the shell, not derived on the core
  enum.** `orgsidian-core` carries no `specta` dependency (the LEAF/façade
  crate-graph rule keeps IPC concerns at the shell boundary) — the same split
  already used for `PlanningKind` next to the parser's own planning-kind
  type. The shell's `StarterVaultKind` has exactly the two variants the core
  enum has; a Freelancer variant cannot be constructed to reach the command.
- **Why `hasConfiguredVault` checks `recent_vaults`, not just in-session
  state.** `AppState.index` always starts `None` on process launch — nothing
  in the codebase yet re-designates a Vault automatically at startup (that
  capability isn't owned by any shipped story). Gating solely on in-session
  state would show the picker on *every* launch, including for returning
  users who already configured a Vault in a previous session — clearly wrong
  per the AC's "no configured Vault" wording. `GlobalSettings.recent_vaults`
  (LD-40, already populated by `designate_vault`'s `push_recent_vault`) is
  the existing persisted signal closest to "has a Vault ever been
  configured", so `has_configured_vault` ORs it with the session check. This
  does **not** restore the user's actual Vault content on relaunch — that is
  a separate, currently unowned capability (a "reopen last Vault on launch"
  story), flagged below as a residual gap rather than solved here, since
  solving it is out of this story's stated scope (the picker component and
  its wiring).
- **Why the "Use my own folder" link embeds `VaultPicker` rather than
  duplicating its logic.** The AC's "routes to Story 3.6's `designateVault`
  flow" is satisfied by reusing the exact component that flow already lives
  in, extended with one optional callback (`onDesignated`) rather than
  forking a second folder-choose/scan-progress implementation that could
  drift from the original.
- **Post-review hardening: the non-empty-folder guard.** The initial
  implementation let `generate_starter_vault` write straight into whatever
  folder the user picked, silently overwriting any same-named `.org` file
  already there — a real first-launch risk (e.g. pointing the picker at
  Documents) and a violation of the "never silently overwrite user data"
  spirit (NFR-16). `crates/orgsidian-shell-app/src/lib.rs` now runs
  `ensure_target_has_no_org_files(path)` BEFORE calling the Story 6.1
  generator: a shallow, top-level-only `read_dir` (deliberately not a
  recursive walk — that's a pre-flight safety check, not a Vault audit) that
  refuses with `OrgError::Vault` when any top-level `.org` file is found,
  worded to steer the user toward an empty folder or "Use my own folder" for
  an existing Vault. A missing/not-yet-created folder still passes (the
  generator's own `create_dir_all` handles it). The frontend needs no new
  wiring — the rejection already flows through `StarterVaultPicker`'s
  existing `catch` → `errorMessage` → `role="alert"` path. Colocated tests:
  `ensure_target_has_no_org_files_{rejects_a_populated_folder,
  allows_folder_with_non_org_files, allows_empty_existing_folder,
  allows_missing_folder}` (Rust) and "surfaces the non-empty-folder refusal
  via the alert path without generating" (`StarterVaultPicker.test.tsx`). Two
  small a11y fixes landed alongside: the "Setting up…" progress text is now
  `aria-live="polite"` + `aria-busy="true"`, and the disabled Freelancer card
  dropped its native `disabled` attribute (which removed it from the tab
  order) in favor of `aria-disabled="true"` + `aria-describedby` pointing at
  the visible "Coming soon" reason, so it stays keyboard/screen-reader
  discoverable while remaining inert (no `onClick` is wired to it).
  Out of scope, left untouched: the generate/designate lock ordering, any
  route-level `today.tsx` tests, the wire enum/command names/
  `designate_vault_impl` factoring, and `sprint-status.yaml`.
- **Residual gap flagged for the orchestrator (decision-grade, not resolved
  here):** no story yet auto-reopens a previously-configured Vault's content
  on a fresh launch (`AppState` always starts empty; only `recent_vaults`
  persists). `hasConfiguredVault` correctly stops the *onboarding picker*
  from reappearing for a returning user, but that returning user currently
  lands on `/today`'s placeholder content with no active index until they
  re-pick their folder via the `VaultPicker` still mounted there. This falls
  out of Story 6.2's scope (picker UI only) and isn't named as an AC on any
  shipped or currently-drafted story; recording it here rather than silently
  absorbing it into this story's scope.

## Verification

**Commands:**
- `cargo test -p orgsidian-shell-app --offline` -- expected: all green, no 3.6/4.8/5.5 regressions, plus the 2 new `resolve_today` tests.
- `cargo clippy -p orgsidian-shell-app --all-targets --offline -- -D warnings` -- expected: 0 warnings from touched files (parser C-compiler warnings pre-existing).
- `cargo fmt -p orgsidian-shell-app -- --check` -- expected: clean.
- `pnpm --filter shell-ui test` -- expected: all green incl. new `VaultPicker.test.tsx` + `StarterVaultPicker.test.tsx`.
- `pnpm --filter shell-ui build` -- expected: `tsc` + `vite build` clean (regenerates `tauri.ts` via the `prebuild` `cargo test --test export_bindings` step first).

**Result (2026-09-05):** Rust: `cargo test -p orgsidian-shell-app --offline` GREEN (15 lib tests incl. 2 new `resolve_today` tests + the `export_bindings` integration test); `cargo clippy -p orgsidian-shell-app --all-targets --offline -- -D warnings` clean; `cargo fmt -p orgsidian-shell-app -- --check` clean. `Cargo.lock`/`Cargo.toml` untouched — zero new dependencies. Frontend: `pnpm --filter shell-ui test` GREEN — 23 files, 260 tests (14 new: 6 `VaultPicker.test.tsx` + 2 `errorMessage` + 8 `StarterVaultPicker.test.tsx`). `pnpm --filter shell-ui build` GREEN (`tsr generate` → `lingui compile` → `export_bindings` regen → `tsc` → `vite build`).

## Spec Change Log

- 2026-09-05 — Implemented. `orgsidian-shell-app` (`designate_vault_impl` extraction, `StarterVaultKind`, `generate_starter_vault`, `has_configured_vault`, bindings regen), `VaultPicker` (`errorMessage` export + `onDesignated` callback + new test file), `StarterVaultPicker.tsx` (new, + tests), `/today` onboarding gate. Freelancer card rendered disabled ("Coming soon") per the locked scope decision — no generator invoked. All in-scope AC wired and tested; all gates green offline. The "auto-reopen last Vault on launch" gap is flagged in Design Notes as unresolved and out of this story's scope. Status → review.
