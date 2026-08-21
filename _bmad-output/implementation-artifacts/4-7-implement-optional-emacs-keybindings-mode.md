---
title: 'Implement optional Emacs keybindings mode'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '32f67e9'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 4.6 built the native cross-platform default keymap as a single source of truth (`keybindings/default.ts`) and deliberately left a seam for an alternate (Emacs) chord set — but no Emacs set exists, there is no way to enable it, and org-mode users with Emacs muscle memory have no path to their familiar chords (FR-5). The seam anticipates a swap where "the active keymap takes precedence".

**Approach:** Add `shell-ui/src/components/editor/keybindings/emacs.ts` declaring `EMACS_KEYMAP` in the SAME `KeymapAction[]` shape over the SAME `KeymapActionId` union as `DEFAULT_KEYMAP`, so `buildDefaultKeymap` and `KeybindingsReference` consume it with no structural change. Chords are the REAL Emacs/org-mode bindings and use two additive `Chord` fields the native set never touches — `ctrl` (a literal `C-`, the same key on every platform) and `then` (multi-stroke, e.g. `C-x C-s`); `chordToCodeMirror` renders these to CM6's space-separated prefix-key form and `formatChord` renders them in Emacs notation (`C-x C-s`). A tiny global preference store `keybindings/keymapMode.ts` (in-memory, session-scoped, event emitter mirroring `schedule.ts`) holds the active mode; the `Editor` host wires the active keymap behind a CM6 `Compartment` (wrapped in `Prec.high`) and subscribes to the store, so toggling reconfigures the live view(s) in place — no reload, no lost edits. `KeybindingsReference` gains `headingId`/`description`/`active` props (presentational, unchanged defaults) and a new `KeybindingsSettings` composer renders the opt-in toggle plus BOTH reference panels (native + "Emacs mode"). Gaps are documented in `docs/user-guide/emacs-keybindings.md`.

