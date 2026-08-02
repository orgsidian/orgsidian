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
/// "kibibytes" rather than "pages" (64 MiB) — LD-4.
const CACHE_SIZE_KIB: i64 = -64_000;

/// WAL auto-checkpoint threshold, in pages — LD-4.
const WAL_AUTOCHECKPOINT_PAGES: i64 = 4_000;

/// The locked set, applied in LD-4/LD-14 order.
///
/// Applied through `execute_batch` rather than `Connection::execute` on
/// purpose: `journal_mode` and `mmap_size` RETURN A ROW, and `execute` rejects
/// any statement that does (`ExecuteReturnedResults`). This compiles cleanly
/// either way and fails only at runtime, so the choice is load-bearing.
const LOCKED_PRAGMAS: &str = "\
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA mmap_size = 268435456;
PRAGMA cache_size = -64000;
PRAGMA temp_store = MEMORY;
PRAGMA wal_autocheckpoint = 4000;
PRAGMA foreign_keys = ON;
";

/// Open the index database at `path`, creating it if absent, and apply the
/// LD-4 locked PRAGMA set.
///
/// The returned connection is a plain `rusqlite::Connection` — pooling and the
/// dedicated writer task are a separate concern and not provided here.
///
/// # Locked PRAGMAs
///
/// `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`,
/// `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000` come
/// straight from LD-4/LD-14.
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
    conn.execute_batch(LOCKED_PRAGMAS)?;
    verify_locked_pragmas(&conn)?;
    Ok(conn)
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
