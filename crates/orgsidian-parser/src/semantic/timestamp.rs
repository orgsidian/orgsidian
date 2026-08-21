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

// ---------------------------------------------------------------------------
// Story 4.8 (FR-9): Schedule/Deadline write support.
//
// The pieces below are the PURE-RUST backend for the FR-9 date picker (locked in
// epic-4-context.md): a date-shortcut resolver, an org-timestamp formatter, and
// a byte-faithful planning-line writer. They are UI-independent — the shell-app
// `set_scheduled` command is a thin wrapper, and every rule here is unit-tested
// below. Nothing panics (LD-41 posture); the writer touches ONLY the planning
// line's bytes, so the rest of the document round-trips byte-identically (FR-2),
// and an edited recurring stamp keeps its `+1w`/`-2d` cookie.
// ---------------------------------------------------------------------------

/// Which planning keyword a Schedule/Deadline write targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningKind {
    /// `SCHEDULED:` — when work on the task should start.
    Scheduled,
    /// `DEADLINE:` — when the task is due.
    Deadline,
}

impl PlanningKind {
    /// The org planning keyword written before the timestamp (`SCHEDULED` /
    /// `DEADLINE`), without the trailing colon.
    pub fn keyword(self) -> &'static str {
        match self {
            PlanningKind::Scheduled => "SCHEDULED",
            PlanningKind::Deadline => "DEADLINE",
        }
    }
}

/// The concrete date (+ optional clock time) a picker — or a resolved shortcut
/// — commits for a planning timestamp. Planning stamps are always active
/// `<…>`, so activeness is not modeled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedStamp {
    /// Calendar date to write.
    pub date: NaiveDate,
    /// Clock time to write (`HH:MM`), when the picker supplied one.
    pub time: Option<NaiveTime>,
}

/// A minimal, byte-faithful edit: replace `from..to` with `insert`. Everything
/// outside `from..to` stays byte-identical, so the FR-2 round-trip contract
/// holds for the rest of the document. A no-op (e.g. removing an absent entry)
/// is `from == to` with an empty `insert`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningEdit {
    /// Byte offset where the replaced region begins.
    pub from: usize,
    /// Byte offset where the replaced region ends.
    pub to: usize,
    /// Replacement text.
    pub insert: String,
}

/// Resolve a relative date shortcut against `today`, the FR-9 fast-entry
/// vocabulary: `today`/`now`, and signed intervals `+N{d,w,m,y}` /
/// `-N{d,w,m,y}` (a bare `+N`/`-N` counts days, so `+1` == `+1d`). Returns
/// `None` for anything else — the caller then tries a literal `YYYY-MM-DD`.
/// Panic-free; month/year math clamps to a valid calendar day
/// (Jan 31 `+1m` → Feb 28/29).
pub fn resolve_date_shortcut(input: &str, today: NaiveDate) -> Option<NaiveDate> {
    let token = input.trim();
    if token.eq_ignore_ascii_case("today") || token.eq_ignore_ascii_case("now") {
        return Some(today);
    }
    let sign: i64 = match token.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &token[1..];
    let (digits, unit) = match rest.as_bytes().last()? {
        b'd' | b'D' => (&rest[..rest.len() - 1], TimeUnit::Day),
        b'w' | b'W' => (&rest[..rest.len() - 1], TimeUnit::Week),
        b'm' => (&rest[..rest.len() - 1], TimeUnit::Month),
        b'y' | b'Y' => (&rest[..rest.len() - 1], TimeUnit::Year),
        c if c.is_ascii_digit() => (rest, TimeUnit::Day),
        _ => return None,
    };
    let value: i64 = digits.parse().ok()?;
    add_interval(today, sign.checked_mul(value)?, unit)
}

/// Advance `date` by `signed_value` units, panic-free (all constructors are
/// `checked_*`). `Hour` does not move a calendar date, so it is a no-op here.
fn add_interval(date: NaiveDate, signed_value: i64, unit: TimeUnit) -> Option<NaiveDate> {
    match unit {
        TimeUnit::Hour => Some(date),
        TimeUnit::Day => date.checked_add_signed(chrono::Duration::days(signed_value)),
        TimeUnit::Week => date.checked_add_signed(chrono::Duration::weeks(signed_value)),
        TimeUnit::Month => add_months(date, signed_value),
        TimeUnit::Year => add_months(date, signed_value.checked_mul(12)?),
    }
}

