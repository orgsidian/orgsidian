// Implements FR-3 — Editor Mode → CodeMirror extension mapping.
//
// Story 4.2 delivers Raw mode. The mode boundary is expressed here so the
// `Editor` host stays mode-agnostic: it swaps this extension set through a
// `Compartment` when the mode changes (Story 4.5 wires the switcher UI).

import { type Extension } from "@codemirror/state";

import { type EditorMode } from "@/lib/tauri";

import { headingDecorations } from "./decorations/headings";
import { tagPillDecorations } from "./decorations/tags";
import { orgSyntaxHighlight } from "./orgLanguage";

/**
 * The Pseudo-WYSIWYG decoration/widget layer (headings, TODO pills, tag pills,
 * timestamp/checkbox/link widgets) is built by Stories 4.3a–4.3f. Each sibling
 * decoration story appends its own extension to this set. Raw and
 * Pseudo-WYSIWYG differ only by this set, so Raw stays decoration-free.
 */
function pseudoWysiwygDecorations(): Extension[] {
  return [headingDecorations(), tagPillDecorations()];
}

/**
 * Build the mode-dependent extensions for `mode`.
 *
 * Org syntax highlighting is present in every mode (the source is always
 * tokenized). Raw mode returns highlighting ONLY — the decoration layer is
 * deliberately excluded, satisfying the Story 4.2 AC "no Pseudo-WYSIWYG
 * decorations are rendered". Pseudo-WYSIWYG and Split add the decoration layer
 * on top of the same highlighting.
 */
export function modeExtensions(mode: EditorMode): Extension {
  const highlight = orgSyntaxHighlight();
  switch (mode) {
    case "raw":
      return highlight;
    case "pseudoWysiwyg":
    case "split":
      return [highlight, ...pseudoWysiwygDecorations()];
  }
}
