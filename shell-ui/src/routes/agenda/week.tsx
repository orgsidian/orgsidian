import { createFileRoute } from "@tanstack/react-router";
import { AgendaWeek } from "@/components/agenda/AgendaWeek";

export const Route = createFileRoute("/agenda/week")({
  component: AgendaWeekRoute,
});

/**
 * Implements FR-7 (Story 6.4): the `/agenda/week` route renders the rolling
 * 7-day Week Agenda view (builds directly on Story 6.3's `/today`).
 */
function AgendaWeekRoute() {
  return (
    <main className="container mx-auto p-8">
      <AgendaWeek />
    </main>
  );
}
