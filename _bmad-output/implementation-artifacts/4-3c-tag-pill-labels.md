---
title: 'Tag pill labels'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '73c8e28'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 shipped Raw mode: org source with syntax-highlight tokens only, and an empty Pseudo-WYSIWYG decoration layer behind a `Compartment`. Epic 4's Pseudo-WYSIWYG mode needs the first of its inline widgets — **tag pills** (FR-4): a headline's trailing `:tag:` / `:tag1:tag2:tag3:` suffix must render as one pill per tag so tag taxonomy is scannable, while the `.org` buffer stays byte-identical (the FR-2 round-trip contract).

**Approach:** Add a mode-scoped CM6 decoration layer (`decorations/tags.ts`) — a `ViewPlugin` that scans visible headline lines for a trailing tag block and emits one `Decoration.replace` per tag. Each replace covers the tag's leading `:` through the next tag's leading `:` (the final tag absorbing the trailing `:`), so every colon falls inside some pill's replaced range and is visually hidden while remaining in the source. The pill widget renders the bare tag name, styled via `--org-accent-tag`. Wire the layer into the Pseudo-WYSIWYG / Split extension set in `editorMode.ts` (the single shared touch-point); Raw mode never includes the set, so it stays decoration-free.

## Boundaries & Constraints

**Always:**
- Pills apply only to a *headline*'s *trailing* tag block (org tags are a headline suffix); a mid-body `:foo:` is never decorated.
- `Decoration.replace` is presentational — the buffer is never mutated; `view.state.doc.toString()` stays byte-identical (FR-2). Colon delimiters are hidden by the widget, preserved in source.
- Follow the LD-6 CM6 recipes: `WidgetType.eq()` shallow-compares by source range (`from`/`to`) plus the rendered label; the layer rebuilds only on `docChanged` / `viewportChanged`.
- Keep Story 4.1/4.2 intact: the layer is additive behind the existing `Compartment`; Raw = highlight only.
- Styling via the `--org-*` token vocabulary (`--org-accent-tag`); it is a token *usage* in `editor.css` (resolving to an existing FR-22 token), kept out of `tokens.css` / the LD-58 contrast gate exactly like Story 4.2's `cm-org-*` colors.
- The FR-4 module carries `// Implements FR-4` as its first line (traceability convention; 4.2's TS convention — the enforcing `tests/traceability.rs` harness lands with Story 4.3a).

**Ask First:**
- Introducing a dedicated, gated `--org-accent-tag` declaration into `tokens.css` (would enter the LD-58 contrast gate) — Story 6.7 refines the palette.
- A click-to-edit `TagPillEditor` React widget under `components/org/` — that is a later story; 4.3c renders (read-only) pills only.

**Never:**
- Mutating the source on render, or on any non-user action.
- Re-implementing the org tokenizer — the tag-character class mirrors `orgLanguage.ts`.
- Decorating non-headline lines, or mid-headline `:…:` that is not the trailing suffix.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Single tag | `* Buy groceries :errand:` | one pill "errand"; colons hidden; doc byte-identical | N/A |
| Multi-tag | `* Plan trip :work:travel:urgent:` | three pills; every colon absorbed; doc byte-identical | N/A |
| Adjacent many | `* TODO Ship it :a:b:c:d:e:` | five pills; round-trip exact | N/A |
| Non-headline `:foo:` | `see :notatag: inline` | no pill (only headline trailing tags) | N/A |
| Mid-headline `:foo:` | `* Note :aside: with words` | no pill (not a trailing suffix) | N/A |
| Empty / malformed | `** Another :: not a tag` | no pill | N/A |
| Edit adds a tag | append ` :new:` to a headline | layer rebuilds on `docChanged`; pill appears | N/A |
| Raw vs Pseudo | same source | Raw: zero pills; Pseudo-WYSIWYG: pills present | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/decorations/tags.ts` -- NEW. `// Implements FR-4`. `tagPillDecorations()` (a `ViewPlugin` building `Decoration.replace` per tag over visible headline lines), `TagPillWidget` (`eq()` by source range + label), and `TAG_PILL_CLASS`. Tag grammar mirrors `orgLanguage.ts` (`[A-Za-z0-9_@%#]`), anchored to the trailing block of a headline.
- `shell-ui/src/components/editor/decorations/tags.test.tsx` -- NEW. Mounts a real CM6 view; asserts pill-per-tag, hidden-colon rendering, byte-identical round-trip, headline-only scope, edit rebuild, and Raw-vs-Pseudo mode gating.
- `shell-ui/src/components/editor/editorMode.ts` -- the sole shared edit: `pseudoWysiwygDecorations()` now returns `[tagPillDecorations()]` (Raw still excludes the set).
- `shell-ui/src/styles/editor.css` -- `.cm-org-tag-pill` styled via `--org-accent-tag` (a usage resolving to `--org-fg-muted`; kept out of `tokens.css` / the LD-58 gate).

## Tasks & Acceptance

**Execution:**
- [x] `decorations/tags.ts` — tag-pill `ViewPlugin`, `TagPillWidget`, block tiling that hides all delimiters.
- [x] `editorMode.ts` — include the layer in the Pseudo-WYSIWYG / Split set.
- [x] `editor.css` — `.cm-org-tag-pill` via `--org-accent-tag`.
- [x] `decorations/tags.test.tsx` — 9 tests (single/multi/adjacent tags, round-trip, headline-only, mid-headline exclusion, empty, edit rebuild, mode gating).

**Acceptance Criteria:**
- Given Story 4.2 + Pseudo-WYSIWYG mode active, when a headline contains a `:tag:` or `:tag1:tag2:tag3:` suffix, then each tag renders as a `Decoration.replace` pill widget styled via `--org-accent-tag` — verified by `tags.test.tsx`.
- And the colon delimiters are visually hidden but preserved in the source (byte-identical round-trip, FR-2) — verified (pill text carries no colon; `doc.toString()` equals source).
- Module carries `// Implements FR-4` traceability.

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings).
- `cargo test --workspace --locked` — pass (43 test binaries, 0 failures; no Rust touched by this story).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + i18n compile + vite).
- `pnpm --filter shell-ui test` — 49 passed (5 files); +9 new in `tags.test.tsx`.
- `pnpm --filter shell-ui i18n:check` — pass (no catalog diff; no user-facing strings added).

## Design Notes

- **Delimiter tiling (the sharp edge):** an `N`-tag block has `N+1` colons, so one colon is always "extra". Tiling each replace as `[leading-colon_i, leading-colon_{i+1})` and extending the last to the block end makes every colon fall inside exactly one pill's replaced range — all hidden, none double-covered (`RangeSetBuilder` requires sorted, non-overlapping ranges).
- **Headline-only, trailing-only:** detection guards on `/^\*+\s/` and an end-anchored `(^|\s):…:$` block, so emphasis-like `:foo:` in body text and mid-headline `:foo:` are never mistaken for a tag suffix.
- **Round-trip is structural:** `Decoration.replace` overlays a widget without changing document text; the pill renders only the bare name, so the colons stay in the buffer and the file is byte-identical — verifiable in `cat`/Emacs (the UX trust contract).
