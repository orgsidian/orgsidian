//! Vault designation: root canonicalization, `.org` discovery, and the
//! vault-relative path form (FR-15 — architecture.md:1058; LD-40, LD-17).
//!
//! This is the FR-15 home the schema comment assigns path-meaning to
//! (migrations/0001_initial-schema.sql:63-69): Story 3.6 designates the vault
//! root, so it owns what a path MEANS in a vault. The decisions made here,
//! resolving the path-identity cluster of deferred rows:
//!
//! - **Canonicalize the root once** ([`canonicalize_vault_root`]) — the single
//!   normalization site the path-identity deferred rows assign to vault-open.
//! - **Store vault-relative, `/`-normalized paths** ([`to_rel_path`]) — the
//!   index, the dirty-buffer registry, and (Epic 5) the watcher all key on the
//!   same canonical form.
//! - **Skip non-UTF-8 filenames** — `files.path` is `TEXT` (`&str`), so a
//!   non-UTF-8 name (legal on Linux/macOS) returns `None` here and the caller
//!   skips-and-counts it (quarantine), never lossily renames it.
//!
//! Case-folding on case-insensitive volumes is deliberately NOT applied (the
//! on-disk case is stored); canonicalizing the root covers the common case and
//! the residual two-spellings collision stays a downgraded deferred row.

use std::io;
use std::path::{Component, Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use crate::atomic::clean_orphan_temp_files;
use crate::error::VaultError;

/// Resolve the designated vault root to its canonical absolute form
/// (`std::fs::canonicalize`: symlinks resolved, `.`/`..` removed). The single
/// place a vault root is normalized — every path handed to the index is
/// relative to THIS result, so two spellings of the same folder collapse here.
///
/// # Errors
///
/// [`VaultError::Io`] if the path does not exist or cannot be resolved.
pub fn canonicalize_vault_root(path: &Path) -> Result<PathBuf, VaultError> {
    std::fs::canonicalize(path).map_err(|source| VaultError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Open a vault root for indexing: canonicalize it, then run the one-shot
/// orphan-temp-file sweep (Story 3.1's [`clean_orphan_temp_files`], wired here
/// per the `lib.rs` integration note). Returns the canonical root the caller
/// hands to the index and the scan.
///
/// Cleanup is **hygiene, not a gate**: a cleanup failure is logged
/// (`tracing::warn`) and swallowed — a vault with an unreadable subdirectory
/// still opens. Only the canonicalization can fail this function.
///
/// # Errors
///
/// [`VaultError::Io`] if the root cannot be canonicalized.
pub fn open_vault_root(path: &Path) -> Result<PathBuf, VaultError> {
    let canonical = canonicalize_vault_root(path)?;
    match clean_orphan_temp_files(&canonical) {
        Ok(report) if report.removed_count() > 0 => tracing::info!(
            vault = %canonical.display(),
            removed = report.removed_count(),
            "cleaned orphaned atomic-write temp files at vault open"
        ),
        Ok(_) => {}
        Err(err) => tracing::warn!(
            vault = %canonical.display(),
            error = %err,
            "orphan-temp cleanup failed at vault open (non-fatal, continuing)"
        ),
    }
    Ok(canonical)
}

/// Recursively discover `.org` files under a (canonical) vault root, in a
/// deterministic sorted order so scan progress counts and tests are stable.
///
/// Skips the `.orgsidian/` config directory and every other dotfile directory
/// (and their subtrees); the root itself is never skipped even if its own name
/// begins with a dot. Extension matching is case-insensitive (`.org`/`.ORG`).
/// Non-UTF-8 filenames are kept in the returned list; the caller resolves them
/// through [`to_rel_path`], which returns `None` so they are skipped-and-counted
/// rather than silently indexed under a lossy name.
///
/// # Errors
///
/// [`VaultError::Io`] if a directory entry cannot be read (the walk is not
/// best-effort here — a scan that silently drops an unreadable subtree would
/// under-report the vault).
pub fn scan_org_files(root: &Path) -> Result<Vec<PathBuf>, VaultError> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(keep_entry) {
        let entry = entry.map_err(|err| walk_error(root, err))?;
        if entry.file_type().is_file() && has_org_extension(entry.path()) {
            files.push(entry.into_path());
        }
    }
    // WalkDir yields in filesystem order; sort for absolute determinism.
    files.sort();
    Ok(files)
}

/// The vault-relative, `/`-normalized string form of `file` under `root`
/// (which must be the canonical root `file` was discovered beneath).
///
/// Returns `None` when `file` is not under `root`, when the relative path
/// escapes it (`..`), or when any component is non-UTF-8 (unrepresentable in
/// `files.path TEXT`) — the caller skips-and-counts that file. Rebuilding the
/// string from [`Component::Normal`] parts joins with `/` on every platform, so
/// a Windows `sub\note.org` and a Unix `sub/note.org` yield the same
/// `sub/note.org` key.
pub fn to_rel_path(root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => parts.push(name.to_str()?),
            // A canonical file under a canonical root has only Normal
            // components; anything else means the input was not what this
            // contract expects, so refuse rather than guess.
            _ => return None,
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// Keep predicate for [`WalkDir::filter_entry`]: prune dotfile directories
/// (including `.orgsidian/`) and their subtrees, but never the walk root.
fn keep_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    // Non-UTF-8 dir names do not begin with an ASCII '.', so `map_or(false, …)`
    // keeps them (their `.org` children are then filtered by `to_rel_path`).
    !entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

/// Case-insensitive `.org` extension test.
fn has_org_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("org"))
}

/// Map a `walkdir` traversal error to a path-contextualized [`VaultError`].
fn walk_error(root: &Path, err: walkdir::Error) -> VaultError {
    let path = err
        .path()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf());
    let source = err
        .into_io_error()
        .unwrap_or_else(|| io::Error::other("directory walk error"));
    VaultError::Io { path, source }
}
