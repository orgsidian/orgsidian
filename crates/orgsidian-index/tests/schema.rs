//! Story 3.3 — schema shape + locked-PRAGMA behavior (LD-4, LD-11, LD-14, FR-17).
//!
//! Every test drives a REAL on-disk database in its own `TempDir`. `:memory:`
//! is deliberately never used: an in-memory database cannot enter WAL — it
//! reports `journal_mode = memory` — so a PRAGMA assertion against one is
//! meaningless, and the FK/FTS tests are cheap enough on disk that a second
//! fixture shape would only invite someone to use the wrong one.
//!
//! These assert BEHAVIOR, not DDL text (Story 1.9 anti-placebo rule):
//! `SCHEMA_SQL.contains("fts5")` proves nothing, opening a database and
//! matching a stemmed query through an external-content table does.

use std::collections::BTreeSet;
use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::TempDir;

use orgsidian_index::{apply_schema, open, IndexError, SCHEMA_SQL};

/// The eight normalized tables mandated by LD-11.
const EXPECTED_TABLES: &[&str] = &[
    "_schema_version",
    "clock_entries",
    "files",
    "headlines",
    "links",
    "properties",
    "tags",
    "vault_meta",
];

/// The two FTS5 virtual tables plus the shadow tables SQLite materializes for
/// them. An EXTERNAL-content FTS5 table gets exactly four shadow tables —
/// `_config`, `_data`, `_docsize`, `_idx`. There is no `_content` shadow: not
/// duplicating the text is the entire point of `content='headlines'`.
const EXPECTED_FTS_TABLES: &[&str] = &[
    "fts_content",
    "fts_content_config",
    "fts_content_data",
    "fts_content_docsize",
    "fts_content_idx",
    "fts_headlines",
    "fts_headlines_config",
    "fts_headlines_data",
    "fts_headlines_docsize",
    "fts_headlines_idx",
];

/// Every named index in the schema (AC4). The `sqlite_autoindex_*` entries
/// SQLite creates for the composite primary keys on `tags`/`properties` and
/// the `TEXT PRIMARY KEY` on `vault_meta` are intentionally excluded — they
/// are unnamed by us and not part of the convention.
const EXPECTED_INDICES: &[&str] = &[
    "idx_clock_entries_headline_id",
    "idx_files_path",
    "idx_headlines_deadline_date",
    "idx_headlines_file_id",
    "idx_headlines_parent_id",
    "idx_headlines_scheduled_date",
    "idx_links_file_id",
    "idx_links_headline_id",
    "idx_links_target",
    "idx_properties_headline_id",
    "idx_tags_tag_headline_id",
];

/// A fresh temp directory + the path a database should live at inside it.
///
/// The `TempDir` is returned alongside the path because dropping it deletes
/// the directory — a test that discards it pulls the database out from under
/// itself.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("index.db");
    (dir, path)
}

/// An open connection with the locked PRAGMAs applied and the schema created.
fn initialized_db() -> (TempDir, Connection) {
    let (dir, path) = temp_db_path();
    let mut conn = open(&path).expect("open index database");
    apply_schema(&mut conn).expect("apply schema to a fresh database");
    (dir, conn)
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

fn expected_set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_owned()).collect()
}

// ---------------------------------------------------------------------------
// AC1 / AC6 — the DDL applies, and only once
// ---------------------------------------------------------------------------

#[test]
fn schema_applies_to_a_fresh_database() {
    let (_dir, conn) = initialized_db();

    // Cheapest possible proof the batch really ran end-to-end: the last
    // statement in the file is an index creation, so if the batch had stopped
    // early this lookup would miss.
    let target_index: String = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_links_target'",
            [],
            |row| row.get(0),
        )
        .expect("the final statement of the batch executed");
    assert_eq!(target_index, "idx_links_target");
}

#[test]
fn re_applying_the_schema_fails_loudly() {
    // The DDL is deliberately NOT `IF NOT EXISTS`-guarded: a migration runner
    // that re-applies version 1 over an initialized database has a bug, and a
    // silently idempotent schema would hide it.
    let (_dir, mut conn) = initialized_db();

    let err = apply_schema(&mut conn).expect_err("re-applying the schema must fail");

    assert!(
        err.to_string().contains("already exists"),
        "expected a duplicate-object error, got: {err}"
    );
}

