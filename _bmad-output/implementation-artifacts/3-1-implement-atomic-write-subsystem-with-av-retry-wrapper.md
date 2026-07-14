# Story 3.1: Implement atomic-write subsystem with AV-retry wrapper

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 25

## Story

As the **user saving my `.org` file**,
I want every save to use temp-file-and-rename atomic semantics with a 3-retry exponential backoff for AV/Search-indexer transient locks,
So that power loss or AV interference never corrupts the source file (NFR-15 + LD-8).

**Traces:** LD-8 (atomic-write-file crate + AV-aware retry wrapper), LD-41 rows "Disk full during atomic write" + "`.tmp` orphan files from prior crash", NFR-15 (atomic writes), architecture "Process Patterns → Error recovery" (bounded exponential backoff, max 3 attempts, base 100ms), architecture Save-cycle data flow (`orgsidian-vault::atomic::write (AV-aware retry)`).

## Scope Fence (read first)

This story is the **production-grade atomic-write subsystem** inside `crates/orgsidian-vault/`: the retry wrapper, the `VaultError` type, the orphan-temp-file cleanup API, and the two Story-3.1-owned LD-41 harness implementations. It is **not**:

- **NOT Story 3.2 (Dirty Buffer).** No `dirty_buffer.rs`, no `DirtyBufferManager`. The vault crate stays atomic-write-only after this story.
- **NOT Story 3.3+ (SQLite).** No `rusqlite`, no schema, nothing in `orgsidian-index`.
- **NOT Story 3.6 (Vault designation UI / Vault open flow).** There is no `Vault::open` or `designateVault` yet. The orphan-cleanup lands as a **public API** (`clean_orphan_temp_files`) with its own tests; Story 3.6 wires it into the actual Vault-open path. Do not invent a `Vault` struct to host it.
- **NOT the watcher writer-ID suppression** (Epic 5) — the save-cycle diagram's "watcher detects own write — suppressed" step is future turf.
- **NOT a replacement of `atomic-write-file`.** LD-8 mandates the crate. Do not hand-roll temp+rename to control temp naming (see Dev Note 3 for the AC-deviation this forces).
- **NOT sentinel turf.** Byte-untouched: `crates/orgsidian-vault/tests/anchor.rs` (Story 1.9 anchor sentinel), all parser/watcher/CLI crates, `.github/workflows/*`, `deny.toml` (unless `cargo deny` actually fails — it should not: `tracing` and `fail` are already workspace deps).
- **NOT `epics.md` / `architecture.md` / `prd.md` edits** — epics.md is the GitHub-issues sync-source (Dev Note 8 for how to record the AC-5 naming variance).

The deliverable is exactly: `src/atomic.rs` + `src/error.rs` module split with retry wrapper (AC1–AC3), fault-injection unit tests via a `FileSystem`-style trait fake (AC4), orphan cleanup API (AC5), the two LD-41 harness implementations + coverage-matrix bookkeeping (AC6), green gates with declared dependency delta (AC7), deferred-work hygiene (AC8).

## Acceptance Criteria

### AC1 — `src/atomic.rs` module owns the write path; public API stays importable at the crate root.

- `crates/orgsidian-vault/src/atomic.rs` exists and wraps the `atomic-write-file` crate (open → `write_all` → `commit`).
- `pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), VaultError>` — return type upgrades from `io::Result<()>` to `Result<(), VaultError>` per the epic AC.
- `lib.rs` re-exports so **both** existing import paths keep working: `orgsidian_vault::atomic_write` (used by `orgsidian-core/src/settings/{global,vault}.rs` and `tests/anchor.rs`) and the architecture's data-flow name `orgsidian_vault::atomic::write` is satisfied by the module path (`atomic::atomic_write` re-exported; an `atomic::write` alias is acceptable but not required).
- `tests/anchor.rs` compiles **unchanged** and stays green (`.expect(...)` works on any `Result<T, E: Debug>` — see Dev Note 1).

### AC2 — `VaultError` gives path-contextualized errors; the Story-1.9 error-path debts are closed.

