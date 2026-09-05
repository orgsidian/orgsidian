---
title: 'Ship dark + light default themes (WCAG AA)'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: 'ec04842'
review_loop_iteration: 0
github_issue: 58
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Story 1.17 scaffolded the LD-58 WCAG AA hard CI gate with a *single* `shell-ui/src/themes/tokens.css` carrying both `:root` (light) and `.dark` values as a structural minimum -- explicitly deferring the real architecture step 3 file layout (`tokens.css` + `dark.css` + `light.css`), the instant `document.body.dataset.theme` switch mechanism, and the Settings → Appearance UI to "Story 6.7". Today there is no theme toggle anywhere in the app, no `dark.css`/`light.css`, and the `.dark`-class selector convention (inherited from the shadcn scaffold) is not reachable by any user action.

**Approach:** Split the Story 1.17 scaffold into the three-file layout: `tokens.css` keeps the vocabulary header + convention doc + a `:root` block (the structural/pre-JS default, values unchanged from Story 1.17 = light), `light.css` restates the same palette under the explicit `body[data-theme="light"]` selector (symmetric with `dark.css`, so "Light" is a real targetable DOM state, not an implicit absence-of-override), and `dark.css` carries the (renamed-only, values unchanged) dark override under `body[data-theme="dark"]`. A new session-only preference store, `themes/themeMode.ts` (mirrors `components/editor/keybindings/keymapMode.ts`'s idiom exactly: module-level singleton, `get`/`set`/`onChange`/`__resetForTests`), resolves `dark` / `light` / `system` to a concrete `dark`/`light` value (via `matchMedia('(prefers-color-scheme: dark)')` for "system", tracked live) and writes it to `document.body.dataset.theme` -- the literal AC mechanism. `styles/app.css`'s Tailwind `@custom-variant dark` and the shadcn oklch `.dark {}` block are re-pointed at the same `body[data-theme="dark"]` selector so there is exactly one theme-switch mechanism app-wide, not two to keep in sync. `contrast.test.ts` (Story 1.17's LD-58 gate #1) is extended, not replaced: it now concatenates all three files and asserts across three selector blocks instead of two. A new `AppearanceSettings.tsx` component (Settings → Appearance: Light / Dark / System default, native radios in a `<fieldset>`) is mounted on the `/today` placeholder route, following the exact `KeybindingsSettings`/`VaultPicker` placeholder-hosting convention already established there.

## Boundaries & Constraints

**Always:**
- Theme switch is literally `document.body.dataset.theme = "dark" | "light"` (never the string `"system"` -- that preference is always resolved to a concrete value first, so `tokens.css`/`light.css`/`dark.css` only ever handle two selectors).
- Reuse and extend the Story 1.17 `contrast.test.ts` + axe-core gate; do not invent a parallel gate. All `--org-*` fg/border tokens keep their `@pair-role` + `@pair-bg` annotations (LD-58 convention, unchanged).
- Every declared theme value in this story stays hex (no OKLch) so the existing `parseHex`/`relativeLuminance`/`contrastRatio` machinery needs no changes.
- Match the `keymapMode.ts` session-only-preference idiom exactly (no `tauri-plugin-store` reach from the frontend for a global, non-per-file preference; no new state library — `shell-ui` has none).
- `--org-*` token vocabulary; colocated Vitest tests; native HTML semantics for a11y (fieldset/legend, native radios) over ARIA roles where the platform already provides the semantics.

**Ask First:**
- Any change to the Story 1.17 pair-metadata annotation convention itself (`@pair-role` / `@pair-bg` syntax) -- extending it (e.g., an "exempt" tier for decorative tokens) would be a larger, separate change.
- Adding any new external dependency (none needed -- `next-themes` is already an unused-for-theming devDependency from the shadcn scaffold, but it structurally cannot target `document.body` — see Design Notes).

**Never:**
- No full FR-22 vocabulary buildout (per-headline-level foregrounds, per-TODO-state accents, tag/link/property accents, state colors) -- out of this story's literal AC (body-text + UI-chrome contrast + instant switch + Settings toggle); `styles/editor.css`'s existing fallback-`var()` pattern is unaffected and continues to serve those needs.
- Do NOT touch `sprint-status.yaml`, `deny.toml`, or any Rust crate (this story is shell-ui/CSS only).
- Do NOT un-`fixme` any of the Story 1.17 `e2e/a11y/*.spec.ts` scaffolds -- the real Settings surface still doesn't exist (this story hosts `AppearanceSettings` on the `/today` placeholder, same as `KeybindingsSettings`/`VaultPicker`).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Explicit "Dark" selection | user clicks the Dark radio | `document.body.dataset.theme = "dark"` instantly; `dark.css`'s `body[data-theme="dark"]` block overrides `tokens.css`'s `:root` | N/A |
| Explicit "Light" selection | user clicks the Light radio | `document.body.dataset.theme = "light"`; `light.css`'s block applies (same values as `:root`) | N/A |
| "System default" selection, OS is dark | `matchMedia('(prefers-color-scheme: dark)').matches === true` | resolves to `"dark"`, writes it to the DOM | `matchMedia` unavailable (non-browser env) → falls back to `"light"` |
| "System default" selected, OS preference changes live | OS flips light↔dark while the app is open | the `change` listener re-resolves and re-applies the DOM attribute + notifies subscribers | only reacts while preference === "system"; an explicit choice does not track OS changes (that's the point of overriding it) |
| Repeated identical `setThemePreference` call | same value as current | no-op: no DOM write, no subscriber emit | N/A |
| Contrast gate | `contrast.test.ts` reads `tokens.css` + `light.css` + `dark.css` | exactly 3 selector blocks parsed (`:root`, `body[data-theme="light"]`, `body[data-theme="dark"]`), all 15 fg/border pairs ≥ their WCAG floor | a missing/renamed selector fails the block-count assertion loudly |
| Cold start (fresh app load) | no prior interaction | preference defaults to `"system"` (mirrors `keymapMode.ts`'s cold-start-reset rule); applied to the DOM at module-import time, before first paint | N/A |
| A throwing `onThemeChange` subscriber | one listener throws | isolated via try/catch + `console.error`; other subscribers still run | mirrors `keymapMode.ts`'s isolation behavior |

</frozen-after-approval>

## Code Map

- `shell-ui/src/themes/tokens.css` -- MODIFY. Removes the Story 1.17 `.dark {}` block (moved to `dark.css`); keeps `:root` (light values, unchanged) + the updated header doc describing the 3-file split. `body-text`/`large-text`/`ui-chrome` pair annotations unchanged.
- `shell-ui/src/themes/light.css` -- NEW. `body[data-theme="light"] { … }`, restating `tokens.css`'s `:root` palette with the same `@pair-role`/`@pair-bg` annotations.
- `shell-ui/src/themes/dark.css` -- NEW. `body[data-theme="dark"] { … }`, the Story 1.17 `.dark` values carried over verbatim (selector renamed only).
- `shell-ui/src/themes/themeMode.ts` -- NEW. `ThemePreference`/`ResolvedTheme` types; `getThemePreference`/`getResolvedTheme`/`setThemePreference`/`onThemeChange`/`__resetThemeModeForTests`; module-load side effect applies the cold-start default + wires a live `matchMedia` change listener.
- `shell-ui/src/themes/themeMode.test.ts` -- NEW. Cold-start default, instant DOM write, no-op on repeat, subscriber notify/unsubscribe/throw-isolation, live system-preference tracking (via `vi.resetModules()` + a stubbed `matchMedia`).
- `shell-ui/src/themes/contrast.test.ts` -- MODIFY. Reads + concatenates all three theme files; asserts 3 selector blocks (`:root`, `body[data-theme="light"]`, `body[data-theme="dark"]`) instead of 2 (`:root`, `.dark`); doc updated.
- `shell-ui/src/styles/app.css` -- MODIFY. Imports `light.css` + `dark.css`; `@custom-variant dark` re-pointed at `body[data-theme="dark"]`; the shadcn oklch `.dark {}` block renamed to `body[data-theme="dark"]` (same mechanism, single source of truth).
- `shell-ui/src/components/settings/AppearanceSettings.tsx` -- NEW. Settings → Appearance panel: Light / Dark / System default via native radios in a `<fieldset>`; `aria-live="polite"` "Currently applied" line.
- `shell-ui/src/components/settings/AppearanceSettings.test.tsx` -- NEW. Renders all 3 options, selection applies instantly + checks the right radio, aria-live region reports the resolved theme, a11y structure (fieldset + 3 radios), reflects a change made via the shared store from elsewhere.
- `shell-ui/src/routes/_layout/today.tsx` -- MODIFY. Mounts `<AppearanceSettings className="mt-8" />` below `KeybindingsSettings`, same placeholder-hosting comment convention as the existing `VaultPicker`/`KeybindingsSettings` mounts.
- `shell-ui/src/main.tsx` -- MODIFY. Side-effect `import "./themes/themeMode"` (applies the cold-start theme before the first render, alongside the existing `app.css` import).

## Tasks & Acceptance

**Execution:**
- [x] Split `tokens.css` into `tokens.css` + `light.css` + `dark.css` per the architecture step 3 file layout; update `app.css` imports + selectors (Tailwind `@custom-variant` + shadcn `.dark` block) to the `body[data-theme]` mechanism.
- [x] `themeMode.ts`: session-only preference store (mirrors `keymapMode.ts`), resolves `system` via `matchMedia`, writes `document.body.dataset.theme` instantly, live-tracks OS changes while `system` is selected.
- [x] Extend `contrast.test.ts` (Story 1.17 gate) to read all 3 theme files and assert 3 selector blocks; verified 15/15 pairs pass WCAG AA on both themes.
- [x] `AppearanceSettings.tsx` + colocated test; mounted on the `/today` placeholder route per the established convention.
- [x] Colocated `themeMode.test.ts`.

**Acceptance Criteria:**
- Given Epic 1 closed, when the themes are committed, then `shell-ui/src/themes/{tokens.css, dark.css, light.css}` declare the `--org-*` CSS variable vocabulary per architecture step 3. *(All three files present; each declares the LD-58-gated `--org-bg-*`/`--org-fg-*`/`--org-border-*` vocabulary.)*
- And theme switching is instant (`document.body.dataset.theme = "dark"`). *(`themeMode.ts`'s `setThemePreference` writes it synchronously; no reload, no re-mount -- a CSS custom-property cascade re-evaluation. Tested.)*
- And contrast ratios for body text and primary UI chrome meet WCAG AA on both themes -- verified by the Story 1.17 LD-58 contrast-matrix Vitest test on the `--org-*-fg`/`--org-*-bg` pairs AND by the Story 1.17 axe-core gate on every `@a11y`-tagged Playwright scenario. *(`contrast.test.ts` extended to 3 selector blocks, 15/15 pairs pass; `pnpm a11y` green -- the 6 `@a11y` scaffolds still report fixme/skipped, unchanged, since their real surfaces don't exist yet.)*
- And `tokens.css` declares the pair-role metadata required by Story 1.17's contrast test (body-text pairs / large-text pairs / UI-chrome pairs). *(Unchanged convention, now also carried by `light.css` + `dark.css`.)*
- And Settings → Appearance allows toggling between dark / light / system-default. *(`AppearanceSettings.tsx`, tested: 3 native radio options, instant apply, `aria-live` status line.)*

## Design Notes

- **Why `document.body`, not `document.documentElement` (`<html>`).** The AC's literal mechanism is `document.body.dataset.theme`. `next-themes` (already an unused-for-theming runtime dependency, pulled in by the shadcn `sonner.tsx` scaffold) was evaluated first per project convention, but its `attribute` prop can only target `<html>` or another explicit element -- its own FAQ states body-targeting is unsupported. Rather than fight the library's design to hit a literal one-line AC, this story ships a small custom store (`themeMode.ts`) mirroring the already-established `keymapMode.ts` idiom, and leaves `next-themes`/`sonner.tsx` untouched (out of `--org-*` scope per Story 1.17's "shadcn tokens are an internal implementation detail" ruling).
- **Why `light.css` duplicates `tokens.css`'s `:root` values instead of staying empty.** `body[data-theme="dark"]` overriding `:root` already produces correct light behavior with no `light.css` content at all (CSS cascade). `light.css` restates the palette anyway so "Light" is a real, symmetric, targetable theme file matching `dark.css`'s shape -- per the architecture step 3 file list requiring all three files to "declare the vocabulary" -- rather than an empty placeholder that would read as an oversight.
- **Why the wider FR-22 vocabulary (headline levels, per-TODO-state accents, tag/link/property, state colors) isn't added here.** Adding them as bare `--org-*` declarations without `@pair-role`/`@pair-bg` annotations would fail the Story 1.17 "every non-bg token has a pair-role" gate (regression); giving them real annotations requires deciding contrast pairing for values that are today used as *background* swatches for small badges (`org-todo-badge--*`), not text-on-background pairs -- a design decision beyond this story's literal AC. `styles/editor.css`'s existing `var(--org-accent-todo, var(--org-border-focus))` fallback pattern is untouched and keeps working exactly as before. Flagged here as a decision-grade scope call for review, not silently assumed.
- **Why theme preference is session-only (no persistence across a restart).** Mirrors `keymapMode.ts`'s explicit precedent: no `tauri-plugin-store` reach from the frontend for a global (non-per-file) preference, since no such command exists yet, and cold start deliberately lands back on "system" (the OS choice) rather than a remembered explicit pick. Persistence is out of this story's AC, so it is not added; cross-restart recall is a natural candidate for a future Settings-persistence story once a global-preference Tauri command exists.

## Verification

**Commands:**
- `pnpm --filter shell-ui test` -- expected: all green.
- `pnpm --filter shell-ui run test:contrast` -- expected: the LD-58 gate green across all 3 selector blocks.
- `pnpm a11y` (root) -- expected: contrast green + the 6 `@a11y` Playwright scaffolds report fixme/skipped (unchanged).
- `pnpm --filter shell-ui build` -- expected: `tsc` + `vite build` clean.
- `pnpm --filter shell-ui run i18n:check` -- expected: no new extracted strings (plain-string convention, matches `KeybindingsSettings`).
- `cargo build --workspace --offline` -- expected: clean (no Rust changes in this story; requires `git submodule update --init --recursive` for `orgsidian-parser`'s `tree-sitter-org` grammar, a pre-existing repo-setup step unrelated to this story).

**Result (2026-09-05, post-code-review):** `pnpm --filter shell-ui test` -- 273/273 GREEN across 24 files. New this story: `themeMode.test.ts` (10 cases), `AppearanceSettings.test.tsx` (7 cases), `themeIntegrity.test.ts` (5 cases: `app.css` switch-mechanism pinning + `:root`/`light.css` palette-equality); `contrast.test.ts` extended to 21 cases (3 selector blocks) from Story 1.17's 15. `pnpm --filter shell-ui run test:contrast` -- 21/21 GREEN (3 selector blocks × 5 pairs + 6 structural/sanity checks); all 15 fg/border pairs independently re-verified in Python against the shipped hex values (body-text ≥4.5:1, large-text/ui-chrome ≥3.0:1 on both themes). `pnpm a11y` -- GREEN (contrast + the 6 `@a11y` scaffolds still fixme/skipped, unchanged). `pnpm --filter shell-ui build` -- GREEN (`tsc` + `vite build`, no new TypeScript errors). `pnpm --filter shell-ui run i18n:check` -- GREEN (no new extracted strings). `cargo build --workspace --offline` -- GREEN after `git submodule update --init --recursive` (needed once per fresh worktree; no Rust files touched by this story). `pnpm-lock.yaml`/`package.json` unchanged (no new dependency).

## Spec Change Log

- 2026-09-05 -- Implemented. `themes/{tokens.css, light.css, dark.css}` (architecture step 3 file split), `themes/themeMode.ts` (session-only dark/light/system preference store, instant `document.body.dataset.theme` switch, live system-preference tracking), `contrast.test.ts` extended to 3 selector blocks, `styles/app.css` re-pointed to the `body[data-theme]` mechanism (Tailwind `@custom-variant` + shadcn `.dark` block), `AppearanceSettings.tsx` (Settings → Appearance) mounted on the `/today` placeholder route. All AC wired and green offline. Status → review.
- 2026-09-05 -- Code-review fixes (no-brainers): `applyToDom` now guards `document.body` (not just `document`); the live `matchMedia` `change` handler gained the same no-op contract as `setThemePreference`; `AppearanceSettings` `<legend>` → "Theme" (drops the doubled "Appearance" screen-reader announcement); added `themes/themeIntegrity.test.ts` pinning the `styles/app.css` switch mechanism (`@custom-variant` + shadcn block → `body[data-theme="dark"]`, no legacy `.dark`) and asserting `:root`/`light.css` palette equality; added `themeMode` explicit-choice-ignores-OS-change test and `AppearanceSettings` "(from system)" qualifier test; hardened an `AppearanceSettings` test to `getElementById`; corrected doc counts + the `next-themes` dependency label + the session-only rationale. Overstated "no unstyled flash" comments softened to match the deferred-module reality (see Open Questions below).

## Open Questions (code-review, for human)

- **Residual cold-start background flash for dark-OS "system" users.** The theme is applied at module-import time (before React renders content), so no rendered UI ever paints in the wrong theme. But the import is a deferred ES module: a dark-OS user can briefly see the empty `<body>` in the `:root` light default before it runs. Fully eliminating this last background-only flash needs an inline blocking `<script>` at the top of `index.html`'s `<body>` (the standard `next-themes`-style guard). Deferred as a scope call: there is no content to flash yet, and it adds a small duplicate of the resolve logic to `index.html`. Confirm whether that inline guard should ship in this story or a follow-up.
- **`--org-bg-elevated` has no contrast pairing and equals `--org-bg-canvas` in light.** `--org-bg-elevated` (`#ffffff` in light = identical to canvas; `#1a1a1a` in dark) is declared but never carries `@pair-role`/`@pair-bg`, so the LD-58 gate does not cover text rendered on it (e.g. `KeybindingsReference`'s badges). Pre-existing from the Story 1.17 scaffold, not introduced here; flagged for a future palette story that gives elevated surfaces a gated fg pair (and, in light, real elevation vs. canvas).

