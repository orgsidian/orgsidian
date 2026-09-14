// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterContextProvider,
} from "@tanstack/react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 7.4 (FR-7): `AgendaCustom` queries `commands.agendaCustom` with the
 * resolved date range + filters, groups the flat (already backend-sorted)
 * result by `agendaDate` into virtualized header/item rows without re-sorting,
 * drives start/end/tag/todo through typed route search params (committed on
 * Apply), keeps the file-path filter as local state, and renders a
 * click-to-open `Link` per item to `/editor/$filePath/$headlineId`.
 */

type AgendaItemDto = {
  headlineId: number;
  filePath: string;
  title: string;
  byteStart: number;
  todoKeyword: string | null;
  scheduledDate: string | null;
  scheduledTime: string | null;
  deadlineDate: string | null;
  deadlineTime: string | null;
  overdue: boolean;
  agendaDate: string;
};

const mocks = vi.hoisted(() => ({
  agendaCustom: vi.fn<(query: unknown) => Promise<AgendaItemDto[]>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: { agendaCustom: mocks.agendaCustom },
}));

// The real `useVirtualizer` measures the scroll element, which has zero height
// under jsdom — it would then render (almost) no rows. Replace it with a
// deterministic pass-through that mounts every row, so the test exercises this
// component's grouping/flattening/wiring rather than react-virtual internals.
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (opts: { count: number }) => ({
    getTotalSize: () => opts.count * 34,
    getVirtualItems: () =>
      Array.from({ length: opts.count }, (_, index) => ({
        index,
        key: index,
        start: index * 34,
        size: 34,
      })),
    measureElement: () => {},
  }),
}));

// Imported AFTER the mocks are registered.
import {
  AgendaCustom,
  presetToRecall,
  resolvePresetWindow,
  type AgendaCustomProps,
  type AgendaCustomSearch,
} from "./AgendaCustom";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.agendaCustom.mockReset();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function testRouter() {
  const rootRoute = createRootRoute();
  const editorRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/editor/$filePath/$headlineId",
  });
  const todayRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/today",
  });
  return createRouter({
    routeTree: rootRoute.addChildren([editorRoute, todayRoute]),
    history: createMemoryHistory({ initialEntries: ["/agenda/custom"] }),
  });
}

function renderCustom(
  search: AgendaCustomSearch = {},
  onSearchChange: (next: AgendaCustomSearch) => void = () => {},
) {
  const router = testRouter();
  act(() => {
    root.render(
      <RouterContextProvider router={router}>
        <AgendaCustom search={search} onSearchChange={onSearchChange} />
      </RouterContextProvider>,
    );
  });
}

/** Render with an arbitrary prop set — used by the Story 7.5 prop tests. */
function renderCustomWith(props: AgendaCustomProps) {
  const router = testRouter();
  act(() => {
    root.render(
      <RouterContextProvider router={router}>
        <AgendaCustom {...props} />
      </RouterContextProvider>,
    );
  });
}

function item(overrides: Partial<AgendaItemDto> & { agendaDate: string }): AgendaItemDto {
  return {
    headlineId: 1,
    filePath: "inbox.org",
    title: "Untitled",
    byteStart: 0,
    todoKeyword: null,
    scheduledDate: null,
    scheduledTime: null,
    deadlineDate: null,
    deadlineTime: null,
    overdue: false,
    ...overrides,
  };
}

