//! Plugin identity metadata reported at load time.

// rationale: `metadata::PluginMetadata` repeats the module name. Consumers
// always reach the type via the crate-root re-export
// (`orgsidian_plugin_api::PluginMetadata`); the `Plugin` prefix is what
// disambiguates from generic `Metadata` types in third-party crates.
#![allow(clippy::module_name_repetitions)]

/// Plugin identity, reported at load time and returned by
/// [`crate::OrgsidianPlugin::metadata`].
///
/// `specta::Type` is **not** derived here on purpose: this crate must stay
/// LEAF, so when the host needs to surface plugin metadata over `tauri-specta`
/// IPC (Settings UI plugin list, Stories 12.x) it will define a façade type
/// in `orgsidian-core` that wraps `PluginMetadata` and derives `specta::Type`
/// itself. Keeps `orgsidian-plugin-api` crates.io-publishable at v1.5+ per
/// LD-10.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    /// Stable plugin identifier (e.g., `"agenda"`, `"quick-capture"`,
    /// `"themes"`). Used as the key in the host's plugin registry; MUST be
    /// unique per running app.
    pub id: String,
    /// Human-readable display name (shown in Settings UI plugin list).
    pub name: String,
    /// `SemVer` string (e.g., `"1.2.3"`). Plugin authors track their own
    /// version independently of the host app version.
    pub version: String,
    /// Author or organisation that published the plugin.
    pub author: String,
}
