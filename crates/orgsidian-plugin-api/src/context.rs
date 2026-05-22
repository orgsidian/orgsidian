//! Host capability surface exposed to plugins.
//!
//! Both [`PluginContext`] and [`HookContext`] are **traits** (LD-5 round-4
//! amendment), passed to plugins as `&dyn` references. The trait split
//! mirrors the lifecycle / hook boundary in [`crate::OrgsidianPlugin`]:
//!
//! - [`PluginContext`] — borrowed at `init`, lifetime = plugin lifetime.
//! - [`HookContext`] — borrowed at hook invocation, lifetime = hook frame.
//!
//! Both traits carry the `Send + Sync` super-bound because plugin invocation
//! happens from the host's async runtime and the
//! `Vec<Box<dyn OrgsidianPlugin>>` registry per LD-25.

use crate::event::Event;
use crate::metadata::PluginMetadata;
use crate::Result;

/// Read-only handle to host capabilities available for the lifetime of a
/// plugin, passed to [`crate::OrgsidianPlugin::init`].
///
/// Day-1 surface is intentionally minimal — only the loop-back metadata
/// accessor. Locking the trait **name** + `Send + Sync` super-bound now
/// lets every consuming story extend the trait as SemVer-minor additive
/// methods per LD-26.
pub trait PluginContext: Send + Sync {
    /// Returns this plugin's metadata as registered at load time.
    ///
    /// Useful for plugins that need to inspect their own host-resolved
    /// identity (e.g., the host may have assigned a stable `id` derived
    /// from the crate name when registering bundled plugins).
    fn plugin_metadata(&self) -> &PluginMetadata;
}

/// Borrowed-for-the-hook-frame handle to host capabilities, passed to every
/// transform / observation hook on [`crate::OrgsidianPlugin`].
///
/// Plugins MUST NOT retain a `&dyn HookContext` reference across calls —
/// the lifetime is bound to the hook frame to keep the surface
/// WASM-compatible per LD-25 + LD-26.
///
/// The day-1 surface deliberately omits the structured `tracing` logger
/// mentioned in the architecture LD-26 prose: adding `tracing` to a leaf
/// crate would pollute the publishable surface with a heavyweight
/// transitive. The logger lands as a SemVer-minor additive method when the
/// first plugin author actually needs it.
pub trait HookContext: Send + Sync {
    /// Reads a file from the active Vault.
    ///
    /// `path` is Vault-relative; the host enforces the Vault allow-list per
    /// LD-17 — paths that escape the active Vault MUST be rejected by the
    /// host implementation.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::HostUnavailable`] if no Vault is
    /// currently active, or [`crate::PluginError::InvalidInput`] if `path`
    /// is malformed or escapes the Vault allow-list.
    fn read_vault_file(&self, path: &str) -> Result<String>;

    /// Queries the index.
    ///
    /// The day-1 surface accepts an opaque query string and returns an
    /// opaque result string. A structured `IndexQuery` enum + typed result
    /// shape will land when `orgsidian-index::query::*` materialises
    /// (Stories 3.x / 8.x); the `String → String` shape is the deliberate
    /// LEAF-preserving escape hatch and is replaceable as a SemVer-minor
    /// additive method per LD-26.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::HostUnavailable`] if the index has not
    /// finished its first scan, or [`crate::PluginError::InvalidInput`] if
    /// `query` is malformed.
    fn query_index(&self, query: &str) -> Result<String>;

    /// Emits an event for fan-out to other plugins and host listeners.
    ///
    /// Ownership of `event` is transferred to the host: the by-value
    /// signature is deliberate so plugins yield the event after emit
    /// (avoiding accidental retention of stale event references across
    /// hook frames) and the host can fan-out / clone N-1 times for
    /// multiple observers without imposing a `Clone` round-trip on every
    /// caller. [`crate::Event`] derives `Clone`, so a plugin that wants
    /// to inspect the emitted value after the call can clone explicitly.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::HostUnavailable`] if the host event
    /// dispatcher is no longer accepting events (e.g., during shutdown).
    fn emit_event(&self, event: Event) -> Result<()>;
}
