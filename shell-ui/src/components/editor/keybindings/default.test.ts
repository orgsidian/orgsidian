import { describe, expect, it, vi, beforeEach } from "vitest";

// Mock tauri-plugin-os so platform resolution is controllable and so the module
// under test does not need a Tauri runtime (mirrors ModeSwitcher.test.tsx).
const mocks = vi.hoisted(() => ({ platform: vi.fn(() => "linux") }));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: mocks.platform }));

import {
  DEFAULT_KEYMAP,
  buildDefaultKeymap,
  chordToCodeMirror,
  findAction,
  formatChord,
  matchesChord,
  resolveIsMac,
  type KeymapAction,
  type KeymapActionId,
} from "./default";

beforeEach(() => {
  mocks.platform.mockReset();
  mocks.platform.mockReturnValue("linux");
});

// -- AC: every daily org action has a documented default chord --------------

describe("DEFAULT_KEYMAP — completeness (FR-5 AC)", () => {
  const REQUIRED: readonly KeymapActionId[] = [
    "save",
    "openAgenda",
    "capture",
    "cycleTodo",
    "setSchedule",
    "setDeadline",
    "clockIn",
    "clockOut",
  ];

  it("declares a chord for every daily org-mode action named in the AC", () => {
    for (const id of REQUIRED) {
      const action = findAction(id);
      expect(action, `missing action: ${id}`).toBeDefined();
      expect(action?.chord.key, `empty chord for ${id}`).toBeTruthy();
    }
  });

  it("also documents find/replace, toggle-checkbox, open, and switch-mode", () => {
    for (const id of [
      "find",
      "toggleCheckbox",
      "openFile",
      "switchMode",
    ] as const) {
      expect(findAction(id)).toBeDefined();
    }
  });

  it("assigns every action a unique CM6 chord string (no conflicts)", () => {
    const keys = DEFAULT_KEYMAP.map((a) => chordToCodeMirror(a.chord));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("gives every live editor action a run handler and reserved ones none", () => {
    for (const action of DEFAULT_KEYMAP) {
      if (action.status === "live" && action.owner === "editor") {
        expect(action.run, `${action.id} live-editor needs run`).toBeTypeOf(
          "function",
        );
      }
      if (action.status === "reserved") {
        expect(action.run, `${action.id} reserved must not run`).toBeUndefined();
        expect(action.reservedNote).toBeTruthy();
      }
    }
  });

  it("gives every action a category and a description", () => {
    for (const action of DEFAULT_KEYMAP) {
      expect(action.category).toBeTruthy();
      expect(action.description).toBeTruthy();
    }
  });
});

// -- AC: platform-detected Cmd vs Ctrl via tauri-plugin-os -------------------

describe("resolveIsMac — platform detection (LD-5)", () => {
  it("is true only on macOS", () => {
    mocks.platform.mockReturnValue("macos");
    expect(resolveIsMac()).toBe(true);
  });

  it("is false on windows / linux", () => {
    mocks.platform.mockReturnValue("windows");
    expect(resolveIsMac()).toBe(false);
    mocks.platform.mockReturnValue("linux");
    expect(resolveIsMac()).toBe(false);
  });

  it("falls back to non-mac when platform() throws (non-Tauri context)", () => {
    mocks.platform.mockImplementation(() => {
      throw new Error("not in tauri");
    });
    expect(resolveIsMac()).toBe(false);
  });
});

describe("formatChord — per-platform rendering", () => {
  it("uses ⌘ symbols on macOS", () => {
    expect(formatChord({ mod: true, alt: true, key: "m" }, true)).toBe("⌘⌥M");
    expect(formatChord({ mod: true, shift: true, key: "a" }, true)).toBe("⌘⇧A");
    expect(formatChord({ mod: true, key: "s" }, true)).toBe("⌘S");
  });

  it("uses Ctrl+…+ words off macOS", () => {
    expect(formatChord({ mod: true, alt: true, key: "m" }, false)).toBe(
      "Ctrl+Alt+M",
    );
    expect(formatChord({ mod: true, key: "s" }, false)).toBe("Ctrl+S");
  });

  it("renders punctuation keys verbatim", () => {
    expect(formatChord({ mod: true, shift: true, key: "," }, false)).toBe(
      "Ctrl+Shift+,",
    );
  });
});

describe("chordToCodeMirror", () => {
  it("emits Mod- (CM6 platform-primary) in modifier order", () => {
    expect(chordToCodeMirror({ mod: true, alt: true, key: "t" })).toBe(
      "Mod-Alt-t",
    );
    expect(chordToCodeMirror({ mod: true, shift: true, key: "a" })).toBe(
      "Mod-Shift-a",
    );
    expect(chordToCodeMirror({ mod: true, key: "s" })).toBe("Mod-s");
  });
});

// -- matchesChord — used by ModeSwitcher's global listener -------------------

function ev(init: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    code: "",
    key: "",
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...init,
  } as KeyboardEvent;
}

describe("matchesChord — platform-aware event matching", () => {
  const chord = { mod: true, alt: true, key: "m" };

  it("matches Ctrl+Alt+M off macOS", () => {
    expect(
      matchesChord(ev({ code: "KeyM", ctrlKey: true, altKey: true }), chord, false),
    ).toBe(true);
  });

  it("requires Cmd (meta) on macOS, not Ctrl", () => {
    expect(
      matchesChord(ev({ code: "KeyM", ctrlKey: true, altKey: true }), chord, true),
    ).toBe(false);
    expect(
      matchesChord(ev({ code: "KeyM", metaKey: true, altKey: true }), chord, true),
    ).toBe(true);
  });

  it("uses event.code so a composed Option glyph in key still matches", () => {
    expect(
      matchesChord(
        ev({ code: "KeyM", key: "µ", metaKey: true, altKey: true }),
        chord,
        true,
      ),
    ).toBe(true);
  });

  it("rejects when a modifier is missing or extra", () => {
    expect(
      matchesChord(ev({ code: "KeyM", ctrlKey: true }), chord, false),
    ).toBe(false); // no Alt
    expect(
      matchesChord(
        ev({ code: "KeyM", ctrlKey: true, altKey: true, shiftKey: true }),
        chord,
        false,
      ),
    ).toBe(false); // extra Shift
  });
});

// -- buildDefaultKeymap ------------------------------------------------------

describe("buildDefaultKeymap", () => {
  it("emits only editor-owned actions (search + global bound elsewhere)", () => {
    const keys = buildDefaultKeymap().map((b) => b.key);
    // find is owned by searchKeymap; switchMode by the ModeSwitcher listener.
    expect(keys).not.toContain("Mod-f");
    expect(keys).not.toContain("Mod-Alt-m");
    // editor-owned live + reserved are present.
    expect(keys).toContain("Mod-Alt-t"); // cycle TODO (live)
    expect(keys).toContain("Mod-Alt-s"); // schedule (live)
    expect(keys).toContain("Mod-s"); // save (reserved)
  });

  it("binds live editor actions to their run and reserved ones to a no-op that calls onReserved", () => {
    const onReserved = vi.fn();
    const bindings = buildDefaultKeymap({ onReserved });
    const save = bindings.find((b) => b.key === "Mod-s");
    const view = {} as never;
    expect(save?.preventDefault).toBe(true);
    expect(save?.run?.(view)).toBe(true);
    expect(onReserved).toHaveBeenCalledTimes(1);
    expect(onReserved.mock.calls[0][0].id).toBe("save");
  });

  it("routes schedule/deadline through their emit handlers", () => {
    const schedule = findAction("setSchedule");
    const deadline = findAction("setDeadline");
    const seen: string[] = [];
    const view = {} as never;
    // The run handlers call emitPlanningRequested; subscribe to observe.
    return import("../schedule").then(({ onPlanningRequested }) => {
      const off = onPlanningRequested((r) => seen.push(r.kind));
      schedule?.run?.(view);
      deadline?.run?.(view);
      off();
      expect(seen).toEqual(["scheduled", "deadline"]);
    });
  });

  it("accepts an explicit actions list (Story 4.7 Emacs seam)", () => {
    const emacsLike: KeymapAction[] = [
      {
        id: "save",
        label: "Save",
        description: "save",
        category: "File",
        chord: { mod: true, key: "x" },
        status: "live",
        owner: "editor",
        run: () => true,
      },
    ];
    const bindings = buildDefaultKeymap({ actions: emacsLike });
    expect(bindings).toHaveLength(1);
  });
});
