//! orgsidian-parser: tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2).
//!
//! Story 1.9 ships the anchor-smoke surface only — `parse()` is a stub that returns
//! `Ok` for any non-empty UTF-8 source. Story 2.2 wires the real tree-sitter-org
//! grammar and replaces this body; the public signature
//! `parse(&str) -> Result<ParseTree, ParseError>` is preserved across that
//! replacement (anchor sentinel discipline — see `tests/anchor.rs`).

mod grammar;

use thiserror::Error;

/// Opaque parse result. Story 1.9 ships a sealed unit-content marker; Story 2.2
/// replaces `_private: ()` with the real fields (`headlines: Vec<Headline>`, …).
#[derive(Debug)]
pub struct ParseTree {
    _private: (),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("empty source")]
    Empty,
}

/// Parse an org-mode source string.
///
/// Story 1.9 stub: validates the input is non-empty UTF-8 (the `&str` bound already
/// enforces UTF-8; the explicit check is structural to remind a future reader that
/// the tree-sitter-org input contract is UTF-8). Returns `Err(ParseError::Empty)`
/// for empty input, `Ok(ParseTree)` otherwise.
pub fn parse(source: &str) -> Result<ParseTree, ParseError> {
    // UTF-8 contract witness (AC4): the `&str` bound already guarantees UTF-8;
    // this binding makes the contract visible in the body so a future
    // tree-sitter-org swap (Story 2.2) keeps the same input invariant.
    let _utf8_source: &str = source;
    if source.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(ParseTree { _private: () })
}

// Story 2.1 (AC4): integration-test-only shim so tests/grammar_link.rs can
// exercise the FFI symbol-link path without promoting `grammar::language` to a
// stable public API. `grammar::language` stays `pub(crate)` (re-exporting it
// directly is forbidden by E0364); the `#[doc(hidden)]` + `_`-prefix shim is
// the standard "internal escape hatch" pattern. `#[cfg(test)]` would NOT work
// because integration tests link against the library compiled without `--test`.
// Story 2.2 removes this when `parse()` consumes `language()` directly.
#[doc(hidden)]
pub fn _language_for_smoke() -> tree_sitter::Language {
    grammar::language()
}
