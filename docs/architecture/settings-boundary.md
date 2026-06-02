# Settings Store Boundary (LD-40 + FR-23)

Authoritative reference for the dual-surface settings architecture. Every
Settings-touching story downstream MUST `grep` this file before adding a new
key to `tauri-plugin-store`. Default answer to "where does this setting live?"
is **TOML** — `tauri-plugin-store` is a closed allowlist of 4 ephemeral keys.

> **Story 1.18** wires the Rust module at
> [`crates/orgsidian-core/src/settings/`](../../crates/orgsidian-core/src/settings/)
> and freezes the v0.1 schema. Downstream stories EXTEND fields (with
> `#[serde(default)]` for forward-compat) but do NOT redesign.

## Authoritative Settings (TOML, OQ-7 dual-surface)

Stored at:

- **Per-Vault:** `<Vault>/.orgsidian/settings.toml` — `VaultSettings`
- **Global:** `<config-dir>/orgsidian/global.toml` — `GlobalSettings`
  (where `<config-dir>` resolves per `dirs::config_dir()`: `~/.config` on Linux,
  `~/Library/Application Support` on macOS, `%APPDATA%` on Windows).

| Field | TOML key | Owner story | UI surface (deferred to) |
| --- | --- | --- | --- |
| `VaultSettings::schema_version` | `schema_version` | Story 1.18 | — (system-managed; bumped on schema migration) |
| `VaultSettings::keybindings` | `keybindings` | Story 1.18 (schema) | Story 12.3 (FR-23 remap UI) |
| `VaultSettings::theme` | `theme` | Story 1.18 (schema) | Story 6.7 (default themes) / 12.1 (user-CSS) |
| `VaultSettings::capture_hotkey` | `capture_hotkey` | Story 1.18 (schema) | Story 8.1 (Quick Capture window) |
| `VaultSettings::agenda_presets` | `agenda_presets` | Story 1.18 (schema) | Story 7.5 (saved filter presets) |
| `VaultSettings::dismissed_coaching` | `dismissed_coaching` | Story 1.18 (schema) | Story 11.5 (persist + reset action) |
| `VaultSettings::ui_mode` | `ui_mode` | Story 1.18 (schema) | Story 11.3 (Plain/Power toggle) |
| `VaultSettings::today_dashboard` | `today_dashboard` | Story 1.18 (schema) | Story 7.2 (Today Dashboard sections) |
| `GlobalSettings::schema_version` | `schema_version` | Story 1.18 | — |
| `GlobalSettings::recent_vaults` | `recent_vaults` | Story 1.18 (schema) | Story 6.2 (starter Vault picker) |
| `GlobalSettings::default_language` | `default_language` | Story 1.18 (schema) | Story 1.6 (lingui locale) / future Settings UI |
| `GlobalSettings::default_theme` | `default_theme` | Story 1.18 (schema) | Story 6.7 |

## Ephemeral UI State (`tauri-plugin-store`-allowed)

This is the **closed allowlist** — any key NOT listed here MUST live in TOML.
Adding a new key to `tauri-plugin-store` requires extending this list AND a
matching ADR-style note in the PR description.

| Key | Type | Reset semantics |
| --- | --- | --- |
| `lastOpenFile` | string path | Survives crash; reset on first launch after fresh install. |
| `windowGeometry` | `{x, y, width, height, monitor}` | Managed by `tauri-plugin-window-state`, not directly written by app code; resets if monitor topology changes. |
| `tutorialProgress` | `{step, completed}` | Owned by Story 13.3; resets via "Replay tutorial" command. |
| `lastVaultPath` | single string path | DIFFERENT from `GlobalSettings::recent_vaults`. This is the "what to auto-reopen at startup" pointer; the user can reset by holding `Shift` on launch. |

## Forbidden Patterns

Each row below names a misuse of `tauri-plugin-store` and the canonical TOML
location that supersedes it. Reviewers should flag any PR that violates these.

| Anti-pattern | Canonical TOML location |
| --- | --- |
| Storing keybindings in `tauri-plugin-store` | `VaultSettings::keybindings` |
| Storing theme paths in `tauri-plugin-store` | `VaultSettings::theme` (or `GlobalSettings::default_theme`) |
| Storing agenda filter presets in `tauri-plugin-store` | `VaultSettings::agenda_presets` |
| Storing coaching dismissal IDs in `tauri-plugin-store` | `VaultSettings::dismissed_coaching` |
| Storing capture hotkeys in `tauri-plugin-store` | `VaultSettings::capture_hotkey` |
| Storing Today Dashboard section toggles in `tauri-plugin-store` | `VaultSettings::today_dashboard` |
| Storing UI Plain/Power mode preference in `tauri-plugin-store` | `VaultSettings::ui_mode` |
| Storing the recent-Vaults history in `tauri-plugin-store` | `GlobalSettings::recent_vaults` |

## Adding a New Setting (decision tree)

1. **Does the user expect to edit this in a text editor?**
   - YES → TOML (`VaultSettings` or `GlobalSettings`).
   - NO → continue.
2. **Does it survive a fresh app install?**
   - NO → `tauri-plugin-store` is OK — but you must first extend the
     ephemeral allowlist above with a PR note explaining why.
   - YES → TOML.
3. **Is it per-Vault or global?**
   - Per-Vault (changes when the user switches Vault): land in
     `VaultSettings` at `<Vault>/.orgsidian/settings.toml`.
   - Global (shared across Vaults): land in `GlobalSettings` at
     `<config-dir>/orgsidian/global.toml`.

When in doubt, default to TOML — the dual-surface OQ-7 commitment biases
toward "user-inspectable, hand-editable, file-authoritative".

## References

- **LD-40** — Vault-self-contained state with 2026-05-20 TOML amendment:
  [`_bmad-output/planning-artifacts/architecture.md#L1188-L1194`](../../_bmad-output/planning-artifacts/architecture.md)
- **OQ-7** — dual-surface settings commitment:
  [`prd.md §10 OQ-7`](../../_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md)
- **FR-23** — keybinding remap requirement:
  [`prd.md §4.3 FR-23`](../../_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md)
- **Story 1.18** — TOML settings authoritative store + this boundary doc:
  [`1-18-toml-settings-authoritative-store-with-hybrid-boundary.md`](../../_bmad-output/implementation-artifacts/1-18-toml-settings-authoritative-store-with-hybrid-boundary.md)
- **Implementation** — Rust module:
  [`crates/orgsidian-core/src/settings/`](../../crates/orgsidian-core/src/settings/)
