---
title: 'Story 7.5 — Implement saved Agenda filter presets'
type: 'feature'
created: '2026-09-14'
status: 'done'
review_loop_iteration: 0
baseline_commit: '099e8c18cc028c0702d2891bfc60de4c384be376'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
  - '{project-root}/CONTRIBUTING.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** FR-7 is incomplete: the Custom Agenda (Story 7.4) has no way to save a recurring query as a named preset. Users must re-enter filters every time, and there are no out-of-the-box "what did I finish?" views. The per-Vault settings store already reserves `[agenda_presets]` but its `AgendaPreset` shape and all UI/query wiring are unbuilt.

**Approach:** Finalize the `AgendaPreset` schema; add core load/save/delete preset functions over the existing TOML settings store; expose them as Tauri commands; add an Agenda sidebar (in the `/agenda/custom` route) that lists, applies, saves, and deletes presets. Ship two default presets (`Done This Week`, `Done This Month`) seeded idempotently on first launch, backed by a NEW completion-date query mode on `agenda_custom` (filter `todo:DONE` by `CLOSED:` date). Extend the starter vaults so both defaults are non-empty day-one.

## Boundaries & Constraints

**Always:**
- Presets persist in `<Vault>/.orgsidian/settings.toml` under `[agenda_presets]` via `read_vault_settings`/`write_vault_settings` (LD-40; supersedes any `agenda-presets.json`). Name is the map key.
- Completion query is a BODY edit of `agenda::custom` + a new `#[non_exhaustive]` field on `CustomAgendaQuery` (both semver-minor under the Story 6.5 freeze). Do NOT change `IndexQuery` trait signatures. All date queries stay in `orgsidian-index::query::agenda`.
- Defaults are seeded once (tracked by a `agenda_presets_seeded` flag) — never re-seeded after the user deletes them.
- Modules implementing FR-7 carry `//! Implements FR-7 …`. New Tauri commands are registered in `collect_commands!` AND anchored in `tests/export_bindings.rs`; DTOs use explicit `#[serde(rename_all = "camelCase")]`, `i64→u32` narrowing at the boundary, no `unwrap`/`panic!` in command bodies.
- Story 7.4's `AgendaCustom` component keeps its existing behavior and its tests passing; new capability arrives via optional, backward-compatible props.
- `cargo fmt --all -- --check` clean before commit; clippy `-D warnings`; `--locked`.

