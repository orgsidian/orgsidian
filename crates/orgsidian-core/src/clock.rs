//! Implements FR-8 (functional)
//!
//! The Clock manager: clock in / out / resume, persisting entries as standard
//! org `CLOCK:` lines in the target headline's `:LOGBOOK:` drawer, plus a
//! per-Vault Active-Clock pointer at `<Vault>/.orgsidian/active-clock.json`
//! that Stories 7.7 (stale-clock prompt) and 7.8 (ClockEditor) build on.
//!
//! # The org file is the source of truth
//!
//! Every `CLOCK:` mutation is a **byte-faithful text splice** into the file's
//! source string, then [`atomic_write`] — never a re-render through the parser
//! serializer (it is raw-passthrough and ignores mutated semantic fields). A
//! clock-in inserts an open `CLOCK: [start]` line under `:LOGBOOK:` (creating
//! the drawer if absent); a clock-out closes the matching open line to
//! `CLOCK: [start]--[end] => HH:MM`; a resume re-activates the most-recent
//! unclosed line without touching the source. The notify-watcher reindexes on
//! the disk change, so no in-command index resync is needed.
//!
//! # Active-clock pointer
//!
//! [`active-clock.json`](active_clock_path) persists EXACTLY
//! `{ headline_id, started_at, last_active_at }` (headline_id = the app-wide
//! index-rowid identity; timestamps as `%Y-%m-%dT%H:%M:%S` strings — chrono
//! has no `serde` feature in this workspace, so they are serialized manually).
//! At most ONE clock is active at a time: clocking into a new headline
//! auto-stops the prior one first. `last_active_at` MUST be refreshed on every
//! window-focus/foreground event (Story 7.7/7.8 depend on it) — see
//! [`refresh_active_clock`]. It starts equal to `started_at`.
//!
//! # Injected wall clock
//!
//! `now` is dependency-injected into every core function as a
//! [`NaiveDateTime`]; the Tauri command layer supplies
//! `chrono::Local::now().naive_local()`. Core never reads the wall clock
//! itself — matching the starter-vault "today is injected" convention and
//! keeping core deterministic/testable.
//!
//! # Per-Vault JSON pattern
//!
//! The `.orgsidian/` dir + `atomic_write` pretty-JSON + default-on-missing +
//! self-heal-on-parse-error shape copies [`crate::coaching`] (the same LEAF
//! façade `atomic_write` + `OrgError::Io` mapping).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike};

use orgsidian_vault::atomic_write;

use crate::error::OrgError;
use crate::parser::analyze;
use crate::parser::semantic::{ClockEntry, DrawerKind, Headline};
use crate::Result as OrgResult;

/// The per-Vault dotfile directory (LD-40), same as [`crate::coaching`].
const VAULT_DOTDIR: &str = ".orgsidian";
/// The Active-Clock pointer file name.
const ACTIVE_CLOCK_FILE: &str = "active-clock.json";
/// The timestamp format for `active-clock.json` (chrono has no `serde` here).
const TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// The persisted Active-Clock pointer — EXACTLY the three fields Stories
/// 7.7/7.8 read. `started_at`/`last_active_at` are `%Y-%m-%dT%H:%M:%S` strings.
///
/// `headline_id` is the app-wide index-rowid identity, narrowed to `u32` (the
/// IPC-boundary narrowing convention; a Vault's headline count never
/// approaches 4 billion).
/// The on-disk keys are snake_case (`headline_id`, `started_at`,
/// `last_active_at`) — this is a Rust-only per-Vault sidecar file the AC
/// mandates by that exact shape, NOT an IPC type (the wire type is
/// `ActiveClockDto` in the shell crate, which keeps camelCase). So there is
/// deliberately no `#[serde(rename_all = "camelCase")]` here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActiveClock {
    /// `headlines.id` — the tracked headline's app-wide identity.
    pub headline_id: u32,
    /// Clock-in timestamp (`%Y-%m-%dT%H:%M:%S`); equals the `CLOCK:` line start.
    pub started_at: String,
    /// Last time the app was known alive with this clock — bumped on every
    /// window-focus (Story 7.7 pre-fills its "adjust end time" from this).
    /// Starts equal to `started_at`.
    pub last_active_at: String,
}

// ---------------------------------------------------------------------------
// Active-clock pointer I/O (the `coaching.rs` per-Vault JSON pattern)
// ---------------------------------------------------------------------------

/// Resolve the Active-Clock pointer path:
/// `<vault>/.orgsidian/active-clock.json`. Pure path arithmetic — no I/O.
pub fn active_clock_path(vault_root: &Path) -> PathBuf {
    vault_root.join(VAULT_DOTDIR).join(ACTIVE_CLOCK_FILE)
}

/// Map any I/O or (de)serialization failure to [`OrgError::Io`] — same
/// disk-concern mapping [`crate::coaching`] uses.
fn clock_io(err: impl std::fmt::Display) -> OrgError {
    OrgError::Io {
        reason: format!("active-clock store: {err}"),
    }
}

/// Read the Active-Clock pointer for `vault_root`, or `None` when there is no
/// active clock (the file does not exist — including "no vault dotdir yet").
///
/// Self-heals a malformed file: a pointer that exists but fails to parse as
/// JSON (hand-edited, crash mid-write) is treated as absent (`Ok(None)`)
/// rather than trapping every command behind a permanent parse error — the
/// next [`clock_in`]/[`clock_out`] overwrites it. Non-parse I/O errors
/// (permission denied, etc.) still propagate.
pub fn active_clock(vault_root: &Path) -> OrgResult<Option<ActiveClock>> {
    let path = active_clock_path(vault_root);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(clock_io(source)),
    };
    match serde_json::from_str(&raw) {
        Ok(clock) => Ok(Some(clock)),
        Err(_parse_err) => Ok(None),
    }
}

/// Persist `clock` as the active pointer, creating `<vault>/.orgsidian/` if
/// absent.
fn write_active_clock(vault_root: &Path, clock: &ActiveClock) -> OrgResult<()> {
    let path = active_clock_path(vault_root);
    let dir = path.parent().expect(".orgsidian path always has a parent");
    fs::create_dir_all(dir).map_err(clock_io)?;
    let body = serde_json::to_string_pretty(clock).map_err(clock_io)?;
    atomic_write(&path, body.as_bytes()).map_err(clock_io)?;
    Ok(())
}

/// Remove the active pointer. A missing file is not an error (idempotent).
fn remove_active_clock(vault_root: &Path) -> OrgResult<()> {
    let path = active_clock_path(vault_root);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(clock_io(source)),
    }
}

// ---------------------------------------------------------------------------
// Pure timestamp / duration / CLOCK-line formatting
// ---------------------------------------------------------------------------

/// `%Y-%m-%dT%H:%M:%S` string for the `active-clock.json` timestamps.
fn format_ts(dt: NaiveDateTime) -> String {
    dt.format(TS_FORMAT).to_string()
}

/// Parse a `%Y-%m-%dT%H:%M:%S` `active-clock.json` timestamp.
fn parse_ts(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, TS_FORMAT).ok()
}

/// Truncate a datetime to whole-minute resolution (zero seconds AND
/// nanoseconds). Org `CLOCK:` stamps and the `active-clock.json` timestamps are
/// both minute-precision (`%H:%M`), but the injected wall clock
/// (`Local::now().naive_local()`) carries sub-minute components — normalizing
/// `now` at every entry point keeps `started_at`, the written CLOCK line, and
/// [`clock_out`]'s `started_at` match all at the same resolution, so the match
/// never spuriously misses (and orphans the open line down the desync branch).
fn truncate_to_minute(dt: NaiveDateTime) -> NaiveDateTime {
    dt.with_second(0)
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}

