---
stepsCompleted: [1, 2, 3, 4]
status: complete
completedAt: '2026-05-19'
revisions:
  - date: 2026-05-19
    summary: Sprint Change Proposal (correct-course) absorbed. NEW Stories 1.13-1.16 added to Epic 1 (GitHub org/repo/Project board + commitlint/husky + git-cliff + Issues sync). Story 1.10 AC extended with Conventional Commits section + test-strategy pointer in CONTRIBUTING.md. Duplicate Story 1.10 block at former lines 592-603 removed (verbatim duplication cleanup). Process Discipline rule H added pointing to `_bmad-output/test-artifacts/test-design.md` as authoritative system-level test strategy. Story 6.10 AC extended with repo visibility flip (private→public) before SM-1 announcement. No other story content modified. See `_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md`.
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/addendum.md
  - _bmad-output/planning-artifacts/architecture.md
partyModeRounds: 2
partyModeRound2Findings:
  - "Persona controlled vocabulary established (Paige)"
  - "Story-level traceability discipline rule + `Traces:` line + `Implements FR-NN` AC (Paige)"
  - "User-voice in `So that` rule + `FR-XX` removal directive (Sally)"
  - "Perf assertions via shared `assert_no_perf_regression!` infrastructure — Story 1.12 (Murat)"
  - "AC refactor rule: >4 And chains split into separate ACs — Story 4.3 split into 4.3a..4.3g (Paige + Murat)"
  - "Microcopy discipline: [draft]/[final] markers + `docs/microcopy-registry.md` (Sally)"
  - "Story 1.11: LD-41 failure-mode test harness (Murat P0)"
  - "Story 2.3 enumerated 14 LD-44 syntax constructs (Murat P0)"
  - "Story 6.5 cargo-semver-checks automation replaces manual freeze gate (Murat)"
  - "Story 6.6 NEW: hardcoded UJ-4 coaching balloons in v0.1; Story 11.4 refactors to registry (Sally)"
  - "Story 7.6 extended with `last_active_at` field for Story 7.7 stale-clock pre-fill (Sally)"
  - "Story 7.7 microcopy rewritten with [draft] marker, safest-default focus, currently-logged value (Sally)"
  - "Story 8.1 return-focus AC (UJ-2 round-trip ≤3s) (Sally)"
  - "Story 8.8 NEW: UJ-6 spine integration test (Sally)"
  - "Story 10.7 NEW: UJ-3 spine integration test with open-clock-warning assertion (Sally)"
  - "Story 11.4 rewrites user-story in user-voice, references Story 6.6 migration (Sally)"
  - "Story 12.4 [MANUAL-GATE] markers + artifact-based AC + lint-checkable structure (Murat)"
  - "Story 13.5 axe-core 0-serious/critical violations CI gate + manual qualitative sign-off (Murat)"
---

# orgsidian - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for orgsidian, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR-1: Open and parse `.org` files from the Vault and render them correctly per the supported org-mode syntax conventions (headlines, TODO states, scheduled timestamps, drawers, inline markup, links). Malformed files must fall back to plain-text view with a structured warning, never crash. Realizes UJ-1, UJ-5.

FR-2: Round-trip preservation — files saved by Orgsidian without user-visible edits must be byte-identical to their on-disk version (modulo documented trailing-newline normalization). Enforced by automated CI gate on every release.

FR-3: Switch the current file between Raw, Pseudo-WYSIWYG, and Split Editor Modes via UI control and keyboard shortcut. Default is Pseudo-WYSIWYG; per-file choice persists across restarts. Mode switch under 200ms on a 5,000-line file.

FR-4: In Pseudo-WYSIWYG mode, render headings with hierarchical font sizes, TODO-state badges, tag pill labels, timestamps as readable dates, checkbox widgets, and clickable links — while the underlying buffer remains source `.org` text. Cursor placement, copy-paste, and find/replace operate on source positions.

FR-5: Provide cross-platform default keybindings (Cmd on macOS, Ctrl on Linux/Windows) and an optional "Emacs keybindings" mode covering daily org-mode actions (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out). In-app keybinding reference panel.

FR-6: Open on the Today Dashboard by default (configurable to start on last-open file). The dashboard shows: items Scheduled for today, items with Deadline today or overdue, items flagged with a configurable "today" tag, Inbox preview (first N entries), and the Active Clock if any. Render within 500ms on a 1,000-file Vault. Each section collapsible with persistent preferences. Empty-state messages per section.

FR-7: Switch the Agenda between Today, Week (rolling 7 days), and Custom (date range picker) views. Filters by tag, TODO state, and file path are composable; filters persist within session. Saved named filter presets. View switch under 200ms on a 1,000-file Vault.

FR-8: Clock in to a Headline (start an Active Clock), clock out (record entry as `CLOCK:` line in LOGBOOK drawer), and resume a previously paused clock. At most one Active Clock at a time; clocking into a new Headline auto-stops the prior. On launch, detect a running Clock from the prior session and prompt: discard / adjust end time / keep running. Compute time totals per Headline, per subtree, per tag, per date range for use in Project Report and Agenda.

FR-9: Add, modify, or remove a Scheduled timestamp or Deadline on the current Headline via keyboard shortcut or context menu. Date picker for fast entry; raw timestamp typing supported in Raw mode. Recurring timestamps (e.g., `<2026-05-19 Mon +1w>`) are preserved on round-trip and respected by Agenda.

FR-10: Invoke Quick Capture from anywhere on the OS via a configurable global hotkey (default `Cmd/Ctrl+Shift+Space`). Dialog appears centered, accepts multi-line text, appends entry to the configured Inbox file on submit, without focus-stealing the main application. End-to-end latency under 1 second. Dialog dismisses on submit and on Escape. Captured entries include a creation timestamp drawer entry by default (format configurable).

FR-11: Provide a system tray menu offering Quick Capture as a fallback to the hotkey, on platforms that support it (macOS menubar, Windows tray, Linux indicator where available). Enabled by default; disable-able in Settings. Functionally identical to hotkey-launched Quick Capture.

FR-12: Full-text search across the Vault via `Cmd/Ctrl+P` (or `Cmd/Ctrl+Shift+F`). Matching results from all `.org` files grouped by file with the matched line previewed. Selecting a result opens the file at that line. Query syntax supports plain words, exact phrase quotes, tag filter (`#tag:`), file filter (`file:`), TODO state filter (`todo:`). Latency under 200ms for first 50 results on a 1,000-file Vault.

FR-13: When the cursor is on a Headline, a sidebar Backlinks panel shows all other Headlines referencing this one via `id:` link or `[[wiki-link]]`. Clicking a backlink navigates to the source. Panel updates within 100ms of cursor moving to a new Headline. Backlinks include the linking Headline's title plus a short context snippet.

FR-14: Project Report export — user selects a scope (file, Headline subtree, or tag) and a date range, triggers an export to PDF or HTML in v1.0. Report includes: TODO completions in range, Clock entries summed per Headline and total, linked notes presented as their Headline title plus a one-line context excerpt grouped by source file (no LLM summarization), and milestone status (Headlines tagged as milestones). Active Clock with no end time is flagged explicitly. Report generation under 5 seconds for a typical scope (50 headlines, 4 weeks). Output formatting customizable via Typst `.typ` templates for PDF and HTML/CSS for HTML (per LD-53).

FR-15: Designate a folder as a Vault via the file picker on first launch or via Settings. Orgsidian recursively indexes all `.org` files in the folder. One Vault open at a time. Initial indexing of a 1,000-file Vault completes in under 30 seconds; indexing progress is visible to the user. Subsequent launches with an unchanged Vault open the cached index in under 1 second.

FR-16: Watch the Vault folder for external file changes. External writes on a file with a clean buffer trigger automatic reload + Agenda re-index; external writes on a file with a Dirty Buffer open a three-pane Merge Dialog (Yours / External / Merged) with per-hunk selection and free-edit of the Merged pane. Saving writes the Merged pane atomically and clears Dirty state; cancelling preserves the Dirty Buffer. External writes detected within 5 seconds on macOS, Linux, and Windows.

FR-17: SQLite index is fully derived — never the source of truth. The index file lives in an OS-conventional application support location (never inside the Vault by default). Deleting the index file and relaunching produces an identical Agenda and Search experience after a rebuild.

FR-18: On first launch with no configured Vault, present four Starter Vault choices: Personal GTD, Student, Freelancer, Empty. Selecting one creates a folder at a user-chosen location and populates it with realistic example `.org` files (project, inbox, journal, someday list), opening directly on a non-empty Today Dashboard with example content. Freelancer starter includes at least one example project with milestones, a clocked task, and a backlink. (v0.1 Alpha ships Personal GTD + Student only; Freelancer + Empty land in v0.5 Beta.)

FR-19: Interactive Tutorial — launchable from a "Get started" menu or first-launch prompt. Walks the user through one full workflow cycle: capture a thought, triage to a project, schedule it, see it in Agenda, clock in/out, generate a one-line report. Estimated time: 10 minutes. Completion tracked locally (no telemetry); re-launchable from Settings. (v1.0 feature.)

FR-20: Plain Mode / Power Mode toggle in Settings. Plain Mode hides a documented list of advanced commands from menus and command palette but keeps them reachable via direct keyboard shortcut. Power Mode exposes everything. Default is Plain Mode for new users; switching modes does not require app restart.

FR-21: Inline Coaching — empty states (empty Today Dashboard, empty Inbox, never-clocked-in, never-searched) display contextual coaching text suggesting the next action. Command palette descriptions written for discoverability. Coaching text is dismissible per-context; "Don't show again" persists. A "show all coaching tips" reset action exists in Settings.

FR-22: Theme — ship dark and light default themes (WCAG AA contrast for body text and primary UI chrome). User can supply a custom CSS file via Settings to override colors, fonts, and spacing; theme switching is instant. Invalid CSS does not crash the app (falls back to default with a warning). Themable CSS token vocabulary (`--org-*` variables) is a public contract from v0.5 Beta onward.

FR-23: Keybinding remapping — user can remap any documented action to a different keybinding via Settings. Remappings persist per Vault. Conflict detection warns when an assigned chord conflicts with an existing binding.

FR-24: Internal Plugin Pattern — Orgsidian's own v1.0 features (Agenda, Quick Capture, Search, Project Report, Themes) are implemented as internal plugins registered against a hooks-and-registry system. The `orgsidian-plugin-api` crate is internal during v0.1 → v1.4; SemVer discipline + contract tests + changelog tracked from day 1. The trait surface (hook-with-priority + observer hybrid, `HookOutcome::{Continue,Replace,Cancel}`, `#[non_exhaustive]` Event enum, `HookContext`/`PluginContext` as traits) is designed WASM-compatible for v1.5+ external publication. Adding a new internal feature does not require modifying core engine code (validated by v0.5 → v1.0 transition).

### NonFunctional Requirements

NFR-1 (Performance — startup): cold launch with cached index to Today Dashboard interactive under 2 seconds on baseline 2020+ M1 / x86_64 hardware with a 1,000-file Vault.

NFR-2 (Performance — typing): editor typing latency under 30ms (perceptual code-editor budget).

NFR-3 (Performance — agenda recompute): under 100ms after a single-file edit on a 1,000-file Vault (incremental index update, not full rebuild).

NFR-4 (Performance — search): under 200ms for first 50 results on a 1,000-file Vault (SQLite FTS5 backing).

NFR-5 (Performance — Quick Capture): end-to-end (hotkey → dialog visible → submit → persisted) under 1 second on a baseline laptop.

NFR-6 (Performance — editor open): opening a 5,000-line `.org` file renders the first screen in under 300ms on baseline hardware.

NFR-7 (Performance — memory): under 500MB resident on a 1,000-file Vault under typical editing load.

NFR-8 (Cross-platform parity): v1.0 ships feature-equivalent macOS, Linux, and Windows builds. macOS + Linux only for v0.1 Alpha and v0.5 Beta. Linux distribution via AppImage (primary) + Flatpak (best-effort); Windows added at v1.0 via MSI.

NFR-9 (Accessibility): WCAG 2.1 AA for body text contrast and keyboard navigation of all menus and primary surfaces. Screen reader support best-effort in v1.0; full a11y audit deferred to v1.5+.

NFR-10 (Internationalization): UI strings extracted for translation in v1.0; default English; translator-facing catalog format `.po` (Gettext) at `packages/shell-ui/src/locales/{lng}/messages.po`, compiled to TypeScript at build time via Lingui v6.x. Translations community-driven; infrastructure ships in v1.0.

NFR-11 (Privacy — no telemetry default): no telemetry by default. Any future opt-in telemetry must be explicit, visible, and disable-able. No telemetry code or UI ships in v1.0 (LD-23).

NFR-12 (Privacy — no network calls in core workflow): open, edit, capture, agenda, search, report, save — none require network access. Auto-update checks are the only built-in network call and are disable-able. CI verifies zero network calls in core paths.

NFR-13 (Privacy — no cloud account): Orgsidian has no account system. No proprietary sync, no Orgsidian-hosted server.

NFR-14 (Data sovereignty — files are source of truth): `.org` files are authoritative; SQLite index is derived and disposable. The Vault folder is the user's folder; Orgsidian creates no files inside it without user action.

NFR-15 (Reliability — atomic file writes): all writes use temp-file-and-rename atomic semantics on macOS, Linux, and Windows. Power loss during a save must not corrupt the source file. AV/Search-indexer transient locks handled by 3-retry exponential backoff (base 100ms).

NFR-16 (Reliability — Single Writer Rule): while Orgsidian holds a Dirty Buffer for a file, it is the sole writer; external writes to dirty files surface the Merge Dialog rather than silent overwrite. Race-condition surface tested deterministically.

NFR-17 (License): MIT. Maximally permissive; compatible with all stack dependencies (Tauri, tree-sitter-org, rusqlite, atomic-write-file, React, Tailwind, TanStack, shadcn, tauri-specta).

NFR-18 (Cost): Free, open-source, forever. No paid tier, no SaaS, no premium plugins.

NFR-19 (Round-trip CI gate): FR-2 enforced by automated CI on every release. L0 byte-identical save-no-op runs on the per-PR subset (~100 representative files, <60s); full corpus runs nightly; merge gate requires per-PR green AND nightly green within last 24h.

NFR-20 (Performance regression gate): perf snapshot regression gate ±10% on median of 5 runs against a fixed 1,000-file corpus, per PR.

NFR-21 (Memory soak regression gate): nightly 12-hour scripted session (200 buffers, 50 plugin re-init cycles, 1000 agenda queries) with RSS drift <10% over 11 hours (LD-43).

### Additional Requirements

The following architectural requirements (LD-1 through LD-53) influence implementation and must be reflected in the epic/story structure. They are not user-visible behaviors but constrain HOW the FRs are realized.

**Stack & Foundation (architecturally locked, day-1 scaffold):**

- LD-1 License MIT for the project itself; all dependencies must be MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unlicense, Zlib, or MPL-2.0 (LD-37 license allowlist). GPL/AGPL/proprietary/unknown licenses fail CI.
- LD-2 Stack: Tauri 2.x + Rust for `orgsidian-shell-app` and `orgsidian-cli`; React 19.1.x + TypeScript + Vite 6.x + Tailwind CSS 4.1.x + shadcn/ui (forked) for `shell-ui/`. Version pinning per [[feedback_version_policy]] (latest stable or LTS); Tauri ecosystem pinned exact-version with quarterly review (LD-47).
- LD-3 Parser: `nvim-orgmode/tree-sitter-org` (MIT) vendored as SHA-pinned git submodule at `crates/orgsidian-parser/grammar/` + custom Rust semantic layer in `@orgsidian/core/src/parser/semantic/`. Maintenance contingency per LD-48 (parser-owner role, v0.3 fork-and-maintain dry run, in-house fork trigger if upstream stalls >6 months).
- LD-4 Index: SQLite via `rusqlite` (FTS5 built-in). PRAGMAs locked: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`. FTS5 tokenizer: `unicode61 remove_diacritics 2` + `porter`. Application-level FTS5 sync (no triggers).
- LD-5/Round-4 amendment: **9-crate** Cargo workspace from day 1 (`orgsidian-parser`, `orgsidian-index`, `orgsidian-watcher`, `orgsidian-vault`, `orgsidian-plugin-api`, `orgsidian-report`, `orgsidian-core`, `orgsidian-cli`, `orgsidian-shell-app`); frontend at `shell-ui/` at repo root (no `packages/` indirection until a 2nd JS package exists). `tools/corpus-extractor/` outside the workspace.
- LD-6 Editor surface: CodeMirror 6 with Pseudo-WYSIWYG via decorators/widgets. Mandatory recipes: `WidgetType.eq()` shallow-equal, `Transaction.userEvent`, no `view.dispatch` inside `update()` while `view.composing`, `widget.ignoreEvent() === false` for interactive widgets.
- LD-7 Single Writer Rule + Dirty Buffer + three-pane Merge Dialog (Yours / External / Merged) as the concurrent-edit integrity contract.
- LD-8 Atomic writes via `atomic-write-file` crate + 3-retry exponential backoff wrapper for AV/Search-indexer transient locks.
- LD-9 File watcher via `notify-rs`; watcher abstraction layer in `core` allows deterministic fakes for unit tests; integration tests use golden traces from real external editors (vim, VS Code, Emacs save sequences). Network mounts and case-folding filesystems documented as v0.1 unsupported configurations.
- LD-10 Plugin API designed in v1.0 as a versioned internal contract; NOT published to crates.io until v1.5+. All v1.0 features consume the same trait surface that will eventually be exposed externally — no parallel "private" hooks.
- LD-52 i18n: Lingui v6.x with SWC plugin + Vite plugin + `eslint-plugin-lingui`; `lingui extract --clean && git diff --exit-code` is a CI gate to prevent catalog drift.
- LD-53 PDF rendering: `typst` embedded via `typst-as-lib` (`typst@0.14` / `typst-pdf@0.14` / `typst-as-lib@0.15`) in `orgsidian-report` crate. Bundled fonts: Inter (Variable), JetBrains Mono, Noto Sans Latin/Cyrillic subset (≤8 MB) for v0.5 Beta; Noto Sans CJK + Arabic subsets added in v1.0. `printpdf` 0.9.x retained as documented downgrade contingency.

**Data Architecture & Migrations:**

- LD-11 SQLite schema (normalized): tables `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `vault_meta`, `_schema_version`; FTS5 virtual tables `fts_headlines` + `fts_content` (external content, application-managed sync). Indices on `(file_path)`, `(headline_id)`, `(scheduled_date)`, `(deadline_date)`, `(tag, headline_id)`. Schema at `crates/orgsidian-index/sql/schema.sql`; typed query API in Rust.
- LD-12 Migrations via `rusqlite_migration` (1.3+); SQL files at `crates/orgsidian-index/migrations/NNNN_kebab-case-description.sql`. Forward-only (index is rebuildable).
- LD-13 Rebuild policy: incremental via watcher; full rebuild triggered by `PRAGMA user_version` mismatch, `PRAGMA integrity_check` failure, or explicit user command (Settings UI + `orgsidian index rebuild` per LD-49).
- LD-14 Connection management: single dedicated writer task (the indexer); reader pool via `deadpool-sqlite` (default size 4).
- LD-15 AST cache: in-memory LRU keyed by `(path, mtime)`, default 64 entries.
- LD-16 Async runtime: Tokio (Tauri default); `tokio::fs` for watcher + indexer; CPU-bound work via `tokio::task::spawn_blocking`.

**Security & Sandboxing:**

- LD-17 Tauri `fs` plugin allow-list: runtime-scoped to user-selected Vault folder + OS-standard config/data/log dirs.
- LD-18 Content Security Policy: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' file://*; connect-src 'self' https://updates.orgsidian.app; img-src 'self' data: file://*; font-src 'self' file://*;`.
- LD-19 Code signing: macOS Apple Developer ID + `notarytool`; Windows code-signing cert (EV evaluated at v1.0); Linux GPG-signed checksums + AppImage signature. Signing keys as GitHub Actions secrets.
- LD-20 Auto-update via `tauri-plugin-updater` with Tauri key pair; single `stable` channel in v1.0; disable-able in Settings.
- LD-21 Vault path constraints: symlinks followed by default (toggle in Settings); network mounts unsupported in v0.1 (polling fallback in v1.0); case-folding filesystems detected and handled.
- LD-22 User CSS loaded from `~/.orgsidian/themes/*.css`; data-exfiltration mitigated by CSP `connect-src 'self'` + `img-src` restricted to `file://*`.
- LD-23 Telemetry: none in v1.0 — no UI, no toggle, no instrumentation code, no backend. Reintroduced as clean opt-in in v1.5+ if and only if a backend exists.

**IPC, Plugin Loading, CLI:**

- LD-24 Tauri IPC with `tauri-specta` (2.x): `collect_commands![]` + `Builder::new().commands(...).export(...)` generates fully typed TypeScript client. Replaces hand-synced wrapper layer. Project-wide camelCase rename configured once in the specta builder.
- LD-25 Plugin loading model — v1.0: **static linking only**. Plugins are workspace crates registered in `Vec<Box<dyn OrgsidianPlugin>>` at compile time. No `cdylib`, no `libloading`, no FFI. v1.5+: WASM via `wasmtime`.
- LD-26 Plugin API trait: hook-with-priority + observer hybrid in `orgsidian-plugin-api` crate. Required methods: `metadata`, `init`, `shutdown`. Optional with defaults: `priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after`. `HookOutcome<T>::{Continue, Replace(T), Cancel(String)}`. `Event` enum `#[non_exhaustive]`; v1.0 variants: `FileOpened`, `FileSaved`, `FileChanged`, `HeadlineEdited`, `ClockStarted`, `ClockStopped`, `CaptureSubmitted`, `AgendaQueried`, `IndexRebuilt`. `HookContext` and `PluginContext` are traits in plugin-api (leaf invariant); `&dyn` references passed to plugins.
- LD-27 CLI command tree (`orgsidian-cli` via `clap` derive): `orgsidian parse <file>` / `orgsidian index {init|rebuild|stats|integrity}` / `orgsidian query {agenda <range>|search <query>|backlinks <headline-id>}` / `orgsidian validate-plugin <path>` / `orgsidian vault {info|init}`. `--json` flag for scripting. CLI is the primary integration-test surface.

**Frontend Architecture:**

- LD-28 Two Tauri windows in v1.0: `main` (editor + Today Dashboard + Agenda + Settings + Merge Dialog) and `quick-capture` (separate, lightweight bundle, single-input for FR-10 latency).
- LD-29 Routing: TanStack Router (latest stable), file-based routes at `shell-ui/src/routes/` as single source of truth for navigation; surfaces: `/today` (default), `/agenda/$view`, `/editor/$filePath/$headlineId?`, `/settings/$section`. Typed search params, typed loader data.
- LD-30 Virtualization: `@tanstack/react-virtual` for Agenda views (1k+ scheduled items), Search results, Backlinks panel. CM6 handles editor virtualization natively.
- LD-31 IPC frontend consumption: `import { commands, events } from '@/lib/tauri'`; never `invoke('command_name', …)` with raw strings.

**Infrastructure & Deployment:**

- LD-32 CI matrix: per-PR job on macOS-arm64 + Ubuntu-LTS (~<90s wall-clock) runs `cargo build/test/clippy -- -D warnings/fmt --check`, `pnpm typecheck/test`, round-trip subset gate (~100 files, <60s), perf snapshot regression gate (±10% median of 5 runs). Nightly full matrix adds Windows + Arch Linux + Ubuntu-LTS + full round-trip corpus (~2000 assertions from `test-org-element.el`) + perf trend dashboard + L2 oracle round-trip test via `emacs --batch`. Merge gate: PR green AND nightly green within last 24h.
- LD-33 Release automation: `cargo-release` (workspace-aware); Rust crates share app version with `v*` tag scheme during v0.1 → v1.4. `orgsidian-plugin-api` separates with its own `plugin-api-v*` cadence from v1.5+. CHANGELOG.md per crate + project root.
- LD-34 Distribution channels: macOS DMG + Homebrew cask in `orgsidian/tap`; Linux AppImage (primary) + Flathub manifest (best-effort); Windows MSI. Chocolatey/Scoop deferred post-v1.0.
- LD-35 Logging: `tracing` + `tracing-subscriber` structured logs; OS-standard log directory with 7-day / 50MB rotation; verbosity `info` default; `tauri-plugin-log` bridges frontend logs into the same files.
- LD-36 Crash reporting: not in v0.1; optionally added in v0.5+ as `sentry-rust` opt-in if a self-hosted Sentry backend is available; otherwise v1.5+.

**Post-Validation Hardening (LD-37..LD-51, LD-53):**

