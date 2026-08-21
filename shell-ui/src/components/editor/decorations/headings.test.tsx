// @vitest-environment happy-dom
import { EditorSelection, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { headingDecorations, ORG_HEADING_CLASS } from "./headings";

/**
 * Story 4.3a (FR-4): heading-hierarchy line decorations for Pseudo-WYSIWYG
 * mode. These tests mount a REAL CodeMirror 6 view over an org fixture and
 * assert:
 *  1. each headline `*`…`******` gets a `cm-org-heading-h{1..6}` LINE decoration;
 *  2. the computed `font-size` decreases monotonically from h1 to h6
 *     (observable via `getComputedStyle` — the AC's measurable contract);
 *  3. the rendered source text is byte-identical to the buffer (FR-2 round-trip
 *     contract — decorations never mutate source);
 *  4. non-headlines (body text, emphasis `*bold*`) get no heading decoration;
 *  5. deeper-than-6 headlines clamp to h6 (no crash, still a valid class).
 *
 * happy-dom (not jsdom) is required so `getComputedStyle` resolves the CM6
 * theme's `em` font sizes to px.
 */

// One headline per level h1..h6, a body line, and an emphasis line.
const SOURCE = [
  "* H1 top",
  "** H2 two",
  "*** H3 three",
  "**** H4 four",
  "***** H5 five",
  "****** H6 six",
  "plain body text, not a heading",
  "*bold* not a heading either",
].join("\n");

let container: HTMLDivElement;
let view: EditorView;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  view = new EditorView({
    parent: container,
    state: EditorState.create({
      doc: SOURCE,
      extensions: [headingDecorations()],
    }),
  });
});

afterEach(() => {
  view.destroy();
  container.remove();
});

/** The `.cm-line` element carrying `cls`, or null. */
function lineWithClass(cls: string): Element | null {
  return container.querySelector(`.cm-line.${cls}`);
}

describe("headingDecorations (Pseudo-WYSIWYG heading hierarchy)", () => {
  it("decorates each headline h1..h6 with its per-level line class", () => {
    for (let level = 1; level <= 6; level++) {
      const cls = ORG_HEADING_CLASS[level - 1];
      const el = lineWithClass(cls);
      expect(el, `expected a .cm-line.${cls}`).not.toBeNull();
      // The decorated element is the line block, and its text is the source
      // line verbatim (leading stars preserved — no source rewrite).
      expect(el?.textContent).toBe(SOURCE.split("\n")[level - 1]);
    }
  });

  it("computes font-size monotonically decreasing from h1 to h6", () => {
    const sizes = ORG_HEADING_CLASS.map((cls) => {
      const el = lineWithClass(cls);
      expect(el, `missing .cm-line.${cls}`).not.toBeNull();
      return parseFloat(getComputedStyle(el as Element).fontSize);
    });
    // Every value is a real px number...
    for (const px of sizes) {
      expect(Number.isFinite(px)).toBe(true);
      expect(px).toBeGreaterThan(0);
    }
    // ...and strictly decreasing h1 > h2 > … > h6.
    for (let i = 1; i < sizes.length; i++) {
      expect(sizes[i - 1]).toBeGreaterThan(sizes[i]);
    }
  });

  it("preserves the source text byte-identically (round-trip / FR-2)", () => {
    // The buffer is unchanged by decorations.
    expect(view.state.doc.toString()).toBe(SOURCE);
    // And the rendered lines reconstruct the source exactly.
    const rendered = Array.from(container.querySelectorAll(".cm-line"))
      .map((line) => line.textContent ?? "")
      .join("\n");
    expect(rendered).toBe(SOURCE);
  });

  it("does not decorate body text or non-headline emphasis asterisks", () => {
    // Neither the plain body line nor `*bold*` carries any heading class.
    const decorated = container.querySelectorAll(
      ORG_HEADING_CLASS.map((c) => `.cm-line.${c}`).join(","),
    );
    // Exactly the six headlines are decorated — nothing more.
    expect(decorated).toHaveLength(6);
    for (const el of Array.from(decorated)) {
      expect(el.textContent?.startsWith("*")).toBe(true);
      // `*bold*` starts with `*` but has no space after the stars, so it must
      // not be among the decorated set.
      expect(el.textContent).not.toContain("bold");
    }
  });

  it("clamps headlines deeper than 6 stars to h6", () => {
    const deep = new EditorView({
      parent: document.createElement("div"),
      state: EditorState.create({
        doc: "******* H7 seven",
        extensions: [headingDecorations()],
      }),
    });
    // Level clamps to h6 — present, and no phantom h7 class.
    expect(deep.dom.querySelector(".cm-line.cm-org-heading-h6")).not.toBeNull();
    expect(deep.dom.querySelector(".cm-line.cm-org-heading-h7")).toBeNull();
    // Source still byte-identical.
    expect(deep.state.doc.toString()).toBe("******* H7 seven");
    deep.destroy();
  });

  it("keeps decorations in sync (and source faithful) after an edit", () => {
    // Promote the body line (index 6) into an H2 by inserting `** ` at its start.
    const bodyLine = view.state.doc.line(7);
    view.dispatch({
      changes: { from: bodyLine.from, insert: "** " },
      // A programmatic edit still routes through a normal transaction; the
      // decoration ViewPlugin recomputes on docChanged (no dispatch of its own).
      selection: EditorSelection.cursor(bodyLine.from),
    });
    // The previously-plain line is now an h2 headline, decorated as such...
    const promoted = Array.from(container.querySelectorAll(".cm-line")).find(
      (el) => el.textContent === "** plain body text, not a heading",
    );
    expect(promoted).toBeDefined();
    expect(promoted?.classList.contains("cm-org-heading-h2")).toBe(true);
    // ...and the buffer reflects exactly the inserted characters, nothing else.
    expect(view.state.doc.line(7).text).toBe(
      "** plain body text, not a heading",
    );
  });
});
