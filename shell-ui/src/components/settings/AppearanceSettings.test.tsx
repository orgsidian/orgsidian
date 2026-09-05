// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { AppearanceSettings } from "./AppearanceSettings";
import {
  __resetThemeModeForTests,
  getThemePreference,
  setThemePreference,
} from "@/themes/themeMode";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  __resetThemeModeForTests();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  __resetThemeModeForTests();
});

function render() {
  act(() => {
    root.render(<AppearanceSettings />);
  });
}

function radio(value: "light" | "dark" | "system"): HTMLInputElement {
  const el = container.querySelector<HTMLInputElement>(
    `[data-testid='theme-option-${value}']`,
  );
  if (el === null) throw new Error(`missing radio for ${value}`);
  return el;
}

describe("AppearanceSettings (Story 6.7, FR-22)", () => {
  it("renders all three options with 'System default' reflecting the cold-start preference", () => {
    render();
    expect(radio("light")).not.toBeNull();
    expect(radio("dark")).not.toBeNull();
    expect(radio("system").checked).toBe(true);
    expect(radio("light").checked).toBe(false);
    expect(radio("dark").checked).toBe(false);
  });

  it("selecting Dark applies the theme instantly and checks the Dark radio", () => {
    render();
    act(() => {
      radio("dark").click();
    });
    expect(getThemePreference()).toBe("dark");
    expect(document.body.dataset.theme).toBe("dark");
    expect(radio("dark").checked).toBe(true);
    expect(radio("system").checked).toBe(false);
  });

  it("selecting Light applies the theme instantly and checks the Light radio", () => {
    render();
    act(() => {
      radio("light").click();
    });
    expect(getThemePreference()).toBe("light");
    expect(document.body.dataset.theme).toBe("light");
    expect(radio("light").checked).toBe(true);
  });

  it("reports the currently applied theme via an aria-live region", () => {
    render();
    act(() => {
      radio("dark").click();
    });
    const live = container.querySelector("[data-testid='resolved-theme']");
    expect(live?.getAttribute("aria-live")).toBe("polite");
    expect(live?.textContent).toContain("Dark");
    // An explicit choice drops the "(from system)" qualifier.
    expect(live?.textContent).not.toContain("from system");
  });

  it("qualifies the applied-theme line with '(from system)' when System default is active", () => {
    render();
    // Cold-start preference is "system"; with no matchMedia in jsdom/happy-dom
    // the resolved theme is Light, and the qualifier must be shown.
    const live = container.querySelector("[data-testid='resolved-theme']");
    expect(radio("system").checked).toBe(true);
    expect(live?.textContent).toContain("(from system)");
  });

  it("is accessible: a labelled section and a fieldset grouping the radios", () => {
    render();
    const section = container.querySelector("section");
    const headingId = section?.getAttribute("aria-labelledby");
    expect(headingId).not.toBeNull();
    // `getElementById`, not `querySelector('#' + id)`: `useId()` output is not
    // guaranteed to be a valid CSS identifier across React versions.
    expect(document.getElementById(headingId ?? "")?.textContent).toContain("Appearance");
    const fieldset = container.querySelector("fieldset");
    expect(fieldset).not.toBeNull();
    expect(fieldset?.querySelectorAll("input[type='radio']")).toHaveLength(3);
  });

  it("reflects a preference change made elsewhere in the app (shared store)", () => {
    render();
    // Not a click within this component -- exercises the same module-level
    // store a different Settings entry point (or a future keyboard shortcut)
    // could write to.
    act(() => {
      setThemePreference("dark");
    });
    expect(radio("dark").checked).toBe(true);
    expect(radio("system").checked).toBe(false);
  });
});
