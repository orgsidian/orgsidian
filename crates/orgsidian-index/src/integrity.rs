//! Read-only integrity verification for `orgsidian index integrity`
//! (Story 3.7) — the scriptable CI integrity gate.
//!
//! [`check_integrity`] runs, on a read connection, the three SQLite/FTS5
//! consistency commands and folds their results into an [`IntegrityReport`]:
//!
//! * `PRAGMA integrity_check` — page/structure/index consistency for the whole
//!   database; a healthy run returns the single row `ok`.
//! * `PRAGMA foreign_key_check` — every FK violation as a row; healthy is zero
//!   rows.
//! * FTS5 `'integrity-check'` on **both** external-content tables
//!   (`fts_headlines`, `fts_content`) — verifies each FTS index matches its
//!   `headlines` content; a mismatch raises `SQLITE_CORRUPT_VTAB`, which is
//!   captured as a failing check rather than propagated.
//!
//! All raw SQL for the inspection commands lives here, in `orgsidian-index`,
//! per the LEAF rule. The FTS `'integrity-check'` command is a verification
//! that reads and validates the index; it does not modify any row.

use rusqlite::Connection;
use serde::Serialize;

use crate::error::IndexError;

/// The outcome of one named integrity command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityCheck {
    /// The command's stable identifier (e.g. `integrity_check`, `fts_headlines`).
    pub name: String,
    /// Whether the command reported the index consistent.
    pub ok: bool,
    /// Failure detail when `ok` is false (the offending rows or the driver's
    /// error message); `None` on success.
    pub detail: Option<String>,
}

/// The aggregate integrity result: `ok` is the AND of every check, so a single
/// failure flips it (and drives the command's non-zero exit code).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    /// `true` only when every check in [`checks`](Self::checks) passed.
    pub ok: bool,
    /// Each command's result, in run order.
    pub checks: Vec<IntegrityCheck>,
}

/// Run every integrity command against a read connection and fold the results.
///
/// # Errors
///
/// [`IndexError::Sqlite`] only if a command cannot be *run at all* (statement
/// preparation fails). A command that runs and reports inconsistency is a
/// failing [`IntegrityCheck`] with `ok = false`, not an `Err` — the report is
/// the product, and the caller maps a non-`ok` report to a non-zero exit.
pub fn check_integrity(conn: &Connection) -> Result<IntegrityReport, IndexError> {
    let checks = vec![
        integrity_check(conn)?,
        foreign_key_check(conn)?,
        fts_integrity_check(conn, "fts_headlines"),
        fts_integrity_check(conn, "fts_content"),
    ];
    let ok = checks.iter().all(|check| check.ok);
    Ok(IntegrityReport { ok, checks })
}

/// `PRAGMA integrity_check`: healthy iff it returns the single row `ok`.
fn integrity_check(conn: &Connection) -> Result<IntegrityCheck, IndexError> {
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let rows: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    let ok = rows.len() == 1 && rows[0] == "ok";
    Ok(IntegrityCheck {
        name: "integrity_check".to_string(),
        ok,
        detail: (!ok).then(|| rows.join("; ")),
    })
}

/// `PRAGMA foreign_key_check`: healthy iff it returns zero rows. Each row names
/// a table, the offending rowid, and the referred table.
fn foreign_key_check(conn: &Connection) -> Result<IntegrityCheck, IndexError> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let rows: Vec<String> = stmt
        .query_map([], |row| {
            let table: String = row.get(0)?;
            let rowid: Option<i64> = row.get(1)?;
            let referred: String = row.get(2)?;
            Ok(match rowid {
                Some(rowid) => format!("{table} rowid {rowid} -> {referred}"),
                None => format!("{table} -> {referred}"),
            })
        })?
        .collect::<Result<_, _>>()?;
    let ok = rows.is_empty();
    Ok(IntegrityCheck {
        name: "foreign_key_check".to_string(),
        ok,
        detail: (!ok).then(|| rows.join("; ")),
    })
}

/// FTS5 `'integrity-check'` for one external-content table: the special
/// `INSERT INTO <tbl>(<tbl>) VALUES('integrity-check')` command validates the
/// index against its content table and errors (`SQLITE_CORRUPT_VTAB`) on a
/// mismatch. The error is captured as a failing check — a corrupt FTS index is
/// a reportable result, not a reason to abort the whole report.
fn fts_integrity_check(conn: &Connection, table: &str) -> IntegrityCheck {
    // `table` is a compile-time-fixed identifier from this module, never user
    // input, so the format-built SQL carries no injection surface.
    let sql = format!("INSERT INTO {table}({table}) VALUES('integrity-check')");
    match conn.execute(&sql, []) {
        Ok(_) => IntegrityCheck {
            name: table.to_string(),
            ok: true,
            detail: None,
        },
        Err(err) => IntegrityCheck {
            name: table.to_string(),
            ok: false,
            detail: Some(err.to_string()),
        },
    }
}
