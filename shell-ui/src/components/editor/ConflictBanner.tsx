import { useEffect, useState } from "react";

import { commands, events } from "@/lib/tauri";

/**
 * Story 5.5 (FR-16 / NFR-16 Single Writer Rule): the calm, non-modal conflict
 * banner shown in the editor surface when an external tool writes a file whose
 * in-memory buffer holds unsaved edits.
 *
 * This is the v0.1 Alpha *fallback* for FR-16 — the full three-pane Merge Dialog
 * is Epic 9. Per the epic UX, this is crisis UX dialed *down*: no modal, no
 * exclamation marks, no warning colors — a surface that makes the user feel held,
 * not alarmed. It surfaces that the save is BLOCKED (the backend `save_file`
 * command refuses the write with `VaultError::ExternalConflict` until the
 * conflict is resolved) and offers two ways out:
 *
 * - **Discard external changes** — clears the block (`commands.discardExternalChanges`)
 *   so the next save overwrites the external write via the normal atomic path,
 *   and dismisses the banner.
 * - **View file in default editor** — opens the file in the OS default app
 *   (`commands.openInDefaultEditor`) so the user can inspect the external write
 *   before deciding.
 *
 * State ownership: CM6 owns the buffer; this banner owns only "is there an
 * unresolved conflict for THIS file?", driven by the `conflict-detected` event.
 * It never touches the buffer.
 *
 * A11y (repo hard-gates WCAG 2.1 AA): the banner is a `role="status"` region
 * with `aria-live="polite"` — announced without stealing focus, matching the
 * epic's "calm, never assertive" directive (an `alert`/`assertive` region would
 * contradict the dialed-down UX). Both actions are native `<button>`s: keyboard
 * operable (Enter/Space) and individually tab-reachable, with a visible focus
 * ring via `--org-border-focus`. Meaning is carried by text, never color alone.
 */
interface ConflictBannerProps {
  /** Absolute path of the `.org` file this editor surface is showing. Only a
   *  `conflict-detected` event for THIS path raises the banner. */
  filePath: string;
}

export function ConflictBanner({ filePath }: ConflictBannerProps) {
  // Whether an unresolved external conflict is currently blocking THIS file's
  // save. Raised by the event, lowered by "Discard external changes".
  const [conflicted, setConflicted] = useState(false);

  useEffect(() => {
    // A NEW file path starts with a clean slate: a stale conflict from a
    // previously-open file must not linger on the newly-shown one.
    setConflicted(false);

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void events.conflictDetected
      .listen((event) => {
        // Only react to a conflict on the file this surface is showing.
        if (event.payload.path === filePath) {
          setConflicted(true);
        }
      })
      .then((dispose) => {
        if (disposed) {
          dispose();
        } else {
          unlisten = dispose;
        }
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [filePath]);

  if (!conflicted) return null;

  async function discard() {
    // Clear the backend block first; only dismiss the banner once it succeeds,
    // so a failed clear leaves the (still-blocking) conflict visible.
    try {
      await commands.discardExternalChanges(filePath);
      setConflicted(false);
    } catch {
      // Keep the banner up — the save is still blocked server-side.
    }
  }

  function viewInEditor() {
    // Best-effort: a failed open must not tear down the banner or throw.
    void commands.openInDefaultEditor(filePath).catch(() => {});
  }

  return (
    <div
      role="status"
      aria-live="polite"
      data-conflict-path={filePath}
      className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-4 py-2 text-sm text-[var(--org-fg-default)]"
    >
      <p className="min-w-0 flex-1">
        <span className="font-medium break-all">{filePath}</span>
        <span className="text-[var(--org-fg-muted)]">
          {" "}
          was changed externally — save blocked.
        </span>
      </p>
      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          onClick={discard}
          className="cursor-pointer rounded border border-[var(--org-border-default)] px-2.5 py-1 text-xs font-medium whitespace-nowrap transition-colors outline-none hover:bg-[var(--org-bg-elevated)] focus-visible:ring-2 focus-visible:ring-[var(--org-border-focus)]"
        >
          Discard external changes
        </button>
        <button
          type="button"
          onClick={viewInEditor}
          className="cursor-pointer rounded border border-[var(--org-border-default)] px-2.5 py-1 text-xs font-medium whitespace-nowrap transition-colors outline-none hover:bg-[var(--org-bg-elevated)] focus-visible:ring-2 focus-visible:ring-[var(--org-border-focus)]"
        >
          View file in default editor
        </button>
      </div>
    </div>
  );
}

// i18n note: banner copy + button labels are plain strings, matching the sibling
// editor/settings components (ModeSwitcher, IndexScanProgress, VaultPicker). The
// Vitest transform (esbuild) does not run the Lingui SWC macro plugin, so
// wrapping these in `<Trans>` would break the component tests; the repo defers
// UI-string extraction to a dedicated i18n pass.
