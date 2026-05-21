import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/_layout/today")({
  component: TodayPlaceholder,
});

function TodayPlaceholder() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Story 1.4 replaces this with the typed specta client.
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="container mx-auto p-8">
      <h1 className="text-2xl font-semibold">Today (placeholder)</h1>
      <p className="text-sm text-muted-foreground mt-2">
        Story 7.1 will replace this with the real Today Dashboard.
      </p>
      <form
        className="mt-6 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          className="border rounded px-2 py-1"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <Button type="submit">Greet</Button>
      </form>
      <p className="mt-3 text-sm">{greetMsg}</p>
    </main>
  );
}
