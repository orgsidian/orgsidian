//! Story 7.7 (FR-8 / UJ-1): the prior-session running-clock launch prompt over
//! a real scanned vault + derived index — the launch summary and the three
//! documented state transitions (Keep tracking / Discard this session / Adjust
//! end time), plus the deferred `clock_in` open-line-adopt guard.
//!
//! Reuses the `tests/clock.rs` harness (a single process-wide
//! `ORGSIDIAN_DATA_DIR` override behind a `OnceLock`; each test owns its vault
//! `TempDir`, whose canonical root hashes to a distinct DB filename, so the
//! tests run in parallel despite sharing the env var).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use orgsidian_core::parser::chrono::{NaiveDate, NaiveDateTime};
use orgsidian_core::{
    active_clock, active_clock_path, clock_discard, clock_in, clock_out, clock_resume, open_index,
    refresh_active_clock, resolve_index_db_path, scan_vault, stale_clock_summary, OrgError,
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

/// A 2026-09-13 (Sunday) local datetime at `hh:mm`.
fn at(h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 13)
        .expect("valid date")
        .and_hms_opt(h, m, 0)
        .expect("valid time")
}

/// A 2026-09-14 (Monday) local datetime at `hh:mm` — the "next-day" launch.
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

fn open_clock_lines(src: &str) -> usize {
    src.lines()
        .filter(|l| l.trim_start().starts_with("CLOCK: [") && !l.contains("--"))
        .count()
}

/// Seed a prior-session stale clock with a 14 h `started_at`→`last_active_at`
/// gap: clock in at 04:00, then bump `last_active_at` to 18:00. The pointer is
/// left in place (simulating an app that closed with the clock still running).
async fn seed_stale_clock(v: &Vault, id: u32) {
    clock_in(&v.root, id, at(4, 0))
        .await
        .expect("clock in 04:00");
    refresh_active_clock(&v.root, at(18, 0)).expect("bump last_active to 18:00");
    let ptr = active_clock(&v.root).expect("pointer").expect("some");
    assert_eq!(ptr.started_at, "2026-09-13T04:00:00");
    assert_eq!(ptr.last_active_at, "2026-09-13T18:00:00");
}

