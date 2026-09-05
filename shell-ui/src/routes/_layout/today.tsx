import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { commands } from "@/lib/tauri";
import { AgendaToday } from "@/components/agenda/AgendaToday";
import { VaultPicker } from "@/components/settings/VaultPicker";
import { KeybindingsSettings } from "@/components/settings/KeybindingsReference";
import { AppearanceSettings } from "@/components/settings/AppearanceSettings";
import { StarterVaultPicker } from "@/components/onboarding/StarterVaultPicker";

export const Route = createFileRoute("/_layout/today")({
  component: TodayRoute,
});

/**
 * Implements FR-7 (Story 6.3) + FR-18 (Story 6.2): the `/today` route. On
 * first launch (no configured Vault) it shows the `StarterVaultPicker`
 * onboarding gate; once a Vault is configured it renders the Agenda Today
 * view. Story 7.1 upgrades the agenda into the full five-section Today
 * Dashboard (Scheduled | Deadline | Today-Tag | Inbox Preview | Active
 * Clock); until then this route IS the Agenda.
 */
function TodayRoute() {
  // Story 6.2 (FR-18): the first-launch onboarding gate. `null` while the
  // check is in flight (render nothing rather than flash the picker then
  // immediately swap it for the real content); `false` shows the
  // `StarterVaultPicker`; `true` shows this route's normal content. A failed
  // check falls back to `true` — a query outage must not trap a returning
  // user behind an onboarding surface they've already been through.
  const [vaultConfigured, setVaultConfigured] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    void commands
      .hasConfiguredVault()
      .then((configured) => {
        if (!cancelled) setVaultConfigured(configured);
      })
      .catch(() => {
        if (!cancelled) setVaultConfigured(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (vaultConfigured === null) {
    return null;
  }

  if (!vaultConfigured) {
    return <StarterVaultPicker onVaultConfigured={() => setVaultConfigured(true)} />;
  }

  return (
    <main className="container mx-auto p-8">
      <AgendaToday />

      {/* Story 3.6: minimal mount for the FR-15 Vault designation surface.
          Story 6.2 hosts the first-launch onboarding picker above (the
          onboarding gate); this manual re-designate entry point stays for
          switching Vaults post-onboarding until the real Settings flow
          (Story 11.x) provides dedicated navigation. */}
      <VaultPicker />

      {/* Story 4.6 + 4.7 (FR-5): Settings → Keybindings — the reference panels
          (native + Emacs) plus the opt-in Emacs-mode toggle. Hosted on this
          route until the real Settings flow (Epic 6 / 11) provides dedicated
          navigation. */}
      <KeybindingsSettings className="mt-8" />

      {/* Story 6.7 (FR-22): Settings → Appearance — dark / light / system-default
          theme toggle. Hosted here until the real Settings flow (Epic 6 / 11)
          provides dedicated navigation, matching the KeybindingsSettings /
          VaultPicker placeholder-route convention above. */}
      <AppearanceSettings className="mt-8" />
    </main>
  );
}
