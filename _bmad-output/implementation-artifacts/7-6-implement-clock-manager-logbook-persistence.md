---
title: 'Story 7.6 — Implement Clock manager + LOGBOOK persistence'
type: 'feature'
created: '2026-09-13'
status: 'done'
review_loop_iteration: 0
baseline_commit: '8aba8c43132404b0518d1fc2609571cf6b6f9c2b'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Orgsidian has no time-tracking. Users cannot clock in/out/resume on a headline, and there is no org-compatible record of tracked time. FR-8 requires clock entries persisted as standard org `CLOCK:` lines in the `:LOGBOOK:` drawer so time tracking survives Orgsidian itself, plus a durable Active-Clock state file that Stories 7.7 (stale-clock prompt) and 7.8 (ClockEditor) build on.

**Approach:** Add a new `orgsidian-core::clock` module that owns the clock manager: clocking in writes an open `CLOCK: [start]` line into the target headline's LOGBOOK (creating the drawer if absent), clocking out closes the matching open line to `CLOCK: [start]--[end] => HH:MM`, and resume re-activates the most-recent unclosed line. The org source file is the source of truth; a per-Vault `.orgsidian/active-clock.json` pointer persists `{ headline_id, started_at, last_active_at }`. Expose `clockIn`/`clockOut`/`clockResume`/`getActiveClock` Tauri commands and a `last_active_at` refresh on window-focus. Provide pure `clock::totals` for time aggregation.

## Boundaries & Constraints

**Always:**
- The org file is the source of truth. All CLOCK mutations are **byte-faithful text splices** on the file source, then `atomic_write` — never re-render via the parser serializer (it is raw-passthrough and ignores mutated semantic fields).
- At most ONE active clock at a time. Clocking into a new headline auto-stops (clocks out) the prior active clock first.
- `active-clock.json` persists EXACTLY `{ headline_id, started_at, last_active_at }` (headline_id = the app-wide index-rowid identity; timestamps as strings). `last_active_at` MUST be refreshed on every window-focus/foreground event — Stories 7.7/7.8 depend on it. This field is a hard requirement.
- clock_out / clock_resume locate the target open CLOCK line by matching `started_at` (robust to byte-offset shifts) — never by a stale index byte offset.
- Wall-clock "now" is dependency-injected into core functions (a `NaiveDateTime`); the Tauri command layer supplies `chrono::Local::now().naive_local()`. Never read the wall clock inside core (matches the starter_vault "today is injected" convention and keeps core deterministic/testable).
- Follow existing conventions: commands are `#[tauri::command] #[specta::specta]` free fns returning `OrgResult<T>`, added to `collect_commands!` in `build_specta()`; per-Vault JSON copies the `coaching.rs` pattern (`.orgsidian/` dir, `atomic_write`, default-on-missing + self-heal-on-parse-error, `OrgError::Io` mapping); wire integers narrowed to `u32`; multi-word DTO fields/enum variants carry `#[serde(rename_all = "camelCase")]`.
- `clock.rs`'s first doc-comment line MUST be exactly `//! Implements FR-8 (functional)`.
- New index items are ADDITIVE free functions only — do NOT modify the frozen `IndexQuery` trait (Story 6.5 semver gate).
- chrono has no `serde` feature in this workspace — serialize timestamps as strings manually.

