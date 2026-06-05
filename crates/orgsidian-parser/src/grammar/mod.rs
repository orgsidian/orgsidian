//! Internal tree-sitter-org grammar binding (Story 2.1, FR-1, LD-48).
//!
//! Implements FR-1.
//!
//! Re-exports the `extern "C" fn tree_sitter_org()` symbol produced by the
//! `build.rs` `cc` compile of `grammar/src/parser.c`. Story 2.2 consumes
//! `language()` to wire `parse(&str) -> Tree`. Story 2.1 itself does NOT
//! call `language()` — the binding is forward-compat only; the
//! anti-placebo-green smoke at `tests/grammar_link.rs` exercises the
//! symbol-link path.

unsafe extern "C" {
    fn tree_sitter_org() -> tree_sitter::Language;
}

/// Get the tree-sitter [`Language`][tree_sitter::Language] for the vendored
/// `nvim-orgmode/tree-sitter-org` grammar. Internal; Story 2.2 promotes to
/// `pub` if the public parse() wrapper needs to expose it (current
/// expectation is that it does not — `parse()` consumes `language()`
/// internally only).
pub(crate) fn language() -> tree_sitter::Language {
    // SAFETY: `tree_sitter_org()` is a thread-safe FFI constructor produced
    // by `tree-sitter generate` (deterministic, no global mutable state);
    // upstream tree-sitter-* crates ship this pattern verbatim.
    unsafe { tree_sitter_org() }
}
