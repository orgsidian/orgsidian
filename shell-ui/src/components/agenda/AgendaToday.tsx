// Implements FR-7 (Agenda Today — Story 6.3 v0.1 subset).
//
// The `/today` route's primary surface: Scheduled-today + Deadline-overdue-
// or-today, grouped by source file, click-to-open at the source Headline.
// Queries `commands.agendaToday` once per mount — the backend query
// (`orgsidian-index::query::agenda::today`) already does the filtering and
// the file-then-position sort, so this component's job is render + grouping
// only, never a second fetch or a client-side re-sort.
//
// The full Today Dashboard (Inbox preview, Active Clock, Today-Tag section,
// collapsible sections, empty-state copy) is Epic 7 (Stories 7.1/7.3) layered
// on top of this surface — this component stays the Scheduled+Deadline list
// Story 7.1 upgrades, not a dashboard of its own.

import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";

import { commands, type AgendaItemDto } from "@/lib/tauri";
import { localTodayIso } from "@/components/editor/schedule";
import { CoachingBalloon } from "@/components/coaching/CoachingBalloon";
import { UJ4_TODAY_INTRO } from "@/components/coaching/coachingIds";

/**
 * Best-effort extraction of a human-readable message from a thrown command
 * error (mirrors `VaultPicker`'s helper — `ErrorHandlingMode::Throw` throws
 * the serialized `OrgError` `{ kind, reason }`).
 */
export function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "reason" in err) {
    return String((err as { reason: unknown }).reason);
  }
  return String(err);
}

/** One source file's Agenda items, in the document order the backend sorted. */
interface AgendaGroup {
  filePath: string;
  items: AgendaItemDto[];
}

/**
 * Partition an already `(filePath, position)`-sorted list into per-file
 * groups, preserving that order (the AC's "grouped by source file" — a
 * stable partition, never a re-sort here).
 */
function groupByFile(items: AgendaItemDto[]): AgendaGroup[] {
  const groups: AgendaGroup[] = [];
  for (const item of items) {
    const current: AgendaGroup | undefined = groups[groups.length - 1];
    if (current !== undefined && current.filePath === item.filePath) {
      current.items.push(item);
    } else {
      groups.push({ filePath: item.filePath, items: [item] });
    }
  }
  return groups;
}

/**
 * The `/today` route's Agenda list. Renders one of: a loading placeholder, an
 * error (query failed — most commonly "no active Vault"), an empty-state
 * line, or the grouped item list.
 */
export function AgendaToday() {
  const [items, setItems] = useState<AgendaItemDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The local calendar day the deadline badges are read against, so a Deadline
  // due today reads "Due today" while a future Deadline carried by a
  // Scheduled-today row is not mislabelled as due today.
  const todayIso = localTodayIso();

  useEffect(() => {
    let disposed = false;
    setError(null);
    setItems(null);

    commands
      .agendaToday(localTodayIso())
      .then((result) => {
        if (!disposed) setItems(result);
      })
      .catch((err: unknown) => {
        if (!disposed) setError(errorMessage(err));
      });

    return () => {
      disposed = true;
    };
  }, []);

  return (
    <section aria-labelledby="agenda-today-heading">
      <div className="flex items-baseline justify-between">
        <h1
          id="agenda-today-heading"
          className="text-2xl font-semibold text-[var(--org-fg-default)]"
        >
          Today
        </h1>
        {/* Story 6.4: the view-switch into the rolling 7-day Week Agenda. */}
        <Link
          to="/agenda/week"
          className="text-sm text-[var(--org-fg-muted)] underline hover:text-[var(--org-fg-default)]"
        >
          View week
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

      {error === null && items !== null && items.length === 0 && (
        <p className="mt-3 text-sm text-[var(--org-fg-muted)]">
          Nothing scheduled or due today.
        </p>
      )}

      {error === null && items !== null && items.length > 0 && (
        <div className="mt-4 flex flex-col gap-6">
          {/* Story 6.6 (UJ-4): points at the first Agenda item — the very
              first row of the very first group below. */}
          <CoachingBalloon
            id={UJ4_TODAY_INTRO}
            dismissLabel="Dismiss the Today Agenda tip"
          >
            <span className="font-medium">This is your day.</span> Click any
            task to open the source file.
          </CoachingBalloon>

          {groupByFile(items).map((group) => (
            <div key={group.filePath}>
              <h2 className="text-sm font-medium text-[var(--org-fg-muted)]">
                {group.filePath}
              </h2>
              <ul className="mt-2 flex flex-col gap-1">
                {group.items.map((item) => (
                  <AgendaRow key={item.headlineId} item={item} todayIso={todayIso} />
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

/**
 * The Deadline badge text for `item` relative to `todayIso`: "Overdue" when the
 * backend flagged it (Deadline strictly before today), "Due today" when the
 * Deadline is exactly today, else a plain "Due" — a Scheduled-today row may also
 * carry a FUTURE Deadline (it matched via the Scheduled leg), which must not be
 * mislabelled as due today.
 */
export function deadlineLabel(item: AgendaItemDto, todayIso: string): string {
  if (item.overdue) return "Overdue";
  if (item.deadlineDate === todayIso) return "Due today";
  return "Due";
}

/**
 * One clickable Agenda row. Navigates to the TanStack Router
 * `/editor/$filePath/$headlineId` route (the route param is a stable
 * identity even though this v0.1 surface does not yet re-fetch by id); the
 * `byteStart` search param is the item's `byte_start` so the editor can place
 * the cursor at the Headline itself rather than just opening the file (see
 * `Editor`'s `initialByteOffset` prop).
 */
function AgendaRow({ item, todayIso }: { item: AgendaItemDto; todayIso: string }) {
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
