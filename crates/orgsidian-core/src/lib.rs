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
pub use index::{
    agenda_today, agenda_week, designate_vault, index_integrity, index_stats, open_index,
    rebuild_index, resolve_index_db_path, resync_file, scan_vault, AgendaItem, IndexHandle,
    IndexStats, IntegrityCheck, IntegrityReport, ResyncOutcome, ScanOutcome, ScanProgress,
};

// Story 5.4 (LD-7 / LD-9 / FR-16): the external-edits reconciler — the hub that
// consumes a debounced `orgsidian_watcher::FileChanged` and routes it to the
// vault clean-buffer reload + index re-sync (the CLEAN branch) or the Story 5.5
// dirty-buffer conflict SEAM. Core is the only crate the LEAF graph rule lets
// wrap the watcher, vault, and index together.
pub mod reconcile;
pub use reconcile::{
    reconcile_external_write, BufferSnapshot, CleanReload, ExternalWriteOutcome, RELOAD_NOTICE,
    RELOAD_NOTICE_DURATION,
};

// Story 5.5 (LD-7 / FR-16 / NFR-16): the DIRTY-branch block-save fallback — the
// hub that runs the injected `BlockWithWarning` strategy over the Dirty Buffer,
// records the block (`PendingConflicts`), gates saves (`save_buffer`), and
// clears the block (`discard_external_changes`). Core is the only crate the LEAF
// graph rule lets wrap `orgsidian-vault`, so the shell reaches the vault
// conflict/pending/atomic surface exclusively through these re-exports.
pub mod conflict;
pub use conflict::{
    discard_external_changes, resolve_dirty_conflict, save_buffer, ConflictNotice,
    PendingConflicts, SharedPendingConflicts,
};

// Re-export the vault conflict-strategy + Dirty-Buffer surface the shell needs
// to construct managed state and inject the active strategy (LEAF façade rule:
// the shell depends on `orgsidian-core` only, never on `orgsidian-vault`).
pub use orgsidian_vault::{
    BlockWithWarning, ConflictStrategy, ResolveConflict, Sha256Hash, SharedDirtyBuffers,
};

// Story 6.6 (FR-21 partial / FR-18 / UJ-4): hardcoded coaching-balloon
// dismissal persistence at `<Vault>/.orgsidian/coaching-dismissed.json` — a
// disposable v0.1 stand-in Story 11.4 removes wholesale (see module docs).
pub mod coaching;
pub use coaching::{
    coaching_dismissed_path, dismiss_coaching, read_dismissed_coaching, UJ4_CAPTURE_INTRO,
    UJ4_TODAY_INTRO,
};

// Story 6.1 (FR-18): the built-in Starter Vault content generator — Personal
// GTD + Student ship here; Freelancer (needs Story 8.7's BacklinksPanel) and
// Empty (Story 11.1) are deferred, see `deferred-work.md`. Uses `atomic_write`
// through this crate's `vault_err` mapper, same LEAF-façade pattern as
// `conflict` above.
pub mod starter_vault;
pub use starter_vault::{generate_starter_vault, StarterVaultKind};

// Story 5.4 (FR-16): re-export the vault's pure cursor-preservation types so the
// shell and tests name `orgsidian_core::{CursorPosition, CursorOutcome}` rather
// than reaching into the vault leaf directly.
pub use orgsidian_vault::reload::{CursorOutcome, CursorPosition};
