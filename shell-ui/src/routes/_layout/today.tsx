import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { commands } from "@/lib/tauri";
import { Button } from "@/components/ui/button";

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
    </main>
  );
}
