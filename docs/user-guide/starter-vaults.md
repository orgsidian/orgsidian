<!-- Implements FR-18 — Starter Vault content reference (Story 6.1). -->

# Starter Vaults

The first time Orgsidian launches with no configured Vault, it offers to
generate a **Starter Vault**: a small, realistic set of `.org` files so you
see the *workflow*, not a blank editor or a syntax reference, in your first
five minutes.

> **Status (v0.1 Alpha):** the generator ships **Personal GTD** and
> **Student**. The picker UI that wires this generator into first-launch
> (`shell-ui/src/components/onboarding/StarterVaultPicker.tsx`) is Story 6.2.
> **Freelancer** and **Empty** are not yet available — see
> [Deferred](#deferred).

Both starters share the same shape:

| File          | Purpose                                                              |
| ------------- | --------------------------------------------------------------------- |
| `inbox.org`   | Unprocessed capture — bare `TODO` items with no dates or scheduling yet. |
| `projects.org` (Personal GTD) / `courses.org` (Student) | One active project/course with Next Actions. |
| `journal.org` | A couple of daily-log entries (reflection, not tasks).                 |
| `someday.org` | The Someday/Maybe parking lot — bare ideas, no states or dates.        |

Every generated file lives flat at the Vault root (no subfolders), and the
Next Actions in `projects.org`/`courses.org` carry `SCHEDULED`/`DEADLINE`
timestamps anchored to the day the Vault was generated ("today"), spread
across the following week — so opening the Vault immediately shows non-empty
Today and Week Agenda content once you're on the `/today` and `/agenda/week`
routes (Stories 6.3/6.4).

## Personal GTD

Modeled on David Allen's *Getting Things Done*: capture everything into an
Inbox, organize into Projects with Next Actions, park the rest on
Someday/Maybe, review regularly.

- **`inbox.org`** — a few unprocessed capture items (call the dentist, a
  birthday gift idea, a library book to return).
- **`projects.org`** — one active project ("Repaint the garage") with a
  `DONE` item already `CLOSED`, a `TODO` scheduled for today, a `NEXT`
  action for tomorrow, and a `TODO` scheduled a few days out that also
  carries a `DEADLINE`.
- **`journal.org`** — today's and yesterday's daily-log entries (inactive
  timestamps, intentionally outside the Agenda).
- **`someday.org`** — ideas not yet committed to (learn woodworking, plan a
  trip, read about home automation).

## Student

Shaped around a term's coursework rhythm rather than GTD's general
life-management categories.

- **`inbox.org`** — a few unprocessed capture items (a question for office
  hours, slides to print, a study group to find).
- **`courses.org`** — one active course ("Introduction to Statistics") with
  a submitted problem set already `CLOSED`, a reading `TODO` scheduled for
  today, a `NEXT` problem set due later in the week (`SCHEDULED` +
  `DEADLINE`), and a review `TODO` scheduled a few days out.
- **`journal.org`** — today's and yesterday's study-log entries (inactive
  timestamps, intentionally outside the Agenda).
- **`someday.org`** — courses/topics worth exploring later (a data-viz
  elective, touch typing, spaced repetition).

## TODO keywords

Both starters use Orgsidian's default TODO sequence (`TODO`/`NEXT` active,
`DONE`/`WAITING` done) — no `#+TODO:` directive is written, so the default
cycling order (`TODO → NEXT → DONE → WAITING → TODO`) applies out of the box.

## Deferred

- **Freelancer** — real-world freelancer vaults are project/client-centric
  and need at least one `[[wiki-link]]`/`id:` reference between Headlines so
  the BacklinksPanel (Story 8.7, not yet built) shows a backlink on first
  launch. Building the Freelancer starter ahead of that panel would ship
  content nothing in the app can display yet, so it is deferred until
  Story 8.7 lands — see
  `_bmad-output/implementation-artifacts/deferred-work.md`.
- **Empty** — an explicit "start from nothing" option with onboarding
  coaching ships in Story 11.1 (v0.5 Beta). Until then, "Use my own folder"
  in the (Story 6.2) picker is the stand-in for anyone who wants an empty
  Vault.
