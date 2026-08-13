//! Story 3.5 (AC4): the concurrency contract of the LD-14 reader pool + LD-7
//! single-writer task, driven against a **real on-disk** WAL database.
//!
//! `:memory:` is deliberately never used: WAL, `busy_timeout`, and a shared
//! connection pool all need a real file (an in-memory database is per-connection,
//! so a pool of them would not even share state). Every test gets its own
//! `TempDir`. Runs on a multi-thread runtime so the many async tasks and the
//! `spawn_blocking` bodies genuinely overlap — a current-thread runtime would
//! serialize them and hide a real deadlock.

use std::path::PathBuf;
use std::sync::Arc;

use orgsidian_index::{IndexError, IndexPool, IndexWriter};
use tempfile::TempDir;

/// A fresh on-disk database path inside a `TempDir` that must be kept alive for
/// the duration of the test (dropping it deletes the directory).
fn temp_db() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("index.db");
    (dir, path)
}

/// Read a `vault_meta` value as text.
fn read_meta(conn: &rusqlite::Connection, key: &str) -> Result<String, IndexError> {
    conn.query_row(
        "SELECT value FROM vault_meta WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .map_err(IndexError::from)
}

/// Upsert a `vault_meta` key/value.
fn write_meta(conn: &mut rusqlite::Connection, key: &str, value: &str) -> Result<(), IndexError> {
    conn.execute(
        "INSERT INTO vault_meta (key, value, updated_at) VALUES (?1, ?2, '')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(IndexError::from)?;
    Ok(())
}

/// 16 concurrent reads on a 4-reader pool complete with no deadlock and no pool
/// exhaustion, and every connection is returned to the pool afterwards.
///
/// `deadpool` queues `get()` calls beyond `max_size`, so the property under test
/// is: all 16 reads are eventually served by only 4 connections (none starved),
/// and none of the 4 leaked.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sixteen_concurrent_reads_on_four_readers_never_deadlock_and_pool_returns_to_full() {
    let (_dir, path) = temp_db();

    // The writer migrates once, making the schema exist before any read is
    // valid. Kept alive for the whole test.
    let writer = IndexWriter::spawn(&path).expect("spawn writer (migrates schema)");
    let pool = IndexPool::new(&path).expect("build reader pool");

    let mut handles = Vec::new();
    for _ in 0..16 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move {
            pool.interact(|conn| {
                conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                    .map_err(IndexError::from)
            })
            .await
        }));
    }

    for handle in handles {
        let value = handle
            .await
            .expect("read task did not panic")
            .expect("read served without deadlock or pool exhaustion");
        assert_eq!(value, 1);
    }

    // All four connections came back — none leaked across an await.
    let status = pool.status();
    assert_eq!(status.max_size, 4, "pool is sized to LD-14's default of 4");
    assert_eq!(
        status.available, 4,
        "every pooled connection was returned after the burst"
    );

    writer.shutdown().await;
}

/// The single writer serializes 32 concurrent read-modify-write increments with
/// no lost updates and no `SQLITE_BUSY`.
///
/// Each `execute` reads the counter and writes back `+1` — a read-modify-write
/// whose window is wide enough that *any* interleaving would lose updates. That
/// the final total is exactly 32 proves the writer applied them strictly in
/// sequence (LD-7 Single Writer Rule).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn writer_serializes_thirty_two_concurrent_writes_with_no_lost_updates() {
    let (_dir, path) = temp_db();

    let writer = Arc::new(IndexWriter::spawn(&path).expect("spawn writer"));

    // Seed the counter at zero.
    writer
        .execute(|conn| write_meta(conn, "counter", "0"))
        .await
        .expect("seed counter");

    let mut handles = Vec::new();
    for _ in 0..32 {
        let writer = Arc::clone(&writer);
        handles.push(tokio::spawn(async move {
            writer
                .execute(|conn| {
                    let current: i64 = conn
                        .query_row(
                            "SELECT CAST(value AS INTEGER) FROM vault_meta WHERE key = 'counter'",
                            [],
                            |row| row.get(0),
                        )
                        .map_err(IndexError::from)?;
                    conn.execute(
                        "UPDATE vault_meta SET value = ?1 WHERE key = 'counter'",
                        [(current + 1).to_string()],
                    )
                    .map_err(IndexError::from)?;
                    Ok(())
                })
                .await
        }));
    }

    for handle in handles {
        handle
            .await
            .expect("write task did not panic")
            .expect("write serialized without SQLITE_BUSY");
    }

    // Read the final total back through the pool.
    let pool = IndexPool::new(&path).expect("build reader pool");
    let total: i64 = pool
        .interact(|conn| {
            conn.query_row(
                "SELECT CAST(value AS INTEGER) FROM vault_meta WHERE key = 'counter'",
                [],
                |row| row.get(0),
            )
            .map_err(IndexError::from)
        })
        .await
        .expect("read counter");

    assert_eq!(
        total, 32,
        "all 32 increments applied exactly once — no lost updates"
    );
}

