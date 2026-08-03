# Story 3.3: Define SQLite schema + locked PRAGMAs

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 27

## Story

As the **user opening my Vault**,
I want a normalized SQLite schema covering `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `vault_meta`, `_schema_version` with FTS5 virtual tables for `fts_headlines` and `fts_content`,
So that Epic 7/8 queries (agenda + search + backlinks) have a performant index from day-1 (LD-4, LD-11).

**Traces:** LD-4 (rusqlite + locked PRAGMAs + FTS5 tokenizer, architecture.md:66), LD-11 (normalized schema + index list + `sql/schema.sql` location, architecture.md:398), LD-14 (connection management — PRAGMAs re-stated, architecture.md:404), FR-17 (SQLite derived index, architecture.md:1060), SQLite naming conventions (architecture.md:694-699), Database boundary rule #5 (architecture.md:1036).

## Scope Fence (read first)

This story is the **DDL + connection-initialization scaffold** inside `crates/orgsidian-index/`, which today is an empty placeholder crate (`src/lib.rs` = 3 doc lines, `Cargo.toml` = zero dependencies). The deliverable is: `sql/schema.sql` (the DDL), `src/connection.rs` (open + locked PRAGMAs), `src/error.rs` (`IndexError`), the `lib.rs` re-exports, and integration tests that *actually apply the schema to a real on-disk database* and assert its shape. It is **NOT**:

- **NOT the migration runner** (Story 3.4). No `rusqlite_migration` dependency, no `migrations/` directory, no `M::up(...)`, no `to_latest()`, no `PRAGMA user_version` bump. This story ships the DDL that 3.4 will wrap; see Dev Note 7 for the seam contract that keeps the two from forking into drifting copies.
- **NOT the connection pool or writer task** (Story 3.5). No `deadpool-sqlite`, no `mpsc`, no Tokio, no `IndexUpdate`. `open()` returns one plain `rusqlite::Connection`.
- **NOT the query API** (Epics 7/8). No `src/query/`, no `IndexQuery` trait, no `SELECT` beyond what tests need to assert schema shape. The Database boundary rule (architecture.md:1036) says all reads go through `orgsidian-index::query::*` — that module does not exist yet and this story does not create it.
- **NOT the index-sync engine** (Story 3.6). Nothing walks a Vault, nothing calls the parser, nothing INSERTs real data. `orgsidian-index` gains **no** dependency on `orgsidian-parser`. The schema is designed *against* the Story 2.3 semantic types (Dev Note 2) but does not import them.
- **NOT the DB file-path policy.** Where the `.db` lives (OS data dir per LD-17/LD-40) is Story 3.6's decision. `open(path: &Path)` takes the path from the caller.
- **NOT rebuild / integrity policy** (LD-13 → Stories 3.4 + 3.7). No `PRAGMA integrity_check` call site, no drop-and-rebuild logic.
- **NOT LD-41 harness turf.** The "SQLite index corruption" row (`docs/failure-modes/coverage-matrix.md:17`, `tests/failure_modes.rs::sqlite_index_corruption_rebuild`) needs the *startup rebuild path*, which is Story 3.4/3.6. `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs` and `docs/failure-modes/coverage-matrix.md` stay **byte-untouched** (`EXPECTED_REMAINING_PLACEHOLDERS` stays at `8`).
- **NOT a cross-crate edge.** `orgsidian-index` is a LEAF (deny.toml:192-194 — only `orgsidian-core` may wrap it). Nothing consumes it yet, so **do not** add an `orgsidian-index` entry to `[workspace.dependencies]` and do not touch `orgsidian-core`.
- **NOT sentinel turf.** Byte-untouched: every `crates/*/tests/anchor.rs`, all of `crates/orgsidian-vault/`, `crates/orgsidian-parser/`, `.github/workflows/*`. `deny.toml` is touchable **only** if `cargo deny check bans` actually fires on a duplicate version introduced by `rusqlite` — and then only with a matching ledger row (AC5).

## Acceptance Criteria

### AC1 — `sql/schema.sql` is the single committed source of the DDL.

- `crates/orgsidian-index/sql/schema.sql` exists and contains the complete DDL: the 8 tables, the 2 FTS5 virtual tables, and the indices — in dependency order, executable top-to-bottom in one `execute_batch` against a fresh database.
- Header comment names the traces (LD-4, LD-11, FR-17) and states the forward-only rule: *this file is DDL for schema version 1; changes after Story 3.4 land as new migration files, never as edits here* (LD-12).
- `src/lib.rs` exposes `pub const SCHEMA_SQL: &str = include_str!("../sql/schema.sql");` — one canonical in-Rust handle so Story 3.4 wraps this exact text instead of copy-pasting it (Dev Note 7).
- Table set is exactly, and only: `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `vault_meta`, `_schema_version`.
- Naming follows architecture.md:694-699 without exception: tables `snake_case` plural, columns `snake_case` singular, indices `idx_<table>_<col1>_<col2>`, foreign keys `<referenced_table_singular>_id`.

### AC2 — Table shapes match the Story 2.3 semantic model (see Dev Note 2 for the column-by-column mapping).

- `files`: `id INTEGER PRIMARY KEY`, `path TEXT NOT NULL`, `mtime_ns INTEGER NOT NULL`, `size_bytes INTEGER NOT NULL`, `indexed_at TEXT NOT NULL`, `quarantined INTEGER NOT NULL DEFAULT 0 CHECK (quarantined IN (0,1))`, `quarantine_reason TEXT`.
- `headlines`: `id INTEGER PRIMARY KEY` (rowid alias — **load-bearing**, it is the FTS5 `content_rowid`), `file_id` FK → `files(id)` `ON DELETE CASCADE`, `parent_id` FK → `headlines(id)` `ON DELETE CASCADE` (NULL at top level), `kind`, `level`, `position`, `byte_start`, `byte_end`, `todo_keyword`, `todo_done`, `title`, `body`, `scheduled_date`, `scheduled_time`, `deadline_date`, `deadline_time`, `closed_date`, `closed_time`.
- `tags`: `headline_id` FK CASCADE, `tag TEXT NOT NULL`, `position INTEGER NOT NULL`, `PRIMARY KEY (headline_id, position)`.
- `properties`: `headline_id` FK CASCADE, `key TEXT NOT NULL`, `value TEXT NOT NULL`, `PRIMARY KEY (headline_id, key)` — matches the parser's documented last-wins duplicate-key collapse (`semantic/headline.rs:67-73`).
- `clock_entries`: `id INTEGER PRIMARY KEY`, `headline_id` FK CASCADE, `start_at TEXT NOT NULL`, `end_at TEXT` (**NULL = running clock** — Story 7.7 depends on this being nullable), `duration_seconds INTEGER`.
- `links`: `id INTEGER PRIMARY KEY`, `file_id` FK CASCADE **NOT NULL**, `headline_id` FK CASCADE **NULLABLE** (preamble links have no headline — Dev Note 2.4), `kind TEXT NOT NULL CHECK (kind IN ('id','file','url','wiki','plain'))` (the five `LinkKind` variants), `target TEXT NOT NULL`, `description TEXT`.
- `vault_meta`: `key TEXT PRIMARY KEY`, `value TEXT NOT NULL`, `updated_at TEXT NOT NULL` — the key/value bag for vault-scoped index state (vault root, last full rebuild, tokenizer used). **Not** per-file state; see Dev Note 5 for the LD-41 wording deviation.
- `_schema_version`: `version INTEGER PRIMARY KEY`, `description TEXT NOT NULL`, `applied_at TEXT NOT NULL` — declared here, **populated by Story 3.4's migration**. `PRAGMA user_version` stays the machine authority (LD-12); this table is the human-readable audit trail (Dev Note 6).
- Every date/datetime column is `TEXT` in ISO-8601 (`YYYY-MM-DD`, `HH:MM`, `YYYY-MM-DDTHH:MM:SS`) — Dev Note 3 explains why, and why it must not be epoch integers.

### AC3 — FTS5 external-content tables + the LD-4 tokenizer.

- `fts_headlines` is `CREATE VIRTUAL TABLE fts_headlines USING fts5(title, content='headlines', content_rowid='id', tokenize='porter unicode61 remove_diacritics 2')`.
- `fts_content` is the same shape over the `body` column: `fts5(body, content='headlines', content_rowid='id', tokenize='porter unicode61 remove_diacritics 2')`.
- **No triggers.** LD-11 mandates application-managed sync; a `CREATE TRIGGER` anywhere in `schema.sql` fails this AC. A comment in `schema.sql` states that the sync obligation (INSERT/DELETE into the FTS tables alongside every `headlines` write, including the `'delete'` command rows external-content tables require) belongs to the Story 3.6 sync engine.
- FTS5 column names **must** match columns that exist on `headlines` (`title`, `body`) — external-content FTS5 resolves them by name against the content table. A rename on either side silently breaks `snippet()`/`highlight()` at query time, not at DDL time.
- A test proves the tokenizer is live and correctly chained: insert a headline row + its FTS rows, then assert (a) a diacritics-folded query matches (`remove_diacritics 2`), and (b) a stemmed query matches (`porter` — e.g. `running` matches indexed `run`). Asserting the DDL string is not sufficient; a mis-ordered `tokenize` argument list is accepted by some forms and silently degrades.

### AC4 — Indices exist and are named per convention.

- `idx_files_path` — **UNIQUE** on `files(path)`. This is LD-11's `(file_path)` index; declaring it explicitly (rather than relying on an inline `UNIQUE` constraint's implicit auto-index) keeps it greppable, named per convention, and assertable via `PRAGMA index_list`.
- `idx_headlines_file_id`, `idx_headlines_parent_id` — subtree + per-file traversal.
- `idx_headlines_scheduled_date`, `idx_headlines_deadline_date` — LD-11's agenda range-scan indices. Plain (non-partial) indices; partial variants (`WHERE … IS NOT NULL`) are a Story 7.1 optimization to make against real query plans, not a guess to bake in now.
- `idx_tags_tag_headline_id` on `tags(tag, headline_id)` — LD-11's `(tag, headline_id)` composite, column order exactly as written (tag-first is what a tag filter scans).
- `idx_properties_headline_id`, `idx_clock_entries_headline_id`, `idx_links_headline_id`, `idx_links_file_id` — LD-11's `(headline_id)` family.
- `idx_links_target` — backlink traversal by target (FR-13); the `links` table is useless for backlinks without it.
- **Disclosed superset:** LD-11 names five index targets (`(file_path)`, `(headline_id)`, `(scheduled_date)`, `(deadline_date)`, `(tag, headline_id)`). `idx_headlines_file_id`, `idx_headlines_parent_id`, `idx_links_file_id` and `idx_links_target` go beyond that list — they are the FK columns the CASCADE deletes and the FR-13 backlink traversal scan, and an unindexed FK turns every `DELETE FROM files` into a full scan of `headlines`/`links`. Record the superset in Completion Notes.
- A test enumerates `PRAGMA index_list` per table (or queries `sqlite_master WHERE type='index' AND name LIKE 'idx_%'`) and asserts the exact expected set — so a future dropped index fails CI instead of silently degrading agenda latency.

### AC5 — `src/connection.rs` applies the LD-4 locked PRAGMAs, verifiably.

- `pub fn open(path: &Path) -> Result<Connection, IndexError>` opens (creating if absent) and applies the PRAGMAs before returning.
- The locked set from LD-4 / LD-14, applied in this order: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`.
- **Plus `foreign_keys=ON`** — a disclosed addition beyond the LD-4 list, and correctness-critical: it is a per-connection, non-persistent setting, and every `ON DELETE CASCADE` in AC2 is a silent no-op without it. Document the rationale in the function doc comment and record it in Completion Notes as a deliberate superset of LD-4.
- PRAGMAs are applied with `execute_batch` (or `pragma_update`), **never** `Connection::execute` — `journal_mode` and `mmap_size` return a row, and `execute` errors with `ExecuteReturnedResults` on statements that do (confirmed against rusqlite docs, Dev Note 4).
- `open()` verifies the outcome rather than assuming it: after applying, read each value back and return `IndexError::Pragma { name, expected, actual }` on mismatch. `journal_mode` in particular can silently refuse to become WAL (Dev Note 4).
- `src/error.rs` defines `#[non_exhaustive] pub enum IndexError` deriving `thiserror::Error`, mirroring the `VaultError` precedent in `crates/orgsidian-vault/src/error.rs`. Variants at minimum: an `Sqlite` wrapper (`#[from] rusqlite::Error`) and the `Pragma` mismatch above. No `OrgError` mapping — that is `orgsidian-core`'s job when the crate is first consumed.
- `lib.rs` re-exports the surface (`pub mod connection; pub mod error;` + `pub use connection::open; pub use error::IndexError;`) and its crate doc goes from "Structural placeholder — implementation lands in Story 3.x" to a present-tense description of what shipped (the placeholder sentence is now false — Story 3.2's review flagged exactly this pattern).

### AC6 — Tests run against a real on-disk database and assert real behavior.

- Tests live in `crates/orgsidian-index/tests/schema.rs` (integration; the DDL needs a temp-file fixture, matching the `crates/orgsidian-vault/tests/atomic.rs` precedent). `tempfile` as a dev-dependency, per-crate, exactly as `orgsidian-vault/Cargo.toml` does it.
- **Never `:memory:` for the PRAGMA tests.** An in-memory database cannot enter WAL — `journal_mode` reports `memory` and the assertion is meaningless. Use `tempfile::TempDir` + a real `.db` path. (An in-memory DB is fine for pure-DDL shape assertions if you want the speed, but say so in a comment.)
- Coverage required:
  - Fresh DB + `execute_batch(SCHEMA_SQL)` succeeds; re-running it on the same DB fails (proves the DDL is not accidentally `IF NOT EXISTS`-idempotent, which would mask migration bugs in 3.4) — or, if you deliberately choose `IF NOT EXISTS`, assert idempotency instead and record the choice in Completion Notes.
  - The exact table set (query `sqlite_master WHERE type='table'`), asserting both presence of the 8 + absence of anything unexpected. External-content FTS5 creates exactly four shadow tables per virtual table — `<name>_config`, `<name>_data`, `<name>_docsize`, `<name>_idx` (no `_content` shadow: that is the point of external content) — verified against SQLite 3.51.0. Enumerate them explicitly rather than asserting a bare count.
  - The exact `idx_%` index set (AC4).
  - Zero triggers: `SELECT count(*) FROM sqlite_master WHERE type='trigger'` is `0` (AC3).
  - Each locked PRAGMA reads back its expected value on a connection from `open()`. Verified read-back values (SQLite 3.51.0): `journal_mode`→`'wal'` (lowercase string), `synchronous`→`1`, `temp_store`→`2`, `cache_size`→`-64000`, `wal_autocheckpoint`→`4000`, `foreign_keys`→`1`. For `mmap_size` assert `> 0` **and** explain in a comment that the build can clamp it below the requested 268435456. Note that `mmap_size` and `wal_autocheckpoint` have no `pragma_*` table-valued function — read them with the `PRAGMA <name>` statement form (which is what `pragma_query_value` emits), not `SELECT * FROM pragma_mmap_size`.
  - FK cascade actually cascades: insert file → headline → tag/property/clock/link, delete the file, assert all descendants are gone. This is the test that fails if `foreign_keys=ON` is ever dropped.
  - `links` accepts a row with `headline_id IS NULL` (preamble link) and rejects an unknown `kind` via the CHECK constraint.
  - FTS5 round-trip + tokenizer behavior (AC3), including one `MATCH` against `fts_content` and one against `fts_headlines`.
- No `unwrap`/`expect`/`panic!` in committed non-test code (Story 2.8 discipline). Tests may `.unwrap()`/`.expect()` freely.

### AC7 — Dependency delta is exactly `rusqlite` (+ `thiserror`, already in the workspace), and every gate stays green.

- `[workspace.dependencies]` gains `rusqlite = { version = "0.40", features = ["bundled"] }` with the established story-attributed comment block (see the neighbouring entries in the root `Cargo.toml` for the house style: story number, LD trace, why this version, license). **0.40.1 is the current stable line** (verified 2026-08-01) — matches `[[feedback_version_policy]]` latest-stable.
- `crates/orgsidian-index/Cargo.toml` consumes `rusqlite = { workspace = true }` + `thiserror = { workspace = true }`, dev-dep `tempfile = "3"`. Its placeholder comment ("Real deps … added in Story 3.x") is replaced. **No** `orgsidian-parser`, **no** `chrono` (the query API adds `rusqlite/chrono` when it first maps a `NaiveDate`; nothing in this story does), **no** `deadpool-sqlite`, **no** `rusqlite_migration`.
- The `bundled` feature is mandatory, not optional: it compiles the SQLite amalgamation with `-DSQLITE_ENABLE_FTS5` (verified in `libsqlite3-sys/build.rs`), which the LD-4 FTS5 requirement depends on. A system SQLite is not guaranteed to have FTS5, is absent entirely on the Windows CI cell, and would give the four LD-32 matrix targets four different SQLite versions. `cc` is already a workspace dependency (Story 2.1), so the C-toolchain requirement is already satisfied on every cell.
- `Cargo.lock` is committed with the new crates (CI runs every cargo invocation `--locked`; a stale lock fails the build).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked`, `cargo deny --locked check all`, `cargo audit` all green.
- **If `cargo deny check bans` reports a duplicate major version** introduced by the `rusqlite` tree: add a `[bans].skip` entry with a `reason = "Story 3.3: …"` **and** the matching row in `docs/security/advisory-exceptions.md` — CI's `scripts/check-allowlist-sync.mjs` step fails on ledger drift. `rusqlite` must **never** appear in `skip`/`skip-tree` itself (deny.toml:127-131 — it is an LD-37 canonical-version invariant). If no duplicate fires, `deny.toml` stays untouched.
- Commitlint-conformant conventional commit; plain message, no AI-credit trailers.

## Tasks / Subtasks

- [x] Task 1 — Dependencies (AC7)
  - [x] Root `Cargo.toml`: add `rusqlite` to `[workspace.dependencies]` with the story-attributed comment block
  - [x] `crates/orgsidian-index/Cargo.toml`: `rusqlite` + `thiserror` deps, `tempfile` dev-dep, replace the placeholder comment
  - [x] `cargo build --workspace --locked` (first build compiles the SQLite amalgamation — expect ~30-60s once, then cached); commit `Cargo.lock`
- [x] Task 2 — DDL (AC1, AC2, AC3, AC4)
  - [x] `crates/orgsidian-index/sql/schema.sql`: header comment + 8 tables + FK/CHECK constraints
  - [x] FTS5 virtual tables with `content=`/`content_rowid=`/`tokenize=`; no triggers; sync-obligation comment
  - [x] Named indices per AC4
  - [x] `pub const SCHEMA_SQL` in `lib.rs`
- [x] Task 3 — Connection init (AC5)
  - [x] `src/error.rs`: `#[non_exhaustive] IndexError` (thiserror), `Sqlite` + `Pragma` variants
  - [x] `src/connection.rs`: `open()` — `execute_batch` PRAGMAs, then read-back verification
  - [x] `lib.rs`: module decls + re-exports + crate-doc rewrite to present tense
- [x] Task 4 — Tests (AC6)
  - [x] `tests/schema.rs`: temp-file DB fixture; table set, index set, zero triggers
  - [x] PRAGMA read-back assertions (all seven, with the `mmap_size` caveat comment)
  - [x] FK cascade delete; nullable `headline_id` on `links`; CHECK rejection on bad `kind`
  - [x] FTS5 round-trip + diacritics + porter-stemming assertions
- [x] Task 5 — Gates + hygiene (AC7)
  - [x] fmt / clippy / build / test / deny / audit
  - [x] Verify sentinels untouched: `git diff main...HEAD --name-only` shows only the intended files (branch-scoped form — `git diff HEAD` is non-probative for already-committed changes, per Story 3.2's review)
  - [x] Add deferred-work entries for the items in Dev Note 8

### Review Findings

Code review 2026-08-02 (bmad-code-review, 3 layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). Every finding below was reproduced empirically against SQLite 3.51.0 using the shipped `sql/schema.sql`, not accepted on a reviewer's assertion.

**Decisions — all four resolved 2026-08-02 and applied**

1. **FTS5 + cascade** → `ON DELETE CASCADE` removed from `headlines.file_id` (option a). Every other cascade stays; nothing else in the schema is FTS-indexed. `DELETE FROM files` is now rejected while headlines remain, forcing the sync engine through the delete path that owns the FTS obligation. The required order is documented on the FTS block in `schema.sql` as executable SQL.
2. **Atomicity** → `apply_schema(&mut Connection)` added (option b), wrapping the batch in `rusqlite::Transaction`. `SCHEMA_SQL` stays free of `BEGIN`/`COMMIT` so Story 3.4 can include it inside `rusqlite_migration`'s own transaction; a new test asserts that property behaviorally.
3. **`parent_id` cross-file** → invariant written into `schema.sql` with the reason the composite FK was rejected (a wholly redundant `UNIQUE (file_id, id)` updated on every headline insert); the enforcing test belongs to Story 3.6.
4. **Defensiveness** → CHECKs added only for states the parser cannot emit (self-parent, inverted span, half-populated TODO pair, quarantined-without-reason). Content-shaped values a real `.org` file can produce stay representable, and the rationale block above `CREATE TABLE headlines` now states the line explicitly instead of leaving it implicit in the `level` comment.

**Decisions required (blocked approval)**

- [x] [Review][Decision] `ON DELETE CASCADE` on `files` makes both FTS5 external-content tables raise `SQLITE_CORRUPT` — Found independently by all three layers; reproduced verbatim. The schema's own "NO TRIGGERS, BY MANDATE" comment (`sql/schema.sql:218-223`) assigns the sync engine the obligation to write the `'delete'` command rows "before an update or delete", but `headlines.file_id … ON DELETE CASCADE` (`sql/schema.sql:93`) means a plain `DELETE FROM files` removes the headline rows *inside SQLite*, with no application hook and no recoverable text. Measured on a database built from the shipped DDL: after `DELETE FROM files WHERE id=1`, `SELECT rowid FROM fts_content WHERE fts_content MATCH 'salary'` still returns the stale hit `1` with no error; `SELECT rowid, body …` → `fts5: missing row 1 from content table 'main'.'headlines' (11)`; `snippet(...)` → `database disk image is malformed (11)`. A routine single-file delete therefore surfaces a corruption error for the whole search index. `deleting_a_file_cascades_to_every_descendant` (`tests/schema.rs`) performs exactly this delete and asserts nothing about the FTS tables, so CI is green. Resolutions to choose between: (a) drop `ON DELETE CASCADE` from `headlines.file_id` so the unsafe path fails loudly with an FK violation and 3.6 must delete headlines FTS-first (deviates from AC2); (b) keep the cascade and make the schema comment + a contract test state that `DELETE FROM files` is not the sanctioned delete path; (c) rely on LD-13 drop-and-rebuild and document the corruption as an accepted recovery trigger.
- [x] [Review][Decision] `SCHEMA_SQL` is not applied atomically, and the obvious fix would break the Story 3.4 seam — `sql/schema.sql` contains no `BEGIN;`/`COMMIT;` and `execute_batch` runs each DDL statement in its own implicit transaction, so a mid-batch failure (FTS5 module absent, ENOSPC, SIGKILL) leaves a permanently half-built database. Reproduced: against a database already holding only `vault_meta`, the batch committed `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `_schema_version` and both FTS5 table sets before failing. Two committed doc claims are false as a result — `src/lib.rs:28-29` ("applying it twice fails, so a half-initialized database cannot pass for a healthy one") and `sql/schema.sql:14-16` ("must fail loudly rather than half-succeed"): the half-succeeded database produces the byte-identical `table files already exists` error a healthy one does, which is also what `re_applying_the_schema_fails_loudly` asserts on. Verified that the full DDL — including both `CREATE VIRTUAL TABLE … fts5`— applies cleanly inside `BEGIN; … COMMIT;`, so a wrap works mechanically. But `rusqlite_migration` already wraps each migration in its own transaction, so a `BEGIN` embedded in the DDL text would fail with "cannot start a transaction within a transaction" the moment Story 3.4 includes this file — actively sabotaging the seam Dev Note 7 exists to protect. Resolutions: (a) wrap in the SQL file and make 3.4's resolution (a) mandatory (`SCHEMA_SQL` re-points at the migration, which owns the transaction); (b) add an `apply_schema(&mut Connection)` helper in 3.3 that uses `Connection::transaction()` — arguably 3.4 turf; (c) leave it, fix the two false doc claims, and record the half-built-database state as a rebuild trigger.
- [x] [Review][Decision] `headlines.parent_id` has no same-file constraint — deleting one file deletes another file's headlines — Reproduced: headline `id=20, file_id=2, parent_id=10` where headline 10 belongs to file 1; `DELETE FROM files WHERE id=1` removed row 20 as well. `sql/schema.sql:95` declares `parent_id INTEGER REFERENCES headlines (id) ON DELETE CASCADE` with no `file_id` pairing, and `links.headline_id` (`sql/schema.sql:166`) has the same shape, so a `links` row's `file_id` and `headline_id → headlines.file_id` can disagree, silently misattributing FR-13 backlinks. The cross-file `parent_id` is itself a Story 3.6 bug, but the schema currently converts that bug into cross-file data loss rather than rejecting it. Resolutions: (a) `UNIQUE (file_id, id)` on `headlines` plus a composite `FOREIGN KEY (file_id, parent_id) REFERENCES headlines (file_id, id) ON DELETE CASCADE` (deviates from AC2's column list); (b) leave it and record "parent_id is same-file" as a written 3.6 invariant with a test in 3.6.
- [x] [Review][Decision] How defensive should schema v1 be? — a cluster of integrity gaps sharing one trade-off — All reproduced against the shipped DDL: self-parent (`id = parent_id`) accepted, and a `parent_id` cycle would make any `WITH RECURSIVE` subtree traversal — the stated purpose of `idx_headlines_parent_id` — non-terminating; `todo_keyword='DONE'` with `todo_done` NULL accepted despite `sql/schema.sql:105`'s "Both NULL together" comment (a `CHECK` over a NULL column evaluates to NULL and passes), so `WHERE todo_done = 0` and `WHERE todo_done = 1` both silently drop the row; `level = -5` and `byte_start=50, byte_end=10` accepted (a consumer slicing `source[byte_start..byte_end]` panics); a `clock_entries` row that is simultaneously running (`end_at` NULL) and finished (`duration_seconds = -99`) accepted, poisoning any `SUM(duration_seconds)`; `quarantined = 1` with `quarantine_reason` NULL accepted, so the LD-41 malformed-file row has nothing to display; duplicate `(headline_id, tag)` accepted because the PK keys the *slot* `(headline_id, position)`, so the very query `idx_tags_tag_headline_id` exists to serve returns the headline twice; empty strings accepted in `files.path`, `tags.tag` and `links.target`. Each has a one-line `CHECK`/`UNIQUE` fix, but they are in direct tension with the permissive philosophy `sql/schema.sql:86-88` already states in writing ("`level` has no CHECK constraint on purpose … a guard would reject documents the parser accepts") — and a constraint that rejects real parser output turns a bad `.org` file into a failed index rather than a quarantined row. Decide the policy once and apply it consistently, rather than per-column.

**Patches (unambiguous, no decision required)**

- [x] [Review][Patch] `vault_meta.key TEXT PRIMARY KEY` is nullable and admits multiple NULL keys [`crates/orgsidian-index/sql/schema.sql:179`] — SQLite's documented legacy quirk: only `INTEGER PRIMARY KEY` implies NOT NULL. Reproduced: two `INSERT … VALUES (NULL,…)` both succeed, `count(*)` → 2. `tags` and `properties` avoid this with explicit `NOT NULL`; `vault_meta` is the one that does not. Fix: `key TEXT NOT NULL PRIMARY KEY`.
- [x] [Review][Patch] `MMAP_SIZE_BYTES` is decorative — the applied value is a hardcoded literal and drift is unverifiable [`crates/orgsidian-index/src/connection.rs:16,34,138`] — `LOCKED_PRAGMAS` hardcodes `268435456` as text; the const's only use is inside an error message, and because `verify_mmap_size` checks only `> 0`, changing the const silently changes nothing. `CACHE_SIZE_KIB` and `WAL_AUTOCHECKPOINT_PAGES` have the same const↔literal duplication but at least fail loudly on drift. Fix: build `LOCKED_PRAGMAS` from the consts.
- [x] [Review][Patch] `cache_size = -64000` is documented as 64 MiB; it is 62.5 MiB [`crates/orgsidian-index/src/connection.rs:19,50`] — 64 MiB is `-65536`. The LD-4 value is correct; the doc arithmetic is wrong in two places.
- [x] [Review][Patch] `fts_headlines_folds_diacritics` cannot distinguish `remove_diacritics 2` from `1` [`crates/orgsidian-index/tests/schema.rs`] — Measured: `Réunion au café` matches `reunion`/`cafe` under both `remove_diacritics 1` and `2`, so dropping the `2` leaves the test green. The story's own mutation check used `remove_diacritics 0`, which both levels catch. Fix: index a multi-diacritic codepoint — verified discriminator: `Nguyễn Việt` matched against `nguyen` returns 0 under rd=1 and 1 under rd=2.
- [x] [Review][Patch] Test comment claims a mis-ordered `tokenize` list degrades silently; it fails loudly [`crates/orgsidian-index/tests/schema.rs`] — Verified: both `'unicode61 remove_diacritics 2 porter'` and `'porter remove_diacritics 2 unicode61'` fail at `CREATE VIRTUAL TABLE` with `error in tokenizer constructor`. The story's own Latest Technical Information section says the same. Fix: correct the comment.
- [x] [Review][Patch] `deleting_a_file_cascades_to_every_descendant` never exercises the `parent_id` cascade [`crates/orgsidian-index/tests/schema.rs`] — The child headline is inserted with the same `file_id` as its parent, so its deletion is fully explained by the `files` FK. Removing `ON DELETE CASCADE` from `sql/schema.sql:95` leaves the assertion passing. Fix: delete a parent *headline* and assert the child is gone.
- [x] [Review][Patch] `named_index_set_is_exactly_the_ld_11_set` filters on the `idx_` prefix, so an off-convention index escapes [`crates/orgsidian-index/tests/schema.rs`] — An added `files_path_idx` or `tmp_debug_ix` passes unnoticed — precisely the case worth catching, given the sibling table-set test asserts the full set. Fix: assert over all non-`sqlite_autoindex_` indices.
- [x] [Review][Patch] The `tempfile` comment claims a mirroring that does not hold [`crates/orgsidian-index/Cargo.toml`] — Comment says "Dev-only, per-crate — mirrors orgsidian-vault/Cargo.toml", but `orgsidian-vault` uses the bare `tempfile = "3"` form while this crate uses `{ workspace = true }`, and "per-crate" misdescribes a workspace-inherited pin. The workspace form is the right call (Completion Note variance 5); the comment should say so.
- [x] [Review][Patch] `#[non_exhaustive]` is on the enum but not on the `Pragma` variant [`crates/orgsidian-index/src/error.rs:30`] — The comment at `:10-11` states new variants must not break downstream exhaustive matches, but adding a field to `Pragma` breaks every `IndexError::Pragma { name, expected, actual }` pattern.
- [x] [Review][Patch] The date/time split rationale is factually incorrect [`crates/orgsidian-index/sql/schema.sql:31-33`] — Claims `'2026-08-02' < '2026-08-02T09:00'` "would sort all-day entries inconsistently against timed ones". Verified: that comparison is true — all-day sorts before same-day timed entries, which is both consistent and conventional org-agenda ordering. The split is still the right call (clean date-only `BETWEEN` range scans); the stated reason is not the reason.
- [x] [Review][Patch] Two doc claims about half-initialized databases are false [`crates/orgsidian-index/src/lib.rs:28-29`, `crates/orgsidian-index/sql/schema.sql:14-16`] — See the atomicity decision above. These need correcting whichever resolution is chosen.
- [x] [Review][Patch] No test covers any `open()` failure path; `IndexError::Pragma` is never constructed [`crates/orgsidian-index/tests/schema.rs`] — The story's headline feature is read-back verification, and the entire branch `src/connection.rs:90-141` is covered only on the success side. `open_applies_every_locked_pragma` re-reads the same PRAGMAs the production code already read — it verifies *application*, not *verification*; replacing `verify_locked_pragmas`'s body with `Ok(())` leaves the suite green. Fix: add negative-path tests (parent directory absent, path is a directory, path is a non-database file) and exercise the `Pragma` variant's formatting.

**Deferred (recorded in deferred-work.md, not actioned here)**

- [x] [Review][Defer] Rowid reuse on `headlines.id` + manually-synced external-content FTS returns live unrelated text for a deleted headline's terms [`crates/orgsidian-index/sql/schema.sql:92`] — deferred, Story 3.6 sync-engine turf
- [x] [Review][Defer] `clock_entries.start_at`/`end_at` are combined datetimes while `headlines` splits date and time, and the parser can emit a date-only `CLOCK:` [`crates/orgsidian-index/sql/schema.sql:149-150`] — deferred, AC2 prescribes the shape; Epic 7 owner
- [x] [Review][Defer] `open()` sets no `application_id` and reads no `user_version`, so it converts any SQLite file it is pointed at to WAL and returns `Ok` [`crates/orgsidian-index/src/connection.rs:79-80`] — deferred, Story 3.4/3.6 turf
- [x] [Review][Defer] Agenda indices are single-column where the natural query is `ORDER BY <date>, <time>` [`crates/orgsidian-index/sql/schema.sql:263-264`] — deferred, AC4 names them this way; Story 7.1 optimization
- [x] [Review][Defer] `files.path TEXT` cannot represent a non-UTF-8 filename [`crates/orgsidian-index/sql/schema.sql:61`] — deferred, folds into the existing Story 3.6 path-identity row
- [x] [Review][Defer] `IndexError::Sqlite` discards the path, contradicting the module doc's "enough context to localize a failure" [`crates/orgsidian-index/src/error.rs:18`] — deferred, `#[from]` shape is AC5-mandated
- [x] [Review][Defer] No test covers FTS staleness on UPDATE or DELETE of a `headlines` row [`crates/orgsidian-index/tests/schema.rs`] — deferred, Story 3.6 owns the sync contract
- [x] [Review][Defer] `_schema_version.version INTEGER PRIMARY KEY` is a rowid alias, so an unbound version is silently invented [`crates/orgsidian-index/sql/schema.sql:197`] — deferred, Story 3.4 owns the migration runner
- [x] [Review][Defer] `idx_properties_headline_id` is fully redundant with `sqlite_autoindex_properties_1` [`crates/orgsidian-index/sql/schema.sql:271`] — deferred, AC4 mandates it
- [x] [Review][Defer] No test can catch removal of `PRAGMA foreign_keys = ON` under the `bundled` feature [`crates/orgsidian-index/src/connection.rs:38`] — deferred, structural limitation worth documenting

## Dev Notes

### 1. What you are walking into

`crates/orgsidian-index/` is an empty shell: `src/lib.rs` holds three doc lines and `Cargo.toml` has an empty `[dependencies]` with a "lands in Story 3.x" comment. There is **no** `tests/` directory, **no** `anchor.rs` (Story 1.9 anchored parser/vault/watcher only — the index crate had no code to anchor). You are creating the crate's entire real surface. Follow the shape `orgsidian-vault` settled into across Stories 3.1/3.2: `lib.rs` = declarations + re-exports only (architecture.md:739), one concern per module, `error.rs` owns the crate error enum, module doc headers naming LD/FR traces.

### 2. Column-by-column mapping to the Story 2.3 semantic types

The schema must be *populatable* from what `orgsidian_parser::semantic::analyze()` actually returns. You do not import the parser (Scope Fence), but every column below has a source field — if you invent a column with no source, or omit a field that has nowhere to go, Story 3.6 pays for it.

**2.1 `headlines` ← `semantic::Headline` (`crates/orgsidian-parser/src/semantic/headline.rs:46-109`)**

| Column | Source | Note |
|---|---|---|
| `level` | `Headline.level: u8` | Parser emits degenerate `0` inside `ERROR` regions and saturates at `255`; the column is a plain `INTEGER` — **do not** add a `CHECK (level BETWEEN 1 AND 6)`, it would reject documents the parser accepts |
| `todo_keyword` / `todo_done` | `Headline.todo_state: Option<TodoState>` → `.keyword` / `.done` | Both NULL together when there is no TODO state; `todo_done` is `0`/`1` |
| `title` | `Headline.title` | Stars, TODO keyword and trailing tags already stripped by the parser |
| `body` | `Headline.raw` | **Own region only** (headline line + planning + drawers + body, excluding child sections) — exactly the granularity FTS5 search results need to jump to |
| `scheduled_date` / `scheduled_time` | `Headline.scheduled: Option<Timestamp>` → `.date` / `.time` | Split into two columns so agenda range-scans hit a date-only index; `Timestamp.raw` is not stored (the parser round-trips from source spans, never from the index) |
| `deadline_*`, `closed_*` | `.deadline`, `.closed` | Same split |
| `byte_start` / `byte_end` | `Headline.span: Range<usize>` | Whole section, headline line through last child |
| `parent_id` / `position` | `Headline.children` recursion | The tree is flattened; `position` is sibling order in document order |
| `kind` | — | `'headline'` or `'preamble'`; see 2.3 |

Timestamp fields deliberately **not** modelled as columns in v1: `Timestamp.active`, `.end_date`/`.end_time` (ranged timestamps), `.repeater`, `.delay`. Recurring-task expansion is Epic 7 turf and will want its own design pass; store what agenda-by-date needs now and let a forward-only migration add the rest (LD-12 makes that cheap, and LD-13 makes a rebuild free). Say this in a comment in `schema.sql` so the omission reads as a decision, not an oversight.

**2.2 `tags` ← `Headline.tags: Vec<Tag>`** — `Tag.name` (colons already stripped), `position` preserves document order.

**2.3 `kind` and the preamble problem.** `semantic::Document` has a `preamble: Option<Preamble>` carrying `text`, `links` and `directives` — the content before the first headline (`#+TITLE:`, intro prose). It is not a `Headline`, so a naive `fts_content` over `headlines` leaves it unsearchable, which is a silent FR-12 gap. Resolution: `headlines.kind TEXT NOT NULL DEFAULT 'headline' CHECK (kind IN ('headline','preamble'))`. A file's preamble is stored as one `headlines` row with `kind='preamble'`, `level=0`, `title=''`, `body=Preamble.text`, `parent_id=NULL`. Do **not** signal it by overloading `level=0` alone — the parser already uses `0` as a degenerate sentinel for malformed input, and conflating the two makes every `WHERE level = 0` query ambiguous. Populating these rows is Story 3.6's job; your job is that the schema has a place for them. `Preamble.directives` (the `#+KEYWORD: value` pairs, including `#+TODO:` sequences) has **no** table in the LD-11 table set — leave it out and file it in deferred-work (Dev Note 8), do not invent a ninth table.

**2.4 `links` ← `Link` (`semantic/link.rs:29-59`)** — `LinkKind` has exactly five variants (`Id`, `File`, `Url`, `Wiki`, `Plain`); the CHECK constraint mirrors them lowercased. `headline_id` **must** be nullable because `Preamble.links` exists and, under the 2.3 convention, preamble links could attach to the synthetic row — but a file-level link with no headline is still representable and the nullable column costs nothing. `file_id` is `NOT NULL` in both cases. The parser's link scan is textual and deliberately over-reports links inside verbatim blocks (`docs/parser/KNOWN_DIVERGENCES.md` entry 1) — the schema stores what it is given; filtering is not a DDL concern.

**2.5 `clock_entries` ← `ClockEntry` (`semantic/drawer.rs:72-84`)** — `start: Timestamp` → `start_at`, `end: Option<Timestamp>` → `end_at` (NULL = clock still running; Story 7.7's "prior session running clock" prompt reads exactly this), `duration: Option<TimeDelta>` → `duration_seconds INTEGER` (the parser already serializes it as whole seconds — `deferred-work.md:178`).

**2.6 `drawers`** — `Headline.drawers: Vec<Drawer>` has no table in LD-11's set and gets none. `:PROPERTIES:` is normalized into `properties`, `:LOGBOOK:` into `clock_entries`; other drawer kinds live in `headlines.body` and are reachable by FTS. Note it in `schema.sql`.

### 3. Dates as ISO-8601 TEXT, not epoch integers

Org timestamps are wall-clock and timezone-less (`NaiveDate` / `NaiveTime` — `semantic/timestamp.rs:112-135`). Converting them to epoch integers requires inventing a timezone, which silently shifts a `SCHEDULED: <2026-08-02 Sun>` across a day boundary for anyone east or west of the assumed zone — a correctness bug that surfaces as "the agenda shows tomorrow's task today" and is unfixable after the fact without a rebuild. ISO-8601 `TEXT` avoids the invention entirely, sorts lexicographically in exactly chronological order (so `BETWEEN` range scans on `idx_headlines_scheduled_date` work), is greppable in `sqlite3` during debugging, and maps to `chrono::NaiveDate` through rusqlite's `chrono` feature whenever the query API wants typed access. Storage cost (10 bytes vs 8) is irrelevant at notes scale. **Split date and time into separate columns** — an all-day `SCHEDULED` has no time, and `'2026-08-02' < '2026-08-02T09:00'` would make a single-column encoding sort all-day entries inconsistently against timed ones.

### 4. rusqlite mechanics that will bite you (verified against current docs)

- **`execute` vs `execute_batch`.** `PRAGMA journal_mode = WAL` and `PRAGMA mmap_size = …` *return a row*. `Connection::execute` rejects any statement that returns rows (`ExecuteReturnedResults`). Use `conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; …")` for the whole set, or `pragma_update` per item (which is itself implemented on top of `execute_batch`). This is the single most common way this story fails at runtime while compiling cleanly.
- **WAL can silently not happen.** `journal_mode=WAL` is a *persistent* database property (it survives in the file), but the switch can fail — most notably on `:memory:` databases (which report `memory`) and on some network filesystems. That is why AC5 requires read-back verification rather than fire-and-forget, and why AC6 forbids `:memory:` for the PRAGMA tests.
- **Read-back values are not the strings you wrote.** `synchronous` reads back `1` (not `NORMAL`), `temp_store` reads back `2` (not `MEMORY`), `journal_mode` reads back lowercase `wal`. Write the assertions against those.
- **`mmap_size` can be clamped.** The build's `SQLITE_MAX_MMAP_SIZE` caps the requested 268435456; a clamped value is not an error. Assert `> 0`, not equality, and comment why.
- **`foreign_keys` is per-connection and non-persistent.** The bundled amalgamation is compiled with `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`, but relying on a build flag for a correctness property is exactly the kind of implicit dependency that breaks when someone flips `bundled` off. Set it explicitly on every connection.
- **`bundled` and the CI matrix.** `libsqlite3-sys`'s bundled build compiles `sqlite3.c` through the `cc` crate with `-DSQLITE_ENABLE_FTS5`, `-DSQLITE_ENABLE_JSON1`, `-DSQLITE_THREADSAFE=1` among others. First compile on a cold cache costs roughly half a minute per cell; `Swatinem/rust-cache@v2` (already wired in `pr.yml`) absorbs it afterwards.
- **Not needed here:** `busy_timeout`. It matters the moment there are concurrent connections, i.e. Story 3.5's reader pool + writer task; setting it now would be guessing at a value with no contention to calibrate against. Leave it, and put it in deferred-work pointing at 3.5 (Dev Note 8) so 3.5 does not inherit a `SQLITE_BUSY` surprise.

### 5. `quarantined` lives on `files`, not in `vault_meta` (disclosed deviation)

LD-41's first row (architecture.md:1200) says a malformed file is *"marked as `quarantined` in `vault_meta`"*. Taken literally that puts per-file state in a vault-scoped key/value bag, which means either one row per file with a synthesized key (`quarantined:/path/to/file.org`) — unindexable, unjoinable, and duplicating the `files` table's key — or a serialized list in a single value, which cannot participate in the `WHERE quarantined = 0` filter every sync query needs. Ship `files.quarantined` + `files.quarantine_reason`; `vault_meta` stays a genuine vault-scoped bag. Record the deviation in Completion Notes and as a deferred-work line noting that LD-41's wording wants an addendum. Do **not** edit `_bmad-output/planning-artifacts/architecture.md` — it is a process archive (architecture.md:1010), and no story has amended it in place.

### 6. `_schema_version` vs `PRAGMA user_version` — both, with distinct jobs

LD-11 lists a `_schema_version` table; LD-12 says versioning happens via `PRAGMA user_version`. They are not redundant if you assign them different jobs, and they *are* a contradiction if you do not. Assignment: `PRAGMA user_version` is the **machine authority** — it is what `rusqlite_migration` reads and writes in Story 3.4, and what LD-13's "user_version mismatch → full rebuild" check compares. `_schema_version` is the **human-readable audit trail**: one row per applied migration with its description and timestamp, for `orgsidian index stats` (Story 3.7) and for a developer opening the file in `sqlite3`. This story only *declares* the table — it inserts nothing. Story 3.4's migration owns the first row. State this split in a comment on the table so 3.4 does not "fix" the apparent duplication by deleting one of them.

### 7. The 3.3 → 3.4 seam (the one thing most likely to go wrong later)

Story 3.4's AC says the migration runner calls `M::up(include_str!("../migrations/0001_initial-schema.sql"))` and that *"`0001_initial-schema.sql` contains the Story 3.3 schema"*. Read naively, 3.4 copies your DDL into a second file and the repo now has two copies of the schema that drift the first time someone edits one. Do not pre-empt 3.4's decision, but leave it a clean seam: (a) `sql/schema.sql` is the only DDL text in this story, (b) `SCHEMA_SQL` is the only in-Rust handle to it, (c) `schema.sql`'s header states the forward-only rule explicitly, and (d) file a deferred-work line naming the fork risk and the two acceptable resolutions — 3.4 makes `migrations/0001_initial-schema.sql` the single source and `SCHEMA_SQL` re-points at it, or 3.4 keeps `sql/schema.sql` canonical and includes it. Whichever it picks, there must be exactly one copy.

### 8. Deferred-work entries to file (`_bmad-output/implementation-artifacts/deferred-work.md`)

Follow the file's existing row format (bold summary, `[path]`, severity, rationale, owner):

- **Schema-to-migration fork risk** [`crates/orgsidian-index/sql/schema.sql`] [MED] — Story 3.4 owner; per Dev Note 7.
- **`busy_timeout` unset** [`crates/orgsidian-index/src/connection.rs`] [MED] — Story 3.5 owner; concurrent readers + a writer task will hit `SQLITE_BUSY` without it.
- **LD-41 wording: `quarantined` modelled on `files`, not `vault_meta`** [architecture.md:1200] [LOW] — per Dev Note 5.
- **`Preamble.directives` (`#+KEYWORD: value`, incl. `#+TODO:` sequences) has no table** [`crates/orgsidian-index/sql/schema.sql`] [LOW] — Epic 7/8 owner; needed when per-file TODO configuration or `#+FILETAGS:` becomes queryable.
- **Ranged/repeating timestamp fields not modelled** (`Timestamp.active`, `end_date`, `end_time`, `repeater`, `delay`) [`crates/orgsidian-index/sql/schema.sql`] [MED] — Epic 7 (recurring-task agenda) owner; per Dev Note 2.1.
- **Index DB duplicates vault text** — external-content FTS5 requires the text to live in `headlines.title`/`headlines.body`, so the `.db` grows to roughly vault-text size plus the FTS index [`crates/orgsidian-index/sql/schema.sql`] [LOW] — acceptable at notes scale and the DB is derived + rebuildable (LD-13, LD-17); revisit only if a large-vault profile (LD-42, 50k files) shows it mattering.
- **Path-identity policy still open** — `files.path` storage form (absolute vs vault-relative, separator normalization, case-folding on macOS/Windows) is undecided; Story 3.2 deferred the parallel `DirtyBufferManager` question and `deferred-work.md:213` assigned the `.org` case-sensitivity question to "the index stories" [`crates/orgsidian-index/sql/schema.sql`] [MED] — Story 3.6 (Vault-open) owner: it designates the Vault root and therefore owns what a path *means*. This story stores whatever string it is handed under a UNIQUE constraint.

### 9. Code conventions (established — follow exactly)

- Module doc header naming LD/FR traces (Stories 1.17/1.18/3.1/3.2 precedent): LD-4, LD-11, LD-14, FR-17.
- `lib.rs` carries declarations and re-exports only; no logic (architecture.md:739).
- `error.rs` owns `IndexError`, `#[non_exhaustive]`, `thiserror::Error` derive — mirror `crates/orgsidian-vault/src/error.rs`.
- No `unwrap`/`expect`/`panic!` in committed non-test code. Story 3.2's review flagged an `.unwrap()` inside a `//!` doc example — if you write a doctest, use `?` or `expect` with a real message, and know it is a running doctest.
- Doc comments on public items encouraged (`clippy::pedantic` is only enforced on `orgsidian-plugin-api`).
- **Do not write forward-looking prose in rustdoc.** Story 3.2's review made this a standing rule: "Story 3.4 wires migrations" in a doc comment becomes a confident falsehood the moment 3.4 does it differently. Describe what shipped; phrase anything else as intent.
- `println!` is forbidden in committed code; use `tracing` if you need to log (the crate does not need it yet — no dependency required).

### Project Structure Notes

- New: `crates/orgsidian-index/sql/schema.sql`, `src/connection.rs`, `src/error.rs`, `tests/schema.rs`. Modified: `crates/orgsidian-index/src/lib.rs`, `crates/orgsidian-index/Cargo.toml`, root `Cargo.toml`, `Cargo.lock`.
- `sql/` is a new directory convention for this crate, mandated by LD-11 ("Schema lives in `crates/orgsidian-index/sql/schema.sql`"). It is not a Rust module; nothing else in the workspace has one.
- Fixtures: none. No entry in the root `fixtures/` directory (that is for cross-crate fixtures only, per CONTRIBUTING).
- Branch per repo convention: `story/3.3-sqlite-schema-pragmas` off `main`; conventional commits (commitlint gate); plain commit messages, no AI-credit trailers. GitHub issue **#27**; label `status:in-review` during code review.

### Testing Standards Summary

- Integration tests at `crates/<crate>/tests/<topic>.rs` (architecture.md:725); unit tests colocated `#[cfg(test)] mod tests` for anything pure. The DDL work is fixture-bound, so `tests/schema.rs` is the right home — the same call the vault made for `tests/atomic.rs`.
- Deterministic: no timing assertions, no wall-clock dependencies. A `TempDir` per test; no shared database file across tests.
- Assert *behavior*, not DDL text. `assert!(SCHEMA_SQL.contains("fts5"))` is placebo-green (Story 1.9's anti-placebo rule); opening a real database, inserting, and matching is not.
- The perf gate (`assert_no_perf_regression!`) does not apply — no baseline exists for this crate and this story introduces no measured surface. `docs/perf/targets.md` assigns FTS5 search budgets to Story 8.4, not here.

### Previous Story Intelligence (from Stories 3.1 + 3.2)

- **Crate shape**: 3.1 established `lib.rs` = thin re-exports, `error.rs` = `#[non_exhaustive]` thiserror enum, module doc headers with LD/FR traces. 3.2 extended it without touching either. Copy that shape into `orgsidian-index`.
- **Stale forward-looking doc prose is a review finding.** 3.2 was pulled up twice on it: `lib.rs` kept a promise-shaped sentence about work that had already shipped, and the new module's docs described Epic 5/9 integrations as if they existed. Your `lib.rs` currently says "Structural placeholder — implementation lands in Story 3.x". Rewrite it, do not append to it.
- **Sentinel evidence must be branch-scoped.** 3.2's Completion Notes cited `git diff --name-only HEAD`, which is empty for already-committed work and therefore proves nothing. Use `git diff main...HEAD --name-only`.
- **Disclose signature/scope variances in Completion Notes**, do not silently absorb them (3.1 + 3.2 both). Your candidates: `foreign_keys=ON` as a superset of the LD-4 locked set (AC5), `files.quarantined` vs LD-41's `vault_meta` wording (Dev Note 5), and the `IF NOT EXISTS` choice if you take it (AC6).
- **New dependency edges cost supply-chain work.** 3.1's churn came from exactly this: a new dep pulled a duplicate version, which needed a `deny.toml` skip plus a ledger row plus the cross-tool sync check. Budget for it (AC7) — `rusqlite` is the first new dep edge since Story 2.8.
- **LEAF rule** (1.18/2.8/3.1/3.2): `orgsidian-index` may only be wrapped by `orgsidian-core` (deny.toml:192-194). This story adds no consumer, so no `[workspace.dependencies]` entry for it and no `orgsidian-core` edit.

### Git Intelligence Summary

Working tree at story-creation: on `main`, clean, HEAD `37149a8` (merge of PR #150, Story 3.2). Recent Epic-3 flow: `story/3.N-<slug>` branch off `main` → implement → `bmad-code-review` → hardening commit → PR merged with `gh pr merge --admin` (branch protection requires 1 review, unsatisfiable solo) → sprint-status + GitHub label updated. Story 3.1 landed the vault atomic-write subsystem (`a31cddb..a8d5a13`); Story 3.2 landed `dirty_buffer.rs` + hardening (`5dc2b4d`, `14d2898`) touching exactly 5 files. Commit type convention: `feat(vault): …` scoped to the crate — use `feat(index): …` here; `chore:` for cross-branch merges (commitlint forbids `merge:`).

### Latest Technical Information

- **`rusqlite` 0.40.1** is the current stable line (crates.io, verified 2026-08-01). MIT. Pull it as `version = "0.40"` with `features = ["bundled"]`.
- **FTS5 availability**: `libsqlite3-sys`'s bundled build compiles the amalgamation with `-DSQLITE_ENABLE_FTS5` (plus `FTS3`, `JSON1`, `RTREE`, `STAT4`, `SQLITE_THREADSAFE=1`, `SQLITE_DEFAULT_FOREIGN_KEYS=1`). There is no separate `fts5` cargo feature to enable — `bundled` is the whole answer, and without it FTS5 depends on whatever the host SQLite was compiled with.
- **FTS5 tokenizer chaining**: `porter` is a wrapper tokenizer that takes the underlying tokenizer and its arguments as its own arguments. LD-4's "`unicode61 remove_diacritics 2` + `porter`" is therefore the single option string `tokenize='porter unicode61 remove_diacritics 2'` — porter first, then the tokenizer it wraps. **Verified end-to-end** against SQLite 3.51.0: with that exact string, `MATCH 'cafe'` finds indexed `Café` (diacritics folded) and `MATCH 'run'` finds indexed `runs`/`running` (stemmed), and `snippet()` works through the external-content table. The reversed string `'unicode61 remove_diacritics 2 porter'` fails loudly at `CREATE VIRTUAL TABLE` time with `error in tokenizer constructor` — so a wrong order cannot ship silently, but it will block you until you fix it.
- **`remove_diacritics 2`** requires SQLite ≥ 3.27; the bundled amalgamation is far newer, so it is safe — and it is the correct choice over `1`, which mishandles diacritics that are part of the base codepoint.
- **`rusqlite_migration` 2.6.0** is the current line — noted only so Story 3.4 does not plan against LD-12's "≥1.3". Do not add it here.
- Rust edition/toolchain: workspace `edition = "2021"`, `rust-toolchain.toml` pins stable; `rustfmt` + `clippy -D warnings` enforced in `pr.yml`.

### References

- Epic AC source: `_bmad-output/planning-artifacts/epics.md` §Epic 3 → Story 3.3 (lines 921-935); Epic 3 summary (line 366)
- LD-4 (rusqlite + locked PRAGMAs + FTS5 tokenizer): `architecture.md:66`; LD-11 (schema + indices + `sql/schema.sql`): `architecture.md:398`; LD-12 (migrations, forward-only): `architecture.md:400`; LD-13 (rebuild policy): `architecture.md:402`; LD-14 (connection management): `architecture.md:404`; LD-17 (fs allow-list / index outside Vault): `architecture.md:414`, `architecture.md:1192`
- SQLite naming conventions: `architecture.md:694-699`; crate organization: `architecture.md:738-742`; test placement: `architecture.md:723-729`; AI-agent rules + anti-patterns: `architecture.md:850-877`
- Database boundary (#5, no raw SQL outside the crate): `architecture.md:1036`; FR-17 traceability: `architecture.md:1060`; FR-12/FR-13 future consumers: `architecture.md:1055-1056`
- LD-41 failure catalog (quarantine row + index-corruption row): `architecture.md:1200`, `architecture.md:1206`; coverage matrix: `docs/failure-modes/coverage-matrix.md:13,17`
- Semantic types the schema must accept: `crates/orgsidian-parser/src/semantic/headline.rs:28-109`, `drawer.rs:24-84`, `link.rs:29-59`, `timestamp.rs:112-135`, `todo.rs:24-30`, `mod.rs:52-99`
- Crate conventions to mirror: `crates/orgsidian-vault/src/error.rs`, `crates/orgsidian-vault/src/lib.rs`, `crates/orgsidian-vault/Cargo.toml`, `crates/orgsidian-vault/tests/atomic.rs`
- Supply chain: `deny.toml:112-174` (bans + the rusqlite invariant at 127-131), `deny.toml:188-210` (LEAF wrappers), `docs/security/advisory-exceptions.md`, `scripts/check-allowlist-sync.mjs`, CI gates in `.github/workflows/pr.yml:120-208`
- Downstream consumers: Story 3.4 (`epics.md:937`), Story 3.5 (`epics.md:952`), Story 3.6 (`epics.md:966`), Story 3.7 (`epics.md:982`), Story 7.7 running-clock prompt, Story 8.4 FTS5 search budgets (`docs/perf/targets.md`)

### Project Context Reference

No `project-context.md` exists in the repo at story-creation (checked 2026-08-01). `architecture.md`, the epic ACs, and this story file are the authoritative context.

## Dev Agent Record

### Agent Model Used

claude-opus-5[1m] (bmad-dev-story)

### Debug Log References

- `cargo test -p orgsidian-index` — 17/17 green (`tests/schema.rs`).
- `cargo test --workspace --locked` — full suite green, no regressions (parser 34+15+24+4+1, core 17, CLI 5, plugin-api 5, failure-modes 2 passed / 8 ignored placeholders unchanged, perf canary 7, settings 7+3+1, index 17).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` — clean (only the pre-existing vendored `scanner.c` `-Wsign-compare` warnings from `orgsidian-parser`'s build script).
- `cargo deny --locked check all` — `advisories ok, bans ok, licenses ok, sources ok`. **No duplicate version fired from the `rusqlite` tree**, so `deny.toml` is untouched and no `docs/security/advisory-exceptions.md` row was needed (AC7's conditional branch did not trigger).
- `cargo audit` — exit 0; 18 allowed warnings, all pre-existing gtk-rs `unmaintained` advisories.
- **Anti-placebo mutation check** (Story 1.9 discipline): temporarily replaced the tokenizer with `unicode61 remove_diacritics 0` and re-ran the FTS tests — `fts_headlines_folds_diacritics`, `fts_content_applies_porter_stemming` and `fts_reads_text_back_through_the_external_content_table` all failed; schema restored and re-verified green. The tokenizer assertions test real behavior, not the DDL string.
- **One test was written wrong and the DB corrected it:** `wal_mode_persists_into_the_database_file` originally also asserted that a bare `Connection::open` reports `foreign_keys = 0` (SQLite's documented default). It reported `1` — the bundled amalgamation is compiled with `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`, exactly as Dev Note 4 warned. The assertion was replaced by `cascades_are_a_no_op_without_foreign_keys`, which explicitly turns the pragma off and proves the cascade stops firing. That is the honest form of the claim: the guarantee comes from `open()`, not from a build flag.

**Post-review verification (2026-08-02).** `cargo test -p orgsidian-index --locked` — 32/32 (29 integration + 3 unit in `connection.rs`), up from 17. Full workspace suite green, no regressions, failure-modes harness still `2 passed / 8 ignored`. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` clean. `cargo deny --locked check all` → `advisories ok, bans ok, licenses ok, sources ok`, `deny.toml` still untouched. `cargo audit` exit 0, same 18 pre-existing gtk-rs warnings. `git diff main...HEAD --name-only` unchanged at the same 11 paths — the review added no file and touched no sentinel.

**Anti-placebo mutation checks on every new assertion** (Story 1.9 discipline). Each mutation was applied, the suite run, and the schema restored: reinstating `ON DELETE CASCADE` on `headlines.file_id` → `deleting_a_file_is_rejected_while_its_headlines_remain` fails; `remove_diacritics 2` → `1` → `fts_headlines_folds_diacritics_at_level_two` fails (the previous `café`-only test survived this mutation, which is why it was rewritten); removing the transaction from `apply_schema` → `a_failed_schema_application_leaves_no_partial_database` fails; dropping `NOT NULL` from `vault_meta.key` → `vault_meta_rejects_a_null_key` fails; each of the four structural CHECKs replaced by `CHECK (1)` → its own test fails and only that one; adding an off-convention index → `named_index_set_is_exactly_the_ld_11_set` fails (under the old `idx_` prefix filter it did not).

**A comment was wrong and the mutation check caught it.** `the_sanctioned_delete_order_leaves_the_fts_index_queryable` claimed to be the test that fails if the cascade is reinstated. It is not — it follows the correct delete order, so the cascade never fires and the test stays green. The comment now says which test actually guards that (`deleting_a_file_is_rejected_while_its_headlines_remain`) and what this one does pin: that the documented order is executable rather than merely written down.

### Completion Notes List

**What shipped.** `crates/orgsidian-index/` went from a 3-line placeholder to its first real surface: `sql/schema.sql` (version-1 DDL — 8 tables, 2 FTS5 external-content tables, 11 named indices), `SCHEMA_SQL` as the single in-Rust handle to it, `open()` with the LD-4 locked PRAGMAs applied *and verified by read-back*, `IndexError`, and 17 integration tests driving real on-disk databases.

**Disclosed variances — read these before approving:**

1. **`foreign_keys=ON` is a superset of the LD-4 locked set (AC5, deliberate).** LD-4/LD-14 name six PRAGMAs; `open()` applies seven. Foreign-key enforcement is per-connection, non-persistent, and OFF by SQLite default — without it every `ON DELETE CASCADE` in the schema is a silent no-op, so deleting a file row would orphan its headlines, tags, properties, clock entries and links rather than removing them. Rationale is in the `open()` doc comment; `cascades_are_a_no_op_without_foreign_keys` pins the consequence in a test.
2. **`files.quarantined` instead of LD-41's `vault_meta` wording (Dev Note 5, deliberate).** Per-file state modelled as columns on `files`, not as synthesized keys in the vault-scoped bag. Rationale in `schema.sql`'s `vault_meta` comment; deferred-work row filed asking for an architecture.md addendum. `architecture.md` itself is untouched (process archive).
3. **Index superset over LD-11 (AC4, disclosed).** LD-11 names five index targets; the schema ships eleven indices. The four extras are `idx_headlines_file_id`, `idx_headlines_parent_id`, `idx_links_file_id` (the FK columns the CASCADEs traverse — unindexed, every `DELETE FROM files` becomes a full scan of `headlines`/`links`) and `idx_links_target` (the FR-13 backlink traversal, without which the `links` table cannot answer the question it exists for). Recorded in a comment above the index block.
4. **DDL is NOT `IF NOT EXISTS`-guarded (AC6, first branch taken).** Re-applying `SCHEMA_SQL` to an initialized database fails with a duplicate-object error, asserted by `re_applying_the_schema_fails_loudly`. A silently idempotent schema would let a Story 3.4 migration bug pass unnoticed.
5. **`tempfile` declared as `{ workspace = true }`, not the bare `tempfile = "3"` AC7 quotes.** AC7 pointed at `orgsidian-vault/Cargo.toml`, which predates the Story 1.12-review hoist of `tempfile` into `[workspace.dependencies]`; `orgsidian-core` and `orgsidian-watcher` both use the workspace form, and the workspace entry's own comment states it "replaces the bare `tempfile = "3"` previously declared at the crate level". Followed the majority convention and `[[feedback_version_policy]]` single-source-of-truth rather than the AC's literal text. Same crate, same version — pin behavior is identical.

**Additional variances introduced by the 2026-08-02 code review (all four decisions above):**

6. **`headlines.file_id` has NO `ON DELETE CASCADE` — a deliberate deviation from AC2.** AC2 specifies `file_id` FK → `files(id)` `ON DELETE CASCADE`. Shipping it produced a reproducible `SQLITE_CORRUPT`: a cascade removes the headline rows inside SQLite, leaving both external-content FTS5 tables pointing at rows that no longer exist, and `snippet()` then raises `database disk image is malformed` for what was a routine single-file delete. `NO ACTION` turns that into an immediate foreign-key rejection instead. Every other cascade in the schema is unchanged — no other table is FTS-indexed. `deleting_a_file_is_rejected_while_its_headlines_remain` pins it; mutation-verified.
7. **`apply_schema(&mut Connection)` is a new public item beyond AC5's surface.** AC5 names `open` and `IndexError`. Executing `SCHEMA_SQL` bare leaves a permanently half-built database on any mid-batch failure — one that reports the same `table files already exists` a healthy database does, so it cannot be told apart. The transaction cannot live in the DDL text without breaking the Story 3.4 seam (`rusqlite_migration` already wraps each migration, and a nested `BEGIN` fails), so it lives in a helper. `SCHEMA_SQL` remains the single DDL text and `schema_sql_executes_inside_a_caller_supplied_transaction` asserts 3.4 can still wrap it.
8. **Four structural CHECK constraints beyond AC2's column list.** `parent_id <> id`, `byte_end >= byte_start`, `(todo_keyword IS NULL) = (todo_done IS NULL)`, and `quarantined = 0 OR quarantine_reason IS NOT NULL`. Scoped deliberately to states the parser cannot emit, where a violation is by construction a Story 3.6 bug; content-shaped values a real `.org` file can produce (duplicate tags, empty strings, `level` outside 1..6, a clock both running and timed) stay representable, consistent with the `level` rationale AC2 already endorses. All four mutation-verified.

**Scope fence held.** No migration runner, no `rusqlite_migration`, no `migrations/`, no `PRAGMA user_version` write. No pool, no `deadpool-sqlite`, no Tokio. No `query/` module, no `IndexQuery`. No parser dependency — the schema is designed *against* the Story 2.3 semantic types (every column has a named source field in Dev Note 2) but imports nothing. No DB-path policy: `open(path: &Path)` takes what it is handed. No `[workspace.dependencies]` entry for `orgsidian-index` and no `orgsidian-core` edit (LEAF rule holds).

**Sentinels verified byte-untouched** via the branch-scoped form `git diff main...HEAD --name-only` (per Story 3.2's review, `git diff HEAD` is non-probative): `deny.toml`, every `crates/*/tests/anchor.rs`, `crates/orgsidian-vault/`, `crates/orgsidian-parser/`, `.github/workflows/*`, `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, `docs/failure-modes/coverage-matrix.md` (`EXPECTED_REMAINING_PLACEHOLDERS` stays at 8 — the coverage test passes unchanged).

**Dependency delta is exactly what AC7 allowed.** `rusqlite 0.40.1` (MIT) with `bundled`, pulling `libsqlite3-sys`, `hashlink`, `hashbrown`, `fallible-iterator`, `fallible-streaming-iterator`, `vcpkg`, plus two wasm-target-only crates (`sqlite-wasm-rs`, `rsqlite-vfs`) that appear in `Cargo.lock` but are never built for a native target. `cargo deny check bans` passes with `deny.toml` untouched.

**Seam left for Story 3.4 (Dev Note 7).** One DDL text (`sql/schema.sql`), one Rust handle (`SCHEMA_SQL`), forward-only rule stated in the file header. 3.4 must pick one of the two documented resolutions and keep exactly one copy — filed as the first deferred-work row.

**Deferred-work rows filed:** 7 (schema-to-migration fork risk, `busy_timeout` unset, LD-41 wording, `Preamble.directives` untabled, ranged/repeating timestamps unmodelled, index DB text duplication, path-identity policy).

### File List

- `Cargo.toml` (modified) — `rusqlite` in `[workspace.dependencies]`
- `Cargo.lock` (modified) — 9 new crate entries
- `crates/orgsidian-index/Cargo.toml` (modified) — deps + dev-deps, placeholder comment replaced
- `crates/orgsidian-index/sql/schema.sql` (new) — version-1 DDL
- `crates/orgsidian-index/src/lib.rs` (modified) — module decls, re-exports, `SCHEMA_SQL`, crate doc rewritten to present tense
- `crates/orgsidian-index/src/connection.rs` (new) — `open()` + locked-PRAGMA application and verification
- `crates/orgsidian-index/src/error.rs` (new) — `IndexError`
- `crates/orgsidian-index/tests/schema.rs` (new) — 17 integration tests
- `_bmad-output/implementation-artifacts/deferred-work.md` (modified) — story-3.3 section, 7 rows
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified) — 3.3 → in-progress → review
- `_bmad-output/implementation-artifacts/3-3-define-sqlite-schema-locked-pragmas.md` (modified) — this file

## Change Log

- 2026-08-01 — Story created (bmad-create-story). Status → ready-for-dev.
- 2026-08-02 — Implementation complete (bmad-dev-story): version-1 SQLite schema, FTS5 external-content tables with the LD-4 tokenizer, LD-11 index set, `open()` with verified locked PRAGMAs, `IndexError`, 17 integration tests. All gates green. Status → review.
- 2026-08-02 — Code review (bmad-code-review, 3 adversarial layers). 26 findings: 4 decisions resolved and applied, 12 patches applied, 10 deferred, 6 dismissed. Substantive changes: `ON DELETE CASCADE` removed from `headlines.file_id` (it made both FTS5 tables raise `SQLITE_CORRUPT` on a routine file delete), `apply_schema` added for atomic DDL application, four structural CHECK constraints, `vault_meta.key NOT NULL`, `LOCKED_PRAGMAS` built from its verified constants, and six corrected doc/comment claims. Tests 17 → 32, every new assertion mutation-verified. All gates green. Status → done.
