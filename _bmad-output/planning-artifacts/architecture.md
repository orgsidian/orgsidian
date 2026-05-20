---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/addendum.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/reconcile-brainstorming.md
  - _bmad-output/brainstorming/brainstorming-session-2026-05-18-1613.md
workflowType: 'architecture'
project_name: 'orgsidian'
user_name: 'Tiziano'
date: '2026-05-19'
lastStep: 8
status: 'complete'
completedAt: '2026-05-19'
revisions:
  - date: 2026-05-19
    summary: Sprint Change Proposal (correct-course) absorbed. LD-5 amended with repo-visibility timing (private during pre-Alpha, flipped to public at v0.1 Alpha tag). LD-33 updated with full CC-enforcement + git-cliff chain. NEW LD-54 (Conventional Commits + CHANGELOG mapping). NEW LD-55 (GitHub Issues sync + Project board). Cross-Cutting Concerns header pointer to `_bmad-output/test-artifacts/test-design.md` as authoritative system-level test strategy. Project Tree amended with commitlint/cliff/husky/sync-issues config. No body content of LD-1..LD-53 modified. See `_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md`.
---

# Architecture Decision Document — Orgsidian

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Reframing constraint

The PRD and addendum (2026-05-19) were calibrated around a **solo developer at ~10h/week** with an 18-month budget. The project will instead be **100% spec-driven** (AI-agent implementation). This invalidates the velocity-based justifications that drove several decisions: contributor-pool size, "default fallback" to mainstream tooling, deferral of work on cost grounds. Under the new regime, **spec correctness and long-term operational soundness are the binding constraints**, not implementation velocity.

This Project Context Analysis re-derives the architectural picture under that reframing, validated by a roundtable critique (Winston/Amelia/Murat/Mary) and external research (Tauri/CM6/tree-sitter-org/SQLite FTS5/Windows atomic-write).

### Requirements Overview

**Functional Requirements — 24 FRs across 6 feature groups:**

- **Editor & Org-mode Fidelity (FR-1 to FR-5):** parser + round-trip preservation + Editor Modes (Raw / Pseudo-WYSIWYG / Split) + inline rendering + cross-platform keybindings with optional Emacs mode. Pseudo-WYSIWYG via CodeMirror 6 decorators is **retained for product-positioning reasons** (lighthouse persona wants to see `.org` source, not hide it) — *not* on the original cost grounds.
- **Planner Core (FR-6 to FR-9):** Today Dashboard on launch + Agenda views (Today / Week / Custom) + Clock (in/out/resume) + Schedule/Deadline editing.
- **Quick Capture, Search, Project Report (FR-10 to FR-14):** OS-level global hotkey + system tray fallback + FTS5 full-text search + Backlinks + PDF/HTML report export.
- **Storage & Index (FR-15 to FR-17):** Vault designation + filesystem watcher with Single Writer Rule + SQLite as fully-derived index.
- **Onboarding (FR-18 to FR-21):** Starter Vault (4 templates) + 10-min Interactive Tutorial + Plain/Power Mode + Inline Coaching.
- **Customization & Extensibility (FR-22 to FR-24):** dark/light themes (CSS-overridable) + keybinding remap + **internal Plugin Pattern** (implementation) + **public Plugin API spec published in v1.0** (exposed externally in v1.5+).

**Non-Functional Requirements — architecturally load-bearing:**

- **Performance budgets** (baseline 2020+ M1 / x86_64, 1000-file Vault): startup <2s cold, typing <30ms, agenda recompute <100ms incremental, search two-tier (<100ms first 10 results, <200ms full 50 results — PRD §4.3 FR-12 + §8 post-2026-05-20), capture end-to-end <1s, memory <500MB resident, **Graph View ≤2s for 5,000 nodes force-directed render** (FR-26; LD-56).
- **Round-trip preservation (FR-2)** enforced by automated CI on every release — non-negotiable trust contract with the org community.
- **Cross-platform parity** for v1.0: macOS + Linux + Windows feature-equivalent. macOS + Linux only in v0.1 Alpha and v0.5 Beta.
- **Data sovereignty:** no telemetry by default, no network calls in core workflow, no cloud account ever.
- **Reliability:** atomic file writes on all platforms; power-loss and AV-interference recovery are hard requirements.
- **Accessibility:** WCAG 2.1 AA contrast + keyboard navigation of menus and primary surfaces; screen reader best-effort.
- **Internationalization:** string extraction infrastructure from v1.0; translations community-driven.

**Scale & Complexity:**

- Primary domain: **cross-platform desktop application** in Rust (Tauri 2.x).
- Complexity: **high**. Custom editor surface bidirectionally bound to source text, derived index with filesystem watcher, stringent performance budgets, plugin architecture mandatory from day 1, three target platforms with subtly divergent webview/filesystem behaviors.
- Estimated architectural components: **~15-18 first-class modules**.

### Locked Architectural Decisions

Decisions that emerge from PRD + addendum, validated and refined by the Party Mode + research pass. Each is treated as a foundational invariant for the rest of this document.

- **LD-1. License: MIT.** Rationale: maximally permissive license enables a healthy plugin ecosystem (the strategic payoff of FR-24 and v1.5+ Plugin API exposure), maintains enterprise adoption, broadest ecosystem compatibility, and minimum operational overhead (no NOTICE file, no modified-file marking, ~170-word license body). **GPL-3.0 explicitly rejected** under the new regime: plugin-contagion would chill the ecosystem the Plugin API is designed to enable. **Apache-2.0 considered and not chosen**: patent grant + retaliation are valuable but the overhead and verbosity didn't pay for themselves for an application-level project where the Rust stack is already broadly Apache-or-MIT-licensed; MIT aligns naturally with CodeMirror 6 (MIT), tree-sitter-org (MIT), and the JS-leaning org-mode community.
- **LD-2. Stack: Tauri 2.x + Rust** for `@orgsidian/shell`. Rationale: memory footprint advantage 5-10× over Electron (validated: 30-50 MB idle vs 200-450 MB), coherence with Rust-native parser and SQLite binding, mature in production for editor-grade CM6 apps (verified: `kelvink96/markora`, `zhitongblog/solomd`). Operational costs (WebKitGTK version churn, contenteditable focus quirk on Linux) are absorbed via CI matrix and a documented known-issues page.
- **LD-3. Parser: `nvim-orgmode/tree-sitter-org` (MIT, active fork, last push 2026-05-05) + custom Rust semantic layer** in `@orgsidian/core/src/parser/semantic/`. Rationale: license-compatible with LD-1, prior art for the semantic-layer pattern exists in `nvim-orgmode/orgmode` (Lua), tree-sitter provides incremental parsing for large files at zero cost. **uniorg rejected** (GPL-3.0 incompatible with LD-1); **`milisims/tree-sitter-org` rejected** (archived 2024-02-11); **fully custom parser deferred** as fallback if tree-sitter-org coverage proves inadequate in Spike 1.
- **LD-4. Index: SQLite via `rusqlite`** (FTS5 built-in, better ergonomics than `better-sqlite3`). PRAGMAs locked: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`. FTS5 tokenizer: `unicode61 remove_diacritics 2` + `porter` (default English; per-Vault overridable). Application-level FTS5 sync (no triggers on external-content tables).
- **LD-5. Monorepo: `@orgsidian/core` (pure logic) + `@orgsidian/shell` (Tauri app) + `@orgsidian/cli` (headless CLI, reopened per Party Mode).** In-process boundary between core and shell. CLI consumes `core` only — no shell dependency. GitHub organization `orgsidian` (newly created — Story 1.13) hosting the monorepo at `orgsidian/orgsidian`; **repo is private during pre-Alpha development and flipped to public at the v0.1 Alpha release tag** (Story 6.10, before SM-1 announcement). Org reserved as namespace for v2+ ancillary repos. Boundary enforcement via `eslint-plugin-boundaries` equivalent for Rust (workspace member visibility rules) + CI checks for cyclic dependencies.
- **LD-6. Editor surface: CodeMirror 6** with Pseudo-WYSIWYG via decorators/widgets. Round-trip fidelity is the source-of-truth contract; widget toggling between `Decoration.replace` ↔ `Decoration.widget` is a known sharp edge (CM discuss #6504) that must be exercised by tests at the v0.1 Alpha gate. Mandatory recipes locked: `WidgetType.eq()` shallow-equal on widget props, `Transaction.userEvent` for widget-triggered changes, no `view.dispatch` inside `update()` while `view.composing` is true, `widget.ignoreEvent() === false` for interactive widgets. Multi-cursor + widget interactions documented as a known limitation in v0.1 (codemirror/dev #111).
- **LD-7. Single Writer Rule + Dirty Buffer + Merge Dialog** as the concurrent-edit integrity contract. External writes on a clean buffer auto-reload; on a dirty buffer trigger the Merge Dialog (three-pane: Yours / External / Merged with hunk-level resolution). Race-condition surface tested deterministically via injected clock and synthetic external-write events; chaos tests offline. **Cross-file extension (LD-57, added 2026-05-20 for FR-25 Refile):** when an operation writes two files atomically (Refile: source loses subtree + destination gains subtree), both files must be clean before the operation begins (dirty → save-first prompt) and the rollback discipline of LD-57 applies.
- **LD-8. Atomic writes: `atomic-write-file` crate** (Rust) + 3-retry exponential backoff wrapper for AV/Search-indexer transient locks (the dominant real-world failure mode on Windows). `MoveFileExW` *not* officially guaranteed atomic — wrapper handles platform differences.
- **LD-9. File watcher: `notify-rs`** (Rust-native, the Tauri default). Watcher abstraction layer in `core` allows deterministic fakes for unit tests; integration tests use golden traces recorded from real external editors (vim, VS Code, Emacs save sequences). Network mounts and case-folding filesystems documented as v0.1 unsupported configurations.
- **LD-10. Plugin API designed in v1.0 as a versioned internal contract; NOT published to crates.io until v1.5+.** The `orgsidian-plugin-api` crate lives inside the monorepo as an internal workspace member throughout v0.1 → v1.4; all v1.0 features (Agenda, Capture, Search, Report, Theme) consume the same trait surface that will eventually be exposed externally — no parallel "private" hooks. SemVer discipline + contract tests + changelog tracked internally from day 1, but external publication and SemVer-1.0 lock happen only when v1.5+ exposes the API to third-party plugin authors. Rationale (Party Mode round 3): publishing a SemVer contract before real plugin authors exist is the anti-pattern that broke React Hooks, Vue Composition API, Svelte runes — the trait shape must absorb feedback from internal-plugin churn before lock-in.

### Open Decisions for Spike 1-2 (focused after research)

- **OD-1.** Coverage of `nvim-orgmode/tree-sitter-org` measured against a test corpus extracted from `org-mode/testing/lisp/test-org-element.el` (Emacs reference). Acceptance: ≥90% coverage on documented org-mode syntax subset; gaps documented in a `KNOWN_DIVERGENCES.md`. Side-product of Spike 1: a reusable test-fixture extractor.
- **OD-2.** CI matrix concretization for Tauri 2.x: macOS-arm64 + Ubuntu LTS + Arch Linux + Windows nightly. Acceptance: smoke + round-trip + perf snapshot on every PR (macOS + one Linux); full suite + Windows on nightly. WebKitGTK version pin documented.
- **OD-3.** `notify-rs` debounce strategy for atomic-save patterns (vim/VS Code/Emacs save sequences emit 3-12 events each). Golden-trace fixtures recorded; debounce window calibrated (likely 100-250ms). Single Writer Rule invariants exercised against these traces.
- **OD-4.** v0.5 Beta re-evaluation of true WYSIWYG (ProseMirror) based on Pseudo-WYSIWYG user feedback — not a Spike 1-2 item but a deferred decision point.

### Cross-Cutting Concerns

The system-level testing strategy consolidating Concerns #1-7 below, plus the risk-prioritized coverage plan for v0.1 → v1.0, is authored as a standalone artifact at **`_bmad-output/test-artifacts/test-design.md`** (TEA workflow, 2026-05-19). That document is the binding strategy for every story's red-phase scaffold (Process Discipline rule A); the LD entries below are referenced by it (not superseded). Implementing AI agents follow `test-design.md` § per-story-type scaffolds + this section's LD constraints jointly.

1. **Round-trip fidelity** — three-level test oracle (Murat): L0 byte-identical save-no-op (CI gate hard), L1 semantic-preserving surgical edit (property-based with `proptest` or `fast-check`), L2 Emacs ground-truth via `emacs --batch` AST comparison on a subset corpus.
2. **Single Writer Rule integrity** — deterministic race exercising via injected clock, three property tests (clean+ext-write → reload; dirty+ext-write → merge; save-during-pending → merge), plus chaos tests with jitter.
3. **Performance budgets** — synthetic CI gates (±10% regression on median of 5 runs, fixed 1000-file corpus) + offline weekly trend on a real 5000+ file corpus (org-roam vault).
4. **Cross-platform parity** — risk-based test matrix: macOS+Linux per PR, Windows nightly. Highest-risk cell: Windows + WebView2 + ReadDirectoryChangesW + atomic-write-from-external-editor (v1.0 blocker).
5. **Plugin API as public contract from v1.0** — contract tests per hook, semantic versioning, separate changelog. v1.0 features cannot bypass the API surface.
6. **Data sovereignty** — zero network calls in core paths verified by CI (network namespace sandbox); telemetry opt-in only with visible status UI; configuration and index outside Vault folder.
7. **i18n + a11y** — string extraction infrastructure from day 1; WCAG AA keyboard + contrast across all surfaces; screen reader best-effort with known gaps documented.
8. **Independent release pipelines** — `@orgsidian/core` published to crates.io (Rust) and optionally to npm as WASM (v2+ web playground); `@orgsidian/shell` shipped as desktop binaries with auto-update; `@orgsidian/cli` published to crates.io. Separate versioning, separate changelogs, separate cadence. Tooling: `cargo-release` or equivalent.
9. **Repo & GitHub org strategy** — single monorepo under org `orgsidian`; org reserved as namespace for v2+ ancillary repos (mobile companion, plugin registry metadata, docs site if it outgrows the monorepo).

### Component Inventory (~15-18 first-class modules)

In `@orgsidian/core`:
1. Parser layer (`tree-sitter-org` wrapper + semantic layer)
2. AST + Org-element model
3. Serializer (round-trip-faithful)
4. SQLite index layer (schema + migrations + FTS5)
5. Index sync engine (file → AST → SQLite, incremental)
6. Filesystem watcher abstraction (`notify-rs` adapter)
7. Single Writer / Dirty Buffer manager
8. Agenda query engine
9. Clock manager (Active Clock state, LOGBOOK persistence)
10. Search engine (FTS5 wrapper + query parser)
11. Backlinks engine
12. Project Report renderer (PDF + HTML pipelines)
13. Plugin registry / hook bus
14. Public Plugin API surface (versioned, contract-tested)
15. Atomic write subsystem (`atomic-write-file` wrapper + retry)

In `@orgsidian/shell`:
16. Editor surface (CodeMirror 6 + org decorators/widgets)
17. UI surfaces (Today Dashboard, Agenda views, Settings, Merge Dialog, Quick Capture window)
18. Theme engine + keybinding manager
19. OS integration (global hotkey, system tray, file pickers, auto-update)
20. Starter Vault generator + Interactive Tutorial state
21. i18n string registry

In `@orgsidian/cli`:
22. Headless CLI consumer of `core` (agenda, search, capture, report commands)

(22 line items above; "first-class modules" count ~15-18 once tightly-related ones are grouped — e.g., Index layer + Index sync + Schema migrations as one logical module.)

### Technical Constraints & Dependencies (post-research)

**Stack — locked:**
- Tauri 2.x + Rust for shell; CodeMirror 6 in webview for editor surface
- `tree-sitter-org` (nvim-orgmode fork) + custom Rust semantic layer for parser
- `rusqlite` for SQLite/FTS5 binding
- `notify-rs` for filesystem watcher
- `atomic-write-file` for atomic file writes (with AV-aware retry wrapper)
- License: MIT

**Platform targets:** macOS-arm64 + Linux-x86_64 (Ubuntu LTS, Arch) for v0.1 Alpha and v0.5 Beta; Windows-x86_64 added in v1.0.

**Distribution:** macOS DMG + Homebrew cask; Linux AppImage (primary) + Flatpak (best-effort); Windows MSI + auto-update via Tauri updater.

## Starter Template Evaluation

### Primary Technology Domain

Cross-platform desktop application with web-technology frontend and Rust backend. Stack already locked in §Project Context Analysis (LD-1 through LD-10): Tauri 2.x + Rust core/CLI, CodeMirror 6 in webview, MIT license.

### Version Policy (project-wide)

Every language, framework, library, SDK, or CLI tool pinned in the Orgsidian tech stack must be the **latest stable release OR the latest LTS release**, with LTS preferred when an LTS track exists.

**Exception — Tauri ecosystem (Tauri itself, official `tauri-plugin-*` plugins, `webkit2gtk-rs`):** breaking changes ship every 6-8 weeks with significant integration cost. These dependencies are pinned to a chosen stable minor at the time of an Orgsidian milestone release; bumps occur with each Orgsidian major/minor and require a documented changelog review. Exception scoped narrowly to the Tauri ecosystem only; all other dependencies follow the general rule.

A dedicated tech-stack reference document will formalize exact version pins and upgrade cadence; until then, this section is the source of truth.

### Starter Bootstrap Approach

**Selected: `pnpm create tauri-app@2` bootstrap, then refactor to multi-crate Cargo workspace as the second implementation story.** Rationale: scaffolder produces a maintainer-validated Tauri 2.x configuration (`tauri.conf.json`, signing config, CSP, updater hookup) and a working Rust↔JS dev loop; mechanical refactor to a Cargo workspace is well-trodden Rust territory and lower-risk than hand-rolling the entire Tauri integration.

### Frontend Framework — Locked

**Selected: React 19.1.x + TypeScript + Vite. No Spike 0 — lock now.** Rationale: under spec-driven AI-agent implementation, prior-art density translates directly to correctness density. React + CodeMirror 6 has the largest public corpus of integration examples and resolved edge cases; the shadcn/ui ecosystem (Radix-based, copy-paste source ownership) is mature on React. Solid's theoretical fine-grained-reactivity advantage was considered and not pursued.

**React-19-specific implementation rules** (per Party Mode round 2):
- `ref` is a regular prop in React 19 — do not use `forwardRef` for new components.
- `EditorView` instantiation must use idempotent `useEffect` cleanup to survive `StrictMode` double-mount in dev: `useEffect(() => { const view = new EditorView(...); return () => view.destroy(); }, [])`.

### Locked Stack Versions (subject to version policy)

| Layer | Version | Notes |
|---|---|---|
| Tauri | 2.10+ (latest stable minor at scaffold time) | Pinned per Tauri-ecosystem exception |
| Rust | latest stable | rustfmt + clippy enforced in CI |
| Node.js | 22.x LTS (Iron) | required by Vite / Vitest toolchain |
| pnpm | latest stable | package manager for JS workspace |
| TypeScript | 5.x latest stable | strict mode |
| React | 19.1.x | `ref`-as-prop, StrictMode-safe `EditorView` lifecycle |
| Vite | 6.x latest stable | upgrade to `rolldown-vite` when GA |
| CodeMirror 6 | latest stable (6.x) | `@codemirror/state` + `view` + `commands` + `language` + `search` |
| Tailwind CSS | 4.1.x | CSS-first config via `@theme` directive; Lightning CSS |
| shadcn/ui | latest (forked into `src/components/ui/`, essentials only) | |
| Zustand | 5.x | cross-cutting UI state; React 19 native |
| Vitest | 2.1.x | with `happy-dom` (CM6 `getComputedStyle` requirement; Bun test rejected on `vi.mock` ESM hoisting parity) |
| Playwright | latest stable | E2E + Tauri WebDriver integration |
| `rusqlite` | latest stable | `bundled` feature + FTS5 |
| `tree-sitter` | latest stable | with `nvim-orgmode/tree-sitter-org` grammar |
| `notify` (notify-rs) | latest stable | watcher |
| `atomic-write-file` | latest stable | atomic writes |
| `proptest` | latest stable | property-based testing |
| `rstest` | latest stable | parameterized testing |
| `insta` | latest stable | snapshot testing |
| `@lingui/*` | 6.x (`^6.0.1` at lock time 2026-05-19) | i18n (LD-52); `@lingui/core`, `@lingui/react`, `@lingui/cli`, `@lingui/vite-plugin`, `@lingui/swc-plugin`, `eslint-plugin-lingui` |
| `react-force-graph-2d` | `1.29.1` (pinned at lock time 2026-05-20) | FR-26 Backlink Graph View canvas + d3-force (LD-56); MIT |
| `@axe-core/playwright` | latest stable | WCAG axe-core integration into Playwright suite (LD-58); MIT |
| `toml` (crate) | latest stable | Settings TOML serialization (LD-40 amended 2026-05-20); MIT/Apache-2.0 |

### Cargo Workspace Layout — 8 Crates from Day 1

Decision: granular split (Amelia's proposal, validated by Winston on parallel-build + API-barrier grounds). One extra day of scaffolding; payoff in incremental compile times, compiler-enforced API barriers, and independent versioning for the published `orgsidian-plugin-api` crate.

```
orgsidian/
├── Cargo.toml                          # workspace root
├── pnpm-workspace.yaml                 # JS sub-workspace declaration
├── package.json
├── LICENSE                             # MIT
├── README.md
├── CONTRIBUTING.md
├── crates/
│   ├── orgsidian-parser/               # tree-sitter-org wrapper + semantic AST builder
│   ├── orgsidian-index/                # rusqlite + FTS5; schema + migrations + query API
│   ├── orgsidian-watcher/              # notify-rs wrapper + debounce + event coalescing
│   ├── orgsidian-vault/                # atomic-write-file + retry + Dirty Buffer manager
│   ├── orgsidian-plugin-api/           # trait definitions; INTERNAL crate (not published) until v1.5+
│   ├── orgsidian-core/                 # façade re-exports + integration glue + cross-crate orchestration
│   ├── orgsidian-cli/                  # bin: headless CLI (parse, index, query, validate-plugin)
│   └── orgsidian-shell-app/            # bin: Tauri app, consumes core, hosts shell-ui
├── packages/
│   └── shell-ui/                       # React 19 + CM6 + Tailwind 4 + shadcn/ui
└── tools/
    └── corpus-extractor/               # utility crate: extracts org-mode test corpus from Emacs sources
```

**Dependency wiring:** `shell-app` → `core`; `cli` → `core`; `core` → `parser` + `index` + `watcher` + `vault` + `plugin-api`; `plugin-api` is leaf (no project deps).

**Plugin API publication strategy:** `orgsidian-plugin-api` is an **internal** monorepo crate throughout v0.1 → v1.4 (not published to crates.io). SemVer discipline applied internally from day 1; the trait shape is allowed to evolve in response to internal-plugin feedback during v0.1 → v1.4 without breaking external contracts. Publication to crates.io with SemVer-1.0 lock occurs at v1.5+ when third-party plugin authors are first invited.

### Frontend Package Layout (`packages/shell-ui/`)

```
shell-ui/
├── src/
│   ├── components/
│   │   ├── ui/                         # shadcn/ui forked + stripped to essentials
│   │   └── org/                        # Orgsidian UI Kit — day-1 mandatory
│   │       ├── TodoStateCycler.tsx     # TODO/NEXT/DONE/WAITING cycle widget
│   │       ├── TagPillEditor.tsx       # tag pill input with autocomplete
│   │       ├── OrgDatePicker.tsx       # date picker with +1w / +1d shortcuts
│   │       ├── PropertyDrawer.tsx      # :PROPERTIES: drawer editor
│   │       ├── ClockEditor.tsx         # clock entry editor (start/end/duration)
│   │       ├── HeadlineRenderer.tsx
│   │       └── ScheduleDeadlineBadge.tsx
│   ├── surfaces/                       # full-screen surfaces
│   │   ├── TodayDashboard/
│   │   ├── Agenda/
│   │   ├── Editor/                     # CM6 host
│   │   ├── Settings/
│   │   ├── QuickCapture/               # separate Tauri window
│   │   ├── MergeDialog/                # custom focus mgmt (3-pane hunk navigation)
│   │   └── CommandPalette/             # cmdk via shadcn
│   ├── coaching/
│   │   ├── coachingRegistry.ts         # centralized inline-coaching content + dismissal conditions
│   │   └── CoachingSlot.tsx
│   ├── stores/                         # Zustand
│   │   ├── clockStore.ts               # Active Clock
│   │   ├── viewStore.ts                # current Agenda view, sidebar state
│   │   ├── settingsStore.ts            # Plain/Power Mode, theme path
│   │   └── coachingStore.ts            # dismissed coaching IDs
│   ├── themes/
│   │   ├── tokens.css                  # default --org-* CSS variables
│   │   ├── dark.css
│   │   └── light.css
│   ├── styles/
│   │   ├── app.css                     # Tailwind 4 @import + @theme
│   │   └── reset.css
│   ├── ipc/                            # Tauri command wrappers (typed)
│   └── main.tsx
├── public/
├── index.html
├── tsconfig.json
├── vite.config.ts
└── package.json
```

### Orgsidian UI Kit — Day-1 Mandatory

The `src/components/org/` directory is a first-class subpackage from day one, not retrofitted later. Rationale (Sally's Party Mode contribution): these widgets are *org-mode-specific*, not generic UI; future plugin authors (v1.0 spec-published Plugin API, v1.5+ exposed) must be able to import and compose them. Burying them inside feature folders forces a painful refactor at v1.0.

### Themable CSS Token Vocabulary (FR-22)

User CSS overrides loaded from `~/.orgsidian/user.css` (or per-Vault path) after the bundle; Lightning CSS cascade resolves naturally.

```
/* Backgrounds */
--org-bg-canvas         /* main editor / surfaces */
--org-bg-surface        /* sidebars, panels */
--org-bg-elevated       /* dialogs, popovers */

