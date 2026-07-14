//! Atomic-write subsystem: temp-file-and-rename semantics with an AV-aware
//! bounded-backoff retry wrapper (LD-8 + NFR-15).
//!
//! The write path wraps the `atomic-write-file` crate (LD-8 mandates it —
//! `renameat` on Unix, `MoveFileExW` on Windows). Transient AV/Search-indexer
//! locks (the dominant Windows failure mode) are retried up to 3 attempts
//! total with exponential backoff from a 100ms base (architecture "Process
//! Patterns → Error recovery"); everything else — disk full, target directory
//! gone — surfaces immediately per the LD-41 disk-full row.
//!
//! Orphaned temp files from dead writers (`kill -9`, power loss) are collected
//! by [`clean_orphan_temp_files`]; Story 3.6 wires it into the Vault-open flow.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use atomic_write_file::AtomicWriteFile;

use crate::error::VaultError;

/// Retry budget: total attempts, including the first (architecture.md "Error
/// recovery": bounded exponential backoff, max 3 attempts).
const MAX_ATTEMPTS: u32 = 3;

/// Base backoff before the second attempt; doubles per retry (100ms, 200ms).
const BASE_BACKOFF: Duration = Duration::from_millis(100);

/// Minimum age before an orphaned temp file is eligible for cleanup. A
/// conservative guard so a concurrent in-flight writer is never raced: a live
/// `atomic_write` holds its temp for milliseconds, not a minute.
const ORPHAN_MIN_AGE: Duration = Duration::from_secs(60);

/// Narrow seam over "perform one full atomic write cycle" so the retry loop
/// is unit-testable with scripted error sequences (AC4). The production impl
/// is [`RealFileSystem`]; test fakes live in `tests/atomic.rs`.
///
/// Not public API — hidden test seam only.
#[doc(hidden)]
pub trait FileSystem {
    /// One open → write → commit cycle. No retry at this level.
    fn write_atomic_once(&self, path: &Path, content: &[u8]) -> io::Result<()>;
}

/// Production [`FileSystem`]: delegates to the `atomic-write-file` crate.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn write_atomic_once(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let mut file = AtomicWriteFile::open(path)?;
        if let Err(err) = write_body(&mut file, content) {
            // Deferred-item closure (Story 1.9 → 3.1): never rely on `Drop`
            // for temp cleanup — discard explicitly; the write error wins.
            if let Err(discard_err) = file.discard() {
                tracing::warn!(
                    path = %path.display(),
                    error = %discard_err,
                    "failed to discard temp file after write error"
                );
            }
            return Err(err);
        }
        // NOTE: `commit()` marks the file finalized before sync+rename, so a
        // commit failure leaves the temp behind (upstream behavior, verified
        // in atomic-write-file 0.3.0 `_commit`). `clean_orphan_temp_files`
        // is the recovery path for that residue.
        file.commit()
    }
}

/// The body of one write attempt between `open` and `commit`. Split out so
/// the LD-41 fail-point can inject post-open failures (disk-full row): the
/// temp file already exists when the failure fires, which makes the
/// discard-on-error path a real assertion target.
fn write_body(file: &mut AtomicWriteFile, content: &[u8]) -> io::Result<()> {
    #[cfg(feature = "failpoints")]
    fail::fail_point!("vault::atomic-write::write", |_| {
        Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "failpoint: injected ENOSPC during atomic write",
        ))
    });
    file.write_all(content)
}

/// Classify an `io::Error` as a transient AV/Search-indexer lock worth
/// retrying (LD-8). Precise, not generous (Dev Note 4):
///
/// - `ERROR_ACCESS_DENIED` maps to [`io::ErrorKind::PermissionDenied`] on
///   Windows; on Unix a `PermissionDenied` retry is harmless and keeps the
///   classifier platform-uniform.
/// - `ERROR_SHARING_VIOLATION` (32) / `ERROR_LOCK_VIOLATION` (33) surface
///   with an uncategorized `ErrorKind` on Windows, so match the raw OS code.
///   The raw check is deliberately NOT gated on the `ErrorKind` (AC3 letter
///   variance, recorded in Completion Notes): `ErrorKind::Uncategorized` is
///   unstable and its raw-code mapping can shift across Rust releases, so
///   kind-gating would silently break the Windows retry path. The ungated
///   check stays platform-uniform: on Unix those codes are EPIPE/EDOM, which
///   an atomic file write never produces.
///
/// Everything else — `NotFound` (target dir gone), directory-shaped targets,
/// ENOSPC — is non-transient and fails immediately.
fn is_transient_lock(err: &io::Error) -> bool {
    if err.kind() == io::ErrorKind::PermissionDenied {
        return true;
    }
    matches!(err.raw_os_error(), Some(32) | Some(33))
}

