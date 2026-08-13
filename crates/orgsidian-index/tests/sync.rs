//! Story 3.6 (AC2): the transactional sync engine's FTS-staleness contract,
//! driven against a **real on-disk** WAL database through the LD-14 writer +
//! reader pool (never `:memory:` — WAL/pool semantics need a real file; Story
//! 3.5 `tests/concurrency.rs` precedent).
//!
//! These tests resolve the "No test covers FTS staleness on UPDATE or DELETE"
//! MED deferred row and the rowid-reuse landmine (`headlines.id` has no
//! `AUTOINCREMENT`): they prove that a headline's external-content FTS rows
//! never outlive the headline text, on UPDATE, on DELETE, and across a reused
//! rowid. Assertions are on real FTS5 `MATCH` hits/misses, not on text.
//!
//! # Anti-placebo (Story 1.9)
//!
//! `fts_delete_pairing_is_load_bearing` documents the mutation that must break
//! a staleness assertion: deleting the `'delete'`-emission loop in
//! `sync::clear_file_rows` makes `update_replaces_old_terms_with_new` and
//! `rowid_reuse_does_not_surface_a_stale_hit` FAIL (old term still matches; the
//! reused rowid resolves to the wrong live headline). Verified by hand during
//! implementation — see the story Debug Log — then restored to green.

use std::path::PathBuf;

use orgsidian_index::{
    delete_file, upsert_file, FileIndexInput, HeadlineInput, IndexPool, IndexWriter,
};
use tempfile::TempDir;

fn temp_db() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("index.sqlite3");
    (dir, path)
}

/// A single-headline file with no preamble — predictable rowids for the
/// reuse test.
fn file_input(rel_path: &str, title: &str, body: &str) -> FileIndexInput {
    FileIndexInput {
        rel_path: rel_path.to_string(),
        mtime_ns: 1,
        size_bytes: 1,
        preamble: None,
        headlines: vec![HeadlineInput {
            level: 1,
            position: 0,
            byte_start: 0,
            byte_end: 10,
            todo_keyword: None,
            todo_done: None,
            title: title.to_string(),
            body: body.to_string(),
            scheduled_date: None,
            scheduled_time: None,
            deadline_date: None,
            deadline_time: None,
            closed_date: None,
            closed_time: None,
            tags: Vec::new(),
            properties: Vec::new(),
            clock_entries: Vec::new(),
            links: Vec::new(),
            children: Vec::new(),
        }],
    }
}

async fn upsert(writer: &IndexWriter, input: FileIndexInput) {
    writer
        .execute(move |conn| upsert_file(conn, &input))
        .await
        .expect("upsert_file");
}

async fn remove(writer: &IndexWriter, rel_path: &str) {
    let rel_path = rel_path.to_string();
    writer
        .execute(move |conn| delete_file(conn, &rel_path))
        .await
        .expect("delete_file");
}

/// How many rows in `table` match the single-term `term`. `table` is one of the
/// FTS5 virtual tables; the whole-table `MATCH` form resolves through
/// `content_rowid` back to `headlines`, so a stale posting surfaces here.
async fn match_count(pool: &IndexPool, table: &'static str, term: &str) -> i64 {
    let term = term.to_string();
    pool.interact(move |conn| {
        let sql = format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1");
        conn.query_row(&sql, rusqlite::params![term], |row| row.get::<_, i64>(0))
            .map_err(Into::into)
    })
    .await
    .expect("match count")
}

