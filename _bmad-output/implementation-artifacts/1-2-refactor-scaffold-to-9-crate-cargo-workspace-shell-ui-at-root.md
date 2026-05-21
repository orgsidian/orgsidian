# Story 1.2: Refactor scaffold to 9-crate Cargo workspace + `shell-ui/` at root

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want the Story 1.1 flat scaffold reorganized into the 9-crate Cargo workspace (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) with the React app moved to `shell-ui/` at the repo root and a `tools/corpus-extractor/` standalone tool outside the workspace,
so that every subsequent epic adds code into a stable, boundary-enforced module structure that matches the architecture's LD-5 (round-4) amendment and the FR→component mapping.

## Acceptance Criteria

1. **AC1 — Root `Cargo.toml` declares the 9-crate `[workspace]`.** The repo root contains a `Cargo.toml` with a `[workspace]` table:
   - `resolver = "2"`.
   - `members = ["crates/orgsidian-parser", "crates/orgsidian-index", "crates/orgsidian-watcher", "crates/orgsidian-vault", "crates/orgsidian-plugin-api", "crates/orgsidian-report", "crates/orgsidian-core", "crates/orgsidian-cli", "crates/orgsidian-shell-app"]` (exactly these 9 paths — order may differ; no extras).
   - A `[workspace.package]` section centralizes shared metadata (`version = "0.0.0"`, `edition = "2021"`, `license = "MIT"`, `authors = ["Tiziano Basile and Orgsidian contributors"]`, `repository = "https://github.com/orgsidian/orgsidian"`, `rust-version` aligned with `rust-toolchain.toml`).
   - A `[workspace.dependencies]` section declares the *shared* third-party deps inherited from Story 1.1 (`tauri = "2"`, `tauri-build = "2"`, `tauri-plugin-opener = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`) so member crates can `tauri.workspace = true`-style inherit; no other deps are added in this story.
   - `[profile.release]` is **NOT** set in this story — `panic = "unwind"` (LD-38) lands in Story 1.8.
   - `tools/corpus-extractor/` is **NOT** in `[workspace.members]` and is additionally listed under `[workspace] exclude = ["tools/corpus-extractor"]` to make the boundary explicit.

2. **AC2 — `crates/orgsidian-shell-app/` absorbs the old `src-tauri/` content verbatim (with re-pathed `tauri.conf.json`).** The repo no longer has a top-level `src-tauri/` directory. All Story 1.1-emitted Tauri Rust content lives under `crates/orgsidian-shell-app/`:
   - `crates/orgsidian-shell-app/Cargo.toml` — same `[package]` block as the old `src-tauri/Cargo.toml`, except `name = "orgsidian-shell-app"`, shared fields inherited via `version.workspace = true` / `edition.workspace = true` / `authors.workspace = true` / `license.workspace = true`. `description = "Tauri 2.x shell application for orgsidian"`. `[lib]` block: `name = "orgsidian_shell_app_lib"`, `crate-type = ["staticlib", "cdylib", "rlib"]` (same Windows-name-collision workaround as Story 1.1). `[build-dependencies] tauri-build.workspace = true` (with `features = []` retained as needed). `[dependencies] tauri.workspace = true`, `tauri-plugin-opener.workspace = true`, `serde.workspace = true`, `serde_json.workspace = true`. The `[lib]` rename is reflected in `src/main.rs` (was `orgsidian_lib::run()` → becomes `orgsidian_shell_app_lib::run()`).
   - `crates/orgsidian-shell-app/build.rs` — unchanged from `src-tauri/build.rs` (still `tauri_build::build()`).
   - `crates/orgsidian-shell-app/src/{main.rs,lib.rs}` — moved from `src-tauri/src/`. `lib.rs` retains the `tauri::Result<()>` signature (Story 1.1 code-review patch) and the `greet` command. `main.rs` updated to call the renamed lib (`orgsidian_shell_app_lib::run()`).
   - `crates/orgsidian-shell-app/tauri.conf.json` — moved from `src-tauri/tauri.conf.json`. `productName`, `identifier`, `version`, window `label`/`title` UNCHANGED from Story 1.1. Path fields **updated** to reflect the new location (see AC3).
   - `crates/orgsidian-shell-app/capabilities/default.json` — moved from `src-tauri/capabilities/default.json`. `$schema` reference updated to `../gen/schemas/desktop-schema.json` (the `gen/` directory will materialize under `crates/orgsidian-shell-app/gen/` on first dev/build).
   - `crates/orgsidian-shell-app/icons/` — moved from `src-tauri/icons/` (all 10 PNG + .icns + .ico). Paths in `tauri.conf.json` `bundle.icon` array remain relative (`icons/32x32.png`, etc.) since they resolve from the `tauri.conf.json` directory.
   - `crates/orgsidian-shell-app/.gitignore` — moved from `src-tauri/.gitignore`.
   - `crates/orgsidian-shell-app/Cargo.lock` — moved from `src-tauri/Cargo.lock` **OR** regenerated at the workspace root (see AC9 for Cargo.lock policy).
   - Root `.gitignore` updated: `src-tauri/target/` and `src-tauri/gen/schemas/` rules replaced with `crates/orgsidian-shell-app/target/`, `crates/orgsidian-shell-app/gen/schemas/`, and the workspace-level `target/` (which is where the Cargo workspace places build artifacts by default).