/* Foregrounds */
--org-fg-default
--org-fg-muted
--org-fg-subtle
--org-fg-headline-h1 .. --org-fg-headline-h6

/* Accents */
--org-accent-todo
--org-accent-next
--org-accent-done
--org-accent-waiting
--org-accent-tag
--org-accent-link
--org-accent-property

/* State */
--org-state-error
--org-state-warning
--org-state-success

/* Borders */
--org-border-default
--org-border-focus
```

### UI Mode Pattern — Plain/Power (FR-20)

`<body data-mode="plain"|"power">` driven by `settingsStore`. Tailwind 4 selectors `data-[mode=plain]:hidden` toggle visibility. Advanced controls remain in the DOM at all times — only visibility flips. Preserves keyboard-shortcut muscle memory across modes (a "hidden" Power-only command remains reachable by its shortcut).

### Inline Coaching Pattern (FR-21)

Centralized `coachingRegistry.ts` maps coaching IDs to content + dismissal conditions; `<CoachingSlot id="..." />` is the only API used in surfaces. Enables future A/B testing of coaching copy and dismissed-ID telemetry under opt-in (PRD §7.1).

### Tauri Plugins — Full Set

- `tauri-plugin-fs` — filesystem access (gated to Vault folder via allow-list)
- `tauri-plugin-dialog` — native file pickers, alerts
- `tauri-plugin-global-shortcut` — Quick Capture global hotkey (FR-10)
- `tauri-plugin-updater` — auto-update mechanism (v1.0)
- `tauri-plugin-window-state` — window position/size persistence
- `tauri-plugin-store` — settings persistence (FR-21 theming preferences, dismissed coaching IDs)
- `tauri-plugin-shell` — `open()` external links and file attachments
- `tauri-plugin-os` — platform detection for keybindings (Cmd vs Ctrl)
- `tauri-plugin-clipboard-manager` — clipboard read/write for capture flows
- `tauri-plugin-log` — structured logs for user-side debug
- `tauri-plugin-process` — `restart()` post-update

NOT added until justified by a story: `tauri-plugin-http`, `tauri-plugin-notification`.

### Initialization Command (First Implementation Story)

```bash
pnpm create tauri-app@2
```

Interactive prompt values:
- Project name: `orgsidian`
- Identifier: `com.orgsidian.app`
- Frontend language: TypeScript / JavaScript
- Package manager: pnpm
- UI template: React
- UI flavor: TypeScript

### First Implementation Stories (preview — stories formalized in `bmad-create-epics-and-stories`)

**Story 1: Bootstrap Tauri 2.x React+TS scaffold.**
- Repo initialized via `pnpm create tauri-app@2` (React + TS, identifier `com.orgsidian.app`).
- `LICENSE` (MIT) + `README.md` at root.
- `pnpm tauri dev` launches the Tauri window on macOS-arm64 and Ubuntu-LTS.
- CI baseline (GitHub Actions): `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `pnpm typecheck`, `pnpm test` on macOS-arm64 + Ubuntu-LTS.

**Story 2: Refactor to multi-crate Cargo workspace + JS sub-workspace.**
- Root `Cargo.toml` with 8 member crates (parser, index, watcher, vault, plugin-api, core, cli, shell-app); empty stubs except `shell-app` receiving the scaffolded `src-tauri/`.
- `pnpm-workspace.yaml` declares `packages/shell-ui/` as JS workspace member; scaffolded `src/` moves there.
- Dependency wiring per the diagram above.
- Tailwind 4 + shadcn/ui (forked into `src/components/ui/`, essentials only) installed.
- Lingui v6.x installed (`@lingui/core`, `@lingui/react`, `@lingui/cli`, `@lingui/vite-plugin`, `@lingui/swc-plugin`, `eslint-plugin-lingui`) per LD-52; `lingui.config.ts` initialized with `en` as source locale; `lingui extract` + `lingui compile` wired into `pnpm` scripts; SWC plugin entry added to `vite.config.ts` (`react({ plugins: [["@lingui/swc-plugin", {}]] })`) + `lingui()` Vite plugin; `eslint-plugin-lingui` added to ESLint config.
- Full Tauri plugin set installed.
- `src/themes/tokens.css` created with the FR-22 variable vocabulary.
- Directory tree documented in `CONTRIBUTING.md`.
- `cargo build --workspace` passes; `pnpm tauri dev` still launches.

**Story 3: Wire CodeMirror 6 host in `packages/shell-ui/src/surfaces/Editor/`.**
- CM6 base extensions installed.
- `<Editor>` React component with StrictMode-safe `EditorView` lifecycle and `ref`-as-prop pattern.
- Smoke test: opens a sample `.org` file with no transformations; round-trips byte-identical on no-op save (precursor to FR-2 CI gate).

(Stories 4+ enter org-mode-specific territory — parser wiring, index bootstrap, agenda first iteration — and are out of scope for the Starter Template Evaluation.)

## Core Architectural Decisions

Resolved during Party Mode round 3 (Amelia + Winston + Mary) with eight modifications applied to the initial proposal. Decisions LD-11..LD-36 below incorporate that resolution.

### Decision Priority Analysis

**Critical (block implementation Story 4+):**
LD-11 SQLite schema, LD-12 migrations, LD-13 rebuild policy, LD-14 connection management, LD-16 async runtime, LD-17 fs allow-list, LD-24 IPC type bridge (`tauri-specta`), LD-26 Plugin API trait shape.

**Important (shape architecture significantly):**
LD-15 AST cache, LD-18 CSP, LD-25 plugin loading model (static workspace crates in v1.0), LD-27 CLI command tree, LD-28 window strategy, LD-29 routing (TanStack Router), LD-32 CI matrix (subset+nightly), LD-33 release automation.

**Deferred (post-v1.0 or non-blocking):**
- LD-25 WASM plugin loader (v1.5+, when external plugin authors first invited).
- `orgsidian-plugin-api` crates.io publication (v1.5+).
- LD-23 telemetry (added in v1.5+ if and only if a real backend exists).
- LD-36 crash reporting (added in v0.5+ if self-hosted Sentry backend available; otherwise v1.5+).
- Chocolatey/Scoop distribution (post-v1.0).

### Data Architecture

**LD-11. SQLite schema — normalized.** Tables: `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `vault_meta`, `_schema_version`. FTS5 virtual tables `fts_headlines` and `fts_content` (external content, application-managed sync — no triggers, per Amelia's Round 1 finding). Indices on `(file_path)`, `(headline_id)`, `(scheduled_date)`, `(deadline_date)`, `(tag, headline_id)`. Schema lives in `crates/orgsidian-index/sql/schema.sql`; typed query API in Rust.

**LD-12. Migrations via `rusqlite_migration` (1.3+).** Zero-extra-dep crate on top of `rusqlite`, MIT-licensed, ~500 LOC, mature. API: `Migrations::new(vec![M::up(...)]).to_latest(&mut conn)`. SQL files at `crates/orgsidian-index/migrations/NNNN_description.sql`; schema versioning via `PRAGMA user_version`. Forward-only (index is rebuildable from `.org` files, so no down-migrations needed). Rationale (Party Mode round 3, Amelia): hand-rolled runner duplicates ~150 LOC of well-tested library code for zero benefit; "total control" is confidence theater when migrations are mostly `CREATE TABLE` statements.

**LD-13. Rebuild policy.** Incremental via filesystem watcher under normal operation. Full rebuild triggered by: (a) `PRAGMA user_version` mismatch with code expectation; (b) `PRAGMA integrity_check` failure on startup; (c) explicit user command (Settings UI + CLI `orgsidian index rebuild`). Full rebuild on a 1000-file Vault completes within the FR-15 NFR budget (<30s on baseline hardware).

**LD-14. Connection management.** Single dedicated writer task (the indexer); reader pool via `deadpool-sqlite` (default size 4). PRAGMAs from LD-4: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`.

**LD-15. AST cache.** In-memory LRU keyed by `(path, mtime)`, default 64 entries (configurable). CM6 owns the editor-buffer state for the currently-open file. No disk cache beyond SQLite.

**LD-16. Async I/O & concurrency.** Tokio runtime (Tauri default). `tokio::fs` for watcher + indexer paths. CPU-bound work (parsing 5000-line org files, serializing large headlines): `tokio::task::spawn_blocking` to avoid blocking the async runtime.

### Security & Sandboxing

No authentication (local-only app, no user accounts — PRD §7.1). Security model is filesystem allow-list + Tauri CSP + signed binaries.

**LD-17. Tauri `fs` plugin allow-list.** Scope limited at runtime to the user-selected Vault folder + OS-standard config/data/log directories. No filesystem access outside these scopes from the webview.

**LD-18. Content Security Policy.**

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline' file://*;
connect-src 'self' https://updates.orgsidian.app;
img-src 'self' data: file://*;
font-src 'self' file://*;
```

`'unsafe-inline'` on `style-src` required by Tailwind 4's atomic CSS injection; `file://` on style/img for user theme CSS and attachments. `connect-src` allow-lists only the updater endpoint.

**LD-19. Code signing.**
- macOS: Apple Developer ID Application certificate + `notarytool` notarization.
- Windows: code signing cert (standard initially; EV upgrade evaluated at v1.0).
- Linux: GPG-signed checksums + AppImage signature.
- Signing keys stored as GitHub Actions secrets; release pipeline signs artifacts.

**LD-20. Auto-update.** `tauri-plugin-updater` with Tauri key pair (private signs releases, public embedded for verification). Single `stable` channel in v1.0. Update check disable-able from Settings (PRD §7.1).

