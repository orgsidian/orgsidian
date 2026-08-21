// Implements FR-9 — Org UI Kit: Schedule/Deadline date picker (Story 4.8).
//
// `OrgDatePicker` is the inline, keyboard-first picker the editor opens for
// "Set Schedule" / "Set Deadline" on the current Headline. It offers a month
// calendar, a clock-time input, and the FR-9 fast-entry relative shortcuts
// (`Today`, `+1d`, `+1w`); Enter commits, Esc cancels (the Fantastical-style
// pattern from epic-4-context.md). It is deliberately dumb about *where* the
// value is written: on confirm it hands a `{ date, time }` value to `onConfirm`
// and the editor controller (`components/editor/schedule.ts`) routes the write
// through the typed `commands.setScheduled` client — this component never
// touches the buffer, IPC, or the parser.
//
// The `+1d`/`+1w` buttons move the *calendar selection* client-side for instant
// feedback; the canonical relative-shortcut semantics live in the pure-Rust
// `resolve_date_shortcut` (used by the command for typed/raw shortcut entry),
// so the two never disagree on `today`/`+1d`/`+1w`.
//
// Styling uses the `--org-*` token vocabulary (LD-6). Strings are plain English,
// matching the sibling Org UI Kit widgets (`TodoStateCycler`) — no lingui macros
// here, so the i18n catalog is unchanged.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

/** Which planning keyword the picker is editing (drives the heading label). */
export type OrgPlanningKind = "scheduled" | "deadline";

/**
 * A picker value. `date` is a local calendar day as `YYYY-MM-DD`; `time` is an
 * optional `HH:MM` clock time (`null` = all-day). This is exactly the shape the
 * editor forwards to `commands.setScheduled` (`TimestampInput`).
 */
export interface OrgDatePickerValue {
  date: string;
  time: string | null;
}

export interface OrgDatePickerProps {
  /** Planning keyword being set — labels the picker and its confirm button. */
  kind: OrgPlanningKind;
  /**
   * The value the current Headline already carries, when any, so the picker
   * opens on the existing date/time (edit) rather than today (add).
   */
  initial?: OrgDatePickerValue | null;
  /**
   * Reference "today" — injectable so tests are deterministic and so the host
   * can pass the user's local date. Defaults to the runtime's current date.
   */
  today?: Date;
  /** Called with the committed value when the user confirms (Enter / button). */
  onConfirm: (value: OrgDatePickerValue) => void;
  /** Called when the user cancels (Esc / button) — no write happens. */
  onCancel: () => void;
}

/** Stable DOM hooks so the host and tests locate the picker and its controls. */
export const ORG_DATE_PICKER_CLASS = {
  root: "org-date-picker",
  grid: "org-date-picker-grid",
  day: "org-date-picker-day",
  daySelected: "org-date-picker-day-selected",
} as const;

const WEEKDAY_LABELS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] as const;
const MONTH_LABELS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
] as const;

/** Local-calendar `YYYY-MM-DD` for `date` (no UTC shift — planning dates are a
 * wall-clock calendar day, not an instant). */
