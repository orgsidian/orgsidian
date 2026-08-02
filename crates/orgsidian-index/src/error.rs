//! `IndexError` — failure type for the SQLite index subsystem (LD-4 locked
//! PRAGMAs, LD-14 connection management, FR-17 derived index).
//!
//! Mirrors the `VaultError` precedent (`orgsidian-vault/src/error.rs`):
//! `#[non_exhaustive]`, `thiserror::Error` derive, and variants that carry
//! enough context to localize a failure without re-deriving it from an opaque
//! driver error. Mapping into `OrgError` belongs to `orgsidian-core`, which is
//! the only crate allowed to wrap this LEAF.

// Epic 3 grows this crate (migrations, reader pool, query API): new variants
// must not be breaking changes for downstream exhaustive matches.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IndexError {
    /// An underlying SQLite failure — open, DDL, statement preparation, or a
    /// PRAGMA read-back query.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A connection PRAGMA did not take the value it was set to.
    ///
    /// Applying a PRAGMA is not the same as it having an effect: SQLite
    /// silently ignores several of them rather than erroring. `journal_mode`
    /// is the dangerous case — it can refuse to become WAL (on `:memory:`
    /// databases, and on some network filesystems) while reporting success,
    /// which would leave the index running with the durability and concurrency
    /// characteristics LD-4 specifically chose against. Every locked PRAGMA is
    /// therefore read back and compared.
    #[error("pragma {name} did not take effect: expected {expected}, got {actual}")]
    Pragma {
        name: &'static str,
        expected: String,
        actual: String,
    },
}
