//! Org timestamp values: active/inactive, date/time (chrono-backed), ranges,
//! repeaters, and delays (FR-1; the FR-9 mapping names this file).
//!
//! The grammar produces `timestamp` nodes only in plan/entry position (the
//! line right after a headline); it does **not** distinguish `<active>` from
//! `[inactive]` structurally — the delimiter byte does. This module therefore
//! parses timestamp *text*: one hand-rolled, panic-free scanner shared by the
//! plan-entry path and the textual `CLOCK:`-line path (drawer contents are
//! raw token soup at the pinned grammar SHA).
//!
//! Lenience contract (LD-41): a malformed value inside an otherwise-shaped
//! timestamp (e.g. month 13) makes the whole timestamp unparseable — the
//! caller records `None` and analysis continues. Nothing here panics.

use std::ops::Range;

use chrono::{NaiveDate, NaiveTime};

/// How a repeater advances a timestamp when its task is marked done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub enum RepeaterKind {
    /// `+` — shift by exactly one interval from the stamped date.
    Cumulate,
    /// `++` — shift by as many intervals as needed to land in the future.
    CatchUp,
    /// `.+` — shift relative to today (restart the interval clock).
    Restart,
}

/// How a `-`/`--` warning delay applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub enum DelayKind {
    /// `-` — delay applies to all occurrences.
    All,
    /// `--` — delay applies to the first occurrence only.
    First,
}

/// Time unit of a repeater or delay interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub enum TimeUnit {
    /// `h`
    Hour,
    /// `d`
    Day,
    /// `w`
    Week,
    /// `m`
    Month,
    /// `y`
    Year,
}

/// A repeater interval, e.g. `+1w`, `++2d`, `.+1m`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Repeater {
    /// Repeat strategy (`+`, `++`, `.+`).
    pub kind: RepeaterKind,
    /// Interval count (the `1` in `+1w`).
    pub value: u32,
    /// Interval unit (the `w` in `+1w`).
    pub unit: TimeUnit,
}

/// A warning-delay interval, e.g. `-2d`, `--1w`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Delay {
    /// Delay strategy (`-`, `--`).
    pub kind: DelayKind,
    /// Interval count.
    pub value: u32,
    /// Interval unit.
    pub unit: TimeUnit,
}

/// One org timestamp, e.g. `<2026-06-10 Wed 10:00-11:00 +1w>` or
/// `[2026-06-09 Tue]--[2026-06-10 Wed]`.
///
/// Day names (`Wed`) are display sugar — they are kept in [`raw`](Self::raw)
/// but not modeled (the date alone determines the weekday).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Timestamp {
    /// `true` for `<…>` (active), `false` for `[…]` (inactive).
    pub active: bool,
    /// Calendar date (start date for ranges).
    pub date: NaiveDate,
    /// Clock time, when present (start time for `10:00-11:00` ranges).
    pub time: Option<NaiveTime>,
    /// End date for `<…>--<…>` ranges.
    pub end_date: Option<NaiveDate>,
    /// End time, from either a `10:00-11:00` range or the second half of a
    /// `<…>--<…>` range. When both are present (degenerate
    /// `<d1 10:00-11:00>--<d2 t>` forms), the second half's time wins; with
    /// no time on the second half, the first half's range end is kept —
    /// consumers needing per-date end-time attribution should re-read
    /// [`raw`](Self::raw) (single-field shape recorded in deferred-work).
    pub end_time: Option<NaiveTime>,
    /// Repeater interval (`+1w`, `++1m`, `.+1d`), when present.
    pub repeater: Option<Repeater>,
    /// Warning delay (`-2d`, `--1w`), when present.
    pub delay: Option<Delay>,
    /// The exact source text of the whole timestamp (both halves of a range).
    pub raw: String,
    /// Byte range of [`raw`](Self::raw) in the `analyze()` input.
    pub span: Range<usize>,
}

