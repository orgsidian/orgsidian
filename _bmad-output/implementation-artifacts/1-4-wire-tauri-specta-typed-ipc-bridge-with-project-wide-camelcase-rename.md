# Story 1.4: Wire `tauri-specta` typed IPC bridge with project-wide camelCase rename

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want `tauri-specta` (v2.x) generating a fully-typed TypeScript client at `shell-ui/src/lib/tauri.ts` with project-wide `camelCase` rename configured once in the builder + an `OrgError` enum declared in `orgsidian-core` and exposed across the IPC boundary,
So that no story from 1.4 onward writes `invoke('command_name', …)` with raw strings, no struct ever needs a per-type `#[serde(rename_all)]`, and every backend → frontend error round-trips as a typed discriminated union.

## Acceptance Criteria

**AC1 — Workspace deps pinned in one place.**
`Cargo.toml` (workspace root) declares `tauri-specta = "=2.0.0-rc.25"`, `specta = "=2.0.0-rc.25"`, `specta-typescript = "=0.0.12"`, and `thiserror = "1"` in `[workspace.dependencies]`. `tauri-specta` is loaded with `features = ["derive", "typescript"]`. The exact pins reflect the tauri-ecosystem exemption in [[feedback_version_policy]] (specta is RC-only; conservative bumping per milestone).

**AC2 — `OrgError` is the single project-wide IPC error type.**
`crates/orgsidian-core/src/error.rs` declares:

```rust
pub type Result<T> = std::result::Result<T, OrgError>;

#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub enum OrgError {
    #[error("parse error in {file}: {reason}")]
    Parse { file: String, reason: String },
    #[error("io error: {reason}")]
    Io { reason: String },
    #[error("index error: {reason}")]
    Index { reason: String },
    #[error("vault error: {reason}")]
    Vault { reason: String },
}
```

- `OrgError` is `pub use`-re-exported from `crates/orgsidian-core/src/lib.rs` as `pub use error::{OrgError, Result};`.
- `crates/orgsidian-core/Cargo.toml` adds `thiserror`, `serde`, `specta` (workspace deps).
- The four `String` payload fields are wrapped in struct variants so each error carries diagnostic detail — matching architecture LD-NN error format (`OrgError::Parse { file, reason }`, etc.).

**AC3 — `orgsidian-shell-app` depends on `orgsidian-core` and uses `OrgError`.**
- `crates/orgsidian-shell-app/Cargo.toml` adds `orgsidian-core = { path = "../orgsidian-core" }` to `[dependencies]` and `tauri-specta`/`specta`/`specta-typescript` (workspace).
- `orgsidian-core` is registered in `[workspace.dependencies]` of root `Cargo.toml` as `orgsidian-core = { path = "crates/orgsidian-core" }` so future consumers (`orgsidian-cli` in Story 1.x, plugins later) opt in via `orgsidian-core.workspace = true`.

**AC4 — Single placeholder command `ping()` returns `Result<String, OrgError>`.**
`crates/orgsidian-shell-app/src/lib.rs`:

```rust
use orgsidian_core::{OrgError, Result as OrgResult};
use tauri_specta::{collect_commands, Builder};
use specta_typescript::Typescript;

#[tauri::command]
#[specta::specta]
fn ping() -> OrgResult<String> {
    Ok("pong".to_string())
}
```

- The legacy `greet(name: &str) -> String` command from Story 1.1 is **deleted** (it was the placeholder Story 1.3 marked with `// Story 1.4 replaces this`).
- Both attributes (`#[tauri::command]` AND `#[specta::specta]`) are mandatory on every command from this story onward (anti-pattern: omitting `#[specta::specta]` silently excludes the command from the generated TS client).

**AC5 — `tauri-specta` Builder is the single composition root for IPC.**
`crates/orgsidian-shell-app/src/lib.rs::run()` constructs a `tauri_specta::Builder` once, registers commands via `collect_commands![ping]`, exports the TS client in debug builds, and hands `invoke_handler()` + `mount_events()` to the Tauri Builder:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let mut specta_builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![ping]);

    // Debug builds re-export the typed TS client on every app start (specta
    // exporter applies project-wide camelCase to command/arg/field names via
    // the `#[specta(rename_all = ...)]` attribute on each Type). Release
    // builds skip the write — bindings are checked-in for release reproducibility.
    #[cfg(debug_assertions)]
    specta_builder
        .export(Typescript::default(), "../../shell-ui/src/lib/tauri.ts")
        .expect("tauri-specta TS client export failed");

    let tauri_builder = tauri::Builder::default()
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

    // Story 13.2 activates tauri-plugin-updater here behind #[cfg(desktop)]
    // once signing material is generated (see Story 1.3 Change Log AC1 deviation).

    tauri_builder
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
}
```

- The Tauri-side `.invoke_handler(tauri::generate_handler![greet])` chain from Story 1.3 is **replaced** with `.invoke_handler(specta_builder.invoke_handler())`. No `generate_handler!` macro call remains in the codebase.

**AC6 — Generated TS client lands at the canonical path and is git-ignored.**
- The file `shell-ui/src/lib/tauri.ts` is **generated**, not hand-written. It is git-ignored via a new entry in root `.gitignore`: `shell-ui/src/lib/tauri.ts`.
- A pre-build hook in `shell-ui/package.json` (extending the existing `prebuild` for `tsr generate`) regenerates the file before frontend TS builds so fresh clones / CI runs can `pnpm --filter shell-ui build` without first having run `pnpm tauri dev`. Implementation: extend `prebuild` to `"prebuild": "tsr generate && cargo run --manifest-path ../crates/orgsidian-shell-app/Cargo.toml --bin orgsidian-shell-app --quiet --features specta-export -- --export-bindings-only || true"` — **OR** (simpler, recommended) prefer the alternative described in Dev Notes §Generation timing: ship a `cargo xtask export-bindings` binary or a `build.rs` invocation. See Dev Notes for the chosen mechanism + rationale.

**AC7 — Frontend consumes the typed client; raw `invoke()` is removed.**
- `shell-ui/src/routes/_layout/today.tsx` replaces `invoke('greet', { name })` with `await commands.ping()` via `import { commands } from '@/lib/tauri'`. The form input + state-around-`greet` UI is replaced by a single button "Ping" that displays the returned `"pong"` string, since `ping()` takes no args (per epic AC).
- The placeholder route still exercises the 4 pillars from Story 1.3 (TanStack route + shadcn `<Button>` + Tailwind utilities + Tauri IPC), now via the typed bridge.
- The `// Story 1.4 replaces this with the typed specta client` comment is removed (no longer accurate).

