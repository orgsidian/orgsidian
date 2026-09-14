// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 7.7 (FR-8 / UJ-1): the prior-session running-clock launch prompt. The
 * suite realizes the ACs:
 *  1. renders no dialog while the `getStaleClock` check is in flight;
 *  2. renders no dialog when there is no stale clock (or the check fails);
 *  3. opens the modal with the tracked Headline, last-active time, and both
 *     durations, with "Adjust end time" marked the default action;
 *  4. each button invokes the right command (resume / discard / adjust-end);
 *  5. Esc invokes Adjust (reveals the pre-filled time picker), never a cancel.
 */

// A local re-declaration of the generated DTO (the test mocks `@/lib/tauri`).
interface StaleClockDto {
  headlineId: number;
  headline: string;
  startedAt: string;
  lastActiveAt: string;
  keepDuration: string;
  adjustDuration: string;
}

const mocks = vi.hoisted(() => ({
  getStaleClock: vi.fn<() => Promise<StaleClockDto | null>>(),
  clockResume: vi.fn<(id: number) => Promise<unknown>>(),
  clockDiscard: vi.fn<() => Promise<void>>(),
  clockAdjustEnd: vi.fn<(endAt: string) => Promise<void>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    getStaleClock: mocks.getStaleClock,
    clockResume: mocks.clockResume,
    clockDiscard: mocks.clockDiscard,
    clockAdjustEnd: mocks.clockAdjustEnd,
  },
}));

// Imported AFTER the mock is registered.
import { StaleClockPrompt } from "./StaleClockPrompt";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const SUMMARY: StaleClockDto = {
  headlineId: 42,
  headline: "Write the report",
  startedAt: "2026-09-13T04:00:00",
  lastActiveAt: "2026-09-13T18:00:00",
  keepDuration: "30:00",
  adjustDuration: "14:00",
};

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.getStaleClock.mockReset();
  mocks.clockResume.mockReset().mockResolvedValue(undefined);
  mocks.clockDiscard.mockReset().mockResolvedValue(undefined);
  mocks.clockAdjustEnd.mockReset().mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

async function render() {
  await act(async () => {
    root.render(<StaleClockPrompt />);
    await Promise.resolve();
    await Promise.resolve();
  });
}

// Radix Dialog renders its content into a portal on document.body.
function dialog() {
  return document.body.querySelector('[data-testid="stale-clock-prompt"]');
}

function buttonByText(text: string): HTMLButtonElement {
  const button = Array.from(
    document.body.querySelectorAll<HTMLButtonElement>("button"),
  ).find((b) => b.textContent?.trim() === text);
  if (!button) throw new Error(`no button with text ${JSON.stringify(text)}`);
  return button;
}

async function click(button: HTMLButtonElement) {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
  });
}

describe("StaleClockPrompt (Story 7.7, FR-8 / UJ-1)", () => {
  it("renders no dialog while the stale-clock check is in flight", async () => {
    mocks.getStaleClock.mockReturnValue(new Promise(() => {})); // never resolves
    await render();
    expect(dialog()).toBeNull();
  });

  it("renders no dialog when there is no stale clock", async () => {
    mocks.getStaleClock.mockResolvedValue(null);
    await render();
    expect(dialog()).toBeNull();
  });

  it("fails safe to no dialog when the check errors (e.g. no active Vault)", async () => {
    mocks.getStaleClock.mockRejectedValue({ reason: "no active vault" });
    await render();
    expect(dialog()).toBeNull();
  });

  it("opens the modal with the headline, last-active time, and both durations", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();

    const el = dialog();
    expect(el).not.toBeNull();
    const text = el?.textContent ?? "";
    expect(text).toContain("Write the report");
    expect(text).toContain("2026-09-13 at 18:00");
    expect(text).toContain("30:00");
    expect(text).toContain("14:00");
  });

  it("marks Adjust end time the default action and focuses it on open", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();

    const adjust = buttonByText("Adjust end time");
    expect(adjust.hasAttribute("data-default-action")).toBe(true);
    expect(document.activeElement).toBe(adjust);
  });

  it("Keep tracking resumes from the original clock", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();
    await click(buttonByText("Keep tracking"));

    expect(mocks.clockResume).toHaveBeenCalledTimes(1);
    expect(mocks.clockResume).toHaveBeenCalledWith(42);
  });

  it("Discard this session removes the open line", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();
    await click(buttonByText("Discard this session"));

    expect(mocks.clockDiscard).toHaveBeenCalledTimes(1);
  });

  it("surfaces a rejected command's reason and keeps the dialog open", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    mocks.clockDiscard.mockRejectedValue({ reason: "no matching open line" });
    await render();

    await click(buttonByText("Discard this session"));

    const alert = document.body.querySelector('[role="alert"]');
    expect(alert).not.toBeNull();
    expect(alert?.textContent).toContain("no matching open line");
    // The modal stays open so the user can choose another action.
    expect(dialog()).not.toBeNull();
  });

  it("Adjust reveals a picker pre-filled from lastActiveAt and confirms an ISO end", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();

    // No picker until Adjust is chosen.
    expect(
      document.body.querySelector('[data-testid="stale-clock-adjust"]'),
    ).toBeNull();

    await click(buttonByText("Adjust end time"));

    const dateInput = document.body.querySelector<HTMLInputElement>(
      'input[type="date"]',
    );
    const timeInput = document.body.querySelector<HTMLInputElement>(
      'input[type="time"]',
    );
    expect(dateInput?.value).toBe("2026-09-13");
    expect(timeInput?.value).toBe("18:00");

    await click(buttonByText("Save end time"));
    expect(mocks.clockAdjustEnd).toHaveBeenCalledTimes(1);
    expect(mocks.clockAdjustEnd).toHaveBeenCalledWith("2026-09-13T18:00:00");
  });

  it("Esc invokes Adjust (opens the picker) rather than cancelling", async () => {
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();

    // Dialog is open, no picker yet.
    expect(dialog()).not.toBeNull();
    expect(
      document.body.querySelector('[data-testid="stale-clock-adjust"]'),
    ).toBeNull();

    await act(async () => {
      document.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      );
      await Promise.resolve();
    });

    // The picker is revealed and the dialog is still open (not dismissed).
    expect(dialog()).not.toBeNull();
    expect(
      document.body.querySelector('[data-testid="stale-clock-adjust"]'),
    ).not.toBeNull();
  });
});
