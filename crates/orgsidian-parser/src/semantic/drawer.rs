//! Drawer classification and `CLOCK:`-line parsing (FR-1).
//!
//! The grammar models `:PROPERTIES:` drawers as structured `property_drawer`
//! nodes (they feed `Headline::properties`), but `:LOGBOOK:` and custom
//! drawers are generic `drawer` nodes whose contents are **unstructured
//! token soup** at the pinned SHA. This module classifies drawers by name
//! and parses `CLOCK:` lines textually out of `:LOGBOOK:` contents — a
//! malformed CLOCK line is not an error, it simply stays raw drawer content
//! (LD-41: never crash on weird-but-real org).

use std::ops::Range;

use chrono::TimeDelta;

use super::timestamp::{self, Timestamp};

/// The three drawer classes the semantic layer distinguishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerKind {
    /// `:PROPERTIES:` — structured key/value pairs (see `Headline::properties`).
    Properties,
    /// `:LOGBOOK:` — matched case-insensitively; CLOCK lines are parsed out
    /// of its contents into `Headline::clocks`.
    Logbook,
    /// Any other drawer; carries the name with its original casing.
    Custom(String),
}

/// One drawer attached to a headline, with its raw contents and spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawer {
    /// Classification by drawer name (case-insensitive for `LOGBOOK`).
    pub kind: DrawerKind,
    /// The drawer name exactly as written (without colons), e.g. `LOGBOOK`.
    pub name: String,
    /// The drawer's contents, exactly as written. For generic drawers this is
    /// the grammar's `contents` region (between the `:NAME:` line and
    /// `:END:`); for the [`Properties`](DrawerKind::Properties) drawer it
    /// covers the property lines (first property through last — the grammar
    /// structures properties individually, so stray non-property lines
    /// outside that range are not included).
    pub contents: String,
    /// Byte range of the whole drawer (`:NAME:` through `:END:`).
    pub span: Range<usize>,
    /// Byte range of [`contents`](Self::contents) in the `analyze()` input.
    /// For a drawer with no contents this is an **empty** range anchored at
    /// the drawer node's end — callers must treat `is_empty()` spans as "no
    /// contents", not index around them.
    pub contents_span: Range<usize>,
}

/// One `CLOCK:` line parsed from `:LOGBOOK:` contents.
///
/// Open form: `CLOCK: [2026-06-10 Wed 08:00]` — start only.
/// Closed form: `CLOCK: [start]--[end] => H:MM` — start, end, and duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockEntry {
    /// Clock-in timestamp (org writes these in inactive `[…]` form).
    pub start: Timestamp,
    /// Clock-out timestamp; `None` while the clock is still running.
    pub end: Option<Timestamp>,
    /// The `=> H:MM` duration as written; `None` for open entries (or when
    /// the duration text does not parse — the entry is kept, the field is
    /// dropped).
    pub duration: Option<TimeDelta>,
    /// Byte range of the parsed CLOCK line content in the `analyze()` input.
    pub span: Range<usize>,
}

/// Scan drawer contents line by line for well-formed `CLOCK:` lines.
/// `offset` is the byte position of `contents[0]` in the original source.
/// Lines that do not parse are skipped — they remain raw drawer content.
pub(crate) fn parse_clock_lines(contents: &str, offset: usize) -> Vec<ClockEntry> {
    let mut entries = Vec::new();
    let mut line_start = 0;
    for line in contents.split_inclusive('\n') {
        if let Some(entry) = parse_clock_line(line, offset + line_start) {
            entries.push(entry);
        }
        line_start += line.len();
    }
    entries
}

