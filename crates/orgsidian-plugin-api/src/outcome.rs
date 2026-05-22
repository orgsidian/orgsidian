//! Hook-return discriminator: what a plugin asks the host to do after a hook.
//!
//! `HookOutcome<T>` is the canonical return shape for transform hooks
//! (`on_save_before`, `on_capture_before`, …) per LD-26.

/// Outcome of a transform hook.
///
/// - `Continue` — plugin had nothing to do; host proceeds with the original
///   value.
/// - `Replace(T)` — plugin returns a transformed value; host substitutes it
///   for the original.
/// - `Cancel(String)` — plugin vetoes the operation; host surfaces the
///   carried reason in error UI / logs.
///
/// A 4th `Defer(Duration)` variant for async escape-hatch semantics is
/// reserved for the LD-50 v0.5 surface review and is deliberately NOT
/// shipped here. The enum is marked `#[non_exhaustive]` so the future
/// variant lands as a SemVer-minor additive bump per LD-26 — consumers
/// MUST include a wildcard `_` arm when matching.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum HookOutcome<T> {
    /// Plugin had nothing to do; host proceeds with the original value.
    Continue,
    /// Plugin transformed the value; host substitutes it for the original.
    Replace(T),
    /// Plugin vetoes the operation; host surfaces the carried reason as a
    /// user-visible message in error UI / logs.
    Cancel(String),
}
