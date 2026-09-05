// Implements FR-7 (Agenda Week — Story 6.4).
//
// The `/agenda/week` route's primary surface: a rolling 7-day window (the
// same Scheduled-within-window + Deadline-overdue-or-within-window legs
// `AgendaToday`'s single-day view already established, widened to
// `orgsidian-index::query::agenda::week`), grouped by calendar date instead
// of by source file, with the current day highlighted. Queries
// `commands.agendaWeek` once per mount — the backend query (plus its
// Rust-side stable sort by `agendaDate`) already does the filtering and the
// date-then-file-then-position sort, so this component's job is render +
// grouping only, never a second fetch or a client-side re-sort.

import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";

import { commands, type AgendaItemDto } from "@/lib/tauri";
import { localTodayIso } from "@/components/editor/schedule";
import { deadlineLabel, errorMessage } from "@/components/agenda/AgendaToday";

/** One calendar day's Agenda items, in the document order the backend sorted. */
interface AgendaDay {
  dateIso: string;
  items: AgendaItemDto[];
}

/**
 * `dateIso` (`YYYY-MM-DD`) plus `days` calendar days, computed entirely in
 * local time (never through a UTC-parsed `Date`, which would drift the
 * result near a DST boundary or a UTC-behind local zone): the components are
 * parsed by hand and fed to the local-time `Date` constructor, mirroring
 * `localTodayIso`'s own local-getter convention.
 */
function addDaysIso(dateIso: string, days: number): string {
  const [year, month, day] = dateIso.split("-").map(Number);
  const shifted = new Date(year, month - 1, day);
  shifted.setDate(shifted.getDate() + days);
  return localTodayIso(shifted);
}

/** The 7 ISO dates in the rolling window `[startDateIso, startDateIso + 6]`. */
function windowDates(startDateIso: string): string[] {
  return Array.from({ length: 7 }, (_, offset) => addDaysIso(startDateIso, offset));
}

/** A short, human display label for a `YYYY-MM-DD` date, e.g. "Sat, Sep 5". */
function formatDayLabel(dateIso: string): string {
  const [year, month, day] = dateIso.split("-").map(Number);
  const date = new Date(year, month - 1, day);
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
  }).format(date);
}

/**
 * Partition an already `(agendaDate, file_path, position)`-sorted list into
 * per-date groups, preserving that order (the AC's "grouped by date" — a
 * stable partition, never a re-sort here, mirroring `AgendaToday`'s
 * `groupByFile`).
 */
function groupByDate(items: AgendaItemDto[]): AgendaDay[] {
  const groups: AgendaDay[] = [];
  for (const item of items) {
    const current: AgendaDay | undefined = groups[groups.length - 1];
    if (current !== undefined && current.dateIso === item.agendaDate) {
      current.items.push(item);
    } else {
      groups.push({ dateIso: item.agendaDate, items: [item] });
    }
  }
  return groups;
}

/**
 * The `/agenda/week` route's Agenda list. Renders one of: a loading
 * placeholder, an error (query failed — most commonly "no active Vault"), or
 * the 7-day window with each day's items (an empty day still gets its own
 * heading, so the user can see which days are free — the AC's "plan beyond
 * today" intent).
 */
export function AgendaWeek() {
  const [items, setItems] = useState<AgendaItemDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The window's first ("current") day — both the query param and the
  // highlight target, computed once per mount so the highlight and the
  // fetch never disagree with each other mid-render.
  const [startDateIso] = useState(() => localTodayIso());

  useEffect(() => {
    let disposed = false;
    setError(null);
    setItems(null);

    commands
      .agendaWeek(startDateIso)
      .then((result) => {
        if (!disposed) setItems(result);
      })
      .catch((err: unknown) => {
        if (!disposed) setError(errorMessage(err));
      });

    return () => {
      disposed = true;
    };
  }, [startDateIso]);

  const groups = items !== null ? groupByDate(items) : [];
  const groupsByDate = new Map(groups.map((group) => [group.dateIso, group]));

  return (
    <section aria-labelledby="agenda-week-heading">
      <div className="flex items-baseline justify-between">
        <h1
          id="agenda-week-heading"
          className="text-2xl font-semibold text-[var(--org-fg-default)]"
        >
          This Week
        </h1>
        <Link
          to="/today"
          className="text-sm text-[var(--org-fg-muted)] underline hover:text-[var(--org-fg-default)]"
        >
          Back to Today
        </Link>
      </div>

      {error !== null && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {error}
        </p>
      )}

      {error === null && items === null && (
        <p className="mt-3 text-sm text-[var(--org-fg-muted)]">Loading…</p>
      )}

      {error === null && items !== null && (
        <div className="mt-4 flex flex-col gap-6">
          {windowDates(startDateIso).map((dateIso) => {
            const isCurrentDay = dateIso === startDateIso;
            const day = groupsByDate.get(dateIso);
            return (
              <div
                key={dateIso}
                className={
                  isCurrentDay
                    ? "rounded border border-[var(--org-border-focus)] p-3"
                    : "p-3"
                }
              >
                <h2 className="text-sm font-medium text-[var(--org-fg-muted)]">
                  {formatDayLabel(dateIso)}
                  {isCurrentDay && (
                    <span className="ml-2 text-xs text-[var(--org-fg-subtle)]">
                      (Today)
                    </span>
                  )}
                </h2>
                {day === undefined ? (
                  <p className="mt-2 text-sm text-[var(--org-fg-muted)]">
                    Nothing scheduled or due.
                  </p>
                ) : (
                  <ul className="mt-2 flex flex-col gap-1">
                    {day.items.map((item) => (
                      <AgendaWeekRow key={item.headlineId} item={item} todayIso={startDateIso} />
                    ))}
                  </ul>
                )}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

/**
 * One clickable Agenda row for the Week view. Same click-to-open
 * `/editor/$filePath/$headlineId` target `AgendaToday`'s row uses, but the
 * source file is shown inline (the grouping key here is the date, not the
 * file, so the file identity is no longer implied by a section heading).
 */
function AgendaWeekRow({ item, todayIso }: { item: AgendaItemDto; todayIso: string }) {
  return (
    <li>
      <Link
        to="/editor/$filePath/$headlineId"
        params={{ filePath: item.filePath, headlineId: String(item.headlineId) }}
        search={{ byteStart: item.byteStart }}
        className="flex items-baseline gap-2 rounded px-2 py-1 text-sm text-[var(--org-fg-default)] hover:bg-[var(--org-bg-surface)]"
      >
        {item.todoKeyword !== null && (
          <span className="font-mono text-xs text-[var(--org-fg-subtle)]">
            {item.todoKeyword}
          </span>
        )}
        <span>{item.title}</span>
        <span className="text-xs text-[var(--org-fg-subtle)]">{item.filePath}</span>
        {item.deadlineDate !== null && (
          <span
            className={
              item.overdue
                ? "text-xs text-destructive"
                : "text-xs text-[var(--org-fg-subtle)]"
            }
          >
            {deadlineLabel(item, todayIso)} ({item.deadlineDate})
          </span>
        )}
      </Link>
    </li>
  );
}
