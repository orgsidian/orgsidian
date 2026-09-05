<!-- Implements FR-21 (partial) / FR-18 / UJ-4 — hardcoded coaching-balloon copy reference (Story 6.6). -->

# Microcopy Registry

Tracks user-facing microcopy that ships ahead of a dedicated content/UX review
pass, so reviewers can find every `[draft]` string in one place instead of
hunting through component source. An entry graduates out of `[draft]` when a
named reviewer signs off on the wording; until then, treat the copy as subject
to change without notice.

## Story 6.6 — UJ-4 hardcoded coaching balloons (v0.1 Alpha)

**Status:** `[draft]`

Two hardcoded coaching balloons realize the UJ-4 "first five minutes"
promise on `/today` ahead of the v0.5 Beta registry-driven `CoachingSlot` API
(Story 11.4). Both are non-modal, dismissible (X button), and persist their
dismissal per-Vault at `<Vault>/.orgsidian/coaching-dismissed.json`, keyed by
the hardcoded coaching IDs below. See
`shell-ui/src/components/coaching/CoachingBalloon.tsx` for the component and
`crates/orgsidian-core/src/coaching.rs` for the persistence seam.

| Coaching ID          | Anchor                                                          | Copy                                                                                     | Rendered? |
| -------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------- |
| `UJ4_TODAY_INTRO`     | First item in the Today Agenda (`AgendaToday.tsx`)                | **This is your day.** Click any task to open the source file.                             | Yes — live now. |
| `UJ4_CAPTURE_INTRO`   | Top of the `/today` route (v0.1 stand-in — see note below)        | **Anything on your mind?** Press Cmd/Ctrl+Shift+Space to capture from anywhere.            | **No — gated until Story 8.1.** See note below. |

### Design tokens

Both balloons reuse the existing FR-22 `--org-*` vocabulary — no new tokens
were added (LD-51's token-addition process does not apply here):

- Surface: `--org-bg-elevated` (background), `--org-border-default` (1px
  border) — matches the "elevated surface" role `tokens.css` already assigns
  dialogs/popovers.
- Text: `--org-fg-default` (body copy), `--org-fg-muted` (dismiss button,
  resting state).
- Hover: `--org-bg-surface` (dismiss button hover background).
- Focus: `--org-border-focus` (visible focus ring on the dismiss button).

### `UJ4_CAPTURE_INTRO` anchor — locked decision (2026-09-05)

The epic AC anchors this balloon to "the Inbox preview section" of the Today
Dashboard. That section is Epic 7 scope (`FR-6`) and does not exist yet —
Story 6.3 shipped only the Scheduled/Deadline Agenda list on `/today`. Rather
than invent a placeholder Inbox preview just to host the balloon, this story
anchors it to a calm top-of-route placement on `/today` (above the Agenda).
The id, copy, and dismissal wiring are real and unchanged from the AC, so
Story 7.x (Inbox preview) or Story 11.4 (registry cutover) can re-anchor the
same `CoachingBalloon` to the real Inbox preview without touching the
persistence seam. See the Story 6.6 story file's Design Notes for the full
reasoning.

### `UJ4_CAPTURE_INTRO` render gate — locked decision (2026-09-05, post-review)

The capture hotkey (Cmd/Ctrl+Shift+Space) that this balloon's copy references
is not wired until Story 8.1 (Epic 8, after v0.1 Alpha). Shipping the balloon
before then would coach a first-run user toward a shortcut that does nothing,
which undermines the UJ-4 "first five minutes" promise rather than serving
it. The id, copy, component, and dismissal persistence are implemented and
frozen exactly as specified above — only rendering is suppressed: `today.tsx`
mounts `<CoachingBalloon id={UJ4_CAPTURE_INTRO}>` behind a single
`CAPTURE_FEATURE_AVAILABLE = false` constant (with a
`// TODO(Story 8.1): render once the capture hotkey is wired` comment).
Story 8.1 flips that one constant to re-enable the balloon; no other change
is needed, and no existing dismissal is lost since the dismissal store is
untouched. `UJ4_TODAY_INTRO` is unaffected — it renders unconditionally.

### Lifecycle

Story 11.4 (v0.5 Beta) **removes** `CoachingBalloon.tsx` and
`crates/orgsidian-core/src/coaching.rs` wholesale when the registry-driven
`CoachingSlot` API ships — this is a disposable v0.1 stand-in, not a component
Story 11.4 extends. It imports the two coaching ID constants
(`shell-ui/src/components/coaching/coachingIds.ts` /
`orgsidian_core::coaching::{UJ4_TODAY_INTRO, UJ4_CAPTURE_INTRO}`) so a Vault
whose user already dismissed a balloon under v0.1 does not see it resurface
under the v0.5 registry.