**Never:**
- Never mutate the `orgsidian-index` `IndexQuery` trait, its methods, or `AgendaItem`.
- Never re-render CLOCK/timestamp text through `serialize_document` from semantic fields.
- Never push, open PRs, run network/`gh` commands, or edit `sprint-status.yaml`.
- No new deps (chrono/serde/serde_json/orgsidian-vault/parser/index already present).
- Out of scope: the ClockEditor UI (7.8), the stale-clock launch modal (7.7), status-bar polish (Epic 13). This story ships the core mechanism + commands + state file + focus listener only.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Clock in, no LOGBOOK | headline with no `:LOGBOOK:` drawer | `:LOGBOOK:`/`:END:` drawer created after PROPERTIES/planning line; open `CLOCK: [now]` inserted; active-clock.json written | N/A |
| Clock in, existing LOGBOOK | headline already has `:LOGBOOK:` | open `CLOCK: [now]` inserted as first line under `:LOGBOOK:`; drawer indentation preserved | N/A |
| Clock in while another active | active-clock.json non-empty for other headline | prior clock auto-closed first, then new open line written; active-clock.json now points at new headline | N/A |
| Clock out | active-clock.json present, matching open line in source | open line closed to `CLOCK: [start]--[now] => HH:MM`; active-clock.json removed | If no matching open line found, remove active-clock.json and return an `OrgError::Vault` describing the desync (never leave a dangling pointer) |
| Clock resume | headline has ≥1 unclosed CLOCK line | most-recent unclosed line's start becomes the active clock (active-clock.json written; source unchanged) | If no unclosed line, start a fresh clock-in |
| Clock out, nothing active | active-clock.json absent | `OrgError::Vault` "no active clock" | error returned, no file writes |
| Duration formatting | start 10:00, end 11:30 | `=> 1:30`; 25h5m → `=> 25:05`; sub-minute rounds down to minutes | N/A |
| last_active_at refresh | window focused, active clock present | active-clock.json `last_active_at` updated to now | silent no-op if no active clock or no vault |
| totals per headline/subtree/tag + range | parsed headlines + scope + optional date range | summed `TimeDelta` of closed entries whose start date is in range; open entries contribute 0 | N/A |
| Malformed active-clock.json | corrupt JSON on disk | treated as absent (self-heal), no panic | returns None / no active clock |

</frozen-after-approval>

## Code Map

