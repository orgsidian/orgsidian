// Implements FR-7 (Story 6.3 v0.1 subset): the click-to-open target the
// Agenda (`AgendaToday.tsx`) navigates to. `$filePath` and `$headlineId` are
// path params per the AC's route shape; TanStack Router percent-encodes each
// param segment on `Link` (so a `$filePath` containing `/` survives as one
// segment) and decodes it back here — see `AgendaToday.tsx`'s `Link`.
//
// `byteStart` is an OPTIONAL search param (not part of the AC's route path)
// carrying the clicked Headline's byte offset, so the editor can place the
// cursor there instead of just opening the file at its top (see `Editor`'s
// `initialByteOffset` prop). Absent/malformed is a silent fall-through to "no
// jump", never a route error — a hand-edited or stale URL must still open the
// file.

import { createFileRoute } from "@tanstack/react-router";

import { Editor } from "@/components/editor/Editor";

interface EditorRouteSearch {
  byteStart?: number;
}

export const Route = createFileRoute("/editor/$filePath/$headlineId")({
  validateSearch: (search: Record<string, unknown>): EditorRouteSearch => ({
    byteStart:
      typeof search.byteStart === "number" && Number.isFinite(search.byteStart)
        ? search.byteStart
        : undefined,
  }),
  component: EditorRoute,
});

function EditorRoute() {
  const { filePath, headlineId } = Route.useParams();
  const { byteStart } = Route.useSearch();

  return (
    <div
      className="h-screen w-full"
      data-headline-id={headlineId}
    >
      <Editor filePath={filePath} initialByteOffset={byteStart} />
    </div>
  );
}
