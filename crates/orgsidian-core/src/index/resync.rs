//! Single-file incremental index re-sync (FR-16 / FR-17): the
//! `orgsidian-index::sync` "incremental" path the Epic 5 watcher drives per
//! external write.
//!
//! Where [`super::scan::scan_vault`] walks the whole vault, this re-syncs ONE
//! file — the shape the LD-42 per-file transactional entry points
//! (`upsert_file`/`delete_file`/`quarantine_file`) were built for ("the shape
//! Epic 5's watcher calls per changed file", `orgsidian_index::sync`). It reuses
//! the scan's read→parse→map mechanism verbatim (`read_and_parse`/`file_stat`/
//! `map::document_to_input`), so an external write and an initial scan produce
//! identical index rows. There is no incremental-skip check here: an external
//! write is, by definition, a change.

use std::path::Path;

use orgsidian_index::{delete_file, quarantine_file, upsert_file};

use super::scan::{file_stat, read_and_parse, ReadParse};
use super::{index_err, map, IndexHandle};
use crate::error::OrgError;

/// What a single-file re-sync did to the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncOutcome {
    /// The file was (re-)indexed with its current headlines.
    Upserted,
    /// The file could not be read/parsed → recorded as `quarantined=1` with no
    /// headlines (LD-41), same as the scan.
    Quarantined,
    /// The file no longer exists on disk → its index rows were removed.
    Deleted,
}

/// Incrementally re-sync the index for the single file at `abs_path` (an
/// absolute path under `index`'s canonical vault root). Submits one per-file
/// writer transaction (LD-42): an `upsert` on success, a `quarantine` on a
/// read/parse failure (LD-41), or a `delete` when the file is gone.
///
/// # Errors
///
/// [`OrgError::Io`] if `abs_path` is not representable as a vault-relative UTF-8
/// path, if its metadata cannot be read, or if the blocking read+parse task
/// panics; [`OrgError::Index`] if the writer operation fails.
pub async fn resync_file(index: &IndexHandle, abs_path: &Path) -> Result<ResyncOutcome, OrgError> {
    let root = index.vault_root();
    let Some(rel_path) = orgsidian_vault::to_rel_path(root, abs_path) else {
        return Err(OrgError::Io {
            reason: format!(
                "cannot re-sync {}: not a UTF-8 path under the vault root {}",
                abs_path.display(),
                root.display()
            ),
        });
    };

    // File removed on disk → drop its index rows (the LD-13 / Epic 5 delete
    // branch the scan never exercises, since a one-shot walk simply stops
    // upserting a vanished file).
    if !abs_path.exists() {
        let rel = rel_path.clone();
        index
            .writer()
            .execute(move |conn| delete_file(conn, &rel))
            .await
            .map_err(index_err)?;
        return Ok(ResyncOutcome::Deleted);
    }

    let (mtime_opt, size_bytes) = match file_stat(abs_path) {
        Ok(stat) => stat,
        // The file vanished in the window between the `exists()` check and this
        // metadata read (TOCTOU) → treat it as a delete rather than an error.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let rel = rel_path.clone();
            index
                .writer()
                .execute(move |conn| delete_file(conn, &rel))
                .await
                .map_err(index_err)?;
            return Ok(ResyncOutcome::Deleted);
        }
        Err(err) => {
            return Err(OrgError::Io {
                reason: format!("metadata read failed for {}: {err}", abs_path.display()),
            })
        }
    };
    // A file with no usable mtime is stored with `0` (never matched against a
    // real incremental probe) — the same fallback the scan uses.
    let mtime_ns = mtime_opt.unwrap_or(0);

    // Read + parse off the async worker threads (LD-16): a big org file must
    // never stall a worker. The parser is lenient (LD-41), so an `Err` is a
    // genuine defensive failure → quarantine.
    let file_owned = abs_path.to_path_buf();
    let outcome = tokio::task::spawn_blocking(move || read_and_parse(&file_owned))
        .await
        .map_err(|err| OrgError::Io {
            reason: format!("re-sync task panicked: {err}"),
        })?;

    match outcome {
        ReadParse::Parsed(document) => {
            let input = map::document_to_input(rel_path, mtime_ns, size_bytes, &document);
            index
                .writer()
                .execute(move |conn| upsert_file(conn, &input))
                .await
                .map_err(index_err)?;
            Ok(ResyncOutcome::Upserted)
        }
        ReadParse::Failed(reason) => {
            let rel = rel_path.clone();
            index
                .writer()
                .execute(move |conn| quarantine_file(conn, &rel, mtime_ns, size_bytes, &reason))
                .await
                .map_err(index_err)?;
            Ok(ResyncOutcome::Quarantined)
        }
    }
}
