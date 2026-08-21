// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 5.5 (FR-16 / NFR-16): the dirty-buffer conflict banner. The suite
 * realizes the ACs and the block/discard flow, driven by a mocked
 * `conflict-detected` event and the typed `discardExternalChanges` /
 * `openInDefaultEditor` commands:
 *  1. renders nothing until a conflict lands for THIS file;
 *  2. on a `conflict-detected` event it surfaces the blocked-save copy + both
 *     actions, in an accessible `role="status"` / `aria-live="polite"` region;
 *  3. "Discard external changes" clears the block (so a subsequent save can
 *     overwrite) and dismisses the banner;
 *  4. "View file in default editor" opens the file;
 *  5. an event for a DIFFERENT file is ignored.
 */

type Payload = { path: string; state: unknown };

const mocks = vi.hoisted(() => {
  const listeners: Array<(event: { payload: Payload }) => void> = [];
  return {
    listeners,
    discardExternalChanges: vi.fn(() => Promise.resolve()),
    openInDefaultEditor: vi.fn(() => Promise.resolve()),
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
  commands: {
    discardExternalChanges: mocks.discardExternalChanges,
    openInDefaultEditor: mocks.openInDefaultEditor,
  },
  events: { conflictDetected: { listen: mocks.listen } },
}));

// Imported AFTER the mock is registered.
import { ConflictBanner } from "./ConflictBanner";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const FILE = "/vault/notes.org";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.listeners.length = 0;
  mocks.discardExternalChanges.mockClear();
  mocks.openInDefaultEditor.mockClear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(filePath: string = FILE) {
  act(() => {
    root.render(<ConflictBanner filePath={filePath} />);
  });
}

function emit(path: string) {
  act(() => {
    mocks.listeners.forEach((listener) =>
      listener({ payload: { path, state: {} } }),
    );
  });
}

function banner() {
  return container.querySelector('[role="status"]');
}

function buttonByText(text: string): HTMLButtonElement | undefined {
  return Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(text),
  ) as HTMLButtonElement | undefined;
}

describe("ConflictBanner", () => {
  it("renders nothing before a conflict is detected", () => {
    render();
    expect(container.textContent).toBe("");
    expect(banner()).toBeNull();
  });

  it("surfaces the blocked-save banner with both actions on a conflict for this file", () => {
    render();
    emit(FILE);

    const region = banner();
    expect(region).not.toBeNull();
    // AC copy: "{path} was changed externally — save blocked."
    expect(region?.textContent).toContain(FILE);
    expect(region?.textContent).toContain("was changed externally");
    expect(region?.textContent).toContain("save blocked");
    expect(buttonByText("Discard external changes")).toBeDefined();
    expect(buttonByText("View file in default editor")).toBeDefined();
  });

  it("uses an accessible, calm live region (role=status, aria-live=polite)", () => {
    render();
    emit(FILE);
    const region = banner();
    // Calm, never assertive (epic UX): status + polite, not alert/assertive.
    expect(region?.getAttribute("aria-live")).toBe("polite");
    expect(region?.getAttribute("role")).toBe("status");
    // Actions are real, keyboard-operable buttons.
    for (const label of ["Discard external changes", "View file in default editor"]) {
      const btn = buttonByText(label);
      expect(btn?.tagName).toBe("BUTTON");
      expect(btn?.getAttribute("type")).toBe("button");
    }
  });

  it("ignores a conflict event for a different file", () => {
    render(FILE);
    emit("/vault/other.org");
    expect(banner()).toBeNull();
  });

  it("clears the block and dismisses the banner on Discard external changes", async () => {
    render();
    emit(FILE);
    expect(banner()).not.toBeNull();

    const discard = buttonByText("Discard external changes")!;
    await act(async () => {
      discard.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    // Backend block cleared for exactly this path → next save can overwrite.
    expect(mocks.discardExternalChanges).toHaveBeenCalledTimes(1);
    expect(mocks.discardExternalChanges).toHaveBeenCalledWith(FILE);
    // Banner dismissed.
    expect(banner()).toBeNull();
  });

  it("opens the file in the default editor on View file", () => {
    render();
    emit(FILE);

    const view = buttonByText("View file in default editor")!;
    act(() => {
      view.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(mocks.openInDefaultEditor).toHaveBeenCalledTimes(1);
    expect(mocks.openInDefaultEditor).toHaveBeenCalledWith(FILE);
    // View does not resolve the conflict — the banner stays up.
    expect(banner()).not.toBeNull();
  });
});
