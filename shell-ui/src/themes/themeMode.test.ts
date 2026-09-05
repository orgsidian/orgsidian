// Implements FR-22 -- theme preference + instant switch tests (Story 6.7).

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  __resetThemeModeForTests,
  getResolvedTheme,
  getThemePreference,
  onThemeChange,
  setThemePreference,
} from "./themeMode";

beforeEach(() => {
  __resetThemeModeForTests(); // cold-start default + no subscribers
});

describe("themeMode -- preference + instant switch (Story 6.7, FR-22)", () => {
  it("defaults to 'system' at cold start (mirrors keymapMode.ts's session-only rule)", () => {
    expect(getThemePreference()).toBe("system");
  });

  it("setThemePreference('dark') writes document.body.dataset.theme = 'dark' instantly", () => {
    setThemePreference("dark");
    expect(document.body.dataset.theme).toBe("dark");
    expect(getResolvedTheme()).toBe("dark");
  });

  it("setThemePreference('light') writes document.body.dataset.theme = 'light' instantly", () => {
    setThemePreference("dark"); // start from the other theme
    setThemePreference("light");
    expect(document.body.dataset.theme).toBe("light");
    expect(getResolvedTheme()).toBe("light");
  });

  it("never writes the literal 'system' to the DOM -- always a resolved concrete value", () => {
    setThemePreference("system");
    expect(["dark", "light"]).toContain(document.body.dataset.theme);
  });

  it("resets to 'system' on a fresh cold start (semantic state resets, mirrors keymapMode.ts)", () => {
    setThemePreference("dark");
    __resetThemeModeForTests(); // simulate a fresh app load
    expect(getThemePreference()).toBe("system");
  });

  it("is a no-op (no emit, no DOM churn) when the preference is unchanged", () => {
    const listener = vi.fn();
    onThemeChange(listener);
    setThemePreference("dark");
    setThemePreference("dark"); // repeat -- no second emit
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("themeMode -- change subscription", () => {
  it("notifies subscribers with (resolved, preference) and stops after unsubscribe", () => {
    const seen: Array<[string, string]> = [];
    const off = onThemeChange((resolved, preference) => seen.push([resolved, preference]));
    setThemePreference("dark");
    setThemePreference("light");
    off();
    setThemePreference("dark"); // no longer observed
    expect(seen).toEqual([
      ["dark", "dark"],
      ["light", "light"],
    ]);
  });

  it("isolates a throwing subscriber from the others", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    const good = vi.fn();
    onThemeChange(() => {
      throw new Error("boom");
    });
    onThemeChange(good);
    expect(() => setThemePreference("dark")).not.toThrow();
    expect(good).toHaveBeenCalledWith("dark", "dark");
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});

describe("themeMode -- live system-preference tracking", () => {
  it("a 'system' preference follows a live OS theme change", async () => {
    const listeners = new Set<(e: { matches: boolean }) => void>();
    const fakeQuery = {
      matches: false,
      addEventListener: (_type: string, listener: (e: { matches: boolean }) => void) => {
        listeners.add(listener);
      },
      removeEventListener: () => {},
    };
    vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(fakeQuery as unknown as MediaQueryList));

    // themeMode.ts captures `matchMedia` once at module-import time, so a
    // fresh module instance is required to observe the stub.
    vi.resetModules();
    const fresh = await import("./themeMode");

    fresh.setThemePreference("system");
    expect(fresh.getResolvedTheme()).toBe("light"); // fakeQuery.matches === false

    fakeQuery.matches = true;
    for (const listener of listeners) listener({ matches: true });

    expect(fresh.getResolvedTheme()).toBe("dark");
    expect(document.body.dataset.theme).toBe("dark");

    vi.unstubAllGlobals();
  });

  it("an explicit choice does NOT track a live OS theme change (guard's false branch)", async () => {
    const listeners = new Set<(e: { matches: boolean }) => void>();
    const fakeQuery = {
      matches: false,
      addEventListener: (_type: string, listener: (e: { matches: boolean }) => void) => {
        listeners.add(listener);
      },
      removeEventListener: () => {},
    };
    vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(fakeQuery as unknown as MediaQueryList));

    vi.resetModules();
    const fresh = await import("./themeMode");

    // Explicit "light" overrides the system preference; a subsequent OS flip
    // to dark must be ignored (the whole point of an explicit choice).
    fresh.setThemePreference("light");
    expect(fresh.getResolvedTheme()).toBe("light");

    fakeQuery.matches = true;
    for (const listener of listeners) listener({ matches: true });

    expect(fresh.getResolvedTheme()).toBe("light");
    expect(document.body.dataset.theme).toBe("light");

    vi.unstubAllGlobals();
  });
});