/// Add (or subtract, for negative `months`) calendar months, clamping the
/// day-of-month down to the target month's last valid day (chrono `Months`
/// semantics). Panic-free.
fn add_months(date: NaiveDate, months: i64) -> Option<NaiveDate> {
    let magnitude = u32::try_from(months.unsigned_abs()).ok()?;
    let step = chrono::Months::new(magnitude);
    if months >= 0 {
        date.checked_add_months(step)
    } else {
        date.checked_sub_months(step)
    }
}

/// Format a planning timestamp as active org source, e.g.
/// `<2026-05-19 Tue 14:00 +1w>`. The weekday is computed from the date (org
/// display sugar) using chrono's locale-independent English `%a`, so a runner's
/// locale can never change the bytes written. `repeater`/`delay` cookies, when
/// carried over from an edited stamp, are re-emitted verbatim after the time.
pub fn format_planning_timestamp(
    stamp: PlannedStamp,
    repeater: Option<Repeater>,
    delay: Option<Delay>,
) -> String {
    let mut out = String::with_capacity(32);
    out.push('<');
    out.push_str(&stamp.date.format("%Y-%m-%d %a").to_string());
    if let Some(time) = stamp.time {
        out.push(' ');
        out.push_str(&time.format("%H:%M").to_string());
    }
    if let Some(rep) = repeater {
        out.push(' ');
        out.push_str(&format_repeater(rep));
    }
    if let Some(del) = delay {
        out.push(' ');
        out.push_str(&format_delay(del));
    }
    out.push('>');
    out
}

/// Single-char org unit suffix (`h`/`d`/`w`/`m`/`y`).
fn unit_char(unit: TimeUnit) -> char {
    match unit {
        TimeUnit::Hour => 'h',
        TimeUnit::Day => 'd',
        TimeUnit::Week => 'w',
        TimeUnit::Month => 'm',
        TimeUnit::Year => 'y',
    }
}

/// `Repeater` → source cookie, e.g. `+1w` / `++2d` / `.+1m`.
fn format_repeater(rep: Repeater) -> String {
    let prefix = match rep.kind {
        RepeaterKind::Cumulate => "+",
        RepeaterKind::CatchUp => "++",
        RepeaterKind::Restart => ".+",
    };
    format!("{prefix}{}{}", rep.value, unit_char(rep.unit))
}

/// `Delay` → source cookie, e.g. `-2d` / `--1w`.
fn format_delay(del: Delay) -> String {
    let prefix = match del.kind {
        DelayKind::All => "-",
        DelayKind::First => "--",
    };
    format!("{prefix}{}{}", del.value, unit_char(del.unit))
}

/// True when `line` is a planning line (its first non-space token is one of the
/// three org planning keywords). Trailing `\r` (CRLF) is irrelevant here.
fn is_planning_line(line: &str) -> bool {
    let head = line.trim_start();
    head.starts_with("SCHEDULED:") || head.starts_with("DEADLINE:") || head.starts_with("CLOSED:")
}

/// Newline style terminating the line whose `\n` sits at `nl_index`: `"\r\n"`
/// when a `\r` immediately precedes it, else `"\n"`. Used so an inserted
/// planning line matches the file's existing line endings.
fn newline_style(source: &str, nl_index: usize) -> &'static str {
    let bytes = source.as_bytes();
    if nl_index < bytes.len()
        && bytes[nl_index] == b'\n'
        && nl_index > 0
        && bytes[nl_index - 1] == b'\r'
    {
        "\r\n"
    } else {
        "\n"
    }
}

/// Where one `KEYWORD: <timestamp>` planning entry sits inside a line.
struct EntryLoc {
    /// Byte offset of the keyword's first character.
    kw_start: usize,
    /// The `<…>` timestamp following `KEYWORD:`, when present and parseable:
    /// `(ts_start, ts_end, repeater, delay)`. `None` when the keyword is
    /// present but its value is missing/unparseable (a replace still targets
    /// just past the colon; carry-over cookies are then absent).
    timestamp: Option<(usize, usize, Option<Repeater>, Option<Delay>)>,
}

