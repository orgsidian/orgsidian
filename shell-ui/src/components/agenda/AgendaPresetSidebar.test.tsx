// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 7.5 (FR-7): `AgendaPresetSidebar` lists the saved presets from
 * `commands.listAgendaPresets`, applies one on click (calling `onApply`),
 * saves the current filter snapshot as a new named preset, and deletes a
 * preset via its context menu — refreshing the list after each mutation.
 */

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

const mocks = vi.hoisted(() => ({
  listAgendaPresets: vi.fn<() => Promise<AgendaPresetDto[]>>(),
  saveAgendaPreset: vi.fn<(preset: AgendaPresetDto) => Promise<null>>(),
  deleteAgendaPreset: vi.fn<(name: string) => Promise<null>>(),
}));

vi.mock("@/lib/tauri", () => ({ commands: mocks }));

// Only `errorMessage` is consumed from AgendaToday — stub it so the test does
// not pull that component's whole render chain.
vi.mock("@/components/agenda/AgendaToday", () => ({
  errorMessage: (err: unknown) =>
    err && typeof err === "object" && "reason" in err
      ? String((err as { reason: unknown }).reason)
      : String(err),
}));

import { AgendaPresetSidebar } from "./AgendaPresetSidebar";
import type { AppliedAgendaFilters } from "./AgendaCustom";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.listAgendaPresets.mockReset();
  mocks.saveAgendaPreset.mockReset();
  mocks.deleteAgendaPreset.mockReset();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function preset(overrides: Partial<AgendaPresetDto> & { name: string }): AgendaPresetDto {
  return {
    view: "custom",
    start: null,
    end: null,
    rollingDays: null,
    tag: null,
    todoState: null,
    filePathGlob: null,
    completed: false,
    ...overrides,
  };
}

const CURRENT: AppliedAgendaFilters = {
  start: "2026-09-05",
  end: "2026-10-04",
  tag: "home",
  todo: "",
  filePathGlob: "",
  completed: false,
};

async function render(
  current: AppliedAgendaFilters | null = CURRENT,
  onApply: (p: AgendaPresetDto) => void = () => {},
) {
  await act(async () => {
    root.render(<AgendaPresetSidebar current={current} onApply={onApply} />);
    await Promise.resolve();
  });
}

