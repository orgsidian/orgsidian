// Implements FR-5 — optional Emacs keybindings mode (Story 4.7).
//
// The opt-in Emacs chord set, built ON the Story 4.6 seam: `EMACS_KEYMAP` is a
// `readonly KeymapAction[]` over the SAME {@link KeymapActionId} union and the
// SAME {@link KeymapAction} shape as `keybindings/default.ts`'s DEFAULT_KEYMAP,
// so every consumer reads it uniformly and the active keymap is a clean swap:
//
//   1. `buildDefaultKeymap({ actions: EMACS_KEYMAP, onReserved })` turns the
//      editor-owned live actions into CM6 `KeyBinding[]` (and reserved ones into
//      documented "coming soon" no-ops) — the exact builder the native set uses.
//   2. `KeybindingsReference` renders it as the "Emacs mode" reference section
//      via the same per-category tables (Story 4.7 panel AC).
//   3. The `Editor` host swaps the active keymap behind a CM6 `Compartment`
//      (see `keymapMode.ts` + `Editor.tsx`): when Emacs mode is on, ONLY this
//      set is wired, so it takes precedence over the native set on conflicts.
//
// Chord idiom (the Fidelity Lighthouse — "how would org-mode do it?"): the
// chords are the REAL Emacs/org-mode bindings, so a migrating power user's
// muscle memory carries over. They use the two additive {@link Chord} fields
// the native set never touches — `ctrl` (a literal `C-`, the SAME key on every
// platform, NOT Cmd) and `then` (multi-stroke, `C-x C-s`). Emacs never remaps
// `C-` to Cmd, so these are platform-independent by construction.
//
// AC-example reconciliation: the epics AC illustrates the style with "`C-x C-s`
// save, `C-c C-c` cycle TODO". `C-x C-s` (save-buffer) is adopted verbatim.
// The "`C-c C-c` cycle TODO" example is reconciled to the faithful org-mode
// bindings: org-mode cycles TODO with `C-c C-t` (`org-todo`), while `C-c C-c`
// (`org-ctrl-c-ctrl-c`) is the context action that TOGGLES A CHECKBOX on a list
// item — so `C-c C-t` → cycle TODO and `C-c C-c` → toggle checkbox here, which
// is what an Emacs org user's fingers actually expect. The reconciliation and
// every gap are documented in `docs/user-guide/emacs-keybindings.md`.
//
// Live vs reserved mirrors the native set exactly (no fake implementations):
// TODO cycle, checkbox toggle, Schedule and Deadline are LIVE and reuse the
// SAME command functions as the native map (one mutation surface, LD-26); save,
// open, capture, agenda and clock in/out are RESERVED (documented chord +
// `reservedNote`, no `run`) because their features ship in later epics.
//
// Owner boundary (same as native): `find` is owned by `@codemirror/search`'s
// searchKeymap and the editor-mode switch by `ModeSwitcher`'s global listener.
// Emacs mode swaps only the editor-owned CM6 keymap, so it does NOT remap those
// two owners; both are listed here with their UNCHANGED native chords so the
// reference panel tells the truth, and the idiomatic-Emacs gap (`C-s` isearch;
// no org analog for a view-mode switch) is documented in the user guide.

import { cycleTodoAtCursor } from "../decorations/todoBadges";
import { toggleCheckboxAtCursor } from "../decorations/checkboxes";
import { emitPlanningRequested } from "../schedule";
import { findAction, type KeymapAction } from "./default";

// The two owner-bound native actions Emacs mode does not remap (search + the
// global mode switch). Reused verbatim from the native map so their chord in the
// Emacs reference section is exactly what stays active in Emacs mode.
const nativeFind = findAction("find");
const nativeSwitchMode = findAction("switchMode");

/**
 * The opt-in Emacs chord set (FR-5). Same {@link KeymapActionId} coverage and
 * declaration order as DEFAULT_KEYMAP, so the reference panel groups identically
 * and a completeness test can assert parity. Chords are the real Emacs/org-mode
 * bindings; see the module header for the live/reserved split and the
 * AC-example reconciliation.
 */