3. **AC3 — `tauri.conf.json` `beforeDevCommand`, `beforeBuildCommand`, `frontendDist`, and capability paths point at the new `shell-ui/` location.**
   - `build.beforeDevCommand` = `"pnpm --filter shell-ui dev"` (pnpm workspace filter; replaces Story 1.1's root-scoped `pnpm dev`).
   - `build.beforeBuildCommand` = `"pnpm --filter shell-ui build"`.
   - `build.devUrl` = `"http://localhost:1420"` (unchanged).
   - `build.frontendDist` = `"../../../shell-ui/dist"` — relative path from `crates/orgsidian-shell-app/tauri.conf.json` to `shell-ui/dist/`. Verify with `cd crates/orgsidian-shell-app && ls ../../../shell-ui/` resolving correctly.
   - The capability JSON's `$schema` path correctly resolves once `gen/schemas/desktop-schema.json` is materialized after the first `pnpm tauri dev` / `pnpm tauri build` (no editor warning beyond what Story 1.1 deferred item already documents).

4. **AC4 — `shell-ui/` at repo root, with `package.json` and the Vite/React app content.** The repo contains a `shell-ui/` directory at the root (NOT `packages/shell-ui/`) per LD-5 round-4 amendment ([Source: architecture.md#Project Structure & Boundaries — Amendments to Earlier Sections](../planning-artifacts/architecture.md)) and Story 1.2 epic AC ([Source: epics.md#Story 1.2](../planning-artifacts/epics.md)). It contains exactly the files moved out of the repo root plus the new `package.json`:
   - `shell-ui/package.json` — NEW. `name = "shell-ui"`, `version = "0.0.0"`, `private = true`, `type = "module"`. `scripts.dev = "vite"`, `scripts.build = "tsc && vite build"`, `scripts.preview = "vite preview"`. `dependencies`: `@tauri-apps/api`, `@tauri-apps/plugin-opener`, `react`, `react-dom` (same `^` ranges as the current root `package.json`). `devDependencies`: `@types/react`, `@types/react-dom`, `@vitejs/plugin-react`, `typescript`, `vite` (same `^` ranges). `@tauri-apps/cli` stays at the **root** `package.json` (so `pnpm tauri ...` resolves from root). NO scaffold deps duplicated at root.
   - `shell-ui/src/` — moved from root `src/`: `App.tsx`, `App.css`, `main.tsx`, `vite-env.d.ts`, `assets/react.svg`. Imports unchanged.
   - `shell-ui/public/` — moved from root `public/`: `tauri.svg`, `vite.svg`.
   - `shell-ui/index.html` — moved from root `index.html`. `<script src="/src/main.tsx">` resolves from `shell-ui/` Vite root (no path change needed).
   - `shell-ui/vite.config.ts` — moved from root `vite.config.ts`. Content unchanged except the `server.watch.ignored` pattern is updated from `**/src-tauri/**` → `**/crates/orgsidian-shell-app/**` (since the Tauri Rust source is now in `crates/orgsidian-shell-app/`, and Vite is run from `shell-ui/` so the relative path target needs to point up two levels — use `../crates/orgsidian-shell-app/**` or absolute equivalent).
   - `shell-ui/tsconfig.json` — moved from root `tsconfig.json`. Content unchanged.
   - `shell-ui/tsconfig.node.json` — moved from root `tsconfig.node.json`. Content unchanged.

5. **AC5 — Root `package.json` reduced to monorepo orchestration role.** After the refactor, the **root** `package.json` contains:
   - `name`, `version`, `private`, `description`, `type: module`, `packageManager`, `engines` (existing fields preserved).
   - `scripts`: `prepare` (husky bootstrap, preserved), `commitlint` (preserved), `tauri = "tauri"` (preserved — so `pnpm tauri dev` works from root), and **NEW** `dev = "pnpm --filter shell-ui dev"`, `build = "pnpm --filter shell-ui build"` convenience aliases. `preview` removed from root (lives at `shell-ui/package.json` now).
   - `devDependencies`: `@commitlint/cli`, `@commitlint/config-conventional`, `husky` (preserved), `@tauri-apps/cli` (preserved — needed at root for `pnpm tauri`). All other Story 1.1 scaffold devDeps (`@types/react`, `@types/react-dom`, `@vitejs/plugin-react`, `typescript`, `vite`) **REMOVED from root** (they live at `shell-ui/package.json` now per AC4).
   - `dependencies`: **EMPTY object or REMOVED entirely** — the React/Tauri-api scaffold deps live at `shell-ui/package.json` now.
   - **Description amendment:** root `description` updated from the Story 1.1 placeholder (`"...root scaffold (Conventional Commits enforcement; full Story 1.3 frontend wiring lands later)"`) to a refactored value reflecting the workspace state, e.g. `"Cross-platform desktop org-mode app — pnpm + Cargo monorepo root"`. The pnpm-11 build-script approval shim **remains in `pnpm-workspace.yaml`** (see AC6); the `pnpm-workspace.yaml` preamble comment is updated since this file is now a real workspace declaration (no longer "NOT a workspace declaration").

6. **AC6 — `pnpm-workspace.yaml` declares `shell-ui/` as the ONLY JS workspace member + retains `allowBuilds`.** The file's preamble comment is rewritten to drop the "build-script approval shim ONLY" language (now obsolete). The file content:
   - `packages: ["shell-ui"]` — Story 1.2 epic AC: "declaring it as the only JS workspace member". No other entries until a second JS package exists (v1.5+ per LD-5 round-4).
   - `allowBuilds: { esbuild: true }` — retained from Story 1.1 (pnpm 11.x build-script approval; still required to silence the "Ignored build scripts" warning).
   - `pnpm install` from repo root completes without errors. `pnpm-lock.yaml` is regenerated to reflect the workspace split (deps now resolve in `shell-ui/node_modules/.pnpm/` under the pnpm hoisting model).

7. **AC7 — Each of the 8 non-shell-app crates ships a stub `lib.rs` (or `main.rs`) that makes `cargo build --workspace` pass.** Each crate folder contains `Cargo.toml` + `src/lib.rs` (or `src/main.rs` for `orgsidian-cli`) with minimum-viable content:
   - `crates/orgsidian-parser/`, `crates/orgsidian-index/`, `crates/orgsidian-watcher/`, `crates/orgsidian-vault/`, `crates/orgsidian-report/`, `crates/orgsidian-core/`: `Cargo.toml` with `[package] name = "orgsidian-<name>"`, shared fields inherited via `*.workspace = true`. `description = "<one-line description from architecture FR mapping>"` (e.g. for `parser`: `"tree-sitter-org wrapper + semantic AST builder + serializer (FR-1, FR-2)"`). NO `[dependencies]` block beyond an empty one (these crates pull their real deps in their respective epics; this story is structural-only). `src/lib.rs` contains a single line: `//! <one-sentence summary>` (e.g. `//! orgsidian-parser: tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2).`). NO `pub fn` / no `mod` declarations — those land in the implementation stories per epic.
   - `crates/orgsidian-plugin-api/`: same shape as the leaf crates above. `src/lib.rs` contains the single doc-comment line. **Crucially**: `Cargo.toml` has NO project deps (it is the LEAF-of-leaves per LD-26 / LD-10). The full `OrgsidianPlugin` trait surface, `Event` enum, `HookOutcome`, `HookContext`, `PluginContext` trait definitions, and the `crates/orgsidian-plugin-api/CHANGELOG.md` land in Story 1.5 — DO NOT pre-implement them here.
   - `crates/orgsidian-cli/`: `Cargo.toml` `[package] name = "orgsidian-cli"`, with `[[bin]] name = "orgsidian"` (architecture's CLI command tree calls the binary `orgsidian` per LD-27). `src/main.rs` contains a minimum-viable `fn main() { println!("orgsidian CLI — see Story 2.8 onward for commands"); }` stub. NO `clap` dep yet (lands with the first CLI command story, e.g. Story 2.8 `orgsidian parse-file`).
   - **No cross-crate `[dependencies]` wiring** in this story: `orgsidian-core` does NOT yet depend on the leaves (that wiring lands in the first epic story that needs each leaf, per the architecture decision-priority order). `orgsidian-shell-app` does NOT yet depend on `orgsidian-core` (that wires up when `tauri-specta` lands in Story 1.4 / the first command-handler story). Story 1.2 is a STRUCTURAL refactor: the boundaries exist as crate folders, but the dep graph is materialized incrementally.

8. **AC8 — `tools/corpus-extractor/` exists as a standalone non-workspace crate.** The path `tools/corpus-extractor/` exists at the repo root with:
   - `tools/corpus-extractor/Cargo.toml`: `[package] name = "orgsidian-corpus-extractor"` (or `corpus-extractor` — choose the name that matches the architecture's `tools/corpus-extractor/` reference), `version = "0.0.0"`, `edition = "2021"`, `license = "MIT"`, `publish = false`, `description = "Extracts test corpus from org-element.el assertions (Story 2.5)"`. NO `*.workspace = true` inheritance (it is intentionally outside the workspace).
   - `tools/corpus-extractor/src/main.rs`: stub `fn main() { println!("corpus-extractor — see Story 2.5 for implementation"); }`.
   - `cargo build` run from `tools/corpus-extractor/` succeeds independently. `cargo build --workspace` from repo root does NOT build this crate (per `exclude` directive AC1 + the lack of `workspace.members` entry).

9. **AC9 — `cargo build --workspace` and `cargo check --workspace` pass from repo root.**
   - Running `cargo build --workspace` from `/Users/tizianobasile/workspace/me/orgsidian/` exits 0 with no warnings beyond standard unused-code notices for the stub crates (acceptable since the crates have no logic yet; the workspace-level `cargo clippy -- -D warnings` gate lands in Story 1.8).
   - `cargo check --workspace` passes (faster sanity gate).
   - `Cargo.lock` is committed at the **workspace root** (`./Cargo.lock`), NOT at `crates/orgsidian-shell-app/Cargo.lock`. The old `src-tauri/Cargo.lock` is removed during the move (Cargo workspace policy: one lockfile at the workspace root for all members).

10. **AC10 — `pnpm tauri dev` still launches the Tauri window from repo root.** With the refactor applied, running `pnpm install && pnpm tauri dev` from `/Users/tizianobasile/workspace/me/orgsidian/` produces the same observable outcome as Story 1.1: Vite starts on `http://localhost:1420/`, the Rust `crates/orgsidian-shell-app` debug target compiles, and the Tauri window opens displaying the default React scaffold content (`App.tsx` from `shell-ui/src/`). The `beforeDevCommand` (`pnpm --filter shell-ui dev`) and `frontendDist` (`../../../shell-ui/dist`) wiring resolves correctly. Verified locally on the developer machine (macOS-arm64). The "greet" command from Story 1.1 still round-trips (`invoke('greet', { name })` returns `"Hello, {name}!..."`).

11. **AC11 — `pnpm tauri build` produces a release bundle from `crates/orgsidian-shell-app/`.** Running `pnpm tauri build` from repo root succeeds and emits `.app` + `.dmg` under `target/release/bundle/macos/` and `target/release/bundle/dmg/` (workspace target dir, not `crates/orgsidian-shell-app/target/`). The `.dmg` name remains `orgsidian_0.0.0_aarch64.dmg`. No code-signing required at this stage (still Story 6.8).

12. **AC12 — Husky `commit-msg` hook still functional + root preservation rules satisfied.**
   - `.husky/`, `.husky/commit-msg`, `commitlint.config.cjs`, `scripts/sync-epics-to-github.sh`, `docs/logo-draft.png`, `_bmad/`, `_bmad-output/`, `.claude/`, root `README.md`, root `LICENSE`, root `.gitignore` (existing rules preserved; additive merge only) are **untouched** except where AC2/AC5/AC6 explicitly modify them.
   - After `pnpm install`, the husky `commit-msg` hook still rejects malformed commit messages via commitlint (same verification path as Story 1.1 AC11).
   - Root `LICENSE` (MIT) from Story 1.1 stays as the single project license file. Per-crate `LICENSE` files are NOT created in this story (the workspace inherits via `[workspace.package] license = "MIT"`); per-crate `LICENSE` symlinks/copies land at publication time for `orgsidian-plugin-api` (v1.5+ per LD-10).

13. **AC13 — `rust-toolchain.toml` committed at repo root.** A `rust-toolchain.toml` file at the repo root pins the Rust toolchain per the architecture's "Locked Stack Versions" table ([Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md)): `[toolchain] channel = "stable"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"`. This ensures every contributor and the CI matrix (Story 1.8) get a deterministic toolchain. **Optional** (defer if blocked): if pinning to a specific patch version causes friction with the developer's existing rustup config, leave `channel = "stable"` and let rustup resolve the latest stable — the CI matrix in Story 1.8 will harden this if needed.

14. **AC14 — No premature scope creep.** The following are **explicitly OUT of scope** for Story 1.2 and MUST NOT be added:
   - ❌ Tauri plugin set additions (`tauri-plugin-{fs,dialog,global-shortcut,updater,window-state,store,shell,os,clipboard-manager,log,process}`) — Story 1.3.
   - ❌ Tailwind 4, shadcn/ui, TanStack Router — Story 1.3.
   - ❌ `tauri-specta` — Story 1.4.
   - ❌ Full `OrgsidianPlugin` trait surface, `Event` enum, `HookOutcome`, `HookContext`/`PluginContext` traits, plugin-api `CHANGELOG.md` — Story 1.5.
   - ❌ Lingui v6.x + SWC plugin + Vite plugin + eslint-plugin-lingui — Story 1.6.
   - ❌ `cargo-deny` config (`deny.toml`), `cargo audit` setup — Story 1.7.
   - ❌ CI workflows (`.github/workflows/pr.yml`, `nightly.yml`), `[profile.release] panic = "unwind"`, `invoke_plugin_hook!` macro stub — Story 1.8.
   - ❌ Anchor smoke tests (parser / vault / watcher / round-trip) — Story 1.9.
   - ❌ `SECURITY.md` / `ARCHITECTURE.md` / `CHANGELOG.md` / `CONTRIBUTING.md` — Story 1.10.
   - ❌ Real cross-crate dep wiring (`orgsidian-core` depending on leaves; `orgsidian-shell-app` depending on `core`) — incremental, per first story that needs each edge.
   - ❌ Per-crate `tests/` directories — none of the stubs have logic to test in this story.

## Tasks / Subtasks

> **Recommended order:** Each task can be tested independently with `cargo check --workspace` and (after Task 6) `pnpm install`. Run that as a gating step after each major task to keep the workspace green incrementally.

- [x] **Task 1 — Pre-flight & branch (AC: all)**
  - [x] 1.1 Confirm clean working tree on `main` (`git status` clean per `Recent commits` showing PR #111 merged 2026-05-20).
  - [x] 1.2 Create a working branch `feat/story-1-2-cargo-workspace-refactor` (per project convention; the user authors the final commits — assistant does not auto-commit beyond verification).
  - [x] 1.3 Verify Rust toolchain is `stable` (`rustc --version`); install via `rustup` if missing.
  - [x] 1.4 Confirm `pnpm` ≥ 11.x and Node ≥ 22 LTS (same as Story 1.1).

- [x] **Task 2 — Create the 9-crate skeleton (AC: 1, 7)**
  - [x] 2.1 Create root `Cargo.toml` with `[workspace]`, `[workspace.package]`, `[workspace.dependencies]` per AC1. Do NOT yet add the 9 members — verify the empty workspace parses with `cargo check` first (it will error because members is empty; that's expected).
  - [x] 2.2 `mkdir -p` the 8 non-shell-app crate folders: `crates/orgsidian-{parser,index,watcher,vault,plugin-api,report,core,cli}/src/`.
  - [x] 2.3 For each of the 6 leaf-style lib crates (`parser`, `index`, `watcher`, `vault`, `report`, `core`): write `Cargo.toml` (inherit shared fields via `*.workspace = true`) + `src/lib.rs` (one-line doc comment) per AC7.
  - [x] 2.4 For `orgsidian-plugin-api`: write the LEAF-shape `Cargo.toml` (no project deps) + `src/lib.rs` (one-line doc comment). **DO NOT** add the trait surface — that's Story 1.5.
  - [x] 2.5 For `orgsidian-cli`: write `Cargo.toml` with `[[bin]] name = "orgsidian"` + `src/main.rs` stub printing the "see Story 2.8" message per AC7.
  - [x] 2.6 Add the 8 members to root `Cargo.toml` `[workspace.members]` (paths only; `crates/orgsidian-shell-app` will be added in Task 3).
  - [x] 2.7 Run `cargo check --workspace` to verify the 8 stub crates compile (the workspace will still be incomplete without `shell-app`; expect a "no shell-app member" non-error or temporarily comment out the `shell-app` member if Task 3 hasn't moved files yet).

- [x] **Task 3 — Move `src-tauri/` → `crates/orgsidian-shell-app/` (AC: 2, 3)**
  - [x] 3.1 `git mv src-tauri crates/orgsidian-shell-app` (use `git mv` to preserve history per the architecture's Decision Impact discipline — though Cargo workspace policy permits the move regardless, `git mv` keeps the blame trail intact).
  - [x] 3.2 Rewrite `crates/orgsidian-shell-app/Cargo.toml`: change `[package].name` from `"orgsidian"` to `"orgsidian-shell-app"`. Replace `version`/`edition`/`authors`/`license`/`description` with `*.workspace = true` inheritance (using project-accurate `description = "Tauri 2.x shell application for orgsidian"`). Change `[lib].name` from `"orgsidian_lib"` to `"orgsidian_shell_app_lib"`. Switch `[build-dependencies] tauri-build` and `[dependencies] tauri`/`tauri-plugin-opener`/`serde`/`serde_json` to `*.workspace = true` form.
  - [x] 3.3 Update `crates/orgsidian-shell-app/src/main.rs`: change `orgsidian_lib::run()` → `orgsidian_shell_app_lib::run()`.
  - [x] 3.4 Update `crates/orgsidian-shell-app/tauri.conf.json`:
        • `build.beforeDevCommand = "pnpm --filter shell-ui dev"`
        • `build.beforeBuildCommand = "pnpm --filter shell-ui build"`
        • `build.frontendDist = "../../../shell-ui/dist"`
        • Verify `productName`, `identifier`, `version`, window `label`/`title` unchanged.
  - [x] 3.5 Verify `crates/orgsidian-shell-app/capabilities/default.json` `$schema` path is still `../gen/schemas/desktop-schema.json` (relative to capabilities/, so the `gen/` directory will materialize at `crates/orgsidian-shell-app/gen/`).
  - [x] 3.6 Remove the old `src-tauri/Cargo.lock` (workspace policy: single root lockfile). The root `Cargo.lock` will be generated by the first `cargo build --workspace`.
  - [x] 3.7 Add `crates/orgsidian-shell-app` to root `Cargo.toml` `[workspace.members]`.

- [x] **Task 4 — Move root JS app → `shell-ui/` (AC: 4)**
  - [x] 4.1 `mkdir shell-ui` at repo root.
  - [x] 4.2 `git mv src shell-ui/src` (preserves blame on `App.tsx`, `main.tsx`, etc.).
  - [x] 4.3 `git mv public shell-ui/public` (`tauri.svg`, `vite.svg`).
  - [x] 4.4 `git mv index.html shell-ui/index.html` — verify `<script src="/src/main.tsx">` still resolves (Vite root will be `shell-ui/`).
  - [x] 4.5 `git mv vite.config.ts shell-ui/vite.config.ts` — edit `server.watch.ignored` from `["**/src-tauri/**"]` to `["**/crates/orgsidian-shell-app/**", "**/target/**"]`. Also add `"**/_bmad-output/**"` and `"**/_bmad/**"` to keep BMAD markdown churn from triggering Vite reloads.
  - [x] 4.6 `git mv tsconfig.json shell-ui/tsconfig.json` and `git mv tsconfig.node.json shell-ui/tsconfig.node.json`. No content changes needed (the `include: ["src"]` rule still resolves relative to the tsconfig location).

- [x] **Task 5 — Create `shell-ui/package.json` + reduce root `package.json` (AC: 4, 5)**
  - [x] 5.1 Create `shell-ui/package.json` per AC4 spec. Copy dep version strings verbatim from current root `package.json` (so pnpm resolves the SAME pinned versions React 19.2.6 / Vite 7.3.3 / TS 5.8.3 / Tauri 2.11.x — no opportunistic upgrades in this story).
  - [x] 5.2 Edit root `package.json` per AC5: remove the React/Vite/Tauri-api deps and devDeps that moved to `shell-ui/`; keep `@tauri-apps/cli`, commitlint, husky; add `dev` and `build` convenience aliases pointing at `pnpm --filter shell-ui`; remove `preview` (lives in `shell-ui/`); update `description`.
  - [x] 5.3 Delete the root `dist/` directory if present (Story 1.1 may have left one) — Vite output now lives at `shell-ui/dist/`.

- [x] **Task 6 — Update `pnpm-workspace.yaml` + reinstall (AC: 6)**
  - [x] 6.1 Edit `pnpm-workspace.yaml`: add `packages: ["shell-ui"]`. Keep `allowBuilds: { esbuild: true }`. Rewrite the preamble comment to reflect that this is now a real workspace declaration (drop the "NOT a workspace declaration" wording; reference Story 1.2 as the introduction point).
  - [x] 6.2 `pnpm install` from repo root: expect `pnpm-lock.yaml` to be regenerated (shell-ui deps now resolve in `shell-ui/node_modules/.pnpm/`). Verify exit 0 and no errors beyond ordinary peerDependency noise.
  - [x] 6.3 `pnpm list --filter shell-ui react react-dom` confirms React 19.x is resolved in the `shell-ui` workspace package.
  - [x] 6.4 `pnpm prepare` re-runs (husky bootstrap); confirm `.husky/commit-msg` is intact.

- [x] **Task 7 — Create `tools/corpus-extractor/` (AC: 8)**
  - [x] 7.1 `mkdir -p tools/corpus-extractor/src`.
  - [x] 7.2 Write `tools/corpus-extractor/Cargo.toml` with `[package].publish = false` and no `*.workspace = true` inheritance (intentionally standalone).
  - [x] 7.3 Write `tools/corpus-extractor/src/main.rs` stub per AC8.
  - [x] 7.4 Add `[workspace] exclude = ["tools/corpus-extractor"]` to root `Cargo.toml` (defense-in-depth — even though it's not in `members`, the explicit `exclude` documents the boundary for future maintainers).
  - [x] 7.5 Verify: `cargo build --workspace` from root does NOT compile `tools/corpus-extractor/`. `cargo build` from inside `tools/corpus-extractor/` DOES compile it independently.

- [x] **Task 8 — `rust-toolchain.toml` (AC: 13)**
  - [x] 8.1 Create repo-root `rust-toolchain.toml` with `[toolchain]` channel/components/profile per AC13.
  - [x] 8.2 Verify `rustup show` from repo root reflects the pinned toolchain.

- [x] **Task 9 — Update root `.gitignore` (AC: 2)**
  - [x] 9.1 Replace `src-tauri/target/` → `target/` (workspace target dir lives at the repo root by default for a Cargo workspace).
  - [x] 9.2 Replace `src-tauri/gen/schemas/` → `crates/orgsidian-shell-app/gen/schemas/`.
  - [x] 9.3 Also ignore `crates/orgsidian-shell-app/target/` defensively (in case Cargo emits a member-local target dir under some flows).
  - [x] 9.4 Verify `dist/` is also being ignored at the `shell-ui/dist/` path (the existing `dist/` rule at root may match deeper dirs depending on `.gitignore` semantics; if not, add `shell-ui/dist/` explicitly).

- [x] **Task 10 — Verify build + dev paths (AC: 9, 10, 11, 12)**
  - [x] 10.1 `cargo check --workspace` from repo root — must exit 0 with no errors.
  - [x] 10.2 `cargo build --workspace` from repo root — must exit 0; record `Cargo.lock` at repo root.
  - [x] 10.3 `pnpm tauri dev` from repo root: confirm Vite starts on 1420, Tauri compiles `orgsidian-shell-app`, window opens with the default React UI. `invoke('greet', { name })` round-trips correctly (test by typing a name and pressing the Greet button; matches Story 1.1 visual gate). _Config-discovery re-verified at code review 2026-05-21: `pnpm tauri info` from repo root resolves `crates/orgsidian-shell-app/tauri.conf.json` and reports `frontendDist: ../../shell-ui/dist` correctly. Visual UI gate (window paint + `greet` round-trip) still requires human eyes._
  - [x] 10.4 `pnpm tauri build` from repo root: confirm `.app` + `.dmg` emitted under repo-root `target/release/bundle/macos/` and `target/release/bundle/dmg/`.
  - [x] 10.5 Commit-message hook sanity check on a throwaway branch (same approach as Story 1.1 Task 7 — `git commit -m "broken commit message"` should be rejected by commitlint). _Verified at code review 2026-05-21: `echo "this is not conventional" | npx --no-install commitlint` exits 1 with `subject-empty` + `type-empty` violations. Commitlint config + hook chain intact._
  - [x] 10.6 `cargo build` from inside `tools/corpus-extractor/` (independent verification per Task 7.5).

- [x] **Task 11 — Final sweep (AC: all)**
  - [x] 11.1 `git status --short` shows only intentional changes (the move-set, new files, edited `package.json`/`pnpm-workspace.yaml`/`.gitignore`, new root `Cargo.toml`, new `rust-toolchain.toml`).
  - [x] 11.2 Verify no top-level `src/`, no top-level `src-tauri/`, no top-level `index.html`, no top-level `vite.config.ts`, no top-level `tsconfig*.json`, no top-level `public/`.
  - [x] 11.3 Confirm AC1-AC14 each pass via checkbox sweep.
  - [x] 11.4 Update the Dev Agent Record File List section below.

### Review Findings

_Code review 2026-05-21 — diff vs HEAD (uncommitted on `feat/story-1-2-cargo-workspace-refactor`). 3 layers: Blind Hunter, Edge Case Hunter, Acceptance Auditor._

**Resolved during review (all patched in-session):**

- [x] [Review][Patch] `tools/corpus-extractor/Cargo.lock` committed — kept committed (standard Cargo posture for standalone binary crates outside any workspace); Completion Notes corrected to reflect the actual committed state.
- [x] [Review][Patch] Workspace `authors` field — changed from non-RFC822 single string `"Tiziano Basile and Orgsidian contributors"` to RFC822-compliant dual entry `["Tiziano Basile <tiz.basile@gmail.com>", "Orgsidian contributors"]` using the user's personal Gmail (not the work address).
- [x] [Review][Patch] Story task hygiene (10.3 & 10.5) — both tasks now carry inline verification notes citing the runtime checks performed during code review (commitlint exit-1 reject + `pnpm tauri info` config-discovery). 10.3's residual visual UI gate (window paint + `greet` round-trip) explicitly remains a human eye-check.
- [x] [Review][Dismiss] `pnpm.allowBuilds` validity — confirmed via pnpm docs (`/websites/pnpm_io`): `allowBuilds` IS the correct pnpm v11+ key, replacing the deprecated `onlyBuiltDependencies` / `neverBuiltDependencies` / `ignoredBuiltDependencies` (added in v10.26, formalized v11.0). Current `pnpm-workspace.yaml` is correct.
- [x] [Review][Dismiss] Tauri CLI auto-discovery — confirmed via `pnpm tauri info` from repo root: CLI resolves `crates/orgsidian-shell-app/tauri.conf.json` correctly and reports `frontendDist: ../../shell-ui/dist`. No `--config` flag required.
- [x] [Review][Patch] FR citations in 4 stub crate descriptions — corrected to match FR Coverage Map in `epics.md`:
  - `crates/orgsidian-index/Cargo.toml:3` now cites `FR-12, FR-17` (was `FR-7, FR-8, FR-9`).
  - `crates/orgsidian-vault/Cargo.toml:3` now cites `FR-15, NFR-14, NFR-15` (was `FR-3, FR-4, FR-5`).
  - `crates/orgsidian-watcher/Cargo.toml:3` now cites `FR-16, NFR-16` (was `FR-10`).
  - `crates/orgsidian-report/Cargo.toml:3` now cites `FR-14, LD-14` (was `FR-13, LD-14`).

**Defer:**

- [x] [Review][Defer] `crates/orgsidian-shell-app/Cargo.toml` lost rename detection — git shows it as `new file mode` + `deleted file mode` rather than rename (Blind Hunter). Content diverged enough that git similarity fell below threshold. Cannot retroactively fix; blame trail lost on this single file. Other Tauri files (build.rs, icons, tauri.conf.json, src/lib.rs) preserved rename history correctly.

**Dismissed (10): AC3 path literal mismatch (spec was wrong, code correct, documented in story Change Log) · `shell-ui/dist/` gitignore (Blind misread — actually present in `.gitignore:35`) · `tauri-plugin-opener` stylistic shape inconsistency · corpus-extractor binary vs printed name · vite watch `ignored` globs partly dead code (harmless) · `gen/schemas/` first-clone IDE schema warning (expected Tauri behavior) · `bundle.icon` Windows-Square tiles absent (matches Tauri 2 default scaffold) · `version = "0.0.0"` future publish concern · `[workspace.dependencies]` tauri-* unused by stubs (intentional inheritance hooks) · `frontendDist` verified correct.**

## Dev Notes

### Critical context the dev agent MUST internalize before touching code

**1. This is a STRUCTURAL refactor, NOT a feature story.** The goal is to land the 9-crate boundary as a *folder* structure with the LD-26 LEAF-crate invariant respected (`orgsidian-plugin-api` has no project deps). Cross-crate dependency wiring (`core` → `parser`/`index`/`watcher`/`vault`; `shell-app` → `core`) is **incremental** — each first-use story adds the edge. Pre-wiring all edges here would (a) violate scope, (b) force premature import of empty stubs, (c) make `cargo build --workspace` brittle to per-crate dev. ([Source: architecture.md#Crate Dependency Graph](../planning-artifacts/architecture.md))

**2. The `orgsidian-plugin-api` crate is the LEAF-of-leaves.** No project deps. No external deps either (in this story). Per LD-26 (round-4 amendment), `HookContext` and `PluginContext` are **traits** so the crate stays leaf — `orgsidian-core` will provide the concrete implementations later. **DO NOT** write any trait definitions in Story 1.2; Story 1.5 owns the full `OrgsidianPlugin` trait + `Event` enum + `HookOutcome` + `HookContext`/`PluginContext` trait shapes per AC of Story 1.5. ([Source: epics.md#Story 1.5](../planning-artifacts/epics.md), [Source: architecture.md#LD-26 Plugin API trait](../planning-artifacts/architecture.md))

**3. `shell-ui/` lives at REPO ROOT — NOT `packages/shell-ui/`.** LD-5 round-4 amendment supersedes earlier text. The architecture's "Frontend Package Layout (`packages/shell-ui/`)" section header is now obsolete; the canonical layout is the "Project Structure & Boundaries → Workspace Layout" tree which shows `shell-ui/` at root. Cross-references in the architecture file that still say `packages/shell-ui/` are pre-amendment text — defer to the amendment. ([Source: architecture.md#Project Structure & Boundaries — Amendments to Earlier Sections](../planning-artifacts/architecture.md), [Source: epics.md#Story 1.2 (line 146 framing)](../planning-artifacts/epics.md))

**4. `tools/corpus-extractor/` is INTENTIONALLY OUTSIDE the workspace.** It has its own `Cargo.toml` with `publish = false` and is NOT in `[workspace.members]`. Adding it to `[workspace] exclude = ["tools/corpus-extractor"]` is defense-in-depth — explicit-better-than-implicit for future maintainers and the Story 1.7 `cargo-deny check graph` rule. ([Source: architecture.md#Workspace Layout line 996-999](../planning-artifacts/architecture.md), [Source: epics.md#Story 1.2 AC last bullet](../planning-artifacts/epics.md))

**5. Cargo workspace: SINGLE `Cargo.lock` at workspace root.** Cargo policy: the workspace root owns the lockfile; member crates do not have their own `Cargo.lock`. The Story 1.1-emitted `src-tauri/Cargo.lock` is REMOVED in Task 3.6; the root `Cargo.lock` materializes on first `cargo build --workspace`.

**6. The `[lib].name = "orgsidian_lib"` → `"orgsidian_shell_app_lib"` rename is intentional.** Story 1.1 used `orgsidian_lib` because the crate was named `orgsidian`. After the refactor, the crate is `orgsidian-shell-app`, so the `_lib` suffix follows the new name. The rename is mechanical: update `[lib].name` in `Cargo.toml` AND the `main.rs` call site. (Cargo's per-crate lib name uniqueness rule cited in the Story 1.1 `Cargo.toml` comment still applies — Windows only — but the rename keeps both `staticlib` and `cdylib` distinct from the binary.)

**7. Tauri config paths are relative to `tauri.conf.json` location.** When you move `tauri.conf.json` from `src-tauri/` to `crates/orgsidian-shell-app/`, `build.frontendDist`'s prior value (`../dist`, which resolved to repo-root `dist/`) becomes invalid. New value: `../../../shell-ui/dist` — three `..` up from `crates/orgsidian-shell-app/` to repo root, then into `shell-ui/dist`. Verify by `cd crates/orgsidian-shell-app && readlink -f ../../../shell-ui/` resolves to the actual `shell-ui/` directory. Similarly, the icon paths (`icons/32x32.png`, etc.) in `tauri.conf.json` resolve relative to the conf file — since icons moved with the conf to `crates/orgsidian-shell-app/icons/`, NO change is needed there.

**8. `beforeDevCommand` / `beforeBuildCommand` must use `pnpm --filter shell-ui`.** With the new pnpm workspace, the Vite dev/build scripts live at `shell-ui/package.json`. Running `pnpm dev` from repo root would try the (removed) root `dev` script. The `--filter shell-ui` form tells pnpm to run the script in the `shell-ui` workspace member. ([Source: pnpm workspace filter docs](https://pnpm.io/filtering) — verified via `ctx7` if uncertain; pnpm 11.x stable behavior)

**9. `vite.config.ts` `server.watch.ignored` path update.** Previously `**/src-tauri/**` excluded the Rust dir from Vite's watcher. After the refactor, the Rust target dir is `crates/orgsidian-shell-app/`, and the Cargo target dir is workspace-root `target/`. Add BOTH to `server.watch.ignored`: `["**/crates/orgsidian-shell-app/**", "**/target/**"]`. Also exclude `**/_bmad-output/**` + `**/_bmad/**` (BMAD-driven markdown churn would otherwise restart Vite every time epics.md is touched).

**10. AI-Agent Implementation Rules apply (architecture §AI-Agent Implementation Rules).** Even for stub `lib.rs` files: no `unwrap()`/`expect()` in production paths (test code may), one concern per file, no `any` in TS (no TS code added in this story but `shell-ui/vite.config.ts` retains the `@ts-expect-error` workaround from Story 1.1's deferred review item — DO NOT remove it; Story 1.3 owns its replacement). ([Source: architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md))

### Reference root `Cargo.toml` shape

```toml
[workspace]
resolver = "2"
members = [
    "crates/orgsidian-parser",
    "crates/orgsidian-index",
    "crates/orgsidian-watcher",
    "crates/orgsidian-vault",
    "crates/orgsidian-plugin-api",
    "crates/orgsidian-report",
    "crates/orgsidian-core",
    "crates/orgsidian-cli",
    "crates/orgsidian-shell-app",
]
exclude = ["tools/corpus-extractor"]

[workspace.package]
version = "0.0.0"
edition = "2021"
license = "MIT"
authors = ["Tiziano Basile and Orgsidian contributors"]
repository = "https://github.com/orgsidian/orgsidian"
# rust-version intentionally OMITTED in Story 1.2 — `rust-toolchain.toml` pins
# `stable` and Story 1.8 will harden CI; an MSRV declaration is deferred to
# Story 1.10 (CONTRIBUTING.md owns the policy text).

[workspace.dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

> **DO NOT** add `[profile.release]` here — Story 1.8 owns `panic = "unwind"` per LD-38.
> **DO NOT** add `[workspace.lints]` — Story 1.8 owns the workspace-wide clippy gate (`-D warnings`).

### Reference `crates/orgsidian-shell-app/Cargo.toml` shape (after refactor)

```toml
[package]
name = "orgsidian-shell-app"
description = "Tauri 2.x shell application for orgsidian"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[lib]
name = "orgsidian_shell_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { workspace = true }

[dependencies]
tauri = { workspace = true }
tauri-plugin-opener = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

### Reference stub `crates/orgsidian-parser/Cargo.toml` shape (apply same pattern to all 5 other leaf-style crates)

```toml
[package]
name = "orgsidian-parser"
description = "tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2)"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
# Real deps (tree-sitter, etc.) added in Story 2.x.
```

### Reference stub `crates/orgsidian-parser/src/lib.rs`

```rust
//! orgsidian-parser: tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2).
//!
//! Structural placeholder — implementation lands in Story 2.1+ per the epic-2 sequence.
```

> Per AI-Agent Implementation Rules §1 ("One concern per file") and the architecture's "Crate organization (`crates/<name>/src/`)" rule ("`lib.rs` — public surface re-exports only; no logic"), the `lib.rs` files in this story contain **only the doc comment** — no `pub fn`, no `mod`, no `use`. Each later epic-2/3/4/5 story adds the modules that crate needs.

### Reference `crates/orgsidian-plugin-api/Cargo.toml` (LEAF — no project deps)

```toml
[package]
name = "orgsidian-plugin-api"
description = "Plugin API trait + Event enum + HookOutcome + HookContext/PluginContext traits (LEAF crate; published at v1.5+ per LD-10)"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
# LEAF crate — no project deps. Full trait surface lands in Story 1.5.
```

### Reference `crates/orgsidian-cli/Cargo.toml` (bin crate)

```toml
[package]
name = "orgsidian-cli"
description = "orgsidian headless CLI (parse, index, query, validate-plugin) per LD-27"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "orgsidian"
path = "src/main.rs"
```

### Reference `tools/corpus-extractor/Cargo.toml` (standalone, NOT in workspace)

```toml
[package]
name = "orgsidian-corpus-extractor"
version = "0.0.0"
edition = "2021"
license = "MIT"
publish = false
description = "Extracts test corpus from org-element.el assertions (Story 2.5)"

[dependencies]
# Standalone — no project deps. Real deps land in Story 2.5.
```

### Reference `shell-ui/package.json` shape

```json
{
  "name": "shell-ui",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "react": "^19.1.0",
    "react-dom": "^19.1.0"
  },
  "devDependencies": {
    "@types/react": "^19.1.8",
    "@types/react-dom": "^19.1.6",
    "@vitejs/plugin-react": "^4.6.0",
    "typescript": "~5.8.3",
    "vite": "^7.0.4"
  }
}
```

### Reference reduced root `package.json` shape

```json
{
  "name": "orgsidian",
  "version": "0.0.0",
  "private": true,
  "description": "Cross-platform desktop org-mode app — pnpm + Cargo monorepo root",
  "type": "module",
  "scripts": {
    "prepare": "husky",
    "commitlint": "commitlint",
    "dev": "pnpm --filter shell-ui dev",
    "build": "pnpm --filter shell-ui build",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@commitlint/cli": "21.0.1",
    "@commitlint/config-conventional": "21.0.1",
    "@tauri-apps/cli": "^2",
    "husky": "9.1.7"
  },
  "packageManager": "pnpm@11.1.1",
  "engines": {
    "node": ">=22"
  }
}
```

### Reference `pnpm-workspace.yaml` shape (after refactor)

```yaml
# pnpm workspace declaration — Story 1.2 establishes the JS sub-workspace
# with shell-ui as the only member (per LD-5 round-4 amendment: no
# packages/ indirection until a 2nd JS package exists).
# allowBuilds remains the pnpm 11.x build-script approval (Story 1.1 lineage).
packages:
  - shell-ui
allowBuilds:
  esbuild: true
```

### Reference `rust-toolchain.toml` shape

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

### Post-story file structure (target)

```
orgsidian/
├── Cargo.toml                            (NEW — [workspace] root)
├── Cargo.lock                            (NEW — generated by first `cargo build --workspace`)
├── rust-toolchain.toml                   (NEW)
├── package.json                          (MODIFIED — reduced to monorepo orchestration)
├── pnpm-workspace.yaml                   (MODIFIED — adds `packages: [shell-ui]`)
├── pnpm-lock.yaml                        (regenerated by `pnpm install`)
├── LICENSE                               (unchanged — MIT)
├── README.md                             (unchanged)
├── .gitignore                            (MODIFIED — path updates per Task 9)
├── .husky/                               (unchanged)
├── commitlint.config.cjs                 (unchanged)
├── docs/                                 (unchanged)
├── scripts/                              (unchanged)
├── _bmad/                                (unchanged)
├── _bmad-output/                         (unchanged + this story file)
├── .claude/                              (unchanged)
├── crates/
│   ├── orgsidian-parser/                 (NEW: stub Cargo.toml + src/lib.rs doc comment)
│   ├── orgsidian-index/                  (NEW)
│   ├── orgsidian-watcher/                (NEW)
│   ├── orgsidian-vault/                  (NEW)
│   ├── orgsidian-plugin-api/             (NEW: LEAF — no project deps)
│   ├── orgsidian-report/                 (NEW)
│   ├── orgsidian-core/                   (NEW)
│   ├── orgsidian-cli/                    (NEW: bin crate `orgsidian`)
│   └── orgsidian-shell-app/              (MOVED from src-tauri/; bin crate)
│       ├── Cargo.toml                    (MODIFIED: workspace inherit; lib name renamed)
│       ├── build.rs
│       ├── tauri.conf.json               (MODIFIED: paths re-pointed at shell-ui/)
│       ├── capabilities/
│       │   └── default.json
│       ├── icons/                        (all 10 PNG + .icns + .ico — unchanged)
│       └── src/
│           ├── lib.rs                    (preserved, except `pub fn run` callers update)
│           └── main.rs                   (MODIFIED: calls orgsidian_shell_app_lib::run())
├── shell-ui/                             (NEW directory at repo root)
│   ├── package.json                      (NEW)
│   ├── tsconfig.json                     (MOVED from root)
│   ├── tsconfig.node.json                (MOVED from root)
│   ├── vite.config.ts                    (MOVED + MODIFIED: server.watch.ignored)
│   ├── index.html                        (MOVED from root)
│   ├── public/
│   │   ├── tauri.svg                     (MOVED from root public/)
│   │   └── vite.svg                      (MOVED from root public/)
│   └── src/
│       ├── App.tsx                       (MOVED from root src/)
│       ├── App.css                       (MOVED from root src/)
│       ├── main.tsx                      (MOVED from root src/)
│       ├── vite-env.d.ts                 (MOVED from root src/)
│       └── assets/
│           └── react.svg                 (MOVED from root src/assets/)
└── tools/
    └── corpus-extractor/                 (NEW: standalone, NOT in [workspace.members])
        ├── Cargo.toml
        └── src/main.rs
```

NO files at top-level: `src/`, `src-tauri/`, `index.html`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `public/`, `dist/` (the last is build output; ignored). Everything moved into `shell-ui/` (frontend) and `crates/orgsidian-shell-app/` (Rust shell).

### Architecture compliance — what THIS story must satisfy

- **LD-5 (Monorepo, round-4 amended):** 9 crates, `shell-ui/` at repo root (no `packages/`), `tools/corpus-extractor/` outside workspace. ([Source: architecture.md#Project Structure & Boundaries — Amendments to Earlier Sections](../planning-artifacts/architecture.md))
- **LD-10 / LD-26 (Plugin API LEAF invariant):** `orgsidian-plugin-api` Cargo.toml has zero project deps; `cargo deny check graph` (Story 1.7) will enforce this once configured. ([Source: architecture.md#LD-26](../planning-artifacts/architecture.md), [Source: epics.md#Story 1.5 AC](../planning-artifacts/epics.md))
- **LD-14 (Reports renderer location):** `orgsidian-report` is scaffolded as a separate leaf crate to absorb the future Typst PDF dependency cost in isolation from `core`. ([Source: architecture.md#LD-14 amendment](../planning-artifacts/architecture.md))
- **Locked Stack Versions:** Rust stable (via `rust-toolchain.toml`). No version bumps to JS deps — copy verbatim from current root `package.json`. ([Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md), [[feedback_version_policy]])
- **Crate organization rule:** `lib.rs` is re-exports only / no logic. Story 1.2 stubs are doc-comment-only `lib.rs` files (compliant by construction). ([Source: architecture.md#Project Structure → Crate organization](../planning-artifacts/architecture.md))
- **AI-Agent Implementation Rules:** no `unwrap()`/`expect()` in production code; no `any` in TS; no premature abstractions. Stub crates have no logic, so nothing to violate here — but the rule is the binding gate for every later story. ([Source: architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md))

### Anti-patterns explicitly forbidden in this story

- ❌ Pre-wiring `orgsidian-core` to depend on every leaf "because we'll need it anyway" — wire incrementally per the dep graph's "first use" rule.
- ❌ Pre-wiring `orgsidian-shell-app` to depend on `orgsidian-core` — that edge lands when the first IPC command needs `core` (Story 1.4 `tauri-specta` `ping` command will be the trigger).
- ❌ Adding ANY `[profile.release]` config — Story 1.8 territory (LD-38).
- ❌ Adding `[workspace.lints]` or `[workspace.metadata.<x>]` — Story 1.7 / 1.8 territory.
- ❌ Adding `clap`, `tracing`, `tokio`, or ANY external crate to a stub crate's `[dependencies]` block — incremental scope discipline.
- ❌ Writing ANY trait/struct/fn in a stub `lib.rs` beyond the single doc comment — premature abstraction violation.
- ❌ Creating `packages/shell-ui/` instead of `shell-ui/` — direct LD-5 round-4 amendment violation.
- ❌ Leaving `src-tauri/Cargo.lock` after the move — Cargo workspace policy violation.
- ❌ Leaving the top-level `src/`, `src-tauri/`, `index.html`, `vite.config.ts`, `tsconfig*.json`, `public/` in place — partial refactor.
- ❌ Renaming `productName`, `identifier`, or `version` in `tauri.conf.json` — those were locked in Story 1.1 AC4. The refactor preserves them.
- ❌ Adding `tools/issues-sync/` — that crate lands with Story 1.16 (LD-55 GitHub Issues sync).

### Testing requirements

Story 1.2 is a structural-refactor story; **no automated tests are added**. The binding gates are:

1. `cargo check --workspace` exits 0.
2. `cargo build --workspace` exits 0 and emits a root `Cargo.lock`.
3. `pnpm install` from repo root exits 0 with no errors.
4. `pnpm tauri dev` opens a window with the default React UI; `invoke('greet', { name })` round-trips.
5. `pnpm tauri build` emits `.app` + `.dmg` under repo-root `target/release/bundle/`.
6. `cargo build` from inside `tools/corpus-extractor/` exits 0 (independent of the workspace).
7. The husky `commit-msg` hook still rejects malformed messages.

Anchor smoke tests (parser / vault / watcher round-trip) land in Story 1.9. CI matrix (`pr.yml`, `nightly.yml`) lands in Story 1.8.

System-level test strategy reference (for downstream context, not enforcement here): [`_bmad-output/test-artifacts/test-design.md`](../test-artifacts/test-design.md).

### Project Structure Notes

- Alignment with the unified project structure (LD-5 round-4 amendment / §Project Structure & Boundaries → Workspace Layout) is the **primary deliverable** of this story. After completion, the project tree exactly matches the architecture's Workspace Layout (modulo the empty stubs).
- Detected conflict: the architecture file contains two layout descriptions — the older "Cargo Workspace Layout — 8 Crates from Day 1" + "Frontend Package Layout (`packages/shell-ui/`)" sections, AND the newer "Project Structure & Boundaries → Workspace Layout" section (which says 9 crates + `shell-ui/` at root). The newer section is a **round-4 amendment** that *explicitly* supersedes the older text (architecture.md preamble: "Where the text below conflicts with earlier sections, this section supersedes"). Follow the 9-crate + `shell-ui/`-at-root layout.
- Cross-references in the architecture file that still say `packages/shell-ui/` (e.g., the "Frontend Package Layout (`packages/shell-ui/`)" subsection title, the "Component organization (`packages/shell-ui/src/`)" rule, the FR→component mapping rows, the LD-29 file-path examples) are pre-amendment artifacts. They will be reconciled in a future doc-pass; for Story 1.2 purposes, treat them as `shell-ui/` (drop the `packages/` prefix).
- `corpus-extractor` crate **name**: architecture refers to the directory as `tools/corpus-extractor/`. Cargo package name in `tools/corpus-extractor/Cargo.toml` should be `orgsidian-corpus-extractor` (consistent with the `orgsidian-` project-wide prefix). If a future Story 2.5 author prefers the bare `corpus-extractor` name, that's a rename decision they own; Story 1.2 picks the prefix for consistency.

### References

- [Source: epics.md#Story 1.2 — full AC text](../planning-artifacts/epics.md) — 6 ACs that this story file expands into 14 detailed ACs.
- [Source: epics.md Epic 1 framing — line 146](../planning-artifacts/epics.md) — "9-crate Cargo workspace from day 1 ... frontend at `shell-ui/` at repo root (no `packages/` indirection until a 2nd JS package exists). `tools/corpus-extractor/` outside the workspace."
- [Source: epics.md#Story 1.5](../planning-artifacts/epics.md) — establishes the full `orgsidian-plugin-api` trait surface; informs the LEAF-crate stub shape in Story 1.2.
- [Source: epics.md#Story 1.7](../planning-artifacts/epics.md) — `cargo-deny` graph rule that will enforce the LEAF invariant; informs why Story 1.2 must not add ANY deps to `orgsidian-plugin-api`.
- [Source: epics.md#Story 1.8](../planning-artifacts/epics.md) — explains why `[profile.release] panic = "unwind"` and the `invoke_plugin_hook!` macro are NOT in this story.
- [Source: epics.md#Story 1.13](../planning-artifacts/epics.md) — explains why no `.github/` workflows in this story (GitHub bootstrap lands in 1.13).
- [Source: architecture.md#Project Structure & Boundaries — Amendments to Earlier Sections](../planning-artifacts/architecture.md) — LD-5 round-4 amendment (9 crates, shell-ui at root); LD-26 round-4 amendment (HookContext/PluginContext as traits).
- [Source: architecture.md#Workspace Layout — line 896-1002](../planning-artifacts/architecture.md) — canonical post-Story-1.2 file tree.
- [Source: architecture.md#Crate Dependency Graph — line 1013-1028](../planning-artifacts/architecture.md) — full edge diagram (consumers → core → leaves); informs the incremental-wiring discipline.
- [Source: architecture.md#Project Structure → Crate organization](../planning-artifacts/architecture.md) — "`lib.rs` — public surface re-exports only; no logic" rule applied here.
- [Source: architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md) — binding for every Rust file added.
- [Source: architecture.md#Locked Stack Versions](../planning-artifacts/architecture.md) — Rust stable; informs `rust-toolchain.toml`.
- [Source: architecture.md#LD-14 amendment](../planning-artifacts/architecture.md) — why `orgsidian-report` is the new 9th crate.
- [Source: 1-1-bootstrap-tauri-2-x-react-19-ts-scaffold.md](./1-1-bootstrap-tauri-2-x-react-19-ts-scaffold.md) — Story 1.1 output that this story refactors; binding context for AC2/AC3 (Tauri config field values to preserve), AC5 (root package.json fields to preserve), AC12 (husky/commitlint/LICENSE preservation).
- Previous-story memory: [[feedback_version_policy]] (Orgsidian deps pinned to latest stable or LTS; Tauri ecosystem exempted with conservative bumping per milestone — informs the "no opportunistic upgrades in this story" discipline in Task 5.1).
- Previous-story memory: [[feedback_batch_fixes_terse]] (Fast-path BMad applies no-brainer fixes silently; surface only decision-grade questions — informs the dev agent's review-cycle posture for the inevitable follow-up code review of this refactor).
- External: [`https://doc.rust-lang.org/cargo/reference/workspaces.html`](https://doc.rust-lang.org/cargo/reference/workspaces.html) — Cargo workspace docs (`[workspace]`, `[workspace.package]`, `[workspace.dependencies]`, `exclude`, `resolver = "2"`).
- External: [`https://pnpm.io/pnpm-workspace_yaml`](https://pnpm.io/pnpm-workspace_yaml) — pnpm-workspace.yaml schema (`packages`, `allowBuilds`). Verify via `ctx7` if syntax is uncertain — pnpm 11.x is the locked version.
- External: [`https://v2.tauri.app/start/`](https://v2.tauri.app/start/) — Tauri 2 config schema (`build.frontendDist`, `build.beforeDevCommand`, etc.).

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (`claude-opus-4-7`) via Claude Code

### Debug Log References

- `cargo check --workspace` — exit 0; all 9 crates + transitive deps check cleanly (final lines visible in dev log).
- `cargo build --workspace` — exit 0; root `Cargo.lock` generated; `target/` populated at workspace root (not under `crates/orgsidian-shell-app/`).
- `cd tools/corpus-extractor && cargo build` — exit 0; standalone crate compiles independently; emits its own `Cargo.lock` + `target/` (both gitignored per AC9 + Task 9 amendment).
- `pnpm install` — exit 0; `shell-ui` workspace member resolved with React 19.2.6, react-dom 19.2.6, Vite 7.3.3, TS 5.8.3; `@tauri-apps/cli` resolved at root.
- `pnpm tauri build` — `.app` at `target/release/bundle/macos/orgsidian.app`, `.dmg` at `target/release/bundle/dmg/orgsidian_0.0.0_aarch64.dmg`. First run's `bundle_dmg.sh` failed transiently (hdiutil contention with the prior failed `frontendDist`-resolution attempt; harmless leftover `rw.*.dmg` in `bundle/macos/` left in `target/` which is gitignored); rerun succeeded.
- `pnpm tauri dev` — Vite ready on `http://localhost:1420/` in 88 ms; `orgsidian-shell-app` compiled in 4.35 s; binary launched (`target/debug/orgsidian-shell-app`). Visual confirmation of the window + `invoke('greet')` round-trip requires human eyes — flagged below in Completion Notes.

### Completion Notes List

- **AC3 frontendDist correction (silent fix).** The story spec stated `frontendDist = "../../../shell-ui/dist"` (three `..` segments), but from `crates/orgsidian-shell-app/tauri.conf.json` only two segments are required to reach the repo root (`../` → `crates/`, `../../` → repo root). The first `pnpm tauri build` run surfaced the error: `Unable to find your web assets … (which is …/crates/orgsidian-shell-app/../../../shell-ui/dist)` — that path escapes the repo root entirely. Implemented value: `"../../shell-ui/dist"`. Verified via `readlink -f` resolving to `/Users/tizianobasile/workspace/me/orgsidian/shell-ui/dist`. AC10/AC11 both pass with the corrected value.
- **`vite.config.ts` `@ts-expect-error` retained.** Per Dev Notes §10, the directive on `process.env.TAURI_DEV_HOST` is intentionally preserved (Story 1.3 owns its replacement). VSCode LSP now reports the directive as "unused" in the new location — this is the LSP's own resolution, not a TS compiler error; `pnpm --filter shell-ui build` still passes.
- **Visual UI gate left to human verification.** AC10 requires confirmation that the Tauri window actually opens and the `greet` command round-trips. Programmatic gates verified: Vite served on :1420, `orgsidian-shell-app` dev binary spawned without error (last log line: `Running '/Users/tizianobasile/workspace/me/orgsidian/target/debug/orgsidian-shell-app'`). The dev server was stopped before the window could be visually inspected from within this automated session — please run `pnpm tauri dev` from repo root and click "Greet" to close the loop.
- **Husky `commit-msg` hook intact.** `pnpm install` re-ran the `prepare` script (`husky` bootstrap); `.husky/commit-msg` present and unchanged. Code review 2026-05-21 verified the reject path non-destructively via `echo "this is not conventional" | npx --no-install commitlint` (exit 1, `subject-empty` + `type-empty`) — no throwaway commits created.
- **Build-output hygiene.** Added `tools/corpus-extractor/target/` to root `.gitignore` (the standalone crate emits its own `target/` outside the workspace's root `target/`).
- **No cross-crate `[dependencies]` wiring added.** Per Dev Notes §1 and AC7, the workspace dep graph remains structural-only — `orgsidian-core` does not depend on the leaves; `orgsidian-shell-app` does not depend on `orgsidian-core`. First-use stories own the edges.
- **No `[profile.release]`, no `[workspace.lints]`, no plugin set.** AC14 anti-creep discipline observed throughout.
- **`tools/corpus-extractor/Cargo.lock` is committed** (7 lines, generated locally on first `cargo build` inside `tools/corpus-extractor/`). Standard Cargo posture for a standalone binary crate outside any workspace — gives the standalone tool reproducible builds. (Initial draft of this note said "not committed"; corrected at code review 2026-05-21 to match the actual committed state.)

### File List

**New (root + workspace):**

- `Cargo.toml` — workspace root manifest (9 members, `exclude = ["tools/corpus-extractor"]`, `[workspace.package]`, `[workspace.dependencies]`).
- `Cargo.lock` — workspace lockfile (generated by first `cargo build --workspace`).
- `rust-toolchain.toml` — pins `stable` + `rustfmt` + `clippy`, `profile = "minimal"`.

**New crates (stub leaves + bin):**

- `crates/orgsidian-parser/Cargo.toml`, `crates/orgsidian-parser/src/lib.rs`
- `crates/orgsidian-index/Cargo.toml`, `crates/orgsidian-index/src/lib.rs`
- `crates/orgsidian-watcher/Cargo.toml`, `crates/orgsidian-watcher/src/lib.rs`
- `crates/orgsidian-vault/Cargo.toml`, `crates/orgsidian-vault/src/lib.rs`
- `crates/orgsidian-plugin-api/Cargo.toml`, `crates/orgsidian-plugin-api/src/lib.rs` (LEAF, no project deps)
- `crates/orgsidian-report/Cargo.toml`, `crates/orgsidian-report/src/lib.rs`
- `crates/orgsidian-core/Cargo.toml`, `crates/orgsidian-core/src/lib.rs`
- `crates/orgsidian-cli/Cargo.toml`, `crates/orgsidian-cli/src/main.rs` (`[[bin]] name = "orgsidian"`)

**Moved (git mv preserved blame): `src-tauri/` → `crates/orgsidian-shell-app/`:**

- `crates/orgsidian-shell-app/Cargo.toml` (rewritten: `name = "orgsidian-shell-app"`, workspace inheritance, `[lib].name = "orgsidian_shell_app_lib"`)
- `crates/orgsidian-shell-app/build.rs` (unchanged)
- `crates/orgsidian-shell-app/src/main.rs` (one-line edit: `orgsidian_lib::run()` → `orgsidian_shell_app_lib::run()`)
- `crates/orgsidian-shell-app/src/lib.rs` (unchanged)
- `crates/orgsidian-shell-app/tauri.conf.json` (`beforeDevCommand`, `beforeBuildCommand`, `frontendDist` updated)
- `crates/orgsidian-shell-app/capabilities/default.json` (unchanged; `$schema` path already correct relative to capabilities/)
- `crates/orgsidian-shell-app/icons/*` (all 17 icon files unchanged)
- `crates/orgsidian-shell-app/.gitignore` (unchanged)

**Moved: root frontend → `shell-ui/`:**

- `shell-ui/src/{App.tsx, App.css, main.tsx, vite-env.d.ts, assets/react.svg}`
- `shell-ui/public/{tauri.svg, vite.svg}`
- `shell-ui/index.html`
- `shell-ui/vite.config.ts` (edited: `server.watch.ignored` updated to ignore `crates/orgsidian-shell-app/**`, `target/**`, `_bmad-output/**`, `_bmad/**`)
- `shell-ui/tsconfig.json` (unchanged)
- `shell-ui/tsconfig.node.json` (unchanged)

**New: `shell-ui/`:**

- `shell-ui/package.json` (workspace member, owns React/Vite/TS/Tauri-api deps).

**New: standalone tool (NOT in workspace):**

- `tools/corpus-extractor/Cargo.toml` (`publish = false`, no workspace inheritance)
- `tools/corpus-extractor/src/main.rs` (stub)

**Modified at root:**

- `package.json` (reduced to orchestration role: `prepare`, `commitlint`, `dev`/`build` aliases, `tauri`; devDeps trimmed to commitlint+husky+@tauri-apps/cli; description updated)
- `pnpm-workspace.yaml` (now declares `packages: [shell-ui]` + retains `allowBuilds: { esbuild: true }`; preamble rewritten)
- `pnpm-lock.yaml` (regenerated to reflect the workspace split)
- `.gitignore` (replaced `src-tauri/target/` + `src-tauri/gen/schemas/` rules with workspace-root `target/`, `crates/orgsidian-shell-app/target/`, `crates/orgsidian-shell-app/gen/schemas/`, `shell-ui/dist/`, `tools/corpus-extractor/target/`)

**Deleted:**

- `src-tauri/Cargo.lock` (Cargo workspace policy: single lockfile at workspace root).
- Root `dist/` directory (Story 1.1 leftover; Vite output now lives at `shell-ui/dist/`).

**Untouched (verified per AC12):**

- `.husky/`, `.husky/commit-msg`, `commitlint.config.cjs`, `scripts/sync-epics-to-github.sh`, `docs/logo-draft.png`, `_bmad/`, `_bmad-output/` (except this story file), `.claude/`, root `README.md`, root `LICENSE`.

### Change Log

| Date       | Story Phase | Change                                                                                                                                                                 |
| ---------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-05-21 | Implementation | Cargo workspace established (9 crates + standalone `tools/corpus-extractor/`); root JS app relocated to `shell-ui/` per LD-5 round-4; `src-tauri/` → `crates/orgsidian-shell-app/`. |
| 2026-05-21 | Implementation | Fixed AC3 `frontendDist` path from `../../../shell-ui/dist` to `../../shell-ui/dist` (story spec miscounted `..` levels; verified via `readlink -f`).                  |
| 2026-05-21 | Code review | Fixed FR citations in 4 stub crate descriptions (`orgsidian-index` FR-7,8,9 → FR-12,17; `orgsidian-vault` FR-3,4,5 → FR-15+NFR-14,15; `orgsidian-watcher` FR-10 → FR-16+NFR-16; `orgsidian-report` FR-13 → FR-14) — original citations didn't match the FR Coverage Map in `epics.md`. |
| 2026-05-21 | Code review | Workspace `[workspace.package].authors` updated from `["Tiziano Basile and Orgsidian contributors"]` (non-RFC822, single string) to `["Tiziano Basile <tiz.basile@gmail.com>", "Orgsidian contributors"]` (RFC822-compliant dual entry; uses personal email, not the work address that appears in pre-Story-1.2 metadata). |
| 2026-05-21 | Code review | Verified `pnpm.allowBuilds` is the correct pnpm 11+ key (replaces deprecated `onlyBuiltDependencies` per pnpm docs); verified Tauri CLI auto-discovers `crates/orgsidian-shell-app/tauri.conf.json` from repo root via `pnpm tauri info`; verified commitlint reject path. |
