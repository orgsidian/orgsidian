// Implements FR-4 — Pseudo-WYSIWYG link rendering (clickable underlined text).
//
// A CodeMirror 6 `ViewPlugin` (LD-6) that renders org links as underlined text
// while keeping the buffer byte-faithful `.org` source (FR-2). It handles the
// four link variants the AC enumerates — `[[id:abc]]`, `[[wiki-link]]`,
// `[[file://path]]`, `http://…` — plus the described bracket form
// `[[target][label]]`.
//
// Three behaviors, all presentational (the doc is NEVER mutated here):
//   1. Underline: a `Decoration.mark` over the link's rendered text, carrying
//      `data-org-link-*` so the click handler can read target + kind off the DOM.
//   2. Bracket reveal: `[[` / `]]` (and, for the described form, the
//      `[[target][` prefix) are hidden with `Decoration.replace` UNLESS the
//      cursor is on that link's line — recomputed on every `selectionSet`.
//   3. Click: a `mousedown` handler emits `LinkClicked { target, kind }` through
//      the shared event surface (`../events`), consumed by the navigation layer
//      (Epic 8). It does not `preventDefault`, so placing the cursor (which
//      reveals the brackets) and normal editing keep working.

import { RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  type EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import { emitLinkClicked, type LinkKind } from "../events";

/**
 * CSS class on the underline mark. Exported so `styles/editor.css` styles it via
 * the `--org-*` token vocabulary and tests can query the rendered span. It
 * doubles as the click-target selector (the `mousedown` handler reads the
 * nearest element carrying it).
 */
export const ORG_LINK_CLASS = "cm-org-link";

/** DOM data-attributes the mark carries so a click can recover target + kind. */
const LINK_TARGET_ATTR = "data-org-link-target";
const LINK_KIND_ATTR = "data-org-link-kind";

/**
 * Classify a raw link path into a {@link LinkKind}. Recognized scheme prefixes
 * win in order; anything else is a wiki-style title reference (org's default).
 * `file:` matches both `file:path` and `file://path`. Case-insensitive so
 * `HTTP://` / `ID:` classify correctly. Exported for direct unit testing.
 */
export function classifyLink(target: string): LinkKind {
  if (/^id:/i.test(target)) return "id";
  if (/^file:/i.test(target)) return "file";
  if (/^https?:\/\//i.test(target)) return "http";
  return "wiki";
}

/**
 * Matches one link per iteration, left-to-right (the `g` flag advances
 * `lastIndex`). Two alternatives:
 *   - bracket link `[[path]]` or `[[path][label]]` — group 1 = path,
 *     group 2 = optional label. `[^\]\n]` keeps a link on one line and stops the
 *     path/label at the first `]` (so `][` and `]]` are not swallowed).
 *   - bare URL `http(s)://…` — group 3. The trailing class excludes whitespace,
 *     angle brackets, and `[]()` so it does not eat surrounding punctuation.
 * Bracket form is listed first, so a bracketed URL matches as a bracket link.
 */
const LINK_RE =
  /\[\[([^\]\n]+?)(?:\]\[([^\]\n]+?))?\]\]|(https?:\/\/[^\s<>[\]()]+)/g;

/** A resolved link occurrence in the document, in absolute doc offsets. */
interface LinkMatch {
  /** Start of the whole link (the first `[` or the URL's first char). */
  from: number;
  /** End of the whole link (after `]]` or the URL's last char). */
  to: number;
  /** Raw link path (bracket inner path, or the whole URL) — the click target. */
  target: string;
  /** Underlined region: the label if present, else the path/URL. */
  markFrom: number;
  markTo: number;
  /**
   * Bracket ranges to hide when the cursor is off this link's line, in
   * increasing `from` order. Empty for a bare URL (nothing to hide).
   */
  hidden: { from: number; to: number }[];
}

/**
 * Find every link in `[from, to)` of `text` (a slice starting at doc offset
 * `base`). Positions are computed from match indices and captured-group lengths
 * so they are exact source offsets — never rendered/visual offsets.
 */
function findLinks(text: string, base: number): LinkMatch[] {
  const links: LinkMatch[] = [];
  LINK_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = LINK_RE.exec(text)) !== null) {
    const start = base + m.index;
    const url = m[3];
    if (url !== undefined) {
      // Bare URL: the whole match is the underlined, clickable target.
      links.push({
        from: start,
        to: start + url.length,
        target: url,
        markFrom: start,
        markTo: start + url.length,
        hidden: [],
      });
      continue;
    }

    // Bracket link. `path` is guaranteed by the alternative that matched.
    const path = m[1] as string;
    const label = m[2];
    const pathStart = start + 2; // after `[[`
    const pathEnd = pathStart + path.length;
    if (label === undefined) {
      // `[[path]]` — underline the path, hide `[[` and `]]`.
      const end = pathEnd + 2; // after `]]`
      links.push({
        from: start,
        to: end,
        target: path,
        markFrom: pathStart,
        markTo: pathEnd,
        hidden: [
          { from: start, to: pathStart }, // `[[`
          { from: pathEnd, to: end }, // `]]`
        ],
      });
    } else {
      // `[[path][label]]` — underline the label; hide `[[path][` and `]]`.
      const labelStart = pathEnd + 2; // after `][`
      const labelEnd = labelStart + label.length;
      const end = labelEnd + 2; // after `]]`
      links.push({
        from: start,
        to: end,
        target: path,
        markFrom: labelStart,
        markTo: labelEnd,
        hidden: [
          { from: start, to: labelStart }, // `[[path][`
          { from: labelEnd, to: end }, // `]]`
        ],
      });
    }
  }
  return links;
}

