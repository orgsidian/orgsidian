// @vitest-environment happy-dom
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";

/**
 * Story 4.3c (FR-4): tag pill labels. The suite drives a real CodeMirror 6 view
 * with the tag-pill decoration layer and asserts:
 *  1. a headline's `:tag:` / `:tag1:tag2:tag3:` suffix renders one pill per tag;
 *  2. the pill shows the bare name — colon delimiters are visually hidden;
 *  3. the buffer is byte-identical to the source (FR-2 round-trip: `replace`
 *     decorations are presentational, they never mutate the doc);
 *  4. only *headline* trailing tags render (mid-body `:foo:` does not);
 *  5. Raw mode excludes the layer entirely (no pills).
 *
 * happy-dom (not jsdom) is required so CM6's `getComputedStyle` calls work.
 */

import { modeExtensions } from "../editorMode";
import { TAG_PILL_CLASS, tagPillDecorations } from "./tags";

let view: EditorView | null = null;

afterEach(() => {
  view?.destroy();
  view = null;
  document.body.innerHTML = "";
});

/** Mount a CM6 view (attached to the DOM so it measures) with `extensions`. */
function mount(doc: string, extensions = [tagPillDecorations()]): EditorView {
  const parent = document.createElement("div");
  document.body.appendChild(parent);
  view = new EditorView({
    parent,
    state: EditorState.create({ doc, extensions }),
  });
  return view;
}

function pillLabels(v: EditorView): string[] {
  return Array.from(v.dom.querySelectorAll(`.${TAG_PILL_CLASS}`)).map(
    (el) => el.textContent ?? "",
  );
}

describe("tag pill labels (Story 4.3c)", () => {
  it("renders a single trailing tag as one pill with the colons hidden", () => {
    const src = "* Buy groceries :errand:\n";
    const v = mount(src);

    expect(pillLabels(v)).toEqual(["errand"]);
    // The pill shows the bare name — no colon delimiters.
    const pill = v.dom.querySelector(`.${TAG_PILL_CLASS}`);
    expect(pill?.textContent).not.toContain(":");
    // Source is byte-identical (colons preserved in the buffer).
    expect(v.state.doc.toString()).toBe(src);
  });

  it("renders each tag of a multi-tag suffix as its own pill", () => {
    const src = "* Plan trip :work:travel:urgent:\n";
    const v = mount(src);

    expect(pillLabels(v)).toEqual(["work", "travel", "urgent"]);
    // Every delimiter colon is absorbed by a pill: no pill text carries one.
    for (const label of pillLabels(v)) expect(label).not.toContain(":");
    expect(v.state.doc.toString()).toBe(src);
  });

  it("keeps the source byte-identical for adjacent multi-tag blocks (round-trip)", () => {
    const src = "* TODO Ship it :a:b:c:d:e:\n";
    const v = mount(src);

    expect(pillLabels(v)).toEqual(["a", "b", "c", "d", "e"]);
    expect(v.state.doc.toString()).toBe(src);
    // The rendered line still yields the exact source text when reconstructed
    // from the CM6 line DOM (widgets replace but preserve underlying content).
    expect(v.state.doc.line(1).text).toBe("* TODO Ship it :a:b:c:d:e:");
  });

  it("only decorates headline trailing tags, not mid-body `:foo:`", () => {
    // Line 1: a body paragraph mentioning `:notatag:` — not a headline.
    // Line 2: a real headline with a trailing tag.
    const src = "see :notatag: inline\n* Real headline :done:\n";
    const v = mount(src);

    expect(pillLabels(v)).toEqual(["done"]);
    expect(v.state.doc.toString()).toBe(src);
  });

  it("renders no pill for a headline without tags or with an empty `::`", () => {
    const v = mount("* Just a title\n** Another :: not a tag\n");
    expect(pillLabels(v)).toEqual([]);
  });

  it("renders no pill when the `:...:` is not at the end of the headline", () => {
    // A `:foo:` mid-headline (followed by more title text) is not a tag suffix.
    const v = mount("* Note :aside: with trailing words\n");
    expect(pillLabels(v)).toEqual([]);
  });

  it("decorates tags across multiple headlines", () => {
    const src = "* One :alpha:\n* Two :beta:gamma:\n";
    const v = mount(src);
    expect(pillLabels(v)).toEqual(["alpha", "beta", "gamma"]);
    expect(v.state.doc.toString()).toBe(src);
  });

  it("re-renders pills after an edit adds a tag (docChanged rebuild)", () => {
    const v = mount("* Task\n");
    expect(pillLabels(v)).toEqual([]);

    // Append the tag suffix to the end of the headline (before its newline).
    v.dispatch({
      changes: { from: v.state.doc.line(1).to, insert: " :new:" },
    });
    // The buffer now ends with a tag suffix; the layer rebuilt for it.
    expect(v.state.doc.toString()).toBe("* Task :new:\n");
    expect(pillLabels(v)).toEqual(["new"]);
  });

  it("excludes the tag-pill layer in Raw mode but includes it in Pseudo-WYSIWYG", () => {
    const src = "* Heading :tag:\n";

    const raw = mount(src, [modeExtensions("raw")]);
    expect(raw.dom.querySelectorAll(`.${TAG_PILL_CLASS}`)).toHaveLength(0);
    raw.destroy();
    document.body.innerHTML = "";

    const pseudo = mount(src, [modeExtensions("pseudoWysiwyg")]);
    expect(pillLabels(pseudo)).toEqual(["tag"]);
  });
});
