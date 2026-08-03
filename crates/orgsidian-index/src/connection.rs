//! Connection opening + the LD-4 locked PRAGMA set (LD-14 connection
//! management, FR-17 derived index).
//!
//! [`open`] is the only sanctioned way to obtain a connection to the index
//! database: the PRAGMAs below are correctness and performance properties of
//! *every* connection, not of the database file, and several of them
//! (`foreign_keys` above all) reset to their defaults on each new connection.

use std::path::Path;

use rusqlite::Connection;

use crate::error::IndexError;

/// Requested memory-map size, in bytes (256 MiB) — LD-4.
const MMAP_SIZE_BYTES: i64 = 268_435_456;

/// Page-cache size in KiB, expressed as the negative form SQLite reads as
/// "kibibytes" rather than "pages" — LD-4. 64000 KiB is 62.5 MiB.
const CACHE_SIZE_KIB: i64 = -64_000;

/// WAL auto-checkpoint threshold, in pages — LD-4.
const WAL_AUTOCHECKPOINT_PAGES: i64 = 4_000;

/// The locked set, applied in LD-4/LD-14 order.
///
/// Built from the constants above rather than written out as a literal, so the
/// values that are APPLIED and the values that are VERIFIED cannot drift apart
/// — editing one const alone would otherwise make every `open` fail at runtime
/// (or, for `mmap_size`, change nothing at all, since it is verified only as
/// `> 0`). The allocation is one per `open`, which is not a hot path.
///
/// Applied through `execute_batch` rather than `Connection::execute` on
/// purpose: `journal_mode` and `mmap_size` RETURN A ROW, and `execute` rejects
/// any statement that does (`ExecuteReturnedResults`). This compiles cleanly
/// either way and fails only at runtime, so the choice is load-bearing.
fn locked_pragmas() -> String {
    format!(
        "PRAGMA journal_mode = WAL;\n\
         PRAGMA synchronous = NORMAL;\n\
         PRAGMA mmap_size = {MMAP_SIZE_BYTES};\n\
         PRAGMA cache_size = {CACHE_SIZE_KIB};\n\
         PRAGMA temp_store = MEMORY;\n\
         PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_PAGES};\n\
         PRAGMA foreign_keys = ON;\n"
    )
}

/// Open the index database at `path`, creating it if absent, and apply the
/// LD-4 locked PRAGMA set.
///
/// The returned connection is a plain `rusqlite::Connection` — pooling and the
/// dedicated writer task are a separate concern and not provided here.
///
/// # Locked PRAGMAs
///
/// `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456` (256 MiB),
/// `cache_size=-64000` (62.5 MiB), `temp_store=MEMORY`,
/// `wal_autocheckpoint=4000` come straight from LD-4/LD-14.
///
/// `foreign_keys=ON` is a **deliberate addition** to that list, and it is
/// correctness-critical rather than a tuning knob: SQLite defaults foreign-key
/// enforcement to OFF, the setting is per-connection and non-persistent, and
/// every `ON DELETE CASCADE` in the schema is a silent no-op without it —
/// deleting a file row would orphan its headlines, tags, properties, clock
/// entries and links instead of removing them. The bundled amalgamation is
/// compiled with `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`, but resting a correctness
/// property on a build flag breaks the moment someone reconsiders the
/// `bundled` feature, so it is set explicitly on every connection.
///
/// # Verification
///
/// Setting a PRAGMA and having it take effect are different things: SQLite
/// silently ignores unsupported or refused values instead of erroring.
/// `open` therefore reads each value back and returns
/// [`IndexError::Pragma`] on mismatch. `mmap_size` is the one exception — the
/// build's `SQLITE_MAX_MMAP_SIZE` may clamp the request to a smaller value,
/// which is not a failure, so it is verified as "memory mapping is enabled"
/// (`> 0`) rather than as an exact match.
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the database cannot be opened or a PRAGMA
/// statement fails; [`IndexError::Pragma`] if an applied PRAGMA did not take
/// the expected value.
pub fn open(path: &Path) -> Result<Connection, IndexError> {
    let conn = Connection::open(path)?;
    conn.execute_batch(&locked_pragmas())?;
    verify_locked_pragmas(&conn)?;
    Ok(conn)
}

/// Apply the version-1 DDL ([`crate::SCHEMA_SQL`]) atomically.
///
/// `SCHEMA_SQL` itself deliberately carries no `BEGIN;`/`COMMIT;`: a migration
/// runner already wraps each migration in its own transaction, and a nested
/// `BEGIN` inside the DDL text would fail with "cannot start a transaction
/// within a transaction" as soon as that runner includes this schema. So the
/// transaction is supplied here instead, for callers applying the DDL directly.
///
/// Without one, `execute_batch` runs each DDL statement in its own implicit
/// transaction and a mid-batch failure (FTS5 module absent, ENOSPC, a killed
/// process) leaves a permanently half-built database — one that reports the
/// same "table files already exists" as a healthy database when the schema is
/// re-applied, and so cannot be told apart from it. [`rusqlite::Transaction`]
/// rolls back on drop, so a failure here leaves the database untouched.
///
/// # Errors
///
/// [`IndexError::Sqlite`] if any statement in the DDL fails — most commonly
/// because the database is already initialized, which is not silently tolerated
/// (the DDL is not `IF NOT EXISTS`-guarded, so a migration bug cannot hide
/// behind idempotency).
pub fn apply_schema(conn: &mut Connection) -> Result<(), IndexError> {
    let tx = conn.transaction()?;
    tx.execute_batch(crate::SCHEMA_SQL)?;
    tx.commit()?;
    Ok(())
}

