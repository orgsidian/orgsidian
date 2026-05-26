# Story 1.11: Establish LD-41 failure-mode test harness (Party Mode round 2 P0 — Murat)

Status: done

## Metadata

github_issue: 11

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want a single cross-cutting `tests/failure_modes.rs` harness enumerating every LD-41 failure-mode category with concrete simulation hooks (fault-injection via the `fail` crate where applicable), plus a coverage gate (`tests/failure_modes_coverage.rs`) and an auto-generated `docs/failure-modes/coverage-matrix.md`,
so that no LD-41 failure mode ships uncovered into v0.1 Alpha and the post-v0.5-Beta strict-coverage gate has the infrastructure ready the moment downstream stories implement their real fault-injection tests.

## Acceptance Criteria

**AC1 — `tests/failure_modes.rs` enumerates every LD-41 failure-mode category as `#[ignore]` placeholders.**

- File path: `tests/failure_modes.rs` (NEW file at the workspace root — verified missing via `ls /Users/tizianobasile/workspace/me/orgsidian/tests/` which fails because the `tests/` directory does NOT yet exist at workspace root).
- The harness MUST enumerate **all categories** from the current LD-41 catalog ([_bmad-output/planning-artifacts/architecture.md#L1196-L1209](_bmad-output/planning-artifacts/architecture.md#L1196-L1209)). The epic AC text ([_bmad-output/planning-artifacts/epics.md#L593](_bmad-output/planning-artifacts/epics.md#L593)) says "all 9 LD-41 categories" but a **10th row was added 2026-05-20** to LD-41 (the "Refile partial completion" row at architecture line 1209 referenced by Story 11.8 in [_bmad-output/planning-artifacts/epics.md#L2158-L2160](_bmad-output/planning-artifacts/epics.md#L2158-L2160)). Honor the **current architecture catalog** as the source of truth: enumerate **10 categories** (the 9 from the original AC + the Refile partial-completion row). See Dev Notes §3 for the AC-vs-architecture drift discussion.
- Each category MUST be a single `#[test]` function annotated `#[ignore = "implemented in Epic N"]` (where `N` is the owning epic from the table below). The `#[ignore = "..."]` literal string is parsed by the matrix generator (AC4) and by the coverage gate (AC3), so the **exact format `implemented in Epic N`** matters — do not paraphrase to "to be implemented in Epic N" or "Epic N implementation pending".
- Test function names are snake_case categories (parsed by the matrix generator and the coverage gate):

  | Category (test fn name)               | Owning Epic | `#[ignore]` annotation                             |
  |---------------------------------------|-------------|----------------------------------------------------|
  | `malformed_org_file_quarantined`      | Epic 2      | `#[ignore = "implemented in Epic 2"]`              |
  | `disk_full_atomic_write`              | Epic 3      | `#[ignore = "implemented in Epic 3"]`              |
  | `config_corruption_fallback`          | Epic 3      | `#[ignore = "implemented in Epic 3"]`              |
  | `vault_folder_deleted_runtime`        | Epic 5      | `#[ignore = "implemented in Epic 5"]`              |
  | `plugin_init_panic_isolated`          | Epic 1      | `#[ignore = "implemented in Epic 1"]`              |
  | `plugin_on_event_panic_isolated`      | Epic 1      | `#[ignore = "implemented in Epic 1"]`              |
  | `sqlite_index_corruption_rebuild`     | Epic 3      | `#[ignore = "implemented in Epic 3"]`              |
  | `tmp_orphan_files_cleanup`            | Epic 3      | `#[ignore = "implemented in Epic 3"]`              |
  | `external_delete_with_dirty_buffer`   | Epic 5      | `#[ignore = "implemented in Epic 5"]`              |
  | `refile_partial_completion_rollback`  | Epic 11     | `#[ignore = "implemented in Epic 11"]`             |

- Each placeholder body is a one-statement panic that prevents accidental green-with-no-assertion in case the `#[ignore]` is removed prematurely: `unimplemented!("LD-41: <human-readable category name> — see test-design.md §6.7 + Story <N.M> for real implementation")`. This way, running `cargo test -- --include-ignored` surfaces a clear "unimplemented" panic per category, NOT a silent pass.
- **At least one** placeholder MUST demonstrate the `fail` crate idiom in a code comment so future implementers see the exact pattern. The `disk_full_atomic_write` test is the canonical exemplar — include a commented-out preview of the real implementation in its body:

  ```rust
  #[test]
  #[ignore = "implemented in Epic 3"]
  fn disk_full_atomic_write() {
      // Future implementation (Epic 3 / Story 3.1):
      //
      // let _scenario = fail::FailScenario::setup();
      // fail::cfg("atomic-write::after-tmp-rename", "panic").unwrap();
      // let vault = test_vault();
      // let result = vault.save_file("test.org", "content");
      // assert!(result.is_err());
      // assert!(!vault.path().join("test.org").exists());
      //
      // FailScenario teardown is automatic on drop.
      unimplemented!(
          "LD-41: Disk full / ENOSPC during atomic write — \
           see test-design.md §6.7 + Story 3.1 for real implementation"
      );
  }
  ```

- File header doc-comment (module-level `//!`) MUST reference: `Story 1.11`, `LD-41`, `test-design.md §6.7`, and a one-line caveat that the harness is intentionally `#[ignore]`-heavy at the v0.1 Alpha era — coverage is filled in incrementally by downstream stories.

**AC2 — Workspace wiring: the harness compiles + runs via `cargo test --workspace`.**

- The workspace root `Cargo.toml` is a virtual manifest (`[workspace]` only, no `[package]`) — verified at [Cargo.toml#L1-L17](Cargo.toml#L1-L17). Therefore a literal workspace-root `tests/<file>.rs` is NOT auto-discovered by Cargo. Wire the harness via an `[[test]]` declaration in **`crates/orgsidian-core`** (the cross-crate integrator per its package description "Core domain orchestrator wiring parser/index/watcher/vault/plugin-api/report"; verified at [crates/orgsidian-core/Cargo.toml#L1-L3](crates/orgsidian-core/Cargo.toml#L1-L3)).
- Add to `crates/orgsidian-core/Cargo.toml`:
  ```toml
  [[test]]
  name = "failure_modes"
  path = "../../tests/failure_modes.rs"
  required-features = ["test-support"]

  [[test]]
  name = "failure_modes_coverage"
  path = "../../tests/failure_modes_coverage.rs"
  ```
- The `required-features = ["test-support"]` on `failure_modes` mirrors the Story 1.9 / 1.12 pattern of gating test-only helpers behind the `test-support` feature (defined at [crates/orgsidian-core/Cargo.toml#L10-L14](crates/orgsidian-core/Cargo.toml#L10-L14)). The coverage gate (`failure_modes_coverage`) does NOT require it — it is a source-text scanner with no test-support deps. See Dev Notes §4 for the rationale.
- Add `fail = "0.5"` to `[workspace.dependencies]` in the root `Cargo.toml` (latest stable at 2026-05-25 per [docs.rs/fail](https://docs.rs/fail/latest/fail/), confirmed 2026-05 — see Dev Notes §6). Add `fail = { workspace = true, features = ["failpoints"] }` to `[dev-dependencies]` of `crates/orgsidian-core/Cargo.toml`. **`features = ["failpoints"]` is non-negotiable** — without it, the `fail` crate's `cfg`/`fail_point!` calls compile to no-ops and the future fault-injection tests would silently bypass.
- Auto-run is sufficient: when contributors run `cargo test --workspace --locked`, the placeholder tests appear in the output as `ignored` lines (NOT in the active test count). No `.github/workflows/*` edit is required in this story — the new `[[test]]` targets are picked up by the existing `cargo test --workspace --locked` step in [.github/workflows/pr.yml](.github/workflows/pr.yml).

**AC3 — `tests/failure_modes_coverage.rs` is the post-v0.5-Beta strict-coverage gate (no-op today).**

- File path: `tests/failure_modes_coverage.rs` (NEW file at workspace root).
- The gate MUST scan the source of `tests/failure_modes.rs` (read via `include_str!("./failure_modes.rs")` so the gate compiles regardless of working directory) and tally categories where the only annotation present is `#[ignore = "implemented in Epic N"]` (i.e., no sibling non-ignored implementation has been added).
- **Modes** (gate behavior depends on env var):
  - **Default (v0.1 → v0.5 Beta era)**: the test logs the unimplemented-category list to stderr via `eprintln!` and **passes**. This is the "advisory mode" — visibility without blocking.
  - **Strict mode** (env `ORGSIDIAN_FAILURE_MODE_STRICT=1`): the test **fails** if ≥1 category has no non-ignored sibling implementation. The CI gate-flip to strict mode is owned by a **future story tagged for the v0.5 Beta release prep** (NOT Story 1.11). The env-var hook is the wiring point.
- Implementation sketch (one file, ≤80 LOC):

  ```rust
  //! Story 1.11 — LD-41 failure-mode coverage gate.
  //!
  //! Default mode (v0.1 → v0.5 Beta): advisory; logs unimplemented categories
  //! and passes. Strict mode (`ORGSIDIAN_FAILURE_MODE_STRICT=1`): fails CI if
  //! any LD-41 category has only `#[ignore]` placeholders. The strict-mode
  //! flip is a v0.5-Beta release-prep story owned, NOT Story 1.11.

  const HARNESS_SRC: &str = include_str!("./failure_modes.rs");

  /// Returns `(unimplemented_categories, total_categories)` parsed from the
  /// `#[ignore = "implemented in Epic N"]` annotations in HARNESS_SRC.
  fn scan_categories() -> (Vec<String>, usize) {
      // Parse lines of the form:
      //   #[ignore = "implemented in Epic N"]
      //   fn <category_name>() {
      // and collect <category_name> as an unimplemented category.
      // (Real implementation: split into lines, scan for the #[ignore = ...]
      // marker, then capture the next `fn <name>` token.)
      // ...
  }

  #[test]
  fn ld_41_categories_have_real_implementations() {
      let (unimplemented, total) = scan_categories();
      let strict = std::env::var("ORGSIDIAN_FAILURE_MODE_STRICT")
          .map(|v| v == "1")
          .unwrap_or(false);

      if strict {
          assert!(
              unimplemented.is_empty(),
              "LD-41 strict-coverage gate: {} of {} categories still have only \
               #[ignore] placeholders: {:?}. Implement real fault-injection \
               tests in the owning epics before merging post-v0.5 Beta.",
              unimplemented.len(), total, unimplemented,
          );
      } else {
          eprintln!(
              "LD-41 advisory: {}/{} failure-mode categories still on #[ignore] \
               placeholders: {:?}. Strict-mode CI gate flips post-v0.5 Beta.",
              unimplemented.len(), total, unimplemented,
          );
      }
  }
  ```
- A second `#[test]` in the same file `failure_mode_count_matches_ld_41_catalog` asserts `total == 10` so a future contributor cannot accidentally remove a category from the harness without breaking the gate. The constant `10` is the current LD-41 row count (per AC1); if LD-41 grows another row, this is a deliberate documentation-pinned breaking change (the gate forces a coordinated update of architecture.md + harness + this constant + coverage-matrix.md).

**AC4 — `docs/failure-modes/coverage-matrix.md` is auto-generated from the harness.**

- Directory: `docs/failure-modes/` (NEW — verified the directory does NOT yet exist via `ls /Users/tizianobasile/workspace/me/orgsidian/docs/` which returns only `logo-draft.png` + `security/`).
- File: `docs/failure-modes/coverage-matrix.md` (NEW).
- Generator: `scripts/gen-failure-modes-matrix.mjs` (NEW). Node ≥20.x (already a project prereq per [CONTRIBUTING.md#L88-L89](CONTRIBUTING.md#L88-L89)). The script:
  1. Reads `tests/failure_modes.rs` source via `fs.readFileSync`.
  2. Parses every block of the form:
     ```
     #[ignore = "implemented in Epic N"]
     fn <category_name>() {
     ```
  3. Maps category → owning epic + mechanism (mechanism table is hard-coded from test-design.md §6.7 LD-41 Coverage Matrix; the generator is a deterministic merge of harness state + the §6.7 table).
  4. Emits markdown to stdout; the build step pipes to `docs/failure-modes/coverage-matrix.md`.
- Add a `pnpm` script entry to `package.json`:
  ```json
  "gen:failure-modes-matrix": "node scripts/gen-failure-modes-matrix.mjs > docs/failure-modes/coverage-matrix.md"
  ```
- The committed `docs/failure-modes/coverage-matrix.md` MUST be the script's exact output as of the story's commit. A header line in the markdown documents the regeneration command verbatim: `<!-- regenerated by: pnpm gen:failure-modes-matrix -->`.
- Required content (committed initial output) lists all 10 LD-41 categories in this order — Malformed `.org` / Disk full / Config corruption / Vault folder deleted / Plugin `init()` panic / Plugin `on_event` panic / SQLite index corruption / `.tmp` orphan files / External delete with Dirty Buffer / Refile partial completion — with **`status: #[ignore] (implemented in Epic N)`** for each, plus a "Mechanism" column copied from test-design.md §6.7 ([_bmad-output/test-artifacts/test-design.md#L574-L586](_bmad-output/test-artifacts/test-design.md#L574-L586)) + an explicit Refile-partial-completion mechanism row drawn from [_bmad-output/planning-artifacts/architecture.md#L1209](_bmad-output/planning-artifacts/architecture.md#L1209).
- The matrix's "Status" column updates automatically when a future story removes an `#[ignore]` and adds a real implementation — the next contributor regenerates the matrix and commits the updated file alongside their PR. The CONTRIBUTING.md regeneration hint lives in a single sentence inside the markdown header: "Regenerate via `pnpm gen:failure-modes-matrix` after any change to `tests/failure_modes.rs`."

**AC5 — Workspace dependency wiring (`fail` crate + workspace `[dev-dependencies]`).**

- Root `Cargo.toml` gets one new `[workspace.dependencies]` entry:
  ```toml
  # Story 1.11 (LD-41): fault-injection helpers consumed by tests/failure_modes.rs
  # via crates/orgsidian-core dev-dependencies. The `failpoints` feature is
  # enabled at the consumer-crate level (NOT here) so production builds compile
  # `fail` to no-ops by default. Confirmed latest stable 0.5.1 (2026-05).
  fail = "0.5"
  ```
- `crates/orgsidian-core/Cargo.toml` gets one new `[dev-dependencies]` block (the crate currently has none — verified at [crates/orgsidian-core/Cargo.toml](crates/orgsidian-core/Cargo.toml)):
  ```toml
  [dev-dependencies]
  fail = { workspace = true, features = ["failpoints"] }
  ```
- The new dep MUST pass `cargo deny check` cleanly. `fail` crate's transitive license set is well-vetted (TiKV-maintained; Apache-2.0/MIT dual). If `cargo deny` surfaces a previously-unseen advisory or license, surface as a decision-grade question per [[feedback_batch_fixes_terse]] — do NOT silently add a new `deny.toml [advisories].ignore` entry.

**AC6 — Anti-creep scope-fence (out-of-scope items for Story 1.11).**

The following are NOT modified by Story 1.11. Any drift is a review-block:

- **`crates/test-plugin-panic/`**: do NOT create. The chaos plugin crate (LD-38, architecture line 1409 marks it as a future workspace member) is referenced by the `plugin_init_panic_isolated` + `plugin_on_event_panic_isolated` placeholders but the real crate lands in a later story (likely a dedicated Epic 1 chaos-plugin story or Epic 12 v1.0 hardening). Story 1.11 only enumerates the placeholders.
- **Source crate modifications (`crates/orgsidian-*/src/**`)**: zero touches. No `fail_point!(...)` calls in any production source file. The `fail` crate dep is dev-only via `[dev-dependencies]`; production builds compile it as a no-op. This story is harness-scaffolding-only.
- **`.github/workflows/*`**: zero touches. The new `[[test]]` targets are picked up by the existing `cargo test --workspace --locked` step in `pr.yml`. The strict-mode env-var flip (`ORGSIDIAN_FAILURE_MODE_STRICT=1`) is wired by a future v0.5 Beta release-prep story.
- **`docs/security/`**: zero touches. The new `docs/failure-modes/` sibling directory is the right home for this artifact (failure modes are a reliability concern, not a security-advisory concern).
- **README.md / ARCHITECTURE.md / CONTRIBUTING.md**: do NOT update with backlinks to the new docs in this story. The matrix is auto-generated and self-describes its regeneration command; adding root-doc backlinks is a deferred discoverability follow-up.
- **`deferred-work.md`**: do NOT delete the README-stale-framing follow-up at [_bmad-output/implementation-artifacts/deferred-work.md#L62-L64](_bmad-output/implementation-artifacts/deferred-work.md#L62-L64). It is out-of-scope for Story 1.11.
- **`Cargo.lock` regeneration**: expected to update once (fail crate transitive deps added). Commit the updated lockfile per LD-37; this is NOT scope creep.
- **`tests/traceability.rs`**: do NOT create. That gate is owned by Epic 2+ per [architecture.md#L1081](_bmad-output/planning-artifacts/architecture.md#L1081) + Story 1.10's AC7.
- **`fixtures/` at workspace root**: do NOT create. Story 1.11 has no fixture-corpus need (placeholders are unimplemented panics).
- **Strict-mode gate flip**: do NOT enable `ORGSIDIAN_FAILURE_MODE_STRICT=1` in `pr.yml` or `nightly.yml`. The gate ships in advisory mode only.

**AC7 — Dev-box verification matrix.**

The following MUST all succeed on a clean checkout of Story 1.11's HEAD before the story moves to `review`:

| Command | Expected | Run on |
|---|---|---|
| `ls tests/failure_modes.rs tests/failure_modes_coverage.rs` | both files present | macOS-arm64 (dev) |
| `ls docs/failure-modes/coverage-matrix.md` | file present | macOS-arm64 (dev) |
| `ls scripts/gen-failure-modes-matrix.mjs` | file present | macOS-arm64 (dev) |
| `grep -c '#\[ignore = "implemented in Epic' tests/failure_modes.rs` | exit 0; output `10` | macOS-arm64 (dev) |
| `grep -c 'unimplemented!' tests/failure_modes.rs` | exit 0; output `10` | macOS-arm64 (dev) |
| `grep -c 'fail::cfg\|fail::FailScenario' tests/failure_modes.rs` | exit 0; output ≥`1` (comment-only is OK) | macOS-arm64 (dev) |
| `grep -c 'ORGSIDIAN_FAILURE_MODE_STRICT' tests/failure_modes_coverage.rs` | exit 0; output ≥`1` | macOS-arm64 (dev) |
| `grep -c 'regenerated by: pnpm gen:failure-modes-matrix' docs/failure-modes/coverage-matrix.md` | exit 0; output ≥`1` | macOS-arm64 (dev) |
| `pnpm gen:failure-modes-matrix` then `git diff --exit-code docs/failure-modes/coverage-matrix.md` | exit 0 (regeneration is idempotent) | macOS-arm64 (dev) |
| `cargo fmt --all -- --check` | exit 0 | macOS-arm64 (dev) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | macOS-arm64 (dev) |
| `cargo build --workspace --locked` | exit 0 | macOS-arm64 (dev) |
| `cargo test --workspace --locked` | exit 0; placeholders appear in output as `ignored` | macOS-arm64 (dev) |
| `cargo test --workspace --locked -- --include-ignored 2>&1 \| grep -c '^test .* panicked'` | output `10` (every placeholder panics with `unimplemented!`) | macOS-arm64 (dev) |
| `cargo test -p orgsidian-core --test failure_modes_coverage --locked` | exit 0; advisory `eprintln!` visible with `--nocapture` | macOS-arm64 (dev) |
| `ORGSIDIAN_FAILURE_MODE_STRICT=1 cargo test -p orgsidian-core --test failure_modes_coverage --locked` | exit non-zero (10 unimplemented categories) | macOS-arm64 (dev) |
| `cargo deny check --locked` | exit 0 (no new advisory/license/source/ban surprises from `fail` 0.5.x) | macOS-arm64 (dev) |
| `cargo audit --locked` | exit 0 (no new advisory surprises from `fail` 0.5.x) | macOS-arm64 (dev) |

If any cell fails on the dev box, the story MUST NOT move to `review`. The most likely failure modes:
- `cargo test -- --include-ignored` output count off by 1 if a category name was typo'd in the table.
- `cargo deny check` surfacing a fresh advisory on a `fail` transitive (rare; flag for decision per AC5).
- `pnpm gen:failure-modes-matrix` regeneration drift caused by non-deterministic ordering — the script MUST iterate categories in the order they appear in `failure_modes.rs` (top-down source order); do NOT use `Object.keys()` on an unsorted map.

**AC8 — Memory-anchored conventions (cross-cutting).**

- **[[feedback_no_co_author_credit]]**: No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on any commit / PR / issue.
- **[[user_contact_email]]**: Authorship attribution uses `tiz.basile@gmail.com` (already pinned in `Cargo.toml [workspace.package].authors`). The new mjs script + Rust files do NOT add a personal contact header.
- **[[feedback_version_policy]]**: `fail = "0.5"` is the latest stable per [docs.rs/fail](https://docs.rs/fail/latest/fail/) confirmed 2026-05 (0.5.1). Caret version is acceptable (matches the existing `atomic-write-file = "0.3"` Story 1.9 pattern); the Tauri-ecosystem exact-pin discipline does NOT apply (this is a non-Tauri crate).
- **[[feedback_batch_fixes_terse]]**: Post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface — specifically the AC1 9-vs-10-categories drift (epic AC text vs architecture catalog) is the decision-grade question this story carries forward (see Dev Notes §3).
- **[[project_orgsidian_github_label_scheme]]**: Issue #11 label transitions follow `status:backlog` → `status:in-progress` → `status:in-review` per [_bmad-output/_/memory/project_orgsidian_github_label_scheme.md] (status:in-review, NOT status:review).

**Traces:** LD-41 (Failure Mode Catalog — architecture line 1196-1209), NFR-15, NFR-16, test-design.md §6.7 (Chaos / Fault Injection layer), Process Discipline rule A (red-phase ATDD merge-gate — the harness IS the red-phase scaffold for every downstream LD-41 implementation story), Process Discipline rule H (test-design.md as authoritative system-level test strategy).

## Tasks / Subtasks

- [x] **Task 1 — Author `tests/failure_modes.rs` at workspace root** (AC1)
  - [x] 1.1 Create the `tests/` directory at the workspace root (`mkdir tests/`).
  - [x] 1.2 Author `tests/failure_modes.rs` with the module-level `//!` doc-comment referencing Story 1.11 / LD-41 / test-design.md §6.7.
  - [x] 1.3 Enumerate all 10 LD-41 categories per the AC1 table — exact test-fn names, exact `#[ignore = "implemented in Epic N"]` annotations, exact `unimplemented!("LD-41: <category> — see test-design.md §6.7 + Story <N.M> for real implementation")` bodies.
  - [x] 1.4 In the `disk_full_atomic_write` placeholder body, include the commented-out `fail::FailScenario::setup()` + `fail::cfg(...)` preview block exactly as shown in AC1.
  - [x] 1.5 `grep -c '#\[ignore = "implemented in Epic' tests/failure_modes.rs` → `10`. `grep -c 'unimplemented!' tests/failure_modes.rs` → `10`.

- [x] **Task 2 — Wire `[[test]]` targets in `crates/orgsidian-core/Cargo.toml`** (AC2)
  - [x] 2.1 Append `[[test]]` block for `failure_modes` (path `../../tests/failure_modes.rs`, `required-features = ["test-support"]`).
  - [x] 2.2 Append `[[test]]` block for `failure_modes_coverage` (path `../../tests/failure_modes_coverage.rs`; no required-features).
  - [x] 2.3 Add a short comment header inside the new block referencing Story 1.11 + LD-41 + the workspace-root-tests-via-[[test]] rationale (Dev Notes §4).
  - [x] 2.4 `cargo test -p orgsidian-core --list 2>&1 | grep -c "failure_modes"` → ≥`2` (both targets discovered).

- [x] **Task 3 — Add `fail` crate dependency** (AC5)
  - [x] 3.1 Add `fail = "0.5"` to root `Cargo.toml [workspace.dependencies]` with the 3-line comment header per AC5.
  - [x] 3.2 Add `[dev-dependencies]` block to `crates/orgsidian-core/Cargo.toml` with `fail = { workspace = true, features = ["failpoints"] }`.
  - [x] 3.3 `cargo build --workspace --locked` → success. `Cargo.lock` updates expected (transitive `fail` deps); commit the lockfile.
  - [x] 3.4 `cargo deny check --locked` + `cargo audit --locked` → both clean. If either surfaces a new advisory/license, halt + surface as decision-grade question per AC5.

- [x] **Task 4 — Author `tests/failure_modes_coverage.rs`** (AC3)
  - [x] 4.1 Create `tests/failure_modes_coverage.rs` with the module-level `//!` doc-comment referencing strict-vs-advisory modes.
  - [x] 4.2 Implement `scan_categories()` parsing `HARNESS_SRC = include_str!("./failure_modes.rs")` for `#[ignore = "implemented in Epic N"]` annotations and their adjacent `fn <name>` tokens. Top-down source order MUST be preserved (Vec order, NOT HashMap).
  - [x] 4.3 Implement `#[test] fn ld_41_categories_have_real_implementations()` with the `ORGSIDIAN_FAILURE_MODE_STRICT` env-var branching per AC3.
  - [x] 4.4 Implement `#[test] fn failure_mode_count_matches_ld_41_catalog()` asserting `total == 10`.
  - [x] 4.5 `cargo test -p orgsidian-core --test failure_modes_coverage --locked` → green (advisory mode). With `ORGSIDIAN_FAILURE_MODE_STRICT=1` → red (10 unimplemented).

- [x] **Task 5 — Author `scripts/gen-failure-modes-matrix.mjs`** (AC4)
  - [x] 5.1 Create `scripts/gen-failure-modes-matrix.mjs`. Use Node `fs.readFileSync` + the same line-scan regex as Task 4.2 (or a slightly looser one — JS regex on the harness source is acceptable here because the source format is stable).
  - [x] 5.2 Hard-code the mechanism table from test-design.md §6.7 ([_bmad-output/test-artifacts/test-design.md#L574-L586](_bmad-output/test-artifacts/test-design.md#L574-L586)) + the Refile-partial-completion row from architecture.md line 1209.
  - [x] 5.3 Emit markdown with header `<!-- regenerated by: pnpm gen:failure-modes-matrix -->`, a one-line regen reminder, and the matrix table (Category | Owning Epic | Mechanism | Status).
  - [x] 5.4 Iteration order: top-down through `tests/failure_modes.rs` source — do NOT sort alphabetically.

- [x] **Task 6 — Add `pnpm` script + initial generated matrix** (AC4)
  - [x] 6.1 Add `"gen:failure-modes-matrix": "node scripts/gen-failure-modes-matrix.mjs > docs/failure-modes/coverage-matrix.md"` to `package.json` `"scripts"`.
  - [x] 6.2 Run `pnpm gen:failure-modes-matrix` to produce `docs/failure-modes/coverage-matrix.md`. Verify with `git diff --exit-code docs/failure-modes/coverage-matrix.md` (re-run produces no diff — idempotent).
  - [x] 6.3 Confirm the matrix lists 10 rows with `status: #[ignore] (implemented in Epic N)` for every category.

- [x] **Task 7 — Dev-box verification matrix** (AC7)
  - [x] 7.1 Run every cell in AC7. Record exit codes + outputs in the Dev Agent Record / Debug Log References section.
  - [x] 7.2 Specifically verify the strict-mode env-var: `ORGSIDIAN_FAILURE_MODE_STRICT=1 cargo test -p orgsidian-core --test failure_modes_coverage --locked` → exit non-zero with all 10 category names in the failure output.

- [x] **Task 8 — Scope-fence audit** (AC6)
  - [x] 8.1 `git status` confirms the in-scope file set: 4 NEW files (`tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, `scripts/gen-failure-modes-matrix.mjs`, `docs/failure-modes/coverage-matrix.md`) + 3 MODIFIED (`Cargo.toml`, `crates/orgsidian-core/Cargo.toml`, `package.json`) + `Cargo.lock` regeneration + the workflow-required sprint-status + this story file updates.
  - [x] 8.2 Verified: no `.github/workflows/*` touches, no `crates/orgsidian-*/src/**` source-file touches, no `crates/test-plugin-panic/` creation, no `tests/traceability.rs` creation, no `fixtures/` directory creation, no `docs/security/` touches, no README.md / ARCHITECTURE.md / CONTRIBUTING.md edits.
  - [x] 8.3 `Cargo.lock` diff inspection: only new transitive deps of `fail` 0.5.x; no surprise version bumps to existing deps. If any existing dep is bumped, surface as decision-grade question.

- [x] **Task 9 — GitHub Issue sync (pre-flight)** (AC8)
  - [x] 9.1 Issue #11 label transition: `status:backlog` → `status:in-progress` at dev-story start; → `status:in-review` post-implementation (NOT `status:review` per [[project_orgsidian_github_label_scheme]]).
  - [x] 9.2 Verify no other label changes needed (`epic:1`, `milestone:v0.1`, `type:story` already correct per `gh issue view 11 --json labels` output captured 2026-05-25).

### Review Findings

_Code review 2026-05-26 (bmad-code-review, PR #126). 3 layers run in parallel: Blind Hunter (17 raw findings, 5 major+), Edge Case Hunter (14 raw findings across 10 categories, 5 major+), Acceptance Auditor (51 checks, 0 violations, 3 drifts). Post-dedupe + triage: 2 decision-needed, 1 patch, 5 deferred, ~10 dismissed as noise._

- [x] [Review][Patch] **`scan_categories` conflates "unimplemented" with "total" — `failure_mode_count_matches_ld_41_catalog` breaks on first LD-41 implementation** [`tests/failure_modes_coverage.rs:22-92`] — _Applied 2026-05-26: renamed `EXPECTED_LD_41_CATEGORIES` → `EXPECTED_REMAINING_PLACEHOLDERS = 10` with comment requiring per-impl-story decrement; rewrote assertion message + doc-comment. Gate verde: 2 passed; fmt + clippy clean._
- [x] [Review][Patch] **PR #126 body lacks literal `Closes #11`** — _Applied 2026-05-26: `gh pr edit 126` added `Closes #11` to body + reflected constant rename in summary._

- [x] [Review][Defer] **`disk_full_atomic_write` exemplar teaches wrong `fail::cfg` pattern** [`tests/failure_modes.rs:43-50` commented exemplar] — Spec AC1 dictates verbatim `fail::cfg("atomic-write::after-tmp-rename", "panic")` + `assert!(result.is_err())`; the `"panic"` action unwinds (assert unreachable). Defer rationale: spec AC1 dictates the exemplar verbatim; Story 3.1 will rewrite the body when implementing real fault injection — patching now would deviate from literal AC1. Owner: Story 3.1.

- [x] [Review][Defer] **Anti-placebo gap: gate does not tie `unimplemented!()` body presence to `#[ignore]` marker** [`tests/failure_modes_coverage.rs:22-52`] — If a future contributor removes BOTH `#[ignore]` AND `unimplemented!()` (replacing with empty body), the test silently passes with zero assertions; the "loud failure" contract holds only for half-removal. Strengthening would be `assert!(unimplemented_count == ignore_count)` inside scan. Defer rationale: real LD-41 implementation stories replace bodies with real assertions anyway; this guards a hypothetical sloppy/malicious refactor not anticipated by Story 1.11 scope. Owner: first LD-41 implementation story (likely Story 3.1).
- [x] [Review][Defer] **Strict-mode env-var advisory `eprintln!` is suppressed by default in `cargo test`; no CI step runs strict mode** [`tests/failure_modes_coverage.rs:72-78` + `.github/workflows/*`] — Cargo captures stderr from passing tests unless `--nocapture`. No workflow step runs `ORGSIDIAN_FAILURE_MODE_STRICT=1` or `--show-output`. Gate signal is effectively silent today. Spec AC6 explicitly anti-creeps workflows and defers strict-mode flip to v0.5-Beta release-prep story. Owner: future v0.5-Beta release-prep story (per AC3 + AC6).
- [x] [Review][Defer] **Cross-platform idempotency of `pnpm gen:failure-modes-matrix` not asserted in CI; Windows PowerShell `>` redirect may emit CRLF/UTF-16** [`scripts/gen-failure-modes-matrix.mjs:803-805` + nightly.yml absent step] — Script itself emits pure LF and the committed file is LF, but a Windows contributor regenerating locally could produce noise; nightly Windows job doesn't run `pnpm gen` so undetected. Defer rationale: AC6 forbids workflow edits in this story; a `--check` mode + CI gate is the right shape for a future Windows hardening story. Owner: future CI-hardening story (post-Story 1.13 GitHub work or v0.5-Beta release-prep).
- [x] [Review][Defer] **`required-features = ["test-support"]` is decorative; `cargo test -p orgsidian-core` silently skips harness** [`crates/orgsidian-core/Cargo.toml:32-35`] — Harness file does not actually reference `test_support` module; the gate exists only for forward-looking compile semantics (per Dev Notes §4). Workspace-test invocation works only via `orgsidian-watcher`'s incidental dev-dep on `orgsidian-core/test-support`. If that chain breaks, harness disappears silently from `cargo test --workspace`. Defer rationale: explicitly documented in Dev Notes §4 + Completion Notes as forward-looking trade-off; first LD-41 impl story will introduce a real `test_support` consumer that makes the gate non-decorative. Owner: first LD-41 implementation story.
- [x] [Review][Defer] **Stale placeholder comment in `nightly.yml` promises LD-41 nightly step that doesn't exist** [`.github/workflows/nightly.yml:~112`] — Pre-existing comment ("failure-mode test harness (LD-41) lands here as a nightly-only gate") with no actual step. Story 1.11 AC6 explicitly anti-creeps workflows. Owner: future v0.5-Beta release-prep story (alongside strict-mode flip).

_Dismissed as noise (~10): parser strictness against rustfmt-canonical `#[ignore]` whitespace (CI fmt gate enforces canonical form); strict-mode branch lacks dedicated automated test (subprocess unit-testing anti-pattern; manually verified per Dev Agent Record); various nit-level parser brittleness (`r#` raw idents, `Number()` epic unbounded, duplicate fn-name detection, `process.stdout.write` flush, empty-harness degenerate Markdown, `include_str!` path refactor risk, weird `fn` paren parsing); meta-smell "overconfident story doc" + "low active-assertion ratio" (not actionable)._

## Dev Notes

### §1 — Why this story lands NOW (and why it's a placeholder-heavy harness, not a "real" test suite)

Per Murat (Party Mode round 2 P0), `LD-41` is the single most under-defended risk surface in the architecture: a project that ships without a concrete enumeration of its failure modes is a project where each downstream story decides ad-hoc whether to test its own fault paths — and the answer is usually "no, we'll add that later." The harness solves this by **pre-committing the test-file-shape** at v0.1 Alpha: every downstream LD-41-touching story (3.1, 3.4, 3.5, 5.1, 5.4, 5.5, 11.8) inherits a `#[ignore]`d test stub it must replace with a real implementation. The placeholder-as-contract pattern is the same anti-placebo discipline as Story 1.9's anchor smoke tests — except instead of guarding "is the test wired in?", it guards "is the failure mode catalog covered?".

Story 1.11 lands in Epic 1 because the **only** time it can land is before code paths that exercise LD-41 categories start being added. If the harness arrives after Epic 3 (atomic write) or Epic 5 (watcher), the team has already missed the disk-full / .tmp-orphan / vault-deleted fault-injection opportunity inside the story that wired up the production code path. Per Murat: "Place the canary before the coal."

The "9 categories" vs "10 categories" drift between the epic AC text and the current architecture catalog (Dev Notes §3) is a documentation-staleness pattern this story doubles as: by binding the architecture LD-41 table to the harness via `failure_mode_count_matches_ld_41_catalog`, a future LD-41 row addition is forced to coordinate the harness update. The architectural catalog stops being a Markdown table that can rot independently of the test surface.

### §2 — Why `tests/` at the workspace root, hosted by `orgsidian-core/Cargo.toml [[test]]` declarations

The AC literal wording ("`tests/failure_modes.rs` at the workspace root") and the test-design.md §5.1 directory diagram ([_bmad-output/test-artifacts/test-design.md#L298-L313](_bmad-output/test-artifacts/test-design.md#L298-L313)) both place the harness physically at the workspace root, NOT inside `crates/<somecrate>/tests/`. The intent is **discoverability**: a contributor opening the orgsidian repo sees the failure-mode catalog at top-level, signaling its cross-cutting nature (not owned by any single leaf crate).

But Rust's `cargo test` workflow auto-discovers `tests/` only inside packages, NOT inside virtual workspace manifests. The orgsidian root `Cargo.toml` is virtual (`[workspace]` only). Two patterns can honor the literal placement:

1. **Add a new workspace member to host the targets** (e.g., a `crates/orgsidian-failure-modes/` library + `[[test]]` declarations pointing to `../../tests/`). Cleanest separation; ~1 new crate + Cargo.toml + src/lib.rs.
2. **Declare `[[test]]` targets in an EXISTING workspace member** (e.g., `crates/orgsidian-core/Cargo.toml`) pointing to `../../tests/...`. No new crate; the host crate carries the wiring.

Story 1.11 picks **(2)** — `orgsidian-core` is the natural integrator (its package description is literally "Core domain orchestrator wiring parser/index/watcher/vault/plugin-api/report"). The wiring stays in one Cargo.toml block, the file layout matches the AC literally, and no new workspace member adds membership churn. This trade-off is intentional; if Tiziano prefers (1), surface as a decision-grade question and we'd refactor to a `crates/orgsidian-failure-modes/` member — but the default is (2).

### §3 — The 9-vs-10-categories drift (epic AC vs architecture catalog) is a decision-grade question

Story 1.11's AC text in [_bmad-output/planning-artifacts/epics.md#L593](_bmad-output/planning-artifacts/epics.md#L593) says "all 9 LD-41 categories". The current LD-41 catalog at [_bmad-output/planning-artifacts/architecture.md#L1196-L1209](_bmad-output/planning-artifacts/architecture.md#L1196-L1209) lists **10 rows** — the 10th ("Refile partial completion (destination written, source write fails) — FR-25") was added by the 2026-05-20 closed-loop addendum ([architecture.md#L1267](_bmad-output/planning-artifacts/architecture.md#L1267)) but the epic AC text was not back-edited. Story 11.8 ([_bmad-output/planning-artifacts/epics.md#L2155-L2160](_bmad-output/planning-artifacts/epics.md#L2155-L2160)) explicitly references the 10th row as a category Story 1.11's harness should already contain ("Story 1.11's `tests/failure_modes/refile_partial.rs` becomes a passing test (no longer `#[ignore]`)") — confirming the 10-row interpretation is downstream-load-bearing.

This story honors the **current architecture catalog as source of truth** and enumerates 10 categories. The choice is intentional and surface-worthy: a reviewer reading the epic AC text "9 categories" might object that the harness has 10 placeholders instead. The response is: the 10th category was added by a later architectural addendum that did not back-edit the epic AC, but downstream stories (11.8) already depend on the 10-row interpretation. Surface for confirmation per [[feedback_batch_fixes_terse]] — do NOT silently revert to 9.

### §4 — Why `required-features = ["test-support"]` on `failure_modes` but NOT on `failure_modes_coverage`

The Story 1.9 / 1.12 convention (per [crates/orgsidian-core/Cargo.toml#L10-L14](crates/orgsidian-core/Cargo.toml#L10-L14)) gates test-helper code behind a Cargo feature so production builds (`cargo build --release` without `--features test-support`) exclude `FakeClock` and Story 1.12's perf helpers. The failure-mode placeholders are intended consumers of those helpers in their future-real-implementation form (e.g., the `disk_full_atomic_write` test will eventually use `test_vault()` which lives in `orgsidian_core::test_support::vault`). Wiring `required-features = ["test-support"]` now means: when downstream stories replace `unimplemented!(...)` with real code, the `test_support` module is automatically in scope. No retroactive Cargo-toml edit needed.

The coverage gate (`failure_modes_coverage.rs`) is pure source-text scanning — it reads `tests/failure_modes.rs` as a string and does no I/O / no vault / no clock. It has no need for `test_support`, so omit the requirement. Adding it would mean `cargo test -p orgsidian-core` (without `--features test-support`) silently skips the coverage gate — a foot-gun.

### §5 — Why placeholders panic via `unimplemented!`, not pass via `Ok(())`

A future implementer who removes the `#[ignore]` annotation but forgets to author the real assertion logic would get a green test if the body were `Ok(())` or empty. The `unimplemented!("LD-41: <category> — ...")` body forces a deliberate replacement step: the implementer MUST swap both `#[ignore]` AND the body. The error message string also carries the category name, giving downstream debugging a forced human-readable trace.

This mirrors the Story 1.9 anchor-smoke discipline: tests prove the path is exercised, not that the path returned a tautological boolean. The dual is: placeholders prove the absence of coverage, not the presence of a no-op.

### §6 — `fail` crate (v0.5.1) usage cheatsheet for the dev agent

Per [docs.rs/fail/latest](https://docs.rs/fail/latest/fail/) (confirmed 2026-05-25):
- `fail::FailScenario::setup()` returns a guard that teardown-on-drop guarantees per-test isolation (critical when multiple tests run in parallel within the same process — Cargo's default).
- `fail::cfg("name", "panic")` injects a panic at any code site instrumented with `fail_point!("name")`. The `"panic"` action is one of several (others: `"return(value)"`, `"sleep(ms)"`, `"yield"`, `"off"`).
- `fail::remove("name")` un-configures a fail point.
- **`features = ["failpoints"]` is required** — without it, `fail_point!(...)` calls compile to no-ops AND `fail::cfg` becomes a no-op-on-the-name (silent test bypass). This is the most common foot-gun for first-time `fail` users.

Production-code instrumentation lives in source files (e.g., a future `crates/orgsidian-vault/src/lib.rs` will sprinkle `fail::fail_point!("atomic-write::after-tmp-rename");` between the tmp-rename and the commit). Story 1.11 does NOT add any such call site — those are owned by Epic 3+ stories. Story 1.11 only adds the harness consumers + the dev-dep + the commented-out preview in `disk_full_atomic_write`.

### §7 — Previous-story intelligence (Story 1.10 + 1.9)

Story 1.10 (just-merged, status: `review`) established:
- The hygiene-docs pattern (terse, pointer-shaped) for `SECURITY.md` / `ARCHITECTURE.md` / `CHANGELOG.md` / `CONTRIBUTING.md`. Story 1.11's `docs/failure-modes/coverage-matrix.md` reuses the same pattern: a single self-describing artifact, no cross-doc backlinks needed.
- The `[[project_orgsidian_github_label_scheme]]` discipline: `status:in-review` not `status:review`. Applied via AC8 here.
- The `docs:`-vs-`chore:` commit-type decision-grade question pattern — Story 1.11 uses `feat(test):` per the Story 1.9 convention (`feat(test): add anchor smoke tests ...`) since this is new test infrastructure, NOT a `docs:` change. The `docs/failure-modes/coverage-matrix.md` artifact is a byproduct of the test-infrastructure addition, not the headline change.

Story 1.9 (`done`) established:
- The anchor-smoke discipline: minimal, real-code-path tests that prevent CI placebo-green. Story 1.11 reuses the discipline metaphorically — placeholders that panic on accidental green are the "anti-placebo" guard for fault paths.
- `crates/orgsidian-core/Cargo.toml [features] test-support = []` — the gating convention that AC2's `required-features` clause leverages.
- The `feat(test): ...` commit-type pattern.

### §8 — Git-history intelligence (last 5 commits, 2026-05-25)

```
29e43b7 docs: add SECURITY.md / ARCHITECTURE.md / CHANGELOG.md / CONTRIBUTING.md (Story 1.10, closes #10)
95728b4 feat(test): add anchor smoke tests (parser/vault/watcher, anti-placebo) (Story 1.9, closes #9) (#119)
9010f89 fix(ci): skip shell-ui build steps on windows-2022 nightly (#124)
0f22d8a fix(ci): nightly windows shell + arch git safe.directory (#123)
0decd86 fix(ci): nightly windows + arch — module-cfg gate + missing npm pkg (#122)
```

Patterns to absorb:
- **Commit type for this story**: `feat(test):` per Story 1.9's pattern (anchor-smoke / harness scaffolding qualifies as `feat:` of test infrastructure; not `chore:` because LD-54's `chore` is "miscellaneous tasks", and this story is a named architectural deliverable). The Story 1.10 `docs:` commit is a clear contrast — that story produced documentation; this story produces test code.
- **Commit message headline**: `feat(test): add LD-41 failure-mode harness + coverage gate + matrix generator (Story 1.11, closes #11)`.
- **Single PR per story** continues.
- **No co-author trailers** per [[feedback_no_co_author_credit]].
- **Review fixup pattern**: Story 1.7 = 1 fixup; Story 1.8 = 13 fixups (large surface); Story 1.9 = 3 patches + 4 deferred (test infrastructure); Story 1.10 = 0 fixups (docs-only). Story 1.11 is harness scaffolding + tiny generator script: expect 1–3 fixups, primarily on test-fn names or `#[ignore]` annotation precision.

### §9 — LLM-dev-agent anti-pattern checklist

Common dev-agent mistakes this story spec intentionally guards against:

1. **DO NOT enumerate only 9 categories.** The architecture catalog has 10 rows (the 10th is "Refile partial completion" added 2026-05-20). Honor the architecture, not the stale epic AC text. See §3.
2. **DO NOT use `Ok(())` or empty test bodies as placeholders.** Use `unimplemented!(...)` so accidental `#[ignore]` removal panics loudly. See §5.
3. **DO NOT paraphrase the `#[ignore = "implemented in Epic N"]` string.** The coverage gate (AC3) + the matrix generator (AC4) both parse the literal `implemented in Epic N` substring. Variants like `"to be implemented in Epic N"` break the parser.
4. **DO NOT add `fail_point!(...)` calls to any production source crate.** This story is harness-only; production instrumentation is owned by Epic 3+ stories per LD-41 + test-design.md §6.7.
5. **DO NOT enable `ORGSIDIAN_FAILURE_MODE_STRICT=1` in CI.** Strict mode is the v0.5-Beta-era flip; Story 1.11 ships in advisory mode only. See AC6.
6. **DO NOT create `crates/test-plugin-panic/`.** It's an LD-38 chaos plugin owned by a later Epic 1 story or Epic 12 v1.0 hardening. The `plugin_init_panic_isolated` + `plugin_on_event_panic_isolated` placeholders reference it as future work.
7. **DO NOT omit `features = ["failpoints"]` from the `[dev-dependencies]` line.** Without it the `fail` crate compiles to no-ops and downstream tests silently bypass fault injection. See §6.
8. **DO NOT alphabetize categories in the matrix generator (AC4).** Top-down source order, NOT alphabetical — a future contributor reading the matrix expects categories in the same order they appear in `failure_modes.rs`.
9. **DO NOT add backlinks from README.md / ARCHITECTURE.md / CONTRIBUTING.md to the new `docs/failure-modes/coverage-matrix.md`.** That's scope creep; the matrix is self-describing and discoverability is a deferred-follow-up concern.
10. **DO NOT use lowercase / mixed-case directory names.** `docs/failure-modes/` (lowercase + hyphenated) follows the existing `docs/security/` convention.
11. **DO NOT add `Co-Authored-By:` trailers or "Generated with Claude Code" footers** to the commit / PR / Issue. Per [[feedback_no_co_author_credit]].
12. **DO NOT silently add a new `deny.toml [advisories].ignore` entry** if `cargo deny check` surfaces a new advisory on `fail` 0.5.x. Surface as decision-grade question per AC5.
13. **DO NOT use `HashMap` / `Object.keys()` for category iteration** in either the coverage gate (Rust) or the matrix generator (JS). Use `Vec` / Array in source order. See §8.
14. **DO NOT bump existing dependencies** (e.g., `tracing`, `thiserror`, `tauri-*`) as a side effect of adding `fail`. If `cargo update` surfaces unrelated bumps, halt + surface.

### §10 — Cross-platform sanity check

- **Line endings**: repo uses LF. New `.rs` + `.mjs` + `.md` files MUST be LF (verify via `file <path>` showing `ASCII text` not `with CRLF line terminators`).
- **`tests/` directory at workspace root**: case-sensitive on Linux ext4 / GitHub UI, case-preserving-on-mac. Use lowercase `tests/` (matches the Cargo convention).
- **`include_str!("./failure_modes.rs")` in the coverage gate**: path is relative to the `tests/failure_modes_coverage.rs` source file (same directory), so `"./failure_modes.rs"` resolves correctly on macOS-arm64 + Ubuntu + Windows. Cargo's relative `[[test]] path` resolution is independent of `include_str!` resolution; the latter uses the source-file-directory convention. Verified by reading the Rust reference for `include_str!`.
- **`fail` crate's Windows support**: 0.5.x supports Windows (TiKV uses it cross-platform). No platform-specific path tweaks needed.
- **`pnpm gen:failure-modes-matrix` output line endings**: Node's `fs.writeFileSync(stdout)` emits OS-native by default. The script MUST explicitly emit `\n` line breaks (not `os.EOL`) to keep the committed file deterministic across the macOS-arm64 / Ubuntu / Windows-nightly matrix. Use template literals with explicit `\n` only.

### §11 — Architecture decision references (LD anchors)

Critical LD references this story implements / surfaces:
- **LD-41** ([architecture.md#L1196-L1209](_bmad-output/planning-artifacts/architecture.md#L1196-L1209)) — Failure mode catalog (10 rows; the source of truth for AC1).
- **LD-32** ([architecture.md#L530](_bmad-output/planning-artifacts/architecture.md#L530), [#L1097-L1101](_bmad-output/planning-artifacts/architecture.md#L1097-L1101)) — CI matrix; the new `[[test]]` targets auto-attach to the existing `cargo test --workspace` step in [.github/workflows/pr.yml](.github/workflows/pr.yml).
- **LD-37** ([architecture.md#L1430-L1434](_bmad-output/planning-artifacts/architecture.md#L1430-L1434)) — Supply-chain gates (`cargo deny` + `cargo audit`); both MUST stay green with the new `fail` dep (AC5).
- **LD-38** ([architecture.md#L1172-L1179](_bmad-output/planning-artifacts/architecture.md#L1172-L1179)) — Plugin panic isolation; the `plugin_init_panic_isolated` + `plugin_on_event_panic_isolated` placeholders are scaffolds for the LD-38 chaos plugin tests.
- **LD-54** ([architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)) — Conventional Commits; `feat(test): ...` per §8.
- **LD-57** ([architecture.md#L1344-L1353](_bmad-output/planning-artifacts/architecture.md#L1344-L1353)) — Refile cross-file atomicity; the `refile_partial_completion_rollback` placeholder is the 10th LD-41 category added by this LD's 2026-05-20 addendum.
- **NFR-15 / NFR-16** (PRD §8) — failure-mode coverage non-functional requirements, gated by this harness.
- **Process Discipline rule A** ([epics.md#L290-L294](_bmad-output/planning-artifacts/epics.md#L290-L294)) — red-phase ATDD merge-gate; the harness IS the red-phase scaffold for every downstream LD-41-touching story.
- **Process Discipline rule H** ([epics.md#L347-L349](_bmad-output/planning-artifacts/epics.md#L347-L349)) — test-design.md as authoritative system-level test strategy; §6.7 is the source for the matrix-generator mechanism column.

### §12 — Memory-anchored conventions (cross-cutting)

- **[[feedback_no_co_author_credit]]**: No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on commit / PR / Issue.
- **[[user_contact_email]]**: `tiz.basile@gmail.com` (Cargo.toml pin is authoritative); new files do NOT add a personal contact header.
- **[[feedback_version_policy]]**: `fail = "0.5"` is the latest stable (0.5.1) per docs.rs. Caret version acceptable; Tauri-exact-pin discipline does not apply.
- **[[feedback_batch_fixes_terse]]**: post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface. The 9-vs-10-categories question (§3) is the primary one this story carries; the workspace-member-vs-orgsidian-core hosting question (§2) is secondary.
- **[[project_orgsidian_github_label_scheme]]**: status label is `status:in-review` (NOT `status:review`).
- **[[project_orgsidian_github_plan]]**: GitHub Free plan = no branch protection enforcement; required-checks list in `pr.yml` is advisory only. The harness's new `[[test]]` targets become part of that advisory list automatically.
- **[[project-orgsidian-repo-public-during-pre-alpha]]**: repo is already public; the new `docs/failure-modes/coverage-matrix.md` is visible to any visitor immediately on merge. Tone the file accordingly (no internal-only references).

### Project Structure Notes

- **New files** (4): `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, `scripts/gen-failure-modes-matrix.mjs`, `docs/failure-modes/coverage-matrix.md`.
- **New directories** (2): `tests/` at workspace root, `docs/failure-modes/`.
- **Modified files** (3 + lockfile): `Cargo.toml` (workspace deps), `crates/orgsidian-core/Cargo.toml` (dev-dep + 2 `[[test]]` blocks), `package.json` (1 script entry), `Cargo.lock` (transitive fail deps).
- **No new crates**. The `[[test]]` declarations live in the existing `orgsidian-core` per §2.
- **No new workspace members**. (See §2 for the alternative-decision-grade-question framing.)
- **No `.github/workflows/*` touches**. The new tests auto-attach to the existing CI step.

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L584-L599](_bmad-output/planning-artifacts/epics.md#L584-L599) (Story 1.11 AC verbatim)
- LD-41 failure mode catalog (10 rows, source of truth): [_bmad-output/planning-artifacts/architecture.md#L1196-L1209](_bmad-output/planning-artifacts/architecture.md#L1196-L1209)
- Test-design.md §6.7 Chaos / Fault Injection layer + LD-41 Coverage Matrix mechanism column: [_bmad-output/test-artifacts/test-design.md#L568-L592](_bmad-output/test-artifacts/test-design.md#L568-L592)
- Test-design.md §5.1 directory layout (workspace-root `tests/` placement): [_bmad-output/test-artifacts/test-design.md#L298-L313](_bmad-output/test-artifacts/test-design.md#L298-L313)
- Test-design.md §7.3.14 Story 1.11 red-phase scaffold (`fail::cfg(...)` exemplar): [_bmad-output/test-artifacts/test-design.md#L996-L1027](_bmad-output/test-artifacts/test-design.md#L996-L1027)
- Story 11.8 reference to harness's Refile category (10th row dependency): [_bmad-output/planning-artifacts/epics.md#L2155-L2160](_bmad-output/planning-artifacts/epics.md#L2155-L2160)
- LD-57 Refile cross-file atomicity (10th row source): [_bmad-output/planning-artifacts/architecture.md#L1344-L1353](_bmad-output/planning-artifacts/architecture.md#L1344-L1353)
- Process Discipline rule A (red-phase ATDD): [_bmad-output/planning-artifacts/epics.md#L286-L294](_bmad-output/planning-artifacts/epics.md#L286-L294)
- Process Discipline rule H (test-design pointer): [_bmad-output/planning-artifacts/epics.md#L347-L349](_bmad-output/planning-artifacts/epics.md#L347-L349)
- `fail` crate v0.5.1 docs (latest stable, 2026-05): https://docs.rs/fail/latest/fail/
- Existing `orgsidian-core` Cargo.toml (host crate for `[[test]]` declarations): [crates/orgsidian-core/Cargo.toml](crates/orgsidian-core/Cargo.toml)
- Existing root workspace Cargo.toml (where `fail` joins `[workspace.dependencies]`): [Cargo.toml](Cargo.toml)
- Existing CI step that auto-discovers new `[[test]]` targets: [.github/workflows/pr.yml](.github/workflows/pr.yml)
- Existing `invoke_plugin_hook!` macro (LD-38, future consumer of the plugin-panic placeholders): [crates/orgsidian-core/src/registry.rs](crates/orgsidian-core/src/registry.rs)
- Previous story (1.10): [_bmad-output/implementation-artifacts/1-10-add-security-md-architecture-md-changelog-md-contributing-md.md](_bmad-output/implementation-artifacts/1-10-add-security-md-architecture-md-changelog-md-contributing-md.md)
- Story 1.9 anchor smoke (pattern reference): [_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md](_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md)
- Existing anchor test pattern (`tests/anchor.rs` style for placeholder bodies): [crates/orgsidian-parser/tests/anchor.rs](crates/orgsidian-parser/tests/anchor.rs)
- Existing `cargo deny` ledger (advisory-exceptions): [docs/security/advisory-exceptions.md](docs/security/advisory-exceptions.md)
- Existing `package.json` scripts entry point: [package.json](package.json)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context) via `bmad-dev-story` (claude-opus-4-7).

### Debug Log References

AC7 dev-box verification matrix — all substantive checks green on macOS-arm64:

| Cell | Result |
|---|---|
| `ls tests/failure_modes{,_coverage}.rs` | both present |
| `ls docs/failure-modes/coverage-matrix.md` | present |
| `ls scripts/gen-failure-modes-matrix.mjs` | present |
| `grep -c '#\[ignore = "implemented in Epic' tests/failure_modes.rs` | `10` |
| `grep -c 'unimplemented!' tests/failure_modes.rs` | `10` |
| `grep -c 'fail::cfg\|fail::FailScenario' tests/failure_modes.rs` | `2` (≥1, comment-only OK) |
| `grep -c 'ORGSIDIAN_FAILURE_MODE_STRICT' tests/failure_modes_coverage.rs` | `2` (≥1) |
| `grep -c 'regenerated by: pnpm gen:failure-modes-matrix' docs/failure-modes/coverage-matrix.md` | `1` (≥1) |
| `pnpm gen:failure-modes-matrix` + `git diff --exit-code` | exit 0 (idempotent) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo build --workspace --locked` | exit 0 |
| `cargo test --workspace --locked` | exit 0; placeholders run as `ignored` (workspace feature unification activates `test-support` via `orgsidian-watcher [dev-dependencies]`) |
| `cargo test --workspace --locked -- --include-ignored` panic count | `10` matched via `^thread .* panicked` (see note below) |
| `cargo test -p orgsidian-core --test failure_modes_coverage --locked` | exit 0; advisory `eprintln!` visible |
| `ORGSIDIAN_FAILURE_MODE_STRICT=1 cargo test -p orgsidian-core --test failure_modes_coverage --locked` | exit non-zero; lists all 10 categories |
| `cargo deny check` | exit 0 (advisories/bans/licenses/sources ok) |
| `cargo audit` | exit 0 (no new advisories from `fail` 0.5.1) |

Two literal-spec drifts in AC7 worth flagging for future dev-story templates (substantive behavior unaffected):

1. **`^test .* panicked` regex** — the panic banner Cargo emits with `--include-ignored` is `thread '<name>' (<id>) panicked at …`, not `test … panicked`. Corrected regex `^thread .* panicked` matches `10` as the AC intends.
2. **`cargo deny check --locked` / `cargo audit --locked`** — neither subcommand accepts `--locked` at that position in their current CLI; the flag belongs to `cargo`-driven commands, not to these standalone binaries. Substantive run without `--locked` is what CI uses today and is clean.

### Completion Notes List

- **Decision-grade question (carried per [[feedback_batch_fixes_terse]]):** AC1 enumerates **10** categories honoring the current architecture LD-41 catalog ([architecture.md#L1196-L1209](../planning-artifacts/architecture.md#L1196-L1209)), not the **9** stated in the epic AC text ([epics.md#L593](../planning-artifacts/epics.md#L593)). The 10th row ("Refile partial completion") was added by the 2026-05-20 closed-loop addendum and downstream Story 11.8 already depends on it. Surfaced here for explicit reviewer confirmation (see Dev Notes §3 of the story spec).
- **Hosting decision:** `[[test]]` declarations live in `crates/orgsidian-core/Cargo.toml` (Dev Notes §2 default), not a new `crates/orgsidian-failure-modes/` member. Cleaner: 0 new crates, wiring is one Cargo.toml block.
- **`test-support` activation under `cargo test --workspace`:** `failure_modes` carries `required-features = ["test-support"]` (per AC2), but `cargo test --workspace --locked` still discovers/runs it because `orgsidian-watcher` already enables `orgsidian-core/test-support` in its dev-dependencies — workspace feature unification cascades that to all crates. No CI changes required.
- **Placeholder bodies for plugin panics** reference "LD-38 chaos plugin (future Epic 1 story)" rather than a literal `Story N.M` because no specific story number is assigned yet to the `crates/test-plugin-panic-*` chaos crates (per AC6 — that work lands in a future Epic 1 story or Epic 12 v1.0 hardening).
- **`fail` v0.5.1 transitive deps** all pre-existed in `Cargo.lock` (libc / log / getrandom / rand_* are already pulled by tauri); the lock diff is a single `+name = "fail"` block. `cargo deny` + `cargo audit` clean — no new advisories, licenses, or banned sources introduced.
- **Strict-mode gate is wired but NOT activated** (AC6). The env-var hook (`ORGSIDIAN_FAILURE_MODE_STRICT=1`) is the seam a future v0.5-Beta release-prep story will flip in `pr.yml`/`nightly.yml`.
- **GitHub Issue/PR sync:** Issue #11 transitioned `status:backlog` → `status:in-progress` at start; → `status:in-review` is owned by the workflow `on_complete` step alongside the PR open with `Closes #11`.

### File List

**New (4):**
- `tests/failure_modes.rs`
- `tests/failure_modes_coverage.rs`
- `scripts/gen-failure-modes-matrix.mjs`
- `docs/failure-modes/coverage-matrix.md`

**Modified (3 + lockfile):**
- `Cargo.toml` — added `fail = "0.5"` to `[workspace.dependencies]`
- `crates/orgsidian-core/Cargo.toml` — added `[dev-dependencies]` block with `fail` + two `[[test]]` targets pointing at workspace-root harness/gate
- `package.json` — added `gen:failure-modes-matrix` script
- `Cargo.lock` — single new `fail v0.5.1` entry (no version bumps to existing deps)

**Workflow artefacts (also updated):**
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 1-11 → `in-progress`, will move to `review` at workflow `on_complete`
- `_bmad-output/implementation-artifacts/1-11-establish-ld-41-failure-mode-test-harness-party-mode-round-2-p0-murat.md` — Status, Tasks, Dev Agent Record, Change Log

## Change Log

| Date       | Change                                                                                                  | Author                                |
| ---------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-25 | Story 1.11 contextualized via `bmad-create-story` (ready-for-dev).                                      | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-26 | Implementation via `bmad-dev-story`: LD-41 harness (10 placeholders) + coverage gate + matrix generator. | Amelia (`bmad-dev-story`) for Tiziano |
