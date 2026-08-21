// Implements FR-5 — cross-platform default keybindings (Story 4.6).
//
// This module is the SINGLE SOURCE OF TRUTH for Orgsidian's native default
// chord set. Every daily org-mode action (save, open, find/replace, cycle TODO,
// toggle checkbox, schedule, deadline, switch editor mode, agenda, capture,
// clock in/out) is declared once here as a {@link KeymapAction}. Three consumers
// read this one table so they can never drift apart:
//
//   1. The CM6 editor keymap — {@link buildDefaultKeymap} turns the editor-bound
//      live actions into `KeyBinding[]` that the `Editor` host wires once.
//   2. The `ModeSwitcher` (Story 4.5) — reads the "switch editor mode" chord for
//      its global window listener + tooltip via {@link findAction} /
//      {@link formatChord} / {@link matchesChord}.
//   3. The Settings → Keybindings reference panel (`components/settings/
//      KeybindingsReference.tsx`) — lists ALL actions, live and reserved, with
//      their per-platform chord and status.
//
// Platform detection (LD-5 stack): `tauri-plugin-os` selects Cmd (macOS) vs
// Ctrl (Linux/Windows). Chords are stored platform-agnostically as a
// {@link Chord} whose `mod` flag is Cmd-or-Ctrl; {@link chordToCodeMirror}
// renders it to CM6's `Mod-` form (CM6 itself maps `Mod` → the platform primary)
// and {@link formatChord} renders the human label for the detected platform.
//
// Emacs mode (Story 4.7): the Emacs chord set lives in `keybindings/emacs.ts`
// as another `readonly KeymapAction[]` over the SAME {@link KeymapActionId}
// union. Both {@link buildDefaultKeymap} and the reference panel accept an
// explicit action list, so the Emacs set drops in with no structural change.
// The active keymap is a clean SWAP behind a CM6 `Compartment` in the `Editor`
// host: when Emacs mode is on, only the Emacs `keymap.of(...)` is wired (the
// native custom set is not present at the same time), so the active keymap wins
// on every conflict (the "active keymap takes precedence" AC of 4.7). The Emacs
// set reuses the two additive {@link Chord} fields below — `ctrl` (literal `C-`)
// and `then` (multi-stroke, `C-x C-s`) — which the native set never sets.
//
// Reserved chords: actions whose feature ships in a later epic (save/open write
// path, agenda, capture, clock in/out) are declared with `status: "reserved"`
// and NO `run` — no fake implementation. Their chord is reserved and documented
// so the map is complete and stable; {@link buildDefaultKeymap} binds them to a
// no-op that surfaces a "coming soon" affordance via the host-supplied
// `onReserved` callback rather than silently doing nothing or, worse, inventing
// behavior.

import { type KeyBinding, type EditorView } from "@codemirror/view";
import { platform } from "@tauri-apps/plugin-os";

import { cycleTodoAtCursor } from "../decorations/todoBadges";
import { toggleCheckboxAtCursor } from "../decorations/checkboxes";
import { emitPlanningRequested } from "../schedule";

/** Every daily org-mode action that carries a default chord (FR-5). */
export type KeymapActionId =
  | "save"
  | "openFile"
  | "find"
  | "cycleTodo"
  | "toggleCheckbox"
  | "setSchedule"
  | "setDeadline"
  | "switchMode"
  | "openAgenda"
  | "capture"
  | "clockIn"
  | "clockOut";

/** Reference-panel grouping for the actions. */
export type KeymapCategory = "File" | "Editing" | "Org" | "View" | "Agenda & time";

/**
 * A platform-agnostic chord. `mod` is the platform primary modifier — Cmd on
 * macOS, Ctrl elsewhere — matching CM6's `Mod`. `key` is a single character
 * (letter/punctuation, lower-case for letters) or a CM6 key name (e.g. `Enter`).
 */
export interface Chord {
  /** Cmd on macOS / Ctrl on Linux+Windows (the platform-primary modifier). */
  mod?: boolean;
  /**
   * A LITERAL Ctrl modifier — the SAME physical key on every platform, never
   * remapped to Cmd. The native default set never uses this; it is the Emacs
   * set's `C-` prefix (Story 4.7), e.g. `C-x C-s`. Distinct from {@link
   * Chord.mod}, which is Cmd on macOS.
   */
  ctrl?: boolean;
  /** Alt/Option on the native set; the Emacs `M-` (Meta) prefix on the Emacs set. */
  alt?: boolean;
  shift?: boolean;
  /** Single char (lower-case letter or punctuation) or a CM6 key name. */
  key: string;
  /**
   * The next stroke of a MULTI-STROKE chord (Story 4.7 Emacs mode: `C-x C-s`).
   * A linked list of strokes; the native single-stroke set never sets this.
   * {@link chordToCodeMirror} renders the sequence space-separated (CM6's
   * prefix-key form) and {@link formatChord} renders it as `C-x C-s`.
   */
  then?: Chord;
}

