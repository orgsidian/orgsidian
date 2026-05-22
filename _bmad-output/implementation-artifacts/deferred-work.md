# Deferred Work

## Deferred from: code review of story-1.1 (2026-05-19)

- **CSP disabled in `src-tauri/tauri.conf.json:22`** [MED] — `"csp": null` ships the app with no Content-Security-Policy; scaffold default. Harden before v0.1 Alpha (broad XSS surface once org-mode content rendering lands in Story 3+).
- **`vite.config.ts:5` `@ts-expect-error` will itself fail compilation if `@types/node` is later added** [MED] — directive becomes "unused" once the error it expects no longer occurs. Revisit when adding @types/node (Story 1.3 frontend wiring may force this).
- **`tsconfig.node.json:3` `composite: true` without `declaration`/`outDir`** [LOW] — `tsc -b` would write `.d.ts` next to `vite.config.ts`. Scaffold-emitted; revisit in Story 1.3 build-config retune.
- **`tsconfig.json` `references` block is dead under `tsc` (only honored by `tsc -b`)** [LOW] — `package.json:34` runs bare `tsc`, so `vite.config.ts` type-checking does NOT happen at build time. Story 1.3.
- **`src/App.tsx:11` `invoke('greet')` unhandled promise rejection** [LOW] — demo UI; Story 1.3 replaces.
- **`src/main.tsx:5` `document.getElementById("root") as HTMLElement` cast hides null** [LOW] — scaffold; Story 1.3.
- **`src/App.tsx:18-26,30` `<a target="_blank">` without `rel="noopener noreferrer"`** [LOW] — demo UI; Story 1.3.
- **`src/App.tsx:39-43` `<input>` half-controlled (no `value` binding)** [LOW] — demo UI; Story 1.3.
- **`src/App.tsx:39-43` `<input>` missing `<label>`/`aria-label`** [LOW] — demo UI accessibility; Story 1.3.
- **`.gitignore` ignores `src-tauri/gen/schemas/` but `src-tauri/capabilities/default.json:2` `$schema` references that path** [LOW] — editor JSON-schema warnings on fresh checkout until first `tauri dev`/`build`. Cosmetic; expected behavior.
- **`.gitignore` only ignores `src-tauri/gen/schemas/`, not future `gen/android/` or `gen/apple/`** [LOW] — mobile targets out of scope until post-v1.0; flag for when mobile is added.

## Deferred from: code review of story-1.2 (2026-05-21)

- **`crates/orgsidian-shell-app/Cargo.toml` shown as new file rather than rename** [LOW] — Content diverged enough during refactor that git rename detection fell below similarity threshold, breaking the blame trail on this single file. Other Tauri files (build.rs, icons, tauri.conf.json, src/lib.rs, src/main.rs) preserved rename history correctly. Cannot retroactively fix without rewriting history.

## Deferred from: code review of story-1.3 (2026-05-21)

- **`shell-ui/src/routes/__root.tsx` does NOT mount `<TooltipProvider>`** [MED] — The first story that mounts a `<Tooltip>` consumer must add `<TooltipProvider>` either at `__root.tsx` (preferred — global) or scoped to the consumer. Without it, Radix Tooltip throws "`Tooltip` must be used within `TooltipProvider`" on first use. Owner: first story to render a tooltip.
- **`shell-ui/src/components/ui/sonner.tsx:14` calls `useTheme()` from `next-themes` outside a `<ThemeProvider>`** [LOW] — degrades to `theme: undefined` → destructure default `"system"` until ThemeProvider wraps the tree. No real toasts are triggered in Story 1.3 so the effect is latent. Owner: Story 6.7 (themes) — wire `<ThemeProvider>` at app root.
- **`shell-ui/src/routes/_layout/today.tsx` has no parent `_layout.tsx` — pathless group children parent to root; adding `_layout.tsx` later retroactively reparents every existing child** [MED] — Story 1.3 spec explicitly designs this as intentional (line 112). Owner: Story 7.1 (Today Dashboard) — when introducing the real layout file, audit every existing child of `routes/_layout/` for layout-wrapping assumptions.

## Deferred from: code review of story-1.4 (2026-05-21)

- **`prebuild` couples frontend build to full Rust toolchain + Tauri compile** [`shell-ui/package.json:8`] [MED] — chosen Option A.3 per spec §Generation timing; full backend compile runs on every `pnpm --filter shell-ui build`. Slow on cold CI caches and requires Rust on TS-only contributor machines. Owner: revisit if CI build time becomes a constraint or if TS-only contributor path becomes a frequent ask.
- **AC9 round-trip validated manually only (no automated end-to-end gate)** [LOW] — deferred to Story 1.7/1.8 (CI scaffolding). Risk: a future regression that breaks `commands.ping()` round-trip will not be caught until manual verification of the next story. Owner: Story 1.7 / 1.8.
- **`Builder::<tauri::Wry>::new()` hardcodes desktop runtime** [`crates/orgsidian-shell-app/src/lib.rs:23`] [LOW] — mobile-incompat; v0.1 is desktop-only. Owner: post-v1.0 mobile story.
- **`thiserror = "1"` floats while transitive `thiserror 2.0.18` is pulled in by tauri-specta** [`Cargo.toml`, `Cargo.lock:160`] [LOW] — two major versions compile (bloat). Owner: Story 1.7 (cargo-deny / cargo-audit).
- **Unmaintained `Inflector` 0.11.4 transitive (last released 2018)** [`Cargo.lock:20-24`] [LOW] — pulled in via `specta-macros`. cargo-audit will flag. Owner: Story 1.7.
- **`pnpm tauri dev` does not run `prebuild` → stale `tauri.ts` race window when commands change** [LOW] — HMR + binary boot reconcile within seconds. Owner: workflow-ergonomics polish story.
- **Concurrent `pnpm tauri dev` + `cargo test --test export_bindings` can torn-write `tauri.ts`** [LOW] — theoretical race; tauri-specta upstream does not do atomic rename. Owner: monitor upstream tauri-specta or workaround via a lockfile in shell-ui prebuild.
- **`setup` closure borrows `specta_builder` after `invoke_handler()` call; pattern not test-asserted** [`crates/orgsidian-shell-app/src/lib.rs:62-67`] [LOW] — depends on tauri-specta API stability across upgrades. Owner: Story 1.7/1.8 (add a smoke test that exercises `commands.ping()` via Tauri's webview in CI).
- **`rm tauri.ts && pnpm tsc` (bare typecheck) reports cryptic missing-module from `today.tsx:3`** [LOW] — documented contributor flow is `pnpm --filter shell-ui build` (triggers `prebuild`). Owner: tech-writer docs or a `pnpm typecheck` script that depends on `prebuild`.
- **`#[serde(tag = "kind")]` on `OrgError` may not round-trip if `Deserialize` is derived later** [`crates/orgsidian-core/src/error.rs:15-17`] [LOW] — `OrgError` is `Serialize`-only by spec. Owner: revisit only if plugin-host deserialization path appears.
- **AC1 spec text omits `features = ["derive"]` requirement on `specta` (disclosed deviation #5)** [LOW] — Completion Notes disclose; epic update could amend spec wording. Owner: next epic-update touching Epic 1 AC text.
- **`tests/export_bindings.rs` does not assert on generated content (only that `export()` does not panic)** [`crates/orgsidian-shell-app/tests/export_bindings.rs`] [MED] — a regression dropping `OrgError` from the bindings or changing the `kind` discriminator would pass green. Owner: Story 1.7/1.8 — promote to a snapshot/golden-file test.
