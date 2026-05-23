//! orgsidian-core: core domain orchestrator (composition root for parser/index/watcher/vault/plugin-api/report).
//!
//! Structural placeholder — cross-crate edges materialize incrementally per first-use story.

mod error;
pub use error::{OrgError, Result};

// Story 1.8 (LD-38): plugin registry + panic-isolation macro. The
// `invoke_plugin_hook!` macro is `#[macro_export]`-hoisted to the crate root.
pub mod registry;

// Story 1.9 (LD-9): deterministic `Clock` trait + `FakeClock` fake for the
// watcher's timeout discipline + downstream perf-snapshot tests (Story 1.12).
// Dual-gated: `cfg(test)` covers internal tests; `feature = "test-support"`
// covers external consumers that opt in via dev-dependencies.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
