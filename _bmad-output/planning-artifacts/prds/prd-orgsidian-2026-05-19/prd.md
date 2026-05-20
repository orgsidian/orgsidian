---
title: Orgsidian
status: final
created: 2026-05-19
updated: 2026-05-20
revisions:
  - date: 2026-05-20
    summary: PRD reconciliation post-UX-design-specification (`_bmad-output/planning-artifacts/ux-design-specification.md`). §1 Vision wedge reframed to "outline + agenda + graph" three views. §3 Glossary adds Refile + Graph View. §4.2 FR-7 notes Done-This-Week/Month default preset for v0.5 Beta; new FR-25 Refile (org-canonical triage primitive). §4.3 FR-12 latency tightened to two-tier (<100ms first 10 / <200ms full 50 per UXD effortless-interactions refinement); FR-13 notes unlinked-references panel as v0.5+ extension; new FR-26 Backlink Graph View. §4.5 FR-18 assumption updated — Freelancer Starter Vault promoted to v0.1 Alpha per lighthouse-persona commitment (UX spec line 115). §6.1 v0.1 Alpha scope adds FR-26 Graph view and updates FR-18 (Personal GTD + Student + Freelancer). §6.2 v0.5 Beta scope adds FR-25 Refile + Done-This-Week preset. §8 Accessibility NFR strengthened: WCAG 2.1 AA contrast + keyboard navigation as hard CI gate from v0.1 Alpha (axe-core + contrast-matrix + Playwright keyboard scenarios). §10 OQ-5 (org syntax coverage) notes operational quarantined-malformed fallback locked by UX Journey 4; OQ-7 (Settings) ✅ Resolved — both GUI + config file ship, config file authoritative; OQ-9 Spike 3 operational invariants locked by UX UXD-8 + Journey 5. §12 Assumptions Index updated for FR-18.
  - date: 2026-05-19
    summary: PRD reconciliation post-architecture (LD-46). §7.3, §10 OQ-1/OQ-2/OQ-8, addendum §A.2 and §A.3 updated to reflect MIT (LD-1) + tree-sitter-org + custom semantic layer (LD-3) + Tauri 2.x (LD-1..LD-10).
  - date: 2026-05-19
    summary: PRD reconciliation wave 2. §7.3 + §10 OQ-1 + addendum §A.2 thread in `tree-sitter-org` vendoring & maintenance contingency (architecture LD-48). §8 names i18n library and translator-facing catalog format per architecture LD-52 (Lingui v6.x; `.po` Gettext). §10 OQ-6 customization-template language updated to reflect LD-53 (Typst `.typ` for PDF path, HTML/CSS for HTML path; `sys.inputs` schema generated from `ReportData`).
  - date: 2026-05-19
    summary: Sprint Change Proposal (correct-course) absorbed. PRD body unchanged. Development-infrastructure constraints (private GitHub repo `orgsidian/orgsidian` flipped to public at v0.1 Alpha tag; Conventional Commits enforcement; system-level test strategy at `_bmad-output/test-artifacts/test-design.md` as authoritative) absorbed by architecture LD-5/LD-33/LD-54/LD-55 and epics Stories 1.13-1.16. PRD §6.1 "Public repository" bullet anchored at v0.1 Alpha release tag (Story 6.10) by construction. §7.1/§7.2 commitments untouched (runtime privacy ≠ source-host privacy). See `_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md`.
---

# PRD: Orgsidian
*Working title — confirm.*

## 0. Document Purpose

This PRD describes **Orgsidian**, an open-source cross-platform desktop application that opens, edits, and organizes `.org` files (Emacs org-mode format) without requiring Emacs. The document targets three audiences simultaneously:

1. **The author (solo developer).** A scope anchor against feature drift, reused as input to `bmad-create-architecture` and `bmad-create-epics-and-stories`.
2. **Potential contributors and early adopters.** A public artifact published alongside the repository to communicate vision, scope, and what help is welcome.
3. **Future-self.** A motivational anchor — when momentum dips at month 9, this document re-grounds the *why*.

The PRD is structured per BMad conventions: Glossary-anchored vocabulary, features grouped with FRs nested and globally numbered, assumptions tagged inline as `[ASSUMPTION: ...]` and indexed in §12. It builds on the brainstorming session of 2026-05-18 (`_bmad-output/brainstorming/brainstorming-session-2026-05-18-1613.md`), which remains the authoritative source for product reasoning. Architectural depth, parser license analysis, and stack rationale live in `addendum.md` alongside this file — downstream consumed by `bmad-create-architecture`.

**Reading paths.** For contributors: §1 Vision → §2 Target User → §5 Non-Goals → §6 MVP Scope → CONTRIBUTING.md (published with v0.1 Alpha) for the help map. For future-Tiziano at month 9 when momentum dips: §1 Vision is the *why*, §11 Success Metrics is the progress checkpoint — the milestones are the anchor, not the daily todo list.

## 1. Vision

Org-mode is the most powerful integrated planner-and-knowledge format ever shipped, but it lives inside Emacs — and Emacs is a hard barrier. Existing escape hatches (Organice in the browser, Logseq with a lossy org dialect, beorg on iOS) each cover a slice of the surface but none combine *desktop-native, cross-platform, OSS, and faithful to the format*.

**Orgsidian is the integrated planner-and-knowledge desktop app for people who want org-mode without Emacs.** It treats tasks, time, and notes as peers — not notes-with-tasks-bolted-on (Obsidian) or tasks-with-notes-attached (NotePlan), but a unified surface where the agenda is the front door and a backlinked knowledge graph is the second click. The product surface is *one object, three views*: a Headline can be approached through its **outline** (the file as written), the **agenda** (the same Headline as a scheduled commitment), or the **graph** (the same Headline as a node in a backlink network). All three views ship in v0.1 Alpha. Files stay as `.org` on the local filesystem; sync is whatever the user already trusts (Git, Syncthing, iCloud).

The marketing wedge is *task-first* — "planner powered by org-mode" is the headline that cuts through the saturated PKM space and surfaces in HN titles and r/orgmode posts. The product underneath is the integration, not a task tool with notes welded on.

The strategic bet is that **the unification itself is the differentiator** — the niche of "desktop-native, cross-platform, OSS, org-faithful planner+PKM" is genuinely unclaimed in 2026, and a small but real demand exists from the diaspora of org-curious users who bounced off Emacs and the Logseq users who lost org support in the DB-version rewrite. This is not a play for the Emacs-loyalist community; that audience will stay in Emacs and that is fine.

## 1.5 Design Principles

The cross-cutting commitments that shape micro-decisions throughout the product. When in doubt — about a default, a UI affordance, a setting, a feature scope — return to these. They are deliberate and load-bearing; brainstorming Phase 3 First Principles re-validated each.

- **Smart Defaults, User in Control.** Every opinionated feature ships with a sensible default *and* a toggle. The default should serve the lighthouse persona out-of-the-box; the toggle exists for the long tail. We do not optimize for power-user configurability over first-launch usability, but we never hide the toggle.
- **Workflow over Syntax.** Onboarding teaches the *workflow* (capture → triage → schedule → agenda → clock → review), not the org-mode markup. Documentation does the same. The syntax is a side effect of the workflow, not the curriculum.
- **The Filesystem is the Source of Truth.** `.org` files own the data. The SQLite index is derived cache, fully rebuildable, never authoritative. Any architectural temptation to reverse this is rejected.
- **Single Writer Rule.** While Orgsidian holds a Dirty Buffer for a file, it is the sole writer. External writes to dirty files surface a Merge Dialog, never silent overwrite. Data loss is not a recoverable error mode.
- **Round-trip Fidelity is the Trust Contract.** A file Orgsidian opens and saves without user edits is byte-identical to the original. This is what separates Orgsidian from the lossy alternatives the org community has learned to distrust. FR-2 enforces it as a hard CI check.
- **Local-first, No Account, No Telemetry by Default.** No cloud account exists. No network calls fire in the core workflow. Any future opt-in telemetry is explicit, visible, and disable-able.
- **Solo-OSS Discipline: Scope Compresses, Quality Holds.** When a milestone budget overruns, the response is feature compression, not quality compression. A shipped 20-feature v1.0 is worth more than an unshipped 30-feature v1.0; a buggy 30-feature v1.0 is worse than both. See §6 and §11 SM-C1.

