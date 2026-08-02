//! orgsidian-index: SQLite index + query API + FTS5 (FR-7, FR-8, FR-9).
//!
//! Story 3.3 ships the foundation: the schema and the connection contract.
//!
//! [`SCHEMA_SQL`] is the version-1 DDL — the eight normalized tables
//! (`files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`,
//! `vault_meta`, `_schema_version`), the two FTS5 external-content search
//! tables over `headlines`, and the LD-11 index set. It is the single
//! committed copy of that text: `sql/schema.sql` on disk, this constant in
//! Rust, and nothing else.
//!
//! [`open`] returns a connection with the LD-4 locked PRAGMAs applied and
//! verified by read-back, including the `foreign_keys=ON` without which every
//! `ON DELETE CASCADE` in the schema is a silent no-op.
//!
//! The index is a derived artifact (LD-13, LD-17): every row here is
//! reconstructible from the Vault's .org files.

pub mod connection;
pub mod error;

pub use connection::open;
pub use error::IndexError;

/// The version-1 schema DDL, verbatim from `sql/schema.sql`.
///
/// Executable top-to-bottom in one `execute_batch` against a fresh database.
/// It is not `IF NOT EXISTS`-guarded: applying it twice fails, so a
/// half-initialized database cannot pass for a healthy one.
///
/// Per LD-12 the file is forward-only — schema changes belong in new migration
/// files rather than in edits to this text.
pub const SCHEMA_SQL: &str = include_str!("../sql/schema.sql");
