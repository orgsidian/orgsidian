// Implements FR-4 — Org UI Kit: TODO-state pill badge widget (click-to-cycle).
//
// Story 4.3b. `TodoStateCycler` is the CM6 `WidgetType` the Pseudo-WYSIWYG
// decoration layer (`editor/decorations/todoBadges.ts`) uses to REPLACE a
// headline's TODO-state keyword with a colored pill. Clicking the pill advances
// the headline to the next state in the configured `#+TODO:` sequence.
//
// The widget is deliberately dumb about *how* the source changes: it only knows
// the keyword's source range and the next state, and it invokes an injected
// `onCycle` handler. The decoration layer owns the actual `Transaction` (tagged
// `userEvent="input.cycle-todo"`) so the TODO cycle routes through one shared
// path (FR-24 / LD-26 plugin-surface consistency), never a private mutation.

import { WidgetType, type EditorView } from "@codemirror/view";

/**
 * The five state buckets a keyword can map to. The first four drive the
 * `--org-accent-{todo,next,done,waiting}` styling tokens; `other` is the
 * fallback for custom `#+TODO:` keywords outside the day-1 vocabulary.
 */
export type TodoStateClass = "todo" | "next" | "done" | "waiting" | "other";

/**
 * Immutable description of one rendered badge. All fields are derived from the
 * source buffer, so the object doubles as the identity used by {@link
 * TodoStateCycler.eq} (LD-6 source-range shallow-equal recipe).
 */
export interface TodoStateSpec {
  /** Source offset of the keyword's first character. */
  readonly from: number;
  /** Source offset one past the keyword's last character. */
  readonly to: number;
  /** The current state keyword exactly as it appears in the source. */
  readonly keyword: string;
  /** The keyword this badge cycles to on click (next in the sequence). */
  readonly next: string;
  /** Which `--org-accent-*` bucket colors the pill. */
  readonly stateClass: TodoStateClass;
}

/**
 * Invoked when the badge is clicked. Receives the live view plus the source
 * range to replace and the replacement keyword; the implementation owns the
 * tagged transaction (see `todoBadges.ts#cycleTodoState`).
 */
export type TodoCycleHandler = (
  view: EditorView,
  change: { from: number; to: number; next: string },
) => void;

/** Stable DOM class on every badge; also the styling hook in `editor.css`. */
export const TODO_BADGE_CLASS = "org-todo-badge";

export class TodoStateCycler extends WidgetType {
  constructor(
    private readonly spec: TodoStateSpec,
    private readonly onCycle: TodoCycleHandler,
  ) {
    super();
  }

  /**
   * LD-6 mandatory recipe: shallow-equal on the source range (plus the keyword
   * and its resolved next-state, so a `#+TODO:` sequence change or an in-place
   * keyword edit forces a redraw). Two badges over the same range showing the
   * same state are interchangeable, so CM6 keeps the existing DOM.
   */
  override eq(other: WidgetType): boolean {
    if (!(other instanceof TodoStateCycler)) return false;
    return (
      this.spec.from === other.spec.from &&
      this.spec.to === other.spec.to &&
      this.spec.keyword === other.spec.keyword &&
      this.spec.next === other.spec.next
    );
  }

  override toDOM(view: EditorView): HTMLElement {
    const el = document.createElement("span");
    el.className = `${TODO_BADGE_CLASS} ${TODO_BADGE_CLASS}--${this.spec.stateClass}`;
    // Expose the state for styling/testing without leaking it into the class
    // string the Raw-mode "no decorations" assertion scans.
    el.dataset.todoState = this.spec.keyword;
    el.textContent = this.spec.keyword;
    el.setAttribute("role", "button");
    el.title = `TODO: ${this.spec.keyword} → ${this.spec.next}`;
    // Suppress the default selection/caret placement so a click reads as a
    // button press, not a text selection, then run the cycle on click.
    el.addEventListener("mousedown", (event) => {
      event.preventDefault();
    });
    el.addEventListener("click", (event) => {
      event.preventDefault();
      this.onCycle(view, {
        from: this.spec.from,
        to: this.spec.to,
        next: this.spec.next,
      });
    });
    return el;
  }

  /**
   * LD-6 mandatory recipe for an interactive widget: return `false` so the
   * editor does NOT swallow events happening inside the pill — the widget's own
   * click handler drives the state cycle.
   */
  override ignoreEvent(): boolean {
    return false;
  }
}
