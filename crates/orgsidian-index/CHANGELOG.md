# Changelog

All notable changes to `orgsidian-index` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
internally from day 1 even though the crate is not published to crates.io
(see LD-10 / LD-33 in `_bmad-output/planning-artifacts/architecture.md`).

This file tracks the crate's own SemVer surface — separate from, and more
granular than, the app-wide `v0.x` release tags `../../CHANGELOG.md` records
— because Story 6.5 (LD-32 / LD-33 / NFR-19) enforces this crate's
`query::*` public API with `cargo-semver-checks` on every PR
(`.github/workflows/pr.yml`); a heading here is the human-readable record of
what that gate is protecting at each point.

## [Unreleased]

## Query API: v1.0 - 2026-09-05

### Added

- **The `IndexQuery` trait** (`crates/orgsidian-index/src/query/mod.rs`, Story 6.5) — the frozen v0.1 `orgsidian-index::query` public API surface, wrapping the four submodules below as default-bodied methods (`agenda_today`, `agenda_week`, `agenda_custom`, `search`, `search_stream`, `backlinks_for_headline`, `backlinks_unlinked_mentions`, `graph_adjacency`). `DefaultIndexQuery` is the zero-sized default implementor.
- **`HeadlineId(pub i64)`** — the shared row-identity newtype the Backlinks/Search/Graph surfaces below use.
- **`query::agenda`** (Stories 6.3, 6.4, 7.4):
  - `today(conn, today) -> Result<Vec<AgendaItem>, IndexError>` — shipped and tested (Story 6.3).
  - `week(conn, start_date) -> Result<Vec<AgendaItem>, IndexError>` — shipped and tested (Story 6.4).
  - `custom(conn, query: &CustomAgendaQuery) -> Result<Vec<AgendaItem>, IndexError>` — frozen signature; stub body pending Story 7.4. `CustomAgendaQuery` is `#[non_exhaustive]`.
- **`query::search`** (Story 8.4 two-tier streaming contract) — frozen signatures; stub bodies pending Story 8.4:
  - `query(conn, &SearchQuery) -> Result<Vec<SearchResult>, IndexError>` (full batch, up to 50 results).
  - `search_stream(conn, &SearchQuery) -> Result<impl Iterator<Item = SearchResult>, IndexError>` (streaming; first 10 yieldable before the full batch completes).
  - `SearchQuery`, `SearchResult` are `#[non_exhaustive]`.
- **`query::backlinks`** (Story 8.6 Backlinks + Story 12.0 v0.5+ Unlinked References) — frozen signatures; stub bodies pending:
  - `for_headline(conn, HeadlineId) -> Result<Vec<Backlink>, IndexError>` (Story 8.6).
  - `unlinked_mentions(conn, HeadlineId) -> Result<Vec<UnlinkedMention>, IndexError>` (Story 12.0, v0.5+).
  - `Backlink`, `UnlinkedMention` are `#[non_exhaustive]`.
- **`query::graph`** (Story 8.10 Backlink Graph adjacency, FR-26, LD-56) — frozen signature; stub body pending:
  - `adjacency(conn, &GraphScope) -> Result<GraphData, IndexError>`.
  - `GraphData { nodes: Vec<NodeRef>, edges: Vec<Edge> }`; `GraphScope::{WholeVault, NeighborhoodOf(HeadlineId, u8)}` (`Tag`/`FilePath` variants reserved for v0.5+); `EdgeKind::{IdLink, WikiLink}`. `GraphScope`, `EdgeKind`, `NodeRef`, `Edge`, `GraphData` are all `#[non_exhaustive]`.

### Enforcement

- `.github/workflows/pr.yml` runs `cargo semver-checks check-release --release-type minor -p orgsidian-index` on every PR, failing on any breaking change to this surface. `--release-type minor` is required (not optional): every workspace crate is version-pinned at `0.0.0` pre-v0.1, so without it the tool sees `0.0.0 -> 0.0.0` and silently skips every lint (verified hands-on during Story 6.5's implementation). See that workflow's comments for the current baseline-rev strategy and the TODO pinning it to the `v0.1.0-alpha` tag once Story 6.10 publishes it.
- Semver-minor growth this baseline is designed to absorb without a major bump: a stub above gaining its real body (Stories 7.4/8.4/8.6/8.10/12.0); a new field on any `#[non_exhaustive]` struct above; a new variant on `GraphScope`/`EdgeKind`; a new `IndexQuery` method with a default body.
- A genuine breaking change to any signature above (or removing/renaming a method or public item) requires an explicit entry in this file plus reviewer override, per the Story 6.5 AC.