- `crates/orgsidian-core/src/clock.rs` — **NEW**. The clock manager. First line `//! Implements FR-8 (functional)`. Public: `ActiveClock { headline_id: u32, started_at: String, last_active_at: String }` (serde, camelCase); `active_clock(vault_root) -> OrgResult<Option<ActiveClock>>`; `active_clock_path(vault_root) -> PathBuf`; `clock_in(vault_root, index_db_or_pool, headline_id, now) -> OrgResult<ActiveClock>`; `clock_out(vault_root, now) -> OrgResult<()>`; `clock_resume(vault_root, ..., headline_id, now) -> OrgResult<ActiveClock>`; `refresh_active_clock(vault_root, now) -> OrgResult<()>`; `totals(scope, range) -> TimeDelta` with `ClockScope<'a> { Headline(&Headline), Subtree(&Headline), Tag { headlines, tag } }` and `DateRange { start, end }` (inclusive NaiveDate). `now` params are `NaiveDateTime`.
- `crates/orgsidian-core/src/lib.rs` — add `pub mod clock;` + `pub use clock::{...}` block, mirroring the `coaching` block (lib.rs:74-79). Re-export the index locate wrapper types as needed.
- `crates/orgsidian-core/src/coaching.rs` — **PATTERN TO COPY** (read fully): `.orgsidian` dir const, path builder, `fs::create_dir_all` + `atomic_write` pretty JSON, read with `NotFound → default` and parse-error → self-heal, local `*_io` closure → `OrgError::Io`. `use orgsidian_vault::atomic_write;`.
- `crates/orgsidian-core/src/index/mod.rs` — reuse the fresh-read-pool pattern from `agenda_today` (mod.rs:263): `resolve_index_db_path(vault_root)`, refuse if absent, `IndexPool::new(&db_path)`, `pool.interact(move |conn| ...).await`. `index_err`/`index_absent_err` mappers live here. Add a `clock`-resolution wrapper (e.g. `pub async fn locate_headline(vault_root, headline_id) -> OrgResult<Option<HeadlineLocation>>`) here or in `clock.rs`, calling the new index free fn.
- `crates/orgsidian-index/src/query/locate.rs` — **NEW**. `pub struct HeadlineLocation { pub file_path: String, pub byte_start: i64, pub byte_end: i64 }` and `pub fn headline(conn: &Connection, id: i64) -> Result<Option<HeadlineLocation>, IndexError>` — `SELECT f.path, h.byte_start, h.byte_end FROM headlines h JOIN files f ON f.id = h.file_id WHERE h.id = ?`. Additive only.
- `crates/orgsidian-index/src/query/mod.rs` — add `pub mod locate;` (and re-export if the module re-exports siblings). Do NOT touch the `IndexQuery` trait (mod.rs:171-250) or `AgendaItem`.
- `crates/orgsidian-parser/src/semantic/{drawer.rs,headline.rs,mod.rs}` — READ-ONLY reference. `Headline { drawers: Vec<Drawer>, clocks: Vec<ClockEntry>, span, children }`; `Drawer { kind: DrawerKind::{Properties,Logbook,Custom}, name, contents, span, contents_span }`; `ClockEntry { start: Timestamp, end: Option<Timestamp>, duration: Option<TimeDelta>, span }`; `Timestamp { active, date: NaiveDate, time: Option<NaiveTime>, ... }`. `analyze(&str) -> Result<Document, ParseError>` reached as `orgsidian_core::parser::analyze`. Serializer is raw-passthrough — do NOT rely on it to emit mutations.
- `crates/orgsidian-parser/src/semantic/timestamp.rs` — `format_planning_timestamp` emits ACTIVE `<...>` form only; CLOCK lines need INACTIVE `[YYYY-MM-DD Day HH:MM]` — format manually with `date.format("%Y-%m-%d %a")` + `time.format("%H:%M")` (locale-independent `%a`).
- `crates/orgsidian-shell-app/src/lib.rs` — add commands `clock_in(headline_id: u32, state)`, `clock_out(state)`, `clock_resume(headline_id: u32, state)`, `get_active_clock(state) -> Option<ActiveClockDto>`; each reads `state.current_vault_root().ok_or_else(no_active_vault)?`, computes `now = Local::now().naive_local()`, calls the core fn. Register all in `collect_commands!` (build_specta, lib.rs:796-813). Add `.on_window_event(move |window, event| { if matches!(event, tauri::WindowEvent::Focused(true)) { let st = window.state::<AppState>(); if let Some(root) = st.current_vault_root() { let _ = orgsidian_core::refresh_active_clock(&root, &now_naive_string()); } } })` to the `tauri_builder` chain (after lib.rs:851, before `.invoke_handler`). Follow the `AgendaItemDto`/`set_scheduled` shape for the DTO + narrowing. Debug builds auto-export `shell-ui/src/lib/tauri.ts`.
- `crates/orgsidian-shell-app/src/main.rs` — no change expected (delegates to `run()`).
- `crates/orgsidian-core/tests/clock.rs` — **NEW** integration tests: local `Fixture` (vault TempDir + index-DB TempDir) copying `tests/scan.rs`; seed an org file, `scan_vault`/`open_index` to get a real `headline_id`, drive `clock_in`/`clock_out`/`clock_resume`, assert org file contents + `active-clock.json`. Async via `tauri::async_runtime::block_on`.
- `tests/traceability.rs` — **NEW** workspace-root grep-smoke (register as a `[[test]]` in `crates/orgsidian-core/Cargo.toml`, mirroring the `settings_trace` entry) asserting `crates/orgsidian-core/src/clock.rs` first line == `//! Implements FR-8 (functional)`. Satisfies the AC's `tests/traceability.rs` reference; keep it extensible for later FR annotations.

## Tasks & Acceptance

