//! Implements FR-6 (Today Dashboard — Story 7.1).
//!
//! The full Today Dashboard the `/today` route renders on launch
//! (`shell-ui/src/components/today/TodayDashboard.tsx`): five computed sections
//! across the whole Vault, in this order —
//!
//! 1. **Scheduled today** — headlines with `SCHEDULED:` on the caller's `today`.
//! 2. **Deadline today-or-overdue** — headlines with `DEADLINE:` on or before
//!    `today` (the `overdue` flag distinguishes strictly-before from due-today).
//! 3. **Today-tag** — headlines carrying a configurable tag (default `today`).
//! 4. **Inbox preview** — the first N headlines of the Vault-root `inbox.org`.
//! 5. **Active clock** — the one running clock (`clock_entries.end_at IS NULL`),
//!    if any — READ-ONLY here; clock in/out/write is Story 7.6.
//!
//! [`today`] composes these as five independent reads over one
//! [`Connection`], returning a single [`TodayDashboard`]. Each section is a
//! separate `SELECT` rather than one partitioned scan: an item scheduled today
//! that also carries a past deadline legitimately appears in BOTH the Scheduled
//! and Deadline sections (org agenda semantics — see the Design Notes in the
//! Story 7.1 spec), so the sections are not a partition of one result set.
//!
//! # Reuse of `agenda::today` semantics
//!
//! The three headline sections (Scheduled / Deadline / Today-tag) reuse
//! [`AgendaItem`] and mirror [`super::agenda::today`] exactly: DONE headlines
//! and quarantined files are excluded, and rows are ordered `(f.path,
//! h.position)` so the frontend's per-file grouping is a stable partition of an
//! already-sorted list, never a client-side re-sort.
//!
//! # Why `today` is a caller-supplied string
//!
//! Same reason as [`super::agenda`]: the index has no notion of the user's
//! timezone, so the frontend hands over its local calendar day as a plain
//! `YYYY-MM-DD` string and ISO-8601 TEXT columns compare lexicographically.

use rusqlite::Connection;

use crate::error::IndexError;
use crate::query::agenda::AgendaItem;

/// Parameters for [`today`]: the caller's local calendar day, the configurable
/// "today" tag, and how many Inbox entries to preview. Read from the per-Vault
/// `TodayDashboardSections` settings at the command boundary (defaults `today`
/// / 5), never a server-side clock read (see the module docs).
///
/// A plain, directly-constructible struct (not `#[non_exhaustive]`) because the
/// command layer in `orgsidian-shell-app` builds one per request from Vault
/// settings.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardParams {
    /// The frontend's local calendar day (`YYYY-MM-DD`).
    pub today: String,
    /// The configurable "today" tag (bare tag text, no leading `#`/trailing
    /// `:`); default `today`.
    pub today_tag: String,
    /// How many `inbox.org` headlines the preview shows; default 5.
    pub inbox_preview_count: usize,
}

/// One Inbox-preview row: a headline near the top of the Vault-root
/// `inbox.org`, carrying enough identity for click-to-open at the source
/// Headline (`/editor/$filePath/$headlineId`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct InboxItem {
    /// `headlines.id` — the click-to-open target's identity.
    pub headline_id: i64,
    /// `files.path` — always `inbox.org` for these rows, carried so the
    /// frontend renders the click-to-open `Link` the same way every section
    /// does.
    pub file_path: String,
    /// Headline title, stars/keyword/tags already stripped.
    pub title: String,
    /// `headlines.byte_start` — the source byte offset for cursor placement.
    pub byte_start: i64,
    /// TODO keyword text, when the headline carries one.
    pub todo_keyword: Option<String>,
}

/// The one running clock, if any: the `clock_entries` row whose `end_at` is
/// NULL, joined to its owning headline + file for display and click-to-open.
/// READ-ONLY in Story 7.1 (clock in/out/write + LOGBOOK persistence is Story
/// 7.6).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct ActiveClock {
    /// The clocked-in headline's `headlines.id` (click-to-open target).
    pub headline_id: i64,
    /// The clocked-in headline's `files.path` (click-to-open target).
    pub file_path: String,
    /// The clocked-in headline's title, stars/keyword/tags stripped.
    pub title: String,
    /// The clocked-in headline's `byte_start` (cursor placement).
    pub byte_start: i64,
    /// `clock_entries.start_at` — when the running clock was started
    /// (ISO-8601), for the ambient "clocked in since" indicator.
    pub start_at: String,
}

