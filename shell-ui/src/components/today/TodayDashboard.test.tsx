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
 * Story 7.1 (FR-6): `TodayDashboard` queries `commands.todayDashboard` once per
 * mount and renders five collapsible sections — Scheduled | Deadline |
 * Today-Tag | Inbox Preview | Active Clock — each toggled by a chevron header
 * (default expanded). Rows are click-to-open `Link`s; the backend already
 * orders/limits every section, so the component never re-sorts.
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

type InboxItemDto = {
  headlineId: number;
  filePath: string;
  title: string;
  byteStart: number;
  todoKeyword: string | null;
};

type ActiveClockDto = {
  headlineId: number;
  filePath: string;
  title: string;
  byteStart: number;
  startAt: string;
};

type TodayDashboardDto = {
  scheduled: AgendaItemDto[];
  deadlines: AgendaItemDto[];
  todayTag: AgendaItemDto[];
  inbox: InboxItemDto[];
  activeClock: ActiveClockDto | null;
};

type DashboardSection =
  | "scheduled"
  | "deadline"
  | "todayTag"
  | "inboxPreview"
  | "activeClock";

type TodayDashboardPrefs = {
  scheduled: boolean;
  deadline: boolean;
  todayTag: boolean;
  inboxPreview: boolean;
  activeClock: boolean;
};

const ALL_EXPANDED: TodayDashboardPrefs = {
  scheduled: false,
  deadline: false,
  todayTag: false,
  inboxPreview: false,
  activeClock: false,
};

const mocks = vi.hoisted(() => ({
  todayDashboard: vi.fn<(today: string) => Promise<TodayDashboardDto>>(),
  getDismissedCoaching: vi.fn<() => Promise<string[]>>(),
  dismissCoaching: vi.fn<(id: string) => Promise<void>>(),
  getTodayDashboardPrefs: vi.fn<() => Promise<TodayDashboardPrefs>>(),
  setTodayDashboardSectionCollapsed:
    vi.fn<(section: DashboardSection, collapsed: boolean) => Promise<void>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    todayDashboard: mocks.todayDashboard,
    getDismissedCoaching: mocks.getDismissedCoaching,
    dismissCoaching: mocks.dismissCoaching,
    getTodayDashboardPrefs: mocks.getTodayDashboardPrefs,
    setTodayDashboardSectionCollapsed: mocks.setTodayDashboardSectionCollapsed,
  },
}));

// Imported AFTER the mock is registered.
import { TodayDashboard } from "./TodayDashboard";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.todayDashboard.mockReset();
  mocks.getDismissedCoaching.mockReset().mockResolvedValue([]);
  mocks.dismissCoaching.mockReset().mockResolvedValue(undefined);
  // Story 7.2: sections default to all-expanded unless a test overrides.
  mocks.getTodayDashboardPrefs.mockReset().mockResolvedValue(ALL_EXPANDED);
  mocks.setTodayDashboardSectionCollapsed
    .mockReset()
    .mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

/** A minimal router so the dashboard's `/editor/...` `Link`s resolve. */
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

function renderDashboard() {
  const router = testRouter();
  act(() => {
    root.render(
      <RouterContextProvider router={router}>
        <TodayDashboard />
      </RouterContextProvider>,
    );
  });
}

function agendaItem(overrides: Partial<AgendaItemDto>): AgendaItemDto {
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
    agendaDate: "2026-09-05",
    ...overrides,
  };
}

function dashboard(overrides: Partial<TodayDashboardDto>): TodayDashboardDto {
  return {
    scheduled: [],
    deadlines: [],
    todayTag: [],
    inbox: [],
    activeClock: null,
    ...overrides,
  };
}

function prefs(overrides: Partial<TodayDashboardPrefs>): TodayDashboardPrefs {
  return { ...ALL_EXPANDED, ...overrides };
}

/** The trigger button whose header text starts with `title`. */
function sectionTrigger(title: string): HTMLButtonElement | undefined {
  return Array.from(container.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(title),
  ) as HTMLButtonElement | undefined;
}

