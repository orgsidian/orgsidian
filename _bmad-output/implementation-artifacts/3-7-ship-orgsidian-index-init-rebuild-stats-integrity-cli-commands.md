---
title: 'Ship `orgsidian index {init|rebuild|stats|integrity}` CLI commands'
type: 'feature'
created: '2026-08-15'
status: 'done'
review_loop_iteration: 0
context: []
baseline_commit: '16888cdf64a7ad1696bba117d522ff7b42463436'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 3 shipped the SQLite index + scan engine (Story 3.6), but there is no headless way to create, rebuild, inspect, or verify an index. LD-49 (`rebuild-index` as a first-class command) and LD-27 (CLI as the primary integration-test surface) are unmet, and there is no scriptable integrity gate for CI.

**Approach:** Add an `orgsidian index` subcommand tree (`init` / `rebuild` / `stats` / `integrity`), each taking a `<vault>` positional and a `--json` flag, driven entirely through `orgsidian-core` (LEAF rule). `init`/`rebuild` reuse Story 3.6's `scan_vault` unchanged; the new read-only SQL (counts + PRAGMA integrity) lives in new `orgsidian-index` modules; `orgsidian-core` exposes thin domain wrappers; the CLI stays a thin dispatch + renderer mirroring the existing `parse` command.

## Boundaries & Constraints

**Always:**
- CLI reaches the index **only** through `orgsidian-core` (deny.toml LEAF rule — a direct `orgsidian-index`/`-vault` edge fails `cargo deny check bans`).
- All new raw SQL (`SELECT COUNT`, `PRAGMA integrity_check`/`foreign_key_check`, FTS `'integrity-check'`) lives in new **`orgsidian-index`** modules (no raw SQL outside that crate), run via `IndexPool::interact`.
- `init`/`rebuild` reuse `orgsidian_core::scan_vault` (Story 3.6) unchanged — the 100-file checkpoint cadence is inherited.
- No `unwrap`/`expect`/`panic!`/`println!` in non-test code; return `ExitCode`, results via locked `writeln!` to stdout, diagnostics via `eprintln!` to stderr, `tracing` in library code.
- `--json` emits **exactly one** JSON object to stdout, no progress/log noise; `integrity` exits **non-zero** on any failing check; clap usage errors keep exit code 2.
- Frozen seams byte-untouched: `orgsidian-index/src/{connection,migrations,pool,writer,sync,identity,error}.rs`, `0001_initial-schema.sql`, every `tests/*`+`anchor.rs`, `architecture.md`.

**Ask First:**
- Adding a new `OrgError`/`IndexError` *variant* (default: stringify into existing `OrgError::Index { reason }`).
- Changing the index DB filename/location scheme or the `ORGSIDIAN_DATA_DIR` override name.