function toIsoDate(date: Date): string {
  const y = date.getFullYear().toString().padStart(4, "0");
  const m = (date.getMonth() + 1).toString().padStart(2, "0");
  const d = date.getDate().toString().padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Parse a `YYYY-MM-DD` string into a local `Date` (midnight), or `null`. */
function fromIsoDate(iso: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (match === null) return null;
  const date = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return Number.isNaN(date.getTime()) ? null : date;
}

/** A copy of `date` advanced by `days` (JS `Date` handles month/year rollover). */
function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

/** Same calendar day (ignores time-of-day). */
function sameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/**
 * Inline Schedule/Deadline date picker. Keyboard-first: Enter commits, Esc
 * cancels; the month grid, time field, and relative shortcuts drive the
 * selection.
 */
export function OrgDatePicker({
  kind,
  initial,
  today,
  onConfirm,
  onCancel,
}: OrgDatePickerProps) {
  const now = useMemo(() => today ?? new Date(), [today]);
  const initialDate = useMemo(
    () => (initial != null ? fromIsoDate(initial.date) : null) ?? now,
    [initial, now],
  );

  const [selected, setSelected] = useState<Date>(initialDate);
  const [time, setTime] = useState<string>(initial?.time ?? "");
  // The month currently shown in the grid (day-of-month is irrelevant here).
  const [viewMonth, setViewMonth] = useState<Date>(
    () => new Date(initialDate.getFullYear(), initialDate.getMonth(), 1),
  );
  const rootRef = useRef<HTMLDivElement | null>(null);

  // Focus the picker on open so Enter/Esc are captured without a click first.
  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  const commit = useCallback(() => {
    onConfirm({ date: toIsoDate(selected), time: time === "" ? null : time });
  }, [onConfirm, selected, time]);

  const selectDay = useCallback((date: Date) => {
    setSelected(date);
    setViewMonth(new Date(date.getFullYear(), date.getMonth(), 1));
  }, []);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (event.key === "Enter") {
        event.preventDefault();
        commit();
      } else if (event.key === "Escape") {
        event.preventDefault();
        onCancel();
      }
    },
    [commit, onCancel],
  );

  // Month grid: leading blanks for the first-of-month weekday, then each day.
  const monthDays = useMemo(() => {
    const year = viewMonth.getFullYear();
    const month = viewMonth.getMonth();
    const firstWeekday = new Date(year, month, 1).getDay();
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const cells: Array<Date | null> = [];
    for (let i = 0; i < firstWeekday; i += 1) cells.push(null);
    for (let d = 1; d <= daysInMonth; d += 1) cells.push(new Date(year, month, d));
    return cells;
  }, [viewMonth]);

  const heading = kind === "scheduled" ? "Set Schedule" : "Set Deadline";

  return (
    <div
      ref={rootRef}
      className={ORG_DATE_PICKER_CLASS.root}
      role="dialog"
      aria-label={heading}
      tabIndex={-1}
      onKeyDown={onKeyDown}
      style={{
        display: "inline-flex",
        flexDirection: "column",
        gap: "0.5rem",
        padding: "0.75rem",
        minWidth: "16rem",
        borderRadius: "8px",
        border: "1px solid var(--org-border-default)",
        backgroundColor: "var(--org-bg-elevated)",
        color: "var(--org-fg-default)",
        boxShadow: "0 8px 24px rgba(0, 0, 0, 0.18)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <strong>{heading}</strong>
        <span aria-live="polite" style={{ color: "var(--org-fg-muted)", fontVariantNumeric: "tabular-nums" }}>
          {toIsoDate(selected)}
          {time !== "" ? ` ${time}` : ""}
        </span>
      </div>

      {/* Relative fast-entry shortcuts (FR-9). */}
      <div style={{ display: "flex", gap: "0.25rem", flexWrap: "wrap" }}>
        <button type="button" data-shortcut="today" onClick={() => selectDay(now)}>
          Today
        </button>
        <button type="button" data-shortcut="+1d" onClick={() => selectDay(addDays(selected, 1))}>
          +1d
        </button>
        <button type="button" data-shortcut="+1w" onClick={() => selectDay(addDays(selected, 7))}>
          +1w
        </button>
      </div>

      {/* Month navigation. */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <button
          type="button"
          aria-label="Previous month"
          onClick={() =>
            setViewMonth((m) => new Date(m.getFullYear(), m.getMonth() - 1, 1))
          }
        >
          {"<"}
        </button>
        <span>
          {MONTH_LABELS[viewMonth.getMonth()]} {viewMonth.getFullYear()}
        </span>
        <button
          type="button"
          aria-label="Next month"
          onClick={() =>
            setViewMonth((m) => new Date(m.getFullYear(), m.getMonth() + 1, 1))
          }
        >
          {">"}
        </button>
      </div>

      {/* Calendar grid. */}
      <div
        className={ORG_DATE_PICKER_CLASS.grid}
        role="grid"
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(7, 1fr)",
          gap: "2px",
          textAlign: "center",
        }}
      >
        {WEEKDAY_LABELS.map((label) => (
          <span key={label} style={{ color: "var(--org-fg-muted)", fontSize: "0.75em" }}>
            {label}
          </span>
        ))}
        {monthDays.map((day, index) =>
          day === null ? (
            // eslint-disable-next-line react/no-array-index-key
            <span key={`blank-${index}`} />
          ) : (
            <button
              key={toIsoDate(day)}
              type="button"
              role="gridcell"
              aria-selected={sameDay(day, selected)}
              className={`${ORG_DATE_PICKER_CLASS.day}${
                sameDay(day, selected) ? ` ${ORG_DATE_PICKER_CLASS.daySelected}` : ""
              }`}
              onClick={() => selectDay(day)}
              style={{
                padding: "0.2rem 0",
                borderRadius: "4px",
                border: "1px solid transparent",
                cursor: "pointer",
                backgroundColor: sameDay(day, selected)
                  ? "var(--org-border-focus)"
                  : "transparent",
                color: sameDay(day, selected) ? "var(--org-bg-canvas)" : "inherit",
              }}
            >
              {day.getDate()}
            </button>
          ),
        )}
      </div>

      {/* Clock time (optional). */}
      <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>Time</span>
        <input
          type="time"
          aria-label="Time"
          value={time}
          onChange={(event) => setTime(event.target.value)}
        />
      </label>

      {/* Commit / cancel. */}
      <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
        <button type="button" data-action="cancel" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" data-action="confirm" onClick={commit}>
          {heading}
        </button>
      </div>
    </div>
  );
}
