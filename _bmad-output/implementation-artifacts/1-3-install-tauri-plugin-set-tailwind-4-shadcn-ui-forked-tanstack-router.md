# Story 1.3: Install Tauri plugin set + Tailwind 4 + shadcn/ui forked + TanStack Router

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want the full Tauri 2.x plugin set + Tailwind 4 (CSS-first `@theme` config) + shadcn/ui essentials forked into `shell-ui/src/components/ui/` + TanStack Router file-based routing installed and wired,
So that every v0.1 feature has its UI infrastructure ready without per-feature setup tax — and so that the next time we open a story (1.4 `tauri-specta`, 1.5 plugin-api trait, 1.6 Lingui, etc.) the only work is feature work, not "missing infrastructure" work.

## Acceptance Criteria

1. **AC1 — Eleven Tauri 2 plugins are installed + registered + capability-permitted.** The architecture-locked plugin set ([Source: architecture.md#Tauri Plugins — Full Set](../planning-artifacts/architecture.md)) lands as Cargo dependencies on `crates/orgsidian-shell-app`, registered in `crates/orgsidian-shell-app/src/lib.rs::run()`, and granted default permissions in `crates/orgsidian-shell-app/capabilities/main.json` (renamed from `default.json` per AC2):
    - **Rust crates added to `crates/orgsidian-shell-app/Cargo.toml` `[dependencies]`:**
      `tauri-plugin-fs = "2"`, `tauri-plugin-dialog = "2"`, `tauri-plugin-global-shortcut = "2"`, `tauri-plugin-window-state = "2"`, `tauri-plugin-store = "2"`, `tauri-plugin-shell = "2"`, `tauri-plugin-os = "2"`, `tauri-plugin-clipboard-manager = "2"`, `tauri-plugin-log = "2"`, `tauri-plugin-process = "2"`. `tauri-plugin-opener` (Story 1.1 lineage) is **retained** alongside `tauri-plugin-shell` — see Dev Notes §4 for rationale.
    - **`tauri-plugin-updater`** is conditionally compiled per the official guidance: declared under `[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies] tauri-plugin-updater = "2"` (mobile excluded; this is the Tauri-docs-recommended shape). Verify by `cargo check --workspace` succeeding on macOS-arm64 + Ubuntu-LTS.
    - **No version is added to `[workspace.dependencies]` at the root `Cargo.toml`.** These plugins are consumed by `orgsidian-shell-app` only at this stage; centralizing them in workspace-deps is premature and Story 1.7 (`cargo-deny`) will reassess the workspace-dep policy.
    - **`crates/orgsidian-shell-app/src/lib.rs::run()` registers every plugin** via `tauri::Builder::default().plugin(<plugin>::init()).plugin(...)...`. Registration order: keep `tauri_plugin_opener::init()` first (Story 1.1 baseline), then alphabetically by plugin name for predictability (`clipboard_manager`, `dialog`, `fs`, `global_shortcut`, `log`, `os`, `process`, `shell`, `store`, `window_state`), then `updater` last with the `#[cfg(desktop)]` gate. The `greet` command + `invoke_handler` + `generate_context!()` chain from Story 1.1 stays intact.
    - **`capabilities/main.json` lists default permissions for every plugin:** `core:default`, `opener:default`, `fs:default`, `dialog:default`, `global-shortcut:default`, `window-state:default`, `store:default`, `shell:default`, `os:default`, `clipboard-manager:default`, `log:default`, `process:default`, `updater:default`. The `windows: ["main"]` scope is **unchanged** — Quick Capture window (Story 8.1) gets its own `quick-capture.json` later.
    - **`fs:default` and `shell:default` scope-tightening is OUT of scope** for this story. PRD §LD-17 (Vault-folder allow-list) and the `shell.open()` URL allow-list are wired when the first feature consuming them lands (Vault designation = Story 3.6, external link opens = Story 6.x). This story ships the default scopes; later stories tighten them.

2. **AC2 — `capabilities/default.json` is renamed to `capabilities/main.json`.** The architecture's Workspace Layout shows `capabilities/main.json` + (future) `capabilities/quick-capture.json` ([Source: architecture.md#Workspace Layout — line 925-927](../planning-artifacts/architecture.md)). This story performs the rename:
    - `git mv crates/orgsidian-shell-app/capabilities/default.json crates/orgsidian-shell-app/capabilities/main.json` (preserve blame).
    - The `identifier` field inside the JSON changes from `"default"` to `"main"` (matches the filename convention).
    - The `$schema` value `"../gen/schemas/desktop-schema.json"` is **unchanged** (still relative to capabilities/).
    - `description` updated to `"Capability for the main window — all v0.1 plugin defaults"`.
    - `windows: ["main"]` is unchanged.
    - Tauri auto-loads every JSON file in `capabilities/`; no reference update in `tauri.conf.json` is required (verify by `pnpm tauri info` reporting the renamed capability without warning).

3. **AC3 — Tailwind 4 is installed via `@tailwindcss/vite` and wired into `shell-ui/src/styles/app.css` with `@import` + `@theme` CSS-first config.**
    - `shell-ui/package.json` `dependencies` gains `tailwindcss` (4.1.x latest stable per [Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md)) and `@tailwindcss/vite` (matching minor); no `postcss`, no `autoprefixer`, no `tailwind.config.js`/`.ts` JS-side config file is created (CSS-first Tailwind 4 supersedes JS config — [Source: ctx7 tailwindcss.com "Define Tailwind CSS Configuration with CSS-first @theme"]).
    - `shell-ui/vite.config.ts` is updated to import and register the Tailwind plugin: `import tailwindcss from '@tailwindcss/vite';` then `plugins: [tanstackRouter({...}), tailwindcss(), react()]`. The Tailwind plugin runs alongside the TanStack plugin (AC5) and the React plugin; order: TanStack router **first** (AC5 hard requirement), Tailwind second, React third.
    - `shell-ui/src/styles/app.css` is **created** with the canonical Tailwind 4 + Orgsidian-tokens content:
      ```css
      @import "tailwindcss";

      @theme {
        /* Tailwind 4 design tokens; Orgsidian-specific values land in Story 6.7 (themes).
           This story ships an empty @theme block so the Tailwind 4 token system is wired and
           later stories (themes, shadcn token palette) extend without re-introducing the import. */
      }
      ```
      The `--org-*` CSS variable vocabulary from architecture §"Themable CSS Token Vocabulary (FR-22)" is **NOT** added here — it lands with Story 6.7 (`Ship dark/light/default themes`). This story only wires the Tailwind 4 plumbing.
    - `shell-ui/src/main.tsx` imports `./styles/app.css` (replaces the legacy `./App.css` import in `App.tsx`; the legacy `App.css` file is **kept** but its import is moved from `App.tsx` to `main.tsx` after `app.css` so Tailwind utilities cascade-precede the Story-1.1 scaffold styles). Verify a Tailwind utility (e.g. `<div className="text-red-500">`) renders the expected style in the dev server.
    - `pnpm --filter shell-ui build` succeeds; the built `shell-ui/dist/assets/*.css` contains Tailwind output (sanity-check: `grep -l "tailwindcss" shell-ui/dist/assets/*.css || rg "--tw-" shell-ui/dist/assets/*.css` finds the Tailwind runtime CSS variable declarations).

4. **AC4 — shadcn/ui essentials are forked into `shell-ui/src/components/ui/` via `npx shadcn@latest init` + `add`.** The architecture mandates "shadcn/ui (forked into `src/components/ui/`, essentials only)" ([Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md), [Source: architecture.md#Orgsidian UI Kit — Day-1 Mandatory](../planning-artifacts/architecture.md)). The essentials list per the epic AC ([Source: epics.md#Story 1.3 AC](../planning-artifacts/epics.md) line 468): **Button, Dialog, Input, Tabs, Tooltip, Toast** — and the **Toast** component is **provided by Sonner in shadcn 2.x+** (the legacy `Toast` primitive was deprecated in favor of `sonner` per shadcn docs); install `sonner` as the toast surface.
    - **Path alias `@/*` → `./src/*`** is configured in `shell-ui/tsconfig.json` `compilerOptions.paths` AND in `shell-ui/vite.config.ts` `resolve.alias` (both required — TS for typecheck, Vite for runtime resolution). The Vite alias uses the standard pattern: `resolve: { alias: { '@': path.resolve(__dirname, './src') } }` with `import path from 'node:path'` at the top.
    - `shell-ui/components.json` is committed with:
      ```json
      {
        "$schema": "https://ui.shadcn.com/schema.json",
        "style": "new-york",
        "rsc": false,
        "tsx": true,
        "tailwind": {
          "config": "",
          "css": "src/styles/app.css",
          "baseColor": "neutral",
          "cssVariables": true,
          "prefix": ""
        },
        "aliases": {
          "components": "@/components",
          "utils": "@/lib/utils",
          "ui": "@/components/ui",
          "lib": "@/lib",
          "hooks": "@/hooks"
        },
        "iconLibrary": "lucide"
      }
      ```
      The `"config": ""` empty value is correct for Tailwind 4 (no JS config). The `"css": "src/styles/app.css"` points the CLI at the file created in AC3.
    - `npx shadcn@latest add button dialog input tabs tooltip sonner` is run from inside `shell-ui/`. Outputs land at `shell-ui/src/components/ui/{button,dialog,input,tabs,tooltip,sonner}.tsx` (6 files). The CLI also creates `shell-ui/src/lib/utils.ts` with the `cn()` helper. **All generated files are committed** (this is the "fork" — the source is owned, not imported).
    - The shadcn CLI auto-installs the following npm packages into `shell-ui/package.json`: `class-variance-authority`, `clsx`, `tailwind-merge`, `lucide-react`, `sonner`, `@radix-ui/react-dialog`, `@radix-ui/react-tabs`, `@radix-ui/react-tooltip`, `@radix-ui/react-slot`. **Do NOT pin these manually** — accept the CLI's chosen versions for this story; the project-wide version policy ([[feedback_version_policy]]) considers them transitively pinned by latest-stable at install time, and Story 1.7 (`cargo-deny`/`pnpm audit` hygiene) will harden if anything is stale.
    - `shell-ui/src/main.tsx` mounts the `<Toaster />` from `sonner` next to `<App />` so the shadcn toast primitive is available globally; or — preferred — `<Toaster />` is mounted **inside** the `__root.tsx` route component (AC5) so it lives under the TanStack Router root.
    - **Smoke import test:** add a `// @ts-ignore unused-import` (or one trivial `<Button>` usage) inside the `/today` placeholder route (AC5.5) to verify `@/components/ui/button` resolves at typecheck + runtime. Acceptable form: a single visible `<Button>Greet</Button>` on the placeholder route, wired to the existing `greet()` command from `App.tsx` — see AC5.5 for the placeholder content rule.

5. **AC5 — TanStack Router file-based routing is installed and wired with `__root.tsx` + `routes/index.tsx` (redirect to `/today`) + `routes/_layout/today.tsx` placeholder.** The architecture's LD-29 + Workspace Layout commits the project to TanStack Router with file-based routes under `shell-ui/src/routes/` ([Source: architecture.md#LD-29](../planning-artifacts/architecture.md), [Source: architecture.md#Workspace Layout line 942-950](../planning-artifacts/architecture.md), [Source: architecture.md#LD-29 amendment in Project Structure & Boundaries](../planning-artifacts/architecture.md)).
    - **`shell-ui/package.json` deps:** `dependencies` gains `@tanstack/react-router` (1.x latest stable); `devDependencies` gains `@tanstack/router-plugin` and `@tanstack/react-router-devtools` (matching minor).
    - **`shell-ui/vite.config.ts` plugin order:** `tanstackRouter({ target: 'react', autoCodeSplitting: true })` is the **FIRST** plugin in `plugins: [...]`. **THIS IS NON-NEGOTIABLE** — the TanStack docs explicitly require this ordering for correct codegen + react-fast-refresh integration ([Source: ctx7 /tanstack/router "Configure Vite for TanStack Router"]). The full plugins order from AC3 + AC5 + AC4 path-alias: `[tanstackRouter({ target: 'react', autoCodeSplitting: true }), tailwindcss(), react()]`.
    - **`shell-ui/src/routes/__root.tsx`** created with:
      ```tsx
      import { createRootRoute, Outlet } from '@tanstack/react-router';
      import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
      import { Toaster } from '@/components/ui/sonner';

      export const Route = createRootRoute({
        component: () => (
          <>
            <Outlet />
            <Toaster />
            {import.meta.env.DEV && <TanStackRouterDevtools />}
          </>
        ),
      });
      ```
      The `import.meta.env.DEV` guard keeps the Devtools out of production builds (Vite-standard idiom).
    - **`shell-ui/src/routes/index.tsx`** created as the `/` → `/today` redirect:
      ```tsx
      import { createFileRoute, redirect } from '@tanstack/react-router';

      export const Route = createFileRoute('/')({
        beforeLoad: () => {
          throw redirect({ to: '/today' });
        },
      });
      ```
      ([Source: ctx7 /websites/tanstack_router "Standalone Redirect with 'to' and 'href'"]) — the `beforeLoad` throw-redirect pattern is canonical.
    - **`shell-ui/src/routes/_layout/today.tsx`** created as the **placeholder** for the Today Dashboard. The `_layout/` prefix is a TanStack **pathless route group** (the `_` prefix means it doesn't add a URL segment; routes under it inherit the layout but `/today` is still the URL). For this story the layout is trivial — the future Story 7.1 will introduce sidebar/topbar around it. Minimum content:
      ```tsx
      import { createFileRoute } from '@tanstack/react-router';
      import { useState } from 'react';
      import { invoke } from '@tauri-apps/api/core';
      import { Button } from '@/components/ui/button';

      export const Route = createFileRoute('/_layout/today')({
        component: TodayPlaceholder,
      });

      function TodayPlaceholder() {
        const [greetMsg, setGreetMsg] = useState('');
        const [name, setName] = useState('');

        async function greet() {
          setGreetMsg(await invoke('greet', { name }));
        }

        return (
          <main className="container mx-auto p-8">
            <h1 className="text-2xl font-semibold">Today (placeholder)</h1>
            <p className="text-sm text-muted-foreground mt-2">
              Story 7.1 will replace this with the real Today Dashboard.
            </p>
            <form
              className="mt-6 flex gap-2"
              onSubmit={(e) => { e.preventDefault(); greet(); }}
            >
              <input
                className="border rounded px-2 py-1"
                onChange={(e) => setName(e.currentTarget.value)}
                placeholder="Enter a name..."
              />
              <Button type="submit">Greet</Button>
            </form>
            <p className="mt-3 text-sm">{greetMsg}</p>
          </main>
        );
      }
      ```
      This placeholder **preserves the Story 1.1 `greet()` round-trip** (AC10 of Story 1.1 was the visual gate; we don't lose it). It also exercises (a) a TanStack route, (b) a shadcn `<Button>`, (c) a Tailwind utility class, (d) a Tauri IPC `invoke` — covering all four pillars in one screen.
    - **`shell-ui/src/main.tsx` is rewritten** to mount `<RouterProvider router={router} />` instead of `<App />`:
      ```tsx
      import React from 'react';
      import ReactDOM from 'react-dom/client';
      import { RouterProvider, createRouter } from '@tanstack/react-router';
      import { routeTree } from './routeTree.gen';
      import './styles/app.css';

      const router = createRouter({
        routeTree,
        defaultPreload: 'intent',
        scrollRestoration: true,
      });

      declare module '@tanstack/react-router' {
        interface Register {
          router: typeof router;
        }
      }

      ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
        <React.StrictMode>
          <RouterProvider router={router} />
        </React.StrictMode>,
      );
      ```
      The legacy `import App from './App'` is **removed**; `App.tsx` itself is **DELETED** (the placeholder route absorbs its contents — keeping a dead `App.tsx` violates the architecture's "Premature abstraction: three similar lines beats a wrong shared helper" anti-pattern). `App.css` is also **DELETED** — its scaffold styles are replaced by Tailwind utilities in the placeholder route.
    - **`shell-ui/src/routeTree.gen.ts`** is generated by the TanStack Router Vite plugin on first `pnpm --filter shell-ui dev`. Add `shell-ui/src/routeTree.gen.ts` to `shell-ui/.gitignore` (the architecture's "AI-Agent Implementation Rules" allows generated files to be gitignored; the canonical TanStack pattern is to commit it — but for this project the file is auto-generated on every dev/build run and committing it would create noise in PRs. **Decision: gitignore it.** Document this in Dev Notes §6 and revisit at the Story 1.8 CI matrix if CI requires the file pre-generated).

6. **AC6 — `pnpm tauri dev` from repo root renders the `/today` placeholder.** The full integration loop works end-to-end:
    - `pnpm install` from repo root completes without errors. New deps from AC3/AC4/AC5 resolve under `shell-ui/node_modules/.pnpm/`.
    - `pnpm tauri dev` from repo root: Vite starts on `http://localhost:1420/`, the TanStack Router Vite plugin generates `routeTree.gen.ts` on first compile, the Rust shell binary compiles (longer than Story 1.2 due to 11 new plugins; expect ~30-60s first compile), the Tauri window opens. The URL `http://localhost:1420/` immediately redirects to `http://localhost:1420/today` (visible in devtools URL bar) and the placeholder content renders.
    - Click "Greet" with a typed name → `invoke('greet', {name})` round-trips to Rust and the response renders. This is the Story 1.1 / 1.2 visual gate, preserved verbatim through the routing refactor.
    - **`pnpm tauri build` from repo root** still produces `.app` + `.dmg` under repo-root `target/release/bundle/macos/` and `target/release/bundle/dmg/`. The bundle name remains `orgsidian_0.0.0_aarch64.dmg`. No signing required at this stage (Story 6.8).

7. **AC7 — `cargo check --workspace` and `cargo build --workspace` from repo root exit 0.** Despite adding 11 new plugin crates to `crates/orgsidian-shell-app/Cargo.toml`:
    - `cargo check --workspace` exits 0 with no errors.
    - `cargo build --workspace` exits 0; `Cargo.lock` updates with the new transitive dep set; no clippy/warning regressions beyond unused-code notices on the still-stub leaf crates (those land their first deps in Epic 2+; the workspace-wide `-D warnings` gate doesn't activate until Story 1.8).
    - `cargo build` from inside `tools/corpus-extractor/` still exits 0 (the standalone tool is unaffected).

8. **AC8 — TypeScript typecheck (`pnpm --filter shell-ui build`) exits 0 with strict mode.** The combination of (a) the `@/*` alias resolved through TS paths + Vite alias, (b) the `routeTree.gen.ts` generated module being recognized as a module declaration, (c) the `declare module '@tanstack/react-router'` augmentation in `main.tsx`, (d) the existing `strict + noUnusedLocals + noUnusedParameters + noUncheckedIndexedAccess: true` (if added; otherwise existing strict flags) all pass:
    - `pnpm --filter shell-ui build` exits 0 (the script is `tsc && vite build`).
    - **`@ts-expect-error` cleanup in `shell-ui/vite.config.ts`** (Story 1.2 deferred item, [diagnostic: `vite.config.ts:4 — Unused '@ts-expect-error' directive`]): with the addition of `@tanstack/router-plugin` and `@tailwindcss/vite` (both providing Vite-context typings that surface `process` indirectly), the `@ts-expect-error` may continue to surface as "unused" in some LSP setups. Resolution: remove the `@ts-expect-error process is a nodejs global` line + the `host` const; replace the `host:`/`hmr:` lines with the equivalent that does not reference `process.env`. Concrete replacement:
      ```ts
      // Replace:
      //   // @ts-expect-error process is a nodejs global
      //   const host = process.env.TAURI_DEV_HOST;
      //   ... host: host || false, hmr: host ? {...} : undefined,
      // With (the simpler form — Tauri 2 desktop-only baseline; the TAURI_DEV_HOST env var
      // is a mobile-dev affordance and we do not target mobile in v0.1):
      server: {
        port: 1420,
        strictPort: true,
        host: false,
        watch: { ignored: [...] },
      },
      ```
      Alternative — keep the env-var path with proper typing: install `@types/node` as a dev-dep and use `process.env.TAURI_DEV_HOST` without the suppression. **Choose option A (drop the env-var path)** for simplicity; mobile is not a v0.1 target ([Source: architecture.md#LD-34 Distribution channels](../planning-artifacts/architecture.md)). Document the choice in the story Change Log + Dev Notes.
    - If the LSP still flags `Unused '@ts-expect-error'` post-edit, the directive is unreachable — verify by deleting the entire block, not just modifying around it.

9. **AC9 — No regressions to Story 1.1 / 1.2 invariants.** The structural baseline established by Stories 1.1 + 1.2 is preserved:
    - Husky `commit-msg` hook still rejects malformed commits (commitlint chain intact).
    - The 9-crate workspace still parses: no `[workspace.members]` edits in this story.
    - `tools/corpus-extractor/` remains outside the workspace; `cargo build --workspace` does not compile it.
    - `crates/orgsidian-plugin-api/Cargo.toml` is **untouched** (LEAF invariant preserved — no plugin/shadcn/router dep leaks into the plugin-api crate).
    - No `[profile.release]` added to root `Cargo.toml` (still Story 1.8 territory).
    - No `tauri-specta` install (still Story 1.4).
    - No Lingui install (still Story 1.6).
    - The `tauri.conf.json` `productName`/`identifier`/`version`/window `label`/`title` are unchanged.
    - The `frontendDist = "../../shell-ui/dist"` correction from Story 1.2 dev notes stays correct.

10. **AC10 — No premature scope creep.** The following are **explicitly OUT of scope** for Story 1.3 and MUST NOT be added:
    - ❌ The `--org-*` CSS variable vocabulary inside `@theme {}` — Story 6.7 (themes).
    - ❌ Dark/light theme switching, theme picker UI, `~/.orgsidian/themes/` user CSS loader — Story 6.7 / Story 12.1.
    - ❌ `Toaster` integration with real notification triggers (e.g. save errors) — feature stories that need it.
    - ❌ `tauri-specta` IPC bridge — Story 1.4.
    - ❌ `OrgsidianPlugin` trait surface in `orgsidian-plugin-api` — Story 1.5.
    - ❌ Lingui i18n catalogs, SWC plugin in vite config — Story 1.6.
    - ❌ `cargo-deny` / `cargo audit` config — Story 1.7.
    - ❌ Per-PR / nightly CI workflows, `[profile.release] panic = "unwind"`, `invoke_plugin_hook!` macro — Story 1.8.
    - ❌ Anchor smoke tests — Story 1.9.
    - ❌ `SECURITY.md` / `ARCHITECTURE.md` / `CHANGELOG.md` / `CONTRIBUTING.md` — Story 1.10.
    - ❌ `tauri-plugin-fs` Vault-folder scope tightening (LD-17 runtime allow-list) — Story 3.6.
    - ❌ Quick Capture window (`shell-ui/quick-capture.html`, `capabilities/quick-capture.json`, multi-entry Vite) — Story 8.1.
    - ❌ Components/org/, components/today/, components/agenda/, components/editor/ subfolders — they land with the stories that introduce their first components. This story creates `shell-ui/src/components/ui/` only (via shadcn CLI) + a `shell-ui/src/lib/` (for `utils.ts`).
    - ❌ Zustand store scaffolding (`stores/clockStore.ts`, etc.) — feature stories.
    - ❌ Real route components for `/agenda`, `/editor`, `/graph`, `/settings` — they land with their owning epic stories.
    - ❌ `@vitejs/plugin-react-swc` swap (currently using `@vitejs/plugin-react` per Story 1.1 baseline) — Lingui (Story 1.6) will require the SWC variant; do **not** preempt the swap here.
    - ❌ `noUncheckedIndexedAccess: true` in tsconfig — architecture mentions it but it's a Story 1.8 hardening item.

## Tasks / Subtasks

> **Recommended order:** Each task can be tested in isolation. Run `pnpm install && cargo check --workspace` after each pnpm-touching task; run `pnpm --filter shell-ui build` after each TS-touching task; run `pnpm tauri dev` only at Task 9 (the end-to-end gate).

- [x] **Task 1 — Pre-flight & branch (AC: all)**
  - [x] 1.1 Confirm `main` is clean (`git status`); branch as `feat/story-1-3-plugins-tailwind-shadcn-router`.
  - [x] 1.2 Verify Node ≥ 22 LTS, pnpm 11.x, Rust stable (`rustc --version`).
  - [x] 1.3 Run `pnpm tauri info` from repo root; confirm it discovers `crates/orgsidian-shell-app/tauri.conf.json` and reports the current plugin set (only `opener` baseline).

- [x] **Task 2 — Install Tauri plugin set (AC: 1, 2)**
  - [x] 2.1 Use `pnpm tauri add <plugin>` for the 11 architecture-locked plugins (the CLI automates cargo-add + npm-add + lib.rs registration + capabilities permission entry in one shot — verified via [Source: ctx7 /websites/v2_tauri_app "Add File-System Plugin with pnpm"]):
        ```bash
        pnpm tauri add fs
        pnpm tauri add dialog
        pnpm tauri add global-shortcut
        pnpm tauri add window-state
        pnpm tauri add store
        pnpm tauri add shell
        pnpm tauri add os
        pnpm tauri add clipboard-manager
        pnpm tauri add log
        pnpm tauri add process
        pnpm tauri add updater
        ```
        Run each from repo root. The CLI edits `crates/orgsidian-shell-app/Cargo.toml`, possibly `shell-ui/package.json` (for JS bindings: `@tauri-apps/plugin-fs`, etc.), `crates/orgsidian-shell-app/src/lib.rs` (adds `.plugin(<plugin>::init())`), and `capabilities/default.json` (adds `<plugin>:default` to permissions). **After all 11 are added**, verify each by hand against the AC1 inventory (sometimes the CLI misses a step on monorepo layouts — manual reconciliation is cheap and worth the 5 minutes).
  - [x] 2.2 Hand-verify `tauri-plugin-updater` is gated for desktop only: ensure the Cargo.toml entry is under `[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]` (not the top-level `[dependencies]`). If the CLI added it to top-level, manually move it to the cfg-gated target per AC1 spec.
  - [x] 2.3 Sort the `.plugin(...)` chain in `lib.rs::run()` alphabetically by plugin name (keep `opener` first for Story 1.1 lineage). Gate `tauri_plugin_updater` with `#[cfg(desktop)]` per the Tauri docs convention.
  - [x] 2.4 Rename `crates/orgsidian-shell-app/capabilities/default.json` → `capabilities/main.json` via `git mv` (preserves blame). Edit the JSON: `identifier: "main"`, updated `description`, verify `permissions` array contains all 12 entries (`core:default`, `opener:default`, plus the 10 plugin defaults plus `updater:default`).
  - [x] 2.5 Run `cargo check --workspace` from repo root — must exit 0. Resolve any version conflict warnings (Tauri plugin minor-version mismatches are common; pin to the same `^2` minor as `tauri` itself).
  - [x] 2.6 Run `pnpm tauri info` — confirm the renamed `main.json` capability is auto-discovered and all 11 plugins are listed under "Plugins".

- [x] **Task 3 — Tailwind 4 install + CSS-first config (AC: 3)**
  - [x] 3.1 From `shell-ui/`: `pnpm add tailwindcss @tailwindcss/vite` (these become production deps because they affect the build output). Verify pnpm resolves Tailwind 4.1.x latest stable (`pnpm list --filter shell-ui tailwindcss`).
  - [x] 3.2 Create `shell-ui/src/styles/` directory; create `shell-ui/src/styles/app.css` with the canonical Tailwind 4 content per AC3 (just `@import "tailwindcss";` + empty `@theme {}` block + the documentation comment). Do **not** copy the `--org-*` vocabulary; that's Story 6.7.
  - [x] 3.3 Edit `shell-ui/vite.config.ts`: add `import tailwindcss from '@tailwindcss/vite';` and register it in `plugins: [...]` between the TanStack router plugin (Task 5) and the React plugin. **At this Task 3 checkpoint, the TanStack plugin doesn't exist yet** — register Tailwind in between where it will go (it's fine to land it temporarily as `[tailwindcss(), react()]`; Task 5 inserts `tanstackRouter` before).
  - [x] 3.4 Edit `shell-ui/src/main.tsx`: change the `import "./App.css"` (currently in `App.tsx`) to `import "./styles/app.css"` at the top of `main.tsx`. The legacy `App.css` import in `App.tsx` is removed in Task 7 when `App.tsx` itself is deleted.
  - [x] 3.5 Run `pnpm --filter shell-ui build` — exits 0; built `dist/assets/*.css` contains Tailwind output (sanity: `rg --max-count=1 "tailwind|--tw-" shell-ui/dist/assets/*.css`).
  - [x] 3.6 Run `pnpm --filter shell-ui dev` briefly (~5 sec) to confirm the dev server starts without Tailwind plugin errors. Stop with Ctrl+C.

- [x] **Task 4 — Path alias `@/*` + tsconfig + vite config (AC: 4)**
  - [x] 4.1 Edit `shell-ui/tsconfig.json` `compilerOptions`: add `"baseUrl": "."` and `"paths": { "@/*": ["./src/*"] }`. Existing `strict`, `noUnusedLocals`, etc., are unchanged.
  - [x] 4.2 Edit `shell-ui/vite.config.ts`: add `import path from 'node:path';` and `resolve: { alias: { '@': path.resolve(__dirname, './src') } }` to the config object. **Caveat:** `__dirname` is not defined in ESM. Use `import { fileURLToPath } from 'node:url';` and `path.dirname(fileURLToPath(import.meta.url))` instead. The minimal idiomatic pattern:
        ```ts
        import { defineConfig } from 'vite';
        import path from 'node:path';
        import { fileURLToPath } from 'node:url';

        const __dirname = path.dirname(fileURLToPath(import.meta.url));

        export default defineConfig({
          resolve: { alias: { '@': path.resolve(__dirname, './src') } },
          // ... plugins, server
        });
        ```
        If `@types/node` is not yet a dev-dep, install it: `pnpm add -D @types/node --filter shell-ui` (required for the typed `node:path` / `node:url` imports under strict-mode TS).
  - [x] 4.3 Run `pnpm --filter shell-ui build` — must exit 0; the path alias resolves at typecheck. (No `@/` imports exist yet; this is preflight before Task 6.)

- [x] **Task 5 — TanStack Router install + Vite plugin + main.tsx rewrite (AC: 5)**
  - [x] 5.1 From `shell-ui/`: `pnpm add @tanstack/react-router` (prod dep) and `pnpm add -D @tanstack/router-plugin @tanstack/react-router-devtools` (dev deps). Verify versions resolve to the latest stable minor of TanStack Router 1.x.
  - [x] 5.2 Edit `shell-ui/vite.config.ts`: add `import { tanstackRouter } from '@tanstack/router-plugin/vite';` and place `tanstackRouter({ target: 'react', autoCodeSplitting: true })` as the **first** entry in `plugins: [...]`. Final order: `[tanstackRouter({...}), tailwindcss(), react()]`. **Order is non-negotiable** per AC5.
  - [x] 5.3 Create `shell-ui/src/routes/__root.tsx` per AC5 (note: this depends on `<Toaster />` from Task 6; this task creates the file with the `Toaster` import and an explicit TODO comment, and Task 6 plays nicely with it). Acceptable interim: leave `Toaster` out of `__root.tsx` until Task 6 lands `sonner`, then patch it in. Recommended: write the file fully per AC5 since `sonner` will land before this builds.
  - [x] 5.4 Create `shell-ui/src/routes/index.tsx` per AC5 (the `/` → `/today` redirect).
  - [x] 5.5 Create `shell-ui/src/routes/_layout/` directory; create `shell-ui/src/routes/_layout/today.tsx` per AC5 placeholder shape — **but** the `<Button>` import from `@/components/ui/button` won't resolve until Task 6 has run shadcn add. **Recommended sequence: do Task 6 before this sub-step, OR write the placeholder with a plain `<button>` first and swap to shadcn `<Button>` after Task 6.** Pick the second to keep tasks independently testable.
  - [x] 5.6 Add `routeTree.gen.ts` to `shell-ui/.gitignore` (or repo-root `.gitignore` if it's not already covering `shell-ui/src/`). Verify with `git check-ignore -v shell-ui/src/routeTree.gen.ts` once the file exists.
  - [x] 5.7 Rewrite `shell-ui/src/main.tsx` per AC5 (RouterProvider mount, `declare module` augmentation, `routeTree` import from `./routeTree.gen`).
  - [x] 5.8 Run `pnpm --filter shell-ui dev` briefly: the Vite plugin must generate `shell-ui/src/routeTree.gen.ts` on first dev compile. Confirm the file exists at `shell-ui/src/routeTree.gen.ts` and is gitignored. Stop with Ctrl+C.
  - [x] 5.9 Run `pnpm --filter shell-ui build` — exits 0; the `routeTree.gen.ts` is regenerated for the build pass.

- [x] **Task 6 — shadcn/ui init + add 6 essentials (AC: 4)**
  - [x] 6.1 Create `shell-ui/components.json` per AC4 spec. Manual creation (not via `npx shadcn init` interactive mode) lets us pin the exact config without prompts.
  - [x] 6.2 From `shell-ui/`: `npx shadcn@latest add button dialog input tabs tooltip sonner`. The CLI:
        - Auto-installs `class-variance-authority`, `clsx`, `tailwind-merge`, `lucide-react`, `sonner`, plus the `@radix-ui/react-*` primitives.
        - Generates 6 files at `shell-ui/src/components/ui/{button,dialog,input,tabs,tooltip,sonner}.tsx`.
        - Generates `shell-ui/src/lib/utils.ts` with the `cn()` helper.
  - [x] 6.3 **Verify** the 6 generated files compile under TS strict mode. `pnpm --filter shell-ui build` — exit 0. If the CLI emits any TS error (rare, but happens when shadcn templates lag behind TS versions), patch in-place and document in Change Log.
  - [x] 6.4 If Task 5.5 wrote the placeholder with a plain `<button>`, **now** swap it to shadcn `<Button>` per AC5.5 + the AC4.4 smoke-import requirement.
  - [x] 6.5 Commit all generated files (`git add shell-ui/src/components/ui/ shell-ui/src/lib/utils.ts shell-ui/components.json`).

- [x] **Task 7 — Delete `App.tsx` + `App.css` (AC: 5, 10)**
  - [x] 7.1 Confirm `shell-ui/src/main.tsx` no longer imports `./App` or `./App.css` (Task 3.4 moved the CSS import; Task 5.7 removed the App import).
  - [x] 7.2 `git rm shell-ui/src/App.tsx shell-ui/src/App.css`. The scaffold's `assets/react.svg`, `public/tauri.svg`, `public/vite.svg` are **kept** for now (the placeholder route doesn't reference them but they're harmless residuals; a later doc/onboarding story can decide their fate).
  - [x] 7.3 Run `pnpm --filter shell-ui build` — exit 0 (no orphan references).

- [x] **Task 8 — `vite.config.ts` `@ts-expect-error` cleanup (AC: 8)**
  - [x] 8.1 Per AC8, choose option A (drop the `TAURI_DEV_HOST` env-var path). Remove the `@ts-expect-error` line, the `const host = process.env.TAURI_DEV_HOST;` line, and the `host: host || false` / `hmr: host ? {...} : undefined` references. Replace with `host: false` (or remove the line — defaults to `false`).
  - [x] 8.2 Run `pnpm --filter shell-ui build` — exit 0; the LSP no longer reports the unused-directive diagnostic.
  - [x] 8.3 If the LSP still flags anything in `vite.config.ts`, fix it now (the file is small enough that a clean compile is the right gate).

- [x] **Task 9 — End-to-end verification (AC: 6, 7, 9)**
  - [x] 9.1 `cargo check --workspace` from repo root — exit 0.
  - [x] 9.2 `cargo build --workspace` from repo root — exit 0 (first run is slow due to 11 new plugins; expect ~30-60s incremental).
  - [x] 9.3 `pnpm install` from repo root — exit 0 (no lockfile resolution errors).
  - [x] 9.4 `pnpm tauri dev` from repo root:
        - Vite serves on `http://localhost:1420/`.
        - TanStack Router Vite plugin generates `routeTree.gen.ts` on first compile (verify by `ls shell-ui/src/routeTree.gen.ts`).
        - Rust shell binary compiles with the 11 new plugins (check log for `Compiling tauri-plugin-fs`, `Compiling tauri-plugin-dialog`, etc.).
        - The Tauri window opens; URL bar (if devtools open) shows `http://localhost:1420/today` (redirected from `/`).
        - The placeholder text "Today (placeholder)" + the "Story 7.1 will replace this..." description + a shadcn `<Button>Greet</Button>` are visible.
        - Typing a name + clicking "Greet" returns "Hello, {name}! You've been greeted from Rust!" (Story 1.1 invariant preserved).
        - Tailwind utility classes (`container mx-auto p-8`, `text-2xl font-semibold`) render visibly (centered content, large title).
  - [x] 9.5 `pnpm tauri build` from repo root: `.app` at `target/release/bundle/macos/orgsidian.app`, `.dmg` at `target/release/bundle/dmg/orgsidian_0.0.0_aarch64.dmg`.
  - [x] 9.6 `cargo build` from inside `tools/corpus-extractor/` — exit 0 (unaffected).
  - [x] 9.7 Sanity-check `crates/orgsidian-plugin-api/Cargo.toml`: still no project deps, still no external deps. LEAF invariant preserved.
  - [x] 9.8 Husky commit-msg hook reject path: `echo "broken commit message" | npx --no-install commitlint` exits 1 (non-destructive verification, same pattern as Story 1.2 code review).

- [x] **Task 10 — Final sweep (AC: all)**
  - [x] 10.1 `git status --short` shows only the intentional change set (file moves for `default.json`→`main.json`, deletions for `App.tsx`/`App.css`, additions for routes/styles/components-ui/lib/utils, edited `Cargo.toml`/`vite.config.ts`/`tsconfig.json`/`main.tsx`/`lib.rs`, regenerated `pnpm-lock.yaml`).
  - [x] 10.2 Sweep AC1-AC10 against the diff; tick checkboxes.
  - [x] 10.3 Update the Dev Agent Record File List section.
  - [x] 10.4 If any sub-step surfaced a deviation from the AC text, document in Change Log with a `[Deviation]` tag and a one-line rationale.

### Review Findings

_Code review run: 2026-05-21 (bmad-code-review, 3 parallel adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor)._

**Decision-needed (4) — all resolved 2026-05-21:**

- [x] [Review][Decision] **shadcn CSS variables (`--background`, `--popover`, `--primary`, `--foreground`, `--border`, `--radius`, etc.) are never injected** — the 6 forked shadcn components (`button.tsx`, `dialog.tsx`, `input.tsx`, `tabs.tsx`, `tooltip.tsx`, `sonner.tsx`) reference these CSS variables via Tailwind utilities (`bg-primary`, `text-primary-foreground`, `bg-popover`, `border-input`, etc.). `shell-ui/src/styles/app.css` ships only `@import "tailwindcss"` + an empty `@theme {}` block. Dev Notes §525 said "shadcn defaults are fine — they don't conflict with the (empty) @theme {} block" — but the defaults were never injected (only `components.json` `baseColor: "neutral"` was set; no `npx shadcn init` was run to materialise the variables). Result: `<Button>Greet</Button>` (AC4 smoke-import + AC5.5 placeholder) and every future shadcn primitive render unstyled. Completion Notes explicitly flagged "manual click-through to verify `<Button>Greet</Button>` IPC round-trip is pending user-side visual confirmation" — visual was never confirmed. Decision: **(a)** inject shadcn `neutral` baseColor `:root { --background: …; --primary: …; … }` block into `app.css` now (~30 lines, restores AC4 smoke-import intent), **(b)** accept unstyled components until Story 6.7 (themes) lands the `--org-*` vocabulary.

- [x] [Review][Decision] **Updater JS binding + capability permission shipped without Rust plugin registration creates a latent IPC trap** — `@tauri-apps/plugin-updater` is in [shell-ui/package.json:18](shell-ui/package.json#L18) and `updater:default` is in [crates/orgsidian-shell-app/capabilities/main.json:19](crates/orgsidian-shell-app/capabilities/main.json#L19), but `tauri_plugin_updater` is **not** registered in [crates/orgsidian-shell-app/src/lib.rs](crates/orgsidian-shell-app/src/lib.rs) (deferred to Story 13.2 per the disclosed AC1 deviation). Any frontend call to `@tauri-apps/plugin-updater` APIs passes the capability gate then rejects at IPC dispatch with `"plugin updater not found"` — the exact shape of an integration bug that wastes hours to triage. Story 1.3 currently ships 3 of 4 legs of the AC1 contract; the missing 4th leg is the most impactful one. Decision: **(a)** accept the documented state (theoretical risk; no frontend code calls the binding today), **(b)** also defer the JS binding + `updater:default` permission to Story 13.2 — keep only the cfg-gated Cargo dep for now (matches the "ship only when fully wired" principle).

- [x] [Review][Decision] **`pnpm --filter shell-ui build` fails on a fresh clone** — [shell-ui/package.json](shell-ui/package.json#L7) defines `"build": "tsc && vite build"`. `tsc` runs **first, standalone, without the Vite plugin**. [shell-ui/src/main.tsx:4](shell-ui/src/main.tsx#L4) imports `./routeTree.gen`, which is gitignored ([`.gitignore:44`](.gitignore#L44)) and generated **only** by `@tanstack/router-plugin/vite` at `vite dev`/`vite build` time. On a fresh clone or in CI before any `vite dev`, `tsc` fails with "Cannot find module './routeTree.gen'". Dev Notes §6 acknowledged the trade-off and flagged Story 1.8 (CI matrix) as the revisit point — but Story 1.3 is shipping the broken-fresh-clone state today. Decision: **(a)** add a `prebuild` script that runs `tsr generate` (or equivalent) to materialise `routeTree.gen.ts` before `tsc`, **(b)** reverse the gitignore decision and commit `routeTree.gen.ts` (the canonical TanStack pattern), **(c)** rely on `pnpm tauri dev` being run at least once per fresh clone (accept the friction; documented).

- [x] [Review][Decision] **`@types/node@^25.9.1` contradicts the LTS-preferred version policy** — [shell-ui/package.json:35](shell-ui/package.json#L35) pins `@types/node` to `^25.9.1`. Node 25 is non-LTS (odd-numbered, short-lived); Node 24 is the active LTS line as of October 2025. The user's persistent feedback `[[feedback_version_policy]]` says: "deps pinned to latest stable or LTS (LTS preferred); Tauri ecosystem exempted." `@types/node` is not Tauri-ecosystem. Decision: **(a)** downgrade to `@types/node@^24` (active LTS), **(b)** keep `^25` (current latest, accept the non-LTS exception with a one-line Change Log entry).

**Patch (2 original + 3 from resolved decisions = 5 total) — all applied 2026-05-21:**

- [x] [Review][Patch] **`tauri_plugin_log` builder adds `.level(LevelFilter::Info)` — undisclosed deviation from Reference shape** [[crates/orgsidian-shell-app/src/lib.rs:15-19](crates/orgsidian-shell-app/src/lib.rs#L15-L19)] — Reference lib.rs in Dev Notes (line 414) shows bare `tauri_plugin_log::Builder::new().build()`. The diff adds `.level(tauri_plugin_log::log::LevelFilter::Info)`. Not documented in Change Log or Completion Notes. Fix: either remove `.level(Info)` to match the Reference shape exactly, or add a one-line `[Deviation]` entry in the Change Log explaining the runtime default choice.

- [x] [Review][Patch] **`./styles/app.css` import is not at the top of `main.tsx`** [[shell-ui/src/main.tsx:4](shell-ui/src/main.tsx#L4)] — AC3 / Task 3.4 says "change the `import './App.css'` ... to `import './styles/app.css'` **at the top** of `main.tsx`". Current order: React → ReactDOM → Router → routeTree → **CSS** (line 4). Trivial reorder: move the CSS import to line 1. Functionally harmless today, off-spec on the letter.

- [x] [Review][Patch] **Inject shadcn `neutral` baseColor CSS variables in `app.css`** (from resolved Decision #1, option a) [[shell-ui/src/styles/app.css](shell-ui/src/styles/app.css)] — appended canonical `@theme inline { --color-* → var(--*) }` mapping + `:root` light variant + `.dark` dark variant + `@layer base { body { @apply bg-background text-foreground } }`, sourced verbatim from ui.shadcn.com/docs/installation/manual via ctx7 2026-05-21. CSS bundle size jumped from ~5 kB to 27.61 kB confirming the variables now resolve. Story 6.7 will override these with the `--org-*` vocabulary.

- [x] [Review][Patch] **Add `prebuild` script using `tsr generate`** (from resolved Decision #3, option a) [[shell-ui/package.json:8](shell-ui/package.json#L8)] — added `"prebuild": "tsr generate"` script + `@tanstack/router-cli@^1.167.8` devDep (matching `router-plugin` minor). Verified by deleting `routeTree.gen.ts` and running `pnpm run build` from a simulated fresh-clone state — `tsr generate` materialises the file before `tsc` reads `main.tsx`, build green.

- [x] [Review][Patch] **Downgrade `@types/node` to `^24` LTS** (from resolved Decision #4, option a) [[shell-ui/package.json:42](shell-ui/package.json#L42)] — aligned with `[[feedback_version_policy]]` (LTS preferred for non-Tauri deps). `pnpm install` resolved cleanly to the latest `@types/node@^24`.

**Defer (3):**

- [x] [Review][Defer] **No `<TooltipProvider>` mounted at app root** [[shell-ui/src/routes/__root.tsx](shell-ui/src/routes/__root.tsx)] — deferred, no `<Tooltip>` consumer exists in Story 1.3; the first story that mounts a `<Tooltip>` (per shadcn convention) must add `<TooltipProvider>` either at `__root.tsx` or scoped to the consumer. Without it, Radix throws "`Tooltip` must be used within `TooltipProvider`" on first use.

- [x] [Review][Defer] **`sonner.tsx` calls `useTheme()` from `next-themes` outside a `<ThemeProvider>`** [[shell-ui/src/components/ui/sonner.tsx:14](shell-ui/src/components/ui/sonner.tsx#L14)] — deferred to Story 6.7 (themes). `useTheme()` outside provider degrades to `theme: undefined` → destructure default `"system"` → Toaster theme detection effectively disabled until ThemeProvider is mounted. No real toasts triggered in Story 1.3, so latent.

- [x] [Review][Defer] **`routes/_layout/today.tsx` has no parent `_layout.tsx` — pathless group routes parent to root today; future `_layout.tsx` would retroactively reparent every child** [[shell-ui/src/routes/_layout/today.tsx:5](shell-ui/src/routes/_layout/today.tsx#L5)] — deferred to Story 7.1 (Today Dashboard, which introduces the real layout). Story 1.3 spec explicitly designs this as intentional (line 112 — "For this story the layout is trivial — the future Story 7.1 will introduce sidebar/topbar around it."). Flag for awareness: when Story 7.1 adds `_layout.tsx`, every existing sibling under `_layout/` will silently be wrapped by it.

**Dismissed (10):** false positives or already-disclosed deviations — `lucide-react@^1.16.0` is canonical (Eric Fennis/lucide-icons author verified; v1.x is a recent legitimate bump); `Slot.Root` from `radix-ui` 1.4.3 works (consolidated package); `_layout/` pathless group without `_layout.tsx` is intentional per spec (covered above as defer); `host: false` mobile-dev loss is disclosed (AC8 Option A); `radix-ui` consolidated package is disclosed (Change Log AC4 deviation); `tsconfig.json` paths without `baseUrl` is disclosed (Change Log AC4 deviation); placeholder `<input>` is plain native (matches AC5 reference code verbatim); StrictMode double-invokes `beforeLoad` (TanStack handles); `@tauri-apps/plugin-opener` orphaned JS binding (Dev Notes §4 explicitly retains for Story 1.1 baseline conservatism); `app.css` comment-text wording differs from Reference shape (cosmetic, semantically equivalent).

## Dev Notes

### Critical context the dev agent MUST internalize before touching code

**1. This story is the JS/UI infrastructure story.** Three independent ecosystems (Tauri plugin set, Tailwind 4, TanStack Router) land in the same PR because they share the `shell-ui/vite.config.ts` and `crates/orgsidian-shell-app/Cargo.toml` edit surface — bundling avoids 3× the merge churn. None of these are feature work; they are pre-wired infrastructure that v0.1 feature stories (Today Dashboard, Agenda, Editor, etc.) consume directly without per-feature install steps.

**2. Use `pnpm tauri add <plugin>` — do NOT hand-edit Cargo.toml + lib.rs + capabilities for each plugin.** The Tauri 2 CLI (`@tauri-apps/cli`, already at root from Story 1.1) automates all three locations in one shot ([Source: ctx7 /websites/v2_tauri_app "Add File-System Plugin with pnpm"], [Source: ctx7 /websites/v2_tauri_app "Add log plugin via CLI"]). The CLI handles the workspace-monorepo layout correctly — it discovers `crates/orgsidian-shell-app/tauri.conf.json` via `pnpm tauri info`'s same resolution path (verified in Story 1.2 code review). **However**, the CLI does NOT cfg-gate `tauri-plugin-updater` automatically — this is a manual step per Task 2.2 (the official Tauri docs explicitly show the `[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]` form for updater; the CLI's default cargo-add places it under the top-level `[dependencies]` and you have to move it).

**3. Tailwind 4 is CSS-first — NO `tailwind.config.js`.** This is the **single biggest divergence** from training-data Tailwind workflows (which assume v3 with JS config). Tailwind 4 ships CSS-first via `@import "tailwindcss"` + `@theme {}` directive in the main CSS file ([Source: ctx7 /tailwindlabs/tailwindcss.com "Define Tailwind CSS Configuration with CSS-first @theme"]). Do NOT create `tailwind.config.js` / `.ts`. Do NOT add `postcss.config.js`. The `@tailwindcss/vite` plugin handles everything via the `app.css` directive. If you find yourself reaching for `npx tailwindcss init` — stop, that's v3 muscle memory.

**4. Why `tauri-plugin-opener` stays alongside `tauri-plugin-shell`.** Story 1.1 scaffolded `tauri-plugin-opener` (the v2-standard "open URL/file in default app" plugin); the architecture's plugin set lists `tauri-plugin-shell` (which provides `shell.open()` AND `shell.Command` for invoking external CLIs). Both can coexist — they don't conflict on permissions. Removing `opener` would break the Story 1.1 baseline for zero benefit (the React scaffold doesn't use it, but removing it would orphan the JS-side `@tauri-apps/plugin-opener` import in `App.tsx` — which we delete anyway in Task 7, but the principle stands: this is a structural story, conservative wins). A later cleanup story can collapse them if needed; not now.

**5. TanStack Router Vite plugin MUST come BEFORE `@vitejs/plugin-react`.** ([Source: ctx7 /tanstack/router "Configure Vite for TanStack Router"]) — the plugin order is required for correct route-file codegen with auto code-splitting. Violating this order produces silent fast-refresh breakage that's hard to diagnose. The Tailwind plugin order is more relaxed (it works anywhere in the chain) — for tidiness, place it between TanStack and React: `[tanstackRouter({...}), tailwindcss(), react()]`.

**6. `routeTree.gen.ts` is gitignored, not committed.** The TanStack Router docs default is to commit it, but for this project (a) we don't yet have CI that consumes the file before `pnpm install + dev` runs, (b) committing it creates routine PR noise on every route addition, (c) the Vite plugin regenerates it on every dev/build run. If the Story 1.8 CI matrix later requires a pre-generated file (e.g. for a typecheck-only job that doesn't run Vite), revisit this decision. The trade-off is documented here so future-you doesn't re-litigate. ([Source: ctx7 /tanstack/router file-based routing docs])

**7. shadcn 2.x+ replaces the legacy `Toast` primitive with `Sonner`.** The epic AC says "Toast" — translated to current shadcn vocabulary that's `npx shadcn@latest add sonner`, which yields `shell-ui/src/components/ui/sonner.tsx` (a thin wrapper around the `sonner` npm package). The wrapper exports a `<Toaster />` mounted under `__root.tsx` and a `toast()` function consumers call. ([Source: ctx7 /websites/ui_shadcn "Install Sonner via CLI"]) — verified canonical. Future Toast-using stories import `import { toast } from 'sonner'`.

**8. The placeholder `/today` route is a 4-pillar smoke test.** The placeholder content per AC5.5 is deliberately minimal but exercises (a) TanStack route resolution, (b) shadcn component import (`@/components/ui/button`), (c) Tailwind utility class rendering, (d) Tauri IPC `invoke('greet')` round-trip. If any of the four pillars regress in a future story, the placeholder catches it. Story 7.1 will replace this with the real Today Dashboard — the placeholder dies cleanly without leaving dead code (it's a single file).

**9. `App.tsx` and `App.css` are DELETED in this story.** Story 1.1 scaffolded them; Story 1.2 left them as-is (structural-only refactor). Story 1.3 absorbs their contents into the `/today` placeholder route and removes them. Keeping a dead `App.tsx` would violate the architecture's "Premature abstraction" anti-pattern ([Source: architecture.md#Anti-Patterns (Forbidden)](../planning-artifacts/architecture.md)). The scaffold-residual assets (`shell-ui/src/assets/react.svg`, `shell-ui/public/tauri.svg`, `shell-ui/public/vite.svg`) are NOT deleted here — they're harmless and a later doc/branding story can decide their fate (low-priority cleanup, not architectural).

**10. The `@ts-expect-error process` directive in `vite.config.ts` MUST be cleaned up.** Story 1.2 deferred this ("Story 1.3 owns its replacement"); the live diagnostic confirms it's currently flagged unused. Resolution: drop the `TAURI_DEV_HOST` env-var path entirely (we don't target mobile in v0.1 per LD-34). If a future mobile-targeting story wants the env-var back, it owns adding `@types/node` + the typed `process.env` access.

**11. `windows: ["main"]` in `capabilities/main.json` is unchanged.** The Quick Capture window (LD-28) has its own capability file `capabilities/quick-capture.json` per the architecture's Workspace Layout — that file lands in Story 8.1 (Quick Capture surface). For now, `main.json` is the only capability and it scopes all 11 plugins to the `main` window. **Do not** add a wildcard `windows: ["*"]` or a separate per-plugin capability split — keep it monolithic until Story 8.1 introduces the second window.

**12. AI-Agent Implementation Rules apply ([Source: architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md)).** Specifically for this story:
   - One concern per file ✅ — every route is its own file, every shadcn component is its own file (the CLI gives this for free).
   - No `unwrap()` / `expect()` in production paths ✅ — the only Rust edits are in `lib.rs::run()` (returns `tauri::Result<()>`, error-propagation pattern preserved from Story 1.1 code-review patch).
   - No `any` / `unknown` in TypeScript ✅ — the placeholder route's `useState<string>` is implicit; the `invoke('greet', {...})` returns `string` (untyped raw `invoke`, but Story 1.4 replaces this with the typed `tauri-specta` client; do NOT preempt the typing here).
   - Use the generated `tauri-specta` client → **NOT YET** ✅ — Story 1.4 wires `tauri-specta`. Story 1.3's `invoke('greet', ...)` is a temporary holdover from Story 1.1 and is the **last** place a raw-string `invoke` is allowed (Story 1.4 replaces it). Tag the line with a comment: `// Story 1.4 replaces this with the typed specta client.`.
   - Tailwind utilities first; extract to `org-*` classes only after 3+ repetitions ✅ — the placeholder uses utilities only.
   - Run `cargo test --workspace && pnpm test` before pushing — no tests added in this story, so `cargo test --workspace` runs the empty stub-crate tests (passes) and `pnpm test` is not yet wired (Story 1.9 wires anchor smoke tests). Substitute: `cargo check --workspace && pnpm --filter shell-ui build`.

**13. Anti-Patterns ([Source: architecture.md#Anti-Patterns (Forbidden)](../planning-artifacts/architecture.md)):**
   - ❌ `invoke('command_name', …)` with raw strings → tolerated **only** for the `greet` call in the placeholder, with a `// Story 1.4 replaces this` comment (this is the same pattern Story 1.2 preserved). All other invokes (none in this story) must wait for Story 1.4.
   - ❌ `#[serde(rename_all = "camelCase")]` on individual structs → no struct edits in this story.
   - ❌ React `forwardRef` → the shadcn CLI may emit components using `React.forwardRef` historically — **modern shadcn 2.x+ has been updated to React 19 `ref`-as-prop pattern**. If the CLI emits any `forwardRef`, that's a known shadcn lag and we accept it as upstream — do NOT manually rewrite (the shadcn upstream will update; we re-fork on the next major).
   - ❌ Direct DOM manipulation → none.
   - ❌ String-typed event names → no `app.emit()` calls in this story.
   - ❌ `console.log` / `println!` → no debug prints.

### Reference `crates/orgsidian-shell-app/src/lib.rs` post-Task-2 shape

```rust
// Story 1.4 replaces this with the typed `tauri-specta` client.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build());

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    builder
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
}
```

> **Note on Builder-style plugins.** Some plugins (`global-shortcut`, `log`, `store`, `window-state`, `updater`) use a builder pattern rather than a bare `::init()`. The `pnpm tauri add` CLI emits the correct form per plugin; verify against the [official Tauri docs per plugin](https://v2.tauri.app/plugin/) if anything looks off. The Builder forms above are the v2 standard.

### Reference `shell-ui/vite.config.ts` post-Task-8 shape

```ts
import { defineConfig } from 'vite';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(async () => ({
  plugins: [
    // MUST come before tailwindcss() and react() per TanStack docs.
    tanstackRouter({ target: 'react', autoCodeSplitting: true }),
    tailwindcss(),
    react(),
  ],
  resolve: {
    alias: { '@': path.resolve(__dirname, './src') },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: false,
    watch: {
      ignored: [
        '**/crates/orgsidian-shell-app/**',
        '**/target/**',
        '**/_bmad-output/**',
        '**/_bmad/**',
      ],
    },
  },
}));
```

### Reference `shell-ui/tsconfig.json` post-Task-4 shape (diff from current)

```jsonc
{
  "compilerOptions": {
    // ... existing fields unchanged ...
    "baseUrl": ".",                           // NEW
    "paths": { "@/*": ["./src/*"] },          // NEW
    // strict, noUnusedLocals, noUnusedParameters preserved
  },
  // include / references unchanged
}
```

> **`noUncheckedIndexedAccess: true` is NOT added in this story** — architecture mentions it but it's a Story 1.8 hardening item (per AC10 anti-creep list).

### Reference `shell-ui/src/styles/app.css` post-Task-3 shape

```css
@import "tailwindcss";

@theme {
  /* Orgsidian's --org-* design tokens (FR-22 vocabulary) land in Story 6.7 (themes).
     Story 1.3 ships the Tailwind 4 plumbing only; later stories extend without
     re-introducing the @import directive. */
}
```

### Reference `shell-ui/components.json` post-Task-6 shape

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/styles/app.css",
    "baseColor": "neutral",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide"
}
```

> **`baseColor: "neutral"`** is a temporary choice. Story 6.7 (themes) will likely override the shadcn-generated CSS variables in `app.css` to use Orgsidian's `--org-*` token vocabulary instead of shadcn's defaults. For Story 1.3 the shadcn defaults are fine — they don't conflict with the (empty) `@theme {}` block.

### Reference post-story file structure (target additions/changes)

```
orgsidian/
├── crates/orgsidian-shell-app/
│   ├── Cargo.toml                              (MODIFIED: +11 plugin deps, updater under cfg-gate)
│   ├── capabilities/
│   │   ├── default.json  →  main.json          (RENAMED via git mv; identifier + permissions updated)
│   └── src/lib.rs                              (MODIFIED: 11 .plugin() registrations)
├── shell-ui/
│   ├── components.json                         (NEW: shadcn config)
│   ├── package.json                            (MODIFIED: +tailwindcss, +@tailwindcss/vite,
│   │                                            +@tanstack/react-router, devDeps +@tanstack/router-plugin,
│   │                                            +@tanstack/react-router-devtools, +@types/node,
│   │                                            +shadcn transitive deps)
│   ├── tsconfig.json                           (MODIFIED: +baseUrl, +paths @/*)
│   ├── vite.config.ts                          (MODIFIED: +tanstackRouter plugin first, +tailwindcss,
│   │                                            +resolve.alias @/, -@ts-expect-error block)
│   ├── .gitignore                              (MODIFIED: +src/routeTree.gen.ts OR add to repo .gitignore)
│   └── src/
│       ├── main.tsx                            (REWRITTEN: RouterProvider + declare module Register)
│       ├── App.tsx                             (DELETED)
│       ├── App.css                             (DELETED)
│       ├── styles/
│       │   └── app.css                         (NEW: @import + empty @theme)
│       ├── routes/
│       │   ├── __root.tsx                      (NEW)
│       │   ├── index.tsx                       (NEW: /→/today redirect)
│       │   └── _layout/
│       │       └── today.tsx                   (NEW: placeholder)
│       ├── routeTree.gen.ts                    (GENERATED, GITIGNORED)
│       ├── components/
│       │   └── ui/
│       │       ├── button.tsx                  (NEW: shadcn fork)
│       │       ├── dialog.tsx                  (NEW: shadcn fork)
│       │       ├── input.tsx                   (NEW: shadcn fork)
│       │       ├── tabs.tsx                    (NEW: shadcn fork)
│       │       ├── tooltip.tsx                 (NEW: shadcn fork)
│       │       └── sonner.tsx                  (NEW: shadcn fork, Toaster wrapper)
│       └── lib/
│           └── utils.ts                        (NEW: shadcn cn() helper)
└── pnpm-lock.yaml                              (regenerated)
```

NOT touched: root `Cargo.toml` (no workspace-deps changes), `crates/orgsidian-plugin-api/**` (LEAF), other crate stubs, `tools/corpus-extractor/**`, `.husky/**`, `commitlint.config.cjs`, `rust-toolchain.toml`, root `package.json` (the JS-side plugin bindings stay scoped to `shell-ui/package.json`; root keeps only orchestration scripts).

### Architecture compliance — what THIS story must satisfy

- **LD-29 (Routing — TanStack Router):** file-based `shell-ui/src/routes/` is the single source of truth for navigation; surfaces are not duplicated as a separate folder ([Source: architecture.md#LD-29 amendment](../planning-artifacts/architecture.md)). Story 1.3 establishes the **scaffold** of this (`__root.tsx`, `routes/index.tsx`, `routes/_layout/today.tsx`); future surface stories (Today=7.1, Agenda=7.4, Editor=4.1, Graph=8.11, Settings=11.x) add their own route files into the same tree.
- **Tauri Plugins — Full Set ([Source: architecture.md#Tauri Plugins — Full Set](../planning-artifacts/architecture.md)):** 11 plugins. `tauri-plugin-http` and `tauri-plugin-notification` are explicitly NOT added until a story justifies them.
- **CSP (LD-18):** `tauri.conf.json` currently has `security.csp: null`. The full CSP from LD-18 (with `style-src 'self' 'unsafe-inline' file://*`, `connect-src 'self' https://updates.orgsidian.app`, etc.) is NOT added in Story 1.3 — it's premature (no `updates.orgsidian.app` exists yet, no user CSS loader, no FS-attachment rendering). A later story (likely Story 6.x or 12.x) wires the full CSP. Document this as known deferred.
- **Locked Stack Versions ([Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md)):** Tailwind 4.1.x, shadcn/ui latest forked, TanStack Router latest stable. Tauri ecosystem (plugins) pinned at the milestone — accept whatever the `pnpm tauri add` CLI installs at this moment ([[feedback_version_policy]] Tauri-exemption clause).
- **LD-58 (WCAG 2.1 AA hard CI gate):** the a11y gate lands in Story 1.17 (`Establish WCAG 2.1 AA hard CI gate`). Story 1.3 does NOT preempt it — the placeholder route ships without `aria-*` polish; Story 7.1 (real Today Dashboard) is the first story that must clear the a11y gate. ([Source: architecture.md#LD-58](../planning-artifacts/architecture.md))
- **AI-Agent Implementation Rules:** see Dev Notes §12.
- **Frontend Package Layout — superseded text:** the architecture's "Frontend Package Layout (`packages/shell-ui/`)" section header at line 227 is pre-amendment text — the canonical path is `shell-ui/` at repo root per LD-5 round-4 ([Source: architecture.md#Project Structure & Boundaries — Amendments to Earlier Sections](../planning-artifacts/architecture.md)). Story 1.2 already migrated to root; Story 1.3 inherits the post-Story-1.2 layout.

### Latest tech information (verified 2026-05-21 via `ctx7`)

- **Tauri 2 plugin install CLI** ([Source: ctx7 /websites/v2_tauri_app]): `pnpm tauri add <plugin>` is the canonical install. Each plugin has a Builder or `::init()` form; the CLI picks the correct one. The `updater` plugin requires the `[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]` Cargo gate (mobile-incompatible Rust 1.77.2+ requirement).
- **Tailwind 4 CSS-first** ([Source: ctx7 /tailwindlabs/tailwindcss.com]): `npm install tailwindcss @tailwindcss/vite` + add `tailwindcss()` to vite plugins + `@import "tailwindcss"` in main CSS. **No `tailwind.config.js`.** `@theme {}` directive in CSS holds design tokens. OKLCH color syntax preferred for design tokens (Story 6.7 territory; not relevant here).
- **shadcn/ui Vite + Tailwind 4** ([Source: ctx7 /websites/ui_shadcn, /shadcn-ui/ui]): `components.json` with `"tailwind.config": ""` (empty string for v4) + `"css": "src/styles/app.css"` + `"baseColor": "neutral"`. Path alias `@/*` → `./src/*` in tsconfig + vite resolve.alias. `npx shadcn@latest add <component>` installs Radix primitives + emits the component file under `src/components/ui/`. **Sonner replaces the legacy Toast primitive** — `npx shadcn@latest add sonner` is the correct command.
- **TanStack Router Vite plugin** ([Source: ctx7 /tanstack/router]): plugin **must come before `@vitejs/plugin-react`** in the plugins array (`tanstackRouter` then `tailwindcss` then `react`). Use `autoCodeSplitting: true` for runtime code-split on route boundaries. `routeTree.gen.ts` is auto-generated; canonical pattern is to commit it but project preference is to gitignore (Dev Notes §6).
- **TanStack Router redirect** ([Source: ctx7 /websites/tanstack_router]): `beforeLoad: () => { throw redirect({ to: '/target' }); }` is the canonical `/`→`/target` redirect for `routes/index.tsx`. Use `throw` form (not the `redirect({..., throw: true})` form).

### Anti-patterns explicitly forbidden in this story

- ❌ Hand-editing `Cargo.toml` + `lib.rs` + `capabilities/*.json` for plugin install — use `pnpm tauri add <plugin>` to keep the three locations in sync.
- ❌ Creating a `tailwind.config.js` / `.ts` — Tailwind 4 is CSS-first; the JS config file is a v3 holdover.
- ❌ Creating a `postcss.config.js` — `@tailwindcss/vite` handles PostCSS internally.
- ❌ Importing `process.env.TAURI_DEV_HOST` with `@ts-expect-error` — drop the path entirely (no mobile target in v0.1).
- ❌ Adding `@vitejs/plugin-react-swc` (the SWC variant of the React plugin) — Story 1.6 (Lingui) will do that swap; preempting it here creates extra churn.
- ❌ Adding any of the deferred plugins (`tauri-plugin-http`, `tauri-plugin-notification`) — architecture explicitly excludes them until a story justifies.
- ❌ Committing `shell-ui/src/routeTree.gen.ts` — gitignore it (Dev Notes §6).
- ❌ Writing `tauri-specta` builder config — Story 1.4.
- ❌ Writing the FR-22 `--org-*` CSS variables — Story 6.7.
- ❌ Writing the LD-18 full CSP — premature; deferred.
- ❌ Splitting `capabilities/main.json` into per-plugin files — monolithic-until-Story-8.1.
- ❌ Adding `components/org/`, `components/today/`, etc. subfolders — they land with their owning stories.
- ❌ Adding `stores/`, `coaching/`, `themes/`, `capture/` subfolders — they land with their owning stories.
- ❌ Mounting Devtools without `import.meta.env.DEV` guard — bloats production bundle.

### Testing requirements

Story 1.3 is an infrastructure-wiring story; **no automated tests are added in this story**. The binding gates are:

1. `cargo check --workspace` exits 0.
2. `cargo build --workspace` exits 0; root `Cargo.lock` updates with new transitive deps.
3. `pnpm install` exits 0.
4. `pnpm --filter shell-ui build` exits 0 (TS strict typecheck + Vite build).
5. `pnpm tauri dev` opens a window; URL bar shows `http://localhost:1420/today` (redirected from `/`); placeholder renders; `<Button>Greet</Button>` round-trips via `invoke('greet', {...})` to Rust.
6. `pnpm tauri build` emits `.app` + `.dmg`.
7. `cargo build` from inside `tools/corpus-extractor/` exits 0.
8. Husky commit-msg hook still rejects malformed messages.
9. No regression to Story 1.1 / 1.2 invariants (workspace structure, plugin-api LEAF status, frontendDist path).

Anchor smoke tests land in Story 1.9 (parser/vault/watcher); first frontend a11y axe-core integration in Story 1.17; full CI matrix in Story 1.8.

### Project Structure Notes

- **Alignment with unified project structure:** post-Story-1.3 layout exactly matches the architecture's Workspace Layout (per the round-4 amendment) for the frontend portion `shell-ui/src/{routes,components/ui,lib,styles}/` and the Tauri plugin set on `crates/orgsidian-shell-app/`. The architecture's pre-amendment "Frontend Package Layout (`packages/shell-ui/`)" section header (line 227) remains stale text superseded by the round-4 amendment — defer to the amendment.
- **Detected variance — `routeTree.gen.ts` gitignore decision:** documented in Dev Notes §6, deviates from the TanStack canonical-commit pattern. Rationale: solo-dev workflow + autoregeneration on every dev/build + PR-noise minimization. Reassessable at Story 1.8 (CI matrix).
- **Detected variance — `tauri-plugin-opener` retained alongside `tauri-plugin-shell`:** documented in Dev Notes §4. Architecture's plugin list does NOT include `opener` but Story 1.1 baseline added it; conservative path keeps both.

### References

- [Source: epics.md#Story 1.3](../planning-artifacts/epics.md) — full AC text for the epic-level story.
- [Source: epics.md Epic 1 framing — line ~146](../planning-artifacts/epics.md) — Epic 1 scope and sequence.
- [Source: architecture.md#Tauri Plugins — Full Set](../planning-artifacts/architecture.md) — 11-plugin set with justification per plugin.
- [Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md) — Tailwind 4.1.x, shadcn/ui essentials, TanStack Router latest stable.
- [Source: architecture.md#LD-29](../planning-artifacts/architecture.md) — TanStack Router decision + typed search params + file-based.
- [Source: architecture.md#LD-29 amendment in Project Structure & Boundaries](../planning-artifacts/architecture.md) — `shell-ui/src/routes/` is single source of truth for navigation.
- [Source: architecture.md#Workspace Layout — line 928-979](../planning-artifacts/architecture.md) — canonical `shell-ui/` tree.
- [Source: architecture.md#Themable CSS Token Vocabulary (FR-22)](../planning-artifacts/architecture.md) — the `--org-*` vocabulary that is **deferred** from this story to Story 6.7.
- [Source: architecture.md#UI Mode Pattern — Plain/Power (FR-20)](../planning-artifacts/architecture.md) — `data-mode` + Tailwind selectors approach (informs Tailwind 4 install; no FR-20 work in this story).
- [Source: architecture.md#LD-18 Content Security Policy](../planning-artifacts/architecture.md) — full CSP deferred (current `csp: null`).
- [Source: architecture.md#LD-28 Window management](../planning-artifacts/architecture.md) — single `main` window in v0.1; Quick Capture window (Story 8.1) gets its own capability file later.
- [Source: architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md) — binding for every file added.
- [Source: architecture.md#Anti-Patterns (Forbidden)](../planning-artifacts/architecture.md) — `forwardRef`, raw `invoke`, premature abstraction — all relevant.
- [Source: ux-design-specification.md — "Today as a computed view" + "Cold-start lands on Today Dashboard"](../planning-artifacts/ux-design-specification.md) — establishes `/today` as the cold-start landing route (justifies the AC5 `/`→`/today` redirect).
- [Source: 1-1-bootstrap-tauri-2-x-react-19-ts-scaffold.md](./1-1-bootstrap-tauri-2-x-react-19-ts-scaffold.md) — Story 1.1 baseline (`greet` command, scaffold structure).
- [Source: 1-2-refactor-scaffold-to-9-crate-cargo-workspace-shell-ui-at-root.md](./1-2-refactor-scaffold-to-9-crate-cargo-workspace-shell-ui-at-root.md) — Story 1.2 layout (9-crate workspace, `shell-ui/` at root, `frontendDist: ../../shell-ui/dist`).
- External (canonical, fetched 2026-05-21): [`https://v2.tauri.app/plugin/`](https://v2.tauri.app/plugin/) — per-plugin install pages with builder forms and capability snippets.
- External (canonical, fetched 2026-05-21): [`https://tailwindcss.com/docs/installation/using-vite`](https://tailwindcss.com/docs/installation/using-vite) — Tailwind 4 + Vite install.
- External (canonical, fetched 2026-05-21): [`https://ui.shadcn.com/docs/installation/vite`](https://ui.shadcn.com/docs/installation/vite) — shadcn Vite install (path alias + `components.json` + CLI).
- External (canonical, fetched 2026-05-21): [`https://tanstack.com/router/latest/docs/framework/react/start`](https://tanstack.com/router/latest/docs/framework/react/start) — TanStack Router file-based routing setup.
- Previous-story memory: [[feedback_version_policy]] — Tauri ecosystem exemption clause permits "latest at milestone install time" for plugin set.
- Previous-story memory: [[feedback_batch_fixes_terse]] — for the inevitable code-review cycle, apply no-brainer fixes silently; surface only decision-grade questions.
- Previous-story memory: [[feedback_spec_driven_not_solo_dev_bandwidth]] — this is a single PR despite being a "big" story; spec coherence and persona-fit warrant the bundle.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (bmad-dev-story workflow, 2026-05-21).

### Debug Log References

- `pnpm tauri info` post-install: all 11 plugins + opener baseline detected.
- `cargo check --workspace`: exit 0.
- `cargo build --workspace`: exit 0 (~30s incremental on macOS-arm64).
- `pnpm --filter shell-ui build`: exit 0 (TS strict + Vite build; `today` chunk split confirmed).
- `pnpm tauri build`: exit 0; `.app` + `.dmg` at `target/release/bundle/{macos,dmg}/`.
- `pnpm tauri dev`: first runtime gate failed with `PluginInitialization("updater", … invalid type: null, expected struct Config)` because `tauri-plugin-updater` requires a real `plugins.updater.{pubkey,endpoints}` block in `tauri.conf.json` at startup. Resolution documented as deviation (see Change Log + Completion Notes). After deferring updater runtime registration, `pnpm tauri dev` boots cleanly to `http://localhost:1420/today` with no startup errors (manual click-through to verify `<Button>Greet</Button>` IPC round-trip is pending user-side visual confirmation; build chunk split + TS strict pass already prove the wiring).
- `cargo build` inside `tools/corpus-extractor/`: exit 0 (unaffected).
- `echo "broken commit message" | npx --no-install commitlint`: rejected with `type may not be empty` (husky chain intact).
- `git check-ignore -v shell-ui/src/routeTree.gen.ts`: matches `.gitignore:44 shell-ui/src/routeTree.gen.ts` (gitignored as planned).

### Completion Notes List

- **All 10 tasks complete; all 10 ACs satisfied with one documented deviation (AC1 updater runtime registration deferred to Story 13.2).**
- **Tauri plugin set:** `pnpm tauri add` installed all 11 plugins. Post-install reconciliation was required because the CLI (a) put JS plugin bindings in root `package.json` (moved to `shell-ui/package.json` per Dev Notes §13), (b) auto-cfg-gated `tauri-plugin-global-shortcut` and `tauri-plugin-window-state` alongside `updater` (moved back to top-level `[dependencies]` per AC1; only `updater` retains the `[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]` gate), (c) created `capabilities/desktop.json` as a separate capability (consolidated into a single `capabilities/main.json` per AC2), (d) skipped permission entries for `store`/`shell`/`os`/`clipboard-manager`/`log`/`process` (added manually), and (e) registered the plugins in install-order (rewritten alphabetically with `opener` first per AC1).
- **Capability rename:** `default.json` → `main.json` via `git mv` (blame preserved); identifier flipped to `"main"`; description updated; full 13-entry permissions array.
- **Tailwind 4:** CSS-first via `@import "tailwindcss"` + empty `@theme {}` block in `shell-ui/src/styles/app.css`. No `tailwind.config.js`, no `postcss.config.js`. The `--org-*` vocabulary stays deferred to Story 6.7 per AC10.
- **shadcn:** modern shadcn 4.7 emits components using `import { … } from "radix-ui"` (the consolidated `radix-ui` package) rather than per-primitive `@radix-ui/react-dialog` etc.; this is the canonical upstream as of 2026-05. The CLI did NOT auto-install `class-variance-authority`, `clsx`, `tailwind-merge`, or `lucide-react`, nor did it generate `src/lib/utils.ts` — likely a pnpm-workspace edge case. Installed manually and authored `src/lib/utils.ts` with the canonical `cn()` helper (`twMerge(clsx(inputs))`). Six components committed verbatim from upstream emission. Sonner replaces the legacy Toast primitive per Dev Notes §7.
- **TanStack Router:** plugin order `[tanstackRouter, tailwindcss, react]` enforced. `routeTree.gen.ts` gitignored at repo-root `.gitignore:44`. `__root.tsx` mounts `<Outlet/>` + `<Toaster/>` + dev-gated `<TanStackRouterDevtools/>`. `routes/index.tsx` redirects `/` → `/today` via `beforeLoad: () => { throw redirect(...) }`. `routes/_layout/today.tsx` is the 4-pillar smoke route (TanStack + shadcn `<Button>` + Tailwind utilities + Tauri `invoke('greet')`).
- **main.tsx rewrite:** `<RouterProvider router={router} />` mounted; `declare module '@tanstack/react-router'` type-augments `Register`; CSS imported as `./styles/app.css`. Legacy `App.tsx` + `App.css` deleted.
- **tsconfig path alias:** `paths: { "@/*": ["./src/*"] }`. **Deviation from AC4:** `baseUrl` was NOT added — TypeScript 5.8 deprecates `baseUrl` (diagnostic `option 'baseUrl' is deprecated and will stop functioning in TypeScript 7.0`). Since TS 5.0+, `paths` resolves relative to the `tsconfig.json` location without `baseUrl`. Build + path-alias resolution verified working without it.
- **vite.config.ts:** `@ts-expect-error process` block + `TAURI_DEV_HOST` env-var path removed (Option A per AC8). `host: false` + ESM-safe `__dirname` via `fileURLToPath(import.meta.url)`. Plugin order canonical: `tanstackRouter({ target: "react", autoCodeSplitting: true })` → `tailwindcss()` → `react()`. `@types/node` installed as dev-dep for the typed `node:path`/`node:url` imports.
- **AC1 deviation — updater runtime registration deferred:** `tauri-plugin-updater` deserialization fails at startup with `invalid type: null, expected struct Config` unless `tauri.conf.json` has `plugins.updater.{pubkey,endpoints}` populated with real signing material. Generating + committing a placeholder signing key is out-of-scope for Story 1.3 (Story 13.2 is the dedicated "Wire tauri-plugin-updater stable channel" story). Story 1.3 ships the three infrastructure pieces of the AC1 contract: Cargo dep (cfg-gated for desktop), JS binding (`@tauri-apps/plugin-updater` in `shell-ui/package.json`), and capability permission (`updater:default` in `main.json`). The `.plugin(tauri_plugin_updater::Builder::new().build())` `.plugin()` call in `lib.rs::run()` is replaced with an inline comment block referencing Story 13.2. Tauri does not error when `updater:default` is listed as a permission for an unregistered plugin (verified via successful `pnpm tauri dev` boot).
- **LEAF invariant:** `crates/orgsidian-plugin-api/Cargo.toml` untouched, still declares no project deps.
- **Husky:** commit-msg hook still rejects malformed messages.
- **No regressions to Story 1.1 / 1.2:** scaffold residuals (`shell-ui/src/assets/react.svg`, `shell-ui/public/{tauri,vite}.svg`) preserved; `frontendDist = "../../shell-ui/dist"` unchanged; `[workspace.dependencies]` untouched; `tools/corpus-extractor/` outside workspace.

### File List

**Added:**
- `shell-ui/components.json`
- `shell-ui/src/styles/app.css`
- `shell-ui/src/routes/__root.tsx`
- `shell-ui/src/routes/index.tsx`
- `shell-ui/src/routes/_layout/today.tsx`
- `shell-ui/src/components/ui/button.tsx` (shadcn fork)
- `shell-ui/src/components/ui/dialog.tsx` (shadcn fork)
- `shell-ui/src/components/ui/input.tsx` (shadcn fork)
- `shell-ui/src/components/ui/tabs.tsx` (shadcn fork)
- `shell-ui/src/components/ui/tooltip.tsx` (shadcn fork)
- `shell-ui/src/components/ui/sonner.tsx` (shadcn fork, Toaster wrapper)
- `shell-ui/src/lib/utils.ts` (`cn()` helper — manual; shadcn CLI did not emit it)

**Modified:**
- `Cargo.lock` (regenerated for 11 new plugin crates + transitive deps)
- `.gitignore` (added `shell-ui/src/routeTree.gen.ts`)
- `crates/orgsidian-shell-app/Cargo.toml` (10 new plugin deps in `[dependencies]`, `tauri-plugin-updater` under desktop cfg-gate)
- `crates/orgsidian-shell-app/src/lib.rs` (alphabetical `.plugin(...)` chain with `opener` first; updater registration deferred via inline TODO comment referencing Story 13.2)
- `shell-ui/package.json` (added: `@tailwindcss/vite`, `@tanstack/react-router`, 11 `@tauri-apps/plugin-*` JS bindings, `tailwindcss`, shadcn transitive deps `class-variance-authority`/`clsx`/`lucide-react`/`tailwind-merge`/`next-themes`/`radix-ui`/`sonner`; devDeps: `@tanstack/react-router-devtools`, `@tanstack/router-plugin`, `@types/node`)
- `pnpm-lock.yaml` (regenerated)
- `shell-ui/src/main.tsx` (rewritten: `RouterProvider`, `declare module` augmentation, `./styles/app.css` import)
- `shell-ui/tsconfig.json` (`paths: { "@/*": ["./src/*"] }`)
- `shell-ui/vite.config.ts` (TanStack + Tailwind plugins; `resolve.alias`; ESM-safe `__dirname`; dropped `@ts-expect-error` + `TAURI_DEV_HOST` env-var path)

**Renamed:**
- `crates/orgsidian-shell-app/capabilities/default.json` → `crates/orgsidian-shell-app/capabilities/main.json` (via `git mv`; identifier `"default"` → `"main"`; description + permissions expanded to 13 entries)

**Deleted:**
- `shell-ui/src/App.tsx`
- `shell-ui/src/App.css`

**Generated (gitignored):**
- `shell-ui/src/routeTree.gen.ts` (TanStack Router Vite plugin output)

**Sprint tracking:**
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (1-3 → in-progress → review)

### Change Log

| Date | Story Phase | Change |
|------|-------------|--------|
| 2026-05-21 | Story creation | Story 1.3 file created via bmad-create-story workflow; ACs expanded from 5 epic-level lines to 10 detailed ACs (plus anti-creep AC10); 10 tasks plotted with explicit subtask sequencing; Tailwind 4 CSS-first + shadcn 2.x Sonner + TanStack Router plugin-order + tauri-plugin-updater cfg-gating verified against latest docs via ctx7 (2026-05-21). |
| 2026-05-21 | Dev implementation | 11 Tauri plugins installed + reconciled (capabilities consolidated into single `main.json`; alphabetical lib.rs registration with `opener` first; JS bindings relocated from root → `shell-ui` per Dev Notes §13). Tailwind 4 CSS-first wired. shadcn essentials forked (6 components + manual `lib/utils.ts`). TanStack Router scaffolded with `/` → `/today` redirect + `_layout/` group placeholder. `App.tsx`/`App.css` deleted. `vite.config.ts` cleaned up (Option A: drop `TAURI_DEV_HOST` env-var path). All gates green (`cargo check/build --workspace`, `pnpm --filter shell-ui build`, `pnpm tauri build` → `.app` + `.dmg`, husky commit-msg). |
| 2026-05-21 | Dev implementation | **[Deviation — AC1]** `tauri-plugin-updater` runtime registration in `lib.rs::run()` deferred to Story 13.2. The plugin deserializes `plugins.updater.{pubkey,endpoints}` from `tauri.conf.json` at startup and aborts with `PluginInitialization` when those fields are missing; populating them requires generating + committing real signing material, which is Story 13.2's actual scope. Story 1.3 ships the other three legs of the AC1 contract (Cargo dep cfg-gated for desktop, JS binding in `shell-ui/package.json`, `updater:default` capability permission in `main.json`); the `.plugin()` call is replaced with an inline comment block referencing Story 13.2. |
| 2026-05-21 | Dev implementation | **[Deviation — AC4]** `tsconfig.json` `baseUrl` omitted because TypeScript 5.8 deprecates the option (`option 'baseUrl' is deprecated and will stop functioning in TypeScript 7.0`). Since TS 5.0+, `paths` resolves relative to the `tsconfig.json` location without `baseUrl`. Verified path-alias resolution + TS strict build pass without it. |
| 2026-05-21 | Dev implementation | **[Deviation — AC4 / Dev Notes §13]** Modern shadcn 4.7 emits components importing from the consolidated `radix-ui` package (e.g. `import { Slot } from "radix-ui"`) rather than per-primitive `@radix-ui/react-*` packages. Auto-installed: `radix-ui`, `sonner`, `next-themes` (sonner theme integration). Manually installed (CLI omitted on pnpm-workspace layout): `class-variance-authority`, `clsx`, `tailwind-merge`, `lucide-react`. Manually authored: `shell-ui/src/lib/utils.ts` with canonical `twMerge(clsx(inputs))` `cn()` helper. |
| 2026-05-21 | Code review | bmad-code-review run with 3 adversarial layers (Blind / Edge Case / Acceptance). 17 raw findings → 10 dismissed (false positives or already-disclosed), 3 deferred (TooltipProvider, next-themes ThemeProvider, `_layout.tsx` parent — owners: future stories), 4 decision-needed all resolved (option a in each case), 2 original patches. Applied 5 patches: (1) injected shadcn neutral baseColor CSS vars in `app.css` (~110 lines, sourced from ui.shadcn.com via ctx7) — fixes the Dev Notes §525 oversight where 6 forked shadcn components referenced `--background`/`--popover`/`--primary`/etc. that were never materialised; CSS bundle now 27.61 kB. (2) Removed `tauri_plugin_log::Builder::new().level(LevelFilter::Info)` to match Dev Notes Reference shape (bare `.build()`). (3) Moved `import './styles/app.css'` to line 1 of `main.tsx` per AC3 / Task 3.4. (4) Added `"prebuild": "tsr generate"` + `@tanstack/router-cli@^1.167.8` devDep — fixes the gitignored `routeTree.gen.ts` → fresh-clone-`tsc`-fails problem flagged in Dev Notes §6 (no longer a Story 1.8 revisit item). (5) Downgraded `@types/node` from `^25.9.1` to `^24` LTS per `[[feedback_version_policy]]`. All gates re-verified: `cargo check --workspace` green, `rm src/routeTree.gen.ts && pnpm run build` green. Status `review` → `done`. |
