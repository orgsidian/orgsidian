import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { commands } from "@/lib/tauri";
import { AgendaToday } from "@/components/agenda/AgendaToday";
import { VaultPicker } from "@/components/settings/VaultPicker";
import { KeybindingsSettings } from "@/components/settings/KeybindingsReference";
import { AppearanceSettings } from "@/components/settings/AppearanceSettings";
import { StarterVaultPicker } from "@/components/onboarding/StarterVaultPicker";
import { CoachingBalloon } from "@/components/coaching/CoachingBalloon";
import { UJ4_CAPTURE_INTRO } from "@/components/coaching/coachingIds";

export const Route = createFileRoute("/_layout/today")({
  component: TodayRoute,
});

// Story 6.6 fix (post-review, 2026-09-05): the capture hotkey
// (Cmd/Ctrl+Shift+Space) is not wired until Story 8.1 (Epic 8, after v0.1
// Alpha) — rendering `UJ4_CAPTURE_INTRO` before then would coach a first-run
// user toward a shortcut that does nothing. The balloon's id, copy, and
// dismissal persistence stay fully intact (Story 8.1/11.4 inherit them
// unchanged) — only its rendering is gated behind this constant.
// TODO(Story 8.1): render once the capture hotkey is wired. Flip this to
// `true` (or wire it to a real "capture hotkey available" signal once one
// exists) — no other change needed.
const CAPTURE_FEATURE_AVAILABLE = false;

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
      {/* Story 6.6 (UJ-4): the Quick Capture nudge balloon. v0.1 anchor
          decision (see the Story 6.6 story file's Design Notes): the AC's
          "Inbox preview section" is Epic 7 (Today Dashboard) scope and does
          not exist on this route yet (Story 6.3 shipped only the
          Scheduled/Deadline Agenda below) — this calm top-of-route
          placement is the honest v0.1 stand-in. Its id/dismissal wiring is
          real so Story 11.4 can re-anchor it without touching the seam.

          GATED (post-review fix, 2026-09-05): the capture hotkey isn't wired
          until Story 8.1, so this balloon is not rendered yet — see
          `CAPTURE_FEATURE_AVAILABLE` above. The component/id/copy/dismissal
          below are otherwise unchanged and ready to un-gate. */}
      {CAPTURE_FEATURE_AVAILABLE && (
        <CoachingBalloon
          id={UJ4_CAPTURE_INTRO}
          className="mb-4"
          dismissLabel="Dismiss the Quick Capture tip"
        >
          <span className="font-medium">Anything on your mind?</span> Press
          Cmd/Ctrl+Shift+Space to capture from anywhere.
        </CoachingBalloon>
      )}

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
