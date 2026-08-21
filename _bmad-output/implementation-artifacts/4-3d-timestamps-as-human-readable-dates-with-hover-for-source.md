---
title: 'Timestamps as human-readable dates with hover-for-source'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '73c8e28'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.2 shipped Raw mode, where org timestamps (`<2026-05-19 Tue 14:00>`, `[2026-05-18 Mon]`) are only syntax-highlighted source text. Epic 4's Pseudo-WYSIWYG mode (FR-4) needs those timestamps rendered as human-readable dates so calendar context is legible — without hiding or mutating the org source (FR-2 round-trip contract, FR-9 timestamp surface).

**Approach:** Add a self-contained CM6 decoration layer (`decorations/timestamps.ts`) that (a) locates timestamps in the visible ranges with the same delimiter vocabulary the Raw highlighter uses, (b) replaces each with a `Decoration.replace` widget showing a locale-formatted date plus the source clock time (e.g. "Tue, May 19 · 14:00"), (c) styles active `<…>` vs inactive `[…]` stamps distinctly via `--org-*` tokens, and (d) reveals the exact raw source in a CM6 `hoverTooltip` after a >300ms dwell. The layer is appended to the Pseudo-WYSIWYG / Split extension set in `editorMode.ts`; Raw stays decoration-free. The full org timestamp grammar is NOT reimplemented in TS — it lives in `orgsidian-parser/src/semantic/timestamp.rs`; this module extracts only the two display fields (date + optional clock time) and carries the source bytes verbatim.

## Boundaries & Constraints

**Always:**
- Source stays byte-identical — decorations are `Decoration.replace` widgets that never mutate the buffer (FR-2). The exact source slice is carried into the widget (`data-org-timestamp-raw`, `aria-label`) and the tooltip.
- Reuse the Epic 2 semantic layer's ownership of the timestamp grammar; do NOT re-parse repeaters / delays / ranges / weekday validation in TS. TS extracts only `YYYY-MM-DD` + optional `H:MM[-H:MM]` for display.
- Weekday is computed from the date via the locale formatter (UTC-anchored), independent of the source day-name (org display sugar the parser does not model, and which may be stale after a hand-edit). The unmodified source is one hover away.
- LD-6 recipes: `WidgetType.eq()` shallow-equals by source content (raw + active kind); non-interactive widget returns `ignoreEvent() === true`; no `view.dispatch` from the widget.
- Colors via the `--org-*` token vocabulary, applied through a co-located `EditorView.theme` (no new tokens, no `tokens.css` / LD-58 gate churn).
- Own the new file `decorations/timestamps.ts`; touch the shared `editorMode.ts` with the smallest possible edit (append one extension to the Pseudo-WYSIWYG set).
- Module carries `// Implements FR-4` as its first line (4.2 TS traceability convention).

