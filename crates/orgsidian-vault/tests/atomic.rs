//! Story 3.1 — AV-aware retry wrapper fault-injection tests (LD-8 + NFR-15).
//!
//! Drives the retry loop through the `FileSystem` trait seam with scripted
//! error sequences and a recording sleeper — no real sleeps, no wall-clock
//! dependence (Testing Standards: deterministic backoff assertions). The
//! happy-path case exercises the real production path end-to-end, mirroring
//! the Story 1.9 anchor's byte-identity check.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::time::Duration;

use orgsidian_vault::atomic::{atomic_write_with, FileSystem};
use orgsidian_vault::VaultError;

/// Test fake: returns a scripted sequence of results and counts attempts.
struct ScriptedFs {
    script: RefCell<VecDeque<io::Result<()>>>,
    attempts: RefCell<u32>,
}

impl ScriptedFs {
    fn new(script: Vec<io::Result<()>>) -> Self {
        Self {
            script: RefCell::new(script.into()),
            attempts: RefCell::new(0),
        }
    }

    fn attempts(&self) -> u32 {
        *self.attempts.borrow()
    }
}

impl FileSystem for ScriptedFs {
    fn write_atomic_once(&self, _path: &Path, _content: &[u8]) -> io::Result<()> {
        *self.attempts.borrow_mut() += 1;
        self.script
            .borrow_mut()
            .pop_front()
            .expect("test script exhausted: retry loop attempted more writes than scripted")
    }
}

fn transient_err() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "AV holds a handle")
}

#[test]
fn transient_error_then_success_retries_once() {
    let fs = ScriptedFs::new(vec![Err(transient_err()), Ok(())]);
    let mut backoffs: Vec<Duration> = Vec::new();

    let result = atomic_write_with(
        &fs,
        |d| backoffs.push(d),
        Path::new("/vault/notes.org"),
        b"content",
    );

    assert!(result.is_ok(), "second attempt succeeds: {result:?}");
    assert_eq!(
        fs.attempts(),
        2,
        "exactly 2 attempts (1 failure + 1 success)"
    );
    assert_eq!(
        backoffs,
        vec![Duration::from_millis(100)],
        "one backoff of base 100ms before the second attempt"
    );
}

#[test]
fn transient_errors_exhaust_after_three_attempts() {
    let fs = ScriptedFs::new(vec![
        Err(transient_err()),
        Err(transient_err()),
        Err(transient_err()),
    ]);
    let mut backoffs: Vec<Duration> = Vec::new();

    let result = atomic_write_with(
        &fs,
        |d| backoffs.push(d),
        Path::new("/vault/notes.org"),
        b"content",
    );

    assert_eq!(fs.attempts(), 3, "exactly 3 attempts total (bounded)");
    assert_eq!(
        backoffs,
        vec![Duration::from_millis(100), Duration::from_millis(200)],
        "exponential backoff schedule: 100ms then 200ms"
    );
    match result {
        Err(VaultError::RetriesExhausted { path, attempts, .. }) => {
            assert_eq!(path, Path::new("/vault/notes.org"));
            assert_eq!(attempts, 3);
        }
        other => panic!("expected RetriesExhausted, got {other:?}"),
    }
}

#[test]
fn non_transient_error_fails_immediately_without_retry() {
    let fs = ScriptedFs::new(vec![Err(io::Error::new(
        io::ErrorKind::NotFound,
        "target directory gone",
    ))]);
    let mut backoffs: Vec<Duration> = Vec::new();

    let result = atomic_write_with(
        &fs,
        |d| backoffs.push(d),
        Path::new("/vault/notes.org"),
        b"content",
    );

    assert_eq!(
        fs.attempts(),
        1,
        "exactly 1 attempt — no retry on non-transient"
    );
    assert!(backoffs.is_empty(), "no backoff requested");
    match result {
        Err(VaultError::Io { path, source }) => {
            assert_eq!(path, Path::new("/vault/notes.org"));
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        other => panic!("expected VaultError::Io, got {other:?}"),
    }
}

#[test]
fn happy_path_writes_byte_identical_via_production_entry_point() {
    const CONTENT: &[u8] = b"* TODO Hello from Story 3.1\n";

    let dir = tempfile::TempDir::new().expect("TempDir must succeed");
    let target = dir.path().join("notes.org");

    orgsidian_vault::atomic_write(&target, CONTENT).expect("happy-path write must succeed");

    let read_back = std::fs::read(&target).expect("read-back must succeed");
    assert_eq!(read_back, CONTENT, "byte-identical after atomic write");
}

#[test]
fn vault_error_exposes_underlying_io_error() {
    let err = VaultError::Io {
        path: Path::new("/vault/notes.org").to_path_buf(),
        source: io::Error::new(io::ErrorKind::NotFound, "gone"),
    };
    assert_eq!(err.into_io().kind(), io::ErrorKind::NotFound);

    let err = VaultError::RetriesExhausted {
        path: Path::new("/vault/notes.org").to_path_buf(),
        attempts: 3,
        source: transient_err(),
    };
    assert_eq!(err.into_io().kind(), io::ErrorKind::PermissionDenied);
}