**LD-21. Vault path constraints.** Symlinks followed by default (toggle in Settings, per addendum §A.5). Network-mounted Vault folders documented as unsupported in v0.1; polling-based fallback in v1.0. Case-folding filesystems (macOS+Windows defaults) detected; path comparisons case-insensitive for these platforms.

**LD-22. User CSS.** Loaded from `~/.orgsidian/themes/*.css`. Threat model: data exfiltration via `url()` on hover/load. Mitigation: CSP `connect-src 'self'` blocks remote requests; `img-src` restricted to `file://*`. No parser-level sanitization — relies on CSP.

**LD-23. Telemetry: none in v1.0.** No UI, no toggle, no instrumentation code, no backend. Rationale (Party Mode round 3): shipping an "infrastructure-disabled with non-functional UI toggle" is a documented dark-pattern footgun for privacy-conscious communities (Logseq 2024 storage-mode toggle, Audacity 2021 Muse Group telemetry — both generated more user anger than the underlying decisions, perceived as manipulative). Orgsidian's target audience (org-mode + privacy-conscious freelancers) has near-zero tolerance for ghost UI. Telemetry reintroduced as a clean opt-in feature in v1.5+ if and only if a real backend exists at that time.

### IPC, Plugin Loading, CLI

**LD-24. Tauri IPC with `tauri-specta` (2.x).** `#[tauri::command]` for frontend→Rust RPC; `app.emit()` for backend→frontend events. JSON serialization via `serde`. **End-to-end typed bridge via `tauri-specta`**: `collect_commands![...]` macro + `Builder::new().commands(...).export(...)` generates the TypeScript client with full signature derivation (args, return type, error type) from Rust definitions. Supports generics, lifetimes, tagged enums. Replaces a manual wrapper layer entirely (eliminates ~300 LOC of hand-synced drift-prone boilerplate at the ~30-50 command scale). Rationale (Party Mode round 3, Amelia): `ts-rs` generates struct→TS only and forces manual command wrapping; `tauri-specta` is the 2026 standard for Tauri 2.x with native integration since late 2024.

**LD-25. Plugin loading model — v1.0: static linking.** Plugins in v1.0 are **regular workspace crates** (no `cdylib`, no `libloading`, no FFI). The host maintains a `Vec<Box<dyn OrgsidianPlugin>>` registry built at compile time from the workspace's bundled-plugin crates. Rationale (Party Mode round 3, Amelia): v1.0 plugins are internal-only and bundled with the app — the dynamic-loading machinery (cdylib + libloading) carries every cost of unstable Rust ABI, `catch_unwind` boundaries, `Cargo.lock` lockstep across host and plugin, and zero symbol versioning while providing zero benefit (no hot-reload, no user-installable plugins). **v1.5+: first real plugin loader is WASM via `wasmtime`** — sandboxed, cross-platform, message-passing-native. The `OrgsidianPlugin` trait (LD-26) is designed from day 1 to be WASM-compatible (message-passing semantics, no synchronous callbacks) so the v1.5+ transition is mechanical, not architectural.

**LD-26. Plugin API trait — hook-with-priority + observer hybrid** (in `orgsidian-plugin-api` crate, internal to the monorepo until v1.5+ per LD-10):

```rust
pub trait OrgsidianPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn init(&mut self, ctx: PluginContext) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;

    /// Plugin priority for hook dispatch ordering; default 0.
    /// Lower values run first; ties resolve by load order.
    fn priority(&self) -> i32 { 0 }

    /// Fire-and-forget observer. Default no-op.
    /// Used for logging, badges, sync-to-external, etc.
    fn on_event(&mut self, event: &Event) -> Result<()> { Ok(()) }

    /// Pre-save hook — plugin may transform content before write.
    fn on_save_before(&mut self, ctx: &HookContext, content: &str)
        -> Result<HookOutcome<String>> { Ok(HookOutcome::Continue) }

    /// Pre-capture hook — plugin may transform a Quick Capture entry before commit.
    fn on_capture_before(&mut self, ctx: &HookContext, entry: &CaptureEntry)
        -> Result<HookOutcome<CaptureEntry>> { Ok(HookOutcome::Continue) }

    /// Agenda query transform — plugin may post-process query results.
    fn on_agenda_query_after(
        &mut self,
        ctx: &HookContext,
        query: &AgendaQuery,
        results: &mut Vec<AgendaItem>,
    ) -> Result<()> { Ok(()) }
}

pub enum HookOutcome<T> {
    Continue,
    Replace(T),
    Cancel(String),
}
```

**`Event` enum is `#[non_exhaustive]`** to allow future event types as a minor bump rather than a major break. v1.0 covers: `FileOpened`, `FileSaved`, `FileChanged`, `HeadlineEdited`, `ClockStarted`, `ClockStopped`, `CaptureSubmitted`, `AgendaQueried`, `IndexRebuilt`. `HookContext` (passed by reference) exposes: `query_index()`, `read_vault_file()`, `emit_event()`, structured `tracing` logger; no mutable references handed to plugins to keep the surface WASM-compatible (message-passing semantics for the v1.5+ transition).

**Rationale (Party Mode round 3):** Orgsidian's reference plugin community is **Emacs** (org-mode native culture), not Obsidian or VS Code. Emacs uses `:before/:after/:around` advice + hooks + custom commands — observer-only covers ~10-20% of expected plugin use cases (logging, badges, reactive sync). The remaining 60%+ (format-on-save, validate-before-commit, capture-template-expansion, agenda transform, custom export pipelines) require **hooks that modify host behavior** (`HookOutcome::Replace` or `Cancel`). Priority ordering + non-exhaustive events are forward-compatibility hedges learned from VS Code (~200+ event surfaces and growing) and Obsidian (~50 and growing).

**SemVer policy** (internal during v0.1 → v1.4, public from v1.5+):
- New `Event` variant: minor.
- New optional trait method with default impl: minor.
- Removed event, removed method, changed signature, changed semantics: major.
- Deprecation cycle: at least one minor before removal.

**LD-27. CLI command tree** (`orgsidian-cli` via `clap` derive macro):

- `orgsidian parse <file>` — parse + emit AST as JSON
- `orgsidian index {init | rebuild | stats | integrity}`
- `orgsidian query {agenda <range> | search <query> | backlinks <headline-id>}`
- `orgsidian validate-plugin <path>` — contract test runner for plugin authors
- `orgsidian vault {info | init}`

Output: human-readable default; `--json` flag for scripting. CLI is the primary integration-test surface for `orgsidian-core` (per Murat: cheaper than Playwright/Tauri WebDriver).

### Frontend Architecture (additions to step 3)

**LD-28. Window management.** Two Tauri windows in v1.0: `main` (editor + Today Dashboard + Agenda + Settings + Merge Dialog) and `quick-capture` (separate, lightweight, single-input — supports FR-10 latency budget <1s). Future plugin-spawned windows possible in v1.5+ via Plugin API.

**LD-29. Routing: TanStack Router (latest stable).** End-to-end type-safe router for the `main` window. Surfaces: `/today` (default), `/agenda/$view`, `/editor/$filePath/$headlineId?`, `/graph` (FR-26; LD-56), `/settings/$section`. Typed search params (`?date=`, `?tag=`, `?todo-state=`), typed loader data (parsed file content, AST), compiler-checked `<Link>` targets with `params={{filePath}}`. Quick Capture window is single-surface, no router. Routes are state-driven; URLs reflect current surface for deep-linking from coaching/help links. Rationale (Party Mode round 3, Amelia): with ~5-7 surfaces in a desktop app (no SSR, no streaming), TanStack Router's compiler-enforced route + param + search safety eliminates the runtime-cast drift inherent to React Router v7's `useParams() → Record<string, string | undefined>`.

**LD-30. Virtualization.** **`@tanstack/react-virtual`** for Agenda views (1k+ scheduled items), Search results, and Backlinks panel. CM6 handles editor internal virtualization natively (viewport-based decoration rendering).

**LD-31. IPC frontend consumption.** The TypeScript client generated by `tauri-specta` (LD-24) is consumed directly from frontend code: `import { commands } from '@/lib/tauri'; const file = await commands.openFile(path);`. No separate manual wrapper layer in `src/ipc/`. Custom error handling and retry logic, where needed, wraps individual call sites — not the bridge itself.

### Infrastructure & Deployment

**LD-32. CI matrix** (GitHub Actions):

- **Per-PR (target: <90s wall-clock total)**: `cargo build/test/clippy -- -D warnings/fmt --check`, `pnpm typecheck/test`, **round-trip subset gate** (~100 representative files from the corpus, <60s), **perf snapshot regression gate** (±10% on median of 5 runs), **a11y hard gate** (LD-58 — contrast-matrix + axe-core + happy-path keyboard scenarios on the 6 primary surfaces; PRD §8 post-2026-05-20) — runs on macOS-arm64 + Ubuntu-LTS.
- **Nightly (full)**: full matrix including Windows + Arch Linux + Ubuntu-LTS; **full round-trip corpus gate** (~2000 assertions extracted from `test-org-element.el`); perf trend dashboard; L2 oracle round-trip test via `emacs --batch` on a subset corpus.
- **Merge gate**: PR can only merge if (a) per-PR job is green AND (b) most recent nightly is green within last 24h. Stale-nightly (>24h failing) blocks all merges to main.
- **Release**: triggered by version tag; builds artifacts per platform, signs, publishes to GitHub Releases.

Rationale (Party Mode round 3): per-PR full-corpus gate is the pattern that atrophies under merge-pressure (rust-analyzer, biomejs, ruff all converged on subset-per-PR + nightly-full + merge-gate-on-nightly). 2000-assertion corpus runs at minimum 3-5 minutes per matrix cell on free GitHub Actions runners — gate would be disabled "just this once" within 6 months.

**LD-33. Release automation.** **`cargo-release`** for the Rust workspace (workspace-aware versioning). All Rust crates (including `orgsidian-plugin-api`) share the app version with tag scheme `v*` during v0.1 → v1.4; `orgsidian-plugin-api` is internal to the monorepo and not published to crates.io until v1.5+. At v1.5+, `orgsidian-plugin-api` separates with its own SemVer cadence and tag scheme `plugin-api-v*` when external publication begins. JS `shell-ui` version-synced with `shell-app`. CHANGELOG.md per crate + project root. CHANGELOG generation is fully automated via **`git-cliff`** (`cliff.toml` at repo root) consuming Conventional Commits (see LD-54) on every `cargo release`. CHANGELOG manual entries (`Deprecated` / `Security`) inserted before tag in `cargo release` hook. See LD-54 (commit enforcement chain) and LD-55 (GitHub Issues sync + Project board) for the surrounding workflow.

**LD-34. Distribution channels.**
- macOS: DMG via Tauri bundler + Homebrew cask in `orgsidian/tap`.
- Linux: AppImage (primary) + Flathub manifest (best-effort).
- Windows: MSI via Tauri bundler. Chocolatey/Scoop deferred post-v1.0.