/// Retry loop generic over the [`FileSystem`] seam and an injectable sleeper
/// so tests assert the backoff schedule deterministically (no real sleeping).
///
/// Not public API — hidden test seam only; production callers use
/// [`atomic_write`].
#[doc(hidden)]
pub fn atomic_write_with(
    fs: &impl FileSystem,
    mut sleep: impl FnMut(Duration),
    path: &Path,
    content: &[u8],
) -> Result<(), VaultError> {
    let mut attempt: u32 = 1;
    loop {
        match fs.write_atomic_once(path, content) {
            Ok(()) => return Ok(()),
            Err(err) if is_transient_lock(&err) => {
                if attempt >= MAX_ATTEMPTS {
                    return Err(VaultError::RetriesExhausted {
                        path: path.to_path_buf(),
                        attempts: attempt,
                        source: err,
                    });
                }
                tracing::warn!(
                    path = %path.display(),
                    attempt,
                    error = %err,
                    "atomic write retry after transient lock"
                );
                sleep(BASE_BACKOFF * 2u32.pow(attempt - 1));
                attempt += 1;
            }
            Err(err) => {
                return Err(VaultError::Io {
                    path: path.to_path_buf(),
                    source: err,
                })
            }
        }
    }
}

/// Atomically write `content` to `path` with AV-aware retry (LD-8 + NFR-15).
///
/// Existing files are overwritten via temp-then-rename; a reader never
/// observes a partial write. Transient locks (see the module docs) are
/// retried up to 3 attempts with 100ms/200ms backoff; non-transient failures
/// return [`VaultError::Io`] immediately with the offending path attached.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), VaultError> {
    atomic_write_with(&RealFileSystem, std::thread::sleep, path, content)
}

/// Outcome of a [`clean_orphan_temp_files`] pass.
#[derive(Debug, Default)]
pub struct CleanupReport {
    /// Paths of the orphaned temp files that were removed, as resolved from
    /// the `vault_root` passed in (absolute iff `vault_root` is absolute).
    pub removed: Vec<PathBuf>,
    /// Per-entry failures the best-effort scan skipped over (unreadable
    /// subdirectory, undeletable orphan). Deletions recorded in `removed`
    /// happened even when this is non-empty.
    pub errors: Vec<(PathBuf, io::Error)>,
}

impl CleanupReport {
    /// Number of orphaned temp files removed by the scan.
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }
}

/// Recursively scan `vault_root` and remove orphaned `atomic-write-file` temp
/// siblings left behind by dead writers (`kill -9`, power loss — the crate
/// documents that aborts without unwinding leak temps).
///
/// A file is an orphan candidate only when ALL of:
///
/// - its name matches the crate's real temp pattern — `.` + `{name}.org` +
///   `.` + 6 ASCII alphanumerics (verified in atomic-write-file 0.3.0,
///   `src/imp/generic.rs::RandomName`);
/// - the `{name}.org` target it points at exists in the same directory — a
///   pattern-shaped name whose target is absent is more plausibly a user file
///   than crate residue, and deleting user data is the one unrecoverable
///   failure (the residue of a crashed first save to a brand-new target is
///   the accepted trade-off);
/// - its mtime is at least 60s old, so a concurrent in-flight writer is
///   never raced.
///
/// Anything else (user dotfiles, `.orgsidian/`, non-`.org` targets, fresh
/// temps) is never touched. Symlinks are never followed — including
/// symlink-to-directory, so orphans inside linked subtrees are out of scope
/// (cycle safety beats coverage there).
///
/// The scan is best-effort: per-entry failures are recorded in
/// [`CleanupReport::errors`] and the scan continues. The only hard error is
/// `vault_root` itself being unreadable.
///
/// Story 3.6 wires this into the Vault-open flow; this story ships the API.
pub fn clean_orphan_temp_files(vault_root: &Path) -> Result<CleanupReport, VaultError> {
    let mut report = CleanupReport::default();
    let now = SystemTime::now();

    let root_entries = std::fs::read_dir(vault_root).map_err(|e| VaultError::Io {
        path: vault_root.to_path_buf(),
        source: e,
    })?;

    // Iterative traversal (no recursion): a pathologically deep tree — e.g.
    // nesting loops manufactured by sync tools — must not overflow the stack.
    let mut pending: Vec<PathBuf> = Vec::new();
    scan_dir_entries(root_entries, vault_root, now, &mut pending, &mut report);
    while let Some(dir) = pending.pop() {
        match std::fs::read_dir(&dir) {
            Ok(entries) => scan_dir_entries(entries, &dir, now, &mut pending, &mut report),
            Err(e) => report.errors.push((dir, e)),
        }
    }
    Ok(report)
}

