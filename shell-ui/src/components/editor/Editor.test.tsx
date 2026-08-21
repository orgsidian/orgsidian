// @vitest-environment happy-dom
import { StrictMode, act, createRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 4.1: the CodeMirror 6 host is StrictMode-safe. The suite realizes the
 * lifecycle ACs and the I/O matrix:
 *  1. mounting renders the loaded source in the CM6 view DOM;
 *  2. a StrictMode mount→cleanup→remount→unmount cycle leaks zero views —
 *     `destroy()` is called once per view created (tracked construction +
 *     destruction counters, plus no orphan `.cm-editor` after unmount);
 *  3. unmounting before `openFile` resolves creates no view and updates no
 *     state after unmount.
 *
 * happy-dom (not jsdom) is required so CM6's `getComputedStyle` calls work.
 */

// Distinctive multi-line fixture source the mocked `openFile` resolves.
const SOURCE = "* Heading alpha\nbody text beta\n";

// Hoisted so the `vi.mock` factories can reference them.
const mocks = vi.hoisted(() => ({
  openFile: vi.fn<(path: string) => Promise<string>>(),
  // Story 4.2: the typed editor-mode client. `getEditorMode` defaults to
  // "no persisted choice" (null) in beforeEach; `setEditorMode` resolves.
  getEditorMode: vi.fn<(path: string) => Promise<string | null>>(),
  setEditorMode: vi.fn<(mode: string, path: string) => Promise<null>>(),
}));

// Track real EditorView construction/destruction without spying on a
// constructor: a thin subclass over the ACTUAL CM6 view keeps real DOM
// rendering (needed to assert source text) while counting instances.
const cm = vi.hoisted(() => ({ created: 0, destroyed: 0 }));

vi.mock("@/lib/tauri", () => ({
  commands: {
    openFile: mocks.openFile,
    getEditorMode: mocks.getEditorMode,
    setEditorMode: mocks.setEditorMode,
  },
}));

vi.mock("@codemirror/view", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@codemirror/view")>();
  class TrackedEditorView extends actual.EditorView {
    constructor(config?: ConstructorParameters<typeof actual.EditorView>[0]) {
      super(config);
      cm.created += 1;
    }
    destroy() {
      cm.destroyed += 1;
      super.destroy();
    }
  }
  return { ...actual, EditorView: TrackedEditorView };
});

// Imported AFTER the mocks are registered.
import { Editor, type EditorHandle } from "./Editor";
import { ORG_TOKEN_CLASS } from "./orgLanguage";

// React needs this flag to run effects under `act` outside a DOM test runner.
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  cm.created = 0;
  cm.destroyed = 0;
  mocks.openFile.mockReset();
  mocks.getEditorMode.mockReset();
  mocks.setEditorMode.mockReset();
  // Default: no persisted mode; a persistence write succeeds.
  mocks.getEditorMode.mockResolvedValue(null);
  mocks.setEditorMode.mockResolvedValue(null);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

/**
 * Flush pending microtasks. Story 4.2's load chains two awaits
 * (`getEditorMode` → `openFile`) before creating the view, so drain several
 * microtask turns to settle the whole chain.
 */
async function flush() {
  await act(async () => {
    for (let i = 0; i < 6; i += 1) {
      await Promise.resolve();
    }
  });
}

