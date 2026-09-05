//! Domain read queries over the derived index (FR-7 Agenda; FR-12 Search;
//! FR-13 Backlinks; FR-26 Graph View).
//!
//! Freezes LD-32 / LD-33 / NFR-19: this module — the [`IndexQuery`] trait
//! plus its four submodules ([`agenda`], [`search`], [`backlinks`],
//! [`graph`]) — IS the v0.1 `orgsidian-index::query` public API surface,
//! `cargo-semver-checks`-enforced from this commit onward
//! (`.github/workflows/pr.yml`, Story 6.5). Per the 2026-05-20
//! reconciliation, the surface is frozen **upfront** rather than grown
//! incrementally through Epic 7/8/12, so those epics land as semver-MINOR
//! body additions to already-declared signatures instead of risking a
//! breaking shape change late in the roadmap.
//!
//! Stories 6.3/6.4 already shipped [`agenda::today`]/[`agenda::week`] as
//! plain, tested functions; this story does not touch their signatures or
//! bodies. It adds:
//!
//! - the sibling stub signatures later stories fill in — [`agenda::custom`]
//!   (Story 7.4), [`search::query`]/[`search::search_stream`] (Story 8.4),
//!   [`backlinks::for_headline`] (Story 8.6),
//!   [`backlinks::unlinked_mentions`] (Story 12.0), [`graph::adjacency`]
//!   (Story 8.10) — each a real, frozen signature with a body that returns
//!   an empty result (see "Why stub bodies return empty, never
//!   `unimplemented!()`" below);
//! - the [`IndexQuery`] trait, wrapping the whole set as default-bodied
//!   methods over an explicit `&Connection` parameter (so a caller can reach
//!   every query through one `impl IndexQuery` value — [`DefaultIndexQuery`]
//!   — instead of importing eight free functions one by one). The free
//!   functions stay the canonical, directly-callable entry points; nothing
//!   about `orgsidian-core`'s existing `agenda::today`/`agenda::week` call
//!   sites changes.
//!
//! # Why `cargo-semver-checks` freezing "works" even though four of the
//! eight query bodies do not exist yet
//!
//! `cargo-semver-checks` (like SemVer itself) only inspects *signatures* —
//! parameter/return types, trait method sets, `#[non_exhaustive]` markers —
//! never function bodies. Shipping the real Story 7.4/8.4/8.6/8.10/12.0
//! signature now, with a body that returns an empty `Vec`/iterator (never a
//! function that can panic a caller who reaches it through the
//! [`IndexQuery`] default method before its story lands), means each
//! eventual real implementation is a pure body edit: `cargo-semver-checks`
//! sees **no API change at all** when, say, Story 7.4 replaces
//! `Ok(Vec::new())` with a real `SELECT`. That is what makes "Epic 7/8/12
//! land as semver-MINOR additions, not breaking changes" true by
//! construction rather than by discipline alone.
//!
//! # `#[non_exhaustive]` policy
//!
//! Every NEW struct/enum this story introduces ([`agenda::CustomAgendaQuery`],
//! [`search::SearchQuery`], [`search::SearchResult`], [`backlinks::Backlink`],
//! [`backlinks::UnlinkedMention`], [`graph::GraphScope`], [`graph::EdgeKind`],
//! [`graph::NodeRef`], [`graph::Edge`], [`graph::GraphData`]) is
//! `#[non_exhaustive]`: a downstream crate can read fields / match variants
//! but cannot construct-by-literal or exhaustively-match, so a later story
//! can add a field (e.g. `SearchQuery` growing a parsed `tag`/`file`/`todo`
//! filter once Story 8.4's query-DSL parser lands) or a variant
//! (`GraphScope::Tag`/`::FilePath`, reserved for v0.5+ per the Story 8.10 AC)
//! without that being a breaking change.
//!
//! `AgendaItem` (Stories 6.3/6.4, already shipped before this freeze lands)
//! is deliberately left exactly as-is: retroactively marking it
//! `#[non_exhaustive]` in the very commit that adds the freeze gate would
//! itself be a shape change the new gate would flag against the pre-freeze
//! baseline. Its own growth path (a deliberate future field addition) is the
//! AC's documented major-bump escape hatch: "semver-major changes... require
//! explicit CHANGELOG bump + reviewer override."
//!
//! # Why stub bodies return empty results, never `unimplemented!()`
//!
//! [`IndexQuery`]'s default methods mean the stub free functions are
//! reachable through a single trait object/impl before their owning story
//! lands. A body that can panic would turn "the API shape exists" into "the
//! API shape exists but is a trap" — worse than not having the surface at
//! all. Every stub therefore returns `Ok(Vec::new())` (or, for
//! [`search::search_stream`], `Ok(std::iter::empty())`) and documents the
//! owning story inline.
//!
//! # `HeadlineId`
//!
//! [`HeadlineId`] is a small `i64` newtype introduced by this freeze so the
//! new Backlinks/Search/Graph types have one typed identity rather than a
//! bare `i64` (self-documenting at call sites: `for_headline(HeadlineId(id))`
//! vs. `for_headline(id)`). It is deliberately a plain (non-`#[non_exhaustive]`)
//! tuple struct with a `pub` field: an opaque row-id wrapper has no plausible
//! future field growth, and callers across crate boundaries (CLI arg
//! parsing, the editor's cursor-headline lookup) need to construct one
//! directly. `AgendaItem::headline_id` (Stories 6.3/6.4, already shipped as a
//! bare `i64`) is left untouched for the same reason `AgendaItem` itself is
//! left untouched above — this story freezes what already exists, it does
//! not retrofit it.
//!
//! # Note on `IndexQuery` and `dyn` dispatch
//!
//! [`IndexQuery::search_stream`] returns `impl Iterator<..>` (a
//! return-position-impl-trait-in-traits method, stable since Rust 1.75) —
//! its concrete return type is opaque and therefore not nameable in a
//! vtable, so `IndexQuery` is NOT object-safe (`dyn IndexQuery` does not
//! compile). Nothing in the codebase needs `dyn IndexQuery` today; a caller
//! wanting one polymorphic value uses `impl IndexQuery` (generic) or the
//! concrete [`DefaultIndexQuery`] / the free functions directly.