- LD-37 Dependency audit & supply-chain hygiene: `Cargo.lock` committed; `cargo audit` per-PR (fail on RUSTSEC ≥ medium); `cargo deny check licenses` allowlist; `cargo deny check bans` blocks duplicate major versions of `tokio`, `serde`, `chrono`, `rusqlite`; `cargo deny check graph` enforces LEAF crate rule. Quarterly review of advisory exceptions in `docs/security/advisory-exceptions.md`.
- LD-38 Plugin panic isolation: `[profile.release] panic = "unwind"`; all plugin invocation sites use `invoke_plugin_hook!` macro that wraps in `std::panic::catch_unwind`. On panic: log + mark plugin disabled-for-session + surface badge in Settings. Chaos test plugin `test-plugin-panic` (workspace crate) deterministically panics in every hook point; CI gate verifies host survives across matrix.
- LD-39 Multi-instance lockfile: `<Vault>/.orgsidian/instance.lock` JSON `{pid, hostname, started_at, locked_until}`; heartbeat every 30s; orphan threshold 5 min; dialog "Open read-only / Force unlock / Cancel" when another instance detected.
- LD-40 Vault-self-contained state: per-Vault state at `<Vault>/.orgsidian/` (keybinding remap, theme path, dismissed coaching, Plain/Power Mode, filter presets, last-open file, lockfile); global state at OS config dir (recent Vault paths, default UI language, default theme); SQLite index stays OUTSIDE Vault. `tauri-plugin-store` configured with two store roots.
- LD-41 Failure mode catalog: documented detection + recovery + test per failure mode (malformed `.org`, disk full, config corruption, Vault deletion, plugin panics, index corruption, `.tmp` orphans, external delete with Dirty Buffer).
- LD-42 Large-vault indexing UX: per-file progress `(N of M, X errors)`, cancellable, checkpoints every 100 files. Scaling targets: <5min for 10k, <20min for 50k (soft targets, nightly CI matrix on 10k/25k/50k synthetic vaults).
- LD-43 Memory soak regression gate (nightly): 12h scripted session, RSS drift <10% over 11 hours. PR-blocking via nightly merge gate.
- LD-44 Round-trip L0 subset corpus selection criteria documented in `docs/adr/0001-corpus-subset-selection.md`; subset regenerated by `tools/corpus-extractor/` on corpus changes. Syntax-feature coverage matrix + size buckets + edge-case bucket.
- LD-45 L2 Emacs ground-truth oracle pinned to two versions in nightly CI (`emacs:29.x` + `emacs:30.x`); canonical AST committed at `crates/orgsidian-parser/tests/canonical_ast/`; divergence triage workflow documented.
- LD-46 PRD reconciliation completed wave-1 and wave-2 (2026-05-19); PRD status `final` preserved.
- LD-47 Tauri ecosystem pinning policy: `Cargo.toml` exact-pin (`=2.X.Y`) for `tauri`, `tauri-build`, `tauri-plugin-*`, `tauri-specta`, `webkit2gtk-rs`. Quarterly Tauri-sync slot at each milestone (v0.2, v0.3, v0.4); v0.4 budgets 2-3 weeks for minor migration if breaking changes accumulated.
- LD-48 `tree-sitter-org` vendoring + maintenance contingency (see Stack & Foundation above).
- LD-49 `rebuild-index` as a first-class command (CLI + Shell UI Settings).
- LD-50 Plugin event surface review before v1.5+ external publication: `docs/plugin-api/v1.0-surface-review.md` committed at v0.5 milestone.
- LD-51 CSS token snapshot test: `shell-ui/src/themes/tokens.test.ts` extracts `--org-*` variables from `tokens.css` and compares against committed snapshot. Renames/removals/additions require explicit acceptance + CHANGELOG entry under "Theme API." Semantic granularity enforced (e.g., `--org-headline-h1-fg`, never `--org-color-blue-500`).

**Implementation Patterns & Consistency Rules** (binding for every PR):

- Naming conventions (Rust snake_case / TS camelCase / SQLite snake_case plural tables / Tauri commands snake_case Rust → camelCase TS auto via specta / Tauri events kebab-case / Plugin API Event variants PascalCase past-tense / CSS `--org-*` prefix universal).
- Test placement (Rust unit `#[cfg(test)] mod tests` co-located; integration `crates/<c>/tests/`; React component tests co-located; CLI integration via `assert_cmd` in `crates/orgsidian-cli/tests/`; E2E in `shell-ui/e2e/`).
- Error format: every `#[tauri::command]` returns `Result<T, OrgError>` (single enum in `orgsidian-core/src/error.rs` deriving `thiserror::Error` + `serde::Serialize` + `specta::Type`).
- Date/time: Rust `chrono::DateTime<Utc>` / `chrono::NaiveDate`; IPC wire ISO 8601 strings (specta default); TS-side native `Date` after parsing.
- IPC payload casing: project-wide specta `camelCase` rename; never per-struct `#[serde(rename_all)]`.
- Null handling: `Option<T>` → `T | null`; never `T | undefined` in IPC types.
- Zustand stores: one file per concern; Immer middleware; selectors exposed as hooks; CM6 state NEVER duplicated into Zustand.
- Logging: structured fields, never string interpolation. No `console.log` / `println!` in committed code.
- AI-Agent rules (mandatory): one concern per file (~400 lines); no `unwrap()` / `expect()` outside tests or `main()`; no silent error swallowing; no `any` / `unknown` in TS; tests with every PR; use generated `tauri-specta` client; use Zustand store hooks not `getState()`; Tailwind utilities first (extract to `org-*` after 3+ repetitions); update LD-NN if implementation forces a change; run `cargo test --workspace && pnpm test` before pushing.
- Anti-patterns forbidden: raw `invoke('command_name', …)`; per-struct `#[serde(rename_all = "camelCase")]`; React `forwardRef` (React 19 uses ref-as-prop); `useEffect` without idempotent cleanup; direct DOM in React components; string-typed event names; duplicating CM6 state into Zustand; conditional render via JSX for Plain/Power Mode; `console.log` / `println!`; premature abstraction.

**FR Traceability Discipline:**

- Every module implementing an FR carries a doc-comment header `//! Implements FR-NN ...`.
- `tests/traceability.rs` at workspace root parses the PRD's FR enumeration and fails if any FR has no `Implements FR-NN` doc-comment match.

**Docs & Discoverability:**

- Root: `README.md`, `CONTRIBUTING.md`, `ARCHITECTURE.md` (high-level + Mermaid dep graph + links), `CHANGELOG.md` (app-level Keep a Changelog), `SECURITY.md` (14-day patch SLA + GitHub Security Advisories + 90-day coordinated disclosure default).
- `docs/architecture.md` canonical; `docs/cli.md` (clap-derived); `docs/plugin-api/` (v1.5+ public); `docs/user-guide/` (v0.5 Beta onward); `docs/adr/`, `docs/architecture/`, `docs/parser/`, `docs/perf/`, `docs/plugin-api/`, `docs/security/`.
- `crates/README.md` one-line per crate (what it does, what it depends on, what it does NOT do).
- `crates/orgsidian-cli/build.rs` invokes `clap_mangen` to generate man pages.
- `examples/plugins/hello-world/` + `examples/plugins/agenda-exporter/` skeleton from day 1.

### UX Design Requirements

(**Updated 2026-05-20.** A UX Design specification now exists at `_bmad-output/planning-artifacts/ux-design-specification.md` (2026-05-20). It elaborates the *one object, three views* wedge (outline + agenda + graph), the Capture/Act/Review interaction trio, and the per-surface interaction patterns. PRD + architecture were reconciled against it on 2026-05-20 — see the PRD frontmatter revision entry and architecture LD-46 closed-loop addendum. This epics file is reconciled in turn (story additions for FR-25 Refile + FR-26 Graph View, Story 1.17 a11y CI gate, Story 1.18 TOML settings, Freelancer v0.1 promotion, Empty v0.5 demotion).)

### FR Coverage Map

| FR | Epic(s) | Notes |
|---|---|---|
| FR-1 (open/parse `.org`) | Epic 2 | Parser & Round-trip Fidelity |
| FR-2 (round-trip preservation) | Epic 2 | L0/L2 CI gates live here |
| FR-3 (Editor Modes switch) | Epic 4 | Raw / Pseudo-WYSIWYG / Split |
| FR-4 (Pseudo-WYSIWYG inline render) | Epic 4 | CM6 decorations + widgets |
| FR-5 (cross-platform keys + Emacs mode) | Epic 4 | `tauri-plugin-os` for Cmd vs Ctrl |
| FR-6 (Today Dashboard on launch) | Epic 7 | Full dashboard surface in v0.5 |
| FR-7 (Agenda Today/Week/Custom) | Epic 6 (Today/Week subset) + Epic 7 (Custom + saved presets, incl. Done-This-Week/Month default presets v0.5 per 2026-05-20) | Split per PRD §6.1 vs §6.2 |
| FR-8 (Clock in/out/resume) | Epic 7 (functional) + Epic 13 (UX polish) | Per PRD §6.2 phasing note |
| FR-9 (Schedule + Deadline editor) | Epic 4 | `OrgDatePicker` + parser semantic/timestamp |
| FR-10 (Global Quick Capture) | Epic 8 | Separate Tauri window `quick-capture` |
| FR-11 (System tray Capture) | Epic 8 | macOS menubar / Windows tray / Linux indicator |
| FR-12 (Full-text FTS5 search) | Epic 8 | `orgsidian-index::query::search` — two-tier streaming per 2026-05-20 (<100ms first 10 / <200ms full 50) |
| FR-13 (Backlinks panel) | Epic 8 (Linked v0.1) + Epic 12 (Unlinked References sub-panel v0.5+ per 2026-05-20) | `BacklinksPanel.tsx` + `query::backlinks` + `query::unlinked_references` |
| FR-14 (Project Report PDF/HTML) | Epic 10 | `orgsidian-report` new crate + LD-53 typst |
| FR-15 (Vault designation) | Epic 3 | First-launch picker + Settings |
| FR-16 (Watcher + Single Writer + Merge) | Epic 5 (fallback block-save) + Epic 9 (full three-pane Merge Dialog) | Split per PRD §6.2; `ConflictState`/`ConflictStrategy` rich-form day-1 (Party Mode P0) |
| FR-17 (SQLite derived index) | Epic 3 | Schema + migrations + rebuild policy |
| FR-18 (Starter Vault selection) | Epic 6 (Personal GTD + Student + Freelancer per 2026-05-20) + Epic 11 (Empty only per 2026-05-20 reshuffle) | Per PRD §6.1 vs §6.2 — Freelancer promoted to v0.1 Alpha for lighthouse-persona first-launch demonstration |
| FR-19 (Interactive Tutorial) | Epic 13 | v1.0 only |
| FR-20 (Plain/Power Mode) | Epic 11 | `data-[mode]` Tailwind selectors |
| FR-21 (Inline Coaching) | Epic 11 | `coachingRegistry.ts` centralized |
| FR-22 (Themes dark/light + CSS) | Epic 6 (dark+light defaults) + Epic 12 (CSS override + LD-51 tokens snapshot) | Split per PRD §6.1 vs §6.2 |
| FR-23 (Keybinding remapping) | Epic 12 | Conflict detection in Settings |
| FR-24 (Internal Plugin Pattern) | Epic 1 (plugin-api scaffold + trait stub) + woven across Epic 2-12 + Epic 8/9 consistency checkpoints + Epic 12 LD-50 surface review sign-off | Cross-cutting; every v1.0 feature consumes same trait surface |
| **FR-25** (Refile a Headline — added 2026-05-20) | Epic 11 (Stories 11.7/11.8/11.9 — primitives + LD-57 cross-file orchestrator + Target Picker UI) | v0.5 Beta; pairs with Quick Capture as inbox-triage primitive; org-canonical Cmd+Shift+R chord (Project Report rebinds to Cmd+Shift+E per Story 10.7 update) |
| **FR-26** (Backlink Graph View — added 2026-05-20) | Epic 8 (Stories 8.10/8.11/8.12 — `query::graph` API + `GraphCanvas`/`GraphNodeList` + cross-webview nightly perf gate) | v0.1 Alpha; third view in *one object, three views*; `react-force-graph-2d@1.29.1` per LD-56; perf ≤2s/5k nodes; a11y textual fallback per LD-58 |

**NFRs** are addressed continuously across all epics via CI gates and review checkpoints. Notable epic-specific NFR placements:

- NFR-19 (Round-trip CI gate) → Epic 2 (gate live from v0.1 Alpha onwards)
- NFR-20 (Perf snapshot regression gate ±10%) → Epic 1 (CI infrastructure) + enforcement from Epic 2
- NFR-21 (Memory soak nightly <10% RSS drift) → **Epic 4 (anticipated from Epic 6 per Party Mode P1 — Murat: CM6 decorations are likely leak source; cost is CI minutes)**
- NFR-15/16 (Atomic writes + Single Writer Rule) → Epic 3 (atomic-write subsystem) + Epic 5 (Dirty Buffer integrity) + Epic 9 (full Merge Dialog completes the invariant)
- NFR-8 (Cross-platform parity) → continuous in CI matrix; Windows feature parity in Epic 13

## Process Discipline — Cross-Cutting Authoring Rules

These rules are binding across all stories below. Existing stories were authored before some rules were finalized; they are brought into compliance via incremental updates as agents work on them. New stories ship compliant from authoring.

### A. Story-Level ATDD (Party Mode round 1 P0 — Murat)

Epic-level granularity is necessary but not sufficient for spec-driven AI-agent implementation. Story-level enforcement is required to avoid "implement then maybe write tests" collapse:

1. Every story authored via `bmad-create-story` skill with testable acceptance criteria.
2. Red-phase acceptance tests scaffolded via `bmad-testarch-atdd` skill **before** any production code is written.
3. Merge gate: PR cannot land unless (a) red-phase test exists and was committed first, AND (b) test transitions red → green via the production code in the same PR series.
4. Story sizing: target 5-10 stories per epic, ~7-15h each. Epics flagged for sharding during Step 3 if they exceed 12 stories.

### B. Persona Controlled Vocabulary (Party Mode round 2 — Paige)

Every `As a {persona}` must use one of these controlled tokens. Drift triggers a vocab-linter CI gate over `epics.md`.

```
END USERS (consume the product)
├── user                  ← default; covers 80% of stories
├── first-time user       ← onboarding-specific (FR-18, FR-19)
├── power user            ← keyboard-driven, advanced commands
├── screen-reader user    ← a11y-specific (NFR-9, Story 13.5)
├── freelance consultant  ← persona-of-record (UJ-1 / UJ-3)
└── early adopter         ← v0.1 Alpha audience (SM-1)

PROJECT ROLES (build the product)
└── author / contributor  ← scaffold, infra, audit, perf gates
```

### C. Traceability Discipline at Story Level (Party Mode round 2 — Paige)

Each story implementing an FR carries a **`Traces:`** line immediately below the `So that…` stanza, listing FRs and UJs covered.

Each AC list for an FR-implementing story includes a final AC of the form:
> *And the implementing module(s) carry `//! Implements FR-NN` as the first doc-comment line, verified by `tests/traceability.rs` (bidirectional: doc-comment ↔ enumerated FR in PRD).*

Stories below were authored before this rule was finalized; they are brought into compliance by agents during implementation as a no-brainer fixup (no separate epic).

### D. User-Voice in `So that` (Party Mode round 2 — Sally)

No `So that` clause may contain `FR-NN` references — that vocabulary belongs in the `Traces:` line. The `So that` clause is the human desire (the JTBD), not the delivery traceability matrix.

Stories below with PM-speak in `So that` (worst offenders: Story 9.1, Story 11.4, Story 13.3) are flagged for inline rewrite during implementation.

### E. Perf Assertions via Shared Infrastructure (Party Mode round 2 — Murat)

Absolute performance numbers (`<500ms`, `<1s`, `<200ms`) are product targets, not CI gate thresholds. Stories with perf AC consume the shared `assert_no_perf_regression!("story-id", baseline_path, || { … })` macro established in **Story 1.12**. The macro fails if the measured latency exceeds the baseline by >20% (median of 5 runs).

Stories below with absolute-number perf AC are interpreted as: "baseline must be at or below this number; subsequent runs may not regress >20%."

### F. AC Refactor Rule (Party Mode round 2 — Paige + Murat consensus)

Any AC block with **>4 `And` lines under a single `Then`** where multiple distinct invariants are tested is a candidate for split. Specifically, if an `And` introduces a new verb of action (clicks, saves, navigates), it is a new `When` masquerading — split into a separate AC group.

Story 4.3 (originally 8 `And` chains over 6 different decoration types) was the exemplar offender — split into Stories 4.3a..4.3g below. Other stories audited; flagged ones noted inline.

### G. Microcopy Discipline (Party Mode round 2 — Sally)

Every story containing user-facing text in AC marks that text with `[microcopy: draft]` if it has not undergone a UX-copy pass, or `[microcopy: final]` if production-ready. A single `docs/microcopy-registry.md` aggregates all `[draft]` strings and tracks copy-pass status.

Stories with currently-draft microcopy (worst offender: Story 7.7) are flagged inline.

### H. System-Level Testing Strategy (LD-54..LD-55 context)

The binding system-level test strategy lives at **`_bmad-output/test-artifacts/test-design.md`** (TEA workflow, 2026-05-19). Per-story red-phase scaffolds (rule A) instantiate the per-story-type scaffolds defined in §7.3 of that document. Coverage targets (§8) and quality gates (§10) of `test-design.md` are CI-enforced via the LD-32 matrix + Story 1.11 failure-mode harness + Story 1.12 perf snapshot infrastructure. Stories below do not duplicate test-design.md content; they reference it by section number where relevant.

## Epic List

### Epic 1: Foundation & CI Baseline
Scaffold the Tauri 2.x + Rust 9-crate workspace, React 19 + CM6 + Tailwind 4 + shadcn frontend at `shell-ui/`, plugin-api crate stub with `OrgsidianPlugin` trait (day-1 shape per LD-26), `tauri-specta` typed IPC bridge, Lingui v6 scaffold (LD-52), `[profile.release] panic = "unwind"` (LD-38), `cargo-deny` + `cargo audit` supply-chain gates (LD-37), SECURITY.md, root README/ARCHITECTURE/CHANGELOG, and CI matrix (per-PR macOS+Ubuntu + nightly Windows+Arch+Ubuntu with merge gate, LD-32). Ships with **3 anchor smoke tests** to prove CI is alive end-to-end (parse trivial `.org`, write+read 1-file round-trip, watcher detect 1 event) — anti-placebo-green discipline per Party Mode P2.
**FRs covered:** none directly (foundation only — enables FR-1..FR-24). Establishes scaffold for FR-24 Internal Plugin Pattern across all subsequent epics.

### Epic 2: Parser & Round-trip Fidelity
Implement `orgsidian-parser` crate with `tree-sitter-org` (nvim-orgmode fork, MIT, SHA-pinned submodule per LD-48) wrapper + custom Rust semantic layer + round-trip-faithful serializer. Light up the L0 byte-identical CI gate on a ~100-file subset (LD-44 selection criteria) per PR, the full ~2000-assertion nightly corpus (extracted via `tools/corpus-extractor/`), and the L2 Emacs ground-truth oracle pinned to `emacs:29.x` + `emacs:30.x` with hand-written canonical AST (LD-45). Establish **fixture governance** (`tests/fixtures/vault-corpus/` versioned via git-LFS, `fixtures.toml` declaring per-epic ownership) per Party Mode P1. Ship `orgsidian parse <file>` as the first public artifact CLI command (LD-27) — a tweet-ready early signal that the parser exists, before Epic 6's v0.1 Alpha public release.
**FRs covered:** FR-1, FR-2.
**Risk profile:** **highest single-epic technical risk in v0.1 Alpha** per all four Party Mode voices — Epic 2 is a go/no-go gate before Epic 4+ work begins.

### Epic 3: Vault & SQLite Index Foundation
Implement `orgsidian-vault` (atomic-write-file LD-8 with 3-retry exponential backoff for AV/Search-indexer transient locks, Dirty Buffer scaffold), `orgsidian-index` (normalized SQLite schema LD-11, `rusqlite_migration` forward-only LD-12, rebuild policy LD-13, connection management LD-14, locked PRAGMAs from LD-4), and the watcher abstraction (`orgsidian-watcher` with `notify-rs` LD-9 + debounce + golden-trace fixtures from vim/VS Code/Emacs save sequences per OD-3). User can designate a Vault folder; initial 1000-file scan completes in <30s with progress UI (LD-42 checkpoints every 100 files); deleting the SQLite index file and relaunching rebuilds identically.
**FRs covered:** FR-15, FR-17. Lays the foundation for FR-16 (Epic 5).

### Epic 4: Editor Surface & Org-mode Awareness
Wire CodeMirror 6 host in `shell-ui/src/components/editor/` with StrictMode-safe `EditorView` lifecycle and React-19 ref-as-prop pattern. Implement Editor Modes (Raw / Pseudo-WYSIWYG / Split) with persistent per-file preference; CM6 decorators/widgets for headings, TODO badges, tag pills, timestamp dates, checkbox widgets, clickable links (LD-6 mandatory recipes: `WidgetType.eq()`, `Transaction.userEvent`, no dispatch during `view.composing`, `widget.ignoreEvent() === false`). Default cross-platform keybindings + optional Emacs mode. Date picker + Schedule/Deadline editing on Headlines (with recurring timestamp `+1w` preservation). **Memory soak nightly gate (LD-43, <10% RSS drift over 11h) activated here** — CM6 decorations are the most likely leak source per Party Mode P1.
**FRs covered:** FR-3, FR-4, FR-5, FR-9.

### Epic 5: External-Edits Co-existence (Safe Fallback)
Connect the watcher to the Dirty Buffer to enforce the Single Writer Rule v0.1 contract: clean buffers auto-reload + re-index on external write; dirty buffers **block save with conflict warning** (no Merge Dialog UI yet — Epic 9). **Day-1 AC per Party Mode P0 (Winston + Murat consensus):** the watcher state machine is implemented as a `ConflictStrategy` pattern with `BlockWithWarning` as one of N strategies, and the `ConflictState` data model is a **rich struct** (`ancestor_hash`, `external_content`, `buffer_content`) not a boolean — even though v0.1 UI consumes only the strategy variant. This avoids the Epic 9 watcher-rewrite trap.
**FRs covered:** FR-16 (v0.1 fallback only — block-save with warning).

### Epic 6: v0.1 Alpha Release — First Launch & Day-One Agenda Snapshot
Implement Starter Vault picker on first launch with **Personal GTD + Student + Freelancer** starters (FR-18 partial — Empty deferred to Epic 11 per 2026-05-20 reconciliation), basic Today + Week Agenda views in `shell-ui/src/components/agenda/` (FR-7 partial — Custom view + saved presets in Epic 7), dark + light default themes with WCAG AA contrast in `shell-ui/src/themes/` (FR-22 partial — CSS customization in Epic 12), macOS DMG + Homebrew cask + Linux AppImage packaging (LD-19 signing, LD-34 distribution), README + landing page + basic docs. **Critical pre-Epic-7 AC per Party Mode P0 (Amelia):** the `IndexQuery` trait in `crates/orgsidian-index/src/query/mod.rs` is **frozen** as a stable API surface before Epic 6 closes — without this freeze, Epic 7 (agenda extensions) and Epic 8 (search/backlinks queries) will collide. Per the 2026-05-20 reconciliation, the v0.1 baseline freeze includes the streaming search contract (`search_stream`), `graph(scope)`, and `unlinked_mentions(headline_id)` — see Story 6.5. **Note on execution order:** Epic 6 closes v0.1 Alpha and therefore runs *after* Epics 7+8 in execution time despite its lower number; Story 6.1's Freelancer content depends on Story 8.7 Backlinks panel for its "≥1 backlink visible" AC. **Closes SM-1.**
**FRs covered:** FR-18 (Personal GTD + Student + Freelancer), FR-7 (Today/Week subset), FR-22 (dark + light defaults), FR-26 (Graph View — via Epic 8 stories 8.10/8.11/8.12 closed-by-release).

### Epic 7: Today Dashboard & Time Tracking
Implement full Today Dashboard surface in `shell-ui/src/components/today/` (FR-6 — Scheduled + Deadline + today-tag + Inbox preview + Active Clock; <500ms render on 1k-file Vault; collapsible sections with persistent preferences; empty-state messages). Add Custom Agenda view (date range picker) + saved filter presets to complete FR-7. Implement Clock in/out/resume in `orgsidian-core/src/clock.rs` + `stores/clockStore.ts` + `shell-ui/src/components/org/ClockEditor.tsx`: LOGBOOK `CLOCK:` persistence, single Active Clock invariant, prior-session running-clock prompt on launch (discard/adjust/keep). UX polish (persistent toggleable status bar, refined timer notifications, clock-time editing affordance) deferred to Epic 13 per PRD §6.2 phasing note.
**FRs covered:** FR-6, FR-7 (Custom + presets — full), FR-8 (functional).

