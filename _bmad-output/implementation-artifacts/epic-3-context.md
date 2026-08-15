# Epic 3 Context: Vault & SQLite Index Foundation

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

This epic builds the persistence and indexing spine every later feature stands on: the `orgsidian-vault` crate (atomic writes plus Dirty Buffer tracking), the `orgsidian-index` crate (normalized SQLite schema, forward-only migrations, connection/rebuild policy), and the `orgsidian-watcher` crate (filesystem change detection). It delivers the user-facing ability to designate a folder as a Vault and have all `.org` files recursively indexed with visible progress, plus CLI commands to init/rebuild/inspect the index. It matters because agenda, search, backlinks, editing, and conflict-handling in later epics all query this index and rely on the atomic-write and Dirty Buffer primitives scaffolded here — while preserving the core invariant that `.org` files are the source of truth and the SQLite index is derived and disposable.

## Stories

- Story 3.1: Implement atomic-write subsystem with AV-retry wrapper
- Story 3.2: Scaffold Dirty Buffer manager
- Story 3.3: Define SQLite schema + locked PRAGMAs
- Story 3.4: Wire `rusqlite_migration` forward-only migrations
- Story 3.5: Add `deadpool-sqlite` reader pool + dedicated writer task
- Story 3.6: Vault designation UI + initial scan progress
- Story 3.7: Ship `orgsidian index {init|rebuild|stats|integrity}` CLI commands

## Requirements & Constraints

- Designating a folder as a Vault recursively indexes all `.org` files. Only one Vault is open at a time. Initial indexing of a 1,000-file Vault must complete in under 30 seconds on baseline hardware, with visible progress; a subsequent launch on an unchanged Vault must open the cached index in under 1 second.
- The SQLite index is fully derived and never the source of truth. Deleting the index file and relaunching must produce an identical agenda and search experience after rebuild. The index file lives in an OS-conventional application-support location, never inside the Vault.
- Every save uses temp-file-and-rename atomic semantics so power loss or interference cannot corrupt the source file. Transient locks (antivirus / OS search indexer) are absorbed by a 3-retry exponential backoff (base 100ms).
- While a file has unsaved buffer changes (Dirty Buffer), Orgsidian is its sole writer. This epic only scaffolds the Dirty Buffer state and detection primitives; enforcement (block-save fallback) and the full three-pane Merge Dialog land in later epics.
- Query-path performance budgets the connection model must enable: agenda recompute under 100ms and search under 200ms (first 50 results) on a 1,000-file Vault, achieved via incremental index updates rather than full rebuilds.
- Large-vault indexing UX: per-file progress reported as `(N of M, X errors)`, cancellable, checkpointed every 100 files; a cancelled scan retains its partial index.
- The CLI is a primary integration-test surface; index subcommands must support a `--json` flag for scripting and exit non-zero on integrity failure.

## Technical Decisions

- **Crates.** `orgsidian-vault` (atomic write + Dirty Buffer), `orgsidian-index` (schema, migrations, pool, queries), `orgsidian-watcher` (`notify-rs` wrapper). All are leaf-respecting workspace crates.
- **Atomic writes** wrap the `atomic-write-file` crate. Fault injection in tests goes through a custom `FileSystem` trait fake. Orphan `*.tmp.<pid>` files from dead PIDs are cleaned up on Vault open.
- **SQLite schema** (normalized): tables `files`, `headlines`, `tags`, `properties`, `clock_entries`, `links`, `vault_meta`, `_schema_version`; FTS5 virtual tables `fts_headlines` and `fts_content` use external-content references with application-managed sync (no triggers). Indices on `(file_path)`, `(headline_id)`, `(scheduled_date)`, `(deadline_date)`, `(tag, headline_id)`. Schema at `crates/orgsidian-index/sql/schema.sql`.
- **Locked PRAGMAs** on connection init: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=268435456`, `cache_size=-64000`, `temp_store=MEMORY`, `wal_autocheckpoint=4000`. FTS5 tokenizer configured as `unicode61 remove_diacritics 2` + `porter`.
- **Migrations** via `rusqlite_migration` (≥1.3), forward-only, from SQL files at `crates/orgsidian-index/migrations/NNNN_kebab-case-description.sql`; `PRAGMA user_version` bumped per migration and used to detect schema drift.
- **Rebuild policy**: incremental via the watcher during normal operation; a full rebuild is triggered by `PRAGMA user_version` mismatch, `PRAGMA integrity_check` failure, or an explicit user/CLI command.
- **Connection management**: a reader pool via `deadpool-sqlite` (default size 4) plus a single dedicated writer implemented as a Tokio task consuming `IndexUpdate` messages over an `mpsc` channel. Readers must never block each other.
- **Async model**: Tokio runtime; `tokio::fs` for watcher/indexer I/O; CPU-bound work offloaded via `spawn_blocking`.
- **State boundary**: the SQLite index stays outside the Vault; per-Vault state lives at `<Vault>/.orgsidian/`; global state (recent Vault paths, defaults) lives in the OS config directory.
- **IPC / commands**: Tauri commands are snake_case in Rust, auto-renamed to camelCase in the generated `tauri-specta` client; the frontend consumes the typed `commands`/`events` client, never raw `invoke` strings. Commands return `Result<T, OrgError>`; progress is pushed as Tauri events (e.g. `IndexProgress { current, total, errors }`).
- **Conventions**: no `unwrap()`/`expect()` outside tests/`main()`; one concern per file (~400 lines); structured `tracing` logging (no `println!`); modules implementing an FR carry an `//! Implements FR-NN` doc-comment header.

## UX & Interaction Patterns

- Vault designation happens through a native file picker — on first launch when no config/Vault exists, and later from Settings. Progress during the initial scan is shown in a non-modal UI, not a blocking modal, and the scan is cancellable.
- Folder-creation or designation failures (disk full, permission denied) surface as an inline error in the picker; Orgsidian must not silently fall back to a default location chosen for the user.
- If the user dismisses the picker without choosing, the app exits cleanly with no confirmation interruption; they can relaunch and pick later.
- The broader Starter Vault picker and populated-Today onboarding flow depend on this epic's designation + indexing path, but the Starter Vault content itself belongs to later epics.

## Cross-Story Dependencies

- Internal order: 3.1 → 3.2 (Dirty Buffer builds on the vault crate); 3.3 → 3.4 (migration 0001 embeds the schema) → 3.5 (pool/writer over the migrated DB); 3.6 depends on 3.3 + 3.4 + 3.5; 3.7 depends on 3.4 + 3.5.
- Upstream: Epic 1 (workspace scaffold, CI, typed IPC bridge, TOML settings store from Story 1.18) and Epic 2 (parser + serializer producing the AST that populates the index) must be in place.
- Downstream: Epic 4 (editor) is gated on Epic 3 closing; Epic 5 builds Single-Writer / block-save enforcement on the Dirty Buffer scaffold; Epic 9 completes the Merge Dialog invariant; Epics 6-8 query this index for agenda, search, and backlinks.