**Never:**
- No new SQLite migration / `closed_date` index in this story (would cascade into `EXPECTED_USER_VERSION`/LD-13 drift + many tests; a `closed_date` range scan is sub-ms at v0.1 scale — deferred optimization, matching the scheduled/deadline-index rationale in `0001`). Leave a code note.
- No new route; `routeTree.gen.ts` is auto-generated. No `sprint-status.yaml` edits. No `window.prompt`.
- Do not promote file-path/`completed` to URL search params (Story 7.4 fixed the four typed params; these stay component-local).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Seed on first list | vault with `agenda_presets_seeded=false` | `list` inserts `Done This Week`+`Done This Month` (only if name absent), sets flag true, persists, returns them | settings read/write err → `OrgError::Io` |
| Idempotent re-seed | user deleted a default, flag already true | `list` returns remaining presets; deleted default NOT resurrected | — |
| Save preset | name + view + filters | upserted into `agenda_presets`, written atomically | — |
| Delete preset | existing name | removed and written; absent name is a no-op `Ok` | — |
| Completion query | `completed_in_range=true`, `[start,end]`, `todo_state="DONE"` | DONE headlines whose `closed_date` ∈ `[start,end]`, grouped by `closed_date`; optional tag/glob applied | inverted range → empty |
| Rolling recall | preset `rolling_days=7` | window resolved to `[today-6, today]` at click time | — |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-core/src/settings/schema.rs` — finalize placeholder `AgendaPreset` (view + `start/end/rolling_days/tag/todo_state/file_path_glob: Option`, `completed: bool`; all `#[serde(default)]`, keep `specta::Type`); ADD `agenda_presets_seeded: bool` to `VaultSettings` (`#[serde(default)]`). Keep `SCHEMA_VERSION_CURRENT = 1` (no shipped data). Schema-lock header applies — extend, don't rename `agenda_presets`.
- `crates/orgsidian-core/src/agenda_presets.rs` (NEW) — `//! Implements FR-7`. `DONE_THIS_WEEK`/`DONE_THIS_MONTH` consts, `default_agenda_presets()`, `list_agenda_presets(vault)` (seed-if-unseeded + persist), `save_agenda_preset(vault,name,preset)`, `delete_agenda_preset(vault,name)`. Map `SettingsError`→`OrgError::Io` (no `From` exists; mirror `coaching_io`). Model on `coaching.rs`.
- `crates/orgsidian-core/src/lib.rs` — add `pub mod agenda_presets;` + `pub use agenda_presets::{…}` and re-export `settings::schema::AgendaPreset`.
- `crates/orgsidian-index/src/query/agenda.rs` — `custom()`: add early branch to a completion query when the new flag is set (SELECT `…, h.closed_date` WHERE `closed_date IS NOT NULL AND closed_date BETWEEN ?1 AND ?2` + `(?N IS NULL OR …)` todo/glob/tag filters; `agenda_date = closed_date`, `overdue=false`; `ORDER BY h.closed_date, f.path, h.position`). ADD `completed_in_range: bool` to `#[non_exhaustive] CustomAgendaQuery`. Code-note the deferred `closed_date` index. Add `custom_completed_*` unit tests.
- `crates/orgsidian-shell-app/src/lib.rs` — ADD `completed_in_range` to `CustomAgendaQueryDto` (+ mapping); NEW `AgendaPresetDto` (camelCase, `name`+fields), `list_agenda_presets`/`save_agenda_preset`/`delete_agenda_preset` commands (resolve `state.current_vault_root().ok_or_else(no_active_vault)?`, model on `dismiss_coaching`); register all in `collect_commands!`.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` — anchors: `listAgendaPresets`, `saveAgendaPreset`, `deleteAgendaPreset`, `AgendaPresetDto`, `completedInRange`. Regenerate `shell-ui/src/lib/tauri.ts`.
- `crates/orgsidian-core/src/starter_vault/personal_gtd.rs` & `student.rs` — add ≥1 more `DONE` headline with a recent `inactive_timestamp` (CLOSED) so each vault has ≥2 DONE within the rolling-7 range; update the in-file invariant tests accordingly (keep byte-for-byte determinism + parse checks).
- `crates/orgsidian-core/tests/settings_round_trip.rs` — update the `agenda_presets` block (lines ~55-62) to the finalized `AgendaPreset` shape.
- `shell-ui/src/components/agenda/AgendaCustom.tsx` — add local `completed` state + a labeled checkbox in the filter form; pass `completedInRange` in the `agendaCustom` query; add optional props `onAppliedChange?(snapshot)` and `presetApply?: {nonce,completed,filePathGlob}` (nonce-keyed effect syncs completed+filePath drafts/applied). URL contract for start/end/tag/todo unchanged.
- `shell-ui/src/components/agenda/AgendaPresetSidebar.tsx` (NEW) — lists presets (`commands.listAgendaPresets`), apply-on-click, inline "save current as…" (name input + button, NOT a `<form>`), delete via right-click context menu AND a keyboard-reachable options button (a11y gate). Reuse `errorMessage` from `AgendaToday`.
- `shell-ui/src/routes/agenda/custom.tsx` — render `<aside><AgendaPresetSidebar/></aside>` beside `AgendaCustom`; own `applied` snapshot + `presetApply` nonce state; `applyPreset` resolves rolling→absolute window (reuse `localTodayIso`/`addDaysIso`), `navigate({search})`, bumps `presetApply`.
- `shell-ui/src/components/agenda/AgendaCustom.test.tsx` — add `completedInRange` tolerance is automatic; add coverage for the checkbox + preset-apply override + `onAppliedChange`.
- `shell-ui/src/components/agenda/AgendaPresetSidebar.test.tsx` (NEW) — list/apply/save/delete + no-vault error, mocking `@/lib/tauri`.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-core/src/settings/schema.rs` — finalize `AgendaPreset`; add `agenda_presets_seeded`.
- [x] `crates/orgsidian-core/src/agenda_presets.rs` (+ `lib.rs`) — preset CRUD + idempotent default seeding; unit tests (seed once, no-resurrect-after-delete, save/delete round-trip).
- [x] `crates/orgsidian-index/src/query/agenda.rs` — `completed_in_range` field + completion branch; `custom_completed_*` unit tests per I/O row.
- [x] `crates/orgsidian-shell-app/src/lib.rs` — DTO field + `AgendaPresetDto` + three commands; register; DTO↔core mapping unit test.
- [x] `crates/orgsidian-shell-app/tests/export_bindings.rs` — anchors; regenerate `tauri.ts`.
- [x] `crates/orgsidian-core/src/starter_vault/{personal_gtd,student}.rs` — ≥2 in-range DONE per vault; update invariant tests.
- [x] `crates/orgsidian-core/tests/settings_round_trip.rs` — new preset shape.
- [x] `shell-ui/src/components/agenda/AgendaCustom.tsx` — completed checkbox + preset props.
- [x] `shell-ui/src/components/agenda/AgendaPresetSidebar.tsx` (NEW) + test.
- [x] `shell-ui/src/routes/agenda/custom.tsx` — sidebar layout + apply/rolling resolution.

