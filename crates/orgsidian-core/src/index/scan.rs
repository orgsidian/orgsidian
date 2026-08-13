//! The initial-scan orchestrator (FR-15/FR-17, LD-42 checkpoints, LD-16
//! `spawn_blocking`, LD-41 quarantine).
//!
//! [`scan_vault`] walks a designated vault, reads + `parser::analyze`s each
//! `.org` file off the async worker threads, maps `Document` → the index's
//! `FileIndexInput`, and submits **one writer transaction per 100-file
//! checkpoint** (LD-42's coupled commit + progress). It is incremental (an
//! unchanged `(mtime_ns, size_bytes)` is skipped, never re-parsed), cancellable
//! at checkpoint boundaries (with the committed prefix retained), and reports
//! progress through a caller-supplied callback so `orgsidian-core` needs no
//! `tauri` dependency (the shell maps [`ScanProgress`] to its specta event).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use orgsidian_index::{file_is_unchanged, SyncOp};

use super::{index_err, map, vault_err, IndexHandle};
use crate::error::OrgError;

/// LD-42 checkpoint interval: commit + emit progress every this many files.
///
/// LD-42 allows this to be configurable via `global.toml`; Story 3.6 ships the
/// constant and defers the config hook (`GlobalSettings` is a frozen consume-
/// only surface here — see the deferred-work note). Batching a whole checkpoint
/// into ONE writer message is also what makes the `WRITER_CHANNEL_CAPACITY=256`
/// bound a non-issue: a 1000-file scan sends ~10 messages, not 1000.
const CHECKPOINT_INTERVAL: usize = 100;

/// A progress snapshot emitted at each checkpoint: `current` of `total` files
/// processed, with `errors` quarantined so far. A plain core struct (no
/// `tauri`/`specta`); the shell constructs its `IndexProgress` event from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanProgress {
    /// Files processed so far (upserted + skipped + quarantined).
    pub current: u32,
    /// Total `.org` files discovered in the vault.
    pub total: u32,
    /// Files quarantined so far (parse/read/non-UTF-8 failures — LD-41).
    pub errors: u32,
}

/// The result of a completed or cancelled scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanOutcome {
    /// Files upserted (successfully indexed) this run.
    pub indexed: u32,
    /// Files skipped as unchanged (incremental fast path).
    pub skipped: u32,
    /// Files quarantined (LD-41).
    pub errors: u32,
    /// `true` if the scan returned early on the cancel flag (committed prefix
    /// retained — LD-42 "resume from last checkpoint, never from zero").
    pub cancelled: bool,
}

/// Scan `index`'s vault: walk → (skip unchanged | read+parse+map) →
/// batch-submit per checkpoint. Emits `progress` at each checkpoint; checks
/// `cancel` at checkpoint boundaries.
///
/// # Errors
///
/// [`OrgError::Vault`] if discovery fails; [`OrgError::Index`] if a writer/
/// reader operation fails; [`OrgError::Io`] if a scan task panics (never in
/// practice — the parser is panic-free, LD-41).
pub async fn scan_vault(
    index: &IndexHandle,
    cancel: &AtomicBool,
    mut progress: impl FnMut(ScanProgress),
) -> Result<ScanOutcome, OrgError> {
    let root = index.vault_root();
    let files = orgsidian_vault::scan_org_files(root).map_err(vault_err)?;
    let total = files.len() as u32;

    let mut current = 0u32;
    let mut indexed = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;
    let mut batch: Vec<SyncOp> = Vec::new();
    let mut since_checkpoint = 0usize;

    // Emit the total immediately so the UI can render "0 of M" before any file
    // is touched.
    progress(ScanProgress {
        current,
        total,
        errors,
    });

    if cancel.load(Ordering::Acquire) {
        return Ok(ScanOutcome {
            indexed,
            skipped,
            errors,
            cancelled: true,
        });
    }

    for file in &files {
        current += 1;
        since_checkpoint += 1;

        let Some(rel_path) = orgsidian_vault::to_rel_path(root, file) else {
            // Non-UTF-8 filename: unrepresentable in `files.path TEXT`. Skip and
            // count (never lossily rename) — resolves the non-UTF-8 deferred
            // row. No `files` row exists to quarantine against without a path.
            errors += 1;
            tracing::warn!(path = %file.display(), "skipping .org file with non-UTF-8 name");
            maybe_checkpoint(
                index,
                cancel,
                &mut progress,
                &mut batch,
                &mut since_checkpoint,
                ScanProgress {
                    current,
                    total,
                    errors,
                },
            )
            .await?;
            if cancelled(cancel) {
                return Ok(cancelled_outcome(indexed, skipped, errors));
            }
            continue;
        };

        let (mtime_ns, size_bytes) = match file_stat(file) {
            Ok(stat) => stat,
            Err(err) => {
                batch.push(SyncOp::Quarantine {
                    rel_path,
                    mtime_ns: 0,
                    size_bytes: 0,
                    reason: format!("metadata read failed: {err}"),
                });
                errors += 1;
                maybe_checkpoint(
                    index,
                    cancel,
                    &mut progress,
                    &mut batch,
                    &mut since_checkpoint,
                    ScanProgress {
                        current,
                        total,
                        errors,
                    },
                )
                .await?;
                if cancelled(cancel) {
                    return Ok(cancelled_outcome(indexed, skipped, errors));
                }
                continue;
            }
        };

        // Incremental skip: an unchanged file is neither re-read, re-parsed, nor
        // re-written. Read through the pool (a single indexed lookup).
        let probe_path = rel_path.clone();
        let unchanged = index
            .pool()
            .interact(move |conn| file_is_unchanged(conn, &probe_path, mtime_ns, size_bytes))
            .await
            .map_err(index_err)?;
        if unchanged {
            skipped += 1;
            maybe_checkpoint(
                index,
                cancel,
                &mut progress,
                &mut batch,
                &mut since_checkpoint,
                ScanProgress {
                    current,
                    total,
                    errors,
                },
            )
            .await?;
            if cancelled(cancel) {
                return Ok(cancelled_outcome(indexed, skipped, errors));
            }
            continue;
        }

        // Read + parse off the async worker threads (LD-16): a big org file
        // must never stall a worker. The parser is lenient (LD-41), so an `Err`
        // is a genuine defensive failure → quarantine.
        let file_owned = file.clone();
        let outcome = tokio::task::spawn_blocking(move || read_and_parse(&file_owned))
            .await
            .map_err(|err| OrgError::Io {
                reason: format!("scan task panicked: {err}"),
            })?;

        match outcome {
            ReadParse::Parsed(document) => {
                batch.push(SyncOp::Upsert(map::document_to_input(
                    rel_path, mtime_ns, size_bytes, &document,
                )));
                indexed += 1;
            }
            ReadParse::Failed(reason) => {
                batch.push(SyncOp::Quarantine {
                    rel_path,
                    mtime_ns,
                    size_bytes,
                    reason,
                });
                errors += 1;
            }
        }

        maybe_checkpoint(
            index,
            cancel,
            &mut progress,
            &mut batch,
            &mut since_checkpoint,
            ScanProgress {
                current,
                total,
                errors,
            },
        )
        .await?;
        if cancelled(cancel) {
            return Ok(cancelled_outcome(indexed, skipped, errors));
        }
    }

    // Final flush of the trailing partial checkpoint.
    submit_batch(index, std::mem::take(&mut batch)).await?;
    progress(ScanProgress {
        current,
        total,
        errors,
    });

    Ok(ScanOutcome {
        indexed,
        skipped,
        errors,
        cancelled: false,
    })
}

