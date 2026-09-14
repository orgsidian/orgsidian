import { useCallback, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";

import {
  AgendaCustom,
  presetToRecall,
  type AgendaCustomSearch,
  type AgendaPresetApply,
  type AppliedAgendaFilters,
} from "@/components/agenda/AgendaCustom";
import { AgendaPresetSidebar } from "@/components/agenda/AgendaPresetSidebar";
import { localTodayIso } from "@/components/editor/schedule";
import type { AgendaPresetDto } from "@/lib/tauri";

/** A real `YYYY-MM-DD` calendar day, or `undefined` for anything malformed. */
export function isoDateOrUndefined(value: unknown): string | undefined {
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

  // Story 7.5: the live filter snapshot (for "Save preset") and the one-shot
  // preset-application signal (for "Recall preset"). `setApplied` is a stable
  // dispatch, so passing it straight as `onAppliedChange` never re-fires the
  // component's report effect.
  const [applied, setApplied] = useState<AppliedAgendaFilters | null>(null);
  const [presetApply, setPresetApply] = useState<AgendaPresetApply | null>(null);

  const applyPreset = useCallback(
    (preset: AgendaPresetDto) => {
      // Map the preset to its concrete recall payload (resolved window +
      // tag/todo → URL; completed + file-path → local filters).
      const recall = presetToRecall(preset, localTodayIso());
      void navigate({ search: recall.search });
      // `setPresetApply` builds a fresh object each apply, so its identity
      // already changes every time and the consuming effect re-syncs even when
      // the same preset is re-applied; `nonce` is a human-readable trace token,
      // not what drives the re-sync.
      setPresetApply({
        nonce: Date.now(),
        completed: recall.completed,
        filePathGlob: recall.filePathGlob,
      });
    },
    [navigate],
  );

  return (
    <main className="container mx-auto p-8">
      <div className="flex gap-6">
        <AgendaPresetSidebar current={applied} onApply={applyPreset} />
        <div className="min-w-0 flex-1">
          <AgendaCustom
            search={search}
            onSearchChange={(next) => {
              void navigate({ search: next });
            }}
            onAppliedChange={setApplied}
            presetApply={presetApply}
          />
        </div>
      </div>
    </main>
  );
}
