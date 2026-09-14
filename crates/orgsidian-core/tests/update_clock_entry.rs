//! Story 7.8 (FR-8): editing an existing CLOSED `CLOCK:` entry end-to-end over
//! a real scanned vault + derived index — the rewritten stamps, the recomputed
//! `=> HH:MM` duration, the byte-faithful remainder, and the guard rejections
//! (end-before-start, out-of-range index, still-running entry, a malformed
//! duration rewritten cleanly, and a stale `entry_index` whose start no longer
//! matches).
//!
//! Mirrors the `tests/clock.rs` harness: a single process-wide
//! `ORGSIDIAN_DATA_DIR` override (set once via a `OnceLock`) with each test on
//! its own vault `TempDir`, whose canonical root hashes to a distinct DB file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use orgsidian_core::parser::chrono::{NaiveDate, NaiveDateTime};
use orgsidian_core::{
    clock_in, clock_out, open_index, resolve_index_db_path, scan_vault, update_clock_entry,
};
use tempfile::TempDir;

static INDEX_BASE: OnceLock<TempDir> = OnceLock::new();

fn ensure_index_base() {
    INDEX_BASE.get_or_init(|| {
        let dir = TempDir::new().expect("index base tempdir");
        std::env::set_var("ORGSIDIAN_DATA_DIR", dir.path());
        dir
    });
}

/// A 2026-09-13 (Sunday) local datetime at `hh:mm` (zero seconds).
fn at(h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 13)
        .expect("valid date")
        .and_hms_opt(h, m, 0)
        .expect("valid time")
}

/// A 2026-09-14 (Monday) local datetime at `hh:mm` — for cross-midnight edits.
fn next_day(h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 14)
        .expect("valid date")
        .and_hms_opt(h, m, 0)
        .expect("valid time")
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, contents).expect("write file");
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).expect("read file")
}

struct Vault {
    _dir: TempDir,
    root: PathBuf,
    db: PathBuf,
}

async fn scanned_vault(files: &[(&str, &str)]) -> Vault {
    ensure_index_base();
    let dir = TempDir::new().expect("vault tempdir");
    let root = dir.path().to_path_buf();
    for (rel, contents) in files {
        write(&root, rel, contents);
    }
    let db = resolve_index_db_path(&root).expect("resolve db path");
    let handle = open_index(&root, &db).await.expect("open index");
    let cancel = AtomicBool::new(false);
    scan_vault(&handle, &cancel, |_| {}).await.expect("scan");
    handle.shutdown().await;
    Vault {
        _dir: dir,
        root,
        db,
    }
}

fn headline_id_by_title(db_path: &Path, title: &str) -> u32 {
    let conn = rusqlite::Connection::open(db_path).expect("open index for read");
    let id: i64 = conn
        .query_row(
            "SELECT id FROM headlines WHERE title = ?1 AND kind = 'headline'",
            [title],
            |row| row.get(0),
        )
        .expect("headline id");
    u32::try_from(id).expect("headline id fits u32")
}