**LD-35. Logging.** **`tracing`** + **`tracing-subscriber`** structured logs (Rust). Output to OS-standard log directory (`~/Library/Logs/Orgsidian/` macOS, `~/.local/share/orgsidian/logs/` Linux, `%APPDATA%\Orgsidian\logs\` Windows). Rotation: 7 days, max 50MB. Verbosity `info` default; override via `RUST_LOG=debug` env or Settings UI. Frontend logs bridge through `tauri-plugin-log` into the same files.

**LD-36. Crash reporting: not in v0.1.** Optionally added in v0.5+ as `sentry-rust` opt-in (default disabled, explicit Settings toggle) if a self-hosted Sentry backend is available; otherwise deferred to v1.5+. v1.0 either ships with crash reporting (because it landed in v0.5) or doesn't — no infrastructure-disabled half-state shipped (same discipline as LD-23).

**LD-52. i18n library: Lingui v6.x.** Frontend localization in `packages/shell-ui/` uses `@lingui/core` + `@lingui/react` with the SWC macro plugin. Vite integration: `@vitejs/plugin-react-swc` with `["@lingui/swc-plugin", {}]` + `@lingui/vite-plugin` for catalog compilation. Catalog format: `.po` (Gettext) in `packages/shell-ui/src/locales/{lng}/messages.po`; compiled to `messages.ts` at build time (zero runtime parser, ~3 kB total runtime footprint). Authoring API: `<Trans>…</Trans>` JSX, `<Plural value… one… other… />`, and `useLingui()` for imperative `` t`…` ``. Natural-language IDs (no manual key trees). `eslint-plugin-lingui` enforces extractability at lint time and is a CI gate. `lingui extract --clean && git diff --exit-code` is a CI gate to prevent catalog drift. Default locale `en` statically imported at boot; other locales lazy-loaded via dynamic `import()` keyed by `navigator.language` + Settings override. Rationale: (a) compile-time message compilation + 3 kB runtime is the smallest fit for the Quick Capture cold-start budget (FR-10, LD-28); (b) ICU MessageFormat at the catalog layer keeps translators on the lingua franca expected by Crowdin/Weblate/Transifex (PRD §8 community-driven translations); (c) natural-language IDs eliminate the namespace+key-tree authoring overhead that an AI-agent solo workflow pays per string, with no compile-time check that the chosen key is sensible; (d) Lingui v6.0 (April 2026) ships native React 19 support, first-party Vite plugin, and a maintained SWC plugin compatible with the `@vitejs/plugin-react-swc` we depend on per stack lock. **react-intl rejected** for this iteration: equivalent ICU expressiveness but the AOT no-parser path is opt-in (requires a Vite alias), runtime is 2–3× larger, and authoring requires explicit per-call IDs. Kept as a clearly bounded fallback if `@lingui/swc-plugin` incompatibility surfaces in Spike 1 (catalog format remains ICU-compatible, so translations port without rework). **i18next rejected**: runtime-default extraction, ICU support is plugin-gated (not native), runtime footprint 6–7× Lingui's, and namespace+key-tree authoring imposes a per-string ceremony tax on the AI-agent workflow. **Fluent (`@fluent/bundle` + `@fluent/react`) rejected**: FTL syntax diverges from ICU, raising friction for community translators (PRD §8) and AI-agent string authoring; runtime parser stays in the bundle; slowest release cadence of the four candidates (no release activity April–May 2026 vs weekly cadence for the alternatives). **`fluent-rs` not applicable**: all localized strings live in the React webview; the Rust core returns structured data, not localized text. Numbered LD-52 (not LD-37) because LD-37..LD-51 were already issued during round-5 hardening at the time this amendment was drafted (2026-05-19); placed here next to the frontend infrastructure decisions for locality.

**LD-53. `orgsidian-report` PDF rendering: `typst` embedded via `typst-as-lib`.**

**Decision.** The `orgsidian-report` crate (LD-14) renders FR-14 Project Report PDFs using the Typst typesetting system embedded as a Rust library. Direct deps pinned per the project version policy: `typst@0.14`, `typst-pdf@0.14`, `typst-as-lib@0.15` (Apache-2.0; allowlist-compatible per LD-1 / LD-37). All rendering is in-process; no subprocess, no native deps, no Python or Qt runtime. Closes the Important Gap previously tracked at the "PDF rendering crate selection" entry below and the corresponding "Areas for Future Enhancement" note.

**Why typst over the other four spike candidates** (verified 2026-05-19, research at `_bmad-output/planning-artifacts/research/technical-pdf-rendering-crate-selection-research-2026-05-19.md`):

- **`wkhtmltopdf` (subprocess) — disqualified.** Upstream repository archived 2023-01-02; org archived 2024-07-10; last binary release 2020-06; depends on Qt 4.8.5 + patched QtWebKit (EOL since 2015). Shipping an unmaintained ~10-year-unpatched in-process browser engine violates the supply-chain hygiene posture set by LD-37.
- **`weasyprint-rs` — does not exist.** No Rust crate of that name on crates.io; no Rust FFI binding to WeasyPrint exists. Only integration path is a Python subprocess, which requires bundling a Python 3 runtime + native Pango/cairo/HarfBuzz alongside the Tauri binary — incompatible with the LD-2 single-binary packaging posture and the no-native-deps pattern established by LD-26 / LD-30 / LD-48.
- **`genpdf` (original) — abandoned.** Last release 0.2.0 in 2021-06; no commits since. Active fork is `genpdfi` 0.2.7 (2026-01).
- **`genpdfi`** — pure-Rust, maintained, small footprint, but layout is Rust-code-only (no template surface for OQ-6 customization), no bidi/RTL shaping (font fallback chain only), typography baseline materially below typst. Not adequate for FR-14 wow-demo bar at PRD §4.3 acceptance.
- **`printpdf` 0.9.1** — strong second choice; pure-Rust, maintained, includes an experimental `html` feature (`PdfDocument::from_html(...)`) that would align cleanly with OQ-6's HTML/CSS template intent. Loses on (a) typography polish (general-purpose PDF generator, not a typesetting engine), (b) bidi/RTL not first-class — adequate for Latin-script v0.5 Beta but a v1.0 PRD §8 community-translation liability, (c) the `html` feature's CSS subset is a moving target less suited to a "must look professional" wow demo. Retained as the documented fallback if a v0.5 Beta typst risk materializes (see "Downgrade path" below).

**Why typst is the right fit, mapped to locked LDs and PRD:**

- **FR-14 acceptance — "readable typography"** (prd.md FR-14 consequences). Typst is a typesetting system (Knuth-Plass line-breaking, `rustybuzz` shaping, kerning, ligatures, hyphenation). Output is comparable to LaTeX/InDesign quality; the FR-14 wow demo lands as a wow demo.
- **PRD §8 community translations (v1.0).** First-class bidi via `text(dir: rtl)` and the Unicode Bidi Algorithm; complex-script shaping (Arabic, Indic, CJK) via `rustybuzz` + `icu_segmenter`. Pre-pays the Arabic/Hebrew/Chinese translator-coverage cost without a v1.0 renderer-swap project.
- **LD-1 (MIT) / LD-37 (`cargo deny` license allowlist).** `typst`, `typst-pdf`, `typst-as-lib` are all Apache-2.0 (allowlist-aligned). Transitive closure (~150 crates) is large but no unmaintained or unauthorized-license deps surfaced in spot-check; first CI run executes the full `cargo audit` + `cargo deny` sweep.
- **LD-2 (Tauri 2.x).** Pure-Rust, in-process, no Python/Qt sidecar; single-binary distribution preserved across macOS / Windows / Linux.
- **LD-14 (`orgsidian-report` crate extraction).** The crate was extracted precisely to absorb heavy PDF deps; typst's binary-size delta (~12–18 MB stripped on `orgsidian-report`) lands inside the LEAF crate and stays out of `cli`'s dependency closure per `cargo deny check graph`.
- **[[feedback_version_policy]] (latest-stable pinning; quarterly Tauri-sync window per LD-47).** Typst's 0.x cadence aligns with the LD-47 quarterly bump rhythm; potential breaking changes batched into the same window.

**Implementation outline** (target: `crates/orgsidian-report/`):

```
crates/orgsidian-report/
├── Cargo.toml                          # deps: typst = "0.14", typst-pdf = "0.14", typst-as-lib = "0.15", serde, serde_json
├── src/
│   ├── lib.rs                          # pub fn render_project_report_pdf(data: &ReportData) -> Result<Vec<u8>, ReportError>
│   ├── pdf_renderer.rs                 # typst engine setup + compile_with_input
│   ├── html_renderer.rs                # static HTML emission (separate path; templater choice deferred to FR-14 sprint, out of LD-53 scope)
│   ├── fonts.rs                        # embedded_font_resolver(): Inter + Noto Sans (Latin/Cyrillic) for v0.5 Beta; CJK/Arabic in v1.0
│   └── data.rs                         # ReportData struct (mirrors core query API), derive Serialize
└── templates/
    └── orgsidian-report-default.typ    # bundled via include_str!; ships as v0.5 Beta default
```

Data flow: `core` returns a `ReportData` struct → `serde_json::to_value` → `TypstEngine::compile_with_input(inputs)` → `Vec<u8>` PDF → `tauri_plugin_dialog` save dialog → `tokio::fs::write`. HTML output uses a parallel `html_renderer.rs` path (not typst-html); choice of HTML templater (`handlebars` vs `minijinja` vs `tera`) deferred to the FR-14 sprint and recorded as an in-sprint micro-decision, out of LD-53 scope.

**Embedded fonts (v0.5 Beta):** Inter (Variable) for sans-serif body; JetBrains Mono for code blocks; Noto Sans subset (Latin + Latin-Ext + Cyrillic) as fallback. All OFL-licensed. Total embedded font payload target: ≤8 MB. v1.0 adds Noto Sans CJK SC + Noto Sans Arabic subsets when PRD §8 translation rollout begins (separate LD).

**OQ-6 reconciliation (v1.0 customization template language).** PRD §10 OQ-6 previously stated: "v1.0 commits to template files for HTML/CSS customization. The exact template variable surface is unspecified. Resolution: drafted in v0.5 spike based on Beta tester feedback." LD-53 changes the PDF-path customization surface from HTML/CSS to **Typst `.typ` templates with a documented `sys.inputs` schema**; the HTML-path customization surface remains HTML/CSS. The PRD wave-2 reconciliation pass (2026-05-19, alongside the LD-46 follow-up) applied this update; the drafting deliverable in v0.5 Beta is now: (a) `orgsidian-report-default.typ` shipped + (b) `docs/customization/report-templates.md` documenting the `sys.inputs` schema generated from the `ReportData` struct.

**Downgrade path (recorded contingency, not pre-implemented).** If the v0.5 Beta sprint surfaces a typst blocker (build-time regression beyond the LEAF-crate envelope, `cargo deny` license rejection on a transitive dep, or a typst-side regression in `rustybuzz` shaping that ships in 0.14.x), the contingency is `printpdf` 0.9.x with the `html` feature: same `orgsidian-report` crate layout, swap `pdf_renderer.rs` for a `printpdf_renderer.rs` consuming HTML templates rendered via a small templater. Expected swap cost: ~3 dev-days. Confidence this contingency is not invoked: HIGH (typst is production-validated as an embedded library in 2025/2026 per Tinymist, Typst.app web playground, and `typst-as-lib` downstream telemetry).

**Decision date:** 2026-05-19. **Closes:** the "PDF rendering crate selection" Important Gap and the "PDF rendering crate selection during v0.5 Beta Project Report sprint" Areas-for-Future-Enhancement entry, both below in this document.

**LD-54. Conventional Commits enforcement + CHANGELOG mapping.**

**Specification.** All commits, PR titles, and CHANGELOG entries follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/). Type vocabulary: `feat`, `fix`, `perf`, `refactor`, `revert`, `docs`, `style`, `test`, `build`, `ci`, `chore`. Breaking changes signalled by `!` (e.g., `feat!:`) or `BREAKING CHANGE:` footer. Scope is optional but recommended; canonical scopes are crate names (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) or `shell-ui` / `docs` / `ci`. No scope-value enum enforced in commitlint to avoid false-positive friction.

**Enforcement chain (Story 1.14):**

- `commitlint.config.cjs` extends `@commitlint/config-conventional`.
- `husky` `commit-msg` hook runs `commitlint --edit "$1"` (in addition to the existing pre-commit hook per Linting & Formatting section).
- `.github/workflows/pr.yml` (or dedicated `commitlint.yml`) runs `commitlint --from origin/main --to HEAD` on every PR.
- PR title check via `amannn/action-semantic-pull-request` (or equivalent) on `pull_request_target` events.

**CHANGELOG mapping** (encoded in `cliff.toml` `[git.commit_parsers]`):

| CC type / footer | Keep-a-Changelog bucket | Notes |
|---|---|---|
| `feat` | **Added** | |
| `fix` | **Fixed** | |
| `perf` | **Changed** | User-visible improvement |
| `refactor` | **Changed** | Only if user-visible (`refactor!` or scope `public-api` / crate-public-surface) |
| `revert` | **Changed** | Entry text includes "Reverts #N" |
| `feat!` / `fix!` / `BREAKING CHANGE:` | **Changed** | Entry prefixed with `⚠ BREAKING:` |
| `docs` / `style` / `test` / `build` / `ci` / `chore` | *(excluded)* | Internal commits |
| `Deprecated` / `Security` (no CC type) | *(manual entries)* | Inserted before `cargo release` tag |

**Generation tool:** `git-cliff` invoked by `cargo release` pre-tag hook (Story 1.15); output overwrites the `Unreleased` section of `CHANGELOG.md` (root) and bumps to versioned heading at release time. `crates/orgsidian-plugin-api/CHANGELOG.md` follows the same flow but scoped to commits touching `crates/orgsidian-plugin-api/**` (per LD-33 separation policy at v1.5+).

**CONTRIBUTING.md section** (Story 1.10 AC) documents the CC vocabulary, scope discipline, examples per type, and the mapping table above.

**LD-55. GitHub Issues sync + label scheme + Project board.**

**Specification.** Every Story N.M in `epics.md` is mirrored as a GitHub Issue in `orgsidian/orgsidian` (one issue per story). Issues, labels, and a single org-level GitHub Project v2 board form the work-tracking surface.

**Label scheme** (`.github/labels.yml`, applied by `actions/github-script` or `crazy-max/ghaction-github-labeler` at Story 1.13):

- **Epic labels:** `epic:1` … `epic:13` (one per epic).
- **Milestone labels:** `milestone:v0.1`, `milestone:v0.5`, `milestone:v1.0` (in addition to native GitHub milestones for date tracking).
- **Status labels:** `status:backlog`, `status:in-progress`, `status:review`, `status:blocked`, `status:done`.
- **Type labels:** `type:story`, `type:bug`, `type:spike`, `type:chore`, `type:docs`, `type:security`.
- **Priority labels** (used sparingly): `priority:p0`, `priority:p1`.

**Issue body template** (`.github/ISSUE_TEMPLATE/story.md`) renders: persona, user-story, AC list, `Traces:` line, `Microcopy` flag, link back to `epics.md#story-N-M`.

**Project board** (Story 1.13): org-level Project v2 at `orgsidian/projects/1`, name "Orgsidian Roadmap". Columns: **Backlog** / **In Progress** / **Review** / **Done**. Two saved views: filtered by `milestone:v0.X`, grouped by `epic:N`. No swim lanes, no custom fields, no automation rules beyond the issue-sync workflow placing new issues in Backlog. Solo-dev discipline guard: do not add complexity unless a pain-point in v0.1 demonstrates a need.

**Sync automation** (Story 1.16): `.github/workflows/sync-issues.yml` runs on push to `main` when `_bmad-output/planning-artifacts/epics.md` changes. A small Rust binary at `tools/issues-sync/` (outside `[workspace.members]`, same convention as `tools/corpus-extractor/`) parses epics.md, extracts Story headers + bodies + `Traces:`, and uses the GitHub REST/GraphQL API (`octocrab` or `gh api` via `std::process::Command`) to:

1. Ensure an Issue exists per Story with title `[Story N.M] <story title>` and body per template.
2. Apply labels (`epic:N`, `milestone:v0.X` derived from epic-to-milestone mapping in `epics.md` §Epic List, `status:backlog` default if new, `type:story`).
3. Add the issue to the GitHub Project v2 board (Backlog column) if not already present.
4. Idempotent: re-running converges; closed issues stay closed; status-label drift not corrected (manual is authoritative once an issue is open).

**Direction:** one-way push (epics.md → Issues) in v0.1. Reverse direction (Issue closure → epics.md `status: done` annotation) deferred — likely never needed for a solo workflow.

**Repo visibility:** the org and repo are created **private** at Story 1.13 and remain private through pre-Alpha; flip to public is part of the v0.1 Alpha release artifact (Story 6.10), anchored before SM-1 announcement. See LD-5.

### Decision Impact Analysis

**Implementation sequence (after Stories 1-3 from step 3):**

1. `orgsidian-index` schema + migrations (`rusqlite_migration`) + connection management (LD-11..LD-14, LD-16). Enables all downstream queries.
2. `orgsidian-parser` tree-sitter-org wrapper + semantic AST builder. Feeds the indexer.
3. `orgsidian-watcher` notify-rs integration + debounce + Single Writer Rule (LD-7 + LD-21).
4. `orgsidian-vault` atomic-write + Dirty Buffer manager (LD-8 + LD-7).
5. `orgsidian-plugin-api` trait + Event enum + HookOutcome (LD-26).
6. `orgsidian-core` integration façade re-exports + static plugin registry (LD-25).
7. `orgsidian-cli` first commands (`parse`, `index init/rebuild/stats`) — establishes integration-test surface.
8. `tauri-specta` IPC bridge setup + first typed commands.
9. Shell editor wiring (Story 3) + Today Dashboard surface — first user-facing milestone.
10. TanStack Router setup + multi-surface routing.
11. Quick Capture window (LD-28) + global shortcut (`tauri-plugin-global-shortcut`).
12. Agenda virtualization (LD-30) + Search (FTS5 query API).
13. Clock manager + Project Report renderer.
14. Settings UI + Theme engine + first internal plugins compiled into static registry.
15. CI matrix (subset+nightly+merge-gate) + signing + release pipeline (LD-32..LD-34).

**Cross-component dependencies:**
- Plugin API trait shape (LD-26) gates the design of every v1.0 feature that consumes hooks (Capture, Save, Agenda query, etc.) since they all dispatch through the same hook bus.
- CSP (LD-18) gates the choice of CSS-in-JS / dynamic style injection strategy; Tailwind 4's atomic CSS approach is compatible only because `'unsafe-inline'` is allow-listed on `style-src`.
- The CLI (LD-27) is the primary integration-test fixture: any new core feature should have a CLI command exercising it before shell integration.
- Single Writer Rule (LD-7) + Atomic Writes (LD-8) + Watcher (LD-21) form an interdependent invariant set; they must be co-designed, not retrofitted.
- `tauri-specta` (LD-24) generates the IPC type bridge that LD-31 consumes — frontend cannot start consuming commands until specta is wired in the Tauri app.
- Static plugin registry (LD-25) means every v1.0 plugin (Capture, Clock, Theme, etc.) is a compile-time workspace dependency of `orgsidian-shell-app` — adding/removing a plugin is a workspace change, not a runtime config change.

## Implementation Patterns & Consistency Rules

These rules constrain HOW agents implement, not WHAT they implement. Following them is mandatory; deviation requires a documented justification in the implementation story (and an addendum to the relevant LD-NN entry).

### Naming Conventions

**Rust** (enforced by `cargo fmt` + `clippy`):
- Functions / variables / modules: `snake_case`
- Types (struct / enum / trait): `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- File names: `snake_case.rs`
- Public surface: explicit `pub` only when consumers need access; default `pub(crate)` or private.

**TypeScript / React**:
- Variables / functions / hooks: `camelCase` (hooks: `useFoo`)
- Components / types / classes: `PascalCase`
- Module-level immutables: `SCREAMING_SNAKE_CASE`; local consts: `camelCase`
- React component files: `PascalCase.tsx` (one component per file)
- Hook files: `useFoo.ts`
- Utility (non-component) files: `camelCase.ts` (`formatDate.ts`, `parseQuery.ts`)
- Barrels and config: `kebab-case` allowed (`index.ts`, `vite.config.ts`)

**SQLite**:
- Tables: `snake_case` plural (`headlines`, `clock_entries`)
- Columns: `snake_case` singular (`headline_id`, `scheduled_at`)
- Indices: `idx_<table>_<col1>_<col2>` (`idx_headlines_scheduled_at`)
- Foreign keys: `<referenced_table_singular>_id` (`file_id`, `headline_id`)
- Migration files: `NNNN_kebab-case-description.sql` (`0003_add-clock-entries-table.sql`)

**Tauri commands**:
- Rust handlers: `snake_case` (`open_file`, `query_agenda`).
- TS client (auto-generated by `tauri-specta`): `camelCase` (`commands.openFile`).
- Casing transform automatic via specta; never hand-write the TS side.

**Tauri events** (strings emitted by `app.emit()`):
- `kebab-case`: `file-changed`, `index-rebuilt`, `clock-tick`.
- Payload type registered with `tauri-specta` for typed `listen()`.

**Plugin API `Event` enum** (Rust):
- `PascalCase` variants: `FileOpened`, `HeadlineEdited`, `ClockStarted`.
- Past tense for completion events.
- `#[non_exhaustive]` requires `_` arm in consumers.

**CSS**:
- Tailwind utilities inline as default.
- Extracted classes only when the utility string repeats 3+ times; prefix `org-` (`org-headline-h1`, `org-todo-badge`).
- CSS variables: `--org-` prefix universally (FR-22 token vocabulary).
- BEM not used; scope from React tree.

### Project Structure

**Test placement**:
- Rust unit tests: `#[cfg(test)] mod tests` at bottom of source file. Co-located.
- Rust integration tests: `crates/<crate>/tests/<topic>.rs`.
- React component tests: co-located `Component.test.tsx` next to `Component.tsx`.
- TS module unit tests: co-located `module.test.ts`.
- E2E / Playwright: `packages/shell-ui/e2e/<topic>.spec.ts`.
- CLI integration tests via `assert_cmd`: `crates/orgsidian-cli/tests/<topic>.rs`.

**Component organization (`packages/shell-ui/src/`)**:
- `components/ui/` — forked shadcn primitives, one file per component.
- `components/org/` — Orgsidian UI Kit, one file per component.
- `surfaces/<SurfaceName>/` — one folder per full-screen surface; folder contains `index.tsx` + `<SurfaceName>.test.tsx` + colocated subcomponents.
- `coaching/`, `stores/`, `themes/`, `styles/` — single-responsibility leaves.
- No "shared components" floating folder; if reused → promote to `components/ui/` or `components/org/`.

**Crate organization (`crates/<name>/src/`)**:
- `lib.rs` — public surface re-exports only; no logic.
- `module.rs` or `module/mod.rs` — one concern per module.
- `error.rs` — crate-level `Error` enum + `Result<T> = std::result::Result<T, Error>` alias.
- `util/` — populated only after a function is reused twice; no premature utility extraction.

### Type & Error Format (IPC + serialization)

**Rust-side**:
- Every `#[tauri::command]` returns `Result<T, OrgError>`.
- `OrgError` is a single enum in `orgsidian-core/src/error.rs` deriving `thiserror::Error` + `serde::Serialize` + `specta::Type`.
- Variants per error category: `OrgError::Parse { file, reason }`, `OrgError::Io { … }`, `OrgError::Index { … }`, `OrgError::Vault { … }`.
- No `unwrap()` / `expect()` in production paths; use `?` and propagate. Tests may `.unwrap()` freely.

**TS-side**:
- `tauri-specta` generates throwing async functions. Errors deserialized to `OrgsidianError` (thin wrapper around `OrgError`).
- Callers use try/catch.
- No `null` / `undefined` for "no result" — empty arrays for empty collections; `Option<T>` → `T | null` (specta default).

**Date/time format**:
- Rust internal: `chrono::DateTime<Utc>` (absolute), `chrono::NaiveDate` (date-only).
- IPC wire: ISO 8601 strings (specta default).
- TS-side: native `Date` after parsing; never raw strings in component props.
- Org file: native `<YYYY-MM-DD Day HH:MM>` — only parser/serializer touch it.

**JSON / IPC payload casing**:
- Rust structs: `snake_case` fields (idiomatic).
- IPC wire + TS-side: `camelCase`.
- Conversion: **project-wide specta `camelCase` rename** configured once in the `tauri-specta` builder; do NOT use per-struct `#[serde(rename_all)]`.

**Null handling**:
- Rust `Option<T>` → TS `T | null` (specta default).
- Never `T | undefined` in IPC types.

### State Management & Communication

**Zustand stores** (`packages/shell-ui/src/stores/`):
- One store file per concern (`clockStore.ts`, `viewStore.ts`, `settingsStore.ts`, `coachingStore.ts`).
- Immer middleware enabled.
- Selectors exposed as hooks: `export const useActiveClock = () => useClockStore(s => s.activeClock);`
- Persistence via `tauri-plugin-store` adapter wrapped as Zustand middleware.
- Editor state (CM6) is NEVER duplicated into Zustand — CM6 owns it.

**Tauri command invocation pattern**:

```typescript
// ✓ Good — specta-generated client; errors bubble
import { commands } from '@/lib/tauri';
const file = await commands.openFile(path);

// ✗ Bad — raw invoke bypasses types
import { invoke } from '@tauri-apps/api/core';
const file = await invoke('open_file', { path });
```

**Tauri event listening pattern**:

```typescript
// ✓ Good — typed event channel via specta
import { events } from '@/lib/tauri';
useEffect(() => {
  const unlisten = events.fileChanged.listen(e => { /* e.payload typed */ });
  return () => { unlisten.then(fn => fn()); };
}, []);
```

### Logging

**Rust** (`tracing` ecosystem):
- Module-level: `use tracing::{info, warn, error, debug, trace};`
- Spans for operation lifecycles: `tracing::info_span!("index_file", path = %path).in_scope(|| { … })`
- **Structured fields, never string interpolation**:
  - ✓ `tracing::info!(file = %path, headlines = count, "indexed file")`
  - ✗ `tracing::info!("indexed file {} with {} headlines", path, count)`
- Levels: `error` user-impacting failure / `warn` degraded behavior / `info` lifecycle / `debug` developer detail / `trace` hot-path detail.

**TypeScript** (`tauri-plugin-log`):
- `import { info, warn, error, debug } from '@tauri-apps/plugin-log';`
- Structured: `info('file changed', { path, source });`
- No `console.log` in committed code (lint rule).

### Process Patterns

**Error recovery**:
- Atomic-write AV retry: bounded exponential backoff, max 3 attempts, base 100ms (LD-8 + `atomic-write-file` wrapper).
- IPC errors: no automatic retries — surface to caller; user-initiated retry only for user-visible operations.
- Filesystem watcher debounce: 250ms coalesce for atomic-save sequences (LD-7).
- Index corruption: `PRAGMA integrity_check` on startup → rebuild from files (LD-13).

**Loading states**:
- Per-surface `isLoading` from query-style hooks.
- Top-level: TanStack Router loader data; surface shows its own skeleton during route transition.
- No global loading spinner.

**Validation**:
- Rust: type system + `Result` propagation; no runtime validation library on internal paths.
- TS at IPC boundary: trust `tauri-specta` types (Rust enforced them); no re-validation.
- TS form input: HTML5 + manual; no zod/yup framework in v1.0 (revisit if forms grow).

### Documentation Conventions

- `orgsidian-plugin-api` public items: `///` doc comments mandatory; `cargo doc --no-deps` clean (no warnings).
- Other Rust crates: doc comments encouraged on public items; optional on `pub(crate)` and private.
- TS components / hooks: JSDoc on exported items when behavior is non-obvious; not required otherwise.
- Architectural decisions changed during implementation: update the relevant LD-NN entry with an addendum line.

