// @vitest-environment happy-dom
import { StrictMode, act, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 4.5 (FR-3): the Editor Mode switcher UI. Two layers of coverage:
 *
 *  1. The `ModeSwitcher` control in isolation (controlled component) — segmented
 *     control renders the active mode, click selects a mode, the
 *     `Cmd/Ctrl+Alt+M` chord cycles Raw → Pseudo-WYSIWYG → Split (platform-aware
 *     via `tauri-plugin-os`), a11y of the segmented control, and StrictMode
 *     listener idempotency.
 *  2. Integration with the real `Editor` host — clicking a segment switches the
 *     editor WITHOUT losing buffer state across every transition (incl. into and
 *     out of Split), the per-file preference persists (round-trips through the
 *     typed client), and a large-doc switch completes well under the 200ms
 *     budget.
 *
 * happy-dom (not jsdom) is required so CM6's `getComputedStyle` works in the
 * integration layer.
 */

const SOURCE = "* Heading alpha\nbody text beta\n";

// Hoisted so the `vi.mock` factories can reference them.
const mocks = vi.hoisted(() => ({
  openFile: vi.fn<(path: string) => Promise<string>>(),
  getEditorMode: vi.fn<(path: string) => Promise<string | null>>(),
  setEditorMode: vi.fn<(mode: string, path: string) => Promise<null>>(),
  // `tauri-plugin-os` platform() — synchronous; drives Cmd (macOS) vs Ctrl.
  platform: vi.fn<() => string>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    openFile: mocks.openFile,
    getEditorMode: mocks.getEditorMode,
    setEditorMode: mocks.setEditorMode,
  },
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

// Imported AFTER the mocks are registered.
import { ModeSwitcher, nextMode } from "./ModeSwitcher";
import { Editor, type EditorHandle } from "./Editor";
import { type EditorMode } from "@/lib/tauri";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.openFile.mockReset();
  mocks.getEditorMode.mockReset();
  mocks.setEditorMode.mockReset();
  mocks.platform.mockReset();
  mocks.getEditorMode.mockResolvedValue(null);
  mocks.setEditorMode.mockResolvedValue(null);
  mocks.openFile.mockResolvedValue(SOURCE);
  // Default to a non-mac platform (Ctrl chord) unless a test opts into macOS.
  mocks.platform.mockReturnValue("linux");
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

async function flush() {
  await act(async () => {
    for (let i = 0; i < 6; i += 1) {
      await Promise.resolve();
    }
  });
}

function segment(mode: EditorMode): HTMLButtonElement | null {
  return container.querySelector<HTMLButtonElement>(`[data-mode='${mode}']`);
}

function pressChord(init: KeyboardEventInit) {
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyM", ...init }));
  });
}

// -- Cycle contract ---------------------------------------------------------

describe("nextMode — cycle order (Story 4.5)", () => {
  it("cycles Raw → Pseudo-WYSIWYG → Split → Raw (wraps)", () => {
    expect(nextMode("raw")).toBe("pseudoWysiwyg");
    expect(nextMode("pseudoWysiwyg")).toBe("split");
    expect(nextMode("split")).toBe("raw");
  });
});

// -- Segmented control (controlled, in isolation) ---------------------------

describe("ModeSwitcher — segmented control", () => {
  function renderSwitcher(mode: EditorMode, onModeChange = vi.fn()) {
    act(() => {
      root.render(<ModeSwitcher mode={mode} onModeChange={onModeChange} />);
    });
    return onModeChange;
  }

  it("renders one segment per mode with the active one pressed", () => {
    renderSwitcher("pseudoWysiwyg");

    const group = container.querySelector("[role='group']");
    expect(group?.getAttribute("aria-label")).toBe("Editor mode");

    // All three modes are present.
    expect(segment("raw")).not.toBeNull();
    expect(segment("pseudoWysiwyg")).not.toBeNull();
    expect(segment("split")).not.toBeNull();

    // Active state is exposed via aria-pressed (not color alone).
    expect(segment("raw")?.getAttribute("aria-pressed")).toBe("false");
    expect(segment("pseudoWysiwyg")?.getAttribute("aria-pressed")).toBe("true");
    expect(segment("split")?.getAttribute("aria-pressed")).toBe("false");

    // Each segment carries an accessible name.
    expect(segment("pseudoWysiwyg")?.getAttribute("aria-label")).toBe(
      "Pseudo-WYSIWYG mode",
    );
    expect(segment("raw")?.getAttribute("aria-label")).toBe("Raw mode");
  });

  it("calls onModeChange with the clicked mode", () => {
    const onModeChange = renderSwitcher("raw");
    act(() => segment("split")?.click());
    expect(onModeChange).toHaveBeenCalledTimes(1);
    expect(onModeChange).toHaveBeenCalledWith("split");
  });

  it("reflects the mode prop as the active segment", () => {
    renderSwitcher("split");
    expect(segment("split")?.getAttribute("aria-pressed")).toBe("true");
    expect(segment("raw")?.getAttribute("aria-pressed")).toBe("false");
  });
});

// -- Keybinding: Cmd/Ctrl+Alt+M --------------------------------------------

describe("ModeSwitcher — Cmd/Ctrl+Alt+M chord", () => {
  it("cycles to the next mode on Ctrl+Alt+M (non-mac platform)", () => {
    mocks.platform.mockReturnValue("windows");
    const onModeChange = vi.fn();
    act(() => {
      root.render(<ModeSwitcher mode="pseudoWysiwyg" onModeChange={onModeChange} />);
    });

    pressChord({ ctrlKey: true, altKey: true });
    expect(onModeChange).toHaveBeenCalledTimes(1);
    expect(onModeChange).toHaveBeenCalledWith("split");
  });

  it("uses Cmd (meta) not Ctrl on macOS", () => {
    mocks.platform.mockReturnValue("macos");
    const onModeChange = vi.fn();
    act(() => {
      root.render(<ModeSwitcher mode="raw" onModeChange={onModeChange} />);
    });

    // Ctrl+Alt+M must NOT fire on macOS…
    pressChord({ ctrlKey: true, altKey: true });
    expect(onModeChange).not.toHaveBeenCalled();

    // …only Cmd+Alt+M does.
    pressChord({ metaKey: true, altKey: true });
    expect(onModeChange).toHaveBeenCalledWith("pseudoWysiwyg");
  });

  it("matches on event.code (macOS Option composes a glyph into key)", () => {
    mocks.platform.mockReturnValue("macos");
    const onModeChange = vi.fn();
    act(() => {
      root.render(<ModeSwitcher mode="raw" onModeChange={onModeChange} />);
    });

    // Option composes "µ" into `key`, but `code` stays "KeyM".
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          code: "KeyM",
          key: "µ",
          metaKey: true,
          altKey: true,
        }),
      );
    });
    expect(onModeChange).toHaveBeenCalledWith("pseudoWysiwyg");
  });

  it("ignores the chord without Alt, and ignores auto-repeat", () => {
    mocks.platform.mockReturnValue("windows");
    const onModeChange = vi.fn();
    act(() => {
      root.render(<ModeSwitcher mode="raw" onModeChange={onModeChange} />);
    });

    pressChord({ ctrlKey: true }); // no Alt
    pressChord({ altKey: true }); // no primary
    pressChord({ ctrlKey: true, altKey: true, repeat: true }); // auto-repeat
    expect(onModeChange).not.toHaveBeenCalled();
  });

  it("registers exactly one listener under StrictMode (fires once per chord)", () => {
    mocks.platform.mockReturnValue("windows");
    const onModeChange = vi.fn();
    act(() => {
      root.render(
        <StrictMode>
          <ModeSwitcher mode="raw" onModeChange={onModeChange} />
        </StrictMode>,
      );
    });

    pressChord({ ctrlKey: true, altKey: true });
    // A leaked duplicate listener from the double-mount would fire twice.
    expect(onModeChange).toHaveBeenCalledTimes(1);
  });

  it("removes the listener on unmount", () => {
    mocks.platform.mockReturnValue("windows");
    const onModeChange = vi.fn();
    act(() => {
      root.render(<ModeSwitcher mode="raw" onModeChange={onModeChange} />);
    });
    act(() => root.unmount());

    pressChord({ ctrlKey: true, altKey: true });
    expect(onModeChange).not.toHaveBeenCalled();
  });
});

