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

// A local re-declaration of the generated discriminated-union DTO (the test
// mocks `@/lib/tauri`): `summary` is the normal prompt, `desynced` the
// discard-only recovery state.
type StaleClockDto =
  | {
      state: "summary";
      headlineId: number;
      headline: string;
      startedAt: string;
      lastActiveAt: string;
      keepDuration: string;
      adjustDuration: string;
    }
  | { state: "desynced"; headlineId: number };

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

// Imported AFTER the mock is registered. The session once-guard is the REAL
// module (not mocked) — its reset hook is called in `beforeEach`.
import { StaleClockPrompt } from "./StaleClockPrompt";
import { __resetStaleClockSessionForTests } from "./staleClockSession";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const SUMMARY: StaleClockDto = {
  state: "summary",
  headlineId: 42,
  headline: "Write the report",
  startedAt: "2026-09-13T04:00:00",
  lastActiveAt: "2026-09-13T18:00:00",
  keepDuration: "30:00",
  adjustDuration: "14:00",
};

const DESYNCED: StaleClockDto = { state: "desynced", headlineId: 999999 };

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  // Reset the per-launch once-guard so each test starts from a fresh app-launch
  // slate (otherwise the first test to run claims the guard and every later
  // render short-circuits to "nothing to show").
  __resetStaleClockSessionForTests();
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

  it("renders the three actions in spec DOM order: Adjust, Keep, Discard", async () => {
    // The frozen spec's on-screen (desktop left→right) order is
    // [Adjust end time] [Keep tracking] [Discard this session]. `DialogFooter`
    // is `sm:flex-row`, so desktop reading order = DOM order — assert DOM order.
    mocks.getStaleClock.mockResolvedValue(SUMMARY);
    await render();

    const labels = Array.from(
      document.body.querySelectorAll<HTMLButtonElement>(
        '[data-testid="stale-clock-prompt"] button',
      ),
    ).map((b) => b.textContent?.trim());
    expect(labels).toEqual([
      "Adjust end time",
      "Keep tracking",
      "Discard this session",
    ]);
  });

  it("does NOT re-appear after a route unmount/remount within the same session", async () => {
    // The bug: `<StaleClockPrompt/>` lives in the `/today` route, which TanStack
    // Router unmounts/remounts on every navigation — the launch check re-ran on
    // each remount, popping the modal (with a destructive Discard) on a LIVE
    // clock. The once-per-launch guard must evaluate exactly once per session.
    mocks.getStaleClock.mockResolvedValue(SUMMARY);

    // First mount (launch): the modal opens and the check runs once.
    await render();
    expect(dialog()).not.toBeNull();
    expect(mocks.getStaleClock).toHaveBeenCalledTimes(1);

    // Navigate away: unmount the route.
    act(() => root.unmount());
    container.remove();
    expect(dialog()).toBeNull();

    // Navigate back: a fresh mount of the same component (same JS session).
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    await render();

    // The prompt must NOT re-appear and the backend check must NOT re-run.
    expect(dialog()).toBeNull();
    expect(mocks.getStaleClock).toHaveBeenCalledTimes(1);
  });

  it("shows a discard-only recovery when the pointer's headline is gone (desync)", async () => {
    // A `desynced` result (headline no longer in the index) must OPEN the modal
    // in a discard-only recovery state — not be swallowed like "nothing to
    // show" — so `clockDiscard` is reachable and the stuck pointer clearable.
    mocks.getStaleClock.mockResolvedValue(DESYNCED);
    await render();

    const el = dialog();
    expect(el).not.toBeNull();
    const text = el?.textContent ?? "";
    expect(text).toContain("no longer in your Vault");

    // Only the Discard action is offered (no Keep / Adjust) and it is the
    // default action.
    const labels = Array.from(
      document.body.querySelectorAll<HTMLButtonElement>(
        '[data-testid="stale-clock-prompt"] button',
      ),
    ).map((b) => b.textContent?.trim());
    expect(labels).toEqual(["Discard this session"]);

    const discardBtn = buttonByText("Discard this session");
    expect(discardBtn.hasAttribute("data-default-action")).toBe(true);
  });

  it("Discard from the desync recovery clears the pointer and closes the modal", async () => {
    // `clock_discard` clears the stuck pointer even in the headline-not-found
    // branch (it returns after clearing the dangling pointer), so the recovery
    // succeeds whether the call resolves or rejects — the modal closes on settle.
    mocks.getStaleClock.mockResolvedValue(DESYNCED);
    // Simulate the backend's headline-not-found branch: it clears the pointer
    // then reports the desync as an error.
    mocks.clockDiscard
      .mockReset()
      .mockRejectedValue({ reason: "headline 999999 is no longer in the index" });
    await render();

    await click(buttonByText("Discard this session"));

    expect(mocks.clockDiscard).toHaveBeenCalledTimes(1);
    // The modal closes even though the command rejected (the pointer is cleared).
    expect(dialog()).toBeNull();
  });
});
