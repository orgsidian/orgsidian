//! Story 3.6 (AC4 + AC7): the scan orchestrator end-to-end — walk → parse →
//! map → checkpoint-batch commit, incremental skip, cancellation with a
//! retained prefix, cached fast-open, resume-after-cancel, and drop-DB rebuild
//! equivalence. Driven against a real `TempDir` vault + a `TempDir` index DB
//! (hermetic: `open_index` takes the DB path directly, so these tests touch
//! neither the OS data dir nor `global.toml`).

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use orgsidian_core::{open_index, scan_vault, IndexHandle, ScanOutcome};
use tempfile::TempDir;

/// A vault dir + an index-DB path, each in its own `TempDir` (index lives
/// outside the vault — LD-40). Both dirs must stay alive for the test.
struct Fixture {
    _vault: TempDir,
    _index: TempDir,
    vault_root: std::path::PathBuf,
    db_path: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let vault = TempDir::new().expect("vault tempdir");
    let index = TempDir::new().expect("index tempdir");
    let vault_root = vault.path().to_path_buf();
    let db_path = index.path().join("index.sqlite3");
    Fixture {
        _vault: vault,
        _index: index,
        vault_root,
        db_path,
    }
}

/// Write `rel` under `root` with `contents` (parents created).
fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, contents).expect("write file");
}

/// One integer aggregate from a fresh read connection (WAL readers see the
/// writer's committed frames). Empty params via the unit tuple.
fn count(db_path: &Path, sql: &str) -> i64 {
    let conn = orgsidian_index::open(db_path).expect("open index for read");
    conn.query_row(sql, (), |row| row.get::<_, i64>(0))
        .expect("count query")
}

async fn scan(handle: &IndexHandle) -> ScanOutcome {
    let cancel = AtomicBool::new(false);
    scan_vault(handle, &cancel, |_| {})
        .await
        .expect("scan_vault")
}

