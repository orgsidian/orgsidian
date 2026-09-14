// Implements FR-7 (saved Agenda filter presets — Story 7.5).
//
// The Agenda sidebar: lists the saved named filter presets for the active
// Vault, applies one on click (restoring its view + filters), saves the
// current filters as a new named preset, and deletes a preset via a context
// menu. Presets live in the per-Vault TOML settings store (`[agenda_presets]`,
// LD-40); this component talks to them exclusively through the typed
// `commands.{listAgendaPresets,saveAgendaPreset,deleteAgendaPreset}` bridge
// (never raw `invoke`). The two default presets (`Done This Week` / `Done This
// Month`) are seeded server-side on the first `listAgendaPresets` call, so a
// fresh Vault shows them without any client seeding.
//
// The sidebar is rendered by the `/agenda/custom` route beside `AgendaCustom`
// (not inside it) so that applying a preset can drive both the URL-owned
// filters (start/end/tag/todo, via the route's `navigate`) and the two
// component-local ones (file-path, completed-mode) in one gesture.

import { useCallback, useEffect, useState } from "react";

import { commands, type AgendaPresetDto } from "@/lib/tauri";
import { errorMessage } from "@/components/agenda/AgendaToday";
import type { AppliedAgendaFilters } from "@/components/agenda/AgendaCustom";

export interface AgendaPresetSidebarProps {
  /**
   * The currently-applied filters, snapshotted by `AgendaCustom` — the payload
   * "Save preset" persists. `null` until the first snapshot arrives (the Save
   * control is disabled until then).
   */
  current: AppliedAgendaFilters | null;
  /** Recall a preset: the route restores its view + filters. */
  onApply: (preset: AgendaPresetDto) => void;
}

/** Build the persisted DTO from the live filter snapshot (an absolute window). */
function presetFromCurrent(name: string, current: AppliedAgendaFilters): AgendaPresetDto {
  const orNull = (value: string) => (value.trim() === "" ? null : value.trim());
  return {
    name,
    view: "custom",
    // Saving from the live view pins the absolute window it currently shows;
    // rolling windows are a property of the shipped defaults only.
    start: orNull(current.start),
    end: orNull(current.end),
    rollingDays: null,
    tag: orNull(current.tag),
    todoState: orNull(current.todo),
    filePathGlob: orNull(current.filePathGlob),
    completed: current.completed,
  };
}

export function AgendaPresetSidebar({ current, onApply }: AgendaPresetSidebarProps) {
  const [presets, setPresets] = useState<AgendaPresetDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [nameDraft, setNameDraft] = useState("");
  // The preset whose context menu is currently open (by name), or null.
  const [menuFor, setMenuFor] = useState<string | null>(null);

  const refresh = useCallback(() => {
    setError(null);
    commands
      .listAgendaPresets()
      .then((result) => setPresets(result))
      .catch((err: unknown) => {
        setPresets([]);
        setError(errorMessage(err));
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  function saveCurrent() {
    const name = nameDraft.trim();
    if (name === "" || current === null) return;
    commands
      .saveAgendaPreset(presetFromCurrent(name, current))
      .then(() => {
        setNameDraft("");
        refresh();
      })
      .catch((err: unknown) => setError(errorMessage(err)));
  }

  function deletePreset(name: string) {
    setMenuFor(null);
    commands
      .deleteAgendaPreset(name)
      .then(() => refresh())
      .catch((err: unknown) => setError(errorMessage(err)));
  }

  return (
    <aside
      aria-label="Saved agenda presets"
      className="w-56 shrink-0 border-r border-[var(--org-border-default)] pr-4"
    >
      <h2 className="text-sm font-semibold text-[var(--org-fg-default)]">Presets</h2>

      {error !== null && (
        <p role="alert" className="mt-2 text-xs text-[var(--org-fg-muted)]">
          {error}
        </p>
      )}

      {presets !== null && presets.length === 0 && error === null && (
        <p className="mt-2 text-xs text-[var(--org-fg-muted)]">No saved presets yet.</p>
      )}

      <ul role="list" className="mt-2 flex flex-col gap-1">
        {(presets ?? []).map((preset) => (
          <li key={preset.name} className="relative">
            <div
              className="flex items-center gap-1"
              onContextMenu={(event) => {
                event.preventDefault();
                setMenuFor((open) => (open === preset.name ? null : preset.name));
              }}
            >
              <button
                type="button"
                onClick={() => onApply(preset)}
                className="flex-1 truncate rounded px-2 py-1 text-left text-sm text-[var(--org-fg-default)] hover:bg-[var(--org-bg-surface)]"
              >
                {preset.name}
              </button>
              <button
                type="button"
                aria-label={`Options for ${preset.name}`}
                aria-haspopup="menu"
                aria-expanded={menuFor === preset.name}
                onClick={() =>
                  setMenuFor((open) => (open === preset.name ? null : preset.name))
                }
                className="rounded px-1.5 py-1 text-sm text-[var(--org-fg-muted)] hover:bg-[var(--org-bg-surface)]"
              >
                ⋯
              </button>
            </div>
            {menuFor === preset.name && (
              <div
                role="menu"
                className="absolute right-0 z-10 mt-1 rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] shadow"
              >
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => deletePreset(preset.name)}
                  className="block w-full px-3 py-1.5 text-left text-sm text-destructive hover:bg-[var(--org-bg-canvas)]"
                >
                  Delete
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>

      <div className="mt-4 flex flex-col gap-1">
        <label className="text-xs text-[var(--org-fg-muted)]" htmlFor="agenda-preset-name">
          Save current filters as…
        </label>
        <input
          id="agenda-preset-name"
          type="text"
          value={nameDraft}
          placeholder="Preset name"
          onChange={(event) => setNameDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              saveCurrent();
            }
          }}
          className="rounded border border-[var(--org-border-default)] bg-[var(--org-bg-surface)] px-2 py-1 text-sm text-[var(--org-fg-default)]"
        />
        <button
          type="button"
          onClick={saveCurrent}
          disabled={nameDraft.trim() === "" || current === null}
          className="rounded bg-[var(--org-border-focus)] px-2 py-1 text-sm font-medium text-[var(--org-bg-canvas)] hover:opacity-90 disabled:opacity-50"
        >
          Save preset
        </button>
      </div>
    </aside>
  );
}