describe("TodayDashboard (Story 7.1, FR-6)", () => {
  it("shows a loading placeholder before the query resolves", () => {
    mocks.todayDashboard.mockReturnValue(new Promise(() => {})); // never resolves
    mocks.getTodayDashboardPrefs.mockReturnValue(new Promise(() => {})); // never resolves — avoid a post-render setPrefs() outside act()
    renderDashboard();

    expect(container.textContent).toContain("Loading…");
  });

  it("surfaces a query failure as an alert (e.g. no active Vault)", async () => {
    mocks.todayDashboard.mockRejectedValue({
      reason: "no active vault; designate a vault first",
    });
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toBe("no active vault; designate a vault first");
  });

  it("renders all five sections in order, each collapsible via a chevron toggle", async () => {
    mocks.todayDashboard.mockResolvedValue(dashboard({}));
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    const headers = ["Scheduled", "Deadline", "Today-Tag", "Inbox Preview", "Active Clock"];
    const triggers = Array.from(container.querySelectorAll("button"));
    // One collapsible trigger (a button) per section, in render order.
    expect(triggers).toHaveLength(5);

    // Assert each of the five specific sections is present DISTINCTLY, in order,
    // and that each carries its OWN accessible count label scoped to that
    // section — so a swapped, dropped, or mislabeled section is caught (a single
    // page-wide `[aria-label="0 items"]` would pass even if two sections were
    // identical or one were missing).
    headers.forEach((title, i) => {
      const trigger = triggers[i];
      expect(trigger.textContent).toContain(title);
      // The per-section count carries an accessible label (WCAG gate): a bare
      // number span would announce e.g. "Scheduled 0" with no meaning. All
      // sections are empty here, so each label reads "0 items".
      const count = trigger.querySelector('[aria-label="0 items"]');
      expect(count, `${title} section must carry its own count label`).not.toBeNull();
      expect(count?.textContent).toBe("0");
    });

    // Exactly one labelled count per section — no extras, no missing.
    expect(container.querySelectorAll('[aria-label="0 items"]')).toHaveLength(5);
  });

  it("renders each section's rows and the active clock", async () => {
    mocks.todayDashboard.mockResolvedValue(
      dashboard({
        scheduled: [agendaItem({ headlineId: 1, title: "Ship v0.1" })],
        deadlines: [
          agendaItem({
            headlineId: 2,
            title: "Overdue task",
            deadlineDate: "2026-09-01",
            overdue: true,
          }),
        ],
        todayTag: [agendaItem({ headlineId: 3, title: "Tagged item" })],
        inbox: [
          { headlineId: 4, filePath: "inbox.org", title: "Captured note", byteStart: 0, todoKeyword: null },
        ],
        activeClock: {
          headlineId: 5,
          filePath: "work.org",
          title: "Deep work",
          byteStart: 12,
          startAt: "2026-09-05T09:00:00",
        },
      }),
    );
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    expect(container.textContent).toContain("Ship v0.1");
    expect(container.textContent).toContain("Overdue task");
    expect(container.textContent).toContain("Overdue (2026-09-01)");
    expect(container.textContent).toContain("Tagged item");
    expect(container.textContent).toContain("Captured note");
    expect(container.textContent).toContain("Deep work");
    expect(container.textContent).toContain("since 2026-09-05T09:00:00");

    // Click-to-open Link carries the filePath/headlineId + byteStart.
    const clockLink = container.querySelector('a[href^="/editor/work.org"]');
    expect(clockLink?.getAttribute("href")).toBe("/editor/work.org/5?byteStart=12");
  });

  it("renders minimal empty-state bodies for empty sections (rich copy is Story 7.3)", async () => {
    mocks.todayDashboard.mockResolvedValue(dashboard({}));
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    // Header still renders; body is a minimal blank line, not rich coaching.
    expect(sectionTrigger("Scheduled")).not.toBeUndefined();
    expect(container.textContent).toContain("Nothing scheduled for today.");
    expect(container.textContent).toContain("No active clock.");
    expect(container.textContent).toContain("Inbox is empty.");
  });

  it("collapses a section body when its chevron toggle is clicked", async () => {
    mocks.todayDashboard.mockResolvedValue(
      dashboard({
        scheduled: [agendaItem({ headlineId: 1, title: "Collapse me" })],
      }),
    );
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    // Expanded by default: the row is visible.
    expect(container.textContent).toContain("Collapse me");

    const trigger = sectionTrigger("Scheduled");
    expect(trigger).not.toBeUndefined();

    await act(async () => {
      trigger!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    // Collapsed: the body content is removed from the DOM.
    expect(container.textContent).not.toContain("Collapse me");
    // The section header itself still renders.
    expect(sectionTrigger("Scheduled")).not.toBeUndefined();
  });

  it("queries todayDashboard exactly once per mount, with today's local date", async () => {
    mocks.todayDashboard.mockResolvedValue(dashboard({}));
    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    expect(mocks.todayDashboard).toHaveBeenCalledTimes(1);
    expect(mocks.todayDashboard).toHaveBeenCalledWith(
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
    );
  });

  // Story 7.2 (FR-6): section collapse state persists across restarts.
  it("restores each section's collapsed state from persisted prefs on mount", async () => {
    mocks.todayDashboard.mockResolvedValue(
      dashboard({
        scheduled: [agendaItem({ headlineId: 1, title: "Persisted hidden row" })],
        deadlines: [agendaItem({ headlineId: 2, title: "Persisted visible row" })],
      }),
    );
    // Scheduled was collapsed last session; Deadline expanded.
    mocks.getTodayDashboardPrefs.mockResolvedValue(prefs({ scheduled: true }));

    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    // Collapsed section: its body row is absent from the DOM, header present.
    expect(sectionTrigger("Scheduled")).not.toBeUndefined();
    expect(container.textContent).not.toContain("Persisted hidden row");
    // Expanded section: its body row is visible.
    expect(container.textContent).toContain("Persisted visible row");
  });

  it("persists a section's collapsed state fire-and-forget when its chevron is toggled", async () => {
    mocks.todayDashboard.mockResolvedValue(
      dashboard({
        scheduled: [agendaItem({ headlineId: 1, title: "Toggle me" })],
      }),
    );
    mocks.getTodayDashboardPrefs.mockResolvedValue(ALL_EXPANDED);

    await act(async () => {
      renderDashboard();
      await Promise.resolve();
    });

    // Nothing persisted until the user toggles.
    expect(mocks.setTodayDashboardSectionCollapsed).not.toHaveBeenCalled();

    // Collapse Scheduled → persist ("scheduled", collapsed = true).
    const trigger = sectionTrigger("Scheduled");
    expect(trigger).not.toBeUndefined();
    await act(async () => {
      trigger!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    expect(mocks.setTodayDashboardSectionCollapsed).toHaveBeenLastCalledWith(
      "scheduled",
      true,
    );

    // Expand it again → persist ("scheduled", collapsed = false).
    await act(async () => {
      sectionTrigger("Scheduled")!.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      );
      await Promise.resolve();
    });

    expect(mocks.setTodayDashboardSectionCollapsed).toHaveBeenLastCalledWith(
      "scheduled",
      false,
    );
    expect(mocks.setTodayDashboardSectionCollapsed).toHaveBeenCalledTimes(2);
  });
});