#[test]
fn a_failed_schema_application_leaves_no_partial_database() {
    // The duplicate-object error above is NOT sufficient on its own: a
    // half-built database raises the byte-identical "table files already
    // exists". What distinguishes the two is whether a failed apply leaves
    // anything behind — and executing SCHEMA_SQL bare does, permanently,
    // because execute_batch commits each DDL statement in its own implicit
    // transaction. apply_schema wraps the batch so the failure rolls back.
    let (_dir, path) = temp_db_path();
    let mut conn = open(&path).expect("open index database");

    // A plausible botched prior run: one table from the middle of the file
    // already present, so the batch gets a long way in before colliding.
    conn.execute_batch(
        "CREATE TABLE vault_meta (
             key        TEXT NOT NULL PRIMARY KEY,
             value      TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );",
    )
    .expect("seed a partially-initialized database");

    apply_schema(&mut conn).expect_err("applying over a partial database must fail");

    let tables = master_names(&conn, "table");
    assert_eq!(
        tables,
        expected_set(&["vault_meta"]),
        "the rolled-back apply must leave exactly what was there before, but \
         found: {tables:?}"
    );
}

// ---------------------------------------------------------------------------
// AC2 / AC3 / AC6 — exact object sets
// ---------------------------------------------------------------------------

#[test]
fn table_set_is_exactly_the_eight_tables_plus_fts_machinery() {
    let (_dir, conn) = initialized_db();

    let actual = master_names(&conn, "table");
    let mut expected = expected_set(EXPECTED_TABLES);
    expected.extend(expected_set(EXPECTED_FTS_TABLES));

    // Asserting the whole set (not just presence) is the point: an accidental
    // ninth table, or a `content=` typo that makes FTS5 materialize a
    // `_content` shadow and silently duplicate the vault text, fails here.
    assert_eq!(actual, expected);
}

#[test]
fn named_index_set_is_exactly_the_ld_11_set() {
    let (_dir, conn) = initialized_db();

    // Filtering on the `idx_` prefix would let an index that VIOLATES the
    // naming convention (`files_path_ix`, `tmp_debug_idx2`) slip through
    // unnoticed — precisely the case worth catching. Only SQLite's own
    // autoindexes, which we do not name, are excluded.
    let named: BTreeSet<String> = master_names(&conn, "index")
        .into_iter()
        .filter(|name| !name.starts_with("sqlite_autoindex_"))
        .collect();

    // A future dropped index must fail CI rather than silently degrade agenda
    // latency into a full table scan.
    assert_eq!(named, expected_set(EXPECTED_INDICES));
}

#[test]
fn idx_files_path_is_unique() {
    let (_dir, conn) = initialized_db();

    let unique: i64 = conn
        .query_row(
            "SELECT \"unique\" FROM pragma_index_list('files') WHERE name = 'idx_files_path'",
            [],
            |row| row.get(0),
        )
        .expect("idx_files_path is listed on files");
    assert_eq!(unique, 1, "idx_files_path must be UNIQUE");

    insert_file(&conn, "/vault/notes.org");
    let duplicate = conn.execute(
        "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at) VALUES (?1, 0, 0, '2026-08-02T10:00:00')",
        ["/vault/notes.org"],
    );
    assert!(
        duplicate.is_err(),
        "a duplicate path must be rejected by the UNIQUE index"
    );
}

#[test]
fn schema_declares_no_triggers() {
    let (_dir, conn) = initialized_db();

    // LD-11 mandates application-managed FTS sync. A trigger here would move
    // that obligation into the schema, where Story 3.6 cannot see it.
    let triggers: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'trigger'",
            [],
            |row| row.get(0),
        )
        .expect("count triggers");
    assert_eq!(triggers, 0);
}

// ---------------------------------------------------------------------------
// AC5 / AC6 — locked PRAGMAs, read back from a real connection
// ---------------------------------------------------------------------------

