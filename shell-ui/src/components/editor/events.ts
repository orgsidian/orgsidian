// Implements FR-4 — editor-originated events (plugin-surface consistency, FR-24/LD-26).
//
// The Pseudo-WYSIWYG editor surface emits semantic events (starting with link
// clicks) through ONE shared internal surface rather than reaching into a
// consumer directly. The navigation layer (Epic 8: click-to-source / backlinks)
// subscribes via `onLinkClicked`; the link decoration (`decorations/links.ts`)
// publishes via `emitLinkClicked`. Keeping this contract in its own tiny module
// lets the decoration and its consumer evolve independently and keeps the event
// type off the (parallel-edited) `editorMode.ts` shared file.
//
// This is a synchronous, in-process pub/sub — deliberately NOT the Tauri/specta
// `events` surface (that is Rust-origin backend telemetry). A link click is a
// pure frontend interaction; routing it through the backend would be a parallel
// private path in the opposite direction.

/**
 * Classification of an org link target. `kind` lets the navigation layer branch
 * without re-parsing: `id` → node-id lookup, `file` → open path, `http` →
 * external URL, `wiki` → fuzzy title/wiki resolution (the org default when no
 * recognized scheme prefix is present).
 */
export type LinkKind = "id" | "wiki" | "file" | "http";

/**
 * A link was clicked in the editor. `target` is the raw link path exactly as it
 * appears in source (the inner `[[…]]` path before any `][description]`, or the
 * whole bare URL) — lossless, so the consumer resolves it however each `kind`
 * requires. `kind` is {@link classifyLink}'s classification of `target`.
 */
export interface LinkClicked {
  target: string;
  kind: LinkKind;
}

type LinkClickedListener = (event: LinkClicked) => void;

// Module-level registry. A `Set` de-duplicates identical listeners and makes
// unsubscription O(1); iteration is over a snapshot so a listener that
// unsubscribes (or subscribes) during dispatch does not corrupt the walk.
const linkClickedListeners = new Set<LinkClickedListener>();

/**
 * Subscribe to {@link LinkClicked} events. Returns an idempotent unsubscribe
 * function (safe to call more than once) — wire it to a React effect cleanup or
 * a plugin teardown so no listener leaks across editor lifecycles.
 */
export function onLinkClicked(listener: LinkClickedListener): () => void {
  linkClickedListeners.add(listener);
  return () => {
    linkClickedListeners.delete(listener);
  };
}

/**
 * Publish a {@link LinkClicked} event to every current subscriber. Iterates a
 * snapshot so subscribe/unsubscribe during dispatch is safe. A throwing
 * listener must not stop the others (one bad consumer cannot starve the rest);
 * its error is reported to the console and dispatch continues.
 */
export function emitLinkClicked(event: LinkClicked): void {
  for (const listener of [...linkClickedListeners]) {
    try {
      listener(event);
    } catch (error) {
      // eslint-disable-next-line no-console
      console.error("onLinkClicked listener threw", error);
    }
  }
}
