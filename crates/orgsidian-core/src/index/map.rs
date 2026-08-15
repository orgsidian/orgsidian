//! `orgsidian_parser::Document` → `orgsidian_index::FileIndexInput` mapping
//! (Story 3.6 Dev Note 1: the LEAF rule forces this into `orgsidian-core`).
//!
//! `orgsidian-index` is a LEAF that may not depend on the parser leaf
//! (deny.toml), so its sync API takes parser-agnostic input structs. This
//! module is the composition point that knows BOTH types — it flattens the
//! parser's owned semantic tree into the index's column-shaped input, mirroring
//! the schema (migrations/0001_initial-schema.sql). Every `Headline` field maps
//! to its column; tags/properties/clocks/links are flattened; the preamble
//! becomes the synthetic `kind='preamble'` row.

use orgsidian_index::{ClockInput, FileIndexInput, HeadlineInput, LinkInput, PreambleInput};

use crate::parser::semantic::{Document, Headline, Link, LinkKind, Preamble, Timestamp};

/// Map one analyzed document to the index's file-level input. `rel_path` is the
/// vault-relative key (from `orgsidian_vault::to_rel_path`); `mtime_ns`/
/// `size_bytes` are the incremental-skip key the orchestrator already read.
pub fn document_to_input(
    rel_path: String,
    mtime_ns: i64,
    size_bytes: i64,
    document: &Document,
) -> FileIndexInput {
    FileIndexInput {
        rel_path,
        mtime_ns,
        size_bytes,
        preamble: document.preamble.as_ref().map(map_preamble),
        headlines: document
            .headlines
            .iter()
            .enumerate()
            .map(|(position, headline)| map_headline(position, headline))
            .collect(),
    }
}

/// Map the document preamble → the synthetic `kind='preamble'` row's input.
fn map_preamble(preamble: &Preamble) -> PreambleInput {
    PreambleInput {
        body: preamble.text.clone(),
        byte_start: preamble.span.start as i64,
        byte_end: preamble.span.end as i64,
        links: preamble.links.iter().map(map_link).collect(),
    }
}

/// Map one headline (recursively) → its `HeadlineInput`. `position` is the
/// sibling order in document order.
fn map_headline(position: usize, headline: &Headline) -> HeadlineInput {
    let (todo_keyword, todo_done) = match &headline.todo_state {
        Some(state) => (Some(state.keyword.clone()), Some(state.done)),
        None => (None, None),
    };

    HeadlineInput {
        level: i64::from(headline.level),
        position: position as i64,
        byte_start: headline.span.start as i64,
        byte_end: headline.span.end as i64,
        todo_keyword,
        todo_done,
        title: headline.title.clone(),
        // `headlines.body` is `Headline.raw` — this section's own region
        // (schema comment migrations/0001_initial-schema.sql:165-168).
        body: headline.raw.clone(),
        scheduled_date: headline.scheduled.as_ref().map(date_of),
        scheduled_time: headline.scheduled.as_ref().and_then(time_of),
        deadline_date: headline.deadline.as_ref().map(date_of),
        deadline_time: headline.deadline.as_ref().and_then(time_of),
        closed_date: headline.closed.as_ref().map(date_of),
        closed_time: headline.closed.as_ref().and_then(time_of),
        tags: headline.tags.iter().map(|t| t.name.clone()).collect(),
        properties: headline
            .properties
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        clock_entries: headline.clocks.iter().map(map_clock).collect(),
        links: headline.links.iter().map(map_link).collect(),
        children: headline
            .children
            .iter()
            .enumerate()
            .map(|(child_position, child)| map_headline(child_position, child))
            .collect(),
    }
}

/// Map one CLOCK entry. `start_at`/`end_at` are combined ISO-8601 datetimes
/// (schema: `clock_entries` uses combined datetimes, unlike the split
/// date/time on `headlines`).
fn map_clock(clock: &crate::parser::semantic::ClockEntry) -> ClockInput {
    ClockInput {
        start_at: datetime_of(&clock.start),
        end_at: clock.end.as_ref().map(datetime_of),
        duration_seconds: clock.duration.map(|d| d.num_seconds()),
    }
}

/// Map one link → its `links` row input, lowercasing `kind` to the schema's
/// CHECK set.
fn map_link(link: &Link) -> LinkInput {
    LinkInput {
        kind: link_kind_str(&link.kind).to_string(),
        target: link.target.clone(),
        description: link.description.clone(),
    }
}

/// Lowercased `LinkKind` per the schema CHECK (`id`/`file`/`url`/`wiki`/`plain`).
fn link_kind_str(kind: &LinkKind) -> &'static str {
    match kind {
        LinkKind::Id => "id",
        LinkKind::File => "file",
        LinkKind::Url => "url",
        LinkKind::Wiki => "wiki",
        LinkKind::Plain => "plain",
    }
}

/// ISO-8601 date (`YYYY-MM-DD`) of a timestamp's start date.
fn date_of(timestamp: &Timestamp) -> String {
    timestamp.date.format("%Y-%m-%d").to_string()
}

/// ISO-8601 time (`HH:MM`, org granularity) of a timestamp, when it carries one.
fn time_of(timestamp: &Timestamp) -> Option<String> {
    timestamp.time.map(|time| time.format("%H:%M").to_string())
}

/// Combined ISO-8601 datetime (`YYYY-MM-DDTHH:MM`, or the bare date when the
/// stamp is date-only) for a `clock_entries` column.
fn datetime_of(timestamp: &Timestamp) -> String {
    match time_of(timestamp) {
        Some(time) => format!("{}T{}", date_of(timestamp), time),
        None => date_of(timestamp),
    }
}