fn scan_dir_entries(
    entries: std::fs::ReadDir,
    dir: &Path,
    now: SystemTime,
    pending: &mut Vec<PathBuf>,
    report: &mut CleanupReport,
) {
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                report.errors.push((dir.to_path_buf(), e));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                report.errors.push((path, e));
                continue;
            }
        };

        if file_type.is_dir() {
            pending.push(path);
            continue;
        }
        // Symlinks (even to directories) are neither followed nor eligible.
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(target_name) = orphan_target_name(name) else {
            continue;
        };
        // Target-existence guard: without a `{stem}.org` sibling the name is
        // more plausibly a user file than crate residue — never delete it.
        if !dir.join(target_name).is_file() {
            continue;
        }
        // mtime-age guard: skip fresh temps (in-flight writers). A file whose
        // mtime cannot be read is skipped conservatively rather than deleted.
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        match now.duration_since(modified) {
            Ok(age) if age >= ORPHAN_MIN_AGE => match std::fs::remove_file(&path) {
                Ok(()) => report.removed.push(path),
                // Already gone — a concurrent cleaner or the user raced us.
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => report.errors.push((path, e)),
            },
            // Fresh temp, or mtime in the future (clock skew) — leave it.
            _ => {}
        }
    }
}

/// Match the `atomic-write-file` temp pattern for an `.org` target —
/// `.` + `{stem}.org` + `.` + exactly 6 ASCII alphanumerics — and return the
/// `{stem}.org` target name the temp points at.
///
/// The name alone cannot rule out a user file: "backup" is 6 ASCII
/// alphanumerics, so `.notes.org.backup` matches. The caller's
/// target-existence + mtime-age guards carry the rest of the safety burden.
fn orphan_target_name(name: &str) -> Option<&str> {
    let rest = name.strip_prefix('.')?;
    let (target, suffix) = rest.rsplit_once('.')?;
    (suffix.len() == 6
        && suffix.chars().all(|c| c.is_ascii_alphanumeric())
        && target.ends_with(".org")
        // Guard against a bare `.org` target name (`..org.abc123`): the temp
        // must belong to a plausible `{stem}.org` file.
        && target.len() > ".org".len())
    .then_some(target)
}

#[cfg(test)]
fn is_orphan_temp_name(name: &str) -> bool {
    orphan_target_name(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_retries_permission_denied() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "AV lock");
        assert!(is_transient_lock(&err));
    }

    #[test]
    fn classifier_retries_windows_sharing_and_lock_violations() {
        // ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION raw codes; the check
        // is platform-uniform (Dev Note 4).
        assert!(is_transient_lock(&io::Error::from_raw_os_error(32)));
        assert!(is_transient_lock(&io::Error::from_raw_os_error(33)));
    }

    #[test]
    fn classifier_rejects_non_transient_kinds() {
        assert!(!is_transient_lock(&io::Error::new(
            io::ErrorKind::NotFound,
            "target dir gone"
        )));
        assert!(!is_transient_lock(&io::Error::new(
            io::ErrorKind::StorageFull,
            "ENOSPC"
        )));
        assert!(!is_transient_lock(&io::Error::new(
            io::ErrorKind::IsADirectory,
            "target is a directory"
        )));
    }

    #[test]
    fn orphan_name_matches_crate_pattern_for_org_targets_only() {
        // Real pattern: `.{basename}.{6 alnum}` for an `.org` target.
        assert!(is_orphan_temp_name(".notes.org.aB3xY9"));
        assert!(is_orphan_temp_name(".hidden.org.abc123"));
        // The name alone cannot rule out a user file — "backup" is 6 ASCII
        // alphanumerics. The target-existence + mtime guards in the scan are
        // what keeps such files safe (see `symlink`/false-positive tests in
        // tests/orphan_cleanup.rs).
        assert!(is_orphan_temp_name(".notes.org.backup"));
        assert_eq!(orphan_target_name(".notes.org.aB3xY9"), Some("notes.org"));

        // Never touched: user dotfiles, wrong suffix length, non-org targets.
        assert!(!is_orphan_temp_name(".gitignore"));
        assert!(!is_orphan_temp_name(".orgsidian"));
        assert!(!is_orphan_temp_name("notes.org"));
        assert!(!is_orphan_temp_name(".notes.org.abc12")); // 5-char suffix
        assert!(!is_orphan_temp_name(".notes.org.abc1234")); // 7-char suffix
        assert!(!is_orphan_temp_name(".notes.org.ab-123")); // non-alnum
        assert!(!is_orphan_temp_name(".settings.toml.abc123")); // non-.org target
        assert!(!is_orphan_temp_name("..org.abc123")); // bare `.org` target
    }
}
