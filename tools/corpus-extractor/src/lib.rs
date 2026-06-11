//! Corpus extractor library (Story 2.5, LD-44 / OD-1).
//!
//! Extracts the L0 round-trip subset (~100 files) and the full nightly corpus
//! from the GNU org-mode test suite (`testing/lisp/test-org-element.el`,
//! pinned-fetch — never vendored; see `fetch` module + ADR 0001).
//!
//! NOTE: deliberately NO `Implements FR-` header — the FR-2 trace is owned by
//! `crates/orgsidian-parser/src/serializer.rs` (single trace owner per FR,
//! CONTRIBUTING §4); this crate is a maintainer tool, not an FR implementer.

pub mod classify;
pub mod elisp;
pub mod emit;
pub mod fetch;
pub mod model;
pub mod select;
pub mod synth;
pub mod validate;