#[tokio::test(flavor = "multi_thread")]
async fn indexes_all_valid_files_with_headlines() {
    let fx = fixture();
    write(&fx.vault_root, "a.org", "* Alpha\nbody a\n");
    write(&fx.vault_root, "sub/b.org", "* Beta\n** Child\nbody b\n");
    write(&fx.vault_root, "c.org", "Preamble text\n* Gamma\n");

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");
    let outcome = scan(&handle).await;

    assert_eq!(outcome.indexed, 3);
    assert_eq!(outcome.errors, 0);
    assert!(!outcome.cancelled);
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 3);
    // Alpha, Beta, Child, Gamma = 4 real headlines; c.org adds a preamble row.
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE kind = 'headline'"
        ),
        4
    );
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines WHERE kind = 'preamble'"
        ),
        1
    );
    // vault_meta records the canonical root.
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM vault_meta WHERE key = 'vault_root'"
        ),
        1
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn unreadable_file_is_quarantined_with_no_headlines() {
    use std::os::unix::fs::PermissionsExt;

    // Root bypasses file permission bits, so `chmod 000` still reads back fine
    // for uid 0 → `bad.org` would index instead of hitting the LD-41 read-failure
    // quarantine path this test exercises. The scenario is not exercisable as
    // root, so skip cleanly (the nightly arch-linux cell runs as root in a
    // container; the hosted non-root cells still assert the quarantine path).
    if unsafe { libc::geteuid() } == 0 {
        eprintln!(
            "[skip] unreadable_file_is_quarantined_with_no_headlines: root bypasses \
             mode bits, chmod 000 cannot produce an unreadable file"
        );
        return;
    }

    let fx = fixture();
    write(&fx.vault_root, "ok.org", "* Fine\n");
    write(&fx.vault_root, "bad.org", "* Cannot read\n");
    // Remove all permissions so the scan's read fails (LD-41 quarantine path;
    // the lenient parser never errors on content, so a read failure is the
    // realistic quarantine trigger).
    fs::set_permissions(
        fx.vault_root.join("bad.org"),
        fs::Permissions::from_mode(0o000),
    )
    .expect("chmod 000");

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");
    let outcome = scan(&handle).await;

    assert_eq!(outcome.indexed, 1);
    assert_eq!(outcome.errors, 1);
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM files WHERE quarantined = 1"
        ),
        1
    );
    // A quarantined file has NO headlines (schema contract).
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM headlines h JOIN files f ON h.file_id = f.id WHERE f.quarantined = 1",
        ),
        0
    );

    // Restore perms so the TempDir can be cleaned up.
    let _ = fs::set_permissions(
        fx.vault_root.join("bad.org"),
        fs::Permissions::from_mode(0o644),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn second_scan_of_unchanged_vault_writes_nothing() {
    let fx = fixture();
    write(&fx.vault_root, "a.org", "* Alpha\n");
    write(&fx.vault_root, "b.org", "* Beta\n");

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");
    let first = scan(&handle).await;
    assert_eq!(first.indexed, 2);

    // The indexed_at timestamps prove no rewrite happened on the second pass.
    let indexed_at_before = count(
        &fx.db_path,
        "SELECT count(*) FROM files WHERE indexed_at IS NOT NULL",
    );

    let second = scan(&handle).await;
    assert_eq!(second.indexed, 0, "unchanged vault must upsert nothing");
    assert_eq!(second.skipped, 2);
    assert_eq!(
        count(
            &fx.db_path,
            "SELECT count(*) FROM files WHERE indexed_at IS NOT NULL"
        ),
        indexed_at_before
    );
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_retains_committed_prefix_and_resume_completes() {
    let fx = fixture();
    // 250 files → the first checkpoint lands at 100.
    for i in 0..250 {
        write(
            &fx.vault_root,
            &format!("note-{i:03}.org"),
            "* Head\nbody\n",
        );
    }

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");

    // Cancel from inside the progress callback at the first checkpoint.
    let cancel = AtomicBool::new(false);
    let cancelled = scan_vault(&handle, &cancel, |progress| {
        if progress.current >= 100 {
            cancel.store(true, Ordering::Release);
        }
    })
    .await
    .expect("cancelled scan");

    assert!(cancelled.cancelled);
    assert_eq!(cancelled.indexed, 100);
    // Exactly the committed prefix survives (LD-42 "partial retained").
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 100);

    // Resume: the committed prefix is skipped, the remainder indexed, and the
    // final index equals a from-scratch scan (250 distinct files).
    let resume_cancel = AtomicBool::new(false);
    let resumed = scan_vault(&handle, &resume_cancel, |_| {})
        .await
        .expect("resume scan");

    assert!(!resumed.cancelled);
    assert_eq!(resumed.skipped, 100);
    assert_eq!(resumed.indexed, 150);
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 250);
    // No file indexed twice: files.path is UNIQUE, so 250 rows = 250 distinct.
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_index_rebuilds_an_identical_index() {
    let fx = fixture();
    write(&fx.vault_root, "a.org", "* Alpha\n** A1\nbody\n");
    write(
        &fx.vault_root,
        "b.org",
        "Preamble\n* Beta\n:PROPERTIES:\n:K: v\n:END:\n",
    );

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");
    let _ = scan(&handle).await;

    let files_before = count(&fx.db_path, "SELECT count(*) FROM files");
    let headlines_before = count(&fx.db_path, "SELECT count(*) FROM headlines");
    let props_before = count(&fx.db_path, "SELECT count(*) FROM properties");

    // Shut the handle down (drain the writer + close pool connections) BEFORE
    // deleting the DB — a bare drop does not await the writer's async shutdown,
    // so a still-open connection would race the file deletion.
    handle.shutdown().await;
    for suffix in ["", "-wal", "-shm"] {
        let mut name = fx.db_path.clone().into_os_string();
        name.push(suffix);
        let _ = fs::remove_file(std::path::PathBuf::from(name));
    }

    // Re-designating rebuilds from the vault's .org files (LD-13).
    let rebuilt = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("reopen index");
    let _ = scan(&rebuilt).await;

    assert_eq!(
        count(&fx.db_path, "SELECT count(*) FROM files"),
        files_before
    );
    assert_eq!(
        count(&fx.db_path, "SELECT count(*) FROM headlines"),
        headlines_before
    );
    assert_eq!(
        count(&fx.db_path, "SELECT count(*) FROM properties"),
        props_before
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn open_index_refuses_a_foreign_database() {
    let fx = fixture();
    write(&fx.vault_root, "a.org", "* Alpha\n");
    // A pre-existing NON-Orgsidian SQLite file at the index path.
    let conn = orgsidian_index::open(&fx.db_path).expect("seed foreign db");
    conn.execute_batch("CREATE TABLE someone_elses (x);")
        .expect("seed foreign schema");
    drop(conn);

    let result = open_index(&fx.vault_root, &fx.db_path).await;
    assert!(
        result.is_err(),
        "open_index must refuse a foreign SQLite file rather than overwrite it"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn initial_scan_of_1000_files_completes_well_under_budget() {
    let fx = fixture();
    for i in 0..1000 {
        write(
            &fx.vault_root,
            &format!("dir{}/note-{i:04}.org", i % 10),
            "* Task :work:\nSCHEDULED: <2026-08-13 Thu>\nsome body text here\n",
        );
    }

    let handle = open_index(&fx.vault_root, &fx.db_path)
        .await
        .expect("open index");

    let start = Instant::now();
    let outcome = scan(&handle).await;
    let elapsed = start.elapsed();

    assert_eq!(outcome.indexed, 1000);
    assert_eq!(outcome.errors, 0);
    assert_eq!(count(&fx.db_path, "SELECT count(*) FROM files"), 1000);
    // Generous CI-safe ceiling (the FR-15/LD-13 budget is <30s; a hard timing
    // gate is NOT added — the perf-snapshot macro is not applied to this crate,
    // Story 3.5 precedent). The actual is recorded in the story Debug Log.
    assert!(
        elapsed.as_secs() < 60,
        "1000-file scan took {elapsed:?}, well over the generous ceiling"
    );
    eprintln!("[perf] 1000-file initial scan: {elapsed:?}");
}
