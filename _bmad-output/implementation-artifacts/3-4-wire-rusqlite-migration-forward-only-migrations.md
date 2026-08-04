# Story 3.4: Wire `rusqlite_migration` forward-only migrations

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 28

## Story

As the **author / contributor**,
I want migrations managed by `rusqlite_migration` from forward-only SQL at `crates/orgsidian-index/migrations/0001_initial-schema.sql`, applied to latest on demand,
So that LD-12 forward-only migration discipline is enforced and schema drift between dev/prod is detectable via `PRAGMA user_version` per LD-13.

**Traces:** LD-12 (migrations via `rusqlite_migration`, forward-only, `PRAGMA user_version`, SQL files at `migrations/NNNN_description.sql` — architecture.md:400), LD-13 (rebuild policy: `user_version` mismatch → full rebuild — architecture.md:402), LD-11 (schema location + normalized tables — architecture.md:398), LD-14 (connection management, PRAGMAs — architecture.md:404), FR-17 (SQLite derived index — architecture.md:1060). Closes the **schema-to-migration fork risk** Story 3.3 filed (`deferred-work.md:233`) and the two version-audit deferrals it left for this story (`deferred-work.md:245,250`).

## Scope Fence (read first)

This story wires the **migration runner** onto the Story 3.3 schema, inside `crates/orgsidian-index/`. The deliverable is: relocate the version-1 DDL to `migrations/0001_initial-schema.sql` (resolving the 3.3 fork seam — see Dev Note 1), add `src/migrations.rs` (the `rusqlite_migration` wiring + `pub fn migrate`), add the `IndexError::Migration` variant, re-point `SCHEMA_SQL`, and add `tests/migrations.rs` that drives a **real on-disk database** to schema version 1 and asserts it. It is **NOT**:

