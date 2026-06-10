//! Internal tree-sitter-org grammar binding (Story 2.1, FR-1, LD-48).
//!
//! Implements FR-1.
//!
//! Re-exports the `extern "C" fn tree_sitter_org()` symbol produced by the
//! `build.rs` `cc` compile of `grammar/src/parser.c`. Story 2.2 consumes
//! `language()` internally from `parse()`; it stays `pub(crate)` — the
//! public surface is `ParseTree` + accessors, not the raw FFI handle.

unsafe extern "C" {
    fn tree_sitter_org() -> tree_sitter::Language;
}

/// Get the tree-sitter [`Language`][tree_sitter::Language] for the vendored
/// `nvim-orgmode/tree-sitter-org` grammar. Internal; Story 2.2 consumes
/// `language()` internally from `parse()` — stays `pub(crate)`.
pub(crate) fn language() -> tree_sitter::Language {
    // SAFETY: `tree_sitter_org()` is a thread-safe FFI constructor produced
    // by `tree-sitter generate` (deterministic, no global mutable state);
    // upstream tree-sitter-* crates ship this pattern verbatim.
    unsafe { tree_sitter_org() }
}
