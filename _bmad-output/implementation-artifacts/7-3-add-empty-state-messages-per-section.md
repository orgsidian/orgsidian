---
title: 'Add empty-state messages per section'
type: 'feature'
created: '2026-09-14'
status: 'done'
review_loop_iteration: 0
baseline_commit: '3eafc897b9fcdec9fddde9f17f18592481121134'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/7-1-implement-today-dashboard-surface.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The five Today Dashboard sections (Scheduled, Deadline, Today-Tag, Inbox Preview, Active Clock) render generic, ad-hoc empty-body strings — Story 7.1 deliberately left copy-blessed empty states to this story (see 7.1 spec "Never" list). Per FR-6 consequences + FR-21 inline-coaching tone, an empty pane must read as a contextual, calm message, not an ambiguous blank.

**Approach:** Create the centralized coaching registry at `shell-ui/src/coaching/coachingRegistry.ts` (the home the epic context and 7.1 spec both name) holding one copy-blessed empty-state string per dashboard section, following the FR-21 calm/declarative tone. Rewire `TodayDashboard.tsx` so every section sources its empty-state copy from the registry instead of inline literals, and update the component test to assert the new copy.

## Boundaries & Constraints

**Always:**
- Empty-state copy is centralized in `shell-ui/src/coaching/coachingRegistry.ts`, keyed by the `DashboardSection` id (`@/lib/tauri`), so all five sections read from one source of truth.
- Use the AC copy-blessed strings verbatim: Scheduled `"No tasks scheduled for today — nice."`, Inbox `"Inbox empty."`, Active Clock `"No active clock — pick a task and start tracking."`. Derive Deadline and Today-Tag copy in the same FR-21 calm/declarative tone (no alarm, no exclamation, no "yet"; empty here is a good state).
- User-facing copy is plain hard-coded English (no lingui/`<Trans>`), matching the established baseline of the dashboard/agenda/coaching cluster (`TodayDashboard`, `AgendaToday`, `CoachingBalloon`, `ConflictBanner`) — carry ConflictBanner's documented i18n-deferral note into the new module.
- Each section renders its OWN contextual message (not one generic string).

**Never:**
- No backend/Rust changes; no new tauri command; no `IndexQuery`/DTO changes. This is a frontend-copy story only.
- Do not rebuild the registry-driven `CoachingSlot` API (Story 11.4) — keep the registry a minimal, forward-compatible data module aligned with, but not implementing, Epic 11.
- Do not add lingui extraction here; do not change section rendering/collapse behavior (Story 7.1/7.2).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Empty Scheduled | `scheduled` is empty | Section body renders `"No tasks scheduled for today — nice."` | N/A |
| Empty Deadline | `deadlines` is empty | Section body renders the deadline empty-state copy | N/A |
| Empty Today-Tag | `todayTag` is empty | Section body renders the today-tag empty-state copy | N/A |
| Empty Inbox | `inbox` is empty | Section body renders `"Inbox empty."` | N/A |
| No active clock | `activeClock` is null | Section body renders `"No active clock — pick a task and start tracking."` | N/A |
| Non-empty section | Section has items | Rows render as before; no empty-state copy shown | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/coaching/coachingRegistry.ts` -- NEW. Centralized empty-state copy. Export a typed `Record<DashboardSection, string>` (import `DashboardSection` from `@/lib/tauri`) mapping each of the five section ids to its copy-blessed message. Module doc-comment: purpose, FR-21 tone, the plain-English i18n-deferral note mirroring `ConflictBanner.tsx:127-131`, and a pointer that Story 11.4 will absorb this into the registry-driven `CoachingSlot` API.
- `shell-ui/src/components/today/TodayDashboard.tsx` -- MODIFY. Import the registry. Replace the three `AgendaList emptyLabel="…"` literals (lines ~134,147,160), `InboxList`'s hard-coded `"Inbox is empty."` (line ~323), and `ActiveClockView`'s `"No active clock."` (line ~358) with registry lookups. Update the top-of-file comment block (lines ~18-22) that currently states empty-state coaching is out of scope / bodies are a "minimal v0.1 stand-in" — this story implements it.
- `shell-ui/src/components/today/TodayDashboard.test.tsx` -- MODIFY. The `"renders minimal empty-state bodies for empty sections (rich copy is Story 7.3)"` test (line ~289) asserts the OLD strings — update its title + assertions to the five new copy-blessed strings.
- `shell-ui/src/components/editor/ConflictBanner.tsx` -- READ-ONLY reference (lines 127-131): the documented plain-string / i18n-deferral baseline to mirror.
- `shell-ui/src/lib/tauri.ts` -- READ-ONLY: `DashboardSection` union type (`scheduled|deadline|todayTag|inboxPreview|activeClock`) is the registry key type.

## Tasks & Acceptance

**Execution:**
- [x] `shell-ui/src/coaching/coachingRegistry.ts` -- NEW centralized `Record<DashboardSection, string>` of copy-blessed empty-state messages (FR-21 tone) with i18n-deferral doc note -- single source of truth for section empty states.
- [x] `shell-ui/src/components/today/TodayDashboard.tsx` -- source all five sections' empty-state copy from the registry; refresh the stale out-of-scope comment -- per-section contextual empty states.
- [x] `shell-ui/src/components/today/TodayDashboard.test.tsx` -- update the empty-state test title + assertions to the new copy-blessed strings -- guards the copy against regressions.

**Acceptance Criteria:**
- Given Story 7.1's dashboard, when a section has no items, then that section renders its copy-blessed empty-state message rather than a blank pane or a generic shared string.
- Given the Scheduled/Inbox/Active-Clock sections are empty, when the dashboard renders, then the exact AC strings `"No tasks scheduled for today — nice."`, `"Inbox empty."`, and `"No active clock — pick a task and start tracking."` appear.
- Given the empty-state copy, when it is defined, then every message lives in `shell-ui/src/coaching/coachingRegistry.ts` (centralized), keyed by `DashboardSection`, and follows the FR-21 calm/declarative tone (aligned with the Epic 11 coaching registry).

## Design Notes

Derived (non-AC-blessed) copy, in the same calm tone (empty = a settled, good state):
- Deadline (`deadline`): `"No deadlines due or overdue — you're clear."`
- Today-Tag (`todayTag`): `"Nothing tagged for today."`

Registry shape — keep it minimal and forward-compatible; a flat typed record is trivial for Story 11.4 to migrate into the richer `CoachingSlot` API:

```ts
import type { DashboardSection } from "@/lib/tauri";