### Epic 8: Capture, Search, Backlinks, Graph View
Implement Quick Capture as a separate Tauri window (`quick-capture.html` + separate Vite bundle for FR-10 <1s latency, LD-28) wired to `tauri-plugin-global-shortcut` (default `Cmd/Ctrl+Shift+Space`); system tray fallback (LD-28 + `orgsidian-shell-app/src/tray.rs`). FTS5 full-text search via `Cmd/Ctrl+P` Command Palette with query syntax `#tag:`, `file:`, `todo:` (`orgsidian-index::query::search`, **two-tier streaming per 2026-05-20: <100ms first 10 results, <200ms full 50 on 1k-file Vault**). Backlinks sidebar panel updating <100ms on Headline cursor move (`orgsidian-index::query::backlinks` + `BacklinksPanel.tsx`). **Backlink Graph View (FR-26 added 2026-05-20)** ships in this epic for v0.1 Alpha: `orgsidian-index::query::graph` adjacency API + `shell-ui/src/components/graph/{GraphCanvas, GraphNodeList}` via `react-force-graph-2d@1.29.1` (LD-56) + `/graph` TanStack route (LD-29) + ≤2s/5k-node cross-webview nightly perf gate + a11y textual fallback per LD-58 (Stories 8.10/8.11/8.12). **Author daily-driving SM-2 sub-criterion (task + clock + backlink in same session) becomes possible at the end of this epic per Party Mode (John).** **Plugin API consistency checkpoint:** verify Capture, Search, and Graph View consume the `OrgsidianPlugin` trait surface unchanged (no parallel "private" hooks); preview for LD-50 final review in Epic 12.
**FRs covered:** FR-10, FR-11, FR-12, FR-13 (Linked v0.1; Unlinked References sub-panel v0.5+ in Epic 12), FR-26.

### Epic 9: Conflict-Safe Concurrent Editing (Full Merge Dialog)
Build the three-pane Merge Dialog (`shell-ui/src/components/merge/` with custom focus management for 3-pane hunk navigation): Yours / External / Merged panes with diff hunks individually selectable (use-yours / use-external) + free-edit of Merged + atomic save on accept + Dirty Buffer preservation on cancel. Consumes the `ConflictState` rich struct + `ConflictStrategy` pattern frozen in Epic 5 — Epic 9 ships the `ThreePaneMergeDialog` strategy variant and **retires** `BlockWithWarning`. Watcher golden-trace fixtures from Epic 5 carry over ~85% unchanged; only the outcome assertion flips per Party Mode (Amelia + Murat consensus). **Sequenced AFTER Epic 8** per Party Mode P1 (Murat: watcher event bus cross-contamination between Capture and Merge requires write path stabilized first). **Plugin API consistency checkpoint** as in Epic 8.
**FRs covered:** FR-16 (full — replaces Epic 5 fallback).

### Epic 10: Project Report Export (Wow Demo)
Implement `crates/orgsidian-report/` new crate (isolated dep cost — `typst@0.14` + `typst-pdf@0.14` + `typst-as-lib@0.15` per LD-53, plus `orgsidian-report-default.typ` template and the `sys.inputs` schema generated from the `ReportData` struct). Bundled fonts (Inter Variable + JetBrains Mono + Noto Sans Latin/Cyrillic subset ≤8 MB for v0.5; CJK + Arabic added in v1.0). HTML path uses parallel `html_renderer.rs` (templater choice deferred to in-sprint micro-decision). User selects scope (file/subtree/tag) + date range → PDF or HTML report in <5s including TODO completions, Clock totals per Headline, linked-notes excerpts grouped by file, milestone status. Active Clock without end-time explicitly flagged. `docs/customization/report-templates.md` documents the `sys.inputs` schema (OQ-6 resolution). **SM-2 wow demo.**
**FRs covered:** FR-14.

### Epic 11: Onboarding Completion, Coaching & Refile
Add **Empty Starter Vault** to the picker (FR-18 completion — Freelancer moved to Epic 6 per 2026-05-20 reconciliation). Implement Plain Mode / Power Mode toggle in Settings via `data-[mode=plain]:hidden` Tailwind selectors (LD-29 — visibility flip, not conditional render; preserves keyboard-shortcut muscle memory). Centralized `coachingRegistry.ts` mapping coaching IDs to content + dismissal conditions; `<CoachingSlot id="..." />` as the only API used in surfaces; "Don't show again" persists per-context; "show all coaching tips" reset action in Settings. **FR-25 Refile (added 2026-05-20)** — the org-canonical inbox-triage primitive — lands here as Stories 11.7/11.8/11.9: subtree extract/insert primitives (`orgsidian-vault::refile`) + LD-57 sequence-with-`.bak` cross-file orchestrator + `RefileTargetPicker.tsx` UI bound to `Cmd/Ctrl+Shift+R` (Project Report rebinds to `Cmd/Ctrl+Shift+E` per Story 10.7 update).
**FRs covered:** FR-18 (Empty only), FR-20, FR-21, FR-25.

### Epic 12: v0.5 Beta Release — Customization, Unlinked References & Plugin Surface Lock
Implement the **Unlinked References** sub-panel on the existing Backlinks UI (FR-13 extension per 2026-05-20): `orgsidian-index::query::unlinked_references` (FTS5 title-match outer-joined against `links` table) + `BacklinksPanel.tsx` Linked/Unlinked sub-tabs. User CSS file override loaded from `~/.orgsidian/themes/*.css` after the bundle (FR-22 full — invalid CSS falls back to default with warning, never crashes). LD-51 `tokens.test.ts` Vitest snapshot test extracts the set of `--org-*` variables from `tokens.css` and locks the public theme API contract. Keybinding remapping in Settings with conflict detection (FR-23, per-Vault persistence via LD-40 `<Vault>/.orgsidian/settings.toml`). **LD-50 plugin event surface review sign-off** — audit every `Event` variant + hook method signature + `HookOutcome` semantics added during Epics 1-11; output `docs/plugin-api/v1.0-surface-review.md` committed before v0.5 → v1.0 transition. Final v0.5 Beta release artifacts + announcement. **Closes SM-2.**
**FRs covered:** FR-13 (Unlinked References extension), FR-22 (CSS customization), FR-23. Closes FR-24 v1.0 contract lock-in path.

### Epic 13: v1.0 — Cross-Platform Launch & Tutorial
Windows MSI packaging via Tauri bundler + code-signing cert (LD-19, EV upgrade evaluated) + WebView2 + ReadDirectoryChangesW reliability hardening (per OQ-3 / OQ-4 known edge cases). Auto-update via `tauri-plugin-updater` stable channel across macOS + Linux + Windows (LD-20). Interactive Tutorial — 10-minute guided cycle (capture → triage → schedule → agenda → clock in/out → one-line report) launchable from "Get started" menu + first-launch prompt; completion tracked locally (no telemetry); re-launchable from Settings (FR-19). Clock UX polish — persistent toggleable status bar, refined timer notifications, clock-time editing affordance (FR-8 polish per PRD §6.2 phasing). Performance budgets verified across full matrix (NFR-1..NFR-7). **A11y graduation:** expand the LD-58 happy-path keyboard scenarios (which ship as hard CI gate from v0.1 per Story 1.17) to representative-coverage per surface; add focus-ring visual snapshot tests; document known limitations + qualitative sign-off in `docs/user-guide/accessibility.md`. Full screen-reader certification (assistive-tech audit) remains deferred to v1.5+ per LD-58 follow-up. Comprehensive `docs/user-guide/` site. Coordinated announcement HN + ProductHunt + org-mode community channels. **Closes SM-3.**
**FRs covered:** FR-19, FR-8 (UX polish), NFR-8 (Windows feature parity added).

---

## Epic 1: Foundation & CI Baseline

Scaffold the entire monorepo, lock day-1 architectural decisions in code, and prove CI is alive end-to-end with anchor smoke tests. No user-facing FRs — establishes the harness for FR-1..FR-24.

### Story 1.1: Bootstrap Tauri 2.x + React 19 + TS scaffold

As the **author / contributor**,
I want a working `pnpm create tauri-app@2` scaffold with React + TypeScript + identifier `com.orgsidian.app`,
So that `pnpm tauri dev` launches a Tauri window on macOS-arm64 and Ubuntu-LTS, ready for incremental refactor.

**Acceptance Criteria:**

**Given** an empty project root,
**When** `pnpm create tauri-app@2` is run with project name `orgsidian`, identifier `com.orgsidian.app`, React + TS, pnpm,
**Then** the resulting scaffold builds via `pnpm tauri build` on macOS-arm64 and Ubuntu-LTS
**And** `pnpm tauri dev` opens a Tauri window with the default React scaffold content
**And** root `LICENSE` (MIT) and `README.md` exist with project name and one-paragraph description.

### Story 1.2: Refactor scaffold to 9-crate Cargo workspace + `shell-ui/` at root

As the **author / contributor**,
I want the scaffold reorganized into the 9-crate Cargo workspace (parser, index, watcher, vault, plugin-api, report, core, cli, shell-app) with `shell-ui/` at repo root,
So that every subsequent epic adds code into a stable, boundary-enforced module structure.

**Acceptance Criteria:**

**Given** Story 1.1 scaffold,
**When** the refactor is applied,
**Then** `Cargo.toml` declares `[workspace]` with 9 members at `crates/orgsidian-{parser,index,watcher,vault,plugin-api,report,core,cli,shell-app}/`
**And** the scaffolded `src-tauri/` content lives in `crates/orgsidian-shell-app/`
**And** the scaffolded `src/` content lives in `shell-ui/src/` with `pnpm-workspace.yaml` declaring it as the only JS workspace member
**And** `cargo build --workspace` passes
**And** `pnpm tauri dev` still launches the Tauri window
**And** `tools/corpus-extractor/` exists with its own `Cargo.toml` (`publish = false`) outside `[workspace.members]`.

### Story 1.3: Install Tauri plugin set + Tailwind 4 + shadcn/ui forked + TanStack Router

As the **author / contributor**,
I want the full Tauri plugin set + Tailwind 4 (CSS-first `@theme` config) + shadcn/ui essentials forked into `shell-ui/src/components/ui/` + TanStack Router file-based routing installed,
So that v0.1 features have their UI infrastructure ready without per-feature setup tax.

**Acceptance Criteria:**

**Given** Story 1.2 workspace,
**When** plugins, Tailwind 4, shadcn essentials, and TanStack Router are installed,
**Then** `crates/orgsidian-shell-app/tauri.conf.json` registers `tauri-plugin-{fs,dialog,global-shortcut,updater,window-state,store,shell,os,clipboard-manager,log,process}`
**And** `shell-ui/src/styles/app.css` declares the Tailwind 4 `@import` + `@theme` directive
**And** `shell-ui/src/components/ui/` contains the shadcn essentials (Button, Dialog, Input, Tabs, Tooltip, Toast) sourced via `npx shadcn@latest add` and committed
**And** `shell-ui/src/routes/__root.tsx` + `routes/index.tsx` (redirect to `/today`) + `routes/_layout/today.tsx` (placeholder) exist with TanStack Router setup wired in `main.tsx`
**And** `pnpm tauri dev` renders the `/today` placeholder route.

### Story 1.4: Wire `tauri-specta` typed IPC bridge with project-wide camelCase rename

As the **author / contributor**,
I want `tauri-specta` (v2.x) generating a fully-typed TypeScript client at `shell-ui/src/lib/tauri.ts` with project-wide `camelCase` rename configured once in the builder,
So that no story ever writes `invoke('command_name', …)` with raw strings or per-struct `#[serde(rename_all)]`.

**Acceptance Criteria:**

**Given** Story 1.2 workspace,
**When** `tauri-specta` is wired with a single placeholder command `ping() -> Result<String, OrgError>`,
**Then** `cargo build --workspace` regenerates `shell-ui/src/lib/tauri.ts` exposing `commands.ping()` returning `Promise<string>`
**And** `OrgError` exists in `crates/orgsidian-core/src/error.rs` deriving `thiserror::Error` + `serde::Serialize` + `specta::Type` with variants `Parse | Io | Index | Vault`
**And** `Builder::new().commands(collect_commands![ping]).config(specta::ts::ExportConfig::default().rename_all(specta::ts::RenameAll::CamelCase))` is the single source of casing.

### Story 1.5: Scaffold `orgsidian-plugin-api` leaf crate with day-1 trait surface

As the **author / contributor**,
I want `crates/orgsidian-plugin-api/` as a LEAF crate (no project deps) containing the `OrgsidianPlugin` trait + `Event` enum (`#[non_exhaustive]`) + `HookOutcome<T>` + `HookContext`/`PluginContext` traits,
So that FR-24 internal Plugin Pattern is woven from Epic 2 onwards without retrofit cost.

**Acceptance Criteria:**

**Given** Story 1.2 workspace,
**When** the trait surface is committed,
**Then** `crates/orgsidian-plugin-api/src/lib.rs` defines the `OrgsidianPlugin` trait with methods `metadata`, `init`, `shutdown`, `priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after` per architecture LD-26
**And** `Event` enum is `#[non_exhaustive]` with variants `FileOpened | FileSaved | FileChanged | HeadlineEdited | ClockStarted | ClockStopped | CaptureSubmitted | AgendaQueried | IndexRebuilt`
**And** `HookOutcome<T>::{Continue, Replace(T), Cancel(String)}` exists
**And** `HookContext` and `PluginContext` are traits (not concrete types) per LD-5 round-4 amendment
**And** `cargo deny check graph` confirms the crate has zero project dependencies
**And** `crates/orgsidian-plugin-api/CHANGELOG.md` exists with `0.0.0 - Initial trait surface` entry per LD-33.

### Story 1.6: Install Lingui v6.x i18n scaffold

As the **author / contributor**,
I want Lingui v6.x installed with SWC plugin + Vite plugin + `eslint-plugin-lingui` + `lingui extract --clean && git diff --exit-code` as a CI gate,
So that NFR-10 translation infrastructure ships in v1.0 without a v0.4 retrofit project.

**Acceptance Criteria:**

**Given** Story 1.3 frontend setup,
**When** Lingui is installed and configured per LD-52,
**Then** `shell-ui/package.json` lists `@lingui/{core,react,cli,vite-plugin,swc-plugin}` and `eslint-plugin-lingui` at `^6.0.1`
**And** `shell-ui/lingui.config.ts` declares `en` as the source locale and `packages/shell-ui/src/locales/{lng}/messages.po` (Gettext) as the catalog format compiled to TypeScript at build time
**And** `shell-ui/vite.config.ts` registers `@lingui/swc-plugin` via `react({ plugins: [["@lingui/swc-plugin", {}]] })` + the `@lingui/vite-plugin`
**And** `pnpm lingui extract` produces `messages.po` containing one `<Trans>` from a smoke string in the root component
**And** CI fails if `lingui extract --clean && git diff --exit-code` produces a diff.

### Story 1.7: Configure `cargo-deny` + `cargo audit` supply-chain hygiene

As the **author / contributor**,
I want `cargo-deny` (licenses allowlist + bans + graph) and `cargo audit` running on every PR with `RUSTSEC` severity ≥ medium failing,
So that LD-37 supply-chain hygiene is enforced before the first feature lands.

**Acceptance Criteria:**

**Given** Story 1.2 workspace,
**When** `cargo-deny` and `cargo audit` are configured,
**Then** `deny.toml` allowlists `MIT | Apache-2.0 | BSD-2-Clause | BSD-3-Clause | ISC | Unlicense | Zlib | MPL-2.0` and rejects `GPL-* | AGPL-* | proprietary | unknown`
**And** `deny.toml` bans duplicate major versions of `tokio`, `serde`, `chrono`, `rusqlite`
**And** `deny.toml` graph rule rejects consumer crates (`shell-app`, `cli`) importing leaf crates (`parser | index | watcher | vault | plugin-api | report`) directly
**And** `cargo audit` runs on per-PR CI and fails on advisory severity ≥ medium
**And** `Cargo.lock` is committed
**And** (added 2026-05-20) the allowlist is verified clean against the post-reconciliation dep additions: `toml` crate (MIT/Apache-2.0; LD-40 TOML settings), `react-force-graph-2d@1.29.1` + transitive `force-graph`/`react-kapsule`/`prop-types` (all MIT; LD-56 Graph View), `@axe-core/playwright` (MIT; LD-58 a11y CI gate). New JS-side dep additions are also subject to `pnpm audit` and the same severity gate; `pnpm` license discipline is enforced via `pnpm licenses` audit in CI.

### Story 1.8: Configure CI matrix + `[profile.release] panic = "unwind"` + `invoke_plugin_hook!` macro stub

As the **author / contributor**,
I want GitHub Actions running per-PR builds on macOS-arm64 + Ubuntu-LTS and nightly on Windows + Arch Linux + Ubuntu-LTS with a merge gate requiring nightly green within 24h,
So that LD-32 CI discipline is live and LD-38 plugin panic isolation is configured day-1.

**Acceptance Criteria:**

**Given** Story 1.7 workspace,
**When** CI workflows and panic policy are configured,
**Then** `.github/workflows/pr.yml` runs `cargo build/test/clippy -- -D warnings/fmt --check` + `pnpm typecheck/test` + `pnpm a11y` (the LD-58 a11y hard gate step established by Story 1.17) on macOS-arm64 + Ubuntu-LTS
**And** `.github/workflows/nightly.yml` runs the full matrix on macOS + Ubuntu + Arch + Windows, including the Story 8.12 cross-webview Graph View perf gate (≤2s/5k-node + ≤500ms steady-state-frame per LD-56)
**And** root `Cargo.toml` declares `[profile.release] panic = "unwind"` per LD-38
**And** `crates/orgsidian-core/src/registry.rs` declares the `invoke_plugin_hook!` macro stub wrapping calls in `std::panic::catch_unwind`
**And** the merge gate (per-PR green AND nightly green within 24h) is configured as a branch protection rule.

### Story 1.9: Add anchor smoke tests (anti-placebo-green per Party Mode P2)

As the **author / contributor**,
I want 3 anchor smoke tests proving the CI scaffold is wired end-to-end before any feature lands,
So that Epic 2 doesn't inherit a CI placebo where green means "compiled" rather than "exercises real code paths" (Murat P2).

**Acceptance Criteria:**

**Given** Stories 1.5 + 1.7 + 1.8,
**When** the anchor tests are committed,
**Then** `crates/orgsidian-parser/tests/anchor.rs` contains a passing test asserting a trivial `* TODO Hello\n` string parses without error (fixture in `crates/orgsidian-parser/tests/fixtures/`)
**And** `crates/orgsidian-vault/tests/anchor.rs` contains a passing test writing a 1-file `.org` content via atomic-write-file and reading it back byte-identical
**And** `crates/orgsidian-watcher/tests/anchor.rs` contains a passing test detecting one filesystem write event within 5 seconds using a deterministic-time fake clock
**And** all three anchor tests run on per-PR CI and fail on regression.

### Story 1.10: Add `SECURITY.md` + `ARCHITECTURE.md` + `CHANGELOG.md` + `CONTRIBUTING.md`

As the **author / contributor**,
I want root-level project hygiene docs in place,
So that contributors and security researchers have a navigable map from day-1 of the public repository.

**Acceptance Criteria:**

**Given** Story 1.1 scaffold,
**When** the docs are committed,
**Then** root `SECURITY.md` declares a 14-day patch SLA + GitHub Security Advisories reporting channel + 90-day coordinated disclosure default per LD-37
**And** root `ARCHITECTURE.md` contains a high-level summary + Mermaid crate dependency graph + link to `docs/architecture.md`
**And** root `CHANGELOG.md` is initialized in Keep-a-Changelog format with an `Unreleased` heading
**And** root `CONTRIBUTING.md` documents the development setup, fixture placement rule (co-located by default; promoted to root `fixtures/` only when ≥2 crates consume), the FR traceability discipline (`Implements FR-NN` doc-comment header), the Conventional Commits vocabulary + scope discipline + CHANGELOG mapping table per LD-54, and a "Testing strategy" section pointing to `_bmad-output/test-artifacts/test-design.md` as the authoritative system-level test strategy.

**Traces:** LD-37 (SECURITY.md), LD-54 (CONTRIBUTING.md CC section), Process Discipline rule H (testing strategy pointer).

### Story 1.11: Establish LD-41 failure-mode test harness (Party Mode round 2 P0 — Murat)

As the **author / contributor**,
I want a single cross-cutting `tests/failure_modes.rs` harness enumerating every LD-41 failure-mode category with concrete simulation hooks (fault-injection via `fail` crate where applicable), so that no failure mode ships uncovered into v0.1 Alpha.

**Acceptance Criteria:**

**Given** Epic 1 closed,
**When** the harness is committed,
**Then** `tests/failure_modes.rs` at the workspace root enumerates all 9 LD-41 categories (malformed `.org`, disk full / `ENOSPC`, config corruption, vault deletion runtime, plugin `init` panic, plugin `on_event` / hook panic, SQLite index corruption, `.tmp` orphan cleanup, external delete of file with Dirty Buffer)
**And** each category has a placeholder test annotated `#[ignore = "implemented in Epic N"]` referencing the epic responsible for the real implementation
**And** the harness imports the `fail` crate fault-injection helpers (`fail::cfg("atomic-write::after-tmp-rename", "panic")` etc.) used across epics
**And** a coverage assertion `tests/failure_modes_coverage.rs` fails CI if any LD-41 category has only `#[ignore]` placeholders beyond v0.5 Beta release tag
**And** `docs/failure-modes/coverage-matrix.md` records the category → epic → story mapping (auto-generated from the harness module).

**Traces:** LD-41 (Failure Mode Catalog), NFR-15, NFR-16.

### Story 1.12: Establish perf snapshot regression infrastructure (Party Mode round 2 P0 — Murat)

As the **author / contributor**,
I want a single shared `assert_no_perf_regression!` macro consumed by every perf-sensitive story, so that absolute-number perf AC do not create flaky CI on heterogeneous hardware and LD-32 ±10% regression discipline is uniform across the codebase.

**Acceptance Criteria:**

**Given** Story 1.8 (CI matrix),
**When** the perf infrastructure is committed,
**Then** `crates/orgsidian-core/src/test_support/perf.rs` exposes `assert_no_perf_regression!(story_id: &str, baseline_path: &str, op: impl Fn())`
**And** the macro runs `op` 5 times, computes the median, compares against the baseline stored at `tests/perf-baselines/{story_id}.json`, and fails if the median exceeds the baseline by >20%
**And** missing-baseline mode (first run) writes the baseline file and emits a non-fatal warning
**And** absolute perf targets from PRD §8 NFRs are documented in `docs/perf/targets.md` separately from the regression gate
**And** the macro is consumed by all perf-AC stories (Stories 4.3a-g, 6.3, 7.1, 8.1, 8.4 (split into `story-8.4-search-10results` <100ms + `story-8.4-search-50results` <200ms per 2026-05-20 two-tier), **8.11 Graph View `story-8.11-graph-5k-render` ≤2s + `story-8.11-graph-steady-frame` ≤500ms per LD-56**, 9.1, 10.6, **11.8 Refile orchestrator round-trip** — referenced inline below).

**Traces:** LD-32 (perf snapshot regression gate), NFR-1..NFR-7, NFR-20.

### Story 1.13: Bootstrap GitHub organization + private repo + label scheme + Project board

As the **author / contributor**,
I want the `orgsidian` GitHub organization created with a private `orgsidian/orgsidian` repo, a normalized label scheme, and a single Project v2 kanban board,
So that work tracking is in place before Epic 2 begins.

**Acceptance Criteria:**

**Given** an authenticated `gh` CLI with org-creation privileges,
**When** Story 1.13 is executed,
**Then** the `orgsidian` GitHub organization exists (created via `gh api orgs` or web UI; idempotent if pre-existing)
**And** `orgsidian/orgsidian` private repo exists with default branch `main` and the local Story 1.1 scaffold pushed
**And** `.github/labels.yml` declares the LD-55 label scheme (`epic:1..13`, `milestone:v0.1|v0.5|v1.0`, `status:backlog|in-progress|review|blocked|done`, `type:story|bug|spike|chore|docs|security`, `priority:p0|p1`)
**And** a labels-sync workflow (`actions/github-script` or `crazy-max/ghaction-github-labeler`) applies `.github/labels.yml` on push to `main`
**And** GitHub Project v2 `orgsidian/projects/1` exists with name "Orgsidian Roadmap" and 4 columns (Backlog / In Progress / Review / Done)
**And** the Project has two saved views: "By Milestone v0.1" (filter `milestone:v0.1`) and "By Epic" (group by `epic:N` label)
**And** `.github/ISSUE_TEMPLATE/story.md` exists with the LD-55 template fields (persona, user story, AC list, `Traces:` line, `Microcopy` flag, link to epics.md anchor).

