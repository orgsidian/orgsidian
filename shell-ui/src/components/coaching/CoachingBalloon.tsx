// Implements FR-21 (partial) / FR-18 / UJ-4 (Story 6.6).
//
// A hardcoded, non-modal coaching balloon for the UJ-4 first-run experience.
// Self-contained (mirrors `ConflictBanner`'s ownership model): on mount it
// asks the backend whether ITS `id` is already dismissed
// (`commands.getDismissedCoaching`), renders nothing while that check is in
// flight or once dismissed, and persists a dismissal
// (`commands.dismissCoaching`) via the X button.
//
// This is the explicitly disposable v0.1 stand-in the epic AC calls for —
// Story 11.4 (v0.5 Beta) REMOVES this component wholesale when the
// registry-driven `CoachingSlot` API ships, importing only the coaching ids
// (`coachingIds.ts`) to honor existing dismissals. Keep this component free
// of anything Story 11.4 would need to preserve beyond that.
//
// A11y (matches the Story 5.5 `ConflictBanner` calm/non-modal idiom): a
// `role="status"` / `aria-live="polite"` region — announced without stealing
// focus, never assertive. The dismiss control is a native `<button>`
// (keyboard-operable, individually tab-reachable) with a visible focus ring
// via `--org-border-focus`. `--org-*` tokens throughout; no new tokens added.

import { useEffect, useState } from "react";

import { commands } from "@/lib/tauri";

interface CoachingBalloonProps {
  /** Hardcoded coaching id (see `coachingIds.ts`) — the dismissal key. */
  id: string;
  /** Balloon copy. */
  children: React.ReactNode;
  className?: string;
  /**
   * Accessible name for the dismiss button. Two balloons can be live on
   * `/today` at once, so give each a distinct name (e.g. "Dismiss this tip
   * about your day") — a bare "Dismiss" leaves a screen-reader user with two
   * indistinguishable controls. Defaults to "Dismiss".
   */
  dismissLabel?: string;
}

export function CoachingBalloon({
  id,
  children,
  className,
  dismissLabel = "Dismiss",
}: CoachingBalloonProps) {
  // `null` while the dismissed-check is in flight — render nothing rather
  // than flash the balloon then immediately hide it for a returning user
  // whose Vault already has this id dismissed.
  const [dismissed, setDismissed] = useState<boolean | null>(null);

  useEffect(() => {
    let disposed = false;
    commands
      .getDismissedCoaching()
      .then((ids) => {
        if (!disposed) setDismissed(ids.includes(id));
      })
      .catch(() => {
        // No active Vault, or some other backend hiccup: fail safe to
        // hidden — a coaching hint is never worth surfacing an error over.
        if (!disposed) setDismissed(true);
      });
    return () => {
      disposed = true;
    };
  }, [id]);

  if (dismissed !== false) return null;

  function dismiss() {
    // Optimistic: hide immediately. A failed persist just means this balloon
    // may resurface on a later launch — an acceptable degrade for a
    // non-critical coaching hint, never worth blocking the dismiss action or
    // re-showing it mid-session.
    setDismissed(true);
    void commands.dismissCoaching(id).catch(() => {});
  }

  return (
    <div
      role="status"
      aria-live="polite"
      data-coaching-id={id}
      className={
        "flex items-start gap-3 rounded-lg border border-[var(--org-border-default)] bg-[var(--org-bg-elevated)] px-3 py-2 text-sm text-[var(--org-fg-default)] shadow-sm" +
        (className ? ` ${className}` : "")
      }
    >
      <p className="min-w-0 flex-1">{children}</p>
      <button
        type="button"
        aria-label={dismissLabel}
        onClick={dismiss}
        className="shrink-0 cursor-pointer rounded p-0.5 leading-none text-[var(--org-fg-muted)] outline-none transition-colors hover:bg-[var(--org-bg-surface)] focus-visible:ring-2 focus-visible:ring-[var(--org-border-focus)]"
      >
        <span aria-hidden="true">×</span>
      </button>
    </div>
  );
}