/// Read every locked PRAGMA back and compare it against what it was set to.
///
/// The expected values are the ones SQLite *reports*, which are not the ones
/// written: `synchronous=NORMAL` reads back `1`, `temp_store=MEMORY` reads back
/// `2`, and `journal_mode=WAL` reads back the lowercase string `wal`.
fn verify_locked_pragmas(conn: &Connection) -> Result<(), IndexError> {
    verify_text(conn, "journal_mode", "wal")?;
    verify_int(conn, "synchronous", 1)?;
    verify_mmap_size(conn)?;
    verify_int(conn, "cache_size", CACHE_SIZE_KIB)?;
    verify_int(conn, "temp_store", 2)?;
    verify_int(conn, "wal_autocheckpoint", WAL_AUTOCHECKPOINT_PAGES)?;
    verify_int(conn, "foreign_keys", 1)?;
    Ok(())
}

fn verify_int(conn: &Connection, name: &'static str, expected: i64) -> Result<(), IndexError> {
    let actual: i64 = conn.pragma_query_value(None, name, |row| row.get(0))?;
    if actual == expected {
        return Ok(());
    }
    Err(IndexError::Pragma {
        name,
        expected: expected.to_string(),
        actual: actual.to_string(),
    })
}

fn verify_text(conn: &Connection, name: &'static str, expected: &str) -> Result<(), IndexError> {
    let actual: String = conn.pragma_query_value(None, name, |row| row.get(0))?;
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    Err(IndexError::Pragma {
        name,
        expected: expected.to_owned(),
        actual,
    })
}

/// `mmap_size` is verified as "enabled", not as an exact match.
///
/// The requested [`MMAP_SIZE_BYTES`] is an upper bound the build may clamp
/// through `SQLITE_MAX_MMAP_SIZE`; a smaller positive value is a healthy
/// outcome. Zero means memory mapping is off entirely, which is the failure
/// worth reporting.
fn verify_mmap_size(conn: &Connection) -> Result<(), IndexError> {
    let actual: i64 = conn.pragma_query_value(None, "mmap_size", |row| row.get(0))?;
    if actual > 0 {
        return Ok(());
    }
    Err(IndexError::Pragma {
        name: "mmap_size",
        expected: format!("> 0 (requested {MMAP_SIZE_BYTES}, build may clamp it lower)"),
        actual: actual.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failing side of the read-back verification, which no integration
    /// test can reach: every PRAGMA in the locked set takes correctly on a
    /// healthy on-disk database, and there is no portable way to make one
    /// refuse. Calling the verifier with a deliberately wrong expectation
    /// exercises the same branch a genuinely refused PRAGMA would.
    ///
    /// Without this, replacing `verify_locked_pragmas`'s body with `Ok(())`
    /// leaves the whole suite green.
    #[test]
    fn a_mismatched_pragma_is_reported_with_both_values() {
        let conn = Connection::open_in_memory().expect("in-memory connection");
        conn.execute_batch("PRAGMA temp_store = MEMORY;")
            .expect("set temp_store");

        // In-memory is fine here: this asserts the comparison logic, not WAL.
        verify_int(&conn, "temp_store", 2).expect("temp_store really is 2");

        let err = verify_int(&conn, "temp_store", 99)
            .expect_err("a value that does not match must be reported");
        let IndexError::Pragma {
            name,
            expected,
            actual,
        } = &err
        else {
            panic!("expected IndexError::Pragma, got {err:?}");
        };
        assert_eq!(*name, "temp_store");
        assert_eq!(expected, "99");
        assert_eq!(actual, "2");
        assert_eq!(
            err.to_string(),
            "pragma temp_store did not take effect: expected 99, got 2"
        );
    }

    #[test]
    fn verify_text_is_case_insensitive() {
        // journal_mode reads back lowercase `wal` whatever case it was written
        // in, and the verifier must not turn that into a spurious failure.
        let conn = Connection::open_in_memory().expect("in-memory connection");
        verify_text(&conn, "journal_mode", "MEMORY").expect("an in-memory db reports `memory`");
    }

    #[test]
    fn the_applied_pragmas_are_built_from_the_verified_constants() {
        // The drift guard: these values are applied as text and verified as
        // integers, and nothing else ties the two together.
        let sql = locked_pragmas();
        assert!(sql.contains(&format!("PRAGMA mmap_size = {MMAP_SIZE_BYTES};")));
        assert!(sql.contains(&format!("PRAGMA cache_size = {CACHE_SIZE_KIB};")));
        assert!(sql.contains(&format!(
            "PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_PAGES};"
        )));
    }
}
