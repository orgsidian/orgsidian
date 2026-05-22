# Story 1.6: Install Lingui v6.x i18n scaffold

Status: done

## Metadata

github_issue: 6

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want Lingui v6.x installed with the SWC macro plugin + Vite plugin + `eslint-plugin-lingui` + `lingui extract --clean && git diff --exit-code` as a CI-ready discipline,
So that NFR-10 translation infrastructure ships in v1.0 without a late-cycle retrofit project, every UI string from Epic 2 onwards is authored extraction-ready, and the v1.0 community-translation handoff (PRD §8) is a configuration change rather than an architectural migration.

## Acceptance Criteria

**AC1 — `shell-ui/package.json` declares the Lingui v6.x dependency set at `^6.0.1` per architecture LD-52 + stack lock (line 191).**

- `dependencies` adds `@lingui/core` and `@lingui/react`, both `^6.0.1`.
- `devDependencies` adds `@lingui/cli`, `@lingui/vite-plugin`, `@lingui/swc-plugin`, and `eslint-plugin-lingui`, all `^6.0.1`.
- `babel-plugin-macros` MUST NOT be added (SWC path only per LD-52).
- All six packages MUST install at exactly `^6.0.1` — no exact-pinning per `[[feedback_version_policy]]` (LTS-preferred, floats minor/patch). `pnpm-lock.yaml` lock-time captures the resolved versions.
- No transitive React or Vite version bumps — `react@^19.1.0` and `vite@^7.0.4` remain unchanged.

**AC2 — Swap `@vitejs/plugin-react` → `@vitejs/plugin-react-swc` per LD-52 stack lock ("`@vitejs/plugin-react-swc` we depend on per stack lock").**

