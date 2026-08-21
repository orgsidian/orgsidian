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
// Seam for Story 4.7 (Emacs mode): the panel accepts an explicit `actions` list
// and a `title`, so the Emacs chord set drops in as a second panel ("Emacs
// mode") over the same {@link KeymapAction} shape with no structural change.

import {
  DEFAULT_KEYMAP,
  formatChord,
  resolveIsMac,
  type KeymapAction,
  type KeymapCategory,
} from "@/components/editor/keybindings/default";

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
  isMac,
  className,
}: KeybindingsReferenceProps) {
  const mac = isMac ?? resolveIsMac();

  return (
    <section className={className} aria-labelledby="keybindings-heading">
      <h2 id="keybindings-heading" className="text-lg font-medium">
        {title}
      </h2>
      <p className="mt-1 text-sm text-muted-foreground">
        Default shortcuts for {mac ? "macOS" : "your platform"}. Actions marked
        “Coming soon” have a reserved chord; the feature ships in a later
        release.
      </p>

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

// i18n note: labels are plain strings, matching VaultPicker / ModeSwitcher and
// the keymap source of truth; UI-string extraction is a dedicated later pass.
