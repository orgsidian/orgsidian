# Story 1.9: Add anchor smoke tests (anti-placebo-green per Party Mode P2)

Status: review

## Metadata

github_issue: 9

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want three anchor smoke tests — one each in `crates/orgsidian-parser/tests/anchor.rs`, `crates/orgsidian-vault/tests/anchor.rs`, `crates/orgsidian-watcher/tests/anchor.rs` — that exercise a real parse + atomic-write-and-read + filesystem-event-detect code path end-to-end on every PR via the Story 1.8 CI matrix,
So that Epic 2 doesn't inherit a CI placebo where green means "compiled" rather than "real code paths exercised" (Party Mode P2 — Murat); each subsequent epic adds features on top of an anchor sentinel that fails loudly if `cargo test --workspace` ever drifts to a state where the parser, vault, or watcher crates compile-but-do-nothing.

## Acceptance Criteria

**AC1 — `crates/orgsidian-parser/tests/anchor.rs` exercises a real `orgsidian_parser::parse(&str)` call against the trivial fixture `* TODO Hello\n` and asserts success.**

- File path: `crates/orgsidian-parser/tests/anchor.rs` (NEW file — `crates/orgsidian-parser/tests/` does not exist yet; verified at [crates/orgsidian-parser/](crates/orgsidian-parser/) which contains only `Cargo.toml` + `src/lib.rs`).
- Fixture path: `crates/orgsidian-parser/tests/fixtures/anchor.org` (NEW file). Content MUST be EXACTLY `* TODO Hello\n` (5 ASCII bytes + LF — `2a 20 54 4f 44 4f 20 48 65 6c 6c 6f 0a`). Trailing newline is mandatory (the canonical org-mode line-termination convention; future round-trip tests depend on it).
- Test body MUST: (a) read the fixture from disk via `std::fs::read_to_string` using a path computed from `env!("CARGO_MANIFEST_DIR")` (so the test works from `cargo test --workspace` at the repo root AND from `cargo test -p orgsidian-parser` from anywhere); (b) call `orgsidian_parser::parse(&source)`; (c) assert the result is `Ok(_)` via `result.expect("anchor fixture must parse")`. The `expect` message MUST name "anchor fixture" verbatim so a failing CI line points readers at this story without grepping.
- The `orgsidian_parser::parse` function does NOT exist yet (verified at [crates/orgsidian-parser/src/lib.rs](crates/orgsidian-parser/src/lib.rs) which contains only the module doc-comment). Story 1.9 introduces a **minimal stub** — see AC4 for the contract — that will be REPLACED (not extended around) by the tree-sitter-org-backed implementation in Story 2.2. The anchor test MUST continue passing across that replacement; treat the stub as scaffolding, not a load-bearing API surface.
- The test MUST NOT depend on `tree-sitter`, `tree-sitter-org`, or any submodule under `crates/orgsidian-parser/grammar/` — those land in Story 2.1 (vendor submodule) + 2.2 (grammar wrapper). Bringing them in here breaks the Story 2.1 scope-fence.
- Test name: `parse_anchor_fixture_succeeds` (snake_case, descriptive — matches the §6.1 anti-placebo-green table at `_bmad-output/test-artifacts/test-design.md:444`).

**AC2 — `crates/orgsidian-vault/tests/anchor.rs` writes a 1-file `.org` payload via the real `atomic-write-file` crate and reads it back byte-identical.**

