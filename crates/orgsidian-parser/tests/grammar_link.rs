//! Story 2.1 — grammar FFI link smoke (anti-placebo-green per LD-48 + Party
//! Mode P2 anchor convention). Without this test, a regression that breaks
//! the cc compile or the extern "C" symbol-link would still pass
//! `cargo build -p orgsidian-parser` if no other module references the
//! symbol — build.rs only emits the object file; the link path is only
//! exercised when something actually calls `language()`.
//!
//! Story 2.2 replaces this smoke with the real `parse()` body test against
//! a tree-sitter `Tree`. The internal `grammar::language()` accessor is
//! `pub(crate)`; this test reaches it via the `#[doc(hidden)]`
//! `_language_for_smoke` shim re-exported at the bottom of
//! `crates/orgsidian-parser/src/lib.rs` (E0364 forbids `pub use` of a
//! `pub(crate)` item directly, hence the wrapper function).

#[test]
fn grammar_language_symbol_links() {
    // Resolve the FFI symbol via the test-only shim. The mere call proves:
    // (a) build.rs ran, (b) cc compiled parser.c + scanner.c,
    // (c) the extern "C" symbol is reachable from Rust.
    let language = orgsidian_parser::_language_for_smoke();
    // tree-sitter Language exposes `abi_version()` (0.26.x renamed from
    // `version()`); calling it confirms the returned handle is a real
    // Language struct, not a null/garbage value.
    assert!(
        language.abi_version() > 0,
        "tree-sitter-org Language must have positive ABI version"
    );
    // Stronger guard: `Parser::set_language` performs the ABI compatibility
    // check between the grammar's compiled version and the host tree-sitter
    // crate's supported range. A grammar compiled by a stale generator
    // returning an ABI outside the host's supported window passes the
    // `abi_version() > 0` smoke but fails here — which is precisely the
    // regression the Story 2.2 `Parser::set_language(&language)?` call would
    // hit downstream, surfacing it now.
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .expect("tree-sitter-org Language must be ABI-compatible with host tree-sitter crate");
}