/// Commit the accumulated batch + emit progress when a checkpoint boundary is
/// reached (`since_checkpoint >= CHECKPOINT_INTERVAL`); a no-op otherwise. The
/// cancel decision is made by the caller AFTER this returns, so the in-flight
/// batch is always committed before an early return (LD-42).
async fn maybe_checkpoint(
    index: &IndexHandle,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(ScanProgress),
    batch: &mut Vec<SyncOp>,
    since_checkpoint: &mut usize,
    snapshot: ScanProgress,
) -> Result<(), OrgError> {
    // Flush also when cancellation is requested, so the in-flight batch is
    // committed (retained partial) rather than dropped.
    if *since_checkpoint < CHECKPOINT_INTERVAL && !cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    submit_batch(index, std::mem::take(batch)).await?;
    *since_checkpoint = 0;
    progress(snapshot);
    Ok(())
}

/// Submit one batch as a single writer transaction (LD-42's one-commit-per-
/// checkpoint). A no-op for an empty batch (skips-only checkpoints).
async fn submit_batch(index: &IndexHandle, batch: Vec<SyncOp>) -> Result<(), OrgError> {
    if batch.is_empty() {
        return Ok(());
    }
    index
        .writer()
        .execute(move |conn| {
            let tx = conn.transaction()?;
            for op in &batch {
                op.apply(&tx)?;
            }
            tx.commit().map_err(Into::into)
        })
        .await
        .map_err(index_err)
}

/// Whether cancellation has been requested.
fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Acquire)
}

/// Build a cancelled [`ScanOutcome`] from the running counters.
fn cancelled_outcome(indexed: u32, skipped: u32, errors: u32) -> ScanOutcome {
    ScanOutcome {
        indexed,
        skipped,
        errors,
        cancelled: true,
    }
}

/// The read + parse outcome for one file.
enum ReadParse {
    /// Successfully read and analyzed.
    Parsed(crate::parser::semantic::Document),
    /// Read or parse failed (LD-41 quarantine reason).
    Failed(String),
}

/// Read `path` and analyze it. Runs on a blocking thread (LD-16). Read failures
/// and the parser's defensive errors both become a quarantine reason.
fn read_and_parse(path: &Path) -> ReadParse {
    match std::fs::read_to_string(path) {
        Ok(content) => match crate::parser::analyze(&content) {
            Ok(document) => ReadParse::Parsed(document),
            Err(err) => ReadParse::Failed(format!("parse error: {err}")),
        },
        Err(err) => ReadParse::Failed(format!("read error: {err}")),
    }
}

/// Filesystem `(mtime_ns, size_bytes)` for the incremental-skip key. A missing
/// or pre-epoch mtime maps to `0` (the file is then never "unchanged", so it is
/// always re-indexed — the safe direction).
fn file_stat(path: &Path) -> std::io::Result<(i64, i64)> {
    let metadata = std::fs::metadata(path)?;
    let size_bytes = metadata.len() as i64;
    let mtime_ns = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|delta| delta.as_nanos() as i64)
        .unwrap_or(0);
    Ok((mtime_ns, size_bytes))
}
