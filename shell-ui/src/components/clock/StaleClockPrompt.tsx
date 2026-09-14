// Implements FR-8 (functional) / UJ-1: the prior-session running-clock launch
// prompt (Story 7.7).
//
// On mount (once) this asks the backend whether a prior-session Active Clock
// exists (`commands.getStaleClock`). While that is in flight, or when there is
// no stale clock, it renders nothing. Otherwise it opens a modal naming the
// tracked Headline, its last-active time, and both candidate durations, and
// offers three actions in safest-default order:
//
//   [Adjust end time]  (default-focused / Enter / Esc) — reveal a time picker
//                       pre-filled from `lastActiveAt`; confirm closes the open
//                       CLOCK line at the chosen end (`commands.clockAdjustEnd`).
//   [Keep tracking]     resume from the original `started_at`, no source
//                       mutation (`commands.clockResume`).
//   [Discard session]   remove the open CLOCK line from the LOGBOOK
//                       (`commands.clockDiscard`).
//
// Esc invokes Adjust (never a cancel/close): the modal has no
// dismiss-without-choosing path. See `docs/microcopy-registry.md` (Story 7.7)
// for the copy and `crates/orgsidian-core/src/clock.rs` for the transitions.

import { useEffect, useState } from "react";

import { commands, type StaleClockDto } from "@/lib/tauri";
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
 *  - `null` — no stale clock, or the check failed / no active Vault (nothing);
 *  - `StaleClockDto` — a prior-session clock to reconcile (open the modal).
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
    let cancelled = false;
    void commands
      .getStaleClock()
      .then((result) => {
        if (cancelled) return;
        setSummary(result);
        if (result) {
          setEndDate(isoDate(result.lastActiveAt));
          setEndTime(isoTime(result.lastActiveAt));
          setOpen(true);
        }
      })
      .catch(() => {
        // Fail safe to hidden (e.g. no active Vault, or a desynced pointer):
        // a launch-check outage must never trap the user behind a modal.
        if (!cancelled) setSummary(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!summary) {
    return null;
  }

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
    run(() => commands.clockResume(summary.headlineId));
  };

  const discard = () => {
    run(() => commands.clockDiscard());
  };

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent
        showCloseButton={false}
        data-testid="stale-clock-prompt"
        // Associate the description, the durations line, and (when present) the
        // error, alongside the DialogDescription — so a screen reader announces
        // all of the modal's explanatory copy.
        aria-describedby={`stale-clock-desc stale-clock-durations${
          error ? " stale-clock-error" : ""
        }`}
        // Default-focus the safest action ("Adjust end time") on open.
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          const target = event.currentTarget as HTMLElement;
          target
            .querySelector<HTMLButtonElement>("[data-default-action]")
            ?.focus();
        }}
        // Esc invokes Adjust (not cancel/close) — the modal has no
        // dismiss-without-choosing path.
        onEscapeKeyDown={(event) => {
          event.preventDefault();
          openAdjust();
        }}
        onInteractOutside={(event) => {
          event.preventDefault();
        }}
        onPointerDownOutside={(event) => {
          event.preventDefault();
        }}
      >
        <DialogHeader>
          <DialogTitle>A clock was still running</DialogTitle>
          <DialogDescription id="stale-clock-desc">
            You were tracking <strong>{summary.headline}</strong>. It was last
            active {lastActiveLabel(summary.lastActiveAt)}.
          </DialogDescription>
        </DialogHeader>

        <p id="stale-clock-durations" className="text-sm text-muted-foreground">
          So far, this session is{" "}
          <strong className="text-foreground">{summary.keepDuration}</strong> if
          you keep tracking, or{" "}
          <strong className="text-foreground">{summary.adjustDuration}</strong>{" "}
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

        <DialogFooter>
          <Button variant="destructive" onClick={discard} disabled={busy}>
            Discard this session
          </Button>
          <Button variant="outline" onClick={keepTracking} disabled={busy}>
            Keep tracking
          </Button>
          <Button
            data-default-action
            onClick={openAdjust}
            disabled={busy}
            aria-expanded={adjusting}
            aria-controls="stale-clock-adjust-panel"
          >
            Adjust end time
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