use rusqlite::Connection;

use crate::error::IndexError;

pub mod agenda;
pub mod backlinks;
pub mod graph;
pub mod search;

/// A `headlines.id` row identity, typed so the Backlinks/Search/Graph
/// surfaces this story freezes never pass a bare `i64` for "which headline".
///
/// Deliberately NOT `#[non_exhaustive]` — see the module docs' `HeadlineId`
/// section for why a single-field id newtype has no growth path worth
/// protecting, and every crate boundary needs to construct one directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct HeadlineId(pub i64);

impl From<i64> for HeadlineId {
    fn from(id: i64) -> Self {
        HeadlineId(id)
    }
}

impl From<HeadlineId> for i64 {
    fn from(id: HeadlineId) -> Self {
        id.0
    }
}

/// The v0.1 frozen `orgsidian-index` query surface (Story 6.5, LD-32 /
/// LD-33 / NFR-19), wrapping [`agenda`], [`search`], [`backlinks`], and
/// [`graph`] as one trait of default-bodied methods.
///
/// Every method takes its `&Connection` explicitly (rather than requiring
/// `Self` to BE a connection) so a default body can call the matching
/// submodule free function without constraining what `Self` is — the
/// trait's only real implementor today is the zero-sized
/// [`DefaultIndexQuery`], but the shape leaves room for a future test double
/// that swaps in different query logic per method, overriding only the ones
/// it cares about.
///
/// # Contract
///
/// - **Input types.** A caller-supplied `&Connection` (LD-14 reader-pool
///   connection) plus a query-specific parameter: a plain `&str` calendar
///   day for the two already-shipped Agenda queries (see
///   [`agenda::today`]/[`agenda::week`] for why these are caller-supplied
///   strings, never a server-side clock read); a `#[non_exhaustive]`
///   parameter struct or a [`HeadlineId`] for every other query, so later
///   stories can grow the parameter shape (new filter fields, new enum
///   variants) without a breaking signature change.
/// - **Return types.** `Result<Vec<T>, IndexError>` for every query except
///   [`search_stream`](IndexQuery::search_stream) (`Result<impl
///   Iterator<Item = search::SearchResult>, IndexError>`, per the Story 8.4
///   two-tier streaming contract) and
///   [`graph_adjacency`](IndexQuery::graph_adjacency) (`Result<GraphData,
///   IndexError>` — a single adjacency-list value, not a `Vec`). Every `T`
///   is a `#[non_exhaustive]` struct (see the module docs) except
///   `agenda::AgendaItem`, frozen as shipped by Stories 6.3/6.4.
/// - **Error variants.** [`IndexError::Sqlite`] for every query that already
///   touches SQLite ([`agenda::today`]/[`agenda::week`]); the four
///   not-yet-implemented queries return `Ok(_)` today (see the module docs'
///   "Why stub bodies return empty results" section) and will surface
///   [`IndexError::Sqlite`] once their real bodies land — an error variant
///   this trait already declares as part of its `Result` return type, so
///   that transition is a body change, not a signature change.
pub trait IndexQuery {
    /// See [`agenda::today`].
    fn agenda_today(
        &self,
        conn: &Connection,
        today: &str,
    ) -> Result<Vec<agenda::AgendaItem>, IndexError> {
        agenda::today(conn, today)
    }

