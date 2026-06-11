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
//! (see `tests/anchor.rs`) and is preserved across stories. The [`semantic`]
//! module (Story 2.3) lifts the raw tree into typed, owned semantic structs
//! via [`analyze`]; the round-trip serializer (Story 2.4, FR-2) builds on
//! both surfaces: [`serialize`] / [`serialize_document`] emit the retained
//! raw regions back, byte-identical to the analyzed source.

mod grammar;
pub mod semantic;
mod serializer;

use thiserror::Error;

pub use semantic::analyze;
pub use serializer::{serialize, serialize_document};

/// Re-export so downstream crates can name `orgsidian_parser::tree_sitter::Node`
/// (etc.) without taking their own `tree-sitter` dependency — the version pin
/// stays single-sourced through this leaf crate.
pub use tree_sitter;

/// Re-export so downstream crates can name the `chrono` date/time types
/// carried by [`semantic::Timestamp`] (`NaiveDate`/`NaiveTime`) without
/// taking their own `chrono` dependency — same single-sourced-pin rationale
/// as the [`tree_sitter`] re-export.
pub use chrono;

/// Parse result wrapping the raw [`tree_sitter::Tree`]. The newtype is the
/// stable API surface (LD-5 crate-API-barrier), but the wrapped tree is
/// deliberately reachable — [`tree`](ParseTree::tree) borrows it and the
/// [`tree_sitter`] re-export makes its full API nameable downstream. The raw
/// tree IS the contract: the pinned `tree-sitter` version is part of this
/// crate's public surface, and a `tree-sitter` major bump is a breaking
/// change for consumers of this crate.
///
/// Node byte-ranges resolve only against the **exact source** passed to
/// [`parse`] — keep it alive and byte-identical to read node text
/// (`node.utf8_text(source.as_bytes())`). A normalized or re-read copy yields
/// garbage spans or out-of-bounds slicing. `ParseTree` is `Send + Sync`
/// (`tree_sitter::Tree` carries both at the pinned 0.26.9; compile-asserted
/// in `tests/grammar.rs`).
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
    /// crate. In a correctly built crate this is unreachable — every
    /// [`parse`] call exercises the FFI link + ABI check at the pinned SHA
    /// (`tests/grammar.rs` keeps it covered); surfaced as an error rather
    /// than a panic to keep the parser panic-free.
    #[error("failed to load tree-sitter-org grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    /// `tree_sitter::Parser::parse` returned `None`. Only happens on a
    /// cancellation flag, a timeout, or a parser with no language set — none
    /// of which applies here (the wrapper sets neither flag and always calls
    /// `set_language` first), so this is defensive — but mapping it keeps
    /// `parse()` total.
    #[error("tree-sitter returned no tree")]
    NoTree,
}

/// Parse an org-mode source string into a raw syntax tree.
///
/// Empty input is valid org: `parse("")` returns `Ok` with an empty
/// `document` root. Malformed constructs surface as `ERROR`/`MISSING` nodes
/// *inside* a valid tree, not as `Err` — both [`ParseError`] arms are
/// defensive (see variant docs). Panic-free by contract.
///
/// Keep `source` to interpret the result: node ranges are byte offsets into
/// it (see [`ParseTree`]). tree-sitter addresses bytes as `u32`, so input
/// beyond 4 GiB is silently not lexed — the tree covers only the addressable
/// prefix. Far beyond any realistic `.org` file; documented for completeness.
pub fn parse(source: &str) -> Result<ParseTree, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::language())?;
    let tree = parser.parse(source, None).ok_or(ParseError::NoTree)?;
    Ok(ParseTree { tree })
}
