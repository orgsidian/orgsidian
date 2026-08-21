// @vitest-environment happy-dom
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ORG_TOKEN_CLASS, orgSyntaxHighlight } from "./orgLanguage";

/**
 * Story 4.2 (FR-3): the org syntax-highlight layer for Raw mode. These tests
 * mount a REAL CodeMirror 6 view over an org fixture and assert:
 *  1. each org construct the AC enumerates (headline asterisks, TODO/DONE
 *     keywords, tags, active/inactive timestamps) is emitted as a highlighted
 *     token span carrying its stable `cm-org-*` class;
 *  2. the rendered source text is byte-faithful (highlighting is presentational);
 *  3. NO Pseudo-WYSIWYG decoration/widget nodes are produced (Raw = tokens only).
 *
 * happy-dom (not jsdom) is required for CM6's `getComputedStyle` calls.
 */

// A fixture exercising every tokenized construct.
const SOURCE = [
  "* TODO Buy milk :errand:home:",
  "** DONE Ship release <2026-05-19 Mon 14:00>",
  "Some body text with an inactive [2026-05-19 Mon] stamp.",
  "*** NEXT Draft the proposal",
  "**** WAITING On review",
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
      extensions: [orgSyntaxHighlight()],
    }),
  });
});

afterEach(() => {
  view.destroy();
  container.remove();
});

/** Collect the text content of every span carrying `cls`. */
function tokenTexts(cls: string): string[] {
  return Array.from(container.querySelectorAll(`.${cls}`)).map(
    (el) => el.textContent ?? "",
  );
}

describe("orgSyntaxHighlight (Raw mode token layer)", () => {
  it("renders the source text byte-faithfully", () => {
    // The concatenated rendered text equals the source (CM6 renders line by
    // line; join with \n to reconstruct).
    const rendered = Array.from(container.querySelectorAll(".cm-line"))
      .map((line) => line.textContent ?? "")
      .join("\n");
    expect(rendered).toBe(SOURCE);
  });

  it("highlights headline asterisks as heading-stars tokens", () => {
    const stars = tokenTexts(ORG_TOKEN_CLASS.headingStars);
    // One token per headline (4 headlines), each the run of leading asterisks,
    // in document order. The body-text line contributes none.
    expect(stars).toEqual(["*", "**", "***", "****"]);
  });

  it("highlights TODO-family and DONE state keywords distinctly", () => {
    expect(tokenTexts(ORG_TOKEN_CLASS.todoKeyword).sort()).toEqual([
      "NEXT",
      "TODO",
      "WAITING",
    ]);
    expect(tokenTexts(ORG_TOKEN_CLASS.doneKeyword)).toEqual(["DONE"]);
  });

  it("highlights tags (colon-delimited) as tag tokens", () => {
    expect(tokenTexts(ORG_TOKEN_CLASS.tag)).toEqual([":errand:home:"]);
  });

  it("highlights active and inactive timestamps distinctly", () => {
    expect(tokenTexts(ORG_TOKEN_CLASS.timestampActive)).toEqual([
      "<2026-05-19 Mon 14:00>",
    ]);
    expect(tokenTexts(ORG_TOKEN_CLASS.timestampInactive)).toEqual([
      "[2026-05-19 Mon]",
    ]);
  });

  it("renders NO Pseudo-WYSIWYG decoration or widget nodes (tokens only)", () => {
    // CM6 widget decorations insert `.cm-widgetBuffer` markers and replaced
    // ranges; Raw mode must produce none.
    expect(container.querySelectorAll(".cm-widgetBuffer")).toHaveLength(0);
    // No decoration ViewPlugin output either (reserved future class).
    expect(container.querySelectorAll("[class*='org-decoration']")).toHaveLength(
      0,
    );
  });

  it("does not highlight a non-headline emphasis asterisk or a bare TODO word", () => {
    const v = new EditorView({
      parent: document.createElement("div"),
      state: EditorState.create({
        // `*bold*` is emphasis (no space after `*`), and TODO mid-title is not
        // in the keyword slot.
        doc: "*bold* text\n* Heading with TODO in title",
        extensions: [orgSyntaxHighlight()],
      }),
    });
    const root = v.dom;
    // The leading `*` of `*bold*` is not a heading-stars token.
    const starTokens = Array.from(
      root.querySelectorAll(`.${ORG_TOKEN_CLASS.headingStars}`),
    ).map((el) => el.textContent);
    expect(starTokens).toEqual(["*"]); // only the real headline on line 2
    // "TODO" inside the title is not a keyword token.
    expect(root.querySelectorAll(`.${ORG_TOKEN_CLASS.todoKeyword}`)).toHaveLength(
      0,
    );
    v.destroy();
  });
});
