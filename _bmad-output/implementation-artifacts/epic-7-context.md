# Epic 7 Context: Today Dashboard & Time Tracking

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Deliver the full Today Dashboard as the app's front door and make time tracking functional. This means upgrading `/today` into a five-section computed view (Scheduled today, Deadline today-or-overdue, configurable "today" tag, Inbox preview, Active Clock) with collapsible sections whose state persists and contextual empty states; completing the Agenda feature with a Custom date-range view and saved named filter presets; and implementing a Clock manager (in/out/resume) that persists to standard org `CLOCK:` lines in the LOGBOOK drawer, enforces a single Active Clock, prompts safely for a running clock left over from a prior session, and offers in-app clock-entry editing. This closes FR-6, the remaining part of FR-7 (Custom + presets), and the functional half of FR-8. Polished time-tracking UX (persistent status bar, refined timer notifications) is deferred to Epic 13.

Turn the day-one Agenda snapshot from Epic 6 into the full Planner Core. This epic delivers the complete Today Dashboard — the confirmed cold-start landing surface and the single most-defining product moment ("opening Today and seeing one's actual day already laid out"). It completes the Agenda (adding the Custom date-range view and saved filter presets on top of Epic 6's Today/Week subset) and implements functional time tracking: clock in/out/resume with org-standard LOGBOOK persistence, the single-Active-Clock invariant, and a safe prior-session running-clock recovery prompt on launch. Together these realize FR-6 (Today Dashboard), the remainder of FR-7 (Agenda), and functional FR-8 (Clock) — the Review and Act primitives that prove Orgsidian's Capture/Act/Review workflow composes. UX polish (persistent status bar, timer notifications) is deliberately deferred to Epic 13.

## Stories

- Story 7.1: Implement Today Dashboard surface
- Story 7.2: Persist Today Dashboard section preferences
- Story 7.3: Add empty-state messages per section
- Story 7.4: Implement Custom Agenda view with date range picker
- Story 7.5: Implement saved Agenda filter presets
- Story 7.6: Implement Clock manager + LOGBOOK persistence
- Story 7.7: Implement prior-session running-clock prompt on launch
- Story 7.8: Implement ClockEditor component for time entry editing

## Requirements & Constraints

- Today Dashboard is the default landing surface on launch (configurable in Settings to reopen the last file instead). It composes items Scheduled for today, Deadline today-or-overdue, items carrying a configurable "today" tag, an Inbox preview of the first N entries (default 5, configurable), and the Active Clock if one exists.
- Performance: cold launch reaches an interactive Today Dashboard in under 2s; dashboard render target is under 500ms on a 1,000-file Vault against the cached index. Agenda recompute after a single-file edit stays under 100ms (incremental, not full rebuild). Render and query paths are guarded by the perf-regression harness (initial absolute baselines above; later runs may not regress more than ~20%).
- Agenda supports Today, Week (rolling 7 days), and Custom (arbitrary date range), with tag / TODO-state / file-path filters that compose. Custom view result lists must scale to 1k+ items.
- Saved presets are named view+filter bundles the user can recall, and delete. Two presets ship by default and are seeded idempotently on first launch: "Done This Week" (`DONE` completed in the rolling 7 days) and "Done This Month" (rolling 30 days); starter-vault content must produce non-empty results for both.
- Clock: at most one Active Clock at a time; clocking into a new Headline auto-stops the prior one. Entries persist as standard org `CLOCK: [start]--[end] => HH:MM` lines in the LOGBOOK drawer (created if absent). Time totals are computable per Headline, subtree, tag, and date range (consumed later by Project Report). A clock still running when the app was closed must be surfaced on next launch and never silently recorded.
- Empty states are deliberate, not blank panes: copy-blessed, next-action-oriented messages following the project's inline-coaching tone, sourced from a centralized coaching registry.
- Accessibility is a hard CI gate: WCAG 2.1 AA contrast plus a keyboard-only happy-path scenario per primary surface (Today Dashboard and Agenda are in scope).

## Technical Decisions

- State ownership (LD-40): the per-Vault TOML settings store (`<Vault>/.orgsidian/settings.toml`) is authoritative for durable settings, including saved presets under an `[agenda_presets]` table — this supersedes the earlier `agenda-presets.json` location. `tauri-plugin-store` is retained only for ephemeral/per-Vault UI state; section collapse/expand preferences persist there (a `today-prefs.json`-style per-Vault file).
- Active Clock persistence: a state file `<Vault>/.orgsidian/active-clock.json` holds `{ headline_id, started_at, last_active_at }`. `last_active_at` is refreshed on every window-focus / app-foreground event via a Tauri window event listener; this field is what the launch-time stale-clock prompt pre-fills its "adjust" time from.
- Routing (LD-29, TanStack Router): typed routes `/today`, `/agenda/$view`, and `/agenda/custom` with typed search params (`?start=`, `?end=`, `?tag=`, `?todo=`) driving state; surfaces are deep-linkable.
- Virtualization (LD-30): Custom Agenda result lists render through `@tanstack/react-virtual` to handle 1k+ grouped items.
- Query surface: agenda/today queries consume the `IndexQuery` trait, frozen as a stable API in Epic 6 (Story 6.5) precisely so Epic 7 extensions don't collide with Epic 8. The SQLite index already carries a normalized `clock_entries` table (LD-11).
- Clock core lives in `orgsidian-core/src/clock.rs`; the Zustand `clockStore.ts` mirrors Active Clock state to the UI. Commands surface as `clockIn(headlineId)`, `clockOut()`, `clockResume(headlineId)`, `updateClockEntry(...)`, and a `totals(scope, range)` computation.
- UI component homes: `shell-ui/src/components/today/TodayDashboard.tsx`, `components/agenda/AgendaCustom.tsx`, `components/org/ClockEditor.tsx`; empty-state copy in `shell-ui/src/coaching/coachingRegistry.ts`.
- Traceability convention: each implementing module carries a `//! Implements FR-N` first doc-comment line, verified by a traceability test.

## UX & Interaction Patterns

- The Today Dashboard is a computed view across the whole Vault, not a single daily document. First-fold ordering is by urgency: Active Clock + Deadlines + the first several Scheduled rows sit above the fold; Inbox preview goes below. Scrolling is expected and accepted.
- Density is medium-tight workshop: ~38px list rows (14px mono, 1.6 line-height, 8px×12px padding), 28px gaps between sections. Sections are collapsible via a chevron toggle.
- Rows are click-to-source: clicking a Headline opens the Editor at the same `:ID:` (resolved by ID lookup, not stale byte offset).
- Clock is a one-keystroke toggle on the current Headline; the Active Clock shows as an ambient status-bar indicator (full status-bar polish is Epic 13). ClockEditor derives its start/end/duration model and LOGBOOK conventions from org-mode; editing any field recomputes the others.
- Running-clock launch prompt: a modal offering Adjust end time / Keep tracking / Discard, ordered safest-first, with keyboard default focus on "Adjust end time"; Enter confirms and Esc invokes Adjust (not cancel). Keep resumes from the original start with no source mutation; Discard removes the open `CLOCK:` line.
- Empty states are one of the project's five deliberate coaching moments; presets follow the "named filtered views" pattern (zero built-in presets historically, now with the two Done-This-* defaults shipped).

## Cross-Story Dependencies

- 7.1 depends on the basic Today view (6.3) and the frozen `IndexQuery` trait (6.5). 7.2 and 7.3 build on 7.1; 7.3's coaching copy aligns with the Epic 11 coaching registry.
- 7.4 depends on the Week Agenda (6.4) and 6.5. 7.5 depends on 7.4; its default-preset fixtures depend on the Story 6.1 starter-vault generators producing qualifying `DONE` content.
- 7.6 depends on Epic 3 (index/write path) and Epic 4 (editor) being closed. 7.7 depends on 7.6 (specifically the `last_active_at` field). 7.8 depends on 7.6.
- Clock data produced here feeds the Epic 10 Project Report. Time-tracking UX polish (persistent toggleable status bar, refined timer notifications, extended clock-time affordances) is deferred to Epic 13.
---

<!-- Alternate compiled epic-7 context (Story 7.4 branch). Retained in full during the Chain A merge so no compilation is dropped; both are kept. -->

## Requirements & Constraints (alternate compilation)

- **FR-6 (Today Dashboard):** default `/today` route shows five sections — Scheduled-today, Deadline-today-or-overdue, configurable "today"-tag items, Inbox preview (first N, default 5, configurable), and Active Clock. Each section collapsible with persisted state; each has an empty-state message. Render budget <500ms on a 1,000-file Vault.
- **FR-7 (Agenda — completion):** add Custom view (date-range picker) and saved named filter presets to finish FR-7 (Epic 6 shipped Today/Week). Filters (tag, TODO state, file path) are composable and persist within session. Ship two default presets out of the box: `Done This Week` (rolling 7 days) and `Done This Month` (rolling 30 days), seeded idempotently on first launch. Agenda view switch under 200ms.
- **FR-8 (Clock — functional):** clock in/out/resume on a Headline; entries persisted as org `CLOCK:` lines in the LOGBOOK drawer; at most one Active Clock (clocking a new Headline auto-stops the prior); resume the most recent unclosed entry; compute time totals per Headline/subtree/tag/date-range; detect and prompt on a running clock from the prior session (discard / adjust end time / keep running). Never silently record an overnight/away gap.
- **FR-21 tone (empty states):** empty-section copy follows inline-coaching tone (calm, blessed microcopy, suggests next action) and is centralized in the coaching registry.
- **Performance:** dashboard render gated by the shared perf-regression harness (absolute <500ms baseline, no >20% regression). Agenda scales to 1k+ items.
- **Depends on** Epic 6 (basic Today/Week views, starter vaults) and the frozen `IndexQuery` trait (Story 6.5) as a stable query surface; Epic 4 (editor) and Epic 3 (index/vault) closed for the Clock work.

## Technical Decisions

- **Layout / files:** Today widgets in `shell-ui/src/components/today/` (`TodayDashboard.tsx`); Agenda in `shell-ui/src/components/agenda/` (`AgendaCustom.tsx`); Clock UI in `shell-ui/src/components/org/ClockEditor.tsx`; core clock logic in `crates/orgsidian-core/src/clock.rs`.
- **Query layer:** all agenda/date queries go through the frozen `IndexQuery` trait (`orgsidian-index::query::agenda`); the SQLite index is derived/disposable, never source of truth. Do not add parallel query paths.
- **Routing (LD-29):** TanStack Router file-based routes; `/agenda/custom` driven by typed search params (`?start=`, `?end=`, `?tag=`, `?todo=`); typed loader data.
- **Virtualization (LD-30):** render agenda/custom result lists via `@tanstack/react-virtual` (must scale to 1k+ items). CM6 handles editor virtualization itself.
- **State (LD-40 settings authority):** per-Vault state lives under `<Vault>/.orgsidian/`; the SQLite index stays outside the Vault. Section collapse prefs → `today-prefs.json` via `tauri-plugin-store`. Filter presets → the per-Vault **TOML** settings store (Story 1.18) under `[agenda_presets]` (this supersedes an earlier `agenda-presets.json` location).
- **Clock state model:** Active Clock persisted to `<Vault>/.orgsidian/active-clock.json` as `{ headline_id, started_at, last_active_at }`. `last_active_at` is refreshed on every app foreground / window-focus event (`tauri::Window::on_window_event`) specifically to power Story 7.7's stale-clock pre-fill. Clock-out appends `CLOCK: [<start>]--[<end>] => HH:MM`, creating the LOGBOOK drawer if absent.
- **IPC commands (typed via tauri-specta):** `clockIn(headlineId)`, `clockOut()`, `clockResume(headlineId)`, `updateClockEntry(headlineId, entryIndex, newStart, newEnd)`; totals via `orgsidian-core::clock::totals(scope, range)`. Consume the generated client (`@/lib/tauri`), never raw `invoke`.
- **Zustand:** `clockStore.ts` for Active Clock (selectors exposed as hooks, e.g. `useActiveClock`); one store per concern; never duplicate CM6 state.
- **Conventions:** modules implementing an FR carry `//! Implements FR-NN` doc-comment (checked by `tests/traceability.rs`); dates via `chrono` (ISO 8601 on the wire); commands return `Result<T, OrgError>`.
- **Plugin surface:** clock events (`ClockStarted`/`ClockStopped`) are part of the `#[non_exhaustive]` Event enum; clock actions are Headline-scoped by construction — keep them flowing through the existing `OrgsidianPlugin` trait surface.

## UX & Interaction Patterns

- **Today as computed view:** the dashboard is a computed slice across the whole Vault, not a single daily document. It is the cold-start landing surface and must feel "populated and sensible day-one with no tweaking."
- **Empty-state copy (blessed microcopy):** e.g. "No tasks scheduled for today — nice.", "Inbox empty.", "No active clock — pick a task and start tracking." Empty Today is one of the five deliberate coaching moments; tone is calm and next-action-oriented, sourced from `coachingRegistry.ts`.
- **Clock as calm ambient timer (Toggl-referent):** minimalist single-button start/stop feel; atomicity under spam-click is a property-tested invariant.
- **Prior-session prompt (Story 7.7) — crisis UX dialed down:** modal offers actions in safest-default order; keyboard-default-focused button is **"Adjust end time"** (Enter confirms, Esc invokes Adjust — not cancel). Show currently-logged duration both "if you keep tracking" and "if you adjust to last-active." "Keep tracking" resumes from original `started_at` with no source mutation; "Discard" removes the open `CLOCK:` line. Copy is `[microcopy: draft]`, final pass recorded in `docs/microcopy-registry.md`.
- **ClockEditor:** start/end/duration fields where editing any one recomputes the others; used both here and reused by the Epic 13 status bar.
- **Accessibility:** a keyboard-only happy-path scenario is required for the Today Dashboard and Agenda surfaces (part of the a11y CI gate).

## Cross-Story Dependencies

- 7.2 and 7.3 depend on 7.1 (dashboard surface).
- 7.4 depends on Epic 6 Story 6.4 (Week view) + 6.5 (frozen `IndexQuery`); 7.5 depends on 7.4 and on Story 1.18 (TOML settings store).
- 7.6 is the clock foundation for 7.7 and 7.8; 7.6's `last_active_at` field is a hard prerequisite for 7.7's stale-clock pre-fill.
- 7.5's default presets require Story 6.1 starter-vault generators to seed fixture content (≥2 `DONE` headlines in range) so both default presets are non-empty on first launch.
- Downstream: Epic 13 adds FR-8 UX polish (persistent status bar reusing `ClockEditor`, timer notifications); FR-8 also feeds Epic 10 Project Report (clocked-time aggregation).