### Linting & Formatting (mandatory CI gates)

- Rust: `cargo fmt --check`, `cargo clippy -- -D warnings`. `clippy::pedantic` enabled on `orgsidian-plugin-api` (public surface); allow-listed elsewhere.
- TS: ESLint + Prettier with React + TanStack Router presets; TS strict mode; `noUncheckedIndexedAccess: true`.
- Pre-commit: `husky` + `lint-staged` on staged files only.

### AI-Agent Implementation Rules (Mandatory)

When implementing a story:

1. **One concern per file.** A new file is cheaper than a god-file. If a file exceeds ~400 lines, split it.
2. **No `unwrap()` / `expect()` outside tests or `main()`.** Use `?` and propagate.
3. **No silent error swallowing.** Every ignored `Result` justified in a comment.
4. **No `any` / `unknown` in TypeScript** unless interfacing with an untyped third-party API; document the boundary.
5. **Add tests with every PR that adds production code.** No coverage gate, but missing tests is a review block.
6. **Use the generated `tauri-specta` client.** Never call `invoke('command-name')` with a raw string.
7. **Use Zustand store hooks, not `getState()` directly.** Reserve `getState()` for non-React code paths.
8. **CSS: Tailwind utilities first.** Extract to `org-*` classes only after 3+ repetitions.
9. **Update LD-NN if implementation forces a change.** Staleness is the failure mode for architectural docs.
10. **Run `cargo test --workspace && pnpm test` before pushing.** Flaky tests: file an issue, do not skip.

### Anti-Patterns (Forbidden)

- ❌ `invoke('command_name', …)` with raw strings (bypasses specta types)
- ❌ `#[serde(rename_all = "camelCase")]` on individual structs (use project-wide specta config)
- ❌ React `forwardRef` (React 19 — ref-as-prop)
- ❌ `useEffect(() => { new EditorView(...) }, [])` without idempotent cleanup (StrictMode breaks)
- ❌ Direct DOM manipulation in React components (use refs or CM6 APIs)
- ❌ String-typed event names in TS (use `events.<eventName>` from specta)
- ❌ Duplicating CM6 editor state into Zustand
- ❌ Conditional render via JSX for Plain/Power Mode (use `data-mode` + Tailwind selectors)
- ❌ `console.log` / `println!` in committed code
- ❌ Premature abstraction: three similar lines beats a wrong shared helper

## Project Structure & Boundaries

### Amendments to Earlier Sections (Party Mode round 4)

Where the text below conflicts with earlier sections, this section supersedes:

- **LD-5 (Monorepo).** Updated to Rust-crate naming (the original npm-style `@orgsidian/core` was placeholder). Workspace = **9 crates**: parser, index, watcher, vault, plugin-api, **report (NEW — extracted from core)**, core, cli, shell-app. Frontend lives at **`shell-ui/` at repo root** (no `packages/` indirection until a second JS package appears in v1.5+). Boundary enforcement: Rust workspace member visibility rules + `cargo-deny check graph` + custom CI check that consumer crates (`shell-app`, `cli`) do not import leaves directly.
- **LD-10 / LD-26 (Plugin API trait shape).** `HookContext` and `PluginContext` are **traits** defined in `orgsidian-plugin-api` (preserves the leaf-crate invariant — no project deps). `orgsidian-core` provides the concrete implementations, passed to plugins as `&dyn HookContext` / `&dyn PluginContext` at runtime. The trait-method code block in LD-26 should be read with `&dyn` on each context parameter (`fn on_save_before(&mut self, ctx: &dyn HookContext, …)`). This is what makes the v1.5+ crates.io publication possible without bundling `core`/`index`/`vault` dependencies into the published plugin API surface.
- **LD-29 (Routing — TanStack Router).** Implementation discipline: file-based `shell-ui/src/routes/` is the **single source of truth** for navigation. Surfaces are not duplicated as a separate folder; route files (`routes/_layout/agenda.$view.tsx`, etc.) own the navigation tree, with components organized by feature under `shell-ui/src/components/<feature>/`.
- **LD-14 (Reports renderer location).** Report rendering (FR-14) moves out of `orgsidian-core` into the new `orgsidian-report` crate so that consumers (CLI included) do not pay the PDF dependency compile cost when they don't need it.

### Workspace Layout

```
orgsidian/
├── Cargo.toml                                # [workspace] root + shared deps
├── Cargo.lock
├── package.json                              # root: declares shell-ui as JS workspace member
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── rust-toolchain.toml                       # stable + clippy + rustfmt
├── .gitignore
├── .editorconfig
├── LICENSE                                   # MIT
├── README.md                                 # project overview + quickstart + links
├── CONTRIBUTING.md                           # development setup + fixture rules + traceability discipline
├── ARCHITECTURE.md                           # high-level summary + Mermaid dep graph + links to docs/architecture.md
├── CHANGELOG.md                              # app-level user-facing, Keep a Changelog
├── .github/
│   └── workflows/
│       ├── pr.yml                            # per-PR job (LD-32)
│       ├── nightly.yml                       # full matrix + full round-trip corpus
│       └── release.yml                       # tag-triggered build + sign + publish
├── crates/
│   ├── README.md                             # one-line description per crate (discoverability)
│   ├── orgsidian-parser/                     # tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2)
│   ├── orgsidian-index/                      # rusqlite + FTS5; schema + migrations + query API (FR-17)
│   ├── orgsidian-watcher/                    # notify-rs + debounce + atomic-save coalescing
│   ├── orgsidian-vault/                      # atomic-write-file + Dirty Buffer + Single Writer Rule
│   ├── orgsidian-plugin-api/                 # LEAF: trait + Event + HookOutcome + HookContext/PluginContext traits
│   │   └── CHANGELOG.md                      # SemVer-tracked from day 1 (published at v1.5+)
│   ├── orgsidian-report/                     # NEW: FR-14 PDF + HTML renderers; isolated from core (heavy deps)
│   ├── orgsidian-core/                       # façade: orchestrator + registry + event_bus + clock + starter_vault
│   ├── orgsidian-cli/                        # bin: headless CLI
│   │   └── man/                              # clap_mangen-generated man pages (via build.rs)
│   └── orgsidian-shell-app/                  # bin: Tauri app, consumes core, hosts shell-ui
│       ├── tauri.conf.json
│       └── capabilities/
│           ├── main.json                     # main window capability allow-list
│           └── quick-capture.json            # quick-capture window capability allow-list
├── shell-ui/                                 # ROOT-LEVEL (no packages/ indirection)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts                        # multi-entry: index.html + quick-capture.html
│   ├── tailwind.config.js                    # minimal; tokens defined in app.css @theme
│   ├── eslint.config.js
│   ├── index.html                            # main window entry
│   ├── quick-capture.html                    # FR-10 separate window entry (separate bundle for latency)
│   ├── public/
│   ├── src/
│   │   ├── main.tsx                          # main window React root + TanStack Router setup
│   │   ├── quick-capture.tsx                 # quick-capture window root (no router, single surface)
│   │   ├── lib/
│   │   │   └── tauri.ts                      # tauri-specta generated (commands + events; do not hand-edit)
│   │   ├── routes/                           # TanStack Router file-based routes (SINGLE source for navigation)
│   │   │   ├── __root.tsx                    # root layout
│   │   │   ├── _layout/
│   │   │   │   ├── today.tsx                 # /today (default landing — FR-6)
│   │   │   │   ├── agenda.$view.tsx          # /agenda/today, /agenda/week, /agenda/custom (FR-7)
│   │   │   │   ├── editor.$filePath.tsx      # /editor/$filePath (FR-1, FR-3, FR-4)
│   │   │   │   ├── graph.tsx                 # /graph (FR-26 Backlink Graph View; LD-29, LD-56)
│   │   │   │   └── settings.$section.tsx     # /settings/general, /settings/themes, etc. (FR-22, FR-23)
│   │   │   └── index.tsx                     # / → redirect to /today
│   │   ├── components/
│   │   │   ├── ui/                           # forked shadcn primitives, essentials only
│   │   │   ├── org/                          # Orgsidian UI Kit (TodoStateCycler, OrgDatePicker, ...)
│   │   │   ├── today/                        # Today Dashboard widgets (FR-6)
│   │   │   ├── agenda/                       # Agenda view components (FR-7)
│   │   │   ├── editor/                       # CM6 host + decorations + keybindings (FR-3, FR-4, FR-5)
│   │   │   ├── settings/                     # Settings panel components
│   │   │   ├── merge/                        # 3-pane Merge Dialog (FR-16; custom focus mgmt)
│   │   │   ├── palette/                      # cmdk Command Palette (FR-12 entry)
│   │   │   ├── graph/                        # FR-26 Backlink Graph View: GraphCanvas.tsx (react-force-graph-2d) + GraphNodeList.tsx (a11y fallback per LD-58)
│   │   │   ├── refile/                       # FR-25 Refile target picker (fuzzy file + outline path) — v0.5 Beta
│   │   │   └── onboarding/                   # Starter Vault picker (FR-18) + Tutorial (FR-19)
│   │   ├── capture/                          # Quick Capture window components (FR-10)
│   │   ├── coaching/                         # FR-21
│   │   │   ├── coachingRegistry.ts           # centralized content + dismissal conditions
│   │   │   └── CoachingSlot.tsx
│   │   ├── stores/                           # Zustand
│   │   │   ├── clockStore.ts                 # Active Clock state
│   │   │   ├── viewStore.ts                  # current Editor Mode, sidebar state
│   │   │   ├── settingsStore.ts              # Plain/Power Mode (FR-20), theme path
│   │   │   └── coachingStore.ts              # dismissed coaching IDs
│   │   ├── themes/
│   │   │   ├── tokens.css                    # default --org-* CSS variable values
│   │   │   ├── dark.css
│   │   │   └── light.css
│   │   └── styles/
│   │       ├── app.css                       # Tailwind 4 @import + @theme directive
│   │       └── reset.css
│   └── e2e/                                  # Playwright + Tauri WebDriver
├── examples/
│   └── plugins/                              # v1.5+ plugin author onboarding (skeleton from day 1)
│       ├── hello-world/                      # ~30-line lifecycle demo
│       └── agenda-exporter/                  # realistic example exercising hooks + observer
├── docs/
│   ├── README.md                             # docs index — "start here" map
│   ├── architecture.md                       # CANONICAL architecture document (not a symlink)
│   ├── cli.md                                # CLI reference, complements --help / man pages
│   ├── plugin-api/                           # plugin author docs (v1.5+ public)
│   │   ├── README.md                         # "your first plugin in 10 minutes"
│   │   ├── contract-tests.md                 # how validate-plugin works
│   │   └── api-reference/                    # cargo-doc generated output
│   └── user-guide/                           # end-user docs (v0.5 Beta onward)
├── fixtures/                                 # CROSS-CRATE fixtures only (per CONTRIBUTING.md rule)
│   ├── subset-pr.json                        # ~100 files for per-PR round-trip gate (LD-32)
│   └── full-nightly.json                     # pointer to full corpus for nightly job
├── tools/
│   └── corpus-extractor/                     # OUTSIDE [workspace.members] — standalone tool
│       ├── Cargo.toml                        # publish = false; not in workspace build
│       └── src/main.rs                       # extracts test corpus from org-element.el assertions
└── _bmad-output/                             # workspace of planning process (archival; not canonical)
    └── planning-artifacts/                   # original PRD + addendum + this architecture draft
```

**Key structural rules:**

- `shell-ui/` lives at repo root (no `packages/` until a 2nd JS package appears).
- Components are organized by feature (`components/agenda/`, `components/editor/`); navigation surfaces are defined by TanStack Router file-based routes in `routes/`.
- `crates/orgsidian-report/` is the only new 9th crate (extracted from `core` for dependency isolation).
- `tools/corpus-extractor/` has its own standalone `Cargo.toml` and is NOT a workspace member; `cargo build --workspace` does not pay its compile cost.
- `docs/architecture.md` is the canonical version; `_bmad-output/` is process archive. No symlinks.
- Fixture placement rule (in `CONTRIBUTING.md`): co-located with the consuming crate by default (e.g., `crates/orgsidian-parser/tests/fixtures/`); promoted to root `fixtures/` only if ≥2 crates consume them.

### Crate Dependency Graph

```
orgsidian-shell-app ──┐
                      ├──► orgsidian-core ──┬──► orgsidian-parser
orgsidian-cli ────────┘                     ├──► orgsidian-index
                                            ├──► orgsidian-watcher
                                            ├──► orgsidian-vault
                                            ├──► orgsidian-report     (NEW; reachable from core)
                                            └──► orgsidian-plugin-api (LEAF; no project deps)
```

- **Leaves**: `orgsidian-parser`, `orgsidian-index`, `orgsidian-watcher`, `orgsidian-vault`, `orgsidian-report`, `orgsidian-plugin-api`. None depend on any other project crate.
- **Façade**: `orgsidian-core` re-exports from leaves + provides orchestrator, registry, event bus, clock state, starter vault templates.
- **Consumers**: `orgsidian-shell-app` and `orgsidian-cli` consume `orgsidian-core` only — never reach into leaves directly.
- **CI gate**: `cargo-deny check graph` + custom check that consumer Cargo.toml does not list leaf crates as direct dependencies (enforced by workspace `[workspace.dependencies]` exposing only `orgsidian-core` to consumers).

### Architectural Boundaries

1. **Process boundary.** One Tauri main process + Tauri-managed WebView (main window) + secondary WebView (Quick Capture window, separate Vite bundle). The CLI is a separate executable; no shared process with the desktop app.
2. **IPC boundary.** `tauri-specta`-typed commands frontend→Rust; `app.emit()` events Rust→frontend. Wire format: JSON, camelCase (project-wide specta config). Errors serialized as `OrgError` enum.
3. **Plugin trait boundary.** `orgsidian-plugin-api` is leaf (no project deps). `HookContext` and `PluginContext` are **traits** in plugin-api; `orgsidian-core` provides concrete implementations. Plugins receive `&dyn HookContext`/`&dyn PluginContext` references. `Event` is `#[non_exhaustive]`.
4. **Filesystem boundary.** Two zones — (a) Vault folder (user-chosen, runtime allow-list via `tauri-plugin-fs`): `.org` file r/w only; (b) App config/data/log dirs (OS-standard): SQLite index, settings store, logs, themes. No FS access outside these zones from any process.
5. **Database boundary.** SQLite index accessed only through `orgsidian-index::query::*` API. No raw SQL from outside `orgsidian-index`. Schema changes go through migrations (LD-12).
6. **Editor state boundary.** CodeMirror 6 owns the buffer for the open file. Never duplicated into Zustand, never persisted separately from `.org` files.
7. **Plugin event boundary.** Plugins receive events through the trait API only — cannot subscribe to Tauri events directly, cannot query SQLite directly, cannot read FS directly. Everything goes through `HookContext`.