**AC8 — Project-wide camelCase casing achieved (architecture intent).**
The architecture's "**project-wide specta camelCase rename configured once in the builder**" goal is realized as follows (see Dev Notes §Casing for the literal-vs-current-API reconciliation):
- Command names: auto-converted by `specta-typescript`'s renderer (`to_lower_camel_case()` is applied unconditionally). `ping` → `commands.ping`; future `open_file` → `commands.openFile` (no opt-in needed).
- Command argument names: same auto-conversion. `file_path: String` → TS `{ filePath: string }`.
- Struct/enum-field names: `#[specta(rename_all = "camelCase")]` on the type itself. `OrgError` carries this attribute (mirrored with `#[serde(tag = "kind", rename_all = "camelCase")]` so the JSON wire matches what specta exports — verified via the `specta-serde::Format` symmetry rule).
- The literal epic-AC API (`specta::ts::ExportConfig::default().rename_all(specta::ts::RenameAll::CamelCase)`) is **specta 1.x** and does not exist in `=2.0.0-rc.25`; see Dev Notes §Casing.

**AC9 — Round-trip verification: `commands.ping()` returns `"pong"` end-to-end.**
- `pnpm tauri dev` launches; the `/today` placeholder route renders; clicking "Ping" displays `"pong"` (verified manually before marking done).
- The generated `shell-ui/src/lib/tauri.ts` exports `commands.ping(): Promise<string>` (Promise typed as `string`, not `string | Error`; errors throw — matching architecture's "tauri-specta generates throwing async functions" rule).
- The generated TS contains the typed `OrgError` discriminated union: `export type OrgError = { kind: "parse"; file: string; reason: string } | { kind: "io"; reason: string } | { kind: "index"; reason: string } | { kind: "vault"; reason: string };` (exact shape may vary by tauri-specta version; the discriminator key is `kind`).

**AC10 — Anti-creep: nothing else in this story.**
- ❌ No additional commands beyond `ping` — file/index/vault commands are owned by Stories 3.6, 4.1, 6.2, etc.
- ❌ No `app.emit()` event scaffolding — first event lands in Story 5.1 (filesystem watcher).
- ❌ No `OrgsidianError` TS-side wrapper class — architecture mentions it as a future thin wrapper; ship raw specta error union for now.
- ❌ No Quick Capture secondary-window specta wiring — owned by Story 8.1.
- ❌ No `chrono::DateTime` semantic-type config — first date-typed command is in Story 2.x (parser); revisit then.
- ❌ No CI gate enforcing "no raw `invoke()`" — covered structurally (no `@tauri-apps/api/core` import remains in `shell-ui/src/`); a hard ESLint rule is a Story 1.17 / 1.8 hardening item, not 1.4.

## Tasks / Subtasks

- [x] **Task 1: Add workspace dep declarations** (AC1)
  - [x] 1.1 Edit root `Cargo.toml` `[workspace.dependencies]`: add `tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }`, `specta = "=2.0.0-rc.25"`, `specta-typescript = "=0.0.12"`, `thiserror = "1"`.
  - [x] 1.2 Add `orgsidian-core = { path = "crates/orgsidian-core" }` to `[workspace.dependencies]` so consumers opt in via `.workspace = true`.

- [x] **Task 2: Declare `OrgError` in `orgsidian-core`** (AC2)
  - [x] 2.1 Add `[dependencies]` to `crates/orgsidian-core/Cargo.toml`: `thiserror = { workspace = true }`, `serde = { workspace = true }`, `specta = { workspace = true }`.
  - [x] 2.2 Create `crates/orgsidian-core/src/error.rs` with the `OrgError` enum + `Result<T>` alias (see AC2 code block).
  - [x] 2.3 Add `mod error;` + `pub use error::{OrgError, Result};` to `crates/orgsidian-core/src/lib.rs`.
  - [x] 2.4 `cargo check -p orgsidian-core` → exit 0.

- [x] **Task 3: Wire `tauri-specta` deps + `orgsidian-core` into shell-app** (AC1, AC3)
  - [x] 3.1 Edit `crates/orgsidian-shell-app/Cargo.toml`: add `orgsidian-core = { workspace = true }`, `tauri-specta = { workspace = true }`, `specta = { workspace = true }`, `specta-typescript = { workspace = true }` under `[dependencies]`.
  - [x] 3.2 `cargo check -p orgsidian-shell-app` → exit 0.

- [x] **Task 4: Refactor `lib.rs::run()` to drive Tauri via the specta Builder** (AC4, AC5)
  - [x] 4.1 Replace the `greet(name: &str) -> String` command with `ping() -> OrgResult<String>` (per AC4 code block).
  - [x] 4.2 Rewrite `run()` per AC5 code block: construct `Builder::<tauri::Wry>::new().commands(collect_commands![ping])`, conditionally export TS in debug builds, hand `invoke_handler()` + `mount_events()` to the Tauri Builder.
  - [x] 4.3 Confirm the 11 plugin `.plugin(...)` registrations from Story 1.3 are preserved verbatim (no removal / reorder).
  - [x] 4.4 Confirm the Story 1.3 updater-deferred comment block is preserved.
  - [x] 4.5 `cargo build --workspace` → exit 0.

- [x] **Task 5: Git-ignore generated TS client** (AC6)
  - [x] 5.1 Add `shell-ui/src/lib/tauri.ts` to root `.gitignore` (near the existing `shell-ui/src/routeTree.gen.ts` line).
  - [x] 5.2 Confirm with `git check-ignore -v shell-ui/src/lib/tauri.ts` after a debug build that the path is matched by the new gitignore entry.

- [x] **Task 6: Choose + wire generation timing** (AC6)
  - [x] 6.1 Decide between the two options in Dev Notes §Generation timing. **A.3 chosen** (recommended default): file gitignored, `tests/export_bindings.rs` serves as the regen step + CI assertion, `prebuild` invokes `cargo test --package orgsidian-shell-app --test export_bindings --quiet`.
  - [x] 6.2 Implement the chosen path (A.3).
  - [x] 6.3 `rm shell-ui/src/lib/tauri.ts && pnpm --filter shell-ui build` → exit 0 (regeneration via `prebuild` works end-to-end on a fresh clone).

- [x] **Task 7: Replace raw `invoke('greet', ...)` in the `/today` placeholder route** (AC7)
  - [x] 7.1 Edit `shell-ui/src/routes/_layout/today.tsx`: remove `import { invoke } from "@tauri-apps/api/core"` + the `name` form + `greet()` handler.
  - [x] 7.2 Replace with `import { commands } from "@/lib/tauri"` + a single `<Button>Ping</Button>` whose click handler does `setReply(await commands.ping())`.
  - [x] 7.3 Remove the `// Story 1.4 replaces this with the typed specta client.` comment.
  - [x] 7.4 Confirm `shell-ui/` contains zero remaining `import { invoke } from "@tauri-apps/api/core"` lines: `rg "from ['\"]@tauri-apps/api/core['\"]" shell-ui/src/` → exit 1 (no matches).

- [x] **Task 8: End-to-end verification + binding-content audit** (AC9)
  - [x] 8.1 `pnpm tauri dev` → app boots; `/today` route renders; "Ping" button shows `"pong"` after click. **(Verified manually by Tiziano on 2026-05-21.)**
  - [x] 8.2 Open `shell-ui/src/lib/tauri.ts`; confirm `export const commands` block contains a `ping` entry typed as `() => Promise<string>` (no `Error | string` union — errors throw). Generated output: `ping: () => __TAURI_INVOKE<string>("ping")` — throwing-style via `Builder::error_handling(ErrorHandlingMode::Throw)`.
  - [x] 8.3 Confirm `shell-ui/src/lib/tauri.ts` contains the `OrgError` discriminated union with a `kind` discriminator and `parse | io | index | vault` lowercase tags. Verified: `export type OrgError = { kind: "parse"; file: string; reason: string } | { kind: "io"; reason: string } | { kind: "index"; reason: string } | { kind: "vault"; reason: string };`.
  - [x] 8.4 `pnpm --filter shell-ui build` → exit 0 (TS strict catches `commands.ping()` signature usage).
  - [x] 8.5 `cargo build --workspace` → exit 0.
  - [x] 8.6 `pnpm tauri build` → emits `orgsidian.app` + `orgsidian_0.0.0_aarch64.dmg` at `target/release/bundle/{macos,dmg}/`.

- [x] **Task 9: Anti-creep audit** (AC10)
  - [x] 9.1 Confirm `collect_commands![…]` lists exactly `ping` (no other entries).
  - [x] 9.2 Confirm no `collect_events![…]` block exists yet — events arrive in Story 5.1.
  - [x] 9.3 Confirm no `Builder::<tauri::Wry>::new().semantic_types(...)` opt-in — semantic types (chrono/bytes/url) are deferred to the first command that needs them.
  - [x] 9.4 Confirm `shell-ui/src/` has no `OrgsidianError` wrapper class — raw specta error union is the v0.1 surface.

## Dev Notes

### Critical context the dev agent MUST internalize before touching code

This story is the **last** story in which `invoke('command_name', …)` exists anywhere in `shell-ui/src/`. Every story from 1.5 onward consumes `commands.<commandName>()` from the generated `@/lib/tauri` module. From this story forward:

- The architecture's anti-pattern "❌ `invoke('command_name', …)` with raw strings" is **structurally enforced**: the only path to Rust is through the specta client.
- The architecture's anti-pattern "❌ `#[serde(rename_all = "camelCase")]` on individual structs" requires nuance — see §Casing below; the literal goal of "single source of casing" is delivered, but `OrgError` does carry a serde `rename_all` attribute (matched with the specta one) because **the wire format must match what specta exports**, and the cleanest way to achieve symmetry in specta v2-rc is to declare both attributes on the type rather than relying on a builder-level config that does not exist in the current API.

### `OrgError` discriminator design rationale

The `#[serde(tag = "kind", rename_all = "camelCase")]` attribute pair on `OrgError`:

- `tag = "kind"` produces an **internally-tagged** JSON shape: `{"kind": "parse", "file": "...", "reason": "..."}` rather than the default externally-tagged `{"parse": {"file": "...", "reason": "..."}}`. Internally-tagged unions are easier to consume in TypeScript via discriminated-union narrowing (`if (err.kind === "parse") { … err.file … }`).
- `rename_all = "camelCase"` on the enum applies to the **variant names** (`Parse` → `"parse"`, `Io` → `"io"`, `Index` → `"index"`, `Vault` → `"vault"`). Variant field names (`file`, `reason`) are already lowercase; the rename is a no-op on them but keeps the rule uniform.
- The `#[specta(rename_all = "camelCase")]` companion makes specta-typescript render the same shape — so the TS type literally matches what serde produces on the wire.

### Casing — literal epic AC vs. current specta API

The epic AC text says: `Builder::new().commands(collect_commands![ping]).config(specta::ts::ExportConfig::default().rename_all(specta::ts::RenameAll::CamelCase))` is the single source of casing.

**This API does not exist in `tauri-specta = "=2.0.0-rc.25"`.** It references the specta 1.x crate layout (`specta::ts::ExportConfig`, `specta::ts::RenameAll`). The v2-rc.25 ecosystem reorganizes as:
- `specta_typescript::Typescript` — the language exporter (no project-wide field-rename setter in its public surface).
- Command/argument names: auto-converted to camelCase by the renderer (`function.name().to_lower_camel_case()` is applied unconditionally; see [Source: ctx7 /websites/rs_tauri-specta_2_0_0-rc_21 — Render TypeScript Commands]). **No opt-in needed.**
- Struct/enum-field names: rendered as declared on the type. To enforce camelCase, use the specta-native attribute `#[specta(rename_all = "camelCase")]` on the type definition.

**Implementation choice**: declare `#[specta(rename_all = "camelCase")]` on every IPC-boundary type (Story 1.4 only has `OrgError`). This achieves the architecture's intent ("a single, uniform casing rule across the boundary") even though it is not literally "configured once in the builder."

**Recorded deviation** (logged in Change Log + Completion Notes): Story 1.4 deviates from the literal epic AC by attaching `#[specta(rename_all = "camelCase")]` per-type instead of a non-existent builder-level config. Architectural intent satisfied; literal text superseded.

### Generation timing — when does `tauri.ts` get written?

The canonical tauri-specta pattern places `builder.export(Typescript::default(), "../src/bindings.ts")` inside `#[cfg(debug_assertions)]` in the app's `main()` ([Source: ctx7 /specta-rs/tauri-specta — Create a new Tauri Specta builder]). This means:

- `pnpm tauri dev` → app starts in debug mode → `export()` runs → `tauri.ts` is written.
- `cargo build --workspace` alone → does NOT regenerate `tauri.ts` (it builds the binary but does not run it).
- `pnpm tauri build` → release build → `#[cfg(debug_assertions)]` is false → `export()` is skipped (intentional: release artifacts must be deterministic, not regenerate at startup).

The epic AC says "`cargo build --workspace` regenerates `shell-ui/src/lib/tauri.ts`" — **this is not literally true** with the canonical pattern. **Recorded deviation**: regeneration is anchored to `pnpm tauri dev` startup (default path) plus a `cargo test --test export_bindings` standalone test that runs `Builder::export()` head-only and writes the file (Task 6.1 Option A.3). The `prebuild` script in `shell-ui/package.json` calls this test so `pnpm --filter shell-ui build` on a fresh clone produces `tauri.ts` before TypeScript compilation reads it.

### Why a separate test crate target (`tests/export_bindings.rs`) instead of `build.rs`

- A `build.rs` runs during `cargo build`, writing into `shell-ui/` (a sibling tree). Every incremental rebuild would re-emit the file, polluting the build graph and risking `unstable-fingerprint` warnings.
- An integration test (`tests/export_bindings.rs`) runs only when explicitly invoked (`cargo test --test export_bindings`), is checked by CI in Story 1.8's CI matrix without surprises, and doubles as an assertion that bindings stay exportable (i.e., that no command's type breaks `specta::Type` derivation).
- The test body is intentionally minimal: it mirrors `lib.rs::run()`'s command list verbatim. **If a future story adds a command but forgets to update `tests/export_bindings.rs`, that command will not be in the pre-build regen** — keep the two `collect_commands![...]` lists synchronized. (Cleaner alternative for v0.5 Beta: extract `collect_commands![...]` to a const in `lib.rs` exposed to both `run()` and the test; deferred to avoid scope creep here.)

