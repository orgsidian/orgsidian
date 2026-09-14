// Implements FR-8 (functional): the Org UI Kit ClockEditor (Story 7.8) — a
// dialog for correcting an existing CLOSED `CLOCK: [start]--[end] => HH:MM`
// entry in the LOGBOOK drawer.
//
// The org file is the source of truth: confirming calls
// `commands.updateClockEntry(headlineId, entryIndex, newStart, newEnd)`, which
// rewrites the one CLOCK line byte-faithfully and recomputes the duration in
// the Rust core (`orgsidian-core::clock::update_clock_entry`). This component
// only edits and validates the two stamps.
//
// The three fields are interdependent (duration = end − start):
//   - editing start or end recomputes the duration;
//   - editing the duration recomputes the end (= start + duration).
//
// `entryIndex` is the 0-based index of the entry within its Headline's clock
// entries in document order — supplied by whatever renders the LOGBOOK list
// (the caller also passes the entry's current start/end and re-renders on
// `onSaved`). The running (open) entry is never edited here; it is corrected via
// the Story 7.7 adjust-end flow, and the core rejects an attempt to edit it.

import { useEffect, useState } from "react";

import { commands } from "@/lib/tauri";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

export interface ClockEditorProps {
  /** `headlines.id` of the Headline owning the entry. */
  headlineId: number;
  /** 0-based index of the entry within the Headline's clocks (document order). */
  entryIndex: number;
  /** The entry's current start, ISO `YYYY-MM-DDTHH:MM[:SS]`. */
  initialStart: string;
  /** The entry's current end, ISO `YYYY-MM-DDTHH:MM[:SS]`. */
  initialEnd: string;
  /** Whether the dialog is open (controlled by the caller). */
  open: boolean;
  /** Open-state changes (Cancel, Esc, outside-click, or a successful save). */
  onOpenChange: (open: boolean) => void;
  /** Fired after a successful write so the caller re-renders the LOGBOOK drawer. */
  onSaved?: () => void;
}

/**
 * Best-effort extraction of a human-readable message from a thrown command
 * error (mirrors `StaleClockPrompt`/`VaultPicker` — `ErrorHandlingMode::Throw`
 * throws the serialized `OrgError` `{ kind, reason }`).
 */
export function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "reason" in err) {
    return String((err as { reason: unknown }).reason);
  }
  return String(err);
}

/** The `YYYY-MM-DD` date part of an ISO datetime. */
function isoDate(ts: string): string {
  return ts.slice(0, 10);
}

/** The `HH:MM` time part of an ISO datetime. */
function isoTime(ts: string): string {
  return ts.slice(11, 16);
}

/** Parse a date+time pair into a local `Date`, or `null` when incomplete/invalid. */
export function toDate(date: string, time: string): Date | null {
  if (!date || !time) return null;
  const d = new Date(`${date}T${time}:00`);
  return Number.isNaN(d.getTime()) ? null : d;
}

/**
 * Format a millisecond delta as org's `H:MM` (hours unbounded, minutes
 * zero-padded). Truncates down to whole minutes and clamps a negative delta to
 * `0:00` — mirrors the Rust `format_duration`.
 */
export function formatDuration(ms: number): string {
  const totalMinutes = Math.max(0, Math.floor(ms / 60000));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return `${hours}:${String(minutes).padStart(2, "0")}`;
}

/** Parse an `H:MM` (or bare `H`) duration into whole minutes, or `null`. */
export function parseDuration(text: string): number | null {
  const trimmed = text.trim();
  const hm = /^(\d+):([0-5]?\d)$/.exec(trimmed);
  if (hm) return Number(hm[1]) * 60 + Number(hm[2]);
  const h = /^(\d+)$/.exec(trimmed);
  if (h) return Number(h[1]) * 60;
  return null;
}

