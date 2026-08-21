import { beforeEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_KEYMAP } from "./default";
import { EMACS_KEYMAP } from "./emacs";
import {
  activeKeymap,
  getKeymapMode,
  isEmacsMode,
  onKeymapModeChange,
  setKeymapMode,
  __resetKeymapModeForTests,
} from "./keymapMode";

beforeEach(() => {
  __resetKeymapModeForTests(); // cold-start default + no subscribers
});

describe("keymapMode — session preference (Story 4.7, FR-5)", () => {
  it("defaults to native ('default') at cold start (UX Principle 3)", () => {
    expect(getKeymapMode()).toBe("default");
    expect(isEmacsMode()).toBe(false);
  });

  it("setKeymapMode switches the active mode for the session", () => {
    setKeymapMode("emacs");
    expect(getKeymapMode()).toBe("emacs");
    expect(isEmacsMode()).toBe(true);
  });

  it("resets to native on a fresh cold start (semantic state resets)", () => {
    setKeymapMode("emacs");
    __resetKeymapModeForTests(); // simulate a fresh app load
    expect(getKeymapMode()).toBe("default");
  });

  it("can be toggled back to native", () => {
    setKeymapMode("emacs");
    setKeymapMode("default");
    expect(getKeymapMode()).toBe("default");
  });
});

describe("keymapMode — change subscription", () => {
  it("notifies subscribers on change and stops after unsubscribe", () => {
    const seen: string[] = [];
    const off = onKeymapModeChange((m) => seen.push(m));
    setKeymapMode("emacs");
    setKeymapMode("default");
    off();
    setKeymapMode("emacs"); // no longer observed
    expect(seen).toEqual(["emacs", "default"]);
  });

  it("does not emit when the value is unchanged", () => {
    const listener = vi.fn();
    onKeymapModeChange(listener);
    setKeymapMode("default"); // already default
    expect(listener).not.toHaveBeenCalled();
    setKeymapMode("emacs");
    setKeymapMode("emacs"); // repeat — no second emit
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("isolates a throwing subscriber from the others", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    const good = vi.fn();
    onKeymapModeChange(() => {
      throw new Error("boom");
    });
    onKeymapModeChange(good);
    expect(() => setKeymapMode("emacs")).not.toThrow();
    expect(good).toHaveBeenCalledWith("emacs");
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});

describe("keymapMode — activeKeymap selector", () => {
  it("maps 'emacs' → EMACS_KEYMAP and 'default' → DEFAULT_KEYMAP", () => {
    expect(activeKeymap("emacs")).toBe(EMACS_KEYMAP);
    expect(activeKeymap("default")).toBe(DEFAULT_KEYMAP);
  });
});
