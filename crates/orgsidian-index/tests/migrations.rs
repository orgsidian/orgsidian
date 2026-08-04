//! Integration tests for the forward-only migration runner (Story 3.4,
//! LD-12/LD-13, FR-17).
//!
//! Every test drives a REAL on-disk database in its own `TempDir`, opened
//! through the production [`open`] — never `:memory:`. `user_version` semantics
//! and WAL behavior differ on an in-memory database, so an in-memory fixture
//! would assert against a connection nothing in production ever holds.
//! ([`rusqlite_migration::Migrations::validate`] legitimately uses an in-memory
//! database internally; that is the crate's own harness, exercised as a unit
//! test next to the private `migrations()` factory, not this fixture.)

use std::collections::BTreeSet;
use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::TempDir;

use orgsidian_index::{apply_schema, migrate, open};

/// A fresh temp directory + the path a database should live at inside it. The
/// `TempDir` is returned so the caller keeps it alive — dropping it deletes the
/// directory out from under the open database.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("index.db");
    (dir, path)
}

/// A fresh on-disk connection (locked PRAGMAs applied) with the migration set
/// run to latest — the production path this story ships.
fn migrated_db() -> (TempDir, Connection) {
    let (dir, path) = temp_db_path();
    let mut conn = open(&path).expect("open index database");
    migrate(&mut conn).expect("migrate a fresh database to latest");
    (dir, conn)
}

/// `PRAGMA user_version`, read via the statement form (it has no table-valued
/// function). This is the exact value LD-13's drift check compares.
fn user_version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read PRAGMA user_version")
}

/// Names from `sqlite_master` of the given type, sorted and deduplicated.
fn master_names(conn: &Connection, kind: &str) -> BTreeSet<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1")
        .expect("prepare sqlite_master query");
    let rows = stmt
        .query_map([kind], |row| row.get::<_, String>(0))
        .expect("query sqlite_master");
    rows.map(|r| r.expect("read sqlite_master row")).collect()
}

/// The named indices only — SQLite's own `sqlite_autoindex_*` entries (for the
/// composite PKs and `vault_meta`'s TEXT PK) are excluded, matching how
/// `tests/schema.rs` counts them.
fn named_indices(conn: &Connection) -> BTreeSet<String> {
    master_names(conn, "index")
        .into_iter()
        .filter(|name| !name.starts_with("sqlite_autoindex_"))
        .collect()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("count query `{sql}`: {e}"))
}

// ---------------------------------------------------------------------------
// AC4 headline — a fresh database reaches schema version 1
// ---------------------------------------------------------------------------

#[test]
fn a_fresh_database_reaches_version_1() {
    let (_dir, conn) = migrated_db();

    // The epic AC's headline: one migration applied ⇒ user_version == 1. This
    // is rusqlite_migration's own version store, and LD-13's rebuild trigger
    // compares exactly this value.
    assert_eq!(user_version(&conn), 1);
}

// ---------------------------------------------------------------------------
// AC4 — migrate is idempotent (the property apply_schema deliberately lacks)
// ---------------------------------------------------------------------------

