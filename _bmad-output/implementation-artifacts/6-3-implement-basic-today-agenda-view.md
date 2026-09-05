---
title: 'Implement basic Today Agenda view'
type: 'feature'
created: '2026-09-05'
status: 'review'
baseline_commit: 'ec04842'
review_loop_iteration: 0
github_issue: 54
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** With Epic 4 closed there is no `/today` surface at all — the route is a Story 3.6/4.6/4.7 placeholder ("Today (placeholder)") that only hosts the interim Vault-picker and Keybindings-settings mounts. `orgsidian-index` has no read/query layer whatsoever (only the write-side `sync` module exists). The user has no way to see "what's due today" on launch, and there is no click-to-open path from an agenda item back to its source Headline in the editor.

**Approach:** Add the FIRST read query to `orgsidian-index` (`query::agenda::today`) — a single `SELECT` over `headlines` JOIN `files`, Scheduled-today + Deadline-overdue-or-today, non-quarantined files only, DONE items excluded, ordered `(file_path, position)` so per-file grouping is a stable partition with no client-side re-sort. `today` is a caller-supplied `YYYY-MM-DD` string (the frontend's local calendar day), never a server-side clock read — the same convention `set_scheduled` (Story 4.8) established. `orgsidian-core` wraps it with a read-only `agenda_today(vault_root, today)` mirroring the existing `index_stats`/`index_integrity` shape (resolve DB path → refuse if absent → fresh `IndexPool`). `orgsidian-shell-app` adds the `agendaToday` command with a camelCase `AgendaItemDto` wire projection (`i64` rowid/byte-offset narrowed to `u32` — specta forbids BigInt export). Frontend: `AgendaToday.tsx` queries once per mount, groups by file, renders a `Link` per item to a NEW TanStack Router route `/editor/$filePath/$headlineId` (percent-encoded `$filePath` segment survives an embedded `/`) carrying an optional `byteStart` search param; `Editor.tsx` gains an `initialByteOffset` prop that places the cursor there on load (best-effort — absent/out-of-range is a silent no-op). The perf-AC ("<500ms on a 1000-file Vault") is gated via the Story 1.12 `assert_no_perf_regression!` macro against a synthetic 1000-file/5-headline fixture, committed baseline included.

## Boundaries & Constraints

**Always:**
- `today` crosses every boundary (SQL param → core fn → IPC command → frontend `localTodayIso()`) as a plain `YYYY-MM-DD` string supplied by the CALLER — never a `chrono::Local::now()` / server-side clock read (timezone-safety precedent from Story 4.8's `set_scheduled`).
- The query result is already sorted `(file_path, position)`; `AgendaToday.tsx` partitions that order into groups and MUST NOT re-sort.
- camelCase IPC wire via the established manual-rename precedent (`ConflictSummary`/`OrgError`): `AgendaItemDto` carries `#[serde(rename_all = "camelCase")]` since the pinned `tauri-specta =2.0.0-rc.25` has no project-wide rename.
- `i64` rowid/byte-offset fields narrow to `u32` at the DTO boundary (specta-typescript forbids exporting `i64`/`u64` — BigInt precision loss); documented as safe because a Vault's headline count / file byte length never approaches 4 billion (same call `ConflictSummary`'s byte lengths already make).
- Match the LEAF crate graph: the new `orgsidian-index::query` module is read ONLY through `orgsidian-core`'s `agenda_today` façade function; `orgsidian-shell-app` never imports `orgsidian-index` directly.
- `--org-*` CSS token vocabulary in `AgendaToday.tsx` (no new tokens invented — Story 6.7 ships the full theme system; overdue is marked with TEXT ("Overdue"/"Due today"), never color alone, per the established LD-58 "never color alone" discipline).
- Match surrounding module-doc/comment density and trace headers (`//! Implements FR-7 (...)`) on every new Rust module.

**Ask First:**
- Any change to the `IndexQuery`/`AgendaQuery` trait shape Story 6.5 will later freeze — this story ships only the plain `agenda::today` function the trait wraps, not the trait itself.
- Adding any new external dependency beyond the warmed lockfile (none needed — `rusqlite` for the orgsidian-core perf test dev-dependency is already pinned workspace-wide, used by `orgsidian-index`).

**Never:**
- No Today Dashboard sections (Inbox Preview, Active Clock, Today-Tag, collapsible chevrons, copy-blessed empty-state tone) — that is Epic 7 (Stories 7.1/7.3) on top of this surface.
- No recurring-timestamp (`+1w` repeater) expansion — the schema stores only the literal date the parser saw (documented "NOT MODELLED IN v1" deferred-work note); a repeating task shows on its stored date only.
- Do NOT touch `sprint-status.yaml`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Headline Scheduled for `today` | `scheduled_date == today` | Included, `overdue=false` | N/A |
| Headline Scheduled for a different day | `scheduled_date != today` | Excluded | N/A |
| Headline Deadline before `today` | `deadline_date < today` | Included, `overdue=true` | N/A |
| Headline Deadline == `today` | `deadline_date == today` | Included, `overdue=false` | N/A |
| Headline Deadline after `today` | `deadline_date > today` | Excluded | N/A |
| Headline marked DONE | `todo_done = 1` | Excluded even if Scheduled/Deadline matches | N/A |
| File quarantined (LD-41) | `files.quarantined = 1` | Its headlines excluded entirely | N/A |
| Multiple matching files | N files, M headlines each | One flat list ordered `(file_path, position)`; frontend groups into N sections in that order, no re-sort | N/A |
| No active Vault | `AppState.index` empty | `agenda_today` command → `OrgError::Vault` ("no active vault; designate a vault first") | frontend renders the message via `role="alert"` |
| No index for the resolved Vault root | DB file absent | `OrgError::Index` ("no index found… run `orgsidian index init`") | same alert path |
| Nothing matches today | empty result | `AgendaToday` renders "Nothing scheduled or due today." | N/A |
| Click an item | any `AgendaItemDto` | `Link` navigates to `/editor/$filePath/$headlineId?byteStart=<n>` | `$filePath` percent-encoded so an embedded `/` survives as one segment |
| Editor opens with `byteStart` | valid in-range offset | Cursor placed there, scrolled into view | out-of-range/absent offset → CM6 default (start of doc), never an error |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-index/src/query/mod.rs` -- NEW. `pub mod agenda;` — the Story 6.3 read-path root, precursor to the Story 6.5 `IndexQuery` trait freeze.
- `crates/orgsidian-index/src/query/agenda.rs` -- NEW. `//! Implements FR-7 (...)`. `AgendaItem` struct + `today(conn, today) -> Result<Vec<AgendaItem>, IndexError>`. Colocated unit tests (Scheduled-today inclusion/exclusion, Deadline overdue/due-today/future, DONE exclusion, quarantined-file exclusion, per-file/position grouping order).
- `crates/orgsidian-index/src/lib.rs` -- MODIFY. `pub mod query;` + crate-doc sentence.
- `crates/orgsidian-core/src/index/mod.rs` -- MODIFY. `pub use orgsidian_index::query::agenda::AgendaItem;` + `agenda_today(vault_root, today)` (mirrors `index_stats`/`index_integrity`'s resolve→refuse-if-absent→fresh-`IndexPool` shape).
- `crates/orgsidian-core/src/lib.rs` -- MODIFY. Re-export `agenda_today, AgendaItem` from the `index` module alongside the existing surface.
- `crates/orgsidian-core/Cargo.toml` -- MODIFY. `rusqlite` added as a dev-dependency (perf test seeds a synthetic index directly); new `[[test]]` block for the perf-AC test (`required-features = ["test-support"]`, same reason `perf_canary` needs it).
- `crates/orgsidian-core/tests/story_6_3_agenda_today_perf.rs` -- NEW. Story 1.12 `assert_no_perf_regression!` gate over a synthetic 1000-file/5-headline-per-file in-memory index; committed baseline at `tests/perf-baselines/story-6.3-agenda-today.json`.
- `tests/perf-baselines/story-6.3-agenda-today.json` -- NEW. First-run baseline (macos-aarch64: ~1.8ms median — comfortably inside the 500ms absolute NFR; linux-x86_64 entry added by the first CI run on that runner per the documented workflow).
- `docs/perf/targets.md` -- MODIFY. New row: "Today Agenda view render (`/today`)" <500ms / 1000-file Vault / Story 6.3 (precursor to NFR-3).
- `crates/orgsidian-shell-app/src/lib.rs` -- MODIFY. `AgendaItemDto` (`#[serde(rename_all="camelCase")]`, `i64`→`u32` narrowing per field) + `From<orgsidian_core::AgendaItem>` + `agenda_today` command (Vault-scoped via `state.current_vault_root()`); registered in `build_specta`; generalized `no_active_vault()`'s message (was editor-mode-specific wording, now Vault-scoped generically since a second command now shares it). Colocated `agenda_item_dto_projects_every_field` test.
- `crates/orgsidian-shell-app/tests/export_bindings.rs` -- MODIFY. Anchors for `agendaToday` + `AgendaItemDto`.
- `shell-ui/src/components/agenda/AgendaToday.tsx` -- NEW. `//! Implements FR-7`. Queries `commands.agendaToday(localTodayIso())` once per mount; loading/error/empty states; groups the already-sorted result by file; renders a `Link` per item.
- `shell-ui/src/components/agenda/AgendaToday.test.tsx` -- NEW. Loading / empty / error / grouping-order / Link-href (incl. slash-in-path percent-encoding) / overdue-vs-due-today labeling / single-query-per-mount.
- `shell-ui/src/routes/_layout/today.tsx` -- MODIFY. Replaces the "Today (placeholder)" heading + Ping button with `<AgendaToday />`; retains the interim `VaultPicker`/`KeybindingsSettings` mounts (Story 6.2/11.1 relocate them).
- `shell-ui/src/routes/editor/$filePath/$headlineId.tsx` -- NEW. The click-to-open target: `validateSearch` for the optional `byteStart` number; renders `<Editor filePath={filePath} initialByteOffset={byteStart} />`.
- `shell-ui/src/components/editor/Editor.tsx` -- MODIFY. New optional `initialByteOffset` prop; on initial load, converts the byte offset to a CM6 doc position (`utf8ByteToJsIndex`, already used by the planning-edit path) and dispatches `{ selection: { anchor }, scrollIntoView: true }` once.
- `shell-ui/src/components/editor/Editor.test.tsx` -- MODIFY. Two new tests: cursor lands at `initialByteOffset`; absent offset is a no-op (default position 0).

## Tasks & Acceptance

**Execution:**
- [x] `orgsidian-index`: `query::agenda::today` + `AgendaItem` + colocated unit tests.
- [x] `orgsidian-core`: `agenda_today` façade function + re-exports.
- [x] `orgsidian-shell-app`: `AgendaItemDto` + `agenda_today` command + `build_specta`/`export_bindings` wiring + colocated test.
- [x] Perf-AC: synthetic-1000-file `assert_no_perf_regression!` test + committed baseline + `docs/perf/targets.md` row.
- [x] Frontend: `AgendaToday.tsx` + colocated tests; `/today` route wiring; `/editor/$filePath/$headlineId` route; `Editor.tsx` `initialByteOffset` + colocated tests.

**Acceptance Criteria:**
- Given Epic 4 closed, when `/today` renders, then `shell-ui/src/components/agenda/AgendaToday.tsx` queries `orgsidian-index::query::agenda::today()` and renders the result. *(Wired end-to-end: `AgendaToday` → `commands.agendaToday` → `orgsidian_core::agenda_today` → `orgsidian_index::query::agenda::today`; tested at every layer.)*
- And items are grouped by source file. *(Backend query is pre-sorted `(file_path, position)`; `groupByFile` partitions without re-sorting; tested with an interleaved-insertion-order fixture proving the sort, not insertion order, drives grouping.)*
- And clicking an item opens the editor at the source Headline via the TanStack Router `/editor/$filePath/$headlineId` route. *(New route exists and is exercised by `AgendaToday`'s `Link`; `byteStart` search param + `Editor`'s `initialByteOffset` place the cursor at the Headline, not just open the file at its top; tested at both the `Link`-href level and the `Editor` cursor-placement level.)*
- And the render completes in <500ms on a 1000-file Vault. *(Story 1.12 `assert_no_perf_regression!` gate committed with a first-run baseline of ~1.8ms median on macos-aarch64 — three orders of magnitude under budget; the absolute target is additionally recorded in `docs/perf/targets.md`.)*

## Design Notes

- **Why `orgsidian-index::query` is a new top-level module, not folded into `sync`.** `sync` is explicitly the write path (Story 3.6); Story 6.5 will freeze a *read*-side `IndexQuery` trait as its own public-API surface (`agenda`/`search`/`backlinks`). Giving reads their own module now means Story 6.5 wraps an already-shaped, already-tested surface rather than carving one out of the write module under freeze pressure.
- **Why `today` is threaded as a string end-to-end rather than parsed into a date type anywhere on the backend.** ISO-8601 date columns sort lexicographically (an existing schema invariant this story reuses, not introduces), so string equality/comparison (`= ?1`, `<= ?1`) is a correct range scan without a single `chrono` parse on the read path. It also sidesteps adding `chrono` as a dependency of the `orgsidian-index` LEAF crate.
- **Why `byte_start`/`byteStart` exists at all, given the AC only names the route shape.** A route that carries `$headlineId` in its path but never uses it to do more than open the bare file would satisfy the AC's letter while missing its point ("opens the editor AT the source Headline"). `byte_start` was already a free column on the same `headlines` row the query already joins, so surfacing it and wiring `Editor`'s existing `utf8ByteToJsIndex` conversion (already used by the Story 4.8 planning-edit path) was a small, load-bearing addition rather than scope creep — confirmed with a dedicated round-trip test on both the `AgendaToday` `Link`-href side and the `Editor` cursor-placement side.
- **Why the perf gate is a direct in-memory `query::agenda::today` benchmark rather than routing through the IPC command.** The query is the entire cost of the read side (`AgendaToday.tsx` does no further backend round-trip once the array lands), so benchmarking it directly is a faithful, hermetic proxy for the AC without needing a live Tauri runtime in the test harness. `assert_no_perf_regression!` requires the `test-support` feature — same pre-existing repo quirk `perf_canary.rs` already carries (`cargo test -p orgsidian-core` alone fails to compile `test_support`'s `serde_json` use without `--features test-support`, since `cfg(test)` alone does not activate an optional dependency; unrelated to this story, verified against `main` before making any change).
- **DONE-exclusion and quarantine-exclusion are query-level, not frontend-level filters.** Filtering before the row leaves the database means a slow render on a large Vault never even serializes rows the UI would immediately discard — a `/today`-render-latency concern given the AC's own 500ms budget.
- **Code-review follow-up: `open_file` resolves a relative `path` against the active Vault root.** `AgendaItemDto.file_path` is `files.path` (the vault-relative, `/`-normalized `rel_path`) — never absolute, by design (kept as-is here; the UI still displays and groups by `rel_path`). Review found `open_file` read that path verbatim via `tokio::fs::read_to_string`, which only works when the process cwd happens to equal the Vault root — not guaranteed in a packaged app, so click-to-open from `/today` could fail to load the file at runtime. Fix: a new `resolve_open_path` helper (`crates/orgsidian-shell-app/src/lib.rs`) joins a relative `path` onto `AppState::current_vault_root()` before reading; an absolute `path` still reads unchanged (back-compat); a relative `path` with no Vault designated returns the same `OrgError::Vault` (`no_active_vault`) other Vault-scoped commands already use. `open_file`'s testable body was split into `open_file_at(path, vault_root)` so the three-way resolution matrix (relative+vault, absolute, relative+no-vault) is unit-tested without a live `tauri::State`. This is deliberately the minimal join-only fix — it does NOT add `..`/symlink vault-escape hardening; that's a `// TODO(vault-scoping)` pointing at the dedicated vault-root-scoping story, not this one.

## Verification

**Commands:**
- `cargo test -p orgsidian-index -p orgsidian-core -p orgsidian-shell-app` -- expected: all green. (`orgsidian-core`'s own default run needs `--features test-support` to compile at all — pre-existing repo quirk, not introduced here.)
- `cargo test -p orgsidian-core --features test-support --test story_6_3_agenda_today_perf` -- expected: green; first run writes/verifies `tests/perf-baselines/story-6.3-agenda-today.json`.
- `cargo clippy -p orgsidian-index -p orgsidian-shell-app --all-targets -- -D warnings` and `cargo clippy -p orgsidian-core --all-targets --features test-support -- -D warnings` -- expected: 0 warnings from touched crates (pre-existing `orgsidian-parser` C-compiler warnings unrelated).
- `cargo fmt -p orgsidian-index -p orgsidian-core -p orgsidian-shell-app -- --check` -- expected: clean.
- `pnpm --filter shell-ui test` (`vitest run`) -- expected: all green, including the new `AgendaToday.test.tsx` and the two new `Editor.test.tsx` cases.
- `pnpm --filter shell-ui build` -- expected: `tsc` + `vite build` succeed (regenerates `routeTree.gen.ts` + `lib/tauri.ts`, both gitignored generated artifacts).

**Result (2026-09-05):** Rust suite GREEN across all 3 touched crates (0 failed; 5 new `query::agenda` unit tests, 1 new shell-app DTO-projection test, 1 new perf-AC test). `cargo clippy` (index/shell-app default; core with `test-support`) and `cargo fmt --check` clean on all three. `cargo test --features test-support --test story_6_3_agenda_today_perf` green; baseline committed (macos-aarch64, ~1.8ms median). `pnpm --filter shell-ui test` GREEN: 22 files, 256 tests (8 new `AgendaToday` tests, 2 new `Editor` cursor-placement tests). `pnpm --filter shell-ui build` succeeds end-to-end (route-based code-splitting confirms `/today` and `/editor/$filePath/$headlineId` both compiled in). `Cargo.lock` updated only for the new dev-dependency declaration (`rusqlite` in `orgsidian-core`, already pinned workspace-wide — no new crate).

## Spec Change Log

- 2026-09-05 — Implemented. `orgsidian-index::query::agenda::today` (new read-path module) + `orgsidian-core::agenda_today` façade + `orgsidian-shell-app`'s `agendaToday` command/`AgendaItemDto`, `AgendaToday.tsx` (new `/today` content) + the new `/editor/$filePath/$headlineId` route + `Editor.tsx`'s `initialByteOffset` cursor-placement, and the Story 1.12 perf-AC gate with a committed baseline. All AC wired and tested end-to-end. Status → review.
- 2026-09-05 — Code-review fix. `open_file` (`crates/orgsidian-shell-app/src/lib.rs`) now resolves a relative incoming `path` against `AppState::current_vault_root()` (via a new `resolve_open_path` helper + `open_file_at` testable body) before reading, so click-to-open from `/today` works when the process cwd differs from the Vault root; an absolute `path` still reads unchanged. `agenda_today`'s return shape and the `rel_path` grouping key are untouched. 3 new colocated Rust unit tests cover relative+vault-root, absolute-unchanged, and relative+no-vault (`OrgError::Vault`); full `open_file` matrix re-verified. Full path-scoping hardening (`..`/symlink vault-escape prevention) is explicitly deferred via a `// TODO(vault-scoping)` to the dedicated vault-root-scoping story — out of scope here.
