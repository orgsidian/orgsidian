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
/// also carry), the byte span of its whole section, and its document-order
/// ordinal among the file's headlines.
///
/// `byte_start` is `Headline::span.start` (the section start — the `*` of the
/// headline line). It is only valid against the file content as it was at the
/// last scan: an in-session `CLOCK:` write shifts later offsets while the index
/// stays frozen (nothing resyncs it in-process), so it MUST NOT be trusted as an
/// absolute splice offset into freshly-read content.
///
/// `ordinal` is the STABLE identity for re-locating a headline in freshly-read
/// content: the 0-based count of headlines in the same file with a smaller
/// `byte_start`, i.e. its position in document (pre-order) order. Clocking never
/// adds, removes, or reorders headlines, so the ordinal survives every `CLOCK:`
/// edit — the clock manager re-parses the current file and picks the headline at
/// this ordinal rather than trusting the stale `byte_start`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlineLocation {
    /// `files.path` for the headline's source file.
    pub file_path: String,
    /// `headlines.byte_start` — the section's byte offset AT THE LAST SCAN (see
    /// the type doc: never a live splice offset).
    pub byte_start: i64,
    /// `headlines.byte_end` — the section's end byte offset at the last scan.
    pub byte_end: i64,
    /// Document-order ordinal among the file's headlines (0-based) — the stable
    /// re-location key (see the type doc).
    pub ordinal: i64,
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
        "SELECT f.path, h.byte_start, h.byte_end,
                (SELECT COUNT(*) FROM headlines h2
                 WHERE h2.file_id = h.file_id
                   AND h2.kind = 'headline'
                   AND h2.byte_start < h.byte_start) AS ordinal
         FROM headlines h
         JOIN files f ON f.id = h.file_id
         WHERE h.id = ?1",
    )?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(HeadlineLocation {
            file_path: row.get(0)?,
            byte_start: row.get(1)?,
            byte_end: row.get(2)?,
            ordinal: row.get(3)?,
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
        // Sole headline in the file → document-order ordinal 0.
        assert_eq!(location.ordinal, 0);
    }

    #[test]
    fn ordinal_reflects_document_order_within_the_file() {
        let mut conn = open_test_db();
        // Three headlines in one file, ascending byte_start = document order.
        crate::upsert_file(
            &mut conn,
            &FileIndexInput {
                rel_path: "notes/many.org".to_string(),
                mtime_ns: 1,
                size_bytes: 1,
                preamble: None,
                headlines: vec![
                    headline_input("First", 0, 10),
                    headline_input("Second", 10, 20),
                    headline_input("Third", 20, 30),
                ],
            },
        )
        .expect("upsert");

        let ordinal_of = |title: &str| {
            let id: i64 = conn
                .query_row(
                    "SELECT id FROM headlines WHERE title = ?1 AND kind = 'headline'",
                    [title],
                    |row| row.get(0),
                )
                .expect("headline id");
            headline(&conn, id)
                .expect("query")
                .expect("location")
                .ordinal
        };
        assert_eq!(ordinal_of("First"), 0);
        assert_eq!(ordinal_of("Second"), 1);
        assert_eq!(ordinal_of("Third"), 2);
    }

    #[test]
    fn returns_none_for_an_unknown_id() {
        let conn = open_test_db();
        assert_eq!(headline(&conn, 9999).expect("query"), None);
    }
}