#[test]
fn migrate_is_idempotent_and_writes_the_audit_row_once() {
    let (_dir, mut conn) = migrated_db();

    assert_eq!(user_version(&conn), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM _schema_version"), 1);

    // Re-running the production path on an already-current database is a no-op:
    // it must not error, must not re-bump the version, and — the part a bare
    // re-apply of the DDL could never satisfy — must not duplicate the audit
    // row. `tests/schema.rs::re_applying_the_schema_fails_loudly` pins the
    // opposite behavior for `apply_schema`; this is why the migration layer
    // exists.
    migrate(&mut conn).expect("second migrate on a current database is a no-op");

    assert_eq!(user_version(&conn), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM _schema_version"), 1);
}

// ---------------------------------------------------------------------------
// AC5 — the audit row is version 1 with an ISO-8601 applied_at
// ---------------------------------------------------------------------------

#[test]
fn the_audit_row_is_version_1_with_iso8601_applied_at() {
    let (_dir, conn) = migrated_db();

    let version: i64 = conn
        .query_row("SELECT version FROM _schema_version", [], |row| row.get(0))
        .expect("read the single _schema_version row");
    assert_eq!(
        version, 1,
        "the audit row's version must be the explicitly bound 1, agreeing with \
         PRAGMA user_version rather than a rowid-alias accident"
    );

    // Shape, not instant: `strftime('%Y-%m-%dT%H:%M:%SZ','now')` is deterministic
    // in FORMAT but not in value, so assert the ISO-8601 UTC shape via a LIKE
    // pattern (`_` matches any single char) — never an exact timestamp.
    let iso_shaped = count(
        &conn,
        "SELECT count(*) FROM _schema_version \
         WHERE version = 1 AND applied_at LIKE '____-__-__T__:__:__Z'",
    );
    assert_eq!(
        iso_shaped, 1,
        "applied_at must be an ISO-8601 UTC timestamp (YYYY-MM-DDThh:mm:ssZ)"
    );
}

// ---------------------------------------------------------------------------
// AC5 — the migrated schema IS the 3.3 schema (anti-fork, made behavioral)
// ---------------------------------------------------------------------------

#[test]
fn the_migrated_schema_matches_the_directly_applied_schema() {
    // Two databases, same DDL, two install paths. `migrate` (production) and
    // `apply_schema` (the 3.3 primitive) share `SCHEMA_SQL`, so their table and
    // named-index sets MUST be identical — the concrete guarantee that
    // relocating the DDL into `0001` did not fork it. They differ only on
    // version-tracking rows (asserted elsewhere), never on shape.
    let (_dir_m, migrated) = migrated_db();

    let (_dir_a, path_a) = temp_db_path();
    let mut applied = open(&path_a).expect("open second index database");
    apply_schema(&mut applied).expect("apply the DDL directly");

    assert_eq!(
        master_names(&migrated, "table"),
        master_names(&applied, "table"),
        "the migrated table set diverged from the directly-applied one"
    );
    assert_eq!(
        named_indices(&migrated),
        named_indices(&applied),
        "the migrated index set diverged from the directly-applied one"
    );
}

// ---------------------------------------------------------------------------
// AC5 — foreign_keys=ON composes with the migration (the cascade still fires)
// ---------------------------------------------------------------------------

#[test]
fn foreign_key_cascade_works_on_a_migrated_database() {
    // `open` sets foreign_keys=ON, then `migrate` runs; this proves the two
    // compose — the self-referential parent_id ON DELETE CASCADE still tears out
    // descendants on a database built through the production path, not just one
    // built through `apply_schema` in tests/schema.rs.
    let (_dir, conn) = migrated_db();

    conn.execute(
        "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at)
         VALUES ('/vault/project.org', 1, 1, '2026-08-04T10:00:00')",
        [],
    )
    .expect("insert file");
    let file_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
         VALUES (?1, NULL, 'headline', 1, 0, 0, 100, 'Parent', '')",
        [file_id],
    )
    .expect("insert parent headline");
    let parent_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
         VALUES (?1, ?2, 'headline', 2, 0, 100, 150, 'Child', '')",
        rusqlite::params![file_id, parent_id],
    )
    .expect("insert child headline");
    let child_id = conn.last_insert_rowid();

    conn.execute("DELETE FROM headlines WHERE id = ?1", [parent_id])
        .expect("delete the parent headline");

    let survivors = conn
        .query_row(
            "SELECT count(*) FROM headlines WHERE id IN (?1, ?2)",
            rusqlite::params![parent_id, child_id],
            |row| row.get::<_, i64>(0),
        )
        .expect("count survivors");
    assert_eq!(
        survivors, 0,
        "the child survived the cascade — is foreign_keys=ON (set by open()) still in effect after migrate?"
    );
}
