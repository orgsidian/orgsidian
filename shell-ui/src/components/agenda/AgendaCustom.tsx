// Implements FR-7 (Custom Agenda — Story 7.4).
//
// The `/agenda/custom` route's primary surface: the general date-range +
// filter Agenda that Today (Story 6.3) and Week (Story 6.4) are fixed-window
// special cases of. A filter bar (start / end date pickers + tag / TODO-state
// / file-path inputs) drives `commands.agendaCustom`; the flat, backend-sorted
// result is grouped by calendar date and rendered through
// `@tanstack/react-virtual` (LD-30) so the list scales to 1k+ items without
// mounting every row.
//
// Per LD-29, the start / end / tag / TODO filters live in typed route search
// params (`?start=`, `?end=`, `?tag=`, `?todo=`) so a range is deep-linkable;
// the route wrapper (`routes/agenda/custom.tsx`) owns the URL and passes the
// resolved values in, so this component is a pure render + fetch surface
// (mirrors how `AgendaWeek` never re-sorts or double-fetches). The file-path
// filter is intentionally NOT a search param — the Story 7.4 AC enumerates
// only the four above — so it is held in local component state.

import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";

import { commands, type AgendaItemDto } from "@/lib/tauri";
import { localTodayIso } from "@/components/editor/schedule";
import { deadlineLabel, errorMessage } from "@/components/agenda/AgendaToday";

/**
 * The default range END offset (in days) from the start when the route carries
 * no `?end=`: a 29-day offset spans 30 inclusive calendar days (today plus the
 * next 29), i.e. the "30-day window" the module docs describe.
 */
const DEFAULT_RANGE_DAYS = 29;

/** The URL-driven filter state — the four typed search params (LD-29). */
export interface AgendaCustomSearch {
  start?: string;
  end?: string;
  tag?: string;
  todo?: string;
}

/**
 * The full set of applied filters (Story 7.5) — the four URL-driven ones plus
 * the two component-local ones (`filePathGlob`, `completed`). The preset
 * sidebar reads this snapshot to "save the current filters" as a preset.
 */
export interface AppliedAgendaFilters {
  start: string;
  end: string;
  tag: string;
  todo: string;
  filePathGlob: string;
  completed: boolean;
}

/**
 * A one-shot preset-application signal (Story 7.5) carrying the two local-only
 * filters that do not live in the URL (`completed`, `filePathGlob`). The route
 * builds a fresh object on every apply, so its identity already changes each
 * time and the consuming effect re-syncs even when the same preset is
 * re-applied; `nonce` is just a human-readable trace token, not the mechanism.
 */
export interface AgendaPresetApply {
  nonce: number;
  completed: boolean;
  filePathGlob: string;
}

export interface AgendaCustomProps {
  /** The current route search params (typed). */
  search: AgendaCustomSearch;
  /**
   * Commit new start/end/tag/todo values to the route (the URL is the source
   * of truth for these four — see the module docs). The route wrapper maps
   * this onto a typed `navigate({ search })`.
   */
  onSearchChange: (next: AgendaCustomSearch) => void;
  /**
   * Story 7.5 (optional): report the currently-applied filters upward whenever
   * they change, so the route's preset sidebar can snapshot them on "Save
   * preset". Omitted in the Story 7.4 tests → identical behavior.
   */
  onAppliedChange?: (applied: AppliedAgendaFilters) => void;
  /**
   * Story 7.5 (optional): a preset the route wants applied. The URL-driven
   * filters (start/end/tag/todo) are applied by the route via `navigate`; this
   * carries only the two local-only filters. Omitted in the Story 7.4 tests.
   */
  presetApply?: AgendaPresetApply | null;
}

/** One calendar day's Agenda items, in the document order the backend sorted. */
interface AgendaDay {
  dateIso: string;
  items: AgendaItemDto[];
}

/**
 * A flattened virtualization row: either a date group header or one Agenda
 * item. The virtualizer indexes this single flat list (headers interleaved
 * with their items) rather than nesting a virtualizer per group.
 */
type FlatRow =
  | { kind: "header"; dateIso: string; count: number }
  | { kind: "item"; item: AgendaItemDto };

/**
 * `dateIso` (`YYYY-MM-DD`) plus `days` calendar days, computed entirely in
 * local time (never through a UTC-parsed `Date`, which would drift near a DST
 * boundary) — mirrors `AgendaWeek`'s own `addDaysIso`.
 */