describe("Editor (StrictMode-safe CM6 host)", () => {
  it("renders the loaded source text in the CM6 view", async () => {
    mocks.openFile.mockResolvedValue(SOURCE);

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" />);
    });
    await flush();

    expect(mocks.openFile).toHaveBeenCalledWith("/vault/notes.org");
    expect(container.querySelector(".cm-editor")).not.toBeNull();
    expect(container.textContent).toContain("Heading alpha");
    expect(container.textContent).toContain("body text beta");
  });

  it("leaks no view across a StrictMode mount/unmount cycle", async () => {
    mocks.openFile.mockResolvedValue(SOURCE);

    await act(async () => {
      root.render(
        <StrictMode>
          <Editor filePath="/vault/notes.org" />
        </StrictMode>,
      );
    });
    await flush();

    // StrictMode double-invokes the effect (mount→cleanup→remount); the
    // disposed guard makes exactly one view creation deterministic — the
    // discarded first mount's load resolves already-disposed and creates none.
    expect(cm.created).toBe(1);
    expect(container.querySelectorAll(".cm-editor")).toHaveLength(1);

    await act(async () => {
      root.unmount();
    });
    await flush();

    // Every created view was destroyed exactly once — no leak, no orphan node.
    expect(cm.destroyed).toBe(cm.created);
    expect(container.querySelector(".cm-editor")).toBeNull();
  });

  it("creates no view when unmounted before openFile resolves", async () => {
    let resolveOpen: ((source: string) => void) | undefined;
    mocks.openFile.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveOpen = resolve;
        }),
    );

    await act(async () => {
      root.render(<Editor filePath="/vault/pending.org" />);
    });

    // Unmount while the load is still pending.
    await act(async () => {
      root.unmount();
    });

    // Late resolution must be ignored by the disposed guard.
    await act(async () => {
      resolveOpen?.(SOURCE);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(cm.created).toBe(0);
    expect(container.querySelector(".cm-editor")).toBeNull();
  });

  it("surfaces an error state and creates no view when openFile rejects", async () => {
    mocks.openFile.mockRejectedValue(new Error("failed to read notes.org"));

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" />);
    });
    await flush();

    expect(cm.created).toBe(0);
    expect(container.querySelector(".cm-editor")).toBeNull();
    const surface = container.querySelector("div");
    expect(surface?.getAttribute("data-error")).toContain("failed to read notes.org");
  });

  it("ignores a rejection that resolves after unmount (catch disposed guard)", async () => {
    let rejectOpen: ((error: unknown) => void) | undefined;
    mocks.openFile.mockImplementation(
      () =>
        new Promise<string>((_, reject) => {
          rejectOpen = reject;
        }),
    );
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    await act(async () => {
      root.render(<Editor filePath="/vault/pending.org" />);
    });
    await act(async () => {
      root.unmount();
    });

    // Rejection arrives only after unmount — the disposed guard must swallow it
    // with no view creation and no post-unmount state update (React would log
    // an act/setState warning through console.error if the guard failed).
    await act(async () => {
      rejectOpen?.(new Error("failed to read pending.org"));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(cm.created).toBe(0);
    expect(container.querySelector(".cm-editor")).toBeNull();
    expect(consoleError).not.toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it("swaps the view when filePath changes, destroying the previous one", async () => {
    mocks.openFile.mockImplementation((path: string) =>
      Promise.resolve(path === "/b.org" ? "B source" : "A source"),
    );

    await act(async () => {
      root.render(<Editor filePath="/a.org" />);
    });
    await flush();
    expect(container.textContent).toContain("A source");
    const createdAfterFirst = cm.created;

    await act(async () => {
      root.render(<Editor filePath="/b.org" />);
    });
    await flush();

    // The old view was torn down and exactly one live view remains, showing B.
    expect(cm.destroyed).toBeGreaterThanOrEqual(1);
    expect(cm.created).toBeGreaterThan(createdAfterFirst);
    expect(container.querySelectorAll(".cm-editor")).toHaveLength(1);
    expect(container.textContent).toContain("B source");
    expect(container.textContent).not.toContain("A source");
  });

  it("exposes a working ref handle (view + focus) as a plain React 19 prop", async () => {
    mocks.openFile.mockResolvedValue(SOURCE);
    const ref = createRef<EditorHandle>();

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" ref={ref} />);
    });
    await flush();

    expect(ref.current?.view).toBeTruthy();
    expect(() => ref.current?.focus()).not.toThrow();
  });
});

/**
 * Story 4.2 (FR-3): Raw editor mode. An org fixture exercising the tokenized
 * constructs so we can assert syntax highlighting is live.
 */
const ORG_FIXTURE = "* TODO Buy milk :errand:\n<2026-05-19 Mon 14:00>\n";