**Reconciliations (this story's charter):**
- **Persistence scope:** the epics/PRD frame keybindings as a preference, but the UX spec Principle 3 (echoed in epic-4-context "Default landing state") makes native keybindings an ABSOLUTE cold-start default and states "semantic state resets". Emacs mode is therefore **session-scoped, in-memory** (resets to native on cold start), not persisted across restart. This is also why it does not reach for `tauri-plugin-store` from the frontend (forbidden by the Editor's boundary) — cross-restart recall is the future "Reopen last session" opt-in.
- **AC example `C-c C-c` cycle TODO:** reconciled to the faithful org-mode bindings (Fidelity Lighthouse). `C-x C-s` save is verbatim; `C-c C-t` (`org-todo`) cycles TODO, and `C-c C-c` (`org-ctrl-c-ctrl-c`) toggles the checkbox — the real org behavior — so both chords are present and faithful.

## Boundaries & Constraints

**Always:**
- `EMACS_KEYMAP` is another view over the SAME source-of-truth types; every consumer (`buildDefaultKeymap`, `KeybindingsReference`) reads it unchanged.
- Live Emacs actions REUSE the native command functions (`cycleTodoAtCursor`, `toggleCheckboxAtCursor`, `emitPlanningRequested`) — one mutation surface, same userEvent tags (FR-24 / LD-26); asserted by an identity test.
- The active keymap is a clean SWAP behind a `Compartment`: native and Emacs sets never coexist, so the active set always wins (the "active keymap takes precedence" AC). `Prec.high` makes it beat CM6's baseline `defaultKeymap`.
- Toggling Emacs mode reconfigures in place — never rebuilds the view or reloads the buffer (buffer-state AC), in single AND both Split panes.
- Reserved Emacs actions mirror the native reserved set exactly (documented chord + `reservedNote`, no `run`, "coming soon" no-op) — no fake implementations.
- Colors via `--org-*` tokens; labels are plain strings (Lingui deferral, matches siblings).

**Never:**
- Refactor 4.6's default map beyond the additive `Chord` fields (`ctrl`, `then`) the seam needs; native chord behavior is byte-unchanged (`mod`-only chords render exactly as before).
- Re-bind find (owned by `@codemirror/search`) or the mode switch (owned by `ModeSwitcher`'s global listener); Emacs mode swaps only the editor-owned CM6 keymap. Both keep their native chords in the Emacs set (documented; the idiomatic-Emacs gap is in the user guide).
- Invent behavior for reserved actions or persist the mode across restart.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior |
|----------|--------------|---------------------------|
| Multi-stroke CM6 render | `{ctrl,key:"x",then:{ctrl,key:"s"}}` | `chordToCodeMirror` → `Ctrl-x Ctrl-s` |
| Triple-stroke render | clock-in chord | `Ctrl-c Ctrl-x Ctrl-i` |
| Emacs human render | save chord, mac or not | `formatChord` → `C-x C-s` (platform-independent) |
| Native render unaffected | `{mod,alt,key:"t"}` | `⌘⌥T` / `Ctrl+Alt+T` (unchanged) |
| Emacs coverage | `EMACS_KEYMAP` | id parity with `DEFAULT_KEYMAP`; unique CM6 keys |
| Live vs reserved | each action | live editor → `run`; reserved → no `run` + `reservedNote` |
| Shared surface | cycleTodo/toggleCheckbox `run` | identical function reference to native map |
| Reserved multi-stroke chord | build `Ctrl-x Ctrl-s` | no-op → `onReserved("save")`, chord consumed |
| Toggle on | Settings switch → emacs | store emits; Editor reconfigures compartment; buffer intact |
| Toggle preserves buffer | edit then toggle | same view instance, `openFile` not re-called, edit survives |
| Split toggle | split mode + toggle | both panes reconfigured, neither rebuilt |
| Open while emacs on | mode=emacs at mount | surface seeds compartment from current mode (no rebuild) |
| Panel | render `KeybindingsSettings` | native + "Emacs mode" panels; Emacs chords in `C-…` notation |
| Active badge | toggle | "Active" text badge moves native ↔ emacs (text, not color) |
| Toggle a11y | render | `role="switch"`, `aria-checked`, `<label for>` name |
| Cold start | fresh load | native default (session reset) |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/keybindings/emacs.ts` — NEW. `EMACS_KEYMAP: readonly KeymapAction[]` — real Emacs/org-mode chords over the same union; live actions reuse the native command functions; reserved actions mirror the native reserved set; find/switchMode kept on native chords (un-remapped owners).
- `shell-ui/src/components/editor/keybindings/emacs.test.ts` — NEW. Coverage + id parity + unique CM6 keys, multi/triple-stroke render (`chordToCodeMirror` + `formatChord`), `buildDefaultKeymap(EMACS_KEYMAP)` (editor-only emission, reserved no-op→onReserved, schedule/deadline emit routing, shared-run identity).
- `shell-ui/src/components/editor/keybindings/keymapMode.ts` — NEW. Global session preference (`getKeymapMode`/`setKeymapMode`/`isEmacsMode`/`onKeymapModeChange`, `activeKeymap` selector, `__resetKeymapModeForTests`); in-memory, cold-start = native.
- `shell-ui/src/components/editor/keybindings/keymapMode.test.ts` — NEW. Default, switch, cold-start reset, subscribe/unsubscribe, no-emit-on-unchanged, throwing-subscriber isolation, selector.
- `shell-ui/src/components/editor/keybindings/default.ts` — EDIT (additive). `Chord` gains optional `ctrl` + `then`; `chordToCodeMirror` handles them (space-joined multi-stroke); `formatChord` renders Emacs notation for `mod`-less chords. Native (`mod`) chords unchanged. Seam comment updated.
- `shell-ui/src/components/settings/KeybindingsReference.tsx` — EDIT. `KeybindingsReference` gains `headingId`/`description`/`active` props (defaults unchanged). NEW `KeybindingsSettings` composer: opt-in toggle (persists via `keymapMode`) + native + "Emacs mode" panels with an "Active" badge.
- `shell-ui/src/components/settings/KeybindingsReference.test.tsx` — EDIT. Adds active-badge test + `KeybindingsSettings` suite (both panels, Emacs `C-…` notation, default/toggle active state, persistence, external-change reflection, switch a11y).
- `shell-ui/src/components/editor/Editor.tsx` — EDIT. Active keybindings behind a `Compartment` (`Prec.high`); `activeKeybindings(mode)` helper; subscribe to `keymapMode` and reconfigure live view(s) in place (single + both Split panes).
- `shell-ui/src/components/editor/Editor.test.tsx` — EDIT. Adds the Emacs-swap suite (reconfigure-not-rebuild, buffer/edit preserved, open-while-emacs-on, Split both-panes).
- `shell-ui/src/routes/_layout/today.tsx` — EDIT. Mounts `KeybindingsSettings` (settings host) in place of the bare `KeybindingsReference`.
- `docs/user-guide/emacs-keybindings.md` — NEW. Emacs chord table + gap register (reserved chords; find/switchMode idiom gaps; AC reconciliation; unbound org prefixes).

## Tasks & Acceptance

**Execution:**
- [x] `emacs.ts` — `EMACS_KEYMAP` over the 4.6 seam (multi-stroke `Chord` fields).
- [x] `keymapMode.ts` — global session preference + `activeKeymap` selector.
- [x] `default.ts` — additive `ctrl`/`then` + multi-stroke/Emacs rendering.
- [x] `Editor.tsx` — active-keymap swap behind a Compartment (precedence, buffer-preserving).
- [x] `KeybindingsReference.tsx` — `KeybindingsSettings` toggle + Emacs-mode section.
- [x] `docs/user-guide/emacs-keybindings.md` — chords + gaps.
- [x] Tests: Emacs keymap unit, keymap-mode store, panel/toggle, Editor swap.

**Acceptance Criteria:**
- Emacs mode enabled in Settings → `emacs.ts` declares the Emacs chord set covering save, agenda, capture, TODO cycle, schedule, deadline, clock in/out — verified (`emacs.test.ts` coverage + id parity; multi-stroke `C-x C-s`).
- Conflicts with the default Cmd/Ctrl shortcuts are resolved by the ACTIVE keymap taking precedence — verified (clean Compartment swap + `Prec.high`; `Editor.test.tsx` reconfigure-not-rebuild + open-while-emacs-on).
- The chord set is documented in the in-app reference panel under an "Emacs mode" section — verified (`KeybindingsReference.test.tsx` `KeybindingsSettings` suite: both panels, `C-…` notation).
- Any gap documented in `docs/user-guide/emacs-keybindings.md` — done (reserved chords, find/switchMode idiom gaps, AC reconciliation, unbound org prefixes).

## Verification

**Commands:** (numbers in the final report)
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `pnpm --filter shell-ui build`
- `pnpm --filter shell-ui test`
- `pnpm --filter shell-ui i18n:check`

## Design Notes

- **Emacs semantics in the `Chord` type.** Native chords carry the platform-primary `mod` (Cmd/Ctrl); Emacs chords carry a literal `ctrl` (`C-`, never Cmd) and may chain via `then` (multi-stroke). Because every native chord has `mod` and no Emacs chord does, `formatChord` cleanly partitions the two notations from one function — native renders `⌘…`/`Ctrl+…`, Emacs renders `C-x C-s`. `matchesChord` (ModeSwitcher's global listener) is untouched; it only ever sees the native switch-mode chord.
- **Session scope, not persistence.** See the Intent reconciliation: UX Principle 3 makes native the absolute cold-start default and resets semantic state. In-memory is therefore the spec-coherent mechanism and is fully testable (the test env exposes no `localStorage`).
- **One mutation surface.** Live Emacs actions reference the exact native `run` functions, proven by an identity assertion — the keyboard/Emacs path is byte-for-byte the same edit as the click and the native chord (FR-2 / FR-24 / LD-26).
- **Owner separation preserved.** Emacs mode reconfigures only the editor-owned keymap Compartment; find (searchKeymap) and the mode switch (global listener) keep their native chords, documented honestly in the panel with the idiomatic-Emacs gap recorded in the user guide.
- **Settings mount.** No Settings router exists yet; `KeybindingsSettings` mounts on the placeholder `today` route alongside `VaultPicker`, exactly as `KeybindingsReference` did in 4.6, until Epic 6/11 build the real Settings flow.