export function addDaysIso(dateIso: string, days: number): string {
  const [year, month, day] = dateIso.split("-").map(Number);
  const shifted = new Date(year, month - 1, day);
  shifted.setDate(shifted.getDate() + days);
  return localTodayIso(shifted);
}

/**
 * Resolve a preset's stored window to a concrete `[start, end]` at recall time
 * (Story 7.5). A rolling preset (`rollingDays = N`) becomes the last N days
 * ending `today` (`[today-(N-1), today]`, so N=7 spans today plus the previous
 * six); an absolute preset restores its stored `start`/`end` (`undefined` when
 * unset, so the view falls back to its own default window).
 */
export function resolvePresetWindow(
  preset: { rollingDays: number | null; start: string | null; end: string | null },
  today: string,
): { start?: string; end?: string } {
  if (preset.rollingDays != null) {
    return { start: addDaysIso(today, -(preset.rollingDays - 1)), end: today };
  }
  return { start: preset.start ?? undefined, end: preset.end ?? undefined };
}

/** The concrete recall payload a preset resolves to (Story 7.5). */
export interface PresetRecall {
  /** URL-owned filters: resolved window + tag/todo. */
  search: AgendaCustomSearch;
  /** Local-only completion-mode filter. */
  completed: boolean;
  /** Local-only file-path glob (empty string = no filter). */
  filePathGlob: string;
}

/**
 * Map a saved preset's filter fields to the concrete recall payload the
 * `/agenda/custom` route applies: the URL search params (resolved window +
 * tag/todo) and the two component-local filters (completion mode + file-path
 * glob). Pure and exported so this field-by-field mapping — the exact place a
 * swapped `tag`/`todo` or dropped `filePathGlob` would silently recall the
 * wrong filters — is unit-testable in isolation.
 */
export function presetToRecall(
  preset: {
    rollingDays: number | null;
    start: string | null;
    end: string | null;
    tag: string | null;
    todoState: string | null;
    filePathGlob: string | null;
    completed: boolean;
  },
  today: string,
): PresetRecall {
  const { start, end } = resolvePresetWindow(preset, today);
  return {
    search: {
      start,
      end,
      tag: preset.tag ?? undefined,
      todo: preset.todoState ?? undefined,
    },
    completed: preset.completed,
    filePathGlob: preset.filePathGlob ?? "",
  };
}

