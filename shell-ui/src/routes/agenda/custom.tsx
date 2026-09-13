import { createFileRoute } from "@tanstack/react-router";

import {
  AgendaCustom,
  type AgendaCustomSearch,
} from "@/components/agenda/AgendaCustom";

/** A real `YYYY-MM-DD` calendar day, or `undefined` for anything malformed. */
function isoDateOrUndefined(value: unknown): string | undefined {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return undefined;
  }
  // Reject shape-valid but impossible calendar dates from a hand-edited URL
  // (e.g. `2026-13-45`), which `new Date` would otherwise silently roll over
  // into a different day and forward to the query.
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(year, month - 1, day);
  return date.getFullYear() === year &&
    date.getMonth() === month - 1 &&
    date.getDate() === day
    ? value
    : undefined;
}

/** A non-empty trimmed string, or `undefined`. */
function nonEmptyStringOrUndefined(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed === "" ? undefined : trimmed;
}

export const Route = createFileRoute("/agenda/custom")({
  // Story 7.4 (LD-29): the typed search params that drive the Custom Agenda —
  // `?start=`, `?end=`, `?tag=`, `?todo=`. Malformed/absent values fall
  // through to `undefined` (the component supplies range defaults) rather than
  // erroring the route, so a hand-edited or stale URL still opens the view.
  validateSearch: (search: Record<string, unknown>): AgendaCustomSearch => ({
    start: isoDateOrUndefined(search.start),
    end: isoDateOrUndefined(search.end),
    tag: nonEmptyStringOrUndefined(search.tag),
    todo: nonEmptyStringOrUndefined(search.todo),
  }),
  component: AgendaCustomRoute,
});

/**
 * Implements FR-7 (Story 7.4): the `/agenda/custom` route renders the Custom
 * Agenda date-range + filter view. The route owns the typed URL search params
 * (LD-29) and hands them, plus a typed navigate, to the component.
 */
function AgendaCustomRoute() {
  const search = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <main className="container mx-auto p-8">
      <AgendaCustom
        search={search}
        onSearchChange={(next) => {
          void navigate({ search: next });
        }}
      />
    </main>
  );
}