/// Seed a genuine CLOSED entry the parser produced itself (clock in then out),
/// so the on-disk drawer shape and indentation are exactly what `analyze()`
/// round-trips — then rewrite it.
#[tokio::test(flavor = "multi_thread")]
async fn edits_a_closed_entry_and_recomputes_duration_byte_faithfully() {
    let v = scanned_vault(&[("a.org", "* Task A\nbody line stays put\n")]).await;
    let id = headline_id_by_title(&v.db, "Task A");

    clock_in(&v.root, id, at(10, 0)).await.expect("clock in");
    clock_out(&v.root, at(11, 0)).await.expect("clock out"); // closed => 1:00
    let before = read(&v.root, "a.org");
    assert!(
        before.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00"),
        "seed closed line: {before:?}"
    );

    // Correct the entry: 10:00-11:00 (1:00) -> 09:30-12:15 (2:45).
    update_clock_entry(&v.root, id, 0, at(10, 0), at(9, 30), at(12, 15))
        .await
        .expect("edit closed entry");

    let after = read(&v.root, "a.org");
    assert!(
        after.contains("CLOCK: [2026-09-13 Sun 09:30]--[2026-09-13 Sun 12:15] => 2:45"),
        "rewritten stamps + recomputed duration: {after:?}"
    );
    assert!(!after.contains("10:00"), "old start gone: {after:?}");
    assert!(!after.contains("11:00"), "old end gone: {after:?}");
    // Byte-faithful remainder: everything except the CLOCK line is identical.
    assert!(after.contains(":LOGBOOK:"), "drawer intact: {after:?}");
    assert!(after.contains(":END:"), "drawer end intact: {after:?}");
    assert!(
        after.contains("body line stays put"),
        "body intact: {after:?}"
    );
    let strip = |s: &str| {
        s.lines()
            .filter(|l| !l.contains("CLOCK:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip(&before), strip(&after), "only the CLOCK line changed");
}

/// A cross-midnight correction (forgot-to-clock-out overnight): the end date
/// input carries a later day, and the duration spans it.
#[tokio::test(flavor = "multi_thread")]
async fn edits_across_midnight() {
    let v = scanned_vault(&[("a.org", "* Overnight\n")]).await;
    let id = headline_id_by_title(&v.db, "Overnight");
    clock_in(&v.root, id, at(22, 0)).await.expect("clock in");
    clock_out(&v.root, at(23, 0)).await.expect("clock out");

    update_clock_entry(&v.root, id, 0, at(22, 0), at(22, 0), next_day(6, 30))
        .await
        .expect("cross-midnight edit");

    let after = read(&v.root, "a.org");
    assert!(
        after.contains("CLOCK: [2026-09-13 Sun 22:00]--[2026-09-14 Mon 06:30] => 8:30"),
        "cross-midnight line: {after:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_end_before_start_without_writing() {
    let v = scanned_vault(&[("a.org", "* Task A\n")]).await;
    let id = headline_id_by_title(&v.db, "Task A");
    clock_in(&v.root, id, at(10, 0)).await.expect("clock in");
    clock_out(&v.root, at(11, 0)).await.expect("clock out");
    let before = read(&v.root, "a.org");

    let err = update_clock_entry(&v.root, id, 0, at(10, 0), at(12, 0), at(11, 0))
        .await
        .expect_err("end before start must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged on reject");
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_out_of_range_index_without_writing() {
    let v = scanned_vault(&[("a.org", "* Task A\n")]).await;
    let id = headline_id_by_title(&v.db, "Task A");
    clock_in(&v.root, id, at(10, 0)).await.expect("clock in");
    clock_out(&v.root, at(11, 0)).await.expect("clock out");
    let before = read(&v.root, "a.org");

    let err = update_clock_entry(&v.root, id, 5, at(10, 0), at(9, 0), at(10, 0))
        .await
        .expect_err("out-of-range index must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged on reject");
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_editing_a_running_entry_without_writing() {
    let v = scanned_vault(&[("a.org", "* Task A\n")]).await;
    let id = headline_id_by_title(&v.db, "Task A");
    clock_in(&v.root, id, at(10, 0)).await.expect("clock in"); // open, no clock_out
    let before = read(&v.root, "a.org");
    assert!(
        before.contains("CLOCK: [2026-09-13 Sun 10:00]\n"),
        "open line present: {before:?}"
    );

    let err = update_clock_entry(&v.root, id, 0, at(10, 0), at(9, 0), at(11, 0))
        .await
        .expect_err("editing a running entry must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged on reject");
}

/// Fix #1 (data-corruption regression): a CLOSED entry whose `=> …` duration
/// suffix is MALFORMED (`=> N/A`) parses with `end == Some(..)` but a span that
/// stops right after `[end]`, not covering the ` => N/A` tail. A span-only
/// splice would leave that stale suffix after the freshly written ` => 2:45`,
/// producing a corrupted double-suffix line. The full-line splice must instead
/// rewrite the WHOLE line to a single clean byte-faithful entry.
#[tokio::test(flavor = "multi_thread")]
async fn edits_a_malformed_duration_entry_into_a_clean_line() {
    let v = scanned_vault(&[(
        "a.org",
        "* Task A\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => N/A\n:END:\nbody line stays put\n",
    )])
    .await;
    let id = headline_id_by_title(&v.db, "Task A");

    // Correct the malformed entry: expected start 10:00, new 09:30-12:15 (2:45).
    update_clock_entry(&v.root, id, 0, at(10, 0), at(9, 30), at(12, 15))
        .await
        .expect("edit malformed-duration entry");

    let after = read(&v.root, "a.org");
    assert!(
        after.contains("CLOCK: [2026-09-13 Sun 09:30]--[2026-09-13 Sun 12:15] => 2:45\n"),
        "clean rewritten line: {after:?}"
    );
    // The corruption this guards against: the stale malformed suffix must be gone
    // and there must be no double `=>` suffix on the line.
    assert!(
        !after.contains("N/A"),
        "stale malformed suffix removed: {after:?}"
    );
    let clock_line = after
        .lines()
        .find(|l| l.contains("CLOCK:"))
        .expect("a CLOCK line");
    assert_eq!(
        clock_line.matches("=>").count(),
        1,
        "exactly one duration suffix (no corruption): {clock_line:?}"
    );
    // Byte-faithful remainder: drawer + body untouched.
    assert!(after.contains(":LOGBOOK:"), "drawer intact: {after:?}");
    assert!(after.contains(":END:"), "drawer end intact: {after:?}");
    assert!(
        after.contains("body line stays put"),
        "body intact: {after:?}"
    );
}

/// Fix #2 (wrong-entry regression): LOGBOOK prepends newest, so an
/// `entry_index` captured by the UI can point at a DIFFERENT entry by the time
/// the edit confirms. When the entry at `entry_index` no longer has the start
/// the caller expected, the edit must be rejected without writing — never a
/// silent rewrite of the wrong entry.
#[tokio::test(flavor = "multi_thread")]
async fn rejects_a_stale_index_whose_entry_start_changed_without_writing() {
    // Two closed entries in one LOGBOOK; index 0 starts at 10:00.
    let v = scanned_vault(&[(
        "a.org",
        "* Task A\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00\nCLOCK: [2026-09-13 Sun 08:00]--[2026-09-13 Sun 09:00] => 1:00\n:END:\n",
    )])
    .await;
    let id = headline_id_by_title(&v.db, "Task A");
    let before = read(&v.root, "a.org");

    // Caller believed index 0 started at 08:00 (stale — it actually starts 10:00).
    let err = update_clock_entry(&v.root, id, 0, at(8, 0), at(8, 30), at(9, 30))
        .await
        .expect_err("stale index (start mismatch) must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged on reject");
}

/// The index still carries a Headline's document-order ordinal after that
/// Headline is removed from the source (the in-process index is not resynced
/// between commands): the ordinal then fails to resolve and the edit is a
/// no-write `OrgError::Vault`, never a wrong-line rewrite.
#[tokio::test(flavor = "multi_thread")]
async fn rejects_when_the_headline_ordinal_no_longer_resolves() {
    // Two headlines → the second is at document-order ordinal 1.
    let v = scanned_vault(&[(
        "a.org",
        "* Task A\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00\n* Task B\n",
    )])
    .await;
    let id_b = headline_id_by_title(&v.db, "Task B");

    // Drop Task B from the source AFTER indexing, so ordinal 1 no longer exists.
    write(&v.root, "a.org", "* Task A\n");
    let before = read(&v.root, "a.org");

    let err = update_clock_entry(&v.root, id_b, 0, at(10, 0), at(9, 0), at(11, 0))
        .await
        .expect_err("a vanished headline ordinal must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged on reject");
}
