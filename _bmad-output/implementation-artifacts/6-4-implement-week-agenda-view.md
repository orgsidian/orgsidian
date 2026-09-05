---
title: 'Implement Week Agenda view'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: 'c074ffa'
review_loop_iteration: 0
github_issue: 55
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 6.3 shipped `/today` — a single-day Agenda. The user has no way to see anything beyond today, so planning ahead ("what does my week look like") is impossible without opening every file by hand.

**Approach:** Add the second `orgsidian-index` read query, `query::agenda::week(conn, start_date)` — the same two legs `today()` established (Scheduled matching; Deadline overdue-or-matching), widened from a single day to the caller-supplied rolling 7-day window `[start_date, start_date + 6 days]` inclusive. The `+6 days` arithmetic runs via SQLite's built-in `date()` scalar function (never `chrono` — `orgsidian-index` is a LEAF crate and the project keeps `chrono`/`time` out of leaves, per Story 1.12 Dev Notes §11). `AgendaItem` gains one new field, `agenda_date` (the calendar day a row groups under — trivially `today` itself for `today()`; for `week()`, the Scheduled date when the Scheduled leg matched, else the Deadline date, collapsed to `start_date` when that Deadline is overdue, mirroring `today()`'s own overdue-collapses-to-today rule). `week()` stable-sorts its rows by `agenda_date` in Rust (ties keep the SQL fetch's `(file_path, position)` order) so the frontend's "grouped by date" AC is a stable partition of an already-sorted list, exactly the "backend sorts, frontend never re-sorts" convention `today()` established for per-file grouping. `orgsidian-core` wraps it as `agenda_week(vault_root, start_date)`, mirroring `agenda_today`'s shape. `orgsidian-shell-app` adds the `agendaWeek` command, reusing the existing `AgendaItemDto` (extended with the new `agendaDate` camelCase field) rather than inventing a parallel DTO. Frontend: `AgendaWeek.tsx` queries once per mount, renders all 7 window days (even empty ones, so the user sees which days are free), highlights the current day, and renders a click-to-open `Link` per item to the existing `/editor/$filePath/$headlineId` route — reusing `AgendaToday`'s `deadlineLabel`/`errorMessage` helpers rather than duplicating them. A new `/agenda/week` route hosts it; a small "View week"/"Back to Today" `Link` pair wires the view-switch the perf AC measures. The perf-AC ("<200ms view-switch on a 1000-file Vault") is gated the same way Story 6.3's was: a Story 1.12 `assert_no_perf_regression!` test benchmarking `agenda::week` directly (the query is the whole cost of the render; `AgendaWeek.tsx` does no further backend round-trip), with a committed baseline.

## Boundaries & Constraints

**Always:**
- `start_date` crosses every boundary (SQL param → core fn → IPC command → frontend `localTodayIso()`) as a plain `YYYY-MM-DD` string supplied by the CALLER — never a `chrono::Local::now()` / server-side clock read (same convention `today()`/`set_scheduled` established).
- The `+6 days` window-end arithmetic runs in SQLite (`date(?1, '+6 days')`), never in Rust — `orgsidian-index` stays `chrono`-free (LEAF crate discipline, Story 1.12 Dev Notes §11).
- The query result is already sorted by `agenda_date` (ties by `(file_path, position)`); `AgendaWeek.tsx` partitions that order into per-date groups and MUST NOT re-sort.
- `AgendaItem`/`AgendaItemDto` are extended (one new `agenda_date`/`agendaDate` field), never duplicated into a parallel Week-specific type; `today()`'s existing behavior/shape stays otherwise unchanged (its `agenda_date` is trivially the caller's `today`).
- camelCase IPC wire via the established manual-rename precedent (`AgendaItemDto` already carries `#[serde(rename_all = "camelCase")]` from Story 6.3).
- Match the LEAF crate graph: `orgsidian-shell-app` never imports `orgsidian-index` directly; `agenda_week` is reached only through `orgsidian-core`'s façade.
- `--org-*` CSS token vocabulary in `AgendaWeek.tsx` (no new tokens invented); overdue is marked with TEXT ("Overdue"/"Due"/"Due today"), never color alone (LD-58).
- Match surrounding module-doc/comment density and trace headers (`//! Implements FR-7 (...)` / `// Implements FR-7`) on every touched module.