- `crates/orgsidian-vault/src/error.rs` defines `VaultError` via `thiserror` (workspace `thiserror = "1"`), following the `SettingsError` precedent (`crates/orgsidian-core/src/settings/error.rs`): variants carry the offending `PathBuf` + `#[source] io::Error`.
- Minimum variants: an I/O-with-path variant and a retries-exhausted variant that records `attempts: u32` (so the caller/log can distinguish "failed once, non-transient" from "AV lock outlived 3 attempts").
- `VaultError` exposes a way for callers to recover the underlying `io::Error` (e.g. `pub fn into_io(self) -> io::Error` or a `source()` walk) — `orgsidian-core/src/settings/{global,vault}.rs` currently do `.map_err(|source| SettingsError::Io { path, source })` with `source: io::Error` and **must be updated to compile against the new signature** without changing `SettingsError`'s public shape (Dev Note 2).
- Deferred-work item closure (both owned by Story 3.1, logged 2026-05-25): (a) temp-file leak on the `write_all` error path — on any error after `AtomicWriteFile::open`, the temp file is explicitly discarded (call `discard()`; do not rely silently on `Drop`) and a test proves no temp sibling remains; (b) bare `io::Error` without context — resolved by `VaultError` carrying the path.

### AC3 — AV-aware retry with bounded exponential backoff.

- On write failure whose `io::Error` matches the transient-lock classifier — `ErrorKind::PermissionDenied`, or "Other"/uncategorized kinds whose `raw_os_error()` is a known Windows sharing/lock violation (`32` ERROR_SHARING_VIOLATION, `33` ERROR_LOCK_VIOLATION) — the wrapper retries the **whole** open→write→commit cycle.
- Backoff: base **100ms**, exponential, **max 3 attempts total** (sleeps of 100ms and 200ms between the 3 tries) per architecture "Error recovery" (architecture.md:822).
- Non-transient errors (e.g. `NotFound`, `IsADirectory`-shaped failures, ENOSPC) fail **immediately** — no retry, per LD-41 disk-full row ("surface error to user; never propagate partial-write corruption").
- The sleep is **injectable** (function pointer, closure, or one-method trait) so unit tests assert the backoff schedule deterministically without real sleeping. Production entry point `atomic_write` uses `std::thread::sleep` internally; the injectable seam is `pub(crate)` or `#[doc(hidden)]` — not public API.
- Each retry emits `tracing::warn!` with **structured fields** (path, attempt, error) per architecture Logging conventions — never string interpolation. `tracing` (workspace dep) is added to `orgsidian-vault`'s `[dependencies]`.

### AC4 — `tests/atomic.rs` injects faults via a custom `FileSystem`-style trait fake and asserts retry behavior.

- The retry loop is generic over a narrow trait (name it `FileSystem` per the epic AC text; one method along the lines of `write_atomic_once(&self, path, content) -> io::Result<()>` is sufficient) — the production impl delegates to `atomic-write-file`; the test fake returns scripted error sequences and records call timestamps/backoff requests.
- `crates/orgsidian-vault/tests/atomic.rs` covers at minimum: transient-error-then-success (asserts 2 attempts, one 100ms backoff requested); transient-errors-exhausted (asserts 3 attempts, backoffs [100ms, 200ms], returns retries-exhausted variant); non-transient error (asserts exactly 1 attempt, immediate error); happy path unchanged (byte-identical read-back, mirroring the anchor).
- No test sleeps in real time; the injected sleeper records rather than sleeps.
- `tests/anchor.rs` remains byte-untouched and green (sentinel discipline).

### AC5 — Orphan temp files from dead writers are cleaned up by a public Vault-open-ready API.

- `pub fn clean_orphan_temp_files(vault_root: &Path) -> Result<CleanupReport, VaultError>` (name/report-shape may vary; must be public, must report how many files were removed) recursively scans `vault_root` and removes orphaned `atomic-write-file` temp siblings.
- **AC-text deviation (approved at story-creation — see Dev Note 3):** the epic AC says orphans match `*.tmp.<pid>` with dead-PID detection; `atomic-write-file` 0.3.0 actually names temps `.{basename}.{6 alphanumeric chars}` (verified in crate source, `src/imp/generic.rs::RandomName`) and exposes **no** temp-naming customization, so PID-based liveness is impossible. The cleanup instead matches the crate's real pattern — a dotfile sibling of an existing-or-plausible target whose name is `.` + `{name}.org` + `.` + 6 alphanumerics — combined with an **mtime-age guard** (only delete temps older than a conservative threshold, e.g. ≥60s, so a concurrent in-flight writer is never raced). Record this deviation in Completion Notes + deferred-work per Dev Note 8.
- Only files matching the full pattern with an `.org` target stem are touched — the scan must never delete user dotfiles (test with a `.hidden.org.abc123`-style orphan vs. `.gitignore`, `.orgsidian/` dir, and a fresh in-flight temp).
- Wiring into the actual Vault-open flow is **Story 3.6's** job; this story ships and tests the API only.

