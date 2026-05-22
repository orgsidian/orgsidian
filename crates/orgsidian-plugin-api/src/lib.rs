//! `orgsidian-plugin-api`: the day-1 trait surface for the Orgsidian internal
//! Plugin Pattern (FR-24).
//!
//! This crate is **LEAF** — it has zero project dependencies, so it can be
//! published to crates.io without bundling host implementation crates. Per
//! LD-10 the crate stays internal-only through v0.1 → v1.4; external
//! publication unlocks at v1.5+ when third-party plugin authors land.
//!
//! ### What ships here
//!
//! - [`OrgsidianPlugin`] — the plugin lifecycle trait (`metadata`, `init`,
//!   `shutdown`, optional hooks).
//! - [`Event`] — `#[non_exhaustive]` enum of host-emitted events.
//! - [`HookOutcome`] — `Continue` / `Replace(T)` / `Cancel(String)`.
//! - [`HookContext`] + [`PluginContext`] — host capability traits.
//! - Payload types: [`PluginMetadata`], [`CaptureEntry`], [`AgendaQuery`],
//!   [`AgendaItem`].
//! - [`PluginError`] + [`Result`] — leaf-local error vocabulary (distinct
//!   from `orgsidian-core::OrgError`).
//!
//! ### See also
//!
//! - LD-10 / LD-26 / LD-5 round-4 amendment in
//!   `_bmad-output/planning-artifacts/architecture.md` for design rationale.
//! - LD-33 for CHANGELOG discipline.
//! - LD-50 for the v0.5 surface-review gate before crates.io publication.

#![warn(clippy::pedantic)]
#![deny(missing_docs)]

mod context;
mod error;
mod event;
mod metadata;
mod outcome;
mod payload;
mod plugin;

pub use context::{HookContext, PluginContext};
pub use error::{PluginError, Result};
pub use event::Event;
pub use metadata::PluginMetadata;
pub use outcome::HookOutcome;
pub use payload::{AgendaItem, AgendaQuery, CaptureEntry};
pub use plugin::OrgsidianPlugin;
