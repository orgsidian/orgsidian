//! orgsidian-parser: tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2).
//!
//! Implements FR-1.
//!
//! Wraps the vendored `nvim-orgmode/tree-sitter-org` grammar (SHA-pinned
//! submodule at `grammar/`, Story 2.1) behind a stable raw-syntax-tree API:
//! [`parse`] turns `.org` source into a [`ParseTree`] carrying a real
//! [`tree_sitter::Tree`], reachable via [`ParseTree::root_node`] /
//! [`ParseTree::tree`]. The public signature
//! `parse(&str) -> Result<ParseTree, ParseError>` is the anchor sentinel
//! (see `tests/anchor.rs`) and is preserved across stories. The semantic
//! layer (Story 2.3) and the round-trip serializer (Story 2.4, FR-2) build
//! on this surface.

mod grammar;

use thiserror::Error;

/// Re-export so downstream crates can name `orgsidian_parser::tree_sitter::Node`
/// (etc.) without taking their own `tree-sitter` dependency — the version pin
/// stays single-sourced through this leaf crate.
pub use tree_sitter;

/// Opaque parse result wrapping the raw [`tree_sitter::Tree`]. The newtype is
/// the stable API surface (LD-5 crate-API-barrier); the wrapped tree is an
/// implementation detail consumers reach through accessors only.
#[derive(Debug)]
pub struct ParseTree {
    tree: tree_sitter::Tree,
}

impl ParseTree {
    /// Root node of the parsed tree. For any `.org` source the root kind is
    /// `document` (the tree-sitter-org root rule).
    pub fn root_node(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }

    /// Borrow the raw [`tree_sitter::Tree`] for walking / cursors / queries
    /// (Story 2.3 semantic-layer consumption path).
    pub fn tree(&self) -> &tree_sitter::Tree {
        &self.tree
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    /// The vendored grammar's ABI is incompatible with the host `tree-sitter`
    /// crate. In a correctly built crate this is unreachable (Story 2.1's
    /// `grammar_link` smoke proved ABI-compat at the pinned SHA); surfaced as
    /// an error rather than a panic to keep the parser panic-free.
    #[error("failed to load tree-sitter-org grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    /// `tree_sitter::Parser::parse` returned `None`. Only happens on a
    /// cancellation flag / timeout, neither of which this wrapper sets, so
    /// this is defensive — but mapping it keeps `parse()` total.
    #[error("tree-sitter returned no tree")]
    NoTree,
}

/// Parse an org-mode source string into a raw syntax tree.
///
/// Empty input is valid org: `parse("")` returns `Ok` with an empty
/// `document` root. Malformed constructs surface as `ERROR`/`MISSING` nodes
/// *inside* a valid tree, not as `Err` — both [`ParseError`] arms are
/// defensive (see variant docs). Panic-free by contract.
pub fn parse(source: &str) -> Result<ParseTree, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::language())?;
    let tree = parser.parse(source, None).ok_or(ParseError::NoTree)?;
    Ok(ParseTree { tree })
}
