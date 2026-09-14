//! Story 7.6 (FR-8): the Clock manager end-to-end over a real scanned vault +
//! derived index — clock in / out / resume, the auto-stop single-clock
//! invariant, `last_active_at` focus refresh, the desync recovery path, and
//! the "locate by started_at, never by a stale byte offset" robustness claim.
//!
//! The clock commands resolve the index from the vault root exactly as the
//! Tauri layer does, so the index base directory is pinned to a single
//! process-wide `ORGSIDIAN_DATA_DIR` override (set once, with happens-before,
//! via a `OnceLock`). Each test uses its OWN vault `TempDir`, whose canonical
//! root hashes to a distinct DB filename under that shared base — so the tests
//! never collide and can run in parallel despite sharing the env var.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use orgsidian_core::parser::chrono::{NaiveDate, NaiveDateTime};
use orgsidian_core::{
    active_clock, active_clock_path, clock_in, clock_out, clock_resume, open_index,
    refresh_active_clock, resolve_index_db_path, scan_vault,
};
use tempfile::TempDir;

/// The shared index base directory. `get_or_init` sets `ORGSIDIAN_DATA_DIR`
/// exactly once (before any test resolves an index path) and keeps the TempDir
/// alive for the whole binary — the `OnceLock` supplies the happens-before that
/// makes the single `set_var` safe against the parallel readers.
static INDEX_BASE: OnceLock<TempDir> = OnceLock::new();

fn ensure_index_base() {
    INDEX_BASE.get_or_init(|| {
        let dir = TempDir::new().expect("index base tempdir");
        std::env::set_var("ORGSIDIAN_DATA_DIR", dir.path());
        dir
    });
}

/// A 2026-09-13 (Sunday) local datetime at `hh:mm:ss`.
fn at_s(h: u32, m: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 13)
        .expect("valid date")
        .and_hms_opt(h, m, s)
        .expect("valid time")
}

/// A 2026-09-13 (Sunday) local datetime at `hh:mm` (zero seconds).
fn at(h: u32, m: u32) -> NaiveDateTime {
    at_s(h, m, 0)
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, contents).expect("write file");
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).expect("read file")
}

/// A scanned vault: its root, and the on-disk index DB path (writer already
/// shut down, so the clock commands read it through their own fresh pools).
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

/// The `headlines.id` for the headline with `title` (WAL readers see the
/// writer's committed frames; rusqlite is a dev-dependency).
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

#[tokio::test(flavor = "multi_thread")]
async fn clock_in_out_resume_autostop_and_refresh() {
    let v = scanned_vault(&[("a.org", "* Task A\n"), ("b.org", "* Task B\n")]).await;
    let id_a = headline_id_by_title(&v.db, "Task A");
    let id_b = headline_id_by_title(&v.db, "Task B");

    // ---- clock in ----
    let clock = clock_in(&v.root, id_a, at(10, 0)).await.expect("clock in");
    assert_eq!(clock.headline_id, id_a);
    assert_eq!(clock.started_at, "2026-09-13T10:00:00");
    assert_eq!(clock.last_active_at, "2026-09-13T10:00:00");
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(clock.clone()));
    let a_src = read(&v.root, "a.org");
    assert!(a_src.contains(":LOGBOOK:"), "LOGBOOK created: {a_src:?}");
    assert!(
        a_src.contains("CLOCK: [2026-09-13 Sun 10:00]\n"),
        "open CLOCK line (no `=>`): {a_src:?}"
    );

    // ---- clock out ----
    clock_out(&v.root, at(11, 30)).await.expect("clock out");
    assert_eq!(active_clock(&v.root).expect("pointer"), None);
    let a_src = read(&v.root, "a.org");
    assert!(
        a_src.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:30] => 1:30"),
        "open line closed with duration: {a_src:?}"
    );

    // ---- resume an existing unclosed line without mutating source ----
    clock_in(&v.root, id_a, at(12, 0))
        .await
        .expect("clock in again");
    fs::remove_file(active_clock_path(&v.root)).expect("simulate restart: drop pointer");
    let before_resume = read(&v.root, "a.org");
    let resumed = clock_resume(&v.root, id_a, at(13, 0))
        .await
        .expect("resume");
    assert_eq!(
        resumed.started_at, "2026-09-13T12:00:00",
        "resume re-activates the existing line's start, not `now`"
    );
    assert_eq!(
        read(&v.root, "a.org"),
        before_resume,
        "resume must not mutate source"
    );

    // ---- auto-stop: clocking into B stops A first ----
    let b_clock = clock_in(&v.root, id_b, at(14, 0))
        .await
        .expect("clock in B");
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(b_clock));
    let a_src = read(&v.root, "a.org");
    assert!(
        a_src.contains("CLOCK: [2026-09-13 Sun 12:00]--[2026-09-13 Sun 14:00] => 2:00"),
        "A auto-closed when B started: {a_src:?}"
    );
    assert!(read(&v.root, "b.org").contains("CLOCK: [2026-09-13 Sun 14:00]\n"));

    // ---- last_active_at refresh ----
    refresh_active_clock(&v.root, at(15, 0)).expect("refresh");
    let after = active_clock(&v.root).expect("pointer").expect("some");
    assert_eq!(
        after.started_at, "2026-09-13T14:00:00",
        "started_at unchanged"
    );
    assert_eq!(
        after.last_active_at, "2026-09-13T15:00:00",
        "last_active_at bumped"
    );

    // ---- clock out with nothing active errors, no writes ----
    clock_out(&v.root, at(16, 0)).await.expect("clock out B");
    assert_eq!(active_clock(&v.root).expect("pointer"), None);
    let err = clock_out(&v.root, at(16, 30))
        .await
        .expect_err("no active clock errors");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
}

