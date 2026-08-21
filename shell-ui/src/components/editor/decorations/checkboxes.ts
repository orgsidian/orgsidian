// Implements FR-4 — Pseudo-WYSIWYG checkbox toggle widget (source-mutating click).
//
// Story 4.3e. In Pseudo-WYSIWYG (and Split) mode an org list-item checkbox
// marker — `[ ]`, `[X]`/`[x]`, or `[-]` (partial) — renders as an interactive
// checkbox. Clicking it mutates the SOURCE (`- [ ]` ↔ `- [X]`) through a single
// CM6 `Transaction` tagged `userEvent="input.toggle-checkbox"`. The buffer stays
// byte-faithful org text except the one toggled state char (FR-2 round-trip
// contract) — this is the ONLY path here that changes the buffer; a re-render
// never mutates source.
//
// LD-6 mandatory widget recipes honored here:
//  - `WidgetType.eq()` compares by source range (from/to) plus rendered state,
//    so a re-render reuses the DOM at a stable range/state and only a real state
//    flip rebuilds the node — the widget is not destroyed unnecessarily.
//  - every widget-triggered change carries a `Transaction.userEvent` tag.
//  - the toggle never dispatches while `view.composing` (IME composition) is
//    active.
//  - `ignoreEvent()` returns `false` so the click reaches the editor and the
//    ViewPlugin's `mousedown` handler resolves the source position via
//    `posAtDOM` (the official CM6 interactive-widget pattern).
//
// FR-24 / LD-26: the edit rides the shared editor transaction/event surface
// (`view.dispatch`) — no private, parallel mutation path.

import { RangeSetBuilder, type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";

/**
 * Stable CSS classes on the rendered checkbox. Exported so tests locate the
 * widget by class and `styles/editor.css` styles it via the `--org-*` token
 * vocabulary (this module emits classes only; colors live in CSS).
 */
export const ORG_CHECKBOX_CLASS = {
  /** The replace-widget wrapper span. */
  wrap: "cm-org-checkbox",
  /** The `<input type="checkbox">` inside the wrapper. */
  input: "cm-org-checkbox-input",
} as const;

/**
 * The `Transaction.userEvent` tag on every checkbox toggle (LD-6). Exported so
 * tests assert the tag via `Transaction.isUserEvent(...)`.
 */
export const TOGGLE_CHECKBOX_USER_EVENT = "input.toggle-checkbox";

/** The three org checkbox states the parser recognizes. */
export type CheckboxState = "empty" | "checked" | "partial";

/** The single character between the brackets for each state. */
const STATE_CHAR: Record<CheckboxState, string> = {
  empty: " ",
  checked: "X",
  partial: "-",
};

/** Map a between-brackets char to a state (or `null` if it is not a marker). */
function stateFromChar(ch: string): CheckboxState | null {
  switch (ch) {
    case " ":
      return "empty";
    case "x":
    case "X":
      return "checked";
    case "-":
      return "partial";
    default:
      return null;
  }
}

/**
 * Toggle rule: a checked box clears to empty; an empty OR partial box becomes
 * checked. (Per Story 4.3e AC: toggle empty ↔ X; a partial box resolves to X on
 * click, matching the org "cycle to done" affordance.)
 */
function nextState(state: CheckboxState): CheckboxState {
  return state === "checked" ? "empty" : "checked";
}

// A list-item head that owns a checkbox: optional indent, a bullet (`-`, `+`,
// or an ordered `1.` / `1)`), inter-token whitespace, then the `[ ]`/`[x]`/
// `[X]`/`[-]` marker. Anchored to line start and run per line, so a bare `[ ]`
// mid-paragraph (not a list item) is never turned into a widget, and a heading
// `*` is never mistaken for a `*` bullet (headings match at column 0 without an
// indent+bullet+marker shape).
const CHECKBOX_LINE = /^(\s*(?:[-+]|\d+[.)])\s+)(\[[ xX-]\])/;

/**
 * Interactive checkbox widget. Carries its source range (`from`/`to`, the
 * marker span) and current `state`.
 */
export class CheckboxWidget extends WidgetType {
  constructor(
    readonly from: number,
    readonly to: number,
    readonly state: CheckboxState,
  ) {
    super();
  }

