# Story 1.17: Establish WCAG 2.1 AA hard CI gate

Status: ready-for-dev

## Metadata

github_issue: 17

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want three hard CI gates enforcing WCAG 2.1 AA — a contrast-matrix Vitest test at [`shell-ui/src/themes/contrast.test.ts`](shell-ui/src/themes/contrast.test.ts), 6 keyboard-only Playwright scaffold scenarios at [`shell-ui/e2e/a11y/`](shell-ui/e2e/a11y/) each invoking `@axe-core/playwright`, and a `pnpm a11y` orchestrator script wired into [`.github/workflows/pr.yml`](.github/workflows/pr.yml) at the reserved slot (line 176) — wired into the per-PR pipeline from day 1,
So that every UI-shipping story downstream (Stories 4.*, 6.*, 7.*, 8.*, 12.*) inherits the a11y floor by construction rather than retroactively (NFR-9 hard gate from v0.1 Alpha per [LD-58 architecture.md:1359-1385](_bmad-output/planning-artifacts/architecture.md#L1359-L1385) + [PRD §8 post-2026-05-20 reconciliation](_bmad-output/planning-artifacts/prd.md) + [UX spec Experience Principle 9](_bmad-output/planning-artifacts/ux-design-specification.md#L180)).

**Traces:** NFR-9, LD-58, LD-32, LD-51, FR-22.

## Acceptance Criteria

### AC1 — Create `shell-ui/src/themes/tokens.css` as the canonical LD-51 source with pair-role metadata.

- **NET-NEW directory** `shell-ui/src/themes/`. Currently does NOT exist (verified: `shell-ui/src/styles/app.css` is the only stylesheet; LD-51 work was deferred to a future story, now opens here as the minimum required for the LD-58 contrast gate).
- **NET-NEW file** `shell-ui/src/themes/tokens.css` (~80 lines) declaring `--org-*` CSS variables per the FR-22 vocabulary documented at [architecture.md:282-311](_bmad-output/planning-artifacts/architecture.md#L282-L311). This is the **canonical source** for `--org-*` tokens (LD-51 — [architecture.md:1299-1305](_bmad-output/planning-artifacts/architecture.md#L1299-L1305)).
- **FILE STRUCTURE** (mirror the FR-22 categorization verbatim; Story 6.7 will refine the palette values, Story 1.17 ships the structure + a minimum body-text + UI-chrome pair so the contrast gate has real input to assert on):
  ```css
  /*
   * Orgsidian design tokens — FR-22 vocabulary (LD-51 canonical source).
   * Story 1.17 ships the structural minimum required by the LD-58 contrast gate
   * (≥1 body-text pair + ≥1 UI-chrome pair per theme); Story 6.7 fills in the full
   * palette per the dark + light theme designs.
   *
   * PAIR-ROLE METADATA CONVENTION (LD-58 gate input):
   * Every fg/bg pair MUST declare its role via a `@pair-role: <role>` comment line
   * immediately preceding the fg token, where <role> ∈ {body-text, large-text, ui-chrome}.
   * Tokens without a declared pair role fail the LD-58 contrast gate (forces explicit
   * categorization rather than ad-hoc heuristics).
   *
   * @pair-role values:
   *   body-text  → ratio_required >= 4.5 (WCAG 2.1 SC 1.4.3 AA)
   *   large-text → ratio_required >= 3.0 (WCAG 2.1 SC 1.4.3 AA, ≥18pt or ≥14pt bold)
   *   ui-chrome  → ratio_required >= 3.0 (WCAG 2.1 SC 1.4.11 AA non-text contrast)
   */

  :root {
    /* === Backgrounds (FR-22) === */
    --org-bg-canvas: #ffffff;     /* main editor / surfaces */
    --org-bg-surface: #f7f7f7;    /* sidebars, panels */
    --org-bg-elevated: #ffffff;   /* dialogs, popovers */

    /* === Foregrounds (FR-22) === */
    /* @pair-role: body-text */
    --org-fg-default: #1a1a1a;
    /* @pair-role: body-text */
    --org-fg-muted: #4a4a4a;
    /* @pair-role: large-text */
    --org-fg-subtle: #6a6a6a;

    /* === Borders (FR-22) === */
    /* @pair-role: ui-chrome */
    --org-border-default: #d4d4d4;
    /* @pair-role: ui-chrome */
    --org-border-focus: #2563eb;
  }

  .dark {
    --org-bg-canvas: #0a0a0a;
    --org-bg-surface: #141414;
    --org-bg-elevated: #1a1a1a;

    /* @pair-role: body-text */
    --org-fg-default: #f5f5f5;
    /* @pair-role: body-text */
    --org-fg-muted: #b5b5b5;
    /* @pair-role: large-text */
    --org-fg-subtle: #888888;

    /* @pair-role: ui-chrome */
    --org-border-default: #2a2a2a;
    /* @pair-role: ui-chrome */
    --org-border-focus: #3b82f6;
  }
  ```
- **IMPORT WIRING**: `shell-ui/src/styles/app.css` must `@import "../themes/tokens.css";` as the first line BEFORE the existing `@import "tailwindcss";` so the `--org-*` variables are available to Tailwind's `@theme` block and to any component that consumes them. The existing shadcn baseline tokens (`--background`, `--foreground`, etc.) in `app.css` remain untouched — they are an **internal implementation detail** consumed by the shadcn primitives; LD-51 explicitly scopes the public theme API to `--org-*` only ([architecture.md:1304](_bmad-output/planning-artifacts/architecture.md#L1304): "semantic granularity (`--org-headline-h1-fg`, `--org-accent-todo`), never structural (`--org-color-blue-500`)").
- **VALUES ARE PROVISIONAL**: the hex values above are placeholders chosen to PASS the contrast gate today (`#1a1a1a` on `#ffffff` ≈ 17.4:1, `#4a4a4a` on `#ffffff` ≈ 9.7:1, `#6a6a6a` on `#ffffff` ≈ 6.0:1 — all well above 4.5:1 body-text floor). Story 6.7 will replace them with the designed Orgsidian palette. **DO NOT** treat these as design decisions; they are gate-input minimum scaffolding. Document this in a `/* PROVISIONAL palette — Story 6.7 will refine */` comment at the top of `tokens.css`.
- **PAIR-ROLE EXTRACTION**: the contrast test (AC2) extracts pairs by reading `@pair-role: <role>` comment lines that immediately precede a `--org-*` token declaration; the test pairs that token (as foreground) against the nearest preceding `--org-bg-*` token in the SAME selector block (`:root` or `.dark`) as background. **Spec the convention explicitly in the file header** so future contributors don't guess.
- **DO NOT** create `shell-ui/src/themes/dark.css` or `shell-ui/src/themes/light.css` referenced in the architecture tree at [architecture.md:260-261](_bmad-output/planning-artifacts/architecture.md#L260-L261) — those are downstream story scope (likely Story 6.7). Story 1.17 ships `tokens.css` only, with both themes inlined via `:root` + `.dark` selectors (mirroring the existing `app.css` pattern at lines 60-127).
- **DO NOT** ship the LD-51 tokens.test.ts snapshot test in this story (deferred to Story 12.2 per sprint-status). Story 1.17's contrast test is a separate, complementary gate.

### AC2 — Create `shell-ui/src/themes/contrast.test.ts` Vitest contrast-matrix test (LD-58 gate #1).

- **PREREQUISITE**: Vitest is NOT currently installed in [`shell-ui/package.json`](shell-ui/package.json) (verified: no `vitest` entry in `devDependencies` as of HEAD). Story 1.17 adds it.
- **VITEST INSTALL**: add to `shell-ui/package.json` `devDependencies`:
  - `"vitest": "^3.0.0"` (latest stable per [[feedback_version_policy]] — verify current latest with `pnpm view vitest version` before pinning; semver-minor pin).
  - `"@vitest/ui": "^3.0.0"` (optional but useful for local development) — **NOT REQUIRED**; omit if it would add license-allowlist drift (`pnpm licenses` will reveal). Default: omit, keep the dep tree lean.
  - `"jsdom": "^25.0.0"` (Vitest's default DOM environment; required even for pure-CSS-parsing tests because Vitest's `environment: 'node'` doesn't expose `document` — though this contrast test doesn't need a DOM, `jsdom` is a no-cost addition that future a11y unit tests will use).
- **NET-NEW file** `shell-ui/vitest.config.ts` (~25 lines):
  ```ts
  /// <reference types="vitest" />
  import { defineConfig } from 'vitest/config';

  /**
   * Vitest config — Story 1.17 (LD-58 a11y gate scaffold).
   *
   * Scope: unit/integration tests for shell-ui internals (themes, utilities, hooks).
   * E2E + axe-core scenarios run via Playwright (see playwright.config.ts).
   */
  export default defineConfig({
    test: {
      environment: 'jsdom',
      globals: false,
      include: ['src/**/*.test.{ts,tsx}'],
      exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
    },
  });
  ```
  **DO NOT** merge this into `vite.config.ts` via the `test` field — keep configs separate so the Tailwind/TanStack-Router/Lingui plugin chain in `vite.config.ts` doesn't pollute the test runtime (and so adding a Vitest plugin in the future doesn't conflict with the Vite build plugin order).
- **NET-NEW file** `shell-ui/src/themes/contrast.test.ts` (~150 lines). Top-level docstring:
  ```ts
  /**
   * Contrast-matrix test — LD-58 gate #1 (Story 1.17).
   *
   * Extracts every (--org-*-fg, --org-*-bg) pair from tokens.css per the
   * `@pair-role:` comment convention; computes the WCAG 2.1 relative-luminance
   * contrast ratio `(L1 + 0.05) / (L2 + 0.05)` per pair; asserts:
   *   - body-text  pairs ≥ 4.5:1 (WCAG 2.1 SC 1.4.3 AA)
   *   - large-text pairs ≥ 3.0:1 (WCAG 2.1 SC 1.4.3 AA)
   *   - ui-chrome  pairs ≥ 3.0:1 (WCAG 2.1 SC 1.4.11 AA non-text contrast)
   *
   * Implements NFR-9 / LD-58.
   */
  ```
- **PUBLIC API** the test exercises (live in the test file; no separate module):
  - `parseTokens(css: string): { selector: string; pairs: { fg: string; bg: string; role: 'body-text' | 'large-text' | 'ui-chrome'; fgName: string; bgName: string }[] }[]` — parses the CSS string, returning per-selector blocks (`:root`, `.dark`) with extracted pairs.
  - `relativeLuminance(rgb: { r: number; g: number; b: number }): number` — implements the WCAG sRGB linearization formula: each channel `c`, if `c <= 0.03928` then `c/12.92` else `((c + 0.055) / 1.055) ** 2.4`; weighted sum `0.2126*R + 0.7152*G + 0.0722*B`. **DO NOT** import a hex-to-luminance library; the formula is ~10 lines and adding a dep for it triggers a license-allowlist review (LD-37) for trivial gain. Reference: [WCAG 2.1 SC 1.4.3 Understanding](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html).
  - `contrastRatio(fg: string, bg: string): number` — parses hex (and `oklch(...)` if present per the existing `app.css` style — but `tokens.css` per AC1 uses hex only; if Story 6.7 introduces OKLch values, contrast.test.ts already handles it by parsing hex first and falling back to a clear error). **DO NOT** silently coerce OKLch to RGB via a guess — if an `oklch(...)` token appears in Story 1.17's `tokens.css`, fail the test loudly with a TODO message pointing at Story 6.7's expected color-space-conversion utility.
- **PARSING DISCIPLINE**:
  - Read `tokens.css` via Node's `fs.readFileSync(new URL('./tokens.css', import.meta.url), 'utf-8')` — synchronous I/O is correct in a test; do NOT introduce a `vite-plugin-import-css` or similar.
  - Split into selector blocks by matching `:root\s*\{[^}]*\}` and `\.dark\s*\{[^}]*\}` via regex; assert exactly 2 blocks present (regression net — if a theme is renamed or dropped, the test fails loud).
  - Within each block, scan line-by-line: when a line matches `/^\s*\/\*\s*@pair-role:\s*(body-text|large-text|ui-chrome)\s*\*\//`, capture the role; the NEXT line declaring `--org-*-fg: <value>;` becomes the foreground; the role is consumed (one-shot, doesn't apply to subsequent tokens). The background for each fg pair is the **nearest preceding `--org-bg-*` declaration** in the same block (track the most recent `--org-bg-*` seen during the line-by-line scan).
  - **TOKENS WITHOUT @pair-role**: every `--org-*-fg` or `--org-fg-*` token in a selector block MUST have an associated `@pair-role:` annotation. The test asserts: `for each --org-*fg* token, role !== undefined` — failure mode: `expect(unpaired).toEqual([])`. This is the LD-58 line 1367 "tokens without declared pair role in `tokens.css` metadata fail the gate" requirement.
- **TEST CASES** (Vitest `describe` / `it`):
  1. `parseTokens(...)` returns exactly 2 selector blocks (`:root` + `.dark`).
  2. Every fg token in each block has a `@pair-role` annotation (no `unpaired` array entries).
  3. Each body-text pair achieves `contrastRatio(fg, bg) >= 4.5`.
  4. Each large-text pair achieves `contrastRatio(fg, bg) >= 3.0`.
  5. Each ui-chrome pair achieves `contrastRatio(fg, bg) >= 3.0`.
  6. `relativeLuminance({r: 255, g: 255, b: 255})` === `1.0` (sanity check).
  7. `relativeLuminance({r: 0, g: 0, b: 0})` === `0.0` (sanity check).
  8. `contrastRatio('#000000', '#ffffff')` === `21.0` (the maximum possible WCAG ratio — sanity check).
- **NO EXCEPTIONS LIST**: Story 1.17 ships a minimum palette that passes cleanly. The "exception_clause + required_redundant_signals" pattern documented at [UX spec § Gate 1](_bmad-output/planning-artifacts/ux-design-specification.md#L2155-L2179) is deferred to Story 6.7 (where the DONE-strikethrough muted-text exception would land). **DO NOT** scaffold the exception-list machinery now — YAGNI.

### AC3 — Add `@axe-core/playwright` + Playwright as devDependencies; create `shell-ui/playwright.config.ts`.

- **PREREQUISITE**: Playwright is NOT currently installed in `shell-ui/package.json` (verified). `@axe-core/playwright` is pinned in the architecture stack-versions table at [architecture.md:193](_bmad-output/planning-artifacts/architecture.md#L193) as "latest stable; MIT". Story 1.7 license allowlist at [deny.toml:72-89](deny.toml#L72-L89) already includes `MIT` — **no `deny.toml` change required** (verified: MIT is the 1st entry in the `allow = [...]` list).
- **DEP ADDITIONS** to `shell-ui/package.json` `devDependencies`:
  - `"@playwright/test": "^1.50.0"` (latest stable per [[feedback_version_policy]] — verify with `pnpm view @playwright/test version` at implementation time; semver-minor pin). MIT.
  - `"@axe-core/playwright": "^4.10.0"` (latest stable; verify with `pnpm view @axe-core/playwright version`). MIT.
- **DO NOT** add `playwright` (the standalone driver package) separately — `@playwright/test` is the test-runner package that bundles the driver. `playwright` standalone is for non-test programmatic use (irrelevant here).
- **POST-INSTALL STEP**: Playwright requires browser-binary download via `pnpm exec playwright install chromium`. **DO NOT** add this as a `postinstall` script in `package.json` — it triggers on every `pnpm install` (including CI) and downloads ~150MB. Instead:
  - Add a manual script: `"e2e:setup": "playwright install chromium"` to `shell-ui/package.json`.
  - The CI workflow (AC5) invokes it as a discrete step before running e2e tests, with caching.
- **NET-NEW file** `shell-ui/playwright.config.ts` (~30 lines):
  ```ts
  /**
   * Playwright config — Story 1.17 (LD-58 a11y gates #2 + #3).
   *
   * Scope: keyboard-only scenarios + axe-core integration for the 6 primary
   * surfaces (Today Dashboard, Agenda, Editor, Quick Capture, Settings, Graph View).
   * Story 1.17 ships scaffolds (test.skip with TODO refs to downstream epics);
   * downstream stories (4.*, 6.*, 7.*, 8.*) fill in the assertions as their
   * surfaces ship.
   *
   * Implements NFR-9 / LD-58.
   */
  import { defineConfig } from '@playwright/test';

  export default defineConfig({
    testDir: './e2e',
    timeout: 30_000,
    fullyParallel: true,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    reporter: process.env.CI ? 'github' : 'list',
    use: {
      baseURL: 'http://localhost:1420',  // matches vite.config.ts default port
      headless: true,
      trace: 'on-first-retry',
    },
    projects: [
      { name: 'chromium', use: { browserName: 'chromium' } },
    ],
    webServer: {
      command: 'pnpm run dev',
      url: 'http://localhost:1420',
      reuseExistingServer: !process.env.CI,
      timeout: 60_000,
    },
  });
  ```
- **CHROMIUM ONLY** in Story 1.17. The architecture matrix at [architecture.md:521-528](_bmad-output/planning-artifacts/architecture.md#L521-L528) targets WebKit (macOS) + WebKitGTK (Linux) + WebView2 (Windows) for the FINAL Tauri app — but the per-PR a11y scaffold runs Playwright against the Vite dev server (web-rendered) on the CI runner, where chromium is the most stable and fastest choice. Cross-browser a11y verification is deferred to v1.0 (Story 13.5).
- **WEB-SERVER COUPLING**: `webServer.command: 'pnpm run dev'` boots the Vite dev server. The dev server's prebuild (`tsr generate && lingui compile && cargo test --locked --package orgsidian-shell-app --test export_bindings --quiet` per `shell-ui/package.json` line 8) IS triggered on `pnpm run dev`. This means the CI a11y step depends on the same prebuild as the `Build shell-ui` step (line 165 in `pr.yml`). **Acceptable**: the prebuild is fast (<10s on warm cache); the CI step ordering (Build → a11y) ensures the prebuild's outputs are cached when a11y runs.
- **DO NOT** boot the full Tauri shell (`pnpm tauri dev`). Tauri's window-manager won't run on CI's headless Linux runners without `xvfb` shim, and the LD-58 keyboard scenarios target the **web layer** (DOM + ARIA), not the Tauri window chrome. The Vite dev server is sufficient. Cross-platform Tauri shell a11y validation lands in Story 13.5.

### AC4 — Create 6 keyboard-only `@a11y`-tagged Playwright scaffold scenarios at `shell-ui/e2e/a11y/`.

- **NET-NEW directory** `shell-ui/e2e/a11y/`. Currently does NOT exist.
- **6 SCAFFOLD FILES** (one per LD-58-required surface):
  1. `shell-ui/e2e/a11y/today-dashboard.spec.ts` — Today Dashboard (Story 7.1 fills in).
  2. `shell-ui/e2e/a11y/agenda.spec.ts` — Agenda Today + Week views (Stories 6.3 + 6.4 fill in).
  3. `shell-ui/e2e/a11y/editor.spec.ts` — Editor surface (Story 4.1 fills in).
  4. `shell-ui/e2e/a11y/quick-capture.spec.ts` — Quick Capture surface (Story 8.1 fills in).
  5. `shell-ui/e2e/a11y/settings.spec.ts` — Settings surface (Story 1.18 begins; full Settings UI in Stories 12.* + 11.*).
  6. `shell-ui/e2e/a11y/graph-view.spec.ts` — Graph View (Story 8.11 fills in; LD-56 fallback in Story 8.10).
- **SCAFFOLD TEMPLATE** (use verbatim for each of the 6 files; substitute `<SURFACE>`, `<STORY-REF>`, `<ROUTE-PLACEHOLDER>`):
  ```ts
  /**
   * @a11y — <SURFACE> keyboard-only happy-path scenario (LD-58 gate #2 + #3).
   *
   * STATUS: SCAFFOLD — passes via test.fixme until <STORY-REF> lands the surface.
   *
   * Story 1.17 wires the LD-58 hard CI gate; the 6 keyboard-only scenarios are
   * shipped as scaffolds (test.fixme) because the surfaces themselves ship in
   * downstream epics. When <STORY-REF> lands, replace test.fixme with test,
   * implement the keyboard-only path, and confirm AxeBuilder passes.
   *
   * Implements NFR-9 / LD-58 (Story 1.17 scaffold).
   */
  import { test, expect } from '@playwright/test';
  import AxeBuilder from '@axe-core/playwright';

  test.describe('@a11y <SURFACE>', () => {
    test.fixme('keyboard-only happy-path + axe-core scan', async ({ page }) => {
      // TODO(<STORY-REF>): navigate to <ROUTE-PLACEHOLDER>, perform a
      // representative action via page.keyboard only (NO mouse.click),
      // and assert the persisted side-effect.

      await page.goto('<ROUTE-PLACEHOLDER>');

      // LD-58 gate #3 — keyboard-only navigation.
      // Example: await page.keyboard.press('Tab'); await page.keyboard.press('Enter');

      // LD-58 gate #2 — axe-core WCAG 2.1 AA scan.
      // serious + critical violations fail; best-practice tier excluded
      // per LD-58 line 1369 (avoid noise that erodes the gate).
      const results = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
        .analyze();
      const blocking = results.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical',
      );
      expect(blocking).toEqual([]);
    });
  });
  ```
- **WHY `test.fixme` NOT `test.skip`**: `test.skip` silently passes — the gate would atrophy unnoticed if a downstream story merges without filling in its a11y scenario. `test.fixme` runs the test, expects it to FAIL, and the test-runner reports a non-passing-but-expected-to-fail status. When the downstream dev replaces `test.fixme` with `test`, the assertions wake up. **Crucial discipline**: this prevents the "gate green but coverage zero" anti-pattern that LD-58 line 1369 explicitly warns against ("avoid noise that erodes the gate").
- **STORY-REF MAPPING TABLE** (use these literal values when substituting `<STORY-REF>` per file):
  | File | `<SURFACE>` | `<STORY-REF>` | `<ROUTE-PLACEHOLDER>` |
  |---|---|---|---|
  | today-dashboard.spec.ts | Today Dashboard | Story 7.1 | `/today` |
  | agenda.spec.ts | Agenda | Story 6.3 (Today view); Story 6.4 (Week view) | `/agenda/today` |
  | editor.spec.ts | Editor | Story 4.1 | `/editor` |
  | quick-capture.spec.ts | Quick Capture | Story 8.1 | (Tauri-window — TODO: decide on test pattern when Story 8.1 lands; may require Tauri shell boot) |
  | settings.spec.ts | Settings | Story 12.3 (keybinding remap) + Story 12.1 (CSS loader) | `/settings` |
  | graph-view.spec.ts | Graph View | Story 8.11 (canvas); Story 8.10 (a11y fallback) | `/graph` |
- **NOTE ON `/today` ROUTE**: at HEAD, `shell-ui/src/routes/_layout/today.tsx` renders a `TodayPlaceholder` component. Story 7.1 will replace it. The today-dashboard.spec.ts scaffold STILL uses `test.fixme` (the placeholder is not the Today Dashboard); the scaffold awakens when Story 7.1 ships.
- **DO NOT** invoke `page.mouse.click(...)` or `page.click(...)` in any `@a11y`-tagged scenario. The LD-58 line 1371 directive is explicit: "Each scenario starts with `page.keyboard` only (no `mouse.click`)". The scaffold's `// Example: ...` comment shows the keyboard pattern; downstream dev fills in the specifics. A `eslint-plugin-playwright` rule or grep check would harden this but is NOT in scope for Story 1.17 (the AC text in epics.md doesn't require it; defer to a tooling-hygiene retro item if it becomes a regression vector).
- **DO NOT** add a `tsconfig.json` for `shell-ui/e2e/`. The existing `shell-ui/tsconfig.json` covers `src/` only by default; extend its `include` to add `e2e/**/*` so Playwright's TypeScript compilation picks up the test files. (Verify the tsconfig `include` array at implementation time; if Playwright requires a separate tsconfig for module-resolution reasons, document the divergence here.)

### AC5 — Wire `pnpm a11y` script into root + shell-ui package.json + `.github/workflows/pr.yml`.

- **`shell-ui/package.json` SCRIPT ADDITIONS**:
  ```json
  "test:contrast": "vitest run src/themes/contrast.test.ts",
  "test:e2e": "playwright test",
  "test:e2e:a11y": "playwright test --grep @a11y",
  "e2e:setup": "playwright install chromium",
  "a11y": "pnpm run test:contrast && pnpm run test:e2e:a11y"
  ```
  Naming follows Story 1.8's `smoke:*` + `test:*` + `audit:*` script conventions (verified: existing scripts at `shell-ui/package.json:6-13` and root `package.json:7-19`).
- **ROOT `package.json` SCRIPT ADDITION**:
  ```json
  "a11y": "pnpm --filter shell-ui a11y"
  ```
  Mirrors the existing `"dev": "pnpm --filter shell-ui dev"` + `"build": "pnpm --filter shell-ui build"` pattern (verified: root `package.json:12-13`).
- **CI WORKFLOW INSERTION** at [`.github/workflows/pr.yml`](.github/workflows/pr.yml). The slot is **explicitly reserved** at line 176 with the comment "Story 1.17: pnpm a11y hard gate (contrast + axe-core + keyboard scenarios) lands here". **DO NOT** remove or rewrite the slot-reservation comment block (lines 174-178) — the other reservations (Story 1.12, Story 2.6) remain valid; just insert the Story 1.17 step UNDER the comment block but ABOVE the `merge-gate-nightly-fresh` job (which starts at line 189).
- **WORKFLOW STEP** (insert after line 178, BEFORE line 180):
  ```yaml
      # Step 16 (Story 1.17, LD-58) — WCAG 2.1 AA hard gate:
      #   (a) contrast-matrix Vitest test on shell-ui/src/themes/tokens.css
      #   (b) 6 @a11y-tagged Playwright scenarios with @axe-core/playwright
      #       (currently scaffolds via test.fixme; downstream stories awaken them)
      # Budget: ≤2-3 min combined per LD-58 line 1371; well within the LD-32
      # <90s/<10min per-PR soft target on warm Playwright browser cache.
      - name: Cache Playwright browsers
        uses: actions/cache@v4
        with:
          path: ~/.cache/ms-playwright
          key: playwright-${{ runner.os }}-${{ hashFiles('shell-ui/package.json') }}
          restore-keys: |
            playwright-${{ runner.os }}-

      - name: Install Playwright chromium
        run: pnpm --filter shell-ui exec playwright install --with-deps chromium

      - name: a11y hard gate (LD-58 — contrast + axe-core + keyboard scenarios)
        run: pnpm a11y
  ```
- **WHY `--with-deps`**: on Ubuntu runners, Playwright requires system libraries (`libnss3`, `libatk-bridge2.0-0`, etc.) that aren't in the base ubuntu-24.04 image. The `--with-deps` flag installs them via `apt-get`. On macOS-14 runners, the flag is a no-op (Playwright bundles macOS deps). This single flag works for both matrix cells.
- **WHY THE CACHE STEP**: chromium download is ~150MB. Without caching, every PR pays ~30s download. With caching, warm runs are ~2s. The cache key is keyed on `shell-ui/package.json` hash because Playwright's binary version is determined by the pinned `@playwright/test` version in `devDependencies`.
- **WORKFLOW MATRIX**: the existing job at `pr.yml:38` runs on `[macos-14, ubuntu-24.04]`. The new step inherits the matrix automatically — a11y runs on both cells. **EXPECTED RUNTIME**: ≤2-3 min combined (per LD-58 line 1371); with `test.fixme` scaffolds, actual Story 1.17 runtime is closer to ~30s (Vitest contrast test + 6 fixme'd Playwright tests that report fixme-status without executing). Downstream stories will bring runtime up to the budget as they awaken scenarios.
- **DO NOT** add the a11y step to `merge-gate-nightly-fresh` (lines 180-235). That job verifies the `nightly.yml` workflow ran within 24h; it's not where new gates are added. The a11y gate is a per-PR gate — its place is in the main matrix job.
- **DO NOT** add a separate `a11y.yml` workflow. The LD-58 line 1375 directive is "LD-32 per-PR job adds a `pnpm a11y` step" — i.e., extend the existing per-PR job, not a new workflow.

### AC6 — Local smoke + CI verification: contrast test passes; 6 scaffolds report fixme-status; no regressions.

- **LOCAL VERIFICATION** (dev agent runs these in order during implementation):
  1. `pnpm install` from repo root — confirm `vitest`, `@playwright/test`, `@axe-core/playwright`, `jsdom` install cleanly; confirm `pnpm-lock.yaml` updates without conflicts; confirm no new transitive dep introduces a license outside the LD-37 allowlist (run `pnpm run audit:licenses:js` to verify).
  2. `pnpm --filter shell-ui exec playwright install chromium` — confirm chromium downloads to `~/.cache/ms-playwright/`.
  3. `pnpm --filter shell-ui run test:contrast` — confirm Vitest discovers and runs `contrast.test.ts`; all 8 test cases (per AC2) pass; no failures.
  4. `pnpm --filter shell-ui run test:e2e:a11y` — confirm Playwright discovers and runs the 6 `@a11y`-tagged spec files; all 6 report `fixme` status (NOT pass, NOT fail, NOT skip — the Playwright reporter says "expected failure" / "test.fixme"); the run exits with code 0 (fixme is not a failure).
  5. `pnpm a11y` (from repo root) — confirm the orchestrator runs both test:contrast + test:e2e:a11y; combined wall-clock ≤30s on a warm cache locally.
- **REGRESSION GUARDS**:
  - `pnpm --filter shell-ui run build` still passes (no TypeScript errors in `e2e/**` or `src/themes/**`).
  - `pnpm --filter shell-ui run i18n:check` still passes (no new translatable strings introduced).
  - `cargo build --workspace` still passes (no Rust-side regressions — Story 1.17 is JS-only).
  - `pnpm run supply-chain` still passes (the two new MIT deps are pre-allowed).
- **CI VERIFICATION** (post-PR-open):
  - The `quality-gates` matrix job (both macos-14 + ubuntu-24.04) passes including the new `a11y hard gate` step.
  - The 6 `@a11y` spec files appear in the Playwright reporter output with `fixme` status; NONE pass (because they're scaffolds — passing would indicate the assertions are running against the placeholder route, which would be wrong); NONE fail (the gate is green).
  - The `merge-gate-nightly-fresh` job is unaffected (Story 1.17 does NOT touch nightly.yml).
- **DO NOT** add a `.github/workflows/nightly.yml` a11y step in this story. LD-58 line 1383 "v0.5+: expand keyboard-only scenario coverage from happy-path to representative-coverage" is the nightly-expansion follow-up; out of scope here.

### AC7 — Documentation + traceability annotations.

- **DOC-COMMENT ANNOTATIONS**: per the epic AC text at [epics.md:718](_bmad-output/planning-artifacts/epics.md#L718) — "the implementing modules carry `//! Implements NFR-9 a11y CI gate (LD-58)` as the first doc-comment line, verified by `tests/traceability.rs`".
  - **NOTE**: the `//!` syntax is Rust-specific; Story 1.17's implementing modules are TypeScript (`tokens.css`, `contrast.test.ts`, the 6 spec files, `vitest.config.ts`, `playwright.config.ts`). The TypeScript equivalent is JSDoc.
  - **NOTE**: `tests/traceability.rs` does NOT currently exist in the repo (verified — no such file). The closest convention is the `//! <crate>: <description>` top-line doc in `crates/*/src/lib.rs`, but that's a Rust-only pattern.
  - **RESOLUTION**: Story 1.17 ships the TypeScript-side traceability via JSDoc on each new file (per the templates in AC2, AC3, AC4):
    - `shell-ui/src/themes/contrast.test.ts` → `/** ... Implements NFR-9 / LD-58. */`
    - `shell-ui/playwright.config.ts` → `/** ... Implements NFR-9 / LD-58 (Story 1.17 scaffold). */`
    - `shell-ui/vitest.config.ts` → `/** ... Story 1.17 (LD-58 a11y gate scaffold). */`
    - Each of the 6 spec files → `/** ... Implements NFR-9 / LD-58 (Story 1.17 scaffold). */`
    - `shell-ui/src/themes/tokens.css` → CSS comment header `/* ... LD-51 canonical source / LD-58 gate input. */`
  - **VERIFICATION**: a grep-based smoke (no test wired) — `grep -r "LD-58" shell-ui/src/themes/ shell-ui/e2e/a11y/ shell-ui/playwright.config.ts shell-ui/vitest.config.ts` MUST return ≥7 hits. **DO NOT** scaffold a `tests/traceability.rs` here just to satisfy the epic-AC literal text — the epic AC was written generically and the literal pattern doesn't fit a JS-only story. Document this divergence in the Dev Agent Record under "AC variance".
- **`docs/` ADDITIONS**: none required for Story 1.17. LD-58 line 1381 ("v0.5+: expand keyboard-only scenario coverage") is a future docs update.
- **CONTRIBUTING.md / SECURITY.md / CHANGELOG.md**:
  - `CHANGELOG.md`: a single Conventional Commits entry suffices (Story 1.14 + Story 1.15 wired git-cliff; the commit message format `feat(ci): wire LD-58 WCAG 2.1 AA hard gate (Story 1.17, closes #17)` will auto-populate the changelog on next git-cliff run).
  - `CONTRIBUTING.md`: NO change required (LD-51 + LD-58 + tokens.css conventions can be documented in Story 6.7 when the full palette + theme-author guide lands).
  - `SECURITY.md`: NO change required (a11y is not a security boundary).

## Tasks / Subtasks

- [ ] **T1: Create `shell-ui/src/themes/tokens.css`** (AC1)
  - [ ] Create `shell-ui/src/themes/` directory.
  - [ ] Write `tokens.css` with FR-22 vocabulary + `@pair-role` annotations + provisional palette (template in AC1).
  - [ ] Update `shell-ui/src/styles/app.css`: `@import "../themes/tokens.css";` as the first line.
- [ ] **T2: Install Vitest + jsdom + create `vitest.config.ts`** (AC2)
  - [ ] `pnpm add -D vitest jsdom --filter shell-ui` (or edit `shell-ui/package.json` directly + `pnpm install`).
  - [ ] Create `shell-ui/vitest.config.ts` with the config in AC2.
  - [ ] Add `"test:contrast"` script to `shell-ui/package.json`.
- [ ] **T3: Write `shell-ui/src/themes/contrast.test.ts`** (AC2)
  - [ ] Implement `parseTokens()`, `relativeLuminance()`, `contrastRatio()` helpers.
  - [ ] Write all 8 test cases per AC2.
  - [ ] Run `pnpm --filter shell-ui run test:contrast` — confirm 8/8 pass.
- [ ] **T4: Install Playwright + @axe-core/playwright + create `playwright.config.ts`** (AC3)
  - [ ] `pnpm add -D @playwright/test @axe-core/playwright --filter shell-ui`.
  - [ ] Run `pnpm --filter shell-ui exec playwright install chromium`.
  - [ ] Create `shell-ui/playwright.config.ts` per AC3 template.
  - [ ] Add `"test:e2e"`, `"test:e2e:a11y"`, `"e2e:setup"` scripts to `shell-ui/package.json`.
  - [ ] Extend `shell-ui/tsconfig.json` `include` to cover `e2e/**/*`.
- [ ] **T5: Create the 6 `@a11y` scaffold specs at `shell-ui/e2e/a11y/`** (AC4)
  - [ ] `today-dashboard.spec.ts` (Story 7.1, route `/today`)
  - [ ] `agenda.spec.ts` (Story 6.3 + 6.4, route `/agenda/today`)
  - [ ] `editor.spec.ts` (Story 4.1, route `/editor`)
  - [ ] `quick-capture.spec.ts` (Story 8.1, Tauri-window — TODO note)
  - [ ] `settings.spec.ts` (Story 12.3 + 12.1, route `/settings`)
  - [ ] `graph-view.spec.ts` (Story 8.11 + 8.10, route `/graph`)
  - [ ] Each uses the verbatim template in AC4 with `test.fixme`.
- [ ] **T6: Wire `pnpm a11y` orchestrator + CI step** (AC5)
  - [ ] Add `"a11y"` script to `shell-ui/package.json` (chains test:contrast + test:e2e:a11y).
  - [ ] Add `"a11y"` script to root `package.json` (delegates to `pnpm --filter shell-ui a11y`).
  - [ ] Insert the Playwright cache + browser-install + a11y step into `.github/workflows/pr.yml` at line ~178 (under the slot-reservation comment block).
- [ ] **T7: Local smoke + regression checks** (AC6)
  - [ ] `pnpm install` — clean.
  - [ ] `pnpm a11y` — passes locally; combined runtime ≤30s on warm cache.
  - [ ] `pnpm --filter shell-ui run build` — passes (no TS errors).
  - [ ] `pnpm --filter shell-ui run i18n:check` — passes.
  - [ ] `cargo build --workspace` — passes.
  - [ ] `pnpm run supply-chain` — passes (new MIT deps clean).
- [ ] **T8: Doc-comment annotations + AC variance note** (AC7)
  - [ ] Add JSDoc `Implements NFR-9 / LD-58` to all 6 spec files + contrast.test.ts + both config files.
  - [ ] Add CSS comment header to `tokens.css`.
  - [ ] Verify `grep -r "LD-58" shell-ui/src/themes/ shell-ui/e2e/a11y/` ≥7 hits.
  - [ ] Record the `tests/traceability.rs` AC variance in the Dev Agent Record (AC7 explicit divergence).
- [ ] **T9: Open PR + ensure CI green**
  - [ ] Commit per Conventional Commits: `feat(ci): wire LD-58 WCAG 2.1 AA hard gate (Story 1.17, closes #17)`.
  - [ ] Confirm `quality-gates` matrix passes on macos-14 + ubuntu-24.04.
  - [ ] Confirm the 6 `@a11y` specs appear in CI logs with `fixme` status (not pass, not fail).

## Dev Notes

### Critical context the dev agent must internalize

1. **Path discrepancy with epic AC text.** The epic AC at [epics.md:711-718](_bmad-output/planning-artifacts/epics.md#L711-L718) uses `packages/shell-ui/...` paths everywhere. The ACTUAL repo path is `shell-ui/...` (no `packages/` prefix) per the Story 1.2 LD-5 round-4 amendment documented at [pnpm-workspace.yaml:1-9](pnpm-workspace.yaml#L1-L9): "no `packages/` indirection until a 2nd JS package exists". **USE THE ACTUAL `shell-ui/` PATH** everywhere; do NOT recreate `packages/`. The architecture document at [architecture.md:258-271](_bmad-output/planning-artifacts/architecture.md#L258-L271) similarly uses `shell-ui/` (root-level). This story's ACs above use the correct paths.

2. **Scaffold-not-implementation discipline.** The 6 `@a11y` specs are SCAFFOLDS via `test.fixme`. Resist the temptation to implement keyboard scenarios for the `/today` route — the route renders a `TodayPlaceholder`, not the real Today Dashboard. Story 7.1 will replace the placeholder; until then, scaffold `test.fixme` is the correct state. Implementing now would (a) lock the scenario against a placeholder that's about to change, (b) green the gate against zero real coverage, (c) violate the LD-58 line 1369 "avoid noise that erodes the gate" principle.

3. **`test.fixme` is intentional, not laziness.** `test.skip` would silently pass; `test.fixme` reports an "expected failure" status that's visible in CI output. Downstream story devs see the fixme'd tests in their PR's Playwright reporter and know exactly what to awaken. This pattern is what makes Story 1.17 a "context-engine" win — the gate is wired AND the awakening points are inventoried.

4. **The traceability AC variance is real and acceptable.** The epic AC text at [epics.md:718](_bmad-output/planning-artifacts/epics.md#L718) demands `//! Implements NFR-9 a11y CI gate (LD-58)` doc-comment first-line + `tests/traceability.rs` verification. Neither convention exists for TypeScript files. Story 1.17 ships JSDoc equivalents (per AC7) and a grep-smoke. Record this divergence in the Dev Agent Record so future-you (or the code-review pass) doesn't flag it as missing work.

5. **License allowlist already accepts MIT.** Both new deps (`@playwright/test`, `@axe-core/playwright`) are MIT. Story 1.7's `deny.toml` allow list at [deny.toml:72-89](deny.toml#L72-L89) includes MIT as the first entry. **DO NOT** modify `deny.toml`. Run `pnpm run audit:licenses:js` after install to verify transitive deps don't introduce a non-allowed license — if they do (unlikely for these two upstreams), STOP and surface a decision-grade question.

6. **CI slot is explicitly pre-reserved.** Line 176 of `.github/workflows/pr.yml` reads: `# Story 1.17: pnpm a11y hard gate (contrast + axe-core + keyboard scenarios) lands here`. **DO NOT** delete the slot-reservation comment block (lines 174-178); insert the new step UNDER the block. The other reservations (Story 1.12, Story 2.6) remain valid.

7. **OKLch vs hex.** The existing `shell-ui/src/styles/app.css` uses OKLch color values. The new `tokens.css` uses hex. This is intentional: Story 1.17's contrast calculator handles hex only (simplest math, ~10 LOC); Story 6.7 can introduce OKLch and extend the calculator when the real Orgsidian palette lands. **DO NOT** unify the color space now — the two stylesheets serve different layers (shadcn baseline vs. `--org-*` public theme API).

### Project Structure Notes

**Alignment with unified project structure**:
- `shell-ui/src/themes/` — NEW directory, matches architecture tree at [architecture.md:258-261](_bmad-output/planning-artifacts/architecture.md#L258-L261) ✓
- `shell-ui/e2e/a11y/` — NEW directory, matches LD-58 scaffolding decision at [architecture.md:1376](_bmad-output/planning-artifacts/architecture.md#L1376) ✓
- `shell-ui/vitest.config.ts` + `shell-ui/playwright.config.ts` — NEW root-level configs, conventional placement ✓
- Root `package.json` `a11y` script — mirrors `dev` + `build` filter pattern ✓

**Detected conflicts or variances** (with rationale):
- The epic AC says `packages/shell-ui/` (stale wording). Actual path is `shell-ui/` per the Story 1.2 amendment. Story 1.17 uses the actual path. Rationale: pnpm-workspace.yaml line 7 + architecture tree are the authoritative source.
- The epic AC mentions `tests/traceability.rs` verification (Rust pattern). Story 1.17's implementing modules are all TypeScript. Ship JSDoc equivalents + grep smoke; document the divergence in the Dev Agent Record.

### Testing Standards Summary

- **Unit tests (Vitest)**: located alongside source files as `*.test.ts`. Excluded from production build via Vite's default heuristics + Vitest's `exclude` glob.
- **E2E tests (Playwright)**: located under `shell-ui/e2e/`. Tagged by directory + by `test.describe('@a11y ...')` + the `--grep @a11y` filter.
- **Test runtime budget (LD-32 + LD-58)**: per-PR full `pnpm a11y` ≤2-3 min on warm cache (LD-58 line 1371); Story 1.17's actual scaffold runtime ~30s on warm cache.
- **CI matrix**: macos-14 + ubuntu-24.04 (existing per [pr.yml:38](.github/workflows/pr.yml#L38)). The new a11y step inherits the matrix.

### References

- Source story: [`epics.md:699-718`](_bmad-output/planning-artifacts/epics.md#L699-L718) — Story 1.17 user-story + AC + Traces.
- Architecture: [`architecture.md:1359-1385`](_bmad-output/planning-artifacts/architecture.md#L1359-L1385) — LD-58 full text (the three gates).
- Architecture: [`architecture.md:1299-1305`](_bmad-output/planning-artifacts/architecture.md#L1299-L1305) — LD-51 (tokens.css canonical-source convention).
- Architecture: [`architecture.md:521-528`](_bmad-output/planning-artifacts/architecture.md#L521-L528) — LD-32 (CI matrix + budgets).
- Architecture: [`architecture.md:193`](_bmad-output/planning-artifacts/architecture.md#L193) — stack-versions pin for `@axe-core/playwright`.
- Architecture: [`architecture.md:282-311`](_bmad-output/planning-artifacts/architecture.md#L282-L311) — FR-22 `--org-*` token vocabulary.
- UX spec: [`ux-design-specification.md:2149-2210`](_bmad-output/planning-artifacts/ux-design-specification.md#L2149-L2210) — full a11y gate definition (3 gates).
- UX spec: [`ux-design-specification.md:180`](_bmad-output/planning-artifacts/ux-design-specification.md#L180) — Experience Principle 9 (accessibility from v0.1).
- Previous story: [`1-16-github-issues-sync-one-issue-per-story.md`](_bmad-output/implementation-artifacts/1-16-github-issues-sync-one-issue-per-story.md) — CI step insertion pattern + supply-chain license-allow protocol.
- Slot reservation: [`.github/workflows/pr.yml:176`](.github/workflows/pr.yml#L176) — explicit `# Story 1.17: pnpm a11y hard gate ...` line.
- WCAG 2.1 SC 1.4.3: [Contrast (Minimum)](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html) — ≥4.5:1 for normal text, ≥3:1 for large text.
- WCAG 2.1 SC 1.4.11: [Non-text Contrast](https://www.w3.org/WAI/WCAG21/Understanding/non-text-contrast.html) — ≥3:1 for UI components.
- @axe-core/playwright: [npm package](https://www.npmjs.com/package/@axe-core/playwright) — `AxeBuilder` API + `withTags()` filter.

### Previous Story Intelligence (from Story 1.16)

Relevant to Story 1.17:

- **CI step ordering**: Story 1.16 wired a separate workflow (`sync-issues.yml`) for the issues-sync tool because it triggers on push-to-main, not per-PR. Story 1.17 is the OPPOSITE — it extends the existing per-PR `pr.yml`. No new workflow file needed.
- **License-allowlist protocol**: Story 1.16 verified `octocrab` MIT/Apache-2.0 cleanliness via `pnpm run audit:licenses:js`. Use the same verification step for `@playwright/test` + `@axe-core/playwright`.
- **Pre-allowed JS license set**: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, Zlib, MPL-2.0, Unicode-3.0, BSL-1.0, Apache-2.0 WITH LLVM-exception ([deny.toml:72-89](deny.toml#L72-L89)). Both new deps + their typical transitive surface (e.g., `axe-core` core itself is Mozilla Public 2.0) are inside this set — verify post-install.
- **Conventional Commits + git-cliff**: Story 1.14 + Story 1.15 wired commitlint + git-cliff. Use the format `feat(ci): wire LD-58 WCAG 2.1 AA hard gate (Story 1.17, closes #17)` for the implementing commit; the changelog auto-populates on next git-cliff run.
- **GitHub Issues sync**: Story 1.16 wired one-issue-per-story sync. Story 1.17 → GitHub issue #17 (per the github_issue metadata at the top of this file). The sync runs on push-to-main when `epics.md` changes; issue #17 already exists since the epic includes Story 1.17.

### Git Intelligence Summary

Recent commits relevant to Story 1.17:

- **`064ea73`** (Merge PR #133): Story 1.16 GitHub Issues sync — established the pattern of "one issue per story" sync; issue #17 exists.
- **`9e2d662`** (Story 1.16): `feat(ci): wire LD-55 GitHub Issues sync via tools/issues-sync` — the most recent CI workflow change; demonstrates the `feat(ci): wire LD-NN ...` commit-title convention.
- **`93df7b4`** (Story 1.15): `feat(ci): wire git-cliff CC → CHANGELOG generation` — same pattern.
- **`22bbb24`** (Story 1.14): `feat(ci): wire commitlint commit-range + PR-title gates` — same pattern.
- **None of the recent 20 commits touch a11y, WCAG, contrast, or axe-core**. Story 1.17 is the first accessibility-gate story in the repo. No prior patterns to override or conflict with.
- **None of the recent 20 commits touch `shell-ui/src/themes/`** (the directory does not yet exist). Net-new scope.

### Latest Technical Information

**Verify versions at implementation time** (per [[feedback_version_policy]] — pin to latest stable LTS):

- **Vitest**: latest stable is the 3.x line. Pin to `^3.0.0` (semver-minor). Vitest 3.x ships first-class TypeScript support, `environment: 'jsdom'` works out-of-the-box, and the `vitest run <path>` form is the idiomatic single-file invocation. Verify with `pnpm view vitest version`.
- **@playwright/test**: latest stable is the 1.x line (1.5x at the time of this writing). Pin to `^1.50.0`. Playwright 1.x is the only line; major version increments are extremely rare. Verify with `pnpm view @playwright/test version`.
- **@axe-core/playwright**: latest stable is the 4.x line. Pin to `^4.10.0`. The `AxeBuilder` class is the entrypoint; `withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])` is the canonical WCAG 2.1 AA filter; `analyze()` returns a `Result` object with `.violations[]`, each having an `.impact` field (`'minor' | 'moderate' | 'serious' | 'critical'`). The LD-58 line 1369 gate is "serious + critical fail" — minor/moderate are reported but non-blocking. Verify with `pnpm view @axe-core/playwright version`.
- **jsdom**: latest stable is the 25.x line. Pin to `^25.0.0`. Vitest's `environment: 'jsdom'` peer-depends on `jsdom`; pinning it explicitly is the idiomatic pattern.
- **WCAG 2.1 contrast formula**: stable since 2008 (WCAG 2.0); the WCAG 2.1 spec (June 2018) didn't change the formula, only added SC 1.4.11 (non-text contrast) and SC 1.4.13 (content on hover). The formula `(L1 + 0.05) / (L2 + 0.05)` is canonical. WCAG 2.2 (October 2023) likewise does not change the contrast formula. Story 1.17 implements the formula directly.
- **axe-core rule set**: WCAG 2.1 AA tags (`wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`) cover the four severity tiers within axe's WCAG mapping. The `best-practice` tag is EXCLUDED per LD-58 line 1369 (would flag e.g. landmark-region misuse on a Tauri webview, which is shell-managed not app-managed).

### Project Context Reference

The repository's project context lives across:
- [`_bmad-output/planning-artifacts/prd.md`](_bmad-output/planning-artifacts/prd.md) — PRD (§8 NFRs, §11 Action Inventory).
- [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md) — Architecture (LD-32, LD-51, LD-58, FR-22, stack-versions).
- [`_bmad-output/planning-artifacts/ux-design-specification.md`](_bmad-output/planning-artifacts/ux-design-specification.md) — UX spec (§ Accessibility — WCAG 2.1 AA as Hard Gate, Principle 9).
- [`_bmad-output/planning-artifacts/epics.md`](_bmad-output/planning-artifacts/epics.md) — Epics + Stories (this story at line 699-718).

The PRD + Architecture were finalized 2026-05-19 with the 2026-05-20 UXD-reconciliation closing the loop ([architecture.md:1267](_bmad-output/planning-artifacts/architecture.md#L1267)); 51 LDs locked, Story 1.17 is part of the post-2026-05-20 reconciliation wave.

## Dev Agent Record

### Agent Model Used

_To be filled by dev agent_

### Debug Log References

_To be filled by dev agent_

### Completion Notes List

_To be filled by dev agent_

### File List

_To be filled by dev agent_
