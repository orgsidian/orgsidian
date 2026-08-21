// Implements FR-4 — Pseudo-WYSIWYG timestamp decorations (FR-9 timestamp
// surface). Renders org timestamps as human-readable dates with a
// hover-for-source tooltip, while the buffer stays byte-faithful source text.
//
// Story 4.3d. In Pseudo-WYSIWYG (and Split) mode an org timestamp —
// `<2026-05-19 Tue 14:00>` (active) or `[2026-05-19 Tue]` (inactive) — renders
// as a `Decoration.replace` widget showing a locale-formatted date plus the
// source time (e.g. "Tue, May 19 · 14:00"); active vs inactive stamps are
// visually distinct. Resting the pointer over a stamp for >300ms reveals a
// tooltip with the exact raw source.
//
// Source-of-truth boundary (LD-6 note "reuse the Epic 2 semantic layer; do NOT
// re-parse timestamps in TS"): the full org timestamp *grammar* — repeaters,
// warning delays, `--` ranges, weekday validation — lives in
// `orgsidian-parser/src/semantic/timestamp.rs` and is NOT reimplemented here.
// This module only (a) *locates* stamps with the same delimiter vocabulary the
// Raw-mode highlighter uses (`orgLanguage.ts`) and (b) extracts the two
// display-relevant fields — the `YYYY-MM-DD` date and the optional clock time —
// to format for reading. The exact source bytes are never mutated: they are
// carried verbatim into the widget and the tooltip, so round-trip fidelity
// (FR-2) is untouched.
//
// Weekday policy: the rendered weekday is *computed from the date* via the
// locale formatter, deliberately independent of the source day-name (which org
// treats as display sugar and the parser does not model — it can be stale after
// a hand-edit). The unmodified source, day-name and all, is always one hover
// away in the tooltip.

import { type EditorState, type Extension, RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  hoverTooltip,
  type Tooltip,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";

/**
 * Stable DOM classes emitted by the timestamp widget and its tooltip. Exported
 * so tests assert by class and the co-located `EditorView.theme` styles them
 * through the `--org-*` token vocabulary (LD-6 styling contract).
 */
export const ORG_TIMESTAMP_CLASS = {
  base: "cm-org-timestamp",
  active: "cm-org-timestamp-active",
  inactive: "cm-org-timestamp-inactive",
  tooltip: "cm-org-timestamp-tooltip",
} as const;

/**
 * Hover dwell (ms) before the raw-source tooltip appears. CM6's `hoverTooltip`
 * fires its source only once the pointer has rested this long — i.e. a hover of
 * >300ms surfaces the tooltip (Story 4.3d AC).
 */
export const TIMESTAMP_HOVER_MS = 300;

// Delimiter-anchored stamp matchers mirroring the Raw highlighter
// (`orgLanguage.ts`): active `<YYYY-MM-DD …>` vs inactive `[YYYY-MM-DD …]`.
// Never spans a newline. A fresh instance per scan keeps `lastIndex` private.
function timestampRegex(): RegExp {
  return /<\d{4}-\d{2}-\d{2}[^>\n]*>|\[\d{4}-\d{2}-\d{2}[^\]\n]*\]/g;
}

// Display-field extractors — the date, and the first clock time (a single
// `H:MM` or a `H:MM-H:MM` range), taken verbatim from the source slice.
const DATE_RE = /(\d{4})-(\d{2})-(\d{2})/;
const TIME_RE = /(\d{1,2}:\d{2}(?:-\d{1,2}:\d{2})?)/;

/**
 * Format a raw org timestamp for reading, e.g. `<2026-05-19 Tue 14:00>` →
 * `"Tue, May 19 · 14:00"`. Returns `null` when the leading date is impossible
 * (e.g. `<2026-13-40>`) so the caller leaves the raw source visible rather than
 * rendering a misleading widget. The weekday is computed from the date via the
 * locale formatter (UTC-anchored so a runner's timezone can never shift the
 * calendar day); the clock time, when present, is kept exactly as written.
 */
export function formatTimestamp(raw: string): string | null {
  const dateMatch = DATE_RE.exec(raw);
  if (dateMatch === null) return null;
  const year = Number(dateMatch[1]);
  const month = Number(dateMatch[2]);
  const day = Number(dateMatch[3]);

  const date = new Date(Date.UTC(year, month - 1, day));
  // Reject impossible dates: `Date.UTC` normalizes overflow (month 13 → next
  // year, day 40 → next month), so any component that changed was invalid.
  if (
    Number.isNaN(date.getTime()) ||
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() !== month - 1 ||
    date.getUTCDate() !== day
  ) {
    return null;
  }

  const dateLabel = new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(date);

  const afterDate = raw.slice(dateMatch.index + dateMatch[0].length);
  const timeMatch = TIME_RE.exec(afterDate);
  return timeMatch === null ? dateLabel : `${dateLabel} · ${timeMatch[1]}`;
}

/**
 * The replace-widget for one rendered timestamp. Holds the exact source slice
 * (`raw`), its delimiter kind (`active`), and the pre-formatted `label`.
 */
export class TimestampWidget extends WidgetType {
  constructor(
    readonly raw: string,
    readonly active: boolean,
    readonly label: string,
  ) {
    super();
  }

