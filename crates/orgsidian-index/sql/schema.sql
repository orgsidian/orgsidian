-- Orgsidian SQLite index — schema version 1.
--
-- Traces: LD-4 (rusqlite + locked PRAGMAs + FTS5 tokenizer), LD-11 (normalized
-- schema + index list; this file's location is mandated there), LD-14
-- (connection management), FR-17 (SQLite derived index).
--
-- FORWARD-ONLY RULE (LD-12): this file is the DDL for schema version 1. Once
-- Story 3.4 lands the migration runner, schema changes arrive as NEW migration
-- files and never as edits here. Editing this file after 3.4 silently diverges
-- fresh databases from migrated ones.
--
-- The whole file is executed top-to-bottom in a single `execute_batch` against
-- a fresh database (statements are in dependency order). It is deliberately
-- NOT `IF NOT EXISTS`-guarded: re-applying it to an initialized database must
-- fail loudly rather than half-succeed, so a migration bug cannot hide behind
-- silent idempotency.
--
-- The index is a DERIVED artifact (LD-13, LD-17): everything here is
-- reconstructible from the Vault's .org files, so a corrupt or stale database
-- is dropped and rebuilt rather than repaired.
--
-- Naming (architecture.md:694-699): tables snake_case plural, columns
-- snake_case singular, indices idx_<table>_<col1>_<col2>, foreign keys
-- <referenced_table_singular>_id.
--
-- Date/time storage: ISO-8601 TEXT, never epoch integers. Org timestamps are
-- wall-clock and timezone-less; converting them to epochs requires inventing a
-- timezone, which shifts `SCHEDULED: <2026-08-02 Sun>` across a day boundary
-- for anyone outside the assumed zone. ISO-8601 text sorts lexicographically in
-- chronological order, so BETWEEN range scans on the agenda indices work.
-- Dates and times are SEPARATE columns because an all-day SCHEDULED has no
-- time, and '2026-08-02' < '2026-08-02T09:00' would sort all-day entries
-- inconsistently against timed ones in a single-column encoding.
--
-- NOT MODELLED IN v1 (deliberate, recorded in deferred-work.md):
--   * Ranged/repeating timestamp fields — Timestamp.active, .end_date,
--     .end_time, .repeater, .delay. Recurring-task expansion is Epic 7 turf and
--     wants its own design pass; store what agenda-by-date needs now. LD-12
--     makes adding columns cheap and LD-13 makes the rebuild free.
--   * Preamble.directives (`#+KEYWORD: value`, including `#+TODO:` sequences) —
--     no table in the LD-11 table set; needed when per-file TODO configuration
--     or #+FILETAGS: becomes queryable (Epic 7/8).
--   * Drawers as a table — Headline.drawers has no table in LD-11's set and
--     gets none. :PROPERTIES: normalizes into `properties`, :LOGBOOK: into
--     `clock_entries`; every other drawer kind stays inside headlines.body and
--     is reachable through FTS.

-- One row per indexed .org file in the Vault.
--
-- `path` carries whatever string the caller hands over; the storage form
-- (absolute vs vault-relative, separator normalization, case-folding on
-- macOS/Windows) is Story 3.6's decision — it designates the Vault root and
-- therefore owns what a path means. The UNIQUE index below is the only
-- identity guarantee this schema makes.
--
-- `quarantined` / `quarantine_reason` implement the LD-41 malformed-file row.
-- LD-41's wording puts that flag in `vault_meta`; it lives here instead
-- because it is per-file state — see the note on `vault_meta` below.
CREATE TABLE files (
    id                INTEGER PRIMARY KEY,
    path              TEXT    NOT NULL,
    -- Filesystem mtime in nanoseconds; paired with size_bytes as the
    -- cheap "has this file changed since indexing?" check.
    mtime_ns          INTEGER NOT NULL,
    size_bytes        INTEGER NOT NULL,
    -- ISO-8601 datetime of the last successful index pass for this file.
    indexed_at        TEXT    NOT NULL,
    quarantined       INTEGER NOT NULL DEFAULT 0 CHECK (quarantined IN (0, 1)),
    quarantine_reason TEXT
);

-- The flattened headline tree, one row per section.
--
-- `id` is an explicit rowid alias and is LOAD-BEARING: it is the
-- `content_rowid` both FTS5 external-content tables resolve against. Renaming
-- or aliasing it away breaks search at query time, not at DDL time.
--
-- `kind` distinguishes a real headline from the synthetic row that carries a
-- file's preamble (the content before the first headline: #+TITLE:, intro
-- prose). Without such a row the preamble is unsearchable, which is a silent
-- FR-12 gap. A preamble row is level 0, title '', body = Preamble.text,
-- parent_id NULL. The `kind` column exists rather than overloading `level = 0`
-- because the parser already emits level 0 as a degenerate sentinel inside
-- ERROR regions — conflating the two makes every `WHERE level = 0` ambiguous.
--
-- `level` has no CHECK constraint on purpose: the parser saturates at 255 and
-- emits 0 for malformed input, so a `BETWEEN 1 AND 6` guard would reject
-- documents the parser accepts.
--
-- Populating these rows is the Story 3.6 sync engine's job.
CREATE TABLE headlines (
    id             INTEGER PRIMARY KEY,
    file_id        INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    -- NULL at top level; self-referential for subtree traversal.
    parent_id      INTEGER REFERENCES headlines (id) ON DELETE CASCADE,
    kind           TEXT    NOT NULL DEFAULT 'headline'
                       CHECK (kind IN ('headline', 'preamble')),
    level          INTEGER NOT NULL,
    -- Sibling order in document order.
    position       INTEGER NOT NULL,
    -- Byte span of the whole section (headline line through last child),
    -- from Headline.span.
    byte_start     INTEGER NOT NULL,
    byte_end       INTEGER NOT NULL,
    -- Both NULL together when the headline carries no TODO state.
    todo_keyword   TEXT,
    todo_done      INTEGER CHECK (todo_done IN (0, 1)),
    -- Stars, TODO keyword and trailing tags already stripped by the parser.
    title          TEXT    NOT NULL,
    -- Headline.raw: this section's OWN region only (headline line + planning +
    -- drawers + body, excluding child sections) — the granularity an FTS hit
    -- needs to jump to.
    body           TEXT    NOT NULL,
    scheduled_date TEXT,
    scheduled_time TEXT,
    deadline_date  TEXT,
    deadline_time  TEXT,
    closed_date    TEXT,
    closed_time    TEXT
);

