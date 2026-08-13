//! LD-14 reader connection pool + LD-16 blocking-safe execution.
//!
//! A [`deadpool`]-managed pool of [`DEFAULT_READERS`] reader connections, each
//! carrying the LD-4 locked PRAGMAs (via [`crate::open`], reused verbatim) and a
//! [`BUSY_TIMEOUT`]. Reads run through [`IndexPool::interact`], which hands the
//! caller a pooled connection inside `tokio::task::spawn_blocking` so blocking
//! rusqlite never runs on an async worker thread (LD-16).
//!
//! This is the **generic** `deadpool` crate plus a local [`ConnectionManager`],
//! **not** `deadpool-sqlite`: `deadpool-sqlite 0.13` pins `rusqlite ^0.38` while
//! the workspace is on `rusqlite 0.40`, so taking it would duplicate `rusqlite`
//! and trip the LD-37 supply-chain invariant (see the root `Cargo.toml` and the
//! Story 3.5 record). The local manager over our own rusqlite-0.40 `Connection`
//! preserves "pool via deadpool, default size 4" per LD-14's intent.
//!
//! The pool ships **no** domain queries — the agenda/search/backlink `SELECT`s
//! (Stories 7.1/8.x) are a separate concern. It also assumes the schema already
//! exists: on a fresh database path the [`crate::IndexWriter`] must be spawned
//! first (it migrates once — LD-12; Story 3.4) before any read is valid.

use std::path::{Path, PathBuf};
use std::time::Duration;

use deadpool::managed::{self, Metrics, Pool, RecycleError, RecycleResult};
use deadpool::Runtime;
use rusqlite::Connection;
use tokio::task::spawn_blocking;

use crate::connection::open;
use crate::error::IndexError;

/// The reader pool size — LD-14's "default size 4".
///
/// The NFR-3 (agenda <100ms) and NFR-4 (search <200ms) budgets are sized
/// against several independent read queries running concurrently rather than
/// serializing behind one connection. `deadpool` **queues** `get()` calls
/// beyond this bound, so the number caps concurrency without capping how many
/// reads can be outstanding.
const DEFAULT_READERS: usize = 4;

/// Busy-handler timeout applied to every connection this story creates — the
/// pooled readers here and the one writer in [`crate::writer`].
///
/// SQLite's default is `0`: a connection that finds the database locked fails
/// **immediately** with `SQLITE_BUSY` instead of waiting. That is exactly the
/// defect the deferred-work rows (Stories 3.3/3.4) assigned to Story 3.5 — and
/// Story 3.5 is where the contention that justifies a timeout first exists: a
/// 4-reader pool plus a dedicated writer whose WAL checkpoint or write
/// transaction routinely overlaps a read.
///
/// Five seconds is long enough to ride out a WAL checkpoint on a notes-scale
/// index (milliseconds) and realistic write contention, yet short enough that a
/// genuine deadlock surfaces as an error rather than hanging forever. It is
/// `pub(crate)` so the writer applies the identical value — one const, both
/// connection sites.
pub(crate) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// `deadpool` manager that mints reader connections for [`IndexPool`].
///
/// [`ConnectionManager::create`] reuses [`crate::open`] verbatim (the LD-4
/// locked PRAGMA set — **not** reimplemented) and adds [`BUSY_TIMEOUT`]. It does
/// **not** migrate: readers open a schema that already exists, and the writer is
/// the sole migration site (LD-12; Story 3.4 Dev Note 5). Every rusqlite call is
/// blocking, so `create` runs its work under `spawn_blocking` (LD-16).
#[derive(Debug)]
pub struct ConnectionManager {
    db_path: PathBuf,
}

impl ConnectionManager {
    fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }
}

impl managed::Manager for ConnectionManager {
    type Type = Connection;
    type Error = IndexError;

    async fn create(&self) -> Result<Connection, IndexError> {
        let db_path = self.db_path.clone();
        // `open` (which opens the file and runs the locked PRAGMAs) and
        // `busy_timeout` are blocking rusqlite; keep pool warm-up off the async
        // worker threads (LD-16). `spawn_blocking`'s `JoinError` only fires if
        // the closure panics — `open`/`busy_timeout` return `Result` and do not
        // panic — so it is mapped to a pool error, never unwrapped.
        spawn_blocking(move || {
            let conn = open(&db_path)?;
            conn.busy_timeout(BUSY_TIMEOUT)?;
            Ok(conn)
        })
        .await
        .map_err(|e| IndexError::PoolBuild(format!("reader connection task failed: {e}")))?
    }