### FR → Component Mapping

| FR | Lives in |
|---|---|
| **FR-1** Open/parse | `orgsidian-parser/src/grammar.rs` + `orgsidian-shell-app/src/commands/file.rs` |
| **FR-2** Round-trip | `orgsidian-parser/src/serializer.rs` + `tests/round_trip.rs` + CI gate (LD-32) |
| **FR-3** Editor Modes | `shell-ui/src/components/editor/ModeSwitcher.tsx` + `stores/viewStore.ts` |
| **FR-4** Pseudo-WYSIWYG | `shell-ui/src/components/editor/decorations/` (CM6 ViewPlugins) |
| **FR-5** Keybindings + Emacs | `shell-ui/src/components/editor/keybindings/` + `tauri-plugin-os` |
| **FR-6** Today Dashboard | `shell-ui/src/components/today/` + `routes/_layout/today.tsx` + `orgsidian-index::query::agenda` |
| **FR-7** Agenda views | `shell-ui/src/components/agenda/` + `routes/_layout/agenda.$view.tsx` + `orgsidian-index::query::agenda` |
| **FR-8** Clock | `orgsidian-core/src/clock.rs` + `shell-ui/src/components/org/ClockEditor.tsx` + `stores/clockStore.ts` |
| **FR-9** Schedule/Deadline | `orgsidian-parser/src/semantic/timestamp.rs` + `shell-ui/src/components/org/OrgDatePicker.tsx` |
| **FR-10** Quick Capture global | `orgsidian-shell-app/src/commands/capture.rs` + `shell-ui/quick-capture.html` + `shell-ui/src/capture/` + `tauri-plugin-global-shortcut` |
| **FR-11** System tray | `orgsidian-shell-app/src/tray.rs` |
| **FR-12** FTS5 search | `orgsidian-index/src/query/search.rs` + `shell-ui/src/components/palette/` |
| **FR-13** Backlinks | `orgsidian-index/src/query/backlinks.rs` (Linked: `:ID:` + `[[wiki-link]]` traversal) + `orgsidian-index/src/query/unlinked_references.rs` (v0.5+: FTS5 title-match outer-joined against `links` table) + `shell-ui/src/components/org/BacklinksPanel.tsx` (Linked/Unlinked sub-tabs) |
| **FR-14** Project Report | **`orgsidian-report/`** (new crate) + `shell-ui/src/components/settings/ReportExport.tsx` |
| **FR-15** Vault designation | `orgsidian-vault/src/path.rs` + `shell-ui/src/components/settings/VaultPicker.tsx` |
| **FR-16** Watcher + Single Writer | `orgsidian-watcher/*` + `orgsidian-vault/src/dirty_buffer.rs` + `shell-ui/src/components/merge/` |
| **FR-17** SQLite derived index | `orgsidian-index/*` + rebuild logic (LD-13) |
| **FR-18** Starter Vault | `orgsidian-core/src/starter_vault/templates/{personal-gtd,student,freelancer,empty}/` (v0.1 Alpha ships personal-gtd + student + freelancer per PRD 2026-05-20; empty in v0.5) + `shell-ui/src/components/onboarding/` |
| **FR-25** Refile a Headline | `orgsidian-vault/src/refile.rs` (subtree extract/insert under `tree-sitter-org`) + `orgsidian-core/src/orchestrator/refile.rs` (cross-file atomicity per LD-57) + `shell-ui/src/components/refile/RefileTargetPicker.tsx` (fuzzy file + outline path) |
| **FR-26** Backlink Graph View | `orgsidian-index/src/query/graph.rs` (`adjacency(scope) -> GraphData { nodes, edges }`) + `shell-ui/src/components/graph/GraphCanvas.tsx` (`react-force-graph-2d` per LD-56) + `shell-ui/src/components/graph/GraphNodeList.tsx` (a11y textual fallback per LD-58) + `shell-ui/src/routes/_layout/graph.tsx` (TanStack route per LD-29) |
| **FR-19** Tutorial | `shell-ui/src/components/onboarding/Tutorial.tsx` |
| **FR-20** Plain/Power Mode | `stores/settingsStore.ts` + Tailwind `data-[mode=plain]:hidden` |
| **FR-21** Inline Coaching | `shell-ui/src/coaching/` |
| **FR-22** Themes | `shell-ui/src/themes/` + `~/.orgsidian/themes/` user CSS path |
| **FR-23** Keybinding remap | `shell-ui/src/components/settings/KeybindingEditor.tsx` + `tauri-plugin-store` |
| **FR-24** Internal Plugin Pattern | `orgsidian-plugin-api/*` + `orgsidian-core/src/registry.rs` + `orgsidian-shell-app/src/plugins/` |

### FR Traceability Discipline

The FR mapping above is not allowed to atrophy. Two-layer enforcement:

