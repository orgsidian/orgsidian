// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 3.6 baseline (folder-choose → `designateVault` → scan-progress) plus
 * Story 6.2's `onDesignated` callback, added so `StarterVaultPicker`'s
 * "Use my own folder" flow (which embeds this component) can dismiss itself
 * once designation succeeds.
 */

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  designateVault: vi.fn(() => Promise.resolve(null)),
  cancelIndexScan: vi.fn(() => Promise.resolve()),
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mocks.open,
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    designateVault: mocks.designateVault,
    cancelIndexScan: mocks.cancelIndexScan,
  },
  events: { indexProgress: { listen: mocks.listen } },
}));

// Imported AFTER the mocks are registered.
import { errorMessage, VaultPicker } from "./VaultPicker";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.open.mockReset();
  mocks.designateVault.mockReset();
  mocks.designateVault.mockResolvedValue(null);
  mocks.cancelIndexScan.mockClear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(onDesignated?: () => void) {
  act(() => {
    root.render(<VaultPicker onDesignated={onDesignated} />);
  });
}

function chooseButton(): HTMLButtonElement {
  return Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.includes("Choose Vault folder"),
  ) as HTMLButtonElement;
}

describe("VaultPicker", () => {
  it("does nothing when the folder dialog is dismissed", async () => {
    mocks.open.mockResolvedValue(null);
    render();
    await act(async () => {
      chooseButton().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.designateVault).not.toHaveBeenCalled();
  });

  it("designates the chosen folder and calls onDesignated on success", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    const onDesignated = vi.fn();
    render(onDesignated);
    await act(async () => {
      chooseButton().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.designateVault).toHaveBeenCalledWith("/vault/path");
    expect(onDesignated).toHaveBeenCalledTimes(1);
  });

  it("surfaces the error and skips onDesignated when designation fails", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    mocks.designateVault.mockRejectedValue({ kind: "vault", reason: "boom" });
    const onDesignated = vi.fn();
    render(onDesignated);
    await act(async () => {
      chooseButton().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(container.textContent).toContain("boom");
    expect(onDesignated).not.toHaveBeenCalled();
  });

  it("works with no onDesignated prop supplied (standalone Settings usage)", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    render();
    await act(async () => {
      chooseButton().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.designateVault).toHaveBeenCalled();
  });
});

describe("errorMessage", () => {
  it("extracts `reason` from an OrgError-shaped object", () => {
    expect(errorMessage({ kind: "io", reason: "disk full" })).toBe("disk full");
  });

  it("falls back to String(err) for anything else", () => {
    expect(errorMessage("oops")).toBe("oops");
  });
});