/// The full Today Dashboard: the five sections, in render order (Scheduled |
/// Deadline | Today-tag | Inbox preview | Active clock).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct TodayDashboard {
    /// Headlines `SCHEDULED:` for the caller's `today`.
    pub scheduled: Vec<AgendaItem>,
    /// Headlines with a `DEADLINE:` on or before `today` (overdue-or-today).
    pub deadlines: Vec<AgendaItem>,
    /// Headlines carrying the configurable "today" tag.
    pub today_tag: Vec<AgendaItem>,
    /// The first N `inbox.org` headlines.
    pub inbox: Vec<InboxItem>,
    /// The one running clock, or `None` when nothing is clocked in.
    pub active_clock: Option<ActiveClock>,
}

/// The shared `SELECT` column list for the three [`AgendaItem`] sections —
/// identical column shape (and therefore row-index contract) to
/// [`super::agenda::today`], so [`agenda_row`] can decode any of them.
const AGENDA_COLUMNS: &str = "h.id, f.path, h.title, h.byte_start, h.todo_keyword,
                h.scheduled_date, h.scheduled_time,
                h.deadline_date, h.deadline_time";

/// Decode one [`AgendaItem`] from a row shaped by [`AGENDA_COLUMNS`], anchoring
/// `overdue` and `agenda_date` to `today` (mirrors [`super::agenda::today`]'s
/// row closure — every dashboard headline belongs on `today` by construction).
fn agenda_row(row: &rusqlite::Row<'_>, today: &str) -> rusqlite::Result<AgendaItem> {
    let deadline_date: Option<String> = row.get(7)?;
    let overdue = deadline_date.as_deref().is_some_and(|date| date < today);
    Ok(AgendaItem {
        headline_id: row.get(0)?,
        file_path: row.get(1)?,
        title: row.get(2)?,
        byte_start: row.get(3)?,
        todo_keyword: row.get(4)?,
        scheduled_date: row.get(5)?,
        scheduled_time: row.get(6)?,
        deadline_date,
        deadline_time: row.get(8)?,
        overdue,
        agenda_date: today.to_string(),
    })
}

/// Scheduled-today section: `scheduled_date = today`, DONE + quarantined
/// excluded, ordered `(f.path, h.position)`.
fn scheduled(conn: &Connection, today: &str) -> Result<Vec<AgendaItem>, IndexError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AGENDA_COLUMNS}
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND (h.todo_done IS NULL OR h.todo_done = 0)
           AND h.scheduled_date = ?1
         ORDER BY f.path, h.position"
    ))?;
    let rows = stmt.query_map([today], |row| agenda_row(row, today))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Deadline section: `deadline_date <= today` (overdue-or-today), DONE +
/// quarantined excluded, ordered `(f.path, h.position)`.
fn deadlines(conn: &Connection, today: &str) -> Result<Vec<AgendaItem>, IndexError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AGENDA_COLUMNS}
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND (h.todo_done IS NULL OR h.todo_done = 0)
           AND h.deadline_date IS NOT NULL
           AND h.deadline_date <= ?1
         ORDER BY f.path, h.position"
    ))?;
    let rows = stmt.query_map([today], |row| agenda_row(row, today))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Today-tag section: headlines carrying `tag`, DONE + quarantined excluded,
/// ordered `(f.path, h.position)`. Uses an `IN (SELECT …)` sub-query rather than
/// a `JOIN tags` so a headline that repeats the tag at two positions still
/// yields exactly one row.
fn today_tag(conn: &Connection, today: &str, tag: &str) -> Result<Vec<AgendaItem>, IndexError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AGENDA_COLUMNS}
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND (h.todo_done IS NULL OR h.todo_done = 0)
           AND h.id IN (SELECT headline_id FROM tags WHERE tag = ?1)
         ORDER BY f.path, h.position"
    ))?;
    let rows = stmt.query_map([tag], |row| agenda_row(row, today))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Inbox-preview section: the first `count` headlines of the Vault-root
