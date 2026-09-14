//! FR → implementing-module traceability grep-smoke.
//!
//! The project's traceability convention (Epic 7 context / Story 7.1): each
//! implementing module carries a `//! Implements FR-N` first doc-comment line.
//! This test asserts that line is present, verbatim, as the FIRST line of each
//! listed module — an anti-placebo guard against a doc-comment being dropped or
//! reworded during a refactor, which would silently break the FR → code map.
//!
//! Created by Story 7.1 (the first story to need it); kept a small table so
//! later FRs extend `TRACE_TABLE` with one row each rather than writing a new
//! test.
//!
//! Lives at the workspace `tests/` root (a cross-cutting artifact, like
//! `settings.rs`) and is pointed at by an explicit `[[test]]` in
//! `crates/orgsidian-core/Cargo.toml`, so `CARGO_MANIFEST_DIR` is that crate —
//! paths below are resolved relative to the workspace root two levels up.

use std::fs;
use std::path::PathBuf;

/// `(fr, module_path_relative_to_workspace_root, expected_first_doc_line_needle)`.
/// The needle must appear on the module's FIRST line.
const TRACE_TABLE: &[(&str, &str, &str)] = &[(
    "FR-6",
    "crates/orgsidian-index/src/query/dashboard.rs",
    "//! Implements FR-6",
)];

/// The workspace root — two levels up from this test's host crate manifest
/// (`crates/orgsidian-core`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must resolve from CARGO_MANIFEST_DIR")
}

#[test]
fn each_fr_module_declares_its_trace_on_the_first_doc_line() {
    let root = workspace_root();
    for (fr, rel_path, needle) in TRACE_TABLE {
        let path = root.join(rel_path);
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{fr}: module {} must be readable: {e}", path.display()));
        let first_line = contents.lines().next().unwrap_or("");
        assert!(
            first_line.contains(needle),
            "{fr}: expected the FIRST line of {} to contain {needle:?}, but it was {first_line:?}",
            path.display(),
        );
    }
}
