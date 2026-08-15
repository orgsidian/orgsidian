# Epic 4 Context: Editor Surface & Org-mode Awareness

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Epic 4 builds Orgsidian's editor surface: a CodeMirror 6 host embedded in the React 19 webview that renders `.org` files with org-mode awareness while keeping the underlying buffer as byte-faithful source text. It delivers three Editor Modes (Raw, Pseudo-WYSIWYG, Split) with persistent per-file preference, inline CM6 decorations/widgets for headings, TODO badges, tag pills, timestamp dates, checkboxes and clickable links, cross-platform default keybindings plus an opt-in Emacs mode, and a Schedule/Deadline date picker. This is the primary editing experience and the product's positioning wedge (show `.org` source, don't hide it). Pseudo-WYSIWYG rendering must never break source-position fidelity or round-trip preservation, and CM6 decorations are the most likely memory-leak source — so the nightly memory soak gate is activated here.

## Stories

- Story 4.1: Wire CodeMirror 6 host with StrictMode-safe lifecycle
- Story 4.2: Implement Raw editor mode
- Story 4.3a: Heading hierarchy decorations
- Story 4.3b: TODO state pill badges (click-to-cycle)
- Story 4.3c: Tag pill labels
- Story 4.3d: Timestamps as human-readable dates with hover-for-source
- Story 4.3e: Checkbox toggle widget (source-mutating click)
- Story 4.3f: Link rendering as clickable underlined text
- Story 4.3g: Source-position fidelity (cursor, copy-paste, find/replace)
- Story 4.4: Implement Split editor mode
- Story 4.5: Implement Editor Mode switcher UI
- Story 4.6: Implement cross-platform default keybindings
- Story 4.7: Implement optional Emacs keybindings mode
- Story 4.8: Implement Schedule/Deadline date picker
- Story 4.9: Activate nightly memory soak gate

## Requirements & Constraints

- **Mode switching (FR-3):** three modes — Raw, Pseudo-WYSIWYG (default), Split. Per-file choice persists across restarts. Mode switch must complete under 200ms on a 5,000-line file.
- **Pseudo-WYSIWYG rendering (FR-4):** headings with hierarchical font sizes, TODO-state badges, tag pill labels, timestamps as readable dates, checkbox widgets, clickable links — while the buffer stays source `.org` text. Cursor placement, copy/paste, and find/replace operate on source character offsets, not rendered positions.
- **Keybindings (FR-5):** cross-platform defaults (Cmd on macOS, Ctrl on Linux/Windows) covering the daily org actions (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out), plus an opt-in Emacs keybindings mode, plus an in-app keybinding reference panel.
- **Schedule/Deadline editing (FR-9):** add/modify/remove a Scheduled timestamp or Deadline on the current Headline via shortcut or context menu, with a date picker for fast entry and raw timestamp typing in Raw mode. Recurring timestamps (e.g. `<2026-05-19 Mon +1w>`) must be preserved on round-trip and respected by Agenda.
- **Round-trip fidelity is the source-of-truth contract:** any file edited/saved without user-visible changes must remain byte-identical (FR-2 CI gate). Decorations and widgets must never mutate source except through explicit user actions.
- **Performance budgets:** typing latency under 30ms; opening a 5,000-line file renders the first screen under 300ms.
- **Memory soak gate (NFR-21 / LD-43), activated in this epic:** nightly 12-hour scripted session (200 buffer open/close cycles, 50 plugin re-init cycles, 1000 agenda queries); RSS sampled every 30 min; fails if drift exceeds 10% over 11 hours (minute 60 → 720); a failing soak blocks all merges to `main`.

## Technical Decisions

- **Editor stack (LD-6):** CodeMirror 6 (`@codemirror/state`, `view`, `commands`, `language`, `search`, latest 6.x). Pseudo-WYSIWYG via CM6 decorators/widgets. Widget toggling between `Decoration.replace` ↔ `Decoration.widget` is a known sharp edge and must be exercised by tests. Mandatory recipes: `WidgetType.eq()` shallow-equal on widget props (compare by source range), `Transaction.userEvent` tag on every widget-triggered change, never call `view.dispatch` inside `update()` while `view.composing` is true, and `widget.ignoreEvent() === false` for interactive widgets.
- **React 19 lifecycle:** `EditorView` created in a `useEffect` with idempotent cleanup (`return () => view.destroy()`) so StrictMode double-mount does not leak a view. Components accept `ref` as a regular prop — no `forwardRef` (React 19 ref-as-prop). Editor tests use Vitest + happy-dom (required for CM6 `getComputedStyle`).
- **State ownership boundary:** CM6 owns the buffer for the open file. CM6 editor state is NEVER duplicated into Zustand and never persisted separately from the `.org` file. Split-mode panes share one `EditorState`. `viewStore.ts` (Zustand) holds only current Editor Mode and sidebar/UI state.
- **File layout:** editor lives under `shell-ui/src/components/editor/` (host `Editor.tsx`, `ModeSwitcher.tsx`, `decorations/` as CM6 ViewPlugins, `keybindings/default.ts` + `keybindings/emacs.ts`). Org UI Kit widgets under `shell-ui/src/components/org/` (day-1 mandatory: `TodoStateCycler`, `TagPillEditor`, `OrgDatePicker`, `PropertyDrawer`, `ClockEditor`, `HeadlineRenderer`, `ScheduleDeadlineBadge`).
- **Persistence (LD-40):** per-file editor preferences via `tauri-plugin-store` at `<Vault>/.orgsidian/editor-prefs.json`.
- **Platform detection (LD-5 stack):** `tauri-plugin-os` selects Cmd vs Ctrl for the default keymap.
- **Timestamp/Schedule backend:** date-shortcut parsing is a pure-Rust function with its own unit tests, independent of UI; the semantic timestamp layer lives in `orgsidian-parser/src/semantic/timestamp.rs`; writes go through `commands.setScheduled(headlineId, timestamp)` (typed `tauri-specta` client — never raw `invoke`).
- **Styling contract:** all colors via `--org-*` CSS tokens (e.g. `--org-accent-{todo,next,done,waiting}`, `--org-accent-tag`, `--org-selection-bg`, `--org-cursor`, `--org-match-bg`). Editor face font is IBM Plex Mono (embedded). Tailwind utilities first; extract to `org-*` classes only after 3+ repetitions.
- **Traceability:** each module implementing an FR carries `//! Implements FR-NN` as its first doc-comment line, verified by `tests/traceability.rs`.
- **Perf regression:** source-fidelity and other hot paths guarded via `assert_no_perf_regression!` (from Story 1.12) — operations must not regress beyond the stated threshold (~20% for source-fidelity ops).
- **Plugin surface consistency (FR-24 / LD-26):** editor actions that fire events (e.g. TODO cycle, save) route through the same internal plugin hook/event surface used everywhere; no parallel private paths.

## UX & Interaction Patterns

- **Default landing state:** Pseudo-WYSIWYG + Plain UI Mode + native keybindings + light theme. These defaults are absolute at cold-start, not session-inherited (window geometry may persist; semantic state resets).
- **Keyboard idiom:** the lighthouse persona is CLI-confident but non-Emacs, so native cross-platform defaults are the default and Emacs is strictly opt-in. Every action also reachable via the Command Palette (`Cmd/Ctrl+K`) with its shortcut hint shown inline — discovery-by-reading, no chord memorization required. The Emacs default keymap (Emacs precedence when active) resolves conflicts with native chords.
- **Inline decorations behavior:** TODO keywords render as click-to-cycle colored pills; tags as pills with colon delimiters hidden but preserved in source; timestamps as locale-formatted dates with a hover tooltip (>300ms) revealing raw source, active `<…>` vs inactive `[…]` visually distinct; checkboxes toggle `- [ ]` ↔ `- [X]` on click; link brackets `[[…]]` hidden unless the cursor is on that line.
- **Schedule/Deadline picker:** one keystroke opens an inline `OrgDatePicker` with a calendar + time picker and natural-language / relative shortcuts (`today`, `+1d`, `+1w`, `next monday`); Enter commits, Esc cancels (Fantastical-style pattern).
- **Filesystem-trust guarantee:** a TODO cycle or any edit writes byte-perfect `.org` to disk with no proprietary metadata — the change must be verifiable in `cat`/`vim`/Emacs. This round-trip fidelity is treated as the UX trust contract, not merely a tech NFR.
- **Accessibility (v0.1 commitment):** keyboard navigation completeness, visible focus, WCAG 2.1 AA contrast across the syntax-token × theme × editor-mode matrix (DONE-muted is the one documented exception), and axe-core zero critical/serious violations.
- Note: the epics file specifies the mode-switch chord as `Cmd/Ctrl+Alt+M`; the UX spec references `Cmd/Ctrl+Shift+M` for the same action — reconcile before wiring.

## Cross-Story Dependencies

- **Upstream:** Epic 4 depends on Epic 2 (parser + round-trip AST — TODO states, timestamps, links, checkboxes) and Epic 3 (Vault + SQLite index + `commands.openFile`) both being closed.
- **Internal ordering:** 4.1 (host) → 4.2 (Raw) → 4.3a–4.3f (decorations, parallelizable) → 4.3g (source-position fidelity, depends on all 4.3 decorations) → 4.4 (Split, needs 4.2+4.3) → 4.5 (mode switcher, needs 4.2+4.3+4.4) → 4.6 (default keys) → 4.7 (Emacs, needs 4.6). 4.8 (date picker) depends on 4.3d semantic timestamp rendering. 4.9 (soak gate) depends on 4.3 + 4.4 existing to have something to soak.
- **Downstream consumers:** Epic 6 (v0.1 Alpha release) requires Epic 4 closed; Epic 7 (Today Dashboard / time tracking) and Epic 8 (Capture/Search/Backlinks/Graph) both build on the closed editor surface. Link clicks emit `LinkClicked { target, kind }` consumed by the navigation layer (Epic 8 backlinks / click-to-source).
