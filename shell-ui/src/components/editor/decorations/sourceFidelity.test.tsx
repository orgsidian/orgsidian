// @vitest-environment happy-dom
import { EditorView } from "@codemirror/view";
import { EditorSelection, type SelectionRange } from "@codemirror/state";
import { afterEach, describe, expect, it } from "vitest";

/**
 * Story 4.3g — source-position fidelity (cursor, copy-paste, find/replace).
 *
 * The suite mounts a real CM6 `EditorView` (happy-dom) with EVERY Story 4.3a–4.3f
 * decoration active (`modeExtensions("pseudoWysiwyg")`) PLUS the source-fidelity
 * find/replace stack (`sourceFidelity()`), over one document that contains a
 * heading, a TODO keyword, tag pills, an active + inactive timestamp, checkboxes,
 * and links — so every decoration kind is in view at once. It then proves the
 * FR-3 / FR-4 contract:
 *
 *   - copy (real `copy` ClipboardEvent) writes SOURCE text, not the rendered
 *     widgets — a heading line copies `** …` (source stars), the TODO line
 *     copies the `TODO` keyword and the `:work:urgent:` colons, and select-all
 *     copies the buffer byte-for-byte;
 *   - find (`SearchCursor`) locates SOURCE offsets even where the source is
 *     hidden behind a widget — `TODO` under the badge, `2026-05-19` under the
 *     human-readable timestamp (a string that does NOT appear in the rendered
 *     DOM at all), the `**` heading stars;
 *   - replace (`replaceAll` command) writes at SOURCE offsets and leaves the rest
 *     byte-identical, and the decoration layer re-renders over the new source;
 *   - paste (real `paste` ClipboardEvent) inserts at the SOURCE offset;
 *   - the buffer round-trips byte-identically with all decorations mounted.
 */

import { modeExtensions } from "../editorMode";
import {
  sourceFidelity,
  SearchCursor,
  SearchQuery,
  setSearchQuery,
  replaceAll,
} from "./sourceFidelity";

// One document exercising all six decoration kinds simultaneously.
const SRC = [
  "#+TODO: TODO NEXT WAITING | DONE",
  "* TODO Buy milk :work:urgent:",
  "Body with http://example.com and [[id:abc][a link]].",
  "** DONE Ship it <2026-05-19 Tue 14:00>",
  "- [ ] pending task",
  "- [X] finished task",
  "*** NEXT Deep dive [2026-01-02 Fri]",
  "",
].join("\n");

let view: EditorView | null = null;

afterEach(() => {
  view?.destroy();
  view = null;
});

/** Mount a Pseudo-WYSIWYG view with all decorations + find/replace. */
function mount(doc: string = SRC): EditorView {
  const v = new EditorView({
    doc,
    parent: document.body,
    extensions: [modeExtensions("pseudoWysiwyg"), sourceFidelity()],
  });
  view = v;
  return v;
}

/**
 * Set the selection then focus, in that order. CM6's copy/paste handlers no-op
 * unless the live DOM selection is inside `contentDOM`; focusing AFTER the
 * selection dispatch syncs the DOM selection without re-entering the in-progress
 * update (happy-dom fires `selectionchange` synchronously, so focusing first
 * would re-enter CM6's dispatch).
 */
function selectThenFocus(v: EditorView, sel: EditorSelection | SelectionRange): void {
  v.dispatch({ selection: sel });
  v.focus();
}

/** 1-based line number whose text starts with `prefix`. */
function lineStarting(v: EditorView, prefix: string): number {
  for (let n = 1; n <= v.state.doc.lines; n += 1) {
    if (v.state.doc.line(n).text.startsWith(prefix)) return n;
  }
  throw new Error(`no line starts with ${JSON.stringify(prefix)}`);
}

/** Select [from,to], fire a real `copy` event, return the clipboard text. */
function copySelection(v: EditorView, from: number, to: number): string {
  selectThenFocus(v, EditorSelection.range(from, to));
  const dt = new DataTransfer();
  const ev = new ClipboardEvent("copy", {
    clipboardData: dt,
    bubbles: true,
    cancelable: true,
  });
  v.contentDOM.dispatchEvent(ev);
  return ev.clipboardData?.getData("text/plain") ?? "";
}

/** Every source offset where `term` occurs, via the CM6 search cursor. */
function findAll(v: EditorView, term: string): number[] {
  const hits: number[] = [];
  const cursor = new SearchCursor(v.state.doc, term);
  while (!cursor.next().done) hits.push(cursor.value.from);
  return hits;
}

describe("source fidelity — round-trip with all decorations", () => {
  it("keeps the buffer byte-identical to the source", () => {
    const v = mount();
    expect(v.state.doc.toString()).toBe(SRC);
  });

  it("actually renders decorations (rendered text differs from source)", () => {
    const v = mount();
    // Sanity: the timestamp source `2026-05-19` is HIDDEN behind the rendered
    // human-readable date, so the distinction the suite relies on is real.
    expect(v.contentDOM.textContent).not.toContain("2026-05-19");
  });
});

