//! LD-7/LD-14 single dedicated writer task (LD-16 `spawn_blocking`, FR-17
//! derived index; the connection model NFR-3/NFR-4 depend on).
//!
//! [`IndexWriter`] owns the **one** writable, migrated connection and serializes
//! every write through it. Writes are submitted as [`IndexUpdate`]s over a
//! bounded `tokio::sync::mpsc` channel; the loop drains them from a dedicated
//! `spawn_blocking` thread with [`blocking_recv`](tokio::sync::mpsc::Receiver::blocking_recv),
//! so the blocking rusqlite calls never stall an async worker thread (LD-16).
//! Because a single connection applies every mutation in receive order, two
//! concurrent [`IndexWriter::execute`] calls can never race a write — this is
//! the Single Writer Rule (LD-7) at the SQLite layer.
//!
//! [`IndexWriter::spawn`] is also the **sole migration site** in the running
//! system (LD-12; Story 3.4 Dev Note 5): it opens, sets `busy_timeout`, and runs
//! `migrate` **once**, before the loop serves anything and before any reader
//! query is valid. Readers (the [`crate::pool`]) never migrate. A consumer must
//! therefore construct the writer before issuing reads on a fresh path.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{spawn_blocking, JoinHandle};

use crate::connection::open;
use crate::error::IndexError;
use crate::migrations::migrate;
use crate::pool::BUSY_TIMEOUT;

/// Bound on the write channel.
///
/// The channel is **bounded**, so [`IndexWriter::execute`]'s `send().await`
/// blocks once the queue is full — natural backpressure that stops a flood of
/// writes from growing memory without limit. Combined with `execute` awaiting
/// each per-write ack, this is ample for notes-scale, interactive writes. The
/// bound is **not** yet calibrated for a bulk re-index (Story 3.6's initial
/// scan, the first bulk writer) — see the deferred-work note.
const WRITER_CHANNEL_CAPACITY: usize = 256;

/// A boxed unit of write-work: it runs against the writer's `&mut Connection`
/// and reports success or a SQL failure.
type WriteThunk = Box<dyn FnOnce(&mut Connection) -> Result<(), IndexError> + Send>;

/// A unit of write-work in transit to the [`IndexWriter`]: a boxed thunk plus a
/// oneshot channel to return its result on.
///
/// This is the **transport**, not a catalogue of concrete index mutations
/// (Story 3.5 Dev Note 4): Story 3.6 sends its real sync SQL — the
/// `INSERT`/`UPDATE`/`DELETE` + FTS sync of the index engine — through this
/// unchanged. The fields are private; callers construct one only indirectly via
/// [`IndexWriter::execute`].
pub struct IndexUpdate {
    thunk: WriteThunk,
    ack: oneshot::Sender<Result<(), IndexError>>,
}

/// The single dedicated writer task (LD-7 Single Writer Rule at the index
/// layer, LD-14 connection management).
///
/// Holds the sending half of the write channel and the loop's `JoinHandle`.
/// Cheaply shared references drive concurrent [`IndexWriter::execute`] calls;
/// they all serialize through the one owned connection.
///
/// # Ordering: this is the sole migration site
///
/// [`IndexWriter::spawn`] migrates once at construction, before the loop serves
/// anything. On a fresh database path a consumer must spawn the writer **before**
/// issuing any read through [`crate::IndexPool`], or a `SELECT` fails with "no
/// such table" (LD-12; Story 3.4 Dev Note 5).
///
/// # Shutdown
///
/// Dropping the `IndexWriter` drops its `Sender`; the loop's `blocking_recv`
/// then returns `None`, it drops the connection, and the blocking thread exits.
/// [`IndexWriter::shutdown`] is the explicit form that also awaits the drain.
pub struct IndexWriter {
    tx: mpsc::Sender<IndexUpdate>,
    handle: JoinHandle<()>,
}

impl IndexWriter {
    /// Open the one writable connection, apply the LD-4 PRAGMAs + `busy_timeout`,
    /// run `migrate` once, then spawn the dedicated writer loop.
    ///
    /// The open/`busy_timeout`/`migrate` sequence runs **inline** here — a
    /// one-time, startup-only blocking cost — so that a migration failure
    /// surfaces synchronously as this function's error rather than from a later
    /// `execute`. The ongoing loop, by contrast, runs on `spawn_blocking`
    /// (LD-16). Must be called from within a Tokio runtime (it uses
    /// `spawn_blocking`).
    ///
    /// # Errors
    ///
    /// [`IndexError::WriterUnavailable`] if called outside a Tokio runtime (the
    /// documented precondition, surfaced as an error rather than a panic from
    /// `spawn_blocking`); [`IndexError::Sqlite`] if the connection cannot be
    /// opened or `busy_timeout` fails; [`IndexError::Pragma`] if a locked PRAGMA
    /// does not take; [`IndexError::Migration`] if migration fails.
    pub fn spawn(db_path: &Path) -> Result<IndexWriter, IndexError> {
        // `spawn_blocking` panics without an ambient Tokio runtime. This function
        // returns `Result`, so surface the missing-runtime precondition as an
        // error instead of letting the panic escape a fallible-looking call.
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(IndexError::WriterUnavailable(
                "IndexWriter::spawn must be called from within a Tokio runtime".to_owned(),
            ));
        }

