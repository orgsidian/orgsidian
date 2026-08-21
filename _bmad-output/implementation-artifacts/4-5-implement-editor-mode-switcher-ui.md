---
title: 'Implement Editor Mode switcher UI'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: 'bdc1826'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 4.2 (Raw), 4.3a–4.3g (Pseudo-WYSIWYG) and 4.4 (Split) give the `Editor` host three modes plus a `mode`/`setMode` handle that switches between them while preserving buffer state and persisting the per-file choice. What is missing (FR-3) is the *discoverability* layer: a visible control that shows the active mode and switches it, plus a default keyboard chord that cycles the three modes — so mode switching is reachable without knowing the handle exists.

**Approach:** Add `shell-ui/src/components/editor/ModeSwitcher.tsx`, a controlled segmented control (three toggle buttons, `aria-pressed` marks the active mode) that reflects a `mode` prop and reports selections through an `onModeChange` callback. It owns the `Cmd/Ctrl+Alt+M` chord (a single, StrictMode-idempotent `window` keydown listener) that cycles Raw → Pseudo-WYSIWYG → Split, with the primary modifier chosen per platform via `tauri-plugin-os` (`platform() === "macos"` → Cmd, else Ctrl). The switcher is a pure UI surface — buffer ownership, in-place vs rebuild switching, the <200ms budget and per-file persistence all already live in `Editor.setMode`, so the consumer simply routes `onModeChange` → `editorRef.current.setMode`. To let a parent mirror the active mode reactively (so the switcher tracks the host's async-loaded persisted mode and every subsequent switch), `Editor` gains one additive optional prop, `onModeChange`, emitted wherever mode is set. No restructuring of the host, no new dependency, no Zustand store introduced in this story.

## Boundaries & Constraints

**Always:**
- The switcher is CONTROLLED: the active mode is a prop, the single source of truth stays upstream (epic-4 state-ownership boundary — CM6 owns the buffer, mode is UI state). Buffer state is never duplicated into the switcher.
- Switching goes through the existing `Editor.setMode` handle — raw↔pseudo reconfigure the live view in place, into/out of Split rebuilds carrying the live doc; both preserve unsaved edits and persist per-file via `commands.setEditorMode`. The switcher adds no parallel persistence or buffer path.
- Colors via the `--org-*` token vocabulary (`--org-bg-surface`/`--org-bg-elevated`/`--org-border-default`/`--org-border-focus`/`--org-fg-default`/`--org-fg-muted`); Tailwind-utility-first.
- The chord uses `event.code === "KeyM"` (not `event.key`) so macOS Option-compose (`µ`) still matches, ignores `event.repeat`, and registers exactly one listener under StrictMode.

**Ask First:**
- Any change to the `mode`/`setMode` handle contract or the `set_editor_mode`/`get_editor_mode` commands — the switcher reuses them verbatim.
- Reconciling the chord discrepancy (see Design Notes) into a shared keymap — deferred to Story 4.6.

**Never:**
- Introduce Zustand / `viewStore.ts` in this story (would touch the frozen lockfile and restructure host ownership); the controlled-prop + `onModeChange` mirror is the minimal wiring.
- Wrap the segment labels in Lingui macros — the Vitest transform (esbuild) does not run the Lingui SWC plugin, so macro-wrapped strings break component tests; UI-string extraction is deferred repo-wide (matches VaultPicker / IndexScanProgress).
- Restructure `Editor.tsx` beyond the single additive `onModeChange` prop.
- Mount the switcher into a live route/screen — the tests are this story's only consumer (as with 4.4).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Render active mode | `mode="pseudoWysiwyg"` | three segments; Pseudo-WYSIWYG `aria-pressed=true`, others false | N/A |
| Click a segment | click `data-mode='split'` | `onModeChange("split")` fired once | N/A |
| Chord (non-mac) | Ctrl+Alt+M | `onModeChange(nextMode(mode))` | ignored without Alt / primary |
| Chord (macOS) | Cmd+Alt+M | fires; Ctrl+Alt+M does NOT fire on macOS | platform read guarded → non-mac fallback |
| Option-compose | `code=KeyM`, `key=µ`, Cmd+Alt | matches on `code` | N/A |
| Auto-repeat | chord with `repeat=true` | ignored (no runaway cycling) | N/A |
| StrictMode | double-mount | exactly one listener → fires once per chord | idempotent add/remove |
| Persisted-mode open | host loads `split` | switcher's Split segment pressed | host fallback path (4.2) |
| Switch every mode | click raw→split→pseudo with an unsaved edit | edit survives all transitions incl. into/out of Split; no `openFile` reload | host guarantees |
| Restart round-trip | select Raw, remount | switcher opens on persisted Raw | host reload path |
| 5000-line switch | click raw on a 5000-line doc | completes < 200ms (in-place reconfigure) | N/A |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/ModeSwitcher.tsx` — NEW. Controlled segmented control (`role="group"`, per-mode `aria-pressed` toggle buttons, `--org-*` tokens); `Cmd/Ctrl+Alt+M` cycle via a single StrictMode-idempotent `window` keydown listener (refs for latest mode/callback so the listener binds once); `platform()` (tauri-plugin-os) picks Cmd vs Ctrl, guarded for non-Tauri. Exports `nextMode` for the cycle contract.
- `shell-ui/src/components/editor/ModeSwitcher.test.tsx` — NEW. Controlled-control coverage (render/active/click/a11y), chord coverage (non-mac, macOS meta-not-ctrl, `code` match, no-Alt/repeat ignore, StrictMode-once, unmount cleanup), and Editor-host integration (persisted-mode reflect, buffer preserved across every transition incl. Split, per-file persistence, restart round-trip, <200ms on 5000 lines).
- `shell-ui/src/components/editor/Editor.tsx` — localized additive change: optional `onModeChange?` prop emitted on the initial persisted-mode load and on every `setMode`, via a latest-callback ref (no effect/handle dep churn). No change to buffer ownership or the switch mechanics.

## Tasks & Acceptance

**Execution:**
- [x] `ModeSwitcher.tsx` — segmented control + platform-aware chord cycle; controlled; `--org-*` styling; a11y (`role="group"`, `aria-pressed`, visible focus).
- [x] `Editor.tsx` — additive `onModeChange` notification so a parent mirrors the active mode.
- [x] `ModeSwitcher.test.tsx` — every AC + adversarial edges (StrictMode listener idempotency, Option-compose, auto-repeat, buffer preserved into/out of Split, persistence round-trip, 5000-line budget).

**Acceptance Criteria:**
- Given 4.2 + 4.3 + 4.4, invoking the switcher renders a segmented control showing the active mode (Raw / Pseudo-WYSIWYG / Split) — verified (render/active-state tests).
- Clicking a mode option switches the editor WITHOUT losing buffer state — verified (integration test types an unsaved edit and asserts it survives raw→split→pseudo, incl. crossing the Split boundary, with no `openFile` reload).
- `Cmd/Ctrl+Alt+M` cycles the three modes — verified (chord tests, macOS Cmd vs non-mac Ctrl, `code` match, repeat/no-Alt ignored).
- Mode switch completes <200ms on a 5000-line file — verified (5000-line switch test asserts elapsed < 200ms; raw↔pseudo is an in-place Compartment reconfigure — no reload/rebuild).
- Per-file mode preference persists across restarts — verified (persist-through-typed-client + remount round-trip tests).

## Verification

**Commands:** (numbers in the Verification section of the final report)
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `pnpm --filter shell-ui build`
- `pnpm --filter shell-ui test`
- `pnpm --filter shell-ui i18n:check`

## Design Notes

- **Chord discrepancy (flagged, not resolved here):** epics.md Story 4.5 AC says `Cmd/Ctrl+Alt+M`; the UX spec says `Cmd/Ctrl+Shift+M`. Per the task directive we follow the epics AC (**Alt**) and leave a code comment in `ModeSwitcher.chordHint` flagging the UX-spec discrepancy for reconciliation when the shared default keymap lands in Story 4.6.
- **Controlled, not self-owning:** the switcher deliberately does not read `Editor.mode` imperatively or hold its own mode state. It takes `mode` as a prop and reports `onModeChange`, so the source of truth stays where the epic-4 boundary places UI state. The one host change (`onModeChange`) is the minimal mechanism for a parent to mirror the host's async-loaded persisted mode reactively — without it the switcher would show the cold-start default while the host opened a file persisted as Split.
- **Why no Zustand yet:** epic-4 context names `viewStore.ts` (Zustand) as the eventual home of the current mode, but Zustand is not a dependency and adding it would touch the frozen lockfile and restructure host ownership — out of scope for a switcher-UI story. The controlled-prop shape is forward-compatible: a later `viewStore` becomes the `mode`/`onModeChange` source with no switcher change.
- **Listener binds once:** latest `mode` and `onModeChange` live in refs updated in an effect, so the keydown listener is registered on `[isMac]` only. StrictMode's add→remove→add nets exactly one listener; `event.repeat` guarding stops held-key runaway; `event.code` (not `event.key`) survives macOS Option-compose.
- **i18n deferral:** segment labels are plain strings, matching sibling components; the repo's Vitest transform does not run the Lingui SWC macro plugin, so `<Trans>` would break tests. `i18n:check` stays clean (no macro-wrapped strings to extract). Documented inline in the component.