### AC6 — The two Story-3.1-owned LD-41 harness placeholders become real fault-injection tests.

- `tests/failure_modes.rs` (workspace root, compiled via `orgsidian-core` `[[test]]` binding, `required-features = ["test-support"]`): `disk_full_atomic_write` and `tmp_orphan_files_cleanup` lose their `#[ignore = "implemented in Epic 3"]` + `unimplemented!` bodies and gain real assertions.
- `disk_full_atomic_write`: uses the `fail` crate fail-point (exemplar in the harness is illustrative, not binding — prefer `fail::cfg(..., "return(...)")`-style error injection over `"panic"`). This requires a fail-point in the vault write path: add `fail = { workspace = true, optional = true }` to `orgsidian-vault` behind a `failpoints` feature (mirroring how `fail` is compiled to no-ops in production builds — see root `Cargo.toml` comment on the `fail` workspace dep), and have `orgsidian-core`'s `test-support` feature activate `orgsidian-vault/failpoints`. Asserts: write fails, target file's prior content is intact, no temp sibling remains.
- `tmp_orphan_files_cleanup`: plants a crate-pattern orphan temp in a fixture dir (simulating a `kill -9` mid-write), calls `clean_orphan_temp_files`, asserts the orphan is gone and legitimate files survive.
- Coordinated bookkeeping (all three touchpoints, per `tests/failure_modes_coverage.rs` doc): `EXPECTED_REMAINING_PLACEHOLDERS` **10 → 8**; the two `#[ignore]` removals; regenerate `docs/failure-modes/coverage-matrix.md` via `pnpm gen:failure-modes-matrix`.

### AC7 — Build, test, and supply-chain gates stay green; dependency delta is declared and bounded.

- `cargo test --workspace` green (including `-p orgsidian-vault`, `-p orgsidian-core --features test-support`), `cargo deny check` + `cargo audit` green, commitlint-conformant commits.
- Dependency delta is exactly: `orgsidian-vault` gains `tracing` (workspace) + optional `fail` (workspace, behind `failpoints`) + whatever dev-deps its new tests need (`tempfile` already present). **No new workspace-level crates.** No version bumps (`atomic-write-file = "0.3"` is the latest stable — 0.3.0, verified on crates.io 2026-07-14).
- No `unwrap`/`expect`/`panic!` in committed non-test code (Story 2.8 convention).

### AC8 — Deferred-work hygiene.

- `deferred-work.md`: mark the two Story-1.9 items owned by 3.1 as closed (with the closing story/PR reference), and add any new deferrals discovered during implementation (candidates: the AC5 naming deviation follow-up on architecture LD-41 wording; EXDEV/cross-device behavior if encountered).

## Tasks / Subtasks

- [x] Task 1 — Module split + `VaultError` (AC1, AC2)
  - [x] Create `src/error.rs` with `VaultError` (thiserror, path-carrying variants + retries-exhausted with `attempts`), `into_io` escape hatch
  - [x] Create `src/atomic.rs`; move the Story-1.9 delegation there; `lib.rs` becomes module declarations + re-exports (`pub use atomic::atomic_write;` etc.) with updated crate docs
  - [x] Explicit `discard()` on the error path after `open` succeeds; test proves no temp sibling remains
  - [x] Update `orgsidian-core/src/settings/{global,vault}.rs` call sites to map `VaultError` → `SettingsError::Io` (keep `SettingsError` shape unchanged)
- [x] Task 2 — Retry wrapper (AC3)
  - [x] Transient-lock classifier (`PermissionDenied` | raw OS 32/33 on uncategorized kinds)
  - [x] Exponential backoff loop: 3 attempts total, sleeps 100ms/200ms, injectable sleeper seam
  - [x] `tracing::warn!` structured-field emission on each retry; add `tracing` dep to vault crate
