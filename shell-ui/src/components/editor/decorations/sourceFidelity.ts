// Implements FR-3, FR-4 — source-position fidelity for cursor, copy/paste, and
// find/replace across the Pseudo-WYSIWYG decoration layer (Story 4.3g).
//
// Stories 4.3a–4.3f layered six decoration types onto Pseudo-WYSIWYG mode,
// several of them `Decoration.replace({ widget })` — the one CM6 construct that
// can leak *rendered* text or *visual* positions into clipboard and search if a
// widget were mis-ranged. FR-3 / FR-4 require the opposite: copy, paste, and
// find/replace must read/write SOURCE character offsets regardless of how many
// decorations are in view (the product's "show the .org source" trust contract).
//
// That guarantee is structural in CodeMirror 6, and this module is where the
// contract is made explicit and made real:
//   - The buffer CM6 owns IS the source `.org` text (Epic 4 state-ownership
//     boundary). Copy/cut serialize `state.sliceDoc()` over the selection
//     ranges; paste dispatches an insert transaction at the selection; the
//     `@codemirror/search` cursor scans the document `Text`. None of these paths
//     consult the decoration set — decorations only change what is painted.
//   - So the fidelity of these operations depends solely on every decoration
//     keying its range to the exact source span, which Stories 4.3a–4.3f already
//     do. Story 4.3g proves it (see `sourceFidelity.test.tsx`).
//
// This extension supplies the missing `search` member of the LD-6 editor stack
// so find/replace is genuinely user-invokable (and provably source-based). It is
// mode-INDEPENDENT — find/replace works identically in Raw, Pseudo-WYSIWYG, and
// Split — so the `Editor` host adds it to the base extension set rather than to
// the Pseudo-WYSIWYG-only decoration set.
//
// No `EditorView.atomicRanges` are registered: making the cursor SKIP a widget's
// collapsed range would render those source offsets unaddressable — the exact
// opposite of "operate on source character offsets". Every source character
// stays reachable by the caret.

import { type Extension } from "@codemirror/state";
import { keymap } from "@codemirror/view";
import { search, searchKeymap } from "@codemirror/search";

/**
 * The source-fidelity base extension: wires `@codemirror/search` (the search
 * facet/panel) plus its default keymap so find/replace is user-invokable in
 * every Editor Mode and operates on the source document. Added once to the
 * `Editor` host's base extensions (mode-independent), never to a per-mode set.
 */
export function sourceFidelity(): Extension {
  return [search({ top: true }), keymap.of(searchKeymap)];
}

// Re-export the CM6 search primitives from a single place so tests (and future
// stories that drive search programmatically, e.g. the command palette) share
// one import surface for the `search` stack member.
export {
  SearchQuery,
  SearchCursor,
  RegExpCursor,
  getSearchQuery,
  setSearchQuery,
  findNext,
  findPrevious,
  replaceNext,
  replaceAll,
  openSearchPanel,
  closeSearchPanel,
} from "@codemirror/search";