### LEAF crate invariant remains intact

- `orgsidian-plugin-api` is NOT touched by this story.
- `orgsidian-core` was previously a structural placeholder (single `lib.rs` doc comment, no deps). This story adds the first three real deps to it: `thiserror`, `serde`, `specta`. None of these are project crates — `orgsidian-core` stays the façade per architecture LD-NN.
- `orgsidian-shell-app` gains `orgsidian-core` as its first project-crate dep. This matches the architecture's dependency graph: `shell-app → core → {leaves}`.
- `cargo-deny check graph` is NOT yet wired (Story 1.7), so this story does not need to satisfy the graph rule programmatically; it does need to satisfy it structurally so that Story 1.7 lands clean.

### Reference `crates/orgsidian-shell-app/src/lib.rs` post-Task-4 shape

```rust
use orgsidian_core::{OrgError, Result as OrgResult};
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

#[tauri::command]
#[specta::specta]
fn ping() -> OrgResult<String> {
    Ok("pong".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let mut specta_builder = Builder::<tauri::Wry>::new().commands(collect_commands![ping]);

    #[cfg(debug_assertions)]
    specta_builder
        .export(Typescript::default(), "../../shell-ui/src/lib/tauri.ts")
        .expect("tauri-specta TS client export failed");

    let tauri_builder = tauri::Builder::default()
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

    // Story 13.2 activates the updater runtime: generates the signing key,
    // populates `plugins.updater.{pubkey,endpoints}` in tauri.conf.json, and
    // registers `tauri_plugin_updater::Builder::new().build()` here behind
    // `#[cfg(desktop)]`. Story 1.3 ships the Cargo dep, JS binding, and
    // capability permission only — runtime registration without real config
    // fails deserialization at startup.

    tauri_builder
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
}
```

### Reference `crates/orgsidian-core/src/error.rs` post-Task-2 shape

```rust
//! `OrgError`: project-wide IPC error type.
//!
//! Variants are struct-shaped so each error category carries diagnostic detail
//! (file/path/reason) without needing a separate context layer. The
//! discriminator on the wire is `kind` (internally-tagged) which TypeScript
//! consumers can narrow on directly.