**Traces:** LD-5 (repo location + visibility), LD-55 (label scheme + Project board).

### Story 1.14: Configure commitlint + husky commit-msg hook + CI gate

As the **author / contributor**,
I want commitlint enforcing Conventional Commits v1.0.0 locally (husky `commit-msg`) and on CI (per-PR job + PR-title check),
So that every commit and PR title qualifies for `git-cliff` CHANGELOG ingestion per LD-54.

**Acceptance Criteria:**

**Given** Story 1.3 frontend setup (husky already on disk per pre-commit hook),
**When** Story 1.14 is executed,
**Then** `package.json` lists `@commitlint/cli` and `@commitlint/config-conventional` at latest stable
**And** `commitlint.config.cjs` at repo root declares `module.exports = { extends: ['@commitlint/config-conventional'] }` with no scope-value enum (encouraged not required)
**And** `.husky/commit-msg` runs `pnpm commitlint --edit "$1"` and fails the commit on a non-conforming message
**And** `.github/workflows/pr.yml` (or a dedicated `commitlint.yml`) adds a step that runs `pnpm commitlint --from origin/main --to HEAD` and fails the PR on any non-conforming commit
**And** an additional CI step using `amannn/action-semantic-pull-request@v5` validates the PR title itself against Conventional Commits
**And** a smoke test confirms that a deliberately-malformed local commit (`git commit -m "broken message"`) is rejected by the `commit-msg` hook
**And** a smoke test confirms that a deliberately-malformed PR title triggers the CI title check failure.

**Traces:** LD-54 (enforcement chain).

### Story 1.15: Configure `git-cliff` for CC → CHANGELOG generation

As the **author / contributor**,
I want `git-cliff` invoked by `cargo release` to regenerate `CHANGELOG.md` from Conventional Commits per the LD-54 mapping table,
So that every release ships an accurate, automation-generated user-facing changelog without manual curation of `feat`/`fix`/`perf` entries.

**Acceptance Criteria:**

**Given** Story 1.14 (commitlint live) and LD-33 release automation context,
**When** Story 1.15 is executed,
**Then** `git-cliff` is installed as a `cargo install` step in the release pipeline (or pinned in `Cargo.toml` `[workspace.metadata]` for reproducibility)
**And** `cliff.toml` at repo root encodes the LD-54 mapping table as `[git.commit_parsers]` (per-CC-type group assignment) + `[changelog.body]` template producing Keep-a-Changelog format with `Added`/`Changed`/`Deprecated`/`Removed`/`Fixed`/`Security` headings
**And** `cargo release` `[hooks.pre-release]` invokes `git-cliff --unreleased --tag <version> --prepend CHANGELOG.md`
**And** a second `git-cliff` invocation scoped to `crates/orgsidian-plugin-api/**` paths regenerates `crates/orgsidian-plugin-api/CHANGELOG.md` (LD-33 separate-changelog discipline)
**And** a smoke test runs `git-cliff --unreleased` against a fixture branch with one `feat:`, one `fix:`, one `perf:`, one `feat!:`, one `chore:` commit and asserts: the `chore:` is excluded, the `feat:` lands under Added, the `fix:` under Fixed, the `perf:` under Changed, and the `feat!:` lands under Changed with a `⚠ BREAKING:` prefix in its entry text
**And** the `Deprecated` and `Security` headings remain present-but-empty when no manual entries exist (template allows empty sections).

**Traces:** LD-33 (release automation), LD-54 (CHANGELOG mapping).

### Story 1.16: GitHub Issues sync — one issue per story

As the **author / contributor**,
I want a one-way sync from `_bmad-output/planning-artifacts/epics.md` to GitHub Issues in `orgsidian/orgsidian` (one issue per Story N.M, idempotent re-runs),
So that the Project board (Story 1.13) and Issue search become navigable surfaces over the 104-story roadmap without manual re-typing.

**Acceptance Criteria:**

**Given** Stories 1.13 (org/repo/labels/Project exist) and 1.10 (CONTRIBUTING.md docs the sync),
**When** Story 1.16 is executed,
**Then** `tools/issues-sync/` exists as a Rust binary (Cargo.toml with `publish = false`, outside `[workspace.members]` per LD-5 convention for `tools/corpus-extractor/`)
**And** the binary parses `epics.md` and extracts each `### Story N.M: <title>` block including persona, user-story, AC list, `Traces:` line, and any flags (`[Microcopy: draft|final]`)
**And** the binary uses `octocrab` (or `gh api` via `std::process::Command` wrapper) to ensure-exists each Issue with title `[Story N.M] <title>`, body rendered per `.github/ISSUE_TEMPLATE/story.md`, labels (`epic:N`, `milestone:v0.X` derived from §Epic List milestone-to-epic mapping, `status:backlog` if new, `type:story`)
**And** the binary places each newly-created Issue into the GitHub Project v2 Backlog column (using Projects v2 GraphQL `addProjectV2ItemById`)
**And** re-running the binary on the same `epics.md` is idempotent — no duplicate issues created, no label thrash, no Project board re-shuffle
**And** `.github/workflows/sync-issues.yml` runs the binary on push-to-main when `_bmad-output/planning-artifacts/epics.md` changes (path filter), with `GITHUB_TOKEN` scoped to issues+projects write
**And** a smoke test against a 2-story fixture `epics-fixture.md` creates 2 issues with correct labels and project placement; a second smoke run with the same fixture creates 0 new issues
**And** a deliberate `status:` label drift (e.g., manually changing an issue to `status:in-progress`) is NOT reset by the sync — manual is authoritative once an issue is open
**And** the workflow is documented in CONTRIBUTING.md alongside the LD-55 reference.

**Traces:** LD-55 (Issues sync + Project board).

### Story 1.17: Establish WCAG 2.1 AA hard CI gate (added 2026-05-20 per LD-58)

As the **author / contributor**,
I want three hard CI gates enforcing WCAG 2.1 AA — contrast-matrix, axe-core, and 6 happy-path keyboard scenarios — wired into the per-PR pipeline from day 1,
So that every UI-shipping story downstream inherits the a11y floor by construction rather than retroactively (NFR-9 hard gate from v0.1 Alpha).

**Traces:** NFR-9, LD-58, LD-32, LD-51.

**Acceptance Criteria:**

**Given** Story 1.8 (per-PR CI pipeline) and Story 1.3 (Tailwind 4 + theme tokens scaffold),
**When** the LD-58 a11y gates are wired,
**Then** `packages/shell-ui/package.json` pins `@axe-core/playwright` (latest stable; MIT) added by Story 1.7 license-allowlist verification
**And** `packages/shell-ui/src/themes/contrast.test.ts` Vitest contrast-matrix test extracts `--org-*-fg` / `--org-*-bg` pairs from `tokens.css` (the LD-51 canonical source), computes WCAG relative-luminance contrast ratio `(L1 + 0.05) / (L2 + 0.05)` per pair, and asserts ≥4.5:1 for body-text pairs and ≥3:1 for large-text / UI-chrome pairs; tokens without declared pair role in `tokens.css` metadata fail the gate
**And** `packages/shell-ui/e2e/a11y/` contains 6 happy-path keyboard-only Playwright scenarios — Today Dashboard, Agenda, Editor, Quick Capture, Settings, Graph View — each tagged `@a11y` and starting `page.keyboard`-only (no `mouse.click`), navigating to the surface and asserting a persisted side-effect of a representative action
**And** each `@a11y` scenario invokes `await new AxeBuilder({ page }).withTags(['wcag2a','wcag2aa','wcag21a','wcag21aa']).analyze()` and fails on violations at `serious` or `critical` impact (best-practice tier excluded to avoid noise that erodes the gate)
**And** `pnpm a11y` orchestrates `pnpm test:contrast` (Vitest) + `pnpm test:e2e -- --grep @a11y` (Playwright) and the script is wired into `.github/workflows/pr.yml` per Story 1.8 AC
**And** the per-PR runtime budget for the 6 `@a11y` scenarios is ≤2-3 min combined on macOS-arm64 + Ubuntu-LTS
**And** exhaustive per-surface coverage is **explicitly deferred to v1.0 graduation** (Story 13.5 narrowed) — Story 1.17 ships the v0.1 hard floor, not the v1.0 ceiling
**And** the implementing modules carry `//! Implements NFR-9 a11y CI gate (LD-58)` as the first doc-comment line, verified by `tests/traceability.rs`.

### Story 1.18: TOML settings authoritative store with hybrid boundary (added 2026-05-20 per LD-40 amendment / OQ-7 resolution)

As the **author / contributor**,
I want a TOML-based authoritative settings store at `<Vault>/.orgsidian/settings.toml` + `<config-dir>/global.toml`, with `tauri-plugin-store` retained only for ephemeral UI state,
So that every downstream Settings-touching story consumes a stable, human-editable, file-authoritative source-of-truth from day 1 — and the dual-surface OQ-7 commitment (PRD §10) is enforced by construction.

**Traces:** LD-40, PRD §10 OQ-7, FR-23.

**Acceptance Criteria:**

**Given** Story 1.2 workspace + Story 1.7 license allowlist verified clean against the `toml` crate (MIT/Apache-2.0),
**When** the settings store is implemented,
**Then** `crates/orgsidian-core/src/settings/` exposes `read_vault_settings(vault_path) -> Result<VaultSettings>` + `write_vault_settings(vault_path, &VaultSettings) -> Result<()>` + analogous `read/write_global_settings`, using the `toml` crate for serialization
**And** `VaultSettings` owns: keybindings remap, theme path, capture hotkey, named agenda filter presets, dismissed coaching IDs, Plain/Power mode preference, Today Dashboard section preferences
**And** `GlobalSettings` owns: list of recent Vault paths, default UI language, default theme for new Vaults
**And** `tauri-plugin-store` is **retained only** for ephemeral UI state: last-open file, window geometry, tutorial progress, last-Vault path — never authoritative settings (explicit boundary list in `docs/architecture/settings-boundary.md`)
**And** writes are atomic via the `orgsidian-vault::atomic-write` infrastructure (Story 3.1 dependency — Story 1.18 may ship the lib API in Epic 1 with the atomic-write wired-up gated until Epic 3 closes)
**And** round-trip fidelity: read TOML → serialize back → byte-identical when no field changed
**And** schema versioning: `[meta] schema_version = 1` mandatory in every TOML file; forward-only migration discipline mirrors LD-12 (no destructive rewrites)
**And** the settings file is just another file under the watcher — external edits while the app is open trigger a reload-or-merge per the LD-7 Single Writer Rule pattern (Story 5.4 wires this; Story 1.18 leaves the hook in place but the watcher integration lands with Epic 5)
**And** the implementing modules carry `//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface)` as the first doc-comment line.

---

## Epic 2: Parser & Round-trip Fidelity

Implement the parser + round-trip serializer, light up L0/L2 CI gates, establish fixture governance, and ship `orgsidian parse <file>` as a public artifact precoce.

### Story 2.1: Vendor `tree-sitter-org` as SHA-pinned git submodule

As the **author / contributor**,
I want `nvim-orgmode/tree-sitter-org` vendored at `crates/orgsidian-parser/grammar/` as a SHA-pinned submodule with no auto-bumping,
So that LD-48 maintenance contingency is in place from day-1 with the parser-owner role formalized.

**Acceptance Criteria:**

**Given** Epic 1 closed,
**When** the submodule is added,
**Then** `crates/orgsidian-parser/grammar/` is a git submodule pointing to a specific SHA of `nvim-orgmode/tree-sitter-org`
**And** `.gitmodules` documents the pin
**And** `CONTRIBUTING.md` names the parser-owner role and the SHA review process for upgrades
**And** `cargo build -p orgsidian-parser` compiles the vendored grammar via `tree-sitter-cli` build hooks.

### Story 2.2: Implement `orgsidian-parser` grammar wrapper

As the **author / contributor**,
I want the tree-sitter-org wrapper in `crates/orgsidian-parser/src/grammar.rs` parsing any `.org` file into a tree-sitter syntax tree,
So that the semantic layer can be built on top in Story 2.3.

**Acceptance Criteria:**

**Given** Story 2.1 grammar vendored,
**When** `pub fn parse(source: &str) -> tree_sitter::Tree` is exposed,
**Then** parsing a representative file (with headlines, TODO states, drawers, links) returns a syntax tree without error
**And** `tests/grammar.rs` asserts the tree has the expected root node type for a 10-line org sample
**And** `cargo doc -p orgsidian-parser` produces `//! Implements FR-1` doc-comment for the module per FR Traceability Discipline.

### Story 2.3: Implement semantic layer (TODO cycling, drawer types, timestamps, link types)

As the **user editing my `.org` vault**,
I want the parser to expose semantic types (TODO state, drawer kind, scheduled/deadline timestamps, link type) beyond raw syntax nodes,
So that FR-1 renders correctly and Epic 4+ can build TODO badges, timestamp pickers, and clickable links against a stable AST.

**Acceptance Criteria:**

**Given** Story 2.2,
**When** the semantic layer in `crates/orgsidian-parser/src/semantic/` is implemented,
**Then** the public API exposes `Headline { todo_state: Option<TodoState>, tags: Vec<Tag>, scheduled: Option<Timestamp>, deadline: Option<Timestamp>, properties: HashMap<String, String>, ... }`
**And** TODO state cycling (TODO → NEXT → DONE → WAITING → TODO) is parameterized by an in-file `#+TODO:` directive
**And** drawer types (PROPERTIES, LOGBOOK, custom) are distinguished
**And** link types (`id:`, `[[wiki]]`, `[[file://]]`, `[[http://]]`) are parsed into variants
**And** unit tests in `tests/semantic.rs` cover each of the following LD-44 syntax constructs (enumerated explicitly per Party Mode round 2 P0 — Murat):
  - Heading levels 1-6 with TODO states (`TODO`, `NEXT`, `DONE`, `WAITING`, custom via `#+TODO:`)
  - Scheduled timestamp (`SCHEDULED:`); active + inactive
  - Deadline timestamp (`DEADLINE:`); active + inactive
  - Clock entries (`CLOCK: [start]--[end] => HH:MM`); open + closed + ranged
  - Recurring timestamps (`<2026-05-19 Mon +1w>`, `+1d`, `+1m`, `+1y`)
  - Drawer `:PROPERTIES:`; `:LOGBOOK:`; custom drawer types
  - Inline markup: `*bold*`, `/italic/`, `=verbatim=`, `~code~`, `+strike+`, `_underline_`
  - Links: `[[id:abc]]`, `[[wiki-link]]`, `[[wiki-link][description]]`, `[[file://path]]`, plain `http://...`
  - Lists: `-`, `+`, numbered `1.`, checkbox `- [ ]` / `- [X]`
  - Tables: simple + with separator row + with formula line `#+TBLFM:`
  - Block elements: `#+BEGIN_SRC`, `#+BEGIN_QUOTE`, `#+BEGIN_EXAMPLE`, `#+BEGIN_VERSE`
  - Inline LaTeX: `$...$`, `\\(...\\)`, `\\[...\\]`
  - Footnotes: `[fn:N]`, `[fn::inline]`
  - Citations: org-cite syntax `[cite:@key]`
  - Tags: `:tag:`, `:tag1:tag2:` (multi)
**And** each enumerated construct has ≥1 unit test in `tests/semantic.rs` named `semantic_{construct_kebab}`
**And** `KNOWN_DIVERGENCES.md` at `docs/parser/` is initialized with the known tree-sitter-org coverage gaps.

**Traces:** FR-1, LD-44 (subset corpus syntax-feature matrix).

### Story 2.4: Implement round-trip-faithful serializer

As the **user editing my `.org` vault**,
I want files saved by Orgsidian without user-visible edits to be byte-identical to their on-disk version (FR-2),
So that the trust contract with the org community is honored.

**Acceptance Criteria:**

**Given** Story 2.3,
**When** `pub fn serialize(headlines: &[Headline]) -> String` is implemented in `crates/orgsidian-parser/src/serializer.rs`,
**Then** opening any file from the LD-44 subset corpus, parsing it, and serializing the AST produces output byte-identical to the input (modulo trailing-newline normalization, documented)
**And** `tests/round_trip.rs` runs the round-trip property on the full subset corpus
**And** `proptest` strategies generate randomized headlines + serialize + parse + serialize → asserts the second serialization is byte-identical to the first
**And** the module carries `//! Implements FR-2`.

### Story 2.5: Build `tools/corpus-extractor` + fixture governance

As the **author / contributor**,
I want `tools/corpus-extractor/` extracting the L0 subset (~100 files) and the full nightly corpus (~2000 assertions) from `org-mode/testing/lisp/test-org-element.el`,
So that LD-44 subset criteria are enforceable and Murat's P1 fixture-governance discipline is in place.

**Acceptance Criteria:**

**Given** Story 2.4,
**When** the extractor is built,
**Then** `tools/corpus-extractor/` is a standalone Cargo project (`publish = false`, outside `[workspace.members]`) that emits `fixtures/subset-pr.json` and `fixtures/full-nightly.json`
**And** the subset satisfies LD-44 syntax-feature-coverage matrix (every construct appears ≥3 times; 30 small + 50 medium + 20 large; edge-case bucket with Unicode/RTL/CRLF/malformed-valid syntax)
**And** `fixtures/fixtures.toml` declares per-epic ownership of every fixture file (e.g., `subset-pr.json owner = "epic-2"`) per Murat's P1
**And** `tests/fixtures/vault-corpus/` is versioned via git-LFS with mutation requiring PR review
**And** `docs/adr/0001-corpus-subset-selection.md` documents the algorithm.

### Story 2.6: Light up L0 round-trip CI gate (per-PR ~100 files <60s)

As the **author / contributor**,
I want the L0 byte-identical round-trip gate running on every PR against the ~100-file subset in <60s wall-clock,
So that FR-2 is enforced as a hard CI contract from v0.1 Alpha onwards per LD-32.

**Acceptance Criteria:**

**Given** Stories 2.4 + 2.5,
**When** the CI gate is wired,
**Then** `.github/workflows/pr.yml` runs `cargo test -p orgsidian-parser round_trip_subset -- --test-threads=4` consuming `fixtures/subset-pr.json`
**And** failure on any subset file fails the PR
**And** total round-trip subset gate runtime is <60s on the GitHub Actions runner
**And** the gate runs on macOS-arm64 + Ubuntu-LTS per PR.

### Story 2.7: Light up nightly full-corpus + L2 Emacs oracle gates

As the **author / contributor**,
I want the full ~2000-assertion corpus + L2 Emacs ground-truth oracle running nightly on pinned `emacs:29.x` + `emacs:30.x`,
So that LD-45 divergence triage workflow is operational and parser drift from Emacs is caught within 24h.

**Acceptance Criteria:**

**Given** Story 2.6,
**When** the nightly gate is wired,
**Then** `.github/workflows/nightly.yml` runs `cargo test -p orgsidian-parser round_trip_full` consuming `fixtures/full-nightly.json` on macOS + Ubuntu + Arch + Windows
**And** the L2 step invokes `emacs:29.x --batch --eval` + `emacs:30.x --batch --eval` against each file in the L2 subset and asserts the produced AST matches `crates/orgsidian-parser/tests/canonical_ast/{file}.json` (committed, peer-reviewed)
**And** divergence triage follows LD-45: both Emacs concordant against Orgsidian → PR-blocking; both Emacs discordant from each other → log in `KNOWN_DIVERGENCES.md`; mixed → human review.

### Story 2.8: Ship `orgsidian parse <file>` CLI command as early public artifact

As an **early adopter curious about Orgsidian's parser**,
I want `orgsidian parse <file> [--json]` printing the parsed AST,
So that I can test Orgsidian's fidelity on my own `.org` files before any GUI ships (light-touch John P1 — early public signal at month ~4 instead of month 6).

**Acceptance Criteria:**

**Given** Stories 2.3 + 2.4,
**When** the CLI command is added to `crates/orgsidian-cli/` via `clap` derive,
**Then** `orgsidian parse <file>` prints a human-readable AST tree to stdout
**And** `orgsidian parse <file> --json` prints the AST as JSON via `serde_json`
**And** `crates/orgsidian-cli/tests/parse_cmd.rs` runs the command via `assert_cmd` against a fixture file
**And** `clap_mangen` generates `crates/orgsidian-cli/man/orgsidian-parse.1` at build time per LD-27.

---

## Epic 3: Vault & SQLite Index Foundation

### Story 3.1: Implement atomic-write subsystem with AV-retry wrapper

As the **user saving my `.org` file**,
I want every save to use temp-file-and-rename atomic semantics with a 3-retry exponential backoff for AV/Search-indexer transient locks,
So that power loss or AV interference never corrupts the source file (NFR-15 + LD-8).

**Acceptance Criteria:**

**Given** Epic 2 closed,
**When** `crates/orgsidian-vault/src/atomic.rs` wraps the `atomic-write-file` crate,
**Then** `pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), VaultError>` writes via temp-file-and-rename
**And** on `IoError::PermissionDenied | IoError::Other` matching AV/Search-indexer patterns, the wrapper retries with exponential backoff (base 100ms, max 3 attempts)
**And** `tests/atomic.rs` injects faults via a custom `FileSystem` trait fake and asserts retry behavior
**And** orphan `*.tmp.<pid>` files from dead PIDs are cleaned up on Vault open per LD-41.

### Story 3.2: Scaffold Dirty Buffer manager

As the **user editing my `.org` file**,
I want Orgsidian to track which open files have unsaved buffer changes (Dirty Buffer state) keyed by file path,
So that Epic 5 can enforce the Single Writer Rule and Epic 9 can route external writes to the Merge Dialog.

**Acceptance Criteria:**

**Given** Story 3.1,
**When** `crates/orgsidian-vault/src/dirty_buffer.rs` is implemented,
**Then** `pub struct DirtyBufferManager` exposes `mark_dirty(path, content)`, `mark_clean(path)`, `is_dirty(path) -> bool`, `get_buffer(path) -> Option<&str>`
**And** the manager is thread-safe (`Arc<Mutex<…>>` or `Arc<RwLock<…>>` per implementation choice)
**And** unit tests cover the lifecycle: clean → dirty → save → clean.

### Story 3.3: Define SQLite schema + locked PRAGMAs

As the **user opening my Vault**,
I want a normalized SQLite schema covering files, headlines, tags, properties, clock_entries, links, vault_meta, schema_version with FTS5 virtual tables for fts_headlines and fts_content,
So that Epic 7/8 queries (agenda + search + backlinks) have a performant index from day-1 (LD-4, LD-11).

**Acceptance Criteria:**

**Given** Epic 1 closed,
**When** `crates/orgsidian-index/sql/schema.sql` is committed,
**Then** the schema declares tables `files | headlines | tags | properties | clock_entries | links | vault_meta | _schema_version`
**And** FTS5 virtual tables `fts_headlines` + `fts_content` use external-content references (no triggers; application-managed sync)
**And** indices exist on `(file_path) | (headline_id) | (scheduled_date) | (deadline_date) | (tag, headline_id)` per LD-11
**And** the connection initialization in `crates/orgsidian-index/src/connection.rs` runs the LD-4 locked PRAGMAs (`journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`)
**And** the FTS5 tokenizer is configured as `unicode61 remove_diacritics 2` + `porter`.

### Story 3.4: Wire `rusqlite_migration` forward-only migrations

As the **author / contributor**,
I want migrations managed by `rusqlite_migration` (≥1.3) from SQL files at `crates/orgsidian-index/migrations/NNNN_kebab-case-description.sql`,
So that LD-12 forward-only migration discipline is enforced and schema drift between dev/prod is detectable via `PRAGMA user_version` per LD-13.

**Acceptance Criteria:**

**Given** Story 3.3,
**When** the migration runner is wired,
**Then** `Migrations::new(vec![M::up(include_str!("../migrations/0001_initial-schema.sql"))]).to_latest(&mut conn)` runs on connection initialization
**And** `crates/orgsidian-index/migrations/0001_initial-schema.sql` contains the Story 3.3 schema
**And** `PRAGMA user_version` is bumped per migration
**And** `tests/migrations.rs` asserts a fresh DB reaches schema version 1 after `to_latest`.

### Story 3.5: Add `deadpool-sqlite` reader pool + dedicated writer task