- [x] Task 3 — `FileSystem` trait seam + `tests/atomic.rs` (AC4)
  - [x] Trait with production impl delegating to `atomic-write-file`; retry loop generic over it
  - [x] Fake with scripted error sequences + recording sleeper
  - [x] Four test cases: retry-then-success / exhausted / non-transient-immediate / happy-path byte-identity
- [x] Task 4 — Orphan cleanup API (AC5)
  - [x] `clean_orphan_temp_files(vault_root)` with crate-pattern match + mtime-age guard + report
  - [x] Tests: orphan removed; user dotfiles + `.orgsidian/` + fresh in-flight temp survive
- [x] Task 5 — LD-41 harness graduation (AC6)
  - [x] `failpoints` feature on vault + fail-point in write path; `test-support` on core activates it
  - [x] Implement `disk_full_atomic_write` + `tmp_orphan_files_cleanup`; remove the two `#[ignore]`s
  - [x] `EXPECTED_REMAINING_PLACEHOLDERS` 10 → 8; `pnpm gen:failure-modes-matrix`
- [x] Task 6 — Gates + hygiene (AC7, AC8)
  - [x] `cargo test --workspace`, `cargo deny check`, `cargo audit`, commitlint
  - [x] deferred-work.md closures + new entries; Completion Notes record the AC5 deviation

## Dev Notes

### 1. Anchor sentinel discipline — the signature change is legal, the anchor file is not editable

`crates/orgsidian-vault/tests/anchor.rs` (Story 1.9, anti-placebo-green per Party Mode P2) must stay **byte-untouched and green**. It calls `orgsidian_vault::atomic_write(&target, ANCHOR).expect(...)` — `.expect` compiles against any `Result<T, E: Debug>`, so the `io::Result<()>` → `Result<(), VaultError>` upgrade does NOT break it as long as `VaultError: Debug` (thiserror derives give you that via `#[derive(Debug, ...)]`). The lib.rs doc comment claiming "`io::Result<()>` is preserved across the Story 3.1 swap" is Story 1.9's forward-looking note about the *argument* surface; the epic AC for this story explicitly specifies `Result<(), VaultError>` — the epic wins. Rewrite that doc comment as part of Task 1.

### 2. LEAF graph rule + the two real callers you must not break