    async fn recycle(&self, conn: &mut Connection, _: &Metrics) -> RecycleResult<IndexError> {
        // Liveness probe: a poisoned/closed connection returns `Err`, so
        // `deadpool` discards it and re-`create`s a fresh one. `SELECT 1` is a
        // constant expression — it reads no page and touches no disk, so it is
        // the one rusqlite call exempt from the `spawn_blocking` rule (there is
        // nothing blocking to move off the worker thread, and `recycle` cannot
        // move the borrowed `&mut Connection` into a `'static` task anyway). No
        // PRAGMA re-application: `create` set them and they persist per
        // connection.
        conn.execute_batch("SELECT 1")
            .map_err(|e| RecycleError::Backend(IndexError::from(e)))?;
        Ok(())
    }
}

/// The LD-14 reader pool: [`DEFAULT_READERS`] connections, each carrying the
/// LD-4 PRAGMAs and a [`BUSY_TIMEOUT`], handed out through [`IndexPool::interact`].
///
/// Cheap to [`Clone`] (an `Arc` bump) so it can be shared across async tasks.
///
/// # Ordering: migrate before you read
///
/// The pool does **not** migrate — readers open a schema that already exists. On
/// a fresh database path a consumer MUST spawn the [`crate::IndexWriter`] first
/// (it opens, sets `busy_timeout`, and runs `migrate` once) before issuing any
/// read; a `SELECT` against an un-migrated database fails with "no such table".
/// The writer is the sole migration site (LD-12; Story 3.4 Dev Note 5).
#[derive(Clone)]
pub struct IndexPool {
    pool: Pool<ConnectionManager>,
}

impl IndexPool {
    /// Build a reader pool of [`DEFAULT_READERS`] connections against the index
    /// database at `db_path`.
    ///
    /// The pool is lazy: connections are created on first `get()`, so a bad path
    /// surfaces from [`IndexPool::interact`], not here. `Runtime::Tokio1` wires
    /// deadpool to the ambient Tokio runtime (required for `get()` queueing).
    ///
    /// # Errors
    ///
    /// [`IndexError::PoolBuild`] if `deadpool` rejects the builder configuration.
    pub fn new(db_path: &Path) -> Result<IndexPool, IndexError> {
        let pool = Pool::builder(ConnectionManager::new(db_path))
            .max_size(DEFAULT_READERS)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| IndexError::PoolBuild(e.to_string()))?;
        Ok(IndexPool { pool })
    }

    /// Run `f` against a pooled reader connection on a blocking thread.
    ///
    /// Acquires a connection (queuing if all [`DEFAULT_READERS`] are busy), then
    /// runs `f` inside `tokio::task::spawn_blocking` so the blocking rusqlite
    /// work never stalls an async worker thread (LD-16). The connection returns
    /// to the pool when `f` completes.
    ///
    /// This mirrors `deadpool-sqlite::interact` and is the **mechanism** the
    /// NFR-3/NFR-4 read queries will use; it bakes in **no** SQL of its own.
    ///
    /// # Errors
    ///
    /// [`IndexError::PoolAcquire`] if a connection cannot be obtained (pool
    /// closed, `create`/`recycle` failed, or the blocking task panicked);
    /// otherwise whatever `f` returns.
    pub async fn interact<F, R>(&self, f: F) -> Result<R, IndexError>
    where
        F: FnOnce(&Connection) -> Result<R, IndexError> + Send + 'static,
        R: Send + 'static,
    {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| IndexError::PoolAcquire(e.to_string()))?;
        // Move the pooled `Object` (which is `Send` because `Connection` is, and
        // `'static` because it owns its handle) into the blocking task. The RAII
        // guard returns the connection to the pool when the closure ends.
        spawn_blocking(move || f(&conn))
            .await
            .map_err(|e| IndexError::PoolAcquire(format!("reader task failed: {e}")))?
    }

    /// The pool's current status — `max_size`, live `size`, and `available`
    /// (idle, ready-to-hand-out) connection counts.
    ///
    /// Exposed so a consumer (and `tests/concurrency.rs`) can assert connections
    /// were returned rather than leaked after a burst of reads.
    pub fn status(&self) -> managed::Status {
        self.pool.status()
    }
}
