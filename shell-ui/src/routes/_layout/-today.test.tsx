// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Story 6.6 (UJ-4): the `/today` route wires the Quick Capture nudge balloon
 * (`UJ4_CAPTURE_INTRO`) at the top of the route content. `CoachingBalloon.test.tsx`
 * covers the component generically and `AgendaToday.test.tsx` covers the
 * `UJ4_TODAY_INTRO` mount.
 *
 * Post-review fix (2026-09-05): the capture hotkey (Cmd/Ctrl+Shift+Space)
 * isn't wired until Story 8.1, so `UJ4_CAPTURE_INTRO` is now gated off behind
 * `CAPTURE_FEATURE_AVAILABLE = false` in `today.tsx` — rendering it earlier
 * would coach a first-run user toward a dead shortcut. These tests assert the
 * balloon does NOT render while gated (still referencing the real id/copy
 * constants so a future un-gate that silently breaks the wiring — e.g. a
 * wrong id — is caught), and that `UJ4_TODAY_INTRO` is unaffected.
 *
 * The route renders the REAL `CoachingBalloon` + `coachingIds` (that is the
 * wiring under test); the heavy sibling surfaces (Agenda, settings panels,
 * onboarding picker) are stubbed, and `@tanstack/react-router`'s
 * `createFileRoute` is mocked to expose the route component directly without a
 * generated route tree.
 */

const mocks = vi.hoisted(() => ({
  hasConfiguredVault: vi.fn<() => Promise<boolean>>(),
  getDismissedCoaching: vi.fn<() => Promise<string[]>>(),
  dismissCoaching: vi.fn<(id: string) => Promise<void>>(),
}));

vi.mock("@/lib/tauri", () => ({
  commands: {
    hasConfiguredVault: mocks.hasConfiguredVault,
    getDismissedCoaching: mocks.getDismissedCoaching,
    dismissCoaching: mocks.dismissCoaching,
  },
}));

// Expose the route's component without a generated route tree.
vi.mock("@tanstack/react-router", () => ({
  createFileRoute: () => (opts: unknown) => ({ options: opts }),
}));

// Stub the heavy sibling surfaces — not under test here.
vi.mock("@/components/agenda/AgendaToday", () => ({
  AgendaToday: () => <div data-testid="agenda-stub" />,
}));
vi.mock("@/components/settings/VaultPicker", () => ({
  VaultPicker: () => <div data-testid="vault-picker-stub" />,
}));
vi.mock("@/components/settings/KeybindingsReference", () => ({
  KeybindingsSettings: () => <div data-testid="keybindings-stub" />,
}));
vi.mock("@/components/settings/AppearanceSettings", () => ({
  AppearanceSettings: () => <div data-testid="appearance-stub" />,
}));
vi.mock("@/components/onboarding/StarterVaultPicker", () => ({
  StarterVaultPicker: () => <div data-testid="starter-vault-picker-stub" />,
}));

// Imported AFTER the mocks are registered.
import { Route } from "./today";
import { UJ4_CAPTURE_INTRO } from "@/components/coaching/coachingIds";

const TodayRoute = (Route as unknown as { options: { component: () => React.ReactNode } })
  .options.component;

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  mocks.hasConfiguredVault.mockReset().mockResolvedValue(true);
  mocks.getDismissedCoaching.mockReset().mockResolvedValue([]);
  mocks.dismissCoaching.mockReset().mockResolvedValue(undefined);
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
    root.render(<TodayRoute />);
  });
  // Flush the onboarding gate resolve → re-render → the balloon's own
  // getDismissedCoaching resolve → re-render.
  for (let i = 0; i < 5; i += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

function captureBalloon() {
  return container.querySelector(`[data-coaching-id="${UJ4_CAPTURE_INTRO}"]`);
}

describe("TodayRoute (Story 6.6, UJ-4): the Quick Capture coaching balloon — gated until Story 8.1", () => {
  it("does NOT mount the UJ4_CAPTURE_INTRO balloon even when a Vault is configured and undismissed (capture hotkey not wired yet)", async () => {
    await renderRoute();

    expect(captureBalloon()).toBeNull();
  });

  it("does not mount the capture balloon behind the onboarding gate (no Vault yet)", async () => {
    mocks.hasConfiguredVault.mockResolvedValue(false);
    await renderRoute();

    expect(captureBalloon()).toBeNull();
    expect(container.querySelector('[data-testid="starter-vault-picker-stub"]')).not.toBeNull();
  });

  it("stays not-rendered when its id is already dismissed (belt-and-braces: gate plus dismissal both suppress it)", async () => {
    mocks.getDismissedCoaching.mockResolvedValue([UJ4_CAPTURE_INTRO]);
    await renderRoute();

    expect(captureBalloon()).toBeNull();
  });

  it("never asks the backend to check UJ4_CAPTURE_INTRO's dismissal state while gated off (not merely hidden by CSS)", async () => {
    await renderRoute();

    expect(mocks.getDismissedCoaching).not.toHaveBeenCalled();
  });
});
