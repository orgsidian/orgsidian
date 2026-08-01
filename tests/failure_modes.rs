//! Story 1.11 — LD-41 failure-mode harness (cross-cutting).
//!
//! Enumerates every LD-41 failure-mode category as `#[ignore]` placeholders so
//! downstream stories inherit a red-phase stub they must replace with real
//! fault-injection tests (see `_bmad-output/test-artifacts/test-design.md §6.7`).
//!
//! Intentionally `#[ignore]`-heavy at the v0.1 Alpha era — coverage is filled
//! in incrementally by the owning epics. Each placeholder body panics with a
//! human-readable category label, so removing an `#[ignore]` without authoring
//! real assertion logic surfaces a loud failure instead of a silent green.
//!
//! Fault-injection mechanism: the `fail` crate (workspace dep, `failpoints`
//! feature enabled at the `orgsidian-core` `[dev-dependencies]` level). The
//! `disk_full_atomic_write` placeholder carries the canonical commented-out
//! exemplar.

#[test]
#[ignore = "implemented in Epic 2"]
fn malformed_org_file_quarantined() {
    unimplemented!(
        "LD-41: Malformed .org file quarantined — \
         see test-design.md §6.7 + Story 2.2 for real implementation"
    );
}

#[test]
fn disk_full_atomic_write() {
    // Story 3.1: inject an ENOSPC-shaped failure after `AtomicWriteFile::open`
    // (the temp sibling already exists when the fail-point fires) and assert
    // the LD-41 disk-full row: error surfaces immediately (no retry), the
    // target's prior content is intact, and no temp residue remains — the
    // explicit `discard()` on the error path is what this proves.
    let scenario = fail::FailScenario::setup();
    // Bare `return` action: the injected error (ENOSPC-shaped `StorageFull`)
    // is fixed inside the fail-point closure in `vault::atomic::write_body` —
    // a `return(...)` payload would be silently ignored, so none is passed.
    fail::cfg("vault::atomic-write::write", "return").expect("fail-point cfg must succeed");

    const PRIOR: &[u8] = b"* TODO prior content survives\n";
    let dir = tempfile::TempDir::new().expect("TempDir must succeed");
    let target = dir.path().join("test.org");
    std::fs::write(&target, PRIOR).expect("seed prior content");

    let result = orgsidian_vault::atomic_write(&target, b"* TODO new content\n");

    assert!(
        matches!(result, Err(orgsidian_vault::VaultError::Io { .. })),
        "injected disk-full must surface immediately as non-transient Io \
         (never RetriesExhausted — ENOSPC must not be retried): {result:?}"
    );
    assert_eq!(
        std::fs::read(&target).expect("target must still be readable"),
        PRIOR,
        "prior content must be intact — no partial-write corruption"
    );
    let residue: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read_dir")
        .map(|e| e.expect("dir entry").file_name())
        .filter(|name| name != "test.org")
        .collect();
    assert!(
        residue.is_empty(),
        "no temp sibling may remain after the failed write: {residue:?}"
    );

    scenario.teardown();
}

#[test]
#[ignore = "implemented in Epic 3"]
fn config_corruption_fallback() {
    unimplemented!(
        "LD-41: Config corruption fallback — \
         see test-design.md §6.7 + Story 3.4 for real implementation"
    );
}

#[test]
#[ignore = "implemented in Epic 5"]
fn vault_folder_deleted_runtime() {
    unimplemented!(
        "LD-41: Vault folder deleted at runtime — \
         see test-design.md §6.7 + Story 5.1 for real implementation"
    );
}

#[test]
#[ignore = "implemented in Epic 1"]
fn plugin_init_panic_isolated() {
    unimplemented!(
        "LD-41: Plugin init() panic isolated — \
         see test-design.md §6.7 + LD-38 chaos plugin (future Epic 1 story) for real implementation"
    );
}

#[test]
#[ignore = "implemented in Epic 1"]
fn plugin_on_event_panic_isolated() {
    unimplemented!(
        "LD-41: Plugin on_event panic isolated — \
         see test-design.md §6.7 + LD-38 chaos plugin (future Epic 1 story) for real implementation"
    );
}

#[test]
#[ignore = "implemented in Epic 3"]
fn sqlite_index_corruption_rebuild() {
    unimplemented!(
        "LD-41: SQLite index corruption rebuild — \
         see test-design.md §6.7 + Story 3.7 for real implementation"
    );
}

#[test]
fn tmp_orphan_files_cleanup() {
    // Story 3.1: simulate a `kill -9` mid-write by planting a temp file
    // matching the real `atomic-write-file` 0.3.0 pattern (`.{name}.org.` +
    // 6 alphanumerics — no PID; see Story 3.1 AC5 deviation note) with a
    // backdated mtime, then assert `clean_orphan_temp_files` removes it while
    // legitimate files survive.
    let dir = tempfile::TempDir::new().expect("TempDir must succeed");

    let legit = dir.path().join("notes.org");
    std::fs::write(&legit, b"* TODO legitimate content\n").expect("seed legit file");

    let orphan = dir.path().join(".notes.org.aB3xY9");
    std::fs::write(&orphan, b"partial write from dead writer").expect("plant orphan");
    // Clear the 60s mtime-age guard (fresh in-flight temps are never raced).
    let stale = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
    std::fs::File::options()
        .write(true)
        .open(&orphan)
        .expect("open orphan")
        .set_times(std::fs::FileTimes::new().set_modified(stale))
        .expect("backdate orphan mtime");

    let report =
        orgsidian_vault::clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 1, "exactly the orphan is collected");
    assert!(!orphan.exists(), "orphan temp must be gone");
    assert!(legit.exists(), "legitimate file must survive");
}

#[test]
#[ignore = "implemented in Epic 5"]
fn external_delete_with_dirty_buffer() {
    unimplemented!(
        "LD-41: External delete with Dirty Buffer — \
         see test-design.md §6.7 + Story 5.5 for real implementation"
    );
}

#[test]
#[ignore = "implemented in Epic 11"]
fn refile_partial_completion_rollback() {
    unimplemented!(
        "LD-41: Refile partial completion rollback — \
         see test-design.md §6.7 + Story 11.8 for real implementation"
    );
}