/** The `YYYY-MM-DD` string `days` calendar days after `dateIso` (local time). */
function addDaysIso(dateIso: string, days: number): string {
  const [year, month, day] = dateIso.split("-").map(Number);
  const d = new Date(year, month - 1, day);
  d.setDate(d.getDate() + days);
  const y = d.getFullYear().toString().padStart(4, "0");
  const m = (d.getMonth() + 1).toString().padStart(2, "0");
  const dd = d.getDate().toString().padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

/** Set a React-controlled input's value and fire the input event React listens for. */
function setInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLInputElement.prototype,
    "value",
  )?.set;
  setter?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("AgendaCustom (Story 7.4, FR-7)", () => {
  it("shows a loading placeholder before the query resolves", () => {
    mocks.agendaCustom.mockReturnValue(new Promise(() => {}));
    renderCustom();

    expect(container.textContent).toContain("Loading…");
  });

  it("surfaces a query failure as an alert (e.g. no active Vault)", async () => {
    mocks.agendaCustom.mockRejectedValue({ reason: "no active vault; designate a vault first" });
    await act(async () => {
      renderCustom();
      await Promise.resolve();
    });

    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toBe("no active vault; designate a vault first");
  });

  it("renders an empty-state message when the range has no items", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom();
      await Promise.resolve();
    });

    expect(container.textContent).toContain("Nothing scheduled or due in this range.");
  });

  it("defaults to a today-anchored range with no filters when no search params are set", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom();
      await Promise.resolve();
    });

    expect(mocks.agendaCustom).toHaveBeenCalledTimes(1);
    const query = mocks.agendaCustom.mock.calls[0][0] as Record<string, string | null>;
    expect(query.startDate).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    // The default window is 30 inclusive days: end = start + 29 days.
    expect(query.endDate).toBe(addDaysIso(query.startDate as string, 29));
    expect(query.tag).toBeNull();
    expect(query.todoState).toBeNull();
    expect(query.filePathGlob).toBeNull();
  });

  it("passes the route's start/end/tag/todo search params into the query", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04", tag: "home", todo: "NEXT" });
      await Promise.resolve();
    });

    const query = mocks.agendaCustom.mock.calls[0][0] as Record<string, unknown>;
    expect(query).toMatchObject({
      startDate: "2026-09-05",
      endDate: "2026-10-04",
      tag: "home",
      todoState: "NEXT",
      filePathGlob: null,
    });
  });

  it("groups items by agendaDate into date headers, preserving backend order", async () => {
    mocks.agendaCustom.mockResolvedValue([
      item({ headlineId: 1, filePath: "a.org", title: "a first", agendaDate: "2026-09-05" }),
      item({ headlineId: 2, filePath: "b.org", title: "b first", agendaDate: "2026-09-20" }),
      item({ headlineId: 3, filePath: "b.org", title: "b second", agendaDate: "2026-09-20" }),
    ]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" });
      await Promise.resolve();
    });

    // Two distinct dates → two headers (with per-day counts).
    const headers = Array.from(container.querySelectorAll("h2"));
    expect(headers).toHaveLength(2);
    expect(headers[1].textContent).toContain("(2)");

    // Items keep the backend's order (no client re-sort).
    const links = Array.from(container.querySelectorAll('a[href^="/editor/"]')).map(
      (el) => el.textContent,
    );
    expect(links[0]).toContain("a first");
    expect(links[1]).toContain("b first");
    expect(links[2]).toContain("b second");
  });

  it("renders a click-to-open Link to /editor/$filePath/$headlineId with the byteStart search param", async () => {
    mocks.agendaCustom.mockResolvedValue([
      item({
        headlineId: 42,
        filePath: "inbox.org",
        title: "Ship v0.1",
        byteStart: 128,
        agendaDate: "2026-09-05",
      }),
    ]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" });
      await Promise.resolve();
    });

    const link = container.querySelector('a[href^="/editor/"]');
    expect(link?.getAttribute("href")).toBe("/editor/inbox.org/42?byteStart=128");
  });

  it("commits the filter drafts to the route search params on Apply", async () => {
    const onSearchChange = vi.fn();
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" }, onSearchChange);
      await Promise.resolve();
    });

    const tagInput = container.querySelector<HTMLInputElement>('input[placeholder="e.g. home"]');
    expect(tagInput).not.toBeNull();
    act(() => setInputValue(tagInput!, "home"));

    const form = container.querySelector("form");
    await act(async () => {
      form?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      await Promise.resolve();
    });

    expect(onSearchChange).toHaveBeenCalledTimes(1);
    expect(onSearchChange).toHaveBeenCalledWith({
      start: "2026-09-05",
      end: "2026-10-04",
      tag: "home",
      todo: undefined,
    });
  });

  it("applies the file-path filter (local state, not a search param) on Apply", async () => {
    const onSearchChange = vi.fn();
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" }, onSearchChange);
      await Promise.resolve();
    });

    const fileInput = container.querySelector<HTMLInputElement>(
      'input[placeholder="e.g. projects/*"]',
    );
    act(() => setInputValue(fileInput!, "projects/*"));

    const form = container.querySelector("form");
    await act(async () => {
      form?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      await Promise.resolve();
    });

    // File path is NOT pushed to the URL...
    expect(onSearchChange).toHaveBeenCalledWith(
      expect.not.objectContaining({ filePath: expect.anything() }),
    );
    // ...but it IS applied to the re-fetched query.
    const calls = mocks.agendaCustom.mock.calls;
    const lastQuery = calls[calls.length - 1][0] as Record<string, unknown>;
    expect(lastQuery.filePathGlob).toBe("projects/*");
  });

  it("defaults completedInRange to false in the query (Story 7.5)", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" });
      await Promise.resolve();
    });

    const query = mocks.agendaCustom.mock.calls[0][0] as Record<string, unknown>;
    expect(query.completedInRange).toBe(false);
  });

  it("sets completedInRange after toggling the Completed checkbox and applying", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustom({ start: "2026-09-05", end: "2026-10-04" });
      await Promise.resolve();
    });

    const checkbox = container.querySelector<HTMLInputElement>('input[type="checkbox"]');
    expect(checkbox).not.toBeNull();
    await act(async () => {
      checkbox!.click(); // toggles checked + fires React onChange
      const form = container.querySelector("form");
      form?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      await Promise.resolve();
    });

    const calls = mocks.agendaCustom.mock.calls;
    const lastQuery = calls[calls.length - 1][0] as Record<string, unknown>;
    expect(lastQuery.completedInRange).toBe(true);
  });

  it("reports the applied filter snapshot via onAppliedChange (Story 7.5)", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    const onAppliedChange = vi.fn();
    await act(async () => {
      renderCustomWith({
        search: { start: "2026-09-05", end: "2026-10-04", tag: "home" },
        onSearchChange: () => {},
        onAppliedChange,
      });
      await Promise.resolve();
    });

    expect(onAppliedChange).toHaveBeenCalled();
    const snapshot = onAppliedChange.mock.calls[onAppliedChange.mock.calls.length - 1][0];
    expect(snapshot).toMatchObject({
      start: "2026-09-05",
      end: "2026-10-04",
      tag: "home",
      filePathGlob: "",
      completed: false,
    });
  });

  it("applies a preset's local filters (completed + file path) via presetApply (Story 7.5)", async () => {
    mocks.agendaCustom.mockResolvedValue([]);
    await act(async () => {
      renderCustomWith({
        search: { start: "2026-09-05", end: "2026-10-04" },
        onSearchChange: () => {},
        presetApply: { nonce: 1, completed: true, filePathGlob: "projects/*" },
      });
      await Promise.resolve();
    });

    const calls = mocks.agendaCustom.mock.calls;
    const lastQuery = calls[calls.length - 1][0] as Record<string, unknown>;
    expect(lastQuery.completedInRange).toBe(true);
    expect(lastQuery.filePathGlob).toBe("projects/*");
  });
});