As the **user querying the Agenda or Search**,
I want a 4-reader pool + single-writer-task connection model so that concurrent reads never block each other,
So that NFR-3 (agenda <100ms) and NFR-4 (search <200ms) budgets are achievable (LD-14).

**Acceptance Criteria:**

**Given** Story 3.4,
**When** the pool is wired,
**Then** `crates/orgsidian-index/src/pool.rs` exposes a `IndexPool` wrapping `deadpool-sqlite::Pool` with default size 4 readers
**And** the single dedicated writer is a Tokio task receiving `IndexUpdate` messages via an `mpsc` channel
**And** `tests/concurrency.rs` asserts 16 concurrent reads complete without deadlock or pool exhaustion.

### Story 3.6: Vault designation UI + initial scan progress

As the **first-time user**,
I want to designate a folder as my Vault via a file picker on first launch (or via Settings later), with a non-modal progress UI showing `(N of M files indexed, X errors)` during the initial scan,
So that FR-15 + LD-42 large-vault indexing UX is honored.

**Acceptance Criteria:**

**Given** Stories 3.3 + 3.4 + 3.5,
**When** the user designates a folder via the picker,
**Then** `commands.designateVault(path)` recursively indexes all `.org` files in the folder
**And** progress is emitted via Tauri events as `IndexProgress { current, total, errors }` every 100 files (LD-42 checkpoints)
**And** the scan is cancellable via `commands.cancelIndexScan()`; partial index is retained
**And** initial indexing of a 1000-file Vault completes in <30s on baseline hardware (NFR per FR-15)
**And** subsequent launches with an unchanged Vault open the cached index in <1s.

### Story 3.7: Ship `orgsidian index {init|rebuild|stats|integrity}` CLI commands

As the **power user or CI operator**,
I want CLI commands to initialize, rebuild, inspect stats, and check integrity of the index,
So that LD-49 (`rebuild-index` as first-class command) and LD-27 (CLI as primary integration-test surface) are operational.

**Acceptance Criteria:**

