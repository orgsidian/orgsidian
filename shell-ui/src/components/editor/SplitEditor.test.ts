// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it } from "vitest";

/**
 * Story 4.4 — the Split surface factory. Direct (non-React) coverage of the
 * two-view, shared-buffer CM6 recipe:
 *  1. builds a 50/50 two-pane surface (Raw left, Pseudo-WYSIWYG right) over one
 *     document;
 *  2. an edit dispatched in EITHER pane updates the other's buffer atomically
 *     (change forwarding), while each pane keeps its own cursor;
 *  3. scroll offset is mirrored between panes;
 *  4. `destroy()` tears down both views + the wrapper and is idempotent.
 *
 * happy-dom (not jsdom) is required so CM6's `getComputedStyle` calls work.
 */

import { createSplitEditor, type SplitSurface } from "./SplitEditor";
import { ORG_HEADING_CLASS } from "./decorations/headings";

const SOURCE = "* Heading alpha\nbody text beta\nsecond body line\n";

let parent: HTMLDivElement;
let surface: SplitSurface | null;

beforeEach(() => {
  parent = document.createElement("div");
  document.body.appendChild(parent);
  surface = null;
});

afterEach(() => {
  surface?.destroy();
  parent.remove();
});

function leftPane(): HTMLElement {
  return parent.querySelector<HTMLElement>("[data-org-split-pane='raw']")!;
}
function rightPane(): HTMLElement {
  return parent.querySelector<HTMLElement>(
    "[data-org-split-pane='pseudoWysiwyg']",
  )!;
}

describe("createSplitEditor (Split mode surface)", () => {
  it("renders a 50/50 two-pane surface over the same buffer", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });

    // Both panes exist and both are flex-1 (equal 50/50 split).
    const left = leftPane();
    const right = rightPane();
    expect(left).not.toBeNull();
    expect(right).not.toBeNull();
    expect(left.className).toContain("flex-1");
    expect(right.className).toContain("flex-1");

    // Each pane is its own CM view, both showing the same source.
    expect(parent.querySelectorAll(".cm-editor")).toHaveLength(2);
    expect(left.textContent).toContain("Heading alpha");
    expect(right.textContent).toContain("Heading alpha");
    expect(surface.primaryView.state.doc.toString()).toBe(SOURCE);
    expect(surface.secondaryView.state.doc.toString()).toBe(SOURCE);
  });

  it("puts Raw extensions left (no decorations) and Pseudo-WYSIWYG right", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });

    // The decoration layer (heading hierarchy class) renders only on the right.
    expect(
      leftPane().querySelector(`.${ORG_HEADING_CLASS[0]}`),
    ).toBeNull();
    expect(
      rightPane().querySelector(`.${ORG_HEADING_CLASS[0]}`),
    ).not.toBeNull();
  });

  it("forwards an edit from the left pane to the right buffer atomically", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });
    const { primaryView, secondaryView } = surface;

    primaryView.dispatch({
      changes: { from: 0, insert: "X" },
      userEvent: "input.type",
    });

    // Both buffers reflect the same edit — one logical buffer.
    expect(primaryView.state.doc.toString()).toBe("X" + SOURCE);
    expect(secondaryView.state.doc.toString()).toBe("X" + SOURCE);
  });

  it("forwards an edit from the right pane to the left buffer atomically", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });
    const { primaryView, secondaryView } = surface;

    secondaryView.dispatch({ changes: { from: 0, insert: "Z" } });

    expect(secondaryView.state.doc.toString()).toBe("Z" + SOURCE);
    expect(primaryView.state.doc.toString()).toBe("Z" + SOURCE);
  });

  it("keeps a per-pane cursor (selection is not forwarded)", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });
    const { primaryView, secondaryView } = surface;

    // Move the left caret; the right caret must stay put (independent panes).
    primaryView.dispatch({ selection: { anchor: 5 } });
    expect(primaryView.state.selection.main.head).toBe(5);
    expect(secondaryView.state.selection.main.head).toBe(0);

    // A change from the left still lands in both buffers even though its
    // selection did not cross over.
    primaryView.dispatch({ changes: { from: 0, insert: "Q" } });
    expect(secondaryView.state.doc.toString()).toBe("Q" + SOURCE);
  });

  it("mirrors scroll offset from one pane to the other", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });
    const { primaryView, secondaryView } = surface;

    primaryView.scrollDOM.scrollTop = 120;
    primaryView.scrollDOM.dispatchEvent(new Event("scroll"));
    expect(secondaryView.scrollDOM.scrollTop).toBe(120);

    // And the reverse direction, without an infinite feedback loop.
    secondaryView.scrollDOM.scrollTop = 40;
    secondaryView.scrollDOM.dispatchEvent(new Event("scroll"));
    expect(primaryView.scrollDOM.scrollTop).toBe(40);
  });

  it("destroy() removes both views and the wrapper, and is idempotent", () => {
    surface = createSplitEditor({ parent, doc: SOURCE, baseExtensions: [] });
    expect(parent.querySelectorAll(".cm-editor")).toHaveLength(2);

    surface.destroy();
    expect(parent.querySelector("[data-org-split='true']")).toBeNull();
    expect(parent.querySelectorAll(".cm-editor")).toHaveLength(0);

    // A second teardown is a harmless no-op.
    expect(() => surface?.destroy()).not.toThrow();
  });
});