#[test]
fn open_applies_every_locked_pragma() {
    let (_dir, path) = temp_db_path();
    let conn = open(&path).expect("open index database");

    let journal_mode: String = pragma(&conn, "journal_mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

    // The read-back values are not the strings that were written:
    // synchronous = NORMAL reports 1, temp_store = MEMORY reports 2.
    assert_eq!(pragma::<i64>(&conn, "synchronous"), 1);
    assert_eq!(pragma::<i64>(&conn, "temp_store"), 2);
    assert_eq!(pragma::<i64>(&conn, "cache_size"), -64_000);
    assert_eq!(pragma::<i64>(&conn, "wal_autocheckpoint"), 4_000);

    // foreign_keys is per-connection and non-persistent. NOTE: this assertion
    // cannot catch its removal from the locked set on THIS build — the bundled
    // amalgamation compiles with -DSQLITE_DEFAULT_FOREIGN_KEYS=1, so a bare
    // connection already reports 1. It only pins the value; what makes the
    // explicit PRAGMA load-bearing is that the default is a build flag, not a
    // guarantee. Recorded in deferred-work.
    assert_eq!(pragma::<i64>(&conn, "foreign_keys"), 1);

    // mmap_size is asserted as "enabled", not as an exact match: the build's
    // SQLITE_MAX_MMAP_SIZE can clamp the requested 268435456 to something
    // smaller, and a clamped value is a healthy outcome rather than an error.
    assert!(
        pragma::<i64>(&conn, "mmap_size") > 0,
        "memory mapping must be enabled (a clamped-but-positive value is fine)"
    );
}

#[test]
fn wal_mode_persists_into_the_database_file() {
    // journal_mode is a property of the file, not of the connection — proving
    // it survives a close/reopen distinguishes "WAL was really engaged" from
    // "the PRAGMA statement returned a row we believed".
    let (_dir, path) = temp_db_path();
    {
        let mut conn = open(&path).expect("open index database");
        apply_schema(&mut conn).expect("apply schema");
    }

    let plain = Connection::open(&path).expect("reopen without going through open()");
    let journal_mode: String = pragma(&plain, "journal_mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
}

#[test]
fn schema_sql_executes_inside_a_caller_supplied_transaction() {
    // The Story 3.4 seam: a migration runner wraps every migration in its own
    // transaction, so SCHEMA_SQL must contain no BEGIN of its own — a nested
    // one fails with "cannot start a transaction within a transaction". This
    // asserts the property behaviorally rather than grepping the DDL text.
    let (_dir, path) = temp_db_path();
    let mut conn = open(&path).expect("open index database");

    let tx = conn
        .transaction()
        .expect("begin a caller-owned transaction");
    tx.execute_batch(SCHEMA_SQL)
        .expect("the DDL must be executable inside someone else's transaction");
    tx.commit().expect("commit");

    let mut expected = expected_set(EXPECTED_TABLES);
    expected.extend(expected_set(EXPECTED_FTS_TABLES));
    assert_eq!(master_names(&conn, "table"), expected);
}

#[test]
fn foreign_keys_off_disables_both_the_cascade_and_the_restriction() {
    // Why open() sets foreign_keys explicitly instead of trusting the build.
    //
    // SQLite's own default is OFF; the bundled amalgamation flips it ON via
    // -DSQLITE_DEFAULT_FOREIGN_KEYS=1, so a bare Connection::open on THIS
    // build happens to enforce keys. That is a property of a build flag, not
    // of the schema. With foreign keys off, BOTH referential behaviors this
    // schema relies on evaporate at once: the ON DELETE CASCADEs stop firing
    // (leaving orphans instead of raising), and the deliberate NO ACTION on
    // headlines.file_id stops rejecting the delete that would corrupt the FTS
    // index. This test pins both consequences, so the `foreign_keys = ON` line
    // in the locked set can never be read as redundant tuning.
    let (_dir, conn) = initialized_db();

    let file_id = insert_file(&conn, "/vault/orphans.org");
    let headline_id = insert_headline(&conn, file_id, "Doomed", "");
    conn.execute(
        "INSERT INTO tags (headline_id, tag, position) VALUES (?1, 'work', 0)",
        [headline_id],
    )
    .expect("insert tag");

    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable foreign keys");
    assert_eq!(pragma::<i64>(&conn, "foreign_keys"), 0);

    // (a) The cascade does not fire: the tag outlives its headline.
    conn.execute("DELETE FROM headlines WHERE id = ?1", [headline_id])
        .expect("delete headline");
    assert_eq!(
        count_where(&conn, "tags", "headline_id", headline_id),
        1,
        "with foreign_keys OFF the cascade must not fire — if it did, this \
         test is no longer proving anything about open()"
    );

    // (b) The restriction does not hold either. Re-insert a headline so the
    // file delete has something to be rejected over, and watch it succeed.
    let orphan_id = insert_headline(&conn, file_id, "Also doomed", "");
    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect("with foreign_keys OFF nothing rejects this");
    assert_eq!(
        count_where(&conn, "headlines", "id", orphan_id),
        1,
        "the headline should have been orphaned, not deleted"
    );
}

/// `PRAGMA <name>` read via the statement form.
///
/// `mmap_size` and `wal_autocheckpoint` have no `pragma_*` table-valued
/// function, so `SELECT * FROM pragma_mmap_size` is not available; the
/// statement form (what `pragma_query_value` emits) works for all of them.
fn pragma<T: rusqlite::types::FromSql>(conn: &Connection, name: &str) -> T {
    conn.pragma_query_value(None, name, |row| row.get(0))
        .unwrap_or_else(|e| panic!("read PRAGMA {name}: {e}"))
}

// ---------------------------------------------------------------------------
// AC2 / AC6 — referential behavior
// ---------------------------------------------------------------------------

fn insert_file(conn: &Connection, path: &str) -> i64 {
    conn.execute(
        "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at)
         VALUES (?1, 1234567890, 42, '2026-08-02T10:00:00')",
        [path],
    )
    .expect("insert file");
    conn.last_insert_rowid()
}

fn insert_headline(conn: &Connection, file_id: i64, title: &str, body: &str) -> i64 {
    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end,
              todo_keyword, todo_done, title, body, scheduled_date)
         VALUES (?1, NULL, 'headline', 1, 0, 0, 100, 'TODO', 0, ?2, ?3, '2026-08-02')",
        rusqlite::params![file_id, title, body],
    )
    .expect("insert headline");
    conn.last_insert_rowid()
}

