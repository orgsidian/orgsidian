//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Schema types are LOCKED here as the v0.1 baseline. Downstream stories
//! EXTEND the schema (add fields with `#[serde(default)]` for forward-compat)
//! rather than redesigning. The forward-compat catch-all field `_extra:
//! toml::Table` on every top-level struct preserves unknown keys across a
//! `read → write` round-trip — a v2-shipped field round-trips through a v1
//! binary intact.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

/// Bump on every backward-incompatible field-meaning change. LD-12 mirror:
/// forward-only migration discipline. Bumping = new app reads old files via
/// `#[serde(default)]`; old app reads new files via `_extra` catch-all.
pub const SCHEMA_VERSION_CURRENT: u32 = 1;

/// Newtype wrapping the on-disk `schema_version` value. Deserialization rejects
/// versions greater than [`SCHEMA_VERSION_CURRENT`] — a v1 binary cannot safely
/// interpret a v2 file because v2's semantic changes are unknown. Future
/// Story 6.7+ relaxes this to a "warn + best-effort read" when the dirty-buffer
/// LD-7 hook lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(transparent)]
pub struct SchemaVersion(pub u32);

impl Default for SchemaVersion {
    fn default() -> Self {
        Self(SCHEMA_VERSION_CURRENT)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = u32::deserialize(deserializer)?;
        if raw > SCHEMA_VERSION_CURRENT {
            return Err(serde::de::Error::custom(format!(
                "schema_version {} is newer than supported version {}",
                raw, SCHEMA_VERSION_CURRENT
            )));
        }
        Ok(SchemaVersion(raw))
    }
}

/// FR-22 active theme. Absolute path or default-light/default-dark sentinel.
/// Story 6.7 lands the user-CSS loader; schema-shape locked here.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    #[default]
    DefaultLight,
    DefaultDark,
    /// Absolute path to a user-supplied CSS file (Story 6.7 / 12.1).
    Custom(PathBuf),
}

/// FR-20 Plain/Power Mode preference (Story 11.3 lands the runtime toggle).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum UiMode {
    #[default]
    Plain,
    Power,
}

/// FR-7 saved named agenda filter preset (Story 7.5 lands the UI).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(default)]
pub struct AgendaPreset {
    /// "today" | "week" | "custom". Semantics finalized in Story 7.5.
    pub view: String,
    /// Free-form tag/TODO-state filter; semantics finalized in Story 7.5.
    pub filters: Vec<String>,
}

/// FR-6 Today Dashboard section preferences (Story 7.2 lands the toggles).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(default)]
pub struct TodayDashboardSections {
    pub show_scheduled: bool,
    pub show_deadlines: bool,
    pub show_clock: bool,
    pub show_inbox: bool,
}

impl Default for TodayDashboardSections {
    fn default() -> Self {
        Self {
            show_scheduled: true,
            show_deadlines: true,
            show_clock: true,
            show_inbox: true,
        }
    }
}

/// Per-Vault authoritative settings. Lives at `<Vault>/.orgsidian/settings.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(default)]
pub struct VaultSettings {
    /// Mandatory header. `SchemaVersion(1)` for v0.1.
    pub schema_version: SchemaVersion,

    /// FR-23 keybinding remap (Story 12.3 lands the UI; schema-shape locked here).
    /// Key = canonical action ID (e.g., "editor.save"); value = chord string ("Cmd+S").
    /// Stored sorted by key for deterministic round-trip.
    pub keybindings: BTreeMap<String, String>,

    /// FR-22 active theme.
    pub theme: ThemeChoice,

    /// FR-10 Quick Capture global hotkey (Story 8.1 lands the wiring).
    pub capture_hotkey: Option<String>,

    /// FR-7 saved named agenda filter presets.
    pub agenda_presets: BTreeMap<String, AgendaPreset>,

    /// FR-21 dismissed coaching IDs (Story 11.5 lands the persist).
    pub dismissed_coaching: BTreeSet<String>,

    /// FR-20 Plain/Power Mode preference.
    pub ui_mode: UiMode,

    /// FR-6 Today Dashboard section preferences.
    pub today_dashboard: TodayDashboardSections,

    /// Forward-compat catch-all: unknown top-level keys land here on read, are
    /// preserved on write. Skipped on serialize when empty (per `toml::Table`'s
    /// default `Serialize` impl for empty maps in `toml = "1"`).
    #[serde(flatten)]
    #[specta(skip)]
    pub _extra: toml::Table,
}

/// Global state shared across Vaults. Lives at `<config-dir>/orgsidian/global.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(default)]
pub struct GlobalSettings {
    pub schema_version: SchemaVersion,

    /// LD-40: list of recent Vault paths. Ordered most-recent-first. Capped at
    /// 10 by callers (deduped).
    pub recent_vaults: Vec<PathBuf>,

    /// Default UI language (LD-52 / lingui locale code, e.g. "en", "it").
    /// `None` = OS locale.
    pub default_language: Option<String>,

    /// Default theme for new Vaults.
    pub default_theme: ThemeChoice,

    /// Forward-compat catch-all (see [`VaultSettings::_extra`]).
    #[serde(flatten)]
    #[specta(skip)]
    pub _extra: toml::Table,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_rejects_future_version() {
        // schema_version > SCHEMA_VERSION_CURRENT must deserialize to an error.
        let raw = format!("schema_version = {}\n", SCHEMA_VERSION_CURRENT + 1);
        let result: Result<VaultSettings, _> = toml::from_str(&raw);
        assert!(
            result.is_err(),
            "expected ParseFailed-equivalent toml::de::Error for future schema_version, got Ok"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("schema_version") && msg.contains("newer"),
            "error message should call out schema_version mismatch, got: {msg}"
        );
    }

    #[test]
    fn schema_version_default_equals_current() {
        assert_eq!(
            SchemaVersion::default(),
            SchemaVersion(SCHEMA_VERSION_CURRENT)
        );
    }

    #[test]
    fn extra_table_round_trips() {
        // Manual TOML with an unknown top-level key — must land in `_extra` on
        // deserialize and re-emerge on serialize.
        let raw = "\
schema_version = 1
forward_compat_key = \"survives\"

[forward_compat_table]
nested = 42
";
        let v: VaultSettings = toml::from_str(raw).expect("parse with unknown keys");
        assert!(v._extra.contains_key("forward_compat_key"));
        assert!(v._extra.contains_key("forward_compat_table"));
        let written = toml::to_string_pretty(&v).expect("serialize");
        assert!(written.contains("forward_compat_key"));
        assert!(written.contains("forward_compat_table"));
        assert!(written.contains("nested = 42"));
    }
}
