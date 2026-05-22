//! The plugin lifecycle trait (`OrgsidianPlugin`).
//!
//! Required methods (`metadata`, `init`, `shutdown`) come straight from
//! LD-26. Optional hooks (`priority`, `on_event`, `on_save_before`,
//! `on_capture_before`, `on_agenda_query_after`) carry default `Continue`
//! / `Ok(())` / `0` implementations so plugins opt in by overriding — this
//! mirrors Emacs Org-mode's `:before` / `:after` / `:around` advice pattern
//! cited as the design inspiration in LD-26.

// rationale: every default-impl method below ignores `self` because the
// default body is a no-op the host calls on plugins that did not override.
// Adding `#[must_use]` per Dev Notes "Specific lints to expect" does NOT
// propagate through `dyn OrgsidianPlugin` dispatch reliably, so we keep the
// signatures clean and silence the pedantic noise at module level.
#![allow(
    clippy::must_use_candidate,
    clippy::unused_self,
    clippy::needless_pass_by_ref_mut
)]

use crate::context::{HookContext, PluginContext};
use crate::event::Event;
use crate::metadata::PluginMetadata;
use crate::outcome::HookOutcome;
use crate::payload::{AgendaItem, AgendaQuery, CaptureEntry};
use crate::Result;

/// The plugin lifecycle trait — every Orgsidian plugin (bundled or, post
/// v1.5+, third-party) implements this.
///
/// The `Send + Sync` super-bounds are mandatory: the host invokes plugins
/// from a `Vec<Box<dyn OrgsidianPlugin>>` registry accessed across the
/// async runtime per LD-25.
///
/// Context parameters are `&dyn PluginContext` / `&dyn HookContext` (LD-5
/// round-4 amendment): dynamic dispatch keeps the trait object-safe and
/// the v1.5+ WASM transition mechanical through a single wasmtime-bound
/// vtable.
pub trait OrgsidianPlugin: Send + Sync {
    /// Returns the plugin's metadata (id, name, version, author).
    ///
    /// Called by the host at registration time and whenever the Settings UI
    /// needs to render the plugin list (Stories 12.x).
    fn metadata(&self) -> PluginMetadata;

    /// Called once at plugin load.
    ///
    /// The plugin receives a borrowed [`PluginContext`] — no ownership
    /// transfer keeps the surface WASM-compatible per LD-25.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Init`] if the plugin cannot complete
    /// its startup work; the host treats `init` failure as a load failure
    /// and the plugin is excluded from the registry.
    fn init(&mut self, ctx: &dyn PluginContext) -> Result<()>;

    /// Called once at plugin unload / app shutdown.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Runtime`] if the plugin cannot complete
    /// its shutdown work; the host logs the error but otherwise proceeds
    /// with shutdown (errors here are advisory, not fatal).
    fn shutdown(&mut self) -> Result<()>;

    /// Plugin priority for hook dispatch ordering.
    ///
    /// Lower values run first; ties resolve by plugin load order. Default
    /// is `0` so plugins that do not care opt out of the ordering question.
    fn priority(&self) -> i32 {
        0
    }

    /// Fire-and-forget observer.
    ///
    /// Default no-op so plugins opt in by overriding. Used for logging,
    /// badges, sync-to-external integrations, and similar non-blocking
    /// observations.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Runtime`] if the plugin's observation
    /// logic errors. The host logs the error; the failure does NOT cancel
    /// the underlying operation that emitted the event.
    fn on_event(&mut self, _event: &Event) -> Result<()> {
        Ok(())
    }

    /// Pre-save hook — plugin may transform content before write.
    ///
    /// Default returns [`HookOutcome::Continue`] so the host proceeds with
    /// the original content; plugins opt in by overriding and returning
    /// [`HookOutcome::Replace`] or [`HookOutcome::Cancel`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Runtime`] if the plugin's transform
    /// errors. The host treats this the same way as
    /// `HookOutcome::Cancel`: the save is aborted and the error is
    /// surfaced.
    fn on_save_before(
        &mut self,
        _ctx: &dyn HookContext,
        _content: &str,
    ) -> Result<HookOutcome<String>> {
        Ok(HookOutcome::Continue)
    }

    /// Pre-capture hook — plugin may transform a Quick Capture entry before
    /// commit.
    ///
    /// Default returns [`HookOutcome::Continue`]; plugins opt in by
    /// overriding.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Runtime`] if the plugin's transform
    /// errors. The host aborts the capture and surfaces the error.
    fn on_capture_before(
        &mut self,
        _ctx: &dyn HookContext,
        _entry: &CaptureEntry,
    ) -> Result<HookOutcome<CaptureEntry>> {
        Ok(HookOutcome::Continue)
    }

    /// Agenda query post-process hook — plugin may mutate the result set.
    ///
    /// Default no-op so plugins opt in by overriding. Common uses: hiding
    /// rows the plugin manages internally, re-ordering by a plugin-specific
    /// priority, decorating `display_text`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::PluginError::Runtime`] if the plugin's
    /// post-processing errors. The host logs the error and surfaces the
    /// **current state** of `results` (which may be partially mutated —
    /// the `&mut Vec<AgendaItem>` was already handed to the plugin and
    /// the host does not snapshot upstream). Plugins SHOULD therefore
    /// treat `results` as transactional: defer mutations until they are
    /// certain the call will return `Ok`, or restore the original state
    /// before returning `Err`.
    fn on_agenda_query_after(
        &mut self,
        _ctx: &dyn HookContext,
        _query: &AgendaQuery,
        _results: &mut Vec<AgendaItem>,
    ) -> Result<()> {
        Ok(())
    }
}