fn count_where(conn: &Connection, table: &str, column: &str, id: i64) -> i64 {
    conn.query_row(
        &format!("SELECT count(*) FROM {table} WHERE {column} = ?1"),
        [id],
        |row| row.get(0),
    )
    .unwrap_or_else(|e| panic!("count {table}.{column} = {id}: {e}"))
}

/// Insert a headline, a child headline, and one row in every dependent table.
fn insert_a_full_subtree(conn: &Connection, file_id: i64) -> (i64, i64) {
    let headline_id = insert_headline(conn, file_id, "Ship the report", "Body text");
    let child_id = conn
        .execute(
            "INSERT INTO headlines
                 (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
             VALUES (?1, ?2, 'headline', 2, 0, 100, 150, 'Subtask', '')",
            rusqlite::params![file_id, headline_id],
        )
        .map(|_| conn.last_insert_rowid())
        .expect("insert child headline");

    conn.execute(
        "INSERT INTO tags (headline_id, tag, position) VALUES (?1, 'work', 0)",
        [headline_id],
    )
    .expect("insert tag");
    conn.execute(
        "INSERT INTO properties (headline_id, key, value) VALUES (?1, 'ID', 'abc-123')",
        [headline_id],
    )
    .expect("insert property");
    conn.execute(
        "INSERT INTO clock_entries (headline_id, start_at, end_at, duration_seconds)
         VALUES (?1, '2026-08-02T09:00:00', NULL, NULL)",
        [headline_id],
    )
    .expect("insert clock entry");
    conn.execute(
        "INSERT INTO links (file_id, headline_id, kind, target, description)
         VALUES (?1, ?2, 'id', 'abc-123', 'see also')",
        rusqlite::params![file_id, headline_id],
    )
    .expect("insert link");

    (headline_id, child_id)
}

#[test]
fn deleting_a_file_is_rejected_while_its_headlines_remain() {
    // headlines.file_id deliberately carries NO cascade. A cascade fires inside
    // SQLite, which would tear the rows out from under both external-content
    // FTS5 tables with no application hook and no recoverable text — leaving an
    // index that raises SQLITE_CORRUPT on snippet(). NO ACTION converts that
    // silent corruption into a loud, immediate rejection, which forces the sync
    // engine through the delete path that owns the FTS obligation.
    let (_dir, conn) = initialized_db();

    let file_id = insert_file(&conn, "/vault/project.org");
    let (headline_id, _child_id) = insert_a_full_subtree(&conn, file_id);

    let err = conn
        .execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect_err("deleting a file with live headlines must be rejected");
    assert!(
        err.to_string().contains("FOREIGN KEY constraint failed"),
        "expected a foreign-key rejection, got: {err}"
    );

    assert_eq!(count_where(&conn, "headlines", "id", headline_id), 1);
    assert_eq!(count_where(&conn, "files", "id", file_id), 1);

    // Once the headlines are gone the file row deletes cleanly.
    conn.execute("DELETE FROM headlines WHERE file_id = ?1", [file_id])
        .expect("delete the file's headlines first");
    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect("the file row now has no dependents");
    assert_eq!(count_where(&conn, "files", "id", file_id), 0);
}

#[test]
fn deleting_a_headline_cascades_to_every_descendant() {
    let (_dir, conn) = initialized_db();

    let file_id = insert_file(&conn, "/vault/project.org");
    let (headline_id, child_id) = insert_a_full_subtree(&conn, file_id);

    // Deleting the PARENT headline — not the file — is what exercises the
    // self-referential parent_id cascade. Deleting the file would explain the
    // child's disappearance through its own file_id and prove nothing about it.
    conn.execute("DELETE FROM headlines WHERE id = ?1", [headline_id])
        .expect("delete the parent headline");

    for (table, column, id) in [
        ("headlines", "id", headline_id),
        ("headlines", "id", child_id),
        ("tags", "headline_id", headline_id),
        ("properties", "headline_id", headline_id),
        ("clock_entries", "headline_id", headline_id),
        ("links", "headline_id", headline_id),
    ] {
        assert_eq!(
            count_where(&conn, table, column, id),
            0,
            "{table}.{column} = {id} survived the cascade — is foreign_keys = ON?"
        );
    }

    // The file row itself is untouched: only headlines cascade from headlines.
    assert_eq!(count_where(&conn, "files", "id", file_id), 1);
}

#[test]
fn the_sanctioned_delete_order_leaves_the_fts_index_queryable() {
    // The contract schema.sql spells out for Story 3.6: write the 'delete'
    // command rows while the text is still readable, THEN delete the headlines,
    // THEN the file. Following it, search stays healthy.
    //
    // This test does NOT guard the missing cascade — verified by mutation: it
    // stays green with ON DELETE CASCADE reinstated, because it never takes the
    // path the cascade would hijack. `deleting_a_file_is_rejected_while_its_
    // headlines_remain` is the test that fails there. What this one pins is
    // that the sanctioned order actually works, so the contract is executable
    // rather than merely written down in a comment.
    let (_dir, conn) = initialized_db();

    let file_id = insert_file(&conn, "/vault/doomed.org");
    let headline_id = insert_indexed_headline(&conn, file_id, "Quarterly", "revenue projections");
    assert_eq!(match_count(&conn, "fts_content", "revenue"), 1);

    conn.execute(
        "INSERT INTO fts_headlines (fts_headlines, rowid, title) VALUES ('delete', ?1, 'Quarterly')",
        [headline_id],
    )
    .expect("retire the fts_headlines row while its text is still readable");
    conn.execute(
        "INSERT INTO fts_content (fts_content, rowid, body)
         VALUES ('delete', ?1, 'revenue projections')",
        [headline_id],
    )
    .expect("retire the fts_content row");

    conn.execute("DELETE FROM headlines WHERE file_id = ?1", [file_id])
        .expect("delete headlines");
    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect("delete file");

    // No stale hit, and — the part that fails on the cascade path — reading a
    // column and calling snippet() succeed instead of raising SQLITE_CORRUPT.
    assert_eq!(match_count(&conn, "fts_content", "revenue"), 0);
    let rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM (SELECT body, snippet(fts_content, 0, '[', ']', '...', 8)
                                   FROM fts_content WHERE fts_content MATCH 'revenue')",
            [],
            |row| row.get(0),
        )
        .expect("reading through the external content table must not raise SQLITE_CORRUPT");
    assert_eq!(rows, 0);
}

