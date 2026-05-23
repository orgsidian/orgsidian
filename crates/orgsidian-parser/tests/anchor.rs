//! Story 1.9 — parser anchor smoke (anti-placebo-green per Party Mode P2).
//!
//! Exercises a real `orgsidian_parser::parse(&str)` call against the trivial
//! fixture `* TODO Hello\n`. Must keep passing across the Story 2.2 swap that
//! replaces the stub with the tree-sitter-org-backed implementation.

use std::path::PathBuf;

#[test]
fn parse_anchor_fixture_succeeds() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("anchor.org");
    let source = std::fs::read_to_string(&fixture_path)
        .expect("anchor fixture must be readable from CARGO_MANIFEST_DIR");
    let result = orgsidian_parser::parse(&source);
    result.expect("anchor fixture must parse");
}
