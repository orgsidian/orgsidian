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

    /// A forward-only migration failed to apply (LD-12). Wraps the runner's own
    /// error, which already distinguishes a malformed migration, a failed
    /// up-migration SQL statement, and a failed hook.
    ///
    /// Mirrors the `Sqlite` variant's `#[from]` shape so `migrate` can map the
    /// runner error with `?` rather than a hand-written match.
    #[error("migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    /// A connection PRAGMA did not take the value it was set to.
    ///
    /// Applying a PRAGMA is not the same as it having an effect: SQLite
    /// silently ignores several of them rather than erroring. `journal_mode`
    /// is the dangerous case — it can refuse to become WAL (on `:memory:`
    /// databases, and on some network filesystems) while reporting success,
    /// which would leave the index running with the durability and concurrency
    /// characteristics LD-4 specifically chose against. Every locked PRAGMA is
    /// therefore read back and compared.
    ///
    /// `#[non_exhaustive]` on the variant as well as the enum: the enum
    /// attribute keeps a new VARIANT from breaking downstream exhaustive
    /// matches, but only this one keeps a new FIELD (a connection path, say)
    /// from breaking every `IndexError::Pragma { name, expected, actual }`
    /// pattern.
    #[error("pragma {name} did not take effect: expected {expected}, got {actual}")]
    #[non_exhaustive]
    Pragma {
        name: &'static str,
        expected: String,
        actual: String,
    },

    /// Building the reader connection pool failed (LD-14). Wraps `deadpool`'s
    /// `BuildError` as a message: the workspace supplies the runtime
    /// explicitly (`Runtime::Tokio1`), so this is a construction-time failure a
    /// caller can surface but cannot retry.
    #[error("failed to build the reader connection pool: {0}")]
    PoolBuild(String),

    /// Acquiring a connection from the reader pool failed (LD-14): the pool
    /// timed out or was closed, or the connection's `create`/`recycle` errored.
    ///
    /// `deadpool`'s `PoolError<IndexError>` NESTS this very type as its inner
    /// error, so it is flattened to a message here rather than wrapped with
    /// `#[from]` — a `#[from]` would make `IndexError` recursively contain
    /// itself. The nested cause, when present, is rendered into the string.
    #[error("failed to acquire a connection from the reader pool: {0}")]
    PoolAcquire(String),

    /// The dedicated writer task is unavailable, so a write could not be
    /// serialized through it (LD-7/LD-14). Either the task has shut down (its
    /// channel `Receiver` was dropped, so the send failed) or the per-write
    /// acknowledgement was canceled before the result arrived.
    ///
    /// Deliberately distinct from [`IndexError::Sqlite`] so a caller can tell
    /// "the index writer is gone" from "the write ran and its SQL failed".
    #[error("index writer unavailable: {0}")]
    WriterUnavailable(String),

    /// A sync-engine invariant the SQL layer cannot express was violated
    /// (Story 3.6). The transactional sync path (`sync::upsert_file` and
    /// friends) reaches a state that "cannot happen" — e.g. the `files` row it
    /// just upserted is not found when its rowid is read back. Returned rather
    /// than `unwrap`/`expect`/`panic!`-ing (no panics in committed non-test
    /// code) so the writer task survives and surfaces the failure on its ack.
    #[error("index sync invariant violated: {0}")]
    Sync(String),
}
