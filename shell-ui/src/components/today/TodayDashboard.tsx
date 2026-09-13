// Implements FR-6 (Today Dashboard — Story 7.1).
//
// The `/today` route's primary surface, upgrading Story 6.3's Agenda list into
// the full five-section dashboard: Scheduled | Deadline | Today-Tag | Inbox
// Preview | Active Clock, each collapsible via a chevron toggle (default
// expanded). Queries `commands.todayDashboard` once per mount — the backend
// (`orgsidian-index::query::dashboard::today`) already filters, orders each
// section `(file_path, position)`, and limits the Inbox preview, so this
// component's job is render only: never a second fetch, never a client-side
// re-sort.
//
// Out of scope here (see the Story 7.1 spec's Never list): section
// collapse/expand PERSISTENCE (Story 7.2); copy-blessed empty-state coaching
// via the coaching registry (Story 7.3) — the blank bodies below are the
// minimal v0.1 stand-in; clock in/out/write + LOGBOOK persistence (Story 7.6)
// — the Active Clock section only READS the running clock for display.

import { useEffect, useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ChevronDown, ChevronRight } from "lucide-react";

import {
  commands,
  type ActiveClockDto,
  type AgendaItemDto,
  type InboxItemDto,
  type TodayDashboardDto,
} from "@/lib/tauri";
import { localTodayIso } from "@/components/editor/schedule";
import { deadlineLabel, errorMessage } from "@/components/agenda/AgendaToday";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

/**
 * The `/today` route's Today Dashboard. Renders a loading placeholder, an error
 * (query failed — most commonly "no active Vault"), or the five collapsible
 * sections.
 */
export function TodayDashboard() {
  const [data, setData] = useState<TodayDashboardDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Computed ONCE so the fetch anchor and the deadline badges can never
  // disagree across a midnight rollover mid-render.
  const todayIso = useMemo(() => localTodayIso(), []);

  useEffect(() => {
    let disposed = false;
    setError(null);
    setData(null);

    commands
      .todayDashboard(todayIso)
      .then((result) => {
        if (!disposed) setData(result);
      })
      .catch((err: unknown) => {
        if (!disposed) setError(errorMessage(err));
      });

    return () => {
      disposed = true;
    };
    // `todayIso` is memoized with a stable `[]`, so this still runs once per
    // mount; listed to satisfy exhaustive-deps.
  }, [todayIso]);

  return (
    <section aria-labelledby="today-dashboard-heading">
      <h1
        id="today-dashboard-heading"
        className="text-2xl font-semibold text-[var(--org-fg-default)]"
      >
        Today
      </h1>

      {error !== null && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {error}
        </p>
      )}

      {error === null && data === null && (
        <p className="mt-3 text-sm text-[var(--org-fg-muted)]">Loading…</p>
      )}

      {error === null && data !== null && (
        <div className="mt-6 flex flex-col gap-7">
          <DashboardSection title="Scheduled" count={data.scheduled.length}>
            <AgendaList
              items={data.scheduled}
              todayIso={todayIso}
              emptyLabel="Nothing scheduled for today."
            />
          </DashboardSection>

          <DashboardSection title="Deadline" count={data.deadlines.length}>
            <AgendaList
              items={data.deadlines}
              todayIso={todayIso}
              emptyLabel="No deadlines due or overdue."
            />
          </DashboardSection>

          <DashboardSection title="Today-Tag" count={data.todayTag.length}>
            <AgendaList
              items={data.todayTag}
              todayIso={todayIso}
              emptyLabel="No items tagged for today."
            />
          </DashboardSection>

          <DashboardSection title="Inbox Preview" count={data.inbox.length}>
            <InboxList items={data.inbox} />
          </DashboardSection>

          <DashboardSection
            title="Active Clock"
            count={data.activeClock !== null ? 1 : 0}
          >
            <ActiveClockView clock={data.activeClock} />
          </DashboardSection>
        </div>
      )}
    </section>
  );
}

/**
 * One collapsible dashboard section: a chevron-toggle header (default expanded)
 * over its body. Local `open` state only — collapse/expand PERSISTENCE is Story
 * 7.2 (see the module header).
 */