-- Headline tags, colons already stripped by the parser. `position` preserves
-- document order and, with headline_id, forms the identity of a tag slot.
CREATE TABLE tags (
    headline_id INTEGER NOT NULL REFERENCES headlines (id) ON DELETE CASCADE,
    tag         TEXT    NOT NULL,
    position    INTEGER NOT NULL,
    PRIMARY KEY (headline_id, position)
);

-- Normalized :PROPERTIES: drawer entries. The (headline_id, key) primary key
-- matches the parser's documented last-wins collapse of duplicate keys.
CREATE TABLE properties (
    headline_id INTEGER NOT NULL REFERENCES headlines (id) ON DELETE CASCADE,
    key         TEXT    NOT NULL,
    value       TEXT    NOT NULL,
    PRIMARY KEY (headline_id, key)
);

-- Normalized :LOGBOOK: CLOCK entries.
--
-- `end_at` NULL means the clock is STILL RUNNING — Story 7.7's "prior session
-- running clock" prompt reads exactly this condition, so the column must stay
-- nullable. `duration_seconds` mirrors the parser, which already serializes
-- the delta as whole seconds.
CREATE TABLE clock_entries (
    id               INTEGER PRIMARY KEY,
    headline_id      INTEGER NOT NULL REFERENCES headlines (id) ON DELETE CASCADE,
    start_at         TEXT    NOT NULL,
    end_at           TEXT,
    duration_seconds INTEGER
);

