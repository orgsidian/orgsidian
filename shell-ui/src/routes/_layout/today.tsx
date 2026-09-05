import { createFileRoute } from "@tanstack/react-router";
import { AgendaToday } from "@/components/agenda/AgendaToday";
import { VaultPicker } from "@/components/settings/VaultPicker";
import { KeybindingsSettings } from "@/components/settings/KeybindingsReference";
import { AppearanceSettings } from "@/components/settings/AppearanceSettings";

export const Route = createFileRoute("/_layout/today")({
  component: TodayRoute,
});

/**
 * Implements FR-7 (Story 6.3): the `/today` route renders the Agenda Today
 * view. Story 7.1 upgrades this into the full five-section Today Dashboard
 * (Scheduled | Deadline | Today-Tag | Inbox Preview | Active Clock); until
 * then this route IS the Agenda.
 */
function TodayRoute() {
  return (
    <main className="container mx-auto p-8">
      <AgendaToday />

      {/* Story 3.6: minimal mount for the FR-15 Vault designation surface.
          Story 6.2 / 11.1 place the real picker in the onboarding/settings
          flow; this route hosts it until then. */}
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