describe("AgendaPresetSidebar (Story 7.5, FR-7)", () => {
  it("lists the presets returned by listAgendaPresets", async () => {
    mocks.listAgendaPresets.mockResolvedValue([
      preset({ name: "Done This Week", rollingDays: 7, completed: true }),
      preset({ name: "@home this month" }),
    ]);
    await render();

    const labels = Array.from(container.querySelectorAll("button")).map((b) => b.textContent);
    expect(labels).toContain("Done This Week");
    expect(labels).toContain("@home this month");
  });

  it("calls onApply with the clicked preset", async () => {
    const target = preset({ name: "Done This Week", rollingDays: 7, completed: true });
    mocks.listAgendaPresets.mockResolvedValue([target]);
    const onApply = vi.fn();
    await render(CURRENT, onApply);

    const applyButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Done This Week",
    );
    act(() => applyButton!.click());

    expect(onApply).toHaveBeenCalledTimes(1);
    expect(onApply).toHaveBeenCalledWith(target);
  });

  it("saves the current filters as a named preset and refreshes", async () => {
    mocks.listAgendaPresets.mockResolvedValue([]);
    mocks.saveAgendaPreset.mockResolvedValue(null);
    await render();

    const nameInput = container.querySelector<HTMLInputElement>("#agenda-preset-name");
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(nameInput, "Weekly review");
    nameInput!.dispatchEvent(new Event("input", { bubbles: true }));

    const saveButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Save preset",
    );
    await act(async () => {
      saveButton!.click();
      await Promise.resolve();
    });

    expect(mocks.saveAgendaPreset).toHaveBeenCalledTimes(1);
    const saved = mocks.saveAgendaPreset.mock.calls[0][0];
    // Assert the FULL normalized DTO shape — a regression that stopped
    // normalizing empty-string filters to `null` (todoState/filePathGlob) or
    // dropped rollingDays/view would fail here, not slip through a partial match.
    expect(saved).toEqual({
      name: "Weekly review",
      view: "custom",
      start: "2026-09-05",
      end: "2026-10-04",
      rollingDays: null,
      tag: "home",
      todoState: null, // CURRENT.todo is "" → normalized to null
      filePathGlob: null, // CURRENT.filePathGlob is "" → normalized to null
      completed: false,
    });
    // Refreshed: initial load + post-save reload.
    expect(mocks.listAgendaPresets).toHaveBeenCalledTimes(2);
  });

  it("deletes a preset via its context menu and refreshes", async () => {
    mocks.listAgendaPresets.mockResolvedValue([preset({ name: "@home this month" })]);
    mocks.deleteAgendaPreset.mockResolvedValue(null);
    await render();

    const optionsButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Options for @home this month"]',
    );
    act(() => optionsButton!.click());

    const deleteButton = Array.from(container.querySelectorAll('[role="menuitem"]')).find(
      (b) => b.textContent === "Delete",
    ) as HTMLButtonElement | undefined;
    await act(async () => {
      deleteButton!.click();
      await Promise.resolve();
    });

    expect(mocks.deleteAgendaPreset).toHaveBeenCalledWith("@home this month");
    expect(mocks.listAgendaPresets).toHaveBeenCalledTimes(2);
  });

  it("surfaces a list failure (e.g. no active Vault) as an alert", async () => {
    mocks.listAgendaPresets.mockRejectedValue({ reason: "no active vault; designate a vault first" });
    await render();

    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toBe("no active vault; designate a vault first");
  });

  it("rejects saving a reserved default name client-side without calling the backend", async () => {
    mocks.listAgendaPresets.mockResolvedValue([]);
    mocks.saveAgendaPreset.mockResolvedValue(null);
    await render();

    const nameInput = container.querySelector<HTMLInputElement>("#agenda-preset-name");
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(nameInput, "Done This Week");
    nameInput!.dispatchEvent(new Event("input", { bubbles: true }));

    const saveButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Save preset",
    );
    await act(async () => {
      saveButton!.click();
      await Promise.resolve();
    });

    // The reserved-name guard short-circuits the round-trip entirely.
    expect(mocks.saveAgendaPreset).not.toHaveBeenCalled();
    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain("reserved default preset name");
  });

  it("surfaces a backend save rejection (e.g. reserved name) inline", async () => {
    mocks.listAgendaPresets.mockResolvedValue([]);
    mocks.saveAgendaPreset.mockRejectedValue({
      reason: '"Weekly" is a reserved default preset name and cannot be overwritten',
    });
    await render();

    const nameInput = container.querySelector<HTMLInputElement>("#agenda-preset-name");
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    // A non-reserved name (so the client guard passes) whose save the backend
    // still rejects — proving the backend error surfaces inline.
    setter?.call(nameInput, "Weekly");
    nameInput!.dispatchEvent(new Event("input", { bubbles: true }));

    const saveButton = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent === "Save preset",
    );
    await act(async () => {
      saveButton!.click();
      await Promise.resolve();
    });

    expect(mocks.saveAgendaPreset).toHaveBeenCalledTimes(1);
    const alert = container.querySelector('[role="alert"]');
    expect(alert?.textContent).toContain("reserved default preset name");
  });

  it("dismisses an open context menu on Escape", async () => {
    mocks.listAgendaPresets.mockResolvedValue([preset({ name: "@home this month" })]);
    await render();

    const optionsButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Options for @home this month"]',
    );
    act(() => optionsButton!.click());
    expect(container.querySelector('[role="menu"]')).not.toBeNull();

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(container.querySelector('[role="menu"]')).toBeNull();
  });

  it("dismisses an open context menu on an outside pointer press", async () => {
    mocks.listAgendaPresets.mockResolvedValue([preset({ name: "@home this month" })]);
    await render();

    const optionsButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Options for @home this month"]',
    );
    act(() => optionsButton!.click());
    expect(container.querySelector('[role="menu"]')).not.toBeNull();

    // A pointerdown outside any preset row closes the menu.
    act(() => {
      document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    });
    expect(container.querySelector('[role="menu"]')).toBeNull();
  });

  it("switches the open menu when a different row's options are clicked", async () => {
    mocks.listAgendaPresets.mockResolvedValue([
      preset({ name: "First" }),
      preset({ name: "Second" }),
    ]);
    await render();

    const first = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Options for First"]',
    );
    const second = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Options for Second"]',
    );

    // Open First's menu.
    act(() => first!.click());
    expect(first!.getAttribute("aria-expanded")).toBe("true");
    expect(second!.getAttribute("aria-expanded")).toBe("false");

    // Clicking Second's options must close First's menu and open Second's — the
    // #8 fix: the outside-pointer dismissal is scoped to the OPEN row, so it no
    // longer leaves First's menu stuck open behind Second's.
    act(() => {
      second!.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      second!.click();
    });
    expect(first!.getAttribute("aria-expanded")).toBe("false");
    expect(second!.getAttribute("aria-expanded")).toBe("true");
    expect(container.querySelectorAll('[role="menu"]').length).toBe(1);
  });
});
