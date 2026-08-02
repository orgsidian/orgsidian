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

use orgsidian_index::{open, SCHEMA_SQL};

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
    let conn = open(&path).expect("open index database");
    conn.execute_batch(SCHEMA_SQL)
        .expect("apply schema to a fresh database");
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
    let (_dir, conn) = initialized_db();

    let err = conn
        .execute_batch(SCHEMA_SQL)
        .expect_err("re-applying the schema must fail");

    assert!(
        err.to_string().contains("already exists"),
        "expected a duplicate-object error, got: {err}"
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

    let named: BTreeSet<String> = master_names(&conn, "index")
        .into_iter()
        .filter(|name| name.starts_with("idx_"))
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

    // foreign_keys is per-connection and non-persistent — this is the
    // assertion that fails if `open` ever stops setting it, and every
    // ON DELETE CASCADE in the schema depends on it.
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
        let conn = open(&path).expect("open index database");
        conn.execute_batch(SCHEMA_SQL).expect("apply schema");
    }

    let plain = Connection::open(&path).expect("reopen without going through open()");
    let journal_mode: String = pragma(&plain, "journal_mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
}

#[test]
fn cascades_are_a_no_op_without_foreign_keys() {
    // Why open() sets foreign_keys explicitly instead of trusting the build.
    //
    // SQLite's own default is OFF; the bundled amalgamation flips it ON via
    // -DSQLITE_DEFAULT_FOREIGN_KEYS=1, so a bare Connection::open on THIS
    // build happens to enforce keys. That is a property of a build flag, not
    // of the schema — turn it off and every ON DELETE CASCADE silently stops
    // working, leaving orphans behind rather than raising an error. This test
    // pins the consequence, so the `foreign_keys = ON` line in the locked set
    // can never be read as redundant tuning.
    let (_dir, path) = temp_db_path();
    let conn = open(&path).expect("open index database");
    conn.execute_batch(SCHEMA_SQL).expect("apply schema");

    let file_id = insert_file(&conn, "/vault/orphans.org");
    let headline_id = insert_headline(&conn, file_id, "Doomed", "");

    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable foreign keys");
    assert_eq!(pragma::<i64>(&conn, "foreign_keys"), 0);

    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect("delete file");

    let orphans: i64 = conn
        .query_row(
            "SELECT count(*) FROM headlines WHERE id = ?1",
            [headline_id],
            |row| row.get(0),
        )
        .expect("count orphans");
    assert_eq!(
        orphans, 1,
        "with foreign_keys OFF the cascade must not fire — if it did, this \
         test is no longer proving anything about open()"
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

#[test]
fn deleting_a_file_cascades_to_every_descendant() {
    let (_dir, conn) = initialized_db();

    let file_id = insert_file(&conn, "/vault/project.org");
    let headline_id = insert_headline(&conn, file_id, "Ship the report", "Body text");
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

    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])
        .expect("delete file");

    for (table, column, id) in [
        ("headlines", "id", headline_id),
        ("headlines", "id", child_id),
        ("tags", "headline_id", headline_id),
        ("properties", "headline_id", headline_id),
        ("clock_entries", "headline_id", headline_id),
        ("links", "headline_id", headline_id),
    ] {
        let remaining: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE {column} = ?1"),
                [id],
                |row| row.get(0),
            )
            .expect("count survivors");
        assert_eq!(
            remaining, 0,
            "{table}.{column} = {id} survived the cascade — is foreign_keys = ON?"
        );
    }
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
fn fts_headlines_folds_diacritics() {
    // `remove_diacritics 2` is the reason an unaccented query finds accented
    // text. A mis-ordered tokenize argument list would degrade this silently
    // rather than failing at DDL time.
    let (_dir, conn) = initialized_db();
    let file_id = insert_file(&conn, "/vault/travel.org");
    insert_indexed_headline(&conn, file_id, "Réunion au café", "");

    assert_eq!(match_count(&conn, "fts_headlines", "reunion"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "cafe"), 1);
    assert_eq!(match_count(&conn, "fts_headlines", "café"), 1);
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
