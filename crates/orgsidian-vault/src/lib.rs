//! orgsidian-vault: vault designation + atomic write subsystem + dirty-buffer manager (FR-3, FR-4, FR-5).
//!
//! Story 1.9 ships the anchor-smoke surface only — `atomic_write` is a single
//! delegation to the `atomic-write-file` crate (LD-8). Story 3.1 wraps this in
//! the 3-retry AV-aware exponential-backoff loop; Story 3.2 introduces the
//! Dirty Buffer module alongside. The public signature
//! `atomic_write(path, content) -> io::Result<()>` is preserved across the
//! Story 3.1 swap (anchor sentinel discipline — see `tests/anchor.rs`).

use std::io::{self, Write};
use std::path::Path;

use atomic_write_file::AtomicWriteFile;

/// Atomically write `content` to `path`. Existing files are overwritten via the
/// `atomic-write-file` crate's temp-then-rename dance (`renameat` on Unix,
/// `MoveFileExW` on Windows). No retry / backoff wrapper — Story 3.1 adds it.
pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(content)?;
    file.commit()
}