  // LD-6 recipe: shallow-equal by source content. `raw` is the exact source
  // slice for this range and `active` its delimiter kind; `label` is a pure
  // function of `raw`, so equal (raw, active) ⇒ equal widget — CM6 then reuses
  // the existing DOM instead of tearing it down on every recompute.
  eq(other: TimestampWidget): boolean {
    return other.raw === this.raw && other.active === this.active;
  }

  toDOM(): HTMLElement {
    const span = document.createElement("span");
    const kind = this.active ? "active" : "inactive";
    span.className = `${ORG_TIMESTAMP_CLASS.base} ${
      this.active ? ORG_TIMESTAMP_CLASS.active : ORG_TIMESTAMP_CLASS.inactive
    }`;
    span.textContent = this.label;
    span.setAttribute("data-org-timestamp", kind);
    // Raw source stays reachable to assistive tech even without the hover
    // tooltip (which is pointer-only).
    span.setAttribute("aria-label", this.raw);
    span.setAttribute("data-org-timestamp-raw", this.raw);
    return span;
  }

  // Non-interactive widget: ignore every event so hover detection, clicks, and
  // selection reach the editor. No widget-local listeners are attached, so
  // there is nothing to leak when CM6 discards the DOM.
  ignoreEvent(): boolean {
    return true;
  }
}

// True when any selection range overlaps [from, to]. When the cursor/selection
// touches a stamp we skip its widget so the raw source is revealed for editing
// — an atomic replace must never trap the caret (source-position fidelity,
// Story 4.3g).
function selectionTouches(state: EditorState, from: number, to: number): boolean {
  return state.selection.ranges.some((range) => range.from <= to && range.to >= from);
}

function buildDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const { state } = view;
  for (const { from, to } of view.visibleRanges) {
    const text = state.sliceDoc(from, to);
    const re = timestampRegex();
    let match: RegExpExecArray | null;
    while ((match = re.exec(text)) !== null) {
      const start = from + match.index;
      const end = start + match[0].length;
      if (selectionTouches(state, start, end)) continue;
      const label = formatTimestamp(match[0]);
      if (label === null) continue;
      builder.add(
        start,
        end,
        Decoration.replace({
          widget: new TimestampWidget(match[0], match[0].startsWith("<"), label),
        }),
      );
    }
  }
  return builder.finish();
}

const timestampPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate): void {
      // Rebuild on content OR viewport change (new stamps enter view) and on
      // selection change (reveal/hide the stamp the caret now touches).
      if (update.docChanged || update.viewportChanged || update.selectionSet) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

/**
 * Hover-tooltip source: if `pos` falls inside a timestamp on its line, return a
 * tooltip whose DOM shows the exact raw source. Exported for direct unit
 * testing (headless happy-dom has no layout to drive a real pointer hover).
 */
export function timestampTooltipSource(view: EditorView, pos: number): Tooltip | null {
  const line = view.state.doc.lineAt(pos);
  const re = timestampRegex();
  let match: RegExpExecArray | null;
  while ((match = re.exec(line.text)) !== null) {
    const start = line.from + match.index;
    const end = start + match[0].length;
    if (pos >= start && pos <= end) {
      const raw = match[0];
      return {
        pos: start,
        end,
        above: true,
        create() {
          const dom = document.createElement("div");
          dom.className = ORG_TIMESTAMP_CLASS.tooltip;
          dom.textContent = raw;
          return { dom };
        },
      };
    }
  }
  return null;
}

const timestampHover = hoverTooltip((view, pos) => timestampTooltipSource(view, pos), {
  hoverTime: TIMESTAMP_HOVER_MS,
});

// Widget + tooltip styling. Colors resolve to the `--org-*` token vocabulary
// (LD-6 styling contract); active reuses the focus accent, inactive the muted
// foreground — matching the Raw-mode highlighter so the two modes read alike.
const timestampTheme = EditorView.theme({
  ".cm-org-timestamp": {
    borderRadius: "3px",
    padding: "0 3px",
    fontVariantNumeric: "tabular-nums",
    cursor: "default",
  },
  ".cm-org-timestamp-active": {
    color: "var(--org-border-focus)",
    backgroundColor: "var(--org-bg-surface)",
  },
  ".cm-org-timestamp-inactive": {
    color: "var(--org-fg-muted)",
    backgroundColor: "var(--org-bg-surface)",
  },
  ".cm-org-timestamp-tooltip": {
    padding: "2px 6px",
    borderRadius: "4px",
    border: "1px solid var(--org-border-default)",
    backgroundColor: "var(--org-bg-elevated)",
    color: "var(--org-fg-default)",
    fontFamily: '"IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
    fontSize: "0.85em",
    whiteSpace: "nowrap",
  },
});

/**
 * The timestamp decoration layer: the replace-widget ViewPlugin, the
 * hover-for-source tooltip, and their styling. Appended to the Pseudo-WYSIWYG /
 * Split extension set by `editorMode.ts`; absent from Raw mode.
 */
export function timestampDecorations(): Extension {
  return [timestampPlugin, timestampHover, timestampTheme];
}
