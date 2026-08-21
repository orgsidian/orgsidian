---
title: 'Link rendering as clickable underlined text'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '52f8fcd'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 shipped Raw mode plus the mode→extension boundary (`editorMode.ts`), with an empty Pseudo-WYSIWYG decoration layer. Epic 4's Pseudo-WYSIWYG mode must render org links (`[[id:abc]]`, `[[wiki-link]]`, `[[file://path]]`, `http://…`) as clickable underlined text while keeping the buffer byte-faithful `.org` source (FR-4, UJ-6, FR-2).

**Approach:** Add a CM6 `ViewPlugin` (LD-6) under `editor/decorations/links.ts` that (a) underlines each link's rendered text via `Decoration.mark`, (b) hides the `[[…]]` bracket markers with `Decoration.replace` when the cursor is not on the link's line and reveals them when it is (recomputing on `selectionSet`), and (c) emits a `LinkClicked { target, kind }` event on click through a small shared frontend event surface (`editor/events.ts`) that the navigation layer (Epic 8) subscribes to — plugin-surface consistency (FR-24 / LD-26). Wire the plugin into the Pseudo-WYSIWYG set in `editorMode.ts` (the only shared file touched).

## Boundaries & Constraints

**Always:**
- Decorations are presentational: the buffer is NEVER mutated (FR-2 round-trip contract). Bracket hiding uses `Decoration.replace` over source ranges; doc offsets are untouched.
- Bracket markers hidden when the cursor is not on the link's line; revealed when it is — recomputed on `docChanged || selectionSet || viewportChanged`.
- Link colors via the existing `--org-*` token vocabulary (reuse `--org-border-focus`, the accent, per 4.2 precedent); no new tokens in `tokens.css` (LD-58 gate).
- `LinkClicked` routes through the shared internal event surface (`editor/events.ts`), not a private path (FR-24 / LD-26).
- FR-4 module carries `// Implements FR-4` as its first line (4.2 TS convention; `tests/traceability.rs` lands with 4.3a).

**Ask First:**
- Introducing new `--org-accent-link` token to `tokens.css` (would enter the LD-58 contrast gate) — links reuse the existing accent token.

**Never:**
- Mutate the buffer on click or hover (navigation is Epic 8; this story only emits the event).
- Refactor Story 4.2 beyond the minimal `editorMode.ts` wiring.
- A barrel `decorations/index.ts` (5 sibling decoration stories run in parallel; direct import minimizes cross-PR conflict).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| id link | `[[id:abc]]`, cursor off-line | `abc`-region underlined; `[[`/`]]` hidden; click → `{target:"id:abc", kind:"id"}` | N/A |
| wiki link | `[[wiki-link]]` | underlined; brackets hidden off-line | N/A |
| file link | `[[file://path]]` | underlined; `kind:"file"` | N/A |
| bare URL | `http://example.com` | whole URL underlined; no brackets to hide; `kind:"http"` | N/A |
| described link | `[[id:x][Label]]` | `Label` underlined; `[[id:x][` + `]]` hidden off-line; click → `{target:"id:x", kind:"id"}` | N/A |
| cursor on link line | any bracket link, cursor on that line | brackets REVEALED (source shown); underline stays | N/A |
| round-trip | any link buffer, no edit | `sliceDoc` byte-identical to source | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/events.ts` — NEW. `// Implements FR-4`. `LinkKind` union + `LinkClicked` interface + a tiny synchronous pub/sub surface (`onLinkClicked`/`emitLinkClicked`) — the shared internal event surface (FR-24/LD-26) the navigation layer (Epic 8) subscribes to.
- `shell-ui/src/components/editor/decorations/links.ts` — NEW. `// Implements FR-4`. `orgLinkDecorations()` CM6 `ViewPlugin` (underline `Decoration.mark`, bracket-hiding `Decoration.replace`, `mousedown` handler → `emitLinkClicked`); exports `classifyLink` + `ORG_LINK_CLASS` for tests/styling.
- `shell-ui/src/components/editor/decorations/links.test.tsx` — NEW. Vitest + happy-dom; underline mark, LinkClicked target+kind for each variant, bracket reveal on/off cursor-line, round-trip fidelity, selection-change recompute.
- `shell-ui/src/components/editor/editorMode.ts` — EDIT (shared, minimal): `pseudoWysiwygDecorations()` returns `[orgLinkDecorations()]`.
- `shell-ui/src/styles/editor.css` — `.cm-org-link` underline + accent color via existing `--org-border-focus`.

## Tasks & Acceptance

**Execution:**
- [x] `events.ts` — `LinkKind`, `LinkClicked`, `onLinkClicked`/`emitLinkClicked`.
- [x] `links.ts` — link-scanning ViewPlugin, `classifyLink`, click emission.
- [x] `editorMode.ts` — append `orgLinkDecorations()` to the Pseudo-WYSIWYG set.
- [x] `editor.css` — `.cm-org-link` styling.
- [x] `links.test.tsx` — all variants, reveal toggle, round-trip, emission.

**Acceptance Criteria:**
- Given Story 4.2 + Pseudo-WYSIWYG active, when a buffer contains any link variant, then the link renders underlined via `Decoration.mark` — verified.
- And clicking emits `LinkClicked { target, kind }` — verified per variant.
- And `[[…]]` markers hidden off cursor-line, visible on cursor-line; source byte-identical — verified.

## Verification

See PR description / final report for exact numbers.

## Design Notes

- **Reveal-on-cursor-line:** the ViewPlugin recomputes its `DecorationSet` on `selectionSet` (plus `docChanged`/`viewportChanged`), building the bracket-hiding `Decoration.replace` ranges only for links whose line holds no selection endpoint. The underline mark is unconditional so the link stays visibly a link in both states.
- **Click emission surface:** the mark carries `data-org-link-target`/`data-org-link-kind`; the plugin's `mousedown` handler reads the nearest `.cm-org-link` and calls `emitLinkClicked` (no `preventDefault`, so cursor placement / editing still works and the reveal toggles). Navigation (Epic 8) subscribes via `onLinkClicked`.
- **No widgets:** hiding uses `Decoration.replace` (no `WidgetType`), so the LD-6 widget recipes (`eq`, `ignoreEvent`, composing) are N/A; nothing is duplicated into Zustand and the doc is never dispatched-to inside `update`.
