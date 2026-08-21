// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 4.8 (FR-9): the Schedule/Deadline date picker. The suite realizes the
 * ACs a picker component can own in isolation:
 *  - opens with a calendar, a time field, and `+1d`/`+1w` relative shortcuts;
 *  - Enter commits, Esc cancels (Fantastical-style);
 *  - confirming hands back a `{ date, time }` value (the editor controller
 *    routes the actual write through `commands.setScheduled`);
 *  - opening on an existing value pre-fills the selection (modify flow).
 */

import {
  OrgDatePicker,
  ORG_DATE_PICKER_CLASS,
  type OrgDatePickerValue,
} from "./OrgDatePicker";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

// A fixed "today" so the calendar and shortcuts are deterministic: 2026-05-19
// is a Tuesday (month is 0-based in the Date constructor).
const TODAY = new Date(2026, 4, 19);

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

interface RenderOptions {
  kind?: "scheduled" | "deadline";
  today?: Date;
  initial?: OrgDatePickerValue | null;
}

function renderPicker(options: RenderOptions = {}) {
  const onConfirm = vi.fn<(value: OrgDatePickerValue) => void>();
  const onCancel = vi.fn<() => void>();
  act(() => {
    root.render(
      <OrgDatePicker
        kind={options.kind ?? "scheduled"}
        today={options.today ?? TODAY}
        initial={options.initial ?? null}
        onConfirm={onConfirm}
        onCancel={onCancel}
      />,
    );
  });
  return { onConfirm, onCancel };
}

function pickerRoot(): HTMLElement {
  const el = container.querySelector(`.${ORG_DATE_PICKER_CLASS.root}`);
  if (el === null) throw new Error("picker root not found");
  return el as HTMLElement;
}

function click(selector: string) {
  const el = container.querySelector(selector) as HTMLElement | null;
  if (el === null) throw new Error(`element not found: ${selector}`);
  act(() => {
    el.click();
  });
}

describe("OrgDatePicker", () => {
  it("labels itself by kind and offers a calendar, time field, and shortcuts", () => {
    renderPicker({ kind: "deadline" });
    const rootEl = pickerRoot();
    expect(rootEl.getAttribute("aria-label")).toBe("Set Deadline");
    expect(container.querySelector(`.${ORG_DATE_PICKER_CLASS.grid}`)).not.toBeNull();
    expect(container.querySelector('input[type="time"]')).not.toBeNull();
    expect(container.querySelector('[data-shortcut="+1d"]')).not.toBeNull();
    expect(container.querySelector('[data-shortcut="+1w"]')).not.toBeNull();
    expect(container.querySelector('[data-shortcut="today"]')).not.toBeNull();
  });

  it("defaults the selection to today and confirms it via the button", () => {
    const { onConfirm } = renderPicker();
    click('[data-action="confirm"]');
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm.mock.calls[0][0]).toEqual<OrgDatePickerValue>({
      date: "2026-05-19",
      time: null,
    });
  });

  it("advances the selection a week with the +1w shortcut", () => {
    const { onConfirm } = renderPicker();
    click('[data-shortcut="+1w"]');
    click('[data-action="confirm"]');
    expect(onConfirm.mock.calls[0][0]).toEqual({ date: "2026-05-26", time: null });
  });

  it("advances a day with +1d and commits on Enter", () => {
    const { onConfirm } = renderPicker();
    click('[data-shortcut="+1d"]');
    act(() => {
      pickerRoot().dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      );
    });
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm.mock.calls[0][0]).toEqual({ date: "2026-05-20", time: null });
  });

  it("cancels on Escape without confirming", () => {
    const { onConfirm, onCancel } = renderPicker();
    act(() => {
      pickerRoot().dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      );
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("pre-fills the selection from an existing value (modify flow)", () => {
    const { onConfirm } = renderPicker({
      initial: { date: "2026-12-25", time: "09:30" },
    });
    // Committing without changing anything returns the pre-filled value.
    click('[data-action="confirm"]');
    expect(onConfirm.mock.calls[0][0]).toEqual({ date: "2026-12-25", time: "09:30" });
  });

  it("carries an edited clock time into the committed value", () => {
    const { onConfirm } = renderPicker();
    const input = container.querySelector('input[type="time"]') as HTMLInputElement;
    // Bypass React 19's controlled-input value tracker so onChange fires.
    const nativeSetter = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    act(() => {
      nativeSetter?.call(input, "14:00");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    click('[data-action="confirm"]');
    expect(onConfirm.mock.calls[0][0]).toEqual({ date: "2026-05-19", time: "14:00" });
  });
});
