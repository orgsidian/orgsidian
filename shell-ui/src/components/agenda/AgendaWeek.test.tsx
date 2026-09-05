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
 * Story 6.4 (FR-7): `AgendaWeek` queries `commands.agendaWeek` once per mount
 * with the local `startDate`, groups the flat (already backend-sorted)
 * result by `agendaDate` without re-sorting, renders all 7 window days (even
 * empty ones) with the current day highlighted, and renders a click-to-open
 * `Link` per item to `/editor/$filePath/$headlineId` — same target
 * `AgendaToday`'s row uses.
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
  agendaWeek: vi.fn<(startDate: string) => Promise<AgendaItemDto[]>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: { agendaWeek: mocks.agendaWeek },
}));

// Imported AFTER the mock is registered.
import { AgendaWeek } from "./AgendaWeek";

// React needs this flag to run effects under `act` in a non-DOM-test-runner.
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.agendaWeek.mockReset();
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
 * `AgendaWeek`'s `Link`s to resolve `/editor/$filePath/$headlineId` and
 * `/today` (the back-link) — no full app route tree needed for a component
 * test.
 */
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
    history: createMemoryHistory({ initialEntries: ["/agenda/week"] }),
  });
}

function renderAgendaWeek() {
  const router = testRouter();
  act(() => {
    root.render(
      <RouterContextProvider router={router}>
        <AgendaWeek />
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

/** The real local calendar day `offsetDays` away from today, `YYYY-MM-DD`. */
function iso(offsetDays: number): string {
  const d = new Date();
  d.setDate(d.getDate() + offsetDays);
  const y = d.getFullYear().toString().padStart(4, "0");
  const m = (d.getMonth() + 1).toString().padStart(2, "0");
  const day = d.getDate().toString().padStart(2, "0");
  return `${y}-${m}-${day}`;
}

describe("AgendaWeek (Story 6.4, FR-7)", () => {
  it("shows a loading placeholder before the query resolves", () => {
    mocks.agendaWeek.mockReturnValue(new Promise(() => {})); // never resolves
    renderAgendaWeek();

    expect(container.textContent).toContain("Loading…");
  });

  it("surfaces a query failure as an alert (e.g. no active Vault)", async () => {
    mocks.agendaWeek.mockRejectedValue({ reason: "no active vault; designate a vault first" });
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toBe("no active vault; designate a vault first");
  });

  it("renders all 7 window days, even when there are no items, with the current day marked", async () => {
    mocks.agendaWeek.mockResolvedValue([]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const headings = container.querySelectorAll("h2");
    expect(headings).toHaveLength(7);

    const emptyLines = Array.from(container.querySelectorAll("p")).filter(
      (el) => el.textContent === "Nothing scheduled or due.",
    );
    expect(emptyLines).toHaveLength(7);

    // Exactly one of the 7 days is marked current.
    const currentDayHeadings = Array.from(headings).filter((h) =>
      h.textContent?.includes("(Today)"),
    );
    expect(currentDayHeadings).toHaveLength(1);
  });

  it("renders the 'Back to Today' view-switch Link to /today", async () => {
    // Mirror of AgendaToday's "View week" assertion: the other half of the
    // view-switch pair the perf AC names must point back at /today.
    mocks.agendaWeek.mockResolvedValue([]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const backToToday = container.querySelector('a[href="/today"]');
    expect(backToToday).not.toBeNull();
    expect(backToToday?.textContent).toContain("Back to Today");
  });

  it("groups items by agendaDate, preserving the backend's sort order, without re-sorting", async () => {
    const day0 = iso(0);
    const day2 = iso(2);
    mocks.agendaWeek.mockResolvedValue([
      item({ headlineId: 1, filePath: "a.org", title: "a first", agendaDate: day0 }),
      item({ headlineId: 2, filePath: "b.org", title: "b first", agendaDate: day2 }),
      item({ headlineId: 3, filePath: "b.org", title: "b second", agendaDate: day2 }),
    ]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const items = Array.from(container.querySelectorAll("li")).map((el) => el.textContent);
    expect(items).toHaveLength(3);
    expect(items[0]).toContain("a first");
    expect(items[1]).toContain("b first");
    expect(items[2]).toContain("b second");
  });

  it("renders a click-to-open Link to /editor/$filePath/$headlineId with the byteStart search param", async () => {
    mocks.agendaWeek.mockResolvedValue([
      item({
        headlineId: 42,
        filePath: "inbox.org",
        title: "Ship v0.1",
        byteStart: 128,
        agendaDate: iso(0),
      }),
    ]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const link = container.querySelector('a[href^="/editor/"]');
    expect(link).not.toBeNull();
    expect(link?.getAttribute("href")).toBe("/editor/inbox.org/42?byteStart=128");
  });

  it("percent-encodes a slash-containing filePath so it survives as one route segment", async () => {
    mocks.agendaWeek.mockResolvedValue([
      item({
        headlineId: 7,
        filePath: "projects/orgsidian.org",
        title: "Ship it",
        agendaDate: iso(0),
      }),
    ]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    const link = container.querySelector('a[href^="/editor/"]');
    expect(link?.getAttribute("href")).toBe(
      "/editor/projects%2Forgsidian.org/7?byteStart=0",
    );
  });

  it("labels an overdue Deadline distinctly from one due within the window", async () => {
    mocks.agendaWeek.mockResolvedValue([
      item({
        headlineId: 1,
        title: "Overdue task",
        deadlineDate: iso(-4),
        overdue: true,
        agendaDate: iso(0),
      }),
      item({
        headlineId: 2,
        title: "Due mid-week task",
        deadlineDate: iso(3),
        overdue: false,
        agendaDate: iso(3),
      }),
    ]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    expect(container.textContent).toContain(`Overdue (${iso(-4)})`);
    expect(container.textContent).toContain(`Due (${iso(3)})`);
  });

  it("queries agendaWeek exactly once per mount, with today's local date", async () => {
    mocks.agendaWeek.mockResolvedValue([]);
    await act(async () => {
      renderAgendaWeek();
      await Promise.resolve();
    });

    expect(mocks.agendaWeek).toHaveBeenCalledTimes(1);
    expect(mocks.agendaWeek).toHaveBeenCalledWith(
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
    );
    expect(mocks.agendaWeek).toHaveBeenCalledWith(iso(0));
  });
});
