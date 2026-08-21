---
title: 'Implement Split editor mode'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: '52f8fcd'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 4.2 (Raw) and 4.3a–4.3f (the Pseudo-WYSIWYG decoration layer) each render ONE view of an `.org` buffer. Epic 4's third Editor Mode — **Split** (FR-3) — must show the SAME buffer in a 50/50 surface: a Raw source view (left) beside a Pseudo-WYSIWYG view (right), with scroll synced, edits from either pane applied atomically to the shared buffer, and the Split choice persisted per-file via Story 4.2's `editor-prefs.json`.

**Approach:** Realize the shared buffer with the canonical CM6 "split view" recipe — two `EditorView`s seeded from the same document whose `dispatchTransactions` FORWARD every change to the sibling (tagged with a sync annotation so a forward never echoes back). The two states therefore hold byte-identical documents at all times; selection is intentionally not forwarded so each pane keeps its own caret. Left pane = `modeExtensions("raw")` (highlight only), right pane = `modeExtensions("pseudoWysiwyg")` (the full decoration layer). Scroll is mirrored with an equality-guarded DOM scroll listener (self-terminating, no feedback loop). The surface is a plain imperative factory (`SplitEditor.ts`) driven by the `Editor` host inside its ONE StrictMode-safe async load chain; crossing the Split boundary rebuilds the surface and hands the LIVE document over so the switch neither reloads from disk nor drops unsaved edits. Persistence reuses Story 4.2's typed `commands.setEditorMode(mode, path)` unchanged.

## Boundaries & Constraints

**Always:**
- Split shares ONE logical buffer across both panes: an edit dispatched in either pane updates the other atomically (change forwarding), per-pane cursors stay independent. This is the CM6-idiomatic realization of "panes SHARE one `EditorState`" (LD state-ownership boundary) — two states kept byte-identical by forwarding.
- CM6 remains the sole owner of the open file's buffer — never duplicated into Zustand, never persisted separately from the `.org` file.
- Keep Story 4.1's StrictMode-safe lifecycle intact: the surface is created in `useEffect` and destroyed in its cleanup; the `disposed` guard spans every async await; `destroy()` tears down BOTH views + wrapper exactly once (no leak across a double-mount). `ref` stays a plain prop (no `forwardRef`).
- Raw ↔ Pseudo-WYSIWYG still reconfigure the single view in place through the `Compartment` (no rebuild); only crossing the Split boundary rebuilds — handing the live doc across (no reload, no lost edits, fidelity untouched, well under the 200ms mode-switch budget).
- Colors via the `--org-*` token vocabulary (the pane divider uses `--org-border-default`).

**Ask First:**
- Any change to the `set_editor_mode` / `get_editor_mode` Rust commands or the `editor-prefs.json` schema — Split reuses Story 4.2's persistence verbatim.

**Never:**
- The mode switcher UI (Story 4.5), keybindings (4.6–4.7), or the date picker (4.8).
- A CM6 `MergeView` / diff surface — Split is two synced editors over one buffer, not a diff.
- Forwarding selection between panes (would couple the carets) or reloading the buffer from disk on a mode switch (would drop unsaved edits).
- Mounting the editor into a live route/screen — the tests are this story's only consumer.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Open persisted Split | file whose stored mode is `split` | two-view 50/50 surface built directly (no single-view flash, no reload); both panes show the source | falls back to default mode on read failure (Story 4.2 path) |
| Edit in left pane | type/insert in Raw pane | change applied to both buffers atomically; right pane doc identical | N/A |
| Edit in right pane | toggle/insert in Pseudo-WYSIWYG pane | change forwarded to left buffer atomically | N/A |
| Scroll a pane | scroll left pane | right pane scroll offset mirrors; no infinite feedback | equality guard self-terminates |
| Switch INTO Split | `setMode("split")` from single view with unsaved edits | surface rebuilt; live doc handed over (no `openFile`); edits survive in both panes; `setEditorMode("split", path)` persisted | persist failure caught, in-session switch unaffected |
| Switch OUT of Split | `setMode("raw"\|"pseudoWysiwyg")` | both split views torn down; single view rebuilt from the live doc | idempotent teardown |
| StrictMode double-mount | dev double-invoke of the effect in Split | exactly one surface survives → 2 views created, both destroyed on unmount | `disposed` guard + idempotent `destroy()` |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/SplitEditor.ts` -- NEW. `//! Implements FR-3`. `createSplitEditor({parent, doc, baseExtensions}) → SplitSurface { primaryView, secondaryView, destroy }`. Two views over one doc; `syncDispatch` forwards changes (sync annotation loop-breaker, preserves `userEvent`); `bindScrollSync` equality-guarded scroll mirror; idempotent `destroy()`.
- `shell-ui/src/components/editor/SplitEditor.test.ts` -- NEW. Factory-level: 50/50 two-pane build, Raw-left/Pseudo-right extension split, atomic edit forwarding both directions, per-pane cursor, scroll mirror, idempotent teardown.
- `shell-ui/src/components/editor/Editor.tsx` -- localized change: extracted `baseEditorExtensions`; `splitRef` + `buildSurface`/`teardownSurface` helpers; `setMode` fast-path (Compartment reconfigure) for raw↔pseudo, rebuild-with-live-doc-handover across the Split boundary; effect + cleanup route through the helpers.
- `shell-ui/src/components/editor/editorMode.ts` -- comment-only: clarifies `case "split"` now mirrors Pseudo-WYSIWYG for single-view callers; the real 50/50 surface lives in `SplitEditor.ts`.
- `shell-ui/src/components/editor/Editor.test.tsx` -- extended: persisted-Split open, switch-into-Split (live buffer carried, no reload, persisted), edits write through from either pane, StrictMode two-view leak check, switch-out-of-Split.

