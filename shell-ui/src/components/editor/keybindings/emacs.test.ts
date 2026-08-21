import { describe, expect, it, vi, beforeEach } from "vitest";

// Mock tauri-plugin-os so platform resolution never needs a Tauri runtime
// (mirrors default.test.ts / ModeSwitcher.test.tsx).
const mocks = vi.hoisted(() => ({ platform: vi.fn(() => "linux") }));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: mocks.platform }));

import {
  DEFAULT_KEYMAP,
  buildDefaultKeymap,
  chordToCodeMirror,
  findAction,
  formatChord,
  type KeymapActionId,
} from "./default";
import { EMACS_KEYMAP } from "./emacs";

beforeEach(() => {
  mocks.platform.mockReset();
  mocks.platform.mockReturnValue("linux");
});

const findEmacs = (id: KeymapActionId) =>
  findAction(id, EMACS_KEYMAP);

// -- AC: the Emacs set covers the same action vocabulary as the native set ----

describe("EMACS_KEYMAP — coverage & shape (Story 4.7 FR-5 AC)", () => {
  it("covers every daily org-mode action named in the AC", () => {
    // save, agenda, capture, TODO cycle, schedule, deadline, clock in/out.
    const required: readonly KeymapActionId[] = [
      "save",
      "openAgenda",
      "capture",
      "cycleTodo",
      "setSchedule",
      "setDeadline",
      "clockIn",
      "clockOut",
    ];
    for (const id of required) {
      expect(findEmacs(id), `missing Emacs action: ${id}`).toBeDefined();
    }
  });

  it("has full id parity with the native DEFAULT_KEYMAP (clean swap)", () => {
    const nativeIds = new Set(DEFAULT_KEYMAP.map((a) => a.id));
    const emacsIds = new Set(EMACS_KEYMAP.map((a) => a.id));
    expect(emacsIds).toEqual(nativeIds);
  });

  it("assigns every action a unique CM6 chord string (no conflicts)", () => {
    const keys = EMACS_KEYMAP.map((a) => chordToCodeMirror(a.chord));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("gives every live editor action a run handler and reserved ones none", () => {
    for (const action of EMACS_KEYMAP) {
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
    for (const action of EMACS_KEYMAP) {
      expect(action.category).toBeTruthy();
      expect(action.description).toBeTruthy();
    }
  });
});

// -- AC: Emacs-style chords, including real multi-stroke sequences ------------

describe("EMACS_KEYMAP — Emacs chord idiom (multi-stroke)", () => {
  it("uses C-x C-s for save (verbatim from the AC)", () => {
    const save = findEmacs("save");
    expect(chordToCodeMirror(save!.chord)).toBe("Ctrl-x Ctrl-s");
    expect(formatChord(save!.chord, false)).toBe("C-x C-s");
    // Platform-independent: literal Ctrl, so macOS renders it identically.
    expect(formatChord(save!.chord, true)).toBe("C-x C-s");
  });

  it("cycles TODO with the faithful C-c C-t (org-todo), not C-c C-c", () => {
    const cycle = findEmacs("cycleTodo");
    expect(chordToCodeMirror(cycle!.chord)).toBe("Ctrl-c Ctrl-t");
    expect(formatChord(cycle!.chord, false)).toBe("C-c C-t");
  });

  it("toggles the checkbox with C-c C-c (org-ctrl-c-ctrl-c)", () => {
    const toggle = findEmacs("toggleCheckbox");
    expect(chordToCodeMirror(toggle!.chord)).toBe("Ctrl-c Ctrl-c");
    expect(formatChord(toggle!.chord, false)).toBe("C-c C-c");
  });

  it("schedules/deadlines with C-c C-s / C-c C-d", () => {
    expect(formatChord(findEmacs("setSchedule")!.chord, false)).toBe("C-c C-s");
    expect(formatChord(findEmacs("setDeadline")!.chord, false)).toBe("C-c C-d");
  });

  it("uses single-prefix bare-key chords for capture (C-c c) and agenda (C-c a)", () => {
    expect(chordToCodeMirror(findEmacs("capture")!.chord)).toBe("Ctrl-c c");
    expect(formatChord(findEmacs("capture")!.chord, false)).toBe("C-c c");
    expect(chordToCodeMirror(findEmacs("openAgenda")!.chord)).toBe("Ctrl-c a");
    expect(formatChord(findEmacs("openAgenda")!.chord, false)).toBe("C-c a");
  });

  it("renders triple-stroke clock chords (C-c C-x C-i / C-o)", () => {
    expect(chordToCodeMirror(findEmacs("clockIn")!.chord)).toBe(
      "Ctrl-c Ctrl-x Ctrl-i",
    );
    expect(formatChord(findEmacs("clockIn")!.chord, false)).toBe("C-c C-x C-i");
    expect(chordToCodeMirror(findEmacs("clockOut")!.chord)).toBe(
      "Ctrl-c Ctrl-x Ctrl-o",
    );
    expect(formatChord(findEmacs("clockOut")!.chord, false)).toBe("C-c C-x C-o");
  });

  it("keeps the un-remapped owners (find/search, mode/global) on native chords", () => {
    // Emacs mode swaps only the editor-owned CM6 keymap; search + the global
    // mode switch keep their native chords so the panel tells the truth.
    const find = findEmacs("find");
    expect(find!.owner).toBe("search");
    expect(formatChord(find!.chord, false)).toBe("Ctrl+F");
    const mode = findEmacs("switchMode");
    expect(mode!.owner).toBe("global");
    expect(formatChord(mode!.chord, false)).toBe("Ctrl+Alt+M");
  });
});

// -- buildDefaultKeymap consumes the Emacs set uniformly (the seam) -----------

describe("buildDefaultKeymap with EMACS_KEYMAP", () => {
  it("emits only editor-owned actions (search + global skipped)", () => {
    const keys = buildDefaultKeymap({ actions: EMACS_KEYMAP }).map((b) => b.key);
    // find (search) and switchMode (global) are NOT emitted here.
    expect(keys).not.toContain("Ctrl+F");
    expect(keys).not.toContain(chordToCodeMirror(findEmacs("switchMode")!.chord));
    // Editor-owned live + reserved multi-stroke chords are present.
    expect(keys).toContain("Ctrl-c Ctrl-t"); // cycle TODO (live)
    expect(keys).toContain("Ctrl-c Ctrl-s"); // schedule (live)
    expect(keys).toContain("Ctrl-x Ctrl-s"); // save (reserved)
    expect(keys).toContain("Ctrl-c Ctrl-x Ctrl-i"); // clock in (reserved, triple)
  });

  it("binds the reserved multi-stroke save to a no-op that calls onReserved", () => {
    const onReserved = vi.fn();
    const bindings = buildDefaultKeymap({ actions: EMACS_KEYMAP, onReserved });
    const save = bindings.find((b) => b.key === "Ctrl-x Ctrl-s");
    const view = {} as never;
    expect(save?.preventDefault).toBe(true);
    expect(save?.run?.(view)).toBe(true);
    expect(onReserved).toHaveBeenCalledTimes(1);
    expect(onReserved.mock.calls[0][0].id).toBe("save");
  });

  it("routes the live Schedule/Deadline chords through emitPlanningRequested", async () => {
    const schedule = findEmacs("setSchedule");
    const deadline = findEmacs("setDeadline");
    const seen: string[] = [];
    const view = {} as never;
    const { onPlanningRequested } = await import("../schedule");
    const off = onPlanningRequested((r) => seen.push(r.kind));
    schedule?.run?.(view);
    deadline?.run?.(view);
    off();
    expect(seen).toEqual(["scheduled", "deadline"]);
  });

  it("reuses the SAME live command functions as the native map (one surface)", () => {
    // cycleTodo/toggleCheckbox run identities match the native map — the Emacs
    // path is the same mutation surface, not a private re-implementation.
    expect(findEmacs("cycleTodo")!.run).toBe(findAction("cycleTodo")!.run);
    expect(findEmacs("toggleCheckbox")!.run).toBe(
      findAction("toggleCheckbox")!.run,
    );
  });
});