/// Midnight, for combining a date-only `CLOCK:` stamp into a datetime.
fn midnight() -> NaiveTime {
    NaiveTime::from_hms_opt(0, 0, 0).expect("00:00:00 is a valid time")
}

/// Format an INACTIVE org stamp `[YYYY-MM-DD Day HH:MM]` — the form org uses
/// for `CLOCK:` lines. `%a` is chrono's locale-independent English weekday, so
/// a runner's locale never changes the bytes written (matches
/// `format_planning_timestamp`'s discipline).
fn format_inactive_stamp(dt: NaiveDateTime) -> String {
    dt.format("[%Y-%m-%d %a %H:%M]").to_string()
}

/// Format an org clock duration `H:MM` (hours unbounded, minutes zero-padded
/// 00-59), computed from a [`TimeDelta`]. Sub-minute components round DOWN to
/// whole minutes (`num_minutes` truncates); a negative delta clamps to `0:00`.
fn format_duration(delta: TimeDelta) -> String {
    let total_minutes = delta.num_minutes().max(0);
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    format!("{hours}:{minutes:02}")
}

// ---------------------------------------------------------------------------
// Pure source-splicing helpers
// ---------------------------------------------------------------------------

/// Replace `source[from..to]` with `insert`, leaving every other byte
/// identical (the FR-2 round-trip contract holds for the rest of the file).
fn splice(source: &str, from: usize, to: usize, insert: &str) -> String {
    let mut out = String::with_capacity(source.len() + insert.len());
    out.push_str(&source[..from.min(source.len())]);
    out.push_str(insert);
    out.push_str(&source[to.min(source.len())..]);
    out
}

/// Byte offset of the start of the line containing `offset`.
fn line_start(source: &str, offset: usize) -> usize {
    let o = offset.min(source.len());
    source[..o].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Byte offset just past the newline terminating the line that `offset` sits
/// on (the start of the next line), or `source.len()` if that line has no
/// trailing newline.
fn next_line_start(source: &str, offset: usize) -> usize {
    let o = offset.min(source.len());
    match source[o..].find('\n') {
        Some(i) => o + i + 1,
        None => source.len(),
    }
}

/// The leading whitespace (indent) of the line containing `offset`.
fn line_indent(source: &str, offset: usize) -> String {
    let start = line_start(source, offset);
    let line = &source[start..];
    let end = line
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(line.len());
    line[..end].to_string()
}

/// The file's newline style — `"\r\n"` if the source uses CRLF anywhere, else
/// `"\n"` — so an inserted line matches existing line endings.
fn detect_newline(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// True when `line` is an org planning line (first token is a planning
/// keyword). Trailing `\r` (CRLF) is irrelevant here.
fn is_planning_line(line: &str) -> bool {
    let head = line.trim_start();
    head.starts_with("SCHEDULED:") || head.starts_with("DEADLINE:") || head.starts_with("CLOSED:")
}

// ---------------------------------------------------------------------------
// Headline / clock lookups
// ---------------------------------------------------------------------------

/// Convert a signed index value (byte offset / ordinal) to `usize`, clamping a
/// (never-produced) negative to 0.
fn to_usize(v: i64) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// Find the headline at document-order `ordinal` (0-based) among `headlines`,
/// counted in pre-order — a headline before its children, siblings in order —
/// the same order the index counts for [`crate::index::HeadlineLocation`]'s
/// `ordinal` (headlines sorted by `byte_start`).
///
/// This is the STABLE re-location identity the clock manager uses in place of
/// the index's absolute `byte_start`: `CLOCK:` edits never add, remove, or
/// reorder headlines, so a headline keeps its ordinal even as its byte offset
/// shifts — whereas `byte_start` goes stale after any in-session clock write to
/// the file (the in-process index is never resynced between commands, so it
/// still reports the last-scan offsets).
fn find_headline_by_ordinal(headlines: &[Headline], ordinal: usize) -> Option<&Headline> {
    fn walk<'a>(
        headlines: &'a [Headline],
        target: usize,
        seen: &mut usize,
    ) -> Option<&'a Headline> {
        for h in headlines {
            if *seen == target {
                return Some(h);
            }
            *seen += 1;
            if let Some(found) = walk(&h.children, target, seen) {
                return Some(found);
            }
        }
        None
    }
    walk(headlines, ordinal, &mut 0)
}

/// The clock-in start datetime of `entry` (combining its date with its time,
/// defaulting a date-only stamp to midnight).
fn clock_start_dt(entry: &ClockEntry) -> NaiveDateTime {
    entry
        .start
        .date
        .and_time(entry.start.time.unwrap_or_else(midnight))
}

/// This headline's own OPEN (unclosed) CLOCK entry whose start matches
/// `started` — the target [`clock_out`] closes, located by `started_at`
/// (robust to byte-offset shifts) rather than a stale byte offset.
fn find_open_clock(headline: &Headline, started: NaiveDateTime) -> Option<&ClockEntry> {
    headline
        .clocks
        .iter()
        .find(|c| c.end.is_none() && clock_start_dt(c) == started)
}

/// The OPEN CLOCK entry matching `started` anywhere in the file's headline
/// tree. [`clock_out`] searches the whole file by `started_at` rather than the
/// index's (possibly stale, since our own last write shifted offsets) headline
/// `byte_start`, per the story's "locate by matching started_at, never by a
/// stale byte offset" constraint.
fn find_open_clock_in_tree(headlines: &[Headline], started: NaiveDateTime) -> Option<&ClockEntry> {
    for h in headlines {
        if let Some(entry) = find_open_clock(h, started) {
            return Some(entry);
        }
        if let Some(entry) = find_open_clock_in_tree(&h.children, started) {
            return Some(entry);
        }
    }
    None
}

/// This headline's most-recent OPEN CLOCK entry (the latest clock-in that was
/// never closed) — what [`clock_resume`] re-activates.
fn most_recent_open_clock(headline: &Headline) -> Option<&ClockEntry> {
    headline
        .clocks
        .iter()
        .filter(|c| c.end.is_none())
        .max_by_key(|c| clock_start_dt(c))
}

// ---------------------------------------------------------------------------
// LOGBOOK insertion (pure)
// ---------------------------------------------------------------------------

/// Compute the byte-faithful clock-in splice for `headline`: where to insert
/// (a zero-width `from == to` position) and the text to insert.
///
/// - **Existing `:LOGBOOK:`**: the open `CLOCK:` line is inserted immediately
///   after the `:LOGBOOK:` header line (org prepends newest), matching the
///   drawer's indentation.
/// - **No `:LOGBOOK:`**: a whole drawer is synthesized
///   (`:LOGBOOK:` / `CLOCK:` / `:END:`) after the `:PROPERTIES:` drawer if
///   present, else after the planning line if present, else right after the
///   headline line — indented to match `:PROPERTIES:` when one exists, else no
///   indent.
fn compute_clock_in_edit(source: &str, headline: &Headline, now: NaiveDateTime) -> (usize, String) {
    let nl = detect_newline(source);
    let stamp = format_inactive_stamp(now);

    if let Some(logbook) = headline
        .drawers
        .iter()
        .find(|d| d.kind == DrawerKind::Logbook)
    {
        let indent = line_indent(source, logbook.span.start);
        let insert_at = next_line_start(source, logbook.span.start);
        let text = format!("{indent}CLOCK: {stamp}{nl}");
        (insert_at, text)
    } else {
        let (insert_at, indent) = logbook_insert_anchor(source, headline);
        let text = format!("{indent}:LOGBOOK:{nl}{indent}CLOCK: {stamp}{nl}{indent}:END:{nl}");
        (insert_at, text)
    }
}