## Tasks & Acceptance

**Execution:**
- [x] `SplitEditor.ts` — two-view shared-buffer factory (change forwarding, scroll sync, idempotent teardown).
- [x] `Editor.tsx` — surface abstraction; Split-boundary rebuild with live-doc handover; single-view fast-path preserved.
- [x] `editorMode.ts` — clarify the `split` case (comment only; no behavior change).
- [x] `SplitEditor.test.ts` + `Editor.test.tsx` — factory + host coverage of every AC and adversarial edge (StrictMode double-view leak, atomic edits both directions, scroll sync, no-reload switch, persistence).

**Acceptance Criteria:**
- Given Stories 4.2 + 4.3, when Split mode is selected, the surface is split 50/50 — Raw source left, Pseudo-WYSIWYG right — over the SAME buffer — verified by `SplitEditor.test.ts` + `Editor.test.tsx`.
- Scroll position is synced between panes — verified (scroll mirror test).
- Edits in either pane update the underlying buffer atomically — verified (forwarding tests, both directions, via the host and the factory).
- Split preference persists per file via Story 4.2's `editor-prefs.json` — verified (`setEditorMode("split", path)` through the typed client on switch).

## Verification

**Commands:**
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass (0 warnings; no Rust changed).
- `cargo test --workspace --locked` — pass (no Rust changed; export_bindings green).
- `pnpm --filter shell-ui build` — pass (tsc strict + specta regen + i18n + vite).
- `pnpm --filter shell-ui test` — 121 passed (editor 16 in Editor.test.tsx + 7 in SplitEditor.test.ts).
- `pnpm --filter shell-ui i18n:check` — pass.

## Design Notes

- **Two states, one buffer:** CM6 has no single `EditorState` object literally shared by two views (each view owns its state field). The canonical split recipe forwards changes between views tagged with a `syncAnnotation`, so the two docs stay byte-identical while carets stay independent. This is the faithful realization of the LD "panes SHARE one `EditorState`" invariant.
- **`dispatchTransactions`, not `dispatch`:** the single-transaction `dispatch` view config is deprecated in CM6 6.43; the factory uses `dispatchTransactions(trs, view)` and iterates.
- **Scroll sync without a reentrancy flag:** setting the sibling only when the offset differs both avoids redundant writes and self-terminates the mirror (the sibling's own listener finds the offsets already equal and writes nothing back) — no `requestAnimationFrame`/flag bookkeeping to leak.
- **Imperative factory over a React component:** the `Editor` host already owns the one StrictMode-safe async load chain and the `EditorHandle`. Making Split a factory the host drives (rather than a second async React subtree) keeps a single load chain, keeps the existing tests' timing intact, and confines the leak-safety contract to one place (`destroy()` in the effect cleanup).
- **Mode-switch boundary:** raw↔pseudo stays a Compartment reconfigure (no rebuild). Only crossing into/out of Split needs a different DOM surface; that rebuild reads the live `view.state.doc` and seeds the new surface from it — so it never re-reads disk and never drops unsaved edits, and stays far under the 200ms budget.
