import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { commands } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { VaultPicker } from "@/components/settings/VaultPicker";
import { KeybindingsReference } from "@/components/settings/KeybindingsReference";

export const Route = createFileRoute("/_layout/today")({
  component: TodayPlaceholder,
});

function TodayPlaceholder() {
  const [reply, setReply] = useState("");

  async function ping() {
    setReply(await commands.ping());
  }

  return (
    <main className="container mx-auto p-8">
      <h1 className="text-2xl font-semibold">Today (placeholder)</h1>
      <p className="text-sm text-muted-foreground mt-2">
        Story 7.1 will replace this with the real Today Dashboard.
      </p>
      <div className="mt-6 flex gap-2">
        <Button type="button" onClick={ping}>
          Ping
        </Button>
      </div>
      <p className="mt-3 text-sm">{reply}</p>

      {/* Story 3.6: minimal mount for the FR-15 Vault designation surface.
          Story 6.2 / 11.1 place the real picker in the onboarding/settings
          flow; this placeholder route hosts it until then. */}
      <VaultPicker />

      {/* Story 4.6 (FR-5): the Settings → Keybindings reference panel. Hosted on
          this placeholder settings route until the real Settings flow (Epic 6 /
          11) provides dedicated navigation. */}
      <KeybindingsReference className="mt-8" />
    </main>
  );
}
