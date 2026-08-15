//! Index-identity guard (LD-13 rebuild triggers, LD-40 index-outside-vault).
//!
//! The index is a derived SQLite file living OUTSIDE the vault, in the OS data
//! dir (LD-40). `open()` (Story 3.3) will happily open ANY path with
//! `READWRITE|CREATE` and convert it to WAL — so a typo'd path silently creates
//! an empty database, and a path pointing at an unrelated SQLite file gets its
//! journal mode rewritten before anyone notices (the exact defect the
//! `application_id` deferred row describes, deferred-work.md:245).
//!
//! This module adds the marker `open()` lacks: a stable `PRAGMA application_id`
//! stamp ([`stamp_application_id`], written once on a freshly created index) and
//! a guard ([`check_index_identity`] / [`inspect_index_file`]) that classifies a
//! file as [`IndexIdentity::Ours`], [`IndexIdentity::Foreign`], or
//! [`IndexIdentity::VersionMismatch`]. It is invoked by the Story 3.6 vault-open
//! path to decide open-cached vs rebuild vs refuse — it does **not** edit
//! `connection.rs::open()` (frozen seam; `tests/schema.rs` stays byte-unchanged).

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::error::IndexError;

/// Orgsidian index magic, written to `PRAGMA application_id`.
///
/// The bytes spell `ORGS` (`0x4F 0x52 0x47 0x53`); the value is a positive
/// `i32` (SQLite stores `application_id` as a signed 32-bit integer). Stable
/// forever: changing it would orphan every existing index. `application_id` is
/// `0` on a fresh, un-stamped SQLite database and on files created by other
/// tools, which is exactly how [`IndexIdentity::Foreign`] is detected.
pub const APPLICATION_ID: i32 = 0x4F52_4753;

/// The `PRAGMA user_version` an index at schema version 1 carries (stamped by
/// the migration runner, Story 3.4). LD-13's drift check compares against it.
const EXPECTED_USER_VERSION: i32 = 1;

/// The tables a fully-migrated version-1 Orgsidian index must contain. Used to
/// distinguish a half-created index of ours (migrated but not yet
/// `application_id`-stamped — see [`IndexIdentity::OursUnstamped`]) from a
/// foreign SQLite file that merely happens to carry `user_version = 1`. A
/// foreign file would need this exact table set to be misclassified as ours,
/// which no unrelated tool produces.
const REQUIRED_TABLES: &[&str] = &[
    "_schema_version",
    "files",
    "headlines",
    "tags",
    "properties",
    "clock_entries",
    "links",
    "vault_meta",
    "fts_headlines",
    "fts_content",
];

/// The three states the identity guard distinguishes — the concrete form of
/// LD-13's rebuild triggers (architecture.md:402).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexIdentity {
    /// Our index at the expected schema version: open it (incrementally
    /// re-scan; unchanged files are skipped → the cached fast-open).
    Ours,
    /// Our schema at the expected `user_version`, but `application_id` is still
    /// the SQLite default (`0`): a first-time creation whose stamp step was
    /// interrupted (a crash between the migration and [`stamp_application_id`]).
    /// Recoverable — the caller re-stamps and opens it, rather than refusing as
    /// [`Foreign`](IndexIdentity::Foreign) and stranding a valid index.
    OursUnstamped,
    /// `application_id` is not ours (`0` or some other tool's magic) and the
    /// file does not carry our schema: this is not our file. Refuse rather than
    /// rewrite someone else's database.
    Foreign,
    /// Our index, but at a different `user_version` than expected: an LD-13
    /// drift signal → drop and rebuild from the vault's `.org` files.
    VersionMismatch,
}

/// Stamp [`APPLICATION_ID`] onto a freshly created index. Called once, after
/// the writer migrates a brand-new database, so subsequent
/// [`check_index_identity`] calls recognize it as [`IndexIdentity::Ours`].
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the PRAGMA write fails.
pub fn stamp_application_id(conn: &Connection) -> Result<(), IndexError> {
    conn.pragma_update(None, "application_id", APPLICATION_ID)?;
    Ok(())
}

/// Classify an already-open connection's database by its `application_id` +
/// `user_version` header fields. The primitive; [`inspect_index_file`] is the
/// path-based wrapper the vault-open flow uses.
///
/// # Errors
///
/// [`IndexError::Sqlite`] if a PRAGMA read fails.
pub fn check_index_identity(conn: &Connection) -> Result<IndexIdentity, IndexError> {
    let application_id: i32 = conn.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if application_id == APPLICATION_ID {
        return Ok(if user_version == EXPECTED_USER_VERSION {
            IndexIdentity::Ours
        } else {
            IndexIdentity::VersionMismatch
        });
    }

    // `application_id` is the SQLite default (0) but the file otherwise carries
    // our full schema at the expected version: a first-time creation whose
    // stamp step was interrupted (crash between migrate and stamp). Recoverable
    // — the caller re-stamps. A foreign file with a coincidental
    // `user_version == 1` fails the full-schema check and stays `Foreign`.
    if application_id == 0 && user_version == EXPECTED_USER_VERSION && has_our_schema(conn)? {
        return Ok(IndexIdentity::OursUnstamped);
    }

    Ok(IndexIdentity::Foreign)
}

/// Whether `conn`'s database carries every table in [`REQUIRED_TABLES`] — the
/// signal that an `application_id`-less file is a half-created Orgsidian index
/// rather than an unrelated SQLite file.
fn has_our_schema(conn: &Connection) -> Result<bool, IndexError> {
    for table in REQUIRED_TABLES {
        let present = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !present {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Classify the database file at `db_path` WITHOUT mutating it. Opens
/// **read-only** (`SQLITE_OPEN_READ_ONLY`) precisely so a [`IndexIdentity::Foreign`]
/// file is not converted to WAL / stripped of `-wal`/`-shm` sidecars the way
/// `open()` would — the guard must be able to refuse a foreign file having
/// touched nothing but its header.
///
/// The caller is expected to have checked that the file exists; a missing file
/// (or a non-database blob) surfaces as [`IndexError::Sqlite`], which the
/// vault-open path treats as "not a usable index".
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the file cannot be opened read-only or a PRAGMA
/// read fails.
pub fn inspect_index_file(db_path: &Path) -> Result<IndexIdentity, IndexError> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    check_index_identity(&conn)
}