**Never:** no query/domain API (agenda/search/backlinks — Epics 7/8; `stats` counts are plain aggregates); no tauri/shell/watcher/schema/migration changes; no new state library; no editing frozen seams (new files or the modifiable `orgsidian-core/src/index/mod.rs` only).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `index init <vault>` fresh | valid vault, no index yet | creates+stamps DB in data dir, scans all `.org`, prints indexed/skipped/errors summary (or JSON), exit 0 | vault path missing/not a dir → `OrgError::Vault`, stderr, exit 1 |
| `index init <vault>` re-run | unchanged, already indexed | opens cached, `scan_vault` skips unchanged files, near-zero writes, exit 0 | — |
| `index rebuild <vault>` | existing index | deletes DB (+`-wal`/`-shm`), fresh create+stamp, full re-scan with progress to stdout, exit 0 | delete/scan failure → `OrgError::Index`, exit 1 |
| `index stats <vault>` | indexed vault | prints file/quarantined/headline/tag/link/FTS-doc counts, schema version+applied_at, last-indexed timestamp | no index present → `OrgError::Index` ("run `index init` first"), exit 1 |
| `index integrity <vault>` healthy | consistent index | runs `integrity_check` + `foreign_key_check` + FTS `'integrity-check'` (both tables), prints OK, exit 0 | — |
| `index integrity <vault>` corrupt | inconsistent index | prints each failing check, exit **non-zero** | no index present → error, exit 1 |
| any subcommand `--json` | + `--json` | single JSON object to stdout, no progress lines; error still text-on-stderr + non-zero | — |
| malformed args | e.g. missing `<vault>` | clap usage error to stderr, exit **2** | — |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-cli/src/cli.rs:32` (`enum Command`) — add `Index{#[command(subcommand)] action}`; new `enum IndexAction{Init,Rebuild,Stats,Integrity}` + shared `struct IndexArgs{vault:PathBuf, #[arg(long)] json:bool}`. **clap+std only** (`include!`'d by `build.rs`).
- `crates/orgsidian-cli/src/main.rs:20` (`fn main()->ExitCode`), `:36` (`run_parse` reference) — add `Command::Index` dispatch arm. `src/render.rs` — `render_document` precedent for human renderers.
- `crates/orgsidian-cli/Cargo.toml` — add `tokio` (workspace: `rt-multi-thread`,`macros`); `orgsidian-core`/`clap`/`serde_json`/`assert_cmd` already present. `build.rs:17,27` — `include!` + `clap_mangen` auto-generates `man/orgsidian-index*.1`; man-page test `tests/parse_cmd.rs:106` is the template.
- `crates/orgsidian-core/src/index/mod.rs` — `designate_vault`(:95 async, =`init`), `open_index`(:117), `IndexHandle`(:40: `vault_root`/`db_path`/`shutdown`), **private** `default_index_db_path`(:183)+`vault_db_filename`(:199, FNV-1a→`index-{hash:016x}.sqlite3`), `index_err`(:255). **Add:** `resolve_index_db_path`, `index_stats`, `index_integrity`, `rebuild_index`, + `ORGSIDIAN_DATA_DIR` override in `default_index_db_path`.
- `crates/orgsidian-core/src/index/scan.rs:66` — `scan_vault(&IndexHandle,&AtomicBool,impl FnMut(ScanProgress))`; `ScanProgress{current,total,errors}`(:33), `ScanOutcome{indexed,skipped,errors,cancelled}`(:44). Consumed unchanged. `src/lib.rs:35` — extend façade re-exports.
- `crates/orgsidian-index/src/lib.rs:51-62` — add `pub mod stats; pub mod integrity;` + re-exports. `src/pool.rs:165` — `IndexPool::interact<F,R>` (read seam), `::new`(:141).
- `crates/orgsidian-index/migrations/0001_initial-schema.sql` (frozen, read for columns) — `files`(:74, `quarantined`:83, `indexed_at`), `headlines`(:123, `kind`:149, `todo_keyword`), `tags`/`properties`/`clock_entries`/`links`(:230,`kind`:235), `vault_meta`(:249), `_schema_version`(:267: `version`/`description`/`applied_at`), `fts_headlines`(:309)/`fts_content`(:316).
- **Frozen (consume, do not edit):** `orgsidian-index/src/{connection,migrations,pool,writer,sync,identity,error}.rs`, `0001_initial-schema.sql`, all `tests/*`+`anchor.rs`, `architecture.md`.

## Tasks & Acceptance

**Execution:**
- [x] `crates/orgsidian-index/src/stats.rs` (new) -- `IndexStats` (`#[derive(Serialize)]`, camelCase) + `collect_stats(conn) -> Result<IndexStats, IndexError>` with the `COUNT`/`MAX(indexed_at)`/schema-version SELECTs; add `serde` to that crate's Cargo.toml if absent.
- [x] `crates/orgsidian-index/src/integrity.rs` (new) -- `IntegrityReport{ok, checks: Vec<IntegrityCheck{name,ok,detail}>}` + `check_integrity(conn)` running `integrity_check` + `foreign_key_check` + FTS `'integrity-check'` on both FTS tables.
- [x] `crates/orgsidian-index/src/lib.rs` -- `pub mod stats; pub mod integrity;` + re-export new types/fns (frozen seams untouched).
- [x] `crates/orgsidian-core/src/index/mod.rs` -- `resolve_index_db_path` (canonicalize + `default_index_db_path`), `index_stats`/`index_integrity` (resolve path, error if DB absent, `IndexPool::new`+`interact` → `OrgError::Index`), `rebuild_index` (remove DB+`-wal`+`-shm`, then `designate_vault`+`scan_vault`+`shutdown`), + `ORGSIDIAN_DATA_DIR` override in `default_index_db_path`.
- [x] `crates/orgsidian-core/src/lib.rs` -- re-export the four new fns + `IndexStats`/`IntegrityReport`.
- [x] `crates/orgsidian-cli/src/cli.rs` -- add `Index`/`IndexAction`/`IndexArgs` clap-derive types (clap+std only).
- [x] `crates/orgsidian-cli/src/index_cmd.rs` (new) + `main.rs` dispatch -- four handlers: build a Tokio runtime, `block_on` core; `init`=`designate_vault`+`scan_vault`, `rebuild`=`rebuild_index`; progress callback prints checkpoints to stdout in human mode, no-op under `--json`; `stats`/`integrity` render text or one `serde_json` object; `integrity` failure → `ExitCode::FAILURE`.
- [x] `crates/orgsidian-cli/Cargo.toml` + root `Cargo.toml` -- add `tokio` workspace dep (`rt-multi-thread`,`macros`).
- [x] `crates/orgsidian-cli/tests/index_cmd.rs` + `tests/fixtures/` (new) -- `assert_cmd` per subcommand against a fixture vault (incl. ≥1 malformed file), each with `ORGSIDIAN_DATA_DIR`→`TempDir`; assert human substrings + `--json` field access (never golden strings); `integrity` exits 0 healthy; a man page names `index`.
- [x] `_bmad-output/implementation-artifacts/sprint-status.yaml` -- flip `3-7-…` to `review` at code-review handoff.

**Acceptance Criteria:**
- Given Stories 3.4+3.5+3.6, when `orgsidian index init <vault>` runs on a fresh vault, then a stamped index is created in the data dir and every `.org` file is indexed (verified by a following `stats`).
- Given an existing index, when `orgsidian index rebuild <vault>` runs, then the DB is dropped and fully rebuilt with checkpoint progress on stdout, and its `stats` counts equal a from-scratch `init` (rebuild-identity).
- Given an indexed vault, when `orgsidian index stats <vault>` runs, then headline count, file count, FTS5 document count, schema version, and last-indexed timestamp are printed (text or, with `--json`, one JSON object).
- Given a healthy index, when `orgsidian index integrity <vault>` runs, then all checks pass and exit code is 0; given a corrupt index, exit code is non-zero.
- Given `--json` on any subcommand, when it runs, then stdout is a single parseable JSON object with no progress lines.
- Given no index for the target vault, when `stats`/`integrity` run, then the command errors on stderr and exits 1 (never panics, never creates a DB).
- All gates green: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked`, `cargo deny --locked check all`; `git diff main...HEAD --name-only` shows only Code-Map-listed files; frozen seams byte-unchanged.

## Design Notes

- **`last-rebuild timestamp` = `MAX(files.indexed_at)`** — no dedicated schema key; a full `rebuild` refreshes every `indexed_at`, so the max is the last full-index time. Avoids a `vault_meta` write, keeps the write path frozen. `null`/"never" when empty.
- **`stats`/`integrity` are read-only — no writer.** Resolve the path, refuse if absent, open just an `IndexPool` and run via `interact`. Do **not** call `open_index` (it drop-rebuilds on version mismatch — wrong for inspection).
- **`rebuild` = delete + init:** remove DB (+`-wal`/`-shm`) then re-run fresh-create + `scan_vault` — the simplest correct LD-13 explicit-rebuild, reusing 3.6 wholesale.
- **`ORGSIDIAN_DATA_DIR`** makes `assert_cmd` tests hermetic (macOS `dirs::data_dir()` isn't XDG-overridable): if set, use it as the index base dir; else `dirs::data_dir()/orgsidian/index`. Also a CI/power-user knob.
- **JSON vs progress:** the progress callback prints only in human mode; under `--json` it's a no-op so stdout is exactly one object. Errors are always text-on-stderr + non-zero (mirrors `parse`; no JSON error envelopes).

## Verification

**Commands:**
- `cargo test -p orgsidian-cli -p orgsidian-index -p orgsidian-core --locked` -- expected: new `index_cmd`/stats/integrity tests + existing suites green.
- `cargo run -p orgsidian-cli -- index init <tmp-vault>` then `... index stats <tmp-vault> --json | jq .` -- expected: JSON with `fileCount`/`headlineCount`/`schemaVersion`.
- `cargo run -p orgsidian-cli -- index integrity <tmp-vault>; echo $?` -- expected: `0` on a healthy index.
- `cargo fmt --all -- --check` && `cargo clippy --workspace --all-targets --locked -- -D warnings` && `cargo deny --locked check all` -- expected: all green.

## Suggested Review Order

**Design intent (entry point)**

- Thin dispatch: four handlers, each `block_on`s the `orgsidian-core` façade — the whole shape at a glance
  [`index_cmd.rs:21`](../../crates/orgsidian-cli/src/index_cmd.rs#L21)

**Command surface**

- The `index` subcommand tree — `init`/`rebuild`/`stats`/`integrity`, each with `<vault>` + `--json` (clap+std only)
  [`cli.rs:60`](../../crates/orgsidian-cli/src/cli.rs#L60)
- Dispatch arm wiring the subcommand into `main`
  [`main.rs:24`](../../crates/orgsidian-cli/src/main.rs#L24)

**Output & exit-code contract**

- `integrity` maps a non-`ok` report to `ExitCode::FAILURE` — the scriptable CI gate
  [`index_cmd.rs:87`](../../crates/orgsidian-cli/src/index_cmd.rs#L87)
- Progress prints to stdout in human mode, no-op under `--json` (one clean object)
  [`index_cmd.rs:125`](../../crates/orgsidian-cli/src/index_cmd.rs#L125)

**Core domain wrappers (LEAF boundary — CLI reaches the index only here)**

- `rebuild_index` = delete DB + `-wal`/`-shm`, then fresh `designate_vault` + `scan_vault`
  [`mod.rs:230`](../../crates/orgsidian-core/src/index/mod.rs#L230)
- `index_stats` / `index_integrity` — read-only: resolve, refuse if absent, open just an `IndexPool` (never `open_index`)
  [`mod.rs:191`](../../crates/orgsidian-core/src/index/mod.rs#L191)
- `resolve_index_db_path` — locate without creating any directory (read-only contract)
  [`mod.rs:308`](../../crates/orgsidian-core/src/index/mod.rs#L308)
- Façade re-exports the new surface
  [`lib.rs:36`](../../crates/orgsidian-core/src/lib.rs#L36)

**Index read SQL (new leaf modules — no raw SQL outside `orgsidian-index`)**

- `collect_stats` — the `COUNT`/`MAX(indexed_at)`/schema-version SELECTs
  [`stats.rs:57`](../../crates/orgsidian-index/src/stats.rs#L57)
- `check_integrity` — `integrity_check` + `foreign_key_check` + FTS `'integrity-check'` on both FTS tables
  [`integrity.rs:57`](../../crates/orgsidian-index/src/integrity.rs#L57)

**Tests (anti-placebo)**

- Corrupt-FTS and rebuild-drops-stale-rows pin the integrity gate and the drop+regenerate guarantee
  [`index_cmd.rs:369`](../../crates/orgsidian-cli/tests/index_cmd.rs#L369)
