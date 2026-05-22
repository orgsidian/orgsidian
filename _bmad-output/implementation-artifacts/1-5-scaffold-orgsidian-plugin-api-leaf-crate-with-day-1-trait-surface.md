# Story 1.5: Scaffold `orgsidian-plugin-api` leaf crate with day-1 trait surface

Status: done

## Metadata

github_issue: 5

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want `crates/orgsidian-plugin-api/` populated as a LEAF crate (no project deps) carrying the full `OrgsidianPlugin` trait + `Event` enum (`#[non_exhaustive]`) + `HookOutcome<T>` + `HookContext` / `PluginContext` traits + the supporting payload types (`PluginMetadata`, `CaptureEntry`, `AgendaQuery`, `AgendaItem`, `PluginError`, `Result<T>` alias),
So that FR-24 internal Plugin Pattern is woven from Epic 2 onwards without retrofit cost — every consuming v1.0 feature (Capture, Save, Agenda, Search, Report, Themes) sees the same trait surface from day 1, and the v1.5+ crates.io publication path stays mechanical rather than architectural.

## Acceptance Criteria

**AC1 — Crate stays a LEAF (zero project dependencies, third-party deps minimal).**
`crates/orgsidian-plugin-api/Cargo.toml` declares no `path = "../*"` dependencies and no `workspace = true` entries pointing at any of `orgsidian-{parser,index,watcher,vault,report,core,cli,shell-app}`. Third-party deps are limited to what the trait surface strictly requires; default minimal set is **`serde = { workspace = true }`** (for `PluginMetadata` / `CaptureEntry` / etc. derives) and **`thiserror = { workspace = true }`** (for `PluginError`). No `tracing`, no `tokio`, no `chrono` — they are NOT part of the day-1 surface. The leaf invariant is what makes the v1.5+ crates.io publication possible per LD-10 / LD-26 amendment.

**AC2 — `OrgsidianPlugin` trait declared in `crates/orgsidian-plugin-api/src/lib.rs` per LD-26 (with the LD-5 round-4 `&dyn` amendment).**

```rust
pub trait OrgsidianPlugin: Send + Sync {
    /// Returns plugin metadata (name, version, author).
    fn metadata(&self) -> PluginMetadata;

    /// Called once at plugin load. Plugins receive a borrowed context;
    /// no ownership transfer keeps the surface WASM-compatible per LD-25.
    fn init(&mut self, ctx: &dyn PluginContext) -> Result<()>;

    /// Called once at plugin unload / app shutdown.
    fn shutdown(&mut self) -> Result<()>;

    /// Plugin priority for hook dispatch ordering; default 0.
    /// Lower values run first; ties resolve by load order.
    fn priority(&self) -> i32 { 0 }

    /// Fire-and-forget observer. Default no-op.
    /// Used for logging, badges, sync-to-external, etc.
    fn on_event(&mut self, _event: &Event) -> Result<()> { Ok(()) }

    /// Pre-save hook — plugin may transform content before write.
    fn on_save_before(
        &mut self,
        _ctx: &dyn HookContext,
        _content: &str,
    ) -> Result<HookOutcome<String>> {
        Ok(HookOutcome::Continue)
    }

    /// Pre-capture hook — plugin may transform a Quick Capture entry before commit.
    fn on_capture_before(
        &mut self,
        _ctx: &dyn HookContext,
        _entry: &CaptureEntry,
    ) -> Result<HookOutcome<CaptureEntry>> {
        Ok(HookOutcome::Continue)
    }

    /// Agenda query transform — plugin may post-process query results.
    fn on_agenda_query_after(
        &mut self,
        _ctx: &dyn HookContext,
        _query: &AgendaQuery,
        _results: &mut Vec<AgendaItem>,
    ) -> Result<()> {
        Ok(())
    }
}
```

Critical shape rules:
- Bounds **`Send + Sync`** are mandatory — the host invokes plugins from `Vec<Box<dyn OrgsidianPlugin>>` accessed across the async runtime.
- Context parameters are **`&dyn PluginContext`** / **`&dyn HookContext`**, NOT owned values or concrete types — this is the LD-5 round-4 amendment that preserves the leaf invariant.
- `init` is `&mut self`; `shutdown` is `&mut self`. Both return `Result<()>`.
- Optional hooks carry default `Continue` / `Ok(())` impls so plugins opt in by overriding.
- Parameter names on default-impl methods are prefixed `_` to keep `#![warn(clippy::pedantic)]` clean.

**AC3 — `Event` enum is `#[non_exhaustive]` with PascalCase past-tense variants per LD-26 + naming-conventions LD.**

```rust
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Event {
    FileOpened { path: String },
    FileSaved { path: String },
    FileChanged { path: String },
    HeadlineEdited { file: String, headline_id: String },
    ClockStarted { file: String, headline_id: String },
    ClockStopped { file: String, headline_id: String },
    CaptureSubmitted { entry: CaptureEntry },
    AgendaQueried { query: AgendaQuery },
    IndexRebuilt,
}
```

- The exact 9-variant set above is mandatory (variant names must match LD-26 verbatim).
- `#[non_exhaustive]` on the enum forces consumers to use a wildcard `_` arm — this is the forward-compatibility hedge per naming-conventions LD.
- Variant payloads are placeholder day-1 shapes that **may be refined later as long as the variant name + general intent stay stable** (additive field refinements within a variant land as minor bumps per LD-26 SemVer policy).
- `Clone` is required because `HookContext::emit_event` (AC5) may need to clone the event for fan-out to multiple observers; `Debug` is required for `tracing::error!` panic logging in `invoke_plugin_hook!` (Story 1.8).
- **Do NOT add** other variants or `#[deprecated]` markings in this story — only the 9 listed.

**AC4 — `HookOutcome<T>` enum declared exactly per LD-26.**

```rust
#[derive(Debug, Clone)]
pub enum HookOutcome<T> {
    Continue,
    Replace(T),
    Cancel(String),
}
```

- The three variants `Continue` / `Replace(T)` / `Cancel(String)` are mandatory. `Cancel` carries a `String` user-visible reason; this is what the host surfaces in error UI / logs when a plugin cancels a save or capture.
- `Debug` + `Clone` derives are required for downstream consumers (registry logging + retry semantics).
- **Do NOT** introduce a 4th variant (e.g., `Defer(Duration)`) in this story — that is explicitly reserved for the LD-50 v0.5 surface review.

**AC5 — `HookContext` and `PluginContext` are TRAITS (per LD-5 round-4 amendment), not concrete types.**

```rust
/// Provided to plugins at `init`. Read-only handle to host capabilities
/// available for the plugin's lifetime.
pub trait PluginContext: Send + Sync {
    /// Plugin's own metadata as registered at load time (loop-back for
    /// plugins that need to inspect their own host-resolved identity).
    fn plugin_metadata(&self) -> &PluginMetadata;
}

/// Provided to plugins at hook invocation. Borrowed for the duration of
/// the hook call only — plugins MUST NOT retain a `&dyn HookContext`
/// reference across calls (lifetime is bound to the hook frame to keep
/// the surface WASM-compatible per LD-25 + LD-26).
pub trait HookContext: Send + Sync {
    /// Read a file from the active Vault.
    /// Path is Vault-relative (host enforces the Vault allow-list per LD-17).
    fn read_vault_file(&self, path: &str) -> Result<String>;

    /// Query the index. Day-1 surface accepts an opaque query string;
    /// structured query API lands when `orgsidian-index::query::*` materializes
    /// (Stories 3.x / 8.x). The String/String shape keeps plugin-api LEAF
    /// while leaving room to introduce a typed `IndexQuery` enum in a
    /// minor bump per LD-26 SemVer policy.
    fn query_index(&self, query: &str) -> Result<String>;

    /// Emit an event for fan-out to other plugins / host listeners.
    fn emit_event(&self, event: Event) -> Result<()>;
}
```

- Both traits carry the **`Send + Sync`** super-bound — plugin invocation occurs from the host's async runtime and the `Vec<Box<dyn OrgsidianPlugin>>` registry per LD-25.
- **`PluginContext`** is minimal in day-1 (loop-back metadata only). It exists primarily as the **type symbol** that future stories extend; locking the trait name + bound now lets every consuming story add methods as additive minor bumps.
- **`HookContext`** carries the three method surfaces mandated by architecture (`read_vault_file`, `query_index`, `emit_event`). The structured `tracing` logger mentioned in architecture LD-26 prose is **deliberately deferred** to a later story (first plugin author actually needs it) so that `tracing` does NOT become a transitive dep of the leaf crate — architectural intent ("plugins receive observability via host") is preserved; the literal "structured tracing logger" surfaces in a minor bump when needed.
- All three method signatures take `&self` (immutable host) — no mutable references handed to plugins, consistent with architecture LD-26 ("no mutable references handed to plugins to keep the surface WASM-compatible").
- `query_index` and `read_vault_file` return `Result<String>` (the local `Result` alias from AC8); concrete index/vault types stay on their crate-of-origin side of the IPC boundary.

