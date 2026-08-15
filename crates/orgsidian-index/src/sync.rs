//! Parser-agnostic transactional sync engine (FR-17 derived index, LD-11
//! application-managed FTS, LD-42 checkpoint writes).
//!
//! This module turns index-native input structs into rows in the eight
//! normalized tables plus the two external-content FTS5 tables. It knows
//! SQLite and FTS5; it does **not** know `orgsidian_parser::Document`. The
//! `Document` → [`FileIndexInput`] mapping lives in `orgsidian-core`
//! (Story 3.6 Dev Note 1): `orgsidian-index` is a LEAF whose only permitted
//! wrapper is `orgsidian-core` (deny.toml:193-194), so it may not depend on
//! the parser leaf. The input types below are the contract core maps into.
//!
//! # The FTS external-content pairing is structural, not a convention
//!
//! Both FTS5 tables are external-content over `headlines`
//! (migrations/0001_initial-schema.sql:309-321): the text is read back from
//! `headlines` through `content_rowid = headlines.id`, there are **no
//! triggers**, and the sync engine MUST write the `'delete'` command rows
//! **before** it deletes or replaces a headline, in the mandated order
//! FTS-delete → `DELETE headlines` → `DELETE files`, all in one transaction
//! (schema:289-308). Skipping the `'delete'` emission does not merely stale
//! the index: combined with `headlines.id`'s reusable rowid (no
//! `AUTOINCREMENT`), a later `MATCH` returns a hit whose `content_rowid` now
//! resolves to a *different, live* headline — wrong results with no error.
//!
//! The mitigation the deferred-work rows demand (the FTS/`AUTOINCREMENT`
//! landmine + "no test covers FTS staleness") is exactly this: the headline
//! mutation and its FTS command row live in **one function, one transaction**
//! ([`clear_file_rows`] emits the `'delete'` rows for every removal path), so
//! no caller can forget the pairing. `tests/sync.rs` proves it on UPDATE,
//! DELETE, and rowid-reuse, with an anti-placebo drop of the emission.
//!
//! # One transaction per file, or many files per transaction
//!
//! The single-file entry points ([`upsert_file`], [`delete_file`],
//! [`quarantine_file`]) each open one `conn.transaction()` — the shape Epic 5's
//! incremental watcher calls per changed file. The [`SyncOp`] form applies to a
//! caller-owned [`Transaction`], so the Story 3.6 scan can batch a whole
//! 100-file checkpoint into ONE commit (LD-42) without nesting transactions.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::IndexError;

/// One `.org` file's fully-mapped index input — the contract `orgsidian-core`
/// produces from `orgsidian_parser::Document`. Field set mirrors the schema
/// columns (migrations/0001_initial-schema.sql). Carries no parser type.
#[derive(Debug, Clone, PartialEq)]
pub struct FileIndexInput {
    /// Vault-relative path, `/`-normalized — the identity key (`files.path`).
    pub rel_path: String,
    /// Filesystem mtime in nanoseconds; paired with [`size_bytes`](Self::size_bytes).
    pub mtime_ns: i64,
    /// File size in bytes; the other half of the "changed since indexed?" key.
    pub size_bytes: i64,
    /// Document preamble (content before the first headline), when present —
    /// stored as a synthetic `kind='preamble'` headline row so it is searchable.
    pub preamble: Option<PreambleInput>,
    /// Top-level headlines in document order; nesting via [`HeadlineInput::children`].
    pub headlines: Vec<HeadlineInput>,
}

/// The document preamble → the synthetic `kind='preamble'` row (level 0,
/// empty title, body = preamble text, `parent_id` NULL).
#[derive(Debug, Clone, PartialEq)]
pub struct PreambleInput {
    /// Raw preamble text; becomes the synthetic row's `body` (FTS-searchable).
    pub body: String,
    /// Byte span start of the preamble region.
    pub byte_start: i64,
    /// Byte span end of the preamble region.
    pub byte_end: i64,
    /// Links found in the preamble (attached to the synthetic preamble row).
    pub links: Vec<LinkInput>,
}