#[test]
fn clock_entries_accept_a_running_clock() {
    // end_at NULL is the running-clock encoding Story 7.7's prior-session
    // prompt reads; a NOT NULL here would make that feature unrepresentable.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/time.org");
    let headline_id = insert_headline(&conn, file_id, "Focus block", "");

    conn.execute(
        "INSERT INTO clock_entries (headline_id, start_at) VALUES (?1, '2026-08-02T09:00:00')",
        [headline_id],
    )
    .expect("a clock entry with no end must be accepted");

    let running: i64 = conn
        .query_row(
            "SELECT count(*) FROM clock_entries WHERE end_at IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("count running clocks");
    assert_eq!(running, 1);
}

#[test]
fn links_accept_a_preamble_row_and_reject_an_unknown_kind() {
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/preamble.org");

    // A link before the first headline has no owning headline.
    conn.execute(
        "INSERT INTO links (file_id, headline_id, kind, target)
         VALUES (?1, NULL, 'url', 'https://orgmode.org')",
        [file_id],
    )
    .expect("a preamble link with NULL headline_id must be accepted");

    let err = conn
        .execute(
            "INSERT INTO links (file_id, headline_id, kind, target)
             VALUES (?1, NULL, 'gopher', 'gopher://example')",
            [file_id],
        )
        .expect_err("an unknown link kind must be rejected");
    assert!(
        err.to_string().contains("CHECK constraint failed"),
        "expected a CHECK constraint violation, got: {err}"
    );
}

#[test]
fn headlines_accept_a_preamble_row_and_reject_an_unknown_kind() {
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/kinds.org");

    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
         VALUES (?1, NULL, 'preamble', 0, 0, 0, 40, '', '#+TITLE: Notes')",
        [file_id],
    )
    .expect("a preamble row must be accepted");

    let err = conn
        .execute(
            "INSERT INTO headlines
                 (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
             VALUES (?1, NULL, 'footnote', 1, 1, 40, 80, 'x', '')",
            [file_id],
        )
        .expect_err("an unknown headline kind must be rejected");
    assert!(
        err.to_string().contains("CHECK constraint failed"),
        "expected a CHECK constraint violation, got: {err}"
    );
}

#[test]
fn properties_collapse_duplicate_keys_last_wins() {
    // Matches the parser's documented duplicate-key behavior: the PK on
    // (headline_id, key) is what makes an UPSERT the natural write.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/props.org");
    let headline_id = insert_headline(&conn, file_id, "Task", "");

    conn.execute(
        "INSERT INTO properties (headline_id, key, value) VALUES (?1, 'EFFORT', '1h')",
        [headline_id],
    )
    .expect("insert property");
    conn.execute(
        "INSERT INTO properties (headline_id, key, value) VALUES (?1, 'EFFORT', '2h')
         ON CONFLICT (headline_id, key) DO UPDATE SET value = excluded.value",
        [headline_id],
    )
    .expect("upsert property");

    let value: String = conn
        .query_row(
            "SELECT value FROM properties WHERE headline_id = ?1 AND key = 'EFFORT'",
            [headline_id],
            |row| row.get(0),
        )
        .expect("read property");
    assert_eq!(value, "2h");
}

// ---------------------------------------------------------------------------
// AC3 / AC6 — FTS5 round-trip and tokenizer behavior
// ---------------------------------------------------------------------------

/// Insert a headline and mirror it into both FTS tables, the way the Story 3.6
/// sync engine will have to (external-content tables index nothing on their
/// own — that is the trade for not duplicating the text).
fn insert_indexed_headline(conn: &Connection, file_id: i64, title: &str, body: &str) -> i64 {
    let headline_id = insert_headline(conn, file_id, title, body);
    conn.execute(
        "INSERT INTO fts_headlines (rowid, title) VALUES (?1, ?2)",
        rusqlite::params![headline_id, title],
    )
    .expect("mirror into fts_headlines");
    conn.execute(
        "INSERT INTO fts_content (rowid, body) VALUES (?1, ?2)",
        rusqlite::params![headline_id, body],
    )
    .expect("mirror into fts_content");
    headline_id
}

fn match_count(conn: &Connection, table: &str, query: &str) -> i64 {
    conn.query_row(
        &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
        [query],
        |row| row.get(0),
    )
    .unwrap_or_else(|e| panic!("MATCH {query:?} against {table}: {e}"))
}

#[test]
fn fts_headlines_folds_diacritics_at_level_two() {
    // The assertions below must discriminate `remove_diacritics 2` from `1`,
    // not merely from `0`. Latin-1 accents do not: `café` matches `cafe` under
    // both levels, so a test built only on those stays green if someone drops
    // the `2`. Level 1 handles only diacritics encoded as separate codepoints;
    // level 2 (SQLite >= 3.27) also folds those baked into the base codepoint,
    // which is what Vietnamese needs — verified: matching `nguyen` against an
    // indexed `Nguyễn` returns 0 under level 1 and 1 under level 2.
    //
    // A mis-ordered tokenize argument list is NOT a risk this test covers: both
    // wrong orders fail loudly at CREATE VIRTUAL TABLE with "error in tokenizer
    // constructor", so the schema could not have been created at all.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/travel.org");
    insert_indexed_headline(&conn, file_id, "Réunion au café", "");
    insert_indexed_headline(&conn, file_id, "Nguyễn Việt Hải", "");

    // Level 1 or 2 — folding is live at all.
    assert_eq!(match_count(&conn, "fts_headlines", "reunion"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "cafe"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "café"), 1);

    // Level 2 only.
    assert_eq!(match_count(&conn, "fts_headlines", "nguyen"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "viet"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "hai"), 1);

    assert_eq!(match_count(&conn, "fts_headlines", "brunch"), 0);
}

#[test]
fn fts_content_applies_porter_stemming() {
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/journal.org");
    insert_indexed_headline(&conn, file_id, "Morning", "She was running along the river");

    // porter stems both sides of the comparison, so the query term need not
    // match the indexed surface form.
    assert_eq!(match_count(&conn, "fts_content", "run"), 1);
    assert_eq!(match_count(&conn, "fts_content", "runs"), 1);
    assert_eq!(match_count(&conn, "fts_content", "running"), 1);
    assert_eq!(match_count(&conn, "fts_content", "swimming"), 0);
}

#[test]
fn fts_reads_text_back_through_the_external_content_table() {
    // The external-content contract: fts_content stores no text of its own, so
    // snippet() can only work by reading headlines.body through content_rowid.
    // If the column names ever drift apart, this is what catches it.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/notes.org");
    let headline_id =
        insert_indexed_headline(&conn, file_id, "Deep work", "Indexing makes search fast");

    let snippet: String = conn
        .query_row(
            "SELECT snippet(fts_content, 0, '[', ']', '...', 8)
             FROM fts_content WHERE fts_content MATCH 'index'",
            [],
            |row| row.get(0),
        )
        .expect("snippet through the external content table");
    assert!(
        snippet.contains("[Indexing]"),
        "snippet did not highlight the match: {snippet}"
    );

    let rowid: i64 = conn
        .query_row(
            "SELECT rowid FROM fts_headlines WHERE fts_headlines MATCH 'deep'",
            [],
            |row| row.get(0),
        )
        .expect("fts_headlines rowid");
    assert_eq!(
        rowid, headline_id,
        "content_rowid must resolve to headlines.id"
    );
}

// ---------------------------------------------------------------------------
// AC5 / AC6 — open() failure paths
// ---------------------------------------------------------------------------
//
// `IndexError::Pragma` is not reachable from here: every locked PRAGMA takes
// correctly on a healthy on-disk database, and there is no portable way to make
// one refuse. The failing side of the read-back verification is unit-tested in
// `src/connection.rs` instead, where the variant can be constructed.

fn assert_is_sqlite_error(err: IndexError, context: &str) {
    assert!(
        matches!(err, IndexError::Sqlite(_)),
        "{context}: expected IndexError::Sqlite, got {err:?}"
    );
}

#[test]
fn open_fails_when_the_parent_directory_does_not_exist() {
    let (dir, _path) = temp_db_path();
    let missing = dir.path().join("no-such-dir").join("index.db");

    let err = open(&missing).expect_err("a missing parent directory must not be created");
    assert_is_sqlite_error(err, "missing parent directory");
}

#[test]
fn open_fails_when_the_path_is_a_directory() {
    let (dir, _path) = temp_db_path();
    let as_dir = dir.path().join("index.db");
    std::fs::create_dir(&as_dir).expect("create a directory where the database should be");

    let err = open(&as_dir).expect_err("a directory is not a database");
    assert_is_sqlite_error(err, "path is a directory");
}

#[test]
fn open_fails_when_the_file_is_not_a_database() {
    // Connection::open is lazy — it does not touch the file — so this failure
    // surfaces from the first locked PRAGMA rather than from the open itself.
    // Either way the caller gets an error instead of a connection to garbage.
    let (dir, _path) = temp_db_path();
    let junk = dir.path().join("index.db");
    std::fs::write(&junk, b"this is an .org file, not a database\n").expect("write junk");

    let err = open(&junk).expect_err("a non-database file must be rejected");
    assert_is_sqlite_error(err, "file is not a database");
}

// ---------------------------------------------------------------------------
// AC2 / AC6 — the structural CHECKs
// ---------------------------------------------------------------------------
//
// These cover states the PARSER CANNOT EMIT, where a violation is by
// construction a Story 3.6 sync-engine bug. Content-shaped values a real .org
// file can produce (duplicate tags, empty strings, `level` outside 1..6, a
// clock that is both running and timed) stay deliberately representable — see
// the rationale block above `CREATE TABLE headlines`.

#[test]
fn a_headline_cannot_be_its_own_parent() {
    // A parent_id cycle makes any WITH RECURSIVE subtree walk — the stated
    // purpose of idx_headlines_parent_id — non-terminating.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/cycles.org");
    let headline_id = insert_headline(&conn, file_id, "Root", "");

    let err = conn
        .execute(
            "UPDATE headlines SET parent_id = id WHERE id = ?1",
            [headline_id],
        )
        .expect_err("a self-parent must be rejected");
    assert!(
        err.to_string().contains("CHECK constraint failed"),
        "expected a CHECK constraint violation, got: {err}"
    );
}

#[test]
fn a_headline_span_cannot_be_inverted() {
    // Headline.span is a Rust Range, so the parser cannot produce this; a
    // consumer slicing `source[byte_start..byte_end]` on it would panic.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/spans.org");

    let err = conn
        .execute(
            "INSERT INTO headlines
                 (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
             VALUES (?1, NULL, 'headline', 1, 0, 500, 100, 'Backwards', '')",
            [file_id],
        )
        .expect_err("byte_end < byte_start must be rejected");
    assert!(
        err.to_string().contains("CHECK constraint failed"),
        "expected a CHECK constraint violation, got: {err}"
    );
}

#[test]
fn a_todo_keyword_and_its_done_flag_are_present_or_absent_together() {
    // Headline.todo_state is a single Option, so a half-populated pair is a
    // sync bug. `todo_done IN (0, 1)` alone does not catch it: a CHECK over a
    // NULL column evaluates to NULL and passes.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/todos.org");

    for (keyword, done) in [(Some("DONE"), None), (None, Some(1))] {
        let err = conn
            .execute(
                "INSERT INTO headlines
                     (file_id, parent_id, kind, level, position, byte_start, byte_end,
                      todo_keyword, todo_done, title, body)
                 VALUES (?1, NULL, 'headline', 1, 0, 0, 10, ?2, ?3, 'Half', '')",
                rusqlite::params![file_id, keyword, done],
            )
            .expect_err("a half-populated TODO pair must be rejected");
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "expected a CHECK constraint violation for ({keyword:?}, {done:?}), got: {err}"
        );
    }

    // Both present and both absent are the two legal shapes.
    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end,
              todo_keyword, todo_done, title, body)
         VALUES (?1, NULL, 'headline', 1, 0, 0, 10, 'TODO', 0, 'Open', '')",
        [file_id],
    )
    .expect("a fully-populated TODO pair is accepted");
    conn.execute(
        "INSERT INTO headlines
             (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
         VALUES (?1, NULL, 'headline', 1, 1, 10, 20, 'Plain', '')",
        [file_id],
    )
    .expect("no TODO state at all is accepted");
}

