//! Headline-location lookup by rowid (FR-8 Story 7.6 support).
//!
//! An ADDITIVE free function outside the frozen [`super::IndexQuery`] trait
//! (Story 6.5 semver gate): the clock manager (`orgsidian-core::clock`) needs
//! to turn an app-wide headline rowid into the source file plus the headline's
//! byte span, so it can splice `CLOCK:` lines into the `:LOGBOOK:` drawer of
//! the right headline. It is deliberately NOT a method on `IndexQuery` — that
//! surface is `cargo-semver-checks`-frozen and this story must not touch it —
//! and it returns its own small `#[non_exhaustive]`-free struct rather than an
//! `AgendaItem`, which carries agenda semantics this lookup does not want.

use rusqlite::Connection;

use crate::error::IndexError;

/// Where a headline lives: its source file path (verbatim, as `files.path`
/// stores it — the vault-relative, `/`-normalized `rel_path` the agenda rows
/// also carry) and the byte span of its whole section in that file's source.
///
/// `byte_start` is `Headline::span.start` (the section start — the `*` of the
/// headline line), the anchor the clock manager analyzes the file at to find
/// the headline whose `:LOGBOOK:` a `CLOCK:` line belongs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlineLocation {
    /// `files.path` for the headline's source file.
    pub file_path: String,
    /// `headlines.byte_start` — the section's byte offset in the file source.
    pub byte_start: i64,
    /// `headlines.byte_end` — the section's end byte offset in the file source.
    pub byte_end: i64,
}

/// Resolve the [`HeadlineLocation`] for `headlines.id` = `id`, or `None` when
/// no headline carries that rowid (a stale/desynced id — the caller decides
/// how to recover).
///
/// # Errors
///
/// [`IndexError::Sqlite`] if the query fails to prepare or run.
pub fn headline(conn: &Connection, id: i64) -> Result<Option<HeadlineLocation>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT f.path, h.byte_start, h.byte_end
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE h.id = ?1",
    )?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(HeadlineLocation {
            file_path: row.get(0)?,
            byte_start: row.get(1)?,
            byte_end: row.get(2)?,
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
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

    fn headline_input(title: &str, byte_start: i64, byte_end: i64) -> HeadlineInput {
        HeadlineInput {
            level: 1,
            position: 0,
            byte_start,
            byte_end,
            todo_keyword: None,
            todo_done: None,
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

    #[test]
    fn resolves_an_existing_headline_to_its_file_and_span() {
        let mut conn = open_test_db();
        crate::upsert_file(
            &mut conn,
            &FileIndexInput {
                rel_path: "notes/tasks.org".to_string(),
                mtime_ns: 1,
                size_bytes: 1,
                preamble: None,
                headlines: vec![headline_input("Track me", 40, 128)],
            },
        )
        .expect("upsert");

        let id: i64 = conn
            .query_row(
                "SELECT id FROM headlines WHERE kind = 'headline'",
                (),
                |row| row.get(0),
            )
            .expect("headline id");

        let location = headline(&conn, id).expect("query").expect("some location");
        assert_eq!(location.file_path, "notes/tasks.org");
        assert_eq!(location.byte_start, 40);
        assert_eq!(location.byte_end, 128);
    }

    #[test]
    fn returns_none_for_an_unknown_id() {
        let conn = open_test_db();
        assert_eq!(headline(&conn, 9999).expect("query"), None);
    }
}