/// Where a synthesized `:LOGBOOK:` drawer goes for `headline`, and the indent
/// to give it: after `:PROPERTIES:` (org's canonical order puts LOGBOOK there,
/// which is also after any planning line), else after the planning line, else
/// right after the headline line.
fn logbook_insert_anchor(source: &str, headline: &Headline) -> (usize, String) {
    if let Some(props) = headline
        .drawers
        .iter()
        .find(|d| d.kind == DrawerKind::Properties)
    {
        let indent = line_indent(source, props.span.start);
        // Anchor from the last byte of `:END:` (span.end - 1) so we land on
        // that line whether or not the drawer span includes its trailing
        // newline, then take the start of the following line.
        let anchor = props.span.end.saturating_sub(1);
        return (next_line_start(source, anchor), indent);
    }

    let headline_line_end = next_line_start(source, headline.span.start);
    let planning_line_end = next_line_start(source, headline_line_end);
    let planning_line = &source[headline_line_end.min(source.len())..planning_line_end];
    if is_planning_line(planning_line) {
        return (planning_line_end, String::new());
    }

    (headline_line_end, String::new())
}

/// Neutralize `headline`'s DUPLICATE open `CLOCK:` lines — every open line
/// EXCEPT the one starting at byte `keep_span_start` (the adopted, most-recent
/// open line) — by closing each to its OWN start `=> 0:00`, a zero-duration
/// splice appended at the line's `span.end` (matching [`clock_out`]'s close
/// shape). Splices are applied in DESCENDING `span.end` order so each edit's
/// offset stays valid against the not-yet-spliced tail — the same reverse-order
/// discipline a multi-edit byte splice needs. Returns the new source; the
/// adopted line and every other byte are left untouched.
fn close_extra_open_lines(source: &str, headline: &Headline, keep_span_start: usize) -> String {
    let mut extras: Vec<&ClockEntry> = headline
        .clocks
        .iter()
        .filter(|c| c.end.is_none() && c.span.start != keep_span_start)
        .collect();
    // Descending by end offset: splice the latest line first so earlier offsets
    // do not shift under us.
    extras.sort_by_key(|c| std::cmp::Reverse(c.span.end));

    let mut out = source.to_string();
    for entry in extras {
        let start = clock_start_dt(entry);
        let insert = format!(
            "--{} => {}",
            format_inactive_stamp(start),
            format_duration(TimeDelta::zero())
        );
        out = splice(&out, entry.span.end, entry.span.end, &insert);
    }
    out
}

// ---------------------------------------------------------------------------
// Clock manager (index-backed)
// ---------------------------------------------------------------------------

/// The [`OrgError::Vault`] returned when a headline resolved in the index could
/// not be found in its freshly-read source (an index/source desync).
fn headline_not_found_err(headline_id: u32, location: &crate::index::HeadlineLocation) -> OrgError {
    OrgError::Vault {
        reason: format!(
            "headline {headline_id} (document-order #{}) was not found in {}",
            location.ordinal, location.file_path
        ),
    }
}

/// Read a file's source, mapping I/O failures to [`OrgError::Io`].
fn read_source(path: &Path) -> OrgResult<String> {
    fs::read_to_string(path).map_err(|err| OrgError::Io {
        reason: format!("failed to read {}: {err}", path.display()),
    })
}

/// Analyze `source`, mapping the (never-firing) parse error to
/// [`OrgError::Parse`].
fn analyze_source(source: &str, file: &str) -> OrgResult<crate::parser::semantic::Document> {
    analyze(source).map_err(|err| OrgError::Parse {
        file: file.to_string(),
        reason: err.to_string(),
    })
}

/// Clock IN on `headline_id` (FR-8): auto-stop any prior active clock, then
/// insert an open `CLOCK: [now]` line into the headline's `:LOGBOOK:` (drawer
/// created if absent) and write the Active-Clock pointer. At most one clock is
/// ever active.
///
/// # Story 7.7 adopt-guard
///
/// If the target headline ALREADY owns an open `CLOCK:` line (its pointer was
/// lost — e.g. a prior-session clock this headline still carries), this does
/// NOT insert a second open line. Instead it ADOPTS the most-recent open line —
/// writing the pointer to that line's start and leaving the source untouched —
/// and NEUTRALIZES any older duplicate open lines by closing each to its own
/// start `=> 0:00`, so exactly one open line (the adopted one) remains. The
/// fresh-insert path above runs only when the headline has no open line.
///
/// # Errors
///
/// [`OrgError::Index`] if no index exists; [`OrgError::Vault`] if the headline
/// is not in the index; [`OrgError::Io`]/[`OrgError::Parse`] on file access.
pub async fn clock_in(
    vault_root: &Path,
    headline_id: u32,
    now: NaiveDateTime,
) -> OrgResult<ActiveClock> {
    let now = truncate_to_minute(now);

    let location = crate::index::locate_headline(vault_root, i64::from(headline_id))
        .await?
        .ok_or_else(|| OrgError::Vault {
            reason: format!("headline {headline_id} is not in the index"),
        })?;
    let file_path = vault_root.join(&location.file_path);

    // At most one active clock: auto-stop the prior one first (the epic's
    // "clocking into a new headline auto-stops the prior active clock").
    //
    // Best-effort, but NOT error-swallowing: a benign desync (`OrgError::Vault`
    // — the prior pointer was stale/already closed; `clock_out` clears it) is
    // expected and we proceed with the switch, but a genuine I/O or parse
    // failure while closing the prior line MUST abort the switch rather than
    // silently lose the old open line.
    if active_clock(vault_root)?.is_some() {
        if let Err(err) = clock_out(vault_root, now).await {
            if !matches!(err, OrgError::Vault { .. }) {
                return Err(err);
            }
        }
    }

    // Re-read the file and locate the target by its STABLE document-order
    // ordinal — never the index's absolute `byte_start`, which is stale after
    // any in-session clock write to this file (the auto-stop just now, or an
    // earlier clock-in on another headline in the same file): the in-process
    // index is never resynced between commands, so it still reports last-scan
    // offsets, while the ordinal survives every `CLOCK:` edit. This is what
    // makes clocking B while A is active in the SAME file work.
    let source = read_source(&file_path)?;
    let doc = analyze_source(&source, &location.file_path)?;
    let headline = find_headline_by_ordinal(&doc.headlines, to_usize(location.ordinal))
        .ok_or_else(|| headline_not_found_err(headline_id, &location))?;

    // Story 7.7 deferred guard: the target may ALREADY own an open `CLOCK:` line
    // (its pointer was lost — e.g. a prior-session clock this Headline still
    // carries, or a hand-authored open line). Inserting a fresh one would orphan
    // the existing open line (a silently double-counted session). Instead ADOPT
    // the most-recent open line (write the pointer to its start, no new line),
    // and neutralize any OLDER duplicate open lines by closing each to its own
    // start `=> 0:00` — so exactly one open line remains, the adopted one.
    if let Some(adopt) = most_recent_open_clock(headline) {
        let adopt_start = clock_start_dt(adopt);
        let adopt_span_start = adopt.span.start;
        let has_extras = headline
            .clocks
            .iter()
            .any(|c| c.end.is_none() && c.span.start != adopt_span_start);
        if has_extras {
            let new_source = close_extra_open_lines(&source, headline, adopt_span_start);
            atomic_write(&file_path, new_source.as_bytes()).map_err(clock_io)?;
        }
        let started = format_ts(adopt_start);
        let clock = ActiveClock {
            headline_id,
            started_at: started.clone(),
            last_active_at: started,
        };
        write_active_clock(vault_root, &clock)?;
        return Ok(clock);
    }

    let (insert_at, text) = compute_clock_in_edit(&source, headline, now);
    let new_source = splice(&source, insert_at, insert_at, &text);
    atomic_write(&file_path, new_source.as_bytes()).map_err(clock_io)?;

    let started = format_ts(now);
    let clock = ActiveClock {
        headline_id,
        started_at: started.clone(),
        last_active_at: started,
    };
    write_active_clock(vault_root, &clock)?;
    Ok(clock)
}