    /// See [`agenda::week`].
    fn agenda_week(
        &self,
        conn: &Connection,
        start_date: &str,
    ) -> Result<Vec<agenda::AgendaItem>, IndexError> {
        agenda::week(conn, start_date)
    }

    /// See [`agenda::custom`] (Story 7.4 — signature frozen now, body
    /// pending).
    fn agenda_custom(
        &self,
        conn: &Connection,
        query: &agenda::CustomAgendaQuery,
    ) -> Result<Vec<agenda::AgendaItem>, IndexError> {
        agenda::custom(conn, query)
    }

    /// See [`search::query`] (Story 8.4 — signature frozen now, body
    /// pending).
    fn search(
        &self,
        conn: &Connection,
        query: &search::SearchQuery,
    ) -> Result<Vec<search::SearchResult>, IndexError> {
        search::query(conn, query)
    }

    /// See [`search::search_stream`] (Story 8.4 — signature frozen now, body
    /// pending). Not `dyn`-compatible — see the module docs' "Note on
    /// `IndexQuery` and `dyn` dispatch".
    fn search_stream<'c>(
        &self,
        conn: &'c Connection,
        query: &search::SearchQuery,
    ) -> Result<impl Iterator<Item = search::SearchResult> + 'c, IndexError> {
        search::search_stream(conn, query)
    }

    /// See [`backlinks::for_headline`] (Story 8.6 — signature frozen now,
    /// body pending).
    fn backlinks_for_headline(
        &self,
        conn: &Connection,
        headline_id: HeadlineId,
    ) -> Result<Vec<backlinks::Backlink>, IndexError> {
        backlinks::for_headline(conn, headline_id)
    }

    /// See [`backlinks::unlinked_mentions`] (Story 12.0 v0.5+ — signature
    /// frozen now, body pending).
    fn backlinks_unlinked_mentions(
        &self,
        conn: &Connection,
        headline_id: HeadlineId,
    ) -> Result<Vec<backlinks::UnlinkedMention>, IndexError> {
        backlinks::unlinked_mentions(conn, headline_id)
    }

    /// See [`graph::adjacency`] (Story 8.10 — signature frozen now, body
    /// pending).
    fn graph_adjacency(
        &self,
        conn: &Connection,
        scope: &graph::GraphScope,
    ) -> Result<graph::GraphData, IndexError> {
        graph::adjacency(conn, scope)
    }
}

