// @vitest-environment happy-dom
import { EditorSelection } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";

/**
 * Story 4.3d: org timestamps render as human-readable dates with a
 * hover-for-source tooltip in Pseudo-WYSIWYG mode. The suite realizes the ACs:
 *  - a `Decoration.replace` widget with a locale-formatted date + time;
 *  - active `<…>` vs inactive `[…]` visually distinct (distinct classes);
 *  - the raw source revealed on hover (tooltip source) and preserved
 *    byte-identically in the buffer (FR-2);
 *  - the caret revealing raw source (no atomic trap → source-position fidelity).
 *
 * happy-dom is required for CM6's `getComputedStyle`.
 */

import {
  formatTimestamp,
  ORG_TIMESTAMP_CLASS,
  TIMESTAMP_HOVER_MS,
  TimestampWidget,
  timestampDecorations,
  timestampTooltipSource,
} from "./timestamps";

// Locale-independent expected label (mirrors the module's formatter options) so
// the assertions hold whatever locale the test runner defaults to.
function expectedDate(year: number, month: number, day: number): string {
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(new Date(Date.UTC(year, month - 1, day)));
}

// `* Meeting\n` = 10 bytes; the active stamp begins at offset 10.
const FIXTURE = "* Meeting\n<2026-05-19 Tue 14:00> draft [2026-05-18 Mon]\n";
const ACTIVE_RAW = "<2026-05-19 Tue 14:00>";
const INACTIVE_RAW = "[2026-05-18 Mon]";
const ACTIVE_START = FIXTURE.indexOf(ACTIVE_RAW);
const INACTIVE_START = FIXTURE.indexOf(INACTIVE_RAW);

let view: EditorView | undefined;

function mount(doc: string): EditorView {
  const parent = document.createElement("div");
  document.body.appendChild(parent);
  view = new EditorView({ parent, doc, extensions: [timestampDecorations()] });
  return view;
}

afterEach(() => {
  view?.destroy();
  view = undefined;
  document.body.innerHTML = "";
});

function widgets(v: EditorView): HTMLElement[] {
  return Array.from(v.dom.querySelectorAll<HTMLElement>(`.${ORG_TIMESTAMP_CLASS.base}`));
}

describe("formatTimestamp", () => {
  it("renders a locale-formatted date + source time for an active stamp", () => {
    // 2026-05-19 is a Tuesday — the label uses the COMPUTED weekday, not the
    // source day-name ("Tue" here happens to agree; see the stale-day-name test).
    expect(formatTimestamp("<2026-05-19 Tue 14:00>")).toBe(
      `${expectedDate(2026, 5, 19)} · 14:00`,
    );
  });

  it("renders date only when the stamp carries no clock time", () => {
    expect(formatTimestamp("[2026-05-18 Mon]")).toBe(expectedDate(2026, 5, 18));
  });

  it("keeps a clock range verbatim", () => {
    expect(formatTimestamp("<2026-06-10 Wed 10:00-11:00>")).toBe(
      `${expectedDate(2026, 6, 10)} · 10:00-11:00`,
    );
  });

  it("computes the weekday from the date, ignoring a stale source day-name", () => {
    // Source claims "Mon" but 2026-05-19 is a Tuesday: the reader must not be
    // misled — the label self-corrects while the raw stays available on hover.
    expect(formatTimestamp("<2026-05-19 Mon 14:00>")).toContain(expectedDate(2026, 5, 19));
    expect(formatTimestamp("<2026-05-19 Mon 14:00>")).not.toContain("Mon");
  });

  it("returns null for an impossible date (leaves raw source visible)", () => {
    expect(formatTimestamp("<2026-13-40 Xxx>")).toBeNull();
    expect(formatTimestamp("[2026-02-30]")).toBeNull();
  });
});

