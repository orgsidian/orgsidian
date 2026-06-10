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
    assert_eq!(
        tree.root_node().kind(),
        "document",
        "tree-sitter-org root node must be `document`"
    );
}

#[test]
fn parse_empty_input_is_ok() {
    // Story 2.2 behavior change: empty `.org` is valid, parses to an empty
    // `document` (Story 1.9 stub returned Err(Empty); see story AC4).
    let tree = orgsidian_parser::parse("").expect("empty source is a valid empty document");
    assert_eq!(tree.root_node().kind(), "document");
}