export const dashboardEmptyState: Record<DashboardSection, string> = {
  scheduled: "No tasks scheduled for today — nice.",
  deadline: "No deadlines due or overdue — you're clear.",
  todayTag: "Nothing tagged for today.",
  inboxPreview: "Inbox empty.",
  activeClock: "No active clock — pick a task and start tracking.",
};
```

`AgendaList` already accepts an `emptyLabel` prop — keep that seam; just pass the registry value at each call site. `InboxList`/`ActiveClockView` read the registry directly (or accept the string) — mirror whichever keeps the diff smallest and consistent.

## Verification

**Commands:**
- `pnpm --filter shell-ui exec tsc --noEmit` -- expected: no type errors (registry key type matches `DashboardSection`).
- `pnpm --filter shell-ui exec vitest run` -- expected: `TodayDashboard.test.tsx` passes with the new copy assertions; full suite green.
- `cargo fmt --all -- --check` -- expected: clean (CI PR gate; run `cargo fmt --all` first even though only TS changed).

## Suggested Review Order

**Empty-state copy (design intent)**

- Entry point: the centralized, exhaustive copy map keyed by `DashboardSection` — the single source of truth and the FR-21 tone/i18n rationale.
  [`coachingRegistry.ts:28`](../../shell-ui/src/coaching/coachingRegistry.ts#L28)

**UI binding**

- Registry import wiring the copy into the dashboard.
  [`TodayDashboard.tsx:47`](../../shell-ui/src/components/today/TodayDashboard.tsx#L47)

- The three `AgendaList` sections now pass their own registry line as `emptyLabel`.
  [`TodayDashboard.tsx:138`](../../shell-ui/src/components/today/TodayDashboard.tsx#L138)

- Inbox and Active-Clock bodies read their own registry line.
  [`TodayDashboard.tsx:331`](../../shell-ui/src/components/today/TodayDashboard.tsx#L331)

**Tests (supporting)**

- Section-scoped body assertions (`toBe` per section) catch a message wired to the wrong section.
  [`TodayDashboard.test.tsx:301`](../../shell-ui/src/components/today/TodayDashboard.test.tsx#L301)

- `sectionBody` helper locating one section's collapsible content.
  [`TodayDashboard.test.tsx:199`](../../shell-ui/src/components/today/TodayDashboard.test.tsx#L199)
