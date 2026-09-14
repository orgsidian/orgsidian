---
title: 'Implement Today Dashboard surface'
type: 'feature'
created: '2026-09-13'
status: 'done'
review_loop_iteration: 0
baseline_commit: '8aba8c43132404b0518d1fc2609571cf6b6f9c2b'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The `/today` route currently renders only the basic combined Today agenda (`AgendaToday`, Story 6.3). FR-6 requires the full Today Dashboard: one screen on launch showing Scheduled-today, Deadline-today-or-overdue, a configurable "today"-tag section, an Inbox preview, and the Active Clock if any — each collapsible.

**Approach:** Add a backend `dashboard::today` query (free functions in the frozen-trait's crate, composing scheduled/deadline/today-tag/inbox/active-clock reads over the cached SQLite index), expose it as one tauri command, and build `TodayDashboard.tsx` rendering five collapsible sections. Gate the backend assembly with the perf harness (<500ms on a 1000-file Vault) and trace FR-6 via a doc-comment verified by `tests/traceability.rs`.

## Boundaries & Constraints

**Always:**
- The implementing backend module (`crates/orgsidian-index/src/query/dashboard.rs`) carries `//! Implements FR-6` as its first doc-comment line; `tests/traceability.rs` verifies it.
- Backend dashboard-data assembly is gated by `assert_no_perf_regression!("story-7.1-today-dashboard", "tests/perf-baselines/story-7.1.json", || { … })` — first run writes the baseline; target <500ms on a synthetic 1000-file Vault; later runs may not regress >20%.
- Five sections render in order: `Scheduled | Deadline | Today-Tag | Inbox Preview | Active Clock`, each collapsible via a chevron toggle (default expanded).
- Inbox preview shows the first N entries, N configurable (default 5); the "today" tag is configurable (default `today`). Both read from per-Vault settings (`TodayDashboardSections`).
- Exclude DONE headlines and quarantined files from Scheduled/Deadline/Today-Tag, matching existing `agenda::today` semantics.
- Reuse existing frontend patterns exactly: inline `useEffect`+`disposed` fetch, `commands.*` from `@/lib/tauri`, `errorMessage`/`deadlineLabel` from `AgendaToday.tsx`, `Link` to `/editor/$filePath/$headlineId`, CSS `--org-*` color tokens, `container mx-auto p-8`.
- New command registered in `collect_commands!` in shell-app and kept in lockstep with the export-bindings test.

**Ask First:**
- Do NOT modify the frozen `IndexQuery` trait surface (`crates/orgsidian-index/src/query/mod.rs`) — additions to that trait risk the cargo-semver-checks gate.

**Never:**
- Section collapse/expand PERSISTENCE (Story 7.2), copy-blessed empty-state coaching messages via `coachingRegistry` (Story 7.3), clock in/out/write logic and LOGBOOK persistence (Story 7.6). 7.1 only READS an existing running clock (`clock_entries` where `end_at IS NULL`) for display.
- Do not re-sort backend-ordered results in the frontend.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Happy path | Vault with scheduled/deadline/tagged/inbox items, one running clock | `TodayDashboard` returns populated `scheduled`, `deadlines`, `todayTag`, `inbox` (≤N), `activeClock: Some` | N/A |
| No active clock | `clock_entries` has no row with `end_at IS NULL` | `activeClock` is `null`; section still renders (header only) | N/A |
| Empty sections | No matching items | Each section renders header + minimal blank body (rich empty copy is Story 7.3) | N/A |
| Inbox absent | No `inbox.org` at Vault root | `inbox` is empty vec | N/A |
| Overdue deadline | `deadline_date < today`, not DONE | Appears in `deadlines` with `overdue = true` | N/A |
| No active vault | No designated Vault | Command errors `no_active_vault`; frontend shows `role="alert"` message | Throw → `errorMessage` |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-index/src/query/agenda.rs` -- REUSE: `AgendaItem` struct + `today()` SELECT shape; dashboard scheduled/deadline/today-tag rows reuse `AgendaItem`. Query excludes DONE + quarantined, orders `(f.path, h.position)`.
- `crates/orgsidian-index/src/query/mod.rs` -- ADD `pub mod dashboard;`. Do NOT touch the frozen `IndexQuery` trait.
- `crates/orgsidian-index/src/query/dashboard.rs` -- NEW: `//! Implements FR-6`; `TodayDashboard`, `InboxItem`, `ActiveClock` structs; `DashboardParams`; `pub fn today(conn, &DashboardParams) -> Result<TodayDashboard, IndexError>`. Sub-queries: scheduled (`scheduled_date = today`), deadlines (`deadline_date <= today`), today-tag (join `tags` on `tag = ?`), inbox (`files.path = 'inbox.org'`, order position, LIMIT N), active-clock (`clock_entries WHERE end_at IS NULL` join headlines/files, LIMIT 1).
- `crates/orgsidian-index/src/lib.rs` -- RE-EXPORT dashboard types + fn (mirror how `agenda`/`AgendaItem` are exported).
- `crates/orgsidian-core/src/index/mod.rs` -- ADD `pub async fn today_dashboard(vault_root, params) -> Result<TodayDashboard, OrgError>` mirroring `agenda_today` (line 263): open index, `spawn_blocking`, call query.
- `crates/orgsidian-core/src/lib.rs` -- RE-EXPORT `today_dashboard` + dashboard types (line 35 block).
- `crates/orgsidian-core/src/settings/schema.rs` -- EXTEND `TodayDashboardSections` (line 85): add `inbox_preview_count: usize` (field default 5) + `today_tag: String` (field default `today`) with `#[serde(default = "…")]` for forward-compat; update `Default` impl (line 92).
- `crates/orgsidian-shell-app/src/lib.rs` -- ADD DTO twins `TodayDashboardDto`/`InboxItemDto`/`ActiveClockDto` (camelCase, `specta::Type`, i64→u32 narrowing per `AgendaItemDto`); `async fn today_dashboard(today, state) -> OrgResult<TodayDashboardDto>` reads vault settings for `today_tag`/`inbox_preview_count`, resolves `current_vault_root()`; register in `collect_commands!`.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- UPDATE command list to include `today_dashboard` (kept in lockstep with `build_specta`).
- `crates/orgsidian-core/tests/story_7_1_today_dashboard_perf.rs` -- NEW: synthetic 1000-file in-memory index (copy pattern from `story_6_3_agenda_today_perf.rs`), gate `dashboard::today` with the perf macro + baseline `tests/perf-baselines/story-7.1.json`.
- `crates/orgsidian-core/Cargo.toml` -- ADD `[[test]]` for `story_7_1_today_dashboard_perf` (path `tests/…`, `required-features = ["test-support"]`) and `[[test]]` for `traceability` (path `../../tests/traceability.rs`).
- `tests/traceability.rs` -- NEW (does not exist yet): grep-smoke asserting `crates/orgsidian-index/src/query/dashboard.rs` first doc line contains `//! Implements FR-6`. Table-driven for future FRs.
- `tests/perf-baselines/story-7.1.json` -- generated on first perf-test run (leave for the run to write).
- `shell-ui/src/components/ui/collapsible.tsx` -- NEW: shadcn-style wrapper over `radix-ui` `Collapsible` (pattern from `ui/tabs.tsx`).
- `shell-ui/src/components/today/TodayDashboard.tsx` -- NEW: `// Implements FR-6` header; fetch `commands.todayDashboard(localTodayIso())`; five collapsible sections w/ `ChevronRight`/`ChevronDown` (lucide-react), per-section `useState` open (default true); rows reuse `Link`/`deadlineLabel`/`errorMessage`.
- `shell-ui/src/routes/_layout/today.tsx` -- SWAP `<AgendaToday />` for `<TodayDashboard />`.
- `shell-ui/src/components/today/TodayDashboard.test.tsx` -- NEW: vitest (jsdom, mock `@/lib/tauri`, memory router) — asserts five sections, chevron toggle collapses body, error/empty states. Follow `AgendaToday.test.tsx`.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/orgsidian-index/src/query/dashboard.rs` -- NEW module (`//! Implements FR-6`) with structs + `today()` composing the five sub-queries -- backend data source for the dashboard.
- [ ] `crates/orgsidian-index/src/query/mod.rs` + `crates/orgsidian-index/src/lib.rs` -- wire + re-export dashboard -- expose to core without touching frozen trait.
- [ ] `crates/orgsidian-core/src/settings/schema.rs` -- add `inbox_preview_count` (5) + `today_tag` (`today`) with field defaults -- configurable inbox N + today tag.
- [ ] `crates/orgsidian-core/src/index/mod.rs` + `lib.rs` -- add `today_dashboard` async facade + re-exports -- core entry point.
- [ ] `crates/orgsidian-shell-app/src/lib.rs` + `tests/export_bindings.rs` -- DTOs + `today_dashboard` command + registration -- typed IPC surface.
- [ ] `crates/orgsidian-core/tests/story_7_1_today_dashboard_perf.rs` + `crates/orgsidian-core/Cargo.toml` -- perf gate on 1000-file synthetic index -- honors NFR <500ms.
- [ ] `tests/traceability.rs` + `[[test]]` wiring -- verify FR-6 doc-comment -- satisfies AC traceability.
- [ ] `shell-ui/src/components/ui/collapsible.tsx` -- radix Collapsible wrapper -- collapsible section primitive.
- [ ] `shell-ui/src/components/today/TodayDashboard.tsx` + `routes/_layout/today.tsx` -- five-section dashboard + route swap -- FR-6 surface.
- [ ] `shell-ui/src/components/today/TodayDashboard.test.tsx` -- component tests -- section rendering + toggle + states.

**Acceptance Criteria:**
- Given a Vault, when the Today Dashboard renders, then `shell-ui/src/components/today/TodayDashboard.tsx` shows five sections `Scheduled | Deadline | Today-Tag | Inbox Preview | Active Clock`, each collapsible via a chevron toggle.
- Given the perf harness, when the backend dashboard assembly runs on a 1000-file Vault, then `assert_no_perf_regression!("story-7.1-today-dashboard", "tests/perf-baselines/story-7.1.json", …)` passes (baseline written first run; <500ms target; ≤20% regression thereafter).
- Given default settings, when the Inbox preview renders, then it shows the first 5 `inbox.org` entries; changing `inbox_preview_count` changes N.
- Given `crates/orgsidian-index/src/query/dashboard.rs`, when `tests/traceability.rs` runs, then it asserts the module's first doc line is `//! Implements FR-6`.
- Given no active vault, when the command runs, then it errors `no_active_vault` and the frontend shows a `role="alert"` message.

## Design Notes

- Perf-gate interpretation: `assert_no_perf_regression!` is a Rust macro (Story 1.12), so it gates the backend dashboard-data assembly (`dashboard::today`) — the measurable proxy for "dashboard render" — not the React render. Synthetic index mirrors `story_6_3_agenda_today_perf.rs`.
- Scheduled vs Deadline may overlap (an item scheduled today with a past deadline appears in both) — intended, matches org agenda semantics; run them as two independent SELECTs rather than partitioning one.
- `tests/traceability.rs` is created here (first story to need it). Keep it a small table `[(fr, relative_path, expected_line)]` so later FRs extend it.
- DTO narrowing: `headline_id`/`byte_start` are `i64` in core but exported as `u32` in DTOs (specta forbids i64/u64) — copy `AgendaItemDto`'s `From` conversion.

## Verification

**Commands:**
- `cargo test -p orgsidian-index` -- expected: dashboard query unit tests pass.
- `cargo test -p orgsidian-core --features test-support` -- expected: `story_7_1_today_dashboard_perf` + `traceability` pass (baseline written first run).
- `cargo test -p orgsidian-shell-app` -- expected: export-bindings test passes with `today_dashboard` registered.
- `cargo build -p orgsidian-shell-app` -- expected: compiles (regenerates specta bindings via prebuild path in debug).
- `cd shell-ui && pnpm test` -- expected: `TodayDashboard.test.tsx` passes.
- `cd shell-ui && pnpm run lint` (or `tsc --noEmit` per project) -- expected: no new lint/type errors.

## Suggested Review Order

**Backend query (design intent)**

- Entry point: the five-section assembly composed under one consistent read transaction.
  [`dashboard.rs:268`](../../crates/orgsidian-index/src/query/dashboard.rs#L268)

- FR-6 trace anchor + section/param data model.
  [`dashboard.rs:1`](../../crates/orgsidian-index/src/query/dashboard.rs#L1)

- Inbox preview: first-N from `inbox.org`, saturating LIMIT (no negative wrap).
  [`dashboard.rs:203`](../../crates/orgsidian-index/src/query/dashboard.rs#L203)

- Active clock: the one running `end_at IS NULL` entry, lowest id if several.
  [`dashboard.rs:234`](../../crates/orgsidian-index/src/query/dashboard.rs#L234)

**Settings (configurable N + today tag)**

- New per-field defaults (today tag "today", inbox count 5) with upgrade-compat.
  [`schema.rs:101`](../../crates/orgsidian-core/src/settings/schema.rs#L101)

**Core + IPC wiring**

- Async facade opening the index and delegating to the dashboard query.
  [`index/mod.rs:319`](../../crates/orgsidian-core/src/index/mod.rs#L319)

- Command reads today-tag/inbox-count from vault settings into `DashboardParams`.
  [`lib.rs:849`](../../crates/orgsidian-shell-app/src/lib.rs#L849)

- Wire DTO twins (camelCase, i64→u32 narrowing).
  [`lib.rs:815`](../../crates/orgsidian-shell-app/src/lib.rs#L815)

**Frontend surface**

- Five collapsible sections; single memoized day for fetch + badges; a11y count label.
  [`TodayDashboard.tsx:42`](../../shell-ui/src/components/today/TodayDashboard.tsx#L42)

- Radix Collapsible wrapper backing each section's chevron toggle.
  [`collapsible.tsx:13`](../../shell-ui/src/components/ui/collapsible.tsx#L13)

- Route swap: `/today` now renders the dashboard.
  [`today.tsx:91`](../../shell-ui/src/routes/_layout/today.tsx#L91)

**Gates & tests (supporting)**

- Perf gate on a synthetic 1000-file Vault (<500ms baseline).
  [`story_7_1_today_dashboard_perf.rs:125`](../../crates/orgsidian-core/tests/story_7_1_today_dashboard_perf.rs#L125)

- FR-6 doc-comment traceability check.
  [`traceability.rs:23`](../../tests/traceability.rs#L23)

- Dashboard unit tests (DONE-exclusion per section, overlap, clock, inbox N).
  [`dashboard.rs:425`](../../crates/orgsidian-index/src/query/dashboard.rs#L425)
