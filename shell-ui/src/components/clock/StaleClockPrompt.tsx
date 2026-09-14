// Implements FR-8 (functional) / UJ-1: the prior-session running-clock launch
// prompt (Story 7.7).
//
// ONCE PER APP LAUNCH. `<StaleClockPrompt/>` is mounted inside the `/today`
// route, which TanStack Router unmounts/remounts on every navigation. The
// stale-clock check therefore runs behind a session-scoped once-guard
// (`claimStaleClockEvaluation`, see `staleClockSession.ts`): it is evaluated a
// single time per app launch, never on each route remount. This is the AC's
// "prior-session running-clock prompt ON LAUNCH": a clock STARTED in the
// current session can never be re-surfaced (navigating back to Today after
// clocking a task no longer pops the modal on a live, seconds-old clock and
// offers a destructive "Discard").
//
// On that single evaluation this asks the backend whether a prior-session
// Active Clock exists (`commands.getStaleClock`), which resolves to one of:
//   - `null` — no stale clock (render nothing);
//   - `{ state: "summary", … }` — a prior-session clock to reconcile; open the
//     modal naming the tracked Headline, its last-active time, and both
//     candidate durations, offering three actions in safest-default order:
//
//       [Adjust end time]  (default-focused / Enter / Esc) — reveal a time
//                           picker pre-filled from `lastActiveAt`; confirm
//                           closes the open CLOCK line at the chosen end
//                           (`commands.clockAdjustEnd`).
//       [Keep tracking]     resume from the original `started_at`, no source
//                           mutation (`commands.clockResume`).
//       [Discard session]   remove the open CLOCK line from the LOGBOOK
//                           (`commands.clockDiscard`).
//
//   - `{ state: "desynced", … }` — the pointer references a Headline no longer
//     in the index (an index/source desync). The full summary cannot be shown,
//     but the stuck `active-clock.json` is still discardable, so the modal
//     opens in a DISCARD-ONLY recovery state (name unknown / headline missing
//     copy). Without this the launch check's `.catch()` would treat the desync
//     like "nothing to show" and hide the modal, stranding the pointer with no
//     in-app recovery.
//
// Esc invokes Adjust (never a cancel/close): the modal has no
// dismiss-without-choosing path. See `docs/microcopy-registry.md` (Story 7.7)
// for the copy and `crates/orgsidian-core/src/clock.rs` for the transitions.

import { useEffect, useState } from "react";

import { commands, type StaleClockDto } from "@/lib/tauri";
import { claimStaleClockEvaluation } from "./staleClockSession";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

/**
 * Best-effort extraction of a human-readable message from a thrown command
 * error (mirrors `VaultPicker`/`AgendaToday` — `ErrorHandlingMode::Throw`
 * throws the serialized `OrgError` `{ kind, reason }`).
 */
export function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "reason" in err) {
    return String((err as { reason: unknown }).reason);
  }
  return String(err);
}

/** The `YYYY-MM-DD` date part of a `%Y-%m-%dT%H:%M:%S` pointer timestamp. */
function isoDate(ts: string): string {
  return ts.slice(0, 10);
}

/** The `HH:MM` time part of a `%Y-%m-%dT%H:%M:%S` pointer timestamp. */
function isoTime(ts: string): string {
  return ts.slice(11, 16);
}

/** Human-readable last-active label, e.g. `2026-09-13 at 18:00`. */
function lastActiveLabel(ts: string): string {
  return `${isoDate(ts)} at ${isoTime(ts)}`;
}

/**
 * Loading gate:
 *  - `undefined` — the `getStaleClock` check is in flight (render nothing);
 *  - `null` — no stale clock, the check failed / no active Vault, or this is a
 *    route remount after the once-per-launch evaluation already ran (nothing);
 *  - `StaleClockDto` — a prior-session clock to reconcile (`summary`) or a
 *    desynced pointer to recover (`desynced`) — open the modal.
 */
type LoadState = StaleClockDto | null | undefined;