describe("timestamp decorations (Pseudo-WYSIWYG)", () => {
  it("replaces active and inactive stamps with formatted-date widgets", () => {
    const v = mount(FIXTURE);
    const rendered = widgets(v);
    expect(rendered).toHaveLength(2);

    const [active, inactive] = rendered;
    expect(active.textContent).toBe(`${expectedDate(2026, 5, 19)} · 14:00`);
    expect(inactive.textContent).toBe(expectedDate(2026, 5, 18));
  });

  it("marks active vs inactive stamps with distinct classes", () => {
    const v = mount(FIXTURE);
    const [active, inactive] = widgets(v);

    expect(active.classList.contains(ORG_TIMESTAMP_CLASS.active)).toBe(true);
    expect(active.classList.contains(ORG_TIMESTAMP_CLASS.inactive)).toBe(false);
    expect(active.getAttribute("data-org-timestamp")).toBe("active");

    expect(inactive.classList.contains(ORG_TIMESTAMP_CLASS.inactive)).toBe(true);
    expect(inactive.classList.contains(ORG_TIMESTAMP_CLASS.active)).toBe(false);
    expect(inactive.getAttribute("data-org-timestamp")).toBe("inactive");
  });

  it("keeps the buffer byte-identical (source preserved, FR-2)", () => {
    const v = mount(FIXTURE);
    expect(v.state.doc.toString()).toBe(FIXTURE);
    // The widget carries the exact source slice for round-trip + a11y.
    expect(widgets(v)[0].getAttribute("data-org-timestamp-raw")).toBe(ACTIVE_RAW);
  });

  it("renders no widget for an impossible date, leaving raw source", () => {
    const v = mount("bad <2026-13-40 Xxx> stamp\n");
    expect(widgets(v)).toHaveLength(0);
    expect(v.dom.textContent).toContain("<2026-13-40 Xxx>");
  });

  it("reveals raw source when the caret touches a stamp (no atomic trap)", () => {
    const v = mount(FIXTURE);
    expect(widgets(v)).toHaveLength(2);

    // Move the caret inside the active stamp: its widget must disappear so the
    // source is editable, while the untouched inactive stamp stays rendered.
    v.dispatch({ selection: EditorSelection.cursor(ACTIVE_START + 3) });
    const remaining = widgets(v);
    expect(remaining).toHaveLength(1);
    expect(remaining[0].getAttribute("data-org-timestamp")).toBe("inactive");
    expect(v.dom.textContent).toContain(ACTIVE_RAW);
  });
});

describe("hover-for-source tooltip", () => {
  it("gates the tooltip behind a >300ms hover dwell", () => {
    expect(TIMESTAMP_HOVER_MS).toBe(300);
  });

  it("returns a tooltip with the exact raw source for a position inside a stamp", () => {
    const v = mount(FIXTURE);
    const tip = timestampTooltipSource(v, ACTIVE_START + 3);
    expect(tip).not.toBeNull();
    expect(tip?.pos).toBe(ACTIVE_START);
    expect(tip?.end).toBe(ACTIVE_START + ACTIVE_RAW.length);
    const dom = tip?.create(v).dom;
    expect(dom?.textContent).toBe(ACTIVE_RAW);
    expect(dom?.className).toBe(ORG_TIMESTAMP_CLASS.tooltip);

    // The inactive stamp resolves to its own raw source.
    expect(timestampTooltipSource(v, INACTIVE_START + 2)?.create(v).dom.textContent).toBe(
      INACTIVE_RAW,
    );
  });

  it("returns null when the position is not over a stamp", () => {
    const v = mount(FIXTURE);
    expect(timestampTooltipSource(v, 2)).toBeNull();
  });
});

describe("TimestampWidget.eq (LD-6 shallow-equal by source)", () => {
  it("is equal for the same raw + active, unequal otherwise", () => {
    const a = new TimestampWidget(ACTIVE_RAW, true, "label");
    const b = new TimestampWidget(ACTIVE_RAW, true, "label");
    const differentRaw = new TimestampWidget("<2026-05-20 Wed>", true, "label");
    const differentKind = new TimestampWidget(ACTIVE_RAW, false, "label");

    expect(a.eq(b)).toBe(true);
    expect(a.eq(differentRaw)).toBe(false);
    expect(a.eq(differentKind)).toBe(false);
  });

  it("ignores widget events so the editor keeps handling them", () => {
    expect(new TimestampWidget(ACTIVE_RAW, true, "label").ignoreEvent()).toBe(true);
  });
});