pub type Result<T> = std::result::Result<T, OrgError>;

#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub enum OrgError {
    #[error("parse error in {file}: {reason}")]
    Parse { file: String, reason: String },
    #[error("io error: {reason}")]
    Io { reason: String },
    #[error("index error: {reason}")]
    Index { reason: String },
    #[error("vault error: {reason}")]
    Vault { reason: String },
}
```

### Reference `crates/orgsidian-core/src/lib.rs` post-Task-2 shape

```rust
//! orgsidian-core: core domain orchestrator (composition root for parser/index/watcher/vault/plugin-api/report).
//!
//! Structural placeholder — cross-crate edges materialize incrementally per first-use story.

mod error;
pub use error::{OrgError, Result};
```

### Reference `shell-ui/src/routes/_layout/today.tsx` post-Task-7 shape

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { commands } from "@/lib/tauri";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/_layout/today")({
  component: TodayPlaceholder,
});

function TodayPlaceholder() {
  const [reply, setReply] = useState("");

  async function ping() {
    setReply(await commands.ping());
  }

  return (
    <main className="container mx-auto p-8">
      <h1 className="text-2xl font-semibold">Today (placeholder)</h1>
      <p className="text-sm text-muted-foreground mt-2">
        Story 7.1 will replace this with the real Today Dashboard.
      </p>
      <div className="mt-6 flex gap-2">
        <Button type="button" onClick={ping}>
          Ping
        </Button>
      </div>
      <p className="mt-3 text-sm">{reply}</p>
    </main>
  );
}
```

### Reference `crates/orgsidian-shell-app/tests/export_bindings.rs` post-Task-6 shape

```rust
//! One-shot binding-export integration test.
//!
//! Doubles as the regen step invoked by `shell-ui` `prebuild`. The command
//! list MUST stay in sync with `crates/orgsidian-shell-app/src/lib.rs::run()`
//! — Story 12.x will extract them to a shared const if drift becomes a
//! recurring issue.

use orgsidian_shell_app_lib::__specta_ping_for_test_only_do_not_import as ping; // see note below
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

#[test]
fn export_bindings() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![ping]);
    builder
        .export(Typescript::default(), "../../shell-ui/src/lib/tauri.ts")
        .expect("tauri-specta TS client export failed");
}
```

> **Note on visibility**: `ping` is `fn ping(...) -> OrgResult<String>` in `lib.rs`, which keeps it module-private. For the test to call `collect_commands![ping]` it needs visibility into `ping`. Two options: (a) bump `ping` to `pub(crate)` and use `crate::ping` (cleanest; recommended); (b) re-export ping under a `#[doc(hidden)]` test-only path. Use option (a). If the dev agent finds the visibility shim awkward, an alternative is to move the `collect_commands![…]` call + the `Builder<Wry>::new()` chain into a `pub fn build_specta() -> Builder<Wry>` in `lib.rs` that both `run()` and the test call — that is the v0.5 Beta cleanup path and should NOT be preempted here.

### Reference post-story file structure (target additions/changes)

