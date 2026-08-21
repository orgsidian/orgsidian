//! `VaultError` — path-contextualized failure type for the vault subsystem
//! (LD-8 atomic writes + NFR-15 corruption safety).
//!
//! Mirrors the `SettingsError` precedent (`orgsidian-core/src/settings/error.rs`):
//! variants carry the offending path plus the underlying `io::Error` as
//! `#[source]`, so callers and logs can localize failures without `ErrorKind`
//! guesswork. Closes the Story-1.9 deferred item "bare `io::Error` without
//! context" (deferred-work.md §story-1.9).

use std::io;
use std::path::PathBuf;

// Epic 3 grows this crate (dirty buffer, SQLite mirror, vault open): new
// variants must not be breaking changes for downstream exhaustive matches.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VaultError {
    /// Non-transient filesystem I/O failure (open, write, commit, scan,
    /// remove). Surfaced immediately — never retried (LD-41 disk-full row:
    /// "surface error to user; never propagate partial-write corruption").
    #[error("vault I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A transient AV/Search-indexer lock outlived the bounded retry budget
    /// (max 3 attempts, exponential backoff from 100ms — architecture "Error
    /// recovery"). `attempts` lets the caller/log distinguish "failed once,
    /// non-transient" from "lock persisted across the whole schedule".
    #[error("atomic write to {path} failed after {attempts} attempts (transient lock outlived retry budget): {source}")]
    RetriesExhausted {
        path: PathBuf,
        attempts: u32,
        #[source]
        source: io::Error,
    },

    /// Story 5.5 (FR-16 / NFR-16 Single Writer Rule): a save was refused because
    /// an external write landed on this file while its buffer held unsaved edits,
    /// and the resulting conflict has not yet been resolved (the v0.1
    /// `BlockWithWarning` fallback — the full Merge Dialog is Epic 9). Unlike the
    /// other variants this is NOT a filesystem failure: nothing was written and
    /// there is no underlying `io::Error` — the write was deliberately blocked to
    /// protect unsaved work. The user clears it by discarding the external
    /// changes (or, in Epic 9, merging), after which the save proceeds.
    #[error("external write conflict on {path}: save blocked until the conflict is resolved")]
    ExternalConflict { path: PathBuf },
}

impl VaultError {
    /// Recover the underlying `io::Error`, discarding the vault context.
    ///
    /// Escape hatch for callers whose own error types carry a plain
    /// `io::Error` source (e.g. `SettingsError::Io` in `orgsidian-core`).
    pub fn into_io(self) -> io::Error {
        match self {
            VaultError::Io { source, .. } => source,
            VaultError::RetriesExhausted { source, .. } => source,
            // No underlying `io::Error` — the write was deliberately blocked, not
            // attempted. Synthesize one carrying the same message so the escape
            // hatch stays total for callers whose error type is `io::Error`.
            other @ VaultError::ExternalConflict { .. } => io::Error::other(other.to_string()),
        }
    }
}
