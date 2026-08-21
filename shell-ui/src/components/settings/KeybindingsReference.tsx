// Implements FR-5 — in-app keybinding reference panel (Story 4.6).
//
// The Settings → Keybindings panel: a read-only reference that lists EVERY
// documented default chord with its action, grouped by category. It reads the
// single source of truth in `components/editor/keybindings/default.ts`, so the
// panel and the live keymap can never drift. Reserved actions (feature ships in
// a later epic) are listed with a "Coming soon" badge and their reserved note,
// so the chord map the user sees is complete and honest — no chord is hidden and
// none is presented as working when it is not.
//
// Platform-aware: chords render as `⌘⌥M` on macOS and `Ctrl+Alt+M` elsewhere,
// resolved once via `resolveIsMac` (tauri-plugin-os, LD-5).
//
// Emacs mode (Story 4.7): the presentational panel accepts an explicit `actions`
// list, a `title`, a `headingId` (so two panels have unique labelled sections)
// and an `active` flag, so the Emacs chord set renders as a second panel ("Emacs
// mode") over the same {@link KeymapAction} shape with no structural change. The
// `KeybindingsSettings` wrapper below composes the native + Emacs panels with the
// opt-in toggle.

import { useEffect, useId, useState } from "react";

import {
  DEFAULT_KEYMAP,
  formatChord,
  resolveIsMac,
  type KeymapAction,
  type KeymapCategory,
} from "@/components/editor/keybindings/default";
import { EMACS_KEYMAP } from "@/components/editor/keybindings/emacs";
import {
  getKeymapMode,
  onKeymapModeChange,
  setKeymapMode,
  type KeymapMode,
} from "@/components/editor/keybindings/keymapMode";

/** Category render order for the reference table. */
const CATEGORY_ORDER: readonly KeymapCategory[] = [
  "File",
  "Editing",
  "Org",
  "View",
  "Agenda & time",
];

interface KeybindingsReferenceProps {
  /**
   * The chord set to display. Defaults to the native {@link DEFAULT_KEYMAP};
   * Story 4.7 passes the Emacs set for its own "Emacs mode" panel.
   */
  actions?: readonly KeymapAction[];
  /** Section heading (defaults to "Keybindings"). */
  title?: string;
  /**
   * DOM id of the heading (defaults to "keybindings-heading"). Set an explicit
   * id when rendering more than one panel so each labelled section is unique.
   */
  headingId?: string;
  /** Intro paragraph under the heading; a sensible default is used when absent. */
  description?: string;
  /**
   * Whether this chord set is the ACTIVE one (Story 4.7). When true an "Active"
   * badge renders beside the heading; status is conveyed by text, not color.
   */
  active?: boolean;
  /**
   * Force the platform for chord rendering (tests). Defaults to the detected
   * platform via `resolveIsMac` (tauri-plugin-os).
   */
  isMac?: boolean;
  className?: string;
}

/** Actions of one category, in declaration order. */
function actionsInCategory(
  actions: readonly KeymapAction[],
  category: KeymapCategory,
): KeymapAction[] {
  return actions.filter((action) => action.category === category);
}

/**
 * FR-5 keybinding reference panel. A per-category table of action → chord, with
 * a "Coming soon" badge on reserved actions.
 *
 * A11y: the section is labelled by its heading; each category is a `<table>`
 * with a visually-hidden `<caption>` naming the group and `scope`-d column
 * headers, so a screen reader announces "Action / Shortcut" per group. Chords
 * render in `<kbd>` with an `aria-label` spelling out the modifiers is
 * unnecessary — the visible text is already the label. Reserved status is
 * conveyed by text ("Coming soon"), never by color alone.
 */