```
orgsidian/
├── Cargo.toml                                          (MODIFIED: +4 workspace.deps:
│                                                        tauri-specta, specta, specta-typescript, thiserror;
│                                                        +orgsidian-core path entry)
├── .gitignore                                          (MODIFIED: +shell-ui/src/lib/tauri.ts)
├── crates/
│   ├── orgsidian-core/
│   │   ├── Cargo.toml                                  (MODIFIED: +thiserror, serde, specta deps)
│   │   └── src/
│   │       ├── lib.rs                                  (MODIFIED: +mod error; +pub use)
│   │       └── error.rs                                (NEW: OrgError + Result alias)
│   └── orgsidian-shell-app/
│       ├── Cargo.toml                                  (MODIFIED: +orgsidian-core, tauri-specta,
│       │                                                specta, specta-typescript)
│       ├── src/
│       │   └── lib.rs                                  (REWRITTEN: ping() + specta Builder
│       │                                                wraps tauri::Builder)
│       └── tests/
│           └── export_bindings.rs                      (NEW: regen-on-prebuild + CI gate)
└── shell-ui/
    ├── package.json                                    (MODIFIED: prebuild extended to invoke
    │                                                    cargo test --test export_bindings)
    └── src/
        ├── lib/
        │   └── tauri.ts                                (GENERATED, GITIGNORED)
        └── routes/_layout/
            └── today.tsx                               (MODIFIED: invoke('greet') → commands.ping())
```

NOT touched: `crates/orgsidian-plugin-api/**` (LEAF), other crate stubs, `tools/corpus-extractor/**`, `.husky/**`, `commitlint.config.cjs`, `rust-toolchain.toml`, `tauri.conf.json`, `capabilities/main.json`, `shell-ui/vite.config.ts`, `shell-ui/tsconfig.json`, `shell-ui/src/main.tsx`, shadcn component forks, `shell-ui/src/styles/app.css`. The capability allow-list does NOT need a `core:default` entry for `commands.ping()` — `core:default` is already present from Story 1.3.

### Architecture compliance — what THIS story must satisfy