- **NOT the connection pool or writer task** (Story 3.5). No `deadpool-sqlite`, no `mpsc`, no Tokio, no `IndexUpdate`. `migrate(&mut conn)` takes one plain `rusqlite::Connection`. Crucially, `migrate` is a **separate** function from `open()` and is **not** folded into it — in Story 3.5's model the single writer migrates once and the reader pool must **not** run migrations, so coupling `migrate` into `open` would be actively wrong for 3.5 (Dev Note 5).
- **NOT a second or "down" migration.** LD-12 is forward-only — the index is rebuildable from `.org` files, so no `M::down`, no rollback path. This story ships exactly one migration (`0001`). The vector is `vec![M::up_with_hook(SCHEMA_SQL, …)]`, length 1.
- **NOT the query API** (Epics 7/8). No `src/query/`, no `IndexQuery`, no `SELECT` beyond what tests need. No `pool.rs`.
- **NOT the index-sync engine** (Story 3.6). Nothing walks a Vault, nothing calls the parser, nothing INSERTs real headline/tag/link data. `orgsidian-index` gains **no** dependency on `orgsidian-parser`. The only row this story writes is the single `_schema_version` audit row inside migration `0001`'s hook (AC4).
- **NOT the DB-path / is-this-our-database policy.** Story 3.3's deferred-work row `deferred-work.md:245` (`open()` sets no `application_id`, reads no `user_version`) is assigned to *this* story **jointly with 3.6**. This story adds the `user_version` **write** side (via the migration) and may read it back in tests, but the "is this file actually our index, and at what version, refuse a foreign SQLite file" *guard* on `open()` depends on the path policy Story 3.6 owns. Do **not** add `application_id` or a foreign-file guard to `open()` here — record that it is now half-addressed and re-file the remainder pointing at 3.6 (Dev Note 6).
- **NOT rebuild / integrity execution** (LD-13 → Stories 3.6 + 3.7). This story makes `user_version` the *detectable* drift signal LD-13 compares against; it does **not** implement the drop-and-rebuild-on-mismatch reaction, `PRAGMA integrity_check`, or any CLI. `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, `docs/failure-modes/coverage-matrix.md` stay **byte-untouched** (`EXPECTED_REMAINING_PLACEHOLDERS` stays at `8`).
- **NOT a schema redesign.** The DDL text is Story 3.3's, moved verbatim — not re-authored. Do not add tables, columns, indices, or CHECKs; do not "fix" anything in the DDL. The only edits to the moved file are the header-comment path/tense corrections in Dev Note 1. Any schema change is a *future* `0002_*.sql`, never an edit to `0001`.
- **NOT a `deadpool`/`chrono`/`from-directory` dependency.** The migration is loaded with `include_str!` + `M::up_with_hook`, so the `from-directory` cargo feature (which pulls `include_dir`) stays **off**. `applied_at` is produced by SQLite's `strftime`, not `chrono` (Dev Note 4) — no `chrono` edge.
- **NOT a cross-crate edge.** `orgsidian-index` is a LEAF (`deny.toml:196-198` — only `orgsidian-core` may wrap it). Nothing consumes it yet, so **do not** add an `orgsidian-index` entry to `[workspace.dependencies]` and do not touch `orgsidian-core`.
- **NOT sentinel turf.** Byte-untouched: every `crates/*/tests/anchor.rs`, all of `crates/orgsidian-vault/`, `crates/orgsidian-parser/`, `.github/workflows/*`, and `_bmad-output/planning-artifacts/architecture.md` (process archive — LD-11's `sql/schema.sql` path reference goes stale by design; record it, do not edit the archive — Dev Note 1). `deny.toml` is touchable **only** if `cargo deny check bans` actually fires on a duplicate version introduced by `rusqlite_migration`'s tree — and then only with a matching ledger row (AC1).

## Acceptance Criteria

### AC1 — Dependency delta is exactly `rusqlite_migration`, version-locked to `rusqlite 0.40`.

- `[workspace.dependencies]` gains `rusqlite_migration = "2.6"` with the established story-attributed comment block (match the neighbouring `rusqlite` entry's house style in the root `Cargo.toml`: story number, LD trace, why this version, license).
- **The version is not free.** `rusqlite_migration` pins a *single* `rusqlite` minor per release: `2.6.0 → rusqlite ^0.40`, `2.5.0 → ^0.39`, `2.4.1 → ^0.38` (verified on crates.io 2026-08-04). Only **2.6.x** is compatible with the workspace's `rusqlite 0.40`; `"2.6"` (not a bare `"2"`) is therefore mandatory, and the comment must state the lock-step so a future `rusqlite` bump does not silently select an incompatible line. License MIT (matches `rusqlite`; verify in the `cargo deny check licenses` pass).
- **No cargo features.** The default feature set is what's wanted; do **not** enable `from-directory` (it pulls `include_dir` for a directory-loading path this story does not use). Confirm the resolved feature set adds no unexpected crate.
- `crates/orgsidian-index/Cargo.toml` consumes `rusqlite_migration = { workspace = true }`. **No** `deadpool-sqlite`, **no** `chrono`, **no** `rusqlite_migration/from-directory`.
- Transitive delta is expected to be just an edge to `log` (already `0.4.29` in `Cargo.lock` — `rusqlite_migration` depends on `log ^0.4`), i.e. no *new* crate beyond `rusqlite_migration` itself. Confirm against `cargo tree -p orgsidian-index` and record the actual delta in Completion Notes.
- `Cargo.lock` is committed with the new crate(s) (CI runs every cargo invocation `--locked`; a stale lock fails the build).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked`, `cargo deny --locked check all`, `cargo audit` all green.
- **If `cargo deny check bans` reports a duplicate major version** from the `rusqlite_migration` tree: add a `[[bans.skip]]` entry with `reason = "Story 3.4: …"` **and** the matching row in `docs/security/advisory-exceptions.md` (CI's `scripts/check-allowlist-sync.mjs` fails on ledger drift). `rusqlite`/`rusqlite_migration` must **never** be added to `skip`/`skip-tree` themselves (`deny.toml:127-131` — LD-37 canonical-version invariant; the binding rule names `rusqlite` explicitly). If no duplicate fires (the likely case — the tree is small), `deny.toml` stays untouched.
- Commitlint-conformant conventional commit (`feat(index): …`); plain message, no AI-credit trailers.

### AC2 — The version-1 DDL moves to `migrations/0001_initial-schema.sql` as the single source; `SCHEMA_SQL` re-points; exactly one copy exists.

- `crates/orgsidian-index/sql/schema.sql` is **`git mv`-ed** to `crates/orgsidian-index/migrations/0001_initial-schema.sql` (preserve history — do not delete-and-recreate). The `sql/` directory is left empty and removed.
- Its **contents are unchanged** except the header comment, which is corrected for the new reality (Dev Note 1): the path self-reference (`sql/schema.sql` → `migrations/0001_initial-schema.sql`), and the tense of the two sentences that spoke of Story 3.4 in the future ("Once Story 3.4 lands the migration runner…") now that 3.4 *is* landing. The forward-only rule stays; it is now literally true (this file **is** migration `0001`, and version-2 changes arrive as `0002_*.sql`).
- `src/lib.rs`: `pub const SCHEMA_SQL: &str = include_str!("../migrations/0001_initial-schema.sql");` — the constant keeps its name and its role as **the single in-Rust handle to the DDL text**; only the path in the `include_str!` changes. The three `lib.rs` doc references to `sql/schema.sql` (`lib.rs:9,26` prose + the `include_str!` at `:41`) are updated to the migrations path.
- **There is exactly one copy of the DDL in the repo** after this story: `migrations/0001_initial-schema.sql` on disk, `SCHEMA_SQL` as its only Rust handle, and `M::up_with_hook(SCHEMA_SQL, …)` as the only migration referencing it (AC3). Grep proof required: `grep -rn "schema.sql" crates/ src/` outside story files shows only the migrations path; no `sql/schema.sql` survives; no second DDL body exists anywhere.
- 3.3's `tests/schema.rs` references `SCHEMA_SQL` and `apply_schema`, **not** the file path, so it must pass **byte-unchanged** — do not edit it (its line-519 comment mentions "schema.sql" as prose about the sync contract, not a path; leave it). If any assertion in `tests/schema.rs` breaks, the move was done wrong.
- LD-11 (architecture.md:398) states "Schema lives in `crates/orgsidian-index/sql/schema.sql`" — this becomes stale. It is a **disclosed deviation**, handled exactly as Story 3.3 handled its LD-41 `quarantined`-wording deviation: record it in Completion Notes and file a deferred-work row asking for an architecture.md addendum. **Do not edit `architecture.md`** (process archive).

### AC3 — `src/migrations.rs` wires `rusqlite_migration` and exposes `migrate`.

- A private builder `fn migrations() -> Migrations<'static>` returns `Migrations::new(vec![M::up_with_hook(SCHEMA_SQL, <audit-row hook>)])` — a **factory**, not a `static`, so no `once_cell`/`LazyLock` dependency or MSRV question is introduced (`SCHEMA_SQL` is `&'static str`, so the `Migrations` is `'static`; constructing a one-element `Vec` on each call is trivially cheap and `to_latest` runs at most once per DB open). Both `migrate` and the tests call this one builder, so the migration set has a single definition.
- `pub fn migrate(conn: &mut Connection) -> Result<(), IndexError>` calls `migrations().to_latest(conn)` and maps the error into `IndexError::Migration`. Doc comment states: it is **idempotent** (a no-op on an already-current DB), it is the **versioned production path to schema v1** (superseding `apply_schema` for real use — Dev Note 3), and it takes `&mut Connection` because `to_latest` needs exclusive access.
- The migration uses `M::up_with_hook`, not bare `M::up`: the SQL half is `SCHEMA_SQL` (kept **pure DDL** — the seam invariant), and the Rust hook writes the `_schema_version` audit row (AC4). `rusqlite_migration` runs each migration inside its own transaction and executes the hook after the SQL succeeds, within that same transaction (verified against the crate's `M::up_with_hook` docs), so `SCHEMA_SQL`'s deliberate absence of `BEGIN;`/`COMMIT;` (the 3.3 seam) is exactly what makes this work — a nested `BEGIN` would fail.
- `src/error.rs` gains `#[error("migration failed: {0}")] Migration(#[from] rusqlite_migration::Error)` on the existing `#[non_exhaustive] IndexError`, mirroring the `Sqlite` `#[from]` variant already there. The enum's `#[non_exhaustive]` already permits the new variant without a breaking change.
- `src/lib.rs` declares `pub mod migrations;` and re-exports `pub use migrations::migrate;`. The crate doc gains a present-tense sentence describing the migration runner as shipped (do **not** write forward-looking prose — Story 3.2/3.3 standing rule; describe what shipped, phrase intent as intent).
- No `unwrap`/`expect`/`panic!` in committed non-test code (the hook returns `rusqlite::Result` via `?`; `migrate` returns `Result`). Tests may `.unwrap()` freely.

### AC4 — `PRAGMA user_version` is bumped to 1, and `0001`'s hook writes the `_schema_version` audit row with an explicit version.

- After `migrate` on a fresh DB, `PRAGMA user_version` reads back `1` (this is `rusqlite_migration`'s own mechanism — it stores the applied count there; one migration ⇒ 1). LD-13's drift check compares exactly this value, so a test must read it via the `PRAGMA user_version` statement form and assert `1`.
- Migration `0001`'s hook executes exactly: `INSERT INTO _schema_version (version, description, applied_at) VALUES (1, '<short description>', strftime('%Y-%m-%dT%H:%M:%SZ','now'))`. This **populates the human audit trail** Story 3.3 declared (`_schema_version`) and explicitly assigned to "Story 3.4's migration" (3.3 Dev Note 6; AC2 of 3.3). Rationale for doing it now rather than deferring: the v1 audit row can *only* be written by migration `0001`'s hook — once `0001` has run on a database, a later story cannot retroactively backfill v1's row without a data migration. Establishing "every migration writes its `_schema_version` row" from `0001` keeps the trail from ever being partial.
- The `version` column is **bound explicitly to `1`**, not left to be auto-assigned. This closes `deferred-work.md:250` (`_schema_version.version` is a rowid alias, so an unbound `INSERT` silently invents a plausible version). A comment on the INSERT states that the explicit bind is what makes `_schema_version.version` agree with `PRAGMA user_version` rather than coincide by luck.
- `applied_at` is ISO-8601 `TEXT` via SQLite `strftime` (Dev Note 4) — no `chrono` dependency, consistent with 3.3's "dates are ISO-8601 TEXT" rule and its no-new-dep discipline. `description` is a short static string (e.g. `'initial schema (Story 3.3 baseline)'`).
- **Disclosed divergence to record in Completion Notes:** `migrate` (production path) yields a DB with the `_schema_version` row + `user_version=1`; 3.3's `apply_schema(SCHEMA_SQL)` (retained test primitive) yields the DDL **only** — no audit row, no `user_version` bump. They cannot drift on *table shape* (both execute `SCHEMA_SQL`); they intentionally differ only on version-tracking rows. Document that `migrate` is the production path and `apply_schema` is a DDL-only primitive for shape tests (Dev Note 3).

### AC5 — `tests/migrations.rs` drives a real on-disk DB and asserts real migration behavior.

- Tests live in `crates/orgsidian-index/tests/migrations.rs` (new integration file, alongside the existing `tests/schema.rs`; `tempfile` dev-dep already present from 3.3). **Never `:memory:` for anything asserting `user_version` semantics against the real `open()` connection** — reuse the 3.3 `TempDir` + real `.db` fixture pattern. (`Migrations::validate` legitimately uses an in-memory DB internally; that is the crate's own test helper, not your fixture.)
- Coverage required:
  - **Fresh DB reaches version 1** (the epic AC's headline): open a fresh `.db` via `open()`, `migrate(&mut conn)`, assert `PRAGMA user_version == 1`. Optionally also assert via `migrations().current_version(&conn) == SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())`.
  - **Idempotency:** calling `migrate` a second time on the same connection is a no-op (`Ok`), `user_version` stays `1`, and no duplicate `_schema_version` row appears (`SELECT count(*) FROM _schema_version` stays `1`). This is the property `apply_schema` deliberately lacks (3.3's `re_applying_the_schema_fails_loudly`) — the whole point of the migration layer.
  - **The migrated schema is the 3.3 schema:** after `migrate`, assert the table set and the `idx_%` index set match what `tests/schema.rs` asserts for `apply_schema` (a fresh-vs-migrated equivalence check — the anti-fork guarantee made behavioral). Assert the FK cascade still works (insert file→headline→child, delete, descendants gone) so `foreign_keys=ON` + migration compose correctly.
  - **The audit row (AC4):** exactly one `_schema_version` row, `version = 1`, `applied_at` matches the ISO-8601 shape (a `strftime`/`LIKE '____-__-__T__:__:__Z'` assertion, not an exact timestamp — deterministic per the no-wall-clock-assertion rule).
  - **`Migrations::validate()` passes** — a `#[test]` calling `migrations().validate().unwrap()`. This is the crate's own guard that the migration SQL is well-formed (it runs the up-migration on a throwaway in-memory DB) and catches a malformed `0001` at test time rather than first-run time.
  - **Anti-fork / seam preserved:** a test (or an assertion) proving `SCHEMA_SQL` is still exactly the `migrations/0001_initial-schema.sql` bytes and that `apply_schema` still works on it (i.e. the 3.3 primitive was not broken by the move) — this is largely covered by `tests/schema.rs` passing unchanged, so an explicit note in Completion Notes citing the unchanged `tests/schema.rs` pass is acceptable in lieu of a redundant test.
  - **Anti-placebo (Story 1.9 discipline):** at least one assertion must fail under a plausible mutation. Verify by temporarily making `migrations()` return an **empty** vector (`vec![]`) → the fresh-DB test must fail (`user_version` would be `0`, `SchemaVersion::NoneSet`); and by removing the hook's INSERT → the audit-row test must fail. Restore and re-run green. Record the mutation check in the Debug Log.
- No `unwrap`/`expect`/`panic!` in committed non-test code; tests may `.unwrap()` freely.

### AC6 — Scope fence holds and every gate stays green.

- `git diff main...HEAD --name-only` (branch-scoped — `git diff HEAD` is non-probative for committed work, per Story 3.2's review) shows only: root `Cargo.toml`, `Cargo.lock`, `crates/orgsidian-index/Cargo.toml`, the `git mv` (`sql/schema.sql` → `migrations/0001_initial-schema.sql`), `src/lib.rs`, `src/error.rs`, `src/migrations.rs` (new), `tests/migrations.rs` (new), `deferred-work.md`, this story file, `sprint-status.yaml`, and — **only if** a duplicate-version ban actually fired — `deny.toml` + `docs/security/advisory-exceptions.md`.
- Sentinels verified byte-untouched (list in the Scope Fence). `tests/schema.rs` unchanged. `architecture.md` unchanged. `EXPECTED_REMAINING_PLACEHOLDERS` stays `8`.
- Deferred-work rows filed/updated per Dev Note 7. The 3.3 fork-risk row (`deferred-work.md:233`) is marked **resolved by Story 3.4 (resolution a)**.
- All gates green (AC1 list).

## Tasks / Subtasks

- [x] Task 1 — Dependency (AC1)
  - [x] Root `Cargo.toml`: add `rusqlite_migration = "2.6"` to `[workspace.dependencies]` with the story-attributed comment block, stating the `rusqlite`-minor lock-step and MIT license
  - [x] `crates/orgsidian-index/Cargo.toml`: `rusqlite_migration = { workspace = true }` (no features)
  - [x] `cargo build --workspace --locked`; commit `Cargo.lock`; run `cargo tree -p orgsidian-index` and record the actual transitive delta
- [x] Task 2 — Relocate the DDL, resolve the fork seam (AC2)
  - [x] `git mv crates/orgsidian-index/sql/schema.sql crates/orgsidian-index/migrations/0001_initial-schema.sql`; remove the now-empty `sql/` dir
  - [x] Correct the moved file's header comment: path self-reference + the two future-tense "Story 3.4" sentences → present tense; forward-only rule retained
  - [x] `src/lib.rs`: re-point `SCHEMA_SQL`'s `include_str!` and update the two doc-prose references to the migrations path
  - [x] Grep-prove exactly one DDL copy; confirm `tests/schema.rs` still passes unchanged
- [x] Task 3 — Migration runner (AC3, AC4)
  - [x] `src/error.rs`: add `Migration(#[from] rusqlite_migration::Error)` variant
  - [x] `src/migrations.rs`: `fn migrations()` factory with `M::up_with_hook(SCHEMA_SQL, hook)`; the hook INSERTs the `_schema_version` row with explicit `version = 1` + `strftime` `applied_at`; `pub fn migrate(&mut Connection)`
  - [x] `src/lib.rs`: `pub mod migrations; pub use migrations::migrate;`; add present-tense crate-doc sentence
- [x] Task 4 — Tests (AC5)
  - [x] `tests/migrations.rs`: fresh-DB→v1, idempotency (no duplicate audit row), fresh-vs-migrated schema equivalence + FK cascade, audit-row shape, `validate()`
  - [x] Anti-placebo mutation checks (empty migration vector; hook INSERT removed); restore green; note in Debug Log
- [x] Task 5 — Gates + hygiene (AC1, AC6)
  - [x] fmt / clippy / build / test / deny / audit
  - [x] `git diff main...HEAD --name-only` scope check; sentinels + `tests/schema.rs` + `architecture.md` untouched; `EXPECTED_REMAINING_PLACEHOLDERS` == 8
  - [x] Update `deferred-work.md`: mark the fork-risk row resolved; update/re-file the `application_id`/`user_version` guard and any FK-during-destructive-migration follow-up (Dev Note 7)

### Review Findings

_Code review 2026-08-04 (bmad-code-review — Blind Hunter · Edge Case Hunter · Acceptance Auditor). Gates re-run and verified locally: `fmt` ✅ · `clippy -D warnings` ✅ · `test` (workspace) ✅._

- [x] [Review][Decision] Supply-chain ledger edited outside the AC6 fence — `.cargo/audit-ignore.txt` + `docs/security/advisory-exceptions.md` add `RUSTSEC-2026-0235` (rkyv), but AC6 pre-authorized ledger edits **only** paired with a `deny.toml` duplicate-ban, which did **not** fire (`deny check bans` = ok). The rkyv advisory is genuinely unrelated (feature-off, not in build graph, `Cargo.lock` delta is `rusqlite_migration` alone), freshly published 2026-08-04, and blocks the `cargo audit` gate. **RESOLVED 2026-08-04 — maintainer ratified the approval; the disclosed fence-widening stands as a ratified decision.** [auditor+blind]
- [x] [Review][Patch] `Cargo.toml` version-lock comment overstates what `"2.6"` enforces — a bare Cargo version is a caret req, so `"2.6"` (= `^2.6`) permits 2.7+ exactly as `"2"` would; the actual line-lock to 2.6.x comes from the `rusqlite = "0.40"` pin (2.7 needs rusqlite 0.41). **FIXED** — comment reworded to attribute the lock to the transitive rusqlite pin. [Cargo.toml] [blind]
- [x] [Review][Patch] FK-cascade test failure message misattributed `foreign_keys=ON` to `migrate` — the PRAGMA is set by `open()`, not `migrate`. **FIXED** — message now reads "is foreign_keys=ON (set by open()) still in effect after migrate?". [crates/orgsidian-index/tests/migrations.rs:218] [blind]
- [x] [Review][Defer] `migrate` on a DB at `user_version > 1` surfaces a raw `IndexError::Migration` instead of the LD-13 drift/rebuild reaction — LD-13 reaction is assigned to Stories 3.6/3.7 by the Scope Fence; this story only makes the drift value detectable. [crates/orgsidian-index/src/migrations.rs:85] — deferred, out-of-scope by spec [edge]
- [x] [Review][Defer] Foreign SQLite file already at `user_version=1` makes `migrate` a silent `Ok` no-op; no `application_id`/foreign-file guard — explicitly Story 3.6 (path policy) per Dev Note 6; already re-filed in deferred-work. [crates/orgsidian-index/src/migrations.rs:85] — deferred, out-of-scope by spec [edge+blind]
- [x] [Review][Defer] No `busy_timeout`: concurrent `migrate` from a second connection/process returns `SQLITE_BUSY` immediately instead of waiting — connection PRAGMA set is Story 3.3 turf, concurrency is Story 3.5's single-writer model. [crates/orgsidian-index/src/connection.rs] — deferred, out-of-scope by spec [edge]
- [x] [Review][Defer] `IndexError::Migration` error path + `migrate` failure/mixed-state paths have no test — coverage gap; low risk (`#[from]` is idiomatic) and no clean failure-injection harness exists without a second bad migration. [crates/orgsidian-index/tests/migrations.rs] — deferred, low-value [blind+edge]
- [x] [Review][Defer] `apply_schema` then `migrate` on one DB fails with a duplicate-object error (the two install paths don't compose) — `apply_schema` is a test-only primitive; production uses `migrate` alone, so this is theoretical. [crates/orgsidian-index/src/migrations.rs:85] — deferred, non-production path [edge+blind]

_Dismissed as spec-prescribed or non-issues (7): explicit `version=1` literal (AC4-mandated bind); `applied_at` `LIKE '____-...'` shape (AC5-mandated exact pattern); `Migration` vs `Sqlite` variant split (AC3-mandated design); anti-fork test compares two distinct execution paths, not a string to itself (byte-equality is covered by unchanged `tests/schema.rs`); `validate()` placed in `src/` unit test (forced by the AC3-private factory — correct); stale `sql/schema.sql` refs in future-owned deferred-work rows (cosmetic, AC2 grep scope clean); "gates self-reported" (independently re-run green above)._

## Dev Notes

### 1. The fork seam and the file move (the central decision — resolution **a**)

Story 3.3 deliberately left the "one copy of the DDL" question for 3.4 to resolve and blessed two options (3.3 Dev Note 7; `deferred-work.md:233`). **Resolution (a) is chosen** (confirmed with the maintainer at story-creation): `sql/schema.sql` moves to `migrations/0001_initial-schema.sql`, and `SCHEMA_SQL` re-points there. Why (a) over (b) (keep `sql/schema.sql`, `M::up(SCHEMA_SQL)` with no on-disk migration file):

- It matches the epic AC's literal `migrations/0001_initial-schema.sql` path.
- It establishes `migrations/` as the canonical home for **every** migration in order — `0002_*.sql`, `0003_*.sql` follow the same pattern, and a developer browsing `migrations/` sees `0001` where they expect it. Under (b) there would be no on-disk `0001` and the directory convention for `0002+` would be undecided.
- The cost is contained: three `lib.rs` references and the moved file's header comment. `tests/schema.rs` references `SCHEMA_SQL`/`apply_schema`, not the path, so it does not change.

The one genuine deviation (a) introduces: LD-11 (architecture.md:398) says the schema "lives in `crates/orgsidian-index/sql/schema.sql`". After the move that is stale. Handle it **exactly** as 3.3 handled its LD-41 `quarantined`-wording deviation: a Completion Note + a deferred-work row requesting an architecture.md addendum. `architecture.md` is a process archive; no story edits it in place.

`git mv` (not delete+create) so the DDL's authorship history survives — this is version-1 of the schema-of-record and its blame is worth keeping.

### 2. `rusqlite_migration` version lock-step (the thing most likely to bite a future bump)

`rusqlite_migration` re-exports `rusqlite` types in its public API (`M::up_with_hook`'s hook takes `&rusqlite::Transaction`), so it pins a **single** `rusqlite` minor per release and bumps in lock-step:

| `rusqlite_migration` | requires `rusqlite` |
|---|---|
| 2.6.0 | `^0.40` ← workspace is here |
| 2.5.0 | `^0.39` |
| 2.4.1 | `^0.38` |

(Verified on crates.io 2026-08-04.) Consequences the dev must honor: pin `"2.6"` (a bare `"2"` would let Cargo pick a line incompatible with `rusqlite 0.40`); and when `rusqlite` is next bumped (say to `0.41`), `rusqlite_migration` **must** bump simultaneously (to `2.7+`) or the workspace fails to resolve — this is a coupled pair, like the Tauri ecosystem's conservative-bump exception in `[[feedback_version_policy]]`. State the coupling in the `Cargo.toml` comment so the next bumper sees it, and file a deferred-work note.

The `from-directory` feature (which would pull `include_dir`) stays **off** — we load with `include_str!` + `M::up_with_hook`. `log ^0.4` is `rusqlite_migration`'s only mandatory dep and `log 0.4.29` is already in `Cargo.lock`, so the real new-crate delta is `rusqlite_migration` alone. Confirm with `cargo tree` and record it.

### 3. `migrate` vs `apply_schema` — two functions, one DDL, deliberately different version-state

After this story there are two ways to install the schema, and that is fine because they share `SCHEMA_SQL` and cannot drift on table shape:

- `apply_schema(&mut Connection)` (Story 3.3) — runs the bare DDL inside a caller transaction. **DDL only:** no `user_version` bump, no `_schema_version` row. Re-running it *fails loudly* (3.3's `re_applying_the_schema_fails_loudly`). It is a **test/low-level primitive** for asserting schema shape.
- `migrate(&mut Connection)` (this story) — the **versioned production path.** Idempotent (no-op when current), bumps `user_version`, writes the `_schema_version` audit row. This is what 3.5's writer and 3.6's vault-open call.

Do **not** retire `apply_schema` — 17+ tests in `tests/schema.rs` depend on it and rewriting them through `migrate` is scope creep and regression risk. Document the split in both functions' doc comments so a future caller does not reach for the wrong one. This is the disclosed divergence AC4 requires in Completion Notes.

Do **not** fold `migrate` into `open()`. In Story 3.5's connection model the single writer task migrates once at startup and the reader pool must never run migrations (readers open a schema that already exists). A combined `open_and_migrate` would push migration onto the read path — wrong for 3.5. Keep `open` (PRAGMAs only) and `migrate` (schema version) as separate composable pieces; 3.5 wires them as it needs.

### 4. `applied_at` via SQLite `strftime`, not `chrono`

The `_schema_version.applied_at` value is produced inside migration `0001`'s hook by SQLite itself: `strftime('%Y-%m-%dT%H:%M:%SZ','now')` yields UTC ISO-8601 (`2026-08-04T18:54:03Z`). This keeps the crate free of a `chrono` edge (the query API will add `rusqlite/chrono` when it first maps a typed `NaiveDate` — nothing here does), matches 3.3's "dates are ISO-8601 TEXT" rule, and is deterministic in *shape* (so the test asserts the `____-__-__T__:__:__Z` pattern, never an exact instant — the no-wall-clock-assertion rule). The hook could equally run the INSERT via SQL entirely; using the hook (rather than appending the INSERT to `SCHEMA_SQL`) is what keeps `SCHEMA_SQL` **pure DDL**, which is the invariant that lets `apply_schema` and `M::up(SCHEMA_SQL)` share the same text.

### 5. `up_with_hook` transaction semantics (verified against the crate docs)

`rusqlite_migration` wraps each migration in its own transaction and runs the hook *after* the SQL succeeds, in that same transaction (crate docs, `M::up_with_hook`). So: `SCHEMA_SQL` (no `BEGIN`/`COMMIT` — the 3.3 seam) executes, then the hook's `_schema_version` INSERT runs, then the library commits — atomically. A `BEGIN` embedded in `SCHEMA_SQL` would raise "cannot start a transaction within a transaction" here; its deliberate absence (3.3) is precisely why the seam works. The hook signature is `impl Fn(&rusqlite::Transaction) -> rusqlite::Result<()>`; use `tx.execute(…)?` and return `Ok(())`.

Foreign keys during migration: `open()` sets `foreign_keys=ON`, then `migrate` runs. Migration `0001` is purely additive (`CREATE TABLE`/`CREATE INDEX`/`CREATE VIRTUAL TABLE` + one INSERT), so FK enforcement being on is harmless — FK checks apply to DML and to dropping referenced tables, neither of which `0001` does. **Forward-looking caution (do not implement):** a *future* table-rebuild migration (SQLite's 12-step `ALTER` dance that drops+recreates a table) must run with `foreign_keys=OFF` around it per SQLite guidance, or it will trip FK checks mid-rebuild. That is a `0002+` concern; file it in deferred-work, do not build a toggle now.

### 6. `user_version` write side lands here; the foreign-file guard does not

Story 3.3 deferred `deferred-work.md:245` — `open()` reads no `user_version` and sets no `application_id`, so it will happily convert an unrelated SQLite file to WAL and return `Ok`. This story addresses **half** of it: the migration now *writes* `user_version`, making "what version is this index" answerable, and LD-13's drift check has a value to compare. It does **not** add the *guard* (refuse/rebuild a file whose `user_version` disagrees, stamp an `application_id` so "our index" is distinguishable from "some SQLite file") — that guard needs the path policy Story 3.6 owns (which file is legitimately the vault's index) and the rebuild reaction LD-13 assigns to 3.6/3.7. Re-file the row: version-write side done in 3.4; the `application_id` stamp + mismatch-guard + rebuild reaction remain, owner Story 3.6 (with 3.7 for the CLI `integrity`/`rebuild` surface).

### 7. Deferred-work entries to file/update (`_bmad-output/implementation-artifacts/deferred-work.md`)

Follow the file's existing row format (bold summary, `[path]`, severity, rationale, owner):

- **Mark RESOLVED** the Story 3.3 row *"Schema-to-migration fork risk"* (`deferred-work.md:233`) — resolved by Story 3.4 via resolution (a): DDL relocated to `migrations/0001_initial-schema.sql`, `SCHEMA_SQL` re-points, one copy.
- **Mark RESOLVED** *"`_schema_version.version` is a rowid alias, so an unbound version is silently invented"* (`deferred-work.md:250`) — migration `0001`'s hook binds `version = 1` explicitly.
- **Update** *"`open()` sets no `application_id` and reads no `user_version`"* (`deferred-work.md:245`) — `user_version` write side done in 3.4; `application_id` stamp + foreign-file/mismatch guard + rebuild reaction remain (Dev Note 6). Owner: Story 3.6.
- **LD-11 wording: schema now lives in `migrations/0001_initial-schema.sql`, not `sql/schema.sql`** [architecture.md:398] [LOW] — architecture.md addendum requested; archive not edited (per 3.3 precedent).
- **`rusqlite_migration` ↔ `rusqlite` version lock-step** [`Cargo.toml`] [MED] — `rusqlite_migration` pins one `rusqlite` minor per release; a `rusqlite` bump forces a simultaneous `rusqlite_migration` bump. Owner: whoever next bumps `rusqlite` (couples like the Tauri exception, `[[feedback_version_policy]]`).
- **FK enforcement during future table-rebuild migrations** [`crates/orgsidian-index/migrations/`] [MED] — a `0002+` that drops/recreates a table must wrap itself in `foreign_keys=OFF` per SQLite's 12-step ALTER guidance; `0001` is additive so it does not apply yet. Owner: the first destructive migration.

### 8. Code conventions (established — follow exactly)

- Module doc header naming LD/FR traces (Stories 1.17/1.18/3.1/3.2/3.3 precedent): LD-12, LD-13, LD-11, LD-14, FR-17 for `migrations.rs`.
- `lib.rs` = declarations + re-exports only, no logic (architecture.md:739).
- `error.rs` owns `IndexError`, `#[non_exhaustive]`, `thiserror::Error` derive — the new `Migration` variant mirrors the existing `Sqlite` `#[from]` shape.
- No `unwrap`/`expect`/`panic!` in committed non-test code; `println!` forbidden (use `tracing` if ever needed — not needed here).
- **No forward-looking prose in rustdoc** (standing rule since Story 3.2): describe what shipped; phrase intent as intent. The crate doc's mention of the pool/query API stays intent-shaped.
- Assert *behavior*, not text: `assert!(SCHEMA_SQL.contains("fts5"))` is placebo-green (Story 1.9). Open a real DB, migrate, read `user_version` and `_schema_version` back.

### Project Structure Notes

- New: `crates/orgsidian-index/src/migrations.rs`, `crates/orgsidian-index/tests/migrations.rs`, `crates/orgsidian-index/migrations/0001_initial-schema.sql` (moved). Modified: `crates/orgsidian-index/src/lib.rs`, `crates/orgsidian-index/src/error.rs`, `crates/orgsidian-index/Cargo.toml`, root `Cargo.toml`, `Cargo.lock`. Removed: `crates/orgsidian-index/sql/` (empty after the move).
- `migrations/` is a new directory convention for this crate, the canonical home for all migration SQL (`NNNN_kebab-case.sql`), replacing the single-file `sql/` dir. It is not a Rust module.
- Fixtures: none new. `tempfile` dev-dep already present from 3.3.
- Branch per repo convention: `story/3.4-rusqlite-migrations` off `main`; conventional commits (`feat(index): …`; commitlint gate); plain messages, no AI-credit trailers. GitHub issue **#28**; label `status:in-review` during code review. Merge via `gh pr merge --admin` (branch protection requires 1 review, unsatisfiable solo — `[[project_orgsidian_github_plan]]`).

### Testing Standards Summary

- Integration tests at `crates/orgsidian-index/tests/migrations.rs` (architecture.md:725); the migration path needs a real temp-file DB (`open()` + `.db`), matching `tests/schema.rs`.
- Deterministic: no timing assertions, no exact wall-clock (`applied_at` asserted by shape). A `TempDir` per test; no shared DB file across tests.
- Anti-placebo mutation check mandatory (AC5) — every new assertion must fail under a plausible mutation.
- The perf gate (`assert_no_perf_regression!`) does not apply — no baseline for this crate; `docs/perf/targets.md` assigns FTS5/agenda budgets to Stories 8.4/7.1, not here.

### Latest Technical Information

- **`rusqlite_migration` 2.6.0** is the current stable line (crates.io, verified 2026-08-04). MIT. Requires `rusqlite ^0.40` — the only line compatible with the workspace pin. Pull as `"2.6"`, default features (no `from-directory`).
- **API surface used:** `Migrations::new(vec![M::up_with_hook(sql, hook)])`, `Migrations::to_latest(&mut Connection) -> Result<()>`, `Migrations::validate() -> Result<()>` (test-only, runs ups on an in-memory DB), `Migrations::current_version(&Connection) -> Result<SchemaVersion>` (`SchemaVersion::NoneSet` on a fresh DB, `SchemaVersion::Inside(NonZeroUsize)` after). `M::up_with_hook(sql: &str, hook: impl Fn(&rusqlite::Transaction) -> rusqlite::Result<()>)` — hook runs after the SQL, in the migration's transaction. Verified against docs.rs/rusqlite_migration (fetched 2026-08-04).
- **`PRAGMA user_version`** is `rusqlite_migration`'s version store — one applied migration ⇒ `user_version = 1`. It has no `pragma_*` table-valued function; read it with the `PRAGMA user_version` statement form (as 3.3 does for `mmap_size`/`wal_autocheckpoint`).
- **`log 0.4.29`** already in `Cargo.lock`; `rusqlite_migration`'s `log ^0.4` adds an edge, not a crate.
- Rust: workspace `edition = "2021"`, `rust-toolchain.toml` channel `stable` (unpinned). The `fn migrations()` factory avoids `LazyLock`/`once_cell`, so no MSRV floor is introduced.

### References

- Epic AC source: `_bmad-output/planning-artifacts/epics.md` §Epic 3 → Story 3.4 (lines 937-950); Epic 3 summary (line 364); downstream Stories 3.5 (952), 3.6 (966), 3.7 (982)
- LD-12 (migrations, forward-only, `user_version`): `architecture.md:400`; LD-13 (rebuild policy, `user_version` mismatch trigger): `architecture.md:402`; LD-11 (schema location — goes stale): `architecture.md:398`; LD-14 (connection management): `architecture.md:404`; FR-17: `architecture.md:1060`; LD-37 canonical-version invariant: `architecture.md:1168`, `deny.toml:113-131`
- Predecessor story (the seam this consumes): `_bmad-output/implementation-artifacts/3-3-define-sqlite-schema-locked-pragmas.md` — Dev Note 7 (the 3.3→3.4 seam), AC2 (`_schema_version` "populated by Story 3.4"), Dev Note 6 (`user_version` vs `_schema_version`)
- Deferred-work rows this story resolves/updates: `deferred-work.md:233` (fork risk — resolve), `:245` (`application_id`/`user_version` — update), `:250` (unbound version — resolve)
- Current crate surface to extend: `crates/orgsidian-index/src/lib.rs` (`SCHEMA_SQL`, re-exports), `src/error.rs` (`IndexError`), `src/connection.rs` (`open`, `apply_schema`), `crates/orgsidian-index/sql/schema.sql` (to be moved), `crates/orgsidian-index/tests/schema.rs` (must pass unchanged), `crates/orgsidian-index/Cargo.toml`, root `Cargo.toml` (`[workspace.dependencies]` house style)
- Conventions to mirror: `crates/orgsidian-vault/src/error.rs`, `architecture.md:694-699` (naming), `:723-729` (test placement), `:738-742` (crate org), `:850-877` (AI-agent rules)
- Supply chain: `deny.toml:113-131` (bans + LD-37 invariant), `:196-198` (LEAF wrappers), `docs/security/advisory-exceptions.md`, `scripts/check-allowlist-sync.mjs`, CI gates in `.github/workflows/pr.yml`
- `rusqlite_migration` docs: https://docs.rs/rusqlite_migration/latest/ (`Migrations`, `M::up_with_hook`, `SchemaVersion`)

### Project Context Reference

No `project-context.md` exists in the repo at story-creation (checked 2026-08-04). `architecture.md`, the epic ACs, Story 3.3's file, and this story are the authoritative context.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8 (Claude Code, dev-story workflow)

### Debug Log References

- **Anti-placebo mutation checks (AC5, mandatory):**
  - Mutation 1 — `migrations()` returns `Migrations::new(vec![])` (empty set): `tests/migrations.rs::a_fresh_database_reaches_version_1` **FAILED** as required (`user_version` reads `0`, not `1`). Restored → green.
  - Mutation 2 — hook's `INSERT INTO _schema_version` removed: `the_audit_row_is_version_1_with_iso8601_applied_at` **and** `migrate_is_idempotent_and_writes_the_audit_row_once` both **FAILED** as required (no audit row written). Restored → green.
- **Transitive delta (AC1):** `cargo tree -p orgsidian-index` + `Cargo.lock` diff confirm exactly **one** new crate — `rusqlite_migration v2.6.0`. Its deps (`log ^0.4`, `rusqlite ^0.40`) were already in the tree; `Cargo.lock` `+name =` delta is the single `rusqlite_migration` line.
- **One-DDL-copy proof (AC2):** `grep -rn "schema.sql" crates/ src/` outside story files → only the `migrations/0001_initial-schema.sql` path (in `lib.rs`) plus two `tests/schema.rs` matches that are a function name and a prose comment (both AC-mandated to stay). No `sql/schema.sql` survives; `sql/` dir removed.
- **`cargo deny --locked check all`:** `advisories ok, bans ok, licenses ok, sources ok` — **no** duplicate-version ban fired from the `rusqlite_migration` tree, so `deny.toml` stays untouched (the expected case).

### Completion Notes List

- **Fork seam resolved via option (a) (Dev Note 1):** `git mv sql/schema.sql → migrations/0001_initial-schema.sql` (history preserved), `SCHEMA_SQL` re-points via `include_str!`, `M::up_with_hook(SCHEMA_SQL, …)` is the sole migration referencing it. Exactly one DDL copy. `tests/schema.rs` passes **byte-unchanged** (29 tests), which is the anti-fork guarantee; `tests/migrations.rs` adds a behavioral fresh-vs-migrated table/index equivalence check on top.
- **Disclosed divergence `migrate` vs `apply_schema` (AC4, Dev Note 3):** `migrate` (production path) yields `_schema_version` row + `user_version=1`; `apply_schema` (retained 3.3 primitive, 17+ tests depend on it) yields DDL only. They share `SCHEMA_SQL` so cannot drift on table shape; they intentionally differ only on version-tracking rows. Both functions' doc comments state the split. `migrate` is deliberately **not** folded into `open()` (Dev Note 5) — 3.5's reader pool must not migrate.
- **`up_with_hook` transaction seam (Dev Note 5):** verified `rusqlite_migration` runs each migration in its own transaction and executes the hook *after* the SQL within that same transaction — so `SCHEMA_SQL`'s deliberate absence of `BEGIN;`/`COMMIT;` (the 3.3 seam) is exactly what makes the hook's INSERT compose atomically. `applied_at` via SQLite `strftime` — no `chrono` edge (Dev Note 4).
- **LD-11 wording deviation disclosed (AC2, Dev Note 1):** LD-11 (`architecture.md:398`) says the schema lives at `sql/schema.sql`; after the move that is stale. Handled per the 3.3 LD-41 precedent — **`architecture.md` NOT edited** (process archive); a deferred-work row requests an addendum.
- **`user_version` write side landed; foreign-file guard did not (Dev Note 6):** `migrate` now stamps `user_version=1`. The `application_id` stamp + mismatch/foreign-file guard on `open()` remain, re-filed against Story 3.6 (path policy owner).
- **Supply-chain: `cargo audit` — maintainer-approved ledger widening.** `cargo audit` surfaced **RUSTSEC-2026-0235** (`rkyv 0.7.46`, archive OOB read) — a **pre-existing** advisory, **not** introduced by this story (fails on `main` too). `rkyv` is an *optional, feature-off* dep of `rust_decimal` (transitive: `byte-unit` → `tauri-plugin-log` → `orgsidian-shell-app`); it is **not in the build graph** (`cargo tree -i rkyv` prints nothing) — a lockfile-only phantom, and no rkyv archive is ever deserialized. Per maintainer decision, added `RUSTSEC-2026-0235` to `.cargo/audit-ignore.txt` + a synced `accept` row in `docs/security/advisory-exceptions.md` (Cargo advisories table). This is a **scope-fence widening beyond AC6's file list** (AC6 pre-authorized supply-chain-ledger edits only for a `deny.toml` duplicate-ban, which did not fire); explicitly approved for this story. `deny.toml` remains untouched; `scripts/check-allowlist-sync.mjs` passes (it validates the License-exceptions table only).
- **All gates green:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked` (all pass), `cargo deny --locked check all`, `cargo audit --deny warnings` (both the workspace and `tools/corpus-extractor` locks). New tests: 5 in `tests/migrations.rs` + 1 `migrations::tests::the_migration_set_validates` unit test (which owns the `validate()` requirement, since the `migrations()` factory is private).

### File List

- `Cargo.toml` (modified) — `[workspace.dependencies]` gains `rusqlite_migration = "2.6"` with the lock-step comment block
- `Cargo.lock` (modified) — `+rusqlite_migration v2.6.0` (only delta)
- `crates/orgsidian-index/Cargo.toml` (modified) — `rusqlite_migration = { workspace = true }`
- `crates/orgsidian-index/sql/schema.sql` → `crates/orgsidian-index/migrations/0001_initial-schema.sql` (renamed via `git mv`; header comment corrected to present tense + new path)
- `crates/orgsidian-index/src/lib.rs` (modified) — `SCHEMA_SQL` include path re-pointed; `pub mod migrations;` + `pub use migrations::migrate;`; crate-doc sentence; three `sql/schema.sql` doc refs updated
- `crates/orgsidian-index/src/error.rs` (modified) — `IndexError::Migration(#[from] rusqlite_migration::Error)` variant
- `crates/orgsidian-index/src/migrations.rs` (new) — `fn migrations()` factory + `pub fn migrate`; `_schema_version` audit-row hook; `validate()` unit test
- `crates/orgsidian-index/tests/migrations.rs` (new) — 5 integration tests over a real on-disk DB
- `_bmad-output/implementation-artifacts/deferred-work.md` (modified) — 2 rows marked RESOLVED, 1 updated (PARTIALLY ADDRESSED), 3 new Story 3.4 rows (LD-11 addendum, `rusqlite_migration` lock-step, FK-during-rebuild)
- `.cargo/audit-ignore.txt` (modified) — `RUSTSEC-2026-0235` (rkyv) added; maintainer-approved
- `docs/security/advisory-exceptions.md` (modified) — synced `accept` row for `RUSTSEC-2026-0235`
- `_bmad-output/implementation-artifacts/3-4-wire-rusqlite-migration-forward-only-migrations.md` (modified) — this story file
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified) — status → in-progress → review

## Change Log

- 2026-08-04 — Story created (bmad-create-story). Fork-seam resolution (a) confirmed with maintainer. Status → ready-for-dev.
- 2026-08-04 — Implemented (bmad-dev-story). `rusqlite_migration 2.6` wired; DDL relocated to `migrations/0001_initial-schema.sql` (fork seam resolved, option a); `migrate`/`IndexError::Migration` added; `_schema_version` audit row (explicit `version=1`, `strftime` `applied_at`); 5 integration tests + validate unit test; anti-placebo mutations verified. Pre-existing `rkyv` RUSTSEC-2026-0235 added to audit-ignore ledger (maintainer-approved). All gates green. Status → review.
