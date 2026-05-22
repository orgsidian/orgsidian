//! Error vocabulary for the plugin API.
//!
//! `PluginError` is intentionally **separate** from `orgsidian_core::OrgError`
//! to preserve the LEAF invariant (LD-5 + LD-10). The host-side
//! `orgsidian-core::registry` owns the `PluginError → OrgError` conversion when
//! that materialises in a later story; adding the `From` impl here would couple
//! the two crates and break the leaf invariant.

// rationale: `error::PluginError` repeats the module name, but consumers
// always reach the type via the crate-root re-export (`orgsidian_plugin_api::PluginError`).
// Renaming the module would push the lint to the type instead; renaming the type
// would lose the "Plugin" prefix that disambiguates from `std::error::Error`.
#![allow(clippy::module_name_repetitions)]

/// Result alias used by every fallible plugin API method.
///
/// This alias intentionally carries [`PluginError`] (NOT
/// `orgsidian_core::OrgError`) to keep the crate LEAF — see crate-level docs.
pub type Result<T> = std::result::Result<T, PluginError>;

/// Error vocabulary returned by the plugin API surface.
///
/// Marked `#[non_exhaustive]` so additional variants land as SemVer-minor
/// additions per LD-26 — consumers MUST include a wildcard `_` arm when
/// matching.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PluginError {
    /// Plugin initialisation failed (e.g., a required host capability was
    /// unavailable at `init`, or the plugin's own startup work errored).
    #[error("plugin init failed: {reason}")]
    Init {
        /// Free-form, plugin-author-supplied message describing the failure.
        reason: String,
    },
    /// Plugin runtime error raised from inside a hook (`on_event`,
    /// `on_save_before`, etc.) — the host surfaces this in logs and (where
    /// the hook outcome is fatal) the user-facing error UI.
    #[error("plugin runtime error: {reason}")]
    Runtime {
        /// Free-form, plugin-author-supplied message describing the failure.
        reason: String,
    },
    /// The plugin asked for a host capability that is not currently
    /// available (e.g., `query_index` before the index has finished its
    /// first scan, or `read_vault_file` outside the Vault allow-list per
    /// LD-17).
    #[error("host capability unavailable: {capability}")]
    HostUnavailable {
        /// Capability name (e.g., `"index"`, `"vault"`).
        capability: String,
    },
    /// Host passed input the plugin could not interpret (e.g., a malformed
    /// path string, an empty required field). Plugins return this to signal
    /// "your call site is wrong, not my internal state."
    #[error("invalid input from host: {reason}")]
    InvalidInput {
        /// Free-form message describing what was malformed.
        reason: String,
    },
}
