// Implements FR-4 — tag pill labels for Pseudo-WYSIWYG editor mode (Story 4.3c).
//
// A headline's trailing `:tag:` / `:tag1:tag2:tag3:` suffix renders as one pill
// per tag: each tag becomes a `Decoration.replace` widget styled via
// `--org-accent-tag`, and the `:` delimiters are visually hidden by the widget
// while remaining byte-identical in the source (the FR-2 round-trip contract —
// `replace` decorations are presentational and never mutate the buffer).
//
// This layer is mode-scoped: `editorMode.ts` includes it only in the
// Pseudo-WYSIWYG / Split decoration set, so Raw mode stays decoration-free.
//
// Tag detection mirrors the org tag grammar already tokenized by
// `orgLanguage.ts` (same `[A-Za-z0-9_@%#]` tag-character class) but is anchored
// differently: pills only apply to the *trailing* tag block of a *headline*
// line (org tags are a headline suffix), not to a `:foo:` appearing mid-body.

import { RangeSetBuilder, type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  type EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";

/** Stable class on the rendered pill span (asserted by tests, styled in CSS). */
export const TAG_PILL_CLASS = "cm-org-tag-pill";

// Org tag characters: word chars plus @ # % _ (mirrors `orgLanguage.ts` TAGS).
const TAG_CHAR = "[A-Za-z0-9_@%#]";
// A headline line: leading `*+` stars followed by whitespace.
const HEADLINE = /^\*+\s/;
// The trailing tag block on a headline: whitespace (or line start), then
// `:t1:t2:…:` anchored at end of line. Group 1 is the leading separator so we
// can locate the block's exact start; group 2 is the `:…:` block itself.
const TAG_BLOCK = new RegExp(
  `(^|\\s)(:${TAG_CHAR}+(?::${TAG_CHAR}+)*:)$`,
);
// Each `:name` occurrence within a block (the leading colon + the tag name).
const TAG_IN_BLOCK = new RegExp(`:(${TAG_CHAR}+)`, "g");

/** A single tag located within a block: its `:`-offset and bare name. */
interface TagHit {
  /** Offset of the tag's leading `:` relative to the block start. */
  colonOffset: number;
  /** The bare tag name (no delimiters) — the pill label. */
  name: string;
}

function collectTags(blockText: string): TagHit[] {
  const hits: TagHit[] = [];
  TAG_IN_BLOCK.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = TAG_IN_BLOCK.exec(blockText)) !== null) {
    hits.push({ colonOffset: m.index, name: m[1] });
  }
  return hits;
}

/**
 * The pill widget for one tag. Per the LD-6 CM6 recipe, `eq()` shallow-compares
 * by source range (`from`/`to`) plus the rendered `label`, so CM6 reuses the DOM
 * node across viewport rebuilds when nothing about this tag moved or changed.
 */
class TagPillWidget extends WidgetType {
  constructor(
    readonly label: string,
    readonly from: number,
    readonly to: number,
  ) {
    super();
  }

  eq(other: TagPillWidget): boolean {
    return (
      other.label === this.label &&
      other.from === this.from &&
      other.to === this.to
    );
  }

  toDOM(): HTMLElement {
    const pill = document.createElement("span");
    pill.className = TAG_PILL_CLASS;
    // The delimiting colons live in the source range this widget replaces; the
    // pill renders only the bare name, so they are hidden but preserved.
    pill.textContent = this.label;
    pill.setAttribute("data-org-tag", this.label);
    return pill;
  }
}

/**
 * Add the replace decorations for one line's trailing tag block, if any. The
 * block is tiled by exactly one `Decoration.replace` per tag (contiguous,
 * non-overlapping, left-to-right — the order `RangeSetBuilder` requires): tag
 * `i` covers from its leading `:` up to the next tag's leading `:`, and the
 * final tag extends to the block end so the trailing `:` is absorbed. Every
 * delimiter therefore falls inside some pill's replaced range and is hidden,
 * while the buffer text is untouched.
 */
function addLineTagPills(
  builder: RangeSetBuilder<Decoration>,
  lineText: string,
  lineStart: number,
): void {
  if (!HEADLINE.test(lineText)) return;
  const block = TAG_BLOCK.exec(lineText);
  if (block === null) return;

  const blockText = block[2];
  const blockStart = lineStart + block.index + block[1].length;
  const blockEnd = blockStart + blockText.length;

  const tags = collectTags(blockText);
  for (let i = 0; i < tags.length; i += 1) {
    const from = blockStart + tags[i].colonOffset;
    const to =
      i + 1 < tags.length
        ? blockStart + tags[i + 1].colonOffset
        : blockEnd;
    builder.add(
      from,
      to,
      Decoration.replace({ widget: new TagPillWidget(tags[i].name, from, to) }),
    );
  }
}

function buildTagDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      addLineTagPills(builder, line.text, line.from);
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

/**
 * The tag-pill decoration layer: a `ViewPlugin` that rebuilds its replace
 * decorations for the visible ranges on document or viewport change. Included
 * only in the Pseudo-WYSIWYG / Split extension sets (see `editorMode.ts`).
 */
export function tagPillDecorations(): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildTagDecorations(view);
      }

      update(update: ViewUpdate): void {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = buildTagDecorations(update.view);
        }
      }
    },
    { decorations: (plugin) => plugin.decorations },
  );
}
