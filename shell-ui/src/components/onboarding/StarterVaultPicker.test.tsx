// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 6.2 (FR-18): the first-launch onboarding picker.
 *
 *  1. renders three primary cards (Personal GTD, Student, Freelancer);
 *  2. the Freelancer card is disabled — Story 6.1 deferred that generator, so
 *     it must never reach `commands.generateStarterVault`;
 *  3. picking Personal GTD/Student prompts for a folder, calls
 *     `commands.generateStarterVault`, and notifies `onVaultConfigured`;
 *  4. dismissing the folder dialog is a no-op;
 *  5. a failed generation surfaces an error and does NOT call
 *     `onVaultConfigured`;
 *  6. "Use my own folder" reveals the (mocked) `VaultPicker`, and ITS
 *     `onDesignated` is wired straight through to `onVaultConfigured`.
 */

type VaultPickerStubProps = { onDesignated?: () => void };

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  generateStarterVault: vi.fn((_kind: string, _path: string, _today: string) =>
    Promise.resolve(null),
  ),
  vaultPickerProps: [] as VaultPickerStubProps[],
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mocks.open,
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    generateStarterVault: mocks.generateStarterVault,
  },
}));

// Stub the embedded "Use my own folder" flow — its own behavior is covered by
// `VaultPicker.test.tsx`; here we only assert it is reached and wired.
vi.mock("@/components/settings/VaultPicker", () => ({
  errorMessage: (err: unknown) =>
    err && typeof err === "object" && "reason" in err
      ? String((err as { reason: unknown }).reason)
      : String(err),
  VaultPicker: (props: VaultPickerStubProps) => {
    mocks.vaultPickerProps.push(props);
    return <div data-testid="own-folder-picker" />;
  },
}));

// Imported AFTER the mocks are registered.
import { StarterVaultPicker } from "./StarterVaultPicker";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.open.mockReset();
  mocks.generateStarterVault.mockReset();
  mocks.generateStarterVault.mockResolvedValue(null);
  mocks.vaultPickerProps.length = 0;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(onVaultConfigured: () => void = () => {}) {
  act(() => {
    root.render(<StarterVaultPicker onVaultConfigured={onVaultConfigured} />);
  });
}

function cardButton(titlePrefix: string): HTMLButtonElement {
  return Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith(titlePrefix),
  ) as HTMLButtonElement;
}

function ownFolderLink(): HTMLButtonElement {
  return Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.includes("Use my own folder"),
  ) as HTMLButtonElement;
}