/** Line numbers (1-based) that hold at least one selection-range endpoint. */
function cursorLineNumbers(view: EditorView): Set<number> {
  const lines = new Set<number>();
  const doc = view.state.doc;
  for (const range of view.state.selection.ranges) {
    const first = doc.lineAt(range.from).number;
    const last = doc.lineAt(range.to).number;
    for (let n = first; n <= last; n += 1) {
      lines.add(n);
    }
  }
  return lines;
}

/**
 * Build the decoration set for the visible ranges: an underline `mark` for every
 * link, plus `replace` decorations hiding the brackets of links whose line has
 * no cursor. Decorations are emitted in strictly increasing `from` order (links
 * are non-overlapping and scanned left-to-right; within a link the hidden `[[`,
 * the mark, and the hidden `]]` are already ordered), as `RangeSetBuilder`
 * requires.
 */
function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const cursorLines = cursorLineNumbers(view);
  const doc = view.state.doc;

  for (const { from, to } of view.visibleRanges) {
    const links = findLinks(view.state.sliceDoc(from, to), from);
    for (const link of links) {
      const onCursorLine = cursorLines.has(doc.lineAt(link.from).number);
      const linkKind = classifyLink(link.target);
      const mark = Decoration.mark({
        class: ORG_LINK_CLASS,
        attributes: {
          [LINK_TARGET_ATTR]: link.target,
          [LINK_KIND_ATTR]: linkKind,
        },
      });

      // Reveal state (cursor on line) OR a bare URL (no brackets): underline
      // only. Off cursor line, a bracket link ALSO hides its leading run
      // (`[[` or `[[path][`) before the mark and its trailing `]]` after it.
      // Emissions stay strictly increasing in `from` (lead.from < markFrom <
      // trail.from), as `RangeSetBuilder` requires.
      const [lead, trail] = link.hidden;
      if (!onCursorLine && lead !== undefined) {
        builder.add(lead.from, lead.to, Decoration.replace({}));
      }
      builder.add(link.markFrom, link.markTo, mark);
      if (!onCursorLine && trail !== undefined) {
        builder.add(trail.from, trail.to, Decoration.replace({}));
      }
    }
  }
  return builder.finish();
}

/**
 * Emit a {@link LinkClicked} for the link under `eventTarget` (a mousedown's
 * `event.target`), if any. Walks up to the nearest `.cm-org-link` element and
 * reads the target + kind it carries. Returns `true` when an event was emitted.
 *
 * Extracted from the `ViewPlugin` handler so the emission contract is unit
 * testable directly (dispatching a synthetic `MouseEvent` into a live CM6 view
 * perturbs the DOM-selection observer); the plugin simply delegates to it.
 */
export function emitLinkClickFromTarget(eventTarget: EventTarget | null): boolean {
  const el =
    eventTarget instanceof Element
      ? eventTarget.closest(`.${ORG_LINK_CLASS}`)
      : null;
  if (el === null) {
    return false;
  }
  const target = el.getAttribute(LINK_TARGET_ATTR);
  const kind = el.getAttribute(LINK_KIND_ATTR) as LinkKind | null;
  if (target === null || kind === null) {
    return false;
  }
  emitLinkClicked({ target, kind });
  return true;
}

/**
 * The Pseudo-WYSIWYG link layer: a `ViewPlugin` providing the decoration set and
 * a `mousedown` handler that emits {@link emitLinkClicked}. Rebuilt on
 * `docChanged` (links added/removed), `selectionSet` (cursor moved → bracket
 * reveal toggles), and `viewportChanged` (scrolled → new visible ranges).
 */
export function orgLinkDecorations() {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildDecorations(view);
      }

      update(update: ViewUpdate) {
        if (
          update.docChanged ||
          update.selectionSet ||
          update.viewportChanged
        ) {
          this.decorations = buildDecorations(update.view);
        }
      }
    },
    {
      decorations: (plugin) => plugin.decorations,
      eventHandlers: {
        mousedown(event) {
          emitLinkClickFromTarget(event.target);
          // Do not preventDefault: let CM place the cursor (which reveals the
          // brackets) — navigation is the consumer's decision (Epic 8).
          return false;
        },
      },
    },
  );
}
