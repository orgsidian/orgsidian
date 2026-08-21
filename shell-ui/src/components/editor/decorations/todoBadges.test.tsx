// @vitest-environment happy-dom
import { EditorView } from "@codemirror/view";
import { type Transaction } from "@codemirror/state";
import { afterEach, describe, expect, it } from "vitest";

/**
 * Story 4.3b — TODO-state pill badges (click-to-cycle). The suite exercises the
 * ACs against a real CM6 `EditorView` (happy-dom, required for CM6 layout):
 *  1. a headline TODO keyword renders as an `org-todo-badge` replace-widget,
 *     colored per state, while the buffer stays byte-faithful;
 *  2. clicking the badge cycles to the next `#+TODO:` state via ONE transaction
 *     tagged `userEvent="input.cycle-todo"`, mutating the source atomically;
 *  3. the widget satisfies the LD-6 `WidgetType.eq()` source-range recipe and
 *     `ignoreEvent() === false`;
 *  4. `#+TODO:` sequences are honored, DONE wraps to the head, mid-title
 *     keywords are never badged, and no dispatch happens while composing.
 */

import { todoBadges, cycleTodoState, cycleTodoAtCursor, CYCLE_TODO_USER_EVENT, resolveTodoSequence, DEFAULT_TODO_SEQUENCE } from "./todoBadges";
import { TodoStateCycler } from "@/components/org/TodoStateCycler";
import { Text } from "@codemirror/state";

let view: EditorView | null = null;
const captured: Transaction[] = [];

afterEach(() => {
  view?.destroy();
  view = null;
  captured.length = 0;
});

/** Mount a live view with the badge extension; record dispatched transactions. */
function mount(doc: string): EditorView {
  const v = new EditorView({
    doc,
    extensions: [todoBadges()],
    parent: document.body,
    dispatchTransactions: (trs, target) => {
      captured.push(...trs);
      target.update(trs);
    },
  });
  view = v;
  return v;
}

function badges(v: EditorView): HTMLElement[] {
  return Array.from(v.dom.querySelectorAll<HTMLElement>(".org-todo-badge"));
}

describe("todoBadges — rendering", () => {
  it("replaces a headline TODO keyword with a colored badge widget", () => {
    const v = mount("* TODO Buy milk\n");
    const found = badges(v);
    expect(found).toHaveLength(1);
    expect(found[0].textContent).toBe("TODO");
    // Colored per state via the state modifier class (→ --org-accent-todo).
    expect(found[0].classList.contains("org-todo-badge--todo")).toBe(true);
    expect(found[0].dataset.todoState).toBe("TODO");
  });

  it("badges each recognized state (TODO/NEXT/DONE/WAITING) with its class", () => {
    const v = mount("* TODO a\n* NEXT b\n* WAITING c\n* DONE d\n");
    const classes = badges(v).map(
      (el) => Array.from(el.classList).find((c) => c.startsWith("org-todo-badge--")),
    );
    expect(classes).toEqual([
      "org-todo-badge--todo",
      "org-todo-badge--next",
      "org-todo-badge--waiting",
      "org-todo-badge--done",
    ]);
  });

  it("does not badge a keyword that is not the headline's first word", () => {
    const v = mount("* Buy TODO milk\nTODO not a headline\n");
    expect(badges(v)).toHaveLength(0);
  });

  it("keeps the source buffer byte-faithful despite the widget replacement", () => {
    const source = "* TODO Buy milk\nbody\n";
    const v = mount(source);
    expect(v.state.doc.toString()).toBe(source);
  });
});

describe("todoBadges — click-to-cycle", () => {
  it("cycles to the next state via one input.cycle-todo transaction, atomically", () => {
    const v = mount("* TODO Buy milk\n");
    badges(v)[0].dispatchEvent(new MouseEvent("click", { bubbles: true }));

    // Source mutated atomically: only the keyword changed.
    expect(v.state.doc.toString()).toBe("* NEXT Buy milk\n");
    // Exactly one transaction, tagged with the shared user event.
    const cycleTrs = captured.filter((tr) => tr.isUserEvent(CYCLE_TODO_USER_EVENT));
    expect(cycleTrs).toHaveLength(1);
    expect(cycleTrs[0].changes.empty).toBe(false);
    // The re-rendered badge now reflects the new state.
    expect(badges(v)[0].textContent).toBe("NEXT");
  });

  it("wraps DONE back to the head of the sequence", () => {
    const v = mount("* DONE Buy milk\n");
    badges(v)[0].dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(v.state.doc.toString()).toBe("* TODO Buy milk\n");
  });

  it("cycleTodoState does not dispatch while the view is composing (LD-6)", () => {
    const v = mount("* TODO Buy milk\n");
    Object.defineProperty(v, "composing", { value: true, configurable: true });
    cycleTodoState(v, { from: 2, to: 6, next: "NEXT" });
    expect(v.state.doc.toString()).toBe("* TODO Buy milk\n");
    expect(captured.filter((tr) => tr.isUserEvent(CYCLE_TODO_USER_EVENT))).toHaveLength(0);
  });
});