/// Clock OUT the active clock (FR-8): close its matching open `CLOCK:` line to
/// `CLOCK: [start]--[now] => HH:MM` and remove the Active-Clock pointer.
///
/// The open line is located by matching `started_at` (robust to byte-offset
/// shifts), never a stale byte offset. If no matching open line is found the
/// pointer is removed (never left dangling) and an [`OrgError::Vault`]
/// describing the desync is returned.
///
/// # Errors
///
/// [`OrgError::Vault`] when there is no active clock, or on a source/pointer
/// desync (pointer cleared); [`OrgError::Index`]/[`OrgError::Io`]/
/// [`OrgError::Parse`] on index/file access.
pub async fn clock_out(vault_root: &Path, now: NaiveDateTime) -> OrgResult<()> {
    let now = truncate_to_minute(now);

    let active = match active_clock(vault_root)? {
        Some(a) => a,
        None => {
            return Err(OrgError::Vault {
                reason: "no active clock to clock out".to_string(),
            })
        }
    };
    // Self-heal an unparseable `started_at` (hand-edited/legacy pointer): clear
    // it and treat as no active clock, rather than erroring on every future
    // call. Truncate to minute so a legacy seconds-bearing value still matches
    // the minute-precision CLOCK line.
    let started = match parse_ts(&active.started_at) {
        Some(dt) => truncate_to_minute(dt),
        None => {
            remove_active_clock(vault_root)?;
            return Err(OrgError::Vault {
                reason: format!(
                    "active clock has an unparseable started_at {:?}; cleared the pointer",
                    active.started_at
                ),
            });
        }
    };

    let location =
        match crate::index::locate_headline(vault_root, i64::from(active.headline_id)).await? {
            Some(loc) => loc,
            None => {
                remove_active_clock(vault_root)?;
                return Err(OrgError::Vault {
                    reason: format!(
                        "active clock references headline {} which is no longer in the index; \
                     cleared the dangling active-clock pointer",
                        active.headline_id
                    ),
                });
            }
        };

    let file_path = vault_root.join(&location.file_path);
    let source = read_source(&file_path)?;
    let doc = analyze_source(&source, &location.file_path)?;
    // Close ONLY the active headline's own open line: scope the `started_at`
    // match to the headline the pointer references (resolved by its stable
    // document-order ordinal, not the stale index `byte_start`), so a stray
    // unclosed `CLOCK:` elsewhere in the same file at a coincidentally identical
    // minute is never closed by mistake. A missing line under the resolved
    // headline is a genuine desync, NOT an excuse to close another headline's
    // line; only when the ordinal fails to resolve at all (headline count
    // changed out from under the index) do we fall back to the whole-tree
    // `started_at` search — still matched by `started_at`, never by an offset.
    let entry = match find_headline_by_ordinal(&doc.headlines, to_usize(location.ordinal)) {
        Some(headline) => find_open_clock(headline, started),
        None => find_open_clock_in_tree(&doc.headlines, started),
    };

    let entry = match entry {
        Some(e) => e,
        None => {
            remove_active_clock(vault_root)?;
            return Err(OrgError::Vault {
                reason: format!(
                    "no open CLOCK line starting at {} found in {}; cleared the dangling \
                     active-clock pointer",
                    active.started_at, location.file_path
                ),
            });
        }
    };

    let end_stamp = format_inactive_stamp(now);
    let duration = format_duration(now - started);
    let insert = format!("--{end_stamp} => {duration}");
    let new_source = splice(&source, entry.span.end, entry.span.end, &insert);
    atomic_write(&file_path, new_source.as_bytes()).map_err(clock_io)?;
    remove_active_clock(vault_root)?;
    Ok(())
}

/// Clock RESUME on `headline_id` (FR-8): re-activate the headline's
/// most-recent unclosed `CLOCK:` line as the active clock WITHOUT mutating the
/// source. If the headline has no unclosed line, this falls back to a fresh
/// [`clock_in`]. Any other headline's active clock is auto-stopped first (at
/// most one active clock).
///
/// # Errors
///
/// As [`clock_in`]/[`clock_out`].
pub async fn clock_resume(
    vault_root: &Path,
    headline_id: u32,
    now: NaiveDateTime,
) -> OrgResult<ActiveClock> {
    let now = truncate_to_minute(now);

    let location = crate::index::locate_headline(vault_root, i64::from(headline_id))
        .await?
        .ok_or_else(|| OrgError::Vault {
            reason: format!("headline {headline_id} is not in the index"),
        })?;

    let file_path = vault_root.join(&location.file_path);
    let source = read_source(&file_path)?;
    let doc = analyze_source(&source, &location.file_path)?;
    // Locate by the stable ordinal, not the stale index `byte_start` (see
    // `clock_in`): a prior in-session clock write to this file leaves the index
    // reporting last-scan offsets.
    let resumed_start = find_headline_by_ordinal(&doc.headlines, to_usize(location.ordinal))
        .and_then(most_recent_open_clock)
        .map(clock_start_dt);

    match resumed_start {
        Some(start_dt) => {
            // Enforce the single-active-clock invariant: stop any OTHER
            // headline's active clock first (never our own — that would close
            // the very line we are resuming). Best-effort: a desynced prior
            // pointer must not block the resume (clock_out clears it anyway).
            if let Some(existing) = active_clock(vault_root)? {
                if existing.headline_id != headline_id {
                    let _ = clock_out(vault_root, now).await;
                }
            }
            let started = format_ts(start_dt);
            let clock = ActiveClock {
                headline_id,
                started_at: started.clone(),
                last_active_at: started,
            };
            write_active_clock(vault_root, &clock)?;
            Ok(clock)
        }
        // No unclosed line to resume → start a fresh clock-in.
        None => clock_in(vault_root, headline_id, now).await,
    }
}