  /**
   * Compare by source range (from/to) plus rendered state. Same checkbox at the
   * same source range in the same state ⇒ reuse the existing DOM across
   * viewport/selection re-renders (no unnecessary destruction). A state flip at
   * that range ⇒ not equal ⇒ CM rebuilds the node to reflect the new state.
   */
  eq(other: CheckboxWidget): boolean {
    return (
      other.from === this.from &&
      other.to === this.to &&
      other.state === this.state
    );
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = ORG_CHECKBOX_CLASS.wrap;
    // Reflect state for styling/tests without leaking source into the a11y tree.
    wrap.dataset.state = this.state;

    const box = document.createElement("input");
    box.type = "checkbox";
    box.className = ORG_CHECKBOX_CLASS.input;
    box.checked = this.state === "checked";
    box.indeterminate = this.state === "partial";
    wrap.appendChild(box);
    return wrap;
  }

  /**
   * Interactive widget (LD-6): return `false` so the editor does NOT ignore the
   * event — it lets the click through to the ViewPlugin's `mousedown` handler,
   * which resolves the source position with `posAtDOM`.
   */
  ignoreEvent(): boolean {
    return false;
  }
}

/** Build the checkbox replace-decorations covering the view's visible ranges. */
function buildCheckboxes(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const { doc } = view.state;
  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = doc.lineAt(pos);
      const match = CHECKBOX_LINE.exec(line.text);
      if (match) {
        const markerStart = line.from + match[1].length;
        const marker = match[2];
        const markerEnd = markerStart + marker.length;
        const state = stateFromChar(marker[1]);
        if (state !== null) {
          builder.add(
            markerStart,
            markerEnd,
            Decoration.replace({
              widget: new CheckboxWidget(markerStart, markerEnd, state),
            }),
          );
        }
      }
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

/**
 * Toggle the checkbox on the line containing `pos`. Re-derives the marker from
 * the live document (never trusts a stale stored offset), mutates exactly the
 * single state char, and rides the shared transaction surface with the LD-6
 * `userEvent` tag. Returns `true` when it dispatched (event handled).
 */
function toggleCheckboxAt(view: EditorView, pos: number): boolean {
  // LD-6: never mutate the buffer mid-IME-composition.
  if (view.composing) return false;

  const line = view.state.doc.lineAt(pos);
  const match = CHECKBOX_LINE.exec(line.text);
  if (!match) return false;

  const markerStart = line.from + match[1].length;
  const stateCharPos = markerStart + 1; // the char between the brackets
  const current = stateFromChar(match[2][1]);
  if (current === null) return false;

  view.dispatch({
    changes: {
      from: stateCharPos,
      to: stateCharPos + 1,
      insert: STATE_CHAR[nextState(current)],
    },
    userEvent: TOGGLE_CHECKBOX_USER_EVENT,
  });
  return true;
}

/**
 * Keyboard-driven checkbox toggle for the line at the main cursor — the command
 * form of the click-to-toggle widget, wired to the default keymap in Story 4.6
 * (`keybindings/default.ts`). Delegates to {@link toggleCheckboxAt} at the
 * cursor position, so the keyboard path and the click path share ONE mutation
 * surface (FR-24 / LD-26) and the same `input.toggle-checkbox` user-event tag.
 * Returns `true` when a checkbox toggled (chord handled) and `false` when the
 * cursor line has none (the chord falls through).
 */
export function toggleCheckboxAtCursor(view: EditorView): boolean {
  return toggleCheckboxAt(view, view.state.selection.main.head);
}

const checkboxPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildCheckboxes(view);
    }

    update(update: ViewUpdate) {
      // Rebuild on document edits (a toggle, or any change that shifts markers)
      // and when the viewport scrolls new lines into view.
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildCheckboxes(update.view);
      }
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    eventHandlers: {
      mousedown(event, view) {
        const target = event.target as HTMLElement | null;
        const wrap = target?.closest?.(`.${ORG_CHECKBOX_CLASS.wrap}`);
        if (wrap === null || wrap === undefined) return false;
        // Prevent the native checkbox from toggling on its own; the source
        // mutation + widget rebuild is the single source of truth.
        event.preventDefault();
        return toggleCheckboxAt(view, view.posAtDOM(wrap));
      },
    },
  },
);

/**
 * The checkbox decoration layer for Pseudo-WYSIWYG / Split modes (FR-4). Raw
 * mode never includes it (see `editorMode.ts`).
 */
export function checkboxDecorations(): Extension {
  return [checkboxPlugin];
}