function DashboardSection({
  title,
  count,
  children,
}: {
  title: string;
  count: number;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);

  return (
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="border-b border-[var(--org-border-default)] pb-4"
    >
      <CollapsibleTrigger className="flex w-full items-center gap-2 rounded px-1 py-1 text-left hover:bg-[var(--org-bg-surface)]">
        {open ? (
          <ChevronDown
            className="size-4 shrink-0 text-[var(--org-fg-muted)]"
            aria-hidden="true"
          />
        ) : (
          <ChevronRight
            className="size-4 shrink-0 text-[var(--org-fg-muted)]"
            aria-hidden="true"
          />
        )}
        <span className="text-sm font-semibold text-[var(--org-fg-default)]">
          {title}
        </span>
        <span
          className="text-xs text-[var(--org-fg-subtle)]"
          aria-label={`${count} items`}
        >
          {count}
        </span>
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-2 pl-6">{children}</CollapsibleContent>
    </Collapsible>
  );
}

/**
 * A section's Agenda rows (Scheduled / Deadline / Today-Tag), or a minimal
 * empty-state line. Rows reuse the same click-to-open `Link` + `deadlineLabel`
 * as `AgendaToday`; the list is rendered in the backend's order, never
 * re-sorted here.
 */
function AgendaList({
  items,
  todayIso,
  emptyLabel,
}: {
  items: AgendaItemDto[];
  todayIso: string;
  emptyLabel: string;
}) {
  if (items.length === 0) {
    return <p className="text-sm text-[var(--org-fg-muted)]">{emptyLabel}</p>;
  }
  return (
    <ul className="flex flex-col gap-1">
      {items.map((item) => (
        <AgendaRow key={item.headlineId} item={item} todayIso={todayIso} />
      ))}
    </ul>
  );
}

/**
 * One clickable Agenda row (mirrors `AgendaToday`'s row): a `Link` to
 * `/editor/$filePath/$headlineId` with the `byteStart` search param so the
 * editor places the cursor at the Headline itself.
 */
function AgendaRow({
  item,
  todayIso,
}: {
  item: AgendaItemDto;
  todayIso: string;
}) {
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

/**
 * The Inbox-preview rows (the first N `inbox.org` headlines), or a minimal
 * empty-state line. Same click-to-open `Link` as the Agenda rows.
 */
function InboxList({ items }: { items: InboxItemDto[] }) {
  if (items.length === 0) {
    return <p className="text-sm text-[var(--org-fg-muted)]">Inbox is empty.</p>;
  }
  return (
    <ul className="flex flex-col gap-1">
      {items.map((item) => (
        <li key={item.headlineId}>
          <Link
            to="/editor/$filePath/$headlineId"
            params={{
              filePath: item.filePath,
              headlineId: String(item.headlineId),
            }}
            search={{ byteStart: item.byteStart }}
            className="flex items-baseline gap-2 rounded px-2 py-1 text-sm text-[var(--org-fg-default)] hover:bg-[var(--org-bg-surface)]"
          >
            {item.todoKeyword !== null && (
              <span className="font-mono text-xs text-[var(--org-fg-subtle)]">
                {item.todoKeyword}
              </span>
            )}
            <span>{item.title}</span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

/**
 * The Active Clock section body: the one running clock as a click-to-open row,
 * or a minimal empty-state line. READ-ONLY (Story 7.1) — clock in/out is Story
 * 7.6.
 */
function ActiveClockView({ clock }: { clock: ActiveClockDto | null }) {
  if (clock === null) {
    return <p className="text-sm text-[var(--org-fg-muted)]">No active clock.</p>;
  }
  return (
    <Link
      to="/editor/$filePath/$headlineId"
      params={{ filePath: clock.filePath, headlineId: String(clock.headlineId) }}
      search={{ byteStart: clock.byteStart }}
      className="flex items-baseline gap-2 rounded px-2 py-1 text-sm text-[var(--org-fg-default)] hover:bg-[var(--org-bg-surface)]"
    >
      <span>{clock.title}</span>
      <span className="text-xs text-[var(--org-fg-subtle)]">
        since {clock.startAt}
      </span>
    </Link>
  );
}