describe("StarterVaultPicker", () => {
  it("renders the three primary Starter Vault cards plus the own-folder link", () => {
    render();
    expect(cardButton("Personal GTD")).toBeDefined();
    expect(cardButton("Student")).toBeDefined();
    expect(cardButton("Freelancer")).toBeDefined();
    expect(ownFolderLink()).toBeDefined();
  });

  it("renders the Freelancer card disabled with a Coming soon affordance", () => {
    render();
    const freelancer = cardButton("Freelancer");
    // Not a native `disabled` attribute — that would drop the card from the
    // tab order and hide the "Coming soon" reason from keyboard/AT users.
    // `aria-disabled` + an explicit `aria-describedby` keep it focusable and
    // discoverable while still conveying (and enforcing, via no `onClick`)
    // that it's inert.
    expect(freelancer.disabled).toBe(false);
    expect(freelancer.getAttribute("aria-disabled")).toBe("true");
    const describedBy = freelancer.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();
    expect(container.querySelector(`#${describedBy}`)?.textContent).toContain("Coming soon");
    expect(freelancer.textContent).toContain("Coming soon");
  });

  it("does nothing when clicking a disabled Freelancer card", async () => {
    render();
    await act(async () => {
      cardButton("Freelancer").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.open).not.toHaveBeenCalled();
    expect(mocks.generateStarterVault).not.toHaveBeenCalled();
  });

  it("does nothing when the folder dialog is dismissed for Personal GTD", async () => {
    mocks.open.mockResolvedValue(null);
    render();
    await act(async () => {
      cardButton("Personal GTD").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.generateStarterVault).not.toHaveBeenCalled();
  });

  it("generates Personal GTD into the chosen folder and reports the Vault configured", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    const onVaultConfigured = vi.fn();
    render(onVaultConfigured);
    await act(async () => {
      cardButton("Personal GTD").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.generateStarterVault).toHaveBeenCalledTimes(1);
    const [kind, path, today] = mocks.generateStarterVault.mock.calls[0];
    expect(kind).toBe("personalGtd");
    expect(path).toBe("/vault/path");
    expect(today).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(onVaultConfigured).toHaveBeenCalledTimes(1);
  });

  it("generates Student into the chosen folder", async () => {
    mocks.open.mockResolvedValue("/other/path");
    const onVaultConfigured = vi.fn();
    render(onVaultConfigured);
    await act(async () => {
      cardButton("Student").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(mocks.generateStarterVault).toHaveBeenCalledWith(
      "student",
      "/other/path",
      expect.any(String),
    );
    expect(onVaultConfigured).toHaveBeenCalledTimes(1);
  });

  it("surfaces an error and does not report configured when generation fails", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    mocks.generateStarterVault.mockRejectedValue({ kind: "io", reason: "disk full" });
    const onVaultConfigured = vi.fn();
    render(onVaultConfigured);
    await act(async () => {
      cardButton("Personal GTD").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(container.textContent).toContain("disk full");
    expect(onVaultConfigured).not.toHaveBeenCalled();
  });

  it("surfaces the non-empty-folder refusal via the alert path without generating", async () => {
    // Backend safety guard (`ensure_target_has_no_org_files`): the command
    // rejects BEFORE writing anything when the chosen folder already holds a
    // top-level `.org` file. The picker must show this as a normal `role`
    // alert" error — never silently continue or prompt to overwrite.
    mocks.open.mockResolvedValue("/vault/path");
    mocks.generateStarterVault.mockRejectedValue({
      kind: "vault",
      reason:
        '/vault/path already contains .org files; pick an empty folder for a Starter Vault, or use "Use my own folder" to designate this folder as your existing Vault instead',
    });
    const onVaultConfigured = vi.fn();
    render(onVaultConfigured);
    await act(async () => {
      cardButton("Personal GTD").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    const alert = container.querySelector('[role="alert"]');
    expect(alert).not.toBeNull();
    expect(alert?.textContent).toContain("already contains .org files");
    expect(alert?.textContent).toContain("Use my own folder");
    expect(onVaultConfigured).not.toHaveBeenCalled();
  });

  it("marks the in-flight card's progress text aria-live and aria-busy", async () => {
    mocks.open.mockResolvedValue("/vault/path");
    let resolveGenerate: (() => void) | undefined;
    mocks.generateStarterVault.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveGenerate = () => resolve(null);
        }),
    );
    render();
    await act(async () => {
      cardButton("Personal GTD").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    // Query the exact live-region span, not an ancestor whose `textContent`
    // also happens to include "Setting up" via this child.
    const progress = container.querySelector('[aria-live="polite"]');
    expect(progress).not.toBeNull();
    expect(progress?.textContent).toContain("Setting up");
    expect(progress?.getAttribute("aria-busy")).toBe("true");

    // Let the pending promise resolve so the test doesn't leak into the next one.
    await act(async () => {
      resolveGenerate?.();
    });
  });

  it("reveals the own-folder VaultPicker and wires its onDesignated straight through", () => {
    const onVaultConfigured = vi.fn();
    render(onVaultConfigured);
    expect(container.querySelector('[data-testid="own-folder-picker"]')).toBeNull();

    act(() => {
      ownFolderLink().dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(container.querySelector('[data-testid="own-folder-picker"]')).not.toBeNull();
    expect(mocks.vaultPickerProps).toHaveLength(1);
    mocks.vaultPickerProps[0].onDesignated?.();
    expect(onVaultConfigured).toHaveBeenCalledTimes(1);
  });
});