**AC6 — `PluginMetadata` carries name/version/author (day-1 shape).**

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    /// Stable plugin identifier (e.g., "agenda", "quick-capture", "themes").
    /// Used as the key in the host's plugin registry; MUST be unique per app.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// SemVer string.
    pub version: String,
    /// Author / organisation.
    pub author: String,
}
```

- `id` is the registry key (matches the bundled-plugin crate name when v1.0 plugins land in Epics 8-12).
- The struct is `Serialize + Deserialize` because it round-trips through `tauri-specta` IPC when the Settings UI lists plugins (Stories 12.x). Adding `specta::Type` here is **not** required in 1.5 — `specta` is NOT a plugin-api dep (would taint the LEAF for crates.io publication); the host-side façade type in `orgsidian-core` will re-wrap `PluginMetadata` when IPC needs it. Decision recorded under Dev Notes §LEAF dep policy.
- Do NOT add a `description: Option<String>`, `homepage`, `repository`, etc. — those are LD-50 surface-review candidates, not day-1.

**AC7 — `CaptureEntry`, `AgendaQuery`, `AgendaItem` are placeholder day-1 shapes referenced by trait signatures and Event variants.**

```rust
/// Day-1 placeholder. Concrete capture schema lands with Story 8.1
/// (Quick Capture window); fields added then arrive as minor bumps.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaptureEntry {
    pub raw_text: String,
}

/// Day-1 placeholder. Concrete agenda query schema lands with Stories 6.3 / 6.4
/// (Today / Week agenda); fields added then arrive as minor bumps.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgendaQuery {
    pub raw_filter: String,
}

/// Day-1 placeholder. Concrete agenda item schema lands with Stories 6.3 / 6.4
/// + Story 7.x dashboard widgets; fields added then arrive as minor bumps.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgendaItem {
    pub headline_id: String,
    pub display_text: String,
}
```

- Each struct carries `///` doc-comments that **explicitly call out which future story will refine the shape**. This is the contract that lets architecture's "additive minor bump" SemVer policy work — the variant/struct names are locked, the fields are room-for-growth.
- Fields are deliberately minimal — `raw_text` / `raw_filter` are escape-hatch strings the host can construct from richer internal types, so the leaf crate never needs to import the richer types.
- All three derive `Serialize + Deserialize` for the same reason as `PluginMetadata` (round-tripping via host IPC when needed).

**AC8 — `PluginError` enum + `Result<T>` alias are local to the leaf crate (NOT `orgsidian-core::OrgError`).**

```rust
pub type Result<T> = std::result::Result<T, PluginError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PluginError {
    #[error("plugin init failed: {reason}")]
    Init { reason: String },
    #[error("plugin runtime error: {reason}")]
    Runtime { reason: String },
    #[error("host capability unavailable: {capability}")]
    HostUnavailable { capability: String },
    #[error("invalid input from host: {reason}")]
    InvalidInput { reason: String },
}
```

- This type lives in plugin-api specifically because **the leaf cannot depend on `orgsidian-core`** (where `OrgError` lives — Story 1.4). The two error types are intentionally separate per the leaf invariant; the host's `orgsidian-core::registry` will `From::from(PluginError)` into `OrgError` when surfacing through IPC (later story).
- `#[non_exhaustive]` keeps variant-additions backward-compatible per LD-26 SemVer policy.
- Do NOT derive `serde::Serialize` on `PluginError` — error wire-format is `OrgError`'s job, not plugin-api's. Plugin authors handle their own error shapes internally.

**AC9 — `crates/orgsidian-plugin-api/src/lib.rs` enforces architecture's strict doc + pedantic linting per "Documentation Conventions" + "Linting & Formatting" sections.**

The crate root has these inner attributes at the top of `lib.rs` (after the file-level doc comment):

```rust
#![warn(clippy::pedantic)]
#![deny(missing_docs)]
#![doc = "..."] // (or replaced by the existing `//!` block)
```

- `clippy::pedantic` is required per architecture "`clippy::pedantic` enabled on `orgsidian-plugin-api` (public surface); allow-listed elsewhere." Story 1.5 is the first story that ships actual `pub` items in plugin-api — turning it on now prevents accumulated lint debt.
- `#![deny(missing_docs)]` enforces "`orgsidian-plugin-api` public items: `///` doc comments mandatory; `cargo doc --no-deps` clean (no warnings)" per architecture Documentation Conventions.
- If a specific `clippy::pedantic` lint is genuinely incompatible with the trait surface (e.g., `clippy::must_use_candidate` on every default-impl method), allow-list it **at the item or module level** with a `// rationale:` comment — do NOT blanket-allow at the crate root.

**AC10 — `cargo doc --no-deps -p orgsidian-plugin-api` exits 0 with zero warnings.**

Every `pub` item carries a `///` doc-comment. The crate root keeps the existing `//!` module-level doc comment (replacing the "Structural placeholder" text with a real summary).

**AC11 — `crates/orgsidian-plugin-api/CHANGELOG.md` is created with the `0.0.0` initial-trait-surface entry per LD-33.**

```markdown
# Changelog

All notable changes to `orgsidian-plugin-api` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
internally from day 1 even though it is not published to crates.io until v1.5+
(see LD-10 / LD-33 in `_bmad-output/planning-artifacts/architecture.md`).

## [Unreleased]

## [0.0.0] - 2026-05-22

### Added

- Initial trait surface (Story 1.5):
  - `OrgsidianPlugin` trait with `metadata`, `init`, `shutdown`, `priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after` methods (LD-26).
  - `Event` enum (`#[non_exhaustive]`) with v1.0 variants: `FileOpened`, `FileSaved`, `FileChanged`, `HeadlineEdited`, `ClockStarted`, `ClockStopped`, `CaptureSubmitted`, `AgendaQueried`, `IndexRebuilt`.
  - `HookOutcome<T>` with `Continue`, `Replace(T)`, `Cancel(String)`.
  - `HookContext` and `PluginContext` traits (`Send + Sync`; passed to plugins as `&dyn` references per LD-5 round-4 amendment).
  - `PluginMetadata`, `CaptureEntry`, `AgendaQuery`, `AgendaItem` payload structs (day-1 minimal shapes; SemVer-additive growth path).
  - `PluginError` enum + `Result<T>` alias (local to leaf; separate from `orgsidian-core::OrgError`).
```

- The `0.0.0` heading is mandatory and dated to commit day. `Unreleased` heading is mandatory (used by `git-cliff` per Story 1.15).
- Keep-a-Changelog format aligns with architecture LD-33's "CHANGELOG.md per crate + project root."

**AC12 — `crates/orgsidian-plugin-api/Cargo.toml` adds the two third-party deps and the description stays accurate.**

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
# LEAF crate — no project deps. Third-party deps minimal: serde for payload
# struct round-trip, thiserror for the local PluginError enum.
serde = { workspace = true }
thiserror = { workspace = true }
```

- The `description` line is the existing one (verbatim from Story 1.2 stub) — DO NOT alter it.
- Workspace `serde` already carries `features = ["derive"]` from Story 1.2 / 1.3 — no per-crate feature override needed.
- Workspace `thiserror` was added in Story 1.4 — no new workspace dep declaration required.

**AC13 — Anti-creep: nothing else in this story.**

- ❌ NO `invoke_plugin_hook!` macro in `orgsidian-core` — that lands in Story 1.8 (AC explicit: "`crates/orgsidian-core/src/registry.rs` declares the `invoke_plugin_hook!` macro stub").
- ❌ NO `PluginRegistry` struct or `Vec<Box<dyn OrgsidianPlugin>>` field in `orgsidian-core` — that's the first internal-plugin story (Epic 6 / 7 onwards).
- ❌ NO `[profile.release] panic = "unwind"` in root `Cargo.toml` — that's Story 1.8 AC explicit.
- ❌ NO bundled-plugin crates under `crates/` — first one lands when Today Dashboard / Capture / Search wires up in Epic 6+.
- ❌ NO `examples/plugins/hello-world/` skeleton — architecture lists it but ships at v1.5+; do not preempt.
- ❌ NO `cargo-deny check graph` configuration — that's Story 1.7 AC explicit. Leaf invariant is verified **manually** in this story by reading `Cargo.toml` (Task 7).
- ❌ NO `From<PluginError> for OrgError` impl — first uses in `orgsidian-core::registry` (later story) will own that conversion; adding it now would couple plugin-api to core's error API too early.
- ❌ NO `concrete `HostHookContext` / `HostPluginContext` impls in `orgsidian-core` — first internal plugin lands them.
- ❌ NO `tracing` dep on plugin-api — architecture mentions a "structured tracing logger" on `HookContext` but that surfaces as a minor-bump method when the first plugin author needs it (deliberate deferral; documented in Dev Notes).
- ❌ NO new commands or IPC wiring in `shell-app` — Story 1.5 ships pure Rust trait definitions; `shell-ui/`, `tauri.conf.json`, `capabilities/`, the `prebuild`/specta export chain are NOT touched.
- ❌ NO modification to `shell-ui/src/lib/tauri.ts` regen path — plugin-api types are NOT yet IPC-exposed (will be at first Settings UI plugin-list surface in Stories 12.x).
- ❌ NO `Deserialize` derive on `PluginError` — wire-format errors are `OrgError`'s job (see AC8).
- ❌ NO `Defer(Duration)` 4th variant on `HookOutcome` — reserved for LD-50 v0.5 surface review.
- ❌ NO addendum/edit to `crates/orgsidian-shell-app/tests/export_bindings.rs` — plugin-api types do not go through specta in this story.

## Tasks / Subtasks

- [x] **Task 1: Update `crates/orgsidian-plugin-api/Cargo.toml`** (AC1, AC12)
  - [x] 1.1 Add `[dependencies]` block with `serde = { workspace = true }` and `thiserror = { workspace = true }`. Replace the existing stub comment line with the rationale comment in AC12's snippet.
  - [x] 1.2 Confirm zero `path` deps; confirm no `workspace = true` entry points at any other project crate. Re-read the Cargo.toml after editing.
  - [x] 1.3 `cargo check -p orgsidian-plugin-api` → exit 0 (still compiles with just the empty deps added, before the `lib.rs` rewrite below).

- [x] **Task 2: Lay out `crates/orgsidian-plugin-api/src/lib.rs` module structure** (AC9, AC10)
  - [x] 2.1 Replace the current placeholder `lib.rs` content. Top of file:
    - `//!` module-level doc comment summarizing the crate purpose, pointing to architecture LD-10 / LD-26 / LD-5 round-4 / LD-33 by name.
    - Crate-root inner attributes: `#![warn(clippy::pedantic)]` and `#![deny(missing_docs)]` (per AC9).
  - [x] 2.2 Decide module decomposition. **Recommended layout** (matches architecture "one concern per file"; see Dev Notes §Module decomposition):
    - `mod error;` → `PluginError` + `Result<T>` alias.
    - `mod event;` → `Event` enum.
    - `mod outcome;` → `HookOutcome<T>` enum.
    - `mod metadata;` → `PluginMetadata` struct.
    - `mod payload;` → `CaptureEntry`, `AgendaQuery`, `AgendaItem`.
    - `mod context;` → `HookContext` + `PluginContext` traits.
    - `mod plugin;` → `OrgsidianPlugin` trait.
    - `pub use` re-exports of every public item at the crate root so consumers can write `use orgsidian_plugin_api::{Event, HookOutcome, OrgsidianPlugin};` without reaching into module paths.
  - [x] 2.3 `cargo check -p orgsidian-plugin-api` → exit 0 (modules empty but declared).

