//! Implements FR-7 (Agenda Today — Story 6.3 v0.1 subset: Scheduled items for
//! `today` + Deadline items overdue-or-today, across the whole Vault, grouped
//! by source file). The full Today Dashboard (Today-Tag, Inbox preview,
//! Active Clock) is Epic 7 (Story 7.1); Week/Custom ranges are Stories
//! 6.4/7.4. Story 6.5 freezes `agenda::{today, week, custom}` together as the
//! v0.1 `IndexQuery` baseline surface — this file ships the first of the
//! three, already real and tested, for that trait to wrap.
//!
//! [`today`] is the query behind `/today`
//! (`shell-ui/src/components/agenda/AgendaToday.tsx`): a single `SELECT` over
//! `headlines` joined to `files`, ordered by `(file_path, position)` so the
//! frontend's "grouped by source file" AC is a stable partition of an
//! already-sorted list — no second sort needed client-side.
//!
//! # Why `today` is a caller-supplied string, not a server-side clock read
//!
//! The index has no notion of the user's timezone, and a
//! `chrono::Local::now()` read on the backend would silently assume the
//! machine's zone is the user's. `set_scheduled` (Story 4.8,
//! `orgsidian-shell-app`) already established the convention of taking the
//! frontend's local calendar day as a plain `YYYY-MM-DD` string; this query
//! follows it. ISO-8601 date columns sort lexicographically (schema note in
//! `migrations/0001_initial-schema.sql`), so `<=` on the TEXT column is a
//! valid overdue-or-today range scan without parsing either side.
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
    /// `true` when `deadline_date` is strictly before `today` — the
    /// "overdue" half of "overdue-or-today" the frontend badges distinctly
    /// from a Deadline that is due today.
    pub overdue: bool,
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
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
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
}