describe("copy reads source offsets, not rendered text", () => {
  it("copies a heading line as source (`** DONE Ship it <…>`)", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "** DONE"));
    const copied = copySelection(v, line.from, line.to);
    expect(copied).toBe("** DONE Ship it <2026-05-19 Tue 14:00>");
    // Source stars + raw timestamp, none of the rendered widget labels.
    expect(copied.startsWith("** ")).toBe(true);
    expect(copied).toContain("<2026-05-19 Tue 14:00>");
  });

  it("copies the TODO keyword and tag colons from a headline (source)", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "* TODO"));
    const copied = copySelection(v, line.from, line.to);
    expect(copied).toBe("* TODO Buy milk :work:urgent:");
    expect(copied).toContain("TODO"); // keyword, not the badge
    expect(copied).toContain(":work:urgent:"); // colons, not just pills
  });

  it("copies a checkbox line as source (`- [ ] …`)", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "- [ ]"));
    const copied = copySelection(v, line.from, line.to);
    expect(copied).toBe("- [ ] pending task");
  });

  it("copies a link line as source, brackets and all", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "Body with"));
    const copied = copySelection(v, line.from, line.to);
    expect(copied).toBe("Body with http://example.com and [[id:abc][a link]].");
  });

  it("select-all copies the whole buffer byte-for-byte", () => {
    const v = mount();
    const copied = copySelection(v, 0, v.state.doc.length);
    expect(copied).toBe(SRC);
  });
});

describe("cursor placement uses source offsets, not rendered positions", () => {
  it("a caret set inside a widget's source range is preserved as that source offset", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "* TODO"));
    // An offset strictly INSIDE the `TODO` keyword (`* T|ODO …`) — visually this
    // sits within the rendered badge, but it is a real source character position.
    const inside = line.from + 3;
    v.dispatch({ selection: EditorSelection.cursor(inside) });
    expect(v.state.selection.main.head).toBe(inside);
    // The source character at that offset is the keyword's, not any rendered text.
    expect(v.state.sliceDoc(line.from, line.from + 6)).toBe("* TODO");
  });

  it("a selection spanning a widget yields the source slice, not the rendered label", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "** DONE"));
    // Select across the human-readable timestamp widget; the slice is the raw
    // source stamp, never the rendered "Tue, May 19 · 14:00".
    v.dispatch({
      selection: EditorSelection.range(line.from, line.to),
    });
    const { from, to } = v.state.selection.main;
    expect(v.state.sliceDoc(from, to)).toBe(
      "** DONE Ship it <2026-05-19 Tue 14:00>",
    );
  });
});

describe("find operates on source offsets, not rendered positions", () => {
  it("finds the `TODO` keyword at the headline's source offset (under the badge)", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "* TODO"));
    const keywordOffset = line.from + 2; // after `* `
    const hits = findAll(v, "TODO");
    expect(hits).toContain(keywordOffset);
  });

  it("finds `2026-05-19` even though the rendered timestamp hides it", () => {
    const v = mount();
    // The string exists ONLY in the source (rendered as "Tue, May 19 · 14:00").
    expect(v.contentDOM.textContent).not.toContain("2026-05-19");
    const hits = findAll(v, "2026-05-19");
    expect(hits).toHaveLength(1);
    const line = v.state.doc.line(lineStarting(v, "** DONE"));
    expect(hits[0]).toBeGreaterThan(line.from);
  });

  it("finds the `**` heading stars in the source", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "** DONE"));
    const hits = findAll(v, "** DONE");
    expect(hits).toContain(line.from);
  });
});

describe("replace writes at source offsets", () => {
  it("replaceAll rewrites the source and the badge re-renders over it", () => {
    const v = mount();
    v.dispatch({
      effects: setSearchQuery.of(
        new SearchQuery({ search: "TODO Buy", replace: "DONE Buy" }),
      ),
    });
    const ok = replaceAll(v);
    expect(ok).toBe(true);

    const doc = v.state.doc.toString();
    // The headline keyword is rewritten at its source offset; everything else
    // (the `#+TODO:` directive, tags, other lines) is byte-identical.
    expect(doc).toContain("* DONE Buy milk :work:urgent:");
    expect(doc).toBe(SRC.replace("* TODO Buy milk", "* DONE Buy milk"));

    // The TODO badge decoration recomputed over the NEW source: a DONE badge now
    // renders for that headline.
    const badges = Array.from(
      v.dom.querySelectorAll<HTMLElement>(".org-todo-badge"),
    ).map((el) => el.textContent);
    expect(badges).toContain("DONE");
  });
});

describe("paste inserts at the source offset", () => {
  it("inserts pasted text at the caret's source offset inside a decorated line", () => {
    const v = mount();
    const line = v.state.doc.line(lineStarting(v, "* TODO"));
    // Caret just before the tag block's leading colon (a source offset that
    // sits amid tag-pill widgets).
    const caret = line.to - ":work:urgent:".length;
    selectThenFocus(v, EditorSelection.cursor(caret));

    const dt = new DataTransfer();
    dt.setData("text/plain", "X");
    const ev = new ClipboardEvent("paste", {
      clipboardData: dt,
      bubbles: true,
      cancelable: true,
    });
    v.contentDOM.dispatchEvent(ev);

    // Inserted exactly at the source offset; the rest of the buffer is intact.
    const expected = SRC.replace(":work:urgent:", "X:work:urgent:");
    expect(v.state.doc.toString()).toBe(expected);
  });
});