/// One headline → one `headlines` row plus its `tags`/`properties`/
/// `clock_entries`/`links` and the paired FTS rows. `children` recurse.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadlineInput {
    /// Heading depth (number of stars); `headlines.level`.
    pub level: i64,
    /// Sibling order in document order; `headlines.position`.
    pub position: i64,
    /// Byte span start of the whole section (`headlines.byte_start`).
    pub byte_start: i64,
    /// Byte span end of the whole section (`headlines.byte_end`).
    pub byte_end: i64,
    /// TODO keyword text, when the headline carries a recognized state. Present
    /// together with [`todo_done`](Self::todo_done) or both absent (schema CHECK).
    pub todo_keyword: Option<String>,
    /// Done-class of the TODO keyword; present iff [`todo_keyword`](Self::todo_keyword) is.
    pub todo_done: Option<bool>,
    /// Headline title (stars/keyword/tags already stripped); `headlines.title`.
    pub title: String,
    /// This section's own raw region; `headlines.body` (the FTS-content column).
    pub body: String,
    /// `SCHEDULED:` date (ISO-8601, date only), when present/parseable.
    pub scheduled_date: Option<String>,
    /// `SCHEDULED:` time (`HH:MM[:SS]`), when the timestamp carries one.
    pub scheduled_time: Option<String>,
    /// `DEADLINE:` date, when present/parseable.
    pub deadline_date: Option<String>,
    /// `DEADLINE:` time, when present.
    pub deadline_time: Option<String>,
    /// `CLOSED:` date, when present/parseable.
    pub closed_date: Option<String>,
    /// `CLOSED:` time, when present.
    pub closed_time: Option<String>,
    /// Tags in document order (colons already stripped).
    pub tags: Vec<String>,
    /// `:PROPERTIES:` key/value pairs (parser already collapsed duplicates).
    pub properties: Vec<(String, String)>,
    /// `:LOGBOOK:` CLOCK entries.
    pub clock_entries: Vec<ClockInput>,
    /// Links found in this headline's own region.
    pub links: Vec<LinkInput>,
    /// Nested subsections, in document order.
    pub children: Vec<HeadlineInput>,
}

/// One `:LOGBOOK:` CLOCK entry → one `clock_entries` row.
#[derive(Debug, Clone, PartialEq)]
pub struct ClockInput {
    /// Clock-in datetime (ISO-8601); `clock_entries.start_at`.
    pub start_at: String,
    /// Clock-out datetime; `None` while the clock is still running.
    pub end_at: Option<String>,
    /// Duration in whole seconds, when the entry is closed and parses.
    pub duration_seconds: Option<i64>,
}

/// One link → one `links` row. `kind` is the lowercased `LinkKind`
/// (`id`/`file`/`url`/`wiki`/`plain` — schema CHECK).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkInput {
    /// Link classification, lowercased to match the schema CHECK.
    pub kind: String,
    /// Raw target text (scheme prefix retained).
    pub target: String,
    /// `[[target][description]]` description, when present.
    pub description: Option<String>,
}

/// A single unit of sync work applied to a caller-owned [`Transaction`].
///
/// The Story 3.6 scan builds a `Vec<SyncOp>` per 100-file checkpoint and
/// applies the whole batch inside ONE `conn.transaction()` (LD-42's coupled
/// commit-per-checkpoint), so a 1000-file vault sends ~10 writer messages, not
/// 1000 — the calibration the `WRITER_CHANNEL_CAPACITY` deferred row wanted
/// (per-checkpoint batching, not a raised bound).
#[derive(Debug, Clone, PartialEq)]
pub enum SyncOp {
    /// Insert-or-replace a file's rows (incremental upsert).
    Upsert(FileIndexInput),
    /// Record a file the parser could not analyze (LD-41): a `files` row with
    /// `quarantined=1` and no headlines.
    Quarantine {
        /// Vault-relative path.
        rel_path: String,
        /// Filesystem mtime in nanoseconds.
        mtime_ns: i64,
        /// File size in bytes.
        size_bytes: i64,
        /// Human-readable quarantine reason (`quarantine_reason`).
        reason: String,
    },
    /// Remove a file and all its rows (FTS-delete-then-DELETE order).
    Delete {
        /// Vault-relative path.
        rel_path: String,
    },
}

impl SyncOp {
    /// Apply this op to `tx`. Composed into a batch by the checkpoint writer;
    /// the batch's single `tx.commit()` is the LD-42 checkpoint boundary.
    pub fn apply(&self, tx: &Transaction<'_>) -> Result<(), IndexError> {
        match self {
            SyncOp::Upsert(input) => upsert_file_tx(tx, input),
            SyncOp::Quarantine {
                rel_path,
                mtime_ns,
                size_bytes,
                reason,
            } => quarantine_file_tx(tx, rel_path, *mtime_ns, *size_bytes, reason),
            SyncOp::Delete { rel_path } => delete_file_tx(tx, rel_path),
        }
    }
}