`orgsidian-vault` is a LEAF crate; `deny.toml` allows only `orgsidian-core` as its wrapper (`cargo deny check bans` fails on any other edge — hard CI gate, do not relitigate, see Story 2.8 Dev Notes). The only production callers today are `orgsidian-core/src/settings/global.rs:59` and `orgsidian-core/src/settings/vault.rs:61`, both shaped `atomic_write(&path, contents.as_bytes()).map_err(|source| SettingsError::Io { path, source })` where `SettingsError::Io.source` is `io::Error`. Keep `SettingsError` untouched (it's Story 1.18 turf with its own tests) and adapt the call sites: `.map_err(|e| SettingsError::Io { path, source: e.into_io() })` or equivalent. Run `-p orgsidian-core` settings tests to confirm.

### 3. Real `atomic-write-file` 0.3.0 surface (verified in vendored source at story-creation) — and the AC5 naming deviation it forces

- API: `AtomicWriteFile::open(path)` / `::options()` (`OpenOptions` has only `read(bool)` + Unix ext for permissions/ownership preservation) → implements `Write` → `commit(self)` / `discard(self)`; `Drop` best-effort-discards uncommitted temps.
- Temp naming: `.{basename}.{6 random alphanumerics}` in the **same directory** as the target (`src/imp/generic.rs::RandomName`, SUFFIX_SIZE = 6). **No PID in the name, no API to customize it.** The epic AC's `*.tmp.<pid>` + dead-PID scan is therefore unimplementable without abandoning the crate — which LD-8 forbids. Resolution chosen at story-creation: match the crate's real pattern + mtime-age guard (AC5). This is a variance to record, not a decision to re-open.
- Crate docs warn: on process abort without unwinding (`kill -9`), temps are left behind — exactly the orphan scenario AC5 cleans up.
- 0.3.0 is the latest stable (crates.io, verified 2026-07-14); workspace pin `"0.3"` is current. Known crate limitation from Story 1.9 review: `commit()` can return `EXDEV` on cross-device targets (tmpfs/bind-mount) — treat as non-transient, surface immediately; add a deferred-work entry only if you actually handle it specially.

### 4. Transient-lock classifier — be precise, not generous

The dominant real-world failure mode (LD-8) is Windows AV/Search-indexer holding a handle during the rename. Mapping: `ERROR_ACCESS_DENIED` → `ErrorKind::PermissionDenied`; `ERROR_SHARING_VIOLATION` (32) / `ERROR_LOCK_VIOLATION` (33) → uncategorized kind, so check `raw_os_error()`. On Unix, `PermissionDenied` retry is harmless and keeps the classifier platform-uniform. Do NOT retry `NotFound` (target dir gone), directory-target errors, or ENOSPC (LD-41: disk-full surfaces immediately with temp cleanup). Classifier goes in `atomic.rs` as a small pure function — trivially unit-testable with `io::Error::from_raw_os_error(32)` etc.

### 5. `fail` crate wiring pattern (follow the Story-1.11 precedent exactly)

Workspace already pins `fail = "0.5"` with the `failpoints` feature deliberately activated **at consumer level, not workspace level** (root Cargo.toml comment: production builds compile `fail` to no-ops). For the vault fail-point: `orgsidian-vault` gets `fail = { workspace = true, optional = true }` + `[features] failpoints = ["dep:fail", "fail/failpoints"]`; the fail-point macro call sits in the production write path (it compiles to nothing when the feature is off); `orgsidian-core`'s existing `test-support` feature adds `orgsidian-vault/failpoints` so the root harness (which runs `required-features = ["test-support"]`) can `fail::cfg(...)`. The harness exemplar's fail-point name `atomic-write::after-tmp-rename` and `"panic"` action are **commented-out pseudo-code** — pick a name matching where the point actually lives and prefer `return`-action error injection; keep `FailScenario::setup()` teardown-on-drop discipline.

### 6. Coverage-matrix coordinated update — three touchpoints or CI screams

`tests/failure_modes_coverage.rs` parses `failure_modes.rs` source and compares against `EXPECTED_REMAINING_PLACEHOLDERS` (currently 10). Story 3.1 removes **two** placeholders → set it to **8** and regenerate `docs/failure-modes/coverage-matrix.md` (`pnpm gen:failure-modes-matrix`, per the file's own regen header). Miss any of the three and the coverage gate fails loudly (by design).

### 7. Logging + code conventions (established, follow exactly)

- `tracing` structured fields only: `tracing::warn!(path = %path.display(), attempt, error = %err, "atomic write retry after transient lock")` — never interpolated strings (architecture Logging section). `warn` level is correct for retries (degraded behavior); a final failure is the caller's `error` to log, not the vault's.
- Doc comments encouraged on public items (vault is not `plugin-api`, so `cargo doc` cleanliness is encouraged-not-gated).
- No `unwrap`/`expect`/`panic!` in committed non-test code (Story 2.8 discipline).
- Module doc header in `atomic.rs`/`error.rs` naming the LD/FR traces (grep-smoke friendly, Story 1.17/1.18 precedent: LD-8 + NFR-15).

### 8. Variances (record in Completion Notes; do NOT edit epics.md / architecture.md / prd.md)

Two known-at-creation variances to disclose in Completion Notes (epics.md is the GitHub-issues sync-source; architecture LD-41 row wording "`*.tmp.<pid>` matching dead PID" needs an addendum, but that edit belongs to a planning-artifact pass, not this story — add a deferred-work entry pointing at it):

1. AC5 orphan pattern: crate-real `.{basename}.{6 alnum}` + mtime-age guard instead of `*.tmp.<pid>` + dead-PID scan (Dev Note 3 rationale).
2. `atomic_write` return type changes a documented Story-1.9 claim (Dev Note 1) — disclosed, epic-mandated.

### Project Structure Notes

- `crates/orgsidian-vault/src/` grows from a single `lib.rs` to `lib.rs` (decls + re-exports) + `atomic.rs` + `error.rs`. Story 3.2 adds `dirty_buffer.rs` alongside; keep `lib.rs` thin so that lands cleanly.
- New tests: `crates/orgsidian-vault/tests/atomic.rs` (crate-local, trait-fake based) — distinct from the root `tests/failure_modes.rs` harness (workspace-level, fail-point based). Both exist on purpose; don't merge them.
- `Cargo.toml` deltas: vault `[dependencies]` + `tracing`, optional `fail`, `[features] failpoints`; core `[features] test-support` grows `orgsidian-vault/failpoints`. Root workspace file: untouched (all deps already pinned there).
- Branch per repo convention: `story/3.1-atomic-write-av-retry` off `main`; conventional commits (commitlint gate); plain commit messages, no AI-credit trailers.

### Testing Standards Summary

- Unit/integration tests colocated per crate under `tests/`; deterministic — no real sleeps (injectable sleeper), no wall-clock dependence (Story 1.9 watcher-anchor hang was a lesson: never poll real time in tests).
- Fault injection: trait fake for crate-local logic (AC4); `fail` crate fail-points only for the cross-cutting LD-41 harness (AC6).
- `tempfile::TempDir` for all filesystem fixtures (workspace dev-dep precedent).
- The anchor test is the regression sentinel — if `tests/anchor.rs` needs editing, the design is wrong; stop and reconsider.

### Previous Story Intelligence (from Stories 1.9, 1.18, 2.8)

- **1.9 (shipped the stub this story replaces):** review logged four deferred items; two are owned here (temp-leak on `write_all` error path; bare `io::Error` context) — close them in deferred-work.md. The other two (watcher mtime race, anchor hang) are Story 5.1 turf — leave them.
- **1.18 (established core→vault edge):** the workspace `orgsidian-vault = { path, version }` entry already exists (with the `cargo deny` version+path pattern comment) — no root Cargo.toml work needed. The settings writers are the live consumers proving the edge.
- **2.8 (latest merged story):** stacked-branch/PR flow works (`gh pr merge --admin` for the solo-unsatisfiable review gate); Completion Notes disclose every deviation with AC references; scope-fence discipline kept the diff reviewable. Story files' Dev Notes are trusted verbatim by the dev agent — which is why the crate-source verification in Dev Note 3 was done at creation time.

### Git Intelligence Summary

Recent history (post-Epic-2 merge, main): story-branch PRs merged via admin (branch protection requires 1 review, unsatisfiable solo); `chore:` commits for cross-branch merges (commitlint forbids `merge:`); label `status:in-review` during CR. Working tree at story-creation: clean, `e14ea7c` (deferred-work L0-gate timing closure).

### Latest Technical Information

- `atomic-write-file` **0.3.0** = latest stable (crates.io, 2026-07-14); last upstream release 2025-09. No breaking changes pending; `unnamed-tmpfile` Linux feature exists but is unsuitable here (O_TMPFILE has no on-disk name → orphans invisible to cleanup, and the crate docs flag early-boot caveats) — stay on the default path.
- `fail` 0.5.1 (workspace-pinned 2026-05) — `fail_point!` macro compiles to no-op without the `failpoints` feature; `FailScenario` is not thread-parallel-safe across tests, so keep the two harness tests' fail-point names disjoint or run them serially (`#[serial]` is not in-tree; disjoint names suffice).
- `thiserror` workspace pin is `"1"` (a transitive `2.x` exists via tauri-specta — known, Story 1.4 deferred item; do not "fix" it here).

### References

- Epic AC source: `_bmad-output/planning-artifacts/epics.md` §Epic 3 → Story 3.1 (lines 892-908)
- LD-8: `_bmad-output/planning-artifacts/architecture.md:70`; retry params: architecture.md:822
- LD-41 rows: architecture.md:1196-1209 (disk-full + tmp-orphan rows); LD-41 harness: `tests/failure_modes.rs` (placeholders at `disk_full_atomic_write`, `tmp_orphan_files_cleanup`), `tests/failure_modes_coverage.rs` (`EXPECTED_REMAINING_PLACEHOLDERS`), `docs/failure-modes/coverage-matrix.md`
- Save-cycle data flow: architecture.md:1117-1131; crate layout: architecture.md:205-220
- Current stub: `crates/orgsidian-vault/src/lib.rs`; anchor sentinel: `crates/orgsidian-vault/tests/anchor.rs`
- Live callers: `crates/orgsidian-core/src/settings/global.rs:59`, `crates/orgsidian-core/src/settings/vault.rs:61`; error precedent: `crates/orgsidian-core/src/settings/error.rs`
- `fail` wiring precedent: root `Cargo.toml` (workspace dep comment), `crates/orgsidian-core/Cargo.toml:60-75` (`[[test]]` bindings + `test-support`)
- Deferred items to close: `_bmad-output/implementation-artifacts/deferred-work.md` §"code review of story-1.9"
- Crate-source verification: `~/.cargo/registry/src/*/atomic-write-file-0.3.0/src/imp/generic.rs` (RandomName), `src/lib.rs:607-660` (commit/discard/Drop)

### Project Context Reference

No `project-context.md` exists in the repo at story-creation (checked 2026-07-14). Architecture + this story file are the authoritative context.

## Dev Agent Record

### Agent Model Used

Claude Fable 5 (claude-fable-5) via Claude Code, 2026-07-14.

### Debug Log References

- RED phase: `tests/atomic.rs` failed to compile (E0432 `no VaultError in the root`) before implementation — confirms test validity.
- `cargo deny check bans` regression isolated by stash-diff: on main `fail` was dev-only (excluded via `multiple-versions-include-dev = false`); the new normal-optional edge `orgsidian-vault → fail` under `[graph] all-features = true` pulled the `rand 0.8` tree into the bans graph against `atomic-write-file`'s `rand 0.9`. Resolved via `[bans].skip` per-incident entries (rand is not an LD-37 canonical-version invariant).
- Pre-existing (on `main`) advisory failures handled: `anyhow` RUSTSEC-2026-0190 fixed by lockfile bump 1.0.102 → 1.0.103 (root + corpus-extractor lockfiles); `quick-xml` RUSTSEC-2026-0194/0195 runtime path fixed by lockfile bump `plist` 1.9.0 → 1.10.0 (quick-xml 0.41); residual quick-xml 0.39.4 (wayland-scanner proc-macro, Linux build-time only) accepted into the advisory-exceptions ledger.

### Completion Notes List

- **AC1** — `src/atomic.rs` + `src/error.rs` split landed; `lib.rs` is decls + re-exports. `atomic_write` upgraded to `Result<(), VaultError>`; `orgsidian_vault::atomic_write` and `orgsidian_vault::atomic::atomic_write` both importable (no extra `atomic::write` alias — module path satisfies the architecture name per AC1). `tests/anchor.rs` byte-untouched and green.
- **AC2** — `VaultError::{Io, RetriesExhausted}` (thiserror, path + `#[source] io::Error`; `RetriesExhausted.attempts: u32`), `into_io()` escape hatch. Both settings call sites adapted; `SettingsError` shape unchanged. Story-1.9 deferred items closed in deferred-work.md: explicit `discard()` on the post-`open` error path (proven by the harness disk-full test asserting no temp sibling) + path-contextualized errors.
- **AC3** — Classifier: `PermissionDenied` OR raw OS code 32/33 (`ERROR_SHARING_VIOLATION`/`ERROR_LOCK_VIOLATION`), platform-uniform per Dev Note 4. Backoff 100ms/200ms, max 3 attempts total; injectable sleeper (`#[doc(hidden)] pub atomic_write_with`, FnMut(Duration) seam); `tracing::warn!` with structured fields (path/attempt/error) on each retry.
- **AC4** — `tests/atomic.rs`: ScriptedFs fake + recording sleeper; 4 mandated cases (retry-then-success asserts 2 attempts + [100ms]; exhausted asserts 3 attempts + [100ms, 200ms] + `RetriesExhausted{attempts:3}`; non-transient asserts 1 attempt, no backoff; happy-path byte-identity through the real production path). Zero real sleeps. Classifier edge cases unit-tested in-module (`from_raw_os_error(32/33)`, NotFound/StorageFull/IsADirectory rejected).
- **AC5 (with approved deviation)** — `clean_orphan_temp_files(vault_root) -> Result<CleanupReport, VaultError>` (recursive; `CleanupReport.removed: Vec<PathBuf>` + `removed_count()`). **Deviation per Dev Note 3:** epic's `*.tmp.<pid>` + dead-PID scan is unimplementable with `atomic-write-file` 0.3.0 (temps are `.{basename}.{6 alnum}`, no naming API); shipped crate-real pattern match restricted to `.org` target stems + ≥60s mtime-age guard. Safety tests: `.gitignore`, `.orgsidian/` contents, non-`.org` temps, fresh in-flight temp all survive; `.hidden.org.abc123`-style orphan collected. mtime backdating via std `File::set_times` (no new dev-deps).
- **AC6** — Both harness tests real (fail-point `vault::atomic-write::write`, `return`-action ENOSPC injection; orphan fixture with backdated mtime); `EXPECTED_REMAINING_PLACEHOLDERS` 10 → 8; matrix regenerated (`pnpm gen:failure-modes-matrix` — the generator drops implemented rows by design). Fail-point names disjoint (only one fail-point exists).
- **AC7** — `cargo test --workspace` green (harness runs there too via feature unification); `-p orgsidian-core --features test-support` green; `cargo deny check` all four green; both CI-style `cargo audit --deny warnings $IGNORES` invocations green; `cargo fmt --check` + `cargo clippy` clean on touched crates; no `unwrap`/`expect`/`panic!` in non-test code. **Dependency delta beyond the AC7 list:** `thiserror` (workspace `"1"`) added to `orgsidian-vault` — mandated by AC2's "define `VaultError` via thiserror" but omitted from AC7's enumeration; zero new workspace crates. Lockfile-only bumps: `anyhow` 1.0.103 (root + extractor), `plist` 1.10.0/`quick-xml` 0.41.0 (advisory fixes, see Debug Log).
- **AC8** — deferred-work.md: two Story-1.9 closures annotated; new story-3.1 stanza (architecture LD-41 wording addendum owner: planning-artifact pass; upstream commit-failure temp-residue discovery, mitigated by AC5 cleanup).
- **Variance disclosure (Dev Note 8):** (1) AC5 orphan pattern deviation as above; (2) `atomic_write` return-type change supersedes the Story-1.9 lib.rs doc claim — doc comment rewritten, epic-mandated.
- **Supply-chain footnote (scope-fence justified):** `deny.toml`, `.cargo/audit-ignore.txt`, and `docs/security/advisory-exceptions.md` were touched because `cargo deny`/`cargo audit` actually failed (fence's explicit escape hatch): bans regression from the new vault→fail edge + two pre-existing-on-main quick-xml advisories (dated 2026-06-29). All exceptions carry rationale + 2026-10-12 review dates in the ledger.

### File List

- `crates/orgsidian-vault/src/atomic.rs` (new)
- `crates/orgsidian-vault/src/error.rs` (new)
- `crates/orgsidian-vault/src/lib.rs` (modified — decls + re-exports, doc rewrite)
- `crates/orgsidian-vault/Cargo.toml` (modified — thiserror, tracing, optional fail, `failpoints` feature)
- `crates/orgsidian-vault/tests/atomic.rs` (new)
- `crates/orgsidian-vault/tests/orphan_cleanup.rs` (new)
- `crates/orgsidian-core/Cargo.toml` (modified — `test-support` activates `orgsidian-vault/failpoints`)
- `crates/orgsidian-core/src/settings/global.rs` (modified — call-site `into_io()` mapping)
- `crates/orgsidian-core/src/settings/vault.rs` (modified — call-site `into_io()` mapping)
- `tests/failure_modes.rs` (modified — two placeholders graduated to real tests)
- `tests/failure_modes_coverage.rs` (modified — `EXPECTED_REMAINING_PLACEHOLDERS` 10 → 8)
- `docs/failure-modes/coverage-matrix.md` (regenerated)
- `deny.toml` (modified — bans skips for rand 0.8 tree + quick-xml 0.39.4; advisory ignores RUSTSEC-2026-0194/0195)
- `.cargo/audit-ignore.txt` (modified — RUSTSEC-2026-0194/0195)
- `docs/security/advisory-exceptions.md` (modified — ledger rows for the above)
- `Cargo.lock` (modified — vault dep edges; lockfile bumps anyhow 1.0.103, plist 1.10.0, quick-xml 0.41.0)
- `tools/corpus-extractor/Cargo.lock` (modified — anyhow 1.0.103)
- `_bmad-output/implementation-artifacts/deferred-work.md` (modified — closures + story-3.1 stanza)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified — status transitions)
- `_bmad-output/implementation-artifacts/3-1-implement-atomic-write-subsystem-with-av-retry-wrapper.md` (this file)

## Change Log

- 2026-07-14 — Story created (create-story workflow); status ready-for-dev; github issue #25 pre-existing via Story 1.16 sync.
- 2026-07-14 — Story implemented (dev-story workflow): atomic-write subsystem with AV-retry wrapper, VaultError, orphan cleanup API, LD-41 harness graduation (2 placeholders → real tests), supply-chain gates restored (bans skips + advisory ledger). Status → review.
