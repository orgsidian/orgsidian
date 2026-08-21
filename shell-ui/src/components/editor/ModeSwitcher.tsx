import { useEffect, useRef } from "react";
import { platform } from "@tauri-apps/plugin-os";

import { type EditorMode } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/**
 * Editor Mode switcher UI (Story 4.5, FR-3).
 *
 * A segmented control that shows the active Editor Mode and switches it — plus
 * a default keybinding that cycles the three modes. The switcher is a *pure UI
 * surface*: it neither owns the buffer nor persists the choice. Both live in the
 * `Editor` host, whose `setMode` reconfigures the live view in place (raw ↔
 * pseudoWysiwyg) or rebuilds the surface carrying the live doc (into/out of
 * Split) and persists per-file via `commands.setEditorMode`. This component
 * only reflects {@link ModeSwitcherProps.mode} and calls
 * {@link ModeSwitcherProps.onModeChange} — so buffer state is preserved and the
 * choice persists exactly as the host already guarantees (Stories 4.2/4.4).
 *
 * State-ownership boundary (epic-4 context, LOCKED): CM6 owns the buffer; the
 * current Editor Mode is UI state. This component takes it as a controlled prop
 * so the single source of truth stays outside the switcher.
 */

/**
 * Display AND cycle order: Raw → Pseudo-WYSIWYG → Split (epics.md Story 4.5 AC
 * and the UX spec's "cycles Raw / Pseudo-WYSIWYG / Split"). Cycling wraps.
 */
const MODE_ORDER: readonly EditorMode[] = ["raw", "pseudoWysiwyg", "split"];

/** User-facing segment labels (see i18n note at the foot of the file). */
const MODE_LABELS: Record<EditorMode, string> = {
  raw: "Raw",
  pseudoWysiwyg: "Pseudo-WYSIWYG",
  split: "Split",
};

/**
 * The next mode in the cycle (wraps at the end). Exported so the cycle contract
 * is unit-testable without simulating a keyboard event. An unknown mode (should
 * never happen given the typed union) restarts the cycle at the first mode.
 */
export function nextMode(mode: EditorMode): EditorMode {
  const index = MODE_ORDER.indexOf(mode);
  return MODE_ORDER[(index + 1) % MODE_ORDER.length];
}

/**
 * True on macOS, where the mode-switch chord uses ⌘ (Cmd); every other platform
 * uses Ctrl. Platform detection is via `tauri-plugin-os` (LD-5 stack). Resolved
 * synchronously (plugin-os caches the value at startup); guarded so a non-Tauri
 * context (plain `vite dev`, tests) falls back to the non-mac chord rather than
 * throwing.
 */
function isMacPlatform(): boolean {
  try {
    return platform() === "macos";
  } catch {
    return false;
  }
}

/** Human-readable chord hint for the active platform, shown as a tooltip. */
function chordHint(isMac: boolean): string {
  // NOTE: epics.md Story 4.5 AC specifies `Cmd/Ctrl+Alt+M`; the UX spec
  // (ux-design-specification.md) references `Cmd/Ctrl+Shift+M` for the same
  // action. We follow the epics AC (Alt) — DISCREPANCY flagged for
  // reconciliation before the keybinding reference panel lands in Story 4.6.
  return isMac ? "⌘⌥M" : "Ctrl+Alt+M";
}

interface ModeSwitcherProps {
  /** The active Editor Mode (controlled — the source of truth lives upstream). */
  mode: EditorMode;
  /**
   * Invoked with the chosen mode on a segment click or a chord cycle. The
   * consumer routes this to `Editor.setMode`, which preserves buffer state and
   * persists per-file.
   */
  onModeChange: (mode: EditorMode) => void;
  /** Optional extra classes for the segmented-control container. */
  className?: string;
}

/**
 * Segmented control + `Cmd/Ctrl+Alt+M` cycle for the three Editor Modes.
 *
 * A11y: the container is a `role="group"` labelled "Editor mode"; each segment
 * is a toggle button whose `aria-pressed` announces the active mode, so screen
 * readers and keyboard users get the selection state without relying on color.
 * Focus is visible via `--org-border-focus`. Each segment is individually
 * tab-reachable and activates on Enter/Space (native `<button>` behavior).
 */
export function ModeSwitcher({ mode, onModeChange, className }: ModeSwitcherProps) {
  const isMac = isMacPlatform();

  // Keep the latest mode + callback in refs so the global key listener is
  // registered ONCE (not re-bound every render) yet always reads current
  // values. Updated in an effect (never during render) to stay concurrent-safe.
  const modeRef = useRef(mode);
  const onModeChangeRef = useRef(onModeChange);
  useEffect(() => {
    modeRef.current = mode;
    onModeChangeRef.current = onModeChange;
  });

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      // Ignore auto-repeat so holding the chord does not spin through modes.
      if (event.repeat) return;
      // Cmd+Alt+M on macOS, Ctrl+Alt+M elsewhere. `event.code` (not `event.key`)
      // because macOS Option composes a glyph ("µ") into `key`, but `code`
      // stays "KeyM".
      const primary = isMac ? event.metaKey : event.ctrlKey;
      if (!primary || !event.altKey || event.code !== "KeyM") return;
      event.preventDefault();
      onModeChangeRef.current(nextMode(modeRef.current));
    }
    window.addEventListener("keydown", onKeyDown);
    // Idempotent teardown: a StrictMode double-mount adds → removes → adds, so
    // exactly one listener survives and none leaks across remounts.
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isMac]);

  return (
    <div
      role="group"
      aria-label="Editor mode"
      title={`Switch editor mode (${chordHint(isMac)})`}
      // `data-active-mode` (not `data-editor-mode`, which the Editor host uses
      // on its own container) so the two never collide in a shared DOM.
      data-active-mode={mode}
      className={cn(
        "inline-flex items-center gap-0.5 rounded-md border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] p-0.5",
        className,
      )}
    >
      {MODE_ORDER.map((option) => {
        const active = option === mode;
        return (
          <button
            key={option}
            type="button"
            data-mode={option}
            data-active={active || undefined}
            aria-pressed={active}
            aria-label={`${MODE_LABELS[option]} mode`}
            onClick={() => onModeChange(option)}
            className={cn(
              "cursor-pointer rounded px-2.5 py-1 text-xs font-medium whitespace-nowrap transition-colors outline-none",
              "focus-visible:ring-2 focus-visible:ring-[var(--org-border-focus)]",
              active
                ? "bg-[var(--org-bg-elevated)] text-[var(--org-fg-default)] shadow-sm"
                : "text-[var(--org-fg-muted)] hover:text-[var(--org-fg-default)]",
            )}
          >
            {MODE_LABELS[option]}
          </button>
        );
      })}
    </div>
  );
}

// i18n note: segment labels are plain strings, matching the sibling editor/
// settings components (VaultPicker, IndexScanProgress). The Vitest transform
// (esbuild) does not run the Lingui SWC macro plugin, so wrapping these in
// `<Trans>` would break component tests; the repo defers UI-string extraction
// to a dedicated i18n pass. "Raw" / "Split" are proper mode names; only
// "Pseudo-WYSIWYG" would need translation and it stays a coined term.
