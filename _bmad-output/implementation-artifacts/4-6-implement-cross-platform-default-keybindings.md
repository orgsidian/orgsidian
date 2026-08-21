---
title: 'Implement cross-platform default keybindings'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '5e00f30'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 4.1–4.5 give the editor its surfaces and a mode switcher, and 4.8 added a Schedule/Deadline picker behind interim chords (`Mod-Alt-s`/`Mod-Alt-d`). But the daily org-mode actions (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out) have no coherent, documented, cross-platform default keymap (FR-5), and there is no in-app place to discover the chords. The chords are also scattered — the mode switch lives hard-coded in `ModeSwitcher`, schedule/deadline in `schedule.ts` — with no single source of truth, and the mode-switch chord has a spec discrepancy (epics `Cmd/Ctrl+Alt+M` vs UX spec `Cmd/Ctrl+Shift+M`) left unreconciled by 4.5.

**Approach:** Add `shell-ui/src/components/editor/keybindings/default.ts` as the SINGLE SOURCE OF TRUTH: a `DEFAULT_KEYMAP` array of typed `KeymapAction`s (id, label, description, category, platform-agnostic `Chord`, `live`/`reserved` status, binding owner, and — for live editor actions — a CM6 `run`). Platform detection (`tauri-plugin-os`) selects Cmd vs Ctrl through `resolveIsMac` + `formatChord`; `chordToCodeMirror` renders CM6's platform-agnostic `Mod-` form. `buildDefaultKeymap` turns the editor-owned live actions into `KeyBinding[]` (and reserved ones into documented no-ops that surface a "coming soon" toast) which the `Editor` host wires once. `ModeSwitcher` now reads its chord from the map (`findAction`/`matchesChord`/`formatChord`) instead of hard-coding it. A new `KeybindingsReference` panel at Settings → Keybindings renders every action, live and reserved, from the same map. Two new keyboard commands — `cycleTodoAtCursor` (todoBadges) and `toggleCheckboxAtCursor` (checkboxes) — give the click-driven widgets a keyboard path over the SAME mutation surface/userEvent tag.