**Ask First:**
- Rendering `<…>--<…>` timestamp ranges as a single fused widget (currently each half renders as its own widget with the literal `--` shown between them — consistent with the parser's own half-split `parse_one`).

**Never:**
- Re-parsing the org timestamp grammar in TS; mutating source from a decoration; a Rust round-trip on every viewport update (decorations must render synchronously).
- Sibling decoration layers (headings 4.3a, TODO pills 4.3b, tags 4.3c, checkbox 4.3e, links 4.3f), the date-picker (4.8), or mode-switcher wiring (4.5).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Active stamp w/ time | `<2026-05-19 Tue 14:00>` | replace widget "Tue, May 19 · 14:00", `cm-org-timestamp-active` | N/A |
| Inactive stamp, no time | `[2026-05-18 Mon]` | replace widget "Mon, May 18", `cm-org-timestamp-inactive` | N/A |
| Clock range | `<2026-06-10 Wed 10:00-11:00>` | "Wed, Jun 10 · 10:00-11:00" (range kept verbatim) | N/A |
| Stale source day-name | `<2026-05-19 Mon 14:00>` | "Tue, May 19 · …" (computed weekday self-corrects) | N/A |
| Impossible date | `<2026-13-40 Xxx>`, `[2026-02-30]` | no widget — raw source left visible | `formatTimestamp` → `null` |
| Caret touches stamp | selection overlaps `[from,to]` | widget suppressed, raw source revealed for editing | N/A |
| Hover >300ms over stamp | pointer rests on rendered widget | tooltip shows exact raw source | source-fn returns `null` off-stamp |
| Raw mode | any of the above | NO timestamp widget (layer excluded from Raw) | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/decorations/timestamps.ts` — NEW. `// Implements FR-4`. `timestampDecorations(): Extension` (ViewPlugin + `hoverTooltip` + `EditorView.theme`); `formatTimestamp`, `timestampTooltipSource`, `TimestampWidget`, `ORG_TIMESTAMP_CLASS`, `TIMESTAMP_HOVER_MS` exported for tests. Locates stamps in visible ranges, skips those the caret touches, guards impossible dates, formats UTC-anchored via `Intl.DateTimeFormat`.
- `shell-ui/src/components/editor/decorations/timestamps.test.tsx` — NEW. 15 Vitest + happy-dom cases: locale format (with/without time, ranges), stale-day-name self-correction, impossible-date null, active/inactive distinct classes, byte-identity, caret-reveal, tooltip raw source, `WidgetType.eq` by source, `ignoreEvent`.
- `shell-ui/src/components/editor/editorMode.ts` — one-line wiring: `pseudoWysiwygDecorations()` now returns `[timestampDecorations()]` (was `[]`). Raw path unchanged → still decoration-free.

## Tasks & Acceptance

**Execution:**
- [x] `timestamps.ts` — replace-widget ViewPlugin, hover-for-source tooltip, `--org-*` theme; grammar boundary + weekday policy documented.
- [x] `timestamps.test.tsx` — 15 cases covering every AC + adversarial edges.
- [x] `editorMode.ts` — append the timestamp layer to the Pseudo-WYSIWYG / Split set.

**Acceptance Criteria:**
- Given Story 4.2 + Pseudo-WYSIWYG active, a buffered timestamp renders as a `Decoration.replace` widget with a locale-formatted date + time — verified by `timestamps.test.tsx` ("replaces active and inactive stamps with formatted-date widgets", `formatTimestamp` cases).
- Hover for >300ms displays a tooltip with the raw source — `hoverTooltip({ hoverTime: 300 })`; source verified by "returns a tooltip with the exact raw source".
- Active vs inactive visually distinct — distinct `cm-org-timestamp-active/-inactive` classes + `data-org-timestamp`, verified.
- Source preserved byte-identical (FR-2) — `doc.toString()` unchanged, verified.

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass (exit 0).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings).
- `cargo test --workspace --locked` — pass (unchanged; TS-only story).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + lingui + vite; 2015 modules).
- `pnpm --filter shell-ui test` — 55 passed (40 prior + 15 new).
- `pnpm --filter shell-ui i18n:check` — pass (no catalog drift; no user-facing strings — dates come from `Intl`).

## Design Notes

- **Weekday is computed, not echoed.** The AC example "Mon, May 19" copies the source day-name, but 2026-05-19 is a Tuesday; the parser treats day-names as unmodeled display sugar. "Locale-formatted" therefore means the weekday is derived from the date (self-correcting for hand-edited stamps). The exact source, day-name included, is always available via the hover tooltip and `aria-label`.
- **Timezone safety.** Dates are constructed with `Date.UTC(...)` and formatted with `timeZone: "UTC"`, so a runner's local timezone can never shift the rendered calendar day. Clock times are kept verbatim from source (no am/pm reinterpretation).
- **No atomic trap.** A `Decoration.replace` widget is skipped whenever a selection range overlaps the stamp, so the caret is never trapped and the raw source is editable — preserving source-position fidelity (feeds Story 4.3g).
- **No manual listeners / leak-free.** The widget attaches no DOM listeners (`ignoreEvent() === true`); the tooltip is managed entirely by CM6's `hoverTooltip`. Both are torn down with the view, keeping the StrictMode double-mount and memory-soak contracts intact.
- **Grammar boundary.** Ranges (`<…>--<…>`) currently render as two adjacent half-widgets with the literal `--` between them (mirrors the parser's `parse_one` half-split); a fused range widget is deferred (Ask-First).