1. **In code (per Paige's proposal):** every module that implements an FR carries a doc-comment header:
   ```rust
   //! Implements FR-12 (full-text search via SQLite FTS5).
   ```
   A simple `grep -r "Implements FR-" crates/ shell-ui/src/` reproduces the live mapping.

2. **In tests:** `tests/traceability.rs` at workspace root parses the PRD's FR-NN enumeration and fails if any FR has no `Implements FR-NN` doc-comment match in the codebase. This makes the mapping a CI gate, not aspirational documentation.

### CLI Documentation Strategy (per Paige)

- `clap` derive macros: `#[command(about = "…", long_about = "…")]` annotations are the **primary documentation** for the CLI. The `--help` output is the user manual.
- `crates/orgsidian-cli/build.rs` invokes **`clap_mangen`** to generate man pages into `crates/orgsidian-cli/man/`. Release pipeline bundles them with the binary on macOS + Linux distributions.
- `docs/cli.md` is a navigable reference document (single source: clap annotations + a thin presentation layer); regenerated by the same build step into Markdown.

### Plugin Author Onboarding (v1.5+, scaffolded from day 1)

- `examples/plugins/hello-world/`: ~30-line lifecycle demo. Implements only `metadata()`, `init()`, `shutdown()`; logs each call.
- `examples/plugins/agenda-exporter/`: realistic example using `on_event(FileSaved)` + `on_agenda_query_after` to export agenda to a third format.
- `docs/plugin-api/README.md`: "Your first plugin in 10 minutes" tutorial. Walks through cloning the hello-world example, modifying it, running `orgsidian validate-plugin`, observing events.
- `docs/plugin-api/contract-tests.md`: how the `validate-plugin` CLI command exercises the trait contract.
- `docs/plugin-api/api-reference/`: `cargo doc --no-deps -p orgsidian-plugin-api` output, published alongside Cargo doc on docs.rs at v1.5+.

### CHANGELOG Strategy

- **`CHANGELOG.md` (root):** app-level user-facing changelog, Keep a Changelog format. Versioned with the desktop app releases (v0.1, v0.5, v1.0, …).
- **`crates/orgsidian-plugin-api/CHANGELOG.md`:** tracked from day 1 even while the crate is unpublished. SemVer-disciplined. When the crate goes public at v1.5+, the changelog comes with it intact.
- **Other crates (parser, index, watcher, vault, report, core, cli, shell-app):** no separate changelog while internal; covered by the root CHANGELOG. If any of these is ever published independently (unlikely for v1.0–v1.4), it gets its own at that time.

### Discoverability Aids (per Paige)

- **`crates/README.md`:** a single-page table — one line per crate, what it does, what it depends on, what it does NOT do. Example row: `orgsidian-watcher — detects filesystem changes. Depends on: notify-rs. Does NOT do: read/write file content (see orgsidian-vault).`
- **`ARCHITECTURE.md` (root):** high-level summary + Mermaid diagram of the crate dependency graph + links to `docs/architecture.md` for full detail.
- **`docs/README.md`:** index of `docs/` — "start here" map distinguishing user-guide vs plugin-api vs architecture references.

### Integration Points (data flow summary)

**Save cycle (FR-2 + LD-7 + LD-26):**
```
User saves in CM6
  → commands.saveFile(path, content)
    → orgsidian-shell-app::commands::file::save
      → orgsidian-core::orchestrator::save
        → orgsidian-core::event_bus dispatches on_save_before hooks (priority-ordered)
          → plugin may HookOutcome::Replace(new_content) or HookOutcome::Cancel(reason)
        → orgsidian-vault::atomic::write (AV-aware retry)
        → orgsidian-watcher detects own write — suppressed via writer-ID token
        → orgsidian-index::sync::incremental re-parses + updates index
        → orgsidian-core::event_bus emits FileSaved event to observers
      → emit "file-changed" Tauri event
    → return Result<(), OrgError>
  → frontend: isDirty=false, last-saved timestamp updated
```

**External-write cycle (FR-16):**
```
External tool writes file
  → orgsidian-watcher::watcher emits raw events
  → orgsidian-watcher::debounce coalesces 3-12 events (250ms window)
  → orgsidian-vault::dirty_buffer checks state for that path
    → CLEAN → reload + re-index → "file-reloaded" Tauri event
    → DIRTY → "file-conflict" event with both versions
      → frontend opens MergeDialog (shell-ui/src/components/merge/)
      → user resolves hunks
      → commands.resolveMerge(path, mergedContent) → atomic write
```

**Plugin loading (v1.0 static):**
```
orgsidian-shell-app::main → Tauri::Builder
  → orgsidian-shell-app::plugins::mod instantiates Vec<Box<dyn OrgsidianPlugin>>
    → each bundled plugin's constructor runs
  → register Vec into Tauri::State<Arc<PluginRegistry>>
  → orgsidian-core::registry.init_all() → plugin.init(&core_ctx) on each (sorted by priority)
  → ready
```

### Development Workflow Integration

**Dev mode** (`pnpm tauri dev`): Vite dev server serves `shell-ui/` with HMR; Cargo builds `orgsidian-shell-app` in debug profile; on Rust change, app rebuilds + Tauri window restarts. `cargo watch -x check -x clippy -x test` recommended in a side terminal during heavy Rust work.

**Build mode** (`pnpm tauri build`): Vite builds `shell-ui/dist/` (production, minified); Cargo builds `orgsidian-shell-app` (release, LTO); Tauri bundler emits platform artifact (DMG / AppImage / MSI per LD-34); `tauri-specta` exports `shell-ui/src/lib/tauri.ts` as part of build pre-step; `clap_mangen` generates `crates/orgsidian-cli/man/` man pages in the CLI's `build.rs`.

**Release** (`.github/workflows/release.yml`): triggered by `v*` tag → matrix build per platform → sign per LD-19 → publish to GitHub Releases + auto-update endpoint (LD-20).

## Post-Validation Hardening (Party Mode round 5)

Step 7 validation surfaced gaps that were not catalogued in steps 1-6. LD-37 through LD-51 below close them. The project tree (step 6) is amended to include `SECURITY.md` at root.

### LD-37. Dependency audit & supply-chain hygiene

- `Cargo.lock` is **committed** (binary-application convention).
- **`cargo audit`** runs per-PR in CI; fails on `RUSTSEC` advisory severity ≥ medium. Auto-update via Dependabot/Renovate PRs for `Cargo.lock`.
- **`cargo deny check licenses`** allowlist: `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unlicense`, `Zlib`, `MPL-2.0` (file-level weak copyleft tolerable). Fail on `GPL-*`, `AGPL-*`, proprietary, unknown.
- **`cargo deny check bans`** blocks duplicate major versions of `tokio`, `serde`, `chrono`, `rusqlite` (canonical-version invariants).
- **`cargo deny check graph`** enforces the LEAF crate rule: consumers (`shell-app`, `cli`) cannot import leaf crates (parser/index/watcher/vault/report/plugin-api) directly.
- Quarterly review of advisory exceptions documented in `docs/security/advisory-exceptions.md`.

### LD-38. Plugin panic isolation under static linking

- `[profile.release] panic = "unwind"` in workspace `Cargo.toml` (overrides Rust default `abort`).
- All plugin invocation sites use the `invoke_plugin_hook!` macro from `orgsidian-core::registry`:
  - Wraps the call in `std::panic::catch_unwind`.
  - On panic: logs via `tracing::error!` with plugin metadata, marks plugin as **disabled-for-session** in `PluginRegistry`, subsequent invocations skip without error.
  - Surfaces a "Plugin disabled due to panic" badge in Settings UI; user can re-enable after restart.
- Chaos test plugin `test-plugin-panic` in `crates/test-plugin-panic/` deterministically panics in every hook point (init / shutdown / on_event / on_save_before / on_capture_before / on_agenda_query_after). CI gate verifies the host process survives all hooks across the matrix.

### LD-39. Multi-instance lockfile

- On Vault open: write `<Vault>/.orgsidian/instance.lock` containing JSON `{pid, hostname, started_at, locked_until}`.
- On Vault open if lockfile present: check liveness (PID exists + `locked_until` recent). If alive, present **dialog**: "Another Orgsidian instance is using this Vault — Open in read-only / Force unlock / Cancel". Force-unlock requires confirmation; read-only disables index writes + watcher.
- Heartbeat: rewrite `locked_until` every 30s; lockfiles with `locked_until` >5 min in the past treated as orphan and auto-cleared.
- Test: CI spawns two `orgsidian` processes against the same fixture Vault; second process must hit the dialog code path; index must not corrupt (`PRAGMA integrity_check` passes after both close).

### LD-40. Vault-self-contained state

- **Per-Vault state** lives in `<Vault>/.orgsidian/`: keybinding remap, theme path, dismissed coaching IDs, Plain/Power Mode preference, named filter presets, last-open file, instance lockfile (LD-39).
- **Global state** lives in OS-conventional config dir (`~/.config/orgsidian/` Linux, `~/Library/Application Support/Orgsidian/` macOS, `%APPDATA%\Orgsidian\` Windows): list of recent Vault paths, default UI language, default theme for new Vaults, telemetry decision (when reintroduced in v1.5+).
- **SQLite index** stays OUTSIDE the Vault (OS data dir; LD-17) because it is derived and rebuildable. It does NOT follow the Vault when moved — the new machine rebuilds it automatically per LD-13.
- User experience: moving a Vault folder to a new machine + re-opening via Settings preserves settings, keybindings, themes, coaching dismissals; only the index rebuilds silently within the FR-15 budget.
- **Format (amended 2026-05-20 per PRD §10 OQ-7 resolution).** Settings are stored as **TOML** in human-readable files: per-Vault at `<Vault>/.orgsidian/settings.toml`; global at `<config-dir>/global.toml`. The TOML file is **authoritative** — the Settings GUI is a thin round-trip editor over it (any GUI change writes the canonical TOML; any external edit to TOML refreshes the GUI on next focus). Rationale: org-mode users expect text-editable config; TOML pairs idiomatically with the Rust core (Cargo precedent) and supports comments (JSON does not); YAML rejected on security history (billion-laughs, YAML 1.1/1.2 inconsistencies) given the local-first/no-network commitment. `tauri-plugin-store` is **retained only for ephemeral UI state** (in-session palette open/closed, scroll positions, transient view toggles) — never authoritative settings.

### LD-41. Failure mode catalog

| Failure Mode | Detection | Recovery | Test |
|---|---|---|---|
| Malformed `.org` file in Vault | Parser returns error on file | Mark file as `quarantined` in `vault_meta`; skip during index sync; FS watcher continues; UI status notice "N files unparseable" | Fixture corpus with deliberately broken files |
| Disk full during atomic write | `WriteFile` ENOSPC or `MoveFileExW` failure | Cleanup `.tmp` file; surface error to user; never propagate partial-write corruption | Fault-injection wrapper around `atomic-write-file` |
| Config corruption (`<Vault>/.orgsidian/settings.json` malformed JSON) | Deserialization fails on startup | Backup corrupted file to `settings.json.broken-{timestamp}`; fall back to defaults; warn user with banner | Property test with random byte corruption via `proptest` |
| Vault folder deleted while app open | FS watcher emits delete event on root | Switch app to read-only mode using in-memory state; prompt user to relocate or close | CI integration test: `rmrf` the vault mid-session |
| Plugin `init()` panic | `catch_unwind` in `invoke_plugin_hook!` (LD-38) | Skip plugin for session; log error; user sees disabled badge in Settings | `test-plugin-panic-init` chaos plugin |
| Plugin `on_event` / hook panic | Same as above | Plugin disabled for session at first panic; surfaced in Settings | `test-plugin-panic-runtime` chaos plugin |
| SQLite index corruption | `PRAGMA integrity_check` on startup fails | Drop + rebuild from files (LD-13) with progress UI (LD-42) | Fixture: pre-corrupted `.db` file |
| `.tmp` orphan files from prior crash | Startup scan for `*.tmp.<pid>` matching dead PID | Delete orphans before opening Vault | Test: `kill -9` mid-write fixture, then restart |
| External tool deletes file with Dirty Buffer open | FS watcher emits delete on path with `is_dirty=true` | Surface banner "File deleted externally. Save will recreate it." Options: Save (recreate), Discard buffer, Save-as-different-file | Integration test with watcher harness |
| Refile partial completion (destination written, source write fails) — FR-25 | `refile_orchestrator` detects source-write error after destination commit | Restore destination from `.bak` backup taken before destination write; surface error "Refile reverted: source could not be updated"; both files end at pre-Refile state | Fault-injection: inject `ENOSPC` on source write after dest commit; verify roll-forward; see LD-57 |

### LD-42. Large-vault indexing UX

- Initial scan reports per-file progress: `(N of M files indexed, X errors)` in a non-modal status panel.
- **Cancellable**: user can abort initial scan; partial index retained; resume from last checkpoint at next open or via `orgsidian index rebuild` (LD-49).
- **Checkpoints every 100 files** (configurable via `~/.config/orgsidian/global.toml`) → SQLite transaction commit + progress UI update.
- Nightly CI matrix tests against synthetic vault fixtures at **10k**, **25k**, **50k** files; performance baselines documented in `docs/perf/large-vault-scaling.md`.
- PRD §8 NFR `<30s` applies to 1k-file vault as previously stated. Scaling targets: <5min for 10k, <20min for 50k (documented soft targets, not gates).
- If scan is killed (process abort, `kill -9`), restart resumes from last checkpoint, never from zero.

### LD-43. Memory soak regression gate

- **Nightly CI job** (Linux only, dedicated runner) runs a 12-hour scripted session: open/close 200 buffers (random vault files), trigger 50 plugin re-init cycles (via `orgsidian index rebuild` + restart loop), execute 1000 agenda queries with varied filters.
- Memory measured every 30 minutes via RSS from `/proc/self/statm`.
- **Drift threshold: <10% RSS** over 11 hours (minute 60 → minute 720, warmup excluded).
- Regression PR-blocking via the nightly merge gate (LD-32 discipline): if drift exceeds threshold on nightly, no PRs merge to `main` until fixed.
- Triage tooling: `dhat` heap profiler attached to a separate diagnostic run on demand; reports committed to `docs/perf/memory-soak-reports/`.

### LD-44. Round-trip L0 subset corpus selection criteria

The "~100 representative files" per-PR L0 gate (LD-32) is selected by a documented algorithm in `tools/corpus-extractor/`:

1. **Syntax-feature coverage matrix.** Every documented org-mode syntax construct must appear at least 3 times in the subset:
   - Heading levels 1-6 with TODO states (`TODO`, `NEXT`, `DONE`, `WAITING`, custom)
   - Scheduled / Deadline / Clock lines (active + inactive + ranged + recurring)
   - Drawers (`:PROPERTIES:`, `:LOGBOOK:`, custom drawer types)
   - Inline markup: `*bold*`, `/italic/`, `=verbatim=`, `~code~`, `+strike+`, `_underline_`
   - Links: `[[id:...]]`, `[[wiki]]`, `[[file://...]]`, plain `[[http://...]]`
   - Lists: `-`/`+`/numbered/checkbox
   - Tables (simple + with formula)
   - Block elements: `#+BEGIN_SRC`, `#+BEGIN_QUOTE`, `#+BEGIN_EXAMPLE`, `#+BEGIN_VERSE`
   - Inline LaTeX (`$...$`, `\\(...\\)`, `\\[...\\]`)
   - Footnotes, citations (org-cite syntax)
2. **Size buckets:** 30 small (<1KB), 50 medium (1-50KB), 20 large (>50KB).
3. **Edge-case bucket:** ≥5 files with Unicode/RTL/case-folded paths, ≥5 with unusual line endings (CRLF, mixed), ≥5 with malformed-but-valid syntax (over-indented properties, trailing whitespace in headlines).
4. ADR `docs/adr/0001-corpus-subset-selection.md` documents the algorithm; subset regenerated on corpus changes by `tools/corpus-extractor`.

### LD-45. L2 Emacs ground-truth oracle pinning

- **Two Emacs versions pinned** in nightly CI: `emacs:29.x` (stable LTS-like) and `emacs:30.x` (current). Both run against the L2 subset corpus.
- Each org file in the L2 subset has a hand-written canonical AST in `crates/orgsidian-parser/tests/canonical_ast/` (committed JSON, peer-reviewed). Meta-test: the test harness verifies Emacs' output matches the canonical AST — if Emacs diverges from the canonical, the issue is in the oracle pipeline, not in Orgsidian.
- **Divergence triage workflow:**
  - Both Emacs versions concordant against Orgsidian → Orgsidian bug (PR-blocking).
  - Both Emacs versions discordant from each other → log as `docs/parser/KNOWN_DIVERGENCES.md` entry with Emacs version range and Orgsidian's chosen behavior; not a PR blocker.
  - One Emacs version concordant with Orgsidian, the other discordant → human review case; defer decision, do not block.

### LD-46. PRD reconciliation TODO

The architecture has selected MIT (LD-1) and `nvim-orgmode/tree-sitter-org` (LD-3). The PRD and addendum still reference the conditional licensing language ("GPL-3.0 if uniorg, MIT/Apache-2.0 if non-GPL parser path"). A reconciliation pass on the PRD is required **before v0.1 Alpha implementation starts**:

- PRD §7.3 → update to "MIT-licensed (decided via architecture workflow, 2026-05-19)."
- PRD §10 OQ-1 and OQ-2 → mark resolved with reference to LD-1..LD-3.
- PRD addendum §A.2 → annotate that Option B (`nvim-orgmode/tree-sitter-org` + custom Rust semantic layer) was selected; resolution dated 2026-05-19; coverage measurement spike (originally Spike 1) reframed as side-task of OD-1 in this document.
- PRD addendum §A.3 → annotate that Tauri 2.x stack confirmed; stack-comparison spike (originally Spike 2) reframed as CI matrix work in OD-2 of this document.

This LD is a TODO outside the architecture document scope; tracked as the first follow-up workstream "PRD reconciliation post-architecture" before v0.1 sprints begin.

**Closed-loop addendum (2026-05-20) — UX-design-specification reconciliation.** A second PRD reconciliation completed 2026-05-20, absorbing the new UX design specification (`_bmad-output/planning-artifacts/ux-design-specification.md`, 2026-05-20). Architecture impact captured in this document via: LD-56 (FR-26 Graph View rendering library), LD-57 (FR-25 Refile cross-file atomicity), LD-58 (WCAG 2.1 AA hard CI gates from v0.1). Plus amendments to LD-7 (cross-file extension pointer), LD-29 (`/graph` route), LD-32 (per-PR a11y gate), LD-40 (TOML config-file authoritative; supersedes `tauri-plugin-store` for settings), LD-41 (Refile partial-failure row), stack-versions table (`react-force-graph-2d@1.29.1`, `@axe-core/playwright`, `toml` crate), FR→Component table (FR-13 amended for unlinked-references; FR-18 annotated with v0.1 templates; FR-25 + FR-26 rows added). PRD/architecture for v0.1 Alpha is now closed-loop on the 2026-05-20 reconciliation wave. Downstream: `epics.md` requires reconciliation in a separate pass (FR-25 + FR-26 story scaffolding; WCAG a11y gate scaffold story in Epic 1; Freelancer Starter Vault content stories promoted to v0.1).

### LD-47. Tauri ecosystem pinning policy

- `Cargo.toml`: **exact-pin** (`=2.X.Y`, not caret) for `tauri`, `tauri-build`, every `tauri-plugin-*`, `tauri-specta`, and the transitive `webkit2gtk-rs`.
- **Quarterly review** of Tauri 2.x changelogs; reserve a "Tauri sync" slot at each Orgsidian milestone (v0.2, v0.3, v0.4) for 2-3 days of bump + adjustment work.
- **v0.4 milestone explicitly budgets 2-3 weeks** for a Tauri minor migration if breaking changes have accumulated since the prior sync.
- **Fallback plan**: if Tauri 2.x evolves catastrophically (hypothetical breaking-redesign), drop to `wry` directly + custom window/event/IPC plumbing. Documented as an escape hatch in `docs/architecture/resilience.md`; pre-budgeted at ~3 weeks of work.

### LD-48. `tree-sitter-org` vendoring + maintenance contingency

- `crates/orgsidian-parser/grammar/` is a **git submodule** pinned to a specific SHA of `nvim-orgmode/tree-sitter-org`. No auto-bumping; SHA review required on each upgrade.
- A designated **"parser owner"** role within the team (one named contributor) maintains working familiarity with the grammar source code.
- **v0.3 milestone reserves 2 weeks** for a fork-and-maintain dry run: the parser owner checks out the upstream, builds it from source, makes a trivial fix to a real issue, runs the full parser test corpus against the fix. Confirms the team can sustain the dependency if upstream stalls.
- **Trigger for in-house fork**: if at any v* milestone the upstream has had no commits for >6 months, fork to `orgsidian-org/tree-sitter-org` and maintain in-house under the same MIT license.

### LD-49. `rebuild-index` as a first-class command

- CLI (already in LD-27): `orgsidian index rebuild`.
- Shell UI: Settings → Vault → "Rebuild Index" button with progress UI (LD-42).
- **Schema migration safety net:** under LD-12 + LD-49, forward-only migrations cover normal evolution. For **radical schema rethinks** (rare), bump the schema version + the app triggers automatic rebuild at next startup with progress UI. User never needs to know there was a "migration" — only that "the index is being refreshed."
- This disinnesca the schema design as a one-way door (Winston's framing): the user-perceived cost of schema changes is bounded by the rebuild duration, not by data loss or manual migration steps.

### LD-50. Plugin event surface review at v0.5 milestone

Before v1.5+ external publication of `orgsidian-plugin-api`, conduct a dedicated event-surface review in v0.5 Beta:

- Audit every `Event` variant added during v0.1 → v0.5 — is the name still semantically correct? Is the granularity still right? Should two variants merge or one split?
- Audit every hook method signature — would adding an optional parameter (`Option<T>`) downstream be a breaking change? If so, document the contract carefully.
- Audit `HookOutcome` semantics — does `Continue / Replace / Cancel` cover all observed plugin use cases from the bundled v0.5 plugins, or is a 4th variant needed (e.g., `Defer(Duration)` for async-rescheduled hooks)?
- **Output**: a sign-off document `docs/plugin-api/v1.0-surface-review.md` committed before the v0.5 → v1.0 transition. v1.0 → v1.5+ publication blocked until this document exists and is reviewed.

### LD-51. CSS token snapshot test

- `shell-ui/src/themes/tokens.css` is the **canonical source** for `--org-*` CSS variable names.
- A Vitest snapshot test in `shell-ui/src/themes/tokens.test.ts` extracts the set of `--org-*` variables defined in `tokens.css` and compares against the committed snapshot `shell-ui/src/themes/__snapshots__/tokens.snap`.
- **Any rename, removal, or addition** of an existing variable fails the test; explicit acceptance requires snapshot update + a CHANGELOG entry under "Theme API."
- **Naming convention enforced**: semantic granularity (`--org-headline-h1-fg`, `--org-accent-todo`), never structural (`--org-color-blue-500`). Structural tokens are an *internal implementation* detail of `tokens.css`, never exposed in the public theme API.
- This locks the FR-22 token vocabulary as a **public contract** with theme authors from v0.5 Beta onward (when the theme loader ships). Token additions during v0.6..v1.0 require coordinated CHANGELOG bumps.

### LD-56. Backlink Graph View rendering library

**Decision.** `react-force-graph-2d@1.29.1` (canvas + d3-force, React-native bindings, MIT).

**Context.** FR-26 (v0.1 Alpha) requires a force-directed Backlink Graph rendering Headlines with `:ID:` as nodes and `[[id:...]]` / `[[wiki-link]]` references as edges, with click-to-Source navigation via `:ID:` lookup, pan/zoom, zoom-in labels, and an empty-state coaching message. Perf budget: ≤5k nodes <2s on 2020+ baseline hardware. Must render inside the Tauri webview alongside the locked React 19 + TanStack Router + CodeMirror 6 stack without bloating boot time.

**Options considered.**
- `react-force-graph-2d`: React-first wrapper over `force-graph` (canvas + d3-force); `onNodeClick(node)` exposes full node payload incl. configurable `nodeId`; peer `react: "*"` (React 19 compatible); deps = `force-graph` + tiny `react-kapsule` + `prop-types`. **Selected.**
- `sigma.js`: WebGL-native, faster at 50k+ nodes, but no React bindings (would need a hand-rolled imperative wrapper), heavier integration cost, WebGL overkill at our 5k budget. Rejected on solo-OSS maintenance burden.
- `cytoscape.js`: layout-rich (cose, dagre, klay) but ~3× the bundle of force-graph and its React wrapper (`react-cytoscapejs`) lags releases. Rejected on bundle + maintenance lag.

**Rationale.**
- Bundle: canvas-only path avoids pulling Three.js/WebGL (2D variant deps: `force-graph` + `react-kapsule` + `prop-types`; no large transitive graph).
- Perf headroom: d3-force-on-canvas comfortably handles 5k nodes in <2s on M1-class hardware, with documented headroom toward 10k for v0.5+ vault scale.
- React-first: idiomatic `<ForceGraph2D graphData={...} nodeId="id" onNodeClick={n => routeTo(n.id)} />` — no imperative bridge, plays cleanly with TanStack Router navigation per LD-29.
- Maintained by `vasturiano` with steady release cadence (1.29.x current); mature, low-surprise surface for solo-OSS upkeep.
- Doesn't lock out v0.5+ features: custom node rendering (`nodeCanvasObject`), subgraph filtering via `graphData` swap, edge styling per `[[id:]]` vs `[[wiki:]]` via `linkColor`/`linkWidth` accessors.

**Consequences.**
- Dependency added to `packages/shell-ui/package.json` at pinned `react-force-graph-2d@1.29.1` (stack-versions table).
- Component lives at `packages/shell-ui/src/components/graph/`.
- Index API: `orgsidian-index/src/query/graph.rs::adjacency(scope) -> GraphData { nodes: Vec<NodeRef{ id, file, title }>, edges: Vec<Edge{ src_id, dst_id, kind }> }`. Reuses the `links` table already populated by FR-13 Backlinks (LD-13).
- Perf gate in LD-32 nightly: synthetic 5k-node force-directed render ≤2s baseline runner; ≤500ms steady-state frame after layout settle.
- A11y fallback: a textual `GraphNodeList.tsx` view (sorted by degree, keyboard-reachable, alphabetical jump) shipped alongside the canvas view; the hard CI gate per LD-58 covers it. Toggle via View menu or `g l` chord.
- Cross-webview tested in CI: macOS WebKit + Linux WebKitGTK + Windows WebView2 (Tauri matrix per LD-32 nightly).

**Open follow-ups.**
- v0.5+: typed-edge styling for `[[id:]]` vs `[[wiki:]]`; subgraph filter UI (by tag, by file); evaluate `nodeCanvasObject` custom labels for high-density zoom-in legibility; benchmark at 10k+ nodes against the v0.5 vault-scale target.

### LD-57. Cross-file write atomicity for FR-25 Refile

**Decision.** **Sequence-with-`.bak`-restore** pattern for atomic Refile across source + destination files.

**Context.** FR-25 Refile moves a Headline + subtree from a source `.org` file to a destination `.org` file. From the user's perspective the operation is atomic — there is no observable state where the Headline exists in both files or in neither. The Single Writer Rule (LD-7) covers per-file integrity; cross-file atomicity is its own problem.

**Options considered.**
- **Sequence-with-`.bak`-restore.** (1) Both files must be clean (no Dirty Buffer); if dirty, prompt save-first. (2) Snapshot destination to `<dest>.bak.<pid>.<ts>` in the same directory. (3) Atomic-write destination (subtree inserted at chosen outline path) via the LD-8 temp-rename pattern. (4) Atomic-write source (subtree removed). (5) On success: delete the `.bak` and emit watcher-suppress tokens for both files. (6) On step-4 failure: restore destination from `.bak`; surface error; both files end at pre-Refile state. **Selected.**
- Write-both-to-temp-then-atomic-rename pair. Stronger theoretically, but cross-directory atomic rename is unreliable on Windows (and even on POSIX is per-directory, not global); the second rename can fail after the first commits — same failure mode as sequence-with-bak but harder to reason about and reverse. Rejected on reliability + Windows fragility. The pattern Emacs `org-refile` itself uses is morally sequence-with-bak.

**Rationale.**
- The `.bak` file is a known recoverable artifact; orphans are collected by the LD-41 startup scan (extended to match `*.bak.*` patterns under the Vault).
- Both-clean precondition keeps the recovery story bounded — no Dirty Buffer state to reconcile during rollback.
- Watcher-suppress tokens (already in the save-cycle data-flow) prevent the watcher from firing Merge Dialogs on Orgsidian's own writes during the operation.
- Implementation lives in `crates/orgsidian-core/src/orchestrator/refile.rs`; the subtree extract/insert primitives live in `orgsidian-vault/src/refile.rs` using the tree-sitter-org grammar for boundary detection.

**Consequences.**
- New row in LD-41 failure catalog (added 2026-05-20) for "Refile partial completion".
- LD-7 extended with cross-file pointer to this LD.
- v0.5 Beta scope (per PRD §6.2 + FR-25): the Refile orchestrator + target picker UI lands here. Story scaffolding in `epics.md` deferred to next reconciliation pass.
- Fault-injection test: inject `ENOSPC` on source write after destination commit; assert destination rolls back from `.bak` and both files match pre-Refile bytes.
- Future-extension hook: the same pattern generalizes to "multi-headline batch Refile" (v0.6+) — same orchestrator, snapshot all touched files first.

### LD-58. Accessibility CI gates (WCAG 2.1 AA hard gate from v0.1 Alpha)

**Decision.** Three hard CI gates enforcing WCAG 2.1 AA from v0.1 Alpha, integrated into the LD-32 per-PR matrix.

**Context.** PRD §8 (post-2026-05-20 reconciliation with UX design specification Experience Principle 9) elevates WCAG 2.1 AA from a soft target to a hard CI gate from v0.1 Alpha. Three gates required: (1) contrast-matrix verification, (2) axe-core automated WCAG rule scan on every primary surface, (3) keyboard-only Playwright scenarios. Full screen-reader certification remains a v1.5+ commitment.

**The three gates.**

1. **Contrast-matrix gate.** A Vitest test in `shell-ui/src/themes/contrast.test.ts` extracts all `--org-*-fg` / `--org-*-bg` token pairs from `tokens.css` (canonical per LD-51) for both `dark` and `light` themes; computes WCAG relative-luminance contrast ratio (`(L1 + 0.05) / (L2 + 0.05)`); asserts ≥4.5:1 for body text pairs and ≥3:1 for large text / UI chrome pairs. New tokens that don't declare their pair role in `tokens.css` metadata fail the gate (forces explicit categorization).

2. **axe-core gate.** `@axe-core/playwright` integrated into the existing Playwright `e2e/` suite. Each happy-path scenario auto-runs `await new AxeBuilder({ page }).analyze()`; any violations at `serious` or `critical` impact fail the test. Configured rule subset: WCAG 2.1 AA tags (`wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`), no best-practice warnings (avoid noise that erodes the gate).

3. **Keyboard-only scenarios gate.** **One happy-path keyboard-only scenario per primary surface** (PRD §8 + UX spec post-2026-05-20 decision): Today Dashboard, Agenda, Editor, Quick Capture, Settings, Graph View. Each scenario starts with `page.keyboard` only (no `mouse.click`), navigates to the surface, completes a representative action (e.g., on Agenda: tab to a Headline, press Enter to open Editor; on Quick Capture: hotkey, type, submit, verify Inbox), and asserts the action's persisted side-effect. Per-PR runtime budget: ≤2-3 min total for the 6 scenarios on macOS-arm64 + Ubuntu-LTS. Exhaustive per-surface coverage is **deferred to v1.0 graduation** — happy-path is the v0.1 hard floor.

**Consequences.**
- Stack-versions table pins `@axe-core/playwright` (latest stable).
- LD-32 per-PR job adds a `pnpm a11y` step running: `pnpm test:contrast` (Vitest) + `pnpm test:e2e -- --grep @a11y` (Playwright kbd + axe scenarios tagged with `@a11y`).
- Test scaffolding lives at `packages/shell-ui/src/themes/contrast.test.ts` + `packages/shell-ui/e2e/a11y/`.
- Validation Coverage (line ~1366) reworded — full **screen-reader** certification remains v1.5+; contrast + keyboard nav are v0.1 hard gates, not deferred.
- Graph View (LD-56) ships its `GraphNodeList.tsx` textual fallback to clear gate #3 — canvas-only graphs are otherwise keyboard-hostile.
- Token authors (theme contributors from v0.5 onward, per LD-51) inherit the contrast gate — themes that fail contrast can't merge.

**Open follow-ups.**
- v0.5+: expand keyboard-only scenario coverage from happy-path to representative-coverage (multiple scenarios per surface). Decision-grade question at v0.5 retro.
- v1.0: evaluate `axe-core` rule expansion (best-practice tier) — likely gradual ramp, not big-bang.
- v1.5+: full assistive-tech certification audit (NVDA + JAWS + VoiceOver matrix); ARIA live regions + structured navigation landmarks across all surfaces.

### Project Tree Amendment

Add to the root tree (step 6):

```
orgsidian/
├── …
├── SECURITY.md                               # security policy + disclosure + patch cadence (per LD-47)
├── docs/
│   ├── adr/
│   │   └── 0001-corpus-subset-selection.md   # LD-44
│   ├── architecture/
│   │   └── resilience.md                     # LD-47 fallback plans
│   ├── parser/
│   │   └── KNOWN_DIVERGENCES.md              # LD-45
│   ├── perf/
│   │   ├── large-vault-scaling.md            # LD-42
│   │   └── memory-soak-reports/              # LD-43 nightly reports
│   ├── plugin-api/
│   │   └── v1.0-surface-review.md            # LD-50 (created at v0.5 milestone)
│   └── security/
│       └── advisory-exceptions.md            # LD-37 quarterly review
├── crates/
│   └── test-plugin-panic/                    # LD-38 chaos test plugin (workspace member, dev-only)
├── shell-ui/src/themes/
│   ├── tokens.test.ts                        # LD-51 snapshot test
│   └── __snapshots__/
│       └── tokens.snap                       # LD-51 committed snapshot
├── commitlint.config.cjs                     # LD-54 enforcement
├── cliff.toml                                # LD-54 CHANGELOG generation (git-cliff)
├── .husky/
│   └── commit-msg                            # LD-54 client-side gate
├── .github/
│   ├── labels.yml                            # LD-55 label scheme
│   ├── ISSUE_TEMPLATE/
│   │   └── story.md                          # LD-55 issue template
│   └── workflows/
│       ├── commitlint.yml                    # LD-54 CI gate (or folded into pr.yml)
│       ├── sync-issues.yml                   # LD-55 epics.md → Issues
│       └── labels-sync.yml                   # LD-55 apply labels.yml
└── tools/
    └── issues-sync/                          # LD-55 sync binary (outside [workspace.members])
```

`SECURITY.md` contents (template):
- Security patch SLA: **within 14 days** of credible disclosure.
- Reporting channel: GitHub Security Advisories (preferred), email fallback `security@orgsidian.example`.
- Supported versions: latest minor of latest major receives patches; older minors best-effort.
- Disclosure policy: 90-day coordinated disclosure default; immediate disclosure for actively exploited.

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:** All technology choices are mutually compatible. License (MIT) is compatible with every dependency in the stack (Tauri, tree-sitter, rusqlite, atomic-write-file, React, Tailwind, TanStack, shadcn, tauri-specta — all MIT or MIT/Apache-2.0 dual). The Rust-native internal stack and the React-native frontend stack are each internally coherent and connect cleanly across the IPC layer (`tauri-specta`).

**Pattern Consistency:** Naming conventions are consistent across surfaces (snake_case Rust + camelCase TS + camelCase IPC wire via project-wide specta rename). Plugin API design (hook-with-priority + observer hybrid, message-passing semantics, `HookContext`/`PluginContext` as traits in the leaf crate) coheres with both v1.0 static linking and the v1.5+ WASM migration target. The 9-crate workspace LEAF discipline is necessary and sufficient to support v1.5+ independent publication of `orgsidian-plugin-api`.

**Structure Alignment:** The 9-crate Cargo workspace + `shell-ui/` at repo root + `tools/corpus-extractor/` outside the workspace matches the documented dependency graph. The `crates/test-plugin-panic/` (LD-38) is the only addition to workspace membership during round 5.

### Requirements Coverage Validation ✅

**Functional Requirements:** All 24 FRs (FR-1 through FR-24) have explicit architectural mapping (step 6 FR table).

**Non-Functional Requirements:**
- Performance budgets addressed by LD-4, LD-14, LD-30, LD-32, LD-42 (large-vault), LD-43 (memory soak).
- Round-trip fidelity enforced at L0/L1/L2 by LD-32 + LD-44 (subset criteria) + LD-45 (Emacs oracle pinning).
- Cross-platform parity via CI matrix (LD-32).
- Data sovereignty via CSP (LD-18), zero telemetry (LD-23), fs allow-list (LD-17), Vault-self-contained state (LD-40).
- Reliability via atomic writes + AV retry (LD-8), Single Writer Rule (LD-7), startup integrity check + rebuild policy (LD-13), failure mode catalog (LD-41), multi-instance lockfile (LD-39), plugin panic isolation (LD-38).
- Accessibility via shadcn/Radix primitives + Merge Dialog custom focus; **WCAG 2.1 AA contrast + keyboard navigation as hard CI gate from v0.1 Alpha** (LD-58 — axe-core + contrast-matrix + Playwright happy-path keyboard scenarios on the 6 primary surfaces; PRD §8 post-2026-05-20). Full screen-reader certification (assistive-tech compatibility audit) remains deferred to v1.5+.
- i18n infrastructure committed; library selected: Lingui v6.x (LD-52).

### Implementation Readiness Validation ✅

**Decision Completeness:** LD-1 through LD-52 + step-6 amendments cover every architectural commitment. Stack version pinning explicit (step-3 versions table + LD-47 Tauri exception + LD-52 Lingui pinning).

**Structure Completeness:** Full workspace tree + per-crate dependency graph + FR→component mapping + integration data-flow diagrams + failure mode catalog (LD-41) + project-tree amendment for round 5 docs.

**Pattern Completeness:** Step 5 covers naming, communication, process, and AI-agent mandatory rules + anti-patterns. Round 5 hardening adds dependency audit, panic isolation, memory regression, schema migration safety net.

### Gap Analysis Results

**Critical Gaps:** None remaining. The 4 critical-disguised-as-important gaps from round 5 are resolved:
- Plugin panic isolation → LD-38.
- Multi-instance concurrency → LD-39.
- Backup/restore (Vault-self-contained state) → LD-40.
- Failure mode catalog → LD-41.

**Important Gaps (resolve before relevant story sprints):**

1. ~~**PDF rendering crate selection for `orgsidian-report`**~~ — ✅ **Resolved 2026-05-19 (LD-53):** `typst` embedded via `typst-as-lib` (`typst@0.14` / `typst-pdf@0.14` / `typst-as-lib@0.15`). Original five-candidate spike (`printpdf`, `genpdf`, `typst`, `wkhtmltopdf`, `weasyprint-rs`) is superseded by the LD-53 decision; `printpdf` 0.9.x retained as the documented downgrade path. v0.5 Beta FR-14 ships both PDF and HTML output paths from day one of the sprint.
2. ~~**PRD reconciliation** (LD-46)~~ — ✅ **Resolved 2026-05-19:** PRD wave-1 reconciliation applied MIT + tree-sitter-org (LD-1, LD-3) to PRD §7.3, OQ-1, OQ-2, OQ-8, addendum §A.2, §A.3; PRD wave-2 reconciliation (same date) threaded LD-48 (vendoring + maintenance contingency) into the same spots and applied LD-52 to PRD §8 + LD-53 to PRD §10 OQ-6. PRD `status: final` preserved; both reconciliations recorded under `revisions:` in the PRD frontmatter and in the PRD `.decision-log.md`.

**Nice-to-Have Gaps (deferrable):**

4. Concrete starter theme example (Gruvbox/Solarized) — v0.5 Beta or later.
5. Pre-commit hook configuration detail — Story 2.
6. Documentation site generator choice (mdbook vs Docusaurus) — v1.0+ when `docs/user-guide/` grows.
7. Pinned `cmdk` version for the Command Palette — scaffold time.

### Architecture Completeness Checklist

**Requirements Analysis**

- [x] Project context thoroughly analyzed (step 2)
- [x] Scale and complexity assessed (step 2)
- [x] Technical constraints identified (steps 1-2 + Party Mode rounds)
- [x] Cross-cutting concerns mapped (step 2)

**Architectural Decisions**

- [x] Critical decisions documented with versions (LD-1..LD-52 + version policy memory)
- [x] Technology stack fully specified (step 3 versions table + LD-24..LD-26 + LD-47)
- [x] Integration patterns defined (LD-24..LD-31)
- [x] Performance considerations addressed (LD-4, LD-14, LD-30, LD-32, LD-42, LD-43)

**Implementation Patterns**

- [x] Naming conventions established (step 5)
- [x] Structure patterns defined (step 5)
- [x] Communication patterns specified (step 5)
- [x] Process patterns documented (step 5 + LD-41 failure modes)

**Project Structure**

- [x] Complete directory structure defined (step 6 + round-5 amendments above)
- [x] Component boundaries established (step 6 boundaries + LD-37 dep graph CI gate)
- [x] Integration points mapped (step 6 data-flow diagrams)
- [x] Requirements to structure mapping complete (step 6 FR table + traceability discipline)

### Architecture Readiness Assessment

**Overall Status:** **READY WITH MINOR GAPS**

All 16 checklist items checked. Zero critical gaps remain after round-5 hardening (LD-37..LD-51 closed the 4 critical-disguised-as-important findings). Two important gaps remain (after LD-52 closed the i18n gap on 2026-05-19) — both deferred to clearly bounded decision windows before relevant story sprints, none blocking v0.1 Alpha bootstrap.

**Confidence Level:** **MEDIUM-HIGH**

Downgraded from "HIGH" (step 7 initial draft) per Party Mode round 5 meta-critique: every prior Party Mode round found at least one critical issue the prior validation had cleared, so any post-validation claim of "HIGH" without a fresh sweep is bias-of-confirmation. Round 5 was the fresh sweep; it surfaced 18 substantive concerns now closed. Confidence is MEDIUM-HIGH (not HIGH) because:

- The 2 remaining important gaps have deferred but real decisions ahead (PDF crate, PRD reconciliation). The i18n library gap was closed by LD-52 on 2026-05-19.
- The "0 critical gaps" claim now depends on the absence of a *6th* round of adversarial findings — historically possible but increasingly unlikely as the surface narrows.
- Downgrade signals: if the PDF spike fails (no acceptable crate), confidence drops to MEDIUM; if PRD reconciliation reveals deeper drift, drops to MEDIUM; if dependency audit (LD-37) on first CI run finds a transitive GPL, drops to LOW.

**Key Strengths:**

- **Stack internal coherence**: MIT license + Rust-native ecosystem alignment + verified production prior art (markora, solomd) + research-validated tooling choices (tauri-specta, rusqlite, atomic-write-file, nvim-orgmode/tree-sitter-org).
- **Plugin API forward-compatibility**: trait-based context + `#[non_exhaustive]` events + hook-with-priority + WASM-compatible message-passing design + internal-until-v1.5+ publication + event surface review gate (LD-50) preserves the v1.5+ public path without breaking changes.
- **Failure-mode discipline**: full failure catalog (LD-41), plugin panic isolation (LD-38), multi-instance safety (LD-39), Vault-self-contained state (LD-40), schema-migration safety net via rebuild-index (LD-49).
- **Test discipline**: three-level round-trip oracle (L0/L1/L2) + L0 subset selection algorithm (LD-44) + L2 Emacs oracle pinning (LD-45) + FR traceability test gate + perf regression gate + memory soak regression gate (LD-43).
- **Supply-chain hygiene**: `cargo audit` + `cargo deny` license allowlist + dependency graph CI check (LD-37), quarterly Tauri sync (LD-47), `tree-sitter-org` vendoring + maintenance contingency (LD-48).
- **Boundary integrity**: LEAF crate rule + Single Writer Rule + CM6-owns-editor-state + filesystem allow-list + DB-access-only-via-query-API + plugin-trait-only-data-access.

**Areas for Future Enhancement:**

- ~~PDF rendering crate selection during v0.5 Beta Project Report sprint.~~ ✅ Resolved 2026-05-19 (LD-53).
- ~~PRD §7.3 / OQ-1 / OQ-2 / addendum reconciliation pass before v0.1 starts.~~ ✅ Resolved 2026-05-19 (LD-46 wave-1 + wave-2; see Important Gap #2 above).
- WASM plugin runtime via `wasmtime` in v1.5+ (architecture preserves the path).
- True WYSIWYG via ProseMirror re-evaluation in v1.5+ based on v0.5 Beta feedback.
- Telemetry / crash reporting decisions in v0.5+ / v1.5+ contingent on backend availability.
- Plugin event surface review (LD-50) at v0.5 milestone before v1.5+ publication.

### Implementation Handoff

**AI Agent Guidelines:**

1. Follow LD-1 through LD-52 (with step-6 amendments) as binding architectural decisions.
2. Use implementation patterns from step 5 consistently across all PRs.
3. Respect project structure and boundaries from step 6 + round 5 amendments.
4. Add `//! Implements FR-NN` doc-comments to every module that satisfies an FR; the `tests/traceability.rs` gate fails if any FR is unmapped.
5. Update LD-NN entries with addendum lines when implementation forces a change. Staleness is the failure mode.
6. When a version is pinned, follow the version policy (latest stable or LTS; Tauri ecosystem exempted per LD-47).
7. Every new dependency added to a `Cargo.toml` must pass `cargo audit` + `cargo deny check licenses` locally before PR.
8. Every plugin invocation site uses the `invoke_plugin_hook!` macro (LD-38).
9. Every new `Event` variant or hook method on `OrgsidianPlugin` is recorded in `crates/orgsidian-plugin-api/CHANGELOG.md` with SemVer rationale.
10. Every change to `--org-*` CSS variables in `shell-ui/src/themes/tokens.css` triggers the snapshot test (LD-51); intentional changes require snapshot update + CHANGELOG entry.

**Pre-v0.1-Alpha Workstreams (before story sprints begin):**

1. **PRD reconciliation** (LD-46): update PRD §7.3, §10 OQ-1, §10 OQ-2, addendum §A.2, §A.3.

**First Implementation Story Sequence:**

1. **Story 1**: Bootstrap Tauri 2.x + React 19 + TS scaffold via `pnpm create tauri-app@2` (project name `orgsidian`, identifier `com.orgsidian.app`).
2. **Story 2**: Refactor to 9-crate Cargo workspace + `shell-ui/` at root + JS workspace; install full Tauri plugin set; install Tailwind 4 + shadcn (forked) + TanStack Router; configure `tauri-specta`; set `[profile.release] panic = "unwind"` (LD-38); commit `Cargo.lock`; add `cargo-deny` config (LD-37); add `SECURITY.md`.
3. **Story 3**: Wire CodeMirror 6 host in `shell-ui/src/components/editor/`; StrictMode-safe `EditorView` lifecycle; first round-trip smoke test.
4. **Story 4**: `orgsidian-index` schema + `rusqlite_migration` (LD-12) + connection pool (LD-14) + `orgsidian index rebuild` CLI command (LD-49).
5. **Story 5+**: Continue feature by feature per FR mapping (step 6).

**Implementation milestone gates** (from PRD §6, with architecture hooks):
- **v0.1 Alpha (Months 3-6)**: scaffold + parser + index + watcher + first surfaces; CI matrix online; round-trip L0 gate live.
- **v0.5 Beta (Months 7-12)**: full feature set; PDF rendering chosen and shipped; theme loader live + LD-51 snapshot test; event surface review LD-50 conducted; chaos test plugin LD-38 in CI.
- **v1.0 (Months 13-18)**: Windows added; perf polish; auto-update channel; full a11y review.
- **v1.5+ (post-1.0)**: `orgsidian-plugin-api` published to crates.io; WASM plugin runtime; ProseMirror WYSIWYG re-evaluation.

— *End of architecture document.*
