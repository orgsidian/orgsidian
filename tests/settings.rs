//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Traceability grep-smoke per Story 1.18 AC8 / T13.
//! Asserts the `LD-40 + FR-23` trace annotation appears in ≥6 files under
//! `crates/orgsidian-core/src/settings/`, mirroring Story 1.17's grep-smoke
//! pattern. Anti-placebo: protects the FR-Traceability discipline against
//! accidental doc-comment deletions during refactor.

use std::fs;
use std::path::{Path, PathBuf};

const TRACE_NEEDLE: &str = "LD-40 + FR-23";
const MIN_FILES_WITH_TRACE: usize = 6;

fn settings_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("src/settings")
        .canonicalize()
        .unwrap_or_else(|e| panic!("settings/ directory must exist: {e}"))
}

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read settings/ dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn ld40_fr23_trace_appears_in_at_least_six_files() {
    let dir = settings_dir();
    let mut files = Vec::new();
    walk_rs_files(&dir, &mut files);

    let mut hits: Vec<PathBuf> = Vec::new();
    for path in &files {
        let raw = fs::read_to_string(path).expect("read .rs file");
        if raw.contains(TRACE_NEEDLE) {
            hits.push(path.clone());
        }
    }

    assert!(
        hits.len() >= MIN_FILES_WITH_TRACE,
        "expected at least {MIN_FILES_WITH_TRACE} files under settings/ to contain {TRACE_NEEDLE:?}; \
         found {} of {}:\nfiles checked: {:#?}\nfiles with trace: {:#?}",
        hits.len(),
        files.len(),
        files,
        hits,
    );
}