/**
 * Where an action's live binding actually lives. `editor` bindings are emitted
 * by {@link buildDefaultKeymap} into the CM6 keymap; `search` bindings come from
 * `@codemirror/search`'s `searchKeymap` (already wired in `sourceFidelity`);
 * `global` bindings are handled by a window listener outside CM6 (the
 * `ModeSwitcher` mode chord). The map still declares the chord for every one so
 * the reference panel and the reservation stay complete — `buildDefaultKeymap`
 * simply skips the non-`editor` ones to avoid double-binding.
 */
export type BindingOwner = "editor" | "search" | "global";

/** One documented default action. */
export interface KeymapAction {
  id: KeymapActionId;
  /** Human action name for the reference panel (plain string; see i18n note). */
  label: string;
  /** One-line description of what the action does. */
  description: string;
  category: KeymapCategory;
  chord: Chord;
  /** `live` = wired today; `reserved` = chord documented, feature ships later. */
  status: "live" | "reserved";
  owner: BindingOwner;
  /** For reserved actions: the epic/story that lands the real behavior. */
  reservedNote?: string;
  /**
   * The CM6 command for a `live` + `editor` action. Absent for reserved actions
   * (no fake impl) and for `search` / `global` actions (bound by their owner).
   */
  run?: (view: EditorView) => boolean;
}

/**
 * The native default chord set. Ordering is the reference-panel display order
 * (grouped by category). This array is the single source of truth referenced by
 * every consumer above.
 *
 * Mode-switch chord reconciliation (Story 4.6): the epics AC specifies
 * `Cmd/Ctrl+Alt+M`; the UX spec references `Cmd/Ctrl+Shift+M`. We adopt
 * **`Cmd/Ctrl+Alt+M`** — it matches the authoritative epics AC AND the chord
 * already shipped in Story 4.5's `ModeSwitcher`, so no live behavior changes and
 * the whole `Cmd/Ctrl+Alt+…` family (mode/schedule/deadline/TODO/checkbox) stays
 * internally consistent. The UX-spec `Shift` variant is intentionally dropped.
 */
export const DEFAULT_KEYMAP: readonly KeymapAction[] = [
  // --- File -----------------------------------------------------------------
  {
    id: "save",
    label: "Save",
    description: "Write the current buffer to disk.",
    category: "File",
    chord: { mod: true, key: "s" },
    status: "reserved",
    owner: "editor",
    // No write-back command exists yet (`open_file` is read-only); the save
    // path lands with the file write-back story. The chord is reserved now.
    reservedNote: "Write-back command lands in a later story.",
  },
  {
    id: "openFile",
    label: "Open file",
    description: "Open another .org file (quick-open).",
    category: "File",
    chord: { mod: true, key: "o" },
    status: "reserved",
    owner: "editor",
    reservedNote: "Quick-open palette lands in Epic 8 (Search).",
  },
  // --- Editing --------------------------------------------------------------
  {
    id: "find",
    label: "Find / Replace",
    description: "Open the find-and-replace panel (operates on source text).",
    category: "Editing",
    chord: { mod: true, key: "f" },
    status: "live",
    // Bound by @codemirror/search's searchKeymap (wired in sourceFidelity), so
    // NOT re-emitted by buildDefaultKeymap; declared here for the reference
    // panel and to reserve the chord in the map.
    owner: "search",
  },
  // --- Org ------------------------------------------------------------------
  {
    id: "cycleTodo",
    label: "Cycle TODO state",
    description: "Advance the current headline's TODO keyword to the next state.",
    category: "Org",
    chord: { mod: true, alt: true, key: "t" },
    status: "live",
    owner: "editor",
    run: cycleTodoAtCursor,
  },
  {
    id: "toggleCheckbox",
    label: "Toggle checkbox",
    description: "Toggle the checkbox on the current list item.",
    category: "Org",
    chord: { mod: true, alt: true, key: "x" },
    status: "live",
    owner: "editor",
    run: toggleCheckboxAtCursor,
  },
  {
    id: "setSchedule",
    label: "Set Schedule",
    description: "Add or change the Scheduled timestamp on the current headline.",
    category: "Org",
    chord: { mod: true, alt: true, key: "s" },
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
    description: "Add or change the Deadline on the current headline.",
    category: "Org",
    chord: { mod: true, alt: true, key: "d" },
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
    description: "Capture a new note or task.",
    category: "Org",
    chord: { mod: true, shift: true, key: "k" },
    status: "reserved",
    owner: "editor",
    reservedNote: "Capture ships in Epic 8.",
  },
  // --- View -----------------------------------------------------------------
  {
    id: "switchMode",
    label: "Switch editor mode",
    description: "Cycle Raw → Pseudo-WYSIWYG → Split.",
    category: "View",
    chord: { mod: true, alt: true, key: "m" },
    status: "live",
    // Handled by ModeSwitcher's global window listener (fires even when the
    // editor is not focused), so NOT emitted by buildDefaultKeymap.
    owner: "global",
  },
  // --- Agenda & time --------------------------------------------------------
  {
    id: "openAgenda",
    label: "Open Agenda",
    description: "Open the agenda view.",
    category: "Agenda & time",
    chord: { mod: true, shift: true, key: "a" },
    status: "reserved",
    owner: "editor",
    reservedNote: "Agenda ships in Epic 7.",
  },
  {
    id: "clockIn",
    label: "Clock in",
    description: "Start the clock on the current headline.",
    category: "Agenda & time",
    chord: { mod: true, shift: true, key: "," },
    status: "reserved",
    owner: "editor",
    reservedNote: "Time tracking ships in Epic 7.",
  },
  {
    id: "clockOut",
    label: "Clock out",
    description: "Stop the running clock.",
    category: "Agenda & time",
    chord: { mod: true, shift: true, key: "." },
    status: "reserved",
    owner: "editor",
    reservedNote: "Time tracking ships in Epic 7.",
  },
];