/// #7: a `now` carrying non-zero seconds still closes the open line — proves
/// the minute-truncation fix (pre-fix, the stored `started_at` kept its seconds
/// and never matched the minute-precision CLOCK line, hitting the desync
/// branch and orphaning the open line).
#[tokio::test(flavor = "multi_thread")]
async fn clock_out_matches_despite_sub_minute_now() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");

    // Clock in at 10:00:37 → stored started_at normalizes to the minute.
    let clock = clock_in(&v.root, id, at_s(10, 0, 37))
        .await
        .expect("clock in");
    assert_eq!(clock.started_at, "2026-09-13T10:00:00", "seconds truncated");

    // Clock out at 11:30:45 → normalizes to 11:30, matches, closes cleanly.
    clock_out(&v.root, at_s(11, 30, 45))
        .await
        .expect("clock out");
    assert_eq!(
        active_clock(&v.root).expect("pointer"),
        None,
        "not a desync"
    );
    let src = read(&v.root, "a.org");
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:30] => 1:30"),
        "line closed, not orphaned: {src:?}"
    );
}

/// #8: two headlines in one file — clocking in on the first shifts the second's
/// byte offset in the (now stale) index; clock-out must still close the correct
/// open line via `started_at` matching, not the stale byte offset.
#[tokio::test(flavor = "multi_thread")]
async fn clock_out_locates_by_started_at_after_offsets_shift() {
    let v = scanned_vault(&[("a.org", "* First\n* Second\n")]).await;
    let id_first = headline_id_by_title(&v.db, "First");

    clock_in(&v.root, id_first, at(10, 0))
        .await
        .expect("clock in first");
    // The splice inserted a LOGBOOK after "* First", shifting "* Second" past
    // its indexed byte_start — the index is now stale for Second.
    clock_out(&v.root, at(11, 0)).await.expect("clock out");

    let src = read(&v.root, "a.org");
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00"),
        "First's line closed via started_at match: {src:?}"
    );
    assert!(
        src.contains("* Second\n"),
        "Second headline intact: {src:?}"
    );
    assert_eq!(active_clock(&v.root).expect("pointer"), None);
}

/// #9: clock in / out / resume on a nested (level-2) child headline mutate the
/// child's OWN `:LOGBOOK:` (exercises the child recursion in
/// `find_headline_by_start` and `find_open_clock_in_tree`).
#[tokio::test(flavor = "multi_thread")]
async fn clock_flow_on_a_nested_child_headline() {
    let v = scanned_vault(&[("a.org", "* Parent\n** Child\nbody\n")]).await;
    let id_child = headline_id_by_title(&v.db, "Child");

    clock_in(&v.root, id_child, at(10, 0))
        .await
        .expect("clock in child");
    let src = read(&v.root, "a.org");
    // The LOGBOOK lands under the child, before its body, not under the parent.
    assert!(
        src.contains("** Child\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\nbody\n"),
        "child's own LOGBOOK mutated: {src:?}"
    );

    clock_out(&v.root, at(10, 30))
        .await
        .expect("clock out child");
    let src = read(&v.root, "a.org");
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 10:30] => 0:30"),
        "child's line closed via tree recursion: {src:?}"
    );

    // Resume the child's (now only, closed) history → no unclosed line → fresh
    // clock-in on the child.
    let resumed = clock_resume(&v.root, id_child, at(11, 0))
        .await
        .expect("resume child");
    assert_eq!(
        resumed.started_at, "2026-09-13T11:00:00",
        "fresh clock-in fallback"
    );
    assert!(read(&v.root, "a.org").contains("CLOCK: [2026-09-13 Sun 11:00]\n"));
}