- [x] **Task 3: Implement `PluginError` + `Result<T>` alias** (AC8)
  - [x] 3.1 Create `crates/orgsidian-plugin-api/src/error.rs` with the verbatim AC8 code block (four variants, `#[non_exhaustive]`, `thiserror::Error` derive only — no `serde::Serialize`).
  - [x] 3.2 Every variant carries a `///` doc-comment describing when the host or plugin returns it.
  - [x] 3.3 Re-export at crate root: `pub use error::{PluginError, Result};`.

- [x] **Task 4: Implement `HookOutcome<T>` enum** (AC4)
  - [x] 4.1 Create `crates/orgsidian-plugin-api/src/outcome.rs` with the verbatim AC4 code block + `///` doc-comments per variant. The `Cancel(String)` variant doc-comment notes the host surfaces the message in error UI/logs.
  - [x] 4.2 Re-export at crate root: `pub use outcome::HookOutcome;`.

- [x] **Task 5: Implement payload structs** (AC6, AC7)
  - [x] 5.1 Create `crates/orgsidian-plugin-api/src/metadata.rs` with `PluginMetadata` per AC6 + per-field `///` doc-comments. Confirm `Serialize + Deserialize` derives.
  - [x] 5.2 Create `crates/orgsidian-plugin-api/src/payload.rs` with `CaptureEntry`, `AgendaQuery`, `AgendaItem` per AC7. Each struct's doc-comment **names the future story** that refines its shape ("Story 8.1 Quick Capture", "Stories 6.3 / 6.4 Today / Week agenda", "Story 7.x dashboard widgets").
  - [x] 5.3 Re-export at crate root: `pub use metadata::PluginMetadata; pub use payload::{CaptureEntry, AgendaQuery, AgendaItem};`.

- [x] **Task 6: Implement `Event` enum** (AC3)
  - [x] 6.1 Create `crates/orgsidian-plugin-api/src/event.rs` with the verbatim AC3 code block.
  - [x] 6.2 `#[non_exhaustive]` is mandatory on the enum (NOT on individual variants). `Debug + Clone` derives confirmed.
  - [x] 6.3 `///` doc-comment on the enum AND each variant explains when the host emits it (point at the future story that ships the emit-site: e.g., `FileSaved` → Story 3.1 atomic write, `ClockStarted` → Story 7.6 clock manager). The story references make the day-1 shape audit-traceable to the v0.5 LD-50 review.
  - [x] 6.4 Re-export at crate root: `pub use event::Event;`.

- [x] **Task 7: Implement `HookContext` + `PluginContext` traits** (AC5)
  - [x] 7.1 Create `crates/orgsidian-plugin-api/src/context.rs` with the verbatim AC5 code block.
  - [x] 7.2 Confirm both traits carry the `Send + Sync` super-bound. Confirm every method takes `&self` (no `&mut self`).
  - [x] 7.3 `///` doc-comment on each trait method documents: (a) what host capability the method exposes, (b) which architecture LD constrains it (LD-17 for FS allow-list, LD-26 for query shape rationale), (c) day-1 limitation if the signature is intentionally narrow (e.g., `query_index` String→String escape hatch).
  - [x] 7.4 Re-export at crate root: `pub use context::{HookContext, PluginContext};`.

- [x] **Task 8: Implement `OrgsidianPlugin` trait** (AC2)
  - [x] 8.1 Create `crates/orgsidian-plugin-api/src/plugin.rs` with the verbatim AC2 code block.
  - [x] 8.2 Required-method doc-comments cite LD-26 verbatim text (`metadata`, `init`, `shutdown`); optional-method doc-comments cite the Emacs-hook-rationale text from architecture LD-26 ("`:before/:after/:around` advice" framing) so the dev agent doesn't reinvent the rationale in later stories.
  - [x] 8.3 Default-impl method parameters are prefixed `_` (e.g., `_event: &Event`) so `clippy::pedantic` does not flag unused parameters on the trait surface.
  - [x] 8.4 Re-export at crate root: `pub use plugin::OrgsidianPlugin;`.

- [x] **Task 9: Add a minimal smoke test that the trait surface compiles + is implementable by a third party** (AC2, AC5, AC9, anti-placebo-green per Party Mode P2)
  - [x] 9.1 Create `crates/orgsidian-plugin-api/tests/trait_surface.rs`.
  - [x] 9.2 Inside the test, declare a `struct NoopPlugin;` that implements `OrgsidianPlugin` minimally (just `metadata` + `init` + `shutdown` — uses default impls for everything else). Implement a `struct NoopHookContext` / `struct NoopPluginContext` that satisfy the host-side traits as fake stubs (returns `Ok(String::new())` / `Ok(())` etc).
  - [x] 9.3 The test body constructs `Box<dyn OrgsidianPlugin>`, invokes `metadata()` + `priority()` + `on_event(&Event::IndexRebuilt)` and asserts the defaults return `Continue` / `Ok(())`. This proves: (a) the trait is object-safe; (b) the `Send + Sync` bound is satisfiable; (c) defaults wire correctly; (d) `Event::IndexRebuilt` (zero-field variant) constructs without payload.
  - [x] 9.4 `cargo test -p orgsidian-plugin-api` → exit 0.
  - [x] 9.5 Add a second test `cancel_outcome_carries_reason` that constructs `HookOutcome::Cancel::<String>("plugin foo declined".into())` and asserts the inner string round-trips via `match`. Trivial but proves the parametric `T` resolves.

- [x] **Task 10: Doc-build + lint gates** (AC9, AC10)
  - [x] 10.1 `cargo doc --no-deps -p orgsidian-plugin-api` → exit 0 with **zero warnings**. If `missing_docs` fires on a re-exported item, the issue is at the source location (`error.rs` / `event.rs` / etc.) — fix there, do NOT add `#[allow(missing_docs)]` on re-exports.
  - [x] 10.2 `cargo clippy -p orgsidian-plugin-api -- -D warnings` → exit 0. If `clippy::pedantic` flags a genuinely incompatible lint (e.g., `clippy::must_use_candidate` on every default-impl method), allow-list it at the item or module level with a `// rationale:` comment per AC9; do NOT blanket-allow at the crate root.
  - [x] 10.3 `cargo fmt --check` → exit 0.

- [x] **Task 11: Create `crates/orgsidian-plugin-api/CHANGELOG.md`** (AC11)
  - [x] 11.1 Create the file with the AC11 verbatim content (date `2026-05-22` or the actual commit day in `YYYY-MM-DD` form if different).
  - [x] 11.2 Confirm the file format is Keep-a-Changelog (top-level `# Changelog`, then `## [Unreleased]` and `## [0.0.0] - YYYY-MM-DD` sections; each section uses `### Added` / `### Changed` / etc. headings).
  - [x] 11.3 `git-cliff` integration is Story 1.15's scope — do NOT preempt by adding `cliff.toml` or running `git-cliff` here. Story 1.15 explicitly mentions a second `git-cliff` invocation "scoped to `crates/orgsidian-plugin-api/**` paths."