describe("todoBadges — #+TODO sequence resolution", () => {
  it("honors a document #+TODO: directive over the default sequence", () => {
    expect(resolveTodoSequence(Text.of(["#+TODO: TODO STARTED | CANCELLED"]))).toEqual([
      "TODO",
      "STARTED",
      "CANCELLED",
    ]);
  });

  it("falls back to the default sequence when no directive is present", () => {
    expect(resolveTodoSequence(Text.of(["* TODO a"]))).toEqual([...DEFAULT_TODO_SEQUENCE]);
  });

  it("cycles through a custom #+TODO: sequence declared in the document", () => {
    const v = mount("#+TODO: TODO STARTED | CANCELLED\n* STARTED task\n");
    const badge = badges(v);
    expect(badge).toHaveLength(1);
    expect(badge[0].textContent).toBe("STARTED");
    badge[0].dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(v.state.doc.toString()).toBe("#+TODO: TODO STARTED | CANCELLED\n* CANCELLED task\n");
  });
});

describe("TodoStateCycler — LD-6 widget recipes", () => {
  const noop = () => {};
  const spec = { from: 2, to: 6, keyword: "TODO", next: "NEXT", stateClass: "todo" as const };

  it("eq() is true for the same source range + state, false otherwise", () => {
    const a = new TodoStateCycler(spec, noop);
    const b = new TodoStateCycler({ ...spec }, noop);
    expect(a.eq(b)).toBe(true);

    expect(a.eq(new TodoStateCycler({ ...spec, from: 3, to: 7 }, noop))).toBe(false);
    expect(a.eq(new TodoStateCycler({ ...spec, keyword: "NEXT" }, noop))).toBe(false);
    expect(a.eq(new TodoStateCycler({ ...spec, next: "WAITING" }, noop))).toBe(false);
  });

  it("ignoreEvent() returns false so the editor honors the interactive click", () => {
    expect(new TodoStateCycler(spec, noop).ignoreEvent()).toBe(false);
  });
});

// Story 4.6 (FR-5): the keyboard command form of the cycle, wired to the
// default keymap. Shares the same mutation surface + userEvent tag as the pill.
describe("cycleTodoAtCursor — keyboard command (Story 4.6)", () => {
  function setCursor(v: EditorView, pos: number) {
    v.dispatch({ selection: { anchor: pos } });
  }

  it("advances the keyword on the headline at the cursor via the shared tag", () => {
    const v = mount("* TODO Buy milk\n");
    setCursor(v, 8); // inside the title, below the keyword's line
    captured.length = 0;
    expect(cycleTodoAtCursor(v)).toBe(true);
    expect(v.state.doc.toString()).toBe("* NEXT Buy milk\n");
    expect(captured[captured.length - 1]?.isUserEvent(CYCLE_TODO_USER_EVENT)).toBe(
      true,
    );
  });

  it("walks up to the nearest headline above the cursor", () => {
    const v = mount("* TODO Parent\nbody line\n");
    setCursor(v, v.state.doc.line(2).from + 2); // on the body line
    expect(cycleTodoAtCursor(v)).toBe(true);
    expect(v.state.doc.line(1).text).toBe("* NEXT Parent");
  });

  it("inserts the first keyword when the headline has none", () => {
    const v = mount("* Bare heading\n");
    setCursor(v, 4);
    expect(cycleTodoAtCursor(v)).toBe(true);
    expect(v.state.doc.line(1).text).toBe("* TODO Bare heading");
  });

  it("returns false (chord falls through) when the cursor is in the preamble", () => {
    const v = mount("preamble text\n* TODO Later\n");
    setCursor(v, 3);
    expect(cycleTodoAtCursor(v)).toBe(false);
    expect(v.state.doc.line(1).text).toBe("preamble text");
  });
});