**Ask First:**
- Any change to the Story 6.5 `IndexQuery` trait shape (this story ships only the plain `agenda::week` function the trait will later wrap, not the trait itself).
- Adding any new external dependency beyond the warmed lockfile (none needed — `chrono` stays out of `orgsidian-index`; SQLite's `date()` does the one arithmetic step).

**Never:**
- No Today Dashboard / Custom-range features (Epic 7 / Story 7.4) — this story is the Week window only.
- No recurring-timestamp (`+1w` repeater) expansion (same "NOT MODELLED IN v1" deferred note `today()` carries).
- Do NOT touch `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Scheduled on the window's start day | `scheduled_date == start_date` | Included, `agenda_date = start_date` | N/A |
| Scheduled on the window's end day (`start_date + 6`) | boundary | Included, `agenda_date = end_date` | N/A |
| Scheduled the day before/after the window | outside `[start, end]` | Excluded | N/A |
| Deadline before `start_date` (overdue) | `deadline_date < start_date` | Included, `overdue=true`, `agenda_date = start_date` (collapses onto the current day) | N/A |
| Deadline within the window, not overdue | `start_date <= deadline_date <= end_date` | Included, `overdue=false`, `agenda_date = deadline_date` | N/A |
| Deadline after the window | `deadline_date > end_date` | Excluded | N/A |
| Row carries both a Scheduled date (in-window) and an unrelated overdue Deadline | both legs present | Groups under the Scheduled date (Scheduled leg wins for grouping); `overdue` still reflects the stale Deadline independent of grouping | N/A |
| Headline marked DONE | `todo_done = 1` | Excluded even if Scheduled/Deadline matches | N/A |
| File quarantined (LD-41) | `files.quarantined = 1` | Its headlines excluded entirely | N/A |
| Multiple dates/files | N dates, M headlines each | One flat list ordered by `agenda_date` (ties by `file_path`/`position`); frontend groups into 7 date sections (including empty ones) in window order, no re-sort | N/A |
| No active Vault | `AppState.index` empty | `agenda_week` command → `OrgError::Vault` | frontend renders the message via `role="alert"` |
| No index for the resolved Vault root | DB file absent | `OrgError::Index` | same alert path |
| A window day has no items | — | That day's section still renders, with "Nothing scheduled or due." | N/A |
| Click an item | any `AgendaItemDto` | `Link` navigates to `/editor/$filePath/$headlineId?byteStart=<n>` (same route Story 6.3 built) | `$filePath` percent-encoded so an embedded `/` survives as one segment |
| View-switch `/today` → `/agenda/week` | click "View week" | Route renders `AgendaWeek`, query completes well under the 200ms budget on a 1000-file Vault | N/A |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-index/src/query/agenda.rs` -- MODIFY. `AgendaItem` gains `pub agenda_date: String`; `today()` sets it to the caller's `today` verbatim; new `week(conn, start_date) -> Result<Vec<AgendaItem>, IndexError>` (SQLite `date(?1, '+6 days')` for the window end; Scheduled-in-window OR Deadline-overdue-or-in-window; Rust-side stable sort by `agenda_date`). Colocated unit tests (window boundaries, overdue-collapse, in-window-deadline-own-day, DONE/quarantine exclusion, Scheduled-leg-wins-over-unrelated-overdue-deadline, date-then-file-then-position ordering).
- `crates/orgsidian-index/src/query/mod.rs` -- MODIFY (doc only). Mentions `agenda::week` alongside `agenda::today` as the two functions Story 6.5 wraps.
- `crates/orgsidian-index/src/lib.rs` -- MODIFY (doc only). Crate-doc sentence updated.
- `crates/orgsidian-core/src/index/mod.rs` -- MODIFY. `agenda_week(vault_root, start_date)` mirroring `agenda_today`'s resolve→refuse-if-absent→fresh-`IndexPool` shape.
- `crates/orgsidian-core/src/lib.rs` -- MODIFY. Re-export `agenda_week` alongside the existing `agenda_today, AgendaItem`.
- `crates/orgsidian-core/Cargo.toml` -- MODIFY. New `[[test]]` block for the Story 6.4 perf-AC test (`required-features = ["test-support"]`, same reason `story_6_3_agenda_today_perf` needs it).
- `crates/orgsidian-core/tests/story_6_4_agenda_week_perf.rs` -- NEW. Story 1.12 `assert_no_perf_regression!` gate over the same synthetic 1000-file/5-headline-per-file in-memory index Story 6.3's gate uses, benchmarking `agenda::week` directly; committed baseline at `tests/perf-baselines/story-6.4-agenda-week.json`.
- `tests/perf-baselines/story-6.4-agenda-week.json` -- NEW. First-run baseline (macos-aarch64: ~2.1ms median — comfortably inside the 200ms absolute NFR).
- `docs/perf/targets.md` -- MODIFY. New row: "Week Agenda view-switch (`/today` → `/agenda/week`)" <200ms / 1000-file Vault / Story 6.4.
- `crates/orgsidian-shell-app/src/lib.rs` -- MODIFY. `AgendaItemDto` gains `agenda_date` (camelCase `agendaDate`); `From<orgsidian_core::AgendaItem>` updated; new `agenda_week` command (Vault-scoped via `state.current_vault_root()`); registered in `build_specta`. Existing `agenda_item_dto_projects_every_field` test extended for the new field.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- MODIFY. Anchor added for `agendaWeek`.
- `shell-ui/src/components/agenda/AgendaWeek.tsx` -- NEW. `// Implements FR-7`. Queries `commands.agendaWeek(localTodayIso())` once per mount; loading/error states; renders all 7 window days (computed via a local-time-safe `addDaysIso` helper), current day highlighted; groups the already-sorted result by `agendaDate`; renders a `Link` per item, reusing `AgendaToday`'s exported `deadlineLabel`/`errorMessage` helpers.
- `shell-ui/src/components/agenda/AgendaWeek.test.tsx` -- NEW. Loading / error / all-7-days-rendered-with-current-day-marked / grouping-order / Link-href (incl. slash-in-path percent-encoding) / overdue-vs-due labeling / single-query-per-mount.
- `shell-ui/src/components/agenda/AgendaToday.tsx` -- MODIFY. Exports `deadlineLabel`/`errorMessage` for reuse (were module-private); adds a "View week" `Link` to `/agenda/week` next to the heading (the view-switch the perf AC measures).
- `shell-ui/src/components/agenda/AgendaToday.test.tsx` -- MODIFY. Test router gains the `/agenda/week` route (for the new Link to resolve); the two click-to-open Link assertions are now selector-scoped to `a[href^="/editor/"]` since the header's new "View week" Link is also an `<a>`.
- `shell-ui/src/routes/agenda/week.tsx` -- NEW. `createFileRoute("/agenda/week")` rendering `<AgendaWeek />`.

## Tasks & Acceptance

**Execution:**
- [x] `orgsidian-index`: `AgendaItem::agenda_date` + `query::agenda::week` + colocated unit tests.
- [x] `orgsidian-core`: `agenda_week` façade function + re-export.
- [x] `orgsidian-shell-app`: `AgendaItemDto::agenda_date` + `agenda_week` command + `build_specta`/`export_bindings` wiring + extended colocated test.
- [x] Perf-AC: synthetic-1000-file `assert_no_perf_regression!` test + committed baseline + `docs/perf/targets.md` row.
- [x] Frontend: `AgendaWeek.tsx` + colocated tests; `/agenda/week` route; `AgendaToday.tsx` view-switch Link + exported helpers.

**Acceptance Criteria:**
- Given Story 6.3, when `/agenda/week` renders, then `shell-ui/src/components/agenda/AgendaWeek.tsx` queries `orgsidian-index::query::agenda::week(start_date)`. *(Wired end-to-end: `AgendaWeek` → `commands.agendaWeek` → `orgsidian_core::agenda_week` → `orgsidian_index::query::agenda::week`; tested at every layer.)*
- And items are grouped by date, with the current day highlighted. *(Backend query is stable-sorted by `agenda_date`; `groupByDate` partitions without re-sorting; all 7 window days render — including empty ones — with the current day visually marked and labeled "(Today)"; tested with an interleaved-insertion-order fixture proving the sort, not insertion order, drives grouping.)*
- And view-switching from `/today` to `/agenda/week` completes in <200ms on a 1000-file Vault. *(Story 1.12 `assert_no_perf_regression!` gate committed with a first-run baseline of ~2.1ms median on macos-aarch64 — two orders of magnitude under budget; the absolute target is additionally recorded in `docs/perf/targets.md`. A "View week"/"Back to Today" `Link` pair wires the actual view-switch interaction the AC names.)*

## Design Notes

- **Why `agenda_date` is a new field on the shared `AgendaItem`, not a Week-only type.** The AC's "grouped by date" needs a single, unambiguous per-row grouping key computed once, server-side — the same architectural stance `overdue` already established (backend computes semantics, frontend only displays). Reusing `AgendaItem`/`AgendaItemDto` (one additive field) instead of inventing a parallel `WeekAgendaItem` avoids duplicating the whole Scheduled/Deadline/overdue shape for a one-field difference; `today()`'s own `agenda_date` is a trivial one-liner (`today.to_string()` for every row), so the addition costs that call site nothing.
- **Why `week`'s date-range arithmetic runs in SQLite, not Rust.** `orgsidian-index` is a LEAF crate; the project's established discipline (Story 1.12 Dev Notes §11, `orgsidian-core::test_support::perf`'s hand-rolled `current_iso8601_utc`) keeps `chrono`/`time` out of leaves. SQLite ships `date(?1, '+6 days')` as a built-in scalar function, so the query computes its own window end without pulling in a date-arithmetic crate — the same reasoning Story 3.4 used for `applied_at` via `strftime` instead of `chrono`.
- **Why the per-row grouping precedence is "Scheduled wins, Deadline falls back, overdue collapses to `start_date`".** A headline can carry both a Scheduled and a Deadline timestamp; when both legs would otherwise match (e.g. Scheduled mid-week AND an unrelated stale Deadline), the Scheduled date is the more specific planning signal — the user placed it there deliberately — so it wins for *grouping*. `overdue` is computed independently of which leg matched (mirrors `today()`), so a row can simultaneously show "grouped under day 3" and "overdue" if its Deadline happens to be stale; that combination is exercised by a dedicated test (`week_scheduled_leg_wins_grouping_over_an_unrelated_overdue_deadline`).
- **Why the stable sort happens in Rust, not as a SQL `ORDER BY` on a `CASE` expression.** The `agenda_date` derivation (Scheduled-if-in-window, else Deadline-collapsed-if-overdue) is exactly the same logic needed to build each row's `agenda_date` field — writing it once in Rust (used both to populate the field and, via `Vec::sort_by`, to order the rows) avoids a second, harder-to-maintain copy of the same branching logic as a hand-written SQL `CASE`. `sort_by` is stable, so ties preserve the SQL fetch's `(file_path, position)` order — the backend still does 100% of the ordering; the frontend still never re-sorts.
- **Why the perf gate benchmarks `agenda::week` directly, not the IPC round-trip.** Identical reasoning to Story 6.3's gate: the query (now plus its Rust-side sort) is the entire cost of `/agenda/week`'s render on the read side, so a hermetic, no-Tauri-runtime benchmark is a faithful proxy for the "view-switch completes in <200ms" AC.
- **Why empty window days still render a section.** A Week Agenda that only shows days with items answers "what's on my plate" but not "which days are free" — the latter is exactly what "so I can plan beyond today" (the story's own justification) asks for. Rendering all 7 days (with a plain "Nothing scheduled or due." line for empty ones) costs nothing extra given the window is always exactly 7 days, computed client-side from `start_date` via a local-time-safe `addDaysIso` helper (mirroring `localTodayIso`'s own local-getter convention, so it never drifts a day near a DST boundary the way a UTC-parsed `Date` + naive day-arithmetic would).

## Verification

**Commands:**
- `cargo test -p orgsidian-index -p orgsidian-core --features test-support -p orgsidian-shell-app --offline` -- expected: all green, no 6.3 regressions.
- `cargo test -p orgsidian-core --features test-support --test story_6_4_agenda_week_perf` -- expected: green; first run writes/verifies `tests/perf-baselines/story-6.4-agenda-week.json`.
- `cargo clippy -p orgsidian-index -p orgsidian-shell-app --all-targets --offline -- -D warnings` and `cargo clippy -p orgsidian-core --all-targets --features test-support --offline -- -D warnings` -- expected: 0 warnings from touched crates (pre-existing `orgsidian-parser` C-compiler warnings unrelated).
- `cargo fmt -p orgsidian-index -p orgsidian-core -p orgsidian-shell-app -- --check` -- expected: clean.
- `pnpm --filter shell-ui test` (`vitest run`) -- expected: all green, including the new `AgendaWeek.test.tsx` and the two updated `AgendaToday.test.tsx` selector assertions.
- `pnpm --filter shell-ui build` -- expected: `tsc` + `vite build` succeed (regenerates `routeTree.gen.ts` + `lib/tauri.ts`, both gitignored generated artifacts); `/agenda/week` compiles as its own route-split chunk.

**Result (2026-09-05):** Rust suite GREEN across all 3 touched crates (0 failed; 6 new `query::agenda::week` unit tests, 1 extended shell-app DTO-projection test, 1 new perf-AC test). `cargo clippy` (index/shell-app default; core with `test-support`) and `cargo fmt --check` clean on all three. `cargo test --features test-support --test story_6_4_agenda_week_perf` green; baseline committed (macos-aarch64, ~2.1ms median). `pnpm --filter shell-ui test` GREEN: 26 files, 291 tests (7 new `AgendaWeek` tests; `AgendaToday` tests updated for the new header Link with no regressions). `pnpm --filter shell-ui build` succeeds end-to-end — `dist/assets/week-*.js` confirms `/agenda/week` compiled in as its own code-split chunk. `Cargo.lock` unchanged (no new crate — SQLite's built-in `date()` function needed no new dependency).

## Spec Change Log

- 2026-09-05 — Implemented. `orgsidian-index::query::agenda::week` (+ the shared `AgendaItem::agenda_date` field) + `orgsidian-core::agenda_week` façade + `orgsidian-shell-app`'s `agendaWeek` command/`AgendaItemDto` extension, `AgendaWeek.tsx` (new `/agenda/week` content, reusing `AgendaToday`'s exported helpers) + the view-switch `Link` pair, and the Story 1.12 perf-AC gate with a committed baseline. All AC wired and tested end-to-end. Status → review.