/// Locate the `keyword` planning entry within `source[line_start..line_end]`.
/// Matches `KEYWORD` immediately followed by `:` (so `SCHEDULED` never matches
/// inside `RESCHEDULED`-style text only when colon-terminated), then the first
/// `<…>`/`[…]` timestamp after the colon.
fn find_entry(source: &str, line_start: usize, line_end: usize, keyword: &str) -> Option<EntryLoc> {
    let line = &source[line_start..line_end];
    let bytes = line.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = line[from..].find(keyword) {
        let kw_rel = from + rel;
        let after_kw = kw_rel + keyword.len();
        if line[after_kw..].starts_with(':') {
            // Skip whitespace after the colon to the timestamp opener.
            let mut ts_rel = after_kw + 1;
            while ts_rel < line.len() && (bytes[ts_rel] == b' ' || bytes[ts_rel] == b'\t') {
                ts_rel += 1;
            }
            let timestamp =
                if ts_rel < line.len() && (bytes[ts_rel] == b'<' || bytes[ts_rel] == b'[') {
                    let ts_start = line_start + ts_rel;
                    parse_at(&source[ts_start..line_end], ts_start)
                        .map(|ts| (ts_start, ts.span.end, ts.repeater, ts.delay))
                } else {
                    None
                };
            return Some(EntryLoc {
                kw_start: line_start + kw_rel,
                timestamp,
            });
        }
        from = after_kw;
    }
    None
}

/// Compute the byte-faithful [`PlanningEdit`] that sets (`new = Some`) or
/// removes (`new = None`) the `kind` planning timestamp on the headline whose
/// line begins at `headline_offset`.
///
/// Behavior:
/// - **Replace** an existing same-kind stamp: only the `<…>` bytes change; the
///   old repeater/delay cookie is carried onto the new stamp (re-picking a date
///   on a recurring task preserves `+1w`).
/// - **Add** to an existing planning line lacking the keyword: append
///   ` KEYWORD: <…>` at the line end.
/// - **Create** a planning line when none follows the headline.
/// - **Remove**: delete the entry (and a separating space); if the planning
///   line becomes blank, delete the whole line. Removing an absent entry is a
///   no-op edit.
///
/// The returned edit's `from..to` never extends past the planning line region,
/// so every other byte of `source` round-trips identically (FR-2). Panic-free;
/// an out-of-range `headline_offset` is clamped.
pub fn set_planning_timestamp(
    source: &str,
    headline_offset: usize,
    kind: PlanningKind,
    new: Option<PlannedStamp>,
) -> PlanningEdit {
    let offset = headline_offset.min(source.len());
    let headline_line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let headline_line_end = source[headline_line_start..]
        .find('\n')
        .map(|i| headline_line_start + i)
        .unwrap_or(source.len());

    // The line immediately after the headline is where planning lives.
    let after_headline = (headline_line_end + 1).min(source.len());
    let planning_line_end = source[after_headline..]
        .find('\n')
        .map(|i| after_headline + i)
        .unwrap_or(source.len());
    let planning_line = &source[after_headline..planning_line_end];
    let keyword = kind.keyword();

    if is_planning_line(planning_line) {
        match find_entry(source, after_headline, planning_line_end, keyword) {
            // Keyword already present with a parseable timestamp.
            Some(EntryLoc {
                timestamp: Some((ts_start, ts_end, repeater, delay)),
                kw_start,
            }) => match new {
                Some(stamp) => PlanningEdit {
                    from: ts_start,
                    to: ts_end,
                    // Carry the recurring cookie over onto the re-picked date.
                    insert: format_planning_timestamp(stamp, repeater, delay),
                },
                None => remove_entry(source, after_headline, planning_line_end, kw_start, ts_end),
            },
            // Keyword present but its value is missing/unparseable: rewrite from
            // just past the colon to the timestamp opener isn't known, so on a
            // set we insert the new stamp right after the keyword+colon; on a
            // remove we drop the keyword+colon token.
            Some(EntryLoc {
                timestamp: None,
                kw_start,
            }) => {
                let colon_end = kw_start + keyword.len() + 1;
                match new {
                    Some(stamp) => PlanningEdit {
                        from: colon_end,
                        to: colon_end,
                        insert: format!(" {}", format_planning_timestamp(stamp, None, None)),
                    },
                    None => remove_entry(
                        source,
                        after_headline,
                        planning_line_end,
                        kw_start,
                        colon_end,
                    ),
                }
            }
            // Planning line exists but without this keyword: append / no-op.
            None => match new {
                Some(stamp) => PlanningEdit {
                    from: planning_line_end,
                    to: planning_line_end,
                    insert: format!(
                        " {keyword}: {}",
                        format_planning_timestamp(stamp, None, None)
                    ),
                },
                None => noop(planning_line_end),
            },
        }
    } else {
        // No planning line follows the headline.
        match new {
            Some(stamp) => {
                let ts = format_planning_timestamp(stamp, None, None);
                if headline_line_end < source.len() {
                    // Insert a fresh planning line right after the headline.
                    let nl = newline_style(source, headline_line_end);
                    PlanningEdit {
                        from: after_headline,
                        to: after_headline,
                        insert: format!("{keyword}: {ts}{nl}"),
                    }
                } else {
                    // Headline is the last line with no trailing newline: add
                    // one before the planning line.
                    let nl = newline_style(source, headline_line_end);
                    PlanningEdit {
                        from: source.len(),
                        to: source.len(),
                        insert: format!("{nl}{keyword}: {ts}"),
                    }
                }
            }
            None => noop(after_headline),
        }
    }
}

