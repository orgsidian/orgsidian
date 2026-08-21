// @vitest-environment happy-dom
import { EditorSelection, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";

import { onLinkClicked, type LinkClicked } from "../events";
import {
  classifyLink,
  emitLinkClickFromTarget,
  ORG_LINK_CLASS,
  orgLinkDecorations,
} from "./links";

/**
 * Story 4.3f (FR-4, UJ-6): links render as clickable underlined text in
 * Pseudo-WYSIWYG mode. These tests mount a REAL CodeMirror 6 view over the four
 * AC link variants (plus the described bracket form) and assert:
 *  1. each link's rendered text carries the `cm-org-link` underline mark;
 *  2. clicking emits `LinkClicked { target, kind }` with the right kind + target;
 *  3. `[[…]]` brackets are hidden when the cursor is off the link's line and
 *     revealed (source shown) when the cursor is on it — recomputed on selection
 *     change;
 *  4. the buffer stays byte-identical (decorations never mutate source, FR-2).
 *
 * happy-dom (not jsdom) is required for CM6's `getComputedStyle` calls; its
 * unmeasured viewport reports the whole (small) doc as visible, so the
 * decoration scan over `visibleRanges` covers every line.
 */

// Off-cursor scaffolding: line 1 holds the default selection (anchor 0), so a
// link placed on a LATER line is always "off the cursor line" until we move it.
const INTRO = "intro line\n";

let views: EditorView[] = [];

afterEach(() => {
  for (const view of views) {
    view.destroy();
  }
  views = [];
});

/**
 * Mount a real CM6 view with only the link decoration layer, parented in the
 * document so CM measures/renders. Selection defaults to offset 0 (line 1).
 */
function mount(doc: string, selection?: number): EditorView {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const view = new EditorView({
    parent: container,
    state: EditorState.create({
      doc,
      selection:
        selection === undefined
          ? undefined
          : EditorSelection.single(selection),
      extensions: [orgLinkDecorations()],
    }),
  });
  views.push(view);
  return view;
}

/** The rendered text of the Nth `.cm-line` (0-based) in a view. */
function lineText(view: EditorView, n: number): string {
  const lines = view.dom.querySelectorAll(".cm-line");
  return lines[n]?.textContent ?? "";
}

/** The single link mark span (first match). */
function linkSpan(view: EditorView): HTMLElement | null {
  return view.dom.querySelector<HTMLElement>(`.${ORG_LINK_CLASS}`);
}

/**
 * Exercise the real mousedown emission path the ViewPlugin delegates to, passing
 * the rendered span as `event.target`. (Dispatching a synthetic `MouseEvent`
 * into a live CM6 view instead perturbs its DOM-selection observer under
 * happy-dom; this calls the exact same code the plugin's handler runs.)
 */
function click(el: Element): boolean {
  return emitLinkClickFromTarget(el);
}

/** Capture LinkClicked events for the duration of a test (auto-unsubscribed). */
function capture(): { events: LinkClicked[]; stop: () => void } {
  const events: LinkClicked[] = [];
  const stop = onLinkClicked((e) => events.push(e));
  return { events, stop };
}

describe("classifyLink (kind classification)", () => {
  it("classifies each recognized scheme and defaults to wiki", () => {
    expect(classifyLink("id:abc")).toBe("id");
    expect(classifyLink("file://path")).toBe("file");
    expect(classifyLink("file:notes/a.org")).toBe("file");
    expect(classifyLink("http://example.com")).toBe("http");
    expect(classifyLink("https://example.com")).toBe("http");
    expect(classifyLink("wiki-link")).toBe("wiki");
    expect(classifyLink("Some Page Title")).toBe("wiki");
  });

  it("is case-insensitive on scheme prefixes", () => {
    expect(classifyLink("ID:abc")).toBe("id");
    expect(classifyLink("FILE://p")).toBe("file");
    expect(classifyLink("HTTP://x")).toBe("http");
  });
});

describe("orgLinkDecorations — underline + kind + click", () => {
  // Each row: source path, expected underlined text, expected {target, kind}.
  const cases = [
    { doc: "[[id:abc]]", text: "id:abc", target: "id:abc", kind: "id" },
    {
      doc: "[[wiki-link]]",
      text: "wiki-link",
      target: "wiki-link",
      kind: "wiki",
    },
    {
      doc: "[[file://path]]",
      text: "file://path",
      target: "file://path",
      kind: "file",
    },
    {
      doc: "http://example.com",
      text: "http://example.com",
      target: "http://example.com",
      kind: "http",
    },
  ] as const;

  for (const c of cases) {
    it(`underlines ${c.kind} link text and emits LinkClicked on click`, () => {
      const view = mount(INTRO + c.doc, 0);
      const span = linkSpan(view);
      expect(span).not.toBeNull();
      expect(span?.textContent).toBe(c.text);

      const { events, stop } = capture();
      const emitted = click(span as HTMLElement);
      stop();

      expect(emitted).toBe(true);
      expect(events).toEqual([{ target: c.target, kind: c.kind }]);
    });
  }

  it("keeps the buffer byte-identical (decorations never mutate source)", () => {
    const source = `${INTRO}[[id:abc]] and http://example.com and [[wiki-link]]`;
    const view = mount(source, 0);
    expect(view.state.doc.toString()).toBe(source);
  });

  it("renders every variant on one line as its own underline mark", () => {
    const view = mount(`${INTRO}[[id:abc]] [[wiki]] http://x.test`, 0);
    const spans = Array.from(
      view.dom.querySelectorAll(`.${ORG_LINK_CLASS}`),
    ).map((el) => el.textContent);
    expect(spans).toEqual(["id:abc", "wiki", "http://x.test"]);
  });
});

describe("orgLinkDecorations — bracket reveal on cursor line", () => {
  it("hides [[ ]] when the cursor is not on the link's line", () => {
    const view = mount(`${INTRO}[[wiki-link]]`, 0); // cursor on line 1
    // Line 2 (index 1) renders only the underlined path — brackets collapsed.
    expect(lineText(view, 1)).toBe("wiki-link");
    expect(linkSpan(view)?.textContent).toBe("wiki-link");
  });

  it("reveals [[ ]] when the cursor is on the link's line", () => {
    const doc = `${INTRO}[[wiki-link]]`;
    const linkLineFrom = EditorState.create({ doc }).doc.line(2).from;
    const view = mount(doc, linkLineFrom + 3); // cursor inside line 2
    // Source shown verbatim, brackets visible.
    expect(lineText(view, 1)).toBe("[[wiki-link]]");
  });

  it("recomputes reveal when the selection moves onto the link line", () => {
    const doc = `${INTRO}[[wiki-link]]`;
    const view = mount(doc, 0); // start off the link line → hidden
    expect(lineText(view, 1)).toBe("wiki-link");

    // Move the cursor onto line 2: selectionSet must rebuild → brackets reveal.
    const linkLineFrom = view.state.doc.line(2).from;
    view.dispatch({ selection: EditorSelection.single(linkLineFrom + 2) });
    expect(lineText(view, 1)).toBe("[[wiki-link]]");

    // And back off the line → hidden again.
    view.dispatch({ selection: EditorSelection.single(0) });
    expect(lineText(view, 1)).toBe("wiki-link");
  });

  it("a bare URL has no brackets to toggle and stays underlined on/off line", () => {
    const doc = `${INTRO}http://example.com`;
    const view = mount(doc, 0);
    expect(lineText(view, 1)).toBe("http://example.com");
    const linkLineFrom = view.state.doc.line(2).from;
    view.dispatch({ selection: EditorSelection.single(linkLineFrom + 2) });
    expect(lineText(view, 1)).toBe("http://example.com");
    expect(linkSpan(view)?.textContent).toBe("http://example.com");
  });
});

describe("orgLinkDecorations — described link form [[target][label]]", () => {
  const doc = `${INTRO}[[id:node-1][Read me]]`;

  it("underlines the label, hides target+brackets off-line, emits raw target", () => {
    const view = mount(doc, 0); // off the link line
    const span = linkSpan(view);
    expect(span?.textContent).toBe("Read me"); // label underlined, not target
    expect(lineText(view, 1)).toBe("Read me"); // `[[id:node-1][` and `]]` hidden

    const { events, stop } = capture();
    click(span as HTMLElement);
    stop();
    // Click target is the raw path (before `][label]`), classified as an id link.
    expect(events).toEqual([{ target: "id:node-1", kind: "id" }]);
  });

  it("reveals the full source when the cursor is on the link line", () => {
    const linkLineFrom = EditorState.create({ doc }).doc.line(2).from;
    const view = mount(doc, linkLineFrom + 4);
    expect(lineText(view, 1)).toBe("[[id:node-1][Read me]]");
  });
});

describe("orgLinkDecorations — click guards & event surface", () => {
  it("stops delivering to an unsubscribed listener", () => {
    const view = mount(`${INTRO}[[wiki-link]]`, 0);
    const { events, stop } = capture();
    stop(); // unsubscribe before any click
    click(linkSpan(view) as Element);
    expect(events).toHaveLength(0);
  });

  it("does not emit for a mousedown outside any link", () => {
    const view = mount(`${INTRO}[[wiki-link]]`, 0);
    const introLine = view.dom.querySelectorAll(".cm-line")[0] as Element;
    const { events, stop } = capture();
    const emitted = click(introLine); // plain text, no link ancestor
    stop();
    expect(emitted).toBe(false);
    expect(events).toHaveLength(0);
  });

  it("does not emit for a null / non-Element mousedown target", () => {
    const { events, stop } = capture();
    const emitted = emitLinkClickFromTarget(null);
    stop();
    expect(emitted).toBe(false);
    expect(events).toHaveLength(0);
  });
});
