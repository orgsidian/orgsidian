// Implements FR-21 (inline-coaching tone) / FR-6 (Today Dashboard — Story 7.3).
//
// The centralized registry of copy-blessed empty-state messages for the Today
// Dashboard's five sections. Story 7.1 rendered generic stand-in bodies and
// deliberately deferred the real copy to here; each section now sources its
// empty-state line from this one source of truth, keyed by `DashboardSection`.
//
// Tone (FR-21, ux-design-specification.md Emotional Design Principles): the app
// is a calm workshop, not a stage. Empty states are declarative one-liners —
// no exclamation, no "yet", no celebration; where a section being empty is a
// settled/good state (Scheduled, Deadline) the copy quietly affirms it, and
// where an action is the natural next step (Active Clock) it names that step.
// These are three of the project's deliberate empty-state coaching moments.
//
// Story 11.4 (v0.5 Beta) introduces the registry-driven `CoachingSlot` API and
// will absorb this module; keeping the shape a flat, typed record makes that
// migration trivial. Do not grow a bespoke API here.
//
// i18n note: copy is plain English, matching the established baseline of the
// dashboard/agenda/coaching cluster (`TodayDashboard`, `AgendaToday`,
// `CoachingBalloon`) and the documented convention in `ConflictBanner.tsx` —
// the repo defers UI-string extraction to a dedicated i18n pass, so these are
// not wrapped in lingui `<Trans>`/`t` here.

import type { DashboardSection } from "@/lib/tauri";

/** Copy-blessed empty-state message per Today Dashboard section. */
export const dashboardEmptyState: Record<DashboardSection, string> = {
  scheduled: "No tasks scheduled for today — nice.",
  deadline: "No deadlines due or overdue — you're clear.",
  todayTag: "Nothing tagged for today.",
  inboxPreview: "Inbox empty.",
  activeClock: "No active clock — pick a task and start tracking.",
};
