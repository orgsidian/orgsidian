//! Implements FR-7 (Agenda Today/Week — Story 6.3 shipped `today`; Story 6.4
//! adds `week`. Story 6.3's subset: Scheduled items for `today` + Deadline
//! items overdue-or-today, across the whole Vault, grouped by source file.
//! Story 6.4's subset: the same two legs over a rolling 7-day window, grouped
//! by calendar date instead). The full Today Dashboard (Today-Tag, Inbox
//! preview, Active Clock) is Epic 7 (Story 7.1); Custom ranges are Story 7.4.
//! Story 6.5 freezes `agenda::{today, week, custom}` together as the v0.1
//! `IndexQuery` baseline surface — this file ships the first two, already
//! real and tested, for that trait to wrap.
//!
//! [`today`] is the query behind `/today`
//! (`shell-ui/src/components/agenda/AgendaToday.tsx`): a single `SELECT` over
//! `headlines` joined to `files`, ordered by `(file_path, position)` so the
//! frontend's "grouped by source file" AC is a stable partition of an
//! already-sorted list — no second sort needed client-side.
//!
//! [`week`] is the query behind `/agenda/week`
//! (`shell-ui/src/components/agenda/AgendaWeek.tsx`): the same two legs
//! (Scheduled within the window; Deadline overdue-or-within-the-window)
//! widened from a single day to a caller-supplied `[start_date, start_date +
//! 6 days]` inclusive range, sorted by [`AgendaItem::agenda_date`] (a
//! stable Rust-side sort over the SQL fetch order, per that field's own
//! docs) so the frontend's "grouped by date" AC is likewise a stable
//! partition, never a re-sort.
//!
//! # Why `today`/`start_date` are caller-supplied strings, not a server-side clock read
//!
//! The index has no notion of the user's timezone, and a
//! `chrono::Local::now()` read on the backend would silently assume the
//! machine's zone is the user's. `set_scheduled` (Story 4.8,
//! `orgsidian-shell-app`) already established the convention of taking the
//! frontend's local calendar day as a plain `YYYY-MM-DD` string; both queries
//! follow it. ISO-8601 date columns sort lexicographically (schema note in
//! `migrations/0001_initial-schema.sql`), so `<=`/`BETWEEN` on the TEXT
//! column is a valid overdue-or-today (or overdue-or-in-window) range scan
//! without parsing either side.
//!
//! # Why `week`'s `+6 days` arithmetic runs in SQLite, not Rust
//!
//! `orgsidian-index` is a LEAF crate and the project's established discipline
//! (Story 1.12 Dev Notes §11, `orgsidian-core::test_support::perf`'s
//! hand-rolled clock) is to keep `chrono`/`time` out of leaf crates. SQLite's
//! built-in `date(?1, '+6 days')` scalar function does the one piece of date
//! arithmetic this query needs, so `week` adds zero new Rust dependencies.
//!
//! # Why DONE items are excluded
//!
//! An agenda that still surfaces every task ever Scheduled/Deadline'd for
//! today, closed ones included, does not answer "what is left of my day"
//! (FR-6/FR-7 intent). Filtered out here rather than left to the frontend so
//! a slow render on a large Vault never ships rows the UI immediately drops.
//!
//! # What this does NOT do (deferred, per `deferred-work.md`)
//!
//! Recurring-timestamp expansion (`+1w` repeaters) is Epic 7 turf — the
//! schema stores only the literal date the parser saw
//! (`migrations/0001_initial-schema.sql`'s "NOT MODELLED IN v1" note), so a
//! repeating task shows on its stored date only, not on every future
//! occurrence.

use rusqlite::Connection;

use crate::error::IndexError;