/// Parse a single `CLOCK:` line; `None` for anything malformed.
fn parse_clock_line(line: &str, line_offset: usize) -> Option<ClockEntry> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("CLOCK:")?;
    let after_keyword = indent + "CLOCK:".len();

    // Start timestamp.
    let ws = rest.len() - rest.trim_start().len();
    let start_pos = after_keyword + ws;
    let (start, consumed) = timestamp::parse_one(&line[start_pos..], line_offset + start_pos)?;
    let mut cursor = start_pos + consumed;

    // Optional `--[end]`. A `--` whose right-hand side does not parse makes
    // the whole line malformed (review fix: it must not be misread as an
    // open/running entry) — it stays raw drawer content.
    let mut end = None;
    if let Some(rest) = line[cursor..].strip_prefix("--") {
        let (end_ts, end_len) = timestamp::parse_one(rest, line_offset + cursor + 2)?;
        end = Some(end_ts);
        cursor += 2 + end_len;
    }

    // Optional ` => H:MM` (only meaningful on closed entries; an unparseable
    // duration drops the field, not the entry).
    let mut duration = None;
    let tail = line[cursor..].trim_start();
    if let Some(dur_text) = tail.strip_prefix("=>") {
        let token = dur_text.trim_start();
        let token = token.split_whitespace().next().unwrap_or("");
        if !token.is_empty() {
            duration = parse_duration(token);
            if duration.is_some() {
                // Advance the span end over the consumed duration token.
                let token_start = line.len() - dur_text.trim_start().len();
                cursor = token_start + token.len();
            }
        }
    }

    Some(ClockEntry {
        start,
        end,
        duration,
        span: line_offset + indent..line_offset + cursor,
    })
}

/// `H:MM` (hours unbounded, e.g. `123:45`; minutes `00`-`59`) →
/// [`TimeDelta`], overflow-safe. Signed or out-of-range components are
/// rejected (review fix: `-1:30` / `1:99` / `1:-5` are not durations — the
/// entry is kept, the field is dropped).
fn parse_duration(token: &str) -> Option<TimeDelta> {
    let (hours, minutes) = token.split_once(':')?;
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if minutes > 59 {
        return None;
    }
    TimeDelta::try_minutes(
        i64::from(hours)
            .checked_mul(60)?
            .checked_add(minutes.into())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_entry_parses_start_end_duration() {
        let line = "CLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:30] =>  1:30\n";
        let entry = parse_clock_line(line, 1000).expect("closed clock line");
        assert!(!entry.start.active);
        assert!(entry.end.is_some());
        assert_eq!(entry.duration, TimeDelta::try_minutes(90));
        assert_eq!(entry.span.start, 1000);
    }

    #[test]
    fn open_entry_has_start_only() {
        let entry = parse_clock_line("CLOCK: [2026-06-10 Wed 08:00]\n", 0).expect("open line");
        assert!(entry.end.is_none());
        assert!(entry.duration.is_none());
        assert_eq!(entry.span, 0..29);
    }

    #[test]
    fn malformed_lines_are_skipped_not_errors() {
        assert_eq!(parse_clock_line("CLOCK: not a timestamp\n", 0), None);
        assert_eq!(parse_clock_line("CLOCKS: [2026-06-10 Wed]\n", 0), None);
        assert_eq!(parse_clock_line("random text\n", 0), None);
        // Unparseable duration keeps the entry, drops the field.
        let entry = parse_clock_line(
            "CLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:30] => soon\n",
            0,
        )
        .expect("entry kept");
        assert!(entry.duration.is_none());
    }

    #[test]
    fn range_with_unparseable_end_is_malformed_not_open() {
        // Review fix (Story 2.3): `--garbage` must not yield an open entry.
        assert_eq!(
            parse_clock_line("CLOCK: [2026-06-09 Tue 10:00]--garbage\n", 0),
            None
        );
        assert_eq!(
            parse_clock_line("CLOCK: [2026-06-09 Tue 10:00]-- => 1:30\n", 0),
            None
        );
    }

    #[test]
    fn nonsense_durations_are_rejected() {
        // Review fix (Story 2.3): signed/out-of-range components drop the
        // field (entry kept), never a negative or rewritten TimeDelta.
        for bad in ["-1:30", "1:99", "1:-5"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
        assert_eq!(parse_duration("123:45"), TimeDelta::try_minutes(7425));
        let line = "CLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:30] => -1:30\n";
        let entry = parse_clock_line(line, 0).expect("entry kept");
        assert!(entry.duration.is_none());
    }

    #[test]
    fn scans_multiple_lines_with_offsets() {
        let contents = "CLOCK: [2026-06-10 Wed 08:00]\njunk line\nCLOCK: [2026-06-11 Thu 09:00]\n";
        let entries = parse_clock_lines(contents, 50);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].span.start, 50);
        assert_eq!(entries[1].span.start, 50 + 30 + 10);
    }
}