/// Parse a full timestamp at the start of `text` (which must begin with `<`
/// or `[`), merging a `--`-joined second half into `end_date`/`end_time`
/// (a repeater/delay written only on the second half is promoted to the
/// merged stamp). A `--` tail whose second half does not parse is left
/// unconsumed: the result degrades to the first half alone and the tail
/// stays raw text outside [`Timestamp::raw`] (lenient posture).
/// `offset` is the byte position of `text[0]` in the original source.
pub(crate) fn parse_at(text: &str, offset: usize) -> Option<Timestamp> {
    let (mut ts, mut consumed) = parse_one(text, offset)?;
    let rest = &text[consumed..];
    if let Some(second) = rest.strip_prefix("--") {
        if let Some((end, end_len)) = parse_one(second, offset + consumed + 2) {
            ts.end_date = Some(end.date);
            ts.end_time = end.time.or(ts.end_time);
            ts.repeater = ts.repeater.or(end.repeater);
            ts.delay = ts.delay.or(end.delay);
            consumed += 2 + end_len;
        }
    }
    ts.raw = text[..consumed].to_string();
    ts.span = offset..offset + consumed;
    Some(ts)
}

/// Parse a single `<…>`/`[…]` half at the start of `text`; does **not**
/// consume a `--` range continuation. Returns the timestamp and the number
/// of bytes consumed. Used directly by the `CLOCK:`-line parser, which needs
/// the two halves as separate start/end stamps.
pub(crate) fn parse_one(text: &str, offset: usize) -> Option<(Timestamp, usize)> {
    let mut bytes = text.bytes();
    let (active, close) = match bytes.next() {
        Some(b'<') => (true, '>'),
        Some(b'[') => (false, ']'),
        _ => return None,
    };
    let close_pos = text.find(close)?;
    let inner = &text[1..close_pos];

    let mut tokens = inner.split_whitespace();
    let date = parse_date(tokens.next()?)?;

    let mut time = None;
    let mut end_time = None;
    let mut repeater = None;
    let mut delay = None;
    for token in tokens {
        if let Some((start, end)) = parse_time_token(token) {
            if time.is_none() {
                time = Some(start);
                end_time = end;
            }
        } else if let Some(rep) = parse_repeater(token) {
            repeater = repeater.or(Some(rep));
        } else if let Some(del) = parse_delay(token) {
            delay = delay.or(Some(del));
        }
        // Anything else (day name, stray text) is tolerated and skipped:
        // it survives in `raw`, and the lenient posture never errors here.
    }

    let consumed = close_pos + 1;
    let ts = Timestamp {
        active,
        date,
        time,
        end_date: None,
        end_time,
        repeater,
        delay,
        raw: text[..consumed].to_string(),
        span: offset..offset + consumed,
    };
    Some((ts, consumed))
}