**Reconciliation (this story's charter):** the mode-switch chord is fixed at **`Cmd/Ctrl+Alt+M`** — the authoritative epics AC and the already-shipped 4.5 chord — keeping the whole `Cmd/Ctrl+Alt+…` family (mode/schedule/deadline/TODO/checkbox) consistent and changing no live behavior. The UX-spec `Shift` variant is dropped.

## Boundaries & Constraints

**Always:**
- The chord map in `keybindings/default.ts` is the ONE source of truth; the CM6 keymap, `ModeSwitcher`, and the reference panel all read it (never re-declare a chord).
- Platform detection via `tauri-plugin-os`, guarded (`try/catch` → non-mac fallback) so plain `vite dev` / Vitest never throw.
- Keyboard TODO-cycle and checkbox-toggle ride the SAME `view.dispatch` + `input.cycle-todo`/`input.toggle-checkbox` userEvent tags as the pill/checkbox click (FR-24 / LD-26 — one mutation surface, never a private path); no dispatch while `view.composing`.
- Reserved actions (feature ships in a later epic) declare a documented chord + `reservedNote` and NO `run` — no fake implementation; `buildDefaultKeymap` binds them to a no-op that consumes the chord and surfaces a "coming soon" toast.
- Colors via `--org-*` tokens; labels are plain strings (Lingui macro deferral, matches VaultPicker / ModeSwitcher).

**Ask First:**
- Changing the reconciled mode-switch chord, or the schedule/deadline chords (they are now the FINAL native keymap, identical to 4.8's interim values so behavior is unchanged).
- Adding a save/agenda/capture/clock implementation — those are reserved here and owned by later epics.

**Never:**
- Re-bind find/replace (owned by `@codemirror/search`'s `searchKeymap`, wired in `sourceFidelity`) or the mode switch (owned by `ModeSwitcher`'s global window listener) inside `buildDefaultKeymap` — that would double-bind; the map still documents them for the panel.
- Invent behavior for reserved actions.
- Restructure the `Editor` host beyond swapping the interim planning keymap for the central keymap.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Platform = macOS | `platform()==="macos"` | `resolveIsMac()===true`; chords render `⌘…` | N/A |
| Platform = win/linux | `platform()` non-mac | `resolveIsMac()===false`; chords render `Ctrl+…` | N/A |
| Off-Tauri | `platform()` throws | `resolveIsMac()===false` (fallback) | guarded try/catch |
| CM6 chord render | `{mod,alt,key:"t"}` | `Mod-Alt-t` | N/A |
| Human chord render | same, mac / non-mac | `⌘⌥T` / `Ctrl+Alt+T` | N/A |
| Chord map complete | DEFAULT_KEYMAP | every AC action (save/agenda/capture/TODO/schedule/deadline/clock in-out) present + unique CM6 key | uniqueness asserted |
| TODO cycle key | cursor on/under `* TODO …` | advances keyword via `input.cycle-todo` tag | falls through (false) in preamble |
| TODO cycle (no kw) | `* Bare` | inserts first sequence keyword | N/A |
| Checkbox toggle key | cursor on `- [ ]` line | `- [X]` via `input.toggle-checkbox` | falls through (false) off a checkbox line |
| Reserved chord | press `Mod-s` (save) | no-op + `onReserved` "coming soon" toast; chord consumed | no fake write |
| Mode switch chord | Ctrl+Alt+M (non-mac) / Cmd+Alt+M (mac) | cycles mode (via ModeSwitcher, reading the map) | Ctrl+Alt+M inert on macOS |
| Reference panel | render | every action listed with per-platform chord; reserved rows badged "Coming soon" | N/A |
| Panel a11y | render | labelled section, per-category `<table>` w/ sr-only caption + scoped headers; status by text not color | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/keybindings/default.ts` — NEW. `DEFAULT_KEYMAP` single source of truth; `Chord` type; `resolveIsMac` / `formatChord` / `chordToCodeMirror` / `matchesChord` / `findAction`; `buildDefaultKeymap({onReserved, actions})` (live→run, reserved→no-op toast, skips `search`/`global` owners; accepts an explicit `actions` list — the Story 4.7 Emacs seam).
- `shell-ui/src/components/editor/keybindings/default.test.ts` — NEW. Map completeness + chord uniqueness, platform resolution incl. off-Tauri fallback, `formatChord`/`chordToCodeMirror`/`matchesChord` (Cmd-vs-Ctrl, Option-compose via `code`, missing/extra modifier), `buildDefaultKeymap` (editor-only emission, reserved no-op→onReserved, schedule/deadline emit routing, explicit-actions seam).
- `shell-ui/src/components/settings/KeybindingsReference.tsx` — NEW. Settings → Keybindings panel; per-category tables of action→chord read from the map; platform-aware; "Coming soon" badge on reserved rows; a11y (labelled section, sr-only captions, scoped headers). Accepts `actions`/`title`/`isMac` (Emacs-panel seam).
- `shell-ui/src/components/settings/KeybindingsReference.test.tsx` — NEW. Lists every action, per-platform chord text (Ctrl/⌘), reserved badge presence/absence, a11y structure, category grouping.
- `shell-ui/src/components/editor/decorations/todoBadges.ts` — ADD `cycleTodoAtCursor(view)`: keyboard cycle for the headline at/above the cursor, reusing `resolveTodoSequence`/`nextState`/`cycleTodoState` (shared tag); inserts first keyword when none; returns false in the preamble. `+ tests`.
- `shell-ui/src/components/editor/decorations/checkboxes.ts` — ADD `toggleCheckboxAtCursor(view)`: delegates to `toggleCheckboxAt` at the cursor (shared tag); returns false off a checkbox line. `+ tests`.
- `shell-ui/src/components/editor/Editor.tsx` — swap the interim `planningKeymap()` wiring for `keymap.of(buildDefaultKeymap({ onReserved: toast }))`; keeps `defaultKeymap` + `sourceFidelity` (search) intact.
- `shell-ui/src/components/editor/ModeSwitcher.tsx` — reads the switch-mode `Chord` from the map; `resolveIsMac`/`formatChord`/`matchesChord` replace the inlined platform + hard-coded chord logic (behavior identical: same global listener, same `Cmd/Ctrl+Alt+M`).
- `shell-ui/src/components/editor/schedule.ts` — doc-only: note `planningKeymap` is superseded as the wiring path by the central keymap (identical chords), retained + tested as a helper.
- `shell-ui/src/routes/_layout/today.tsx` — mount `<KeybindingsReference>` alongside `VaultPicker` (placeholder settings host, matching the VaultPicker precedent) until the real Settings flow (Epic 6/11).

## Tasks & Acceptance

**Execution:**
- [x] `keybindings/default.ts` — central chord map + platform detection + CM6 keymap builder; Emacs seam.
- [x] `KeybindingsReference.tsx` — Settings → Keybindings panel reading the map.
- [x] `cycleTodoAtCursor` / `toggleCheckboxAtCursor` — keyboard command forms sharing the click paths' mutation surface.
- [x] `Editor.tsx` — wire the central keymap once (reserved → "coming soon" toast).
- [x] `ModeSwitcher.tsx` — source its chord from the central map.
- [x] Tests: keymap unit, reference-panel render/a11y, the two new commands.

**Acceptance Criteria:**
- `keybindings/default.ts` declares the default chord set with platform-detected Cmd vs Ctrl via `tauri-plugin-os` — verified (`default.test.ts` platform + format tests; `resolveIsMac` guarded fallback).
- Every daily org-mode action (save, agenda, capture, TODO cycle, schedule, deadline, clock in/out) has a documented default chord — verified (map-completeness test asserts each id present with a chord; live actions wired, reserved actions documented with `reservedNote`, chords unique).
- An in-app reference panel at Settings → Keybindings lists all documented chords with their actions — verified (`KeybindingsReference.test.tsx` asserts every action row, per-platform chord text, reserved badges, a11y structure); mounted on the settings host route.

## Verification

**Commands:** (numbers in the Verification section of the final report)
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `pnpm --filter shell-ui build`
- `pnpm --filter shell-ui test`
- `pnpm --filter shell-ui i18n:check`

## Design Notes

- **Mode-switch reconciliation (resolved here):** fixed at `Cmd/Ctrl+Alt+M` — the authoritative epics AC and the chord already shipped in 4.5 — so no live behavior changes and the `Cmd/Ctrl+Alt+…` family stays consistent. The UX-spec `Shift` variant is intentionally dropped; the discrepancy comment left in 4.5's `ModeSwitcher` is removed now that the map is the source of truth.
- **Single source of truth + Emacs seam (Story 4.7):** every consumer reads `DEFAULT_KEYMAP`. `buildDefaultKeymap` and `KeybindingsReference` both accept an explicit `actions` list over the SAME `KeymapActionId` union, so `keybindings/emacs.ts` drops in with no structural change. When Emacs mode is active its `keymap.of(...)` is added with higher CM6 precedence than the native set so the active keymap wins on conflicts (4.7's "active keymap takes precedence" AC).
- **Reserved vs live — no fake implementations:** there is no buffer write-back command yet (`open_file` is read-only), and agenda/capture/clock in-out ship in Epics 6/7/8; those actions are `reserved` (documented chord, `reservedNote`, no `run`). `buildDefaultKeymap` binds them to a no-op that consumes the chord and calls `onReserved` (the host shows a "coming soon" toast) so the chord map is complete and stable without inventing behavior. Reserved chords deliberately avoid the Chromium/WebKit devtools/reload combos (`Ctrl/Cmd+Shift+I/J/C`, `Cmd+Alt+I`) so the reserved no-ops never steal a devtools shortcut.
- **Owner separation avoids double-binding:** find/replace (`Mod-f`) is bound by `@codemirror/search`'s `searchKeymap` (already wired in `sourceFidelity`) and the mode switch by `ModeSwitcher`'s global window listener (fires even when the editor is unfocused). Both are documented in the map (`owner: "search"`/`"global"`) for the reference panel but skipped by `buildDefaultKeymap`.
- **Keyboard = click, one surface:** `cycleTodoAtCursor`/`toggleCheckboxAtCursor` reuse the existing widget commands and their userEvent tags, so the keyboard path is byte-for-byte the same mutation as the click (FR-2 round-trip contract, FR-24/LD-26 shared surface). Both return `false` when the cursor is not on an applicable line so the chord falls through.
- **i18n deferral:** panel + map labels are plain strings (matches VaultPicker / ModeSwitcher); `i18n:check` stays clean.
- **Settings mount:** no Settings router exists yet; the panel is mounted on the placeholder `today` route alongside `VaultPicker`, exactly as `VaultPicker` itself is hosted until Epic 6/11 build the real Settings flow.