export function StaleClockPrompt() {
  const [summary, setSummary] = useState<LoadState>(undefined);
  const [open, setOpen] = useState(false);
  const [adjusting, setAdjusting] = useState(false);
  const [endDate, setEndDate] = useState("");
  const [endTime, setEndTime] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Once per app launch, not per route mount: a route remount (navigating
    // back to Today) must not re-run the check and pop the modal on a clock the
    // user started in THIS session. `claimStaleClockEvaluation` returns true
    // only for the first caller of the session; later remounts render nothing.
    if (!claimStaleClockEvaluation()) {
      setSummary(null);
      return;
    }

    let cancelled = false;
    void commands
      .getStaleClock()
      .then((result) => {
        if (cancelled) return;
        setSummary(result);
        if (result) {
          if (result.state === "summary") {
            setEndDate(isoDate(result.lastActiveAt));
            setEndTime(isoTime(result.lastActiveAt));
          }
          setOpen(true);
        }
      })
      .catch(() => {
        // Fail safe to hidden (e.g. no active Vault, or an unparseable
        // started_at): a launch-check outage must never trap the user behind a
        // modal. The headline-not-found desync does NOT arrive here — it
        // resolves as a `desynced` result above so its recovery is offered.
        if (!cancelled) setSummary(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!summary) {
    return null;
  }

  const desynced = summary.state === "desynced";

  // Run one reconciliation command, then close the modal on success. A failure
  // surfaces the reason and leaves the modal open so the user can choose again.
  const run = (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    void action()
      .then(() => {
        setOpen(false);
      })
      .catch((err) => {
        setError(errorMessage(err));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const openAdjust = () => {
    setError(null);
    setAdjusting(true);
  };

  const confirmAdjust = () => {
    run(() => commands.clockAdjustEnd(`${endDate}T${endTime}:00`));
  };

  const keepTracking = () => {
    if (summary.state !== "summary") return;
    run(() => commands.clockResume(summary.headlineId));
  };

  const discard = () => {
    run(() => commands.clockDiscard());
  };

  // Discard from the desync-recovery state: `clock_discard` clears the stuck
  // pointer even in the headline-not-found branch (it returns after clearing
  // the dangling pointer), so the recovery has succeeded whether the call
  // resolves or rejects — close the modal on settle rather than trapping the
  // user behind a message that says the pointer was already cleared.
  const discardRecovery = () => {
    setBusy(true);
    setError(null);
    void commands
      .clockDiscard()
      .catch(() => {
        // The headline-not-found branch clears the stuck pointer, THEN reports
        // the desync as an error — the recovery has still succeeded, so swallow
        // it (never trap the user behind a message that says the pointer was
        // already cleared).
      })
      .finally(() => {
        setBusy(false);
        setOpen(false);
      });
  };

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent
        showCloseButton={false}
        data-testid="stale-clock-prompt"
        // Associate the description, the durations line, and (when present) the
        // error, alongside the DialogDescription — so a screen reader announces
        // all of the modal's explanatory copy.
        aria-describedby={
          desynced
            ? `stale-clock-desc${error ? " stale-clock-error" : ""}`
            : `stale-clock-desc stale-clock-durations${
                error ? " stale-clock-error" : ""
              }`
        }
        // Default-focus the safest action on open ("Adjust end time" in the
        // summary state; "Discard this session" is the only action in recovery).
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          const target = event.currentTarget as HTMLElement;
          target
            .querySelector<HTMLButtonElement>("[data-default-action]")
            ?.focus();
        }}
        // Esc invokes Adjust (not cancel/close) in the summary state — the modal
        // has no dismiss-without-choosing path. In the recovery state there is
        // no Adjust, so Esc is simply swallowed (still no dismiss path).
        onEscapeKeyDown={(event) => {
          event.preventDefault();
          if (!desynced) openAdjust();
        }}
        onInteractOutside={(event) => {
          event.preventDefault();
        }}
        onPointerDownOutside={(event) => {
          event.preventDefault();
        }}
      >
        {summary.state === "desynced" ? (
          <>
            <DialogHeader>
              <DialogTitle>A clock was still running</DialogTitle>
              <DialogDescription id="stale-clock-desc">
                A clock was left running, but the task it was tracking is no
                longer in your Vault, so it can't be resumed or adjusted. You can
                discard this session to clear it.
              </DialogDescription>
            </DialogHeader>

            {error && (
              <p
                id="stale-clock-error"
                role="alert"
                className="text-sm text-destructive"
              >
                Couldn't update the clock: {error}
              </p>
            )}

            <DialogFooter>
              <Button
                data-default-action
                variant="destructive"
                onClick={discardRecovery}
                disabled={busy}
              >
                Discard this session
              </Button>
            </DialogFooter>
          </>
        ) : (
          <>
            <DialogHeader>
              <DialogTitle>A clock was still running</DialogTitle>
              <DialogDescription id="stale-clock-desc">
                You were tracking <strong>{summary.headline}</strong>. It was
                last active {lastActiveLabel(summary.lastActiveAt)}.
              </DialogDescription>
            </DialogHeader>

            <p
              id="stale-clock-durations"
              className="text-sm text-muted-foreground"
            >
              So far, this session is{" "}
              <strong className="text-foreground">
                {summary.keepDuration}
              </strong>{" "}
              if you keep tracking, or{" "}
              <strong className="text-foreground">
                {summary.adjustDuration}
              </strong>{" "}
              if you adjust to the last active time.
            </p>

            {adjusting && (
              <div
                id="stale-clock-adjust-panel"
                className="flex flex-col gap-2 rounded-md border p-3"
                data-testid="stale-clock-adjust"
              >
                <label className="flex items-center justify-between gap-2 text-sm">
                  End date
                  <input
                    type="date"
                    value={endDate}
                    aria-label="End date"
                    onChange={(event) => setEndDate(event.target.value)}
                    className="rounded-md border bg-background px-2 py-1 text-sm"
                  />
                </label>
                <label className="flex items-center justify-between gap-2 text-sm">
                  End time
                  <input
                    type="time"
                    value={endTime}
                    aria-label="End time"
                    onChange={(event) => setEndTime(event.target.value)}
                    className="rounded-md border bg-background px-2 py-1 text-sm"
                  />
                </label>
                <Button
                  onClick={confirmAdjust}
                  disabled={busy || !endDate || !endTime}
                >
                  Save end time
                </Button>
              </div>
            )}

            {error && (
              <p
                id="stale-clock-error"
                role="alert"
                className="text-sm text-destructive"
              >
                Couldn't update the clock: {error}
              </p>
            )}

            {/* Spec order (left→right on desktop): [Adjust end time] [Keep
                tracking] [Discard this session]. `DialogFooter` is
                `flex-col-reverse … sm:flex-row`, so the DESKTOP row reads in DOM
                order — the buttons are authored Adjust→Keep→Discard to match the
                frozen spec. Default focus + Enter + Esc all target "Adjust end
                time". */}
            <DialogFooter>
              <Button
                data-default-action
                onClick={openAdjust}
                disabled={busy}
                aria-expanded={adjusting}
                aria-controls="stale-clock-adjust-panel"
              >
                Adjust end time
              </Button>
              <Button variant="outline" onClick={keepTracking} disabled={busy}>
                Keep tracking
              </Button>
              <Button variant="destructive" onClick={discard} disabled={busy}>
                Discard this session
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