**Execution:**
- [ ] `crates/orgsidian-index/src/query/locate.rs` — add `HeadlineLocation` + `headline(conn, id)` free fn (additive), with inline unit test against an in-memory DB (copy the `query/mod.rs` test-db helper).
- [ ] `crates/orgsidian-index/src/query/mod.rs` — declare `pub mod locate;`; leave `IndexQuery`/`AgendaItem` untouched.
- [ ] `crates/orgsidian-core/src/clock.rs` — implement the clock manager (see Code Map + Design Notes). First line `//! Implements FR-8 (functional)`. Include inline `#[cfg(test)]` unit tests for the pure pieces: CLOCK line format (open/closed), duration `HH:MM` formatting, LOGBOOK insert (create-if-absent + prepend-if-present), open-line close/match, `totals` for headline/subtree/tag/range, active-clock.json round-trip + self-heal.
- [ ] `crates/orgsidian-core/src/lib.rs` — wire `pub mod clock;` + re-exports.
- [ ] `crates/orgsidian-shell-app/src/lib.rs` — add the four commands + `ActiveClockDto`, register in `collect_commands!`, add the `on_window_event` focus listener; add a small `now_naive_string()` helper. Inline unit tests for the DTO mapping (mirror `agenda_item_dto` tests).
- [ ] `crates/orgsidian-core/tests/clock.rs` — integration tests over a real scanned vault + index (clock in/out/resume, auto-stop, last_active_at refresh).
- [ ] `tests/traceability.rs` + `crates/orgsidian-core/Cargo.toml` — add the grep-smoke test + `[[test]]` registration.

**Acceptance Criteria:**
- Given a scanned vault, when `clockIn(headlineId)` runs, then the Active Clock is set (active-clock.json = `{headline_id, started_at, last_active_at}`) and an open `CLOCK: [start]` line exists in the headline's LOGBOOK (drawer created if absent).
- Given an active clock, when `clockOut()` runs, then the matching open line becomes `CLOCK: [start]--[end] => HH:MM` under the LOGBOOK and active-clock.json is removed.
- Given a headline with a prior unclosed CLOCK line, when `clockResume(headlineId)` runs, then that most-recent unclosed entry becomes the active clock without mutating the source.
- Given an active clock on headline A, when `clockIn(B)` runs, then A is auto-clocked-out first and only B is active (at most one active clock).
- Given closed clock entries, when `clock::totals(scope, range)` runs for per-headline / per-subtree / per-tag / date-range, then it returns the correct summed duration.
- Given an active clock, when a window-focus/foreground event fires, then active-clock.json `last_active_at` is refreshed (via `on_window_event` `Focused(true)`).
- Given the module, then `clock.rs`'s first doc line is `//! Implements FR-8 (functional)` and `tests/traceability.rs` asserts it.
- Given the frozen index API, then no change is made to `IndexQuery`/`AgendaItem` (cargo-semver-checks stays green).

## Design Notes

CLOCK line grammar (org standard, matches `drawer.rs` parser): open `CLOCK: [2026-09-13 Sat 10:00]`; closed `CLOCK: [2026-09-13 Sat 10:00]--[2026-09-13 Sat 11:30] => 1:30`. Duration is `H:MM` (hours unbounded, minutes zero-padded 00-59), computed `end - start`. Format inactive stamps as `[{date.format("%Y-%m-%d %a")} {time.format("%H:%M")}]`.

LOGBOOK placement: analyze the file, find the target headline (at index `byte_start` for clock_in; by open-line/`started_at` match for clock_out/resume). If a `DrawerKind::Logbook` drawer exists, insert the new open CLOCK line immediately after its `:LOGBOOK:` header line (org prepends newest). If absent, synthesize `:LOGBOOK:\n<indent>CLOCK: ...\n<indent>:END:\n` positioned after the `:PROPERTIES:` drawer if present, else after the planning line if present, else right after the headline line. Match the indentation of an existing PROPERTIES/LOGBOOK drawer when one exists; default to no indent otherwise. Do the edit as a byte splice into the file `source` string, then `atomic_write`; the notify-watcher reindexes on disk change (no explicit resync needed in-command).

Golden round-trip test: `analyze(file)` → `clock_in` splice → `analyze(new)` shows one open ClockEntry with the expected start; `clock_out` splice → `analyze` shows it closed with the right duration; original bytes outside the LOGBOOK are unchanged.

