//! Day-1 placeholder payload structs referenced by trait signatures and
//! [`crate::Event`] variants.
//!
//! Each struct here is intentionally minimal: the variant or struct **name**
//! is locked, the **fields** are room-for-growth. Future stories add fields
//! as SemVer-minor additive bumps per LD-26.

/// Day-1 placeholder for a Quick Capture entry.
///
/// The concrete schema lands with Story 8.1 (Quick Capture window); fields
/// added then arrive as SemVer-minor additive bumps per LD-26. The `raw_text`
/// escape-hatch lets the host construct a `CaptureEntry` from richer internal
/// types without leaking those types into the leaf crate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaptureEntry {
    /// Raw user-entered capture text (template expansion happens host-side
    /// before plugins see the entry).
    pub raw_text: String,
}

/// Day-1 placeholder for an agenda query.
///
/// The concrete schema lands with Stories 6.3 / 6.4 (Today / Week agenda);
/// fields added then arrive as SemVer-minor additive bumps per LD-26. The
/// `raw_filter` escape-hatch lets the host construct an `AgendaQuery` from
/// richer internal types without leaking those types into the leaf crate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgendaQuery {
    /// Free-form filter expression — semantics defined host-side when the
    /// typed query API lands (Stories 6.3 / 6.4 / 8.x).
    pub raw_filter: String,
}

/// Day-1 placeholder for a single agenda result row.
///
/// The concrete schema lands with Stories 6.3 / 6.4 plus the Story 7.x
/// dashboard widgets; fields added then arrive as SemVer-minor additive
/// bumps per LD-26.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgendaItem {
    /// Stable identifier of the source headline (host-defined ID format;
    /// `String` keeps the leaf crate independent of UUID / int-id choices).
    pub headline_id: String,
    /// Pre-rendered display text for the agenda row (the host owns the
    /// formatting pass; plugins may post-process via `on_agenda_query_after`).
    pub display_text: String,
}