/** `start + minutes`, returned as a local date/time pair for the native inputs. */
export function addMinutes(
  date: string,
  time: string,
  minutes: number,
): { date: string; time: string } | null {
  const base = toDate(date, time);
  if (base == null) return null;
  const next = new Date(base.getTime() + minutes * 60000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return {
    date: `${next.getFullYear()}-${pad(next.getMonth() + 1)}-${pad(next.getDate())}`,
    time: `${pad(next.getHours())}:${pad(next.getMinutes())}`,
  };
}

export function ClockEditor({
  headlineId,
  entryIndex,
  initialStart,
  initialEnd,
  open,
  onOpenChange,
  onSaved,
}: ClockEditorProps) {
  const [startDate, setStartDate] = useState(isoDate(initialStart));
  const [startTime, setStartTime] = useState(isoTime(initialStart));
  const [endDate, setEndDate] = useState(isoDate(initialEnd));
  const [endTime, setEndTime] = useState(isoTime(initialEnd));
  const [duration, setDuration] = useState("0:00");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Reset every field from the props whenever the dialog (re)opens or the target
  // entry changes, so reusing one <ClockEditor/> for different entries never
  // shows a stale entry's values.
  useEffect(() => {
    if (!open) return;
    const s = { date: isoDate(initialStart), time: isoTime(initialStart) };
    const e = { date: isoDate(initialEnd), time: isoTime(initialEnd) };
    setStartDate(s.date);
    setStartTime(s.time);
    setEndDate(e.date);
    setEndTime(e.time);
    const start = toDate(s.date, s.time);
    const end = toDate(e.date, e.time);
    setDuration(start && end ? formatDuration(end.getTime() - start.getTime()) : "0:00");
    setError(null);
  }, [open, initialStart, initialEnd]);

  const recomputeDuration = (sd: string, st: string, ed: string, et: string) => {
    const start = toDate(sd, st);
    const end = toDate(ed, et);
    if (start && end) setDuration(formatDuration(end.getTime() - start.getTime()));
  };

  const onStartDate = (v: string) => {
    setStartDate(v);
    recomputeDuration(v, startTime, endDate, endTime);
  };
  const onStartTime = (v: string) => {
    setStartTime(v);
    recomputeDuration(startDate, v, endDate, endTime);
  };
  const onEndDate = (v: string) => {
    setEndDate(v);
    recomputeDuration(startDate, startTime, v, endTime);
  };
  const onEndTime = (v: string) => {
    setEndTime(v);
    recomputeDuration(startDate, startTime, endDate, v);
  };

  // Editing the duration recomputes the END (start stays fixed).
  const onDuration = (v: string) => {
    setDuration(v);
    const minutes = parseDuration(v);
    if (minutes == null) return;
    const next = addMinutes(startDate, startTime, minutes);
    if (next) {
      setEndDate(next.date);
      setEndTime(next.time);
    }
  };

  const start = toDate(startDate, startTime);
  const end = toDate(endDate, endTime);
  const valid = !!start && !!end && end.getTime() >= start.getTime();

  const save = () => {
    if (!valid || busy) return;
    setBusy(true);
    setError(null);
    void commands
      .updateClockEntry(
        headlineId,
        entryIndex,
        // The entry's ORIGINAL start — the core verifies the entry still at
        // `entryIndex` starts here (LOGBOOK reorders on re-clock), rejecting a
        // stale index rather than rewriting the wrong entry.
        initialStart,
        `${startDate}T${startTime}:00`,
        `${endDate}T${endTime}:00`,
      )
      .then(() => {
        onSaved?.();
        onOpenChange(false);
      })
      .catch((err: unknown) => {
        setError(errorMessage(err));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        data-testid="clock-editor"
        aria-describedby={`clock-editor-desc${!valid ? " clock-editor-invalid" : ""}${error ? " clock-editor-error" : ""}`}
      >
        <DialogHeader>
          <DialogTitle>Edit time entry</DialogTitle>
          <DialogDescription id="clock-editor-desc">
            Change the start or end time; the duration updates to match. Editing
            the duration moves the end time.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-2">
          <label className="flex items-center justify-between gap-2 text-sm">
            Start date
            <input
              type="date"
              value={startDate}
              aria-label="Start date"
              onChange={(event) => onStartDate(event.target.value)}
              className="rounded-md border bg-background px-2 py-1 text-sm"
            />
          </label>
          <label className="flex items-center justify-between gap-2 text-sm">
            Start time
            <input
              type="time"
              value={startTime}
              aria-label="Start time"
              onChange={(event) => onStartTime(event.target.value)}
              className="rounded-md border bg-background px-2 py-1 text-sm"
            />
          </label>
          <label className="flex items-center justify-between gap-2 text-sm">
            End date
            <input
              type="date"
              value={endDate}
              aria-label="End date"
              onChange={(event) => onEndDate(event.target.value)}
              className="rounded-md border bg-background px-2 py-1 text-sm"
            />
          </label>
          <label className="flex items-center justify-between gap-2 text-sm">
            End time
            <input
              type="time"
              value={endTime}
              aria-label="End time"
              onChange={(event) => onEndTime(event.target.value)}
              className="rounded-md border bg-background px-2 py-1 text-sm"
            />
          </label>
          <label className="flex items-center justify-between gap-2 text-sm">
            Duration (H:MM)
            <input
              type="text"
              inputMode="numeric"
              value={duration}
              aria-label="Duration"
              onChange={(event) => onDuration(event.target.value)}
              className="rounded-md border bg-background px-2 py-1 text-sm"
            />
          </label>
        </div>

        {!valid && (
          <p
            id="clock-editor-invalid"
            role="alert"
            className="text-sm text-destructive"
          >
            End time must be at or after the start time.
          </p>
        )}

        {error && (
          <p
            id="clock-editor-error"
            role="alert"
            className="text-sm text-destructive"
          >
            Couldn't save the entry: {error}
          </p>
        )}

        <DialogFooter>
          <Button
            data-default-action
            onClick={save}
            disabled={!valid || busy}
          >
            Save
          </Button>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={busy}
          >
            Cancel
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
