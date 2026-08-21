// Implements FR-4 — heading-hierarchy line decorations (Pseudo-WYSIWYG).
//
// Story 4.3a: in Pseudo-WYSIWYG mode a headline `* … ` through `****** … `
// renders with a hierarchical font size (h1 largest → h6 smallest) so document
// structure is visible at a glance (UJ-1). This is a CodeMirror 6 line
// decoration (LD-6): the decoration only adds a `cm-org-heading-h{1..6}` class
// to the line's block element — it NEVER touches the buffer, so the underlying
// `.org` source stays byte-identical (the FR-2 round-trip contract).
//
// Styling ships with the extension as a CM6 theme (font size only, via `em` so
// headings scale with the editor face) rather than an external stylesheet: the
// extension is self-contained wherever it is loaded, and colors continue to
// come from the `--org-*` token vocabulary applied by `orgLanguage`'s
// highlight layer. Interaction recipes for widgets (WidgetType.eq,
// Transaction.userEvent, ignoreEvent, no-dispatch-while-composing) do not apply
// here: a line decoration is purely presentational and dispatches nothing.

import { RangeSetBuilder, type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

/** Maximum heading level with a distinct face; deeper headings clamp to h6. */
const MAX_HEADING_LEVEL = 6;

/**
 * Stable per-level line-decoration classes (`cm-org-heading-h1` … `-h6`).
 * Exported so tests can assert the decoration is applied by class. The `org`
 * prefix keeps them clear of CodeMirror's built-in class namespace.
 */
export const ORG_HEADING_CLASS = Array.from(
  { length: MAX_HEADING_LEVEL },
  (_, i) => `cm-org-heading-h${i + 1}`,
) as readonly string[];

// A headline is a run of leading `*` immediately followed by whitespace (org
// syntax). `line.text` carries no trailing newline, so a lone `*` with no space
// is correctly NOT a headline — matching `orgLanguage`'s `HEADLINE_STARS`.
const HEADLINE = /^(\*+)(?=\s)/;

// One `Decoration.line` per level, created once and reused (line decorations
// are value objects keyed by their spec, so sharing them lets the RangeSet
// coalesce identical entries).
const HEADING_LINE_DECO: readonly Decoration[] = ORG_HEADING_CLASS.map((cls) =>
  Decoration.line({ class: cls }),
);

/**
 * Font-size hierarchy for the heading classes — strictly decreasing h1→h6, in
 * `em` so the whole ladder scales with the editor font. Font size only; weight
 * and color stay with the highlight/token layer and the `--org-*` tokens.
 */
const headingTheme = EditorView.theme({
  ".cm-org-heading-h1": { fontSize: "1.6em", fontWeight: "600" },
  ".cm-org-heading-h2": { fontSize: "1.45em", fontWeight: "600" },
  ".cm-org-heading-h3": { fontSize: "1.3em", fontWeight: "600" },
  ".cm-org-heading-h4": { fontSize: "1.2em", fontWeight: "600" },
  ".cm-org-heading-h5": { fontSize: "1.1em", fontWeight: "600" },
  ".cm-org-heading-h6": { fontSize: "1.0em", fontWeight: "600" },
});

/**
 * Build the heading line-decoration set for the view's visible ranges only
 * (viewport-scoped, so large files stay cheap). `RangeSetBuilder` requires
 * ascending, non-overlapping starts; iterating line-by-line yields exactly
 * that. Line decorations are added at the line start (`from === to === line.from`).
 */
function buildHeadingDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      const match = HEADLINE.exec(line.text);
      if (match !== null) {
        const level = Math.min(match[1].length, MAX_HEADING_LEVEL);
        builder.add(line.from, line.from, HEADING_LINE_DECO[level - 1]);
      }
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

/**
 * The heading-hierarchy CM6 ViewPlugin: recomputes its line decorations when
 * the document or the viewport changes, and otherwise reuses the cached set.
 * It owns no disposable resources beyond the decoration set, so CM6's own
 * plugin teardown is sufficient (no extra cleanup, no leak across StrictMode
 * remounts — the plugin instance dies with the view).
 */
const headingDecorationPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildHeadingDecorations(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildHeadingDecorations(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

/**
 * The heading-hierarchy decoration extension (line decorations + their font
 * ladder theme). Added to the Pseudo-WYSIWYG / Split extension set in
 * `editorMode.ts`; absent from Raw mode, which stays decoration-free.
 */
export function headingDecorations(): Extension {
  return [headingDecorationPlugin, headingTheme];
}
