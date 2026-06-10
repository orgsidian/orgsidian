//! Story 2.2 — grammar wrapper test (FR-1). Replaces the Story 2.1
//! `tests/grammar_link.rs` FFI-link smoke: the real `parse()` call now
//! exercises the same cc-compile + extern-"C" symbol-link path end-to-end
//! (set_language + parse), so the dedicated link smoke is subsumed.

/// A ≥10-line representative org sample: heading levels with TODO states,
/// a PROPERTIES drawer, SCHEDULED/DEADLINE, inline markup, and a link.
const SAMPLE: &str = "\
#+TITLE: Sample
* TODO Top heading :work:
SCHEDULED: <2026-06-10 Wed>
:PROPERTIES:
:ID: abc-123
:END:
Some *bold* and /italic/ text with a [[id:abc-123][link]].
** DONE Sub heading
DEADLINE: <2026-06-12 Fri>
- [ ] a checkbox item
- [X] a done item
";

#[test]
fn parse_returns_document_root() {
    let tree = orgsidian_parser::parse(SAMPLE).expect("representative sample must parse");
    let root = tree.root_node();
    assert_eq!(
        root.kind(),
        "document",
        "tree-sitter-org root node must be `document`"
    );
    assert!(
        root.child_count() > 0,
        "sample must produce children under the root, not a bare `document`"
    );
    assert_eq!(
        root.end_byte(),
        SAMPLE.len(),
        "the grammar must consume the whole sample"
    );
    assert!(
        !root.has_error(),
        "the well-formed sample must parse without ERROR/MISSING nodes"
    );
}

#[test]
fn parse_empty_input_is_ok() {
    // Story 2.2 behavior change: empty `.org` is valid, parses to an empty
    // `document` (Story 1.9 stub returned Err(Empty); see story AC4).
    let tree = orgsidian_parser::parse("").expect("empty source is a valid empty document");
    let root = tree.root_node();
    assert_eq!(root.kind(), "document");
    assert_eq!(
        root.child_count(),
        0,
        "empty input must yield zero children"
    );
}

#[test]
fn parse_error_display_is_wired() {
    // Exercises the thiserror Display derivation — both `ParseError` arms are
    // defensive and never constructed by `parse()` in a correctly built crate.
    assert_eq!(
        orgsidian_parser::ParseError::NoTree.to_string(),
        "tree-sitter returned no tree"
    );
}

#[test]
fn parse_tree_is_send_and_sync() {
    // Compile-time guard: `tree_sitter::Tree` is `Send + Sync` at the pinned
    // 0.26.9, so `ParseTree` must be too. A future tree-sitter bump dropping
    // either auto-trait should fail here, not in a downstream crate.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<orgsidian_parser::ParseTree>();
}
