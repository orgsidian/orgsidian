//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Public surface — authoritative TOML settings store at
//! `<Vault>/.orgsidian/settings.toml` (per-Vault) and
//! `<config-dir>/orgsidian/global.toml` (global). Every downstream Settings-
//! touching story consumes `read/write_*_settings` from this module from day 1,
//! per the LD-40 2026-05-20 amendment.

pub mod error;
pub mod global;
pub mod meta;
pub mod schema;
pub mod vault;

pub use error::{SettingsError, SettingsResult};
pub use global::{global_settings_path, read_global_settings, write_global_settings};
pub use schema::{GlobalSettings, SchemaVersion, VaultSettings, SCHEMA_VERSION_CURRENT};
pub use vault::{read_vault_settings, vault_settings_path, write_vault_settings};