/// Insert-or-replace one file's index rows inside its own transaction (LD-42
/// per-file durability, the shape Epic 5's watcher calls). Delegates to
/// [`upsert_file_tx`]; see it for the FTS pairing contract.
///
/// # Errors
///
/// [`IndexError::Sqlite`] on any SQL failure; [`IndexError::Sync`] on a
/// "cannot happen" invariant (the just-upserted `files` row not found).
pub fn upsert_file(conn: &mut Connection, input: &FileIndexInput) -> Result<(), IndexError> {
    let tx = conn.transaction()?;
    upsert_file_tx(&tx, input)?;
    tx.commit()?;
    Ok(())
}

/// Remove one file and all its rows inside its own transaction, honoring the
/// FTS-`'delete'`-then-`DELETE headlines`-then-`DELETE files` order.
pub fn delete_file(conn: &mut Connection, rel_path: &str) -> Result<(), IndexError> {
    let tx = conn.transaction()?;
    delete_file_tx(&tx, rel_path)?;
    tx.commit()?;
    Ok(())
}

/// Record `rel_path` as quarantined (LD-41): a `files` row with
/// `quarantined=1, quarantine_reason=reason` and **no** headlines. Any prior
/// rows for the path are cleared first (FTS-safe).
pub fn quarantine_file(
    conn: &mut Connection,
    rel_path: &str,
    mtime_ns: i64,
    size_bytes: i64,
    reason: &str,
) -> Result<(), IndexError> {
    let tx = conn.transaction()?;
    quarantine_file_tx(&tx, rel_path, mtime_ns, size_bytes, reason)?;
    tx.commit()?;
    Ok(())
}

