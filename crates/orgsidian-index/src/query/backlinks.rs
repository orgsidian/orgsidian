//! Implements FR-13 (v0.1 `IndexQuery` baseline — Story 8.6 Backlinks +
//! Story 12.0 v0.5+ Unlinked References).
//!
//! Story 8.6 will implement [`for_headline`]: every other Headline that
//! references a given Headline via an `id:` link or a `[[wiki-link]]`.
//! Story 12.0 (v0.5+) will implement [`unlinked_mentions`]: places where a
//! Headline's title textually appears elsewhere without a formal link. Both
//! signatures are frozen now (Story 6.5) so Story 12.0 lands as a
//! semver-minor body addition rather than a new-method shape change late in
//! the roadmap — the whole point of the 2026-05-20 "freeze upfront"
//! reconciliation this story implements.

use std::path::PathBuf;

use rusqlite::Connection;

use super::HeadlineId;
use crate::error::IndexError;

/// One Backlink: another Headline that references the queried one, via
/// `id:` link or `[[wiki-link]]`, plus a short context snippet (one line of
/// surrounding source text) per the Story 8.6 AC.
///
/// `#[non_exhaustive]`: Story 8.6 may need to distinguish `id:`-link from
/// `[[wiki-link]]` backlinks (mirroring [`super::graph::EdgeKind`]) once it
/// is actually built.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct Backlink {
    /// The identity of the Headline that CONTAINS the reference (the
    /// linking Headline, not the one being linked to).
    pub headline_id: HeadlineId,
    /// `files.path` of the linking Headline's source file.
    pub file: PathBuf,
    /// The linking Headline's title.
    pub title: String,
    /// One line of source text surrounding the `id:`/`[[wiki-link]]`
    /// reference.
    pub context_snippet: String,
}

/// Every Headline linking to `headline_id` via `id:` or `[[wiki-link]]`.
///
/// **Story 8.6 stub**: this is the v0.1 `IndexQuery` baseline signature
/// (Story 6.5), frozen ahead of its Story 8.6 implementation. The body
/// below returns an empty `Vec` rather than `unimplemented!()` so it is
/// safely reachable through [`super::IndexQuery::backlinks_for_headline`]'s
/// default method before Story 8.6 lands.
///
/// # Errors
///
/// [`IndexError::Sqlite`] once implemented; the stub body never errors.
pub fn for_headline(
    conn: &Connection,
    headline_id: HeadlineId,
) -> Result<Vec<Backlink>, IndexError> {
    // Story 8.6 TODO: implement the `links` table lookup (both `id:` and
    // `[[wiki-link]]` reference kinds). Unused until then.
    let _ = conn;
    let _ = headline_id;
    Ok(Vec::new())
}

/// One unlinked mention: a place where `headline_id`'s title appears in
/// another Headline's body text without a formal `id:`/`[[wiki-link]]`
/// reference — one entry per linking Headline (deduplicated by Headline, not
/// one per textual occurrence), per the Story 12.0 AC.
///
/// `#[non_exhaustive]`: Story 12.0 may need a match-kind field
/// (whole-word vs. substring) once its FTS5 matching mode is configurable.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct UnlinkedMention {
    /// `files.path` of the mentioning Headline's source file.
    pub file: PathBuf,
    /// The identity of the Headline that mentions the queried one.
    pub headline_id: HeadlineId,
    /// The mentioning Headline's title.
    pub headline_title: String,
    /// One line of source text surrounding the textual mention.
    pub context_snippet: String,
}

/// Every Headline whose body text mentions `headline_id`'s title without a
/// formal link — the FTS5-plus-outer-join query the Story 12.0 AC describes.
///
/// **Story 12.0 (v0.5+) stub**: this is the v0.1 `IndexQuery` baseline
/// signature (Story 6.5), frozen well ahead of its Story 12.0
/// implementation — per the 2026-05-20 reconciliation's explicit rationale:
/// "signature in baseline so v0.5 lands as semver-minor body addition, not
/// breaking". The body below returns an empty `Vec` rather than
/// `unimplemented!()` so it is safely reachable through
/// [`super::IndexQuery::backlinks_unlinked_mentions`]'s default method long
/// before Story 12.0 lands.
///
/// # Errors
///
/// [`IndexError::Sqlite`] once implemented; the stub body never errors.
pub fn unlinked_mentions(
    conn: &Connection,
    headline_id: HeadlineId,
) -> Result<Vec<UnlinkedMention>, IndexError> {
    // Story 12.0 TODO: implement the FTS5 title-text query outer-joined
    // against `links`, excluding the source Headline and any Headline
    // already formally linked. Unused until then.
    let _ = conn;
    let _ = headline_id;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::apply_schema(&mut conn).expect("apply schema");
        conn
    }

    #[test]
    fn for_headline_stub_returns_empty_without_erroring() {
        let conn = open_test_db();
        let result = for_headline(&conn, HeadlineId(1)).expect("stub must not error");
        assert!(result.is_empty());
    }

    #[test]
    fn unlinked_mentions_stub_returns_empty_without_erroring() {
        let conn = open_test_db();
        let result = unlinked_mentions(&conn, HeadlineId(1)).expect("stub must not error");
        assert!(result.is_empty());
    }
}