**Given** Stories 3.4 + 3.5,
**When** the commands are added to `crates/orgsidian-cli/`,
**Then** `orgsidian index init <vault>` creates a fresh index for the Vault
**And** `orgsidian index rebuild <vault>` drops + rebuilds from `.org` files with progress to stdout (matching the UI's checkpoint cadence)
**And** `orgsidian index stats <vault>` prints headline count, file count, FTS5 document count, schema version, last-rebuild timestamp
**And** `orgsidian index integrity <vault>` runs `PRAGMA integrity_check` and exits non-zero on failure
**And** each command supports `--json` flag for scripting
**And** `crates/orgsidian-cli/tests/index_cmd.rs` exercises each command via `assert_cmd` against a fixture Vault.

---

## Epic 4: Editor Surface & Org-mode Awareness

### Story 4.1: Wire CodeMirror 6 host with StrictMode-safe lifecycle

As the **user opening a `.org` file**,
I want a CodeMirror 6 editor surface rendering the file content with StrictMode-safe `EditorView` lifecycle and React 19 `ref`-as-prop pattern,
So that no double-mount in dev causes a leaked `EditorView` instance (LD-6 + LD-2 React-19 implementation rule).

**Acceptance Criteria:**

**Given** Epic 3 closed,
**When** `shell-ui/src/components/editor/Editor.tsx` is implemented,
**Then** the component uses `useEffect(() => { const view = new EditorView(...); return () => view.destroy(); }, [])` for idempotent cleanup
**And** the component accepts `ref` as a regular prop (no `forwardRef`)
**And** opening a fixture `.org` file via `commands.openFile(path)` displays the source text in the editor
**And** `tests/Editor.test.tsx` (Vitest + happy-dom) mounts/unmounts the component and asserts no `EditorView` leak via a tracked-destruction spy.

### Story 4.2: Implement Raw editor mode

As the **user editing my `.org` file**,
I want a Raw editor mode showing plain `.org` source with syntax highlighting only (no decorations or widgets),
So that I can edit the underlying text without any visual layer interfering.

**Acceptance Criteria:**

**Given** Story 4.1,
**When** Raw mode is selected,
**Then** the editor shows the source text with syntax-highlight tokens only (org-mode-aware: headline asterisks, TODO state keywords, tags, timestamps)
**And** no Pseudo-WYSIWYG decorations are rendered
**And** `commands.setEditorMode("raw", filePath)` persists the choice via `tauri-plugin-store` at `<Vault>/.orgsidian/editor-prefs.json` per LD-40.

### Story 4.3a: Heading hierarchy decorations

As the **user editing my `.org` file**,
I want headings rendered with hierarchical font sizes (h1 largest → h6 smallest) when Pseudo-WYSIWYG mode is active,
So that document structure is visible at a glance.

**Traces:** FR-4, UJ-1.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active,
**When** a buffer contains headings `* H1` through `****** H6`,
**Then** each heading renders via a CodeMirror line decoration with computed CSS `font-size` monotonically decreasing from h1 to h6 (observable via `getComputedStyle`)
**And** the underlying source text (`* H1\n** H2\n…`) is preserved byte-identical (round-trip via FR-2 gate)
**And** the implementing module carries `//! Implements FR-4` as the first doc-comment line, verified by `tests/traceability.rs`.

### Story 4.3b: TODO state pill badges (click-to-cycle)

As the **user**,
I want TODO state keywords (TODO, NEXT, DONE, WAITING) rendered as colored pill badges with click-to-cycle behavior,
So that I can advance task state without typing.

**Traces:** FR-4, UJ-1.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active,
**When** a headline contains a TODO state keyword,
**Then** the keyword renders as a `Decoration.replace` widget with color mapped per state via `--org-accent-{todo,next,done,waiting}` CSS tokens
**And** clicking the widget cycles to the next state in the configured `#+TODO:` sequence via a `Transaction` tagged with `userEvent="input.cycle-todo"`
**And** the source text is updated atomically to the new keyword
**And** the widget passes the LD-6 recipe `WidgetType.eq()` source-range shallow-equal check.

### Story 4.3c: Tag pill labels

As the **user**,
I want tags (`:tag1:tag2:`) rendered as pill labels separated visually from the headline text,
So that tag taxonomy is immediately scannable.

**Traces:** FR-4.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active,
**When** a headline contains `:tag:` or `:tag1:tag2:tag3:` suffix,
**Then** each tag renders as a `Decoration.replace` pill widget with `--org-accent-tag` styling
**And** the colon delimiters are visually hidden but preserved in the source.

### Story 4.3d: Timestamps as human-readable dates with hover-for-source

As the **user**,
I want timestamps (`<2026-05-19 Mon 14:00>`, `[2026-05-19 Mon]`) rendered as human-readable dates with a tooltip showing the raw source on hover,
So that calendar context is readable without losing access to the org syntax.

**Traces:** FR-4, FR-9.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active,
**When** a buffer contains a timestamp,
**Then** the timestamp renders as a `Decoration.replace` widget with locale-formatted date + time (e.g., "Mon, May 19 · 14:00")
**And** hover for >300ms displays a tooltip with the raw source `<2026-05-19 Mon 14:00>`
**And** active vs inactive timestamps (`<…>` vs `[…]`) are visually distinct.

### Story 4.3e: Checkbox toggle widget (source-mutating click)

As the **user**,
I want clicking a checkbox widget to toggle the source `- [ ]` ↔ `- [X]`,
So that task completion is one click.

**Traces:** FR-4.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active + a `- [ ]` checkbox in the buffer,
**When** the user clicks the rendered widget,
**Then** the source text mutates `- [ ]` → `- [X]` via a `Transaction` tagged with `userEvent="input.toggle-checkbox"`
**And** the widget re-renders to reflect the new state
**And** `WidgetType.eq()` compares by source range (so a re-render does not destroy the widget unnecessarily)
**And** the widget passes LD-6 recipes (`widget.ignoreEvent() === false`).

### Story 4.3f: Link rendering as clickable underlined text

As the **user**,
I want links (`[[id:abc]]`, `[[wiki-link]]`, `[[file://path]]`, `http://...`) rendered as clickable underlined text,
So that following references is one click.

**Traces:** FR-4, UJ-6.

**Acceptance Criteria:**

**Given** Story 4.2 + Pseudo-WYSIWYG mode active,
**When** a buffer contains any link variant,
**Then** the link target renders as underlined text via `Decoration.mark`
**And** clicking the link emits a `LinkClicked { target, kind }` event consumed by the navigation layer
**And** the bracket markers `[[…]]` are visually hidden when the cursor is not on the line; visible when the cursor is on the line.

### Story 4.3g: Source-position fidelity (cursor, copy-paste, find/replace)

As the **power user**,
I want cursor placement, copy-paste, and find/replace operations to operate on source positions, not rendered positions, regardless of how many decorations are in view,
So that the editor is trustable across modes.

**Traces:** FR-3, FR-4.

**Acceptance Criteria:**

**Given** any combination of decorations from Stories 4.3a..4.3f active,
**When** the user invokes copy, paste, or find/replace,
**Then** the operation reads/writes source character offsets (not visual offsets)
**And** copying a heading line copies `** Heading` (source), not the rendered text
**And** find/replace on `TODO` finds the source keyword, not the rendered badge
**And** `assert_no_perf_regression!("story-4.3g-source-fidelity", …)` (Story 1.12) confirms operation latency does not regress >20% from baseline.

### Story 4.4: Implement Split editor mode

As the **user learning org-mode**,
I want a Split editor mode showing Raw source on the left and Pseudo-WYSIWYG preview on the right, scroll-synced,
So that I can build muscle memory for the syntax by seeing both sides simultaneously (FR-3).

**Acceptance Criteria:**

**Given** Stories 4.2 + 4.3,
**When** Split mode is selected,
**Then** the editor surface is split 50/50 between Raw and Pseudo-WYSIWYG views of the same buffer
**And** scroll position is synced between panes via shared `EditorState`
**And** edits in either pane update the underlying buffer atomically
**And** Split mode preference persists per file via Story 4.2's `editor-prefs.json`.

### Story 4.5: Implement Editor Mode switcher UI

As the **user**,
I want a UI control + keyboard shortcut to switch between Raw / Pseudo-WYSIWYG / Split modes for the current file,
So that FR-3 mode switching is discoverable and fast.

**Acceptance Criteria:**

**Given** Stories 4.2 + 4.3 + 4.4,
**When** the user invokes the mode switcher,
**Then** `shell-ui/src/components/editor/ModeSwitcher.tsx` renders a segmented control showing the active mode
**And** clicking a mode option switches the editor without losing buffer state
**And** the default keybinding `Cmd/Ctrl+Alt+M` cycles through the three modes
**And** mode switch completes in <200ms on a 5000-line file (FR-3 NFR)
**And** the per-file mode preference persists across app restarts.

### Story 4.6: Implement cross-platform default keybindings

As the **macOS / Linux / Windows user**,
I want default keybindings following desktop platform conventions (Cmd on macOS, Ctrl on Linux/Windows) for save, open, find, agenda, capture, TODO cycle, schedule, deadline, clock in/out,
So that FR-5 default keys feel native on my platform.

**Acceptance Criteria:**

**Given** Story 4.5,
**When** the default keymap is wired,
**Then** `shell-ui/src/components/editor/keybindings/default.ts` declares the default chord set with platform-detected `Cmd` vs `Ctrl` via `tauri-plugin-os`
**And** every daily org-mode action (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out) has a documented default chord
**And** an in-app reference panel at Settings → Keybindings lists all documented chords with their actions.

### Story 4.7: Implement optional Emacs keybindings mode

As the **org-mode user with Emacs muscle memory**,
I want an opt-in "Emacs keybindings" mode rebinding editor actions to Emacs-style chords (`C-x C-s` save, `C-c C-c` cycle TODO, etc.),
So that I can use Orgsidian without retraining my fingers (FR-5).

**Acceptance Criteria:**

**Given** Story 4.6,
**When** Emacs mode is enabled in Settings,
**Then** `shell-ui/src/components/editor/keybindings/emacs.ts` declares the Emacs chord set covering save, agenda, capture, TODO cycle, schedule, deadline, clock in/out
**And** conflicts with default Cmd/Ctrl shortcuts are resolved by the active keymap taking precedence
**And** the chord set is documented in the in-app reference panel under "Emacs mode"
**And** any gap (Emacs chord present but Orgsidian action unmapped) is documented in `docs/user-guide/emacs-keybindings.md`.

### Story 4.8: Implement Schedule/Deadline date picker

As the **user planning my work**,
I want to add, modify, or remove a Scheduled timestamp or Deadline on the current Headline via keyboard shortcut or context menu, with a date picker for fast entry and raw-typing fallback in Raw mode,
So that FR-9 timestamp editing is friction-free.

**Acceptance Criteria:**

**Given** Story 4.3 (semantic timestamp rendering),
**When** the user invokes "Set Schedule" or "Set Deadline" on the current Headline,
**Then** `shell-ui/src/components/org/OrgDatePicker.tsx` opens with a calendar + time picker + `+1d`/`+1w` shortcuts
**And** confirming the picker writes `SCHEDULED: <YYYY-MM-DD Day HH:MM>` (or `DEADLINE:`) to the planning section under the Headline via `commands.setScheduled(headlineId, timestamp)`
**And** recurring timestamps (e.g., `<2026-05-19 Mon +1w>`) are preserved on round-trip and respected by Agenda
**And** Raw mode allows raw typing of `SCHEDULED:` and `DEADLINE:` lines without picker invocation.

### Story 4.9: Activate nightly memory soak gate (LD-43, anticipated per Party Mode P1)

As the **author / contributor**,
I want the LD-43 nightly 12-hour memory soak gate active from this epic onwards (anticipated from Epic 6 per Party Mode P1 — CM6 decorations are the most likely leak source),
So that decoration / widget memory leaks are caught within 24h of introduction.

**Acceptance Criteria:**

**Given** Stories 4.3 + 4.4,
**When** the nightly soak job is wired,
**Then** `.github/workflows/nightly.yml` adds a dedicated Linux runner job running a 12-hour scripted session (200 buffer open/close cycles + 50 plugin re-init cycles + 1000 agenda queries)
**And** RSS is sampled every 30 minutes via `/proc/self/statm`
**And** the job fails if RSS drift >10% over 11 hours (warmup excluded, minute 60 → minute 720)
**And** a failing soak blocks all PR merges to `main` until resolved (per LD-32 stale-nightly merge gate).

---

## Epic 5: External-Edits Co-existence (Safe Fallback)

### Story 5.1: Implement `notify-rs` filesystem watcher with debounce

As the **user editing my `.org` files in VS Code alongside Orgsidian**,
I want Orgsidian to detect external file changes within 5 seconds and debounce atomic-save event sequences (vim/VS Code/Emacs emit 3-12 events per save),
So that FR-16 detection is reliable and Epic 8 Capture doesn't spuriously trigger merge state machines (LD-9).

**Acceptance Criteria:**

**Given** Epic 3 closed,
**When** `crates/orgsidian-watcher/src/watcher.rs` wraps `notify-rs`,
**Then** the watcher emits a single `FileChanged { path }` event after a 250ms debounce window coalesces atomic-save delete+create+modify sequences
**And** the watcher abstraction layer in `WatcherFacade` allows deterministic fakes for unit tests
**And** external writes are detected within 5 seconds on macOS, Linux, and Windows (LD-9 NFR)
**And** network mounts and case-folding filesystems are documented as v0.1 unsupported configurations in `docs/architecture/resilience.md`.

### Story 5.2: Record golden-trace fixtures from vim / VS Code / Emacs save sequences

As the **author / contributor**,
I want golden traces of real external-editor save sequences (vim swap+rename, VS Code temp+rename, Emacs backup+save) committed as fixtures,
So that OD-3 debounce calibration is data-driven and Epic 9 Merge Dialog tests can replay the same traces.

**Acceptance Criteria:**

**Given** Story 5.1,
**When** the fixtures are recorded,
**Then** `crates/orgsidian-watcher/tests/golden_traces/{vim, vscode, emacs}.json` contain timestamped event sequences
**And** `tests/debounce.rs` replays each trace and asserts the watcher emits exactly one `FileChanged` event per save
**And** `fixtures/fixtures.toml` declares ownership of these traces as `owner = "epic-5"` per Murat P1.

### Story 5.3: Implement `ConflictState` rich struct + `ConflictStrategy` pattern (Party Mode P0)

As the **author / contributor**,
I want the conflict state modeled as a rich struct (`ancestor_hash`, `external_content`, `buffer_content`) with `ConflictStrategy` as a pattern (variants: `BlockWithWarning`, `ThreePaneMergeDialog`) from day-1,
So that Epic 9 swaps the strategy variant without rewriting the watcher state machine (Winston + Murat consensus from Party Mode P0).

**Acceptance Criteria:**

**Given** Stories 5.1 + 5.2 + 3.2,
**When** the strategy pattern is implemented,
**Then** `crates/orgsidian-vault/src/conflict.rs` declares `pub struct ConflictState { ancestor_hash: Sha256Hash, external_content: String, buffer_content: String, file_path: PathBuf }`
**And** `pub enum ConflictStrategy` declares variants `BlockWithWarning | ThreePaneMergeDialog` with `pub trait ResolveConflict { fn resolve(&self, state: ConflictState) -> Resolution }` implemented by each
**And** the watcher state machine consumes `&dyn ResolveConflict` (active strategy injected at startup)
**And** `tests/conflict_strategy.rs` parameterizes a single test suite over both strategies and asserts contract invariants (`Resolution::Block`, `Resolution::WriteMerged`, `Resolution::Cancel`)
**And** Epic 9 will swap the active strategy without modifying the state machine.

### Story 5.4: Implement clean-buffer auto-reload + re-index on external write

As the **user using Orgsidian alongside VS Code**,
I want a file with a clean buffer to be reloaded automatically when externally modified, with the cursor position preserved if the line is unchanged, resetting to top if the line was deleted (FR-16),
So that external edits don't require manual reload.

**Acceptance Criteria:**

**Given** Stories 5.1 + 5.2 + 5.3,
**When** the watcher detects an external write on a file with `DirtyBufferManager::is_dirty(path) == false`,
**Then** the in-memory buffer is refreshed from disk
**And** the SQLite index is incrementally re-synced for that file (`orgsidian-index::sync::incremental`)
**And** the editor's cursor position is preserved if the cursor's source line is unchanged; otherwise reset to top
**And** a non-modal status notice "file reloaded from disk" appears for 3 seconds
**And** `tests/external_write_clean.rs` replays a golden trace and asserts buffer + index + cursor invariants.

### Story 5.5: Implement Dirty-Buffer block-save fallback with conflict warning UI

As the **user with unsaved changes when an external write occurs**,
I want Orgsidian to block the save attempt and surface a conflict warning ("The file was changed externally. Resolve manually or discard external changes."),
So that v0.1 Alpha ships the FR-16 safety contract (NFR-16 Single Writer Rule) without the full Merge Dialog UI (which lands in Epic 9).

**Acceptance Criteria:**

**Given** Stories 5.3 + 5.4,
**When** the active `ConflictStrategy` is `BlockWithWarning` and an external write is detected on a file with a Dirty Buffer,
**Then** the watcher emits a `ConflictDetected { path, state }` Tauri event
**And** the frontend renders a banner in the editor surface: "{path} was changed externally — save blocked. [Discard external changes] [View file in default editor]"
**And** save attempts via `commands.saveFile(path, content)` return `Err(OrgError::Vault(VaultError::ExternalConflict { path }))`
**And** clicking "Discard external changes" allows a subsequent save to overwrite (still atomic-write)
**And** the module carries `//! Implements FR-16 (v0.1 fallback strategy)`.

---

## Epic 6: v0.1 Alpha Release — First Launch & Day-One Agenda Snapshot

### Story 6.1: Implement Personal GTD + Student + Freelancer Starter Vault content + generator

As the **first-time user**,
I want choosing "Personal GTD", "Student", or "Freelancer" Starter Vault to create a folder populated with realistic `.org` files (one project, an inbox, a journal, a someday list, agenda content populated for "today" relative to first launch),
So that I see the workflow, not the syntax, in my first 5 minutes (FR-18 v0.1 Alpha — Freelancer promoted from v0.5 per 2026-05-20 reconciliation to give the lighthouse persona the full integration demo on first launch).

**Traces:** FR-18, UJ-3, UJ-4.

**Acceptance Criteria:**

**Given** Epic 3 closed AND Story 8.7 Backlinks sidebar panel shipped (required by the Freelancer ≥1-backlink AC),
**When** the generator runs for a chosen starter,
**Then** `crates/orgsidian-core/src/starter_vault/{personal_gtd, student, freelancer}.rs` writes the starter's `.org` files to the user-chosen folder
**And** the agenda content includes Scheduled timestamps for "today" relative to first-launch date
**And** the Inbox file `inbox.org` exists at Vault root
**And** opening the Vault immediately shows non-empty Today/Week Agenda content
**And** the Freelancer starter additionally includes ≥1 project with ≥3 milestones, ≥1 clocked task in the LOGBOOK, and ≥1 `id:` or `[[wiki-link]]` reference between Headlines — so the BacklinksPanel (Story 8.7) shows ≥1 backlink for the project's main Headline on first launch (demonstrating the integration UJ-3 narrates)
**And** each starter ships from real-world GTD / Student / Freelancer vault patterns documented in `docs/user-guide/starter-vaults.md`
**And** the implementing modules carry `//! Implements FR-18 (Personal GTD + Student + Freelancer starters; Empty in Story 11.1)` as the first doc-comment line.

### Story 6.2: Implement Starter Vault picker UI on first launch

As the **first-time user**,
I want the first-launch screen offering "Personal GTD", "Student", or "Freelancer" Starter Vault choices,
So that I'm onboarded into the workflow within seconds (FR-18 v0.1 Alpha — Empty option deferred to v0.5 per 2026-05-20 reconciliation; user with existing `.org` folder can still designate via Settings → Vault until then).

**Acceptance Criteria:**

**Given** Story 6.1,
**When** Orgsidian launches with no configured Vault,
**Then** `shell-ui/src/components/onboarding/StarterVaultPicker.tsx` renders three primary options (Personal GTD, Student, Freelancer) plus a secondary "Use my own folder" link that routes to Story 3.6's `designateVault` flow (this is the v0.1 stand-in for the Empty Starter — the explicit Empty card with onboarding-coaching ships in Story 11.1 v0.5 Beta)
**And** selecting Personal GTD, Student, or Freelancer prompts for a target folder via `tauri-plugin-dialog`, then invokes the generator from Story 6.1
**And** the "Use my own folder" link prompts for an existing `.org` folder to designate via Story 3.6's `designateVault` flow
**And** the picker is dismissed once a Vault is configured and the user lands on the `/today` route.

### Story 6.3: Implement basic Today Agenda view

As the **user**,
I want the `/today` route showing today's Scheduled items + Deadline items overdue-or-today, grouped by file with click-to-open behavior,
So that I see "my day" on launch (FR-7 partial — Today view subset of the full Today Dashboard which lands in Epic 7).

**Acceptance Criteria:**

**Given** Epic 4 closed,
**When** `/today` renders,
**Then** `shell-ui/src/components/agenda/AgendaToday.tsx` queries `orgsidian-index::query::agenda::today()` and renders the result
**And** items are grouped by source file
**And** clicking an item opens the editor at the source Headline via the TanStack Router `/editor/$filePath/$headlineId` route
**And** the render completes in <500ms on a 1000-file Vault (precursor to NFR for full Today Dashboard).

### Story 6.4: Implement Week Agenda view

As the **user**,
I want the `/agenda/week` route showing a rolling 7-day Agenda grouped by date,
So that I can plan beyond today (FR-7 partial).

**Acceptance Criteria:**

**Given** Story 6.3,
**When** `/agenda/week` renders,
**Then** `shell-ui/src/components/agenda/AgendaWeek.tsx` queries `orgsidian-index::query::agenda::week(start_date)`
**And** items are grouped by date with the current day highlighted
**And** view-switching from `/today` to `/agenda/week` completes in <200ms on a 1000-file Vault (FR-7 consequences).

### Story 6.5: Freeze `IndexQuery` trait public API (automated via `cargo-semver-checks`)

As the **author / contributor**,
I want the `orgsidian-index::query` trait surface (`IndexQuery` trait + `AgendaQuery`/`SearchQuery`/`BacklinksQuery` types) frozen at the end of Epic 6 with **automated semver enforcement** via `cargo-semver-checks`,
So that Epic 7 + Epic 8 cannot accidentally break the contract in CI (Murat round-2 fix — replaces manual freeze gate with automated check).

**Traces:** LD-32, LD-33, NFR-19.

**Acceptance Criteria:**

**Given** Stories 3.5 + 6.3 + 6.4,
**When** the freeze gate is applied,
**Then** `crates/orgsidian-index/src/query/mod.rs` declares the `IndexQuery` trait with `///` doc-comments documenting the contract (input types, return types, error variants)
**And** the v0.1 baseline trait surface (per the 2026-05-20 reconciliation decision to freeze upfront rather than rely on semver-minor additions during Epic 8) includes: `agenda::{today, week, custom}` (Stories 6.3, 6.4, 7.4); `search::query(q) -> Vec<SearchResult>` *and* `search::search_stream(q) -> impl Iterator<SearchResult>` (Story 8.4 two-tier streaming `<100ms first 10` / `<200ms full 50`); `backlinks::for_headline(id) -> Vec<Backlink>` (Story 8.6); `backlinks::unlinked_mentions(headline_id) -> Vec<UnlinkedMention>` (Story 12.0 v0.5+, but signature in baseline so v0.5 lands as semver-minor body addition, not breaking); `graph::adjacency(scope) -> GraphData { nodes: Vec<NodeRef{ id, file, title }>, edges: Vec<Edge{ src_id, dst_id, kind }> }` (Story 8.10 FR-26)
**And** `crates/orgsidian-index/CHANGELOG.md` records a `Query API: v1.0` entry at the end of this epic with the published semver baseline (incl. the streaming + graph + unlinked-mentions additions)
**And** `.github/workflows/pr.yml` runs `cargo-semver-checks check-release --baseline-rev v0.1.0-alpha.x -p orgsidian-index` and **fails the PR** on any breaking change to the `query::*` public surface
**And** semver-minor additions (new trait method with default impl, new variant on `#[non_exhaustive]` enum) pass the check; semver-major changes (signature, removal) require explicit CHANGELOG bump + reviewer override.

### Story 6.6: Hardcoded coaching balloons for UJ-4 v0.1 first-run (Party Mode round 2 P0 — Sally)

As the **first-time user** landing on the Today Dashboard with example content from a Starter Vault,
I want a small "what is this?" balloon pointing at the agenda ("This is your day. Click any task to open the source file."),
So that I understand what I'm looking at without reading a manual — honoring the UJ-4 first-5-minutes promise that v0.1 ships without waiting for the full coaching registry of Epic 11.

**Traces:** FR-21 (partial — hardcoded subset), FR-18, UJ-4.

**Acceptance Criteria:**

**Given** Stories 6.1 + 6.2 + 6.3 (StarterVaultPicker + Today Agenda view),
**When** the user finishes Starter Vault selection and lands on `/today` for the first time,
**Then** a non-modal balloon renders pointing at the first agenda item with the text `[microcopy: draft]` "**This is your day.** Click any task to open the source file."
**And** a second balloon renders on the Inbox preview section: "**Anything on your mind?** Press `Cmd/Ctrl+Shift+Space` to capture from anywhere."
**And** dismissing either balloon (X button) persists the dismissal at `<Vault>/.orgsidian/coaching-dismissed.json` keyed by hardcoded coaching IDs (`UJ4_TODAY_INTRO`, `UJ4_CAPTURE_INTRO`)
**And** the hardcoded balloons are **directly removed** by Story 11.4 when the registry-driven `CoachingSlot` API ships in v0.5 Beta (refactor, not addition) — Story 11.4 imports the same coaching IDs to honor the dismissals
**And** the balloon text and design tokens are recorded in `docs/microcopy-registry.md` with status `[draft]`.

### Story 6.7: Ship dark + light default themes (WCAG AA)

As the **user**,
I want Orgsidian to ship with dark and light default themes meeting WCAG AA contrast for body text and primary UI chrome,
So that NFR-9 accessibility baseline is honored from v0.1 (FR-22 partial — CSS override lands in Epic 12).

**Acceptance Criteria:**

**Given** Epic 1 closed,
**When** the themes are committed,
**Then** `shell-ui/src/themes/{tokens.css, dark.css, light.css}` declare the `--org-*` CSS variable vocabulary per architecture step 3
**And** theme switching is instant (`document.body.dataset.theme = "dark"`)
**And** contrast ratios for body text and primary UI chrome meet WCAG AA on both themes — **verified by the Story 1.17 LD-58 contrast-matrix Vitest test on the `--org-*-fg`/`--org-*-bg` pairs AND by the Story 1.17 axe-core gate on every `@a11y`-tagged Playwright scenario** (replaces the prior axe-core-E2E-spec wording; the gate is now per-PR + canonical via Story 1.17)
**And** `tokens.css` declares the pair-role metadata required by Story 1.17's contrast test (body-text pairs / large-text pairs / UI-chrome pairs) so the Vitest gate has structured input rather than ad-hoc heuristics
**And** Settings → Appearance allows toggling between dark / light / system-default.

### Story 6.8: Implement macOS DMG packaging + signing + notarization

As the **macOS adopter**,
I want a signed and notarized `.dmg` installer downloadable from GitHub Releases,
So that I can install Orgsidian without Gatekeeper warnings (LD-19 + LD-34).

**Acceptance Criteria:**

**Given** Epic 1 + Stories 6.1-6.6,
**When** a `v0.1.0-alpha.x` tag is pushed,
**Then** `.github/workflows/release.yml` builds the macOS-arm64 DMG via Tauri bundler
**And** the DMG is signed with the Apple Developer ID Application certificate (key stored as GitHub Actions secret)
**And** the DMG is notarized via `notarytool` and the staple is attached
**And** the artifact is uploaded to the GitHub Release page
**And** a Homebrew cask formula is published to `orgsidian/tap` per LD-34.

### Story 6.9: Implement Linux AppImage packaging

As the **Linux adopter**,
I want an `.AppImage` downloadable from GitHub Releases,
So that I can run Orgsidian on Ubuntu/Arch without distro-specific packaging (LD-34).

**Acceptance Criteria:**

**Given** Story 6.7's release pipeline,
**When** a release tag is pushed,
**Then** `.github/workflows/release.yml` builds the Linux-x86_64 AppImage via Tauri bundler
**And** the AppImage is GPG-signed with checksums published alongside
**And** the artifact is uploaded to the GitHub Release page
**And** a Flathub manifest is filed best-effort per LD-34.

### Story 6.10: Publish v0.1 Alpha — README, landing page, announcement

As the **prospective adopter discovering Orgsidian**,
I want the GitHub README + a landing page + an HN/Reddit announcement explaining the vision, install paths, and v0.1 capabilities,
So that SM-1 (50 technical comments + 10 early adopters) becomes measurable.

**Acceptance Criteria:**

**Given** Stories 6.7 + 6.8,
**When** v0.1 Alpha ships,
**Then** root `README.md` is rewritten with vision, install paths (DMG/Homebrew/AppImage), feature summary, and a "How to contribute" section
**And** a minimal landing page exists at `docs/landing/index.html` (or external static-site host) pointing to the GitHub Release
**And** the `orgsidian/orgsidian` repository visibility is flipped from private to public before the announcement post is published (`gh api -X PATCH /repos/orgsidian/orgsidian -f visibility=public` or web UI), and a smoke check confirms the public README + LICENSE render at the public URL per LD-5
**And** an announcement draft for HN + Reddit r/orgmode is committed at `docs/announcements/v0.1-alpha.md` (timing/posting at author's discretion).

---

## Epic 7: Today Dashboard & Time Tracking

### Story 7.1: Implement Today Dashboard surface

As the **user**,
I want the `/today` route upgraded to the full Today Dashboard showing Scheduled items for today, Deadline items today-or-overdue, items flagged with a configurable "today" tag, the Inbox preview (first N entries), and the Active Clock if any,
So that "my day" is one screen on launch (FR-6).

**Acceptance Criteria:**

**Given** Stories 6.3 + 6.5,
**When** the Today Dashboard renders,
**Then** `shell-ui/src/components/today/TodayDashboard.tsx` renders five sections: `Scheduled | Deadline | Today-Tag | Inbox Preview | Active Clock`
**And** each section is collapsible with a chevron toggle
**And** dashboard render is gated by `assert_no_perf_regression!("story-7.1-today-dashboard", "tests/perf-baselines/story-7.1.json", || { … })` (Story 1.12 infra) — absolute target <500ms on a 1000-file Vault per NFR is the initial baseline; subsequent runs may not regress >20%
**And** Inbox preview shows the first N entries (default N=5, configurable in Settings)
**And** the implementing module carries `//! Implements FR-6` as the first doc-comment line, verified by `tests/traceability.rs`.

**Traces:** FR-6, UJ-1.

### Story 7.2: Persist Today Dashboard section preferences

As the **user**,
I want each section's collapsed/expanded state to persist across app restarts,
So that my preferences survive.

**Acceptance Criteria:**

**Given** Story 7.1,
**When** the user collapses or expands a section,
**Then** the state is persisted via `tauri-plugin-store` at `<Vault>/.orgsidian/today-prefs.json` per LD-40 per-Vault state
**And** relaunching the app restores the same section states.

### Story 7.3: Add empty-state messages per section

As the **user**,
I want each section's empty state to show a contextual message ("No tasks scheduled for today — nice.") rather than a blank pane,
So that empty isn't ambiguous (FR-6 consequences + FR-21 inline-coaching tone).

**Acceptance Criteria:**

**Given** Story 7.1,
**When** a section has no items,
**Then** the section renders a copy-blessed empty-state message ("No tasks scheduled for today — nice." / "Inbox empty." / "No active clock — pick a task and start tracking.")
**And** each message follows the FR-21 tone documented in `shell-ui/src/coaching/coachingRegistry.ts` (centralized; aligns with Epic 11 coaching).

### Story 7.4: Implement Custom Agenda view with date range picker

As the **user**,
I want the `/agenda/custom` route with a date range picker, completing FR-7,
So that I can plan for arbitrary date ranges beyond Today / Week.

**Acceptance Criteria:**

**Given** Story 6.4 + 6.5 (`IndexQuery` frozen),
**When** the Custom Agenda view renders,
**Then** `shell-ui/src/components/agenda/AgendaCustom.tsx` exposes a date range picker (start / end) + filter inputs (tag, TODO state, file path)
**And** the result list is grouped by date and rendered via `@tanstack/react-virtual` per LD-30 (must scale to 1k+ items)
**And** typed search params (`?start=`, `?end=`, `?tag=`, `?todo=`) drive the route per LD-29.

### Story 7.5: Implement saved Agenda filter presets

As the **user**,
I want to save named Agenda filter presets and recall them with one click,
So that recurring queries (e.g., "@home tag this week") are friction-free (FR-7 consequences).

**Acceptance Criteria:**

**Given** Story 7.4,
**When** the user saves a preset,
**Then** the preset is persisted in the per-Vault TOML settings store (Story 1.18 — under `[agenda_presets]`) with name + view + filters; LD-40 supersedes the prior `agenda-presets.json` location
**And** the Agenda sidebar shows the saved presets list
**And** clicking a preset restores the view + filters
**And** preset deletion is available via a context menu
**And** (added 2026-05-20 per PRD §4.2 FR-7 enhancement) **two default named presets ship out of the box**: `Done This Week` (filter `todo:DONE` + completion date in the rolling-7-days range) and `Done This Month` (analogous, rolling-30-days). Defaults are seeded into the per-Vault TOML store on first launch — idempotent, not re-seeded if the user deleted them
**And** the Story 6.1 starter-vault generators include fixture content that surfaces non-empty results in both default presets on first launch (≥2 `DONE` headlines completed in the relevant ranges).

### Story 7.6: Implement Clock manager + LOGBOOK persistence

As the **user**,
I want to clock in / clock out / resume on a Headline, with entries persisted as standard org `CLOCK:` lines in the LOGBOOK drawer,
So that my time tracking is org-compatible and survives Orgsidian itself (FR-8 functional).

**Acceptance Criteria:**

**Given** Epic 4 + Epic 3 closed,
**When** `crates/orgsidian-core/src/clock.rs` is implemented,
**Then** `commands.clockIn(headlineId)` sets the Active Clock and starts wall-clock tracking
**And** `commands.clockOut()` stops the Active Clock and appends `CLOCK: [<start>]--[<end>] => HH:MM` to the LOGBOOK drawer under the Headline (creating the drawer if absent)
**And** `commands.clockResume(headlineId)` resumes a previously paused entry (most recent unclosed CLOCK: line for that Headline)
**And** at most one Active Clock at a time; clocking into a new Headline auto-stops the prior
**And** time totals (per Headline, per subtree, per tag, per date range) are computable via `orgsidian-core::clock::totals(scope, range)`
**And** the Active Clock state file `<Vault>/.orgsidian/active-clock.json` persists `{ headline_id, started_at, last_active_at }` where `last_active_at` is refreshed on every app foreground / window-focus event via `tauri::Window::on_window_event` listener (Party Mode round 2 P0 — Sally — needed for Story 7.7 stale-clock pre-fill)
**And** the implementing module carries `//! Implements FR-8 (functional)` as the first doc-comment line, verified by `tests/traceability.rs`.

**Traces:** FR-8, UJ-1.

### Story 7.7: Implement prior-session running-clock prompt on launch

As the **freelance consultant** who closed the laptop yesterday at 18:00 with a clock still running,
I want Orgsidian on relaunch to surface the still-tracking session with the exact duration that would be recorded if I did nothing, and offer me three actions in safest-default order,
So that I never silently record 14 hours from leaving the app open overnight (UJ-1 edge case).

**Traces:** FR-8, UJ-1.

**Acceptance Criteria:**

**Given** Story 7.6 (with `last_active_at` field persistence),
**When** Orgsidian launches with a non-empty Active Clock state in `<Vault>/.orgsidian/active-clock.json`,
**Then** a modal prompts with the following `[microcopy: draft]` content (final copy pass recorded in `docs/microcopy-registry.md`):
> **Still tracking from yesterday?**
> *{Headline}* was being tracked when Orgsidian last closed at *{last_active_at | formatted}*.
> Currently logged: **{HH:MM} (if you keep tracking) / {HH:MM} (if you adjust to last-active)**.
> `[ Adjust end time ]`  `[ Keep tracking ]`  `[ Discard this session ]`

**And** the keyboard default-focused button is "Adjust end time" (safest); Enter confirms; Esc invokes Adjust (not Cancel)
**And** "Adjust end time" opens a time picker pre-filled with `last_active_at` from Story 7.6
**And** "Keep tracking" resumes the clock from the original `started_at` (no source mutation)
**And** "Discard this session" removes the open `CLOCK:` line from the source LOGBOOK drawer
**And** an integration test in `crates/orgsidian-core/tests/stale_clock.rs` fakes a 14-hour gap between `started_at` and `last_active_at` and asserts each button produces the documented state transition.

### Story 7.8: Implement ClockEditor component for time entry editing

As the **user reviewing my clocked time**,
I want to edit a clock entry's start / end / duration via a dedicated UI,
So that mistakes (forgot-to-clock-out, wrong day) can be corrected in-app (FR-8 partial — full status-bar polish lands in Epic 13).

**Acceptance Criteria:**

**Given** Story 7.6,
**When** the user clicks a clock entry in the LOGBOOK drawer (or via Agenda),
**Then** `shell-ui/src/components/org/ClockEditor.tsx` opens with start / end / duration fields
**And** editing any field recomputes the others (duration = end - start)
**And** confirming writes the updated `CLOCK:` line via `commands.updateClockEntry(headlineId, entryIndex, newStart, newEnd)`
**And** the LOGBOOK drawer is re-rendered.

---

## Epic 8: Capture, Search, Backlinks

### Story 8.1: Implement Quick Capture as separate Tauri window

As the **user wanting to capture a thought without breaking flow**,
I want pressing a global hotkey (default `Cmd/Ctrl+Shift+Space`) to open a small Quick Capture dialog centered on my screen with a multi-line input, without focus-stealing the main app,
So that captures land in my Inbox in <1s (FR-10, LD-28).

**Acceptance Criteria:**

**Given** Epic 4 closed,
**When** `tauri-plugin-global-shortcut` is wired and the user presses the hotkey from any app,
**Then** the `quick-capture` Tauri window (separate Vite bundle from `shell-ui/quick-capture.html` per LD-28) opens centered on the screen
**And** the input is multi-line and auto-focused
**And** the main Orgsidian window does not steal focus or come to foreground
**And** end-to-end latency (hotkey → window visible → submit → entry persisted to Inbox) is gated by `assert_no_perf_regression!("story-8.1-capture-e2e", …)` (Story 1.12) — initial baseline ≤1s per NFR-5; subsequent runs may not regress >20%
**And** **return-focus invariant** (Party Mode round 2 P0 — Sally — UJ-2 round-trip ≤3s): after the user presses Enter to submit, the previously-focused application (recorded at hotkey-press time via platform-specific calls — `NSWorkspace.shared.frontmostApplication` on macOS, `GetForegroundWindow` on Windows, `_NET_ACTIVE_WINDOW` on X11) regains focus within a baseline gated by `assert_no_perf_regression!("story-8.1-return-focus", …)` (initial baseline ≤500ms)
**And** the global hotkey is configurable in Settings → Capture.

**Traces:** FR-10, NFR-5, UJ-2.

### Story 8.2: Implement Inbox append with timestamp drawer

As the **user submitting a Quick Capture**,
I want the captured text appended to the configured Inbox file with a creation timestamp drawer entry,
So that I can later sort captures chronologically (FR-10).

**Acceptance Criteria:**

**Given** Story 8.1,
**When** the user submits a capture via Enter,
**Then** `commands.submitCapture(text)` appends a new Headline to the configured Inbox file (default `<Vault>/inbox.org`)
**And** the appended Headline carries a `:PROPERTIES:` drawer with `:CREATED: <YYYY-MM-DD Day HH:MM>`
**And** the timestamp format is configurable in Settings → Capture
**And** the Quick Capture window dismisses on submit and on Escape, returning focus to the prior application
**And** module carries `//! Implements FR-10`.

### Story 8.3: Implement system tray Quick Capture fallback

As the **user without a working global hotkey** (e.g., conflict with OS),
I want a system tray menu entry offering Quick Capture as a fallback,
So that I can still capture without the hotkey (FR-11).

**Acceptance Criteria:**

**Given** Stories 8.1 + 8.2,
**When** the user clicks the tray icon → "Quick Capture",
**Then** the same `quick-capture` window opens with identical UX to the hotkey flow
**And** the tray entry is enabled by default and disable-able in Settings → Capture
**And** `orgsidian-shell-app/src/tray.rs` registers the tray icon on macOS menubar, Windows tray, Linux indicator (where available).

### Story 8.4: Implement FTS5 search query API

As the **user looking for an old note**,
I want a full-text search across all `.org` files in my Vault with query syntax supporting plain words, exact phrase quotes, tag filter (`#tag:`), file filter (`file:`), TODO state filter (`todo:`),
So that FR-12 surface is queryable from any consumer (UI, CLI, plugin).

**Acceptance Criteria:**

**Given** Epic 3 closed,
**When** `crates/orgsidian-index/src/query/search.rs` exposes **two API entry points** per the 2026-05-20 two-tier reconciliation: `pub fn search(query: &SearchQuery) -> Result<Vec<SearchResult>, IndexError>` (full-batch, returns up to 50 results) AND `pub fn search_stream(query: &SearchQuery) -> Result<impl Iterator<Item = SearchResult>, IndexError>` (streaming — first 10 yieldable for early UI render before the full batch completes; `rusqlite::Statement::query_map` is the natural backing primitive),
**Then** the query parser handles plain words, `"exact phrase"`, `#tag:value`, `file:path-glob`, `todo:STATE`
**And** results are returned grouped by file with the matched line previewed
**And** latency is gated by **two** perf assertions (Story 1.12) per the FR-12 two-tier budget: `assert_no_perf_regression!("story-8.4-search-10results", …)` initial baseline ≤100ms for time-to-first-10 streaming results on a 1000-file Vault, AND `assert_no_perf_regression!("story-8.4-search-50results", …)` initial baseline ≤200ms for the full 50 results — per NFR-4 + PRD §4.3 FR-12 post-2026-05-20
**And** both entry points are surfaced through the frozen `IndexQuery` trait (Story 6.5 baseline — `search` + `search_stream` shipped upfront, not added during Epic 8)
**And** the CLI command `orgsidian query search <query>` exposes the same API per LD-27.

**Traces:** FR-12, NFR-4, UJ-6.

### Story 8.5: Implement Command Palette UI with search

As the **user**,
I want pressing `Cmd/Ctrl+P` to open a Command Palette where typing a query searches across the Vault and selecting a result opens the file at that line,
So that "find anything" is one keystroke away (FR-12).

**Acceptance Criteria:**

**Given** Story 8.4,
**When** the user invokes the palette,
**Then** `shell-ui/src/components/palette/CommandPalette.tsx` (built on `cmdk` via shadcn) opens with a query input
**And** typing a query consumes the **streaming** API from `commands.searchStream(query)` debounced by 50ms (paints first 10 results progressively before the full 50 arrive — per Story 8.4 two-tier contract; tested by `e2e/palette-streaming.spec.ts` asserting first-10-paint before full-50 completion)
**And** selecting a result navigates to `/editor/$filePath/$headlineId` via TanStack Router
**And** `Cmd/Ctrl+Shift+F` is an additional binding documented in the keybinding reference panel.

### Story 8.6: Implement Backlinks query API

As the **user reading a note**,
I want a query returning all other Headlines that reference the current Headline via `id:` link or `[[wiki-link]]`,
So that FR-13 backlinks surface is queryable from any consumer.

**Acceptance Criteria:**

**Given** Epic 3 closed,
**When** `crates/orgsidian-index/src/query/backlinks.rs` exposes `pub fn backlinks(headline_id: HeadlineId) -> Result<Vec<Backlink>, IndexError>`,
**Then** each `Backlink` includes the linking Headline's title + a short context snippet (one line of surrounding source)
**And** both `id:` and `[[wiki-link]]` references are indexed and returned
**And** the CLI command `orgsidian query backlinks <headline-id>` exposes the same API per LD-27.

### Story 8.7: Implement Backlinks sidebar panel

As the **user**,
I want a sidebar panel showing the Backlinks for the current Headline, updating <100ms after the cursor moves,
So that note discovery is two clicks away (FR-13).

**Acceptance Criteria:**

**Given** Story 8.6,
**When** the cursor is on a Headline in the editor,
**Then** `shell-ui/src/components/org/BacklinksPanel.tsx` renders the Backlinks list grouped by source file
**And** the panel updates within 100ms of cursor moving to a new Headline (FR-13 NFR)
**And** clicking a backlink navigates to the source via the editor route
**And** module carries `//! Implements FR-13`.

### Story 8.8: UJ-6 spine — Riccardo searches across two years (Party Mode round 2 P0 — Sally)

As the **freelance consultant** who tracked a client engagement two years ago,
I want a single end-to-end flow taking me from `Cmd/Ctrl+P` → typed query → grouped results → click → editor at the exact Headline → Backlinks sidebar revealing related notes,
So that UJ-6 is testable as a coherent journey rather than fragmented across Stories 8.4-8.7.

**Traces:** UJ-6 (spine), FR-12, FR-13.

**Acceptance Criteria:**

**Given** Stories 8.4 + 8.5 + 8.6 + 8.7 closed,
**When** the integration spine test runs,
**Then** `shell-ui/e2e/uj6-search-spine.spec.ts` (Playwright + Tauri WebDriver) executes the following scripted flow on a fixture Vault containing ≥2 years of dated `.org` files with `id:` cross-references:
  1. Press `Cmd+P` → palette opens within 100ms
  2. Type "kubernetes ingress" → first 10 results paint within the Story 8.4 two-tier perf budget (`<100ms` time-to-first-10 streaming); full 50 results complete within `<200ms`
  3. Assert results are **grouped by file** with the matched line previewed (not a flat list)
  4. Click the first result → editor route `/editor/$filePath/$headlineId` opens at the exact Headline (line scrolled into view, cursor on the source line)
  5. Backlinks sidebar (Story 8.7) renders within 100ms with ≥1 backlink showing the linking Headline title + context snippet
  6. Click the backlink → navigate to the linked source Headline; original Backlinks panel updates in <100ms
**And** the spine test runs on macOS-arm64 + Ubuntu-LTS per PR (Windows nightly)
**And** any single-step failure produces a screenshot of the failing step in `test-results/`.

### Story 8.9: Plugin API consistency checkpoint (preview LD-50)

As the **author / contributor**,
I want a checkpoint verifying that Capture, Search, Backlinks consume the `OrgsidianPlugin` trait surface without parallel "private" hooks,
So that LD-50 final surface review at Epic 12 ratifies a coherent API (Murat P2 — bring forward the review preview).

**Acceptance Criteria:**

**Given** Stories 8.2 + 8.4 + 8.6 + 8.10,
**When** the checkpoint review is performed,
**Then** `docs/plugin-api/v0.5-checkpoint-epic-8.md` lists the `Event` variants emitted by Capture / Search / Backlinks / Graph (`CaptureSubmitted`, `AgendaQueried`, `GraphRequested`, etc.) and the hook methods consumed
**And** any deviations from LD-26 are flagged for resolution before Epic 9 begins
**And** the checkpoint result is committed and reviewed by the parser-owner.

### Story 8.10: Implement Backlink Graph adjacency query API (added 2026-05-20 per LD-56 / FR-26)

As the **user**,
I want a query returning the Vault's `:ID:`-keyed Headlines as nodes and `[[id:...]]` / `[[wiki-link]]` references as edges, scoped to a subgraph,
So that FR-26 Graph View surface is queryable from any consumer (UI canvas, a11y list-view, CLI).

**Traces:** FR-26, LD-56, LD-13 (reuses `links` table).

**Acceptance Criteria:**

**Given** Story 8.6 Backlinks query API (shares the `links` table),
**When** `crates/orgsidian-index/src/query/graph.rs` exposes `pub fn adjacency(scope: GraphScope) -> Result<GraphData, IndexError>`,
**Then** `GraphData { nodes: Vec<NodeRef { id: HeadlineId, file: PathBuf, title: String }>, edges: Vec<Edge { src_id: HeadlineId, dst_id: HeadlineId, kind: EdgeKind }> }`
**And** `EdgeKind::{IdLink, WikiLink}` distinguishes `[[id:...]]` from `[[wiki-link]]` (typed-edge styling for v0.5+ is non-breaking — UI can ignore the distinction in v0.1)
**And** `GraphScope::{WholeVault, NeighborhoodOf(HeadlineId, depth: u8)}` covers the v0.1 surfaces; the `Tag(TagId)` and `FilePath(PathBuf)` variants are reserved in the enum (marked `#[non_exhaustive]`) for v0.5+ subgraph filtering
**And** the `IndexQuery` trait (Story 6.5 frozen baseline) exposes `graph::adjacency` — Epic 8 implements the signature already declared in the baseline
**And** the CLI command `orgsidian query graph <scope>` exposes the same API per LD-27, emitting JSON adjacency
**And** module carries `//! Implements FR-26 (Backlink Graph adjacency)` as the first doc-comment.

### Story 8.11: Implement Graph View canvas + a11y textual fallback (added 2026-05-20 per LD-56 / FR-26 / LD-58)

As the **user**,
I want a `/graph` route rendering my Vault's backlink graph as a force-directed canvas (with pan/zoom, click-to-Source, zoom-in labels) AND as a keyboard-reachable textual node list,
So that the *one object, three views* (outline + agenda + graph) wedge is live in v0.1 Alpha and the LD-58 keyboard-only happy-path scenario for Graph View has a non-canvas target to drive (FR-26 + NFR-9).

**Traces:** FR-26, LD-56, LD-58, LD-29 (route), UJ-6 adjacent.

**Acceptance Criteria:**

**Given** Story 8.10 (`query::graph::adjacency`) AND Story 1.17 (a11y gate) AND Story 1.7 license allowlist (verified clean against `react-force-graph-2d@1.29.1` + transitive deps),
**When** the Graph View is implemented,
**Then** `packages/shell-ui/package.json` pins `react-force-graph-2d` at exact `1.29.1` (MIT)
**And** `shell-ui/src/routes/_layout/graph.tsx` declares the `/graph` TanStack route per LD-29 with typed loader data invoking `commands.adjacency({ scope: { type: 'WholeVault' } })`
**And** `shell-ui/src/components/graph/GraphCanvas.tsx` renders the adjacency via `<ForceGraph2D graphData={...} nodeId="id" onNodeClick={n => router.navigate({ to: '/editor/$filePath/$headlineId', params: { filePath: n.file, headlineId: n.id } })} />`
**And** node labels are visible at zoom-in (per LD-56 follow-up — `nodeCanvasObject` custom draw acceptable if defaults insufficient)
**And** `shell-ui/src/components/graph/GraphNodeList.tsx` renders a keyboard-reachable textual list of nodes sorted by degree (descending), with alphabetical jump-to-letter, fulfilling the LD-58 a11y fallback requirement; toggle between canvas + list via View menu and `g l` chord (Plain Mode hides `g l` via `data-[mode=plain]:hidden`; the View-menu toggle is always reachable)
**And** **empty-state** when the Vault has zero `:ID:` properties shows the inline-coaching balloon (Story 6.6 / 11.4 — new coaching ID `GRAPH_EMPTY_INTRO`) pointing to `docs/user-guide/headline-ids.md` per workflow-over-syntax discipline
**And** perf is gated by `assert_no_perf_regression!("story-8.11-graph-5k-render", …)` initial baseline ≤2s for a synthetic 5000-node force-directed render on 2020+ baseline hardware, AND `assert_no_perf_regression!("story-8.11-graph-steady-frame", …)` baseline ≤500ms for steady-state frame after layout settle — per LD-56 budget
**And** the LD-58 keyboard-only Playwright scenario (Story 1.17) for "Graph View" tabs into `GraphNodeList`, presses Enter on the top-degree node, and asserts the `/editor/...` route lands at the expected Headline
**And** modules carry `//! Implements FR-26` doc-comment header.

### Story 8.12: Cross-webview Graph View nightly perf gate (added 2026-05-20 per LD-56)

As the **author / contributor**,
I want the Graph View `≤2s / 5k nodes` + `≤500ms steady-state frame` budgets verified nightly across macOS WebKit + Linux WebKitGTK + Windows WebView2,
So that the LD-56 perf headroom claim is empirically valid on every supported webview (catches WebKit-vs-Blink rendering divergence before it surfaces in user bug reports).

**Traces:** LD-56, LD-32, NFR-7 (cross-platform parity).

**Acceptance Criteria:**

**Given** Stories 8.11 + 1.12 (perf macro) + Story 1.8 nightly matrix,
**When** the cross-webview gate runs,
**Then** `.github/workflows/nightly.yml` adds a `graph-view-perf-matrix` job running on macOS-arm64 + Ubuntu-LTS + Windows-2022 (the platforms where Tauri webview matches the locked stack)
**And** the job spins up a synthetic 5000-node + 8000-edge fixture Vault and measures both perf assertions on each platform
**And** results are written to `tests/perf-baselines/cross-webview/graph-{platform}.json` with per-platform baselines (WebKit may differ from WebView2 — separate baselines, same ≤2s ceiling)
**And** the merge gate (Story 1.8) blocks if any platform regresses >20% above its baseline
**And** the dashboard at `docs/perf/cross-webview-trends.md` is regenerated on each successful nightly with the 14-day trend.

---

## Epic 9: Conflict-Safe Concurrent Editing (Full Merge Dialog)

### Story 9.1: Implement three-pane Merge Dialog UI

As the **user with unsaved changes when an external write occurs**,
I want a Merge Dialog with three panes (Yours / External / Merged) opening within 2 seconds of the conflict detection,
So that FR-16 full UX lands and replaces the Epic 5 block-save fallback.

**Acceptance Criteria:**

**Given** Epic 5 closed (Stories 5.3 + 5.5 in particular),
**When** the active `ConflictStrategy` is `ThreePaneMergeDialog` and a conflict is detected,
**Then** `shell-ui/src/components/merge/MergeDialog.tsx` opens a modal with three side-by-side panes: Yours (left, Dirty Buffer), External (right, on-disk), Merged (center, initialized to a 3-way merge result with `ancestor_hash` as base)
**And** the dialog uses custom focus management to navigate hunks via `Tab` / `Shift+Tab`
**And** dialog open latency is gated by `assert_no_perf_regression!("story-9.1-merge-dialog-open", …)` (Story 1.12) — initial baseline ≤2s from conflict detection event
**And** the implementing module carries `//! Implements FR-16 (full Merge Dialog strategy)` as the first doc-comment line, verified by `tests/traceability.rs`.

**Traces:** FR-16 (full), UJ-5.

**PRD Reconciliation Note:** PRD §2.4 UJ-5 references "Merge Dialog showing both versions side-by-side" — this is the editorial expectation under-specified. Architecture LD-7 + this story implement the **three-pane** standard (Yours / External / Merged) which is the industry standard for conflict resolution UIs. A PRD §2.4 annotation will be filed to align the wording with the implementation.

### Story 9.2: Implement hunk-level diff selection

As the **user resolving a merge conflict**,
I want each diff hunk in the three panes to be individually selectable (use-yours / use-external) with the Merged pane updating live,
So that I can resolve conflicts hunk-by-hunk without losing either side's work (FR-16 consequences).

**Acceptance Criteria:**

**Given** Story 9.1,
**When** hunks are detected via a diff algorithm (Myers or Histogram, choice documented),
**Then** each hunk shows side-by-side use-yours / use-external buttons
**And** clicking either updates the Merged pane live
**And** keyboard shortcuts `H` (use-yours) and `L` (use-external) on the focused hunk match the button actions.

### Story 9.3: Implement free-edit of Merged pane + atomic save + cancel preservation

As the **user resolving a merge conflict**,
I want to free-edit the Merged pane (typing in it, beyond per-hunk selection) and confirm with atomic save — or cancel without losing my Dirty Buffer,
So that complex merges have an escape hatch and cancelling is safe.

**Acceptance Criteria:**

**Given** Stories 9.1 + 9.2,
**When** the user types in the Merged pane,
**Then** the Merged pane content is editable as a CM6 buffer
**And** confirming via "Save Merge" invokes `commands.resolveMerge(path, mergedContent)` which atomic-writes (Story 3.1) and clears Dirty Buffer state
**And** cancelling via "Cancel" closes the dialog, preserves the Dirty Buffer, and leaves the file on disk untouched
**And** integration test in `crates/orgsidian-vault/tests/merge_atomicity.rs` exercises power-loss simulation during merge save and asserts no corruption.

### Story 9.4: Swap active `ConflictStrategy` to `ThreePaneMergeDialog` + retire `BlockWithWarning`

As the **user upgrading from v0.1 Alpha to v0.5 Beta**,
I want the active conflict strategy switched from `BlockWithWarning` to `ThreePaneMergeDialog`, with the fallback retired,
So that v0.5 Beta ships the polished FR-16 contract.

**Acceptance Criteria:**

**Given** Stories 9.1 + 9.2 + 9.3 + 5.3,
**When** the strategy is swapped,
**Then** `crates/orgsidian-vault/src/conflict.rs` registers `ThreePaneMergeDialog` as the default `ConflictStrategy` at app startup
**And** the `BlockWithWarning` strategy + UI banner code is removed from `shell-ui/src/components/editor/`
**And** `tests/conflict_strategy.rs` runs the parametrized suite from Story 5.3 against only `ThreePaneMergeDialog`
**And** the watcher state machine is unchanged (validates Party Mode P0 prediction — only the strategy swap, no rewrite).

### Story 9.5: Plugin API consistency checkpoint (preview LD-50)

As the **author / contributor**,
I want a second checkpoint verifying the Merge Dialog consumed `OrgsidianPlugin` hooks consistently,
So that LD-50 final review at Epic 12 has both Capture/Search and Merge data points.

**Acceptance Criteria:**

**Given** Stories 9.1-9.4,
**When** the checkpoint is performed,
**Then** `docs/plugin-api/v0.5-checkpoint-epic-9.md` lists any `Event` variants or `HookOutcome` semantics that surfaced during merge implementation (e.g., a hypothetical `MergeResolved` event)
**And** discrepancies with LD-26 are flagged for resolution at Epic 12.

---

## Epic 10: Project Report Export (Wow Demo)

### Story 10.1: Scaffold `orgsidian-report` crate with `typst-as-lib` integration

As the **author / contributor**,
I want a new `crates/orgsidian-report/` crate isolating PDF rendering deps from `core` and `cli` consumers,
So that LD-53 binary-size delta lands inside the LEAF crate per `cargo deny check graph`.

**Acceptance Criteria:**

**Given** Epic 1 + Story 1.7 (`cargo-deny` graph rule),
**When** the crate is scaffolded,
**Then** `crates/orgsidian-report/Cargo.toml` declares `typst@0.14`, `typst-pdf@0.14`, `typst-as-lib@0.15`, `serde`, `serde_json`
**And** `crates/orgsidian-report/src/lib.rs` declares the public surface `pub fn render_project_report_pdf(data: &ReportData) -> Result<Vec<u8>, ReportError>`
**And** `cargo deny check graph` confirms `orgsidian-cli` does NOT transitively depend on `orgsidian-report`
**And** module carries `//! Implements FR-14`.

### Story 10.2: Define `ReportData` struct + serde wiring

As the **author / contributor**,
I want a `ReportData` struct mirroring the core query API + deriving `Serialize` for the typst `sys.inputs` schema,
So that LD-53 data flow (`core` → `ReportData` → `serde_json::to_value` → typst) is type-safe.

**Acceptance Criteria:**

**Given** Story 10.1,
**When** `ReportData` is defined,
**Then** `crates/orgsidian-report/src/data.rs` declares `ReportData { scope: ReportScope, range: DateRange, todo_completions: Vec<TodoCompletion>, clock_totals: HashMap<HeadlineId, Duration>, linked_notes: Vec<LinkedNote>, milestones: Vec<Milestone> }`
**And** all fields derive `Serialize` via `serde`
**And** `crates/orgsidian-report/tests/data_roundtrip.rs` asserts `serde_json::to_value(&data)` produces the expected JSON shape consumed by typst's `sys.inputs`.

### Story 10.3: Implement default `.typ` template + `sys.inputs` schema docs

As the **user customizing my report**,
I want a default `orgsidian-report-default.typ` template bundled with Orgsidian and `docs/customization/report-templates.md` documenting the `sys.inputs` schema,
So that LD-53 OQ-6 customization surface is documented (closes the v0.5 deliverable per architecture).

**Acceptance Criteria:**

**Given** Story 10.2,
**When** the default template is committed,
**Then** `crates/orgsidian-report/templates/orgsidian-report-default.typ` is bundled via `include_str!` and consumes `sys.inputs` keys matching `ReportData` fields
**And** the template renders TODO completions grouped by week, total clock time, linked notes presented as Headline title + one-line excerpt grouped by source file, milestone status
**And** `docs/customization/report-templates.md` documents the `sys.inputs` schema, generated from the `ReportData` struct via a build-time script
**And** the docs include a "write your own template" walkthrough.

### Story 10.4: Bundle fonts (Inter + JetBrains Mono + Noto Sans subset)

As the **user generating a report in Italian / Spanish / Cyrillic-language vaults**,
I want bundled fonts covering Latin + Latin-Ext + Cyrillic with total payload ≤8 MB,
So that LD-53 v0.5 font payload target is met without external font dependencies.

**Acceptance Criteria:**

**Given** Story 10.3,
**When** the fonts are bundled,
**Then** `crates/orgsidian-report/fonts/{Inter-Variable.ttf, JetBrainsMono-Regular.ttf, NotoSans-Latin-Cyrillic.ttf}` are included via `include_bytes!`
**And** `crates/orgsidian-report/src/fonts.rs` exposes an `embedded_font_resolver()` consumed by `TypstEngine`
**And** total embedded font payload is ≤8 MB
**And** all fonts are OFL-licensed and the licenses ship in `crates/orgsidian-report/fonts/LICENSES.txt`
**And** v1.0 contingency for CJK + Arabic subsets is documented in `docs/plans/v1.0-font-rollout.md`.

### Story 10.5: Implement HTML renderer (parallel path)

As the **user wanting an HTML report for email or web**,
I want a parallel HTML output path with its own templater,
So that FR-14 HTML format ships alongside PDF (LD-53 scope).

**Acceptance Criteria:**

**Given** Story 10.2,
**When** the HTML renderer is implemented,
**Then** `crates/orgsidian-report/src/html_renderer.rs` exposes `pub fn render_project_report_html(data: &ReportData) -> Result<String, ReportError>`
**And** the chosen templater (`handlebars` vs `minijinja` vs `tera`) is recorded as an in-sprint micro-decision documented in `crates/orgsidian-report/README.md`
**And** the HTML output is printer-friendly (CSS print-media query; no clipping at A4 + Letter)
**And** unit test `tests/html_renderer.rs` asserts the rendered HTML contains expected sections (TODO completions, Clock totals, etc.).

### Story 10.6: Implement Report Export UI in Settings

As the **user shipping a client report**,
I want a Settings → Project Report screen with scope picker (file / Headline subtree / tag), date range picker, output format (PDF / HTML), and a "Generate" button,
So that the v0.5 wow demo is one click away from any vault (FR-14 + UJ-3).

**Acceptance Criteria:**

**Given** Stories 10.1 + 10.5,
**When** the UI is implemented,
**Then** `shell-ui/src/components/settings/ReportExport.tsx` renders the scope picker + date range picker + format radio + Generate button
**And** clicking Generate invokes `commands.generateReport(scope, range, format)` which calls the appropriate renderer and returns a byte buffer
**And** the byte buffer is saved via `tauri-plugin-dialog`'s save dialog
**And** report generation is gated by `assert_no_perf_regression!("story-10.6-report-typical-scope", …)` (Story 1.12) — initial baseline ≤5s for typical scope (50 headlines, 4 weeks of activity) per FR-14 NFR
**And** an Active Clock with no end-time is flagged explicitly in the generated report with `[microcopy: draft]` text "⚠ {Headline}: Clock running, no end-time recorded" per FR-14 consequences (UJ-3 critical edge case).

**Traces:** FR-14, UJ-3.

### Story 10.7: UJ-3 spine — Sofia ships a client report (Party Mode round 2 P0 — Sally)

As the **freelance consultant** wrapping up a 4-week client engagement,
I want a single end-to-end flow taking me from a project file's context → "Project Report" action → date-range + PDF picker → formatted PDF saved → ready to attach to an invoice email,
So that UJ-3 is testable as a coherent journey and the critical edge case (Active Clock with no end-time flagged explicitly in the PDF) is exercised as an integration assertion.

**Traces:** UJ-3 (spine), FR-14, FR-8.

**Acceptance Criteria:**

**Given** Stories 10.1-10.6 closed,
**When** the integration spine test runs,
**Then** `shell-ui/e2e/uj3-report-spine.spec.ts` (Playwright + Tauri WebDriver) executes the following scripted flow on a fixture Vault containing a 4-week project with ≥3 milestones, ≥10 clocked tasks, ≥5 linked notes, and one deliberately-open `CLOCK:` line with no end-time:
  1. Open the project file in the editor
  2. Invoke "Project Report" action from the context menu (or `Cmd/Ctrl+Shift+E` for "Export" — rebound from the previously-listed `Cmd/Ctrl+Shift+R` per the 2026-05-20 reconciliation, which freed `Cmd/Ctrl+Shift+R` for the org-canonical Refile chord per Story 11.9)
  3. Pick date range "last 4 weeks" + format "PDF"
  4. Click "Generate" → PDF byte buffer is produced within Story 10.6 perf budget
  5. PDF is saved to a target path via `tauri-plugin-dialog`
  6. Inspect the generated PDF (via `pdfium-render` or `pdf-extract` in the test harness): assert ≥1 page contains "Total: …", grouped TODO completions per week, milestone status section, and a **prominent warning** for the deliberately-open clock entry containing the literal `⚠` glyph and the Headline name
**And** the spine test runs on macOS-arm64 + Ubuntu-LTS per PR
**And** the test fails if the open-clock warning is absent or buried below the fold (rendered only on page 2+).

---

## Epic 11: Onboarding Completion & Coaching

### Story 11.1: Add Empty Starter Vault picker card + flow polish

*(Re-scoped 2026-05-20: was "Add Freelancer Starter Vault content + generator" — Freelancer promoted to Story 6.1 v0.1 Alpha per the UX spec lighthouse-persona commitment; this story is what's left of the FR-18 completion in v0.5 Beta, surfacing the explicit Empty card with onboarding coaching. Subsumes the prior Story 11.2 "Add Empty Starter Vault flow" — see Story 11.2 below for the no-op marker.)*

As the **first-time user with an existing `.org` folder**,
I want an explicit "Empty (use my own folder)" card on the first-launch Starter Vault picker — visually equal to the Personal GTD / Student / Freelancer cards, with onboarding coaching that confirms no files will be written into my existing folder,
So that experienced org-mode users have a peer first-launch path to designating their own vault, not just a secondary link (FR-18 v0.5 completion).

**Traces:** FR-18, UJ-4.

**Acceptance Criteria:**

**Given** Story 6.2 ships the v0.1 picker with three primary cards + a "Use my own folder" link,
**When** Story 11.1 lands in v0.5 Beta,
**Then** `shell-ui/src/components/onboarding/StarterVaultPicker.tsx` adds a fourth peer card "Empty (use my own folder)" alongside Personal GTD / Student / Freelancer
**And** selecting the Empty card prompts for an existing `.org` folder via `tauri-plugin-dialog`, then designates it via Story 3.6's `designateVault` flow
**And** no new files are written to the chosen folder (the no-content invariant from the prior Story 11.2 is preserved)
**And** the picker also shows a one-line confirmation message `[microcopy: draft] "Orgsidian will not write any new files into this folder."` so the user understands the safety contract
**And** the legacy "Use my own folder" secondary link from Story 6.2 is removed (the explicit card supersedes it)
**And** the app opens directly on the `/today` route with whatever the Vault contains
**And** the implementing modules carry `//! Implements FR-18 (Empty Starter Vault — v0.5 Beta completion of the FR-18 picker)`.

### Story 11.2: ~~Add Empty Starter Vault flow~~ — **subsumed into Story 11.1 (2026-05-20)**

*This story was subsumed into Story 11.1 during the 2026-05-20 reconciliation. The behavior previously described here (Empty option → `tauri-plugin-dialog` → `designateVault` → no new files → land on `/today`) is now an AC of the re-scoped Story 11.1, which combines the explicit picker card with the underlying flow. Story 11.2 is kept as a no-op marker so downstream artifacts referencing "Story 11.2" do not orphan, but no implementation work happens here. Refer all FR-18 Empty Starter Vault work to Story 11.1.*

**Traces:** FR-18 (see Story 11.1).

### Story 11.3: Implement Plain/Power Mode toggle with `data-[mode]` Tailwind selectors

As the **new user**,
I want a Plain Mode hiding advanced commands, properties drawers, and rarely-used keybindings — toggleable to Power Mode without restart,
So that my first-launch surface area is minimal (FR-20 + LD-29 visibility flip).

**Acceptance Criteria:**

**Given** Epic 1 closed,
**When** the toggle is implemented,
**Then** `<body data-mode="plain"|"power">` is driven by `stores/settingsStore.ts` via Zustand
**And** advanced controls use Tailwind selectors `data-[mode=plain]:hidden` to flip visibility
**And** advanced controls remain in the DOM at all times — only visibility flips (preserves keyboard-shortcut muscle memory: a "hidden" Power-only command remains reachable by its shortcut)
**And** Settings → General → "Plain Mode / Power Mode" exposes the toggle with the default being Plain Mode for new users
**And** mode switch does not require app restart.

### Story 11.4: Refactor hardcoded coaching balloons into registry-driven `CoachingSlot` API

As the **first-time user** staring at a Today Dashboard or empty Inbox for the first time and wondering what to try first,
I want a friendly voice telling me what to do next without forcing me to read a manual,
So that I don't close the app within 90 seconds because I didn't understand it.

**Traces:** FR-21 (full), UJ-4, refactors Story 6.6 hardcoded balloons.

**Acceptance Criteria:**

**Given** Stories 6.6 (hardcoded v0.1 balloons) + 11.3 (Plain/Power mode),
**When** the registry refactor lands,
**Then** `shell-ui/src/coaching/coachingRegistry.ts` maps coaching IDs to `{ content: string, condition: () => boolean }` covering at minimum: `UJ4_TODAY_INTRO` + `UJ4_CAPTURE_INTRO` (migrated from Story 6.6 hardcoded), plus `EMPTY_INBOX`, `NEVER_CLOCKED_IN`, `NEVER_SEARCHED`
**And** the Story 6.6 hardcoded balloons are **deleted** from the Today Dashboard and Inbox source — replaced by `<CoachingSlot id="UJ4_TODAY_INTRO" />` etc. (no duplicate rendering)
**And** existing `<Vault>/.orgsidian/coaching-dismissed.json` dismissals from Story 6.6 (keyed by `UJ4_TODAY_INTRO`, `UJ4_CAPTURE_INTRO`) are honored without re-prompting — migration test in `shell-ui/src/coaching/coachingRegistry.test.ts` exercises this
**And** `<CoachingSlot id="..." />` is the only API used in surfaces (no inline balloon markup remains)
**And** the implementing module carries `//! Implements FR-21` as the first doc-comment line, verified by `tests/traceability.rs`.

### Story 11.5: Persist coaching dismissals + reset action

As the **user**,
I want "Don't show again" to persist a coaching dismissal per-context, and a Settings → Coaching → "Show all coaching tips again" reset to clear all dismissals,
So that FR-21 is self-correcting.

**Acceptance Criteria:**

**Given** Story 11.4,
**When** the user clicks "Don't show again" on a coaching slot,
**Then** the dismissal is persisted via `tauri-plugin-store` at `<Vault>/.orgsidian/coaching-dismissed.json` per LD-40 per-Vault state
**And** the slot does not render again on subsequent renders
**And** Settings → Coaching → "Show all coaching tips again" clears the dismissal store
**And** `stores/coachingStore.ts` exposes the reset action.

### Story 11.6: Wire up command palette descriptions for discoverability

As the **user**,
I want command palette descriptions written for discoverability ("Capture a thought from anywhere" rather than "Quick Capture"),
So that searching for "thought" in the palette surfaces Quick Capture (FR-21 consequences).

**Acceptance Criteria:**

**Given** Story 8.5 + Story 11.4,
**When** command descriptions are audited,
**Then** every command registered in the palette has a `description: string` written for the user's mental model, not the implementation noun
**And** an end-to-end test in `shell-ui/e2e/palette-discoverability.spec.ts` asserts that typing "thought", "find", "track time", "report", "refile", "graph" surfaces the correct commands.

### Story 11.7: Implement Refile subtree extract/insert primitives (added 2026-05-20 per FR-25 / LD-57)

As the **author / contributor**,
I want round-trip-faithful `extract_subtree(file, headline_id)` and `insert_subtree(file, dest_outline_path, subtree)` primitives in `orgsidian-vault`,
So that Story 11.8's cross-file orchestrator has a tested foundation for moving a Headline + its children between files without whitespace drift (FR-25 v0.5 Beta foundation).

**Traces:** FR-25, LD-57, FR-2 (round-trip).

**Acceptance Criteria:**

**Given** Epic 2 closed (tree-sitter-org + semantic layer) AND Epic 3 closed (atomic-write),
**When** `crates/orgsidian-vault/src/refile.rs` is implemented,
**Then** `pub fn extract_subtree(path: &Path, id: HeadlineId) -> Result<Subtree, RefileError>` uses tree-sitter-org boundaries (heading-level + body extent up to next sibling/ancestor) to extract the full subtree (children inclusive, including LOGBOOK + PROPERTIES drawers)
**And** `pub fn insert_subtree(path: &Path, dest_outline_path: &OutlinePath, subtree: &Subtree) -> Result<(), RefileError>` writes the subtree into the destination file at the chosen outline path with heading-level adjusted to match the parent's depth + 1
**And** unit tests cover: multi-level subtrees (depth ≥3), subtrees with nested LOGBOOK + PROPERTIES drawers, subtrees containing recurring timestamps (preserved verbatim), subtrees at end-of-file (no trailing-newline drift), empty-body subtrees (heading only), subtrees with `:ID:` property (preserved verbatim — critical for Backlinks/Graph cross-refs to survive Refile)
**And** **round-trip property**: `extract(file, id) → insert(other_file, path, subtree) → extract(other_file, new_id)` yields a byte-identical subtree (modulo heading-level adjustment, which is documented)
**And** module carries `//! Implements FR-25 primitives (subtree extract + insert)` doc-comment.

### Story 11.8: Implement Refile cross-file orchestrator (added 2026-05-20 per FR-25 / LD-57)

As the **user with a thought captured in the Inbox**,
I want to move it to the right project file via a single action that's atomic from my perspective — either the Refile completes fully or neither file changes,
So that triage from Inbox to project is friction-free without risking partial-state corruption (FR-25 + LD-57).

**Traces:** FR-25, LD-57, LD-7 (cross-file Single Writer extension), LD-41 (failure catalog).

**Acceptance Criteria:**

**Given** Story 11.7 primitives, Story 3.1 atomic-write, Story 3.2 Dirty Buffer manager, Story 5.3 ConflictState (rich struct) AND Story 1.11 LD-41 failure-mode harness AND Story 1.12 perf macro,
**When** `crates/orgsidian-core/src/orchestrator/refile.rs` is implemented,
**Then** the orchestrator implements the LD-57 **sequence-with-`.bak`-restore** pattern: (a) precondition check — both source and destination files must be clean (no Dirty Buffer); if either dirty, return `RefileError::SaveFirstRequired` so the UI prompts the user; (b) snapshot destination to `<dest>.bak.<pid>.<ts>` in the same directory (the snapshot lives inside the Vault; LD-41 startup scan extended in Story 1.11 to clean `*.bak.*` orphans from dead PIDs); (c) atomic-write destination with the subtree inserted via Story 11.7; (d) atomic-write source with the subtree removed via Story 11.7; (e) on step-d success, delete the `.bak` and emit watcher-suppress tokens for both files; (f) on step-d failure, restore destination from `.bak`, surface `RefileError::Reverted { reason }`, both files end at pre-Refile byte-state
**And** **fault-injection test** in `crates/orgsidian-core/tests/failure_modes.rs` (Story 1.11 harness): inject `ENOSPC` on the source atomic-write after destination commit; assert destination is restored from `.bak` and **both files are byte-identical to their pre-Refile state**; assert the user-facing error is `RefileError::Reverted` with a specific reason; assert no `.bak` orphan remains
**And** **watcher-suppress integrity**: the LD-7 Single Writer Rule data-flow (Story 5.4) extends to emit suppress tokens for both files for the duration of the Refile operation; the Merge Dialog (Story 9.1) is NOT triggered by Orgsidian's own writes during Refile
**And** the LD-41 "Refile partial completion" row (added 2026-05-20 to architecture.md) flips from placeholder to live: Story 1.11's `tests/failure_modes/refile_partial.rs` becomes a passing test (no longer `#[ignore]`)
**And** perf is gated by `assert_no_perf_regression!("story-11.8-refile-roundtrip", …)` initial baseline ≤200ms end-to-end on a 1k-file Vault with a typical (≤10-Headline) subtree
**And** the index is re-synced after Refile via `notify-rs` watcher event (both files emit modify events that the watcher handles per Story 5.1 — Refile does not need a separate re-index code path)
**And** module carries `//! Implements FR-25 (cross-file orchestrator per LD-57)` doc-comment.

### Story 11.9: Implement Refile Target Picker UI (added 2026-05-20 per FR-25)

As the **user** triaging an Inbox capture,
I want `Cmd/Ctrl+Shift+R` to open a fast picker — fuzzy-match on file paths AND on outline paths within the chosen file — so I can move the current Headline to its right home in two or three keystrokes,
So that Refile becomes the keyboard-first triage primitive UJ-4 promises (FR-25 + UX spec Effortless Interactions).

**Traces:** FR-25, UJ-4 adjacent (inbox triage), NFR-9 (keyboard-first).

**Acceptance Criteria:**

**Given** Story 11.8 orchestrator + Story 8.5 Command Palette infrastructure (reuses `cmdk` patterns),
**When** the user invokes Refile,
**Then** `shell-ui/src/components/refile/RefileTargetPicker.tsx` is the surface, opened by `Cmd/Ctrl+Shift+R` (the org-canonical Refile chord — Project Report was rebound to `Cmd/Ctrl+Shift+E` per Story 10.7 update)
**And** the picker is **two-stage**: stage 1 fuzzy-matches on file paths (excluding the source file itself); stage 2, after a file is selected, fuzzy-matches on outline paths within that file (showing `Headline / sub-Headline / sub-sub-Headline` breadcrumbs)
**And** keyboard navigation: arrow keys to move selection, Enter to advance/commit, Esc to cancel; mouse fallback works but the keyboard-only path is the primary tested flow (one of the Story 1.17 LD-58 `@a11y` Playwright scenarios for the Editor surface — Editor + Refile keyboard sequence)
**And** confirming the Refile invokes `commands.refileHeadline({ srcPath, srcHeadlineId, dstPath, dstOutlinePath })` which calls Story 11.8's orchestrator
**And** if Story 11.8 returns `RefileError::SaveFirstRequired`, the picker shows a modal "Save the source/destination file first?" with options Save-and-Refile / Cancel — no silent failures
**And** post-Refile UI: the editor view follows the moved subtree to its new location (`/editor/$dstPath/$headlineId` route) so the user sees their work landed; a toast confirms `[microcopy: draft] "Moved to <breadcrumb>"`
**And** keybinding remapping (Story 12.3) respects the Cmd/Ctrl+Shift+R default and allows reassignment with conflict detection
**And** module carries `//! Implements FR-25 (Refile target picker UI)` doc-comment.

---

## Epic 12: v0.5 Beta Release — Customization, Unlinked References & Plugin Surface Lock

### Story 12.0: Implement Backlinks Unlinked References sub-panel (added 2026-05-20 per FR-13 extension)

As the **user reading a note**,
I want the Backlinks sidebar to also surface **unlinked mentions** — places where the current Headline's title appears in another file's body without an explicit `id:` or `[[wiki-link]]` reference — so I can promote those to formal links,
So that the knowledge-graph stitching effort is incremental and discoverable rather than a manual audit (FR-13 v0.5+ extension per UX spec Tier 2 Roam pattern).

**Traces:** FR-13 (extension), UX spec Tier 2 Roam pattern.

**Acceptance Criteria:**

**Given** Story 8.4 FTS5 search + Story 8.6 Backlinks query API + Story 6.5 frozen `IndexQuery` baseline (which already declared the `unlinked_mentions` signature),
**When** Story 12.0 ships,
**Then** `crates/orgsidian-index/src/query/unlinked_references.rs` implements the `unlinked_mentions(headline_id: HeadlineId) -> Result<Vec<UnlinkedMention>, IndexError>` body: FTS5 full-text query for the current Headline's title (configurable whole-word vs substring; default whole-word) outer-joined against the `links` table, excluding the source Headline itself and any Headline that already has a formal link to it
**And** `UnlinkedMention { file: PathBuf, headline_id: HeadlineId, headline_title: String, context_snippet: String }` returns one entry per mention, deduplicating by linking Headline (one entry per Headline that mentions, not one per textual occurrence)
**And** `shell-ui/src/components/org/BacklinksPanel.tsx` adds two collapsible sub-tabs: **Linked** (existing FR-13 v0.1 surface) and **Unlinked References** (Story 12.0 v0.5+) — Linked is the default-open sub-tab
**And** each unlinked mention has a **"Promote to link"** action that inserts a `[[wiki-link]]` at the mention site (atomic-write per LD-8; respects Single Writer Rule)
**And** perf is gated by `assert_no_perf_regression!("story-12.0-unlinked-mentions", …)` initial baseline ≤100ms on a 1k-file Vault (same FR-13 NFR ceiling as Linked Backlinks)
**And** the CLI command `orgsidian query unlinked-mentions <headline-id>` exposes the same API per LD-27
**And** module carries `//! Implements FR-13 extension (unlinked references — v0.5+)` doc-comment.

### Story 12.1: Implement user CSS file loader

As the **user wanting a custom theme**,
I want to point Settings → Appearance → Custom CSS at a file path on disk, with invalid CSS falling back to default with a warning rather than crashing the app,
So that FR-22 CSS customization is robust.

**Acceptance Criteria:**

**Given** Story 6.6 (default themes),
**When** the user picks a CSS file,
**Then** `commands.setUserCssPath(path)` validates the file is readable and parseable
**And** valid CSS is loaded after the bundle via `<link rel="stylesheet" href="file://...">` injection (CSP `style-src 'self' 'unsafe-inline' file://*` per LD-18)
**And** invalid CSS triggers a Settings banner: "Custom CSS at {path} could not be loaded — falling back to default theme. {error message}"
**And** the path is persisted at `<Vault>/.orgsidian/theme.json` per LD-40
**And** theme switching (override → default → override) is instant (no app restart).

### Story 12.2: Implement `tokens.css` snapshot test (LD-51)

As the **author / contributor**,
I want a Vitest snapshot test at `shell-ui/src/themes/tokens.test.ts` extracting the set of `--org-*` variables from `tokens.css` and comparing against a committed snapshot,
So that LD-51 public theme API contract is locked from v0.5 Beta onward.

**Acceptance Criteria:**

**Given** Story 6.6 + Story 12.1,
**When** the snapshot test is committed,
**Then** `shell-ui/src/themes/tokens.test.ts` parses `tokens.css`, extracts the set of `--org-*` variable names, sorts them, and compares against `shell-ui/src/themes/__snapshots__/tokens.snap`
**And** any rename, removal, or addition of an existing variable fails the test
**And** acceptance requires snapshot update + a CHANGELOG entry under "Theme API"
**And** the naming convention is enforced: semantic granularity (`--org-headline-h1-fg`, `--org-accent-todo`), never structural (`--org-color-blue-500`)
**And** per-PR CI runs this test.

### Story 12.3: Implement keybinding remapping UI in Settings

As the **user with strong opinions about my chord set**,
I want Settings → Keybindings → "Remap action" to override any documented action with a different chord, with conflict detection warning when an assigned chord conflicts with an existing binding,
So that FR-23 customization is friction-free.

**Acceptance Criteria:**

**Given** Story 4.6 (default keymap),
**When** the user remaps an action,
**Then** `shell-ui/src/components/settings/KeybindingEditor.tsx` shows the current chord, accepts new chord input via a "Press the new chord" capture, and saves on confirm
**And** conflict detection warns: "{chord} is already bound to {action}. Reassign anyway?"
**And** remappings persist per-Vault at `<Vault>/.orgsidian/keybindings.json` per LD-40
**And** module carries `//! Implements FR-23`.

### Story 12.4: [MANUAL-GATE] Conduct LD-50 plugin event surface review + commit sign-off doc

As the **author / contributor**,
I want a formal review of every `Event` variant + hook method signature + `HookOutcome` semantics added during Epics 1-11, with `docs/plugin-api/v1.0-surface-review.md` committed before v0.5 → v1.0 transition,
So that LD-50 gate is honored and v1.5+ public publication is unblocked.

**Traces:** LD-50, FR-24.

**Acceptance Criteria (artifact-based per Party Mode round 2 P0 — Murat; manual qualitative judgment is human-in-the-loop):**

**Given** Stories 8.9 + 9.5 checkpoints + completed v0.5 Beta plugin set,
**When** the manual review is performed by the parser-owner role,
**Then** `docs/plugin-api/v1.0-surface-review.md` exists, is non-empty, and contains the following **lint-checkable structural requirements** (verified by `scripts/check-surface-review.sh` as a CI gate):
  - A section per `Event` variant currently defined in `orgsidian-plugin-api::Event` (one heading per variant, lint asserts variant count matches)
  - A section per hook method currently defined on `OrgsidianPlugin` trait (lint asserts method count matches)
  - A `HookOutcome` semantics section
  - A final `## Signed-off-by` line containing at least one signoff in the format `parser-owner @username (YYYY-MM-DD)`
**And** any `Event` or hook surface changes that arise from the review land as separate PRs **before** the `v0.5.0` release tag — release pipeline refuses to tag if `docs/plugin-api/v1.0-surface-review.md` lacks the lint-asserted structure
**And** the qualitative content (semantic correctness, granularity judgments) is reviewed by the parser-owner and other reviewers via PR review — explicitly not automatable.

### Story 12.5: Publish v0.5 Beta release + announcement

As the **author**,
I want v0.5 Beta packaged across macOS + Linux with the full feature set and a Beta announcement going out to the 10+ Alpha adopters + r/orgmode + HN,
So that SM-2 (author daily-driving 4 weeks + 100 beta testers) becomes measurable.

**Acceptance Criteria:**

**Given** Epics 7-12 closed,
**When** `v0.5.0-beta.1` is tagged,
**Then** `.github/workflows/release.yml` builds and signs macOS DMG + Linux AppImage
**And** the release notes at `docs/announcements/v0.5-beta.md` highlight FR-14 Project Report (wow demo), FR-12 search, FR-13 backlinks, FR-16 full Merge Dialog, FR-22 CSS customization
**And** the announcement is drafted (timing/posting at author's discretion)
**And** `docs/user-guide/` is comprehensive enough for new beta testers to onboard without author 1-on-1.

---

## Epic 13: v1.0 — Cross-Platform Launch & Tutorial

### Story 13.1: Implement Windows MSI packaging + code signing

As the **Windows adopter**,
I want a signed `.msi` installer downloadable from GitHub Releases,
So that I can install Orgsidian on Windows 11 without SmartScreen warnings (NFR-8 + LD-19 + LD-34).

**Acceptance Criteria:**

**Given** Stories 6.7 + 6.8 (macOS+Linux pipeline),
**When** Windows packaging is added,
**Then** `.github/workflows/release.yml` builds the Windows-x86_64 MSI via Tauri bundler on a Windows runner
**And** the MSI is signed with the Windows code-signing certificate (key stored as GitHub Actions secret; EV upgrade evaluated)
**And** the artifact is uploaded to the GitHub Release page
**And** the `nightly` CI matrix Windows job is upgraded from "compiles" to "runs golden-path smoke test" (open vault, edit, save, search, close) per Murat P1 (Windows risk surfaced earlier).

### Story 13.2: Wire `tauri-plugin-updater` stable channel across all 3 platforms

As the **user wanting bug fixes without manual reinstall**,
I want Orgsidian to check for updates on launch (configurable / disable-able) and apply them in-place via `tauri-plugin-updater`,
So that LD-20 auto-update mechanism is operational at v1.0 launch.

**Acceptance Criteria:**

**Given** Stories 6.7 + 6.8 + 13.1,
**When** the updater is configured,
**Then** the Tauri key pair is generated (private signs releases, public embedded for verification)
**And** the release pipeline signs each `v*` release artifact with the private key
**And** the app checks the updater endpoint `https://updates.orgsidian.app` on launch (CSP `connect-src 'self' https://updates.orgsidian.app` per LD-18)
**And** Settings → General → "Check for updates" toggles the check (default ON)
**And** an available update triggers a non-modal banner: "Orgsidian v{X} is available. [Install on restart]"
**And** the updater works on macOS DMG + Linux AppImage + Windows MSI.

### Story 13.3: Implement Interactive Tutorial 10-minute workflow cycle

As the **first-time user**,
I want an Interactive Tutorial launchable from a "Get started" menu item or first-launch prompt, walking me through one full workflow cycle in 10 minutes (capture → triage → schedule → agenda → clock in/out → one-line report),
So that FR-19 onboarding teaches the workflow.

**Acceptance Criteria:**

**Given** Epics 6-11 closed,
**When** the tutorial is implemented,
**Then** `shell-ui/src/components/onboarding/Tutorial.tsx` renders a step-by-step guided experience with progress indicator
**And** each step prompts the user to perform a real action (capture a thought, triage to a project, set a Schedule, see it in Agenda, clock in then out, generate a report) and detects completion automatically
**And** tutorial completion is tracked locally (no telemetry) at `<Vault>/.orgsidian/tutorial-progress.json`
**And** tutorial is re-launchable from Settings → Help → "Run interactive tutorial"
**And** estimated time-to-completion is ~10 minutes (validated by manual run; documented).

### Story 13.4: Implement Clock UX polish (persistent status bar + timer notifications)

As the **user tracking time daily**,
I want a persistent toggleable status bar showing the Active Clock + a refined timer notification when I clock out + a clock-time editing affordance from the status bar,
So that FR-8 daily-driver UX matches the depth of the planning surface (PRD §6.2 phasing — polish moves to v1.0).

**Acceptance Criteria:**

**Given** Story 7.6 (functional Clock),
**When** the polish ships,
**Then** `shell-ui/src/components/clock/ActiveClockStatusBar.tsx` renders at the bottom of the main window, showing the active Headline + elapsed time + click-to-edit
**And** Settings → Clock → "Show status bar" toggles the bar (default ON; persists per LD-40)
**And** clocking out triggers a system notification "Clocked out of {Headline} — {HH:MM} recorded"
**And** clicking the status bar opens `ClockEditor` for the active entry (Story 7.8 reused).

### Story 13.5: Graduate a11y from happy-path to representative-coverage (v1.0 narrowing per 2026-05-20)

*(Narrowed 2026-05-20: Story 1.17 — added in the post-PRD-2026-05-20 reconciliation — now ships the LD-58 hard CI gates from v0.1 Alpha (axe-core + contrast-matrix + 6 happy-path keyboard scenarios). The "deferred to v1.5+" wording that previously framed this story is no longer accurate. Story 13.5 narrows to the v1.0 graduation work: expanding happy-path to representative-coverage, focus-ring snapshot tests, qualitative sign-off. Full screen-reader certification (assistive-tech audit) remains v1.5+.)*

As the **screen-reader-using or keyboard-only user**,
I want the per-PR a11y CI gates (already live from v0.1 per Story 1.17) expanded at v1.0 to cover representative scenarios — not just happy-path — across every primary surface, with visible focus rings and Tab-order snapshots verified,
So that the v1.0 a11y posture matches the launch credibility bar (NFR-9 v1.0 graduation; full assistive-tech certification still a v1.5+ commitment).

**Traces:** NFR-9 (graduation), LD-58 (extension), Story 1.17 (foundation).

**Acceptance Criteria:**

**Given** Story 1.17 LD-58 hard gates have been green from v0.1 through v0.5 AND all v1.0 UI epics closed,
**When** the v1.0 a11y graduation work is performed,
**Then** `shell-ui/e2e/a11y/` expands from the 6 happy-path scenarios (Story 1.17) to **representative-coverage** per surface: each primary surface (Today Dashboard, Agenda, Editor, Quick Capture, Settings, Merge Dialog, Graph View, **Refile Picker**, Command Palette) gets 3-5 keyboard-only scenarios exercising distinct interaction paths
**And** focus rings are verified visible on all interactive elements via Playwright `getComputedStyle(focusedElement).outline` assertions on the expanded scenarios
**And** Tab order matches visual flow on each surface — Playwright snapshot test of focus sequence vs DOM order; intentional reorderings (skip-link to main, focus-trap in Merge Dialog 3-pane) are explicitly documented
**And** the axe-core gate (already per-PR via Story 1.17) is unchanged — Story 13.5 does NOT raise the gate to best-practice tier; that remains a v1.5+ evaluation
**And** known limitations (full screen-reader certification deferred to v1.5+; assistive-tech matrix NVDA + JAWS + VoiceOver pending) are documented in `docs/user-guide/accessibility.md`
**And** the manual qualitative judgment ("does focus order feel right with a screen reader?") is performed by a human reviewer and recorded in `docs/user-guide/accessibility.md` § Sign-off (Party Mode round 2 — Murat: automated gates handle the objective WCAG criteria; subjective experience requires human review).

### Story 13.6: Build comprehensive `docs/user-guide/` site

As the **user discovering features**,
I want a navigable user guide covering all v1.0 capabilities (Editor Modes, Agenda, Capture, Search, Backlinks, Project Report, Themes, Keybindings, Starter Vaults, Merge Dialog),
So that I don't need to ask the author every question.

**Acceptance Criteria:**

**Given** all feature epics closed,
**When** the docs site is built,
**Then** `docs/user-guide/` contains pages for every feature with screenshots + keyboard shortcuts + common workflows
**And** `docs/README.md` provides a "start here" index distinguishing user-guide / plugin-api / architecture references
**And** the docs are renderable as static HTML (toolchain choice — mdbook or Docusaurus — recorded as in-sprint micro-decision; nice-to-have gap from architecture)
**And** a link from the in-app Help menu opens the published docs site.

### Story 13.7: Publish v1.0 + coordinated launch announcement

As the **author**,
I want v1.0 packaged across macOS + Linux + Windows with auto-update online, comprehensive docs, and a coordinated announcement (HN + ProductHunt + r/orgmode + 1-2 productivity newsletters),
So that SM-3 (1000 downloads in 30 days + newsletter coverage + 3 external PRs in 60 days) becomes measurable.

**Acceptance Criteria:**

**Given** Stories 13.1-13.6 closed,
**When** `v1.0.0` is tagged,
**Then** the release pipeline builds and signs macOS DMG + Linux AppImage + Windows MSI
**And** the release notes at `docs/announcements/v1.0.md` highlight cross-platform parity, the full feature set, the polished workflow
**And** the HN + ProductHunt + r/orgmode posts are drafted with launch-day timing coordinated
**And** at least 1 newsletter pitch is sent (timing/posting at author's discretion)
**And** the `examples/plugins/{hello-world, agenda-exporter}/` skeletons are ready for v1.5+ external plugin authors (scaffolded since Epic 1 per architecture).