/**
 * True on macOS (chord uses ⌘); every other platform uses Ctrl. Platform
 * detection via `tauri-plugin-os` (LD-5). Guarded so a non-Tauri context (plain
 * `vite dev`, Vitest) falls back to the non-mac form rather than throwing —
 * shared by `ModeSwitcher` so the whole app resolves the platform one way.
 */
export function resolveIsMac(): boolean {
  try {
    return platform() === "macos";
  } catch {
    return false;
  }
}

/** Look up a documented action by id (from {@link DEFAULT_KEYMAP} by default). */
export function findAction(
  id: KeymapActionId,
  actions: readonly KeymapAction[] = DEFAULT_KEYMAP,
): KeymapAction | undefined {
  return actions.find((action) => action.id === id);
}

/** Render `key` for display (upper-case single letters, names verbatim). */
function displayKey(key: string): string {
  return key.length === 1 ? key.toUpperCase() : key;
}

/**
 * The CM6 `KeyBinding.key` string for a chord, e.g. `Mod-Alt-t`. `Mod` is CM6's
 * platform-primary modifier (Cmd on macOS, Ctrl elsewhere), so one string works
 * on every platform — the reason CM6 bindings need no platform branch.
 */
export function chordToCodeMirror(chord: Chord): string {
  const parts: string[] = [];
  if (chord.mod) parts.push("Mod");
  if (chord.ctrl) parts.push("Ctrl");
  if (chord.alt) parts.push("Alt");
  if (chord.shift) parts.push("Shift");
  parts.push(chord.key);
  const stroke = parts.join("-");
  // Multi-stroke (Emacs `C-x C-s`): CM6 reads space-separated key names as a
  // prefix-key sequence, so recursively join each following stroke with a space.
  return chord.then ? `${stroke} ${chordToCodeMirror(chord.then)}` : stroke;
}

/**
 * An Emacs-style chord: it carries no platform-primary `mod`, and uses a literal
 * `C-` (ctrl) / `M-` (alt/Meta) prefix or is multi-stroke (`then`). Every native
 * default chord carries `mod`, so this cleanly partitions the two idioms — the
 * reference panel renders each in its own notation from one `formatChord`.
 */
function isEmacsChord(chord: Chord): boolean {
  return (
    !chord.mod &&
    (chord.ctrl === true || chord.alt === true || chord.then !== undefined)
  );
}

/** One Emacs stroke in `C-`/`M-`/`S-` notation (keys stay lower-case: `C-x`). */
function formatEmacsStroke(chord: Chord): string {
  let out = "";
  if (chord.ctrl) out += "C-";
  if (chord.alt) out += "M-";
  if (chord.shift) out += "S-";
  return out + chord.key;
}