        let mut conn = open(db_path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        migrate(&mut conn)?;

        let (tx, rx) = mpsc::channel::<IndexUpdate>(WRITER_CHANNEL_CAPACITY);
        let db_path = db_path.to_path_buf();
        // The loop OWNS `conn` and runs on Tokio's blocking-thread pool: every
        // rusqlite call blocks, and `blocking_recv` blocks between messages, so
        // it must stay off the async worker threads (LD-16). A plain async task
        // calling blocking rusqlite would stall a worker thread. `db_path` rides
        // along so the loop can rebuild the connection after a panicked thunk.
        let handle = spawn_blocking(move || writer_loop(conn, rx, db_path));
        Ok(IndexWriter { tx, handle })
    }

    /// Serialize one write through the dedicated connection.
    ///
    /// Boxes `f`, sends it down the channel, and awaits its result. Every
    /// mutation routes through the one connection in receive order, so two
    /// concurrent `execute` calls can never race a write (LD-7/LD-14).
    ///
    /// # Errors
    ///
    /// [`IndexError::WriterUnavailable`] if the writer task is gone (the send
    /// failed) or dropped the acknowledgement before replying — distinct from a
    /// SQL error, so a caller can tell "writer gone" from "the write ran and
    /// failed". Otherwise whatever `f` returned.
    pub async fn execute<F>(&self, f: F) -> Result<(), IndexError>
    where
        F: FnOnce(&mut Connection) -> Result<(), IndexError> + Send + 'static,
    {
        let (ack_tx, ack_rx) = oneshot::channel();
        let update = IndexUpdate {
            thunk: Box::new(f),
            ack: ack_tx,
        };
        self.tx.send(update).await.map_err(|_| {
            IndexError::WriterUnavailable(
                "the writer task has shut down (channel closed)".to_owned(),
            )
        })?;
        match ack_rx.await {
            Ok(result) => result,
            Err(_) => Err(IndexError::WriterUnavailable(
                "the writer dropped the acknowledgement before replying".to_owned(),
            )),
        }
    }

    /// Close the channel and wait for the writer to drain and drop its
    /// connection.
    ///
    /// The explicit counterpart to dropping the `IndexWriter`: it consumes
    /// `self`, drops the `Sender` (so the loop's `blocking_recv` returns `None`),
    /// and awaits the loop's completion. Awaiting cannot panic — the loop body
    /// never panics — so the `JoinHandle` result is discarded.
    pub async fn shutdown(self) {
        let IndexWriter { tx, handle } = self;
        drop(tx);
        let _ = handle.await;
    }
}

/// Drain the write channel from a blocking thread, applying each unit of work to
/// the owned connection in order.
///
/// `blocking_recv` returns `None` once every `Sender` has dropped (graceful
/// shutdown), at which point the loop drops `conn` and returns. A caller that
/// dropped its `execute` future leaves a closed ack channel; the failed
/// `ack.send` is ignored — the write already ran or failed, so the loop simply
/// moves to the next unit of work.
///
/// A thunk that *panics* (rather than returning `Err`) must not tear down the
/// sole writer — otherwise one bad write would fail every future write for the
/// process lifetime. The panic is caught, reported on the ack as a distinct
/// [`IndexError::WriterUnavailable`], and the connection is **rebuilt**: a panic
/// mid-transaction can leave `conn` with an open transaction, so it is discarded
/// rather than reused. If the rebuild fails (the database is unreachable) the
/// writer cannot recover and the loop exits; pending senders then observe the
/// closed channel.
fn writer_loop(mut conn: Connection, mut rx: mpsc::Receiver<IndexUpdate>, db_path: PathBuf) {
    while let Some(update) = rx.blocking_recv() {
        let IndexUpdate { thunk, ack } = update;
        match catch_unwind(AssertUnwindSafe(|| thunk(&mut conn))) {
            Ok(result) => {
                let _ = ack.send(result);
            }
            Err(_) => {
                let _ = ack.send(Err(IndexError::WriterUnavailable(
                    "a write thunk panicked; the write did not commit".to_owned(),
                )));
                match reopen(&db_path) {
                    Ok(fresh) => conn = fresh,
                    Err(_) => break,
                }
            }
        }
    }
}

/// Reopen the one writable connection after a panicked write poisoned the old
/// one, applying the LD-4 PRAGMAs + [`BUSY_TIMEOUT`] but **not** `migrate` — the
/// schema already exists (the writer migrated once at [`IndexWriter::spawn`]).
fn reopen(db_path: &Path) -> Result<Connection, IndexError> {
    let conn = open(db_path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}