**Acceptance Criteria:**
- Given the Custom Agenda, when the user saves a named preset, then it appears in the Agenda sidebar and persists to `settings.toml` under `[agenda_presets]`.
- Given a saved preset, when clicked, then the view + filters (date window, tag, todo, file-path, completed mode) are restored.
- Given a preset row, when the context menu Delete is used, then the preset is removed from the sidebar and `settings.toml`.
- Given a fresh Vault, when the sidebar first loads, then `Done This Week` (rolling 7) and `Done This Month` (rolling 30) exist and each returns ≥1 result from starter fixtures; deleting a default and reloading does NOT resurrect it.
- Given the `IndexQuery` freeze, when the completion mode lands, then only bodies/`#[non_exhaustive]` fields change — no trait-signature change.

## Spec Change Log

- 2026-09-14 — Step-04 multi-lens review (blind / edge-case / verification-gap). No intent_gap or bad_spec (intent clear, no spec deviation) → no loopback. Patches applied: (P) extracted `presetToRecall` pure mapping + unit tests, closing the untested `applyPreset` field-mapping gap; (A) `save_agenda_preset` no longer sets `agenda_presets_seeded` so a save-before-list still seeds defaults (+ test); (G) starter-vault CLOSED-window assertions pinned to the preset's exact `[today-6, today]` window; (J/L) preset context menu gains Escape + outside-pointer dismissal and the error alert uses `text-destructive`; (Q) export-bindings anchors add `rollingDays`/`todoState`/`filePathGlob`; (R) corrected the `nonce` rationale. Rejected as out-of-v0.1-scope / by-design: delete & overwrite confirmations, reset-to-defaults affordance, active-preset highlight/ordering (UX polish); old-`filters` migration (schema was a never-written placeholder — confirmed safe); reopened-task stale-CLOSED (defaults pin `todo:DONE`; org removes CLOSED on reopen); read-time seed write & `presets_io` mapping (match established `settings`/`coaching` conventions); backend empty-name guard (UI-guarded, no fitting `OrgError` variant).

## Design Notes

Completion mode reuses the whole 7.4 render path: setting `agenda_date = closed_date` means the frontend groups completed items by their close date with zero UI changes. `todo:DONE` + completion range = `closed_date BETWEEN` (only DONE items carry `CLOSED:`) plus the existing `(?N IS NULL OR todo_keyword=?)` filter.

Default preset value (illustrative): `{ view:"custom", rolling_days:Some(7), todo_state:Some("DONE"), completed:true }`. Rolling windows resolve at click time so "this week/month" always means the last N days; absolute `start`/`end` presets pin a fixed window.

Sidebar/`AgendaCustom` split avoids Story 7.4 test-selector collisions: the sidebar lives in the route (not in `AgendaCustom`), so `AgendaCustom`'s single `<form>` and `<h2>` date headers are untouched. `AgendaCustom` reports applied filters via `onAppliedChange` (for "save current") and receives preset applies via a nonce-keyed `presetApply` prop (for "recall") — both optional, so 7.4 tests that omit them see identical behavior.

## Verification

**Commands:**
- `cargo fmt --all -- --check` — clean.
- `cargo test -p orgsidian-core -p orgsidian-index -p orgsidian-shell-app --locked` — green (preset CRUD, completion query, starter-vault invariants, round-trip, DTO mapping, bindings export).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — no warnings.
- `pnpm --filter shell-ui exec tsc --noEmit` — no type errors.
- `pnpm --filter shell-ui exec vitest run` — sidebar + AgendaCustom suites green.
