// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { KeybindingsReference } from "./KeybindingsReference";
import { DEFAULT_KEYMAP } from "@/components/editor/keybindings/default";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
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
});