export const EMACS_KEYMAP: readonly KeymapAction[] = [
  // --- File -----------------------------------------------------------------
  {
    id: "save",
    label: "Save",
    description: "Write the current buffer to disk (Emacs save-buffer).",
    category: "File",
    // C-x C-s (multi-stroke) — the canonical Emacs save chord.
    chord: { ctrl: true, key: "x", then: { ctrl: true, key: "s" } },
    status: "reserved",
    owner: "editor",
    reservedNote: "Write-back command lands in a later story (same as native).",
  },
  {
    id: "openFile",
    label: "Open file",
    description: "Open another .org file (Emacs find-file).",
    category: "File",
    // C-x C-f — Emacs find-file.
    chord: { ctrl: true, key: "x", then: { ctrl: true, key: "f" } },
    status: "reserved",
    owner: "editor",
    reservedNote: "Quick-open palette lands in Epic 8 (Search).",
  },
  // --- Editing --------------------------------------------------------------
  {
    id: "find",
    label: "Find / Replace",
    description:
      "Open the find-and-replace panel. Emacs mode does not remap search; " +
      "it keeps the native chord (C-s isearch is not yet wired — see the guide).",
    category: "Editing",
    // Owned by @codemirror/search's searchKeymap, NOT re-emitted here; keeps its
    // native chord because Emacs mode swaps only the editor-owned keymap.
    chord: nativeFind?.chord ?? { mod: true, key: "f" },
    status: "live",
    owner: "search",
  },
  // --- Org ------------------------------------------------------------------
  {
    id: "cycleTodo",
    label: "Cycle TODO state",
    description: "Advance the current headline's TODO keyword (org-todo).",
    category: "Org",
    // C-c C-t — org-mode's org-todo (the faithful TODO cycle chord).
    chord: { ctrl: true, key: "c", then: { ctrl: true, key: "t" } },
    status: "live",
    owner: "editor",
    run: cycleTodoAtCursor,
  },
  {
    id: "toggleCheckbox",
    label: "Toggle checkbox",
    description:
      "Toggle the checkbox on the current list item (org-ctrl-c-ctrl-c).",
    category: "Org",
    // C-c C-c — org-ctrl-c-ctrl-c; on a checkbox item this toggles the box.
    chord: { ctrl: true, key: "c", then: { ctrl: true, key: "c" } },
    status: "live",
    owner: "editor",
    run: toggleCheckboxAtCursor,
  },
  {
    id: "setSchedule",
    label: "Set Schedule",
    description: "Add or change the Scheduled timestamp (org-schedule).",
    category: "Org",
    // C-c C-s — org-schedule.
    chord: { ctrl: true, key: "c", then: { ctrl: true, key: "s" } },
    status: "live",
    owner: "editor",
    run: (view) => {
      emitPlanningRequested({ kind: "scheduled", view });
      return true;
    },
  },
  {
    id: "setDeadline",
    label: "Set Deadline",
    description: "Add or change the Deadline (org-deadline).",
    category: "Org",
    // C-c C-d — org-deadline.
    chord: { ctrl: true, key: "c", then: { ctrl: true, key: "d" } },
    status: "live",
    owner: "editor",
    run: (view) => {
      emitPlanningRequested({ kind: "deadline", view });
      return true;
    },
  },
  {
    id: "capture",
    label: "Capture",
    description: "Capture a new note or task (org-capture).",
    category: "Org",
    // C-c c — org-capture (single C- prefix, bare continuation key).
    chord: { ctrl: true, key: "c", then: { key: "c" } },
    status: "reserved",
    owner: "editor",
    reservedNote: "Capture ships in Epic 8 (same as native).",
  },
  // --- View -----------------------------------------------------------------
  {
    id: "switchMode",
    label: "Switch editor mode",
    description:
      "Cycle Raw → Pseudo-WYSIWYG → Split. An Orgsidian affordance with no " +
      "org-mode analog, so it keeps its native chord in Emacs mode.",
    category: "View",
    // Owned by ModeSwitcher's global listener (not re-emitted here); reuses the
    // native chord because there is no Emacs equivalent for a view-mode switch.
    chord: nativeSwitchMode?.chord ?? { mod: true, alt: true, key: "m" },
    status: "live",
    owner: "global",
  },
  // --- Agenda & time --------------------------------------------------------
  {
    id: "openAgenda",
    label: "Open Agenda",
    description: "Open the agenda view (org-agenda).",
    category: "Agenda & time",
    // C-c a — org-agenda dispatcher.
    chord: { ctrl: true, key: "c", then: { key: "a" } },
    status: "reserved",
    owner: "editor",
    reservedNote: "Agenda ships in Epic 7 (same as native).",
  },
  {
    id: "clockIn",
    label: "Clock in",
    description: "Start the clock on the current headline (org-clock-in).",
    category: "Agenda & time",
    // C-c C-x C-i — org-clock-in (three strokes).
    chord: {
      ctrl: true,
      key: "c",
      then: { ctrl: true, key: "x", then: { ctrl: true, key: "i" } },
    },
    status: "reserved",
    owner: "editor",
    reservedNote: "Time tracking ships in Epic 7 (same as native).",
  },
  {
    id: "clockOut",
    label: "Clock out",
    description: "Stop the running clock (org-clock-out).",
    category: "Agenda & time",
    // C-c C-x C-o — org-clock-out (three strokes).
    chord: {
      ctrl: true,
      key: "c",
      then: { ctrl: true, key: "x", then: { ctrl: true, key: "o" } },
    },
    status: "reserved",
    owner: "editor",
    reservedNote: "Time tracking ships in Epic 7 (same as native).",
  },
];

// i18n note: labels/descriptions are plain strings, matching DEFAULT_KEYMAP and
// the sibling editor/settings components; UI-string extraction is a later pass.
