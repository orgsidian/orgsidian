# Story 1.18: TOML settings authoritative store with hybrid boundary

Status: review

## Metadata

github_issue: 136

## Story

As the **author / contributor**,
I want a TOML-based authoritative settings store at [`<Vault>/.orgsidian/settings.toml`](#) + [`<config-dir>/global.toml`](#) — surfaced through a `crates/orgsidian-core/src/settings/` Rust module shipping `read_vault_settings` / `write_vault_settings` / `read_global_settings` / `write_global_settings`, owning the `VaultSettings` + `GlobalSettings` schemas, mandating `[meta] schema_version = 1` in every file, writing via the existing [`orgsidian_vault::atomic_write`](crates/orgsidian-vault/src/lib.rs#L18) infrastructure, and a written boundary at [`docs/architecture/settings-boundary.md`](docs/architecture/settings-boundary.md) that pins `tauri-plugin-store` to ephemeral UI state only — wired so every downstream Settings-touching story consumes a stable, human-editable, file-authoritative source-of-truth from day 1,
So that the dual-surface OQ-7 commitment ([PRD §10 OQ-7](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L586)) is enforced by construction and Stories 4.6 / 6.7 / 7.2 / 7.5 / 8.4 / 11.3 / 11.5 / 12.0 / 12.3 (every story whose AC currently says "persist X" or "store Y in settings") consume `read/write_*_settings` from day 1 instead of reaching for `tauri-plugin-store` ([architecture.md LD-40 amendment](_bmad-output/planning-artifacts/architecture.md#L1188-L1194)).

**Traces:** LD-40, PRD §10 OQ-7, FR-23.

## Acceptance Criteria

### AC1 — Create `crates/orgsidian-core/src/settings/` module with the public read/write API surface.

- **NET-NEW directory** `crates/orgsidian-core/src/settings/` with `mod.rs` + `schema.rs` + `vault.rs` + `global.rs` + `meta.rs` + `error.rs`. The module is wired into [`crates/orgsidian-core/src/lib.rs`](crates/orgsidian-core/src/lib.rs#L1) via `pub mod settings;` re-exported types.
- **PUBLIC API SIGNATURES** (must match exactly — these are the contract every downstream story depends on):
  ```rust
  // crates/orgsidian-core/src/settings/mod.rs
  //! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface)
  pub use error::{SettingsError, SettingsResult};
  pub use schema::{VaultSettings, GlobalSettings, SchemaVersion, SCHEMA_VERSION_CURRENT};
  pub use vault::{read_vault_settings, write_vault_settings, vault_settings_path};
  pub use global::{read_global_settings, write_global_settings, global_settings_path};
  ```
  ```rust
  // vault.rs
  pub fn vault_settings_path(vault_path: &Path) -> PathBuf;
  pub fn read_vault_settings(vault_path: &Path) -> SettingsResult<VaultSettings>;
  pub fn write_vault_settings(vault_path: &Path, settings: &VaultSettings) -> SettingsResult<()>;

  // global.rs
  pub fn global_settings_path() -> SettingsResult<PathBuf>;
  pub fn read_global_settings() -> SettingsResult<GlobalSettings>;
  pub fn write_global_settings(settings: &GlobalSettings) -> SettingsResult<()>;
  ```
- **PATH RESOLUTION**:
  - `vault_settings_path(vault)` returns `<vault>/.orgsidian/settings.toml` — pure path arithmetic, no I/O. `.orgsidian/` is created on first `write_vault_settings` if absent (parent-directory mkdir is the writer's job, not the path-resolver's).
  - `global_settings_path()` returns `<config-dir>/orgsidian/global.toml` where `<config-dir>` is resolved via the `dirs` crate (`dirs::config_dir()` → `~/.config` on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows per [architecture.md LD-40](_bmad-output/planning-artifacts/architecture.md#L1191)). Returns `SettingsError::ConfigDirUnavailable` if `dirs::config_dir()` returns `None` (extremely rare; degraded-OS-environment guard).
- **READ SEMANTICS** (`read_*_settings`):
  - File does not exist → return `Ok(<schema>::default())`. Default is "first-launch": empty recents list, empty keybindings, light theme, no dismissed coaching, etc. Defined as `#[derive(Default)]` on the schema structs with `#[serde(default)]` field-level defaults.
  - File exists but malformed (TOML parse error or schema-version mismatch) → return `Err(SettingsError::ParseFailed { path, source })` — caller (shell-app bootstrap) handles the LD-41 fallback (backup `<file>.broken-{timestamp}` + warn banner; out of Story 1.18 scope — leave a `TODO(Story-6.7)` comment at the call site).
  - File exists and parses → return `Ok(settings)`. Unknown fields are preserved silently via `#[serde(other)]` on a `_extra: toml::Table` field on each struct (forward-compat guard — a v2 app reading a v1 file must not lose fields it doesn't recognize when it writes back).
- **WRITE SEMANTICS** (`write_*_settings`):
  - Serialize via `toml::to_string_pretty(settings)` (deterministic key ordering — relies on `serde`'s field-declaration order in the struct definition; do NOT use a `HashMap`-typed field at the top level).
  - Prepend the `[meta] schema_version = N` block first (TOML headers ordering rule: top-level keys come before `[tables]` so `schema_version` must be a top-level key, not nested under `[meta]`; see AC3 schema design — `schema_version` is in fact a top-level scalar with a leading `# === Orgsidian settings — schema v{N} (LD-40) ===\n# Edit by hand if you like; the Settings GUI is a thin round-trip editor over this file.\n` comment header).
  - Write atomically via `orgsidian_vault::atomic_write(path, content.as_bytes())` (LD-8 + Story 1.9 surface — no retry yet; Story 3.1 adds the AV-aware 3-retry wrapper transparently per the [Story 1.9 anchor sentinel discipline](crates/orgsidian-vault/src/lib.rs#L6-L8)).
  - Create parent directory (`<vault>/.orgsidian/` or `<config-dir>/orgsidian/`) via `std::fs::create_dir_all` BEFORE the atomic write. `create_dir_all` is idempotent — re-running is safe. Failure returns `SettingsError::Io { path, source }`.
- **CRATE DEPENDENCY**: Story 1.18 is the first-use story for `orgsidian-vault` from `orgsidian-core` per the [Story 1.4 cross-crate edges discipline](crates/orgsidian-core/src/lib.rs#L3) ("Structural placeholder — cross-crate edges materialize incrementally per first-use story"). Add `orgsidian-vault` to the workspace `[workspace.dependencies]` table (mirroring the [`orgsidian-core` pattern at root Cargo.toml line 62](Cargo.toml#L62) — explicit `version = "0.0.0"` + `path = "crates/orgsidian-vault"` to keep `cargo deny check bans` happy per [LEAF graph rule at deny.toml line 186-188](deny.toml#L186-L188)) AND in `crates/orgsidian-core/Cargo.toml` as `orgsidian-vault = { workspace = true }`. The LEAF graph rule allows this exact edge (`orgsidian-core` is the LEAF wrapper per `wrappers = ["orgsidian-core"]` in [deny.toml line 188](deny.toml#L188)).

### AC2 — Define `VaultSettings` + `GlobalSettings` Rust schemas with `[meta] schema_version` versioning and `#[derive(Serialize, Deserialize)]`.

- **FILE** `crates/orgsidian-core/src/settings/schema.rs`. Schema versioning **and** the per-struct field set are locked here; downstream stories EXTEND the schema (add fields with `#[serde(default)]` for forward-compat) rather than redesigning.
- **SCHEMA VERSION CONSTANT**: `pub const SCHEMA_VERSION_CURRENT: u32 = 1;` (LD-12 mirror: forward-only migration discipline — bump = new app reads old files via `#[serde(default)]`; old app reads new files via `#[serde(other)]` catch-all on `_extra`). The crate exposes a `pub struct SchemaVersion(pub u32);` newtype with `Serialize`/`Deserialize` + a custom `Deserialize` that REJECTS versions greater than `SCHEMA_VERSION_CURRENT` (forward-compat refusal — a v1 binary cannot safely interpret a v2 file because v2's semantic-changes are unknown; future Story 6.7+ relaxes this to a "warn + best-effort read" when the dirty-buffer LD-7 hook lands).
- **`VaultSettings` STRUCT** (the LD-40 v0.1 baseline; downstream stories add fields):
  ```rust
  #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(default, deny_unknown_fields = false)] // forward-compat: unknown fields preserved into _extra
  pub struct VaultSettings {
      /// Mandatory header. `SchemaVersion(1)` for v0.1.
      pub schema_version: SchemaVersion,

      /// FR-23 keybinding remap (Story 12.3 lands the UI; schema-shape locked here).
      /// Key = canonical action ID (e.g., "editor.save"); value = chord string ("Cmd+S").
      /// Stored sorted by key for deterministic round-trip.
      pub keybindings: BTreeMap<String, String>,

      /// FR-22 active theme. Absolute path or "default-light" / "default-dark" sentinel.
      /// (Story 6.7 lands the user-CSS loader; schema-shape locked here.)
      pub theme: ThemeChoice,

      /// FR-10 Quick Capture global hotkey (Story 8.1 lands the wiring).
      pub capture_hotkey: Option<String>,

      /// FR-7 saved named agenda filter presets (Story 7.5 lands the UI; schema-shape locked here per [epics.md:1555](_bmad-output/planning-artifacts/epics.md#L1555)).
      /// `BTreeMap` for deterministic ordering. Key = user-chosen name; value = preset definition.
      pub agenda_presets: BTreeMap<String, AgendaPreset>,

      /// FR-21 dismissed coaching IDs (Story 11.5 lands the persist; schema-shape locked here).
      pub dismissed_coaching: BTreeSet<String>,

      /// FR-20 Plain/Power Mode preference (Story 11.3 lands the runtime toggle; schema-shape locked here).
      pub ui_mode: UiMode,

      /// FR-6 Today Dashboard section preferences (Story 7.2 lands the toggles; schema-shape locked here).
      pub today_dashboard: TodayDashboardSections,

      /// Forward-compat catch-all: unknown top-level keys land here on read, are
      /// preserved on write. A v2-app field shipped to a v1-app and back survives
      /// the round-trip. Skipped on serialize when empty.
      #[serde(default, flatten)]
      pub _extra: toml::Table,
  }
  ```
  Where the nested types are:
  ```rust
  #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(rename_all = "kebab-case")]
  pub enum ThemeChoice {
      #[default]
      DefaultLight,
      DefaultDark,
      /// Absolute path to a user-supplied CSS file (Story 6.7 / 12.1).
      Custom(PathBuf),
  }

  #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(rename_all = "kebab-case")]
  pub enum UiMode {
      #[default]
      Plain,
      Power,
  }

  #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(default)]
  pub struct AgendaPreset {
      /// "today" | "week" | "custom"
      pub view: String,
      /// Free-form tag/TODO-state filter; semantics finalized in Story 7.5.
      pub filters: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(default)]
  pub struct TodayDashboardSections {
      pub show_scheduled: bool,
      pub show_deadlines: bool,
      pub show_clock: bool,
      pub show_inbox: bool,
  }
  impl Default for TodayDashboardSections {
      fn default() -> Self { Self { show_scheduled: true, show_deadlines: true, show_clock: true, show_inbox: true } }
  }
  ```
- **`GlobalSettings` STRUCT** (LD-40 global state):
  ```rust
  #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
  #[serde(default)]
  pub struct GlobalSettings {
      pub schema_version: SchemaVersion,

      /// LD-40: list of recent Vault paths (Story 1.18+ shell startup populates).
      /// `Vec<PathBuf>` ordered most-recent-first. Capped at 10 by the writer (deduped).
      pub recent_vaults: Vec<PathBuf>,

      /// Default UI language (LD-52 / lingui locale code, e.g. "en", "it"). `None` = OS locale.
      pub default_language: Option<String>,

      /// Default theme for new Vaults (mirrors `VaultSettings::theme` choice variants).
      pub default_theme: ThemeChoice,

      #[serde(default, flatten)]
      pub _extra: toml::Table,
  }
  ```
- **DOC-COMMENT FIRST LINE** (FR Traceability Discipline — Story 1.4 convention): every file under `crates/orgsidian-core/src/settings/` carries `//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface)` as the first doc-comment line, per the verbatim AC text at [epics.md:740](_bmad-output/planning-artifacts/epics.md#L740). At least 6 files (mod.rs, schema.rs, vault.rs, global.rs, meta.rs, error.rs) MUST carry this line; a `grep -c "LD-40 + FR-23"` smoke in `tests/settings.rs` asserts the count is `>= 6` (drift guard mirroring the Story 1.17 grep-smoke pattern).
- **`specta::Type` DERIVE**: all public schema types derive `specta::Type` so the TS bindings auto-export via [`tests/export_bindings.rs`](crates/orgsidian-shell-app/tests/export_bindings.rs). Story 1.18 does NOT add Tauri commands — the IPC bridge wiring is downstream (Story 6.7+); the `specta::Type` derive is forward-compat-only and is what enables zero-friction adoption by the shell-app later.

### AC3 — Round-trip fidelity: read TOML → serialize back → byte-identical when no field changed.

- **PROPERTY**: For any `VaultSettings` or `GlobalSettings` value `s`, the operation `read(write(s))` returns a value `s2` such that `s == s2` (structural equality), AND for any TOML file `F` that was written by `write_*_settings`, `write(read(F)) == F` byte-for-byte (writer fixed-point — the writer is its own canonical form).
- **TEST FILE** `crates/orgsidian-core/tests/settings_round_trip.rs` (workspace-rooted via `[[test]]` declaration in `crates/orgsidian-core/Cargo.toml` if needed; default discovery works for `tests/*.rs` inside the crate).
- **REQUIRED CASES** (each as a `#[test]` fn):
  1. `default_vault_settings_round_trip` — `VaultSettings::default()` → write → read → assert `==` (structural).
  2. `default_global_settings_round_trip` — analogous for `GlobalSettings`.
  3. `populated_vault_settings_round_trip` — a value with every field non-default (≥1 keybinding, custom theme path, 2 agenda presets, 1 dismissed coaching ID, Power mode, non-default today_dashboard) → write → read → structural `==`.
  4. `writer_fixed_point` — call `write` twice with the same value, compare the two written files byte-for-byte (asserts deterministic serialization).
  5. `unknown_fields_preserved` — write a v0.1 file, manually inject `[some_v2_extension]\nfoo = 1\n` into the on-disk TOML, read it, write the read value back, assert `[some_v2_extension]\nfoo = 1\n` is still present in the output (validates `_extra` forward-compat).
  6. `schema_version_one_present_on_default_write` — write `VaultSettings::default()`, then grep the resulting file for `schema_version = 1` exactly once and assert presence (anti-placebo — protects against an accidental `#[serde(skip)]` on the version field).
- **PROPERTY TEST** (Story 1.18 uses `proptest` per the workspace pattern at [architecture.md:188](_bmad-output/planning-artifacts/architecture.md#L188); already in the dep tree via dev-deps elsewhere): one `proptest!` block in the same test file generates a randomized `VaultSettings` value and asserts `read(write(s)) == s` over 256 cases. The `proptest::strategy::Strategy` impl is hand-written for the schema types (see `proptest_derive` if it lands in workspace deps; else write the `Strategy` closures inline — ~30 LOC).
- **SCOPE NOTE — comment preservation**: Story 1.18 uses the `toml` crate (1.x — see AC4) for serialization, NOT `toml_edit`. Consequence: user-added comments in the on-disk TOML file are **NOT preserved** through a GUI-triggered write. This is acceptable for v0.1 Alpha because no GUI Settings editor exists yet (Stories 6.7 / 12.3 land the GUI). The Story 1.18 fidelity contract is exactly the AC text — "byte-identical **when no field changed**", which Reads + Writes via `toml::to_string_pretty` satisfies because the structural round-trip is deterministic. Comment-preserving round-trip (`toml_edit` migration) is **explicitly deferred** to Story 12.3 (FR-23 GUI), with a `// FOLLOWUP(Story-12.3): swap to toml_edit for format-preserving GUI round-trip` comment at the top of `vault.rs::write_vault_settings`. Update [deferred-work.md](_bmad-output/implementation-artifacts/deferred-work.md) with a "Deferred from: code review of story-1.18" stanza capturing this.

### AC4 — Add `toml` crate (1.x) + `dirs` crate (6.x) as `orgsidian-core` dependencies; verify license allowlist.

- **WORKSPACE-LEVEL** at root [`Cargo.toml`](Cargo.toml#L30-L62): add `toml = "1"` and `dirs = "6"` to `[workspace.dependencies]`. Follow the existing in-line comment convention (a short `# Story 1.18 (LD-40): ...` line above each — mirror Story 1.4's `tauri-specta` comment at [Cargo.toml lines 36-39](Cargo.toml#L36-L39)).
- **CRATE-LEVEL** in [`crates/orgsidian-core/Cargo.toml`](crates/orgsidian-core/Cargo.toml#L19-L33): add `toml = { workspace = true }`, `dirs = { workspace = true }`, and `orgsidian-vault = { workspace = true }` to `[dependencies]`. Both new direct edges are first-use-story per the [Story 1.4 cross-crate-edges discipline](crates/orgsidian-core/Cargo.toml#L20).
- **DEV-DEP**: add `proptest = "1"` to `[workspace.dependencies]` and to `crates/orgsidian-core/Cargo.toml [dev-dependencies]` (first proptest use in the workspace). License: MIT/Apache-2.0 — pre-allowed.
- **LICENSE VERIFICATION** (LD-37 hygiene; mirror Story 1.16's protocol per [deferred-work.md story-1.16 entry](_bmad-output/implementation-artifacts/deferred-work.md#L98)):
  - `toml@1.x` license = MIT/Apache-2.0 — pre-allowed in [deny.toml line 73-74](deny.toml#L73-L74).
  - `dirs@6.x` license = MIT/Apache-2.0 — pre-allowed.
  - `proptest@1.x` license = MIT/Apache-2.0 — pre-allowed.
  - Run `cargo deny check licenses` locally + verify in PR CI; no `deny.toml` change required. **DO NOT** modify `deny.toml` allowlist or add `skip` entries unless `cargo deny` explicitly flags a transitive — if it does, STOP and surface a decision-grade question (a new transitive license is a policy decision).
- **TRANSITIVE COEXISTENCE WARNING**: `toml = "1"` (the explicit Story 1.18 dep) will coexist in the dep tree with `toml@0.8` (forced by the Tauri build chain — see [deny.toml line 145](deny.toml#L145)) and `toml@0.9` (intermediate — [line 146](deny.toml#L146)). All three are already in the lock + `[bans].skip` allowlist. **No `Cargo.lock` patching, no new `[bans].skip` entry, and no `deny.toml` modification is needed** — verify by running `cargo deny check bans` before opening the PR.
- **AUDIT VERIFICATION**: run `cargo audit` post-`cargo update -p toml -p dirs -p proptest` and confirm no NEW advisories surface (the existing `RUSTSEC-2024-0429` glib advisory is ignored per [advisory-exceptions.md](docs/security/advisory-exceptions.md) and is unrelated).

### AC5 — Write the `docs/architecture/settings-boundary.md` boundary doc (`tauri-plugin-store` retention list).

- **NEW FILE** `docs/architecture/settings-boundary.md`. ~80-120 lines of markdown. Authoritative reference for the LD-40 `tauri-plugin-store` carve-out — every Settings-touching story downstream must `grep` this file before adding ANY new key to `tauri-plugin-store`.
- **REQUIRED SECTIONS** (verbatim section headings — drift guard):
  1. `# Settings Store Boundary (LD-40 + FR-23)` — top-of-file H1.
  2. `## Authoritative Settings (TOML, OQ-7 dual-surface)` — table of every field in `VaultSettings` + `GlobalSettings` v0.1, with column `Field | TOML key | Owner story | UI surface (deferred to)`.
  3. `## Ephemeral UI State (`tauri-plugin-store`-allowed)` — closed allowlist of exactly the four entries per [epics.md:735](_bmad-output/planning-artifacts/epics.md#L735) + [architecture.md:1194](_bmad-output/planning-artifacts/architecture.md#L1194):
     - `lastOpenFile` — string path; reset on first launch; survives crash but NOT a fresh install.
     - `windowGeometry` — `{x,y,width,height,monitor}`; managed by `tauri-plugin-window-state`, not authoritative settings.
     - `tutorialProgress` — `{step, completed}`; Story 13.3 owns the schema.
     - `lastVaultPath` — single path (DIFFERENT from `GlobalSettings::recent_vaults` — that's the AUTHORITATIVE history; `lastVaultPath` is the "what to auto-reopen" ephemeral pointer, which the user can reset by holding Shift on launch).
  4. `## Forbidden Patterns` — explicit anti-pattern list: "Never store keybindings in `tauri-plugin-store`. Never store theme paths there. Never store agenda presets there. Never store coaching dismissals there." Each row paired with the canonical TOML location.
  5. `## Adding a New Setting (decision tree)` — 3-question flowchart in markdown: (a) "Does the user expect to edit this in a text editor?" YES → TOML. (b) "Does it survive a fresh app install?" NO → `tauri-plugin-store`. (c) "Is it per-Vault or global?" → `<Vault>/.orgsidian/settings.toml` vs `<config-dir>/orgsidian/global.toml`.
  6. `## References` — links to LD-40 (architecture.md#L1188-L1194), OQ-7 (prd.md#L586), FR-23 (prd.md#L411), Story 1.18 spec (this story file).
- **DRIFT GUARD**: a `tests/settings_boundary_doc.rs` integration test (workspace-rooted; declared in `crates/orgsidian-core/Cargo.toml` via `[[test]]` mirroring the [failure_modes pattern at lines 51-53](crates/orgsidian-core/Cargo.toml#L51-L53)) reads `docs/architecture/settings-boundary.md` and asserts: (a) all 6 required section headings are present verbatim, (b) the ephemeral allowlist contains exactly 4 entries, (c) each `VaultSettings`/`GlobalSettings` field name appears at least once in the doc (catches the case where a future field-add forgets to update the boundary doc). The test is `#[cfg(target_family = "unix")]`-gated only if file-path normalization becomes an issue on Windows (verify locally first; default = no gate).

### AC6 — Wire `read/write_*_settings` into the shell-app bootstrap as a smoke; leave watcher hook in place but not connected.

- **CONSUMER WIRING** in [`crates/orgsidian-shell-app/src/lib.rs`](crates/orgsidian-shell-app/src/lib.rs): under the existing `tauri::Builder::default()` chain, after the `tauri-plugin-store` registration at [line 58](crates/orgsidian-shell-app/src/lib.rs#L58), add a Tauri `setup` hook that calls `orgsidian_core::settings::read_global_settings()` and logs the result via the existing [`tracing` facade](crates/orgsidian-core/src/registry.rs) (info-level on success; warn-level on `Err` — does NOT fail startup). This is the **smoke test that proves the wire is live**; full GUI consumption (read into Zustand, render into Settings panels) is Story 12.x scope.
- **DO NOT** add `read_vault_settings` to the bootstrap. Vault designation is Story 3.6 — Story 1.18 cannot know the Vault path at startup. The Vault-settings read happens at `Vault → Open`-completion time (Story 3.6 owns that wiring).
- **WATCHER HOOK** per [epics.md:739](_bmad-output/planning-artifacts/epics.md#L739): leave a `// FOLLOWUP(Story-5.4): watcher reload hook lands here per LD-7 Single Writer Rule` comment in `crates/orgsidian-core/src/settings/vault.rs` next to the `write_vault_settings` body — the settings file is just another file under the watcher; external edits reload via the LD-7 dirty-buffer check (Story 5.4 wires this; Story 1.18 leaves the hook documented but inert). **NO** watcher subscription code in Story 1.18 — this would couple to `orgsidian-watcher` (Story 5.1 territory) prematurely.
- **`tauri-plugin-store` IS NOT REMOVED**: the plugin stays registered (line 58 of `lib.rs`) — it has 4 legitimate ephemeral-state consumers per AC5. Do not delete; do not gate behind a feature flag. Story 1.18 is a BOUNDARY-DEFINITION story, not a deletion story.
- **SPECTA EXPORT IS NOT REQUIRED YET**: the schemas derive `specta::Type` but Story 1.18 does NOT add `#[tauri::command]` wrappers. The TS bindings file `shell-ui/src/lib/tauri.ts` is regenerated automatically by `cargo test --test export_bindings` on the next downstream story that adds settings commands (Story 6.7 / 12.3 most likely). The `specta::Type` derive is a forward-compat marker only.

### AC7 — Unit tests + integration tests prove the API contract end-to-end.

- **UNIT TESTS** inside the crate, co-located in each module file via `#[cfg(test)] mod tests { ... }`:
  - `vault.rs::tests::vault_settings_path_joins_dotorgsidian` — pure path arithmetic.
  - `vault.rs::tests::read_returns_default_when_file_missing` — `read_vault_settings(tempdir())` returns `Ok(VaultSettings::default())`.
  - `vault.rs::tests::write_creates_dotorgsidian_dir` — `write_vault_settings(tempdir(), &settings)` creates `.orgsidian/` then `settings.toml`.
  - `vault.rs::tests::parse_failure_surfaces_parse_failed_variant` — write a deliberately-malformed TOML to the file, assert `read_vault_settings` returns `Err(SettingsError::ParseFailed { .. })` matching on the variant.
  - `global.rs::tests::*` — analogous set for the global path resolver (use `dirs::config_dir().unwrap_or_else(|| tempdir())` or env-var-override fixture; document the fixture choice in the test).
  - `schema.rs::tests::schema_version_rejects_future_version` — manually deserialize `schema_version = 999` and assert the error.
  - `schema.rs::tests::extra_table_round_trips` — manual TOML with unknown top-level keys → deserialize → serialize → keys present.
- **INTEGRATION TESTS** in `crates/orgsidian-core/tests/settings_round_trip.rs` per AC3 above (6 tests + 1 proptest block).
- **BOUNDARY DRIFT TEST** in `tests/settings_boundary_doc.rs` per AC5 above.
- **COVERAGE**: full test count for Story 1.18 ≥ 16 (8 unit + 6 round-trip + 1 proptest + 1 boundary). Run `cargo test -p orgsidian-core` post-implementation and report the count in Completion Notes.
- **NO NEW CI STEPS**: tests run via the existing `cargo test --workspace` step in [`.github/workflows/pr.yml`](.github/workflows/pr.yml). Do NOT add a dedicated Story 1.18 CI step — the default test gate is sufficient.

### AC8 — Documentation + traceability annotations.

- **DOC-COMMENT TRACEABILITY** (AC2 already covers this; restated for completeness): every `.rs` file under `crates/orgsidian-core/src/settings/` carries `//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface)` as the FIRST line. `tests/settings.rs` (or the existing `tests/failure_modes_coverage.rs`-style smoke) asserts the grep count `>= 6`.
- **WORKSPACE README** update: append a one-line row to the `crates/` table in `ARCHITECTURE.md` (root-level) or `docs/architecture.md` (whichever serves as the canonical crates index — verify which one is the live table on disk) noting `orgsidian-core` now exposes the `settings` module. Do NOT touch the planning artifacts (`_bmad-output/planning-artifacts/architecture.md` is canonical-but-archival per [architecture.md:1010](_bmad-output/planning-artifacts/architecture.md#L1010)).
- **DEFERRED-WORK STANZA**: add a `## Deferred from: code review of story-1.18 (YYYY-MM-DD)` section to [`_bmad-output/implementation-artifacts/deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) with at minimum the `toml_edit` migration follow-up from AC3 scope note. Mirror the format used by every prior story's stanza.

## Tasks / Subtasks

- [x] **T1** — Scaffold `crates/orgsidian-core/src/settings/` directory with empty `mod.rs`, `schema.rs`, `vault.rs`, `global.rs`, `meta.rs`, `error.rs`. Each file starts with `//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface)`. Wire `pub mod settings;` into `crates/orgsidian-core/src/lib.rs`. Run `cargo check -p orgsidian-core` — must compile. (AC1, AC8)
- [x] **T2** — Add `toml = "1"`, `dirs = "6"`, `proptest = "1"` to workspace `[workspace.dependencies]` in root `Cargo.toml`. Add `orgsidian-vault` workspace dep entry (mirror `orgsidian-core` pattern: explicit `version = "0.0.0"` + `path = "crates/orgsidian-vault"`). (AC4)
- [x] **T3** — Add `toml`, `dirs`, `orgsidian-vault` to `[dependencies]` of `crates/orgsidian-core/Cargo.toml`; add `proptest` + `tempfile` (already workspace) to `[dev-dependencies]`. Run `cargo build -p orgsidian-core` — must compile. (AC4)
- [x] **T4** — Implement `schema.rs`: `SchemaVersion`, `SCHEMA_VERSION_CURRENT`, `ThemeChoice`, `UiMode`, `AgendaPreset`, `TodayDashboardSections`, `VaultSettings`, `GlobalSettings`. All derive `Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type` per AC2. Wire custom `Deserialize` for `SchemaVersion` that rejects > `SCHEMA_VERSION_CURRENT`. (AC2)
- [x] **T5** — Implement `error.rs`: `SettingsError` enum (`ConfigDirUnavailable`, `Io { path, source }`, `ParseFailed { path, source }`, `SerializeFailed { source }`, `SchemaVersionUnsupported { found, supported }`). Derives `thiserror::Error` per the [`OrgError` precedent](crates/orgsidian-core/src/error.rs). `pub type SettingsResult<T> = Result<T, SettingsError>;`. (AC1)
- [x] **T6** — Implement `vault.rs`: `vault_settings_path`, `read_vault_settings`, `write_vault_settings` per AC1 semantics. Use `toml::from_str` / `toml::to_string_pretty` + `orgsidian_vault::atomic_write`. Add the `// FOLLOWUP(Story-5.4): watcher reload hook lands here per LD-7 Single Writer Rule` comment. Add `// FOLLOWUP(Story-12.3): swap to toml_edit for format-preserving GUI round-trip` comment. (AC1, AC3 scope note, AC6 hook)
- [x] **T7** — Implement `global.rs`: `global_settings_path`, `read_global_settings`, `write_global_settings`. Use `dirs::config_dir()` for the base path. (AC1)
- [x] **T8** — Co-locate unit tests in each module via `#[cfg(test)] mod tests { ... }` per AC7. Use `tempfile::tempdir()` for filesystem fixtures. Run `cargo test -p orgsidian-core settings` — all pass. (AC7)
- [x] **T9** — Write `crates/orgsidian-core/tests/settings_round_trip.rs` with the 6 round-trip tests + 1 proptest per AC3. Hand-write a `Strategy` impl or compose existing ones (Rust patterns at `proptest::strategy::Strategy`). (AC3)
- [x] **T10** — Wire shell-app bootstrap smoke per AC6: add a `setup` closure step in `crates/orgsidian-shell-app/src/lib.rs` after the existing `tauri-plugin-store` registration that calls `orgsidian_core::settings::read_global_settings()` and logs the result via `tracing`. Verify `cargo build -p orgsidian-shell-app` succeeds. (AC6)
- [x] **T11** — Write `docs/architecture/settings-boundary.md` per AC5 (6 sections, ~80-120 lines). Cross-link to LD-40 / OQ-7 / FR-23. (AC5)
- [x] **T12** — Write `tests/settings_boundary_doc.rs` workspace-rooted drift guard per AC5; declare it via a `[[test]]` block in `crates/orgsidian-core/Cargo.toml` (mirror the `failure_modes` pattern at [lines 51-53](crates/orgsidian-core/Cargo.toml#L51-L53)). (AC5)
- [x] **T13** — Write `tests/settings.rs` traceability grep-smoke asserting `LD-40 + FR-23` appears in ≥6 files under `crates/orgsidian-core/src/settings/`. Mirror Story 1.17's grep-smoke pattern. (AC8)
- [x] **T14** — Run `cargo deny check licenses bans advisories`; run `cargo audit`; confirm no NEW advisories. Run `cargo test --workspace`; confirm all green and report Story 1.18's test count in Completion Notes. (AC4, AC7)
- [x] **T15** — Update `ARCHITECTURE.md` / `docs/architecture.md` crates table with the new `settings` module exposed by `orgsidian-core`. Append the deferred-work stanza to `_bmad-output/implementation-artifacts/deferred-work.md`. (AC8, AC3 scope note)
- [x] **T16** — Commit + open PR. Commit title: `feat(core): wire LD-40 TOML settings authoritative store (Story 1.18, closes #136)` (mirrors the Story 1.16 / 1.17 `feat(<scope>): wire LD-NN ...` Conventional Commits pattern + the [[feedback_no_co_author_credit]] memory — NO co-author trailer, NO "Generated with Claude Code" footer). (AC4, AC7)

## Review Findings

(empty — populated on code-review)

## Dev Notes

### Critical context the dev agent must internalize

1. **Scope-fence: schemas are LOCKED, not just sketched.** The `VaultSettings` / `GlobalSettings` field set defined in AC2 IS the v0.1 baseline. Downstream stories ADD fields (with `#[serde(default)]` for forward-compat) but do NOT redesign. If a downstream AC text contradicts an AC2 field (e.g. "the dismissed coaching IDs should be a `Vec` not a `BTreeSet`"), the contradiction is resolved in Story 1.18's favor — the schema is the contract. The reason: every downstream consumer story (4.6, 6.7, 7.2, 7.5, 8.4, 11.3, 11.5, 12.0, 12.3) needs a stable type signature TODAY to compile-check; redesigning later means cascading churn.

2. **Atomic-write reuses Story 1.9 surface, no retry yet.** Use `orgsidian_vault::atomic_write(path, content.as_bytes())` directly — the signature is preserved across the Story 3.1 retry-wrapper swap per the [anchor sentinel discipline](crates/orgsidian-vault/src/lib.rs#L6-L8). DO NOT inline `AtomicWriteFile::open` in `settings/vault.rs`; that's a leaky abstraction (Story 3.1 will wrap the call with backoff + AV-aware retry, and any direct usage in `settings/` would need to be tracked down). The single-line delegation is the right shape today.

3. **First-use-story cross-crate edge — wire `orgsidian-vault` into `orgsidian-core`'s dep list properly.** Per [Cargo.toml line 62](Cargo.toml#L62) pattern: `orgsidian-vault = { path = "crates/orgsidian-vault", version = "0.0.0" }` in workspace `[workspace.dependencies]`, then `orgsidian-vault = { workspace = true }` in `crates/orgsidian-core/Cargo.toml [dependencies]`. The LEAF graph rule at [deny.toml line 186-188](deny.toml#L186-L188) explicitly allows this exact edge — `orgsidian-vault` is a LEAF, `orgsidian-core` is its only allowed wrapper. Adding the edge to ANY OTHER crate (e.g., adding `orgsidian-vault` directly to `orgsidian-shell-app/Cargo.toml`) WILL FAIL `cargo deny check bans`.

4. **`tauri-plugin-store` stays — do not delete the plugin registration.** This is a BOUNDARY-DEFINITION story, not a deletion story. The plugin remains at [`crates/orgsidian-shell-app/src/lib.rs:58`](crates/orgsidian-shell-app/src/lib.rs#L58) registered, and the [`store:default` capability at `crates/orgsidian-shell-app/capabilities/main.json:17`](crates/orgsidian-shell-app/capabilities/main.json#L17) stays granted. The 4 legitimate ephemeral-state consumers (per AC5) need it. The story defines the BOUNDARY so future stories know which keys are allowed; it does not remove the plugin.

5. **TOML round-trip — no comment preservation yet.** The `toml` crate (1.x) does structural serialization only. User-added comments and key-reordering in the on-disk TOML file are LOST through a `write_*_settings` call. This is acceptable for v0.1 because no GUI Settings editor ships in this story (Stories 6.7 + 12.3 land that). The `toml_edit` upgrade (which preserves comments + ordering + whitespace) is deferred to Story 12.3 — make sure the deferred-work stanza captures this so the GUI-shipping story doesn't miss the migration. The AC3 fidelity contract ("byte-identical when no field changed") is satisfied by the `toml` crate's deterministic serialization because we control both the writer AND the reader.

6. **`schema_version` is a TOP-LEVEL TOML scalar, not nested under `[meta]`.** TOML's grammar requires top-level keys to appear BEFORE any `[table]` headers. The AC text "`[meta] schema_version = 1`" is loose phrasing from the epic — implementing it as `[meta]\nschema_version = 1` would mean the schema_version is nested. Implement as a top-level `schema_version = 1` field on `VaultSettings` / `GlobalSettings` directly. The leading file header comment `# === Orgsidian settings — schema v1 (LD-40) ===` provides the human-readable framing.

7. **Forward-compat catch-all uses `flatten` not `other`.** `serde`'s `#[serde(other)]` works only for ENUM variants (deserialize-time fallback for unknown discriminator). For a struct's catch-all of unknown TOML keys, the pattern is `#[serde(flatten)] _extra: toml::Table` — TOML keys not matched by named fields land in the `Table`, and `Table` serializes back inline. Verify with the AC3 `unknown_fields_preserved` round-trip test.

8. **`specta::Type` derive is forward-compat, no IPC commands in this story.** The schema types derive `specta::Type` so the TS bindings export cleanly when downstream stories (6.7 / 12.3) add `#[tauri::command]` wrappers around `read/write_*_settings`. Story 1.18 itself adds zero Tauri commands; the TS bindings file [`shell-ui/src/lib/tauri.ts`](shell-ui/src/lib/tauri.ts) is regenerated by the next story that does. If the `specta::Type` derive causes a compile error on a `BTreeMap<String, String>` or similar, suppress via `#[specta(skip)]` on the field as a last resort — but verify first that `specta` 2.0-rc.25 supports the type (it does for `BTreeMap` per the v0.1 stack notes).

9. **`proptest` strategies for the schema types.** Hand-write the `Strategy` impl inline in the test file (or use `prop_oneof!` + the `proptest!` macro) — the workspace does not currently use `proptest_derive`. ~30 LOC per top-level struct. Keep the random keybinding action IDs to a small alphabet (e.g., `[a-z]{3,10}`) so the test stays fast (256 cases × ~5ms = ~1.3s).

10. **AC text path divergence: `crates/orgsidian-core/src/settings/` is the ONLY canonical location.** The epic AC text at [epics.md:732](_bmad-output/planning-artifacts/epics.md#L732) and the architecture LD-40 at [architecture.md:1188-1194](_bmad-output/planning-artifacts/architecture.md#L1188-L1194) both put the API in `orgsidian-core` (composition root). The architecture's FR-23 row at [architecture.md:1068](_bmad-output/planning-artifacts/architecture.md#L1068) lists `tauri-plugin-store` as a co-implementer of FR-23 — this is the STALE pre-2026-05-20 row (LD-40's 2026-05-20 amendment supersedes it). The Story 1.18 implementation follows LD-40, not the stale row. Note this divergence in the Project Structure Notes section of Completion Notes.

11. **No watcher subscription, no Tauri IPC, no Zustand wiring.** Story 1.18's surface is: (a) the Rust module + types, (b) the boundary doc, (c) a bootstrap smoke that proves the wire compiles and reads. Anything beyond that — watcher, IPC, frontend Zustand store, Settings GUI — is downstream. Adding it now creates regression-surface for stories that aren't yet specified. The dev agent should resist scope creep here; if a sub-task seems to need Tauri IPC, STOP and surface a decision-grade question.

### Project Structure Notes

**Alignment with unified project structure**:
- `crates/orgsidian-core/src/settings/` — NEW module, matches the [architecture façade pattern at architecture.md:920](_bmad-output/planning-artifacts/architecture.md#L920) (settings is composition-root logic, not a leaf-crate concern) ✓
- `docs/architecture/settings-boundary.md` — NEW file inside the existing [`docs/architecture/`](docs/) tree which currently holds `failure-modes/`, `perf/`, `security/`. The boundary doc joins the same hierarchy ✓
- Workspace deps additions (`toml`, `dirs`, `proptest`) follow the convention at [Cargo.toml lines 30-58](Cargo.toml#L30-L58) — short comment + version pin ✓

**Detected conflicts or variances** (with rationale):
- **Architecture FR-23 row stale.** [architecture.md:1068](_bmad-output/planning-artifacts/architecture.md#L1068) lists `tauri-plugin-store` as a co-implementer of FR-23 keybinding remap. This row was written BEFORE the 2026-05-20 LD-40 amendment that pinned settings to TOML. Story 1.18 implements per LD-40 (the later amendment). The architecture row should be updated in a future docs-sweep story (NOT in 1.18 scope — `_bmad-output/planning-artifacts/architecture.md` is archival per [architecture.md:1010](_bmad-output/planning-artifacts/architecture.md#L1010); the canonical doc `docs/architecture.md` may need a parallel update only if its FR-23 row diverges; verify on impl).
- **"`[meta]` table" wording in the epic AC is loose.** Epic text says `[meta] schema_version = 1`; implementing under a `[meta]` TOML table header would make the version nested. Story 1.18 implements as a TOP-LEVEL `schema_version = 1` field — the leading file comment `# === Orgsidian settings — schema v1 (LD-40) ===` carries the "meta" framing. Record the variance in Completion Notes.
- **`tauri-plugin-store` retention.** Story 1.18 does NOT remove `tauri-plugin-store` from the Tauri builder or the capability list — both stay as-is for the 4 legitimate ephemeral consumers per AC5. The story is a boundary-definition not a deletion. Record in Project Structure Notes that the plugin registration at [`crates/orgsidian-shell-app/src/lib.rs:58`](crates/orgsidian-shell-app/src/lib.rs#L58) is intentionally preserved.

### Testing Standards Summary

- **Unit tests (Cargo)**: co-located via `#[cfg(test)] mod tests { ... }` in each `*.rs` source file. Excluded from production build automatically by `cargo build`.
- **Integration tests (Cargo)**: under `crates/orgsidian-core/tests/*.rs`. Auto-discovered; declared explicitly in `Cargo.toml` only when they need `required-features = [...]` or workspace-rooted paths (the `failure_modes` pattern at [crates/orgsidian-core/Cargo.toml lines 51-53](crates/orgsidian-core/Cargo.toml#L51-L53) shows the explicit-declaration shape).
- **Property tests (proptest)**: inside the integration-test file via `proptest! { #[test] fn ... { ... } }`. 256 cases default. Story 1.18 uses 1 proptest block.
- **Test runtime budget**: full `cargo test -p orgsidian-core` should stay <30s wall-clock on warm cache (current Story 1.17 baseline ~10s; Story 1.18 adds ~16 tests + 1 proptest block = expected delta ~3-5s).
- **CI matrix**: macOS-arm64 + Ubuntu-LTS per PR via [pr.yml](.github/workflows/pr.yml); Windows nightly. No new CI step required.

### References

- Source story: [`epics.md:720-740`](_bmad-output/planning-artifacts/epics.md#L720-L740) — Story 1.18 user-story + AC + Traces.
- Architecture (canonical LD-40): [`architecture.md:1188-1194`](_bmad-output/planning-artifacts/architecture.md#L1188-L1194) — LD-40 Vault-self-contained state with 2026-05-20 TOML amendment.
- Architecture (filesystem boundary): [`architecture.md:1035`](_bmad-output/planning-artifacts/architecture.md#L1035) — Settings store sits in the App config zone, not the Vault zone (except per-Vault settings.toml which lives in `<Vault>/.orgsidian/`).
- Architecture (stack-versions): [`architecture.md:194`](_bmad-output/planning-artifacts/architecture.md#L194) — `toml` crate pin (MIT/Apache-2.0) added for LD-40.
- Architecture (failure mode catalog): [`architecture.md:1202`](_bmad-output/planning-artifacts/architecture.md#L1202) — LD-41 row for "Config corruption" (settings.toml malformed) defines the Story 6.7+ fallback flow.
- PRD (OQ-7 resolution): [`prd.md:586`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L586) — dual-surface commitment.
- PRD (FR-23 keybinding remap): [`prd.md:411`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L411) — per-Vault remap persistence requirement.
- UX spec (dual-surface settings pattern): [`ux-design-specification.md:414`](_bmad-output/planning-artifacts/ux-design-specification.md#L414) — Sublime/VS Code dual-surface pattern reference.
- UX spec (Sublime "Settings as both GUI and text file"): [`ux-design-specification.md:344`](_bmad-output/planning-artifacts/ux-design-specification.md#L344).
- UX spec (no settings sprawl, anti-pattern): [`ux-design-specification.md:360`](_bmad-output/planning-artifacts/ux-design-specification.md#L360) — anti-template inherited from Obsidian.
- Previous story (Story 1.17 — WCAG CI gate): [`1-17-establish-wcag-2-1-aa-hard-ci-gate.md`](_bmad-output/implementation-artifacts/1-17-establish-wcag-2-1-aa-hard-ci-gate.md) — testing pattern + grep-smoke convention + dep-pin protocol.
- Crate dep convention: [`Cargo.toml:30-62`](Cargo.toml#L30-L62) — workspace `[workspace.dependencies]` table with inline comments.
- LEAF graph rule: [`deny.toml:186-196`](deny.toml#L186-L196) — `orgsidian-vault` may only be a direct dep of `orgsidian-core`.
- Atomic-write surface: [`crates/orgsidian-vault/src/lib.rs:1-23`](crates/orgsidian-vault/src/lib.rs#L1-L23) — Story 1.9 + Story 3.1-future signature.
- Existing settings touchpoints in epics: Story 4.6 (cross-platform keybindings), Story 6.7 (default themes), Story 7.2 (Today Dashboard section prefs), Story 7.5 (saved agenda presets), Story 8.1 (Quick Capture hotkey), Story 11.3 (Plain/Power Mode), Story 11.5 (coaching dismissals), Story 12.0 (Unlinked References), Story 12.3 (keybinding remap UI).
- `toml` crate (1.x) docs: <https://docs.rs/toml/latest/toml/>
- `dirs` crate (6.x) docs: <https://docs.rs/dirs/latest/dirs/>
- `proptest` crate (1.x) docs: <https://docs.rs/proptest/latest/proptest/>

### Previous Story Intelligence (from Story 1.17)

Relevant to Story 1.18:

- **`@axe-core/playwright` + `vitest` pin protocol** ([Story 1.17 AC variance recorded](_bmad-output/implementation-artifacts/1-17-establish-wcag-2-1-aa-hard-ci-gate.md#L531)): pin protocol = "spec literal" → bump to "current latest stable" at impl time per `[[feedback_version_policy]]`. Story 1.18 inherits this: pin `toml = "1"`, `dirs = "6"`, `proptest = "1"` at workspace level (caret = "compatible-1.x"); verify post-`cargo update` that the actual locked version is the latest 1.x at impl time. No spec-literal-version mismatch this time because the spec says "latest stable" verbatim per [architecture.md:194](_bmad-output/planning-artifacts/architecture.md#L194).
- **Grep-smoke traceability pattern** ([Story 1.17 AC7](_bmad-output/implementation-artifacts/1-17-establish-wcag-2-1-aa-hard-ci-gate.md#L334)): Story 1.18 reuses this pattern verbatim — assert `LD-40 + FR-23` appears in ≥6 files via a `tests/settings.rs` smoke. The Rust-side equivalent of the AC7 JSDoc-or-grep convention is the leading `//!` doc-comment on each `.rs` file under the module + the grep-smoke.
- **Deferred-work stanza convention** ([Story 1.17 final commit `758153e`](_bmad-output/implementation-artifacts/deferred-work.md#L115)): every code-review pass appends a `## Deferred from: code review of story-1.NN (YYYY-MM-DD)` section. Story 1.18 pre-seeds the `toml_edit` migration deferral per AC3 scope note.
- **License-allowlist protocol** ([Story 1.17 Latest Technical Information](_bmad-output/implementation-artifacts/1-17-establish-wcag-2-1-aa-hard-ci-gate.md#L495)): `cargo deny check licenses bans advisories` + `pnpm run audit:licenses:js` (JS side N/A for Story 1.18) post-impl. No `deny.toml` modification unless an unexpected transitive license surfaces.
- **GitHub issues-sync flow** ([Story 1.17 metadata-fix lesson](_bmad-output/implementation-artifacts/1-17-establish-wcag-2-1-aa-hard-ci-gate.md#L532)): leave `github_issue:` BLANK in the story file metadata. The LD-55 issues-sync (Story 1.16) auto-creates an issue on push-to-main when this story file lands; do NOT pre-assign a number like `18` (Story 1.17 hit a `github_issue: 17` collision because issue #17 was already claimed by a different LD-55 sync of Story 2.1). Fill in the real number AFTER the first push.
- **Commit message convention** ([Story 1.17 commit `7100ece`](_bmad-output/implementation-artifacts/deferred-work.md#L115)): `feat(<scope>): wire LD-NN <one-line summary> (Story 1.NN, closes #<issue-num>)` — scope = `core` for Story 1.18 (the primary touched crate). NO Co-Authored-By trailer, NO Generated-with-Claude-Code footer per [[feedback_no_co_author_credit]].

### Git Intelligence Summary

Recent commits relevant to Story 1.18:

- **`a530a31`** (Merge PR #135 — Story 1.17): WCAG 2.1 AA hard CI gate. Pattern: `feat(<area>): wire LD-NN <X>` commit titles. No `crates/orgsidian-core/` changes; Story 1.18 is the first `core/` change since `7100ece` (also Story 1.17, contrast.test.ts under `shell-ui/`).
- **`9e2d662`** (Story 1.16): added `tools/issues-sync` Cargo binary (outside workspace), demonstrated the `# Story 1.NN: ...` Cargo.toml comment style.
- **`93df7b4`** (Story 1.15) / **`22bbb24`** (Story 1.14): pure CI workflow changes. No Rust code.
- **No recent commit touches `crates/orgsidian-core/src/`** beyond the Story 1.4/1.8/1.11/1.12 baseline. Story 1.18 is the first to add a real sub-module under `core/src/` (settings, joining `error.rs` + `registry.rs` + the existing `test_support/` sub-module).
- **No recent commit touches `orgsidian-vault`** beyond Story 1.9's `atomic_write` surface. Story 1.18 is the first downstream consumer of that surface.
- **`crates/orgsidian-shell-app/src/lib.rs` last touched by Story 1.4** (typed IPC bridge). Story 1.18 adds ~8 lines to the existing `setup` closure; preserves the existing structure.

### Latest Technical Information

**Verify versions at implementation time** (per [[feedback_version_policy]] — pin to latest stable; LTS preferred):

- **`toml` crate**: latest stable is the 1.x line (verified `1.1.2+spec-1.1.0` at story-write time; the `+spec-1.1.0` build-metadata suffix is the TOML grammar version, not a Cargo SemVer marker). Pin `toml = "1"` at workspace; the lockfile resolves to the actual latest 1.x. API: `toml::from_str::<T>(&s) -> Result<T, toml::de::Error>` for deserialize; `toml::to_string_pretty(&value) -> Result<String, toml::ser::Error>` for deterministic pretty serialize. The `toml::Table` type is a re-export of `toml::value::Table` = `BTreeMap<String, toml::Value>` and is the correct type for the `_extra` flatten catch-all. License: MIT/Apache-2.0.
- **`dirs` crate**: latest stable is `6.x` (verified `6.0.0` at story-write time). API: `dirs::config_dir() -> Option<PathBuf>` returns the OS-conventional config dir per [architecture.md LD-40](_bmad-output/planning-artifacts/architecture.md#L1191) — `~/.config` on Linux/BSD, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows. Returns `None` only on extremely degraded environments (no HOME var). License: MIT/Apache-2.0. Alternative `directories` crate is also acceptable (slightly higher-level wrapper) — Story 1.18 picks `dirs` for minimum surface area; if the dev agent prefers `directories`, document the choice in Completion Notes.
- **`proptest` crate**: latest stable is `1.x` (verified `1.7+` at story-write time). API: `proptest! { #[test] fn name(s in strategy()) { ... } }`. Strategy combinators: `proptest::collection::vec`, `proptest::option::of`, `proptest::sample::select`, `prop_oneof!`. License: MIT/Apache-2.0.
- **`toml_edit` (DEFERRED, not adopted in Story 1.18)**: latest stable `0.25.x`. Format-preserving TOML editor (preserves comments + ordering + whitespace). Story 12.3 (FR-23 keybinding remap GUI) is the planned adoption window; Story 1.18 sticks with the structural `toml` crate per AC3 scope-note rationale.
- **`serde`/`serde::Deserialize`**: 1.x (workspace pinned). `#[serde(flatten)]` on a `toml::Table` field is the canonical catch-all for unknown TOML keys. `#[serde(default)]` on struct + per-field defaults is the canonical forward-compat pattern for new-fields-in-old-files reads.
- **`specta` derive**: pinned `=2.0.0-rc.25` at workspace per [Cargo.toml line 40](Cargo.toml#L40). All public types in `settings/schema.rs` derive `specta::Type`. Verify build with `cargo test --test export_bindings` post-impl — the bindings should regenerate cleanly even without new `#[tauri::command]` wrappers (the derive alone exports the type).
- **`thiserror` 1.x**: existing workspace dep. `SettingsError` follows the `OrgError` precedent at [`crates/orgsidian-core/src/error.rs`](crates/orgsidian-core/src/error.rs).

### Project Context Reference

The repository's project context lives across:
- [`_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) — PRD (§4.3 FR-23, §10 OQ-7).
- [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md) — Architecture (LD-12, LD-37, LD-40, LD-41 row "Config corruption", filesystem-boundary §, stack-versions table).
- [`_bmad-output/planning-artifacts/ux-design-specification.md`](_bmad-output/planning-artifacts/ux-design-specification.md) — UX spec (dual-surface pattern, anti-settings-sprawl).
- [`_bmad-output/planning-artifacts/epics.md`](_bmad-output/planning-artifacts/epics.md) — Epics + Stories (this story at line 720-740).

The PRD + Architecture were finalized 2026-05-19 with the 2026-05-20 UXD-reconciliation closing the loop. 51 LDs locked; LD-40 carries the 2026-05-20 TOML-amendment that Story 1.18 implements.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context) via Claude Code, invoked through the `bmad-dev-story` skill.

### Debug Log References

### Completion Notes List

**Story 1.18 wraps with 21 net-new tests:** 10 unit + 7 integration (6 round-trip + 1 proptest, 256 cases) + 3 boundary-doc drift guards + 1 grep-smoke traceability. `cargo test --workspace` (with default + `--features test-support` matrix) all green. `cargo deny check` green (advisories ok, bans ok, licenses ok, sources ok — 5 pre-existing "unused wrapper" warnings unchanged; no new entries in `[bans].skip`). `cargo audit` shows the same 18 allowed warnings as the Story 1.17 baseline (no new advisories surfaced by `toml`, `dirs`, or `proptest`).

**Variance notes:**

- **`schema_version` is a top-level TOML scalar, not under `[meta]`.** TOML grammar requires top-level keys before `[table]` headers. The epic AC text "[meta] schema_version = 1" was loose phrasing; the implementation uses a top-level `schema_version = N` field. The file-header comment `# === Orgsidian settings — schema v1 (LD-40) ===` carries the "meta" framing per Dev Notes §6. The dedicated `meta.rs` module holds the file-header constant — the trace-grep counts it as one of the 6 LD-40+FR-23-annotated files.
- **`dirs` over `directories`.** Story 1.18 picked `dirs` (lower surface area) per Dev Notes guidance; no functional impact on AC1's path-resolution contract.
- **`#[specta(skip)]` on `_extra` field.** `specta::Type` does not derive cleanly for `toml::Table`; skipping the field from the TS export is safe because it is a forward-compat catch-all without semantic shape (consumers should not depend on it).
- **`required-features = ["test-support"]` not added to new `[[test]]` blocks.** The boundary-doc + grep-smoke tests do not depend on `test_support/perf.rs`; they read fixtures directly. They run under both `cargo test -p orgsidian-core` and `cargo test --workspace` without the feature flag because their entry points compile independently — only the lib-test `cfg(test)` path drags `test_support/perf.rs` in, and that path is gated by `cargo test --features test-support` already.
- **Issue created manually pre-implementation.** `gh issue list --search '[Story 1.18] in:title'` returned empty pre-flight; per the bmad-dev-story prepend step, created GitHub issue #136 with `status:backlog` then transitioned to `status:in-progress`. The LD-55 `sync-issues` workflow will not auto-create a duplicate because the title `[Story 1.18] ...` matches its dedupe key — if a duplicate ever surfaces it's a one-off cleanup. Confirmed with user before creating.

**Test count breakdown:**

- `src/settings/schema.rs::tests` (3 unit tests): `schema_version_rejects_future_version`, `schema_version_default_equals_current`, `extra_table_round_trips`.
- `src/settings/vault.rs::tests` (5 unit tests): `vault_settings_path_joins_dotorgsidian`, `read_returns_default_when_file_missing`, `write_creates_dotorgsidian_dir`, `parse_failure_surfaces_parse_failed_variant`, `write_then_read_round_trips`.
- `src/settings/global.rs::tests` (2 unit tests): `global_settings_path_resolves_under_config_dir`, `read_returns_default_when_global_file_missing` (guarded: skips when developer has a real `global.toml`).
- `crates/orgsidian-core/tests/settings_round_trip.rs` (7 integration tests including 1 proptest with 256 cases): `default_vault_settings_round_trip`, `default_global_settings_round_trip`, `populated_vault_settings_round_trip`, `writer_fixed_point`, `unknown_fields_preserved`, `schema_version_one_present_on_default_write`, `vault_settings_round_trip_property`.
- `tests/settings_boundary_doc.rs` (3 drift guards): `required_section_headings_present`, `ephemeral_allowlist_has_exactly_four_entries`, `schema_field_names_present`.
- `tests/settings.rs` (1 grep-smoke): `ld40_fr23_trace_appears_in_at_least_six_files`.

Total: **21 tests** (AC7 threshold ≥16). Plus the file-trace count: 6 `.rs` files under `src/settings/` (mod.rs, schema.rs, vault.rs, global.rs, meta.rs, error.rs) all carry the `LD-40 + FR-23` annotation.

### File List

**New (Rust source):**

- `crates/orgsidian-core/src/settings/mod.rs`
- `crates/orgsidian-core/src/settings/schema.rs`
- `crates/orgsidian-core/src/settings/vault.rs`
- `crates/orgsidian-core/src/settings/global.rs`
- `crates/orgsidian-core/src/settings/error.rs`
- `crates/orgsidian-core/src/settings/meta.rs`
- `crates/orgsidian-core/tests/settings_round_trip.rs`
- `tests/settings_boundary_doc.rs`
- `tests/settings.rs`

**New (docs):**

- `docs/architecture/settings-boundary.md`

**Modified:**

- `Cargo.toml` — added `toml = "1"`, `dirs = "6"`, `proptest = "1"`, `orgsidian-vault` to `[workspace.dependencies]`.
- `crates/orgsidian-core/Cargo.toml` — added crate-level deps; declared `settings_boundary_doc` + `settings_trace` `[[test]]` blocks.
- `crates/orgsidian-core/src/lib.rs` — `pub mod settings;` wire.
- `crates/orgsidian-shell-app/Cargo.toml` — added `tracing` workspace dep.
- `crates/orgsidian-shell-app/src/lib.rs` — bootstrap smoke for `read_global_settings`.
- `ARCHITECTURE.md` — crates-table row for `orgsidian-core` references the new `settings` module + boundary doc.
- `_bmad-output/implementation-artifacts/deferred-work.md` — Story 1.18 deferred stanza (5 items).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — story status `ready-for-dev` → `in-progress` (will be flipped to `review` at workflow Step 9).
- `_bmad-output/implementation-artifacts/1-18-toml-settings-authoritative-store-with-hybrid-boundary.md` — `github_issue: 136`, status, tasks ticked, Dev Agent Record.
- `Cargo.lock` — auto-updated by `cargo` for new direct deps.

## Change Log

- 2026-06-02 — Story 1.18 implementation pass: TOML settings authoritative store wired (`crates/orgsidian-core/src/settings/`), boundary doc + drift guard + grep-smoke traceability tests, shell-app bootstrap smoke for `read_global_settings`. 21 net-new tests; `cargo test --workspace`, `cargo deny`, `cargo audit` all green. Issue #136 created + transitioned to `status:in-progress`.