/// The incremental skip predicate: is the file at `rel_path` already indexed
/// with a matching `(mtime_ns, size_bytes)`? The primitive that makes the
/// cached fast-open and resume-after-cancel cheap (Story 3.6 AC7) — an
/// unchanged file is neither re-read nor re-parsed nor re-written.
///
/// Returns `Ok(false)` when the path is absent. A quarantined-but-unchanged
/// file also reads as unchanged (its `(mtime, size)` still match), so a
/// re-scan does not re-parse a file that will only re-quarantine.
pub fn file_is_unchanged(
    conn: &Connection,
    rel_path: &str,
    mtime_ns: i64,
    size_bytes: i64,
) -> Result<bool, IndexError> {
    let found: Option<(i64, i64)> = conn
        .query_row(
            "SELECT mtime_ns, size_bytes FROM files WHERE path = ?1",
            params![rel_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(matches!(found, Some((m, s)) if m == mtime_ns && s == size_bytes))
}

/// ISO-8601 UTC "now", computed by SQLite so the index crate needs no clock
/// dependency. Millisecond precision, `T`/`Z`-framed to match the schema's
/// ISO-8601 date/time convention.
const INDEXED_AT_NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

/// Transaction-scoped upsert: the real work behind [`upsert_file`] and
/// [`SyncOp::Upsert`]. Clears the file's existing rows (FTS-safe), upserts the
/// `files` row, then inserts the preamble + headline tree with their paired
/// FTS rows — all against the caller's `tx`.
fn upsert_file_tx(tx: &Transaction<'_>, input: &FileIndexInput) -> Result<(), IndexError> {
    clear_file_rows(tx, &input.rel_path)?;

    tx.execute(
        &format!(
            "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at, quarantined, quarantine_reason)
             VALUES (?1, ?2, ?3, {INDEXED_AT_NOW}, 0, NULL)
             ON CONFLICT(path) DO UPDATE SET
                 mtime_ns          = excluded.mtime_ns,
                 size_bytes        = excluded.size_bytes,
                 indexed_at        = excluded.indexed_at,
                 quarantined       = 0,
                 quarantine_reason = NULL"
        ),
        params![input.rel_path, input.mtime_ns, input.size_bytes],
    )?;
    let file_id = file_id_for(tx, &input.rel_path)?;

    if let Some(preamble) = &input.preamble {
        insert_preamble(tx, file_id, preamble)?;
    }
    for headline in &input.headlines {
        insert_headline(tx, file_id, None, headline)?;
    }
    Ok(())
}

/// Transaction-scoped delete: FTS-`'delete'` the file's headlines, delete its
/// links + headlines, then the `files` row — the mandated order, in `tx`.
fn delete_file_tx(tx: &Transaction<'_>, rel_path: &str) -> Result<(), IndexError> {
    clear_file_rows(tx, rel_path)?;
    tx.execute("DELETE FROM files WHERE path = ?1", params![rel_path])?;
    Ok(())
}

/// Transaction-scoped quarantine: clear any prior rows, then upsert a
/// `quarantined=1` `files` row with a reason and no headlines.
fn quarantine_file_tx(
    tx: &Transaction<'_>,
    rel_path: &str,
    mtime_ns: i64,
    size_bytes: i64,
    reason: &str,
) -> Result<(), IndexError> {
    clear_file_rows(tx, rel_path)?;
    tx.execute(
        &format!(
            "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at, quarantined, quarantine_reason)
             VALUES (?1, ?2, ?3, {INDEXED_AT_NOW}, 1, ?4)
             ON CONFLICT(path) DO UPDATE SET
                 mtime_ns          = excluded.mtime_ns,
                 size_bytes        = excluded.size_bytes,
                 indexed_at        = excluded.indexed_at,
                 quarantined       = 1,
                 quarantine_reason = excluded.quarantine_reason"
        ),
        params![rel_path, mtime_ns, size_bytes, reason],
    )?;
    Ok(())
}

/// Remove every `headlines`/`links` row for `rel_path`, emitting the FTS
/// `'delete'` command rows FIRST while the original text is still readable
/// (external-content contract, schema:289-308). The `files` row is left in
/// place — callers upsert or delete it. A no-op when the path is not indexed.
///
/// This is the single choke point through which every removal passes, so the
/// FTS pairing cannot be forgotten by any caller (resolves the rowid-reuse /
/// FTS-staleness deferred rows).
fn clear_file_rows(tx: &Transaction<'_>, rel_path: &str) -> Result<(), IndexError> {
    let Some(file_id) = tx
        .query_row(
            "SELECT id FROM files WHERE path = ?1",
            params![rel_path],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    else {
        return Ok(());
    };

    // Step 1: FTS `'delete'` rows for every existing headline, using the exact
    // stored text as content-rowid resolution requires. Read fully before any
    // DELETE so the borrowed statement is dropped first.
    let existing: Vec<(i64, String, String)> = {
        let mut stmt = tx.prepare("SELECT id, title, body FROM headlines WHERE file_id = ?1")?;
        let rows = stmt.query_map(params![file_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect::<Result<_, _>>()?
    };
    for (headline_id, title, body) in &existing {
        tx.execute(
            "INSERT INTO fts_headlines (fts_headlines, rowid, title) VALUES ('delete', ?1, ?2)",
            params![headline_id, title],
        )?;
        tx.execute(
            "INSERT INTO fts_content (fts_content, rowid, body) VALUES ('delete', ?1, ?2)",
            params![headline_id, body],
        )?;
    }

    // Step 2: links (covers preamble/file-scoped links too), then headlines
    // (which cascade tags/properties/clock_entries). Order after the FTS
    // 'delete' rows, never before.
    tx.execute("DELETE FROM links WHERE file_id = ?1", params![file_id])?;
    tx.execute("DELETE FROM headlines WHERE file_id = ?1", params![file_id])?;
    Ok(())
}

/// Insert the synthetic `kind='preamble'` row + its paired FTS rows + links.
fn insert_preamble(
    tx: &Transaction<'_>,
    file_id: i64,
    preamble: &PreambleInput,
) -> Result<(), IndexError> {
    tx.execute(
        "INSERT INTO headlines
            (file_id, parent_id, kind, level, position, byte_start, byte_end, title, body)
         VALUES (?1, NULL, 'preamble', 0, 0, ?2, ?3, '', ?4)",
        params![
            file_id,
            preamble.byte_start,
            preamble.byte_end,
            preamble.body
        ],
    )?;
    let headline_id = tx.last_insert_rowid();
    insert_fts_rows(tx, headline_id, "", &preamble.body)?;
    for link in &preamble.links {
        insert_link(tx, file_id, Some(headline_id), link)?;
    }
    Ok(())
}

/// Insert one headline row (+ FTS, tags, properties, clocks, links), then
/// recurse into its children with the new rowid as their `parent_id`. Parent
/// is always inserted before its children, so `last_insert_rowid()` gives the
/// real parent rowid and the same-file `parent_id` invariant holds.
fn insert_headline(
    tx: &Transaction<'_>,
    file_id: i64,
    parent_id: Option<i64>,
    headline: &HeadlineInput,
) -> Result<(), IndexError> {
    tx.execute(
        "INSERT INTO headlines
            (file_id, parent_id, kind, level, position, byte_start, byte_end,
             todo_keyword, todo_done, title, body,
             scheduled_date, scheduled_time, deadline_date, deadline_time, closed_date, closed_time)
         VALUES (?1, ?2, 'headline', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            file_id,
            parent_id,
            headline.level,
            headline.position,
            headline.byte_start,
            headline.byte_end,
            headline.todo_keyword,
            headline.todo_done,
            headline.title,
            headline.body,
            headline.scheduled_date,
            headline.scheduled_time,
            headline.deadline_date,
            headline.deadline_time,
            headline.closed_date,
            headline.closed_time,
        ],
    )?;
    let headline_id = tx.last_insert_rowid();
    insert_fts_rows(tx, headline_id, &headline.title, &headline.body)?;

    for (position, tag) in headline.tags.iter().enumerate() {
        tx.execute(
            "INSERT INTO tags (headline_id, tag, position) VALUES (?1, ?2, ?3)",
            params![headline_id, tag, position as i64],
        )?;
    }
    for (key, value) in &headline.properties {
        // Parser collapses duplicate keys last-wins; OR REPLACE keeps that
        // contract against the (headline_id, key) primary key defensively.
        tx.execute(
            "INSERT OR REPLACE INTO properties (headline_id, key, value) VALUES (?1, ?2, ?3)",
            params![headline_id, key, value],
        )?;
    }
    for clock in &headline.clock_entries {
        tx.execute(
            "INSERT INTO clock_entries (headline_id, start_at, end_at, duration_seconds)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                headline_id,
                clock.start_at,
                clock.end_at,
                clock.duration_seconds
            ],
        )?;
    }
    for link in &headline.links {
        insert_link(tx, file_id, Some(headline_id), link)?;
    }

    for child in &headline.children {
        insert_headline(tx, file_id, Some(headline_id), child)?;
    }
    Ok(())
}

/// Insert the paired external-content FTS rows for a headline rowid. Adding
/// content to an external-content table is an INSERT keyed on the
/// `content_rowid` (`headlines.id`).
fn insert_fts_rows(
    tx: &Transaction<'_>,
    headline_id: i64,
    title: &str,
    body: &str,
) -> Result<(), IndexError> {
    tx.execute(
        "INSERT INTO fts_headlines (rowid, title) VALUES (?1, ?2)",
        params![headline_id, title],
    )?;
    tx.execute(
        "INSERT INTO fts_content (rowid, body) VALUES (?1, ?2)",
        params![headline_id, body],
    )?;
    Ok(())
}

/// Insert one `links` row.
fn insert_link(
    tx: &Transaction<'_>,
    file_id: i64,
    headline_id: Option<i64>,
    link: &LinkInput,
) -> Result<(), IndexError> {
    tx.execute(
        "INSERT INTO links (file_id, headline_id, kind, target, description)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            file_id,
            headline_id,
            link.kind,
            link.target,
            link.description
        ],
    )?;
    Ok(())
}

/// Insert-or-update a `vault_meta` key/value pair. Story 3.6 records the
/// canonical `vault_root` here at vault open (LD-40 vault-scoped index state).
/// A single statement, so it takes a shared `&Connection` and is callable
/// directly inside an `IndexWriter::execute` closure.
///
/// # Errors
///
/// [`IndexError::Sqlite`] on any SQL failure.
pub fn set_vault_meta(conn: &Connection, key: &str, value: &str) -> Result<(), IndexError> {
    conn.execute(
        &format!(
            "INSERT INTO vault_meta (key, value, updated_at) VALUES (?1, ?2, {INDEXED_AT_NOW})
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        ),
        params![key, value],
    )?;
    Ok(())
}

/// Read back the rowid of the `files` row for `rel_path`. Returns
/// [`IndexError::Sync`] rather than panicking if the row the caller just
/// upserted is absent (a "cannot happen" that must not tear down the writer).
fn file_id_for(tx: &Transaction<'_>, rel_path: &str) -> Result<i64, IndexError> {
    tx.query_row(
        "SELECT id FROM files WHERE path = ?1",
        params![rel_path],
        |row| row.get::<_, i64>(0),
    )
    .optional()?
    .ok_or_else(|| IndexError::Sync(format!("files row missing after upsert: {rel_path}")))
}
