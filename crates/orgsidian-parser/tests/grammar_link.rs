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
}