Timestamps in active-clock.json: ISO-like `%Y-%m-%dT%H:%M:%S` strings (chrono has no serde here). `started_at` equals the CLOCK line's start; `last_active_at` starts equal to `started_at` and is bumped on focus.

## Verification

**Commands:**
- `cargo test -p orgsidian-core` — expected: all unit + integration (clock.rs) tests pass.
- `cargo test -p orgsidian-index` — expected: locate.rs unit test + existing tests pass.
- `cargo test -p orgsidian-shell-app` — expected: DTO/command unit tests pass.
- `cargo test -p orgsidian-core --test traceability` — expected: the FR-8 first-line assertion passes.
- `cargo build -p orgsidian-shell-app` — expected: compiles (window-focus listener + commands wired; debug build re-exports `shell-ui/src/lib/tauri.ts` — confirm `clockIn/clockOut/clockResume/getActiveClock` appear).
- `cargo clippy -p orgsidian-core -p orgsidian-index -p orgsidian-shell-app --all-targets` — expected: no new warnings (workspace lint posture).
</content>
</invoke>

## Suggested Review Order

**Clock manager core**

- Entry point — org-standard clock-in: resolve headline, splice open `CLOCK:` line, auto-stop prior, write pointer.
  [`clock.rs:414`](../../crates/orgsidian-core/src/clock.rs#L414)

- The minute-precision invariant — the critical fix that makes production clock-out match the source line.
  [`clock.rs:167`](../../crates/orgsidian-core/src/clock.rs#L167)

- Clock-out closes the open line matched by `started_at` (not a stale byte offset); self-heals a desynced pointer.
  [`clock.rs:474`](../../crates/orgsidian-core/src/clock.rs#L474)

- Resume re-activates the most-recent unclosed line; falls back to a fresh clock-in.
  [`clock.rs:557`](../../crates/orgsidian-core/src/clock.rs#L557)

- Byte-faithful LOGBOOK splice (create drawer if absent, preserve indent + newline style).
  [`clock.rs:336`](../../crates/orgsidian-core/src/clock.rs#L336)

- Open-line lookup, recursive over child headlines, matched by start time.
  [`clock.rs:299`](../../crates/orgsidian-core/src/clock.rs#L299)

- Persisted sidecar schema — snake_case `{headline_id, started_at, last_active_at}`, the 7.7/7.8 contract.
  [`clock.rs:75`](../../crates/orgsidian-core/src/clock.rs#L75)

- Pure time aggregation (headline / subtree / tag / date range).
  [`clock.rs:661`](../../crates/orgsidian-core/src/clock.rs#L661)

**Headline resolution (additive index surface — frozen IndexQuery untouched)**

- New free fn maps a headline rowid → file path + byte span.
  [`locate.rs:40`](../../crates/orgsidian-index/src/query/locate.rs#L40)

- Core fresh-pool wrapper the clock manager calls.
  [`index/mod.rs:318`](../../crates/orgsidian-core/src/index/mod.rs#L318)

**Command + window-focus wiring**

- The four Tauri commands; `now` injected as local wall time here, never read inside core.
  [`lib.rs:816`](../../crates/orgsidian-shell-app/src/lib.rs#L816)

- `last_active_at` refresh on window-focus — the hard requirement for Story 7.7; non-panicking `try_state`.
  [`lib.rs:953`](../../crates/orgsidian-shell-app/src/lib.rs#L953)

- IPC DTO keeps camelCase for the TS client (distinct from the snake_case sidecar).
  [`lib.rs:790`](../../crates/orgsidian-shell-app/src/lib.rs#L790)

**Tests & traceability (peripherals)**

- End-to-end + regression tests over a real scanned vault (in/out/resume, auto-stop, sub-minute, offset-shift, nested, CRLF).
  [`tests/clock.rs:1`](../../crates/orgsidian-core/tests/clock.rs#L1)

- FR-8 first-doc-line grep-smoke.
  [`traceability.rs:1`](../../tests/traceability.rs#L1)
