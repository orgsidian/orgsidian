//! Plugin registry + panic-isolation macro (LD-38 stub).
//!
//! This module ships in **Story 1.8** as a panic-isolation primitive. The
//! `PluginRegistry` struct is a stub that materializes incrementally:
//! - Story 1.8 (this story): the macro shape + a no-op registry that tracks
//!   `disabled_for_session` plugin IDs only.
//! - Future stories (post-Epic-1): registry mounts the real `Vec<Box<dyn
//!   OrgsidianPlugin>>` per LD-25 once a host consumer needs it.
//!
//! ### LD-38 contract
//!
//! Every plugin invocation site (real ones land in Epic 4+) MUST use the
//! `invoke_plugin_hook!` macro from this module. The macro:
//! - Wraps the call in `std::panic::catch_unwind` so a plugin panic does NOT
//!   propagate past the host process boundary.
//! - On panic: logs via `tracing::error!` with the plugin's metadata.id,
//!   marks the plugin as `disabled_for_session` in the registry, and
//!   substitutes a default value so the caller's control flow continues.
//! - The `[profile.release] panic = "unwind"` override in workspace
//!   `Cargo.toml` is what makes `catch_unwind` actually catch under
//!   `--release` (Rust default is `panic = "abort"` which would terminate the
//!   process before the handler runs).
//!
//! ### Why a stub now
//!
//! LD-38 is a day-1 architectural invariant: every future plugin invocation
//! site MUST go through this macro. Shipping the macro now (even with a
//! stub registry) means downstream stories add real hook calls through the
//! invariant rather than retrofitting it later (where the retrofit cost
//! grows linearly with the number of invocation sites).

use std::collections::HashSet;
use std::sync::Mutex;

/// Plugin registry (Story 1.8 stub).
///
/// Tracks `disabled_for_session` plugin IDs. Future stories grow this into
/// the real `Vec<Box<dyn OrgsidianPlugin>>` host registry per LD-25.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    disabled: Mutex<HashSet<String>>,
}

impl PluginRegistry {
    /// Construct a fresh, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a plugin as disabled for the rest of the process lifetime.
    ///
    /// Subsequent `is_disabled(id)` queries return `true`. The registry
    /// resets at process restart (no on-disk persistence) per LD-38 ("user
    /// can re-enable after restart").
    pub fn disable_for_session(&self, plugin_id: &str) {
        if let Ok(mut guard) = self.disabled.lock() {
            guard.insert(plugin_id.to_string());
        }
    }

    /// Returns `true` if the plugin has been marked disabled this session.
    #[must_use]
    pub fn is_disabled(&self, plugin_id: &str) -> bool {
        self.disabled
            .lock()
            .map(|guard| guard.contains(plugin_id))
            .unwrap_or(false)
    }
}

/// LD-38 panic-isolation macro.
///
/// Wraps a plugin hook invocation in `std::panic::catch_unwind`. On panic:
/// logs the failure via `tracing::error!`, marks the plugin as
/// `disabled_for_session` in `$registry`, and yields `$default` so the
/// caller's control flow continues.
///
/// ### Arguments
///
/// - `$registry: &PluginRegistry` — the host's registry (used to record the
///   session-disable on panic).
/// - `$plugin_id: &str` — the plugin's metadata.id (used in the log message
///   and the disable record).
/// - `$default: expr` — fallback value substituted into the call site when
///   the hook panics. For `on_event` (returns `Result<()>`) callers should
///   pass `Ok(())`; for `on_save_before` (returns `Result<HookOutcome<String>>`)
///   pass `Ok(HookOutcome::Continue)`; etc.
/// - `$call: expr` — the actual hook invocation (a closure or block that
///   does the work).
///
/// ### Why `AssertUnwindSafe`
///
/// `catch_unwind` requires its argument to be `UnwindSafe`. Hook closures
/// often capture `&mut dyn OrgsidianPlugin` (not `UnwindSafe`), so we wrap
/// in `AssertUnwindSafe` to acknowledge that we accept post-panic state
/// being potentially inconsistent — the plugin is about to be disabled
/// anyway, so logical consistency of its internal state no longer matters.
#[macro_export]
macro_rules! invoke_plugin_hook {
    ($registry:expr, $plugin_id:expr, $default:expr, $call:expr) => {{
        let registry: &$crate::registry::PluginRegistry = $registry;
        let plugin_id: &str = $plugin_id;
        if registry.is_disabled(plugin_id) {
            $default
        } else {
            let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| $call));
            match result {
                Ok(value) => value,
                Err(_panic) => {
                    ::tracing::error!(
                        target: "orgsidian::plugin",
                        plugin_id = plugin_id,
                        "plugin panicked in hook; disabling for session per LD-38",
                    );
                    registry.disable_for_session(plugin_id);
                    $default
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disable_and_check() {
        let registry = PluginRegistry::new();
        assert!(!registry.is_disabled("foo"));
        registry.disable_for_session("foo");
        assert!(registry.is_disabled("foo"));
        assert!(!registry.is_disabled("bar"));
    }

    #[test]
    fn test_macro_returns_value_on_ok() {
        let registry = PluginRegistry::new();
        let value: i32 = invoke_plugin_hook!(&registry, "p1", -1_i32, { 42_i32 });
        assert_eq!(value, 42);
        assert!(!registry.is_disabled("p1"));
    }

    #[test]
    fn test_macro_catches_panic_and_returns_default() {
        // Suppress the default panic hook printout for this test only — the
        // panic is *expected* and the macro's `catch_unwind` handles it.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let registry = PluginRegistry::new();
        let value: i32 = invoke_plugin_hook!(&registry, "p1", -1_i32, {
            panic!("boom");
            #[allow(unreachable_code)]
            0_i32
        });

        std::panic::set_hook(prev_hook);

        assert_eq!(value, -1);
        assert!(registry.is_disabled("p1"));
    }

    #[test]
    fn test_macro_short_circuits_on_already_disabled() {
        let registry = PluginRegistry::new();
        registry.disable_for_session("p1");

        // If the body were entered, this would panic — the assertion below
        // therefore proves the short-circuit path was taken.
        let value: i32 = invoke_plugin_hook!(&registry, "p1", -1_i32, {
            panic!("would-panic-if-entered");
            #[allow(unreachable_code)]
            0_i32
        });
        assert_eq!(value, -1);
        assert!(registry.is_disabled("p1"));
    }
}