/// Refresh the active clock's `last_active_at` to `now` (FR-8; called on every
/// window-focus/foreground event so Stories 7.7/7.8 have a fresh "last known
/// alive" timestamp). A silent no-op when there is no active clock (or no
/// Vault dotdir yet).
///
/// # Errors
///
/// [`OrgError::Io`] only on a genuine pointer read/write failure (permission
/// denied); an absent pointer is `Ok(())`.
pub fn refresh_active_clock(vault_root: &Path, now: NaiveDateTime) -> OrgResult<()> {
    let now = truncate_to_minute(now);
    if let Some(mut clock) = active_clock(vault_root)? {
        clock.last_active_at = format_ts(now);
        write_active_clock(vault_root, &clock)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Stale-clock prompt (Story 7.7) — launch summary + discard transition
// ---------------------------------------------------------------------------

/// A prior-session running clock surfaced at launch (Story 7.7 / UJ-1 edge
/// case): the tracked Headline, its pointer timestamps, and the two candidate
/// durations the modal offers. `keep_duration` is `now - started_at` (what
/// "Keep tracking" would have accrued); `adjust_duration` is
/// `last_active_at - started_at` (what "Adjust end time" pre-fills). Both are
/// org `H:MM` strings (negative clamps `0:00`), formatted by [`format_duration`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleClockSummary {
    /// `headlines.id` — the tracked Headline's app-wide identity.
    pub headline_id: u32,
    /// The tracked Headline's display title.
    pub headline: String,
    /// The normalized clock-in timestamp (`%Y-%m-%dT%H:%M:%S`, minute-truncated).
    pub started_at: String,
    /// The normalized last-active timestamp (`%Y-%m-%dT%H:%M:%S`,
    /// minute-truncated; a malformed pointer value falls back to `started_at`)
    /// — the "adjust end time" pre-fill, always valid for frontend slicing.
    pub last_active_at: String,
    /// `now - started_at`, `H:MM` (the "Keep tracking" running total).
    pub keep_duration: String,
    /// `last_active_at - started_at`, `H:MM` (the "Adjust end time" total).
    pub adjust_duration: String,
}

/// The outcome of a launch-time stale-clock check (Story 7.7). A prior-session
/// pointer resolves to one of these; `None` from [`stale_clock_summary`] means
/// there is nothing to prompt at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleClock {
    /// A fully-resolved prior-session clock — the normal three-action prompt.
    Summary(StaleClockSummary),
    /// The pointer is present but its `headline_id` is no longer in the index
    /// (an index/source desync). The full summary cannot be built — there is no
    /// title or duration to show — but the pointer is real and the open line is
    /// still discardable, so the caller offers a DISCARD-ONLY recovery to clear
    /// the stuck `active-clock.json` in-app rather than hiding the prompt (which
    /// would strand the pointer with no in-app recovery). Carries the orphaned
    /// `headline_id` for diagnostics.
    Desynced {
        /// The pointer's `headline_id` — no headline in the index carries it.
        headline_id: u32,
    },
}

/// Summarize a prior-session Active Clock for the launch prompt (Story 7.7).
/// `None` when there is no active clock (nothing to prompt). Otherwise resolve
/// the tracked Headline's title from the index and compute both candidate
/// durations against the injected `now`, returning [`StaleClock::Summary`].
///
/// When the pointer's `headline_id` is no longer in the index (an index/source
/// desync) the summary cannot be built, but the pointer is still present and
/// discardable, so this returns [`StaleClock::Desynced`] (NOT an error) — the
/// caller renders a discard-only recovery so [`clock_discard`] is reachable and
/// the stuck pointer can be cleared in-app.
///
/// # Errors
///
/// [`OrgError::Vault`] on an unparseable `started_at` (a hand-edited/legacy
/// pointer; self-heals on the next clock mutation). [`OrgError::Index`]/
/// [`OrgError::Io`] on index/file access.
pub async fn stale_clock_summary(
    vault_root: &Path,
    now: NaiveDateTime,
) -> OrgResult<Option<StaleClock>> {
    let now = truncate_to_minute(now);

    let active = match active_clock(vault_root)? {
        Some(a) => a,
        None => return Ok(None),
    };

    let started = parse_ts(&active.started_at)
        .map(truncate_to_minute)
        .ok_or_else(|| OrgError::Vault {
            reason: format!(
                "active clock has an unparseable started_at {:?}",
                active.started_at
            ),
        })?;
    // A malformed `last_active_at` falls back to `started_at` (adjust = 0:00)
    // rather than trapping the whole prompt behind a parse error.
    let last_active = parse_ts(&active.last_active_at)
        .map(truncate_to_minute)
        .unwrap_or(started);

    // A `headline_id` no longer in the index is a caller-recoverable DESYNC, not
    // an error: the pointer (and its open CLOCK line) still exist and can be
    // discarded. Surface it as `Desynced` so the launch prompt can offer a
    // discard-only recovery instead of silently stranding the pointer.
    let headline =
        match crate::index::headline_title(vault_root, i64::from(active.headline_id)).await? {
            Some(title) => title,
            None => {
                return Ok(Some(StaleClock::Desynced {
                    headline_id: active.headline_id,
                }))
            }
        };

    Ok(Some(StaleClock::Summary(StaleClockSummary {
        headline_id: active.headline_id,
        headline,
        keep_duration: format_duration(now - started),
        adjust_duration: format_duration(last_active - started),
        // Return NORMALIZED (parsed-or-fallback) timestamps, never the raw
        // pointer strings: a malformed `last_active_at` would otherwise slice
        // garbage into the Adjust picker's date/time prefill. Both are now
        // always valid `%Y-%m-%dT%H:%M:%S`.
        started_at: format_ts(started),
        last_active_at: format_ts(last_active),
    })))
}

/// Clock DISCARD (Story 7.7): remove the active clock's OPEN `CLOCK:` line from
/// its `:LOGBOOK:` entirely — the "Discard this session" action, for a clock
/// left running across sessions that the user wants to drop rather than record.
/// Locates the open line exactly like [`clock_out`] (scoped to the pointer's
/// Headline by its stable document-order ordinal, matched by `started_at`,
/// never a stale byte offset), then splice-deletes the whole line
/// (`line_start..next_line_start`, taking its trailing newline) — leaving a
/// valid, possibly empty `:LOGBOOK:` drawer — and clears the pointer.
///
/// # Errors
///
/// [`OrgError::Vault`] when there is no active clock, or on a source/pointer
/// desync (pointer cleared); [`OrgError::Index`]/[`OrgError::Io`]/
/// [`OrgError::Parse`] on index/file access.
pub async fn clock_discard(vault_root: &Path) -> OrgResult<()> {
    let active = match active_clock(vault_root)? {
        Some(a) => a,
        None => {
            return Err(OrgError::Vault {
                reason: "no active clock to discard".to_string(),
            })
        }
    };
    let started = match parse_ts(&active.started_at) {
        Some(dt) => truncate_to_minute(dt),
        None => {
            remove_active_clock(vault_root)?;
            return Err(OrgError::Vault {
                reason: format!(
                    "active clock has an unparseable started_at {:?}; cleared the pointer",
                    active.started_at
                ),
            });
        }
    };

    let location =
        match crate::index::locate_headline(vault_root, i64::from(active.headline_id)).await? {
            Some(loc) => loc,
            None => {
                remove_active_clock(vault_root)?;
                return Err(OrgError::Vault {
                    reason: format!(
                        "active clock references headline {} which is no longer in the index; \
                         cleared the dangling active-clock pointer",
                        active.headline_id
                    ),
                });
            }
        };

    let file_path = vault_root.join(&location.file_path);
    let source = read_source(&file_path)?;
    let doc = analyze_source(&source, &location.file_path)?;
    // Scope the `started_at` match to the pointer's Headline (see `clock_out`);
    // only when the ordinal fails to resolve at all do we fall back to the
    // whole-tree search — still matched by `started_at`, never by an offset.
    let entry = match find_headline_by_ordinal(&doc.headlines, to_usize(location.ordinal)) {
        Some(headline) => find_open_clock(headline, started),
        None => find_open_clock_in_tree(&doc.headlines, started),
    };

    let entry = match entry {
        Some(e) => e,
        None => {
            remove_active_clock(vault_root)?;
            return Err(OrgError::Vault {
                reason: format!(
                    "no open CLOCK line starting at {} found in {}; cleared the dangling \
                     active-clock pointer",
                    active.started_at, location.file_path
                ),
            });
        }
    };

    // Delete the WHOLE open line (with its trailing newline): from the start of
    // the line the entry sits on to the start of the next line.
    let ls = line_start(&source, entry.span.start);
    let le = next_line_start(&source, entry.span.start);
    let new_source = splice(&source, ls, le, "");
    atomic_write(&file_path, new_source.as_bytes()).map_err(clock_io)?;
    remove_active_clock(vault_root)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Time aggregation (pure)
// ---------------------------------------------------------------------------

/// An inclusive calendar-day range for [`totals`] filtering.
///
/// `#[non_exhaustive]`: a later story may add a knob (e.g. a time-of-day bound
/// or half-open flag), so freezing the shape as growable now keeps that a
/// semver-minor addition — mirroring the `agenda::CustomAgendaQuery` policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DateRange {
    /// First day included (inclusive).
    pub start: NaiveDate,
    /// Last day included (inclusive).
    pub end: NaiveDate,
}