/** A short, human display label for a `YYYY-MM-DD` date, e.g. "Sat, Sep 5, 2026". */
function formatDayLabel(dateIso: string): string {
  const [year, month, day] = dateIso.split("-").map(Number);
  const date = new Date(year, month - 1, day);
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

/**
 * Partition an already `(agendaDate, file_path, position)`-sorted list into
 * per-date groups, preserving that order (the AC's "grouped by date" — a
 * stable partition, never a re-sort here, mirroring `AgendaWeek`). Only dates
 * that actually carry items get a group: a custom range can span months, so
 * (unlike Week's fixed 7 days) empty days are omitted rather than rendered as
 * empty headers.
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

/** Flatten the date groups into the interleaved header/item list to virtualize. */
function flattenRows(groups: AgendaDay[]): FlatRow[] {
  const rows: FlatRow[] = [];
  for (const group of groups) {
    rows.push({ kind: "header", dateIso: group.dateIso, count: group.items.length });
    for (const item of group.items) {
      rows.push({ kind: "item", item });
    }
  }
  return rows;
}

/**
 * The `/agenda/custom` route's Agenda list. Renders the filter bar plus one
 * of: a loading placeholder, an error (query failed — most commonly "no
 * active Vault"), an empty-state line, or the virtualized date-grouped list.
 */
export function AgendaCustom({
  search,
  onSearchChange,
  onAppliedChange,
  presetApply,
}: AgendaCustomProps) {
  // Resolve the effective range: default to a 30-day window starting today
  // when the route carries no explicit start/end (a first visit to
  // `/agenda/custom` with no params).
  const resolvedStart = search.start ?? localTodayIso();
  const resolvedEnd = search.end ?? addDaysIso(resolvedStart, DEFAULT_RANGE_DAYS);
  const tag = search.tag ?? "";
  const todo = search.todo ?? "";
  // The ACTUAL current calendar day the deadline badges are read against —
  // never the range start (a custom range may not begin today), so a Deadline
  // due today reads "Due today" regardless of where the window starts.
  const todayIso = localTodayIso();

  // Draft inputs the user edits before pressing Apply. Re-synced whenever the
  // URL-driven values change (a deep-link or a browser back/forward), which
  // never races user typing since those only change on an explicit Apply.
  const [startDraft, setStartDraft] = useState(resolvedStart);
  const [endDraft, setEndDraft] = useState(resolvedEnd);
  const [tagDraft, setTagDraft] = useState(tag);
  const [todoDraft, setTodoDraft] = useState(todo);
  const [filePathDraft, setFilePathDraft] = useState("");
  // The file-path filter actually applied to the query (local-only, not a
  // search param — see the module docs).
  const [appliedFilePath, setAppliedFilePath] = useState("");
  // Story 7.5 completion mode: filter the window on the CLOSED date and include
  // DONE headlines (the "Done This …" preset semantics). Local-only, like the
  // file-path filter — the Story 7.4 URL contract stays the four typed params.
  const [completedDraft, setCompletedDraft] = useState(false);
  const [completed, setCompleted] = useState(false);

  useEffect(() => {
    setStartDraft(resolvedStart);
    setEndDraft(resolvedEnd);
    setTagDraft(tag);
    setTodoDraft(todo);
  }, [resolvedStart, resolvedEnd, tag, todo]);

  const [items, setItems] = useState<AgendaItemDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    setError(null);
    setItems(null);

    commands
      .agendaCustom({
        startDate: resolvedStart,
        endDate: resolvedEnd,
        tag: tag === "" ? null : tag,
        todoState: todo === "" ? null : todo,
        filePathGlob: appliedFilePath === "" ? null : appliedFilePath,
        completedInRange: completed,
      })
      .then((result) => {
        if (!disposed) setItems(result);
      })
      .catch((err: unknown) => {
        if (!disposed) setError(errorMessage(err));
      });

    return () => {
      disposed = true;
    };
  }, [resolvedStart, resolvedEnd, tag, todo, appliedFilePath, completed]);

  // Story 7.5: apply a preset's two local-only filters. Keyed on the whole
  // `presetApply` object, which is route state that only changes when a preset
  // is actually applied (a fresh `nonce` each time), so this never fights the
  // user's own edits between applies.
  useEffect(() => {
    if (presetApply == null) return;
    setCompletedDraft(presetApply.completed);
    setCompleted(presetApply.completed);
    setFilePathDraft(presetApply.filePathGlob);
    setAppliedFilePath(presetApply.filePathGlob);
  }, [presetApply]);

  // Story 7.5: report the applied filter snapshot upward for the preset
  // sidebar's "Save current" action. `onAppliedChange` is memoized by the route
  // so this fires only when the applied filters actually change.
  useEffect(() => {
    onAppliedChange?.({
      start: resolvedStart,
      end: resolvedEnd,
      tag,
      todo,
      filePathGlob: appliedFilePath,
      completed,
    });
  }, [resolvedStart, resolvedEnd, tag, todo, appliedFilePath, completed, onAppliedChange]);

  const rows = useMemo(
    () => (items !== null ? flattenRows(groupByDate(items)) : []),
    [items],
  );

  function applyFilters(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSearchChange({
      start: startDraft,
      end: endDraft,
      tag: tagDraft.trim() === "" ? undefined : tagDraft.trim(),
      todo: todoDraft.trim() === "" ? undefined : todoDraft.trim(),
    });
    setAppliedFilePath(filePathDraft.trim());
    setCompleted(completedDraft);
  }

  return (
    <section aria-labelledby="agenda-custom-heading">
      <div className="flex items-baseline justify-between">
        <h1
          id="agenda-custom-heading"
          className="text-2xl font-semibold text-[var(--org-fg-default)]"
        >
          Custom Agenda
        </h1>
        <Link
          to="/today"
          className="text-sm text-[var(--org-fg-muted)] underline hover:text-[var(--org-fg-default)]"
        >
          Back to Today
        </Link>
      </div>

      <form
        onSubmit={applyFilters}
        aria-label="Agenda filters"
        className="mt-4 flex flex-wrap items-end gap-3"
      >
        <label className="flex flex-col gap-1 text-sm text-[var(--org-fg-muted)]">
          Start
          <input
            type="date"
            value={startDraft}
            max={endDraft}
            onChange={(event) => setStartDraft(event.target.value)}
            className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-[var(--org-fg-default)]"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm text-[var(--org-fg-muted)]">
          End
          <input
            type="date"
            value={endDraft}
            min={startDraft}
            onChange={(event) => setEndDraft(event.target.value)}
            className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-[var(--org-fg-default)]"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm text-[var(--org-fg-muted)]">
          Tag
          <input
            type="text"
            value={tagDraft}
            placeholder="e.g. home"
            onChange={(event) => setTagDraft(event.target.value)}
            className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-[var(--org-fg-default)]"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm text-[var(--org-fg-muted)]">
          TODO state
          <input
            type="text"
            value={todoDraft}
            placeholder="e.g. NEXT"
            onChange={(event) => setTodoDraft(event.target.value)}
            className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-[var(--org-fg-default)]"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm text-[var(--org-fg-muted)]">
          File path
          <input
            type="text"
            value={filePathDraft}
            placeholder="e.g. projects/*"
            onChange={(event) => setFilePathDraft(event.target.value)}
            className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-[var(--org-fg-default)]"
          />
        </label>
        <label className="flex items-center gap-2 text-sm text-[var(--org-fg-muted)]">
          <input
            type="checkbox"
            checked={completedDraft}
            onChange={(event) => setCompletedDraft(event.target.checked)}
            className="h-4 w-4 rounded border-[var(--org-border-default)]"
          />
          Completed (by close date)
        </label>
        <button
          type="submit"
          className="rounded bg-[var(--org-border-focus)] px-3 py-1.5 text-sm font-medium text-[var(--org-bg-canvas)] hover:opacity-90"
        >
          Apply
        </button>
      </form>

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
          {completed
            ? "Nothing completed in this range."
            : "Nothing scheduled or due in this range."}
        </p>
      )}

      {error === null && items !== null && items.length > 0 && (
        <AgendaCustomList rows={rows} todayIso={todayIso} />
      )}
    </section>
  );
}