-- Every link the parser found, for FR-13 backlink traversal.
--
-- `headline_id` is nullable because Preamble.links exists: a link before the
-- first headline has no owning headline. `file_id` is NOT NULL in both cases.
-- `kind` mirrors the five LinkKind variants, lowercased.
--
-- The parser's link scan is textual and deliberately over-reports links inside
-- verbatim blocks (docs/parser/KNOWN_DIVERGENCES.md entry 1). The schema stores
-- what it is given; filtering is not a DDL concern.
CREATE TABLE links (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    headline_id INTEGER REFERENCES headlines (id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL
                    CHECK (kind IN ('id', 'file', 'url', 'wiki', 'plain')),
    target      TEXT    NOT NULL,
    description TEXT
);

-- Vault-SCOPED index state: vault root, last full rebuild, tokenizer in use.
-- A genuine key/value bag — deliberately NOT per-file state. Per-file flags
-- (notably `quarantined`) live as columns on `files`, where they can be
-- indexed, joined, and filtered by `WHERE quarantined = 0`; a synthesized
-- `quarantined:/path/to/file.org` key here would be none of those things.
CREATE TABLE vault_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Human-readable migration audit trail: one row per applied migration.
--
-- This is NOT redundant with `PRAGMA user_version`, and neither should be
-- removed in favour of the other. They have distinct jobs:
--   * `PRAGMA user_version` is the MACHINE authority (LD-12) — what the
--     migration runner reads and writes, and what LD-13's
--     "user_version mismatch -> full rebuild" check compares.
--   * `_schema_version` is the HUMAN audit trail — what `orgsidian index stats`
--     (Story 3.7) prints and what a developer sees opening the file in sqlite3.
--
-- This schema only DECLARES the table; it inserts nothing. The migration that
-- applies version 1 owns the first row.
CREATE TABLE _schema_version (
    version     INTEGER PRIMARY KEY,
    description TEXT    NOT NULL,
    applied_at  TEXT    NOT NULL
);

-- ---------------------------------------------------------------------------
-- FTS5 external-content search tables (LD-4, FR-12)
-- ---------------------------------------------------------------------------
--
-- Both are external-content tables over `headlines`: the text is not duplicated
-- into an FTS-owned content table, it is read back from `headlines` through
-- `content_rowid = headlines.id`. The indexed column names MUST match real
-- `headlines` columns (`title`, `body`) — FTS5 resolves them by name against
-- the content table, and a rename on either side breaks snippet()/highlight()
-- silently at query time rather than loudly at DDL time.
--
-- Tokenizer (LD-4): `porter` is a WRAPPER tokenizer that takes the underlying
-- tokenizer and its arguments as its own arguments, so the order in the single
-- option string is porter-first. `remove_diacritics 2` (SQLite >= 3.27) folds
-- diacritics that are part of the base codepoint, which `1` mishandles.
--
-- NO TRIGGERS, BY MANDATE (LD-11): FTS sync is application-managed. The Story
-- 3.6 sync engine owns the obligation to INSERT into these tables alongside
-- every `headlines` insert AND to write the `'delete'` command rows that
-- external-content tables require before an update or delete — an
-- external-content FTS5 table cannot recover the old text on its own. Adding a
-- CREATE TRIGGER here is a schema violation, not an optimization.
CREATE VIRTUAL TABLE fts_headlines USING fts5(
    title,
    content='headlines',
    content_rowid='id',
    tokenize='porter unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE fts_content USING fts5(
    body,
    content='headlines',
    content_rowid='id',
    tokenize='porter unicode61 remove_diacritics 2'
);

-- ---------------------------------------------------------------------------
-- Indices (LD-11)
-- ---------------------------------------------------------------------------
--
-- LD-11 names five index targets: (file_path), (headline_id), (scheduled_date),
-- (deadline_date), (tag, headline_id). The set below is a DISCLOSED SUPERSET:
-- idx_headlines_file_id, idx_headlines_parent_id, idx_links_file_id and
-- idx_links_target go beyond it. The first three are the foreign-key columns
-- the ON DELETE CASCADEs traverse — an unindexed FK turns every
-- `DELETE FROM files` into a full scan of `headlines`/`links` — and the last is
-- the FR-13 backlink traversal scan, without which the `links` table cannot
-- answer the question it exists for.

-- LD-11's (file_path). Declared as a named UNIQUE index rather than an inline
-- UNIQUE constraint so it is greppable, follows the naming convention, and
-- shows up in PRAGMA index_list under a name a test can assert.
CREATE UNIQUE INDEX idx_files_path ON files (path);

-- Per-file listing + subtree traversal; both are CASCADE targets.
CREATE INDEX idx_headlines_file_id ON headlines (file_id);
CREATE INDEX idx_headlines_parent_id ON headlines (parent_id);

-- LD-11 agenda range scans. Plain, not partial: a `WHERE ... IS NOT NULL`
-- variant is a Story 7.1 optimization to make against real query plans, not a
-- guess to bake in now.
CREATE INDEX idx_headlines_scheduled_date ON headlines (scheduled_date);
CREATE INDEX idx_headlines_deadline_date ON headlines (deadline_date);

-- LD-11's (tag, headline_id), column order exactly as specified: a tag filter
-- scans tag-first.
CREATE INDEX idx_tags_tag_headline_id ON tags (tag, headline_id);

-- LD-11's (headline_id) family — the CASCADE + per-headline lookup paths.
CREATE INDEX idx_properties_headline_id ON properties (headline_id);
CREATE INDEX idx_clock_entries_headline_id ON clock_entries (headline_id);
CREATE INDEX idx_links_headline_id ON links (headline_id);
CREATE INDEX idx_links_file_id ON links (file_id);

-- FR-13 backlink traversal: "which links point at this target?".
CREATE INDEX idx_links_target ON links (target);