/// One Agenda row: a Scheduled-today or Deadline-overdue-or-today Headline,
/// carrying enough source-file identity for the frontend's per-file grouping
/// and click-to-open (`/editor/$filePath/$headlineId`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AgendaItem {
    /// `headlines.id` — the click-to-open target's identity (the route's
    /// `$headlineId`).
    pub headline_id: i64,
    /// `files.path`, verbatim (whatever storage form Story 3.6 chose) — the
    /// grouping key and the other half of the click-to-open target (the
    /// route's `$filePath`).
    pub file_path: String,
    /// Headline title, stars/keyword/tags already stripped.
    pub title: String,
    /// `headlines.byte_start` — the section's byte offset in the file's
    /// source text. Carried so the click-to-open editor can place the cursor
    /// at the Headline itself, not just open the file (the route's optional
    /// `byteStart` search param).
    pub byte_start: i64,
    /// TODO keyword text, when the headline carries one (e.g. `TODO`,
    /// `NEXT`); `None` for a plain headline with no TODO state.
    pub todo_keyword: Option<String>,
    /// `SCHEDULED:` date, when this row matched via the Scheduled-today leg.
    pub scheduled_date: Option<String>,
    /// `SCHEDULED:` time, when the timestamp carries one.
    pub scheduled_time: Option<String>,
    /// `DEADLINE:` date, when this row matched via the Deadline leg.
    pub deadline_date: Option<String>,
    /// `DEADLINE:` time, when the timestamp carries one.
    pub deadline_time: Option<String>,
    /// `true` when `deadline_date` is strictly before the query's anchor date
    /// (`today` for [`today`], `start_date` for [`week`]) — the "overdue"
    /// half of "overdue-or-today" the frontend badges distinctly from a
    /// Deadline that is due today/within the window.
    pub overdue: bool,
    /// The calendar day (`YYYY-MM-DD`) this row is grouped under in an
    /// Agenda view: for [`today`], always the caller's `today` (every row
    /// qualifies precisely because it belongs on that one day). For [`week`],
    /// the Scheduled date when the row matched via the Scheduled leg, else
    /// the Deadline date — collapsed to `start_date` when that Deadline is
    /// overdue, mirroring [`today`]'s own overdue-collapses-to-today rule so
    /// a past-due item surfaces once, under the window's first ("current")
    /// day, rather than under its stale original date. The frontend groups
    /// and highlights purely off this field; it never re-derives it.
    pub agenda_date: String,
}

