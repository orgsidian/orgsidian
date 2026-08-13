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

// Story 1.18 (LD-40): TOML settings authoritative store. First cross-crate edge
// from `orgsidian-core` to `orgsidian-vault` (LEAF graph rule per deny.toml).
pub mod settings;

/// Story 2.8 (LD-27/LD-37): parser façade — `orgsidian-core` is the only
/// permitted wrapper of the `orgsidian-parser` LEAF (Crate Dependency Graph
/// "Façade" role; deny.toml LEAF rule), so consumers name
/// `orgsidian_core::parser::{analyze, serialize_document}` instead of
/// depending on the leaf directly.
pub use orgsidian_parser as parser;

// Story 3.6 (LD-37 / FR-15 / FR-17): the index façade — the scan orchestrator
// that wires the parser and index LEAVES together (the only module naming
// both). Owns Vault designation, the initial-scan engine, and the incremental
// `Document` → index-row mapping.
pub mod index;
pub use index::{designate_vault, open_index, scan_vault, IndexHandle, ScanOutcome, ScanProgress};
