---
title: 'Heading hierarchy decorations'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '52f8fcd'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 shipped the Raw editor mode and expressed the mode → extension boundary (`editorMode.ts`), leaving the Pseudo-WYSIWYG decoration layer empty. Epic 4 needs the first Pseudo-WYSIWYG decoration: headlines `* H1` through `****** H6` rendered with a hierarchical font size (h1 largest → h6 smallest) so document structure is visible at a glance (FR-4, UJ-1) — while the buffer stays byte-faithful `.org` source.

**Approach:** Add a CodeMirror 6 **line decoration** ViewPlugin (LD-6) under `components/editor/decorations/headings.ts` that tags each headline's line block with a stable `cm-org-heading-h{1..6}` class, computed over the view's visible ranges only. Ship the font-size ladder as a co-located CM6 theme (`em`-based, strictly decreasing h1→h6) so the extension is self-contained and observable via `getComputedStyle`. Wire it into the Pseudo-WYSIWYG / Split extension set through the single `pseudoWysiwygDecorations()` seam in `editorMode.ts`; Raw stays decoration-free by construction.

## Boundaries & Constraints

**Always:**
- Heading rendering is a **line decoration** — it only adds a class to the line block; it NEVER mutates the buffer. Source stays byte-identical (FR-2 round-trip contract).
- Font size strictly decreases h1→h6; the ladder is observable through `getComputedStyle` (the AC's measurable contract; happy-dom resolves the `em` sizes to px).
- The module carries `// Implements FR-4` as its first doc-comment line (repo TS traceability convention, mirroring 4.2's `// Implements FR-3`).
- Implementation lives in its OWN new file (`decorations/headings.ts` + colocated test); the only shared touch is the minimal `editorMode.ts` wiring (one import + one array entry), to minimize conflicts with the 5 sibling 4.3x decoration stories.
- No `view.dispatch` from the plugin: a line decoration is presentational, so the widget recipes (WidgetType.eq / Transaction.userEvent / ignoreEvent / no-dispatch-while-composing) do not apply.

**Never:**
- TODO pills (4.3b), tag pills (4.3c), timestamps (4.3d), checkboxes (4.3e), links (4.3f) — those are sibling stories with their own files.
- Refactoring 4.2's files beyond the one-line wiring seam.
- New `--org-*` tokens (would enter the LD-58 contrast gate) — heading decoration adds font size only; colors continue to come from the existing highlight layer.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| h1..h6 headlines | `* …` through `****** …` in Pseudo-WYSIWYG | each line block gets `cm-org-heading-h{level}`; computed font-size strictly decreasing h1→h6 | N/A |
| Round-trip | any decorated buffer | `doc.toString()` unchanged; rendered lines reconstruct source | N/A |
| Deep headline | `******* …` (7+ stars) | level clamps to h6 (no phantom h7 class) | N/A |
| Non-headline | body text, `*bold*` (no space after stars) | no heading class applied | N/A |
| Edit promotes a line | insert `** ` at a body line start | line gains `cm-org-heading-h2`; only the inserted chars change the buffer | N/A |
| Raw mode | same buffer, Raw mode | no heading decoration (excluded from the Raw extension set) | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/decorations/headings.ts` -- NEW. `// Implements FR-4`. Line-decoration ViewPlugin over visible ranges + co-located `em` font-size theme; exports `headingDecorations()` and `ORG_HEADING_CLASS`. Deeper-than-6 headlines clamp to h6.
- `shell-ui/src/components/editor/decorations/headings.test.tsx` -- NEW. happy-dom + real CM6 view: per-level class, monotonic `getComputedStyle` font-size, byte-identical round-trip, non-headline negatives, h7 clamp, edit-resync.
- `shell-ui/src/components/editor/editorMode.ts` -- WIRING ONLY: import `headingDecorations` + return it from `pseudoWysiwygDecorations()` (single seam shared with 4.3b–4.3f).

## Tasks & Acceptance

**Execution:**
- [x] `decorations/headings.ts` — line-decoration ViewPlugin + font ladder theme; `ORG_HEADING_CLASS` export.
- [x] `decorations/headings.test.tsx` — 6 tests (class, monotonic font-size, round-trip, negatives, clamp, edit-resync).
- [x] `editorMode.ts` — wire into the Pseudo-WYSIWYG/Split decoration set.

**Acceptance Criteria:**
- Given Story 4.2 + Pseudo-WYSIWYG mode active, when a buffer holds `* H1`…`****** H6`, then each renders via a CM6 line decoration with computed CSS `font-size` monotonically decreasing h1→h6 (via `getComputedStyle`) — verified by `headings.test.tsx`.
- And the underlying source is preserved byte-identical (round-trip / FR-2) — verified.
- And the module carries `// Implements FR-4` as its first doc-comment line (repo TS traceability convention) — present.

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings).
- `cargo test --workspace --locked` — pass (no failures; Rust untouched by this story).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + i18n + vite).
- `pnpm --filter shell-ui test` — 46 passed (editor decorations: 6 new).
- `pnpm --filter shell-ui i18n:check` — pass.

## Design Notes

- **Styling ships with the extension, not an external CSS import.** A CM6 `EditorView.theme` mounts its StyleModule wherever the extension loads, so the font ladder is self-contained AND observable via `getComputedStyle` in the happy-dom test env (a bundled `import "./editor.css"` would not apply there). This also avoids touching the shared `editor.css`, reducing sibling-story conflicts. Colors stay with the highlight layer / `--org-*` tokens; the theme carries font size (+ weight) only.
- **`em`, not `px`.** happy-dom resolves the `em` sizes to px (verified), so the AC's `getComputedStyle` monotonic check holds while headings scale with the editor face in the real app.
- **Line decoration, never a replace/widget.** The buffer is never rewritten; the leading `*` stars are preserved in source and on screen — the round-trip contract is structural, not test-enforced-only.
- **Viewport-scoped build.** Decorations are computed over `view.visibleRanges` and recomputed only on `docChanged`/`viewportChanged`, keeping large files cheap and avoiding retained state (no leak across StrictMode remounts — the plugin dies with the view).
- **Traceability.** The AC references `//! Implements FR-4` verified by `tests/traceability.rs`; that Rust doc-comment form is for Rust modules. This is a TS module, so it follows the repo's TS convention (`// Implements FR-4`, as 4.2's `orgLanguage.ts`/`editorMode.ts` use `// Implements FR-3`). No Rust `tests/traceability.rs` harness is introduced here (it would be a cross-cutting, conflict-prone addition outside this story's file boundary).