/// Reading a matched column must not raise `fts5: missing row` — the failure a
/// skipped `'delete'` produces. Returns the number of rows the projection
/// yields (0 after a correct delete, never an error).
async fn snippet_probe(pool: &IndexPool, term: &str) -> i64 {
    let term = term.to_string();
    pool.interact(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT snippet(fts_content, 0, '[', ']', '…', 8)
             FROM fts_content WHERE fts_content MATCH ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![term], |row| row.get::<_, String>(0))?;
        let mut count = 0i64;
        for row in rows {
            // Materializing the snippet is what raises SQLITE_CORRUPT / missing
            // row on a stale external-content index; force it.
            let _ = row?;
            count += 1;
        }
        Ok(count)
    })
    .await
    .expect("snippet probe")
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_makes_terms_matchable() {
    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer");
    let pool = IndexPool::new(&db_path).expect("build pool");

    upsert(&writer, file_input("a.org", "alpha", "apple pie")).await;

    assert_eq!(match_count(&pool, "fts_headlines", "alpha").await, 1);
    assert_eq!(match_count(&pool, "fts_content", "apple").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_replaces_old_terms_with_new() {
    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer");
    let pool = IndexPool::new(&db_path).expect("build pool");

    upsert(&writer, file_input("a.org", "alpha", "apple pie")).await;
    // Re-upsert the SAME path with changed title + body.
    upsert(&writer, file_input("a.org", "beta", "banana bread")).await;

    // The OLD terms no longer match (the FTS 'delete' rows fired before the
    // headline was replaced) — this is the assertion the anti-placebo breaks.
    assert_eq!(match_count(&pool, "fts_headlines", "alpha").await, 0);
    assert_eq!(match_count(&pool, "fts_content", "apple").await, 0);
    // The NEW terms match.
    assert_eq!(match_count(&pool, "fts_headlines", "beta").await, 1);
    assert_eq!(match_count(&pool, "fts_content", "banana").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_removes_terms_without_missing_row_error() {
    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer");
    let pool = IndexPool::new(&db_path).expect("build pool");

    upsert(&writer, file_input("a.org", "gamma", "grape juice")).await;
    assert_eq!(match_count(&pool, "fts_content", "grape").await, 1);

    remove(&writer, "a.org").await;

    // No hit, and materializing a snippet does not raise `fts5: missing row`.
    assert_eq!(match_count(&pool, "fts_headlines", "gamma").await, 0);
    assert_eq!(snippet_probe(&pool, "grape").await, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn rowid_reuse_does_not_surface_a_stale_hit() {
    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer");
    let pool = IndexPool::new(&db_path).expect("build pool");

    // File A's single headline takes rowid 1.
    upsert(&writer, file_input("a.org", "gamma", "grape juice")).await;
    // Delete A: rowid 1 is freed AND its FTS 'delete' rows fire.
    remove(&writer, "a.org").await;
    // File B's single headline REUSES rowid 1 (no AUTOINCREMENT).
    upsert(&writer, file_input("b.org", "delta", "date syrup")).await;

    // A query for A's old term must return nothing — NOT B's row, which now
    // owns the reused rowid. A skipped 'delete' would make 'gamma' resolve
    // through content_rowid 1 to B's live "date syrup" body.
    assert_eq!(match_count(&pool, "fts_headlines", "gamma").await, 0);
    assert_eq!(match_count(&pool, "fts_content", "grape").await, 0);
    // B's own terms are correct.
    assert_eq!(match_count(&pool, "fts_headlines", "delta").await, 1);
    assert_eq!(match_count(&pool, "fts_content", "date").await, 1);
}

// --- Identity guard (AC2): application_id stamp + foreign/mismatch classify ---

#[tokio::test(flavor = "multi_thread")]
async fn fresh_stamped_index_classifies_as_ours() {
    use orgsidian_index::{inspect_index_file, stamp_application_id, IndexIdentity};

    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer"); // migrates → user_version 1
    writer
        .execute(|conn| stamp_application_id(conn))
        .await
        .expect("stamp application_id");
    writer.shutdown().await; // release the file before the read-only inspect

    assert_eq!(
        inspect_index_file(&db_path).expect("inspect ours"),
        IndexIdentity::Ours
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stamped_index_at_wrong_user_version_classifies_as_mismatch() {
    use orgsidian_index::{inspect_index_file, stamp_application_id, IndexIdentity};

    let (_dir, db_path) = temp_db();
    let writer = IndexWriter::spawn(&db_path).expect("spawn writer");
    writer
        .execute(|conn| {
            stamp_application_id(conn)?;
            // Simulate a future schema drift (LD-13 trigger).
            conn.pragma_update(None, "user_version", 2)?;
            Ok(())
        })
        .await
        .expect("stamp + bump user_version");
    writer.shutdown().await;

    assert_eq!(
        inspect_index_file(&db_path).expect("inspect mismatch"),
        IndexIdentity::VersionMismatch
    );
}

#[test]
fn unstamped_sqlite_file_classifies_as_foreign() {
    use orgsidian_index::{inspect_index_file, IndexIdentity};

    let (_dir, db_path) = temp_db();
    // A plain SQLite file created by "some other tool": application_id stays 0.
    let conn = rusqlite::Connection::open(&db_path).expect("open plain sqlite");
    conn.execute_batch("CREATE TABLE not_ours (x);")
        .expect("seed foreign schema");
    drop(conn);

    assert_eq!(
        inspect_index_file(&db_path).expect("inspect foreign"),
        IndexIdentity::Foreign
    );
}