/** A full Emacs chord, strokes space-joined (`C-x C-s`). Platform-independent. */
function formatEmacsChord(chord: Chord): string {
  const stroke = formatEmacsStroke(chord);
  return chord.then ? `${stroke} ${formatEmacsChord(chord.then)}` : stroke;
}

/**
 * Human-readable chord label for the detected platform. macOS uses the symbol
 * form `⌘⌥⇧K` (matching Story 4.5's existing `⌘⌥M`); other platforms use the
 * `Ctrl+Alt+Shift+K` form.
 */
export function formatChord(chord: Chord, isMac: boolean): string {
  // Emacs-style chords have no platform-primary `mod`; they use literal
  // `C-`/`M-`/`S-` prefixes and may be multi-stroke, so render them in Emacs
  // notation (`C-x C-s`), which is the same on every platform (`isMac` N/A).
  if (isEmacsChord(chord)) return formatEmacsChord(chord);
  const key = displayKey(chord.key);
  if (isMac) {
    let out = "";
    if (chord.mod) out += "⌘";
    if (chord.alt) out += "⌥";
    if (chord.shift) out += "⇧";
    return out + key;
  }
  const parts: string[] = [];
  if (chord.mod) parts.push("Ctrl");
  if (chord.alt) parts.push("Alt");
  if (chord.shift) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/**
 * Does a `keydown` event match `chord` on the detected platform? Used by the
 * `ModeSwitcher`'s global (non-CM6) listener so its chord stays driven by this
 * map rather than hard-coded. Letter keys are matched via `event.code`
 * (`KeyM`), because macOS Option composes a glyph into `event.key` — `code`
 * stays layout-stable. `mod` maps to `metaKey` on macOS and `ctrlKey`
 * elsewhere; the opposite primary modifier must be absent so Ctrl+Alt+M does
 * not also fire on macOS.
 */
export function matchesChord(
  event: Pick<
    KeyboardEvent,
    "code" | "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey"
  >,
  chord: Chord,
  isMac: boolean,
): boolean {
  const primary = isMac ? event.metaKey : event.ctrlKey;
  const otherPrimary = isMac ? event.ctrlKey : event.metaKey;
  if ((chord.mod ?? false) !== primary) return false;
  // A chord without `mod` must not fire when the primary modifier is held; a
  // chord with `mod` must not fire when the *other* primary is also held.
  if (chord.mod && otherPrimary) return false;
  if ((chord.alt ?? false) !== event.altKey) return false;
  if ((chord.shift ?? false) !== event.shiftKey) return false;
  if (chord.key.length === 1 && /[a-z]/.test(chord.key)) {
    return event.code === `Key${chord.key.toUpperCase()}`;
  }
  return event.key === chord.key;
}

/**
 * Build the CM6 `KeyBinding[]` for the editor-owned actions of a keymap. Live
 * `editor` actions bind to their `run`; reserved `editor` actions bind to a
 * no-op that calls `onReserved` (the host surfaces "coming soon") and consumes
 * the chord so it does not fall through to some unrelated default. `search` and
 * `global` actions are skipped — their bindings live with their owners
 * (`searchKeymap` / `ModeSwitcher`) and re-emitting them here would double-bind.
 *
 * Pass an explicit `actions` list to build an alternate keymap (Story 4.7's
 * Emacs set) with no structural change.
 */
export function buildDefaultKeymap(options?: {
  onReserved?: (action: KeymapAction) => void;
  actions?: readonly KeymapAction[];
}): KeyBinding[] {
  const actions = options?.actions ?? DEFAULT_KEYMAP;
  const bindings: KeyBinding[] = [];
  for (const action of actions) {
    if (action.owner !== "editor") continue;
    const key = chordToCodeMirror(action.chord);
    if (action.status === "live" && action.run) {
      const run = action.run;
      bindings.push({ key, preventDefault: true, run });
    } else if (action.status === "reserved") {
      bindings.push({
        key,
        preventDefault: true,
        run: () => {
          options?.onReserved?.(action);
          return true;
        },
      });
    }
  }
  return bindings;
}

// i18n note: `label`/`description` strings are plain (no Lingui `<Trans>`),
// matching the sibling editor/settings components (ModeSwitcher, VaultPicker).
// The Vitest transform (esbuild) does not run the Lingui SWC macro plugin, so
// wrapping these would break the component/unit tests; the repo defers
// UI-string extraction to a dedicated i18n pass.