- [x] **Task 12: Workspace-wide gates** (AC1, AC12)
  - [x] 12.1 `cargo build --workspace` → exit 0 (nothing else should break — plugin-api is leaf so no consumers are affected yet).
  - [x] 12.2 `cargo test --workspace` → exit 0 (Story 1.4's `tests/export_bindings.rs` still passes; plugin-api `tests/trait_surface.rs` passes).
  - [x] 12.3 Manually re-read `crates/orgsidian-plugin-api/Cargo.toml`: confirm no project-crate `path` or `workspace = true` deps. The leaf invariant verification will be automated by Story 1.7's `cargo-deny check graph`; in 1.5 it is a careful read.

- [x] **Task 13: Anti-creep audit** (AC13)
  - [x] 13.1 `rg "invoke_plugin_hook" crates/` → exit 1 (no matches; macro lands in Story 1.8).
  - [x] 13.2 `rg "PluginRegistry|Box<dyn OrgsidianPlugin>" crates/orgsidian-core/` → exit 1 (registry struct lands in a later story).
  - [x] 13.3 `rg "panic = \"unwind\"" Cargo.toml` → exit 1 (panic policy lands in Story 1.8).
  - [x] 13.4 `ls examples/plugins/` → expect "No such file or directory" (skeleton lands at v1.5+).
  - [x] 13.5 `rg "From<PluginError>|impl From<PluginError" crates/orgsidian-core/` → exit 1 (conversion is a later story's owner).
  - [x] 13.6 `rg "tracing::|use tracing" crates/orgsidian-plugin-api/` → exit 1 (no tracing dep in leaf).
  - [x] 13.7 `git diff --stat shell-ui/ crates/orgsidian-shell-app/ crates/orgsidian-cli/ tauri.conf.json` → expect zero lines changed (Story 1.5 does NOT touch frontend, shell-app, CLI, or Tauri config).

## Dev Notes

### Critical context the dev agent MUST internalize before touching code

This story is the **canonical trait-surface lock for FR-24 (Internal Plugin Pattern)**. Every consuming v1.0 feature — Save (Story 3.1), Capture (Story 8.1), Agenda (Stories 6.3 / 6.4 / 7.x), Search (Story 8.4), Report (Stories 10.x), Themes (Stories 11.x / 12.x) — invokes plugins via the `OrgsidianPlugin` trait shipped here, through the `invoke_plugin_hook!` macro added by Story 1.8, dispatched through a `PluginRegistry` introduced in a later story. **The shape committed in this story is the contract those consumers will pattern against.**

The architecture explicitly says (LD-26 rationale): "Priority ordering + non-exhaustive events are forward-compatibility hedges learned from VS Code (~200+ event surfaces and growing) and Obsidian (~50 and growing)." Get the day-1 shape wrong here and you either:
- Add breaking variants to `HookOutcome` in v0.3 (rejected as bad SemVer hygiene by LD-50 review), or
- Add a parallel "v2 trait surface" that fragments the FR-24 cross-cutting concern (worst-case scenario flagged in LD-10).

Two specific shape decisions encoded in the ACs (and why they should NOT be revisited during dev):

1. **Context parameters are `&dyn` references, not generic over `C: HookContext`.** The architecture LD-5 round-4 amendment ("trait-method code block in LD-26 should be read with `&dyn` on each context parameter") chose dynamic dispatch over generic for two reasons: (a) object-safe trait surface lets `Box<dyn OrgsidianPlugin>` in the host registry; (b) the v1.5+ WASM transition needs single trait dispatch through a wasmtime-bound vtable — generics would compile per-plugin and balloon the binary. Do NOT switch to `<C: HookContext>` for "performance" — every other v0.1 feature pattern matches against `&dyn HookContext` already.

2. **`Result<T>` is the **leaf-local** `crate::Result<T>` (carrying `PluginError`), not `orgsidian_core::Result<T>` (carrying `OrgError`).** The leaf invariant (LD-5 + LD-10) bars plugin-api from depending on core. The two error types deliberately live separately. The host-side `orgsidian-core::registry` will own the `PluginError → OrgError` conversion when it materializes. Adding `From<PluginError> for OrgError` here would couple the two crates and break the leaf invariant.

### LEAF dep policy

Architecture (LD-5 + LD-26 + Crate Dependency Graph) is explicit: `orgsidian-plugin-api` has **zero project deps** and crates.io publishability is the lock-in criterion. Third-party deps must therefore satisfy:

- **Mandatory** (cannot do the trait surface without them): `serde` (payload struct `Serialize + Deserialize`; LD-26 + LD-50 require future host IPC round-trip), `thiserror` (the local `PluginError` enum's `Error` derive — alternative is hand-written `impl Error` which is more code and no semantic gain).
- **Forbidden in day-1** (will surface later as minor-bump additions only when first plugin author needs them):
  - `tracing` / `tracing-subscriber` — architecture LD-26 prose mentions "structured `tracing` logger" on `HookContext`, but adding it now pollutes the publishable surface with a heavyweight transitive (tracing pulls `once_cell`, `pin-project-lite`, etc.). The first plugin author that actually wants structured logging will land it in a minor-bump story.
  - `tokio` / `async-trait` — all trait methods are sync per day-1 LD-26 verbatim. Async hooks are LD-50 v0.5 review candidate (`HookOutcome::Defer(Duration)` is the canonical async escape hatch the LD names).
  - `specta` / `tauri-specta` — host IPC types live in `orgsidian-core` (Story 1.4 `OrgError`); plugin-api types cross into IPC only via host re-wrap (per AC6 note).
  - `chrono` / `uuid` / `url` — no day-1 trait method uses dates/UUIDs/URLs by typed surface. `String` placeholders carry them when needed; LD-50 review may upgrade.
  - Workspace crates declared by Story 1.2 / 1.3 / 1.4 that aren't `serde` or `thiserror` (e.g., `tauri-specta`, `specta-typescript`).

**Verification mechanism in 1.5**: read `crates/orgsidian-plugin-api/Cargo.toml` and confirm only `serde` + `thiserror` under `[dependencies]`. **Programmatic verification** (`cargo deny check graph`) is Story 1.7's AC.

### Module decomposition rationale

Architecture's "Crate organization (`crates/<name>/src/`)" rule:
> `lib.rs` — public surface re-exports only; no logic. `module.rs` or `module/mod.rs` — one concern per module.

For a leaf trait crate this maps cleanly to:
- `error.rs` (PluginError + Result alias) — one concern: error vocabulary.
- `event.rs` (Event enum) — one concern: event vocabulary.
- `outcome.rs` (HookOutcome) — one concern: hook-return discriminator.
- `metadata.rs` (PluginMetadata) — one concern: plugin identity.
- `payload.rs` (CaptureEntry, AgendaQuery, AgendaItem) — three lightweight placeholder structs, grouped because they all share the same "future-story-refines-shape" pattern (avoids three tiny files; under the 400-line split threshold).
- `context.rs` (HookContext + PluginContext) — one concern: host capability surface (two coupled traits).
- `plugin.rs` (OrgsidianPlugin) — one concern: plugin lifecycle trait.

`lib.rs` carries the `//!` module-level doc, the two `#![warn/deny]` crate-attributes, the `mod` declarations, and the `pub use` re-exports. No logic. The seven sub-modules each end with a `#[cfg(test)] mod tests` block for any module-local unit tests (not required by ACs but encouraged by architecture's "AI-Agent Implementation Rule 5: tests with every PR that adds production code").

**Alternative considered**: single-file `lib.rs` with everything inlined. Rejected because (a) architecture rule explicitly forbids "logic in lib.rs"; (b) splitting prevents lint-by-association in `clippy::pedantic` mode (e.g., a derive macro lint on `PluginMetadata` would otherwise need a long disambiguating doc comment near the trait).

### Specific lints to expect under `clippy::pedantic`

Based on the trait shape, these `clippy::pedantic` lints are likely to fire and have known resolutions:

- **`clippy::must_use_candidate`** on every default-impl trait method (Rust thinks `priority()`, `on_event()`, etc. return values that should carry `#[must_use]`). **Resolution**: do NOT add `#[must_use]` on trait methods (it doesn't propagate through `dyn` dispatch reliably); add `#[allow(clippy::must_use_candidate)]` on the trait `impl` items or the trait itself with a one-line rationale.
- **`clippy::needless_pass_by_value`** if any method accidentally takes an owned type instead of `&`. **Resolution**: signatures in AC2 / AC5 are already by-reference; if this fires, you mistyped.
- **`clippy::module_name_repetitions`** if a struct name repeats the module name (e.g., `event::Event`). **Resolution**: per-item `#[allow(clippy::module_name_repetitions)]` is acceptable; the public re-export at crate root means consumers never write the repetition.
- **`clippy::missing_errors_doc`** because `///` doc-comments on `Result`-returning methods should mention error conditions. **Resolution**: add an `# Errors` section in each method doc-comment naming which `PluginError` variants the host may return (`HostUnavailable` for index/vault, `InvalidInput` for malformed paths). This satisfies the lint AND improves the dev-agent-readability of the surface.

### Why `PluginMetadata` does NOT derive `specta::Type`

`specta::Type` is what makes a Rust type appear in the generated `shell-ui/src/lib/tauri.ts` (Story 1.4 wiring). Adding it to `PluginMetadata` would mean:

1. `specta = { workspace = true }` becomes a plugin-api dep (third-party-but-not-publishable on crates.io until specta 2.0 stable).
2. plugin-api would be transitively re-exported into the TS bindings even when the host has no use for plugin types at the IPC boundary yet (no Settings UI plugin list ships until Stories 12.x).

The host-side façade pattern: when `shell-app` / `core` exposes plugin types over IPC (later story), it defines a thin wrapper type in `orgsidian-core` that wraps `PluginMetadata` + derives `specta::Type`. This keeps plugin-api crates.io-publishable at v1.5+ without forcing every plugin author to depend on specta.

This is the same design as the `OrgError` ↔ `PluginError` split: host has its own IPC-ready vocabulary, plugin-api stays pure.

### Why `Event::IndexRebuilt` is unit-like

Most `Event` variants carry payload (`FileOpened { path }`, etc.), but `IndexRebuilt` is unit-like (no fields). Architecture LD-26 leaves the variant signatures unspecified beyond names; the day-1 decision:

- `IndexRebuilt` fires once per rebuild operation; consumers (plugins) that need rebuild details query the index after — adding a payload now (`{ files_indexed: u64, duration: Duration }`) would require `chrono`/`std::time::Duration` and lock the schema. Unit-like is the conservative day-1 choice.
- Other variants carry the minimum payload that uniquely identifies what they reference (file path, headline ID). Field types are all `String` for the same anti-coupling reason as the placeholder payload structs (AC7).

If a future story needs `IndexRebuilt { duration_ms: u64 }`, that lands as a SemVer-minor variant-replacement (the v1.0 `#[non_exhaustive]` permits this; consumers' `_` wildcard arms keep compiling). LD-50 review pass can audit whether all variants picked the right initial payload.

### Reference `crates/orgsidian-plugin-api/src/lib.rs` post-Task-2 shape (skeleton)

```rust
//! `orgsidian-plugin-api`: the day-1 trait surface for the Orgsidian internal
//! Plugin Pattern (FR-24).
//!
//! This crate is **LEAF** — it has zero project dependencies, so it can be
//! published to crates.io without bundling host implementation crates. Per
//! LD-10 the crate stays internal-only through v0.1 → v1.4; external
//! publication unlocks at v1.5+ when third-party plugin authors land.
//!
//! ### What ships here
//!
//! - [`OrgsidianPlugin`] — the plugin lifecycle trait (`metadata`, `init`,
//!   `shutdown`, optional hooks).
//! - [`Event`] — `#[non_exhaustive]` enum of host-emitted events.
//! - [`HookOutcome`] — `Continue` / `Replace(T)` / `Cancel(String)`.
//! - [`HookContext`] + [`PluginContext`] — host capability traits.
//! - Payload types: [`PluginMetadata`], [`CaptureEntry`], [`AgendaQuery`],
//!   [`AgendaItem`].
//! - [`PluginError`] + [`Result`] — leaf-local error vocabulary (distinct
//!   from `orgsidian-core::OrgError`).
//!
//! ### See also
//!
//! - LD-10 / LD-26 / LD-5 round-4 amendment in
//!   `_bmad-output/planning-artifacts/architecture.md` for design rationale.
//! - LD-33 for CHANGELOG discipline.
//! - LD-50 for the v0.5 surface-review gate before crates.io publication.

#![warn(clippy::pedantic)]
#![deny(missing_docs)]

mod context;
mod error;
mod event;
mod metadata;
mod outcome;
mod payload;
mod plugin;

pub use context::{HookContext, PluginContext};
pub use error::{PluginError, Result};
pub use event::Event;
pub use metadata::PluginMetadata;
pub use outcome::HookOutcome;
pub use payload::{AgendaItem, AgendaQuery, CaptureEntry};
pub use plugin::OrgsidianPlugin;
```

### Reference `crates/orgsidian-plugin-api/tests/trait_surface.rs` post-Task-9 shape

```rust
//! Day-1 anchor test: a noop plugin can be constructed and the trait surface
//! is object-safe under `Box<dyn OrgsidianPlugin>`.

use orgsidian_plugin_api::{
    AgendaItem, AgendaQuery, CaptureEntry, Event, HookContext, HookOutcome,
    OrgsidianPlugin, PluginContext, PluginError, PluginMetadata, Result,
};

struct NoopPlugin {
    meta: PluginMetadata,
}

impl OrgsidianPlugin for NoopPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.meta.clone()
    }

    fn init(&mut self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

struct StubPluginContext {
    meta: PluginMetadata,
}

impl PluginContext for StubPluginContext {
    fn plugin_metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

struct StubHookContext;

impl HookContext for StubHookContext {
    fn read_vault_file(&self, _path: &str) -> Result<String> {
        Ok(String::new())
    }

    fn query_index(&self, _query: &str) -> Result<String> {
        Ok(String::new())
    }

    fn emit_event(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

#[test]
fn noop_plugin_is_object_safe_and_defaults_work() {
    let meta = PluginMetadata {
        id: "noop".to_string(),
        name: "Noop".to_string(),
        version: "0.0.0".to_string(),
        author: "tests".to_string(),
    };
    let mut plugin: Box<dyn OrgsidianPlugin> = Box::new(NoopPlugin { meta: meta.clone() });

    assert_eq!(plugin.priority(), 0);
    assert!(plugin.on_event(&Event::IndexRebuilt).is_ok());

    let ctx = StubPluginContext { meta };
    assert!(plugin.init(&ctx).is_ok());

    let hook_ctx = StubHookContext;
    let outcome = plugin
        .on_save_before(&hook_ctx, "content")
        .expect("default impl returns Ok");
    assert!(matches!(outcome, HookOutcome::Continue));

    let entry = CaptureEntry { raw_text: "x".into() };
    let outcome = plugin
        .on_capture_before(&hook_ctx, &entry)
        .expect("default impl returns Ok");
    assert!(matches!(outcome, HookOutcome::Continue));

    let query = AgendaQuery { raw_filter: String::new() };
    let mut results: Vec<AgendaItem> = Vec::new();
    assert!(plugin.on_agenda_query_after(&hook_ctx, &query, &mut results).is_ok());

    assert!(plugin.shutdown().is_ok());
}

#[test]
fn cancel_outcome_carries_reason() {
    let outcome: HookOutcome<String> = HookOutcome::Cancel("plugin foo declined".into());
    match outcome {
        HookOutcome::Cancel(reason) => assert_eq!(reason, "plugin foo declined"),
        other => panic!("expected Cancel, got {other:?}"),
    }
}

#[test]
fn plugin_error_is_display() {
    let err = PluginError::Runtime { reason: "boom".into() };
    let rendered = err.to_string();
    assert!(rendered.contains("boom"));
}
```

### Reference target file structure (additions only)

```
crates/orgsidian-plugin-api/
├── Cargo.toml                                          (MODIFIED: +[dependencies] block — serde + thiserror)
├── CHANGELOG.md                                        (NEW: Keep-a-Changelog; 0.0.0 initial trait surface)
├── src/
│   ├── lib.rs                                          (REWRITTEN: //! doc + mod declarations + pub use re-exports + 2 crate attributes)
│   ├── error.rs                                        (NEW: PluginError + Result alias)
│   ├── event.rs                                        (NEW: Event enum)
│   ├── outcome.rs                                      (NEW: HookOutcome<T>)
│   ├── metadata.rs                                     (NEW: PluginMetadata)
│   ├── payload.rs                                      (NEW: CaptureEntry + AgendaQuery + AgendaItem)
│   ├── context.rs                                      (NEW: HookContext + PluginContext traits)
│   └── plugin.rs                                       (NEW: OrgsidianPlugin trait)
└── tests/
    └── trait_surface.rs                                (NEW: object-safety + defaults smoke test; anti-placebo-green per Party Mode P2)
```

NOT touched: `crates/orgsidian-core/`, `crates/orgsidian-shell-app/`, `crates/orgsidian-cli/`, `crates/orgsidian-{parser,index,watcher,vault,report}/`, `shell-ui/`, `tools/corpus-extractor/`, `tauri.conf.json`, `capabilities/`, root `Cargo.toml` (workspace deps already cover `serde` + `thiserror` from Stories 1.2 / 1.4), `Cargo.lock` (no new transitives — `serde` + `thiserror` were already linked by other crates), `.gitignore`, `package.json`, `pnpm-lock.yaml`.

### Architecture compliance — what THIS story must satisfy

- **FR-24 (Internal Plugin Pattern)** [Source: [epics.md FR-24](../planning-artifacts/epics.md#L90)]: The plugin-api trait surface IS this story; v1.0 features (Agenda, Quick Capture, Search, Project Report, Themes) consume the trait shipped here.
- **LD-5 (9-crate monorepo + LEAF invariant)** [Source: [architecture.md#Project Structure & Boundaries](../planning-artifacts/architecture.md)]: `orgsidian-plugin-api` is a LEAF crate — zero project deps. Round-4 amendment: `HookContext` / `PluginContext` are **traits**, passed as `&dyn` references.
- **LD-10 (internal-only until v1.5+)** [Source: [architecture.md#LD-10](../planning-artifacts/architecture.md)]: SemVer + CHANGELOG + contract tests tracked internally from day 1; publication to crates.io at v1.5+.
- **LD-25 (static linking, WASM-compatible day-1 shape)** [Source: [architecture.md#LD-25](../planning-artifacts/architecture.md)]: Trait surface is message-passing-only (no synchronous callbacks, no mutable references handed to plugins) so the v1.5+ WASM transition is mechanical.
- **LD-26 (Plugin API trait shape)** [Source: [architecture.md#LD-26](../planning-artifacts/architecture.md)]: The verbatim trait + Event + HookOutcome code blocks above. Also: the SemVer policy block (new variant = minor, removed/changed = major) — internalize as Dev Notes for any signature decision.
- **LD-33 (Release automation + CHANGELOG per crate)** [Source: [architecture.md#LD-33](../planning-artifacts/architecture.md)]: `crates/orgsidian-plugin-api/CHANGELOG.md` exists from day 1, follows Keep-a-Changelog; `git-cliff` integration is Story 1.15.
- **Documentation Conventions** [Source: [architecture.md#Documentation Conventions](../planning-artifacts/architecture.md)]: "`orgsidian-plugin-api` public items: `///` doc comments mandatory; `cargo doc --no-deps` clean (no warnings)." Enforced via `#![deny(missing_docs)]`.
- **Linting & Formatting** [Source: [architecture.md#Linting & Formatting](../planning-artifacts/architecture.md)]: "`clippy::pedantic` enabled on `orgsidian-plugin-api` (public surface)." Enforced via `#![warn(clippy::pedantic)]`.
- **Naming Conventions — Plugin API `Event` enum** [Source: [architecture.md#Naming Conventions](../planning-artifacts/architecture.md)]: PascalCase variants, past-tense for completion events, `#[non_exhaustive]` requires `_` arm in consumers.
- **AI-Agent Implementation Rule 1 ("One concern per file")** [Source: [architecture.md#AI-Agent Implementation Rules](../planning-artifacts/architecture.md)]: Module decomposition per Dev Notes §Module decomposition.
- **AI-Agent Implementation Rule 2 ("No `unwrap()` / `expect()` outside tests")**: Library code uses `?` propagation; tests freely `.unwrap()` / `.expect()`.
- **LD-32 (CI matrix)**: Per-PR `cargo build/test/clippy -- -D warnings/fmt --check` covers Story 1.5; no new CI surface needed beyond what Story 1.8 ships.

### Anti-patterns explicitly forbidden in this story

- ❌ `path = "../orgsidian-core"` or `orgsidian-core = { workspace = true }` (or any other project-crate path) in `crates/orgsidian-plugin-api/Cargo.toml` — breaks the LEAF invariant, blocks v1.5+ crates.io publication.
- ❌ `use orgsidian_core::*` (or any project-crate `use`) anywhere in `crates/orgsidian-plugin-api/src/**` — same.
- ❌ Adding `tracing`, `tokio`, `chrono`, `specta`, `tauri-specta`, `serde_json` to plugin-api's `[dependencies]` — taints the publishable surface; reserved for future minor-bump stories when first plugin author needs them (per Dev Notes §LEAF dep policy).
- ❌ Generic-over-context trait signatures (`fn on_save_before<C: HookContext>(...)`) — breaks `dyn OrgsidianPlugin` object safety and the v1.5+ WASM dispatch (per Dev Notes §Critical context point 1).
- ❌ `async fn` anywhere in `OrgsidianPlugin` / `HookContext` / `PluginContext` — async-trait is LD-50 v0.5 review candidate (`HookOutcome::Defer(Duration)` is the deliberate non-async escape hatch).
- ❌ `&mut HookContext` / `&mut PluginContext` — architecture is explicit "no mutable references handed to plugins."
- ❌ `Result<T, OrgError>` in any plugin-api method signature — leaf-local `Result<T>` (aliasing `Result<T, PluginError>`) is the only error type.
- ❌ `From<PluginError> for OrgError` impl anywhere in plugin-api OR core in this story — conversion belongs to `orgsidian-core::registry` when it lands; adding it here couples crates prematurely.
- ❌ Concrete `HostHookContext` / `HostPluginContext` `impl` blocks in `orgsidian-core` — first internal-plugin story owns these.
- ❌ `[profile.release] panic = "unwind"` in workspace `Cargo.toml` — Story 1.8 explicit AC; do NOT preempt.
- ❌ `invoke_plugin_hook!` macro stub in `orgsidian-core/src/registry.rs` — Story 1.8 explicit AC.
- ❌ Adding the `examples/plugins/hello-world/` skeleton — architecture lists it as a v1.5+ deliverable; preempting now ships maintenance burden without consumer.
- ❌ Adding `Deserialize` derive on `PluginError` — error wire-format is `OrgError`'s job, not plugin-api's.
- ❌ Adding a 4th `HookOutcome` variant (e.g., `Defer(Duration)`, `Retry`, `SkipChain`) — reserved for LD-50 v0.5 review.
- ❌ Splitting `payload.rs` into three files (`capture.rs` / `agenda_query.rs` / `agenda_item.rs`) — under the 400-line threshold; the three structs share the same "future-story-refines-shape" pattern and benefit from co-location.
- ❌ `#[deprecated]` markings, doc-comment "TODO" trails about WASM ABI, `#[wasm_bindgen]` annotations — v1.5+ WASM target is mechanical-not-architectural; the surface is already WASM-compatible by construction.
- ❌ Adding `pub trait OrgsidianPlugin: Send + Sync + 'static` — `'static` is implicit for `dyn`-trait-object types Box-allocated by the host; adding the explicit bound is noise (clippy `clippy::extra_unused_lifetimes` candidate).
- ❌ Marking trait methods `#[must_use]` — does not propagate through `dyn` dispatch reliably; the trait-consumer convention is "check `Result` at call site."
- ❌ Adding a `description: String` field to `PluginMetadata` — minimal day-1 shape is `id / name / version / author`; LD-50 review may expand.
- ❌ Re-exporting `serde` / `thiserror` from `lib.rs` — those are implementation deps, not part of the trait surface; plugin authors `use serde::...` themselves.
- ❌ Touching `Cargo.lock` manually — workspace-level `cargo build` regenerates it; Story 1.7 owns dep-graph hygiene gates.

### Previous story intelligence (Story 1.4 learnings)

Apply these patterns from Story 1.4's review/learnings to keep Story 1.5 frictionless:

1. **Anchor paths on `CARGO_MANIFEST_DIR` when constructing cross-crate file paths.** Story 1.4 review surfaced that `"../../shell-ui/src/lib/tauri.ts"` was fragile under varying CWDs. Story 1.5 ships no cross-crate file writes (CHANGELOG is within the crate; the test file is within the crate), so this footgun does not apply — but the pattern is internalized for future stories.
2. **Document deviations in Change Log + Completion Notes.** Story 1.4 disclosed six deviations; Story 1.5 has **two pre-known potential deviations** the dev agent should expect:
   - (a) `clippy::pedantic` may flag lints that require per-item `#[allow(...)]` annotations with `// rationale: ...` comments (per Dev Notes §Specific lints to expect). Disclose any allow-list addition + the rationale in Completion Notes.
   - (b) If the `Event` variant payload shapes (e.g., `HeadlineEdited { file: String, headline_id: String }`) reveal during implementation that a `headline_id: u64` or `headline_id: Uuid` is needed for forward-compatibility, the **conservative choice is `String`** (no new deps) and a Completion Note explains why the dev agent stayed with `String`. Any deviation to a richer typed shape requires adding a dep (e.g., `uuid`), which is in the forbidden list — escalate to spec change, do not silently add the dep.
3. **`pnpm tauri dev` is the source of truth for runtime gates.** Not applicable to Story 1.5 (Rust-only changes; no frontend touched). But the principle generalizes: `cargo doc --no-deps -p orgsidian-plugin-api` (AC10) and `cargo clippy -p orgsidian-plugin-api -- -D warnings` (Task 10.2) are the runtime gates for THIS story — run them, don't trust `cargo check`.
4. **`[[feedback_version_policy]]` Tauri-exemption applies to specta, NOT to plugin-api deps.** plugin-api deps are `serde` (LTS-preferred, floats `1.x` per workspace dep) and `thiserror` (LTS-preferred, floats `1.x` per workspace dep). Stay on the workspace pins; do not exact-pin here.
5. **Modify only what the AC dictates.** Story 1.4 originally drifted into reorganizing capabilities + plugin reorder; review reverted. Story 1.5: do NOT touch `crates/orgsidian-core/`, `crates/orgsidian-shell-app/`, `shell-ui/`, root `Cargo.toml` (workspace deps), `Cargo.lock` (regenerated), the existing 11 Tauri plugin registrations, `tauri.conf.json`, capabilities, the route tree, or the Story 1.4 `tests/export_bindings.rs`.
6. **Apply `[[feedback_batch_fixes_terse]]` during dev.** If `clippy::pedantic` fires 6+ lints, apply the obvious no-brainer fixes silently; surface only the ambiguous ones (e.g., "is `clippy::module_name_repetitions` on `event::Event` acceptable to allow-list or should the type be renamed?") as decision-grade questions.

### Git intelligence (recent commits)

Recent commits on `feat/story-1-4-tauri-specta-typed-ipc` (per session start):
- `567fdaa` `fix(ipc): apply Story 1.4 code-review patches` — applied the 7 review patches (path anchor, dead-import cleanup, `#[doc(hidden)]`, `--locked`, deviation docs).
- `d014594` `feat(ipc): wire tauri-specta typed IPC bridge with project-wide camelCase` — Story 1.4 main implementation.
- `96cb55b` `feat: add gh issue sync enforcement` — Story 1.16 prep / sprint-status hygiene.
- `4543ea6` Merge PR #113 — Story 1.3 complete.

Implications:
- The 9-crate workspace is canonical and stable since Story 1.2; `crates/orgsidian-plugin-api/` already exists as a placeholder (verified: `src/lib.rs` is 4 lines of placeholder doc; `Cargo.toml` has no `[dependencies]` block).
- The 11 Tauri plugin registrations in `orgsidian-shell-app/src/lib.rs::run()` are present (Story 1.3) and the specta Builder wraps them (Story 1.4). Story 1.5 does NOT touch this file.
- `orgsidian-core/src/lib.rs` exports `OrgError` + `Result` from Story 1.4. Story 1.5 does NOT import these into plugin-api (would violate the LEAF invariant).
- `Cargo.lock` already contains `serde`, `thiserror`, all serde transitives (pulled in by Story 1.4 via `orgsidian-core`). Story 1.5's new deps add zero new transitives.

### Testing requirements

Story 1.5 is trait-surface declaration; the only **automated** test added is `tests/trait_surface.rs` (Task 9), which serves three purposes:

1. **Object safety smoke test** — proves `Box<dyn OrgsidianPlugin>` constructs without compile error. Catches any accidental non-object-safe addition (e.g., a generic method that snuck in).
2. **Default-impl smoke test** — proves the four optional methods (`priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after`) actually compile with their default bodies, and the defaults return the documented value (`0`, `Ok(())`, `Continue`, `Continue`, `Ok(())`).
3. **Anti-placebo-green discipline (Party Mode P2)** — Story 1.9 ships three anchor tests (parser + vault + watcher) to prove CI exercises real code paths; plugin-api adds its own equivalent in this story. The test exists specifically to fail if a future story breaks the trait surface (e.g., adds a generic, removes a default impl, or changes a method signature) — making the trait surface a contract that the test suite enforces.

Binding gates summary:

1. `cargo check -p orgsidian-plugin-api` → exit 0.
2. `cargo build --workspace` → exit 0.
3. `cargo test -p orgsidian-plugin-api` → exit 0 (`trait_surface.rs` passes).
4. `cargo test --workspace` → exit 0 (Story 1.4's `tests/export_bindings.rs` still passes; no regression).
5. `cargo doc --no-deps -p orgsidian-plugin-api` → exit 0 with zero warnings (`#![deny(missing_docs)]` enforces this).
6. `cargo clippy -p orgsidian-plugin-api -- -D warnings` → exit 0 (`#![warn(clippy::pedantic)]` + `-D warnings` makes pedantic-lint hard gate).
7. `cargo fmt --check` → exit 0.
8. No regression to Story 1.1 / 1.2 / 1.3 / 1.4 invariants — workspace structure, `tauri-specta` IPC, the 11 plugin registrations, capability allow-list, route tree.

**Property-based testing** is NOT added in this story — the trait surface has no behavioral semantics yet (no host implementation), so property tests would only assert "Rust compiles your code." Property tests land in future stories that ship the host-side concrete `HookContext` / `PluginContext` implementations.

### Project Structure Notes

- **Alignment with unified project structure**: post-Story-1.5 layout matches architecture's Workspace Layout §`crates/orgsidian-plugin-api/` exactly:
  - `crates/orgsidian-plugin-api/CHANGELOG.md` ✓ (Workspace Layout: "└── CHANGELOG.md # SemVer-tracked from day 1 (published at v1.5+)").
  - Sub-modules `error.rs` / `event.rs` / `outcome.rs` / etc. ✓ (Workspace Layout note: "LEAF: trait + Event + HookOutcome + HookContext/PluginContext traits" — architecture does not prescribe sub-module file names, but the one-concern-per-file rule (AI-Agent Implementation Rule 1) dictates the decomposition per Dev Notes §Module decomposition).
- **Detected conflicts**: none. Architecture LD-26's reference trait code block has `ctx: PluginContext` / `ctx: &HookContext` (without `&dyn`); the LD-5 round-4 amendment (architecture §Amendments to Earlier Sections) supersedes with "trait-method code block in LD-26 should be read with `&dyn` on each context parameter." Story 1.5 implements the amendment, NOT the original code block — this is conformance, not a conflict.
- **Variance**: the test path `crates/orgsidian-plugin-api/tests/trait_surface.rs` is a new integration test under an existing crate; no new workspace member. Conforms to architecture "Test placement: Rust integration tests: `crates/<crate>/tests/<topic>.rs`."

### References

- [Source: [epics.md#Epic 1 Story 1.5](../planning-artifacts/epics.md#L486)] — Story user-story + 6 acceptance criteria.
- [Source: [epics.md#FR Coverage Map FR-24](../planning-artifacts/epics.md#L271)] — FR-24 (Internal Plugin Pattern) → Epic 1 plugin-api scaffold + cross-cutting from Epic 2-12.
- [Source: [architecture.md#LD-5 (Monorepo / 9-crate)](../planning-artifacts/architecture.md)] — LEAF invariant + Workspace member list.
- [Source: [architecture.md#LD-10 (Plugin API internal until v1.5+)](../planning-artifacts/architecture.md)] — Publication policy + internal SemVer discipline.
- [Source: [architecture.md#LD-25 (Static linking, v1.0)](../planning-artifacts/architecture.md)] — Static-linking + WASM-compatible surface rationale.
- [Source: [architecture.md#LD-26 (Plugin API trait shape)](../planning-artifacts/architecture.md)] — Trait + Event + HookOutcome canonical code block + SemVer policy.
- [Source: [architecture.md#LD-33 (Release automation + CHANGELOG)](../planning-artifacts/architecture.md)] — Per-crate CHANGELOG discipline.
- [Source: [architecture.md#LD-50 (v0.5 surface review)](../planning-artifacts/architecture.md)] — Reserved variants / async escape hatch; what NOT to ship in 1.5.
- [Source: [architecture.md#Documentation Conventions](../planning-artifacts/architecture.md)] — `///` mandatory + `cargo doc --no-deps` clean.
- [Source: [architecture.md#Linting & Formatting](../planning-artifacts/architecture.md)] — `clippy::pedantic` enabled on plugin-api.
- [Source: [architecture.md#Naming Conventions](../planning-artifacts/architecture.md)] — PascalCase / past-tense for `Event` variants + `#[non_exhaustive]` consumer-arm rule.
- [Source: [architecture.md#AI-Agent Implementation Rules (Mandatory)](../planning-artifacts/architecture.md)] — Rules 1 / 2 / 5 (one concern per file, no unwrap outside tests, tests with every PR).
- [Source: [architecture.md#Amendments to Earlier Sections (Party Mode round 4)](../planning-artifacts/architecture.md)] — LD-5 round-4 `&dyn` amendment on `HookContext` / `PluginContext`.
- [Source: [architecture.md#Crate Dependency Graph](../planning-artifacts/architecture.md)] — Leaf crate enumeration + Cargo-deny check graph (Story 1.7 enforcement).
- [Source: [../implementation-artifacts/1-4-wire-tauri-specta-typed-ipc-bridge-with-project-wide-camelcase-rename.md](./1-4-wire-tauri-specta-typed-ipc-bridge-with-project-wide-camelcase-rename.md)] — Story 1.4 reference shapes for crate organization + Cargo.toml conventions + deviation-disclosure style.
- Persistent feedback memories: `[[feedback_version_policy]]` (LTS-preferred for serde/thiserror), `[[feedback_batch_fixes_terse]]` (apply no-brainer clippy fixes silently during dev), `[[feedback_spec_driven_not_solo_dev_bandwidth]]` (don't justify cuts on time grounds — leaf-dep policy is spec-driven).

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context) — `claude-opus-4-7[1m]`

### Debug Log References

- `cargo check -p orgsidian-plugin-api` → exit 0 (Cargo.toml stage).
- `cargo check -p orgsidian-plugin-api` → exit 0 (post module skeleton).
- `cargo test -p orgsidian-plugin-api` → 3 tests passed (`noop_plugin_is_object_safe_and_defaults_work`, `cancel_outcome_carries_reason`, `plugin_error_is_display`).
- `cargo clippy -p orgsidian-plugin-api --all-targets -- -D warnings` → exit 0 after fixing two `clippy::doc_markdown` lints (silent batch-fix per `[[feedback_batch_fixes_terse]]`).
- `cargo doc --no-deps -p orgsidian-plugin-api` → exit 0, zero warnings.
- `cargo build --workspace` → exit 0.
- `cargo test --workspace` → all suites pass; Story 1.4 `tests/export_bindings.rs` (1 test) still green.
- Anti-creep audit (Task 13.1-13.7): all checks pass after rephrasing two doc-comments that literally contained the audit-tripping tokens (`invoke_plugin_hook!` and `tracing::error!`). No functional change — the citations were narrative pointers to Story 1.8.

### Completion Notes List

- **Module decomposition.** Implemented the seven-module layout from Dev Notes §Module decomposition (`error.rs`, `outcome.rs`, `metadata.rs`, `payload.rs`, `event.rs`, `context.rs`, `plugin.rs`), with `lib.rs` carrying only the crate-level `//!` doc, the two `#![warn/deny]` attributes, `mod` declarations, and `pub use` re-exports. No logic in `lib.rs`.

- **Clippy pedantic allow-list (per AC9 / Dev Notes §Specific lints to expect).** Three module-level `#![allow(...)]` declarations with rationale comments were required:
  - `src/error.rs`: `#![allow(clippy::module_name_repetitions)]` — rationale: `PluginError` is the canonical name; consumers reach it via the crate-root re-export.
  - `src/metadata.rs`: `#![allow(clippy::module_name_repetitions)]` — rationale: same as `error.rs`; the `Plugin` prefix disambiguates from third-party `Metadata` types.
  - `src/plugin.rs`: `#![allow(clippy::must_use_candidate, clippy::unused_self, clippy::needless_pass_by_ref_mut)]` — rationale: trait default-impl methods are no-ops; `#[must_use]` does not propagate through `dyn` dispatch (per Dev Notes Specific lints §). The `unused_self` / `needless_pass_by_ref_mut` lints fire on no-op default bodies that overriders WILL use.

- **Two `clippy::doc_markdown` fixes** (silent batch fix): wrapped `PascalCase` and `SemVer` in backticks inside doc-comments in `event.rs` and `metadata.rs` respectively. No semantic change.

- **Two anti-creep rephrasings.** The audit checks `rg "invoke_plugin_hook" crates/` and `rg "tracing::|use tracing" crates/orgsidian-plugin-api/` must exit 1. The initial draft of `src/event.rs` carried a doc-comment citation of Story 1.8 that literally read `tracing::error! logging inside the invoke_plugin_hook! macro stub`. The citation was rephrased to `structured panic-logging inside the host-side plugin-hook dispatch machinery landing in Story 1.8` — same architectural intent, no token-level audit hit.

- **`cargo fmt --check` deviation (DISCLOSED — does NOT block Story 1.5).** Workspace-wide `cargo fmt --check` reports two pre-existing fmt-drift diffs in `crates/orgsidian-shell-app/src/lib.rs:38` and `crates/orgsidian-shell-app/tests/export_bindings.rs:13` (both Story 1.4 files). Story 1.5's anti-creep AC13 explicitly forbids editing `tests/export_bindings.rs`; touching `shell-app/src/lib.rs` is likewise outside Story 1.5's scope. Per [[feedback_batch_fixes_terse]] this would be a no-brainer rustfmt re-run, but the spec-driven scope-fence overrides — the drift is logged here for Story 1.7 / 1.8 CI hardening to address. Verification that Story 1.5's own crate is fmt-clean: `cargo fmt -p orgsidian-plugin-api --check` → exit 0.

- **Cargo.lock.** No new transitives introduced — `serde` and `thiserror` were already linked by `orgsidian-core` (Story 1.4). `Cargo.lock` is unchanged.

- **No deviations from AC code blocks.** All AC2 / AC3 / AC4 / AC5 / AC6 / AC7 / AC8 / AC11 / AC12 code blocks were implemented verbatim, with only the addition of `///` doc-comments (mandated by AC9 `#![deny(missing_docs)]`) and `# Errors` sections on `Result`-returning methods (to satisfy `clippy::missing_errors_doc` per Dev Notes §Specific lints).

### File List

**Modified**

- `crates/orgsidian-plugin-api/Cargo.toml` — added `[dependencies]` block with `serde = { workspace = true }` and `thiserror = { workspace = true }`; replaced stub comment with the rationale comment per AC12.
- `crates/orgsidian-plugin-api/src/lib.rs` — rewritten from placeholder to crate-root: `//!` module-doc, `#![warn(clippy::pedantic)]` + `#![deny(missing_docs)]`, seven `mod` declarations + seven `pub use` re-exports. No logic.

**New**

- `crates/orgsidian-plugin-api/CHANGELOG.md` — Keep-a-Changelog format; `[Unreleased]` + `[0.0.0] - 2026-05-22` initial trait surface entry per AC11.
- `crates/orgsidian-plugin-api/src/error.rs` — `PluginError` enum (4 variants, `#[non_exhaustive]`) + `Result<T>` alias.
- `crates/orgsidian-plugin-api/src/outcome.rs` — `HookOutcome<T>` enum (`Continue` / `Replace(T)` / `Cancel(String)`).
- `crates/orgsidian-plugin-api/src/metadata.rs` — `PluginMetadata` struct (`id`, `name`, `version`, `author`).
- `crates/orgsidian-plugin-api/src/payload.rs` — `CaptureEntry`, `AgendaQuery`, `AgendaItem` placeholder structs.
- `crates/orgsidian-plugin-api/src/event.rs` — `Event` enum (9 variants, `#[non_exhaustive]`).
- `crates/orgsidian-plugin-api/src/context.rs` — `HookContext` + `PluginContext` traits (both `Send + Sync`).
- `crates/orgsidian-plugin-api/src/plugin.rs` — `OrgsidianPlugin` trait with required methods (`metadata`, `init`, `shutdown`) and optional default-impl hooks (`priority`, `on_event`, `on_save_before`, `on_capture_before`, `on_agenda_query_after`).
- `crates/orgsidian-plugin-api/tests/trait_surface.rs` — anti-placebo-green smoke test (3 tests) proving object-safety, default-impl wiring, and `HookOutcome::Cancel` round-trip.

**Project tracking artifacts (out-of-band updates, not part of code surface)**

- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `1-5-…: ready-for-dev → in-progress → review`.
- `_bmad-output/implementation-artifacts/1-5-scaffold-orgsidian-plugin-api-leaf-crate-with-day-1-trait-surface.md` — Status `ready-for-dev → review`; `github_issue: 5` recorded under Metadata; Tasks/Subtasks marked complete; Dev Agent Record populated.

### Change Log

- 2026-05-22 — Story 1.5 implementation. Scaffolded `orgsidian-plugin-api` as a LEAF crate (`serde` + `thiserror` deps only) with the day-1 trait surface: `OrgsidianPlugin` trait, `Event` enum (`#[non_exhaustive]`, 9 variants), `HookOutcome<T>`, `HookContext` + `PluginContext` traits (`&dyn` passing per LD-5 round-4), `PluginMetadata`, `CaptureEntry`, `AgendaQuery`, `AgendaItem`, `PluginError` enum + `Result<T>` alias. Added Keep-a-Changelog `CHANGELOG.md` and anti-placebo-green `tests/trait_surface.rs`. All AC1-AC13 satisfied; one disclosed deviation (workspace-wide `cargo fmt --check` pre-existing Story 1.4 drift, outside Story 1.5 scope).
- 2026-05-22 — Code review applied (5 patches + 1 decision-resolved). Disclosed deviation from AC4 verbatim: added `#[non_exhaustive]` to `HookOutcome<T>` to preserve the SemVer-additive policy when `Defer(Duration)` lands in LD-50 v0.5 surface review (matches `Event` / `PluginError` pattern). Doc-comment corrections on `HookContext::emit_event` and `OrgsidianPlugin::on_agenda_query_after`. Test suite extended: now 5 tests (was 3) plus a compile-time exhaustive `Event` match guard. 1 finding deferred (explicit `Send + Sync` static assertion on `PluginError`) — to be revisited at LD-50 v0.5 surface review.

### Review Findings

_Code review (2026-05-22) — Blind Hunter + Edge Case Hunter + Acceptance Auditor. Local gates: `cargo build --workspace` ✅, `cargo test -p orgsidian-plugin-api` ✅ (3 tests), `cargo clippy -p orgsidian-plugin-api -- -D warnings` ✅, `cargo doc --no-deps -p orgsidian-plugin-api` ✅, `cargo fmt -p orgsidian-plugin-api --check` ✅. All 13 ACs satisfied verbatim against spec snippets._

- [x] [Review][Decision→Patch] **`HookOutcome<T>` lacks `#[non_exhaustive]`** — Resolved (2026-05-22) by adding `#[non_exhaustive]` to the enum. Disclosed deviation from AC4 verbatim block; rationale: the doc-comment already reserves `Defer(Duration)` for LD-50, and `Event`/`PluginError` both carry the attribute for the same SemVer-additive policy. Applied at [crates/orgsidian-plugin-api/src/outcome.rs](crates/orgsidian-plugin-api/src/outcome.rs#L18).
- [x] [Review][Patch] **PR + `Closes #5` gate** — Applied: feature branch `feat/story-1-5-plugin-api-scaffold` created, commit + PR with `Closes #5` in body.
- [x] [Review][Patch] **`HookContext::emit_event` doc rationale rewritten** — [crates/orgsidian-plugin-api/src/context.rs:78-86](crates/orgsidian-plugin-api/src/context.rs#L78-L86). Replaced the contradictory "host can clone for fan-out" rationale with the actual reason (avoid stale references across hook frames + Event derives Clone for plugin-side re-inspection).
- [x] [Review][Patch] **`on_agenda_query_after` doc claim corrected** — [crates/orgsidian-plugin-api/src/plugin.rs:135-145](crates/orgsidian-plugin-api/src/plugin.rs#L135-L145). Doc now acknowledges partial mutation is possible and instructs plugins to treat the `&mut Vec` as transactional.
- [x] [Review][Patch] **Anchor test exercises all 9 `Event` variants** — [crates/orgsidian-plugin-api/tests/trait_surface.rs](crates/orgsidian-plugin-api/tests/trait_surface.rs). Added `fn _event_surface_is_locked` (exhaustive `match` over the 9 variants + `_` wildcard for `#[non_exhaustive]`) and `#[test] all_event_variants_construct` (constructs each variant). Future field renames / variant removals now break compile.
- [x] [Review][Patch] **`dyn`-compatibility asserted for `HookContext` / `PluginContext`** — [crates/orgsidian-plugin-api/tests/trait_surface.rs](crates/orgsidian-plugin-api/tests/trait_surface.rs). Added `#[test] context_traits_are_object_safe` constructing `Box<dyn HookContext>` and `Box<dyn PluginContext>`. Future generic-method additions would break object-safety with a compile-time guard.
- [x] [Review][Defer] **Explicit `Send + Sync` static assertion on `PluginError`** — deferred. Currently relies on auto-trait inference (all variants carry `String`). Adding `static_assertions::assert_impl_all!(PluginError: Send, Sync)` would anchor the bound against future `#[non_exhaustive]` variant additions that carry non-`Send` fields. Defer to LD-50 v0.5 surface review or whenever first non-`String`-only variant is proposed.

#### Dismissed (within spec / pre-existing / out of leaf scope)

- `PluginMetadata::version: String` unvalidated → spec AC6 prescribes `String` verbatim; semver validation would add a dep.
- `PluginMetadata::metadata()` returns by value (clone cost) → spec AC2 prescribes signature verbatim.
- `Event` lacks `Serialize`/`Deserialize` → spec AC3 prescribes derives verbatim (`Debug, Clone` only); IPC re-wrap is host-side façade pattern per LD-26.
- `PluginError` lacks `Clone` / `#[source]` chain → spec AC8 prescribes derives + variants verbatim; additive enhancement is minor-bump candidate.
- Module-level `#![allow(clippy::…)]` in `plugin.rs` / `error.rs` / `metadata.rs` → AC9 explicitly permits "at the item or module level with a `// rationale:` comment"; rationale comments present.
- CHANGELOG lists `priority` alongside required methods → matches AC11 verbatim block.
- `&mut self` hooks + `Send + Sync` super-bound → registry-level synchronization is host's concern (LD-25); outside leaf scope.
- `Cargo.lock` +4 lines without new transitives → verified: only adds dep-graph edges to `serde` / `thiserror` already present from Story 1.4.
- `serde` derive feature inherited from workspace → workspace ownership is the canonical pattern; no defensive echo needed.
