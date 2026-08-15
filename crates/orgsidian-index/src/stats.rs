//! Read-only aggregate counts for `orgsidian index stats` (Story 3.7).
//!
//! [`collect_stats`] runs a handful of `SELECT COUNT`s plus the schema-version
//! and last-indexed lookups against a **read** connection (the LD-14 reader
//! pool via [`crate::IndexPool::interact`]) — it never opens the writer and
//! never mutates a row. All raw SQL for the inspection commands lives here, in
//! `orgsidian-index`, per the LEAF rule (no raw SQL outside this crate).
//!
//! `last_indexed_at` is deliberately `MAX(files.indexed_at)` rather than a
//! dedicated `vault_meta` key: a full `rebuild` refreshes every file's
//! `indexed_at`, so the maximum is the last full-index time — and reading it
//! keeps the write path frozen. It is `None` ("never") on an empty index.

use rusqlite::Connection;
use serde::Serialize;

use crate::error::IndexError;

/// A snapshot of a derived index's aggregate state — the shape
/// `orgsidian index stats` prints (human table) or serializes (`--json`, one
/// camelCase object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    /// Rows in `files` (every indexed `.org` file, including quarantined ones).
    pub file_count: i64,
    /// Rows in `files` with `quarantined = 1` (LD-41 malformed-file rows).
    pub quarantined_count: i64,
    /// Rows in `headlines` — real headlines plus the synthetic `kind='preamble'`
    /// rows, i.e. exactly what the FTS tables index.
    pub headline_count: i64,
    /// Rows in `tags`.
    pub tag_count: i64,
    /// Rows in `links`.
    pub link_count: i64,
    /// Documents indexed by the `fts_headlines` FTS5 table (one per `headlines`
    /// row) — the FR-12 search corpus size.
    pub fts_doc_count: i64,
    /// The highest applied migration version (`PRAGMA`-authority is
    /// `user_version`; this is the human `_schema_version` audit row).
    pub schema_version: i64,
    /// ISO-8601 timestamp that schema version was applied.
    pub schema_applied_at: String,
    /// ISO-8601 timestamp of the most recent per-file index pass, or `None`
    /// when the index holds no files.
    pub last_indexed_at: Option<String>,
}

/// Collect the [`IndexStats`] for an already-migrated index on a read
/// connection.
///
/// # Errors
///
/// [`IndexError::Sqlite`] if any count/lookup fails — most notably if the
/// `_schema_version` audit row is missing (a corrupt or un-migrated database,
/// which the caller resolves by refusing rather than repairing).
pub fn collect_stats(conn: &Connection) -> Result<IndexStats, IndexError> {
    let file_count = count(conn, "SELECT COUNT(*) FROM files")?;
    let quarantined_count = count(conn, "SELECT COUNT(*) FROM files WHERE quarantined = 1")?;
    let headline_count = count(conn, "SELECT COUNT(*) FROM headlines")?;
    let tag_count = count(conn, "SELECT COUNT(*) FROM tags")?;
    let link_count = count(conn, "SELECT COUNT(*) FROM links")?;
    let fts_doc_count = count(conn, "SELECT COUNT(*) FROM fts_headlines")?;

    let (schema_version, schema_applied_at) = conn.query_row(
        "SELECT version, applied_at FROM _schema_version ORDER BY version DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // `MAX(indexed_at)` is NULL on an empty `files` table; `Option<String>`
    // reads that back as `None` ("never").
    let last_indexed_at: Option<String> =
        conn.query_row("SELECT MAX(indexed_at) FROM files", [], |row| row.get(0))?;

    Ok(IndexStats {
        file_count,
        quarantined_count,
        headline_count,
        tag_count,
        link_count,
        fts_doc_count,
        schema_version,
        schema_applied_at,
        last_indexed_at,
    })
}

/// Run a single scalar `SELECT COUNT(*)`-shaped query and return the count.
fn count(conn: &Connection, sql: &str) -> Result<i64, IndexError> {
    Ok(conn.query_row(sql, [], |row| row.get(0))?)
}