/// #10: resume on a headline with NO unclosed line falls back to a fresh
/// clock-in whose `started_at` equals `now`.
#[tokio::test(flavor = "multi_thread")]
async fn resume_without_an_open_line_starts_a_fresh_clock() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");

    let clock = clock_resume(&v.root, id, at(9, 15)).await.expect("resume");
    assert_eq!(
        clock.started_at, "2026-09-13T09:15:00",
        "started_at == now (fresh)"
    );
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(clock));
    let src = read(&v.root, "a.org");
    assert!(src.contains(":LOGBOOK:") && src.contains("CLOCK: [2026-09-13 Sun 09:15]\n"));
}

/// #11: resume while a DIFFERENT headline is active closes the other and leaves
/// the resumed one sole active; resuming the SAME active headline does NOT
/// close its own line.
#[tokio::test(flavor = "multi_thread")]
async fn resume_stops_a_different_active_clock_but_not_its_own() {
    let v = scanned_vault(&[("a.org", "* Task A\n"), ("b.org", "* Task B\n")]).await;
    let id_a = headline_id_by_title(&v.db, "Task A");
    let id_b = headline_id_by_title(&v.db, "Task B");

    // Give B an unclosed line, then drop the pointer (as if a prior session).
    clock_in(&v.root, id_b, at(9, 0)).await.expect("clock in B");
    fs::remove_file(active_clock_path(&v.root)).expect("drop pointer");
    // Make A the active clock.
    clock_in(&v.root, id_a, at(10, 0))
        .await
        .expect("clock in A");

    // Resume B while A is active → A auto-closed, B sole active at its own 09:00.
    let resumed = clock_resume(&v.root, id_b, at(11, 0))
        .await
        .expect("resume B");
    assert_eq!(resumed.headline_id, id_b);
    assert_eq!(resumed.started_at, "2026-09-13T09:00:00");
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(resumed));
    assert!(
        read(&v.root, "a.org")
            .contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00"),
        "A (the different clock) was auto-closed"
    );
    let b_before = read(&v.root, "b.org");
    assert!(
        b_before.contains("CLOCK: [2026-09-13 Sun 09:00]\n"),
        "B's line still open after resuming it"
    );

    // Resume B again while B IS active → must NOT close B's own line.
    let again = clock_resume(&v.root, id_b, at(12, 0))
        .await
        .expect("resume B again");
    assert_eq!(again.started_at, "2026-09-13T09:00:00");
    assert_eq!(read(&v.root, "b.org"), b_before, "B's own line untouched");
}

/// #12: clock-out desync — the pointer references a start no open line has;
/// the pointer is cleared (never left dangling) and an `OrgError::Vault` is
/// returned.
#[tokio::test(flavor = "multi_thread")]
async fn clock_out_desync_clears_the_pointer_and_errors() {
    let v = scanned_vault(&[("a.org", "* Task\n")]).await;
    let id = headline_id_by_title(&v.db, "Task");

    // Open a clock, then rewrite the file so the open line no longer exists.
    clock_in(&v.root, id, at(10, 0)).await.expect("clock in");
    assert!(active_clock(&v.root).expect("pointer").is_some());
    write(&v.root, "a.org", "* Task\n"); // drop the LOGBOOK/open line

    let err = clock_out(&v.root, at(11, 0))
        .await
        .expect_err("a desynced pointer must error");
    assert!(
        matches!(err, orgsidian_core::OrgError::Vault { .. }),
        "got {err:?}"
    );
    assert_eq!(
        active_clock(&v.root).expect("pointer"),
        None,
        "desync clears the dangling pointer"
    );
}

