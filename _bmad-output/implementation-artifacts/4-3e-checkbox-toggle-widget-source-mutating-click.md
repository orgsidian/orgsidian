---
title: 'Checkbox toggle widget (source-mutating click)'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '73c8e28'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 established the mode boundary: Raw renders org syntax-highlight tokens only, while Pseudo-WYSIWYG / Split spread an (empty-today) decoration layer on top of the same highlighting. Epic 4's FR-4 needs the first interactive decoration — a **checkbox toggle widget**: in Pseudo-WYSIWYG mode an org list-item checkbox `- [ ]` renders as a clickable checkbox, and clicking it mutates the SOURCE `- [ ]` ↔ `- [X]` so task completion is one click, while the buffer stays byte-faithful org text (FR-2 round-trip contract).

**Approach:** Add a self-contained CM6 decoration module `decorations/checkboxes.ts` (a `ViewPlugin` producing `Decoration.replace` widgets over the `[ ]`/`[X]`/`[-]` marker). On click, the ViewPlugin's `mousedown` handler resolves the source position via `posAtDOM` and dispatches a single `Transaction` tagged `userEvent="input.toggle-checkbox"` that replaces exactly the one state char. The widget honors the LD-6 recipes: `WidgetType.eq()` compares by source range + state (re-render reuses DOM, state flip rebuilds), `ignoreEvent()` returns `false` (interactive), and the toggle never dispatches while `view.composing`. Wired into Pseudo-WYSIWYG via the smallest possible edit to `editorMode.ts`.

## Boundaries & Constraints

**Always:**
- The click is the ONLY buffer mutation; a re-render never changes source. The change is byte-identical except the single toggled state char (FR-2).
- The mutation rides the shared editor transaction surface (`view.dispatch`) with the `userEvent` tag — no private, parallel mutation path (FR-24 / LD-26).
- LD-6 widget recipes: `eq()` by source range, `Transaction.userEvent` on every widget-triggered change, no dispatch during `view.composing`, `ignoreEvent() === false`.
- Colors via the existing `--org-*` token vocabulary (kept out of `tokens.css` / the LD-58 gate). Module carries `// Implements FR-4` as its first doc-comment line (4.2's TS traceability convention).

**Ask First:**
- Introducing new `--org-*` accent tokens (would enter the LD-58 contrast gate) — this story reuses existing FR-22 tokens (`--org-border-focus`).

**Never:**
- Refactor 4.2 beyond the minimal `editorMode.ts` wiring; touch sibling decoration stories' surfaces (headings/TODO/tag/timestamp/link).
- Reveal-on-cursor logic, list re-numbering, or parent/child checkbox propagation (out of scope for 4.3e).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Empty → checked | `- [ ] Buy milk`, click | source → `- [X] Buy milk`; `Transaction.isUserEvent("input.toggle-checkbox")`; input re-renders checked | N/A |
| Checked → empty | `- [X] Ship it`, click | source → `- [ ] Ship it` | N/A |
| Partial resolve | `- [-] Halfway`, click | source → `- [X] Halfway` | N/A |
| Ordered list | `1. [ ] item`, click | decorated + toggles | N/A |
| Multi-checkbox offsets | three `- [ ]` lines, click the 2nd | only the 2nd line's char mutates | N/A |
| Non-list bracket | `a bare [ ] mid paragraph` | NOT decorated (no widget) | N/A |
| IME composition | `view.composing === true`, click | no dispatch; source unchanged | guard returns early |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/decorations/checkboxes.ts` -- NEW. `// Implements FR-4`. `CheckboxWidget` (`eq` by from/to/state, `ignoreEvent()===false`, `toDOM` input reflecting checked/indeterminate), `buildCheckboxes` (per-line `Decoration.replace` over the marker), `toggleCheckboxAt` (composing guard, single-char change, `userEvent` tag), `checkboxPlugin` (`ViewPlugin` + `eventHandlers.mousedown` via `posAtDOM`), `checkboxDecorations()`. Exports `ORG_CHECKBOX_CLASS`, `TOGGLE_CHECKBOX_USER_EVENT`, `CheckboxState`, `CheckboxWidget`.
- `shell-ui/src/components/editor/decorations/checkboxes.test.tsx` -- NEW. Real CM6 view over org fixtures: state rendering, click toggle + userEvent tag + byte-fidelity, partial→checked, two-toggle round-trip, multi-checkbox offset math, non-list negative, IME guard, `eq()` + `ignoreEvent()` unit assertions.
- `shell-ui/src/components/editor/editorMode.ts` -- minimal wiring: `pseudoWysiwygDecorations()` now returns `[checkboxDecorations()]`.
- `shell-ui/src/styles/editor.css` -- `.cm-org-checkbox` / `.cm-org-checkbox-input` styling via existing `--org-*` tokens.

## Tasks & Acceptance

**Execution:**
- [x] `decorations/checkboxes.ts` — checkbox `ViewPlugin` + widget + source-mutating toggle.
- [x] `checkboxes.test.tsx` — Vitest + happy-dom: source mutation, userEvent tag, eq-by-range, ignoreEvent, offset math, round-trip, composing guard.
- [x] `editorMode.ts` — add `checkboxDecorations()` to the Pseudo-WYSIWYG set.
- [x] `editor.css` — checkbox styling via existing tokens.

**Acceptance Criteria:**
- Given Pseudo-WYSIWYG mode + a `- [ ]` in the buffer, clicking the widget mutates source `- [ ]` → `- [X]` via a `Transaction` tagged `userEvent="input.toggle-checkbox"` — verified.
- Widget re-renders to reflect the new state; `eq()` compares by source range; `ignoreEvent() === false` — verified.
- Source preserved byte-identical except the toggle (round-trip test) — verified.

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass (no Rust changes).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings).
- `cargo test --workspace --locked` — pass (247 passed, 0 failed).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + lingui + vite).
- `pnpm --filter shell-ui test` — pass (50 passed; checkbox suite 9).
- `pnpm --filter shell-ui i18n:check` — pass (no catalog diff).

## Design Notes

- **`ignoreEvent() === false` is the interactive recipe:** CM6's `WidgetType.ignoreEvent` defaults to ignoring all events; returning `false` lets the editor process the click so the ViewPlugin's `mousedown` handler runs and `posAtDOM(wrap)` resolves the marker's source position. This is the official CM6 boolean-toggle pattern and satisfies LD-6.
- **`eq()` compares from/to + state:** "by source range" keeps the DOM stable across viewport/selection re-renders (same range + state ⇒ reuse); a toggle flips `state` ⇒ not-equal ⇒ CM rebuilds the node to reflect it. So a re-render never destroys the widget unnecessarily, yet a real state change is reflected.
- **Byte-fidelity by construction:** the toggle replaces exactly the one char between the brackets (`markerStart + 1`), re-derived from the live doc at click time (never a stale stored offset), so a multi-checkbox line's other markers are untouched and a double-toggle round-trips byte-identically.
- **Anchored regex avoids false positives:** `CHECKBOX_LINE` requires indent + bullet/ordered-marker + `[c]` at line start, so a heading `*` is never a bullet and a mid-paragraph `[ ]` is never decorated.