describe("resolvePresetWindow (Story 7.5)", () => {
  it("resolves a rolling preset to the last N days ending today", () => {
    // N=7 → today plus the previous six days.
    expect(
      resolvePresetWindow({ rollingDays: 7, start: null, end: null }, "2026-09-14"),
    ).toEqual({ start: "2026-09-08", end: "2026-09-14" });
    // N=30 → today minus 29.
    expect(
      resolvePresetWindow({ rollingDays: 30, start: null, end: null }, "2026-09-14"),
    ).toEqual({ start: "2026-08-16", end: "2026-09-14" });
  });

  it("restores an absolute preset's stored window unchanged", () => {
    expect(
      resolvePresetWindow(
        { rollingDays: null, start: "2026-09-01", end: "2026-09-30" },
        "2026-09-14",
      ),
    ).toEqual({ start: "2026-09-01", end: "2026-09-30" });
  });

  it("falls back to undefined bounds when an absolute preset has no window", () => {
    expect(
      resolvePresetWindow({ rollingDays: null, start: null, end: null }, "2026-09-14"),
    ).toEqual({ start: undefined, end: undefined });
  });
});

describe("presetToRecall (Story 7.5)", () => {
  it("maps every preset field to the correct recall slot", () => {
    const recall = presetToRecall(
      {
        rollingDays: null,
        start: "2026-09-01",
        end: "2026-09-30",
        tag: "home",
        todoState: "NEXT",
        filePathGlob: "projects/*",
        completed: true,
      },
      "2026-09-14",
    );
    // tag/todo land in the URL search (never swapped), window is the absolute one.
    expect(recall.search).toEqual({
      start: "2026-09-01",
      end: "2026-09-30",
      tag: "home",
      todo: "NEXT",
    });
    // completed + file-path are the local-only filters.
    expect(recall.completed).toBe(true);
    expect(recall.filePathGlob).toBe("projects/*");
  });

  it("resolves a rolling 'Done This Week' default and normalizes nulls", () => {
    const recall = presetToRecall(
      {
        rollingDays: 7,
        start: null,
        end: null,
        tag: null,
        todoState: "DONE",
        filePathGlob: null,
        completed: true,
      },
      "2026-09-14",
    );
    expect(recall.search).toEqual({
      start: "2026-09-08",
      end: "2026-09-14",
      tag: undefined,
      todo: "DONE",
    });
    expect(recall.completed).toBe(true);
    // A null glob becomes the empty string (no filter), never the string "null".
    expect(recall.filePathGlob).toBe("");
  });
});