/// Reads interleaved with writes never deadlock, and a read after a write sees
/// the committed state (WAL readers + a single writer compose).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reads_interleaved_with_writes_observe_committed_state() {
    let (_dir, path) = temp_db();

    let writer = Arc::new(IndexWriter::spawn(&path).expect("spawn writer"));
    let pool = IndexPool::new(&path).expect("build reader pool");

    // Read-after-write: a value written through the writer is visible to a
    // subsequent pooled read.
    writer
        .execute(|conn| write_meta(conn, "observed", "v0"))
        .await
        .expect("write v0");
    let seen = pool
        .interact(|conn| read_meta(conn, "observed"))
        .await
        .expect("read observed");
    assert_eq!(seen, "v0", "a pooled read observes the committed write");

    // Interleave 8 writes (distinct keys) with 8 reads (trivial SELECTs); assert
    // no deadlock — every task returns Ok.
    let mut handles = Vec::new();
    for i in 0..8 {
        let writer = Arc::clone(&writer);
        handles.push(tokio::spawn(async move {
            writer
                .execute(move |conn| write_meta(conn, &format!("k{i}"), "x"))
                .await
        }));
    }
    let mut read_handles = Vec::new();
    for _ in 0..8 {
        let pool = pool.clone();
        read_handles.push(tokio::spawn(async move {
            pool.interact(|conn| {
                conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                    .map_err(IndexError::from)
            })
            .await
        }));
    }
    for handle in handles {
        handle
            .await
            .expect("write task did not panic")
            .expect("interleaved write served");
    }
    for handle in read_handles {
        handle
            .await
            .expect("read task did not panic")
            .expect("interleaved read served");
    }

    // Read-after-write again, at the end of the interleaving.
    writer
        .execute(|conn| write_meta(conn, "observed", "v1"))
        .await
        .expect("write v1");
    let seen = pool
        .interact(|conn| read_meta(conn, "observed"))
        .await
        .expect("read observed again");
    assert_eq!(seen, "v1", "the latest committed write is observed");
}

/// The `busy_timeout` customizer actually ran: a pooled connection reports the
/// configured value (5000 ms), not SQLite's fail-immediately default of 0. This
/// behaviourally verifies the deferred-work resolution rather than asserting it
/// in a comment.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pooled_connections_carry_the_configured_busy_timeout() {
    let (_dir, path) = temp_db();

    // Migrate so the pool has a valid schema to open against.
    let writer = IndexWriter::spawn(&path).expect("spawn writer");
    let pool = IndexPool::new(&path).expect("build reader pool");

    let timeout_ms: i64 = pool
        .interact(|conn| {
            conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .map_err(IndexError::from)
        })
        .await
        .expect("read busy_timeout");

    assert_eq!(
        timeout_ms, 5000,
        "pooled connections carry the 5s busy_timeout, not the default 0"
    );

    // The writer's own connection carries it too (both connection sites share
    // the one BUSY_TIMEOUT const).
    let writer_timeout_ms: i64 = {
        let (tx, rx) = std::sync::mpsc::channel();
        writer
            .execute(move |conn| {
                let v: i64 = conn
                    .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                    .map_err(IndexError::from)?;
                let _ = tx.send(v);
                Ok(())
            })
            .await
            .expect("read writer busy_timeout");
        rx.recv().expect("writer reported its busy_timeout")
    };
    assert_eq!(
        writer_timeout_ms, 5000,
        "the writer connection carries the same 5s busy_timeout"
    );

    writer.shutdown().await;
}
