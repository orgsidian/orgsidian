//! Implements FR-26 (v0.1 `IndexQuery` baseline — Story 8.10 Backlink Graph
//! adjacency query API, LD-56).
//!
//! Story 8.10 will implement [`adjacency`]: the Vault's `:ID:`-keyed
//! Headlines as nodes and `[[id:...]]` / `[[wiki-link]]` references as
//! edges, scoped to a subgraph, reusing the same `links` table
//! [`super::backlinks`] queries (LD-13). Story 6.5 freezes the shape now —
//! `GraphData { nodes, edges }`, `GraphScope`, `EdgeKind` — so Story 8.10
//! implements the signature already declared in the baseline rather than
//! inventing it.

use std::path::PathBuf;

use rusqlite::Connection;

use super::HeadlineId;
use crate::error::IndexError;

/// Which subgraph [`adjacency`] returns.
///
/// `#[non_exhaustive]`: the Story 8.10 AC reserves `Tag(TagId)` and
/// `FilePath(PathBuf)` variants for v0.5+ subgraph filtering — not added yet
/// (no `TagId` type exists in this crate today), but `#[non_exhaustive]`
/// means adding them later is semver-minor, not a breaking change to every
/// exhaustive `match` on this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum GraphScope {
    /// Every `:ID:`-keyed Headline and cross-reference in the Vault.
    WholeVault,
    /// The subgraph within `depth` hops of one Headline.
    NeighborhoodOf(HeadlineId, u8),
}

/// Distinguishes an `[[id:...]]` reference from a `[[wiki-link]]` reference
/// on an [`Edge`] — per the Story 8.10 AC, "typed-edge styling for v0.5+ is
/// non-breaking — UI can ignore the distinction in v0.1".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum EdgeKind {
    /// An `[[id:...]]` reference.
    IdLink,
    /// A `[[wiki-link]]` reference.
    WikiLink,
}

/// One graph node: an `:ID:`-keyed Headline, carrying enough identity for
/// the frontend's click-to-Source and label rendering.
///
/// `#[non_exhaustive]`: Story 8.10/8.11 may need a node-degree or
/// last-modified field once the force-directed layout is tuned.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct NodeRef {
    /// The Headline's identity.
    pub id: HeadlineId,
    /// `files.path` of the Headline's source file.
    pub file: PathBuf,
    /// Headline title, stars/keyword/tags already stripped.
    pub title: String,
}

/// One graph edge: a directed reference from `src_id` to `dst_id`.
///
/// `#[non_exhaustive]`: mirrors [`NodeRef`]'s growth allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct Edge {
    /// The Headline containing the reference.
    pub src_id: HeadlineId,
    /// The Headline being referenced.
    pub dst_id: HeadlineId,
    /// Which reference syntax this edge came from.
    pub kind: EdgeKind,
}

/// The full result of an [`adjacency`] query: every node and edge in the
/// requested [`GraphScope`].
///
/// `#[non_exhaustive]`: leaves room for e.g. a `truncated: bool` flag if a
/// future story caps result size for very large subgraphs.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct GraphData {
    /// Every `:ID:`-keyed Headline in scope.
    pub nodes: Vec<NodeRef>,
    /// Every `[[id:...]]` / `[[wiki-link]]` reference in scope.
    pub edges: Vec<Edge>,
}

/// The Vault's backlink graph, scoped per `scope`.
///
/// **Story 8.10 stub**: this is the v0.1 `IndexQuery` baseline signature
/// (Story 6.5), frozen ahead of its Story 8.10 implementation. The body
/// below returns an empty `GraphData` rather than `unimplemented!()` so it
/// is safely reachable through [`super::IndexQuery::graph_adjacency`]'s
/// default method before Story 8.10 lands.
///
/// # Errors
///
/// [`IndexError::Sqlite`] once implemented; the stub body never errors.
pub fn adjacency(conn: &Connection, scope: &GraphScope) -> Result<GraphData, IndexError> {
    // Story 8.10 TODO: implement the `links`-table adjacency-list query,
    // scoped per `GraphScope`. Unused until then.
    let _ = conn;
    let _ = scope;
    Ok(GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
    })
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
    fn adjacency_stub_returns_empty_without_erroring() {
        let conn = open_test_db();
        let result = adjacency(&conn, &GraphScope::WholeVault).expect("stub must not error");
        assert!(result.nodes.is_empty());
        assert!(result.edges.is_empty());
    }

    #[test]
    fn adjacency_stub_accepts_neighborhood_scope() {
        let conn = open_test_db();
        let result = adjacency(&conn, &GraphScope::NeighborhoodOf(HeadlineId(1), 2))
            .expect("stub must not error");
        assert!(result.nodes.is_empty());
        assert!(result.edges.is_empty());
    }
}