describe("Editor — Raw editor mode (Story 4.2)", () => {
  it("loads and applies the persisted Raw mode on open", async () => {
    mocks.openFile.mockResolvedValue(ORG_FIXTURE);
    mocks.getEditorMode.mockResolvedValue("raw");
    const ref = createRef<EditorHandle>();

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" ref={ref} />);
    });
    await flush();

    // The persisted choice was read through the typed client for this file.
    expect(mocks.getEditorMode).toHaveBeenCalledWith("/vault/notes.org");
    expect(ref.current?.mode).toBe("raw");
    expect(container.querySelector("[data-editor-mode='raw']")).not.toBeNull();
  });

  it("renders org syntax-highlight tokens and no decoration widgets in Raw mode", async () => {
    mocks.openFile.mockResolvedValue(ORG_FIXTURE);
    mocks.getEditorMode.mockResolvedValue("raw");

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" />);
    });
    await flush();

    // Syntax-highlight tokens are present (org-mode-aware): headline stars,
    // TODO keyword, tag, active timestamp.
    expect(container.querySelector(`.${ORG_TOKEN_CLASS.headingStars}`)).not.toBeNull();
    expect(container.querySelector(`.${ORG_TOKEN_CLASS.todoKeyword}`)?.textContent).toBe(
      "TODO",
    );
    expect(container.querySelector(`.${ORG_TOKEN_CLASS.tag}`)?.textContent).toBe(":errand:");
    expect(
      container.querySelector(`.${ORG_TOKEN_CLASS.timestampActive}`)?.textContent,
    ).toBe("<2026-05-19 Mon 14:00>");

    // NO Pseudo-WYSIWYG decorations/widgets are rendered in Raw mode.
    expect(container.querySelectorAll(".cm-widgetBuffer")).toHaveLength(0);
    expect(container.querySelectorAll("[class*='org-decoration']")).toHaveLength(0);

    // Source stays byte-faithful (highlighting is presentational).
    const rendered = Array.from(container.querySelectorAll(".cm-line"))
      .map((line) => line.textContent ?? "")
      .join("\n");
    expect(rendered).toBe(ORG_FIXTURE);
  });

  it("persists a mode switch via the typed commands.setEditorMode client", async () => {
    mocks.openFile.mockResolvedValue(ORG_FIXTURE);
    // Default landing mode when nothing is persisted.
    mocks.getEditorMode.mockResolvedValue(null);
    const ref = createRef<EditorHandle>();

    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" ref={ref} />);
    });
    await flush();

    expect(ref.current?.mode).toBe("pseudoWysiwyg");

    await act(async () => {
      ref.current?.setMode("raw");
    });
    await flush();

    // Routed through the typed client (never a raw invoke) with (mode, filePath).
    expect(mocks.setEditorMode).toHaveBeenCalledWith("raw", "/vault/notes.org");
    expect(ref.current?.mode).toBe("raw");
    expect(container.querySelector("[data-editor-mode='raw']")).not.toBeNull();
    // The switch reconfigures in place — the same single view, no reload.
    expect(container.querySelectorAll(".cm-editor")).toHaveLength(1);
  });

  it("round-trips a mode choice: what setEditorMode persists, getEditorMode reloads", async () => {
    mocks.openFile.mockResolvedValue(ORG_FIXTURE);
    // A tiny in-memory store shared by the two mocked commands proves the
    // set → reload contract flows through the typed client.
    const persisted = new Map<string, string>();
    mocks.setEditorMode.mockImplementation((mode: string, path: string) => {
      persisted.set(path, mode);
      return Promise.resolve(null);
    });
    mocks.getEditorMode.mockImplementation((path: string) =>
      Promise.resolve(persisted.get(path) ?? null),
    );

    // First session: switch to Raw, which persists.
    const firstRef = createRef<EditorHandle>();
    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" ref={firstRef} />);
    });
    await flush();
    await act(async () => {
      firstRef.current?.setMode("raw");
    });
    await flush();
    expect(persisted.get("/vault/notes.org")).toBe("raw");

    // Second session (remount): the persisted Raw mode is reloaded.
    await act(async () => root.unmount());
    container.remove();
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    const secondRef = createRef<EditorHandle>();
    await act(async () => {
      root.render(<Editor filePath="/vault/notes.org" ref={secondRef} />);
    });
    await flush();

    expect(secondRef.current?.mode).toBe("raw");
    expect(container.querySelector("[data-editor-mode='raw']")).not.toBeNull();
  });
});
