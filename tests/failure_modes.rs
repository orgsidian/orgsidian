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
#[ignore = "implemented in Epic 3"]
fn disk_full_atomic_write() {
    // Future implementation (Epic 3 / Story 3.1):
    //
    // let _scenario = fail::FailScenario::setup();
    // fail::cfg("atomic-write::after-tmp-rename", "panic").unwrap();
    // let vault = test_vault();
    // let result = vault.save_file("test.org", "content");
    // assert!(result.is_err());
    // assert!(!vault.path().join("test.org").exists());
    //
    // FailScenario teardown is automatic on drop.
    unimplemented!(
        "LD-41: Disk full / ENOSPC during atomic write — \
         see test-design.md §6.7 + Story 3.1 for real implementation"
    );
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
#[ignore = "implemented in Epic 3"]
fn tmp_orphan_files_cleanup() {
    unimplemented!(
        "LD-41: .tmp orphan files cleanup — \
         see test-design.md §6.7 + Story 3.1 for real implementation"
    );
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