/// `YYYY-MM-DD` → `NaiveDate`, rejecting impossible dates via the panic-free
/// `from_ymd_opt` constructor.
fn parse_date(token: &str) -> Option<NaiveDate> {
    let mut parts = token.splitn(3, '-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// `H:MM` or `H:MM-H:MM` → start time + optional end time.
fn parse_time_token(token: &str) -> Option<(NaiveTime, Option<NaiveTime>)> {
    match token.split_once('-') {
        Some((start, end)) => Some((parse_hm(start)?, parse_hm(end))),
        None => Some((parse_hm(token)?, None)),
    }
}

/// `H:MM` / `HH:MM` → `NaiveTime`, panic-free.
fn parse_hm(text: &str) -> Option<NaiveTime> {
    let (hours, minutes) = text.split_once(':')?;
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    NaiveTime::from_hms_opt(hours, minutes, 0)
}

/// `+1w` / `++1m` / `.+1d` → [`Repeater`].
fn parse_repeater(token: &str) -> Option<Repeater> {
    let (kind, rest) = if let Some(rest) = token.strip_prefix("++") {
        (RepeaterKind::CatchUp, rest)
    } else if let Some(rest) = token.strip_prefix(".+") {
        (RepeaterKind::Restart, rest)
    } else {
        let rest = token.strip_prefix('+')?;
        (RepeaterKind::Cumulate, rest)
    };
    let (value, unit) = parse_interval(rest)?;
    Some(Repeater { kind, value, unit })
}

/// `-2d` / `--1w` → [`Delay`].
fn parse_delay(token: &str) -> Option<Delay> {
    let (kind, rest) = if let Some(rest) = token.strip_prefix("--") {
        (DelayKind::First, rest)
    } else {
        let rest = token.strip_prefix('-')?;
        (DelayKind::All, rest)
    };
    let (value, unit) = parse_interval(rest)?;
    Some(Delay { kind, value, unit })
}

/// `1w` → `(1, Week)`; the unit must be the final character. A zero value
/// (`+0d`) is rejected (review fix: a zero-interval repeater would make any
/// downstream repeat-advance loop forever) — the token is skipped as stray
/// text and survives in `raw`.
fn parse_interval(text: &str) -> Option<(u32, TimeUnit)> {
    let unit = match text.as_bytes().last()? {
        b'h' => TimeUnit::Hour,
        b'd' => TimeUnit::Day,
        b'w' => TimeUnit::Week,
        b'm' => TimeUnit::Month,
        b'y' => TimeUnit::Year,
        _ => return None,
    };
    let value: u32 = text[..text.len() - 1].parse().ok()?;
    if value == 0 {
        return None;
    }
    Some((value, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid test date")
    }

    #[test]
    fn parses_full_active_stamp() {
        let text = "<2026-06-10 Wed 10:00-11:00 +1w -2d>";
        let ts = parse_at(text, 100).expect("must parse");
        assert!(ts.active);
        assert_eq!(ts.date, date(2026, 6, 10));
        assert_eq!(ts.time, NaiveTime::from_hms_opt(10, 0, 0));
        assert_eq!(ts.end_time, NaiveTime::from_hms_opt(11, 0, 0));
        assert_eq!(
            ts.repeater,
            Some(Repeater {
                kind: RepeaterKind::Cumulate,
                value: 1,
                unit: TimeUnit::Week
            })
        );
        assert_eq!(
            ts.delay,
            Some(Delay {
                kind: DelayKind::All,
                value: 2,
                unit: TimeUnit::Day
            })
        );
        assert_eq!(ts.raw, text);
        assert_eq!(ts.span, 100..100 + text.len());
    }

    #[test]
    fn parses_inactive_and_date_range() {
        let ts = parse_at("[2026-06-09 Tue]--[2026-06-10 Wed 08:00]", 0).expect("must parse");
        assert!(!ts.active);
        assert_eq!(ts.date, date(2026, 6, 9));
        assert_eq!(ts.end_date, Some(date(2026, 6, 10)));
        assert_eq!(ts.end_time, NaiveTime::from_hms_opt(8, 0, 0));
        assert_eq!(ts.span, 0..40);
    }

    #[test]
    fn rejects_impossible_dates_without_panicking() {
        assert_eq!(parse_at("<2026-13-40 Xxx>", 0), None);
        assert_eq!(parse_at("<not-a-date>", 0), None);
        assert_eq!(parse_at("plain text", 0), None);
        assert_eq!(parse_at("<2026-06-10", 0), None, "unterminated stamp");
    }

    #[test]
    fn day_names_and_stray_tokens_are_tolerated() {
        let ts = parse_at("<2026-06-10 Mer 10:00 garbage>", 0).expect("lenient");
        assert_eq!(ts.time, NaiveTime::from_hms_opt(10, 0, 0));
    }

    #[test]
    fn zero_value_intervals_are_stray_text() {
        // Review fix (Story 2.3): `+0d` must not become a Repeater/Delay.
        assert_eq!(parse_repeater("+0d"), None);
        assert_eq!(parse_delay("-0w"), None);
        let ts = parse_at("<2026-06-10 Wed +0d>", 0).expect("stamp still parses");
        assert_eq!(ts.repeater, None);
        assert_eq!(ts.raw, "<2026-06-10 Wed +0d>", "token survives in raw");
    }

    #[test]
    fn unparseable_range_tail_degrades_to_first_half() {
        // Documented lenient posture: the `--…` tail stays outside `raw`.
        let ts = parse_at("<2026-06-10 Wed>--<garbage>", 7).expect("first half");
        assert_eq!(ts.end_date, None);
        assert_eq!(ts.raw, "<2026-06-10 Wed>");
        assert_eq!(ts.span, 7..7 + 16);
    }

    #[test]
    fn repeater_kinds_and_units() {
        for (text, kind, unit) in [
            ("+1h", RepeaterKind::Cumulate, TimeUnit::Hour),
            ("++3m", RepeaterKind::CatchUp, TimeUnit::Month),
            (".+2y", RepeaterKind::Restart, TimeUnit::Year),
        ] {
            let rep = parse_repeater(text).expect("repeater");
            assert_eq!(rep.kind, kind);
            assert_eq!(rep.unit, unit);
        }
        assert_eq!(parse_repeater("+w"), None, "missing value");
        assert_eq!(parse_repeater("+1x"), None, "unknown unit");
    }
}