/// Zero-sized default implementation of [`IndexQuery`] — every method uses
/// its trait-declared default body (a thin wrapper over the matching
/// submodule free function), so `DefaultIndexQuery.agenda_today(&conn,
/// today)` and `agenda::today(&conn, today)` always agree. The free
/// functions remain the canonical, directly-callable entry points (as
/// `orgsidian-core` already does for `agenda::today`/`agenda::week`); this
/// type exists for a caller that wants one `impl IndexQuery` value instead
/// of importing eight free functions individually.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultIndexQuery;

impl IndexQuery for DefaultIndexQuery {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{FileIndexInput, HeadlineInput};
    use rusqlite::Connection;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::apply_schema(&mut conn).expect("apply schema");
        conn
    }

    /// The trait's default `agenda_today` must produce byte-identical
    /// results to calling `agenda::today` directly — proves the wrapper
    /// adds a calling convention, not a second implementation to drift out
    /// of sync.
    #[test]
    fn default_index_query_agenda_today_matches_free_function() {
        let mut conn = open_test_db();
        let mut h = HeadlineInput {
            level: 1,
            position: 0,
            byte_start: 0,
            byte_end: 10,
            todo_keyword: Some("TODO".to_string()),
            todo_done: Some(false),
            title: "Scheduled today".to_string(),
            body: String::new(),
            scheduled_date: Some("2026-09-05".to_string()),
            scheduled_time: None,
            deadline_date: None,
            deadline_time: None,
            closed_date: None,
            closed_time: None,
            tags: Vec::new(),
            properties: Vec::new(),
            clock_entries: Vec::new(),
            links: Vec::new(),
            children: Vec::new(),
        };
        h.scheduled_date = Some("2026-09-05".to_string());
        crate::upsert_file(
            &mut conn,
            &FileIndexInput {
                rel_path: "a.org".to_string(),
                mtime_ns: 1,
                size_bytes: 1,
                preamble: None,
                headlines: vec![h],
            },
        )
        .expect("upsert");

        let via_trait = DefaultIndexQuery
            .agenda_today(&conn, "2026-09-05")
            .expect("query");
        let via_free_fn = agenda::today(&conn, "2026-09-05").expect("query");

        assert_eq!(via_trait, via_free_fn);
        assert_eq!(via_trait.len(), 1);
    }

    /// The stub queries (unbuilt stories) must not panic when reached
    /// through the trait's default methods — they return empty results, not
    /// `unimplemented!()` (see the module docs).
    #[test]
    fn stub_queries_are_reachable_through_the_trait_without_panicking() {
        let conn = open_test_db();
        let q = DefaultIndexQuery;

        let custom = q
            .agenda_custom(
                &conn,
                &agenda::CustomAgendaQuery {
                    start_date: "2026-09-05".to_string(),
                    end_date: "2026-09-12".to_string(),
                    tag: None,
                    todo_state: None,
                    file_path_glob: None,
                },
            )
            .expect("stub must not error");
        assert!(custom.is_empty());

        let searched = q
            .search(
                &conn,
                &search::SearchQuery {
                    text: "kubernetes".to_string(),
                },
            )
            .expect("stub must not error");
        assert!(searched.is_empty());

        let streamed = q
            .search_stream(
                &conn,
                &search::SearchQuery {
                    text: "kubernetes".to_string(),
                },
            )
            .expect("stub must not error");
        assert_eq!(streamed.count(), 0);

        let backlinks = q
            .backlinks_for_headline(&conn, HeadlineId(1))
            .expect("stub must not error");
        assert!(backlinks.is_empty());

        let unlinked = q
            .backlinks_unlinked_mentions(&conn, HeadlineId(1))
            .expect("stub must not error");
        assert!(unlinked.is_empty());

        let graph = q
            .graph_adjacency(&conn, &graph::GraphScope::WholeVault)
            .expect("stub must not error");
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn headline_id_roundtrips_through_i64() {
        let id: HeadlineId = 42i64.into();
        assert_eq!(id, HeadlineId(42));
        let back: i64 = id.into();
        assert_eq!(back, 42);
    }
}
