// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { KeybindingsReference, KeybindingsSettings } from "./KeybindingsReference";
import { DEFAULT_KEYMAP } from "@/components/editor/keybindings/default";
import {
  getKeymapMode,
  setKeymapMode,
  __resetKeymapModeForTests,
} from "@/components/editor/keybindings/keymapMode";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  __resetKeymapModeForTests();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(isMac: boolean) {
  act(() => {
    root.render(<KeybindingsReference isMac={isMac} />);
  });
}

describe("KeybindingsReference (Story 4.6, FR-5)", () => {
  it("lists every documented action with its action row", () => {
    render(false);
    for (const action of DEFAULT_KEYMAP) {
      const row = container.querySelector(`[data-action='${action.id}']`);
      expect(row, `missing row for ${action.id}`).not.toBeNull();
      // The action label is rendered.
      expect(row?.textContent).toContain(action.label);
    }
  });

  it("shows each chord in the platform form (Ctrl off macOS)", () => {
    render(false);
    const save = container.querySelector("[data-action='save'] [data-chord]");
    expect(save?.textContent).toBe("Ctrl+S");
    const cycle = container.querySelector(
      "[data-action='cycleTodo'] [data-chord]",
    );
    expect(cycle?.textContent).toBe("Ctrl+Alt+T");
  });

  it("shows ⌘ symbols on macOS", () => {
    render(true);
    const save = container.querySelector("[data-action='save'] [data-chord]");
    expect(save?.textContent).toBe("⌘S");
    const mode = container.querySelector(
      "[data-action='switchMode'] [data-chord]",
    );
    expect(mode?.textContent).toBe("⌘⌥M");
  });

  it("marks reserved actions with a 'Coming soon' badge and live actions without", () => {
    render(false);
    const reserved = container.querySelector(
      "[data-action='save'] [data-badge='reserved']",
    );
    expect(reserved?.textContent).toContain("Coming soon");
    const live = container.querySelector(
      "[data-action='cycleTodo'] [data-badge='reserved']",
    );
    expect(live).toBeNull();
  });

  it("is accessible: labelled section + per-category table captions and scoped headers", () => {
    render(false);
    const section = container.querySelector("section");
    expect(section?.getAttribute("aria-labelledby")).toBe("keybindings-heading");
    expect(
      container.querySelector("#keybindings-heading")?.textContent,
    ).toContain("Keybindings");
    // Each category renders a table with an sr-only caption.
    const captions = container.querySelectorAll("table > caption");
    expect(captions.length).toBeGreaterThan(0);
    // Row headers use scope="row" so the action name is the row's header.
    const rowHeader = container.querySelector(
      "[data-action='save'] th[scope='row']",
    );
    expect(rowHeader).not.toBeNull();
  });

  it("groups actions under their category (data-category attr present)", () => {
    render(false);
    for (const category of ["File", "Editing", "Org", "View", "Agenda & time"]) {
      expect(
        container.querySelector(`table[data-category='${category}']`),
        `missing category ${category}`,
      ).not.toBeNull();
    }
  });

  it("renders an 'Active' badge only when the set is active", () => {
    act(() => {
      root.render(<KeybindingsReference isMac={false} active />);
    });
    expect(
      container.querySelector("[data-badge='active']")?.textContent,
    ).toContain("Active");
    act(() => {
      root.render(<KeybindingsReference isMac={false} active={false} />);
    });
    expect(container.querySelector("[data-badge='active']")).toBeNull();
  });
});

/** The section (native or Emacs) whose heading contains `title`. */
function panelByTitle(title: string): HTMLElement | undefined {
  return [...container.querySelectorAll<HTMLElement>("section")].find((s) =>
    s.querySelector("h2")?.textContent?.includes(title),
  );
}

describe("KeybindingsSettings — Emacs mode toggle + panels (Story 4.7, FR-5)", () => {
  function renderSettings() {
    act(() => {
      root.render(<KeybindingsSettings isMac={false} />);
    });
  }

  it("renders BOTH the native and the Emacs reference panels", () => {
    renderSettings();
    expect(panelByTitle("Native keybindings")).toBeTruthy();
    expect(panelByTitle("Emacs mode")).toBeTruthy();
  });

  it("documents the Emacs chords in Emacs (C-…) notation under the Emacs panel", () => {
    renderSettings();
    const emacs = panelByTitle("Emacs mode")!;
    expect(
      emacs.querySelector("[data-action='save'] [data-chord]")?.textContent,
    ).toBe("C-x C-s");
    expect(
      emacs.querySelector("[data-action='cycleTodo'] [data-chord]")?.textContent,
    ).toBe("C-c C-t");
    expect(
      emacs.querySelector("[data-action='clockIn'] [data-chord]")?.textContent,
    ).toBe("C-c C-x C-i");
  });

  it("defaults to native active (toggle off) and marks the native panel Active", () => {
    renderSettings();
    const toggle = container.querySelector<HTMLInputElement>(
      "[data-testid='emacs-mode-toggle']",
    );
    expect(toggle?.checked).toBe(false);
    expect(getKeymapMode()).toBe("default");
    expect(
      panelByTitle("Native keybindings")?.querySelector("[data-badge='active']"),
    ).not.toBeNull();
    expect(
      panelByTitle("Emacs mode")?.querySelector("[data-badge='active']"),
    ).toBeNull();
  });

  it("enabling the toggle persists Emacs mode and moves the Active badge", () => {
    renderSettings();
    const toggle = container.querySelector<HTMLInputElement>(
      "[data-testid='emacs-mode-toggle']",
    )!;
    // A click flips the checkbox and drives React's controlled onChange.
    act(() => {
      toggle.click();
    });
    // Set through the shared keymapMode store.
    expect(getKeymapMode()).toBe("emacs");
    // Active badge is now on the Emacs panel, not the native one.
    expect(
      panelByTitle("Emacs mode")?.querySelector("[data-badge='active']"),
    ).not.toBeNull();
    expect(
      panelByTitle("Native keybindings")?.querySelector("[data-badge='active']"),
    ).toBeNull();
  });

  it("reflects an external keymap-mode change (shared store subscription)", () => {
    renderSettings();
    act(() => {
      setKeymapMode("emacs");
    });
    const toggle = container.querySelector<HTMLInputElement>(
      "[data-testid='emacs-mode-toggle']",
    );
    expect(toggle?.checked).toBe(true);
  });

  it("exposes the toggle as an accessible switch with a programmatic label", () => {
    renderSettings();
    const toggle = container.querySelector<HTMLInputElement>(
      "[data-testid='emacs-mode-toggle']",
    );
    expect(toggle?.getAttribute("role")).toBe("switch");
    // The <label htmlFor> associates a visible name with the control.
    const label = container.querySelector(`label[for='${toggle?.id}']`);
    expect(label?.textContent).toContain("Emacs keybindings mode");
  });
});