#[test]
fn a_quarantined_file_must_carry_a_reason() {
    // The LD-41 malformed-file row exists to be shown to the user; one with
    // nothing to show is a sync bug, not a document.
    let (_dir, conn) = initialized_db();

    let err = conn
        .execute(
            "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at, quarantined)
             VALUES ('/vault/broken.org', 0, 0, '2026-08-02T10:00:00', 1)",
            [],
        )
        .expect_err("quarantined = 1 with no reason must be rejected");
    assert!(
        err.to_string().contains("CHECK constraint failed"),
        "expected a CHECK constraint violation, got: {err}"
    );

    conn.execute(
        "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at, quarantined, quarantine_reason)
         VALUES ('/vault/broken.org', 0, 0, '2026-08-02T10:00:00', 1, 'parse error at line 4')",
        [],
    )
    .expect("a quarantined file with a reason is accepted");
}

#[test]
fn vault_meta_rejects_a_null_key() {
    // Only an INTEGER PRIMARY KEY implies NOT NULL: a bare `TEXT PRIMARY KEY`
    // accepts NULL, and accepts it repeatedly, because the PK index treats
    // NULLs as distinct. The explicit NOT NULL is what closes it.
    let (_dir, conn) = initialized_db();

    let err = conn
        .execute(
            "INSERT INTO vault_meta (key, value, updated_at)
             VALUES (NULL, 'x', '2026-08-02T10:00:00')",
            [],
        )
        .expect_err("a NULL key must be rejected");
    assert!(
        err.to_string().contains("NOT NULL constraint failed"),
        "expected a NOT NULL constraint violation, got: {err}"
    );

    conn.execute(
        "INSERT INTO vault_meta (key, value, updated_at)
         VALUES ('tokenizer', 'porter unicode61 remove_diacritics 2', '2026-08-02T10:00:00')",
        [],
    )
    .expect("a real key is accepted");
    let err = conn
        .execute(
            "INSERT INTO vault_meta (key, value, updated_at)
             VALUES ('tokenizer', 'other', '2026-08-02T10:00:00')",
            [],
        )
        .expect_err("the primary key must still reject a duplicate");
    assert!(
        err.to_string().contains("UNIQUE constraint failed"),
        "expected a UNIQUE constraint violation, got: {err}"
    );
}