// -- Integration with the Editor host --------------------------------------

/**
 * Minimal realistic wiring: the switcher is controlled by mode state that
 * mirrors the host (fed by `Editor.onModeChange`), and a segment selection
 * routes to `Editor.setMode` — exactly how a screen would wire the two. This
 * exercises the real host so buffer-preservation and persistence are proven
 * end-to-end, not stubbed.
 */
function Harness({ filePath }: { filePath: string }) {
  const editorRef = useRef<EditorHandle>(null);
  const [mode, setMode] = useState<EditorMode>("pseudoWysiwyg");
  return (
    <>
      <ModeSwitcher
        mode={mode}
        onModeChange={(next) => editorRef.current?.setMode(next)}
      />
      <Editor filePath={filePath} ref={editorRef} onModeChange={setMode} />
    </>
  );
}

describe("ModeSwitcher — integration with Editor host", () => {
  it("reflects the persisted mode the host loads on open", async () => {
    mocks.getEditorMode.mockResolvedValue("split");

    await act(async () => {
      root.render(<Harness filePath="/vault/notes.org" />);
    });
    await flush();

    // The switcher's active segment tracks the host's async-loaded mode.
    expect(segment("split")?.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector("[data-editor-mode='split']")).not.toBeNull();
  });

  it("switches through every mode via clicks WITHOUT losing buffer state", async () => {
    mocks.getEditorMode.mockResolvedValue(null); // opens Pseudo-WYSIWYG

    await act(async () => {
      root.render(<Harness filePath="/vault/notes.org" />);
    });
    await flush();

    // An unsaved edit typed into the live buffer. Read text from the Editor
    // host container (`data-editor-mode`), never the switcher's own segments.
    const editorRootText = () =>
      container.querySelector<HTMLElement>("[data-editor-mode]")?.textContent ?? "";
    expect(container.querySelector(".cm-content")).not.toBeNull();

    await act(async () => {
      // Dispatch through the primary view found in the DOM.
      const view = (
        await import("@codemirror/view")
      ).EditorView.findFromDOM(
        container.querySelector<HTMLElement>(".cm-editor")!,
      );
      view?.dispatch({ changes: { from: 0, insert: "EDIT " }, userEvent: "input.type" });
    });
    expect(editorRootText()).toContain("EDIT * Heading alpha");

    const openCalls = mocks.openFile.mock.calls.length;

    // Pseudo-WYSIWYG → Raw (in-place Compartment reconfigure).
    await act(async () => segment("raw")?.click());
    await flush();
    expect(container.querySelector("[data-editor-mode='raw']")).not.toBeNull();
    expect(segment("raw")?.getAttribute("aria-pressed")).toBe("true");
    expect(editorRootText()).toContain("EDIT * Heading alpha");

    // Raw → Split (rebuild carrying the live doc → two panes).
    await act(async () => segment("split")?.click());
    await flush();
    expect(container.querySelectorAll(".cm-editor")).toHaveLength(2);
    expect(container.textContent).toContain("EDIT * Heading alpha");

    // Split → Pseudo-WYSIWYG (rebuild back to a single view).
    await act(async () => segment("pseudoWysiwyg")?.click());
    await flush();
    expect(container.querySelectorAll(".cm-editor")).toHaveLength(1);
    expect(segment("pseudoWysiwyg")?.getAttribute("aria-pressed")).toBe("true");
    expect(editorRootText()).toContain("EDIT * Heading alpha");

    // The buffer was carried live across every switch — never reloaded.
    expect(mocks.openFile.mock.calls.length).toBe(openCalls);
  });

  it("persists each selection per-file through the typed client", async () => {
    mocks.getEditorMode.mockResolvedValue(null);

    await act(async () => {
      root.render(<Harness filePath="/vault/notes.org" />);
    });
    await flush();

    await act(async () => segment("split")?.click());
    await flush();

    expect(mocks.setEditorMode).toHaveBeenCalledWith("split", "/vault/notes.org");
  });

  it("round-trips a selection across an app restart (persist → reload)", async () => {
    const persisted = new Map<string, string>();
    mocks.setEditorMode.mockImplementation((mode: string, path: string) => {
      persisted.set(path, mode);
      return Promise.resolve(null);
    });
    mocks.getEditorMode.mockImplementation((path: string) =>
      Promise.resolve(persisted.get(path) ?? null),
    );

    // Session 1: select Raw via the switcher.
    await act(async () => {
      root.render(<Harness filePath="/vault/notes.org" />);
    });
    await flush();
    await act(async () => segment("raw")?.click());
    await flush();
    expect(persisted.get("/vault/notes.org")).toBe("raw");

    // Session 2 (remount): the switcher opens on the persisted Raw mode.
    await act(async () => root.unmount());
    container.remove();
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    await act(async () => {
      root.render(<Harness filePath="/vault/notes.org" />);
    });
    await flush();
    expect(segment("raw")?.getAttribute("aria-pressed")).toBe("true");
  });

  it("switches a 5000-line file well under the 200ms budget (FR-3 NFR)", async () => {
    const bigDoc = Array.from({ length: 5000 }, (_, i) => `* Heading ${i}`).join("\n");
    mocks.openFile.mockResolvedValue(bigDoc);
    mocks.getEditorMode.mockResolvedValue(null);

    await act(async () => {
      root.render(<Harness filePath="/vault/big.org" />);
    });
    await flush();

    const start = performance.now();
    await act(async () => segment("raw")?.click());
    const elapsed = performance.now() - start;

    expect(container.querySelector("[data-editor-mode='raw']")).not.toBeNull();
    // In-place Compartment reconfigure — no reload, no rebuild.
    expect(elapsed).toBeLessThan(200);
  });
});
