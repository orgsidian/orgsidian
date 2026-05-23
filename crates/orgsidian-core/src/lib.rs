//! orgsidian-core: core domain orchestrator (composition root for parser/index/watcher/vault/plugin-api/report).
//!
//! Structural placeholder — cross-crate edges materialize incrementally per first-use story.

mod error;
pub use error::{OrgError, Result};

// Story 1.8 (LD-38): plugin registry + panic-isolation macro. The
// `invoke_plugin_hook!` macro is `#[macro_export]`-hoisted to the crate root.
pub mod registry;