export function KeybindingsReference({
  actions = DEFAULT_KEYMAP,
  title = "Keybindings",
  headingId = "keybindings-heading",
  description,
  active,
  isMac,
  className,
}: KeybindingsReferenceProps) {
  const mac = isMac ?? resolveIsMac();
  const intro =
    description ??
    `Shortcuts for ${mac ? "macOS" : "your platform"}. Actions marked ` +
      "“Coming soon” have a reserved chord; the feature ships in a later release.";

  return (
    <section className={className} aria-labelledby={headingId}>
      <h2 id={headingId} className="text-lg font-medium">
        {title}
        {active === true && (
          <span
            data-badge="active"
            className="ml-2 rounded bg-[var(--org-bg-elevated)] px-1.5 py-0.5 text-xs font-normal text-[var(--org-fg-default)]"
          >
            Active
          </span>
        )}
      </h2>
      <p className="mt-1 text-sm text-muted-foreground">{intro}</p>

      <div className="mt-4 space-y-6">
        {CATEGORY_ORDER.map((category) => {
          const rows = actionsInCategory(actions, category);
          if (rows.length === 0) return null;
          return (
            <table
              key={category}
              className="w-full border-collapse text-sm"
              data-category={category}
            >
              <caption className="sr-only">{category} shortcuts</caption>
              <thead>
                <tr className="border-b border-[var(--org-border-default)] text-left text-xs text-muted-foreground uppercase">
                  <th scope="col" className="py-1 pr-4 font-medium">
                    {category}
                  </th>
                  <th scope="col" className="py-1 font-medium">
                    Shortcut
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows.map((action) => (
                  <tr
                    key={action.id}
                    data-action={action.id}
                    data-status={action.status}
                    className="border-b border-[var(--org-border-default)]/50 last:border-0 align-top"
                  >
                    <th scope="row" className="py-2 pr-4 font-normal">
                      <span className="font-medium text-[var(--org-fg-default)]">
                        {action.label}
                      </span>
                      {action.status === "reserved" && (
                        <span
                          data-badge="reserved"
                          className="ml-2 rounded bg-[var(--org-bg-elevated)] px-1.5 py-0.5 text-xs text-muted-foreground"
                        >
                          Coming soon
                        </span>
                      )}
                      <span className="block text-xs text-muted-foreground">
                        {action.description}
                        {action.status === "reserved" &&
                          action.reservedNote != null &&
                          ` (${action.reservedNote})`}
                      </span>
                    </th>
                    <td className="py-2">
                      <kbd
                        data-chord
                        className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-1.5 py-0.5 font-mono text-xs whitespace-nowrap"
                      >
                        {formatChord(action.chord, mac)}
                      </kbd>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          );
        })}
      </div>
    </section>
  );
}

interface KeybindingsSettingsProps {
  /** Force the platform for chord rendering (tests). */
  isMac?: boolean;
  className?: string;
}

/**
 * Settings → Keybindings (Story 4.7, FR-5): the opt-in Emacs-mode toggle plus
 * BOTH reference panels — the native default set and the Emacs set — so the
 * Emacs chords are documented and discoverable whether or not the mode is on
 * (the "documented in the reference panel under an Emacs mode section" AC).
 *
 * The toggle persists the GLOBAL preference through `keymapMode.ts`
 * ({@link setKeymapMode}); the `Editor` host subscribes to the same store and
 * reconfigures its live keybindings Compartment, so switching never reloads the
 * buffer. Local state mirrors the store (seeded from {@link getKeymapMode} and
 * kept in sync via {@link onKeymapModeChange}) so the toggle and the "Active"
 * badges reflect changes made anywhere in the app.
 *
 * A11y: the toggle is a real checkbox with `role="switch"`, a programmatic
 * label, and `aria-checked`; the active set is marked by text ("Active"), never
 * color alone. Each panel is its own labelled `<section>` with a unique heading
 * id so a screen reader can navigate between the two sets.
 */
export function KeybindingsSettings({
  isMac,
  className,
}: KeybindingsSettingsProps) {
  const [mode, setMode] = useState<KeymapMode>(() => getKeymapMode());
  // Reflect changes made elsewhere (or by the Editor) — one store, one truth.
  useEffect(() => onKeymapModeChange(setMode), []);
  const emacs = mode === "emacs";
  const toggleId = useId();

  return (
    <section className={className} aria-labelledby={`${toggleId}-heading`}>
      <h2 id={`${toggleId}-heading`} className="text-lg font-medium">
        Keybindings
      </h2>

      <div className="mt-2 flex items-start gap-3">
        <input
          type="checkbox"
          role="switch"
          id={toggleId}
          data-testid="emacs-mode-toggle"
          checked={emacs}
          aria-checked={emacs}
          onChange={(event) =>
            setKeymapMode(event.target.checked ? "emacs" : "default")
          }
          className="mt-0.5"
        />
        <label htmlFor={toggleId} className="text-sm">
          <span className="font-medium text-[var(--org-fg-default)]">
            Emacs keybindings mode
          </span>
          <span className="block text-xs text-muted-foreground">
            Opt in to Emacs / org-mode chords (e.g. <kbd>C-x C-s</kbd> save,{" "}
            <kbd>C-c C-t</kbd> cycle TODO). When on, the Emacs set takes
            precedence over the native defaults. Native is the default.
          </span>
        </label>
      </div>

      <div className="mt-6 space-y-8">
        <KeybindingsReference
          title="Native keybindings"
          headingId={`${toggleId}-native`}
          actions={DEFAULT_KEYMAP}
          active={!emacs}
          isMac={isMac}
        />
        <KeybindingsReference
          title="Emacs mode"
          headingId={`${toggleId}-emacs`}
          actions={EMACS_KEYMAP}
          active={emacs}
          description={
            "Emacs / org-mode chords. Multi-stroke chords like C-x C-s are typed " +
            "as a sequence. Enable the toggle above to make these active."
          }
          isMac={isMac}
        />
      </div>
    </section>
  );
}

// i18n note: labels are plain strings, matching VaultPicker / ModeSwitcher and
// the keymap source of truth; UI-string extraction is a dedicated later pass.
