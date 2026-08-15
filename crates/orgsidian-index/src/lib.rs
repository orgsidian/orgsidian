//! orgsidian-index: SQLite index + query API + FTS5 (FR-7, FR-8, FR-9).
//!
//! Story 3.3 ships the foundation: the schema and the connection contract.
//!
//! [`SCHEMA_SQL`] is the version-1 DDL — the eight normalized tables
//! (`files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`,
//! `vault_meta`, `_schema_version`), the two FTS5 external-content search
//! tables over `headlines`, and the LD-11 index set. It is the single
//! committed copy of that text: `migrations/0001_initial-schema.sql` on disk,
//! this constant in Rust, and nothing else.
//!
//! [`open`] returns a connection with the LD-4 locked PRAGMAs applied and
//! verified by read-back, including the `foreign_keys=ON` without which every
//! `ON DELETE CASCADE` in the schema is a silent no-op. [`apply_schema`]
//! installs the DDL on such a connection inside a transaction.
//!
//! [`migrate`] is the versioned production path (LD-12): it runs the
//! forward-only migration set — currently the single `0001` that installs
//! `SCHEMA_SQL` — bumping `PRAGMA user_version` (LD-13's drift signal) and
//! writing the `_schema_version` audit row. `apply_schema` stays a DDL-only
//! primitive for schema-shape tests.
//!
//! [`IndexPool`] is the LD-14 reader pool: four `deadpool`-managed connections,
//! each carrying the LD-4 PRAGMAs and a `busy_timeout`, handed out through the
//! blocking-safe [`IndexPool::interact`] helper (rusqlite runs on
//! `spawn_blocking`, LD-16). [`IndexWriter`] is the single dedicated writer
//! task (LD-7 Single Writer Rule at the index layer): it owns the one writable,
//! migrated connection and serializes every write submitted through
//! [`IndexWriter::execute`] over an `mpsc` channel. The query API those reads
//! and writes will carry is a later concern and not provided here.
//!
//! The index is a derived artifact (LD-13, LD-17): every row here is
//! reconstructible from the Vault's .org files.
//!
//! [`sync`] is the Story 3.6 write path: a parser-agnostic, transactional sync
//! engine ([`upsert_file`]/[`delete_file`]/[`quarantine_file`] and the batched
//! [`SyncOp`]) that maps index-native input structs into the tables and the two
//! external-content FTS5 tables, structurally pairing every headline mutation
//! with its FTS `'delete'` command row. [`identity`] is the LD-13 guard:
//! [`stamp_application_id`] marks a fresh index and [`check_index_identity`] /
//! [`inspect_index_file`] classify a file as ours, foreign, or version-drifted.

pub mod connection;
pub mod error;
pub mod identity;
pub mod integrity;
pub mod migrations;
pub mod pool;
pub mod stats;
pub mod sync;
pub mod writer;

pub use connection::{apply_schema, open};
pub use error::IndexError;
pub use identity::{
    check_index_identity, inspect_index_file, stamp_application_id, IndexIdentity, APPLICATION_ID,
};
pub use integrity::{check_integrity, IntegrityCheck, IntegrityReport};
pub use migrations::migrate;
pub use pool::IndexPool;
pub use stats::{collect_stats, IndexStats};
pub use sync::{
    delete_file, file_is_unchanged, quarantine_file, set_vault_meta, upsert_file, ClockInput,
    FileIndexInput, HeadlineInput, LinkInput, PreambleInput, SyncOp,
};
pub use writer::{IndexUpdate, IndexWriter};

/// The version-1 schema DDL, verbatim from `migrations/0001_initial-schema.sql`.
///
/// Executable top-to-bottom in one `execute_batch` against a fresh database.
/// It is not `IF NOT EXISTS`-guarded: applying it twice fails with a
/// duplicate-object error, so a migration bug cannot hide behind silent
/// idempotency.
///
/// The text carries no `BEGIN;`/`COMMIT;`, so that a migration runner can
/// include it inside its own transaction. Prefer [`apply_schema`] over
/// executing this constant directly — run bare, a mid-batch failure leaves a
/// half-built database that reports the same duplicate-object error a healthy
/// one does.
///
/// Per LD-12 the file is forward-only — schema changes belong in new migration
/// files rather than in edits to this text.
pub const SCHEMA_SQL: &str = include_str!("../migrations/0001_initial-schema.sql");
