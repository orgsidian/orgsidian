// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 3.6 (AC6): the non-modal scan-progress component renders correctly
 * across the idle → indexing → complete/cancelled states, driven by mocked
 * `index-progress` events, with a working Cancel button.
 */

type Payload = { current: number; total: number; errors: number };

// Hoisted shared state so the `vi.mock` factory can reference it.
const mocks = vi.hoisted(() => {
  const listeners: Array<(event: { payload: Payload }) => void> = [];
  return {
    listeners,
    cancelIndexScan: vi.fn(() => Promise.resolve()),
    listen: (cb: (event: { payload: Payload }) => void) => {
      listeners.push(cb);
      return Promise.resolve(() => {
        const idx = listeners.indexOf(cb);
        if (idx >= 0) listeners.splice(idx, 1);
      });
    },
  };
});

vi.mock("@/lib/tauri", () => ({
  events: { indexProgress: { listen: mocks.listen } },
  commands: { cancelIndexScan: mocks.cancelIndexScan },
}));

// Imported AFTER the mock is registered.
import { IndexScanProgress, type ScanPhase } from "./IndexScanProgress";

// React needs this flag to run effects under `act` in a non-DOM-test-runner.
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.listeners.length = 0;
  mocks.cancelIndexScan.mockClear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(phase: ScanPhase, onCancel: () => void = () => {}) {
  act(() => {
    root.render(<IndexScanProgress phase={phase} onCancelRequested={onCancel} />);
  });
}

function emit(payload: Payload) {
  act(() => {
    mocks.listeners.forEach((listener) => listener({ payload }));
  });
}

describe("IndexScanProgress", () => {
  it("renders nothing while idle", () => {
    render("idle");
    expect(container.textContent).toBe("");
  });

  it("shows live N-of-M counts and a Cancel button while indexing", () => {
    render("indexing");
    emit({ current: 3, total: 10, errors: 1 });
    expect(container.textContent).toContain("3 of 10 files indexed, 1 errors");
    // The LD-41 unparseable notice surfaces when errors > 0.
    expect(container.textContent).toContain("1 files unparseable");
    const cancel = container.querySelector("button");
    expect(cancel?.textContent).toBe("Cancel");
  });

  it("uses aria-live=polite on the counts region", () => {
    render("indexing");
    emit({ current: 1, total: 4, errors: 0 });
    const live = container.querySelector('[aria-live="polite"]');
    expect(live).not.toBeNull();
  });

  it("shows the final count on completion with no Cancel button", () => {
    render("complete");
    emit({ current: 10, total: 10, errors: 0 });
    expect(container.textContent).toContain("Indexed 10 of 10 files");
    expect(container.querySelector("button")).toBeNull();
  });

  it("shows the retained partial with a resume hint when cancelled", () => {
    render("cancelled");
    emit({ current: 3, total: 10, errors: 0 });
    expect(container.textContent).toContain("Cancelled — 3 of 10 indexed");
    expect(container.textContent).toContain("Resume any time");
  });

  it("invokes cancelIndexScan and notifies the parent on Cancel", () => {
    const onCancel = vi.fn();
    render("indexing", onCancel);
    emit({ current: 2, total: 10, errors: 0 });
    const cancel = container.querySelector("button") as HTMLButtonElement;
    act(() => {
      cancel.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(mocks.cancelIndexScan).toHaveBeenCalledTimes(1);
  });
});
