---
title: 'Story 7.4 — Implement Custom Agenda view with date range picker'
type: 'feature'
created: '2026-09-13'
status: 'done'
review_loop_iteration: 0
baseline_commit: '8aba8c43132404b0518d1fc2609571cf6b6f9c2b'
context:
  - '{project-root}/_bmad-output/planning-artifacts/epics.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Orgsidian ships Today (`/today`) and Week (`/agenda/week`) agendas, but FR-7 also promises planning over an arbitrary date range. There is no `/agenda/custom` surface, and the frozen `IndexQuery::agenda_custom` (Story 6.5) is still an empty-returning stub.

**Approach:** Fill the frozen `agenda::custom` body with a real Scheduled/Deadline range query plus optional tag / TODO-state / file-path filters (the general case Today/Week are special cases of), expose it over a new typed Tauri command, and add the `/agenda/custom` route + `AgendaCustom` component: a date-range picker and filter inputs whose start/end/tag/todo live in typed URL search params (LD-29), rendering the date-grouped result through `@tanstack/react-virtual` (LD-30) so it scales to 1k+ items.

## Boundaries & Constraints

**Always:** Reuse the frozen `IndexQuery` surface — `agenda::custom`'s change is a pure BODY edit of the already-frozen signature (semver-MINOR, per the query mod docs), never a signature change. Follow the existing agenda conventions: caller-supplied `YYYY-MM-DD` local dates (never a server clock read), backend does all filtering + the stable `agenda_date` sort, frontend only partitions/renders (no client re-sort or second fetch). Exclude DONE items and quarantined files, mirroring `today`/`week`. Match the `--org-*` token vocabulary and the `AgendaWeek` row/Link patterns. Pin new deps to latest stable.

**Ask First:** Any change to the `IndexQuery` trait signature, `CustomAgendaQuery`'s existing fields, or `AgendaItem`. Adding a Tauri command beyond `agenda_custom`.