/// What [`totals`] aggregates over.
///
/// `#[non_exhaustive]`: later stories may add scopes (e.g. whole-Vault, or a
/// file-path scope for the v0.5 Project Report), so a downstream `match` must
/// carry a wildcard arm and this stays a semver-minor addition.
#[non_exhaustive]
pub enum ClockScope<'a> {
    /// A single headline's own `CLOCK:` entries (not its children).
    Headline(&'a Headline),
    /// A headline plus all its descendants, recursively.
    Subtree(&'a Headline),
    /// Every headline in `headlines` (recursively) carrying `tag`.
    Tag {
        /// The headlines to search (typically `Document::headlines`).
        headlines: &'a [Headline],
        /// The bare tag name (no colons) to match.
        tag: &'a str,
    },
}

/// Sum the closed `CLOCK:` durations in `scope`, optionally restricted to
/// entries whose start DATE falls within `range` (inclusive). Open (unclosed)
/// entries contribute `0`. Returns a [`TimeDelta`].
pub fn totals(scope: ClockScope<'_>, range: Option<DateRange>) -> TimeDelta {
    match scope {
        ClockScope::Headline(h) => sum_own_clocks(h, range),
        ClockScope::Subtree(h) => sum_subtree(h, range),
        ClockScope::Tag { headlines, tag } => sum_tag(headlines, tag, range),
    }
}

/// Whether `date` falls within `range` (inclusive), or always when no range.
fn in_range(date: NaiveDate, range: Option<DateRange>) -> bool {
    match range {
        Some(r) => date >= r.start && date <= r.end,
        None => true,
    }
}

/// The duration a single closed entry contributes: its written `=> H:MM` when
/// present, else `end - start` computed from the stamps (clamped non-negative).
fn entry_duration(entry: &ClockEntry) -> TimeDelta {
    if let Some(d) = entry.duration {
        return d;
    }
    match entry.end.as_ref() {
        Some(end) => {
            let end_dt = end.date.and_time(end.time.unwrap_or_else(midnight));
            (end_dt - clock_start_dt(entry)).max(TimeDelta::zero())
        }
        None => TimeDelta::zero(),
    }
}

/// Sum a single headline's own closed clock durations, filtered by `range`.
fn sum_own_clocks(headline: &Headline, range: Option<DateRange>) -> TimeDelta {
    headline
        .clocks
        .iter()
        .filter(|c| c.end.is_some() && in_range(c.start.date, range))
        .map(entry_duration)
        .fold(TimeDelta::zero(), |acc, d| acc + d)
}

/// Sum a headline plus all descendants, filtered by `range`.
fn sum_subtree(headline: &Headline, range: Option<DateRange>) -> TimeDelta {
    let mut total = sum_own_clocks(headline, range);
    for child in &headline.children {
        total += sum_subtree(child, range);
    }
    total
}

/// Sum the own clocks of every headline (recursively) carrying `tag`. Summing
/// OWN clocks per matching headline means a tagged parent and tagged child are
/// each counted once, never double-counted.
fn sum_tag(headlines: &[Headline], tag: &str, range: Option<DateRange>) -> TimeDelta {
    let mut total = TimeDelta::zero();
    for h in headlines {
        if h.tags.iter().any(|t| t.name == tag) {
            total += sum_own_clocks(h, range);
        }
        total += sum_tag(&h.children, tag, range);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .expect("valid date")
            .and_hms_opt(h, min, 0)
            .expect("valid time")
    }

    // ---- timestamp / duration formatting ----

    #[test]
    fn formats_inactive_clock_stamp_with_computed_weekday() {
        // 2026-09-13 is a Sunday.
        assert_eq!(
            format_inactive_stamp(dt(2026, 9, 13, 10, 0)),
            "[2026-09-13 Sun 10:00]"
        );
    }

    #[test]
    fn open_and_closed_clock_line_shapes() {
        let stamp = format_inactive_stamp(dt(2026, 9, 13, 10, 0));
        assert_eq!(stamp, "[2026-09-13 Sun 10:00]");
        // Closed shape assembled the way clock_out does.
        let end = format_inactive_stamp(dt(2026, 9, 13, 11, 30));
        let dur = format_duration(dt(2026, 9, 13, 11, 30) - dt(2026, 9, 13, 10, 0));
        assert_eq!(
            format!("CLOCK: {stamp}--{end} => {dur}"),
            "CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:30] => 1:30"
        );
    }

    #[test]
    fn duration_formats_hours_and_zero_padded_minutes() {
        assert_eq!(
            format_duration(dt(2026, 9, 13, 11, 30) - dt(2026, 9, 13, 10, 0)),
            "1:30"
        );
        // 25h05m.
        assert_eq!(
            format_duration(dt(2026, 9, 14, 11, 5) - dt(2026, 9, 13, 10, 0)),
            "25:05"
        );
        // Sub-minute rounds DOWN to minutes.
        let start = dt(2026, 9, 13, 10, 0);
        let end = start + TimeDelta::seconds(90 * 60 + 30); // 1:30:30
        assert_eq!(format_duration(end - start), "1:30");
        // Zero / negative clamp.
        assert_eq!(format_duration(TimeDelta::zero()), "0:00");
        assert_eq!(format_duration(TimeDelta::seconds(-100)), "0:00");
    }

    // ---- LOGBOOK insertion (create-if-absent + prepend-if-present) ----

    fn only_headline(source: &str) -> Headline {
        analyze(source)
            .expect("analyze")
            .headlines
            .into_iter()
            .next()
            .expect("one headline")
    }

    #[test]
    fn clock_in_creates_logbook_after_properties() {
        let source = "* Task\n:PROPERTIES:\n:ID: abc\n:END:\nBody\n";
        let headline = only_headline(source);
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 10, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\n:PROPERTIES:\n:ID: abc\n:END:\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\nBody\n"
        );
        // Re-analyze: exactly one open clock with the expected start.
        let re = only_headline(&out);
        assert_eq!(re.clocks.len(), 1);
        assert!(re.clocks[0].end.is_none());
        assert_eq!(clock_start_dt(&re.clocks[0]), dt(2026, 9, 13, 10, 0));
    }

    #[test]
    fn clock_in_creates_logbook_after_planning_line_when_no_properties() {
        let source = "* Task\nSCHEDULED: <2026-09-13 Sun>\nBody\n";
        let headline = only_headline(source);
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 9, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\nSCHEDULED: <2026-09-13 Sun>\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 09:00]\n:END:\nBody\n"
        );
    }

    #[test]
    fn clock_in_creates_logbook_right_after_headline_when_bare() {
        let source = "* Task\n";
        let headline = only_headline(source);
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 9, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 09:00]\n:END:\n"
        );
    }

    #[test]
    fn clock_in_prepends_into_existing_logbook_preserving_indent() {
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-12 Sat 08:00]--[2026-09-12 Sat 09:00] => 1:00\n:END:\n";
        let headline = only_headline(source);
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 10, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-12 Sat 08:00]--[2026-09-12 Sat 09:00] => 1:00\n:END:\n"
        );
        // Newest first: two entries, the new one open at the top.
        let re = only_headline(&out);
        assert_eq!(re.clocks.len(), 2);
        assert!(re.clocks[0].end.is_none());
        assert_eq!(clock_start_dt(&re.clocks[0]), dt(2026, 9, 13, 10, 0));
    }

    // ---- open-line close / match ----

    #[test]
    fn closing_an_open_line_matches_by_start_and_writes_duration() {
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\n";
        let headline = only_headline(source);
        let started = dt(2026, 9, 13, 10, 0);
        let entry = find_open_clock(&headline, started).expect("open entry");
        let now = dt(2026, 9, 13, 11, 30);
        let insert = format!(
            "--{} => {}",
            format_inactive_stamp(now),
            format_duration(now - started)
        );
        let out = splice(source, entry.span.end, entry.span.end, &insert);
        assert_eq!(
            out,
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:30] => 1:30\n:END:\n"
        );
        let re = only_headline(&out);
        assert_eq!(re.clocks.len(), 1);
        assert!(re.clocks[0].end.is_some());
        assert_eq!(re.clocks[0].duration, TimeDelta::try_minutes(90));
    }

    #[test]
    fn find_open_clock_ignores_closed_and_non_matching() {
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-12 Sat 08:00]--[2026-09-12 Sat 09:00] => 1:00\n:END:\n";
        let headline = only_headline(source);
        assert!(find_open_clock(&headline, dt(2026, 9, 13, 10, 0)).is_some());
        // A closed line's start does not match as an OPEN line.
        assert!(find_open_clock(&headline, dt(2026, 9, 12, 8, 0)).is_none());
        // A start no line has.
        assert!(find_open_clock(&headline, dt(2026, 1, 1, 0, 0)).is_none());
    }

    // ---- totals ----

    fn totals_fixture() -> crate::parser::semantic::Document {
        // Parent :work: with two closed clocks; a child :personal: with one.
        let source = "\
* Parent :work:
:LOGBOOK:
CLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:00] => 1:00
CLOCK: [2026-09-14 Mon 09:00]--[2026-09-14 Mon 11:30] => 2:30
:END:
** Child :personal:
:LOGBOOK:
CLOCK: [2026-09-15 Tue 08:00]--[2026-09-15 Tue 08:45] => 0:45
CLOCK: [2026-09-16 Wed 08:00]
:END:
";
        analyze(source).expect("analyze")
    }

    #[test]
    fn totals_per_headline_sums_own_closed_clocks_only() {
        let doc = totals_fixture();
        let parent = &doc.headlines[0];
        // 1:00 + 2:30 = 3:30 (children excluded).
        assert_eq!(
            totals(ClockScope::Headline(parent), None),
            TimeDelta::try_minutes(210).unwrap()
        );
    }

    #[test]
    fn totals_per_subtree_includes_children_and_ignores_open_entries() {
        let doc = totals_fixture();
        let parent = &doc.headlines[0];
        // 3:30 + 0:45 (child); the child's OPEN entry contributes 0.
        assert_eq!(
            totals(ClockScope::Subtree(parent), None),
            TimeDelta::try_minutes(210 + 45).unwrap()
        );
    }

    #[test]
    fn totals_per_tag_sums_matching_headlines() {
        let doc = totals_fixture();
        assert_eq!(
            totals(
                ClockScope::Tag {
                    headlines: &doc.headlines,
                    tag: "work"
                },
                None
            ),
            TimeDelta::try_minutes(210).unwrap()
        );
        assert_eq!(
            totals(
                ClockScope::Tag {
                    headlines: &doc.headlines,
                    tag: "personal"
                },
                None
            ),
            TimeDelta::try_minutes(45).unwrap()
        );
    }

    #[test]
    fn totals_filters_by_inclusive_date_range() {
        let doc = totals_fixture();
        let parent = &doc.headlines[0];
        // Only the 2026-09-14 entry (2:30) falls in this single-day range.
        let range = Some(DateRange {
            start: NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(),
        });
        assert_eq!(
            totals(ClockScope::Subtree(parent), range),
            TimeDelta::try_minutes(150).unwrap()
        );
    }

    // ---- active-clock.json round-trip + self-heal ----

    #[test]
    fn active_clock_path_joins_dotorgsidian() {
        assert_eq!(
            active_clock_path(Path::new("/vaults/work")),
            Path::new("/vaults/work/.orgsidian/active-clock.json")
        );
    }

    #[test]
    fn active_clock_round_trips_and_defaults_to_none_when_missing() {
        let dir = tempdir().expect("tempdir");
        assert_eq!(active_clock(dir.path()).expect("read"), None);

        let clock = ActiveClock {
            headline_id: 7,
            started_at: "2026-09-13T10:00:00".to_string(),
            last_active_at: "2026-09-13T10:00:00".to_string(),
        };
        write_active_clock(dir.path(), &clock).expect("write");
        assert_eq!(active_clock(dir.path()).expect("read back"), Some(clock));
    }

    #[test]
    fn active_clock_self_heals_malformed_json_to_none() {
        let dir = tempdir().expect("tempdir");
        let path = active_clock_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();
        assert_eq!(active_clock(dir.path()).expect("self-heal"), None);
    }

    #[test]
    fn refresh_active_clock_bumps_last_active_at_only() {
        let dir = tempdir().expect("tempdir");
        let clock = ActiveClock {
            headline_id: 3,
            started_at: "2026-09-13T10:00:00".to_string(),
            last_active_at: "2026-09-13T10:00:00".to_string(),
        };
        write_active_clock(dir.path(), &clock).expect("seed");
        refresh_active_clock(dir.path(), dt(2026, 9, 13, 12, 30)).expect("refresh");
        let after = active_clock(dir.path()).expect("read").expect("some");
        assert_eq!(after.started_at, "2026-09-13T10:00:00");
        assert_eq!(after.last_active_at, "2026-09-13T12:30:00");
    }

    #[test]
    fn refresh_active_clock_is_a_noop_without_an_active_clock() {
        let dir = tempdir().expect("tempdir");
        refresh_active_clock(dir.path(), dt(2026, 9, 13, 12, 30)).expect("noop");
        assert_eq!(active_clock(dir.path()).expect("still none"), None);
    }

    #[test]
    fn active_clock_json_on_disk_keys_are_snake_case() {
        // The Rust-only sidecar file must persist EXACTLY
        // `{ headline_id, started_at, last_active_at }` (AC + Stories 7.7/7.8).
        let dir = tempdir().expect("tempdir");
        let clock = ActiveClock {
            headline_id: 9,
            started_at: "2026-09-13T10:00:00".to_string(),
            last_active_at: "2026-09-13T10:00:00".to_string(),
        };
        write_active_clock(dir.path(), &clock).expect("write");
        let raw = fs::read_to_string(active_clock_path(dir.path())).expect("read raw");
        assert!(raw.contains("\"headline_id\""), "{raw}");
        assert!(raw.contains("\"started_at\""), "{raw}");
        assert!(raw.contains("\"last_active_at\""), "{raw}");
        assert!(!raw.contains("headlineId"), "must not be camelCase: {raw}");
        assert!(!raw.contains("startedAt"), "must not be camelCase: {raw}");
        assert!(
            !raw.contains("lastActiveAt"),
            "must not be camelCase: {raw}"
        );
    }

    // ---- fix #1: minute truncation ----

    #[test]
    fn truncate_to_minute_zeroes_seconds_and_nanoseconds() {
        let with_secs = NaiveDate::from_ymd_opt(2026, 9, 13)
            .unwrap()
            .and_hms_nano_opt(12, 34, 56, 789)
            .unwrap();
        assert_eq!(truncate_to_minute(with_secs), dt(2026, 9, 13, 12, 34));
        assert_eq!(
            format_ts(truncate_to_minute(with_secs)),
            "2026-09-13T12:34:00"
        );
    }

    // ---- #13: totals over a closed line WITHOUT a `=> H:MM` suffix ----

    #[test]
    fn totals_uses_end_minus_start_when_no_duration_suffix() {
        // A valid closed org CLOCK line can omit `=> H:MM`; the parser then
        // reports end=Some, duration=None, so `entry_duration` falls back to
        // end - start.
        let source =
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 11:30]\n:END:\n";
        let headline = only_headline(source);
        assert_eq!(headline.clocks.len(), 1);
        assert!(headline.clocks[0].end.is_some());
        assert_eq!(headline.clocks[0].duration, None, "no `=>` suffix parsed");
        assert_eq!(
            totals(ClockScope::Headline(&headline), None),
            TimeDelta::try_minutes(90).unwrap()
        );
    }

    #[test]
    fn entry_duration_clamps_a_backwards_closed_range_to_zero() {
        // end before start (hand-edited/degenerate) → non-negative clamp.
        let source =
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 11:00]--[2026-09-13 Sun 10:00]\n:END:\n";
        let headline = only_headline(source);
        assert_eq!(
            totals(ClockScope::Headline(&headline), None),
            TimeDelta::zero()
        );
    }

    // ---- #14: CRLF byte-faithful splice ----

    #[test]
    fn clock_in_inserts_crlf_terminated_lines_into_a_crlf_source() {
        let source = "* Task\r\nBody\r\n";
        let headline = only_headline(source);
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 9, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\r\n:LOGBOOK:\r\nCLOCK: [2026-09-13 Sun 09:00]\r\n:END:\r\nBody\r\n"
        );
    }

    // ---- #15: indented LOGBOOK prepend preserves indentation ----

    // ---- Story 7.7: discard line-delete splice ----

    #[test]
    fn discard_deletes_the_whole_open_line_leaving_an_empty_drawer() {
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\n";
        let headline = only_headline(source);
        let entry = find_open_clock(&headline, dt(2026, 9, 13, 10, 0)).expect("open entry");
        // The clock_discard splice: line_start..next_line_start of the entry.
        let ls = line_start(source, entry.span.start);
        let le = next_line_start(source, entry.span.start);
        let out = splice(source, ls, le, "");
        assert_eq!(out, "* Task\n:LOGBOOK:\n:END:\n");
        // Re-analyze: the drawer is still valid org and carries no clocks.
        let re = only_headline(&out);
        assert_eq!(re.clocks.len(), 0);
        assert!(re.drawers.iter().any(|d| d.kind == DrawerKind::Logbook));
    }

    // ---- Story 7.7: duplicate open-line neutralization (clock_in guard) ----

    #[test]
    fn close_extra_open_lines_neutralizes_older_duplicates_to_zero() {
        // Two open lines: the newer (10:00) is adopted; the older (08:00) must be
        // closed to its own start `=> 0:00`. Newest is prepended (org order).
        let source =
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-13 Sun 08:00]\n:END:\n";
        let headline = only_headline(source);
        let adopt = most_recent_open_clock(&headline).expect("an open line");
        assert_eq!(clock_start_dt(adopt), dt(2026, 9, 13, 10, 0));
        let out = close_extra_open_lines(source, &headline, adopt.span.start);
        assert_eq!(
            out,
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-13 Sun 08:00]--[2026-09-13 Sun 08:00] => 0:00\n:END:\n"
        );
        // Re-analyze: exactly one open line remains — the adopted 10:00 one.
        let re = only_headline(&out);
        let open: Vec<_> = re.clocks.iter().filter(|c| c.end.is_none()).collect();
        assert_eq!(open.len(), 1);
        assert_eq!(clock_start_dt(open[0]), dt(2026, 9, 13, 10, 0));
    }

    #[test]
    fn close_extra_open_lines_leaves_a_single_open_line_untouched() {
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 10:00]\n:END:\n";
        let headline = only_headline(source);
        let adopt = most_recent_open_clock(&headline).expect("an open line");
        // No extras → source unchanged.
        assert_eq!(
            close_extra_open_lines(source, &headline, adopt.span.start),
            source
        );
    }

    #[test]
    fn close_extra_open_lines_neutralizes_two_older_duplicates_in_reverse_order() {
        // THREE open lines (12:00 newest/adopted, 10:00 and 08:00 extras). The
        // reverse-order (descending span.end) splice must close BOTH older lines
        // to their own start `=> 0:00` and leave exactly the adopted one open.
        let source = "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 12:00]\nCLOCK: [2026-09-13 Sun 10:00]\nCLOCK: [2026-09-13 Sun 08:00]\n:END:\n";
        let headline = only_headline(source);
        let adopt = most_recent_open_clock(&headline).expect("an open line");
        assert_eq!(clock_start_dt(adopt), dt(2026, 9, 13, 12, 0));
        let out = close_extra_open_lines(source, &headline, adopt.span.start);
        assert_eq!(
            out,
            "* Task\n:LOGBOOK:\nCLOCK: [2026-09-13 Sun 12:00]\nCLOCK: [2026-09-13 Sun 10:00]--[2026-09-13 Sun 10:00] => 0:00\nCLOCK: [2026-09-13 Sun 08:00]--[2026-09-13 Sun 08:00] => 0:00\n:END:\n"
        );
        // Re-analyze: exactly one open line remains — the adopted 12:00 one.
        let re = only_headline(&out);
        let open: Vec<_> = re.clocks.iter().filter(|c| c.end.is_none()).collect();
        assert_eq!(open.len(), 1);
        assert_eq!(clock_start_dt(open[0]), dt(2026, 9, 13, 12, 0));
    }

    // ---- Story 7.7: launch-summary duration math (14 h gap) ----

    #[test]
    fn stale_summary_durations_split_keep_vs_adjust() {
        // Started 04:00, last-active 18:00 (14 h), now next-day 10:00 (30 h).
        let started = dt(2026, 9, 13, 4, 0);
        let last_active = dt(2026, 9, 13, 18, 0);
        let now = dt(2026, 9, 14, 10, 0);
        assert_eq!(
            format_duration(now - started),
            "30:00",
            "keep = now - started"
        );
        assert_eq!(
            format_duration(last_active - started),
            "14:00",
            "adjust = last_active - started"
        );
        // A last_active BEFORE started (degenerate) clamps to 0:00.
        assert_eq!(format_duration(started - last_active), "0:00");
    }

    #[test]
    fn clock_in_prepends_into_indented_logbook_preserving_indent() {
        let source = "* Task\n  :LOGBOOK:\n  CLOCK: [2026-09-12 Sat 08:00]--[2026-09-12 Sat 09:00] => 1:00\n  :END:\n";
        let headline = only_headline(source);
        // Only proceed if the parser classifies the indented drawer as LOGBOOK.
        assert!(
            headline
                .drawers
                .iter()
                .any(|d| d.kind == DrawerKind::Logbook),
            "parser must recognize the indented :LOGBOOK: drawer"
        );
        let (at, text) = compute_clock_in_edit(source, &headline, dt(2026, 9, 13, 10, 0));
        let out = splice(source, at, at, &text);
        assert_eq!(
            out,
            "* Task\n  :LOGBOOK:\n  CLOCK: [2026-09-13 Sun 10:00]\n  CLOCK: [2026-09-12 Sat 08:00]--[2026-09-12 Sat 09:00] => 1:00\n  :END:\n"
        );
    }
}
