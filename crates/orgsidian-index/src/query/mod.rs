//! Domain read queries over the derived index (FR-7 Agenda; FR-12 Search and
//! FR-13 Backlinks land in Epic 8).
//!
//! Story 6.5 freezes this module's public surface as the `IndexQuery` trait +
//! `AgendaQuery`/`SearchQuery`/`BacklinksQuery` types (v0.1 baseline,
//! `cargo-semver-checks`-enforced from then on). Until that freeze lands, each
//! submodule ships the plain function its own story needs — [`agenda::today`]
//! (Story 6.3) and [`agenda::week`] (Story 6.4) are the first two — so the
//! trait Story 6.5 writes wraps working, already-tested query bodies rather
//! than inventing the shape from scratch.

pub mod agenda;