/// `inbox.org`, in document order. An absent `inbox.org` yields an empty vec.
/// DONE items are NOT excluded here (an inbox is a capture queue, not an
/// agenda), but quarantined files and the synthetic preamble row are.
fn inbox(conn: &Connection, count: usize) -> Result<Vec<InboxItem>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT h.id, f.path, h.title, h.byte_start, h.todo_keyword
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND f.path = 'inbox.org'
         ORDER BY h.position
         LIMIT ?1",
    )?;
    // Saturate rather than `as i64`: a huge `usize` wrapping negative would be
    // read by SQLite as an UNLIMITED `LIMIT`, loading the whole inbox.
    let limit = i64::try_from(count).unwrap_or(i64::MAX);
    let rows = stmt.query_map([limit], |row| {
        Ok(InboxItem {
            headline_id: row.get(0)?,
            file_path: row.get(1)?,
            title: row.get(2)?,
            byte_start: row.get(3)?,
            todo_keyword: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Active-clock section: the one `clock_entries` row still running (`end_at IS
/// NULL`), joined to its headline + file. `None` when nothing is clocked in.
/// `LIMIT 1` — the single-Active-Clock invariant is enforced on the write side
/// (Story 7.6); this read is defensive if two ever coexist, taking the
/// lowest-id (earliest-recorded) running entry.
fn active_clock(conn: &Connection) -> Result<Option<ActiveClock>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT h.id, f.path, h.title, h.byte_start, c.start_at
         FROM clock_entries c
         JOIN headlines h ON h.id = c.headline_id
         JOIN files f ON f.id = h.file_id
         WHERE c.end_at IS NULL
           AND f.quarantined = 0
           AND h.kind = 'headline'
         ORDER BY c.id
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], |row| {
        Ok(ActiveClock {
            headline_id: row.get(0)?,
            file_path: row.get(1)?,
            title: row.get(2)?,
            byte_start: row.get(3)?,
            start_at: row.get(4)?,
        })
    })?;
    match rows.next() {
        Some(item) => Ok(Some(item?)),
        None => Ok(None),
    }
}

/// Assemble the full [`TodayDashboard`] for `params` — the five sections read
/// independently over one [`Connection`]. The measurable backend proxy for the
/// FR-6 "dashboard render" the perf gate holds under 500 ms on a 1000-file
/// Vault (`tests/story_7_1_today_dashboard_perf.rs`).
///
/// # Errors
///
/// [`IndexError::Sqlite`] if any of the five sub-queries fails to prepare or
/// run.
pub fn today(conn: &Connection, params: &DashboardParams) -> Result<TodayDashboard, IndexError> {
    // Run all five section reads inside ONE deferred read transaction so they
    // observe a single consistent SQLite snapshot — otherwise a concurrent
    // writer landing between two sections could produce a torn dashboard (e.g.
    // a headline gone from Scheduled but still counted under Today-tag).
    // `unchecked_transaction` works on `&Connection` (it does not need `&mut`);
    // this is read-only, so the explicit `commit` just ends the snapshot.
    let tx = conn.unchecked_transaction()?;
    let dashboard = TodayDashboard {
        scheduled: scheduled(&tx, &params.today)?,
        deadlines: deadlines(&tx, &params.today)?,
        today_tag: today_tag(&tx, &params.today, &params.today_tag)?,
        inbox: inbox(&tx, params.inbox_preview_count)?,
        active_clock: active_clock(&tx)?,
    };
    tx.commit()?;
    Ok(dashboard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{ClockInput, FileIndexInput, HeadlineInput};
    use rusqlite::Connection;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::apply_schema(&mut conn).expect("apply schema");
        conn
    }

    fn headline(title: &str, position: i64) -> HeadlineInput {
        HeadlineInput {
            level: 1,
            position,
            byte_start: 0,
            byte_end: 10,
            todo_keyword: Some("TODO".to_string()),
            todo_done: Some(false),
            title: title.to_string(),
            body: String::new(),
            scheduled_date: None,
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
        }
    }

    fn file(rel_path: &str, headlines: Vec<HeadlineInput>) -> FileIndexInput {
        FileIndexInput {
            rel_path: rel_path.to_string(),
            mtime_ns: 1,
            size_bytes: 1,
            preamble: None,
            headlines,
        }
    }

    fn params(today: &str) -> DashboardParams {
        DashboardParams {
            today: today.to_string(),
            today_tag: "today".to_string(),
            inbox_preview_count: 5,
        }
    }

    #[test]
    fn scheduled_section_holds_only_scheduled_today_excluding_done() {
        let mut conn = open_test_db();
        let mut sched = headline("Scheduled today", 0);
        sched.scheduled_date = Some("2026-09-05".to_string());
        let mut tomorrow = headline("Scheduled tomorrow", 1);
        tomorrow.scheduled_date = Some("2026-09-06".to_string());
        let mut done = headline("Scheduled but done", 2);
        done.scheduled_date = Some("2026-09-05".to_string());
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("a.org", vec![sched, tomorrow, done])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let titles: Vec<_> = dash.scheduled.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["Scheduled today"]);
    }

    #[test]
    fn deadline_section_holds_overdue_and_due_today_with_overdue_flag() {
        let mut conn = open_test_db();
        let mut overdue = headline("Overdue", 0);
        overdue.deadline_date = Some("2026-09-01".to_string());
        let mut due_today = headline("Due today", 1);
        due_today.deadline_date = Some("2026-09-05".to_string());
        let mut future = headline("Future", 2);
        future.deadline_date = Some("2026-09-10".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![overdue, due_today, future]))
            .expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let by_title: std::collections::HashMap<_, _> = dash
            .deadlines
            .iter()
            .map(|i| (i.title.as_str(), i.overdue))
            .collect();
        assert_eq!(by_title.len(), 2);
        assert_eq!(by_title.get("Overdue"), Some(&true));
        assert_eq!(by_title.get("Due today"), Some(&false));
        assert!(!by_title.contains_key("Future"));
    }

    #[test]
    fn today_tag_section_matches_configured_tag_and_dedupes_repeats() {
        let mut conn = open_test_db();
        let mut tagged = headline("Tagged today", 0);
        // Same tag at two positions must NOT double the row.
        tagged.tags = vec!["today".to_string(), "today".to_string()];
        let mut other = headline("Tagged work", 1);
        other.tags = vec!["work".to_string()];
        crate::upsert_file(&mut conn, &file("a.org", vec![tagged, other])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let titles: Vec<_> = dash.today_tag.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["Tagged today"], "one row per matching headline");
    }

    #[test]
    fn today_tag_section_honors_a_custom_tag() {
        let mut conn = open_test_db();
        let mut tagged = headline("Focus item", 0);
        tagged.tags = vec!["focus".to_string()];
        crate::upsert_file(&mut conn, &file("a.org", vec![tagged])).expect("upsert");

        let custom = DashboardParams {
            today: "2026-09-05".to_string(),
            today_tag: "focus".to_string(),
            inbox_preview_count: 5,
        };
        let dash = today(&conn, &custom).expect("query");

        let titles: Vec<_> = dash.today_tag.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["Focus item"]);
        // The default `today` tag matches nothing here.
        assert!(today(&conn, &params("2026-09-05"))
            .expect("query")
            .today_tag
            .is_empty());
    }

    #[test]
    fn inbox_section_previews_first_n_in_document_order() {
        let mut conn = open_test_db();
        let entries = (0..10)
            .map(|p| headline(&format!("Inbox {p}"), p))
            .collect();
        crate::upsert_file(&mut conn, &file("inbox.org", entries)).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let titles: Vec<_> = dash.inbox.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Inbox 0", "Inbox 1", "Inbox 2", "Inbox 3", "Inbox 4"],
            "the first 5 (default N), in position order"
        );
        assert!(dash.inbox.iter().all(|i| i.file_path == "inbox.org"));
    }

    #[test]
    fn inbox_preview_count_is_configurable() {
        let mut conn = open_test_db();
        let entries = (0..10)
            .map(|p| headline(&format!("Inbox {p}"), p))
            .collect();
        crate::upsert_file(&mut conn, &file("inbox.org", entries)).expect("upsert");

        let mut p = params("2026-09-05");
        p.inbox_preview_count = 3;
        let dash = today(&conn, &p).expect("query");

        assert_eq!(dash.inbox.len(), 3);
    }

    #[test]
    fn inbox_absent_yields_empty_vec() {
        let mut conn = open_test_db();
        let mut sched = headline("Scheduled today", 0);
        sched.scheduled_date = Some("2026-09-05".to_string());
        crate::upsert_file(&mut conn, &file("notes.org", vec![sched])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert!(dash.inbox.is_empty(), "no inbox.org at root → empty preview");
    }

    #[test]
    fn inbox_section_includes_done_items() {
        // Capture-queue semantics: the Inbox preview is a raw view of the first
        // N `inbox.org` headlines and, unlike the agenda sections, deliberately
        // does NOT filter out DONE. A future "consistency fix" that copied the
        // DONE predicate from the sibling sections must break this test.
        let mut conn = open_test_db();
        let mut open_item = headline("Open capture", 0);
        open_item.todo_keyword = None;
        open_item.todo_done = None;
        let mut done = headline("Captured then done", 1);
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("inbox.org", vec![open_item, done])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let titles: Vec<_> = dash.inbox.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Open capture", "Captured then done"],
            "the DONE capture must remain in the Inbox preview"
        );
    }

    #[test]
    fn active_clock_reads_the_one_running_entry() {
        let mut conn = open_test_db();
        let mut running = headline("Clocked in", 0);
        running.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T09:00:00".to_string(),
            end_at: None,
            duration_seconds: None,
        }];
        let mut closed = headline("Clocked out", 1);
        closed.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T08:00:00".to_string(),
            end_at: Some("2026-09-05T08:30:00".to_string()),
            duration_seconds: Some(1800),
        }];
        crate::upsert_file(&mut conn, &file("a.org", vec![running, closed])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let clock = dash.active_clock.expect("a running clock is present");
        assert_eq!(clock.title, "Clocked in");
        assert_eq!(clock.file_path, "a.org");
        assert_eq!(clock.start_at, "2026-09-05T09:00:00");
    }

    #[test]
    fn no_running_clock_yields_none() {
        let mut conn = open_test_db();
        let mut closed = headline("Clocked out", 0);
        closed.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T08:00:00".to_string(),
            end_at: Some("2026-09-05T08:30:00".to_string()),
            duration_seconds: Some(1800),
        }];
        crate::upsert_file(&mut conn, &file("a.org", vec![closed])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert!(dash.active_clock.is_none());
    }

    #[test]
    fn deadline_section_excludes_done_items() {
        let mut conn = open_test_db();
        let mut done = headline("Done but overdue", 0);
        done.deadline_date = Some("2026-09-01".to_string());
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("a.org", vec![done])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert!(
            dash.deadlines.is_empty(),
            "a DONE headline with a due/overdue deadline must not appear"
        );
    }

    #[test]
    fn today_tag_section_excludes_done_items() {
        let mut conn = open_test_db();
        let mut done = headline("Done but tagged", 0);
        done.tags = vec!["today".to_string()];
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("a.org", vec![done])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert!(
            dash.today_tag.is_empty(),
            "a DONE headline carrying the today-tag must not appear"
        );
    }

    #[test]
    fn active_clock_takes_the_lowest_id_of_multiple_running_entries() {
        // Defensive: the single-Active-Clock invariant is a write-side rule
        // (Story 7.6). If two running entries ever coexist, the read pins the
        // lowest-id (earliest-recorded) one per `ORDER BY c.id LIMIT 1`.
        let mut conn = open_test_db();
        let mut first = headline("First clocked in", 0);
        first.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T08:00:00".to_string(),
            end_at: None,
            duration_seconds: None,
        }];
        let mut second = headline("Second clocked in", 1);
        second.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T10:00:00".to_string(),
            end_at: None,
            duration_seconds: None,
        }];
        crate::upsert_file(&mut conn, &file("a.org", vec![first, second])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let clock = dash.active_clock.expect("a running clock is present");
        assert_eq!(
            clock.title, "First clocked in",
            "the lowest-id running entry wins"
        );
        assert_eq!(clock.start_at, "2026-09-05T08:00:00");
    }

    #[test]
    fn sections_exclude_quarantined_files() {
        let mut conn = open_test_db();

        // Seed a headline in `bad.org` that would land in every headline-backed
        // section — scheduled today, overdue deadline, today-tagged, and with a
        // running clock — plus a preview-eligible `inbox.org` headline, then
        // flip `quarantined = 1` on both files WITHOUT clearing their rows (a
        // direct UPDATE, not `quarantine_file`, which wipes the file's rows and
        // so would let the assertions pass without exercising the join filter).
        // With the rows intact, ALL FIVE sections must still come back empty
        // purely because each SELECT carries `f.quarantined = 0`.
        let mut everything = headline("Would-be everywhere", 0);
        everything.scheduled_date = Some("2026-09-05".to_string());
        everything.deadline_date = Some("2026-09-01".to_string());
        everything.tags = vec!["today".to_string()];
        everything.clock_entries = vec![ClockInput {
            start_at: "2026-09-05T09:00:00".to_string(),
            end_at: None,
            duration_seconds: None,
        }];
        crate::upsert_file(&mut conn, &file("bad.org", vec![everything])).expect("upsert bad");
        crate::upsert_file(
            &mut conn,
            &file("inbox.org", vec![headline("Inbox item", 0)]),
        )
        .expect("upsert inbox");
        conn.execute(
            "UPDATE files SET quarantined = 1, quarantine_reason = 'parse error'
             WHERE path IN ('bad.org', 'inbox.org')",
            [],
        )
        .expect("mark quarantined");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert!(dash.scheduled.is_empty());
        assert!(dash.deadlines.is_empty());
        assert!(dash.today_tag.is_empty());
        assert!(dash.inbox.is_empty(), "quarantined inbox.org yields no preview");
        assert!(
            dash.active_clock.is_none(),
            "a running clock in a quarantined file must not surface"
        );
    }

    #[test]
    fn scheduled_and_deadline_can_overlap_for_one_headline() {
        // An item scheduled today AND carrying a past deadline appears in BOTH
        // sections (org agenda semantics — the sections are not a partition).
        let mut conn = open_test_db();
        let mut both = headline("Scheduled with stale deadline", 0);
        both.scheduled_date = Some("2026-09-05".to_string());
        both.deadline_date = Some("2026-09-01".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![both])).expect("upsert");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        assert_eq!(dash.scheduled.len(), 1);
        assert_eq!(dash.deadlines.len(), 1);
        assert!(dash.scheduled[0].overdue, "carries the overdue deadline flag");
        assert!(dash.deadlines[0].overdue);
    }

    #[test]
    fn headline_sections_order_by_file_then_position() {
        let mut conn = open_test_db();
        let mut b1 = headline("b first", 0);
        b1.scheduled_date = Some("2026-09-05".to_string());
        let mut b2 = headline("b second", 1);
        b2.scheduled_date = Some("2026-09-05".to_string());
        let mut a1 = headline("a first", 0);
        a1.scheduled_date = Some("2026-09-05".to_string());
        crate::upsert_file(&mut conn, &file("b.org", vec![b1, b2])).expect("upsert b");
        crate::upsert_file(&mut conn, &file("a.org", vec![a1])).expect("upsert a");

        let dash = today(&conn, &params("2026-09-05")).expect("query");

        let ordering: Vec<_> = dash
            .scheduled
            .iter()
            .map(|i| (i.file_path.as_str(), i.title.as_str()))
            .collect();
        assert_eq!(
            ordering,
            vec![
                ("a.org", "a first"),
                ("b.org", "b first"),
                ("b.org", "b second"),
            ]
        );
    }
}