**Never:** Do not modify `sprint-status.yaml`. Do not touch `agenda::today`/`agenda::week` bodies or signatures. Do not add recurring-timestamp expansion (deferred). Do not put the file-path filter into a URL search param — the AC enumerates only `?start=`, `?end=`, `?tag=`, `?todo=`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Scheduled in range | headline SCHEDULED within `[start,end]` | one row, grouped under its scheduled date | N/A |
| Deadline in range | DEADLINE within `[start,end]`, not done | row grouped under its deadline date, `overdue=false` | N/A |
| Overdue deadline | DEADLINE `< start` | row included, collapsed onto `start`, `overdue=true` | N/A |
| Out-of-range | Scheduled/Deadline outside `[start,end]` and not overdue | excluded | N/A |
| DONE / quarantined | todo_done=1, or quarantined file | excluded | N/A |
| tag filter | `tag=@home` | only headlines carrying that tag | N/A |
| todo filter | `todo=NEXT` | only headlines with that TODO keyword | N/A |
| file-path filter | glob e.g. `projects*` | only rows from matching `files.path` | N/A |
| no active vault | command called, no vault | reject with `OrgError::Vault` | surfaced as `role="alert"` |
| 1k+ items | large result | virtualized render, no full DOM mount | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-index/src/query/agenda.rs` -- `custom()` stub to implement (body only); `CustomAgendaQuery` fields already frozen (`start_date`,`end_date`,`tag`,`todo_state`,`file_path_glob`); model the SELECT on `week()` (two legs + `agenda_date` derivation + stable sort). Add unit tests beside the `week_*` tests.
- `crates/orgsidian-index/src/query/mod.rs` -- `IndexQuery::agenda_custom` default body already wires to `agenda::custom`; DO NOT edit signatures.
- `crates/orgsidian-index/migrations/0001_initial-schema.sql` -- `tags(headline_id,tag,position)`, `headlines.todo_keyword/todo_done`, `files.path/quarantined`; index `idx_tags_tag_headline_id` supports the tag EXISTS subquery.
- `crates/orgsidian-core/src/index/mod.rs` -- add `agenda_custom(vault_root, &CustomAgendaQuery)` mirroring `agenda_week` (fresh `IndexPool`, index-absent guard); re-export `CustomAgendaQuery`.
- `crates/orgsidian-core/src/lib.rs` -- add `agenda_custom`, `CustomAgendaQuery` to the `index::` re-export line.
- `crates/orgsidian-shell-app/src/lib.rs` -- add `CustomAgendaQueryDto` (`#[serde(rename_all="camelCase")]`, specta::Type) + `agenda_custom` command (maps DTO→core query, `AgendaItemDto::from`); register in `collect_commands!`.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- add anchors `agendaCustom`, `CustomAgendaQueryDto`.
- `shell-ui/src/components/agenda/AgendaWeek.tsx` / `AgendaToday.tsx` -- reuse `groupByDate`/`errorMessage`/`deadlineLabel` patterns; add a "View custom" cross-link.
- `shell-ui/src/routes/agenda/week.tsx` -- route pattern to mirror for `routes/agenda/custom.tsx`.
- `shell-ui/src/routes/editor/$filePath/$headlineId.tsx` -- `validateSearch` pattern for typed search params.
- `shell-ui/src/components/agenda/AgendaWeek.test.tsx` -- test harness pattern (mock `@/lib/tauri`, memory router).
- `shell-ui/package.json` -- add `@tanstack/react-virtual@^3.14.12` (React 19 peer OK).

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-index/src/query/agenda.rs` -- implement `custom()` body: SELECT over `[start,end]` with optional `(?N IS NULL OR cond)` filters for todo/file-glob and a tag EXISTS subquery; derive `agenda_date` + stable sort exactly as `week`. Add `custom_*` unit tests for every I/O row.
- [x] `crates/orgsidian-core/src/index/mod.rs` + `lib.rs` -- add `agenda_custom` wrapper + re-export `CustomAgendaQuery`.
- [x] `crates/orgsidian-shell-app/src/lib.rs` -- add `CustomAgendaQueryDto` + `agenda_custom` command; register in `collect_commands!`; add a DTO→query mapping unit test.
- [x] `crates/orgsidian-shell-app/tests/export_bindings.rs` -- add `agendaCustom` + `CustomAgendaQueryDto` anchors; regenerate `shell-ui/src/lib/tauri.ts`.
- [x] `shell-ui/src/components/agenda/AgendaCustom.tsx` -- filter bar (start/end date inputs + tag/todo/file-path inputs) + `@tanstack/react-virtual` date-grouped list; URL search params drive start/end/tag/todo, file-path is local state.
- [x] `shell-ui/src/routes/agenda/custom.tsx` -- route with `validateSearch` for `?start=/?end=/?tag=/?todo=`, rendering `AgendaCustom`.
- [x] `shell-ui/src/components/agenda/AgendaCustom.test.tsx` -- cover loading/error/grouping/virtualization/search-param wiring.
- [x] `shell-ui/src/components/agenda/AgendaWeek.tsx` (+`AgendaToday.tsx`) -- add "View custom" cross-link.
- [x] `shell-ui/package.json` -- add `@tanstack/react-virtual`.

**Acceptance Criteria:**
- Given a Vault with Scheduled/Deadline headlines, when `/agenda/custom?start=A&end=B` renders, then only in-range (or overdue) non-DONE, non-quarantined items appear, grouped by date, virtualized.
- Given tag/todo/file-path filter inputs, when applied, then the result is narrowed accordingly; start/end/tag/todo are reflected in the URL and deep-linkable (typed search params, LD-29).
- Given no active Vault, when the view loads, then a `role="alert"` error is shown rather than a crash.
- Given the `IndexQuery` freeze, when `agenda_custom` lands, then only bodies change — `cargo-semver-checks` sees no API change.

## Design Notes

Optional filters stay a single prepared statement via the `(?N IS NULL OR <cond>)` idiom (rusqlite binds `Option<String>` to NULL), so no dynamic SQL string building. Tag filter is an `EXISTS (SELECT 1 FROM tags t WHERE t.headline_id = h.id AND t.tag = ?)` subquery (hits `idx_tags_tag_headline_id`). File-path filter uses SQLite `GLOB`. `agenda_date` derivation copies `week` verbatim (in-window Scheduled wins; else non-overdue Deadline day; else collapse to `start`), and the same trailing stable `sort_by(agenda_date)`.

Virtualization: flatten groups to `Array<{kind:'header',dateIso} | {kind:'item',item}>` and drive one `useVirtualizer({count, getScrollElement, estimateSize, overscan})` over a scroll container; position rows absolutely with `measureElement` for variable heights. Only days that have items get a header (a custom range can span months — unlike Week's fixed 7 empty-day headers).

Default range when params absent: `start = today`, `end = today + 29 days` (a 30-day window — "beyond Week"). File-path is intentionally NOT a URL param (AC lists only four).

## Verification

**Commands:**
- `cargo test -p orgsidian-index query::agenda` -- expected: new `custom_*` tests pass
- `cargo test -p orgsidian-core` -- expected: pass (index façade)
- `cargo test -p orgsidian-shell-app --test export_bindings` -- expected: pass, regenerates `tauri.ts` with `agendaCustom`
- `cargo build -p orgsidian-shell-app` -- expected: compiles
- `pnpm --filter shell-ui exec tsc --noEmit` -- expected: no type errors
- `pnpm --filter shell-ui test` -- expected: AgendaCustom + existing suites green

## Suggested Review Order

**Query core (backend behavior)**

- Entry point: the general range+filter query — inverted-range guard, two date legs, optional filters, `agenda_date` derivation + stable sort (mirrors `week`).
  [`agenda.rs:308`](../../crates/orgsidian-index/src/query/agenda.rs#L308)

- The frozen parameter struct whose fields this story finally consumes (signature unchanged — body-only edit, semver-MINOR).
  [`agenda.rs:260`](../../crates/orgsidian-index/src/query/agenda.rs#L260)

**IPC boundary**

- The core async façade wrapper (fresh pool, index-absent guard), mirroring `agenda_week`.
  [`mod.rs:317`](../../crates/orgsidian-core/src/index/mod.rs#L317)

- The `#[tauri::command]` + its camelCase DTO and the `#[non_exhaustive]`-safe DTO→query mapping.
  [`lib.rs:791`](../../crates/orgsidian-shell-app/src/lib.rs#L791)

**Frontend surface**

- The component: resolves the range, drives `agendaCustom`, groups by date; note real-today used for deadline labels, file-path kept local.
  [`AgendaCustom.tsx:130`](../../shell-ui/src/components/agenda/AgendaCustom.tsx#L130)

- The LD-30 virtualizer over the flattened header/item row list.
  [`AgendaCustom.tsx:316`](../../shell-ui/src/components/agenda/AgendaCustom.tsx#L316)

- The typed route (LD-29): `?start/?end/?tag/?todo` validation, rejecting impossible calendar dates.
  [`custom.tsx:37`](../../shell-ui/src/routes/agenda/custom.tsx#L37)

**Supporting**

- Rust unit tests covering every I/O-matrix row (range bounds, overdue collapse, filters, inverted range).
  [`agenda.rs:530`](../../crates/orgsidian-index/src/query/agenda.rs#L530)

- Component tests (mock `@/lib/tauri` + `useVirtualizer`): grouping, search-param wiring, default-range width, Apply.
  [`AgendaCustom.test.tsx:1`](../../shell-ui/src/components/agenda/AgendaCustom.test.tsx#L1)

- Bindings-export anchors guarding the new command + DTO surface.
  [`export_bindings.rs:1`](../../crates/orgsidian-shell-app/tests/export_bindings.rs#L1)