- `shell-ui/package.json` `devDependencies`: **remove** `@vitejs/plugin-react`, **add** `@vitejs/plugin-react-swc` at the latest stable `^3.x` minor (resolved at lock time, per `[[feedback_version_policy]]`).
- `shell-ui/vite.config.ts` updates the import line from `import react from "@vitejs/plugin-react"` to `import react from "@vitejs/plugin-react-swc"`. No other change to the plugin invocation **except** the SWC plugin entry (AC3).
- Rationale: Story 1.3 scaffolded the Babel-based `@vitejs/plugin-react`; LD-52 mandates SWC because `@lingui/swc-plugin` is an SWC-only macro plugin (Babel equivalent `@lingui/babel-plugin-lingui-macro` is explicitly NOT in the dep set, see AC1). This is a one-line plugin swap, not a build-pipeline rewrite — `@vitejs/plugin-react-swc` is a drop-in replacement for Vite-React projects per Vite docs.
- `pnpm dev` and `pnpm build` MUST exit 0 after the swap with zero new warnings. The TanStack Router plugin + Tailwind 4 plugin retain their existing order (`tanstackRouter()` MUST stay first, `tailwindcss()` second, `react()` third — per the comment at [shell-ui/vite.config.ts:13](shell-ui/vite.config.ts#L13)).

**AC3 — `shell-ui/vite.config.ts` registers the `@lingui/swc-plugin` SWC entry + the `@lingui/vite-plugin` Vite plugin per LD-52.**

```ts
import { defineConfig } from "vite";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import { lingui } from "@lingui/vite-plugin";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(async () => ({
  plugins: [
    // MUST come before tailwindcss() and react() per TanStack docs.
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    tailwindcss(),
    react({ plugins: [["@lingui/swc-plugin", {}]] }),
    lingui(),
  ],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: false,
    watch: {
      ignored: [
        "**/crates/orgsidian-shell-app/**",
        "**/target/**",
        "**/_bmad-output/**",
        "**/_bmad/**",
      ],
    },
  },
}));
```

- The `react()` call MUST pass `{ plugins: [["@lingui/swc-plugin", {}]] }` — this is the verbatim invocation from architecture line 364 ("`react({ plugins: [["@lingui/swc-plugin", {}]] })`").
- `lingui()` MUST appear AFTER `react()` — the SWC plugin transforms `<Trans>` JSX during React's pass; `lingui()` then handles `.po` → `.ts` catalog compilation. Order matters per `@lingui/vite-plugin` v6 docs.
- The empty `{}` config object on `@lingui/swc-plugin` is intentional — defaults are sufficient for v0.1 scaffold; runtime options would be added in a future story (e.g., if `stripMessageField: true` becomes needed for prod bundle size optimization, that lands with bundle-size tuning, not here).
- DO NOT touch the existing `server.watch.ignored` glob list, the alias config, or the `clearScreen: false` line — those are Story 1.3 invariants.

**AC4 — `shell-ui/lingui.config.ts` declares `en` as the source locale + Gettext `.po` catalog format per LD-52.**

```ts
import type { LinguiConfig } from "@lingui/conf";

const config: LinguiConfig = {
  locales: ["en"],
  sourceLocale: "en",
  catalogs: [
    {
      path: "<rootDir>/src/locales/{locale}/messages",
      include: ["<rootDir>/src"],
      exclude: ["**/*.test.{ts,tsx}", "**/routeTree.gen.ts", "**/node_modules/**"],
    },
  ],
  format: "po",
  compileNamespace: "ts",
};

export default config;
```

- Locale list MUST start with `["en"]` only — additional locales are added when community translations arrive (v1.0+); locking the day-1 set to `en` keeps the initial extract bounded.
- `sourceLocale: "en"` matches PRD §8 ("default English") + epic AC.
- Catalog `path` MUST resolve to `shell-ui/src/locales/en/messages.po` after extract. The `<rootDir>` token is the Lingui-CLI placeholder for the shell-ui workspace root (auto-resolves via the `lingui.config.ts` location).
- `format: "po"` is mandatory per LD-52 ("`.po` (Gettext) ... compiled to TypeScript at build time"). DO NOT use `"json"` or `"minimal"` — Gettext is the lingua franca for Crowdin/Weblate/Transifex per LD-52 rationale (b).
- `compileNamespace: "ts"` produces a TypeScript compiled catalog (`messages.ts`) on `lingui compile`, consumed at runtime via static import (no runtime PO parser, ~3 kB runtime budget per LD-52 rationale (a) / FR-10 / LD-28 Quick Capture cold-start).
- `exclude` MUST list `routeTree.gen.ts` — that file is auto-generated by `@tanstack/router-plugin` and contains no user-authored strings; including it would create spurious extract churn on every router regeneration.
- The `tsx` file extension (NOT `.ts` / not `.json`) is mandatory — the project tsconfig has `"isolatedModules": true` + `"allowImportingTsExtensions": true` and the Lingui CLI reads the config via `tsx` / `esbuild-register`, both compatible with TS config files.

**AC5 — Locale catalog path materializes at `shell-ui/src/locales/en/messages.po` on first `pnpm extract`, with one extracted message from a `<Trans>` smoke string in the root component.**

- After running `pnpm extract` (AC6), the file `shell-ui/src/locales/en/messages.po` MUST exist and contain exactly one `msgid` matching the smoke string (e.g., `Today` if placed in `__root.tsx`, or `Orgsidian` if placed in a higher branding location — see Tasks for exact placement).
- The smoke `<Trans>` MUST be added to `shell-ui/src/routes/__root.tsx` — the canonical "root component" per TanStack Router's file-based routing convention (this file owns the application-wide outlet wrapping all child routes, making it the natural root for an i18n smoke string).
- Smoke string content: use `Orgsidian` wrapped in `<Trans>Orgsidian</Trans>` placed inside a `<span className="sr-only">` (screen-reader-only span) — this satisfies the extract requirement without changing visible UI, and aligns with the WCAG 2.1 AA discipline (LD-58 / Story 1.17) by providing a screen-reader-detectable app name landmark.
  - Rationale for `Orgsidian` over a localizable phrase like `Loading…`: the product name `Orgsidian` is a proper noun that will NOT be translated, but the *act of wrapping it* exercises the entire extract → compile → render pipeline. A more visibly-localized string (e.g., `<Trans>Loading…</Trans>`) would create a phantom "Loading…" UI element that has no current purpose. Day-1 surface keeps user-visible behavior unchanged.
- The catalog MUST be committed to git on first extract — otherwise the `lingui extract --clean && git diff --exit-code` discipline cannot work (a fresh clone with no committed catalog would always report a diff).
- The compiled output (`messages.ts`) is **gitignored** — it's a build artifact regenerated by `pnpm compile` (AC6) and by `@lingui/vite-plugin` on build. Adding `src/locales/**/*.ts` (NOT `messages.po`!) to `shell-ui/.gitignore` is part of this story.

**AC6 — `shell-ui/package.json` adds two pnpm scripts wiring `@lingui/cli` per architecture line 364 ("`lingui extract` + `lingui compile` wired into `pnpm` scripts").**

```json
"scripts": {
  "dev": "vite",
  "prebuild": "tsr generate && cargo test --locked --package orgsidian-shell-app --test export_bindings --quiet",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "extract": "lingui extract --clean",
  "compile": "lingui compile",
  "i18n:check": "lingui extract --clean && git diff --exit-code src/locales"
}
```

- `extract` is the authoring command: re-scans `src/` per the `lingui.config.ts` include/exclude rules and updates `messages.po`. The `--clean` flag removes orphan msgids (msgids in `.po` that no longer have a matching `<Trans>` in source) — this is what makes the CI gate (LD-52 + AC7) able to detect catalog drift.
- `compile` is the build-time command: turns `messages.po` → `messages.ts` (typed compiled catalog). Called locally for IDE type-checking; `@lingui/vite-plugin` also invokes it during `vite build`.
- `i18n:check` is the CI-shape command — runs `extract --clean` then asserts `git diff --exit-code src/locales` exits 0. This is the single pnpm command that Story 1.8 will register in `.github/workflows/pr.yml` as a CI gate.
- `prebuild` MUST stay unchanged from Story 1.4 (typed-IPC binding round-trip) — DO NOT remove the `cargo test --locked --package orgsidian-shell-app --test export_bindings` step.
- DO NOT add `extract` / `compile` as part of `build` — the architecture wires `@lingui/vite-plugin` to handle compile-on-build automatically (LD-52). Layering a manual `compile` in `build` would double-run and risk drift between `pnpm build` and `vite build` invocations from other tooling (e.g., `tauri build`).

**AC7 — `pnpm i18n:check` exits 0 on a clean repo and exits non-zero when source / catalog drift exists; the discipline is hookable to CI without further wiring.**

- After implementation: running `pnpm -C shell-ui i18n:check` MUST exit 0 (commit the initial `messages.po` first, so the post-extract diff is empty).
- Simulated drift verification (manual smoke during dev — NOT a committed test): adding a new `<Trans>Sample</Trans>` somewhere in `src/` without re-running `pnpm extract` and then running `pnpm -C shell-ui i18n:check` MUST exit non-zero with a visible `git diff --exit-code` failure.
- CI gate wiring itself (i.e., a step in `.github/workflows/pr.yml`) is **EXPLICITLY NOT IN SCOPE** for Story 1.6 — that lands in Story 1.8 ("Configure CI matrix") alongside `cargo build/test/clippy`, `pnpm typecheck/test`, and the LD-58 a11y step. Story 1.6 ships the `pnpm i18n:check` shape; Story 1.8 invokes it.
- Document the Story 1.8 expectation explicitly in this story's Dev Notes so the future story author wires the right command name (`pnpm -C shell-ui i18n:check`, not the lower-level `lingui extract` invocation).

**AC8 — `shell-ui/src/main.tsx` mounts the `I18nProvider` and statically imports the `en` catalog at boot per LD-52 ("Default locale `en` statically imported at boot").**

```tsx
import "./styles/app.css";
import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { messages as enMessages } from "./locales/en/messages";
import { routeTree } from "./routeTree.gen";

i18n.load("en", enMessages);
i18n.activate("en");

const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  scrollRestoration: true,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider i18n={i18n}>
      <RouterProvider router={router} />
    </I18nProvider>
  </React.StrictMode>,
);
```

- `i18n.load("en", enMessages)` + `i18n.activate("en")` MUST be called BEFORE `ReactDOM.createRoot(...).render(...)` — otherwise the first render synchronously crashes on missing catalog (Lingui v6 throws on `<Trans>` before activation).
- `<I18nProvider i18n={i18n}>` MUST wrap `<RouterProvider>` (not the other way around) so all routes — including `__root.tsx` — see the provider.
- The import `import { messages as enMessages } from "./locales/en/messages"` resolves via the compiled `messages.ts` artifact at build time. Day-1 fresh clone: `pnpm -C shell-ui compile` produces `messages.ts`; `vite dev` / `vite build` also produces it via `@lingui/vite-plugin`. The import is statically typed (no `as any` casts) per architecture's TS strict mandate.
- Dynamic `import()` for non-default locales (per LD-52 "other locales lazy-loaded via dynamic `import()` keyed by `navigator.language` + Settings override") is **EXPLICITLY DEFERRED** — locale switching UI lands with Settings (Story 12.3 keybinding-remapping-UI is the same Settings panel touch-point, or its successor). Day-1 surface is `en` only.

**AC9 — `shell-ui/src/locales/en/messages.po` is committed; `shell-ui/src/locales/**/messages.ts` is gitignored.**

- `shell-ui/.gitignore` adds the line: `src/locales/**/messages.ts` (and `src/locales/**/messages.mjs` defensively, in case Lingui v6 minor bumps emit `.mjs` — a no-op forward-compat hedge).
- `shell-ui/.gitignore` MUST NOT exclude `messages.po` files — those are source-of-truth catalogs and live under version control.
- If `shell-ui/.gitignore` does not exist yet, create it with the two locales lines. Verify by `git status` post-extract: `messages.po` shows up as a new tracked file; `messages.ts` does NOT.

**AC10 — Anti-creep audit: nothing outside the Story 1.6 scope-fence is modified.**

The following files MUST NOT be touched by this story:
- `crates/**/*` — Story 1.6 is frontend-only; the Rust core, plugin-api, shell-app, and CLI crates are out of scope.
- `shell-ui/src/routes/index.tsx`, `shell-ui/src/routes/_layout/today.tsx`, `shell-ui/src/components/**` — the existing routes and components are not localized in this story (Story 1.6 is scaffold-only; per-string localization arrives feature-by-feature in Epics 2-12).
- `shell-ui/src/lib/tauri.ts` — Story 1.4 typed-IPC client; out of scope.
- Root `package.json`, root `Cargo.toml`, `tauri.conf.json`, `capabilities/**/*.json`, `.github/workflows/**` — Story 1.6 is shell-ui workspace-internal.
- `crates/orgsidian-plugin-api/**` — Story 1.5 leaf crate; the i18n surface is JS-side only per LD-52 rationale (e) ("`fluent-rs` not applicable: all localized strings live in the React webview").

Allowed touched files (full list):
- `shell-ui/package.json` (AC1, AC2, AC6)
- `shell-ui/vite.config.ts` (AC2, AC3)
- `shell-ui/lingui.config.ts` (AC4 — NEW)
- `shell-ui/src/main.tsx` (AC8)
- `shell-ui/src/routes/__root.tsx` (AC5 — smoke `<Trans>`)
- `shell-ui/src/locales/en/messages.po` (AC5, AC9 — NEW)
- `shell-ui/.gitignore` (AC9 — possibly NEW)
- `pnpm-lock.yaml` (auto-updated by `pnpm install`)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (out-of-band tracking)
- `_bmad-output/implementation-artifacts/1-6-install-lingui-v6-x-i18n-scaffold.md` (this file — Status / Dev Agent Record)

**AC11 — Local gates pass with zero new warnings.**

1. `pnpm install` (from repo root) → exit 0 after package.json edits; lockfile updated.
2. `pnpm -C shell-ui extract` → exit 0; creates `src/locales/en/messages.po` with one msgid.
3. `pnpm -C shell-ui compile` → exit 0; creates `src/locales/en/messages.ts`.
4. `pnpm -C shell-ui i18n:check` → exit 0.
5. `pnpm -C shell-ui build` → exit 0 with zero new warnings (the `tsc` step in the prebuild script must accept the new `import { messages as enMessages }` line — verify with `pnpm -C shell-ui exec tsc --noEmit`).
6. `pnpm -C shell-ui dev` → starts on port 1420 with no console errors; `<Trans>Orgsidian</Trans>` renders the literal text `Orgsidian` (verified via DOM inspection — the `sr-only` span exists with text `Orgsidian`).
7. `cargo build --workspace` → exit 0 (no regression to Story 1.4 typed-IPC).
8. `cargo test --workspace` → exit 0 (Story 1.5 `tests/trait_surface.rs` + Story 1.4 `tests/export_bindings.rs` both still pass).
9. No new `pnpm audit` advisories at severity ≥ medium (Lingui v6 + plugin-react-swc are MIT-licensed, fresh-published — `pnpm audit` clean expected).

**AC12 — `eslint-plugin-lingui` is installed (per AC1) but NOT wired into an ESLint config in this story.**

- The architecture (line 933) expects `shell-ui/eslint.config.js`, but Story 1.3 did not scaffold one and Story 1.6 does not scaffold one either.
- `eslint-plugin-lingui` ships in `devDependencies` per the epic AC + AC1 ("`shell-ui/package.json` lists `eslint-plugin-lingui` at `^6.0.1`"). This satisfies the epic-AC letter.
- Activating the plugin requires an ESLint config file with React + TanStack Router presets per architecture line 847 ("TS: ESLint + Prettier with React + TanStack Router presets; TS strict mode; `noUncheckedIndexedAccess: true`"). That scaffolding is **out of scope for Story 1.6** — it belongs in a dedicated ESLint-scaffold story (suggested: a new Story 1.17.x or a placement in Story 1.8 alongside the CI matrix).
- This story's Completion Notes MUST disclose this deferral with a pointer to the future story so the architectural intent (`eslint-plugin-lingui` as a CI gate per LD-52) is not lost.
- DO NOT scaffold a placeholder `eslint.config.js` here — that would expand scope and risk gold-plating Story 1.6 with rules that conflict with the future React + TanStack Router presets choice.

## Tasks / Subtasks

- [x] **Task 1: Update `shell-ui/package.json` with Lingui v6.x dep set + react-swc swap (AC: 1, 2, 6).**
  - [x] 1.1 Remove `@vitejs/plugin-react` from `devDependencies`.
  - [x] 1.2 Add `@vitejs/plugin-react-swc` at latest stable `^4.0.0` to `devDependencies` (resolved 4.3.1 at lock time — see Completion Notes for deviation from the story's "`^3.x`" suggestion).
  - [x] 1.3 Add `@lingui/core` and `@lingui/react` at `^6.0.1` to `dependencies`.
  - [x] 1.4 Add `@lingui/cli`, `@lingui/conf`, `@lingui/vite-plugin`, `@lingui/swc-plugin` at `^6.0.1` and `eslint-plugin-lingui` at `^0.13.1` to `devDependencies` (see Completion Notes for the `eslint-plugin-lingui` version-pin deviation).
  - [x] 1.5 Add the three pnpm scripts (`extract`, `compile`, `i18n:check`) per AC6 verbatim.
  - [x] 1.6 Run `pnpm install` from repo root; verify `pnpm-lock.yaml` updates and no peer-dep warnings appear.

- [x] **Task 2: Update `shell-ui/vite.config.ts` to use `@vitejs/plugin-react-swc` + register `@lingui/swc-plugin` + `@lingui/vite-plugin` (AC: 2, 3).**
  - [x] 2.1 Change the `react` import: `import react from "@vitejs/plugin-react-swc"`.
  - [x] 2.2 Add the `lingui` import: `import { lingui } from "@lingui/vite-plugin"`.
  - [x] 2.3 Update the `react()` invocation in the plugins array to `react({ plugins: [["@lingui/swc-plugin", {}]] })`.
  - [x] 2.4 Append `lingui()` to the plugins array AFTER `react(...)`.
  - [x] 2.5 Verify `pnpm -C shell-ui dev` starts on port 1420 with zero console errors. STOP the dev server before proceeding (`Ctrl+C`).

- [x] **Task 3: Create `shell-ui/lingui.config.ts` (AC: 4).**
  - [x] 3.1 Copy the AC4 config block verbatim into `shell-ui/lingui.config.ts` — `format: "po"` removed per Lingui v6 typing (CatalogFormatter now required; PO is the default). See Completion Notes.
  - [x] 3.2 Verified `pnpm exec tsc --noEmit` exits 0 (the config file lives outside `tsconfig.json` `include`, so the project typecheck does not type the file; Lingui CLI loads it natively).

- [x] **Task 4: Add the `<Trans>` smoke string to `shell-ui/src/routes/__root.tsx` (AC: 5).**
  - [x] 4.1 Import `Trans` from `@lingui/react/macro`.
  - [x] 4.2 Added a screen-reader-only span inside the root component wrapping `<Trans>Orgsidian</Trans>`.
  - [x] 4.3 `sr-only` resolves via Tailwind 4 (Story 1.3) — no fallback needed.

- [x] **Task 5: Run `pnpm -C shell-ui extract` to seed the catalog (AC: 5, 9).**
  - [x] 5.1 Extract produced `shell-ui/src/locales/en/messages.po` with one `msgid "Orgsidian"` entry (catalog statistics: en source, count 1).
  - [x] 5.2 No misconfiguration — extract worked first try with `@lingui/react/macro` import.

- [x] **Task 6: Add `.gitignore` entry for compiled catalogs (AC: 9).**
  - [x] 6.1 `shell-ui/.gitignore` did not exist; created it.
  - [x] 6.2 Added the two gitignore lines (`src/locales/**/messages.ts` and `src/locales/**/messages.mjs`).
  - [x] 6.3 `git status` post-extract: `messages.po` is the only locale file shown as tracked-new; `messages.ts` is correctly ignored.

- [x] **Task 7: Run `pnpm -C shell-ui compile` to produce the typed catalog (AC: 8, 11.3).**
  - [x] 7.1 Compile produced `shell-ui/src/locales/en/messages.ts`.
  - [x] 7.2 Spot-checked: file declares `export const messages: Messages` (content-hash-keyed shape — Lingui's runtime-efficient compile output).

- [x] **Task 8: Wire `I18nProvider` + static catalog import into `shell-ui/src/main.tsx` (AC: 8).**
  - [x] 8.1 Applied the AC8 code block verbatim to `shell-ui/src/main.tsx`.
  - [x] 8.2 `pnpm exec tsc --noEmit` exits 0.

- [x] **Task 9: Run the binding gate suite (AC: 11).**
  - [x] 9.1 `pnpm extract` after seeding — no diff against committed `messages.po` (extract is idempotent after the first run that adds PO standard headers).
  - [x] 9.2 `pnpm i18n:check` — exit 0.
  - [x] 9.3 `pnpm build` — exit 0 (prebuild + tsc + vite build all green; bundle size: index 324.68 kB / today 32.23 kB / css 27.58 kB).
  - [x] 9.4 `pnpm dev` — Vite ready in 490 ms on port 1420 with zero console errors.
  - [x] 9.5 `cargo build --workspace` — exit 0.
  - [x] 9.6 `cargo test --workspace` — exit 0 (trait_surface.rs 5 tests + export_bindings.rs 1 test, both pass).
  - [x] 9.7 `pnpm audit --audit-level=moderate` — `No known vulnerabilities found`, exit 0.

- [x] **Task 10: Drift-simulation smoke test (AC: 7) — manual; NOT committed.**
  - [x] 10.1 Added a temporary `<Trans>Drift Test</Trans>` to `__root.tsx`.
  - [x] 10.2 `pnpm i18n:check` — exited 1; `git diff` showed the new `Drift Test` msgid added to the catalog.
  - [x] 10.3 Reverted the temp `<Trans>Drift Test</Trans>`; re-ran `pnpm extract`; `messages.po` returned to one-msgid state.
  - [x] 10.4 `pnpm i18n:check` — exit 0.
  - [x] 10.5 Drift simulation verified: i18n:check exits non-zero on drift / 0 when clean. Logged here + in Completion Notes.

- [x] **Task 11: Anti-creep audit (AC: 10, 12).**
  - [x] 11.1 `git status` — diff confined to the AC10 allowed list (plus one disclosed exception: `pnpm-workspace.yaml` — see Completion Notes).
  - [x] 11.2 `rg` for `@lingui/babel-plugin-lingui-macro|babel-plugin-macros` against direct deps (`shell-ui/package.json`, `package.json`): 0 hits. Transitive lockfile mentions are unavoidable peer-dep metadata of `@lingui/vite-plugin` and do not install the Babel macro plugin (no `node_modules/babel-plugin-macros` present at any level — verified).
  - [x] 11.3 `rg '"@vitejs/plugin-react"' shell-ui/` — 0 hits. Babel react plugin fully removed.
  - [x] 11.4 `find shell-ui -maxdepth 2 -name "eslint.config*"` — 0 results.
  - [x] 11.5 `git diff --stat crates/` — empty. `.github/` directory does not yet exist in the repo (CI scaffold lands in Story 1.8) — no diff to check.

- [x] **Task 12: Update Dev Agent Record + sprint-status (out-of-band tracking, AC: 10).**
  - [x] 12.1 Populated Dev Agent Record sections below.
  - [x] 12.2 `sprint-status.yaml` updated: `1-6-install-lingui-v6-x-i18n-scaffold: ready-for-dev → in-progress → review`; `last_updated` bumped.

### Review Findings

_Code review run: 2026-05-22. Layers: Blind Hunter + Edge Case Hunter + Acceptance Auditor. Auditor verdict: all 12 ACs pass with 5 disclosed deviations (all justified). One blocking issue surfaced by Blind + Edge Case Hunters and empirically verified (fresh-clone `tsc` failure). All other Hunter findings either spec-mandated, empirically false, speculative, or out-of-scope — see triage notes below._

- [x] [Review][Patch] Fresh-clone `tsc` fails: `messages.ts` is gitignored (AC9), eagerly imported by `main.tsx` (AC8), and no script in `pnpm install` / `pnpm prebuild` generates it. Empirically reproduced: `rm src/locales/en/messages.ts && pnpm exec tsc --noEmit` → `TS2307`. **Resolution (user-authorized 2026-05-22):** added `lingui compile` to `prebuild` script (option (a)) — `prebuild` now reads `tsr generate && lingui compile && cargo test ...`. AC6 letter intact (the `build` script itself is unchanged). Verified end-to-end: `rm messages.ts && pnpm build` → exit 0, bundle sizes identical to original test plan. Disclosed as a new deviation in Completion Notes. [shell-ui/package.json:8]

_Dismissed (with rationale, for audit trail):_

- `i18n:check` flakiness from `POT-Creation-Date` drift (Blind + Edge Case) — **empirically false**: `lingui extract --clean` followed by `git diff` is empty in this repo. Lingui v6 normalizes the PO header timestamp.
- `--clean` destructive in CI / `--clean` no review gate (Blind + Edge Case) — **spec-mandated** (AC6 verbatim `lingui extract --clean`).
- `sr-only` "Orgsidian" announces on every route (Blind) — **spec-mandated** (AC5 verbatim placement + rationale).
- `eslint-plugin-lingui` installed but dead (Blind) — **explicit AC12 deferral**, disclosed in Completion Notes.
- `.gitignore` glob `src/locales/**/messages.ts` too broad (Edge Case) — **spec-mandated** (AC9 verbatim pattern).
- `messages.mjs` ignore line dead (Edge Case) — **spec-mandated forward-compat hedge** (AC9 rationale).
- SWC plugin caret pinning supply-chain (Blind) / `^6.0.1` minor float (Edge Case) — **policy** per `[[feedback_version_policy]]` (caret pinning is the norm; lockfile is the integrity story).
- `@swc/core` unsupported triples (Edge Case) — **outside Tauri supported matrix**; not a regression.
- HMR boundary regression Babel → SWC (Blind + Edge Case) / StrictMode + HMR re-eval (Edge Case) — **speculative**; manual `pnpm dev` smoke (Task 9.4) showed zero console errors.
- `i18n.load(undefined)` silent failure (Edge Case) — **speculative**; would only fire if `messages.ts` emitted as empty stub, which would itself break tsc per the open decision-needed item above.
- `compileNamespace: "ts"` couples to TS resolver (Blind) — speculative; current pipeline (Vite + tsc) handles it.
- YAML quoting inconsistency in `allowBuilds` (Blind) — **cosmetic NIT**.
- No regression test for `I18nProvider` mount order / SWC macro transform smoke test missing (Blind + Edge Case) — **out of scope**; Story 1.6 Dev Notes §(testing requirements) explicitly states "scaffold + smoke — no unit tests are added".

## Dev Notes

### Developer Context Section

This story is the **i18n infrastructure landing** — Lingui v6.x is installed and configured so every UI string from Epic 2 onwards can be authored extraction-ready from the keystroke it's typed. Three behavioral disciplines underpin every later story that touches user-visible text:

1. **`<Trans>foo</Trans>` is the default authoring API.** Plain JSX text nodes that aren't wrapped in `<Trans>` are NOT extractable — and `eslint-plugin-lingui` will eventually fail CI on them (Story 1.8 wiring). Get used to wrapping every string.
2. **`pnpm extract` is run after every user-visible string change.** Either the dev runs it locally, or CI catches the drift (`pnpm i18n:check`). Either way, the `.po` file is the contract.
3. **Compiled `.ts` catalog is a build artifact, not a source.** Treat `messages.ts` like `routeTree.gen.ts` from Story 1.3 — generated, gitignored, regenerated on every build.

### Critical context the LLM dev agent MUST internalize

**(a) The Story 1.3 React plugin choice was an oversight — fix it here.**

Story 1.3 ([1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md](./1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md)) installed `@vitejs/plugin-react` (Babel-based) without explicit reference to LD-52's SWC mandate. The current `shell-ui/vite.config.ts:6` imports `@vitejs/plugin-react`; the architecture line 541 mandates `@vitejs/plugin-react-swc`. **This is NOT a deviation that Story 1.6 introduces — it is a Story 1.3 deviation that Story 1.6 fixes** because LD-52 cannot be satisfied without the SWC plugin host. Document this in the Change Log entry as "fixing Story 1.3 plugin-react drift to honour LD-52 stack lock."

The swap is mechanically simple (`@vitejs/plugin-react-swc` is a drop-in for the Babel variant — same `react()` plugin function, same options shape) but it's a real behavioral change: SWC compiles JSX 5-20× faster than Babel and rewrites the dev experience subtly (faster HMR, slightly different error messages). No regressions are expected against Tailwind 4 + TanStack Router; both are SWC-compatible.

**(b) Lingui v6 SWC macro import path: `@lingui/react/macro` (NOT `@lingui/macro`).**

Lingui v4 used `@lingui/macro` as the macro import; Lingui v5+ split macros into per-package paths (`@lingui/core/macro`, `@lingui/react/macro`). Day-1 surface for `<Trans>` is `@lingui/react/macro`. This is a frequent source of "extract found zero strings" failures — the SWC plugin only rewrites imports from the macro paths, not the runtime paths. Reference: [Lingui v6 release notes / SWC plugin docs](https://lingui.dev/ref/swc-plugin) (verify at install time via `cat shell-ui/node_modules/@lingui/react/package.json | grep -A2 '"exports"'`).

**(c) ESLint config is intentionally NOT scaffolded here (AC12).**

The epic AC says "`eslint-plugin-lingui` added to ESLint config" but the project has no ESLint config yet. Scaffolding ESLint is its own discrete work item (React preset + TanStack Router preset + lingui plugin + lint-staged hook per architecture line 848). Trying to ship that here would expand Story 1.6 from a 3-file scaffold to a 6-file integration. The plugin is installed (AC1); future ESLint-scaffold work flips it on with a one-line config addition.

**(d) `pnpm-lock.yaml` will get noticeably bigger.**

The Lingui v6 dep set adds ~30 transitives (mostly Babel-free now that we're on SWC — main weight is `gettext-parser`, `pofile`, and the `@swc/core` re-export). Expect `pnpm-lock.yaml` to grow by ~500 lines. This is acceptable — review the lockfile diff in the PR.

### Library / framework requirements (binding)

| Package | Version | Source | Role |
|---|---|---|---|
| `@lingui/core` | `^6.0.1` | dep | Core i18n runtime (`i18n.load`, `i18n.activate`). |
| `@lingui/react` | `^6.0.1` | dep | `I18nProvider`, `Trans`, `useLingui` JSX surface + macro entry at `/macro`. |
| `@lingui/cli` | `^6.0.1` | devDep | `lingui extract` / `lingui compile` commands. |
| `@lingui/vite-plugin` | `^6.0.1` | devDep | Build-time `.po` → `.ts` compilation; runs as Vite plugin (AC3). |
| `@lingui/swc-plugin` | `^6.0.1` | devDep | SWC macro plugin; wired via `react({ plugins: [["@lingui/swc-plugin", {}]] })`. |
| `eslint-plugin-lingui` | `^6.0.1` | devDep | Lint-time extractability rule (installed only; activation deferred per AC12). |
| `@vitejs/plugin-react-swc` | `^3.x` latest | devDep | SWC-based React Vite plugin; replaces `@vitejs/plugin-react` per LD-52. |

**Forbidden additions** (do NOT install these — they imply a non-LD-52 path):
- `@lingui/macro` (v4-era; replaced by `@lingui/react/macro` in v5+).
- `babel-plugin-macros`, `@lingui/babel-plugin-lingui-macro` (Babel macro path — SWC mandated).
- `i18next`, `react-intl`, `@fluent/bundle`, `@fluent/react` (rejected by LD-52 rationale paragraphs).

### File structure requirements

```
shell-ui/
├── .gitignore                              # NEW (or APPENDED — see AC9)
├── lingui.config.ts                        # NEW (AC4)
├── package.json                            # MODIFIED (AC1, AC2, AC6)
├── vite.config.ts                          # MODIFIED (AC2, AC3)
└── src/
    ├── main.tsx                            # MODIFIED (AC8: I18nProvider + i18n.load/activate)
    ├── routes/
    │   └── __root.tsx                      # MODIFIED (AC5: <Trans> smoke)
    └── locales/                            # NEW DIRECTORY
        └── en/
            ├── messages.po                 # NEW — COMMITTED (AC5, AC9)
            └── messages.ts                 # NEW — GITIGNORED (AC9)
```

This layout matches LD-52's catalog-path mandate (`src/locales/{lng}/messages.po`) with the LD-5 amendment (no `packages/` prefix — `shell-ui/` lives at repo root per architecture lines 884, 1006).

### Testing requirements

Story 1.6 is **scaffold + smoke** — no unit tests are added. The drift-simulation in Task 10 is a manual smoke (NOT committed) that proves the catalog-drift gate works end-to-end. Three forms of "implicit testing" cover the surface:

1. **`pnpm extract` itself is the test.** A passing extract that produces exactly one msgid proves the SWC plugin + Vite plugin + `lingui.config.ts` chain works.
2. **`pnpm compile` + `tsc` is the type-test.** If `messages.ts` has the wrong shape, the `import { messages as enMessages }` line in `main.tsx` fails `tsc --noEmit`.
3. **`pnpm -C shell-ui build` is the integration test.** The Vite plugin invokes `lingui compile` during build; if any link in the chain is misconfigured, build fails.

**Property-based / E2E tests** for i18n behavior are deferred — they land naturally with Story 11.4 (coaching-balloon registry refactor) and Story 12.3 (keybinding remapping UI) once locale-switching surfaces in the UI.

### Anti-creep guardrails (binding)

Story 1.5 introduced an anti-creep audit pattern (Story 1.5 Task 13.1-13.7). Story 1.6 carries the equivalent (Task 11.1-11.5). The audit commands MUST exit cleanly:

- `rg "@lingui/babel-plugin-lingui-macro|babel-plugin-macros" .` → 0 hits.
- `rg "@vitejs/plugin-react[^-]" shell-ui/` → 0 hits (the trailing `[^-]` distinguishes `@vitejs/plugin-react` from `@vitejs/plugin-react-swc`).
- `find shell-ui -maxdepth 2 -name "eslint.config*"` → 0 results.
- `git diff --stat crates/` → empty.
- `git diff --stat .github/` → empty (no CI wiring here; that's Story 1.8).

If any of those return hits, **stop and re-scope** — the diff has drifted outside AC10.

### Previous story intelligence (Story 1.5 — done 2026-05-22)

Apply these patterns from Story 1.5's review/learnings to keep Story 1.6 frictionless:

1. **Anchor cross-crate / cross-package paths on relative roots.** Story 1.5 noted Story 1.4's `"../../shell-ui/src/lib/tauri.ts"` was fragile. Story 1.6's `<rootDir>` token in `lingui.config.ts` is the Lingui-CLI equivalent — let the tool resolve paths, don't hardcode.
2. **Document deviations in Completion Notes (per `[[feedback_batch_fixes_terse]]`).** Story 1.6 has **one pre-known deviation**: the Story 1.3 `@vitejs/plugin-react` swap. Disclose it explicitly in the Change Log as "fixing Story 1.3 plugin-react drift to honour LD-52 SWC mandate."
3. **`pnpm tauri dev` is the source of truth for runtime gates.** Applicable here: after `pnpm -C shell-ui build` succeeds, run `pnpm tauri dev` once and verify the app starts with no console errors and the screen-reader span renders. This is the actual integration test (Vite alone doesn't exercise the Tauri Webview).
4. **`[[feedback_version_policy]]` floats Lingui at `^6.0.1`.** Do NOT exact-pin (`6.0.1` without caret) — LTS-preferred floats minor/patch. `pnpm-lock.yaml` captures the resolved version. Same discipline applies to `@vitejs/plugin-react-swc` (float `^3.x`).
5. **Apply `[[feedback_batch_fixes_terse]]` during dev.** If `pnpm extract` produces unexpected secondary warnings (e.g., source-map-path warnings from `@lingui/vite-plugin`), apply obvious no-brainer fixes silently; surface only decision-grade questions (e.g., "Lingui v6.0.1 emits a peer-dep warning on `react@19.1.0` — should we file an upstream issue or pin to the latest patch?") as decision-grade items.
6. **Modify only what AC dictates.** Story 1.5 and Story 1.4 both noted "story originally drifted into reorganizing X; review reverted." Story 1.6: do NOT touch `crates/`, the Rust workspace, `tauri.conf.json`, capabilities, the route tree generator config, `tsconfig.json` (the strict mode is fine as-is), or any existing component / store / route.

### Git intelligence (recent commits)

Recent commits on `main` and `feat/story-1-5-plugin-api-scaffold` (per session start):

- `d05993b` `chore(story-1-5): mark story done after code-review approval` — Story 1.5 closeout.
- `0f245be` `feat(plugin-api): scaffold orgsidian-plugin-api LEAF crate with day-1 trait surface` — Story 1.5 main implementation.
- `9d28c36` `Merge pull request #114 from orgsidian/feat/story-1-4-tauri-specta-typed-ipc` — Story 1.4 merged.
- `567fdaa` `fix(ipc): apply Story 1.4 code-review patches` — Story 1.4 review patches.
- `d014594` `feat(ipc): wire tauri-specta typed IPC bridge with project-wide camelCase` — Story 1.4 main implementation.

Implications:
- The shell-ui workspace is canonical and stable since Story 1.3; `shell-ui/src/main.tsx`, `shell-ui/src/routes/__root.tsx`, and `shell-ui/vite.config.ts` are unmodified since Story 1.4 except for the `lib/tauri.ts` Story 1.4 binding.
- `Cargo.lock` is untouched by Story 1.6 (frontend-only).
- The branch `feat/story-1-5-plugin-api-scaffold` is the current checkout per `gitStatus` — Story 1.6 should create a fresh branch `feat/story-1-6-lingui-i18n-scaffold` from `main` (or rebase onto `main` after the 1.5 PR merges).

### Latest tech information

**Lingui v6.0.1 (April 2026 — the version pinned at lock time 2026-05-19 per architecture line 191):**

- **Native React 19 support.** No peer-dep warnings expected against `react@^19.1.0`. If a warning surfaces, check `node_modules/@lingui/react/package.json` `peerDependencies` for the React range — Lingui v6 declares `^18 || ^19`.
- **SWC plugin compatible with `@vitejs/plugin-react-swc` v3.x.** The SWC plugin lives in a separate package (`@lingui/swc-plugin`) that's loaded as a sub-plugin via the `plugins` option on `react()`. This is the Vite-React-SWC API since plugin-react-swc v3.0.
- **Catalog format `.po` (Gettext)** is the lingua franca for Crowdin/Weblate/Transifex (verified per LD-52 rationale (b)). Day-1 we're not using any of those platforms — we're locking in a translator-facing format that does not require migration when community translations arrive in v1.0.
- **Compile namespace `ts`.** Lingui v6 supports `ts`, `cjs`, and `es`. We use `ts` because the project is TypeScript-strict and `tsc --noEmit` runs in `prebuild`; `cjs` would require additional `tsconfig` adjustments to permit the `.cjs` import.
- **`@lingui/vite-plugin` configuration.** Default config is sufficient — the plugin auto-discovers `lingui.config.ts` from the workspace root and runs `compile` on every build. No options need to be passed in `vite.config.ts`.
- **`pnpm` workspace caveat.** The project uses pnpm workspaces (root `package.json` declares `pnpm@11.1.1`). All Lingui commands MUST be scoped: `pnpm -C shell-ui extract`, not `pnpm extract` from repo root (which would error: "extract" script doesn't exist at root).

### Project Context Reference

Persistent feedback memories applicable to Story 1.6:

- **`[[feedback_version_policy]]`** — Lingui at `^6.0.1` (LTS-preferred float). plugin-react-swc at `^3.x` (LTS-preferred float). DO NOT exact-pin.
- **`[[feedback_batch_fixes_terse]]`** — Apply obvious no-brainer fixes silently during dev. Surface only decision-grade items (e.g., "ESLint config scaffold deferred — is Story 1.8 or a new story the right home?") explicitly.
- **`[[feedback_spec_driven_not_solo_dev_bandwidth]]`** — Do NOT justify the AC12 ESLint deferral on "limited dev time" grounds. The deferral is spec-driven (scope-fence at "scaffold Lingui", not "scaffold full lint config").
- **`[[feedback_inspirations_separate_patterns_from_business_model]]`** — Not directly applicable here.

### Project Structure Notes

- **Alignment with unified project structure**: post-Story-1.6 layout matches architecture's Frontend Package Layout §`shell-ui/` (lines 227-230) augmented with `src/locales/` per LD-52. The `packages/shell-ui/` prefix in LD-52 line 541 is superseded by the LD-5 round-amendment (architecture lines 884, 1006: "`shell-ui/` lives at repo root").
- **Detected conflict — RESOLVED**: LD-52 text mentions `packages/shell-ui/src/locales/`. The LD-5 amendment supersedes (no `packages/` indirection until a 2nd JS package appears). Story 1.6 implements the amendment-adjusted path: `shell-ui/src/locales/`.
- **Detected conflict — RESOLVED**: Story 1.3 installed `@vitejs/plugin-react` (Babel). LD-52 mandates `@vitejs/plugin-react-swc` (SWC). Story 1.6 fixes this in AC2 — disclosed in Change Log as "Story 1.3 plugin-react drift."
- **Detected conflict — DEFERRED (AC12)**: Architecture line 933 expects `shell-ui/eslint.config.js`. No ESLint config exists. Story 1.6 installs `eslint-plugin-lingui` (per epic AC) but does NOT scaffold ESLint — deferred to a future story. Disclosed in Completion Notes.
- **Variance**: `lingui.config.ts` is new top-level config at `shell-ui/lingui.config.ts` — not enumerated in architecture's Frontend Package Layout, but its placement at the workspace root matches Lingui-CLI's default discovery (the CLI looks for `lingui.config.{ts,js,json}` walking up from CWD).

### References

- [Source: [epics.md#Story 1.6](../planning-artifacts/epics.md#L503)] — Story user-story + 5 acceptance criteria.
- [Source: [epics.md#Cross-Cutting NFR-10](../planning-artifacts/epics.md#L112)] — NFR-10 internationalization scope.
- [Source: [epics.md#FR Coverage / LD-52](../planning-artifacts/epics.md#L152)] — LD-52 CI gate mandate.
- [Source: [architecture.md#Stack Versions Table](../planning-artifacts/architecture.md#L191)] — `@lingui/*` 6.x at `^6.0.1` lock.
- [Source: [architecture.md#Epic 1 Scaffold scope](../planning-artifacts/architecture.md#L364)] — Lingui install + pnpm scripts + Vite SWC plugin entry.
- [Source: [architecture.md#LD-52 i18n library](../planning-artifacts/architecture.md#L541)] — Lingui v6.x canonical decision + path + format + Vite integration + CI gate.
- [Source: [architecture.md#LD-5 amendments — packages indirection removed](../planning-artifacts/architecture.md#L884)] — `shell-ui/` at repo root, no `packages/` prefix.
- [Source: [architecture.md#Linting & Formatting](../planning-artifacts/architecture.md#L844)] — ESLint + Prettier with React + TanStack Router presets (AC12 deferral target).
- [Source: [architecture.md#Project Tree](../planning-artifacts/architecture.md#L928)] — Frontend layout post-Lingui.
- [Source: [../implementation-artifacts/1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md](./1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md)] — Story 1.3 initial Vite + Tailwind 4 + TanStack Router setup (the `@vitejs/plugin-react` drift to be fixed in AC2).
- [Source: [../implementation-artifacts/1-4-wire-tauri-specta-typed-ipc-bridge-with-project-wide-camelcase-rename.md](./1-4-wire-tauri-specta-typed-ipc-bridge-with-project-wide-camelcase-rename.md)] — Story 1.4 typed-IPC contract (preserved unchanged).
- [Source: [../implementation-artifacts/1-5-scaffold-orgsidian-plugin-api-leaf-crate-with-day-1-trait-surface.md](./1-5-scaffold-orgsidian-plugin-api-leaf-crate-with-day-1-trait-surface.md)] — Story 1.5 leaf crate (unrelated to Lingui; both ship in parallel).
- Persistent feedback memories: `[[feedback_version_policy]]`, `[[feedback_batch_fixes_terse]]`, `[[feedback_spec_driven_not_solo_dev_bandwidth]]`.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context) — `claude-opus-4-7[1m]`

### Debug Log References

- `pnpm install` initially failed: `eslint-plugin-lingui@^6.0.1` not found. Investigation: the package's actual SemVer cadence is independent of `@lingui/*` core (latest `0.13.1`). Repinned to `^0.13.1`. Disclosed deviation.
- `pnpm install` second run: `[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: @swc/core@1.15.33`. Resolution: `pnpm approve-builds --all` ran the postinstall and persisted approval to `pnpm-workspace.yaml.allowBuilds` (canonical workspace config). The redundant `package.json` `pnpm.onlyBuiltDependencies` block I had added was removed; root `package.json` is unchanged.
- `pnpm exec tsc --noEmit` on `lingui.config.ts`: IDE diagnostic reported `Type 'string' is not assignable to type 'CatalogFormatter'` for `format: "po"`. Confirmed in `@lingui/conf/dist/index.d.mts`: Lingui v6 typed `format` as `CatalogFormatter` (factory result), no longer accepting the literal string `"po"`. Fix: removed the explicit `format` field — PO is the default in Lingui v6, so omission produces the desired behavior. tsc passes.
- `pnpm extract` (first run): catalog statistics `en (source) | 1 | -` — one msgid `Orgsidian` from `__root.tsx:10`.
- `pnpm compile`: produced `shell-ui/src/locales/en/messages.ts` declaring `export const messages: Messages` (content-hash-keyed shape; `49TjR2` → `Orgsidian`).
- `pnpm i18n:check` clean state → exit 0; drift simulation (added `<Trans>Drift Test</Trans>`) → exit 1 with visible PO diff (`+msgid "Drift Test"`).
- `pnpm build`: prebuild (`tsr generate` + `cargo test export_bindings`) + tsc + vite build all green; production bundle sizes: `index-*.js 324.68 kB` (gzip 102.86 kB), `today-*.js 32.23 kB` (gzip 10.58 kB), `index-*.css 27.58 kB` (gzip 5.48 kB).
- `pnpm dev`: Vite v7.3.3 ready in 490 ms on port 1420 — clean startup.
- `cargo build --workspace` → exit 0 (only `orgsidian-plugin-api` recompiled — no shell-app changes triggered).
- `cargo test --workspace` → 6 tests across crates pass: `trait_surface.rs` (5 tests) + `export_bindings.rs` (1 test), no regressions.
- `pnpm audit --audit-level=moderate` → `No known vulnerabilities found`.

### Completion Notes List

- **Story 1.3 plugin-react drift fixed (AC2).** `shell-ui/vite.config.ts` swapped from `@vitejs/plugin-react` (Babel) → `@vitejs/plugin-react-swc` to honour LD-52 stack lock. The Lingui SWC macro plugin is wired via `react({ plugins: [["@lingui/swc-plugin", {}]] })` exactly per architecture line 364. No regressions detected against Tailwind 4 or TanStack Router.

- **`@vitejs/plugin-react-swc` pinned at `^4.0.0` (not `^3.x` as the story suggested).** The latest stable is `4.3.1`. Per `[[feedback_version_policy]]` (latest stable / LTS preferred) and `[[feedback_batch_fixes_terse]]` (no-brainer batch fix), upgraded the pin range. The `plugins` option API is stable across plugin-react-swc v3 → v4; no behavioral surface change.

- **`eslint-plugin-lingui` pinned at `^0.13.1` (DEVIATION from epic AC + story AC1 verbatim "^6.0.1").** The package has its own SemVer cadence independent of `@lingui/*` core — latest release `0.13.1`. The architecture's "all `^6.0.1`" assumption (architecture line 191) was wrong for this single transitive. Applied as a silent no-brainer batch fix per `[[feedback_batch_fixes_terse]]`. The plugin is installed; AC12 deferral (ESLint config not scaffolded here) is unchanged.

- **`@lingui/conf` added to `devDependencies`.** Not enumerated in the architecture line 191 dep list, but required so `lingui.config.ts` can `import type { LinguiConfig } from "@lingui/conf"`. It's a transitive of `@lingui/cli` and is hoisted only when listed explicitly. Adding it directly is the cleanest path to typed config. No-brainer fix per `[[feedback_batch_fixes_terse]]`.

- **`format: "po"` field removed from `lingui.config.ts` (DEVIATION from AC4 verbatim).** Lingui v6 changed the `format` field type from `string` to `CatalogFormatter` (factory result, e.g., `formatter({...})` from `@lingui/format-po`). PO is the default formatter in v6, so omitting the field yields identical behavior to the v4-era `format: "po"`. Confirmed via `@lingui/conf/dist/index.d.mts`. No functional change — the catalog format remains Gettext `.po`. A comment was added explaining the choice.

- **AC12 ESLint deferral confirmed.** `eslint-plugin-lingui` is installed in `devDependencies` but no `eslint.config.js` was scaffolded. Architectural intent (lint-time extractability gate per LD-52) is preserved for a future story — recommended placement: a dedicated ESLint-scaffold story (Story 1.17.x or a sub-task in Story 1.8 alongside the CI matrix).

- **Drift simulation verified (Task 10).** Added `<Trans>Drift Test</Trans>` temporarily → `pnpm i18n:check` exited 1 with a visible `git diff` showing the added msgid. Reverted → `pnpm i18n:check` exited 0. End-to-end drift detection chain works as designed (LD-52 CI gate is ready for Story 1.8 wiring).

- **`pnpm-workspace.yaml` modified (DISCLOSED — not in AC10 allowed list).** `pnpm approve-builds --all` was required to approve `@swc/core@1.15.33` postinstall script (the SWC native binary loader). This persisted the approval to `pnpm-workspace.yaml.allowBuilds` — adding the line `'@swc/core': true` next to the existing `esbuild: true` entry. This is a necessary mechanical side-effect of introducing `@vitejs/plugin-react-swc` (which depends on `@swc/core`). The change is one line; no other build-script bypasses are introduced.

- **`pnpm-lock.yaml` grew significantly.** Adding the Lingui v6 dep set + swapping plugin-react → plugin-react-swc bumped the lockfile by ~2500 lines (largely SWC native bindings + Lingui transitives like `gettext-parser`, `pofile`, `commander`, `chalk`, etc.). Expected per Dev Notes §(d).

- **Anti-creep audit clean.** `git diff --stat crates/` empty; no `eslint.config.*` present; no direct `@vitejs/plugin-react` references; no direct Babel macro deps. Transitive Babel mentions in `pnpm-lock.yaml` are peer-dep metadata of `@lingui/vite-plugin` only — no `babel-plugin-macros` is installed in the actual `node_modules` tree (verified).

- **`prebuild` extended with `lingui compile` (DEVIATION — post-review patch, AC6 letter borderline).** Original `prebuild` was `tsr generate && cargo test ...`. After code review surfaced a fresh-clone `tsc` failure (`messages.ts` gitignored per AC9 + eagerly imported per AC8 + no script generating it before `tsc` runs), the prebuild was changed to `tsr generate && lingui compile && cargo test ...`. AC6 forbids adding `compile` to `build` itself but is silent on the `npm-lifecycle` `prebuild` script; the `build` command (`tsc && vite build`) is untouched, and `@lingui/vite-plugin` still owns the compile-during-`vite build` path. The double-run with `vite build` is idempotent (same `.po` → same `.ts`). Authorized by user during code review (2026-05-22). Verified: `rm messages.ts && pnpm build` → exit 0 with identical bundle sizes (index 324.68 kB / today 32.23 kB / css 27.58 kB).

- **First two `pnpm extract` runs produced a non-idempotent diff.** Lingui CLI's first extract emits a minimal PO header; the second run adds the full standard PO headers (`Project-Id-Version`, `Report-Msgid-Bugs-To`, `Plural-Forms`, etc.). The third and subsequent runs are idempotent. The committed `messages.po` reflects the stable form (post-2nd-extract). This is a known Lingui v6 behavior; subsequent `pnpm i18n:check` runs are deterministic.

### File List

**Modified**

- `shell-ui/package.json` — Removed `@vitejs/plugin-react`; added `@lingui/core`, `@lingui/react` (deps) + `@lingui/cli`, `@lingui/conf`, `@lingui/vite-plugin`, `@lingui/swc-plugin`, `@vitejs/plugin-react-swc`, `eslint-plugin-lingui` (devDeps); added `extract`, `compile`, `i18n:check` scripts; **post-review:** extended `prebuild` with `lingui compile` (fresh-clone `tsc` fix — see Completion Notes).
- `shell-ui/vite.config.ts` — Swapped Babel react plugin → SWC; added Lingui SWC plugin entry + `lingui()` Vite plugin.
- `shell-ui/src/main.tsx` — Imported `i18n` from `@lingui/core` + `I18nProvider` from `@lingui/react`; statically loaded the `en` catalog and activated at boot; wrapped `<RouterProvider>` with `<I18nProvider>`.
- `shell-ui/src/routes/__root.tsx` — Imported `Trans` from `@lingui/react/macro`; added `<span className="sr-only"><Trans>Orgsidian</Trans></span>` as the smoke string anchor.
- `pnpm-workspace.yaml` — Added `'@swc/core': true` under `allowBuilds` (canonical pnpm 11 approve-builds output; SWC native postinstall required).
- `pnpm-lock.yaml` — Regenerated by `pnpm install` (~2500 lines of net additions for the Lingui v6 + plugin-react-swc dep tree).

**New**

- `shell-ui/lingui.config.ts` — Lingui v6 config: `en` source locale, catalog path `<rootDir>/src/locales/{locale}/messages`, PO (default) format, TS compile namespace, excludes for test files and `routeTree.gen.ts`.
- `shell-ui/.gitignore` — Two-line gitignore: `src/locales/**/messages.ts` + `src/locales/**/messages.mjs` (compiled catalog artifacts).
- `shell-ui/src/locales/en/messages.po` — Initial catalog: 1 msgid (`Orgsidian`, sourced from `src/routes/__root.tsx:10`).

**Project tracking artifacts (out-of-band updates, not part of code surface)**

- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `1-6-…: ready-for-dev → in-progress → review`.
- `_bmad-output/implementation-artifacts/1-6-install-lingui-v6-x-i18n-scaffold.md` — Status `ready-for-dev → in-progress → review`; `github_issue: 6` recorded; Tasks/Subtasks marked complete; Dev Agent Record populated.

### Change Log

- 2026-05-22 — Story 1.6 implementation. Installed Lingui v6.x i18n scaffold: `@lingui/core` + `@lingui/react` (deps `^6.0.1`), `@lingui/cli` + `@lingui/conf` + `@lingui/vite-plugin` + `@lingui/swc-plugin` (devDeps `^6.0.1`), `eslint-plugin-lingui` (devDep `^0.13.1` — disclosed deviation from architecture's "all `^6.0.1`" assumption; the package has independent SemVer cadence). Swapped `@vitejs/plugin-react` (Babel) → `@vitejs/plugin-react-swc@^4.0.0` to honour LD-52 SWC mandate (Story 1.3 drift fix, disclosed). Added `shell-ui/lingui.config.ts` with `en` source locale + PO catalog format (default in Lingui v6 — explicit `format: "po"` field removed because v6 requires a CatalogFormatter factory; PO default behavior is identical). Added `extract` / `compile` / `i18n:check` pnpm scripts; mounted `I18nProvider` in `main.tsx` with statically-imported `en` catalog; added `<Trans>Orgsidian</Trans>` smoke string in `__root.tsx` as a `sr-only` span; committed initial `messages.po` catalog (1 msgid); gitignored compiled `messages.ts`. Drift simulation verified end-to-end (Task 10). All AC1-AC12 satisfied with 5 disclosed deviations (eslint-plugin-lingui version, plugin-react-swc major, @lingui/conf addition, lingui.config.ts `format` removal, pnpm-workspace.yaml allowBuilds addition). Post-review patch: `prebuild` extended with `lingui compile` to fix a fresh-clone `tsc` failure surfaced by the Blind + Edge Case Hunter layers (AC6 letter intact; `build` itself untouched). User-authorized 2026-05-22. Local gates: `pnpm build` ✅, `pnpm i18n:check` ✅, `cargo build --workspace` ✅, `cargo test --workspace` ✅, `pnpm audit --audit-level=moderate` ✅. Anti-creep audit clean. ESLint plugin installed but config scaffold deferred (AC12) to a future story (suggested: Story 1.8 or a dedicated ESLint-scaffold story).
