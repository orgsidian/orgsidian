// @vitest-environment jsdom
import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 7.5 (FR-7): route-level wiring test for `/agenda/custom`. Unlike
 * `custom.test.tsx` (which stubs both children as `() => null` to unit-test the
 * `validateSearch` guard), this renders the REAL `AgendaPresetSidebar` +
 * `AgendaCustom` through the real `AgendaCustomRoute`, so the `applyPreset`
 * (resolve → navigate → setPresetApply) path and the sidebar⇄list prop wiring
 * are actually exercised: clicking a preset navigates with the resolved search,
 * and "Save current" captures the applied snapshot.
 */

const navigate = vi.fn();

// A fixed route search so the "Save current" snapshot is a meaningful,
// non-default window.
const routeSearch = { start: "2026-09-01", end: "2026-09-30", tag: "work", todo: undefined };

vi.mock("@tanstack/react-router", () => ({
  createFileRoute: () => (opts: Record<string, unknown>) => ({
    ...opts,
    useSearch: () => routeSearch,
    useNavigate: () => navigate,
  }),
  Link: ({ children }: { children?: unknown }) => children as never,
}));

const commandsMock = vi.hoisted(() => ({
  listAgendaPresets: vi.fn(),
  saveAgendaPreset: vi.fn(),
  deleteAgendaPreset: vi.fn(),
  agendaCustom: vi.fn(),
}));
vi.mock("@/lib/tauri", () => ({ commands: commandsMock }));

// Deterministic "today" for the no-arg call (drives rolling-window recall),
// while still honoring an explicit `Date` argument so `addDaysIso` (which calls
// `localTodayIso(shifted)`) computes real dates — a constant-return mock would
// wrongly collapse every rolling window onto today.
vi.mock("@/components/editor/schedule", () => ({
  localTodayIso: (now?: Date) => {
    if (now === undefined) return "2026-09-14";
    const y = now.getFullYear().toString().padStart(4, "0");
    const m = (now.getMonth() + 1).toString().padStart(2, "0");
    const d = now.getDate().toString().padStart(2, "0");
    return `${y}-${m}-${d}`;
  },
}));

// Both real components import these from AgendaToday; stub to avoid pulling its
// render chain (the list is never populated here, so deadlineLabel is unused).
vi.mock("@/components/agenda/AgendaToday", () => ({
  errorMessage: (err: unknown) =>
    err && typeof err === "object" && "reason" in err
      ? String((err as { reason: unknown }).reason)
      : String(err),
  deadlineLabel: () => "",
}));

import { Route } from "./custom";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const RouteComponent = (Route as unknown as { component: () => ReactElement }).component;

type AgendaPresetDto = {
  name: string;
  view: string;
  start: string | null;
  end: string | null;
  rollingDays: number | null;
  tag: string | null;
  todoState: string | null;
  filePathGlob: string | null;
  completed: boolean;
};

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  navigate.mockReset();
  commandsMock.listAgendaPresets.mockReset();
  commandsMock.saveAgendaPreset.mockReset();
  commandsMock.deleteAgendaPreset.mockReset();
  commandsMock.agendaCustom.mockReset();
  // The list is always empty so AgendaCustom never mounts its virtualized rows.
  commandsMock.agendaCustom.mockResolvedValue([]);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

async function renderRoute() {
  await act(async () => {
    root.render(<RouteComponent />);
    await Promise.resolve();
  });
  // Flush the async list/query + the applied-snapshot report effect.
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe("AgendaCustomRoute wiring (Story 7.5, FR-7)", () => {
  it("clicking a preset navigates with the resolved search", async () => {
    const rolling: AgendaPresetDto = {
      name: "Rolling week",
      view: "custom",
      start: null,
      end: null,
      rollingDays: 7,
      tag: "home",
      todoState: "DONE",
      filePathGlob: "notes/*",
      completed: true,
    };
    commandsMock.listAgendaPresets.mockResolvedValue([rolling]);
    await renderRoute();

    const applyButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Rolling week",
    );
    expect(applyButton).toBeDefined();
    act(() => applyButton!.click());

    // rollingDays=7 → [today-6, today] = [2026-09-08, 2026-09-14]; tag/todo pass
    // through; the two local-only filters (completed/filePathGlob) do NOT go to
    // the URL.
    expect(navigate).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith({
      search: {
        start: "2026-09-08",
        end: "2026-09-14",
        tag: "home",
        todo: "DONE",
      },
    });
  });

  it("'Save current' captures the applied filter snapshot", async () => {
    commandsMock.listAgendaPresets.mockResolvedValue([]);
    commandsMock.saveAgendaPreset.mockResolvedValue(null);
    await renderRoute();

    // The Save control is enabled only once AgendaCustom has reported its
    // applied snapshot up through the route — proving the wiring is live.
    const nameInput = container.querySelector<HTMLInputElement>("#agenda-preset-name");
    expect(nameInput).not.toBeNull();
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(nameInput, "Work month");
    nameInput!.dispatchEvent(new Event("input", { bubbles: true }));

    const saveButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Save preset",
    );
    expect((saveButton as HTMLButtonElement).disabled).toBe(false);
    await act(async () => {
      saveButton!.click();
      await Promise.resolve();
    });

    expect(commandsMock.saveAgendaPreset).toHaveBeenCalledTimes(1);
    // The snapshot reflects the route search (start/end/tag) with empty
    // local-only filters normalized to null.
    expect(commandsMock.saveAgendaPreset).toHaveBeenCalledWith({
      name: "Work month",
      view: "custom",
      start: "2026-09-01",
      end: "2026-09-30",
      rollingDays: null,
      tag: "work",
      todoState: null,
      filePathGlob: null,
      completed: false,
    });
  });
});
