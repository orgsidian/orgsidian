// @vitest-environment happy-dom
import { EditorSelection, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it, vi } from "vitest";

/**
 * Story 4.8 (FR-9): the Schedule/Deadline editing controller. Covers the pieces
 * the picker delegates to:
 *  - UTF-8 byte <-> UTF-16 offset conversion (so non-ASCII buffers edit the
 *    right bytes — the FR-2 round-trip hinge);
 *  - resolving the *current Headline* from the selection;
 *  - reading an existing planning value for the modify flow;
 *  - applying a backend `PlanningEdit` as one tagged, offset-correct
 *    transaction, and skipping no-ops;
 *  - routing a write through `commands.setScheduled`;
 *  - the keymap publishing picker-open requests.
 *
 * happy-dom is required for CM6's `getComputedStyle`.
 */

const setScheduled = vi.fn();
vi.mock("@/lib/tauri", () => ({
  commands: { setScheduled: (...args: unknown[]) => setScheduled(...args) },
}));

import {
  applyPlanningEdit,
  currentHeadlineId,
  currentPlanningValue,
  jsIndexToUtf8Byte,
  onPlanningRequested,
  PLANNING_USER_EVENT,
  planningKeymap,
  setPlanning,
  utf8ByteToJsIndex,
} from "./schedule";

const views: EditorView[] = [];

function makeView(doc: string, cursor = 0): EditorView {
  const state = EditorState.create({
    doc,
    selection: EditorSelection.single(cursor),
  });
  const view = new EditorView({ state, parent: document.body });
  views.push(view);
  return view;
}

afterEach(() => {
  for (const view of views.splice(0)) view.destroy();
  setScheduled.mockReset();
});

describe("offset conversion", () => {
  it("round-trips ASCII offsets 1:1", () => {
    const source = "* Task\nbody\n";
    expect(jsIndexToUtf8Byte(source, 7)).toBe(7);
    expect(utf8ByteToJsIndex(source, 7)).toBe(7);
  });

  it("bridges UTF-16 and UTF-8 for multibyte text", () => {
    // "* Café\n": é is one UTF-16 unit but two UTF-8 bytes, so the newline sits
    // at JS index 6 / byte 7, and the line-after starts at JS 7 / byte 8.
    const source = "* Café\n";
    expect(source.length).toBe(7);
    expect(jsIndexToUtf8Byte(source, 7)).toBe(8);
    expect(utf8ByteToJsIndex(source, 8)).toBe(7);
    // A byte offset past the end clamps to the string length.
    expect(utf8ByteToJsIndex(source, 999)).toBe(7);
  });
});

describe("currentHeadlineId", () => {
  it("returns the headline line at the cursor", () => {
    const view = makeView("* First\nbody\n", 0);
    expect(currentHeadlineId(view.state)).toBe(0);
  });

  it("walks up to the nearest headline above the cursor", () => {
    const doc = "* First\nbody line\n** Second\nmore\n";
    const secondStart = doc.indexOf("** Second");
    const cursor = doc.indexOf("more");
    const view = makeView(doc, cursor);
    expect(currentHeadlineId(view.state)).toBe(secondStart);
  });

  it("returns null when the cursor is in the preamble", () => {
    const view = makeView("preamble text\n* Later\n", 3);
    expect(currentHeadlineId(view.state)).toBeNull();
  });
});

describe("currentPlanningValue", () => {
  it("reads an existing scheduled date + time", () => {
    const view = makeView("* Task\nSCHEDULED: <2026-05-19 Tue 14:00>\nbody\n", 0);
    expect(currentPlanningValue(view.state, "scheduled")).toEqual({
      date: "2026-05-19",
      time: "14:00",
    });
  });

  it("scopes to the requested keyword on a shared planning line", () => {
    const view = makeView(
      "* Task\nDEADLINE: <2026-05-30 Sat> SCHEDULED: <2026-05-19 Tue>\n",
      0,
    );
    expect(currentPlanningValue(view.state, "deadline")).toEqual({
      date: "2026-05-30",
      time: null,
    });
    expect(currentPlanningValue(view.state, "scheduled")).toEqual({
      date: "2026-05-19",
      time: null,
    });
  });

  it("returns null when the keyword is absent", () => {
    const view = makeView("* Task\nSCHEDULED: <2026-05-19 Tue>\n", 0);
    expect(currentPlanningValue(view.state, "deadline")).toBeNull();
  });
});

