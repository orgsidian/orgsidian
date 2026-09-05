//! Implements FR-12 (v0.1 `IndexQuery` baseline — Story 8.4 two-tier FTS5
//! search).
//!
//! Story 8.4 will implement the full-text search query engine (plain words,
//! `"exact phrase"`, `#tag:value`, `file:path-glob`, `todo:STATE`) over the
//! two FTS5 external-content tables `SCHEMA_SQL` already declares. Story 6.5
//! freezes the two-entry-point contract that story must satisfy — per the
//! 2026-05-20 two-tier reconciliation — ahead of its implementation:
//!
//! - [`query`]: a full batch, up to 50 results (`≤200ms` initial perf
//!   baseline on a 1000-file Vault).
//! - [`search_stream`]: the same search, streamed — the first 10 results
//!   yieldable before the full batch completes (`≤100ms` initial perf
//!   baseline for time-to-first-10). `rusqlite::Statement::query_map` is the
//!   natural backing primitive per the Story 8.4 AC.
//!
//! Both are stub bodies today (see the parent module's docs, "Why stub
//! bodies return empty results") — Story 8.4 replaces them with the real
//! FTS5 query, a body change against this already-frozen signature.

use std::path::PathBuf;

use rusqlite::Connection;

use super::HeadlineId;
use crate::error::IndexError;

/// A search query — a raw, unparsed query string today. Wrapped in a
/// `#[non_exhaustive]` struct (rather than a bare `&str` parameter) so Story
/// 8.4 can grow it — e.g. pre-parsed structured fields alongside the raw
/// text — without changing [`query`]/[`search_stream`]'s signature.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct SearchQuery {
    /// The raw query text, exactly as typed by the user (plain words,
    /// `"exact phrase"`, `#tag:value`, `file:path-glob`, `todo:STATE` —
    /// Story 8.4 owns the parser).
    pub text: String,
}

/// One search hit: the matching Headline's identity plus a one-line preview
/// of the matched source text, grouped by file per the Story 8.4 AC.
///
/// `#[non_exhaustive]`: Story 8.4 may need to add a relevance score or a
/// matched-field marker once the FTS5 ranking is tuned.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct SearchResult {
    /// The matching Headline's identity.
    pub headline_id: HeadlineId,
    /// `files.path` — the grouping key, mirroring [`super::agenda::AgendaItem::file_path`].
    pub file: PathBuf,
    /// Headline title, stars/keyword/tags already stripped.
    pub title: String,
    /// One line of source text surrounding (or containing) the match.
    pub matched_line: String,
}

/// Full-batch search: up to 50 [`SearchResult`]s, grouped by file.
///
/// **Story 8.4 stub**: this is the v0.1 `IndexQuery` baseline signature
/// (Story 6.5), frozen ahead of its Story 8.4 implementation. The body
/// below returns an empty `Vec` rather than `unimplemented!()` so it is
/// safely reachable through [`super::IndexQuery::search`]'s default method
/// before Story 8.4 lands.
///
/// # Errors
///
/// [`IndexError::Sqlite`] once implemented; the stub body never errors.
pub fn query(
    conn: &Connection,
    search_query: &SearchQuery,
) -> Result<Vec<SearchResult>, IndexError> {
    // Story 8.4 TODO: implement the FTS5 query-DSL parser + full-batch
    // search over the two external-content FTS5 tables. Unused until then.
    let _ = conn;
    let _ = search_query;
    Ok(Vec::new())
}

/// Streaming search: the same search as [`query`], but yielding results
/// incrementally so the first 10 can paint before the full batch of up to
/// 50 completes (Story 8.4's two-tier perf contract: `≤100ms` to first 10,
/// `≤200ms` to full 50, on a 1000-file Vault).
///
/// Returns `impl Iterator` (opaque) rather than a named iterator type: Story
/// 8.4's real implementation will back this with a `rusqlite::Statement`
/// query-map iterator, an implementation detail the frozen signature does
/// not commit to.
///
/// **Story 8.4 stub**: frozen signature, pending body — see [`query`]'s docs
/// for the rationale.
///
/// # Errors
///
/// [`IndexError::Sqlite`] once implemented; the stub body never errors.
pub fn search_stream<'conn>(
    conn: &'conn Connection,
    search_query: &SearchQuery,
) -> Result<impl Iterator<Item = SearchResult> + 'conn, IndexError> {
    // Story 8.4 TODO: implement the streaming FTS5 query-map iterator.
    // Unused until then; `conn`'s lifetime is threaded through the return
    // type now so the real implementation has room to borrow it.
    let _ = conn;
    let _ = search_query;
    Ok(std::iter::empty())
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
    fn query_stub_returns_empty_without_erroring() {
        let conn = open_test_db();
        let result = query(
            &conn,
            &SearchQuery {
                text: "kubernetes ingress".to_string(),
            },
        )
        .expect("stub must not error");
        assert!(result.is_empty());
    }

    #[test]
    fn search_stream_stub_yields_nothing_without_erroring() {
        let conn = open_test_db();
        let mut iter = search_stream(
            &conn,
            &SearchQuery {
                text: "kubernetes ingress".to_string(),
            },
        )
        .expect("stub must not error");
        assert!(iter.next().is_none());
    }
}