/// Reviewer BLOCKER regression: clocking into B while A is active in the SAME
/// file must auto-stop A and open B, leaving EXACTLY B active — no error. The
/// auto-stop write shifts B past its stale indexed `byte_start`, so a raw
/// `byte_start` lookup for B fails; the fix re-resolves B by its structural
/// tree position. Pre-fix this returned `Err(Vault { "headline .. was not found
/// at byte .." })`, closing A but never opening B.
#[tokio::test(flavor = "multi_thread")]
async fn clock_in_switch_between_two_headlines_in_one_file() {
    let v = scanned_vault(&[("ab.org", "* First\n* Second\n")]).await;
    let id_first = headline_id_by_title(&v.db, "First");
    let id_second = headline_id_by_title(&v.db, "Second");

    clock_in(&v.root, id_first, at(10, 0))
        .await
        .expect("clock in First");
    // Switch to Second while First is active (same file) — must NOT error.
    let b = clock_in(&v.root, id_second, at(11, 0))
        .await
        .expect("switch to Second in the same file");

    // Exactly B is active.
    assert_eq!(b.headline_id, id_second);
    assert_eq!(b.started_at, "2026-09-13T11:00:00");
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(b));

    let src = read(&v.root, "ab.org");
    // First auto-closed at the switch instant.
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00"),
        "First auto-closed when switching to Second: {src:?}"
    );
    // Second has a fresh OPEN line.
    assert!(
        src.contains("* Second\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 11:00]\n:END:\n"),
        "Second opened with its own LOGBOOK: {src:?}"
    );
    // Exactly one open (no `--`) CLOCK line remains in the whole file — B's.
    let open_lines = src
        .lines()
        .filter(|l| l.trim_start().starts_with("CLOCK: [") && !l.contains("--"))
        .count();
    assert_eq!(open_lines, 1, "exactly one open clock line (B's): {src:?}");

    // Switch BACK to First (Second→First) while Second is active — the ordinal
    // re-location must still resolve both headlines despite the accumulated
    // same-file offset shifts from every prior CLOCK write.
    let a2 = clock_in(&v.root, id_first, at(12, 0))
        .await
        .expect("switch back to First in the same file");
    assert_eq!(a2.headline_id, id_first);
    assert_eq!(a2.started_at, "2026-09-13T12:00:00");
    assert_eq!(active_clock(&v.root).expect("pointer"), Some(a2));

    let src = read(&v.root, "ab.org");
    // Second auto-closed on the switch back.
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 11:00]--[2026-09-13 Sun 12:00] => 1:00"),
        "Second auto-closed when switching back to First: {src:?}"
    );
    // First now has a fresh OPEN line (prepended above its earlier closed one).
    assert!(
        src.contains("CLOCK: [2026-09-13 Sun 12:00]\n"),
        "First re-opened: {src:?}"
    );
    let open_lines = src
        .lines()
        .filter(|l| l.trim_start().starts_with("CLOCK: [") && !l.contains("--"))
        .count();
    assert_eq!(
        open_lines, 1,
        "exactly one open clock line (First's): {src:?}"
    );
}

/// Fix #2 (MAJOR): `clock_out` must close only the ACTIVE headline's own open
/// line. With a stray unclosed `CLOCK:` in another headline of the same file at
/// a coincidentally identical minute, clocking out must close the active
/// headline's line and leave the stray one untouched. Pre-fix the whole-tree
/// `started_at` search closed the FIRST match (the stray line) instead.
#[tokio::test(flavor = "multi_thread")]
async fn clock_out_scopes_to_the_active_headline_not_a_stray_open_line() {
    // Alpha carries a pre-existing stray open CLOCK at 10:00; Beta will get its
    // own open CLOCK at the same minute via clock_in.
    let v = scanned_vault(&[(
        "s.org",
        "* Alpha\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\n* Beta\n",
    )])
    .await;
    let id_beta = headline_id_by_title(&v.db, "Beta");

    clock_in(&v.root, id_beta, at(10, 0))
        .await
        .expect("clock in Beta");
    // Two open lines at 10:00 now exist (Alpha's stray + Beta's). Clock out must
    // close Beta's (the active one), not Alpha's coincidental line.
    clock_out(&v.root, at(11, 0)).await.expect("clock out Beta");

    let src = read(&v.root, "s.org");
    assert!(
        src.contains(
            "* Beta\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00"
        ),
        "Beta's own line was closed: {src:?}"
    );
    assert!(
        src.contains("* Alpha\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\n"),
        "Alpha's stray open line was left untouched: {src:?}"
    );
    assert_eq!(active_clock(&v.root).expect("pointer"), None);
}