/**
 * The virtualized (LD-30) date-grouped list. A single `useVirtualizer` indexes
 * the flat header/item row list; only the rows in (and near) the viewport are
 * mounted, so a 1k+ item range never mounts every row. Row heights vary
 * (headers vs. items, wrapping titles), so each row reports its measured
 * height back via `measureElement`.
 */
function AgendaCustomList({ rows, todayIso }: { rows: FlatRow[]; todayIso: string }) {
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => (rows[index].kind === "header" ? 30 : 34),
    overscan: 12,
  });

  return (
    <div
      ref={scrollRef}
      data-testid="agenda-custom-scroll"
      className="mt-4 overflow-auto rounded border border-[var(--org-border-default)]"
      style={{ height: "70vh" }}
    >
      <ul
        role="list"
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
          margin: 0,
          padding: 0,
          listStyle: "none",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];
          return (
            <li
              key={virtualRow.key}
              data-index={virtualRow.index}
              ref={virtualizer.measureElement}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              {row.kind === "header" ? (
                <h2 className="bg-[var(--org-bg-surface)] px-3 py-1 text-sm font-medium text-[var(--org-fg-muted)]">
                  {formatDayLabel(row.dateIso)}
                  <span className="ml-2 text-xs text-[var(--org-fg-subtle)]">
                    ({row.count})
                  </span>
                </h2>
              ) : (
                <AgendaCustomRow item={row.item} todayIso={todayIso} />
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/**
 * One clickable Agenda row for the Custom view. Same click-to-open
 * `/editor/$filePath/$headlineId` target `AgendaWeek`'s row uses, with the
 * source file shown inline (the grouping key here is the date, not the file).
 */
function AgendaCustomRow({ item, todayIso }: { item: AgendaItemDto; todayIso: string }) {
  return (
    <Link
      to="/editor/$filePath/$headlineId"
      params={{ filePath: item.filePath, headlineId: String(item.headlineId) }}
      search={{ byteStart: item.byteStart }}
      className="flex items-baseline gap-2 px-3 py-1 text-sm text-[var(--org-fg-default)] hover:bg-[var(--org-bg-surface)]"
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
  );
}