describe("applyPlanningEdit", () => {
  it("applies a byte-offset edit as a tagged transaction", () => {
    let sawUserEvent = false;
    const observer = EditorView.updateListener.of((update) => {
      for (const tr of update.transactions) {
        if (tr.isUserEvent(PLANNING_USER_EVENT)) sawUserEvent = true;
      }
    });
    const state = EditorState.create({
      doc: "* Task\n",
      selection: EditorSelection.single(0),
      extensions: [observer],
    });
    const view = new EditorView({ state, parent: document.body });
    views.push(view);

    applyPlanningEdit(view, { from: 7, to: 7, insert: "SCHEDULED: <2026-05-19 Tue>\n" });
    expect(view.state.doc.toString()).toBe("* Task\nSCHEDULED: <2026-05-19 Tue>\n");
    expect(sawUserEvent).toBe(true);
  });

  it("converts multibyte byte offsets to document positions", () => {
    const view = makeView("* Café\n", 0);
    // Byte offset 8 == JS index 7 (line after the headline). Insert there.
    applyPlanningEdit(view, { from: 8, to: 8, insert: "DEADLINE: <2026-05-19 Tue>\n" });
    expect(view.state.doc.toString()).toBe("* Café\nDEADLINE: <2026-05-19 Tue>\n");
  });

  it("skips a genuine no-op edit", () => {
    const view = makeView("* Task\nbody\n", 0);
    const before = view.state.doc.toString();
    applyPlanningEdit(view, { from: 5, to: 5, insert: "" });
    expect(view.state.doc.toString()).toBe(before);
  });
});

describe("setPlanning", () => {
  it("sends the byte-offset headline id and applies the returned edit", async () => {
    const view = makeView("* Task\n", 0);
    setScheduled.mockResolvedValue({
      from: 7,
      to: 7,
      insert: "SCHEDULED: <2026-05-26 Tue>\n",
    });

    await setPlanning(view, "scheduled", { date: "+1w", time: null }, "2026-05-19");

    expect(setScheduled).toHaveBeenCalledTimes(1);
    expect(setScheduled).toHaveBeenCalledWith(
      "* Task\n",
      0,
      "scheduled",
      { date: "+1w", time: null },
      "2026-05-19",
    );
    expect(view.state.doc.toString()).toBe("* Task\nSCHEDULED: <2026-05-26 Tue>\n");
  });

  it("removes when value is null (timestamp omitted on the wire)", async () => {
    const view = makeView("* Task\nSCHEDULED: <2026-05-19 Tue>\nbody\n", 0);
    setScheduled.mockResolvedValue({ from: 7, to: 35, insert: "" });

    await setPlanning(view, "scheduled", null, "2026-05-19");

    expect(setScheduled.mock.calls[0][3]).toBeNull();
    expect(view.state.doc.toString()).toBe("* Task\nbody\n");
  });

  it("is a no-op when the cursor is not under a headline", async () => {
    const view = makeView("preamble\n", 0);
    await setPlanning(view, "scheduled", { date: "2026-05-19", time: null }, "2026-05-19");
    expect(setScheduled).not.toHaveBeenCalled();
  });
});

describe("planningKeymap", () => {
  it("publishes a picker-open request for each kind", () => {
    const received: string[] = [];
    const unlisten = onPlanningRequested((request) => received.push(request.kind));
    const view = makeView("* Task\n", 0);
    const bindings = planningKeymap();

    const schedule = bindings.find((b) => b.key === "Mod-Alt-s");
    const deadline = bindings.find((b) => b.key === "Mod-Alt-d");
    expect(schedule?.run?.(view)).toBe(true);
    expect(deadline?.run?.(view)).toBe(true);

    expect(received).toEqual(["scheduled", "deadline"]);
    unlisten();
  });
});