/// Hand-write the `active-clock.json` pointer verbatim (snake_case on-disk keys)
/// — for the desync/malformed cases the normal `clock_in` path never produces.
fn write_pointer_json(root: &Path, headline_id: u32, started_at: &str, last_active_at: &str) {
    let path = active_clock_path(root);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir .orgsidian");
    let body = format!(
        "{{\n  \"headline_id\": {headline_id},\n  \"started_at\": {started_at:?},\n  \"last_active_at\": {last_active_at:?}\n}}\n"
    );
    fs::write(&path, body).expect("write pointer");
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_summary_reports_headline_and_both_durations() {
    let v = scanned_vault(&[("a.org", "* Write the report\n")]).await;
    let id = headline_id_by_title(&v.db, "Write the report");
    seed_stale_clock(&v, id).await;

    // Launch next-day at 10:00 → keep = 30 h, adjust = 14 h.
    let summary = stale_clock_summary(&v.root, next_day(10, 0))
        .await
        .expect("summary")
        .expect("a stale clock exists");
    assert_eq!(summary.headline_id, id);
    assert_eq!(summary.headline, "Write the report");
    assert_eq!(summary.started_at, "2026-09-13T04:00:00");
    assert_eq!(summary.last_active_at, "2026-09-13T18:00:00");
    assert_eq!(summary.keep_duration, "30:00");
    assert_eq!(summary.adjust_duration, "14:00");
}

#[tokio::test(flavor = "multi_thread")]
async fn no_active_clock_yields_no_summary() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    assert_eq!(
        stale_clock_summary(&v.root, next_day(10, 0))
            .await
            .expect("summary"),
        None
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn keep_tracking_preserves_started_at_and_leaves_source_unchanged() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    seed_stale_clock(&v, id).await;
    let before = read(&v.root, "a.org");

    // "Keep tracking" reuses clock_resume — re-adopts the open line, no mutation.
    let resumed = clock_resume(&v.root, id, next_day(10, 0))
        .await
        .expect("keep tracking");
    assert_eq!(
        resumed.started_at, "2026-09-13T04:00:00",
        "started_at from the original open line, not `now`"
    );
    assert_eq!(read(&v.root, "a.org"), before, "source unchanged");
    assert!(
        active_clock(&v.root).expect("pointer").is_some(),
        "pointer kept"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn discard_removes_the_open_line_and_clears_the_pointer() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    seed_stale_clock(&v, id).await;
    assert!(read(&v.root, "a.org").contains("CLOCK: [2026-09-13 Sun 04:00]\n"));

    clock_discard(&v.root).await.expect("discard");
    let src = read(&v.root, "a.org");
    assert!(
        !src.contains("CLOCK:"),
        "the open CLOCK line was removed: {src:?}"
    );
    assert!(
        src.contains(":LOGBOOK:") && src.contains(":END:"),
        "the (now empty) LOGBOOK drawer is still valid org: {src:?}"
    );
    assert_eq!(
        active_clock(&v.root).expect("pointer"),
        None,
        "pointer cleared"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn adjust_end_closes_the_line_at_last_active_and_clears_the_pointer() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    seed_stale_clock(&v, id).await;

    // "Adjust end time" reuses clock_out at the chosen end (here: last_active).
    clock_out(&v.root, at(18, 0)).await.expect("adjust end");
    let src = read(&v.root, "a.org");
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 04:00]--[2026-09-13 Sun 18:00] => 14:00"),
        "closed at 18:00 for a 14 h session: {src:?}"
    );
    assert_eq!(
        active_clock(&v.root).expect("pointer"),
        None,
        "pointer cleared"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn adjust_end_before_start_clamps_duration_to_zero() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    seed_stale_clock(&v, id).await;

    // An end before the start clamps to `=> 0:00` (clock_out's non-negative clamp).
    clock_out(&v.root, at(3, 0))
        .await
        .expect("adjust end < start");
    let src = read(&v.root, "a.org");
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 04:00]--[2026-09-13 Sun 03:00] => 0:00"),
        "backwards range clamps to 0:00: {src:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn clock_in_adopts_a_pre_existing_open_line_without_a_second() {
    // A Headline that already owns ONE open CLOCK line, but no pointer.
    let v = scanned_vault(&[(
        "a.org",
        "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 08:00]\n:END:\n",
    )])
    .await;
    let id = headline_id_by_title(&v.db, "Task");
    assert!(active_clock(&v.root).expect("pointer").is_none());

    let adopted = clock_in(&v.root, id, at(10, 0))
        .await
        .expect("clock in adopts the open line");
    assert_eq!(
        adopted.started_at, "2026-09-13T08:00:00",
        "pointer written to the existing open line's start, not `now`"
    );
    let src = read(&v.root, "a.org");
    assert_eq!(
        open_clock_lines(&src),
        1,
        "no second open line inserted: {src:?}"
    );
    assert!(src.contains("CLOCK: [2026-09-13 Sun 08:00]\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn clock_in_neutralizes_duplicate_open_lines() {
    // A Headline with TWO open CLOCK lines (10:00 newest, 08:00 older), no pointer.
    let v = scanned_vault(&[(
        "a.org",
        "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-13 Sun 08:00]\n:END:\n",
    )])
    .await;
    let id = headline_id_by_title(&v.db, "Task");

    let adopted = clock_in(&v.root, id, at(12, 0))
        .await
        .expect("clock in adopts the most-recent open line");
    assert_eq!(
        adopted.started_at, "2026-09-13T10:00:00",
        "adopts the most-recent (10:00) open line"
    );
    let src = read(&v.root, "a.org");
    assert_eq!(
        open_clock_lines(&src),
        1,
        "exactly one open line remains (the adopted 10:00): {src:?}"
    );
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 08:00]--[2026-09-13 Sun 08:00] => 0:00"),
        "the older duplicate was neutralized to 0:00: {src:?}"
    );
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 10:00]\n"),
        "the adopted open line is untouched: {src:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn discard_without_an_active_clock_errors() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let err = clock_discard(&v.root)
        .await
        .expect_err("no active clock must error");
    assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn discard_desync_clears_the_pointer_and_errors() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    seed_stale_clock(&v, id).await;

    // Rewrite the source so the open line the pointer references no longer exists.
    write(&v.root, "a.org", "* Task\n");
    let err = clock_discard(&v.root)
        .await
        .expect_err("a desynced pointer must error");
    assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
    assert_eq!(
        active_clock(&v.root).expect("pointer"),
        None,
        "desync clears the dangling pointer"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn summary_normalizes_a_malformed_last_active_at_to_zero_adjust() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");
    // A well-formed started_at but a garbage last_active_at.
    write_pointer_json(&v.root, id, "2026-09-13T04:00:00", "not-a-timestamp");

    let summary = stale_clock_summary(&v.root, next_day(10, 0))
        .await
        .expect("summary")
        .expect("a stale clock exists");
    assert_eq!(
        summary.adjust_duration, "0:00",
        "a malformed last_active_at falls back to started_at (adjust = 0:00)"
    );
    // The returned last_active_at is NORMALIZED (falls back to started_at), never
    // the raw garbage — so the frontend can safely slice it.
    assert_eq!(summary.last_active_at, "2026-09-13T04:00:00");
    assert_eq!(summary.started_at, "2026-09-13T04:00:00");
    assert_eq!(summary.keep_duration, "30:00");
}

#[tokio::test(flavor = "multi_thread")]
async fn summary_errors_when_the_pointer_headline_is_not_in_the_index() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    // A valid pointer, but a headline_id no headline carries.
    write_pointer_json(
        &v.root,
        999_999,
        "2026-09-13T04:00:00",
        "2026-09-13T18:00:00",
    );

    let err = stale_clock_summary(&v.root, next_day(10, 0))
        .await
        .expect_err("an unknown headline_id must error");
    assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
}