/// A zero-width no-op edit at `at`.
fn noop(at: usize) -> PlanningEdit {
    PlanningEdit {
        from: at,
        to: at,
        insert: String::new(),
    }
}

/// Delete the entry spanning `[kw_start, entry_end)` on the planning line
/// `[line_start, line_end)`, swallowing one separating space so no dangling gap
/// remains; if the line is then blank, delete the whole line (with its
/// newline).
fn remove_entry(
    source: &str,
    line_start: usize,
    line_end: usize,
    kw_start: usize,
    entry_end: usize,
) -> PlanningEdit {
    let bytes = source.as_bytes();
    let mut rstart = kw_start;
    let mut rend = entry_end;
    // Prefer swallowing a space before the entry; else one after it.
    while rstart > line_start && (bytes[rstart - 1] == b' ' || bytes[rstart - 1] == b'\t') {
        rstart -= 1;
    }
    if rstart == line_start {
        while rend < line_end && (bytes[rend] == b' ' || bytes[rend] == b'\t') {
            rend += 1;
        }
    }
    let remaining = format!("{}{}", &source[line_start..rstart], &source[rend..line_end]);
    if remaining.trim().is_empty() {
        // Whole planning line goes, including its terminating newline.
        let del_end = (line_end + 1).min(source.len());
        PlanningEdit {
            from: line_start,
            to: del_end,
            insert: String::new(),
        }
    } else {
        PlanningEdit {
            from: rstart,
            to: rend,
            insert: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid test date")
    }

    /// Apply a [`PlanningEdit`] to `source`, returning the new buffer — mirrors
    /// what the CM6 transaction does on the TS side, so the round-trip
    /// assertions below exercise the real write.
    fn apply(source: &str, edit: &PlanningEdit) -> String {
        format!(
            "{}{}{}",
            &source[..edit.from],
            edit.insert,
            &source[edit.to..]
        )
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

    // ---- Story 4.8 (FR-9): date-shortcut resolver ----

    #[test]
    fn resolves_today_and_now() {
        let today = date(2026, 5, 19);
        assert_eq!(resolve_date_shortcut("today", today), Some(today));
        assert_eq!(resolve_date_shortcut("  Today ", today), Some(today));
        assert_eq!(resolve_date_shortcut("NOW", today), Some(today));
    }

    #[test]
    fn resolves_signed_day_and_week_shortcuts() {
        let today = date(2026, 5, 19); // Tue
        assert_eq!(resolve_date_shortcut("+1d", today), Some(date(2026, 5, 20)));
        assert_eq!(
            resolve_date_shortcut("+1", today),
            Some(date(2026, 5, 20)),
            "bare +N == days"
        );
        assert_eq!(resolve_date_shortcut("+1w", today), Some(date(2026, 5, 26)));
        assert_eq!(resolve_date_shortcut("-2d", today), Some(date(2026, 5, 17)));
        assert_eq!(
            resolve_date_shortcut("+0d", today),
            Some(today),
            "zero offset is allowed here"
        );
    }

    #[test]
    fn resolves_month_and_year_shortcuts_clamping_day() {
        // Jan 31 +1m clamps to Feb 28 (2026 is not a leap year).
        assert_eq!(
            resolve_date_shortcut("+1m", date(2026, 1, 31)),
            Some(date(2026, 2, 28))
        );
        assert_eq!(
            resolve_date_shortcut("+1y", date(2026, 5, 19)),
            Some(date(2027, 5, 19))
        );
        assert_eq!(
            resolve_date_shortcut("-1y", date(2026, 5, 19)),
            Some(date(2025, 5, 19))
        );
    }

    #[test]
    fn rejects_non_shortcuts() {
        let today = date(2026, 5, 19);
        assert_eq!(
            resolve_date_shortcut("2026-05-19", today),
            None,
            "literal date is not a shortcut"
        );
        assert_eq!(resolve_date_shortcut("+", today), None);
        assert_eq!(resolve_date_shortcut("+d", today), None, "missing value");
        assert_eq!(resolve_date_shortcut("tomorrow", today), None);
        assert_eq!(resolve_date_shortcut("", today), None);
    }

    // ---- Story 4.8 (FR-9): timestamp formatting ----

    #[test]
    fn formats_planning_timestamp_with_computed_weekday() {
        let stamp = PlannedStamp {
            date: date(2026, 5, 19), // a Tuesday
            time: None,
        };
        assert_eq!(
            format_planning_timestamp(stamp, None, None),
            "<2026-05-19 Tue>"
        );

        let with_time = PlannedStamp {
            date: date(2026, 5, 19),
            time: NaiveTime::from_hms_opt(14, 0, 0),
        };
        assert_eq!(
            format_planning_timestamp(with_time, None, None),
            "<2026-05-19 Tue 14:00>"
        );
    }

    #[test]
    fn formats_planning_timestamp_re_emits_cookies() {
        let stamp = PlannedStamp {
            date: date(2026, 5, 19),
            time: NaiveTime::from_hms_opt(9, 30, 0),
        };
        let rep = Some(Repeater {
            kind: RepeaterKind::Cumulate,
            value: 1,
            unit: TimeUnit::Week,
        });
        let del = Some(Delay {
            kind: DelayKind::All,
            value: 2,
            unit: TimeUnit::Day,
        });
        assert_eq!(
            format_planning_timestamp(stamp, rep, del),
            "<2026-05-19 Tue 09:30 +1w -2d>"
        );
    }

    // ---- Story 4.8 (FR-9): planning-line writer ----

    fn planned(y: i32, m: u32, d: u32, hm: Option<(u32, u32)>) -> PlannedStamp {
        PlannedStamp {
            date: date(y, m, d),
            time: hm.map(|(h, mn)| NaiveTime::from_hms_opt(h, mn, 0).expect("valid time")),
        }
    }

    #[test]
    fn creates_planning_line_when_none_exists() {
        let source = "* Task\nBody text\n";
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 5, 19, None)),
        );
        assert_eq!(
            apply(source, &edit),
            "* Task\nSCHEDULED: <2026-05-19 Tue>\nBody text\n"
        );
    }

    #[test]
    fn creates_deadline_line_on_headline_without_trailing_newline() {
        let source = "* Task"; // last line, no newline
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Deadline,
            Some(planned(2026, 5, 19, Some((17, 0)))),
        );
        assert_eq!(
            apply(source, &edit),
            "* Task\nDEADLINE: <2026-05-19 Tue 17:00>"
        );
    }

    #[test]
    fn replaces_existing_scheduled_and_preserves_body() {
        let source = "* Task\nSCHEDULED: <2026-05-19 Tue>\n:PROPERTIES:\n:ID: x\n:END:\n";
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 6, 1, None)),
        );
        assert_eq!(
            apply(source, &edit),
            "* Task\nSCHEDULED: <2026-06-01 Mon>\n:PROPERTIES:\n:ID: x\n:END:\n"
        );
    }

    #[test]
    fn replace_carries_recurring_cookie_over() {
        // AC: recurring timestamps preserved on round-trip. Re-picking a date
        // on a recurring task keeps the `+1w` cookie (and the `-2d` delay).
        let source = "* Weekly review\nSCHEDULED: <2026-05-19 Tue +1w -2d>\n";
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 5, 26, None)),
        );
        assert_eq!(
            apply(source, &edit),
            "* Weekly review\nSCHEDULED: <2026-05-26 Tue +1w -2d>\n"
        );
    }

    #[test]
    fn adds_second_keyword_to_existing_planning_line() {
        // A planning line already carries DEADLINE; adding SCHEDULED appends it
        // and leaves the existing entry byte-identical.
        let source = "* Task\nDEADLINE: <2026-05-30 Sat>\n";
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 5, 19, Some((10, 0)))),
        );
        assert_eq!(
            apply(source, &edit),
            "* Task\nDEADLINE: <2026-05-30 Sat> SCHEDULED: <2026-05-19 Tue 10:00>\n"
        );
    }

    #[test]
    fn removes_only_entry_drops_whole_planning_line() {
        let source = "* Task\nSCHEDULED: <2026-05-19 Tue>\nBody\n";
        let edit = set_planning_timestamp(source, 0, PlanningKind::Scheduled, None);
        assert_eq!(apply(source, &edit), "* Task\nBody\n");
    }

    #[test]
    fn removes_one_entry_keeps_sibling_on_line() {
        let source = "* Task\nDEADLINE: <2026-05-30 Sat> SCHEDULED: <2026-05-19 Tue>\n";
        let edit = set_planning_timestamp(source, 0, PlanningKind::Scheduled, None);
        assert_eq!(apply(source, &edit), "* Task\nDEADLINE: <2026-05-30 Sat>\n");
    }

    #[test]
    fn remove_absent_entry_is_noop() {
        let source = "* Task\nBody\n";
        let edit = set_planning_timestamp(source, 0, PlanningKind::Deadline, None);
        assert_eq!(edit.from, edit.to);
        assert_eq!(edit.insert, "");
        assert_eq!(
            apply(source, &edit),
            source,
            "no-op leaves the buffer identical"
        );
    }

    #[test]
    fn preserves_crlf_line_endings_on_insert() {
        let source = "* Task\r\nBody\r\n";
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 5, 19, None)),
        );
        assert_eq!(
            apply(source, &edit),
            "* Task\r\nSCHEDULED: <2026-05-19 Tue>\r\nBody\r\n"
        );
    }

    #[test]
    fn write_is_byte_faithful_and_reparses_via_analyze() {
        // End-to-end: writing SCHEDULED then re-analyzing yields the timestamp,
        // and only the planning line changed. Also proves the second headline is
        // untouched byte-for-byte.
        let source = "* One\nsome body\n* Two\nSCHEDULED: <2026-01-01 Thu>\n";
        // Offset of the FIRST headline is 0.
        let edit = set_planning_timestamp(
            source,
            0,
            PlanningKind::Scheduled,
            Some(planned(2026, 5, 19, Some((9, 0)))),
        );
        let out = apply(source, &edit);
        assert_eq!(
            out,
            "* One\nSCHEDULED: <2026-05-19 Tue 09:00>\nsome body\n* Two\nSCHEDULED: <2026-01-01 Thu>\n"
        );
        let doc = crate::analyze(&out).expect("analyze is total");
        let first = &doc.headlines[0];
        let sched = first.scheduled.as_ref().expect("scheduled parsed back");
        assert_eq!(sched.date, date(2026, 5, 19));
        assert_eq!(sched.time, NaiveTime::from_hms_opt(9, 0, 0));
    }

    #[test]
    fn targets_the_headline_at_the_given_offset() {
        // Offset points at the SECOND headline; only its planning gets written.
        let source = "* One\n* Two\nBody two\n";
        let two_offset = source.find("* Two").expect("has second headline");
        let edit = set_planning_timestamp(
            source,
            two_offset,
            PlanningKind::Deadline,
            Some(planned(2026, 5, 19, None)),
        );
        assert_eq!(
            apply(source, &edit),
            "* One\n* Two\nDEADLINE: <2026-05-19 Tue>\nBody two\n"
        );
    }
}