## 2. Target User

### 2.1 Primary Persona

**The Independent Knowledge Worker (a freelancer or consultant by canonical example).** Manages their own projects end-to-end: client work, deliverables, time tracking, invoicing prep, and the knowledge accumulated across engagements. Comfortable with the command line and a text editor; not necessarily an Emacs user. Has tried Obsidian and either still uses it (with friction around task management) or churned to a planner tool and misses the note depth. Values local files, plain text, and ownership of their data. Will not adopt a tool that requires a SaaS subscription, a cloud account, or learning Emacs.

This persona is the **lighthouse** — the audience the product must serve excellently. Broader audiences (students, researchers, GTD enthusiasts who don't use Emacs, developers managing personal projects) follow naturally if the lighthouse experience is right.

### 2.2 Jobs To Be Done

- **Plan and execute** projects with tasks, deadlines, and time tracking visible alongside the related notes — without context-switching between a planner app and a notes app.
- **Capture** thoughts, todos, and references friction-free from anywhere on the OS, so nothing is lost between apps.
- **Review** what was done today, this week, this project — both for self-management and for client-facing reports.
- **Own** the files. Read them in any text editor, sync them via the user's preferred mechanism, commit them to Git.
- **Avoid Emacs.** Get the org-mode workflow benefit without the editor learning curve.
- **(For the author, additionally.)** Build the tool I want to use every day, and ship it in a form other people can also use.

### 2.3 Non-Users (v1)

Explicit non-audiences for v1.0 so scope stays honest:

- **Emacs power users** who want a non-Emacs companion. The Emacs ecosystem already serves them; mobile companions (beorg, Orgzly) cover the on-the-go gap. Orgsidian does not aim to replace their `.emacs.d`.
- **Mobile-first users.** No iOS/Android app in v1; pair with beorg/Orgzly if mobile capture is needed.
- **Teams needing real-time collaboration.** No CRDT, no multi-user editing. Single-user, file-based.
- **Markdown-native users uninterested in org-mode.** Obsidian, Logseq, and others serve this audience well.
- **Enterprises needing SSO, audit logs, or admin controls.** Orgsidian is consumer-grade OSS.

### 2.4 Key User Journeys

*Numbered globally as UJ-1 through UJ-N. FRs reference journeys by ID inline ("realizes UJ-3"). All journeys assume macOS or Linux (Windows arrives in v1.0); the user has installed Orgsidian and pointed it at a folder of `.org` files (the **Vault**) or started from a **Starter Vault**.*

- **UJ-1. Mara opens her day.**
  Mara is a freelance UX consultant. She opens her laptop at 09:15 and launches Orgsidian. The app opens directly on the **Today Dashboard**: scheduled items for today (one client call at 11:00, one deliverable due EOD), three TODOs flagged for today, the clocked-in task from yesterday afternoon still resumable, and her inbox of three captures from yesterday evening. She clicks the deliverable task. The Editor opens to that Headline in its source file, with surrounding notes visible. She starts the Clock. **Edge case:** if Orgsidian was last open with a stale Clock running overnight, on launch it prompts her to discard, adjust, or keep the running Clock — never silently records 14 hours.

- **UJ-2. Tiziano captures a thought without breaking flow.**
  Tiziano (author/dogfooder) is debugging an issue in another app. He presses `Cmd+Shift+Space` (the global hotkey). A small **Quick Capture** dialog appears centered on his screen. He types two lines describing the bug and a TODO to revisit tomorrow. He presses Enter. The dialog dismisses, the entry lands in his **Inbox** (a configured `.org` file), and he is back in the other app inside three seconds total. He does not launch Orgsidian.

- **UJ-3. Sofia ships a client report.**
  Sofia, a freelance researcher, has been tracking a 4-week engagement in Orgsidian: TODOs completed, hours clocked per subtask, notes linked from interview transcripts. Friday afternoon she opens the project file, presses the **Project Report** action, picks the date range (last 4 weeks) and the output format (PDF). A formatted report appears: completed work grouped by week, total hours, linked notes summarized, milestone status. She reviews, exports, attaches to her invoice email. The export took under one minute end-to-end. **Edge case:** if a clocked task has no end time (clock still running), the report flags it explicitly rather than guessing.

- **UJ-4. Alex tries Orgsidian for the first time.**
  Alex, a freelancer curious about org-mode, downloads Orgsidian and launches it. The first screen offers four **Starter Vaults**: *Personal GTD*, *Student*, *Freelancer*, or *Empty (use my own folder)*. Alex picks *Freelancer*. The app creates a folder, populates four `.org` files with realistic example content (one example project, an inbox, a journal, a someday list), and opens directly on the Today Dashboard — already showing example tasks and one example clocked item. The first thing Alex sees is the *workflow*, not the syntax. A small **Inline Coach** balloon points at the agenda: *"This is your day. Click any task to open the source file."* Alex clicks. The Editor opens in **Plain Mode** (essential commands only). Alex's first 5 minutes are spent doing real work, not reading documentation.

- **UJ-5. Mara survives an external file change.**
  Mara edits the same project file in VS Code on the side (occasionally needed for bulk find-and-replace). She saves the file in VS Code. Orgsidian, which had the file open with no unsaved changes, detects the external write within seconds, reloads the file, and refreshes the Agenda. Mara sees a small status note ("file reloaded from disk"). **Edge case:** if Orgsidian *did* have unsaved buffer changes when the external write occurred, it opens a **Merge Dialog** showing both versions side-by-side rather than silently overwriting either. This is the **Single Writer Rule + Dirty Buffer** pattern (see §4.4).

- **UJ-6. Riccardo searches across two years of notes.**
  Riccardo, a freelance developer, types `Cmd+P` and starts typing "kubernetes ingress". The **Search** surface shows full-text matches across all `.org` files (FTS5-backed), grouped by file, with the matched line highlighted. He picks one. The Editor opens to that line and the right sidebar shows **Backlinks** — every other note that references this Headline by ID or wiki-link. He follows a backlink. He is two clicks from a forgotten client engagement note that becomes the answer.

## 3. Glossary

*Downstream workflows and readers must use these terms exactly. FRs, UJs, and SMs use Glossary terms verbatim. If §4 introduces a new domain noun, add it here in the same pass.*

- **Vault** — A user-designated root folder containing the `.org` files Orgsidian indexes and operates on. One Vault open at a time in v1. May contain nested subfolders. Filesystem-native: the Vault is just a folder, readable and editable by any other tool.
- **Org File** — A plain-text file with `.org` extension, conforming to the org-mode syntax (headlines, properties, schedules, deadlines, TODO states, clocking, drawers, inline markup). The canonical source of truth for all content.
- **Headline** — An org-mode heading line beginning with one or more asterisks. Each Headline may carry a TODO state, tags, properties, scheduled/deadline timestamps, and clocked time entries.
- **TODO State** — A workflow keyword attached to a Headline (e.g., `TODO`, `DONE`, `NEXT`, `WAITING`). Customizable per Vault via in-file `#+TODO:` directive.
- **Scheduled Item** — A Headline with a `SCHEDULED:` timestamp. Surfaces on the Agenda from the scheduled date forward.
- **Deadline** — A Headline with a `DEADLINE:` timestamp. Surfaces on the Agenda with deadline-specific warnings.
- **Clock** — A time entry on a Headline, started and stopped by the user. Persisted as standard org `CLOCK:` lines inside the LOGBOOK drawer. The currently-running Clock is the **Active Clock**; at most one is active at a time.
- **Agenda** — A computed view across all Headlines in the Vault that have Scheduled or Deadline timestamps, filtered and grouped by date. The default agenda surface in Orgsidian.
- **Today Dashboard** — A specific Agenda view defaulting to today's items, plus the Inbox, plus the Active Clock if any. The first screen on app launch.
- **Inbox** — A user-designated `.org` file (default: `inbox.org` at Vault root) that receives Quick Capture entries.
- **Quick Capture** — A lightweight, OS-level capture surface (global hotkey, optional system tray menu) that appends a new entry to the Inbox without focus-stealing the main application.
- **Backlink** — A reference from one Headline to another, either by `id:` property link or by `[[wiki-link]]`. Backlinks are indexed bidirectionally.
- **Graph View** — A visualization of the Vault's `:ID:`-keyed Headlines as nodes and `[[id:...]]` / `[[wiki-link]]` references as edges. The third view in the *one object, three views* model (outline / agenda / graph). Any node opens its source Headline via Click-to-Source.
- **Refile** — A user-triggered operation that moves a Headline (and its subtree) to a different location, either elsewhere in the same file or into another file. The canonical org-mode triage primitive: it is how an Inbox entry becomes a real project task. Target is selected via a fast picker (file + outline path).
- **Index** — A SQLite database derived from the Vault contents. Caches parsed structure (Headlines, properties, timestamps, links) for fast agenda and search queries. Always rebuildable from the `.org` files; never the source of truth.
- **Starter Vault** — A pre-populated Vault template shipped with Orgsidian (Personal GTD, Student, Freelancer, Empty). Used during onboarding to give the user immediate working content rather than a blank canvas.
- **Project Report** — A user-triggered export (PDF or HTML) summarizing TODO completions, clocked time, linked notes, and milestone status for a selected scope (a file, a Headline subtree, or a tag) over a selected date range.
- **Editor Mode** — One of: **Raw** (plain `.org` source), **Pseudo-WYSIWYG** (syntax-highlighted source with rendered headings/links/checkboxes), **Split** (Raw + rendered preview side-by-side). User-switchable per file.
- **Plain Mode / Power Mode** — A UI progressive-disclosure setting. Plain Mode hides advanced commands, properties drawers, and rarely-used keybindings; Power Mode exposes everything. Toggle in Settings.
- **Single Writer Rule** — A storage invariant: Orgsidian is the sole writer of files it has open with unsaved buffer changes. External writes to *unchanged* files reload automatically; external writes to files with **Dirty Buffer** state trigger the Merge Dialog.
- **Dirty Buffer** — In-memory editor state for a file that diverges from the file on disk because of unsaved user edits.
- **Plugin Pattern (internal)** — An architectural commitment in v1.0: Orgsidian's own features are built against a hooks-and-registry system internally, even though no public plugin API is exposed yet. This positions a public API in v1.5+ without requiring a rewrite.

## 4. Features

*Each subsection is a coherent feature: behavioral description first, FRs nested under it. FRs are numbered globally so downstream artifacts (epics, stories, architecture components) have stable references. Phasing (v0.1 Alpha → v0.5 Beta → v1.0) is captured in §6 MVP Scope; in this section, **all FRs describe the v1.0 product** unless an `[ASSUMPTION]` says otherwise.*

### 4.1 Editor & Org-mode Fidelity

**Description:** The core surface. A code-editor-grade text editor (CodeMirror 6 is the design intent) that opens `.org` files and renders them with awareness of org syntax: TODO states, tags, timestamps, drawers, inline markup, headings, lists, checkboxes, links. Three Editor Modes give the user the choice between source-faithful editing (Raw), comfort (Pseudo-WYSIWYG via syntax highlighting), and learning (Split). All modes write the same canonical `.org` text — no proprietary format, no lossy transform. Realizes UJ-1, UJ-2, UJ-3, UJ-5, UJ-6.

The fidelity bar is **non-lossy round-trip**: any file Orgsidian opens and saves without user edits must be byte-identical to the original (modulo trailing-newline normalization, which is documented). This is the explicit lesson learned from Logseq's lossy org dialect; it is the trust contract with the org community.

**Functional Requirements:**

#### FR-1: Open and parse `.org` files

A user can open any `.org` file from the Vault and see it rendered correctly per the org-mode syntax conventions supported by Orgsidian (subset documented; see §10 Open Questions on parser coverage). Realizes UJ-1, UJ-5.

**Consequences (testable):**
- Opening a representative org file (with headlines, TODO states, scheduled timestamps, drawers, inline markup, links) renders without parse errors visible to the user.
- A corpus of community-standard org files (e.g., the org-mode manual, org-roam example vaults) opens without parse errors in 95%+ of files; any failures log a structured warning and fall back to plain-text view rather than crashing.

#### FR-2: Round-trip preservation

Files saved by Orgsidian without user-visible edits are byte-identical to their on-disk version (modulo trailing-newline normalization, documented in Settings).

**Consequences (testable):**
- A round-trip test (open file → save without editing → diff) yields zero changes on the documented corpus.
- Round-trip preservation is enforced by automated CI on every release.

#### FR-3: Switch Editor Modes

A user can switch the current file between Raw, Pseudo-WYSIWYG, and Split modes via a UI control and a keyboard shortcut. The default mode is Pseudo-WYSIWYG; the choice is persisted per file. Realizes UJ-4.

**Consequences (testable):**
- Mode switch completes in under 200ms on a 5,000-line org file.
- Per-file mode preference persists across app restarts.

#### FR-4: Inline rendering in Pseudo-WYSIWYG mode

In Pseudo-WYSIWYG mode, the editor renders headings with hierarchical font sizing, TODO-state badges, tags as pill labels, timestamps as readable dates, checkboxes as toggleable widgets, and links as clickable underlined text — while the underlying buffer remains the source `.org` text.

**Consequences (testable):**
- Cursor placement, copy-paste, and find/replace operate on source positions, not rendered positions.
- Toggling a checkbox widget updates the source `- [ ]`/`- [X]` syntax and re-renders.

#### FR-5: Cross-platform keybindings with optional Emacs mode

Default keybindings follow desktop platform conventions (Cmd on macOS, Ctrl on Linux/Windows; single-letter modifiers). An optional "Emacs keybindings" mode is selectable in Settings and re-binds editor actions to Emacs-style chords (`C-x C-s`, `C-c C-c`, etc.) for muscle-memory continuity.

**Consequences (testable):**
- Default keybindings documented in a single in-app reference panel.
- Emacs keybindings mode covers the actions a typical org-mode user touches daily (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out); gaps documented.

**Feature-specific NFRs:**
- **Performance:** Opening a 5,000-line org file renders the first screen in under 300ms on a baseline laptop (2020+ M1 / equivalent x86_64). Typing latency under 30ms (perceptual budget for code editors).
- **Reliability:** No crash on malformed input — fall back to a "treat as plain text" view with a warning banner.

**Notes:**
- `[ASSUMPTION: parser choice is uniorg or tree-sitter-org with a custom semantic layer; resolved in §10 Open Questions OQ-1 and addendum §A.2.]`
- `[NOTE FOR PM]: ProseMirror-based true WYSIWYG is explicitly deferred to v1.5+; see §5 Non-Goals and addendum §A.4.]`

### 4.2 Planner Core — Agenda & Today Dashboard

**Description:** The differentiator. Orgsidian opens on the Today Dashboard, not on a file list, not on a graph view. The Agenda is a computed view across the entire Vault: every Headline with a Scheduled or Deadline timestamp, grouped by date, filterable by tag, TODO state, and file. Time-tracking (Clock) is a peer first-class concept — at any moment a user may clock into a Headline; the Active Clock is visible persistently in the UI (in a toggleable status bar). Realizes UJ-1, UJ-3.

This feature embodies the positioning. Tasks, time, and notes are peers — Agenda is the surface that proves it.

**Functional Requirements:**

#### FR-6: Today Dashboard on launch

Launching the app opens the Today Dashboard as the default view (configurable in Settings to start on last-open file instead). The dashboard shows: items Scheduled for today, items with Deadline today or overdue, items flagged with a "today" tag (configurable), the Inbox preview (first N entries), and the Active Clock if any. Realizes UJ-1.

**Consequences (testable):**
- On launch, Today Dashboard renders within 500ms on a Vault of 1,000 files (cached index).
- Each section is collapsible; user preferences persist.
- Empty-state messages exist for each section ("No tasks scheduled for today — nice." style; see §4.5 Onboarding for tone).

#### FR-7: Agenda views — Today, Week, Custom

A user can switch the Agenda between Today, Week (rolling 7 days), and Custom (date range picker). Filters by tag, TODO state, and file path are composable.

**Consequences (testable):**
- Switching views completes in under 200ms on a 1,000-file Vault.
- Filters persist within session; user can save named filter presets.
- v0.5 Beta ships a **Done-This-Week** and **Done-This-Month** default named filter preset (filter: `:DONE:` + date range) as a first-class review surface; the weekly-review JTBD depends on it.

#### FR-8: Clock in, clock out, clock resume

A user can clock into a Headline (start an Active Clock), clock out (stop and record the entry as a `CLOCK:` line in the LOGBOOK drawer), and resume a previously paused clock. At most one Active Clock at a time; clocking into a new Headline auto-stops the prior. Realizes UJ-1, UJ-3.

**Consequences (testable):**
- Clock entries persist as standard org `CLOCK: [YYYY-MM-DD Day HH:MM]--[YYYY-MM-DD Day HH:MM] => HH:MM` lines in LOGBOOK drawer.
- An Active Clock that was running when the app was last closed is detected on next launch and prompts: discard / adjust end time / keep running. Realizes UJ-1 edge case.
- Time totals (per Headline, per subtree, per tag, per date range) are computed for use in Project Report (§4.3) and Agenda views.

#### FR-9: Schedule and Deadline on a Headline

A user can add, modify, or remove a Scheduled timestamp or Deadline on the current Headline via keyboard shortcut or context menu. A date picker affords fast entry; raw timestamp typing is also supported in Raw mode.

**Consequences (testable):**
- Modifications write standard org `SCHEDULED:` / `DEADLINE:` lines in the planning section under the Headline.
- Recurring timestamps (e.g., `<2026-05-19 Mon +1w>`) are preserved on round-trip and respected by Agenda (they show up on the next occurrence after completion).

#### FR-25: Refile a Headline

*(Added 2026-05-20 — numbered out of section order to preserve the stable global FR numbering downstream artifacts already reference. Conceptually belongs in this Planner Core section as the triage primitive that pairs with Quick Capture: Inbox → real project.)*

A user can trigger a Refile action on the current Headline (default keyboard shortcut) and select a target location via a fast picker (file + outline path / Headline). On confirmation, the Headline and its full subtree are moved to the chosen location and the source position is removed. The operation persists as a standard org-mode subtree move — no Orgsidian-specific metadata. Realizes the inbox-triage JTBD (§2.2).

**Consequences (testable):**
- Target picker shows file paths + outline paths; fuzzy-matches on both.
- Subtree integrity preserved: child Headlines, properties, drawers (LOGBOOK, PROPERTIES), and timestamps move with the parent.
- Refile triggers Single Writer Rule discipline (FR-16) on both source and destination files: dirty buffer state is respected, atomic write on completion.
- Undo restores the Headline to its prior location with byte-identical content.

**Feature-specific NFRs:**
- **Performance:** Agenda recomputation after a single-file edit completes in under 100ms on a 1,000-file Vault (incremental index update, not full rebuild).

**Notes:**
- `[ASSUMPTION: "today" tag is opt-in and configurable; no opinion on tag taxonomy beyond defaults.]`

### 4.3 Quick Capture, Search, Project Report

**Description:** The supporting trio that makes the planner+PKM integration usable in practice. Quick Capture eliminates the "open the app" friction for thoughts. Search (full-text + Backlink-aware) makes years-old notes findable. Project Report is the wow demo for v0.5 Beta — a one-click formatted export of a project's progress, designed for client-facing reuse. Realizes UJ-2, UJ-3, UJ-6.

**Functional Requirements:**

#### FR-10: Global Quick Capture

A user can invoke a Quick Capture dialog from anywhere on the OS via a configurable global hotkey (default `Cmd/Ctrl+Shift+Space`). The dialog appears centered, accepts multi-line text, and on submission appends the entry to the configured Inbox file. The main application does not steal focus. Realizes UJ-2.

**Consequences (testable):**
- End-to-end capture latency (hotkey → dialog visible → submit → entry persisted) under 1 second on a baseline laptop.
- The dialog dismisses on submit and on Escape; the user returns to the prior application.
- Captured entries include a creation timestamp drawer entry by default; format configurable.

#### FR-11: System tray quick-capture (optional)

On platforms that support it (macOS menubar, Windows tray, Linux indicator if available), a system tray menu offers Quick Capture as a fallback to the hotkey.

**Consequences (testable):**
- Tray entry is enabled by default and disable-able in Settings.
- Tray-launched capture is functionally identical to hotkey-launched.

#### FR-12: Full-text search across the Vault

A user can invoke search (default `Cmd/Ctrl+P` or `Cmd/Ctrl+Shift+F`) and type a query; matching results from across all `.org` files are returned, grouped by file, with the matched line previewed. Selecting a result opens the file at that line. Realizes UJ-6.

**Consequences (testable):**
- Search query latency two-tier on a 1,000-file Vault (SQLite FTS5 backing): under **100ms** for first 10 results (the streaming-results coherence budget — what the user sees first); under **200ms** for the full 50 results.
- Query syntax supports: plain words, exact phrase quotes, tag filter (`#tag:`), file filter (`file:`), TODO state filter (`todo:`).

#### FR-13: Backlinks for the current Headline

When the cursor is on a Headline, a sidebar panel shows all other Headlines that reference this one via `id:` link or `[[wiki-link]]`. Clicking a backlink navigates to the source. Realizes UJ-6.

**Consequences (testable):**
- Backlink panel updates within 100ms of cursor moving to a new Headline.
- Backlinks include both the linking Headline's title and a short context snippet.
- An **Unlinked References** sub-panel (text-matches on the current Headline's title that are not yet `[[link]]`-ified) is a v0.5+ extension of this FR; v0.1 ships only Linked Backlinks (explicit `id:` and `[[wiki-link]]` references).

#### FR-14: Project Report export

A user can select a scope (a file, a Headline subtree, or a tag) and a date range and trigger a Project Report export. Output formats: PDF and HTML in v1.0. The report includes: TODO completions in range, Clock entries summed per Headline and total, **linked notes presented as their Headline title plus a one-line context excerpt around the link reference, grouped by source file** (no LLM-generated summarization — see §5 Non-Goals), and milestone status (Headlines tagged as milestones if convention used). Realizes UJ-3.

**Consequences (testable):**
- Report generation for a typical scope (50 headlines, 4 weeks of activity) completes in under 5 seconds.
- Generated PDFs are visually consistent and printer-friendly (no clipping, readable typography).
- A running Active Clock with no end time is flagged explicitly in the report rather than guessed.
- Output formatting is customizable via a template file (CSS for HTML, template variables for header/footer). `[ASSUMPTION: template customization is a v1.0 feature, not v0.5.]`

#### FR-26: Backlink Graph View

*(Added 2026-05-20 — numbered out of section order to preserve stable global FR numbering. Conceptually belongs in this section as the visualization peer of FR-13 Backlinks.)*

A user can open a **Graph View** surface (e.g., from a sidebar action or a `Cmd/Ctrl+G` shortcut) that visualizes the Vault as a graph: nodes are Headlines that carry an `:ID:` property; edges are `[[id:...]]` and `[[wiki-link]]` references between them. The user can pan and zoom; clicking a node opens the source Headline in the Editor (**Click-to-Source**). Realizes the *one object, three views* commitment from §1 Vision (outline + agenda + graph). Ships in v0.1 Alpha as the third defining view.

**Consequences (testable):**
- Graph renders within 2 seconds on a 1,000-file Vault with ≤5,000 nodes; degrades gracefully (e.g., neighborhood-only view) beyond that.
- Click-to-Source navigates to the exact Headline (not just the file) using the `:ID:`-lookup invariant, not byte offsets — robust to external edits.
- Graph respects the same Vault-scope as Agenda and Search; no cross-Vault edges.
- Empty-state messaging when no `:ID:` properties are present points the user to documentation on how `:ID:` enables graph + backlink workflows (workflow over syntax — §1.5).

**Feature-specific NFRs:**
- **Privacy:** Quick Capture, Search, Project Report, and Graph View operate entirely locally; no network calls. Telemetry is opt-in; no defaults phone home (see §7 Constraints).

### 4.4 Storage & Index — Filesystem-native + SQLite

**Description:** Files are the source of truth. The Vault is a folder. `.org` files are plain text. SQLite is a derived index — a cache for fast Agenda queries and Search, rebuildable at any time from the files. The user can use Git, Syncthing, iCloud, Dropbox, or any other file sync mechanism without Orgsidian's awareness or interference. The **Single Writer Rule** protects integrity: while Orgsidian has a Dirty Buffer for a file, it is the sole writer; external writes to dirty files surface a Merge Dialog rather than silent overwrite. Realizes UJ-5.

This is the trust contract with the user. Their data is on their disk, in a format any tool can read, and Orgsidian cannot corrupt it.

**Functional Requirements:**

#### FR-15: Designate and open a Vault

A user can designate a folder as a Vault via the file picker on first launch or via Settings. Orgsidian recursively indexes all `.org` files in the folder. One Vault open at a time.

**Consequences (testable):**
- Initial indexing of a 1,000-file Vault completes in under 30 seconds on baseline hardware.
- Indexing progress is visible to the user during the initial scan.
- Subsequent launches with an unchanged Vault open the cached index instantly (under 1 second).

#### FR-16: Filesystem watcher with Single Writer Rule

Orgsidian watches the Vault folder for external file changes. When an external write is detected on a file with **no Dirty Buffer**, the file is reloaded automatically and the Agenda re-indexed. When an external write is detected on a file **with** a Dirty Buffer, the Merge Dialog opens. Realizes UJ-5.

**Consequences (testable):**
- External writes are detected within 5 seconds on macOS, Linux, and Windows.
- Reload on a clean buffer preserves cursor position if the line is unchanged; resets to top if the line was deleted.
- Merge Dialog presents three panes: **Yours** (Dirty Buffer, left), **External** (on-disk version, right), **Merged** (result, center). Diff hunks are detected and individually selectable: each hunk can be set to *use yours* or *use external*, and the Merged pane updates live. The user may also free-edit the Merged pane. Saving writes the Merged pane content atomically to the file and clears Dirty Buffer state; cancelling preserves the Dirty Buffer and leaves the file on disk untouched.

#### FR-17: SQLite index is fully derived

The index is never the source of truth. A user can delete the index file at any time; on next launch, Orgsidian rebuilds it from the `.org` files.

**Consequences (testable):**
- The index file lives in an OS-conventional location (e.g., `~/Library/Application Support/Orgsidian/index-{vault-hash}.db` on macOS); never inside the Vault folder by default.
- Deleting the index file and relaunching produces an identical Agenda and Search experience after a rebuild.

**Feature-specific NFRs:**
- **Reliability:** Power loss during a save must not corrupt the source `.org` file. Implementation: atomic write via temp-file-and-rename (the standard POSIX pattern). `[ASSUMPTION: cross-platform atomic write semantics are tractable on macOS, Linux, and Windows; addendum §A.3.]`
- **Data sovereignty:** No file in the Vault is read or written by Orgsidian without an explicit user-visible action (open Vault, edit, capture, save). The Vault directory is never enumerated to a network destination.

### 4.5 Onboarding — Workflow-first

**Description:** The lesson from the brainstorming first-principles pass: the barrier to org-mode is workflow, not syntax. Onboarding is therefore workflow-first. New users land on a working Today Dashboard populated by a Starter Vault, with Inline Coaching guiding the first 5 minutes through real activity (open a task, clock in, capture a thought, see it appear, run the agenda for tomorrow). The Interactive Tutorial — a 10-minute guided experience that has the user complete a real cycle (capture → triage → schedule → agenda → clock → report) — is the second touch. Plain Mode is the default; Power Mode is one click away. Realizes UJ-4.

**Functional Requirements:**

#### FR-18: Starter Vault selection on first launch

On first launch with no configured Vault, the user is presented with four Starter Vault choices: Personal GTD, Student, Freelancer, Empty. Selecting one creates a folder at a user-chosen location and populates it with pre-built `.org` files (one example project, an inbox, a journal, a someday list, agenda content populated for "today" relative to first-launch date). Realizes UJ-4.

**Consequences (testable):**
- Each Starter Vault opens to a non-empty Today Dashboard.
- The Freelancer Starter includes at least one example project with milestones, a clocked task, and a backlink — to demonstrate the integration immediately.
- `[ASSUMPTION: v0.1 Alpha ships Personal GTD + Student + Freelancer starters (Freelancer added 2026-05-20 per UX spec line 115 — required for the lighthouse persona to experience the full integration on first launch); Empty lands in v0.5 Beta. Scope expansion absorbed by the §6 v0.1 Alpha ceiling-with-compression discipline.]`

#### FR-19: Interactive Tutorial — workflow-first

A user can launch the Interactive Tutorial from a "Get started" menu item or first-launch prompt. The Tutorial walks them through one full workflow cycle: capture a thought, triage to a project, schedule it, see it in agenda, clock in/out, generate a one-line report. Estimated time: 10 minutes. Realizes UJ-4 (extended).

**Consequences (testable):**
- Tutorial completion is tracked locally (no telemetry); completion state is shown in Settings.
- Tutorial can be re-launched from Settings.
- `[ASSUMPTION: Tutorial is a v1.0 feature, not v0.5; Starter Vault carries the v0.5 onboarding load.]`

#### FR-20: Plain Mode / Power Mode toggle

The user-visible feature surface is reduced in Plain Mode (advanced commands, properties drawers, deep customizations hidden). Power Mode exposes everything. Toggle is in Settings; default is Plain Mode for new users.

**Consequences (testable):**
- Plain Mode hides a documented list of commands from menus and command palette; they remain available via direct keyboard shortcut for users who know them.
- Switching modes does not require app restart.

#### FR-21: Inline Coaching

Empty states (empty Today Dashboard, empty Inbox, never-clocked-in, never-searched) display contextual coaching text suggesting the next action. The command palette descriptions are written for discoverability ("Capture a thought from anywhere" rather than "Quick Capture").

**Consequences (testable):**
- Coaching text is dismissible per-context; "Don't show again" persists.
- A "show all coaching tips" reset action exists in Settings.

**Notes:**
- `[NOTE FOR PM]: Workflow Recipes (GTD/PARA/Zettelkasten/Weekly Review/OKR gallery) are explicitly deferred to v1.5+; this PRD does not commit to them.]`

### 4.6 Customization & Extensibility

**Description:** Themes (CSS-based), keybinding remapping, and an internal Plugin Pattern (architecturally committed in v1.0; publicly exposed as a Plugin API in v1.5+). The Plugin Pattern decision is foundational — Orgsidian's own v1.0 features are built against hooks-and-registry internally, so a v1.5 public API does not require a rewrite.

**Functional Requirements:**

#### FR-22: Theme — dark and light defaults, CSS customizable

Orgsidian ships with dark and light default themes. A user can supply a custom CSS file via Settings to override colors, fonts, and spacing. Theme switching is instant.

**Consequences (testable):**
- Dark/light defaults pass WCAG AA contrast for body text and primary UI chrome.
- Custom theme CSS is loaded from a user-specified file path; invalid CSS does not crash the app (falls back to default with a warning).

#### FR-23: Keybinding remapping

A user can remap any documented action to a different keybinding via Settings. Remappings persist per Vault.

**Consequences (testable):**
- Conflict detection: if a user assigns a chord that conflicts with an existing binding, a warning surfaces with the conflicting action.

#### FR-24: Internal Plugin Pattern (no public API in v1)

Orgsidian's own features (Agenda, Quick Capture, Search, Project Report, Themes) are implemented as internal plugins registered against a hooks-and-registry system. The system is not publicly documented or exposed in v1.0. Realizes a v1.5+ public API path.

**Consequences (testable):**
- A documented internal interface for "plugin = registers handlers for events" exists in the codebase.
- Adding a new internal feature does not require modifying core engine code (validated by the v0.5 → v1.0 transition).

**Notes:**
- `[ASSUMPTION: public Plugin API and a plugin marketplace are out of scope for v1.0. Confirmed in §5 Non-Goals and brainstorming §Theme 5.]`

## 5. Non-Goals (Explicit)

Things Orgsidian is *not* and will *not* do in v1.0. These prevent the "let me also add this nearby thing" drift at every level — epic, ticket, code.

- **Not an Emacs replacement.** No Lisp evaluation, no Emacs key compatibility beyond the documented Emacs keybindings option, no `org-babel` code execution.
- **Not a real-time collaborative editor.** No CRDT, no operational transform, no presence indicators, no comments-on-headlines. Single-user, file-based.
- **Not a mobile app.** No iOS or Android client in v1. Pair with beorg/Orgzly for mobile capture.
- **Not a sync service.** No proprietary sync, no Orgsidian-hosted account, no Orgsidian server of any kind. Users sync via their own choice (Git, Syncthing, iCloud, Dropbox, rsync).
- **Not a true WYSIWYG editor in v1.** Pseudo-WYSIWYG via syntax highlighting only. ProseMirror-based true WYSIWYG deferred to v1.5+ ([ASSUMPTION: brainstorming §Phase 4 budget rationale]).
- **Not a plugin marketplace.** Internal Plugin Pattern only in v1; public API and marketplace in v1.5+.
- **Not an AI/LLM-augmented editor in v1.** No semantic search, no LLM-suggested completions, no LLM-summarized notes. Architectural hook preserved for v1.5+; nothing user-visible.
- **Not a multi-Vault tool in v1.** One Vault open at a time.
- **Not a notes-only tool.** Notes without the planner integration is Obsidian's space; we do not compete there.
- **Not a planner-only tool.** Planner without the notes integration is NotePlan's space; we do not compete there.

## 6. MVP Scope

The product ships in three named milestones. Each is publicly released. The first one is the MVP for this PRD's purposes — the smallest credible artifact that validates the positioning hypothesis with real users.

**Budget realism note.** Each milestone's calendar budget (~160h Alpha, ~240h Beta, ~240h v1.0) was set in brainstorming Phase 4 against an itemized feature list that, summed, runs higher than the budget — typical of solo-OSS planning. Treat the budgets as *ceilings* that force feature compression, not as point estimates. The discipline (per §1.5 Design Principles): when a feature exceeds its share, cut scope, not quality. See addendum §A.7.

### 6.1 In Scope — v0.1 Alpha (MVP)

Target: Months 3-6 (~160h budget). Goal: first public release. Validates that real org-mode users will install a non-Emacs desktop tool and use it on their existing vaults without rage-quitting on fidelity issues.

- **FR-1, FR-2** — Open and parse `.org` files; round-trip preservation.
- **FR-3, FR-4** — Editor Modes (Raw + Pseudo-WYSIWYG; Split optional if budget allows).
- **FR-5** — Cross-platform keybindings (Emacs mode deferred to v0.5 if Alpha schedule slips).
- **FR-7** — Agenda views (Today and Week; Custom deferred to v0.5).
- **FR-9** — Schedule and Deadline editing.
- **FR-15, FR-17** — Vault designation, SQLite index rebuild-from-files.
- **FR-16** — Filesystem watcher (Single Writer Rule; Merge Dialog can be deferred to v0.5 if needed, in v0.1 fallback is "block save with conflict warning").
- **FR-18** — Starter Vault selection (Personal GTD + Student + Freelancer; Empty deferred to v0.5).
- **FR-22** — Dark + light themes (CSS customization deferred to v0.5).
- **FR-26** — Backlink Graph View (third defining view per §1 *one object, three views*; ships with Click-to-Source and basic pan/zoom).
- macOS + Linux packaging. Windows in v1.0.
- Public repository, README, landing page, basic documentation.

**Scope-expansion note (2026-05-20).** v0.1 Alpha absorbs two additions from the UX design spec — Freelancer Starter Vault (was v0.5) and FR-26 Graph View (newly added) — against the ~160h ceiling. Per §1.5 Solo-OSS Discipline and the §6 budget realism note, the ceiling is held by compression: if the integration of these two pushes the milestone past schedule, the response is to compress within FR-18 (smaller Freelancer sample), FR-26 (simpler layout algorithm), or to time-box the late additions and ship them in a v0.1.1 point release rather than to slip the v0.1 announcement.

**SM-1 success criterion (v0.1 Alpha):** Announcement post on HN/Reddit r/orgmode gathers at least 50 technical comments; at least 10 early adopters report using Orgsidian on their existing `.org` vaults for at least one week.

### 6.2 In Scope — v0.5 Beta

Target: Months 7-12 (~240h budget). Goal: daily-driver-grade tool, public Beta launch with a wow demo. Validates that the integrated planner+PKM proposition holds up under sustained use.

Adds:
- **FR-6** — Today Dashboard (full).
- **FR-8** — Clock in/out/resume.
- **FR-10, FR-11** — Quick Capture (global hotkey + tray fallback).
- **FR-12, FR-13** — Search (FTS5) + Backlinks.
- **FR-14** — Project Report export (wow demo for Beta launch).
- **FR-16** — Merge Dialog (full).
- **FR-18** — Starter Vault: Empty added (Freelancer shipped in v0.1).
- **FR-20, FR-21** — Plain/Power Mode + Inline Coaching.
- **FR-22** — Theme CSS customization.
- **FR-23** — Keybinding remapping.
- **FR-25** — Refile a Headline (paired with Quick Capture — the triage primitive that turns inbox captures into placed work).
- **FR-7 enhancement** — Done-This-Week / Done-This-Month default named filter presets (per §4.2 FR-7 note).
- **FR-13 enhancement** — Backlinks panel adds an Unlinked References sub-panel toggle (per §4.3 FR-13 note).

**Phasing note on time tracking.** Brainstorming Phase 4 explicitly moved time tracking to v1.0 for UX maturity reasons. v0.5 Beta ships **functional** Clock (start/stop/resume, persistence as standard org `CLOCK:` lines) because Project Report (FR-14) — the Beta wow demo — depends on clocked data. The polished time-tracking UX (persistent toggleable status bar, clock-time editing affordance, refined timer notifications) moves to v1.0 per the brainstorming roadmap swap.

**SM-2 success criterion (v0.5 Beta):** Author uses Orgsidian as their daily driver (5 days/week, ≥10 hours/week active editing) for 4 consecutive weeks; 100+ beta testers active; bug reports cluster in 3-5 areas (indicates real usage, not surface-level trial).

### 6.3 In Scope — v1.0

Target: Months 13-18 (~240h budget). Goal: public launch — the "official" announcement, polished and Windows-ready.

Adds:
- **FR-19** — Interactive Tutorial (workflow-first, 10 min).
- Time Tracking UI polish (persistent toggleable status bar, clock-time editing affordance).
- Starter Vault: full set (Personal GTD, Student, Freelancer, Empty) polished.
- Windows packaging + auto-update across all three platforms.
- Performance polish (Agenda <100ms on 1,000-file Vault; Search <200ms; file open <300ms).
- Comprehensive documentation site.
- Coordinated announcement: HN, ProductHunt, org-mode community channels.

**SM-3 success criterion (v1.0):** 1,000+ downloads in first 30 days post-launch; coverage in 1-2 productivity/org-mode newsletters; at least 3 external contributors with merged PRs within 60 days.

### 6.4 Out of Scope for MVP (and v1.0)

Deferred to v1.5+ or later. Each is here because it would be valuable but does not fit the v1.0 budget or positioning.

- **True WYSIWYG (ProseMirror + Org schema)** — v1.5+. Pseudo-WYSIWYG via syntax highlighting covers 80% of the experience at 20% of the cost (60-80h vs. 240-320h per brainstorming).
- **Public Plugin API + Marketplace** — v1.5+. Internal Plugin Pattern in v1 preserves the path.
- **Workflow Recipes gallery** — v1.5+. `[NOTE FOR PM]: emotionally load-bearing; revisit if v1.0 timeline permits.]`
- **AI/LLM features (semantic search, summarization, suggestions)** — v1.5+. Architectural hooks only in v1.
- **Self-hostable sync server** — v2+. Git/Syncthing/iCloud cover the use case for v1.
- **Mobile app (iOS/Android)** — v2+. beorg/Orgzly are the recommended pairing.
- **Multi-Vault** — v2+. One vault at a time in v1.
- **`org-babel` code execution** — out of scope permanently. Emacs owns this.
- **Real-time collaboration** — out of scope permanently. Different product.

## 7. Constraints and Guardrails

Cross-cutting guarantees the product makes to the user. These are non-negotiable for v1.0 and would be public commitments in the README.

### 7.1 Privacy

- **No telemetry by default.** Any future opt-in telemetry must be opt-in with an explicit consent UI and an in-app status display showing what is sent and when.
- **No network calls in the core workflow.** Open, edit, capture, agenda, search, report, save — none of these require network access. Auto-update checks are the only built-in network call and are disable-able.
- **No cloud account, ever.** Orgsidian does not have an account system.

### 7.2 Data sovereignty

- **The `.org` files are the source of truth.** The SQLite index is derived and disposable.
- **The Vault folder is the user's folder.** Orgsidian creates no files inside it without user action; configuration and index live in OS-conventional application support directories.
- **Round-trip preservation is enforced.** FR-2 is a hard contract.

### 7.3 Cost

- **Free, open-source, forever.** License: **MIT** (decided via architecture workflow, 2026-05-19; see architecture LD-1 and addendum §A.2). Maximally permissive license enables the v1.5+ Plugin API ecosystem; license-aligned with the chosen parser path (LD-3). Parser dependency sustainability is governed by architecture LD-48 (`tree-sitter-org` vendored as a SHA-pinned git submodule under `crates/orgsidian-parser/grammar/`, named parser-owner role, v0.3 fork-and-maintain dry run, in-house fork trigger if upstream stalls >6 months).
- **No paid tier, no SaaS, no premium plugins.** The author's commitment.
- **Optional voluntary funding mechanism** (GitHub Sponsors, Open Collective) may exist but never gates features.
- **Author capacity is roughly 10 hours per week.** The roadmap in §6 is paced to this constraint. Sustained over-budget months are a leading indicator that scope needs to compress (per §1.5 Design Principles), not that the author needs to push harder.

### 7.4 Reliability

- **Round-trip preservation (FR-2)** is the integrity contract.
- **Atomic file writes** via temp-file-and-rename on all platforms (FR-15 NFR).
- **The Single Writer Rule (FR-16)** prevents silent data loss in concurrent-edit scenarios.

## 8. Cross-Cutting Non-Functional Requirements

Requirements that apply across features and would otherwise repeat. Specific budgets are calibrated to baseline 2020+ hardware (Apple M1 / equivalent x86_64) and a Vault of 1,000 files unless noted.

- **Startup time (cold launch with cached index):** under 2 seconds to Today Dashboard interactive.
- **Editor typing latency:** under 30ms (perceptual code-editor budget).
- **Agenda recompute after single-file edit:** under 100ms (incremental index update).
- **Search latency:** under 200ms for first 50 results on 1,000-file Vault.
- **Quick Capture end-to-end:** under 1 second from hotkey to entry persisted.
- **Memory footprint:** under 500MB resident on a 1,000-file Vault under typical editing load. `[ASSUMPTION: feasible with chosen stack; benchmark in Spike 3.]`
- **Cross-platform parity:** v1.0 ships with feature-equivalent macOS, Linux, and Windows builds. Linux distribution via AppImage or Flatpak (deb/rpm best-effort). `[ASSUMPTION: packaging effort budget per brainstorming Action Plan 4.]`
- **Accessibility:** **WCAG 2.1 AA** for body text contrast and full keyboard navigation of all menus, dialogs, and primary surfaces — enforced as a **hard CI gate from v0.1 Alpha** (per UX spec Experience Principle 9 + §Accessibility, 2026-05-20). Three CI gates: (1) automated contrast-matrix verification across all theme tokens (dark + light defaults), (2) `axe-core` rules pass on every primary surface (Today Dashboard, Editor, Agenda, Quick Capture, Settings, Merge Dialog, Graph View), (3) Playwright keyboard-only scenarios cover the core flows (capture → refile → schedule → clock → search → report). Screen-reader semantics (ARIA roles, live regions) are best-effort in v0.1 and graduate by v1.0; `[NOTE FOR PM]: full screen-reader audit and assistive-tech certification deferred to v1.5+.]`
- **Internationalization:** UI strings extracted for translation in v1.0; default English; user-contributed translations welcomed via repo. Frontend i18n library: **Lingui v6.x** (decided via architecture workflow, 2026-05-19; see architecture LD-52). Translator-facing catalog format: **`.po` (Gettext)** at `packages/shell-ui/src/locales/{lng}/messages.po`, compiled to TypeScript at build time — the lingua franca expected by Crowdin / Weblate / Transifex, so community contributors are not forced into a project-specific format. `[ASSUMPTION: actual translations are community-driven, not author-shipped; the translation infrastructure ships in v1.0, populated locales arrive as community contributions.]`

## 9. Why Now

Three signals make 2026 the right window:

1. **Logseq's DB version dropped org-mode support** (active 2024-2025 community thread on discuss.logseq.com). An audience of Logseq-org users is actively shopping for an alternative.
2. **No new desktop-native `.org` editor has emerged since 2024.** Organice continues as a web app, Logseq pivoted, beorg/Orgzly are mobile-only. The desktop-native, cross-platform gap remains open and visible to anyone searching.
3. **Tauri 2.0 matured in 2024-2025**, making cross-platform desktop OSS in Rust meaningfully cheaper for a solo developer than Electron's resource footprint or Qt's UI complexity. The stack constraint that has historically blocked solo-OSS desktop tools has materially eased.

The risk of "wait and someone else does it" is low (the audience is small and the niche has been visible for years), but the risk of "another year of friction for org-curious users with no good option" is real and worth acting on now.

## 10. Open Questions

Numbered for tracking. Each is a future ticket or follow-up research, not a silent gap.

- **OQ-1. Parser choice and license implications.** ✅ **Resolved 2026-05-19** (architecture LD-1, LD-3, LD-48). Adopted Option (b): `nvim-orgmode/tree-sitter-org` (MIT, active fork, last push 2026-05-05) + a custom Rust semantic layer in `@orgsidian/core/src/parser/semantic/` filling TODO cycling, drawer types, deadline/scheduled semantics, link types, and table-formula gaps. uniorg rejected (GPL-3.0 contagion incompatible with Plugin-API strategy); `milisims/tree-sitter-org` rejected (archived 2024-02-11); fully custom parser deferred as fallback if tree-sitter-org coverage proves inadequate. The original 8-week Spikes 1-2 time-box is superseded; coverage measurement is now scoped as a side-task of OD-1 in the architecture document. **Maintenance contingency (LD-48):** grammar is vendored as a SHA-pinned git submodule at `crates/orgsidian-parser/grammar/` (no auto-bump, SHA review per upgrade); a named parser-owner role maintains grammar-source familiarity; v0.3 reserves 2 weeks for a fork-and-maintain dry run; in-house fork to `orgsidian-org/tree-sitter-org` triggers if upstream has no commits for >6 months at any v* milestone.
- **OQ-2. Stack — Tauri (Rust) vs. Electron (TypeScript).** ✅ **Resolved 2026-05-19** (architecture LD-1..LD-10). Adopted Tauri 2.x + Rust core/CLI + CodeMirror 6 in webview, locked by the architecture workflow. The original Spike 2 stack-comparison is reframed as ongoing CI matrix work in OD-2 of the architecture document (cross-webview CodeMirror 6 consistency under load). Parser-side dependency sustainability — coupled to this stack choice per the parser+stack co-decision rationale — is governed separately by architecture LD-48 (see OQ-1).
- **OQ-3. File watcher cross-platform reliability.** fsevents on macOS, inotify on Linux, ReadDirectoryChangesW on Windows all have known edge cases (renames, network mounts, case-folding filesystems). **Resolution:** Spike 2 in Months 1-2; documented limits.
- **OQ-4. Atomic write semantics on Windows.** POSIX temp-file-and-rename is canonical on macOS/Linux; Windows has historically been finickier. **Resolution:** investigated during Spike 2; if blocking, fall back to documented "write through" strategy with structured warning.
- **OQ-5. Org-mode syntax coverage scope.** Org's full spec is huge (babel, latex export, tables with formulas, drawer types, link types). What subset does v1.0 support, and what subset is documented "not supported, opens as plain text"? **Resolution:** explicit syntax-coverage matrix shipped with v0.1 Alpha README. **Operational fallback locked 2026-05-20 (UX spec Journey 4 + architecture LD-41):** files that fail the parser/semantic-layer contract are *quarantined* rather than dropped — opened in Raw mode with a banner offering "Attempt repair" / "Edit raw"; the Vault index records the quarantine so the file is excluded from Agenda/Search until repaired. The syntax-coverage matrix deliverable remains; the quarantined-malformed behavior is the runtime answer to "what happens when a file is outside the documented subset".
- **OQ-6. Project Report template customization.** v1.0 commits to template files: HTML/CSS for the HTML output path; Typst `.typ` templates with a documented `sys.inputs` schema for the PDF output path. The `.typ` schema is generated from the `ReportData` struct in `orgsidian-report` and ships as `docs/customization/report-templates.md` alongside the default `orgsidian-report-default.typ` template. (Per architecture LD-53; PDF rendering via `typst` embedded as a Rust library — see PDF rendering research at `_bmad-output/planning-artifacts/research/technical-pdf-rendering-crate-selection-research-2026-05-19.md`.) **Resolution:** drafting deliverable lands in the v0.5 Beta sprint based on Beta tester feedback.
- **OQ-7. Settings UI vs. config file.** ✅ **Resolved 2026-05-20** (UX spec Transferable UX Patterns, line 414 — VS Code / Sublime dual-surface pattern). Both ship: a Settings GUI dialog *and* a text config file. The **config file is authoritative**; the Settings GUI is a thin editor over it that round-trips faithfully (any change in the GUI writes the canonical config file format; any external edit to the config file refreshes the GUI on focus). Config file format and exact path are decided during the v0.5 Beta design pass; the dual-surface commitment is locked.
- **OQ-8. License of the project itself.** ✅ **Resolved 2026-05-19** (architecture LD-1). Adopted **MIT**. GPL-3.0 rejected (plugin-API contagion); Apache-2.0 considered but not chosen (overhead/verbosity not justified for an application-level project where the Rust stack is already broadly Apache-or-MIT-licensed). See §7.3 and addendum §A.2.
- **OQ-9. Pre-MVP spike outputs and acceptance criteria.** Spikes 1-2 (parser + stack, coupled per OQ-1), Spike 3 (filesystem watcher with Single Writer Rule cross-platform), Spike 4 (SQLite index benchmark on a synthetic 1,000-file vault for agenda + search query latency). Each spike needs a written acceptance bar (pass/fail criteria, success metric) before starting so the time-box has teeth. **Resolution:** spike plans published at the start of Month 1 as a `_bmad-output/spike-plans/` artifact set. **Spike 3 design invariants locked 2026-05-20** by UX spec (UXD-8 `:ID:`-lookup-at-navigation-time, not byte-offset; Journey 5 Merge Dialog 3-pane spec): the watcher spike's role is now narrowed to *platform reliability validation* (fsevents/inotify/ReadDirectoryChangesW edge cases — renames, network mounts, case-folding) rather than design discovery; the operational behavior is already designed.

## 11. Success Metrics

Each SM cross-references the FRs and milestone it validates. Counter-metrics counterbalance specific primary metrics.

**Primary**

- **SM-1.** Alpha public reception: at least 50 technical comments on the v0.1 Alpha announcement post; at least 10 early adopters self-reporting one week of use on their existing `.org` vaults. Validates the core fidelity bet (FR-1, FR-2) and the audience hypothesis (§2).
- **SM-2.** Beta daily-driver adoption: the author uses Orgsidian 5 days/week, ≥10 hours/week active editing, for 4 consecutive weeks at v0.5 Beta. 100+ external beta testers active. **Integration validation sub-criterion:** in ≥3 distinct sessions per week, the author touches all three peer concerns within the same session — at least one Agenda or task interaction, at least one Clock entry, at least one Backlink traversal or note-link follow. The sub-criterion guards against a daily-driver pattern that uses only the planner or only the notes side (which would still pass the headline metric). Together they validate the integrated planner+PKM proposition (FR-6 through FR-14).
- **SM-3.** v1.0 launch: 1,000+ downloads in first 30 days; coverage in 1-2 productivity or org-mode newsletters; at least 3 external contributors with merged PRs within 60 days. Validates that v1.0 is publish-worthy and community-attractive.

**Secondary**

- **SM-4.** Round-trip preservation passes on a community-standard org corpus (org-mode manual, org-roam example vaults) at every release. Validates FR-2 as a hard contract.
- **SM-5.** Median monthly bug-report-to-fix latency under 14 days through v0.5 and v1.0. Validates that the project is alive enough for community contribution.

**Counter-metrics (do not optimize)**

- **SM-C1.** Feature count. *Counterbalances SM-3.* The temptation at v1.0 will be to ship more features for the launch; the discipline is that v1.0 ships *fewer* features done well over more features done halfway. A 30-feature v1.0 with three crashes is worse than a 20-feature v1.0 with zero. Adding a feature should require removing one or extending the timeline; never compress quality.
- **SM-C2.** Public Plugin API timeline. *Counterbalances "contributors want extensibility now."* Holding the public Plugin API to v1.5+ is intentional. Releasing an unstable plugin API in v1.0 to attract contributors means breaking changes that destroy contributor trust. Do not optimize for "more plugins" in v1.0.
- **SM-C3.** Emacs-user conversion rate. *Counterbalances community pressure to court the Emacs audience.* If Emacs power users adopt Orgsidian, that is fine but not a success signal. Optimizing for that audience (Emacs key parity, `org-babel` support, etc.) would distort the product away from its real audience.

## 12. Assumptions Index

Every `[ASSUMPTION]` from the document, surfaced for explicit confirmation. Tiziano: please review and correct each.

- **§4.1 FR-5 Notes** — Parser choice is uniorg or tree-sitter-org with a custom semantic layer; resolved in OQ-1 and addendum §A.2.
- **§4.2 FR-7 Notes** — "today" tag is opt-in and configurable; no opinion on tag taxonomy beyond defaults.
- **§4.3 FR-14** — Project Report template customization is a v1.0 feature, not v0.5.
- **§4.4 FR-15 NFR** — Cross-platform atomic write semantics are tractable on macOS, Linux, and Windows.
- **§4.5 FR-18** — v0.1 Alpha ships Personal GTD + Student + Freelancer starters (Freelancer added 2026-05-20 per UX spec for the lighthouse persona); Empty lands in v0.5 Beta. Scope expansion absorbed by §6 ceiling-with-compression discipline.
- **§4.5 FR-19** — Interactive Tutorial is a v1.0 feature, not v0.5; Starter Vault carries the v0.5 onboarding load.
- **§4.6 FR-24 Notes** — Public Plugin API and a plugin marketplace are out of scope for v1.0.
- **§5 Non-Goals** — True WYSIWYG deferred to v1.5+ on the basis of brainstorming Phase 4 budget rationale.
- **§7.3 Cost** — License: GPL-3.0 if uniorg parser is adopted; MIT or Apache-2.0 if non-GPL parser path proves viable. Tied to OQ-1.
- **§8 Cross-Cutting NFRs — Memory footprint** — Sub-500MB feasible with chosen stack; benchmark in Spike 3.
- **§8 Cross-Cutting NFRs — Cross-platform parity** — Packaging effort budget per brainstorming Action Plan 4.
- **§8 Cross-Cutting NFRs — i18n** — Translation infrastructure is a v1.0 concern; actual translations are community-driven.

---

*End of PRD body. Architectural depth, parser license analysis, and stack rationale live in `addendum.md` alongside this file.*
