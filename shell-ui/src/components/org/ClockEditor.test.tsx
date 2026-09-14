// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 7.8 (FR-8): the ClockEditor. The suite realizes the ACs a props-driven
 * editor can own in isolation:
 *  1. opens pre-filled with the entry's start / end and the computed duration;
 *  2. editing start or end recomputes the duration;
 *  3. editing the duration recomputes the end (start fixed);
 *  4. an end before the start blocks Save (validation);
 *  5. Save calls `updateClockEntry(headlineId, entryIndex, newStart, newEnd)`
 *     with the ISO stamps, fires `onSaved`, and closes;
 *  6. a command failure surfaces the reason and leaves the dialog open.
 */

const mocks = vi.hoisted(() => ({
  updateClockEntry: vi.fn<
    (
      headlineId: number,
      entryIndex: number,
      newStart: string,
      newEnd: string,
    ) => Promise<null>
  >(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: { updateClockEntry: mocks.updateClockEntry },
}));

// Imported AFTER the mock is registered.
import { ClockEditor } from "./ClockEditor";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.updateClockEntry.mockReset().mockResolvedValue(null);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

interface RenderOptions {
  headlineId?: number;
  entryIndex?: number;
  initialStart?: string;
  initialEnd?: string;
}

async function render(options: RenderOptions = {}) {
  const onOpenChange = vi.fn<(open: boolean) => void>();
  const onSaved = vi.fn<() => void>();
  await act(async () => {
    root.render(
      <ClockEditor
        headlineId={options.headlineId ?? 42}
        entryIndex={options.entryIndex ?? 0}
        initialStart={options.initialStart ?? "2026-09-13T10:00:00"}
        initialEnd={options.initialEnd ?? "2026-09-13T11:00:00"}
        open
        onOpenChange={onOpenChange}
        onSaved={onSaved}
      />,
    );
    await Promise.resolve();
  });
  return { onOpenChange, onSaved };
}

// Radix Dialog renders into a portal on document.body.
function input(label: string): HTMLInputElement {
  const el = document.body.querySelector<HTMLInputElement>(
    `input[aria-label="${label}"]`,
  );
  if (!el) throw new Error(`no input labelled ${JSON.stringify(label)}`);
  return el;
}

function buttonByText(text: string): HTMLButtonElement {
  const button = Array.from(
    document.body.querySelectorAll<HTMLButtonElement>("button"),
  ).find((b) => b.textContent?.trim() === text);
  if (!button) throw new Error(`no button with text ${JSON.stringify(text)}`);
  return button;
}

// Set a controlled input's value the way React observes a real user edit: the
// native value setter + a bubbling `input` event (React's onChange backing).
async function setInput(el: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  await act(async () => {
    setter?.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    await Promise.resolve();
  });
}

async function click(button: HTMLButtonElement) {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
  });
}

describe("ClockEditor (Story 7.8, FR-8)", () => {
  it("pre-fills start, end, and the computed duration", async () => {
    await render();
    expect(input("Start date").value).toBe("2026-09-13");
    expect(input("Start time").value).toBe("10:00");
    expect(input("End date").value).toBe("2026-09-13");
    expect(input("End time").value).toBe("11:00");
    expect(input("Duration").value).toBe("1:00");
  });

  it("recomputes the duration when the end time is edited", async () => {
    await render();
    await setInput(input("End time"), "12:30");
    expect(input("Duration").value).toBe("2:30");
  });

  it("recomputes the duration when the start time is edited", async () => {
    await render();
    await setInput(input("Start time"), "09:15");
    expect(input("Duration").value).toBe("1:45");
  });

  it("recomputes the end time when the duration is edited (start fixed)", async () => {
    await render();
    await setInput(input("Duration"), "3:15");
    expect(input("Start time").value).toBe("10:00"); // unchanged
    expect(input("End time").value).toBe("13:15");
  });

  it("blocks Save when the end is before the start", async () => {
    await render();
    await setInput(input("End time"), "09:00"); // before 10:00 start
    expect(buttonByText("Save").disabled).toBe(true);
    const alert = document.body.querySelector('[id="clock-editor-invalid"]');
    expect(alert?.textContent).toContain("at or after the start");
    await click(buttonByText("Save"));
    expect(mocks.updateClockEntry).not.toHaveBeenCalled();
  });

  it("saves via updateClockEntry with the ISO stamps, then fires onSaved and closes", async () => {
    const { onOpenChange, onSaved } = await render({
      headlineId: 7,
      entryIndex: 2,
    });
    await setInput(input("Start time"), "09:30");
    await setInput(input("End time"), "12:15");
    await click(buttonByText("Save"));

    expect(mocks.updateClockEntry).toHaveBeenCalledTimes(1);
    expect(mocks.updateClockEntry).toHaveBeenCalledWith(
      7,
      2,
      "2026-09-13T09:30:00",
      "2026-09-13T12:15:00",
    );
    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("surfaces a command failure and keeps the dialog open", async () => {
    mocks.updateClockEntry.mockRejectedValueOnce({
      kind: "vault",
      reason: "headline 42 is not in the index",
    });
    const { onSaved } = await render();
    await click(buttonByText("Save"));

    const alert = document.body.querySelector('[id="clock-editor-error"]');
    expect(alert?.textContent).toContain("not in the index");
    expect(onSaved).not.toHaveBeenCalled();
  });
});