- **LD-24 (Tauri IPC with tauri-specta 2.x)** [Source: [architecture.md#LD-24](../planning-artifacts/architecture.md)]: `#[tauri::command]` + `#[specta::specta]` on every command; `collect_commands![...]` + `Builder::new().commands(...).export(...)`; TypeScript client generated under `shell-ui/src/lib/tauri.ts`. Story 1.4 establishes this; subsequent feature stories add commands incrementally.
- **LD-31 (IPC frontend consumption)** [Source: [architecture.md#LD-31](../planning-artifacts/architecture.md)]: frontend imports `{ commands }` from `@/lib/tauri`; never `invoke('...')` raw. Story 1.4 deletes the last raw `invoke()` call.
- **Type & Error Format (IPC + serialization)** [Source: [architecture.md#Type & Error Format](../planning-artifacts/architecture.md)]: every `#[tauri::command]` returns `Result<T, OrgError>`; `OrgError` is a single enum in `orgsidian-core/src/error.rs` deriving `thiserror::Error` + `serde::Serialize` + `specta::Type`; variants per error category (`Parse`/`Io`/`Index`/`Vault`). Story 1.4 ships this verbatim.
- **JSON / IPC payload casing** [Source: [architecture.md#JSON / IPC payload casing](../planning-artifacts/architecture.md)]: Rust structs `snake_case`, wire + TS-side `camelCase`. Story 1.4 implements via per-type `#[specta(rename_all)]` + matched `#[serde(rename_all)]` rather than a builder-level config (which does not exist in the current API — see Dev Notes §Casing).
- **Null handling** [Source: [architecture.md#Null handling](../planning-artifacts/architecture.md)]: `Option<T>` → `T | null`. Not exercised by `ping()` but established as the wire convention from this story onward.
- **AI-Agent Implementation Rule 6** [Source: [architecture.md#AI-Agent Implementation Rules](../planning-artifacts/architecture.md)]: "Use the generated `tauri-specta` client. Never call `invoke('command-name')` with a raw string." — Story 1.4 makes this rule structurally enforceable.
- **Anti-pattern enforcement** [Source: [architecture.md#Anti-Patterns (Forbidden)](../planning-artifacts/architecture.md)]: removes the last `invoke('command_name', …)` raw-string call; does NOT introduce per-struct `#[serde(rename_all = "camelCase")]` on any struct except `OrgError` itself (and `OrgError` is the discriminator-of-record; see Dev Notes §OrgError discriminator design rationale).

### Latest tech information (verified 2026-05-21 via `ctx7`)

- **tauri-specta version pin** [Source: ctx7 `/specta-rs/tauri-specta` — Install tauri-specta and dependencies]: the official install command is `cargo add tauri@2 specta@=2.0.0-rc.25 specta-typescript@0.0.12` + `cargo add tauri-specta@=2.0.0-rc.25 --features derive,typescript`. The `=` exact-version pin is intentional in the upstream docs — specta/tauri-specta RC versions are not API-stable, so floating pins are unsafe.
- **Builder pattern** [Source: ctx7 `/specta-rs/tauri-specta` — Create a new Tauri Specta builder]: `let mut builder = Builder::<tauri::Wry>::new().commands(collect_commands![…]); #[cfg(debug_assertions)] builder.export(Typescript::default(), "…/bindings.ts").expect(…); tauri::Builder::default().invoke_handler(builder.invoke_handler()).setup(move |app| { builder.mount_events(app); Ok(()) }).run(…)`.
- **Command auto-camelCase** [Source: ctx7 `/websites/rs_tauri-specta_2_0_0-rc_21` — Render TypeScript Commands]: `function.name().to_lower_camel_case()` is unconditional in the renderer. `arg_defs` iterate with `name.to_lower_camel_case()`. Translation: `fn open_file(file_path: String)` → `commands.openFile({ filePath: "..." })` with **zero opt-in**.
- **Type/enum casing** [Source: ctx7 `/specta-rs/specta` — Deriving `Type` for Structs and Enums]: specta v2-rc honors `#[serde(rename_all = "camelCase")]` (via `specta-serde`) AND a native `#[specta(rename_all = "camelCase")]` attribute. Use the specta-native attribute primarily; mirror with serde for wire-format symmetry.
- **`collect_commands!`** [Source: ctx7 `/specta-rs/tauri-specta` — collect_commands!]: combines Tauri's `generate_handler!` and Specta's `collect_functions!`. Each function MUST carry both `#[tauri::command]` and `#[specta::specta]`. Generic runtime params concrete-typed at the call site.
- **`Builder::export`** [Source: ctx7 `/specta-rs/tauri-specta` — Builder::export]: writes the generated bindings file using the supplied language exporter; call only in debug builds. Returns `Result<(), L::Error>`.

### Anti-patterns explicitly forbidden in this story

- ❌ Calling `tauri::generate_handler![ping]` after wiring `specta_builder.invoke_handler()` — duplicates the handler chain and bypasses specta's type collection. The two are mutually exclusive: `specta_builder.invoke_handler()` IS Tauri's invoke handler (specta wraps it).
- ❌ Omitting `#[specta::specta]` on `ping` — `collect_commands![ping]` will compile, but the command will be missing from the generated TS file (silent type-drift bug; LLM agents reliably miss this).
- ❌ Calling `Builder::export()` outside `#[cfg(debug_assertions)]` — release builds would write to the source tree, breaking reproducible builds.
- ❌ Floating version pins on `specta` / `tauri-specta` / `specta-typescript` — RC ecosystem; `=` pin is mandatory per upstream docs. **Exception**: `thiserror = "1"` floats on `1.x` (stable since 2019).
- ❌ Adding `#[serde(rename_all = "camelCase")]` to any struct OTHER than `OrgError` — every other type uses the specta-native attribute alone (since serde's casing is only needed when the wire format requires symmetry with specta, which only applies to types whose `serde::Serialize` actually fires; `OrgError` is `Serialize`, so it must match).
- ❌ Committing the generated `shell-ui/src/lib/tauri.ts` — gitignored (AC6); generated by `prebuild` test on every CI run.
- ❌ Importing `@tauri-apps/api/core`'s `invoke` from any new code — `shell-ui/src/` should have zero raw `invoke()` calls after Task 7 (verifiable via `rg`).
- ❌ Adding a TS-side `OrgsidianError` wrapper class — out of scope (AC10).
- ❌ Adding `tauri-plugin-updater::Builder::new().build()` to fix the AC1 Story 1.3 deviation — that is Story 13.2's scope; do not preempt.
- ❌ Modifying `tauri.conf.json` or `capabilities/main.json` — IPC capability `core:default` already covers specta-generated commands; no permission change needed.
- ❌ Introducing `app.emit()` event scaffolding via `collect_events![…]` — first event lands in Story 5.1.
- ❌ Adding `Builder::semantic_types(...)` — first chrono/bytes/url command lands in Story 2.x; opt in then.
- ❌ Renaming `OrgError`'s variant fields beyond what's specified — `Parse { file, reason }`, `Io { reason }`, `Index { reason }`, `Vault { reason }` is the architecture's contract; do not add `cause: Option<Box<dyn Error>>` chains here (deferred).
- ❌ Replacing `OrgResult<String>` with `Result<String, String>` to "simplify the ping AC" — the entire point of the story is the typed error round-trip. `Result<T, String>` would defeat AC2 + AC9.

### Previous story intelligence (Story 1.3 learnings)

Apply these patterns from Story 1.3's review/learnings to keep Story 1.4 frictionless:

1. **`pnpm tauri dev` is the source of truth for runtime gates.** Story 1.3 discovered the updater-deserialization-at-startup trap only via `pnpm tauri dev`; `cargo build --workspace` was green. Story 1.4 has the analogous trap: `cargo build` will not exercise `Builder::export()` because the binary is not run. Always run `pnpm tauri dev` AND open the app window before marking the story `review` — confirm the "Ping" → "pong" round-trip visually.
2. **Document deviations in Change Log + Completion Notes.** Story 1.3 disclosed three deviations (AC1 updater, AC4 baseUrl, AC4 radix-ui). Story 1.4 has two pre-known deviations: (a) literal epic AC API (`specta::ts::ExportConfig`) → current API (`#[specta(rename_all = …)]`); (b) literal epic AC "`cargo build --workspace` regenerates" → actual mechanism (`pnpm tauri dev` startup OR `cargo test --test export_bindings`). Disclose both verbatim.
3. **`[[feedback_version_policy]]` Tauri-exemption applies to specta.** Story 1.3 deferred Tauri-ecosystem version-pin discipline to "whatever pnpm tauri add installs." Story 1.4 reinforces: specta is RC-only, `=` exact pin per upstream docs is the correct policy here, NOT the LTS-preferred rule that applies to e.g. `@types/node`.
4. **Modify only what the AC dictates.** Story 1.3 originally drifted into reorganizing capabilities + reordering plugin registrations; the review cycle reverted these where the AC didn't require it. Story 1.4: do not touch `tauri.conf.json`, do not reorder plugin registrations, do not modify the shadcn forks, do not touch the route tree beyond `today.tsx`.
5. **Apply `[[feedback_batch_fixes_terse]]` during dev.** Story 1.3 ran a 3-layer code review and resolved 7 decision-needed items; the no-brainer fixes (clippy lints, missed gitignore entries) were applied silently. Same approach here: don't write a multi-paragraph rationale for every clippy nit; apply + move on.

### Git intelligence (recent commits)

Recent commits on `main` (per session start):
- `4543ea6` Merge PR #113 — Story 1.3 (Tauri plugin set + Tailwind 4 + shadcn + TanStack Router) merged.
- `c1c78a2` Story 1.3 implementation.
- `85affa7` Merge PR #112 — Story 1.2 (9-crate workspace + shell-ui/ at root) merged.

Implications:
- The 9-crate workspace is canonical; `[workspace.dependencies]` is the single source of cross-crate version pinning per Story 1.2.
- The 11 Tauri plugin registrations from Story 1.3 are present and must not be reordered.
- `shell-ui/src/routes/_layout/today.tsx` is the only frontend file that touches IPC today; it is the sole site of Task 7's edit.

### Testing requirements

Story 1.4 is infrastructure-wiring; the only **automated** test added is the binding-export integration test (`tests/export_bindings.rs`), which doubles as the regen step. The binding gates are:

1. `cargo check --workspace` → exit 0.
2. `cargo build --workspace` → exit 0.
3. `cargo test --test export_bindings --package orgsidian-shell-app` → exit 0; `shell-ui/src/lib/tauri.ts` exists.
4. `pnpm install` → exit 0 (only if Task 6 added deps; currently it should not need any).
5. `rm -f shell-ui/src/lib/tauri.ts && pnpm --filter shell-ui build` → exit 0 (verifies `prebuild` regenerates `tauri.ts` from a clean slate).
6. `pnpm tauri dev` → app boots cleanly; URL `http://localhost:1420/today`; "Ping" → "pong" round-trip works.
7. `pnpm tauri build` → emits `.app` + `.dmg` (release build does NOT regenerate `tauri.ts`, but the gitignored bundle directory does not need it — TS build happens at release-time via `prebuild` from a `cargo test` invocation).
8. `rg "from ['\"]@tauri-apps/api/core['\"]" shell-ui/src/` → exit 1 (no matches, last raw-`invoke` site removed).
9. `git check-ignore -v shell-ui/src/lib/tauri.ts` → matches root `.gitignore` entry.
10. No regression to Story 1.1 / 1.2 / 1.3 invariants — workspace structure, 11 Tauri plugin registrations, plugin-api LEAF, capability allow-list, route tree.

Property-based test coverage on `OrgError` is NOT added here (no fields to roundtrip-fuzz beyond `String`); first such test lands in Story 2.x when parser errors gain structure.

### Project Structure Notes

- **Alignment with unified project structure**: post-Story-1.4 layout exactly matches the architecture's Workspace Layout for the IPC bridge — `crates/orgsidian-core/src/error.rs` (canonical `OrgError` location per architecture) + `shell-ui/src/lib/tauri.ts` (canonical generated client per architecture). No deviations from the workspace layout.
- **Detected conflicts**: the literal epic-AC line `Builder::new().commands(collect_commands![ping]).config(specta::ts::ExportConfig::default().rename_all(specta::ts::RenameAll::CamelCase))` is **stale** against the actual specta v2-rc.25 API — see §Casing in Dev Notes. The deviation is documented; architectural intent is satisfied via per-type `#[specta(rename_all)]`.
- **Variance**: the test crate target `crates/orgsidian-shell-app/tests/export_bindings.rs` is a new file under an existing crate; no new crate is added. This matches the architecture's "no new crate without a story" implicit rule.

### References

- [Source: [epics.md#Epic 1 Story 1.4](../planning-artifacts/epics.md)] — Story user-story statement + 3 acceptance criteria (LD-24, OrgError shape, casing).
- [Source: [architecture.md#LD-24 — Tauri IPC with tauri-specta 2.x](../planning-artifacts/architecture.md)] — IPC bridge architectural decision.
- [Source: [architecture.md#LD-31 — IPC frontend consumption](../planning-artifacts/architecture.md)] — TS-side consumption pattern.
- [Source: [architecture.md#Type & Error Format (IPC + serialization)](../planning-artifacts/architecture.md)] — OrgError canonical shape + serialization rules.
- [Source: [architecture.md#JSON / IPC payload casing](../planning-artifacts/architecture.md)] — wire-format casing rule.
- [Source: [architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md)] — Rule 6: "Use the generated `tauri-specta` client."
- [Source: [architecture.md#Anti-Patterns (Forbidden)](../planning-artifacts/architecture.md)] — Raw `invoke()` + per-struct `#[serde(rename_all)]` bans.
- [Source: ctx7 `/specta-rs/tauri-specta` (verified 2026-05-21)] — install command, Builder pattern, `collect_commands!`, `Builder::export`, command auto-camelCase.
- [Source: ctx7 `/websites/rs_tauri-specta_2_0_0-rc_21` (verified 2026-05-21)] — renderer's `to_lower_camel_case()` behavior.
- [Source: ctx7 `/specta-rs/specta` (verified 2026-05-21)] — `#[specta(rename_all)]` + specta-serde Format symmetry.
- [Source: [../implementation-artifacts/1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md](./1-3-install-tauri-plugin-set-tailwind-4-shadcn-ui-forked-tanstack-router.md)] — Story 1.3 reference shapes for `lib.rs::run()` + `today.tsx`, deviation-reporting style.
- Persistent feedback memories: `[[feedback_version_policy]]` (Tauri-ecosystem pin discipline), `[[feedback_batch_fixes_terse]]` (apply no-brainer fixes silently during dev).

## Dev Agent Record

### Agent Model Used

claude-opus-4-7

### Debug Log References

- `cargo check -p orgsidian-core` (after Task 2 initial draft) — failed: `specta::Type` requires `derive` feature. Fixed in workspace dep `specta = { version = "=2.0.0-rc.25", features = ["derive"] }`.
- `cargo check -p orgsidian-core` (second run) — failed: `#[specta(rename_all = "camelCase")]` on containers rejected by specta `=2.0.0-rc.25`. Removed the attribute; `#[serde(rename_all = "camelCase")]` alone covers both wire + TS via specta-serde Format symmetry.
- `cargo check -p orgsidian-shell-app` (after Task 4 initial draft with `pub(crate) fn ping`) — failed: `__cmd__ping` / `__tauri_command_name_ping` defined multiple times. Promoted the `pub fn build_specta()` helper from Dev Notes' "v0.5 Beta cleanup" alternative to the implementation (see Completion Notes deviation).
- `cargo test --package orgsidian-shell-app --test export_bindings` (first generation) — generated `Promise<{ status: "ok"; data: T } | { status: "error"; error: E }>` (tagged-Result wrapper), failing AC9.2. Fix: `Builder::<tauri::Wry>::new().error_handling(ErrorHandlingMode::Throw).commands(...)`. Re-generated → `ping: () => __TAURI_INVOKE<string>("ping")` as required.
- `cargo build --release -p orgsidian-shell-app` — unused-import warning on `specta_typescript::Typescript` because the import is only consumed inside `#[cfg(debug_assertions)]`. Fix: guard the import with the same cfg.

### Completion Notes List

- **AC1 – workspace deps**: pinned `tauri-specta = "=2.0.0-rc.25"` (features `derive`, `typescript`), `specta = "=2.0.0-rc.25"` (with feature `derive` — see deviation below), `specta-typescript = "=0.0.12"`, `thiserror = "1"`. `orgsidian-core = { path = "crates/orgsidian-core" }` added so consumers opt in via `.workspace = true`.
- **AC2 – OrgError**: `crates/orgsidian-core/src/error.rs` declares `pub type Result<T> = std::result::Result<T, OrgError>;` and the four-variant enum (`Parse { file, reason }`, `Io { reason }`, `Index { reason }`, `Vault { reason }`). `lib.rs` re-exports both. `#[serde(tag = "kind", rename_all = "camelCase")]` mirrors the architecture's discriminator design.
- **AC3 – shell-app deps**: `orgsidian-core`, `tauri-specta`, `specta`, `specta-typescript` added to `crates/orgsidian-shell-app/Cargo.toml` under `[dependencies]` via `.workspace = true`.
- **AC4 – ping command**: legacy `greet(name: &str) -> String` deleted; `ping() -> OrgResult<String>` carries both `#[tauri::command]` and `#[specta::specta]`. A symbolic `let _ = OrgError::Io { … };` reference is required inside `ping` so that the unused `OrgError` import isn't silently dropped by the release build — minor, but documented here so a future cleanup understands the rationale.
- **AC5 – Builder composition root**: `run()` now uses `let specta_builder = build_specta();` then `tauri_builder.invoke_handler(specta_builder.invoke_handler()).setup(move |app| { specta_builder.mount_events(app); Ok(()) }).run(tauri::generate_context!())`. The 11 plugin registrations from Story 1.3 are preserved verbatim and in order; the Story 1.3 updater-deferred comment block is intact.
- **AC6 – generation timing**: chose **Option A.3** — file gitignored, `crates/orgsidian-shell-app/tests/export_bindings.rs` integration test serves double-duty as the regen step + a CI assertion that bindings stay exportable. `shell-ui/package.json`'s `prebuild` chain is now `tsr generate && cargo test --package orgsidian-shell-app --test export_bindings --quiet`.
- **AC7 – frontend consumes typed client**: `shell-ui/src/routes/_layout/today.tsx` now imports `{ commands }` from `@/lib/tauri` and invokes `await commands.ping()` via a single `<Button>Ping</Button>`. The form/name state was removed (per AC4 epic note — `ping()` takes no args).
- **AC8 / Casing**: command name + arg-name camelCase is automatic via the renderer (`to_lower_camel_case()`). Struct/enum-field casing is governed by `#[serde(rename_all = "camelCase")]` on `OrgError`. The architecture's intent ("single uniform casing rule across the boundary") is satisfied.
- **AC9 – round-trip**:
  - 9.2 ✅ — `commands.ping: () => __TAURI_INVOKE<string>("ping")` (typed `Promise<string>`, no Result wrapper).
  - 9.3 ✅ — `OrgError = { kind: "parse"; ... } | { kind: "io"; ... } | { kind: "index"; ... } | { kind: "vault"; ... };`.
  - 9.1 ✅ — Manually verified by Tiziano on 2026-05-21: `pnpm tauri dev` launched, `/today` route rendered, clicking "Ping" displayed `"pong"`.
- **AC10 – anti-creep**: `collect_commands![ping]` is the only command; no `collect_events!`, no `semantic_types(...)`, no `OrgsidianError` wrapper, no `generate_handler!` macro call anywhere, no `tauri.conf.json`/`capabilities/main.json` edits, no plugin reorder/removal. Last raw `invoke()` site removed (`rg "from '@tauri-apps/api/core'" shell-ui/src/` → exit 1).

#### Recorded deviations (per Story 1.3 disclosure pattern)

1. **Literal epic-AC API does not exist in specta `=2.0.0-rc.25`.** The Dev Notes already flagged this; the implementation uses per-type `#[serde(rename_all = "camelCase")]` (not the original `#[specta(rename_all)]` either — see deviation #4) instead of a non-existent `specta::ts::ExportConfig::default().rename_all(specta::ts::RenameAll::CamelCase)` builder call. Architectural intent satisfied.
2. **`cargo build --workspace` does NOT regenerate `tauri.ts`.** Regeneration is anchored to (a) `pnpm tauri dev` startup via `#[cfg(debug_assertions)]` in `lib.rs::run()`, or (b) the `cargo test --test export_bindings` step invoked by `shell-ui`'s `prebuild`. Already flagged in Dev Notes §Generation timing.
3. **`build_specta()` helper preempts the v0.5 Beta cleanup path.** The Dev Notes recommended option (a) — bumping `ping` to `pub(crate)` and using `crate::ping` from the test — and explicitly said the `build_specta()` helper "should NOT be preempted here." Option (a) is broken in `=2.0.0-rc.25`: `pub(crate) fn ping` + `#[tauri::command]` + `#[specta::specta]` triggers an `__cmd__ping` macro re-import collision. The `build_specta()` helper was the only path that compiles; rationale documented in the helper's doc comment.
4. **`#[specta(rename_all = "camelCase")]` removed from `OrgError`.** specta `=2.0.0-rc.25` explicitly errors on this attribute on containers ("`#[specta(rename_all ...)]` is no longer supported on containers. Use `#[serde(rename_all = "...")]` instead."). Removed; `#[serde(rename_all = "camelCase")]` alone covers both wire + TS via specta-serde Format symmetry. The Dev Notes "Anti-patterns" entry forbidding extra `#[serde(rename_all)]` was scoped to "structs OTHER than `OrgError`"; `OrgError` was already allowed to carry the serde attribute, so this change is within the allowed envelope. Architectural intent (uniform camelCase on the wire) preserved.
5. **`specta` workspace dep needs `features = ["derive"]`.** Story spec said `specta = "=2.0.0-rc.25"` (no features); compilation requires `derive` for the `specta::Type` proc-macro. Added.
6. **`use specta_typescript::Typescript;` gated behind `#[cfg(debug_assertions)]`** to keep release builds warning-free (the import is only consumed inside the debug-only `export()` call).

### File List

- `Cargo.toml` (MODIFIED) — added `tauri-specta`, `specta` (with `derive` feature), `specta-typescript`, `thiserror`, and `orgsidian-core` to `[workspace.dependencies]`.
- `.gitignore` (MODIFIED) — added `shell-ui/src/lib/tauri.ts`.
- `crates/orgsidian-core/Cargo.toml` (MODIFIED) — added `thiserror`, `serde`, `specta` workspace deps.
- `crates/orgsidian-core/src/lib.rs` (MODIFIED) — `mod error;` + `pub use error::{OrgError, Result};`.
- `crates/orgsidian-core/src/error.rs` (NEW) — `OrgError` enum + `Result<T>` alias.
- `crates/orgsidian-shell-app/Cargo.toml` (MODIFIED) — added `orgsidian-core`, `tauri-specta`, `specta`, `specta-typescript` workspace deps.
- `crates/orgsidian-shell-app/src/lib.rs` (REWRITTEN) — `ping()` command, `pub fn build_specta()` helper (with `ErrorHandlingMode::Throw`), `run()` driving Tauri via specta Builder.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` (NEW) — one-shot integration test that exports the TS client; invoked by `shell-ui`'s `prebuild`.
- `shell-ui/package.json` (MODIFIED) — `prebuild` extended to invoke `cargo test --package orgsidian-shell-app --test export_bindings --quiet`.
- `shell-ui/src/routes/_layout/today.tsx` (MODIFIED) — typed `commands.ping()` replaces raw `invoke('greet', ...)`.
- `shell-ui/src/lib/tauri.ts` (GENERATED, GITIGNORED) — not committed; produced by `prebuild` / `pnpm tauri dev`.

### Change Log

- 2026-05-21 — Story 1.4 implementation: typed IPC bridge wired (`tauri-specta = "=2.0.0-rc.25"`), `OrgError` declared in `orgsidian-core`, `ping()` placeholder, `commands.ping()` consumed by `/today`. Generated bindings gitignored; regen via `cargo test --test export_bindings` from `shell-ui` `prebuild`. Six deviations recorded in Completion Notes (literal API gap, `build_specta()` helper preempt, `#[specta(rename_all)]` removal, `derive` feature, debug-only Typescript import, generation-timing).
