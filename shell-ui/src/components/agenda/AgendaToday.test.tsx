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
 * Story 6.3 (FR-7): `AgendaToday` queries `commands.agendaToday` once per
 * mount, groups the flat (already backend-sorted) result by source file
 * without re-sorting, and renders a click-to-open `Link` per item to
 * `/editor/$filePath/$headlineId` (`byteStart` search param carried through
 * for the cursor-jump — see `Editor`'s `initialByteOffset` prop).
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
};

const mocks = vi.hoisted(() => ({
  agendaToday: vi.fn<(today: string) => Promise<AgendaItemDto[]>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: { agendaToday: mocks.agendaToday },
}));

// Imported AFTER the mock is registered.
import { AgendaToday } from "./AgendaToday";

// React needs this flag to run effects under `act` in a non-DOM-test-runner.
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.agendaToday.mockReset();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

/**
 * A minimal router carrying just enough of the real route tree for
 * `AgendaToday`'s `Link`s to resolve `/editor/$filePath/$headlineId` — no
 * full app route tree needed for a component test.
 */
function testRouter() {
  const rootRoute = createRootRoute();
  const editorRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/editor/$filePath/$headlineId",
  });
  return createRouter({
    routeTree: rootRoute.addChildren([editorRoute]),
    history: createMemoryHistory({ initialEntries: ["/today"] }),
  });
}

function renderAgendaToday() {
  const router = testRouter();
  act(() => {
    root.render(
      <RouterContextProvider router={router}>
        <AgendaToday />
      </RouterContextProvider>,
    );
  });
}

function item(overrides: Partial<AgendaItemDto>): AgendaItemDto {
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

describe("AgendaToday (Story 6.3, FR-7)", () => {
  it("shows a loading placeholder before the query resolves", () => {
    mocks.agendaToday.mockReturnValue(new Promise(() => {})); // never resolves
    renderAgendaToday();

    expect(container.textContent).toContain("Loading…");
  });

  it("shows an empty-state line when there is nothing today", async () => {
    mocks.agendaToday.mockResolvedValue([]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    expect(container.textContent).toContain("Nothing scheduled or due today.");
  });

  it("surfaces a query failure as an alert (e.g. no active Vault)", async () => {
    mocks.agendaToday.mockRejectedValue({ reason: "no active vault; designate a vault first" });
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toBe("no active vault; designate a vault first");
  });

  it("groups items by source file, preserving the backend's sort order", async () => {
    mocks.agendaToday.mockResolvedValue([
      item({ headlineId: 1, filePath: "a.org", title: "a first" }),
      item({ headlineId: 2, filePath: "b.org", title: "b first" }),
      item({ headlineId: 3, filePath: "b.org", title: "b second" }),
    ]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    const headings = Array.from(container.querySelectorAll("h2")).map(
      (el) => el.textContent,
    );
    expect(headings).toEqual(["a.org", "b.org"]);

    const items = Array.from(container.querySelectorAll("li")).map(
      (el) => el.textContent,
    );
    expect(items).toHaveLength(3);
    expect(items[0]).toContain("a first");
    expect(items[1]).toContain("b first");
    expect(items[2]).toContain("b second");
  });

  it("renders a click-to-open Link to /editor/$filePath/$headlineId with the byteStart search param", async () => {
    mocks.agendaToday.mockResolvedValue([
      item({ headlineId: 42, filePath: "inbox.org", title: "Ship v0.1", byteStart: 128 }),
    ]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    const link = container.querySelector("a");
    expect(link).not.toBeNull();
    expect(link?.getAttribute("href")).toBe("/editor/inbox.org/42?byteStart=128");
  });

  it("percent-encodes a slash-containing filePath so it survives as one route segment", async () => {
    mocks.agendaToday.mockResolvedValue([
      item({ headlineId: 7, filePath: "projects/orgsidian.org", title: "Ship it" }),
    ]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    const link = container.querySelector("a");
    // TanStack Router's default `Link` param encoding turns `/` into `%2F` so
    // the whole vault-relative path stays inside the `$filePath` segment
    // rather than being split into extra path segments.
    expect(link?.getAttribute("href")).toBe(
      "/editor/projects%2Forgsidian.org/7?byteStart=0",
    );
  });

  it("labels an overdue Deadline distinctly from one due today, and does not mislabel a future Deadline", async () => {
    // Dates are computed relative to the real local `today` the component
    // reads (`localTodayIso()`), so this stays deterministic on any run date.
    const iso = (offsetDays: number) => {
      const d = new Date();
      d.setDate(d.getDate() + offsetDays);
      const y = d.getFullYear().toString().padStart(4, "0");
      const m = (d.getMonth() + 1).toString().padStart(2, "0");
      const day = d.getDate().toString().padStart(2, "0");
      return `${y}-${m}-${day}`;
    };
    const todayIso = iso(0);
    const pastIso = iso(-4);
    const futureIso = iso(5);

    mocks.agendaToday.mockResolvedValue([
      item({
        headlineId: 1,
        title: "Overdue task",
        deadlineDate: pastIso,
        overdue: true,
      }),
      item({
        headlineId: 2,
        title: "Due today task",
        deadlineDate: todayIso,
        overdue: false,
      }),
      // A Scheduled-today row that also carries a future Deadline (matched via
      // the Scheduled leg): overdue is false but it is NOT due today.
      item({
        headlineId: 3,
        title: "Scheduled with later deadline",
        scheduledDate: todayIso,
        deadlineDate: futureIso,
        overdue: false,
      }),
    ]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    expect(container.textContent).toContain(`Overdue (${pastIso})`);
    expect(container.textContent).toContain(`Due today (${todayIso})`);
    // The future Deadline reads a plain "Due", never "Due today".
    expect(container.textContent).toContain(`Due (${futureIso})`);
    expect(container.textContent).not.toContain(`Due today (${futureIso})`);
  });

  it("queries agendaToday exactly once per mount, with today's local date", async () => {
    mocks.agendaToday.mockResolvedValue([]);
    await act(async () => {
      renderAgendaToday();
      await Promise.resolve();
    });

    expect(mocks.agendaToday).toHaveBeenCalledTimes(1);
    expect(mocks.agendaToday).toHaveBeenCalledWith(
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
    );
  });
});