/// Scheduled-today + Deadline-overdue-or-today, across every non-quarantined
/// file, ordered by `(file_path, position)` (document order within a file,
/// files in path order).
///
/// `today` is an ISO-8601 `YYYY-MM-DD` calendar day — the frontend's local
/// `new Date()`, never a server-side clock read (see module docs).
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the query fails to prepare or run.
pub fn today(conn: &Connection, today: &str) -> Result<Vec<AgendaItem>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT h.id, f.path, h.title, h.byte_start, h.todo_keyword,
                h.scheduled_date, h.scheduled_time,
                h.deadline_date, h.deadline_time
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND (h.todo_done IS NULL OR h.todo_done = 0)
           AND (
                h.scheduled_date = ?1
                OR (h.deadline_date IS NOT NULL AND h.deadline_date <= ?1)
           )
         ORDER BY f.path, h.position",
    )?;

    let rows = stmt.query_map([today], |row| {
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
            // Every row here qualified precisely because it belongs on
            // `today` (Scheduled-today, or Deadline overdue-or-today) — the
            // single-day view has no other grouping day to compute.
            agenda_date: today.to_string(),
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Scheduled-within-the-window + Deadline-overdue-or-within-the-window, across
/// every non-quarantined file, for the caller-supplied rolling 7-day window
/// `[start_date, start_date + 6 days]` inclusive. Sorted by
/// [`AgendaItem::agenda_date`] (a stable sort, so same-day rows preserve the
/// SQL fetch's `(file_path, position)` order) — the frontend's "grouped by
/// date" AC is a stable partition of this already-sorted list, no second sort
/// needed client-side. The AC's "current day" is `start_date` itself.
///
/// `start_date` is an ISO-8601 `YYYY-MM-DD` calendar day — the frontend's
/// local `new Date()`, never a server-side clock read (see module docs).
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the query fails to prepare or run.
pub fn week(conn: &Connection, start_date: &str) -> Result<Vec<AgendaItem>, IndexError> {
    // The one piece of date arithmetic this query needs (`+6 days`) runs in
    // SQLite, not Rust — see the module docs on why `orgsidian-index` (a LEAF
    // crate) does not pull in `chrono` for it.
    let end_date: String =
        conn.query_row("SELECT date(?1, '+6 days')", [start_date], |row| row.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT h.id, f.path, h.title, h.byte_start, h.todo_keyword,
                h.scheduled_date, h.scheduled_time,
                h.deadline_date, h.deadline_time
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE f.quarantined = 0
           AND h.kind = 'headline'
           AND (h.todo_done IS NULL OR h.todo_done = 0)
           AND (
                (h.scheduled_date IS NOT NULL AND h.scheduled_date BETWEEN ?1 AND ?2)
                OR (h.deadline_date IS NOT NULL AND h.deadline_date <= ?2)
           )
         ORDER BY f.path, h.position",
    )?;

    let mut items = stmt
        .query_map(rusqlite::params![start_date, end_date], |row| {
            let scheduled_date: Option<String> = row.get(5)?;
            let deadline_date: Option<String> = row.get(7)?;
            let overdue = deadline_date
                .as_deref()
                .is_some_and(|date| date < start_date);
            // Precedence: an in-window Scheduled date wins (a row can carry
            // both a Scheduled and an unrelated Deadline); otherwise fall
            // back to the Deadline leg, collapsing an overdue Deadline onto
            // `start_date` (mirrors `today`'s overdue-collapses-to-today
            // rule, extended so the window's first/"current" day is the
            // collapse target).
            let agenda_date = match scheduled_date.as_deref() {
                Some(date) if date >= start_date && date <= end_date.as_str() => date.to_string(),
                _ => match deadline_date.as_deref() {
                    Some(date) if !overdue => date.to_string(),
                    _ => start_date.to_string(),
                },
            };
            Ok(AgendaItem {
                headline_id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                byte_start: row.get(3)?,
                todo_keyword: row.get(4)?,
                scheduled_date,
                scheduled_time: row.get(6)?,
                deadline_date,
                deadline_time: row.get(8)?,
                overdue,
                agenda_date,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // Stable sort: ties (same `agenda_date`) keep the SQL fetch's
    // `(file_path, position)` order, since `sort_by` never reorders equal
    // elements. This is the backend doing the sort the frontend must not
    // redo — the same "partition an already-sorted list" convention `today`
    // established for per-file grouping, applied here to per-date grouping.
    items.sort_by(|a, b| a.agenda_date.cmp(&b.agenda_date));

    Ok(items)
}

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

    /// A minimal one-headline file, defaults chosen so a caller only sets the
    /// fields its scenario cares about.
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

    #[test]
    fn includes_scheduled_today_and_excludes_other_days() {
        let mut conn = open_test_db();
        let mut h_today = headline("Scheduled today", 0);
        h_today.scheduled_date = Some("2026-09-05".to_string());
        let mut h_tomorrow = headline("Scheduled tomorrow", 1);
        h_tomorrow.scheduled_date = Some("2026-09-06".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![h_today, h_tomorrow])).expect("upsert");

        let items = today(&conn, "2026-09-05").expect("query");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Scheduled today");
        assert_eq!(items[0].scheduled_date.as_deref(), Some("2026-09-05"));
        assert!(!items[0].overdue);
    }

    #[test]
    fn includes_deadline_overdue_and_due_today_but_not_future() {
        let mut conn = open_test_db();
        let mut overdue = headline("Overdue deadline", 0);
        overdue.deadline_date = Some("2026-09-01".to_string());
        let mut due_today = headline("Due today", 1);
        due_today.deadline_date = Some("2026-09-05".to_string());
        let mut future = headline("Future deadline", 2);
        future.deadline_date = Some("2026-09-10".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![overdue, due_today, future]))
            .expect("upsert");

        let items = today(&conn, "2026-09-05").expect("query");

        assert_eq!(items.len(), 2);
        let by_title: std::collections::HashMap<_, _> = items
            .iter()
            .map(|i| (i.title.as_str(), i.overdue))
            .collect();
        assert_eq!(by_title.get("Overdue deadline"), Some(&true));
        assert_eq!(by_title.get("Due today"), Some(&false));
        assert!(!by_title.contains_key("Future deadline"));
    }

    #[test]
    fn excludes_done_items() {
        let mut conn = open_test_db();
        let mut done = headline("Already done", 0);
        done.scheduled_date = Some("2026-09-05".to_string());
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("a.org", vec![done])).expect("upsert");

        let items = today(&conn, "2026-09-05").expect("query");

        assert!(items.is_empty(), "DONE items must not appear in the agenda");
    }

    #[test]
    fn excludes_quarantined_files() {
        let mut conn = open_test_db();
        crate::quarantine_file(&mut conn, "bad.org", 1, 1, "parse error").expect("quarantine");

        let items = today(&conn, "2026-09-05").expect("query");

        assert!(items.is_empty());
    }

    #[test]
    fn groups_by_file_then_document_position() {
        let mut conn = open_test_db();
        let mut b1 = headline("b first", 0);
        b1.scheduled_date = Some("2026-09-05".to_string());
        let mut b2 = headline("b second", 1);
        b2.scheduled_date = Some("2026-09-05".to_string());
        let mut a1 = headline("a first", 0);
        a1.scheduled_date = Some("2026-09-05".to_string());
        crate::upsert_file(&mut conn, &file("b.org", vec![b1, b2])).expect("upsert b");
        crate::upsert_file(&mut conn, &file("a.org", vec![a1])).expect("upsert a");

        let items = today(&conn, "2026-09-05").expect("query");

        let ordering: Vec<_> = items
            .iter()
            .map(|i| (i.file_path.as_str(), i.title.as_str()))
            .collect();
        assert_eq!(
            ordering,
            vec![
                ("a.org", "a first"),
                ("b.org", "b first"),
                ("b.org", "b second")
            ]
        );
    }

    #[test]
    fn week_includes_scheduled_within_window_boundaries_and_excludes_outside() {
        let mut conn = open_test_db();
        let mut before = headline("Scheduled day before window", 0);
        before.scheduled_date = Some("2026-09-04".to_string());
        let mut start = headline("Scheduled on start day", 1);
        start.scheduled_date = Some("2026-09-05".to_string());
        let mut end = headline("Scheduled on end day", 2);
        end.scheduled_date = Some("2026-09-11".to_string());
        let mut after = headline("Scheduled day after window", 3);
        after.scheduled_date = Some("2026-09-12".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![before, start, end, after]))
            .expect("upsert");

        let items = week(&conn, "2026-09-05").expect("query");

        let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Scheduled on start day", "Scheduled on end day"]
        );
        assert_eq!(items[0].agenda_date, "2026-09-05");
        assert_eq!(items[1].agenda_date, "2026-09-11");
    }

    #[test]
    fn week_deadline_overdue_collapses_to_start_day_due_within_window_on_own_day_future_excluded() {
        let mut conn = open_test_db();
        let mut overdue = headline("Overdue deadline", 0);
        overdue.deadline_date = Some("2026-09-01".to_string());
        let mut due_mid_week = headline("Due mid-week", 1);
        due_mid_week.deadline_date = Some("2026-09-08".to_string());
        let mut future = headline("Future deadline", 2);
        future.deadline_date = Some("2026-09-20".to_string());
        crate::upsert_file(
            &mut conn,
            &file("a.org", vec![overdue, due_mid_week, future]),
        )
        .expect("upsert");

        let items = week(&conn, "2026-09-05").expect("query");

        assert_eq!(
            items.len(),
            2,
            "future deadline outside the window is excluded"
        );
        let by_title: std::collections::HashMap<_, _> = items
            .iter()
            .map(|i| (i.title.as_str(), (i.overdue, i.agenda_date.as_str())))
            .collect();
        assert_eq!(
            by_title.get("Overdue deadline"),
            Some(&(true, "2026-09-05")),
            "an overdue deadline collapses onto the window's first/current day"
        );
        assert_eq!(
            by_title.get("Due mid-week"),
            Some(&(false, "2026-09-08")),
            "a deadline due within the window groups under its own day"
        );
    }

    #[test]
    fn week_excludes_done_items() {
        let mut conn = open_test_db();
        let mut done = headline("Already done", 0);
        done.scheduled_date = Some("2026-09-06".to_string());
        done.todo_keyword = Some("DONE".to_string());
        done.todo_done = Some(true);
        crate::upsert_file(&mut conn, &file("a.org", vec![done])).expect("upsert");

        let items = week(&conn, "2026-09-05").expect("query");

        assert!(
            items.is_empty(),
            "DONE items must not appear in the week agenda"
        );
    }

    #[test]
    fn week_excludes_quarantined_files() {
        let mut conn = open_test_db();
        crate::quarantine_file(&mut conn, "bad.org", 1, 1, "parse error").expect("quarantine");

        let items = week(&conn, "2026-09-05").expect("query");

        assert!(items.is_empty());
    }

    #[test]
    fn week_scheduled_leg_wins_grouping_over_an_unrelated_overdue_deadline() {
        let mut conn = open_test_db();
        // Both legs match: Scheduled for a mid-window day AND an unrelated,
        // long-overdue Deadline. Grouping must follow the Scheduled date, not
        // collapse to the window's first day.
        let mut both = headline("Scheduled + stale deadline", 0);
        both.scheduled_date = Some("2026-09-07".to_string());
        both.deadline_date = Some("2026-08-01".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![both])).expect("upsert");

        let items = week(&conn, "2026-09-05").expect("query");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].agenda_date, "2026-09-07");
        assert!(
            items[0].overdue,
            "the overdue flag still reflects the stale deadline independent of grouping"
        );
    }

    #[test]
    fn week_scheduled_out_of_window_falls_back_to_in_window_deadline_day() {
        let mut conn = open_test_db();
        // The subtlest arm of the `agenda_date` derivation: a row carrying a
        // Scheduled date OUTSIDE the window (here in the past, a started-but-
        // not-done task) but pulled in via an in-window, non-overdue Deadline.
        // The Scheduled leg must NOT win the grouping key (its date is out of
        // window); the row groups under the Deadline's own day. Without the
        // `date >= start_date && date <= end_date` bounds on the Scheduled
        // guard, `agenda_date` would become the out-of-window Scheduled date,
        // which the frontend's fixed 7-day window would then silently drop.
        let mut past_sched_due_this_week = headline("Started earlier, due this week", 0);
        past_sched_due_this_week.scheduled_date = Some("2026-09-01".to_string());
        past_sched_due_this_week.deadline_date = Some("2026-09-08".to_string());
        crate::upsert_file(&mut conn, &file("a.org", vec![past_sched_due_this_week]))
            .expect("upsert");

        let items = week(&conn, "2026-09-05").expect("query");

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].agenda_date, "2026-09-08",
            "an out-of-window Scheduled date must not win grouping; the row groups \
             under its in-window Deadline day"
        );
        assert!(
            !items[0].overdue,
            "the in-window Deadline is not overdue (2026-09-08 >= start 2026-09-05)"
        );
    }

    #[test]
    fn week_groups_by_date_then_file_then_document_position() {
        let mut conn = open_test_db();
        // Two files, interleaved insertion order, spanning two distinct
        // agenda dates, proves the sort is by `agenda_date` first (not
        // insertion order or file order).
        let mut b_day2 = headline("b day2 first", 0);
        b_day2.scheduled_date = Some("2026-09-06".to_string());
        let mut b_day2_second = headline("b day2 second", 1);
        b_day2_second.scheduled_date = Some("2026-09-06".to_string());
        let mut a_day1 = headline("a day1", 0);
        a_day1.scheduled_date = Some("2026-09-05".to_string());
        crate::upsert_file(&mut conn, &file("b.org", vec![b_day2, b_day2_second]))
            .expect("upsert b");
        crate::upsert_file(&mut conn, &file("a.org", vec![a_day1])).expect("upsert a");

        let items = week(&conn, "2026-09-05").expect("query");

        let ordering: Vec<_> = items
            .iter()
            .map(|i| {
                (
                    i.agenda_date.as_str(),
                    i.file_path.as_str(),
                    i.title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            ordering,
            vec![
                ("2026-09-05", "a.org", "a day1"),
                ("2026-09-06", "b.org", "b day2 first"),
                ("2026-09-06", "b.org", "b day2 second"),
            ]
        );
    }
}