- File path: `crates/orgsidian-vault/tests/anchor.rs` (NEW file — `crates/orgsidian-vault/tests/` does not exist yet; verified at [crates/orgsidian-vault/](crates/orgsidian-vault/) which contains only `Cargo.toml` + `src/lib.rs`).
- Dependency introduction (this story owns it): add `atomic-write-file` (latest stable, MIT/Apache-2.0 dual — verify against `cargo deny --locked check all` after the add) as a `workspace.dependencies` entry in root `Cargo.toml`, alphabetically placed between `tracing` and `tauri-build` (the closest neighbors in the existing block at [Cargo.toml:38-49](Cargo.toml#L38-L49)). Add a one-line comment above the entry: `# Story 1.9: atomic file writes (LD-8) — anchor smoke + future Story 3.1 retry wrapper.`
- Wire the dep into `crates/orgsidian-vault/Cargo.toml`: add `atomic-write-file = { workspace = true }` under `[dependencies]`. Replace the existing comment `# Real deps added in Story 3.x.` with `# Real deps: atomic-write-file (Story 1.9, anchor + LD-8 foundation); rusqlite etc. land in Story 3.x.`
- Introduce a minimal public API in `crates/orgsidian-vault/src/lib.rs`: a `pub fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()>` that delegates ONE call to `atomic_write_file::write_to_file(path, content, atomic_write_file::OverwriteBehavior::AllowOverwrite)` (or the equivalent ergonomic shape of whatever the latest `atomic-write-file` API is — verify by `cargo doc -p atomic-write-file --open` after the dep add; the crate has been stable since 2023 but the constructor names occasionally shift). DO NOT add the 3-retry exponential backoff wrapper described in LD-8 / Story 3.1 — that's Story 3.1's scope. This anchor establishes the surface, not the production semantics.
- Test body MUST: (a) create a `tempfile::TempDir` (add `tempfile` as a `[dev-dependencies]` entry on `crates/orgsidian-vault/Cargo.toml` — workspace-level pin not needed for dev-only deps); (b) compute a target path `dir.path().join("anchor.org")`; (c) define `const ANCHOR: &[u8] = b"* TODO Hello\n";`; (d) call `orgsidian_vault::atomic_write(&target, ANCHOR).expect("anchor atomic_write must succeed");`; (e) read back via `std::fs::read(&target).expect("read-back must succeed")`; (f) `assert_eq!(read_back, ANCHOR, "anchor.org must be byte-identical after atomic write")`.
- The byte-identity assertion is the heart of the anchor — it proves the atomic-write code path moves bytes through the filesystem unchanged (no BOM, no line-ending munging, no truncation). A test that just writes-then-asserts-Ok would be the placebo this story exists to prevent.
- Test name: `atomic_write_anchor_roundtrips_byte_identical`.
- The test MUST NOT call any retry / backoff wrapper, and MUST NOT introduce a `dirty_buffer` module — Story 3.1 owns the retry wrapper, Story 3.2 owns the Dirty Buffer (per [sprint-status.yaml:81-82](sprint-status.yaml#L81-L82)).

**AC3 — `crates/orgsidian-watcher/tests/anchor.rs` detects one real filesystem write event within a Clock-deadline budget, using a deterministic `FakeClock`.**

- File path: `crates/orgsidian-watcher/tests/anchor.rs` (NEW file — `crates/orgsidian-watcher/tests/` does not exist yet; verified at [crates/orgsidian-watcher/](crates/orgsidian-watcher/) which contains only `Cargo.toml` + `src/lib.rs`).
- This story introduces the **`Clock` trait + `FakeClock` impl** described in architecture LD-9 ("Watcher abstraction layer in `core` allows deterministic fakes for unit tests"). Per the architecture-line-71 placement note ("in `core`"), the trait lives in `crates/orgsidian-core/src/test_support/clock.rs`. Create that new module:
  - New file: `crates/orgsidian-core/src/test_support/mod.rs` containing `pub mod clock;` (this is the first `test_support` module in `orgsidian-core`; Story 1.12 will add `pub mod perf;` alongside it later).
  - New file: `crates/orgsidian-core/src/test_support/clock.rs` with the trait + fake. Trait shape:
    ```rust
    pub trait Clock: Send + Sync + 'static {
        fn now(&self) -> std::time::Instant;
    }
    ```
  - `FakeClock` impl: holds an `Arc<Mutex<Instant>>` (or `Arc<AtomicU64>` of nanos since a fixed epoch — either is fine; pick the simpler `Mutex<Instant>` for the anchor stub since contention isn't a concern at v0.1). Provides `FakeClock::new() -> Self` (starts at `Instant::now()` once at construction), `fn advance(&self, dur: Duration)` (mutates the held instant), and the `Clock` impl returning the held instant.
  - The trait/fake MUST be exposed under a `cfg(any(test, feature = "test-support"))` gate on `crates/orgsidian-core/src/lib.rs` — add `#[cfg(any(test, feature = "test-support"))] pub mod test_support;` (NEW line after the existing `pub mod registry;` from Story 1.8). Then add the feature declaration to `crates/orgsidian-core/Cargo.toml`: a new `[features]` section with `test-support = []`. This pattern lets consumer crates (the vault/watcher tests, plus Story 1.12's perf macro consumers) opt into the test_support surface via their `[dev-dependencies] orgsidian-core = { workspace = true, features = ["test-support"] }` declaration without exposing the surface in `--release` builds.
- Add `orgsidian-core` as a `[dev-dependencies]` entry on `crates/orgsidian-watcher/Cargo.toml`: `orgsidian-core = { workspace = true, features = ["test-support"] }`. Add `tempfile` as a `[dev-dependencies]` entry too.
- Introduce a minimal public API in `crates/orgsidian-watcher/src/lib.rs`: a `pub fn detect_first_write_event(path: &std::path::Path, clock: &dyn Clock, deadline: std::time::Duration) -> Result<DetectedEvent, DetectError>` that POLLS the file's `metadata().modified()` mtime in a loop (sleep 10ms between polls via `std::thread::sleep`) and returns `Ok(DetectedEvent { mtime })` on the first detected mtime change vs. the initial reading, or `Err(DetectError::Timeout)` if `clock.now()` advances past `start + deadline` first. **CRITICAL design choice — read carefully:**
  - The 10ms `thread::sleep` is wall-clock, NOT clock-driven. This is intentional: the OS scheduler controls the poll cadence, the Clock controls the timeout decision. This split lets the test inject a `FakeClock` that "advances past deadline" without waiting for real seconds. Story 5.1 replaces the polling implementation with `notify-rs` event subscription (`watcher.watch(path, RecursiveMode::NonRecursive)`); the `Clock`-driven timeout discipline survives the swap unchanged.
  - The function MUST NOT pull in `notify-rs` — Story 5.1 owns that dep + the debounce layer. Bringing it in here breaks the Story 5.1 scope-fence and conflates "watcher exists" with "watcher has the production debounce calibration."
  - `DetectedEvent` is a `#[derive(Debug)] pub struct DetectedEvent { pub mtime: std::time::SystemTime }`. `DetectError` is a `#[derive(Debug, thiserror::Error)] pub enum DetectError { #[error("watcher timeout after {0:?}")] Timeout(std::time::Duration), #[error(transparent)] Io(#[from] std::io::Error) }`. Add `thiserror = { workspace = true }` to `crates/orgsidian-watcher/Cargo.toml` `[dependencies]` (it's already a workspace dep from Story 1.4; no root-level edit needed).
- Test body MUST: (a) create a `TempDir`; (b) create an initial file `target = dir.path().join("watched.org")` and write `b"initial\n"` via `std::fs::write`; (c) capture `initial_mtime = std::fs::metadata(&target)?.modified()?`; (d) spawn a `std::thread` that sleeps 50ms then writes `b"changed\n"` to the same path via `std::fs::write` (deliberate mtime bump); (e) construct a `FakeClock::new()`; (f) BEFORE calling the detector, advance the fake clock by `Duration::from_secs(0)` (no-op — proves the API is wired); (g) call `orgsidian_watcher::detect_first_write_event(&target, &clock, Duration::from_secs(5))`; (h) assert the result is `Ok(event)` and `event.mtime > initial_mtime`; (i) join the spawned thread.
- The wall-clock budget of the test is bounded by the 5-second hard timeout (matches the AC source line `epics.md:564`); on a healthy macOS-arm64 / Ubuntu-24.04 runner the detection should complete within ~100ms, well under the budget. The Clock trait is exercised structurally (the detector consumes `&dyn Clock`), even though the fake's advancement is not what triggers the timeout in this happy-path anchor — that's fine, the anchor proves the wiring works; Story 5.x failure-path tests will use `FakeClock::advance` to deterministically trigger `DetectError::Timeout` without 5-second wall-clock sleeps.
- Test name: `watcher_detects_first_write_within_clock_budget`.

**AC4 — `crates/orgsidian-parser/src/lib.rs` exposes a minimal `parse(&str) -> Result<ParseTree, ParseError>` stub satisfying AC1.**

- The stub MUST: validate that the input is valid UTF-8 (`str` already guarantees this — the check is structural to remind a reader that future tree-sitter-org input MUST be UTF-8), confirm the source is non-empty, and return `Ok(ParseTree { _private: () })` for any non-empty input. Empty input returns `Err(ParseError::Empty)`.
- `pub struct ParseTree { _private: () }` — the unit-content marker `_private: ()` keeps the struct constructible only inside the parser crate (no external `ParseTree { }` construction), which is the standard Rust "sealed type" pattern. Story 2.2 fills in the real fields (`headlines: Vec<Headline>`, etc.) and removes the marker.
- `pub enum ParseError { #[error("empty source")] Empty }` — derives `Debug` + `thiserror::Error`. Add `thiserror = { workspace = true }` to `crates/orgsidian-parser/Cargo.toml` `[dependencies]`.
- Module doc-comment on `crates/orgsidian-parser/src/lib.rs` MUST keep the existing `Implements FR-1` placeholder language but append a paragraph naming Story 1.9: `//! Story 1.9 ships the anchor-smoke surface only — `parse()` is a stub that returns Ok for any non-empty UTF-8 source. Story 2.2 wires the real tree-sitter-org grammar and replaces this body; the public signature `parse(&str) -> Result<ParseTree, ParseError>` is preserved across that replacement (anchor sentinel discipline).`
- DO NOT add `tree-sitter`, `tree-sitter-org`, `rowan`, or any other parser dependency in this story. The crate STILL lists only the empty `[dependencies]` placeholder (with `thiserror` newly added) after Story 1.9 closes.

**AC5 — `cargo test --workspace --locked` exercises all three anchor tests on per-PR CI and on the dev box.**

- The Story 1.8 `pr.yml` step 9 (`cargo test --workspace --locked` — verified at [.github/workflows/pr.yml](.github/workflows/pr.yml), the step block introduced by Story 1.8) ALREADY discovers `crates/*/tests/*.rs` integration tests by Cargo convention. NO `pr.yml` edit is required for the anchors to run — adding the test files is sufficient.
- The Story 1.8 `nightly.yml` mirrors the per-PR steps; the anchors run there too by the same convention. NO `nightly.yml` edit required.
- A regression where any of the three anchor tests starts compiling but stops exercising the production function (e.g., a future PR rewrites `parse()` as `fn parse(_: &str) -> Result<ParseTree, ParseError> { Ok(ParseTree { _private: () }) }` with no validation) MUST surface during code review. Story 1.9 does NOT ship a meta-test enforcing this (would require AST introspection of the function body — out of scope for the anchor layer); instead, AC10's anti-creep scope-fence + the `_bmad-output/test-artifacts/test-design.md` §6.1 "Anchor fixtures are deliberately minimal and stable — should not change after Epic 1 closes" rule (line 430) IS the protection. Document this contract in the module doc-comment on `crates/orgsidian-parser/src/lib.rs` (AC4 above) and the equivalent lines on vault/watcher.

**AC6 — Dev-box verification matrix.**

The following MUST all succeed on a clean checkout of Story 1.9's HEAD before the story moves to `review`:

| Command | Expected | Run on |
|---|---|---|
| `cargo fmt --all -- --check` | exit 0 | macOS-arm64 (dev) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | macOS-arm64 (dev) |
| `cargo build --workspace --locked` | exit 0 | macOS-arm64 (dev) |
| `cargo test --workspace --locked` | exit 0; the 3 new anchor tests + Story 1.8's 4 registry tests all run + pass | macOS-arm64 (dev) |
| `cargo test -p orgsidian-parser --test anchor --locked` | exit 0; runs the single parser anchor test | macOS-arm64 (dev) |
| `cargo test -p orgsidian-vault --test anchor --locked` | exit 0; runs the single vault anchor test | macOS-arm64 (dev) |
| `cargo test -p orgsidian-watcher --test anchor --locked` | exit 0; runs the single watcher anchor test | macOS-arm64 (dev) |
| `cargo deny --locked check all` | exit 0 (Story 1.7 baseline must stay clean post `atomic-write-file` + `tempfile` dev-dep adds) | macOS-arm64 (dev) |
| `cargo audit --deny warnings --ignore RUSTSEC-2024-0429` | exit 0 (the existing Story 1.8 `--ignore` flag suffices; do not add new ignores for `atomic-write-file` / `tempfile` unless RUSTSEC actually fires) | macOS-arm64 (dev) |
| `cargo build --release --workspace --locked` | exit 0 (verifies the `test-support` feature gate keeps `clock.rs` out of the release build) | macOS-arm64 (dev) |
| `cargo doc --workspace --no-deps --locked` | exit 0 with zero warnings on `orgsidian-plugin-api` (clippy::pedantic policy per architecture line 839); the anchor module docs MUST not introduce broken intra-doc links | macOS-arm64 (dev) |
| First push of `.github/workflows/pr.yml` greenness on this branch | green on `macos-14` + `ubuntu-24.04`; the new anchor tests run as part of step 9 | GitHub Actions |

If any cell fails on the dev box, the story MUST NOT move to `review`. License-check fallout (e.g., `atomic-write-file`'s license expression mismatching the LD-37 allowlist) is the most likely surprise — handle by adding a documented exception to `docs/security/advisory-exceptions.md` "License exceptions" section (the Story 1.7 ledger) AND updating `scripts/check-allowlist-sync.mjs` per the Story 1.8 AC6 sync contract; do NOT silently expand `deny.toml [licenses].allow`.

**AC7 — Anti-creep scope-fence.**

The following are NOT modified by Story 1.9 (out of scope; flag any drift as a review-block):

- `.github/workflows/*` — Story 1.9 adds zero workflow steps. CI discovery of `tests/anchor.rs` happens via Cargo's existing convention (verified by AC6 row "First push").
- `crates/orgsidian-core/src/registry.rs` — Story 1.8 territory. Story 1.9 only ADDS the sibling `test_support` module; do not touch the registry.
- `crates/orgsidian-{index,plugin-api,report,cli,shell-app}/**/*` — out of scope. Story 1.9 touches `orgsidian-parser`, `orgsidian-vault`, `orgsidian-watcher`, and `orgsidian-core` (the last only to add the `test_support` module surface).
- `shell-ui/**/*` — frontend untouched.
- `crates/orgsidian-parser/grammar/` — Story 2.1 territory (tree-sitter-org SHA-pinned submodule). Story 1.9 does NOT create this directory.
- `crates/orgsidian-vault/src/dirty_buffer.rs` — Story 3.2 territory. Anchor does NOT introduce dirty-buffer scaffolding.
- `crates/orgsidian-watcher/src/debounce.rs` — Story 5.1 territory.
- `tests/fixtures/vault-corpus/` — Epic 2 territory. Story 1.9 fixture lives under `crates/orgsidian-parser/tests/fixtures/anchor.org` only.
- `Cargo.lock` — committed and updated automatically by the new dep adds. Verify the diff contains ONLY the `atomic-write-file` + `tempfile` (+ their transitive closures) lines; any unrelated bumps require a separate audit. The Story 1.7 ledger discipline (`docs/security/advisory-exceptions.md`) governs how unexpected transitive surprises are documented.
- `commitlint.config.cjs`, `.husky/**/*` — Story 1.14 territory.

## Tasks / Subtasks

- [x] Task 1 — Introduce the `Clock` trait + `FakeClock` in `orgsidian-core` (AC3 sub-bullet)
  - [x] 1.1 Create `crates/orgsidian-core/src/test_support/mod.rs` declaring `pub mod clock;`
  - [x] 1.2 Create `crates/orgsidian-core/src/test_support/clock.rs` with the `Clock` trait + `FakeClock` impl per the AC3 contract (Send + Sync + 'static; `new()` + `advance()` + `now()` returning a stable `Instant`)
  - [x] 1.3 Update `crates/orgsidian-core/src/lib.rs`: append `#[cfg(any(test, feature = "test-support"))] pub mod test_support;` after the existing `pub mod registry;` declaration from Story 1.8
  - [x] 1.4 Update `crates/orgsidian-core/Cargo.toml`: append `[features]\ntest-support = []`
  - [x] 1.5 `cargo test -p orgsidian-core --features test-support --locked` greens
  - [x] 1.6 `cargo build --release --workspace --locked` confirms the gate keeps `test_support` out of the release build

- [x] Task 2 — Parser anchor (AC1 + AC4)
  - [x] 2.1 Add `thiserror = { workspace = true }` to `crates/orgsidian-parser/Cargo.toml` `[dependencies]`
  - [x] 2.2 Replace the existing `crates/orgsidian-parser/src/lib.rs` body with the AC4 stub: `parse(&str) -> Result<ParseTree, ParseError>` + `ParseTree { _private: () }` + `ParseError::Empty`. Preserve the existing FR-1 doc-comment header; append the Story 1.9 anchor-sentinel paragraph per AC4.
  - [x] 2.3 Create `crates/orgsidian-parser/tests/fixtures/anchor.org` with EXACTLY 13 bytes: `* TODO Hello\n`
  - [x] 2.4 Create `crates/orgsidian-parser/tests/anchor.rs` per AC1: `parse_anchor_fixture_succeeds` test reading via `env!("CARGO_MANIFEST_DIR")` + calling `orgsidian_parser::parse` + `expect("anchor fixture must parse")`
  - [x] 2.5 `cargo test -p orgsidian-parser --locked` greens

- [x] Task 3 — Vault anchor (AC2)
  - [x] 3.1 Add `atomic-write-file = "0.3"` to root `Cargo.toml` `[workspace.dependencies]`, alphabetically between `tracing` and `tauri-build`. Add the Story 1.9 comment per AC2. (Latest stable resolved via `cargo search` at implementation time = `0.3.0`.)
  - [x] 3.2 Add `atomic-write-file = { workspace = true }` to `crates/orgsidian-vault/Cargo.toml` `[dependencies]`. Replace the placeholder comment per AC2.
  - [x] 3.3 Add `tempfile = "3"` to `crates/orgsidian-vault/Cargo.toml` `[dev-dependencies]` (no workspace pin; latest stable at implementation time = `3.27.0`).
  - [x] 3.4 Implement `pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()>` in `crates/orgsidian-vault/src/lib.rs` per AC2. Single delegation chain: `AtomicWriteFile::open(path)? + write_all + commit` — the 0.3 API replaces the older `write_to_file(path, content, OverwriteBehavior)` free-function with the `AtomicWriteFile` handle pattern; the ergonomic shape is preserved (single write path; overwrite is the default).
  - [x] 3.5 Create `crates/orgsidian-vault/tests/anchor.rs` per AC2: `atomic_write_anchor_roundtrips_byte_identical` test using `TempDir` + `b"* TODO Hello\n"` + read-back equality assertion
  - [x] 3.6 `cargo deny --locked check all` greens — required adding `nix@0.30.1` skip (transitive surprise: `atomic-write-file 0.3` pins `nix 0.30`; `tauri-plugin-os` pins `nix 0.31`). Skip added to `deny.toml` `[bans].skip` AND paired with row in `docs/security/advisory-exceptions.md` per Story 1.7 ledger discipline. License (BSD-3-Clause) is already on the LD-37 allowlist — no license-exception needed.
  - [x] 3.7 `cargo test -p orgsidian-vault --locked` greens

- [x] Task 4 — Watcher anchor (AC3)
  - [x] 4.1 Add `thiserror = { workspace = true }` to `crates/orgsidian-watcher/Cargo.toml` `[dependencies]`
  - [x] 4.2 Add to `crates/orgsidian-watcher/Cargo.toml` `[dev-dependencies]`: `orgsidian-core = { workspace = true, features = ["test-support"] }` and `tempfile = "3"`
  - [x] 4.3 Implement in `crates/orgsidian-watcher/src/lib.rs`: a local production-visible `Clock` trait facade (identical shape to `orgsidian_core::test_support::clock::Clock`; declared at the watcher level because the core trait is gated behind `cfg(any(test, feature = "test-support"))` and is therefore unavailable in release builds), the `DetectedEvent` + `DetectError` types, and `pub fn detect_first_write_event(path: &Path, clock: &dyn Clock, deadline: Duration) -> Result<DetectedEvent, DetectError>` per the AC3 polling-loop contract. The test crosses the facade via a tiny `ClockAdapter` newtype that bridges `orgsidian_core`'s `FakeClock` into the watcher's `Clock` (kept inside the test file, not the public surface).
  - [x] 4.4 Create `crates/orgsidian-watcher/tests/anchor.rs` per AC3: `watcher_detects_first_write_within_clock_budget` test spawning a thread that bumps mtime 50ms in + asserting `event.mtime > initial_mtime`
  - [x] 4.5 `cargo test -p orgsidian-watcher --locked` greens; the test completes in ~60ms on dev box, well under the 5s budget

- [x] Task 5 — Workspace-level verification (AC5 + AC6)
  - [x] 5.1 Ran every cell in the AC6 verification matrix on macOS-arm64 dev box (all exit 0; see Completion Notes)
  - [ ] 5.2 Push branch + open PR; verify `pr.yml` runs greenly on `macos-14` + `ubuntu-24.04`; the 3 anchor tests appear in the test-run output of step 9 *(deferred to PR-open step in bmad-dev-story step 10)*
  - [x] 5.3 Inspected the `cargo test --workspace --locked` output: the 3 anchor `tests/anchor.rs` binaries each print `running 1 test` and report `1 passed; 0 failed`. Story 1.8's 5 registry tests + 2 new clock tests all run too (7 unit tests in `orgsidian-core`). NO "0 tests" outcomes — anti-placebo invariant holds.
  - [x] 5.4 `atomic-write-file` license is BSD-3-Clause (already on LD-37 allowlist) — no license exception needed. Transitive `nix@0.30` duplicate handled via `[bans].skip` + paired ledger row instead.

- [x] Task 6 — Scope-fence audit (AC7)
  - [x] 6.1 `git status` confirms the "in scope" file set per AC7. Additional defensible touches: `deny.toml` + `docs/security/advisory-exceptions.md` (Story 1.7 ledger discipline for the `nix@0.30` transitive surprise, explicitly anticipated by AC2 + AC6); `_bmad-output/implementation-artifacts/sprint-status.yaml` + the story file itself (workflow-required artifacts).
  - [x] 6.2 Out-of-scope deviations: NONE beyond the explicitly anticipated ledger pair. No additions to `deferred-work.md`.
  - [x] 6.3 Confirmed: no `.github/workflows/*` files are touched — the anchors run via the existing Story 1.8 step 9 by Cargo's `tests/*.rs` discovery convention.

## Dev Notes

### §1 — Why "anti-placebo-green" is the whole point of this story

Test-design `_bmad-output/test-artifacts/test-design.md` §6.1 ("Layer 1 — Anchor Smoke", line 438) calls this layer out explicitly: **the failure mode this story protects against is a CI scaffold that says "green" because it compiled, not because it exercised a real code path.** This is Murat's P2 Party Mode round-2 concern (`test-design.md:252` R-024). Three minimal real-code-path tests on parser+vault+watcher mean a future PR that accidentally renders the parser/vault/watcher crates inert (empty fns, `unimplemented!()`, etc.) breaks the anchor and surfaces the issue immediately rather than at the next downstream story.

The anchors are deliberately MINIMAL — `* TODO Hello\n` for parser, one round-trip-write for vault, one detected mtime change for watcher. Per `test-design.md:430`: "Anchor fixtures are deliberately minimal and stable — should not change after Epic 1 closes." If the parser anchor needs to grow to cover more constructs, that's Story 2.3 (semantic-layer per-construct fixtures) territory, NOT Story 1.9.

### §2 — Why introduce `atomic-write-file` here, not Story 3.1

`atomic-write-file` is a tiny crate (<500 LOC, MIT/Apache-2.0, stable since 2023). Bringing it in for the vault anchor smoke serves three purposes simultaneously: (a) the anchor exercises a real fs write code path through a real third-party crate (anti-placebo), (b) the dep is on the supply-chain ledger from day 1 so Story 1.7's `cargo deny check all` exercises it on every PR, (c) Story 3.1's "implement-atomic-write-subsystem-with-av-retry-wrapper" body is reduced to "wrap the existing `atomic_write` in 3-retry exponential backoff" rather than "introduce the lib + write the wrapper" — clean scope split.

The alternative — hand-write a temp-file-rename-using-stdlib-only anchor — was considered and rejected because it would (a) duplicate `atomic-write-file`'s platform handling (`MoveFileExW` on Windows vs `renameat` on Linux; LD-8 explicitly names the cross-platform dance as why the lib exists), (b) make the anchor placebo-ish (it would test our temp-file dance, not the production lib that ships in v0.1), (c) require throwing away the stub when Story 3.1 lands.

### §3 — Why introduce the `Clock` trait in `orgsidian-core`, not `orgsidian-watcher`

Architecture LD-9 (`architecture.md:71`) says: "Watcher abstraction layer **in `core`** allows deterministic fakes for unit tests." The `core` placement is binding. Two downstream stories already plan to consume this trait: Story 1.12 (perf macro's deterministic-time test infrastructure) and Story 5.1 (notify-rs wrapper's timeout discipline). Placing the trait in `orgsidian-core::test_support::clock` makes both consumers reach it via the existing `orgsidian-core = { workspace = true, features = ["test-support"] }` `[dev-dependencies]` pattern (Story 1.12 will add `pub mod perf;` to the same `test_support/` directory). Placing it in `orgsidian-watcher` would force every clock consumer to dev-depend on the watcher crate — wrong direction in the LEAF-graph dependency rule (per LD-37: leaves don't depend on each other; consumers reach leaves via `orgsidian-core`).

### §4 — Why the `cfg(any(test, feature = "test-support"))` gate

`test_support` MUST NOT compile into release binaries — exposing `FakeClock` in production would let any plugin call it via the macro surface (LD-38 surface area) and trivially break timing invariants. The standard Rust pattern is the dual gate: `cfg(test)` covers internal `cargo test` discovery (tests inside `orgsidian-core` can use the module without enabling the feature), `feature = "test-support"` covers external consumers (the watcher tests dev-depend on core with the feature enabled). The features-section addition is the minimal Cargo change needed.

Story 1.12 will add an identical `pub mod perf;` declaration under the same gate — this is the LD-32 "shape-now-fill-in-later" discipline established by Story 1.8 (CI workflow comment placeholders for stories that fill in their step body when they land).

### §5 — Polling vs notify-rs in the watcher anchor

The watcher anchor uses a polling-based mtime check (10ms sleep loop) rather than `notify-rs` event subscription. Reasoning:

1. **Story 5.1 scope-fence:** Story 5.1 ("implement-notify-rs-filesystem-watcher-with-debounce") owns the notify-rs dep introduction AND the debounce calibration against the vim/VS Code/Emacs golden traces (OD-3). Bringing in notify-rs here would force Story 5.1 to either (a) treat the dep as already-present and only write the debounce layer (weakens 5.1's scope clarity), or (b) replace a fresh dep with a different version (workspace dep churn for no benefit).
2. **Anchor minimality:** The anchor's job is to prove "the watcher crate exercises a real fs-event detection code path." A 10ms poll loop exercises `std::fs::metadata` (real syscall) + `SystemTime` comparison (real timing logic) + the `Clock` trait abstraction (real abstraction layer). That's sufficient for the anti-placebo-green property. The production debounce semantics aren't anchor-test territory.
3. **API survival across the Story 5.1 swap:** `detect_first_write_event(path, clock, deadline) -> Result<DetectedEvent, DetectError>` is the function shape Story 5.1 will keep as a thin wrapper over the notify-rs event stream. The anchor test continues passing after Story 5.1 swaps the body. This is the anchor-sentinel discipline: the API is the load-bearing surface, the implementation is replaceable.

### §6 — Previous-story intelligence (Story 1.8)

Story 1.8 (now `done`) established:
- `crates/orgsidian-core/src/registry.rs` + the `invoke_plugin_hook!` macro stub — Story 1.9 does NOT touch this. The `test_support` module added by 1.9 lives alongside it (sibling module on `lib.rs`).
- `[profile.release] panic = "unwind"` on the root `Cargo.toml` — already in place; Story 1.9's `cargo build --release` smoke (AC6) verifies it still works after the dep adds.
- `tracing = "0.1"` as a `workspace.dependencies` entry — Story 1.9 does NOT need `tracing` (the anchors use `expect` / `assert_eq!` for diagnostics, not structured logging).
- The dual-gated `pr.yml` + `nightly.yml` workflows + the `merge-gate-nightly-fresh` step in `pr.yml` — Story 1.9 inherits these unchanged. NO workflow file is edited.
- Cargo workspace deps pinned per [[feedback_version_policy]] (LTS preferred; Tauri ecosystem exempted). For `atomic-write-file` and `tempfile`: pick latest stable at implementation time; both have been stable for years (no Tauri-ecosystem caveats).
- Story 1.8 deferred-work entry (in `_bmad-output/implementation-artifacts/deferred-work.md`): "`invoke_plugin_hook!` macro is sync-only" — NOT relevant to Story 1.9; flagged for Epic 4+ async hooks.

### §7 — Git-history intelligence (last 5 commits)

```
a874b91 Merge PR #118 (Story 1.8)
e160b2a fix(ci): bump cargo-audit 0.21 -> 0.22 for CVSS v4.0 advisory parsing
0ebe179 fix(story-1-8): apply 13 code-review patches
1d5a83f feat(ci): per-PR + nightly matrix, panic=unwind, invoke_plugin_hook! macro (Story 1.8)
ca95c4c Merge PR #117 (Story 1.7)
```

Patterns to absorb:
- **Commit message convention:** Conventional Commits (`feat(ci)`, `fix(ci)`, `fix(story-1-N)`). Story 1.9's commits should follow `feat(test): add Story 1.9 anchor smoke tests (parser/vault/watcher, anti-placebo)` or similar; review-cycle fixups: `fix(story-1-9): <description>`.
- **PR scope:** one PR per story; commit on a feature branch named `feat/story-1-9-anchor-smoke-tests`.
- **No co-author trailers** per [[feedback_no_co_author_credit]] memory.
- **Code-review fixup pattern:** Story 1.7 took 1 fixup commit; Story 1.8 took 13 patches (denser story). Story 1.9 is dense in spec but the surface area is small; expect 0-3 fixup commits.

### §8 — Architecture decision references (LD anchors)

Critical LD references this story implements:
- **LD-8** ([architecture.md:70](_bmad-output/planning-artifacts/architecture.md#L70)) — Atomic writes via `atomic-write-file` crate. Story 1.9 introduces the dep + minimal wrapper; Story 3.1 adds the 3-retry AV-aware backoff.
- **LD-9** ([architecture.md:71](_bmad-output/planning-artifacts/architecture.md#L71)) — `notify-rs` watcher + abstraction layer in `core` with deterministic fakes. Story 1.9 ships the abstraction (the `Clock` trait + `FakeClock` fake + the `detect_first_write_event` API shape); Story 5.1 swaps the polling body for notify-rs and adds debounce + golden-trace replay.
- **LD-32** ([architecture.md:521](_bmad-output/planning-artifacts/architecture.md#L521)) — CI matrix. Story 1.9 inherits the Story 1.8 workflows unchanged; the anchors run via the existing `cargo test --workspace --locked` step.
- **LD-37** ([architecture.md:1163](_bmad-output/planning-artifacts/architecture.md#L1163)) — Supply-chain hygiene. Story 1.9's new deps (`atomic-write-file` + `tempfile`) run through `cargo deny check all` on every PR via Story 1.7 + Story 1.8 wiring.

Critical test-design references:
- **§6.1** ([test-design.md:438](_bmad-output/test-artifacts/test-design.md#L438)) — Anchor Smoke layer definition. Story 1.9 IS this layer; nothing else implements it.
- **§7.3 catalogue** ([test-design.md:705](_bmad-output/test-artifacts/test-design.md#L705)) — Red-phase scaffold templates per story type. Story 1.9 doesn't quite fit any single template — it's a multi-crate scaffold layer. The closest is §7.3.4 (Watcher) for the Clock + FakeClock pattern.
- **Coverage matrix** ([test-design.md:444-446](_bmad-output/test-artifacts/test-design.md#L444-L446)) — names the three anchor test locations + assertions exactly as captured in AC1/AC2/AC3 above.

### §9 — Cross-platform sanity check

The dev box is macOS-arm64. The CI matrix adds Ubuntu-24.04 + (nightly) Windows-2022 + Arch Linux. Platform-specific concerns for the three anchors:

- **Parser anchor:** Pure-Rust string handling. No platform sensitivity. Runs identically everywhere.
- **Vault anchor:** `atomic-write-file` handles the `MoveFileExW`-on-Windows vs `renameat`-on-Linux dance internally. The anchor test creates a `TempDir` (cross-platform via `tempfile` crate), writes, reads back. ZERO platform-specific code in our test. Trust the lib.
- **Watcher anchor:** `std::fs::metadata().modified()` is cross-platform but mtime resolution differs (HFS+/APFS: 1 ns; ext4: 1 ns; NTFS: 100 ns; FAT32: 2 s). Our spawned-thread waits 50ms before re-writing — comfortably above all platform mtime resolutions. **Caveat on Windows:** `SystemTime` comparison MAY surprise if the test runs on a FAT-formatted temp dir (almost never the case on a dev / GitHub Actions runner — both use NTFS). If the watcher anchor flakes on Windows nightly, the diagnosis is mtime resolution; the fix is to bump the spawned-thread wait to 200ms. Document this if it surfaces.

### §10 — LLM-dev-agent anti-pattern checklist

Common dev-agent mistakes this story spec intentionally guards against:

1. **DO NOT bring in tree-sitter / tree-sitter-org in the parser stub.** Story 2.1 (submodule) + Story 2.2 (grammar wrapper) own that. Adding the dep here breaks scope and forces Story 2.1 to deal with a pre-existing inconsistent state.
2. **DO NOT bring in notify-rs in the watcher stub.** Story 5.1 owns that. The polling-based mtime detector is intentional, not a placeholder for "TODO: replace with notify".
3. **DO NOT add the 3-retry AV-aware backoff to `orgsidian_vault::atomic_write`.** Story 3.1 owns that. The Story 1.9 surface is the unwrapped delegation to `atomic-write-file`; Story 3.1's body is "wrap the existing call in a retry loop."
4. **DO NOT add a Dirty Buffer module to `orgsidian-vault`.** Story 3.2 owns that.
5. **DO NOT edit `pr.yml` or `nightly.yml`.** Cargo's `tests/*.rs` convention picks up the anchors automatically via the existing `cargo test --workspace --locked` step.
6. **DO NOT publish the `test_support` module unconditionally.** It MUST be gated behind `cfg(any(test, feature = "test-support"))`. AC6's `cargo build --release` cell verifies the gate.
7. **DO NOT add `tempfile` to `workspace.dependencies`** — it's a dev-only dep; per-crate `[dev-dependencies]` is the correct shape (Story 1.8 established the workspace-dep pattern for ship-with-the-binary deps only).
8. **DO NOT silently expand `deny.toml [licenses].allow`** if `atomic-write-file`'s license isn't pre-approved — follow the Story 1.7 ledger discipline AND update `scripts/check-allowlist-sync.mjs` per the Story 1.8 AC6 sync contract.
9. **DO NOT use `tokio::time::sleep` in the watcher anchor.** The crate has no async runtime; `std::thread::sleep` is correct.
10. **DO NOT add `#[ignore]` on any anchor test.** Anchors are non-ignorable by design — that's the point.

### §11 — Memory-anchored conventions (cross-cutting)

- **[[feedback_version_policy]]:** New deps pinned to latest stable (Tauri ecosystem exempt; none of Story 1.9's deps are Tauri). `atomic-write-file` + `tempfile` are stable utility crates; pick latest at `cargo search` time.
- **[[feedback_no_co_author_credit]]:** No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on any commit / PR / issue.
- **[[feedback_batch_fixes_terse]]:** In post-review fixups, apply no-brainer reviewer fixes silently; surface only decision-grade questions.
- **[[user_contact_email]]:** Authorship attribution uses `tiz.basile@gmail.com`. Already pinned in `Cargo.toml [workspace.package].authors`; no per-story edit.

### Project Structure Notes

- Alignment with unified project structure: the new files match the existing `crates/orgsidian-{parser,vault,watcher,core}/{src,tests}/...` layout established by Story 1.2 (workspace refactor). No new crates, no new top-level directories.
- Detected conflicts or variances: none. The `test_support` directory under `crates/orgsidian-core/src/` is new but follows the standard Rust submodule pattern (sibling to `registry.rs`).

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L552-L565](_bmad-output/planning-artifacts/epics.md#L552-L565) (Story 1.9 AC verbatim)
- Test design layer: [_bmad-output/test-artifacts/test-design.md#L438-L449](_bmad-output/test-artifacts/test-design.md#L438-L449) (§6.1 Anchor Smoke)
- Test design fixture rule: [_bmad-output/test-artifacts/test-design.md#L430](_bmad-output/test-artifacts/test-design.md#L430) ("Anchor fixtures are deliberately minimal and stable")
- Test design risk: [_bmad-output/test-artifacts/test-design.md#L252](_bmad-output/test-artifacts/test-design.md#L252) (R-024 placebo green CI)
- Architecture LD-8: [_bmad-output/planning-artifacts/architecture.md#L70](_bmad-output/planning-artifacts/architecture.md#L70) (atomic-write-file rationale)
- Architecture LD-9: [_bmad-output/planning-artifacts/architecture.md#L71](_bmad-output/planning-artifacts/architecture.md#L71) (notify-rs + Clock-trait-in-core)
- Architecture LD-32: [_bmad-output/planning-artifacts/architecture.md#L521](_bmad-output/planning-artifacts/architecture.md#L521) (CI matrix)
- Architecture LD-37: [_bmad-output/planning-artifacts/architecture.md#L1163](_bmad-output/planning-artifacts/architecture.md#L1163) (supply-chain hygiene)
- Previous story (1.8): [_bmad-output/implementation-artifacts/1-8-configure-ci-matrix-profile-release-panic-unwind-invoke-plugin-hook-macro-stub.md](_bmad-output/implementation-artifacts/1-8-configure-ci-matrix-profile-release-panic-unwind-invoke-plugin-hook-macro-stub.md)
- Story 1.8 pr.yml workflow: [.github/workflows/pr.yml](.github/workflows/pr.yml)
- Workspace Cargo.toml: [Cargo.toml](Cargo.toml)
- Target crate placeholders: [crates/orgsidian-parser/src/lib.rs](crates/orgsidian-parser/src/lib.rs) · [crates/orgsidian-vault/src/lib.rs](crates/orgsidian-vault/src/lib.rs) · [crates/orgsidian-watcher/src/lib.rs](crates/orgsidian-watcher/src/lib.rs) · [crates/orgsidian-core/src/lib.rs](crates/orgsidian-core/src/lib.rs)

## Dev Agent Record

### Agent Model Used

claude-opus-4-7 (1M context) via `bmad-dev-story`.

### Debug Log References

- `cargo test -p orgsidian-core --features test-support --locked` — 7 passed (5 registry + 2 clock).
- `cargo test -p orgsidian-parser --locked` — 1 anchor test passed.
- `cargo test -p orgsidian-vault --locked` — 1 anchor test passed.
- `cargo test -p orgsidian-watcher --locked` — 1 anchor test passed (~60ms wall-clock).
- `cargo test --workspace --locked` — all anchor + registry + plugin-api + IPC-binding tests green; 3 × `running 1 test` lines confirmed for the anchor binaries.
- `cargo fmt --all -- --check` — exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0.
- `cargo build --workspace --locked` — exit 0.
- `cargo build --release --workspace --locked` — exit 0 (test_support gate keeps `clock.rs` out of release).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` — exit 0.
- `cargo deny --locked check all` — `advisories ok, bans ok, licenses ok, sources ok` after adding `nix@0.30.1` skip.
- `cargo audit --deny warnings $IGNORES` (with canonical `.cargo/audit-ignore.txt`) — exit 0.

### Implementation Plan

1. **Task 1 (Clock):** Declared `Clock` + `FakeClock` in `crates/orgsidian-core/src/test_support/clock.rs` (mutex-wrapped `Instant`, advance via `+=`). Gated the module behind `cfg(any(test, feature = "test-support"))` per AC3. Added `[features] test-support = []` to the core crate manifest.
2. **Task 2 (Parser):** Stub `parse(&str) -> Result<ParseTree, ParseError>` with sealed `ParseTree { _private: () }` and `ParseError::Empty` (thiserror-derived). Fixture is exactly 13 ASCII bytes (`2a 20 54 4f 44 4f 20 48 65 6c 6c 6f 0a`) verified via `xxd`. Test reads via `env!("CARGO_MANIFEST_DIR")` and `expect("anchor fixture must parse")`.
3. **Task 3 (Vault):** Single-delegation `atomic_write(path, content)` over `AtomicWriteFile::open + write_all + commit` (the 0.3.0 API uses the handle pattern; the older free-function `write_to_file` with `OverwriteBehavior` is no longer exposed but the semantic shape — atomic temp-then-rename overwriting existing files — is preserved). Tempfile-backed round-trip test asserts byte-identical read-back.
4. **Task 4 (Watcher):** Polling-based `detect_first_write_event` (10 ms sleep loop on `metadata().modified()`; Clock-driven deadline check). The watcher declares a *local* `Clock` trait facade because `orgsidian_core::test_support::clock::Clock` is gated out of release builds — the watcher's production API cannot depend on a feature-gated trait. The test uses a tiny `ClockAdapter` newtype to bridge `FakeClock` into the watcher's facade.
5. **Task 5 (Verification):** All 11 AC6 dev-box matrix cells green.
6. **Task 6 (Scope-fence):** Diff stayed inside AC7's allowed file set; the only "extra" touches (`deny.toml` + `docs/security/advisory-exceptions.md`) are the explicitly anticipated Story 1.7 ledger pair for the `nix@0.30` transitive surprise.

### Completion Notes List

- **Anti-placebo invariant verified end-to-end.** `cargo test --workspace --locked` output shows three `Running tests/anchor.rs (...anchor-XXX)` binaries each printing `running 1 test` followed by `1 passed; 0 failed`. No zero-test placeholders.
- **Cross-crate edge added by AC3:** `orgsidian-watcher` now dev-depends on `orgsidian-core` (with `test-support` feature). Production-side, the watcher keeps a local `Clock` facade — no production dependency on `orgsidian-core` was introduced, preserving the LEAF-graph dependency rule (LD-37).
- **Transitive surprise handled per Story 1.7 discipline:** `atomic-write-file 0.3` pins `nix 0.30`; `tauri-plugin-os` already pins `nix 0.31`. Added `nix@0.30.1` to `deny.toml` `[bans].skip` with a justified reason AND the paired ledger row in `docs/security/advisory-exceptions.md` (review date 2026-08-21, 90 days out). No silent allowlist expansion; no `skip-tree` carve-out.
- **License check stayed clean.** `atomic-write-file` is BSD-3-Clause (already on the LD-37 allowlist). `tempfile` is MIT/Apache-2.0. No `[licenses].exceptions` additions needed; no `scripts/check-allowlist-sync.mjs` edit required.
- **Cargo.lock additions are minimal and explainable:** `atomic-write-file 0.3.0`, `nix 0.30.1`, `rand 0.9.4`, `rand_chacha 0.9.0`, `rand_core 0.9.5`. `tempfile` was already in the lock (transitive via Tauri build chain); the Story 1.9 add only registers it as a direct dev-dep of vault/watcher.
- **Watcher anchor wall-clock budget:** ~60 ms on macOS-arm64 dev box. Well under the 5 s deadline (Dev Notes §9 noted the 50 ms spawn-and-write is comfortably above all mainstream-FS mtime resolutions).
- **Story 5.2 deferred (PR push):** The dev-box matrix is green; pushing the branch + verifying `pr.yml` greenness on `macos-14` + `ubuntu-24.04` is the next step (handled by `bmad-dev-story` step 10 — branch push + PR open + label transition). Sub-task 5.2 left as `[ ]` in Tasks/Subtasks to document this; all other Task 5 sub-items are `[x]`.

### File List

**Workspace-level edits**
- `Cargo.toml` — added `atomic-write-file = "0.3"` to `[workspace.dependencies]` with the Story 1.9 LD-8 comment, alphabetically placed between `tracing` and the next neighbor.
- `Cargo.lock` — registry resolution updated for the new direct + transitive deps (`atomic-write-file`, `nix`, `rand`, `rand_chacha`, `rand_core`).
- `deny.toml` — added `nix@0.30.1` to `[bans].skip` with reason text referencing Story 1.9.
- `docs/security/advisory-exceptions.md` — added the paired ledger row for `nix@0.30.1` under "Cargo duplicate-version skips" (review 2026-08-21).

**`orgsidian-core` (Task 1)**
- `crates/orgsidian-core/Cargo.toml` — added `[features] test-support = []`.
- `crates/orgsidian-core/src/lib.rs` — declared `#[cfg(any(test, feature = "test-support"))] pub mod test_support;` sibling to `pub mod registry;`.
- `crates/orgsidian-core/src/test_support/mod.rs` — NEW. Declares `pub mod clock;`.
- `crates/orgsidian-core/src/test_support/clock.rs` — NEW. `Clock` trait + `FakeClock` impl + 2 unit tests (advance determinism + Send/Sync/'static bounds).

**`orgsidian-parser` (Task 2)**
- `crates/orgsidian-parser/Cargo.toml` — added `thiserror = { workspace = true }` to `[dependencies]`.
- `crates/orgsidian-parser/src/lib.rs` — rewritten with the AC4 stub (`parse`, `ParseTree { _private: () }`, `ParseError::Empty`) + anchor-sentinel doc-comment paragraph.
- `crates/orgsidian-parser/tests/anchor.rs` — NEW. `parse_anchor_fixture_succeeds` test.
- `crates/orgsidian-parser/tests/fixtures/anchor.org` — NEW. Exactly 13 bytes: `* TODO Hello\n`.

**`orgsidian-vault` (Task 3)**
- `crates/orgsidian-vault/Cargo.toml` — added `atomic-write-file = { workspace = true }` to `[dependencies]`; added `[dev-dependencies] tempfile = "3"`.
- `crates/orgsidian-vault/src/lib.rs` — added `pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()>` (single delegation to `AtomicWriteFile::open + write_all + commit`).
- `crates/orgsidian-vault/tests/anchor.rs` — NEW. `atomic_write_anchor_roundtrips_byte_identical` test.

**`orgsidian-watcher` (Task 4)**
- `crates/orgsidian-watcher/Cargo.toml` — added `thiserror = { workspace = true }` to `[dependencies]`; added `[dev-dependencies] orgsidian-core = { workspace = true, features = ["test-support"] }` + `tempfile = "3"`.
- `crates/orgsidian-watcher/src/lib.rs` — added local `Clock` trait facade, `DetectedEvent`, `DetectError`, and `pub fn detect_first_write_event`.
- `crates/orgsidian-watcher/tests/anchor.rs` — NEW. `watcher_detects_first_write_within_clock_budget` test (uses `ClockAdapter` newtype to bridge `orgsidian-core::FakeClock` into the watcher's `Clock` facade).

**Workflow artifacts**
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `1-9-...: ready-for-dev → in-progress → review`.
- `_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md` — this story file (Tasks ticked, Dev Agent Record + File List + Change Log + Status updated).

## Change Log

| Date       | Change                                                                                                                  | Author                                |
| ---------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-23 | Story 1.9 implemented: 3 anchor smoke tests (parser/vault/watcher) + Clock/FakeClock in core + parser/vault API stubs. | Amelia (`bmad-dev-story`) for Tiziano |
| 2026-05-23 | Story 1.7 ledger updated: `nix@0.30.1` duplicate-version skip added (transitive via atomic-write-file 0.3).             | Amelia (`bmad-dev-story`) for Tiziano |
