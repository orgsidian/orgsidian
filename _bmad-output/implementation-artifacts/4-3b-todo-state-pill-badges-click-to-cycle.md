---
title: 'TODO state pill badges (click-to-cycle)'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '73c8e28'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 shipped Raw mode plus the mode→extension boundary (`editorMode.ts`), where the Pseudo-WYSIWYG decoration layer is currently empty. Epic 4 needs the first inline widget of that layer: in Pseudo-WYSIWYG (and Split) mode a headline's TODO-state keyword should render as a colored, clickable pill that advances the task state without typing (FR-4, UJ-1).

**Approach:** Add a CM6 decoration `ViewPlugin` (`editor/decorations/todoBadges.ts`) that scans visible headlines and `Decoration.replace`s each recognized TODO-state keyword with a `TodoStateCycler` widget (Org UI Kit, `components/org/TodoStateCycler.ts`). Clicking the pill routes through one shared command, `cycleTodoState`, which dispatches a single `Transaction` tagged `userEvent="input.cycle-todo"` that replaces the keyword's exact source range with the next state in the resolved `#+TODO:` sequence. Wire the extension into `editorMode.ts`'s `pseudoWysiwygDecorations()` (one import + one array entry). Colors resolve through `--org-accent-{todo,next,done,waiting}` tokens.

## Boundaries & Constraints

**Always:**
- Widget is `Decoration.replace` over the keyword's exact source range; the buffer stays byte-faithful (FR-2 round-trip contract) — the only mutation is the explicit click.
- The cycle is ONE atomic transaction tagged `userEvent="input.cycle-todo"`; `cycleTodoState` is the single shared path both the widget and future keybindings/command-palette use (FR-24 / LD-26 plugin-surface consistency) — no private mutation route.
- LD-6 widget recipes: `WidgetType.eq()` shallow-equal on the source range (+ keyword + next state); `ignoreEvent() === false` for the interactive pill; never dispatch while `view.composing` is true.
- Colors via `--org-accent-{todo,next,done,waiting}` tokens; module carries `// Implements FR-4` as its first doc-comment line (4.2 TS traceability convention).
- Keep Story 4.2 wiring untouched beyond the minimal `editorMode.ts` edit (conflict-minimization with the parallel 4.3a/4.3c–4.3f decoration stories).

**Ask First:**
- Declaring the `--org-accent-*` tokens in `tokens.css` (would enter the LD-58 contrast gate and require WCAG-passing values + `@pair-role`/`@pair-bg` annotations). This story CONSUMES the tokens with fallbacks to existing FR-22 tokens; the palette declarations belong to Story 6.7, exactly as Story 4.2 deferred.

**Never:**
- Cursor-reveal of the raw keyword (that reveal-on-cursor behavior + atomic-range navigation is Story 4.3g), other decorations (4.3a/4.3c–4.3f), Split mode (4.4), the switcher UI (4.5), or keybindings (4.6).
- A Rust/backend command for the cycle — the state change is a pure CM6 buffer edit; persistence is the existing save path.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Render badge | `* TODO Buy milk` in Pseudo-WYSIWYG | keyword replaced by `.org-todo-badge.org-todo-badge--todo` pill, text `TODO` | N/A |
| Click cycle | click the `TODO` pill | one `input.cycle-todo` transaction → source becomes `* NEXT Buy milk`; pill re-renders as `NEXT` | N/A |
| Wrap | click a `DONE` pill (default seq) | source cycles back to `TODO` | N/A |
| Custom sequence | `#+TODO: TODO STARTED | CANCELLED` + `* STARTED x` | pill cycles `STARTED → CANCELLED` | falls back to default seq if no directive |
| Mid-title keyword | `* Buy TODO milk` / `TODO not a headline` | NOT badged (keyword must be the headline's first word) | N/A |
| Composing (IME) | `view.composing === true` | `cycleTodoState` is a no-op (LD-6) | no dispatch |
| Raw mode | any file | extension absent → zero badges (Story 4.2 guarantee) | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/org/TodoStateCycler.ts` -- NEW. `// Implements FR-4`. CM6 `WidgetType` subclass: `eq()` source-range shallow-equal, `ignoreEvent() === false`, click → injected `onCycle` handler. Exports `TODO_BADGE_CLASS`, `TodoStateSpec`, `TodoStateClass`, `TodoCycleHandler`.
- `shell-ui/src/components/editor/decorations/todoBadges.ts` -- NEW. `// Implements FR-4`. `ViewPlugin` building the replace-decoration set over visible headlines; `resolveTodoSequence(doc)` (`#+TODO:`/`#+SEQ_TODO:`/`#+TYP_TODO:` parsing with `DEFAULT_TODO_SEQUENCE` fallback); `cycleTodoState(view, change)` — the single `input.cycle-todo` command; exports `todoBadges()`.
- `shell-ui/src/components/editor/decorations/todoBadges.test.tsx` -- NEW. 12 tests (happy-dom, real `EditorView`): rendering + per-state class, byte-faithful buffer, click cycle + `userEvent` tag + atomic change, DONE wrap, composing no-op, sequence resolution, custom-sequence cycle, `eq()` matrix, `ignoreEvent()`.
- `shell-ui/src/components/editor/editorMode.ts` -- MINIMAL: one import + `pseudoWysiwygDecorations()` returns `[todoBadges()]`.
- `shell-ui/src/styles/editor.css` -- `.org-todo-badge` + per-state modifiers referencing `--org-accent-*` tokens with FR-22 fallbacks (kept out of `tokens.css` / the LD-58 gate).

## Verification

- `cargo fmt --all -- --check`: pass. `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean (0 warnings). `cargo test --workspace --locked`: all pass, 0 failed.
- `pnpm --filter shell-ui build`: pass (tsc + vite). `pnpm --filter shell-ui test`: 52 passed / 52 (5 files), incl. 12 new. `pnpm --filter shell-ui i18n:check`: pass (no locale diff).
