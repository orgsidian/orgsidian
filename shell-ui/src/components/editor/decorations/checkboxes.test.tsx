// @vitest-environment happy-dom
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";

import {
  CheckboxWidget,
  ORG_CHECKBOX_CLASS,
  TOGGLE_CHECKBOX_USER_EVENT,
  checkboxDecorations,
} from "./checkboxes";

/**
 * Story 4.3e (FR-4): the Pseudo-WYSIWYG checkbox toggle widget. These tests
 * mount a REAL CodeMirror 6 view over an org fixture with the checkbox
 * decoration layer and assert:
 *  1. each marker state (`[ ]`/`[X]`/`[-]`) renders as a checkbox reflecting it;
 *  2. clicking mutates the SOURCE `- [ ]` ↔ `- [X]` via a `Transaction` tagged
 *     `userEvent="input.toggle-checkbox"`, byte-identical except the toggled char;
 *  3. the widget re-renders to reflect the new state;
 *  4. `WidgetType.eq()` compares by source range + state; `ignoreEvent()` is
 *     `false` (LD-6 interactive-widget recipes);
 *  5. offset math is correct across multiple checkboxes;
 *  6. no dispatch mid-IME-composition; a bare non-list `[ ]` is not decorated.
 *
 * happy-dom (not jsdom) is required for CM6's `getComputedStyle` calls.
 */

let view: EditorView | undefined;
let container: HTMLDivElement | undefined;

afterEach(() => {
  view?.destroy();
  view = undefined;
  container?.remove();
  container = undefined;
});

interface Harness {
  view: EditorView;
  container: HTMLDivElement;
  /** userEvents observed on dispatched transactions. */
  userEvents: string[];
}

function mount(doc: string): Harness {
  const userEvents: string[] = [];
  const el = document.createElement("div");
  document.body.appendChild(el);
  const v = new EditorView({
    parent: el,
    state: EditorState.create({
      doc,
      extensions: [
        checkboxDecorations(),
        EditorView.updateListener.of((update) => {
          for (const tr of update.transactions) {
            if (tr.isUserEvent(TOGGLE_CHECKBOX_USER_EVENT)) {
              userEvents.push(TOGGLE_CHECKBOX_USER_EVENT);
            }
          }
        }),
      ],
    }),
  });
  view = v;
  container = el;
  return { view: v, container: el, userEvents };
}

/** Click the nth (default first) rendered checkbox via a real bubbling event. */
function clickCheckbox(el: HTMLElement, index = 0): void {
  const inputs = el.querySelectorAll(`.${ORG_CHECKBOX_CLASS.input}`);
  const input = inputs[index] as HTMLInputElement | undefined;
  if (input === undefined) throw new Error(`no checkbox at index ${index}`);
  input.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
}

function checkboxInputs(el: HTMLElement): HTMLInputElement[] {
  return Array.from(el.querySelectorAll(`.${ORG_CHECKBOX_CLASS.input}`));
}

describe("checkboxDecorations (Pseudo-WYSIWYG checkbox widget)", () => {
  it("renders each marker state as a checkbox reflecting it", () => {
    const { container: el } = mount(
      ["- [ ] empty", "- [X] checked", "- [-] partial"].join("\n"),
    );
    const inputs = checkboxInputs(el);
    expect(inputs).toHaveLength(3);
    expect(inputs[0].checked).toBe(false);
    expect(inputs[0].indeterminate).toBe(false);
    expect(inputs[1].checked).toBe(true);
    expect(inputs[2].indeterminate).toBe(true);
    // The replace decoration hides the raw `[ ]` glyph text.
    expect(el.querySelector(".cm-line")?.textContent).not.toContain("[ ]");
  });

  it("decorates ordered-list checkboxes and does NOT decorate a bare non-list [ ]", () => {
    const { container: el } = mount(
      ["1. [ ] ordered", "a bare [ ] mid paragraph", "- [x] lower-x done"].join(
        "\n",
      ),
    );
    const inputs = checkboxInputs(el);
    // Only the ordered-list head and the `- [x]` item are checkboxes (2), the
    // mid-paragraph `[ ]` is left as source text.
    expect(inputs).toHaveLength(2);
    expect(inputs[1].checked).toBe(true); // lower-case `x` counts as checked
  });

  it("toggles source [ ] -> [X] on click via a tagged transaction, byte-faithful otherwise", () => {
    const { view: v, container: el, userEvents } = mount("- [ ] Buy milk\n");
    clickCheckbox(el);

    expect(v.state.doc.toString()).toBe("- [X] Buy milk\n");
    expect(userEvents).toEqual([TOGGLE_CHECKBOX_USER_EVENT]);
    // The widget re-rendered to the new checked state.
    expect(checkboxInputs(el)[0].checked).toBe(true);
  });

  it("toggles source [X] -> [ ] on click", () => {
    const { view: v, container: el } = mount("- [X] Ship it\n");
    clickCheckbox(el);
    expect(v.state.doc.toString()).toBe("- [ ] Ship it\n");
    expect(checkboxInputs(el)[0].checked).toBe(false);
  });

  it("resolves a partial [-] to checked [X] on click", () => {
    const { view: v } = mount("- [-] Halfway\n");
    // Query fresh each time via the live container.
    const el = v.dom;
    clickCheckbox(el);
    expect(v.state.doc.toString()).toBe("- [X] Halfway\n");
  });

  it("round-trips byte-identically across two toggles", () => {
    const source = "- [ ] round trip\n";
    const { view: v } = mount(source);
    const el = v.dom;
    clickCheckbox(el); // -> [X]
    clickCheckbox(el); // -> [ ]
    expect(v.state.doc.toString()).toBe(source);
  });

  it("mutates only the clicked checkbox's char across multiple checkboxes (offset math)", () => {
    const source = ["- [ ] first", "- [ ] second", "- [ ] third"].join("\n");
    const { view: v, container: el } = mount(source);
    clickCheckbox(el, 1); // the SECOND checkbox

    expect(v.state.doc.toString()).toBe(
      ["- [ ] first", "- [X] second", "- [ ] third"].join("\n"),
    );
  });

  it("does not dispatch while the view is composing (IME guard)", () => {
    const { view: v, container: el, userEvents } = mount("- [ ] no toggle\n");
    Object.defineProperty(v, "composing", {
      get: () => true,
      configurable: true,
    });
    clickCheckbox(el);
    expect(v.state.doc.toString()).toBe("- [ ] no toggle\n");
    expect(userEvents).toEqual([]);
  });
});

describe("CheckboxWidget (LD-6 recipes)", () => {
  it("eq() compares by source range and state", () => {
    const base = new CheckboxWidget(3, 6, "empty");
    // Same range + same state → equal (DOM reused across re-renders).
    expect(base.eq(new CheckboxWidget(3, 6, "empty"))).toBe(true);
    // Same range, different state → not equal (re-render to reflect the flip).
    expect(base.eq(new CheckboxWidget(3, 6, "checked"))).toBe(false);
    // Different range → not equal.
    expect(base.eq(new CheckboxWidget(10, 13, "empty"))).toBe(false);
  });

  it("ignoreEvent() returns false (interactive widget)", () => {
    expect(new CheckboxWidget(0, 3, "empty").ignoreEvent()).toBe(false);
  });
});
