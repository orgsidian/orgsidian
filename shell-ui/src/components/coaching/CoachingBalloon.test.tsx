// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 6.6 (FR-21 partial / FR-18 / UJ-4): the hardcoded UJ-4 coaching
 * balloon. The suite realizes the ACs:
 *  1. renders nothing while the dismissed-check is in flight;
 *  2. renders nothing once the backend reports this id already dismissed;
 *  3. renders the balloon copy in an accessible `role="status"` /
 *     `aria-live="polite"` region when not dismissed;
 *  4. renders nothing when the dismissed-check fails (fail-safe to hidden —
 *     e.g. no active Vault);
 *  5. clicking the X button persists the dismissal (`commands.dismissCoaching`)
 *     and hides the balloon immediately (optimistic — a failed persist must
 *     not re-show it or block the dismiss).
 */

const mocks = vi.hoisted(() => ({
  getDismissedCoaching: vi.fn<() => Promise<string[]>>(),
  dismissCoaching: vi.fn<(id: string) => Promise<void>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    getDismissedCoaching: mocks.getDismissedCoaching,
    dismissCoaching: mocks.dismissCoaching,
  },
}));

// Imported AFTER the mock is registered.
import { CoachingBalloon } from "./CoachingBalloon";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const ID = "UJ4_TODAY_INTRO";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.getDismissedCoaching.mockReset();
  mocks.dismissCoaching.mockReset().mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(id: string = ID) {
  act(() => {
    root.render(<CoachingBalloon id={id}>Test coaching copy.</CoachingBalloon>);
  });
}

function balloon(id: string = ID) {
  return container.querySelector(`[data-coaching-id="${id}"]`);
}

describe("CoachingBalloon (Story 6.6, FR-21 partial / UJ-4)", () => {
  it("renders nothing while the dismissed-check is in flight", () => {
    mocks.getDismissedCoaching.mockReturnValue(new Promise(() => {})); // never resolves
    render();
    expect(balloon()).toBeNull();
  });

  it("renders the balloon copy in an accessible, calm live region when not dismissed", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([]);
    await act(async () => {
      render();
      await Promise.resolve();
    });

    const region = balloon();
    expect(region).not.toBeNull();
    expect(region?.getAttribute("role")).toBe("status");
    expect(region?.getAttribute("aria-live")).toBe("polite");
    expect(region?.textContent).toContain("Test coaching copy.");
  });

  it("renders nothing when this id is already dismissed", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([ID]);
    await act(async () => {
      render();
      await Promise.resolve();
    });
    expect(balloon()).toBeNull();
  });

  it("ignores a dismissed id that is not this balloon's", async () => {
    mocks.getDismissedCoaching.mockResolvedValue(["SOME_OTHER_ID"]);
    await act(async () => {
      render();
      await Promise.resolve();
    });
    expect(balloon()).not.toBeNull();
  });

  it("fails safe to hidden when the dismissed-check errors (e.g. no active Vault)", async () => {
    mocks.getDismissedCoaching.mockRejectedValue({
      reason: "no active vault; designate a vault first",
    });
    await act(async () => {
      render();
      await Promise.resolve();
    });
    expect(balloon()).toBeNull();
  });

  it("has a keyboard-operable, individually focusable dismiss button", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([]);
    await act(async () => {
      render();
      await Promise.resolve();
    });

    const button = balloon()?.querySelector("button");
    expect(button?.tagName).toBe("BUTTON");
    expect(button?.getAttribute("type")).toBe("button");
  });

  it("persists the dismissal and hides the balloon on click", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([]);
    await act(async () => {
      render();
      await Promise.resolve();
    });
    expect(balloon()).not.toBeNull();

    const button = balloon()!.querySelector("button")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(mocks.dismissCoaching).toHaveBeenCalledTimes(1);
    expect(mocks.dismissCoaching).toHaveBeenCalledWith(ID);
    expect(balloon()).toBeNull();
  });

  it("hides the balloon on click even when the persist call rejects", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([]);
    mocks.dismissCoaching.mockRejectedValue(new Error("disk full"));
    await act(async () => {
      render();
      await Promise.resolve();
    });

    const button = balloon()!.querySelector("button")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    // Optimistic dismiss: a failed persist must not re-show the balloon.
    expect(balloon()).toBeNull();
  });
});
