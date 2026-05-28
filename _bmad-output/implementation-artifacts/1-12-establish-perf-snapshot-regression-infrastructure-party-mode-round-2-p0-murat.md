# Story 1.12: Establish perf snapshot regression infrastructure (Party Mode round 2 P0 — Murat)

Status: done

## Metadata

github_issue: 12

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want a single shared `assert_no_perf_regression!` macro consumed by every perf-sensitive story (with median-of-5 sampling, ±20% gate vs a JSON baseline, missing-baseline-mode bootstrap, and TC-2 `runner_class` scoping baked in from day 1) plus a companion `docs/perf/targets.md` documenting the PRD §8 NFR absolute targets separately from the regression gate,
so that absolute-number perf AC do not create flaky CI on heterogeneous hardware and LD-32 ±10% regression discipline is uniform across the codebase from Story 4.3a onwards.

## Acceptance Criteria

**AC1 — `crates/orgsidian-core/src/test_support/perf.rs` exposes the macro + impl.**

- File path: `crates/orgsidian-core/src/test_support/perf.rs` (NEW file — verified missing via `ls crates/orgsidian-core/src/test_support/` which returns only `clock.rs` + `mod.rs`).
- The module is gated identically to `clock.rs` — it lives under the existing `#[cfg(any(test, feature = "test-support"))]` `test_support` module ([crates/orgsidian-core/src/lib.rs:16-17](crates/orgsidian-core/src/lib.rs#L16-L17)). Update [crates/orgsidian-core/src/test_support/mod.rs:7](crates/orgsidian-core/src/test_support/mod.rs#L7) by adding `pub mod perf;` immediately after the existing `pub mod clock;` line; the stale forward-reference comment ("Story 1.12 will add `pub mod perf;` alongside it") in [crates/orgsidian-core/src/test_support/mod.rs:4](crates/orgsidian-core/src/test_support/mod.rs#L4) MUST be updated to past tense ("Story 1.12 adds the `perf` submodule alongside it") so future readers see the surface as already realized.
- Module exposes:
  - `pub struct PerfReport { /* fields per AC2 */ }` — the structured result returned by the impl (for self-tests + future tooling).
  - `pub enum PerfOutcome { BaselineWritten, WithinTolerance, Regressed }` — the discriminant used by self-tests to assert behavior without parsing panic strings.
  - `pub fn assert_no_perf_regression_impl(story_id: &str, baseline_path: &Path, samples_ns: &[u128]) -> PerfReport` — pure-policy entrypoint (no timing, no closure execution). Callers pre-compute the 5 samples; the impl handles median + baseline I/O + comparison + warning emission. This split is non-negotiable per Dev Notes §3 (self-testability without sleep-based flakiness).
  - `#[macro_export] macro_rules! assert_no_perf_regression { ... }` — the public surface consumed by perf-AC stories. Wraps `assert_no_perf_regression_impl` with the 5-sample timing harness via `std::time::Instant`. Hoisted to crate root via `#[macro_export]` so consumers write `use orgsidian_core::assert_no_perf_regression;` (matching the [test-design.md §7.3.10 scaffold](_bmad-output/test-artifacts/test-design.md#L909-L926) literal `use orgsidian_core::test_support::perf::assert_no_perf_regression;`). The macro re-export path MUST work via BOTH `orgsidian_core::assert_no_perf_regression` AND `orgsidian_core::test_support::perf::assert_no_perf_regression` so the scaffold's literal import line compiles unchanged. See Dev Notes §4 for the `pub use` trick.

**AC2 — Macro semantics: median-of-5, ±20% tolerance, missing-baseline bootstrap, `runner_class` scoping (TC-2).**

The macro and its impl MUST implement these semantics exactly (drift is a review-block):

1. **Sampling**: macro runs `op` exactly **5 times** sequentially in the same process, recording elapsed nanoseconds for each invocation via `std::time::Instant::now() / .elapsed()`. The samples vector is `Vec<u128>` (nanoseconds; `Duration::as_nanos()` returns `u128`). No warm-up discard — all 5 samples count. This matches the LD-32 ([architecture.md#L523](_bmad-output/planning-artifacts/architecture.md#L523), [test-design.md §6.9](_bmad-output/test-artifacts/test-design.md#L606-L626)) spec verbatim.
2. **Aggregation**: median (NOT mean — median is robust to GHA noise spikes per [test-design.md §6.9](_bmad-output/test-artifacts/test-design.md#L608)). For 5 samples, sort ascending and pick index 2.
3. **Tolerance**: **20%** above baseline (`measured_median > baseline_median * 120 / 100` using integer arithmetic in `u128` to avoid float drift across platforms). The 20% is the [architecture.md L331](_bmad-output/planning-artifacts/architecture.md#L331) macro spec value — distinct from LD-32's per-PR-gate-headline ±10% (which is the *trend gate* target, not the per-invocation per-story tolerance). Do NOT use 10%; the epic AC text and architecture line 331 + 611 both lock 20%.
4. **`runner_class` scoping (TC-2 addendum, [test-design.md L144-150](_bmad-output/test-artifacts/test-design.md#L144-L150))**: baselines are per-`(story_id, runner_class)` tuple. The JSON file shape is:
   ```json
   {
     "story_id": "story-1.12-self-test-canary",
     "tolerance_pct": 20,
     "samples": 5,
     "baselines": {
       "macos-arm64": {
         "median_ns": 12345678,
         "created_at": "2026-05-28T10:00:00Z"
       },
       "linux-x86_64": {
         "median_ns": 23456789,
         "created_at": "2026-05-28T10:00:00Z"
       }
     }
   }
   ```
   `runner_class` derivation: prefer `format!("{}-{}", env::consts::OS, env::consts::ARCH)` (e.g., `macos-aarch64`, `linux-x86_64`) — these are compile-time constants and identical on local dev box + CI runners of the same class. Do NOT use `RUNNER_OS` / `RUNNER_ARCH` env vars (set only on GitHub Actions; would make local runs incomparable across the CI⇄dev boundary and is an unnecessary divergence point). See Dev Notes §5.
5. **Missing-baseline modes** (the impl returns `PerfOutcome::BaselineWritten` in BOTH cases):
   - File does not exist at `baseline_path` → impl creates parent dir if needed, writes a new JSON with `baselines: { <current_runner_class>: { median_ns, created_at: <ISO-8601 UTC> } }`, emits ONE-LINE `eprintln!` warning `"perf: writing initial baseline for {story_id} on {runner_class} ({median_ns} ns) — RE-RUN required for regression gating"`, returns `Ok(PerfReport { outcome: BaselineWritten, ... })`.
   - File exists but lacks the current `runner_class` entry → impl reads the file, INSERTS the new `runner_class` entry (preserving existing entries verbatim — line-by-line file rewrite is acceptable; see Dev Notes §6), emits the same one-line warning, returns `BaselineWritten`.
   - **Both missing-baseline modes are non-fatal** per the epic AC ("missing-baseline mode (first run) writes the baseline file and emits a non-fatal warning"). The macro does NOT panic; the test passes.
6. **Within-tolerance**: returns `PerfOutcome::WithinTolerance` silently (no `eprintln!`; the caller's `#[test]` passes with no extra noise).
7. **Regression**: the IMPL returns `PerfOutcome::Regressed` (does NOT panic — separation of policy from caller's choice). The MACRO `panic!`s with the message format `"perf regression: {story_id} on {runner_class}: measured median {measured_ns} ns exceeds baseline {baseline_ns} ns by {pct}% (tolerance: 20%, samples: {samples})\nBaseline file: {baseline_path}"` (single string, no ANSI). The split means impl self-tests assert via `outcome ==` without `#[should_panic]` brittleness; the macro is what consumer stories use, and `panic!` is the right shape there.
8. **JSON I/O**: use `serde_json` (already in `[workspace.dependencies]` per [Cargo.toml#L38](Cargo.toml#L38)). Add `serde_json = { workspace = true, optional = true }` to `crates/orgsidian-core/Cargo.toml [dependencies]` and tie it to the `test-support` feature: `test-support = ["dep:serde_json"]`. See AC4 + Dev Notes §7. Do NOT hand-roll JSON parsing.

**AC3 — Workspace-root `tests/perf-baselines/` directory established + path-resolution discipline.**

- Directory: `tests/perf-baselines/` (NEW — at workspace root, NOT under any crate). Verified missing via `ls tests/` which shows only `failure_modes.rs` + `failure_modes_coverage.rs` (Story 1.11 outputs).
- Marker file: `tests/perf-baselines/.gitkeep` (empty file; commit so the directory persists even with no baselines yet).
- README file: `tests/perf-baselines/README.md` (NEW; <30 lines) MUST contain:
  - One-line purpose: "Per-story performance baselines consumed by `assert_no_perf_regression!` (Story 1.12)."
  - The JSON shape spec verbatim from AC2.5 (copy-paste so future contributors don't have to grep this story file).
  - A pointer to `docs/perf/targets.md` (the absolute-target reference) and to the [test-design.md §6.9](_bmad-output/test-artifacts/test-design.md#L606-L626) layer 9 spec.
  - A one-line note: "Baselines are auto-written on first run (missing-baseline mode). Commit the JSON file alongside the perf-AC story PR that introduced it."
- **Path-resolution convention** (CRITICAL — most likely dev-agent foot-gun per Dev Notes §8): the `baseline_path` argument to the macro is a **relative path from the workspace root**, NOT from `CARGO_MANIFEST_DIR`. The impl MUST resolve it via a `workspace_root()` helper that walks up from `env!("CARGO_MANIFEST_DIR")` (compile-time env, available unconditionally in proc-macro-free Rust) looking for the first `Cargo.toml` containing a `[workspace]` table. This makes baseline paths uniform regardless of which crate's `tests/` directory hosts the consumer.
- The `workspace_root()` helper:
  ```rust
  fn workspace_root() -> std::path::PathBuf {
      let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
      loop {
          let cargo_toml = dir.join("Cargo.toml");
          if cargo_toml.exists() {
              let txt = std::fs::read_to_string(&cargo_toml).unwrap_or_default();
              if txt.contains("[workspace]") {
                  return dir;
              }
          }
          if !dir.pop() {
              panic!("perf: could not find workspace root from CARGO_MANIFEST_DIR={}", env!("CARGO_MANIFEST_DIR"));
          }
      }
  }
  ```
  This is `pub(crate)`-visible (NOT `pub`) — consumer stories MUST use the macro, NOT call the helper directly.

**AC4 — `Cargo.toml` + `crates/orgsidian-core/Cargo.toml` dependency wiring.**

- `crates/orgsidian-core/Cargo.toml` updates:
  ```toml
  # Append to existing [features] block (verified shape at L10-L14):
  [features]
  # Story 1.9 (LD-9): opt-in surface for the `test_support` module ...
  # Story 1.12 (LD-32 / NFR-20): perf baseline JSON I/O is gated under the same
  # test-support feature so production builds exclude serde_json from the leaf
  # crate's transitive set.
  test-support = ["dep:serde_json"]
  ```
  ```toml
  # Append to existing [dependencies] block:
  # Story 1.12 (LD-32 / NFR-20): JSON I/O for perf baselines under tests/perf-baselines/.
  # `optional = true` + the `dep:serde_json` activator on the `test-support` feature
  # (above) keep this OUT of production binary builds. Pin floats on 1.x (workspace dep).
  serde_json = { workspace = true, optional = true }
  ```
  ```toml
  # Append to existing [dev-dependencies] block (verified at L28-L31, currently only `fail`):
  # Story 1.12: self-tests for the perf macro use tempdir-scoped baselines so
  # parallel test runs do not collide on shared baseline files. Pattern matches
  # orgsidian-watcher's existing `tempfile = "3"` dev-dep.
  tempfile = "3"
  ```
- Root `Cargo.toml` updates: NONE expected. `serde_json` is already at `[workspace.dependencies]` ([Cargo.toml#L38](Cargo.toml#L38) `serde_json = "1"`). `tempfile` is NOT yet at the workspace level (`orgsidian-watcher` declares it bare-string per its Cargo.toml). Continue that convention: bare `tempfile = "3"` in orgsidian-core's `[dev-dependencies]`, NOT a new `[workspace.dependencies]` entry. Per [[feedback_version_policy]], `tempfile` 3.x is the current latest-stable major; caret on `"3"` is acceptable.
- Lockfile expectation: `Cargo.lock` updates with `tempfile` + its transitive deps (already pulled in transitively by other deps; expect zero or minimal new lockfile entries). `serde_json` already locked. Commit the updated lockfile per LD-37.
- `cargo deny check` + `cargo audit` MUST stay clean. Neither `tempfile` nor `serde_json` are surprises (both well-vetted, dual-licensed Apache-2.0/MIT in the allowlist). If either surfaces an advisory, surface as decision-grade question per [[feedback_batch_fixes_terse]] — do NOT silently add a new `deny.toml [advisories].ignore` entry.

**AC5 — `docs/perf/targets.md` documents absolute targets separately from the regression gate.**

- Directory: `docs/perf/` (NEW — verified via `ls docs/` which shows `failure-modes/`, `logo-draft.png`, `security/`).
- File: `docs/perf/targets.md` (NEW; ~60-80 lines).
- Required sections (in this order):
  1. **Purpose statement** (3-4 lines): absolute perf budgets from [PRD §8 NFRs](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) (calibrated to 2020+ M1 / equivalent x86_64 hardware) are documented HERE as design targets. They are NOT the regression gate. The gate is `assert_no_perf_regression!` ±20% on median of 5 vs the committed baseline. The relationship: absolute targets define "is this story's baseline acceptable at all"; the regression gate defines "did the next change break it." Reference [architecture.md L331](_bmad-output/planning-artifacts/architecture.md#L331).
  2. **Target table** (Markdown table, columns: NFR | Surface | Target | Calibration vault size | First-baseline owner story | LD/Architecture anchor). Required rows (cross-checked against [test-design.md §6.9 + §10.4](_bmad-output/test-artifacts/test-design.md#L606-L626) and [architecture.md L45](_bmad-output/planning-artifacts/architecture.md#L45)):

      | NFR | Surface | Target | Calibration | Owner | Anchor |
      |---|---|---|---|---|---|
      | NFR-1 | App startup (cold) | <2s | 1000-file vault | Story 13.x polish | architecture.md L45 |
      | NFR-2 | Editor typing latency | <30ms | n/a | Story 4.3a-g | architecture.md L45 |
      | NFR-3 | Agenda recompute (incremental) | <100ms | 1000-file vault | Story 7.1 + 7.4 | architecture.md L45 |
      | NFR-4 (split per PRD §4.3 FR-12) | FTS5 search — first 10 results | <100ms | 1000-file vault | Story 8.4 | epics.md L1687 |
      | NFR-4 (split) | FTS5 search — full 50 results | <200ms | 1000-file vault | Story 8.4 | epics.md L1687 |
      | NFR-5 | Quick Capture end-to-end (hotkey → persist) | <1s | n/a | Story 8.1 | epics.md L1639 |
      | NFR-6 | Editor open 5000-line `.org` | <300ms | n/a | Story 4.3g | test-design.md §3.1 R-015 |
      | NFR-7 | Resident memory | <500MB | 1000-file vault | Story 4.9 (soak) | architecture.md L45 |
      | FR-14 | Project Report typical scope | <5s | 50 headlines, 4 weeks | Story 10.6 | epics.md L2005 |
      | FR-26 / LD-56 | Graph View 5k-node force-directed render | ≤2s | 5000 nodes | Story 8.11 | architecture.md §LD-56 |
      | FR-26 / LD-56 | Graph View steady-state frame | ≤500ms | 5000 nodes | Story 8.11 | architecture.md §LD-56 |
      | — | Merge Dialog open | ≤2s | conflict-detection-trigger | Story 9.1 | epics.md L1849 |

  3. **Gate-vs-Target rationale** (2-3 paragraphs): why the macro uses ±20% relative gate, not absolute thresholds. Reference [test-design.md L201-L218](_bmad-output/test-artifacts/test-design.md#L201-L218) (Story 1.12 design rationale) + the TC-2 hardware-heterogeneity argument.
  4. **Baseline workflow** (numbered list, 4-6 items): the consumer-story lifecycle — (a) write the perf-AC test with `assert_no_perf_regression!`, (b) first run on a clean baseline writes the JSON file, (c) commit the baseline JSON alongside the story PR, (d) subsequent PRs run the same test against the committed baseline, (e) if a deliberate perf change shifts the baseline, update the JSON in the same PR with a `perf:` Conventional Commit type (per [architecture.md L606](_bmad-output/planning-artifacts/architecture.md#L606) + LD-54). One-sentence on `runner_class`: baselines are per-runner-class; the first CI runner of each class to see the test writes its own entry.
  5. **Self-describing footer**: a one-line note `<!-- Regeneration: this file is hand-curated; update on any PRD §8 NFR change or addition of a perf-AC story. -->`.

**AC6 — Macro is consumed by perf-AC stories (forward-reference scaffolding ONLY in this story).**

This story does NOT implement the perf gates inside downstream stories. It establishes the macro + baseline directory + targets doc, and adds **one canary self-test** that exercises the macro end-to-end. The forward-reference list (for downstream story authors) is:

| Story | Baseline JSON path | Notes |
|---|---|---|
| 4.3g (source-position fidelity) | `tests/perf-baselines/story-4.3g-source-fidelity.json` | epics.md L1144 |
| 6.3 (Today Agenda) | `tests/perf-baselines/story-6.3-today-agenda.json` | initial baseline ≤500ms / 1k-vault per NFR-3 |
| 7.1 (Today Dashboard) | `tests/perf-baselines/story-7.1-today-dashboard.json` | epics.md L1499; initial baseline ≤500ms / 1k-vault per NFR |
| 8.1 (Quick Capture e2e) | `tests/perf-baselines/story-8.1-capture-e2e.json` | epics.md L1639; initial baseline ≤1s per NFR-5 |
| 8.4 (search — two gates) | `tests/perf-baselines/story-8.4-search-10results.json` AND `tests/perf-baselines/story-8.4-search-50results.json` | epics.md L1687 — two separate baselines per FR-12 two-tier (≤100ms first-10 + ≤200ms full-50) |
| 8.11 (graph — two gates) | `tests/perf-baselines/story-8.11-graph-5k-render.json` AND `tests/perf-baselines/story-8.11-graph-steady-frame.json` | epics.md L614 LD-56; ≤2s render + ≤500ms steady-state |
| 9.1 (Merge Dialog open) | `tests/perf-baselines/story-9.1-merge-dialog-open.json` | epics.md L1849; initial baseline ≤2s |
| 10.6 (Project Report typical) | `tests/perf-baselines/story-10.6-report-typical-scope.json` | epics.md L2005; initial baseline ≤5s |
| 11.8 (Refile orchestrator round-trip) | `tests/perf-baselines/story-11.8-refile-orchestrator.json` | epics.md L601 forward-reference (initial baseline TBD by Story 11.8 author) |

The `story_id` string convention is `"story-{N.M}-{kebab-case-surface}"` so it sorts and matches the baseline file basename. This story DOES NOT pre-write any of these JSON files — they are written by their owner stories' first CI run (missing-baseline mode).

**AC7 — Canary self-test exercises the macro end-to-end + impl self-tests cover outcome states.**

- File: `crates/orgsidian-core/tests/perf_canary.rs` (NEW; ~80-120 lines).
- This is the only `[[test]]`-target consumer of the macro added by Story 1.12 itself. Wires into the existing `cargo test --workspace --locked` step automatically because it's a standard `tests/` file inside the `orgsidian-core` package (NOT the workspace root; see Dev Notes §9 for why this lives inside the crate rather than alongside `tests/failure_modes.rs`).
- Required `#[test]` functions (exact names — the AC7 dev-box matrix below `grep`s for them):
  1. `perf_impl_writes_baseline_on_first_run` — tempdir-scoped baseline_path; pass `samples_ns = &[100, 100, 100, 100, 100]`; assert `outcome == PerfOutcome::BaselineWritten`; assert file exists post-call; parse JSON; assert `baselines[current_runner_class].median_ns == 100`.
  2. `perf_impl_passes_within_tolerance` — tempdir baseline pre-populated with `median_ns: 100` for the current runner_class; pass `samples_ns = &[105, 110, 115, 120, 118]` (median 115, which is +15% — within 20% tolerance); assert `outcome == PerfOutcome::WithinTolerance`.
  3. `perf_impl_flags_regression_beyond_tolerance` — tempdir baseline pre-populated with `median_ns: 100`; pass `samples_ns = &[130, 135, 140, 145, 150]` (median 140, which is +40%); assert `outcome == PerfOutcome::Regressed`; assert returned `PerfReport.measured_ns == 140` + `PerfReport.baseline_ns == 100`. The impl does NOT panic on regression (per AC2.7); only the macro does. Self-tests target the impl.
  4. `perf_impl_treats_missing_runner_class_as_first_run` — tempdir baseline pre-populated with `median_ns: 999_999_999` for a synthetic runner_class string `"never-matches-anything-xyz"`; pass any 5 samples; assert `outcome == PerfOutcome::BaselineWritten` (NOT Regressed against the unrelated entry); post-call, parse JSON and assert the file now has BOTH the original `"never-matches-anything-xyz"` entry AND a new entry for the current runner_class.
  5. `perf_impl_median_of_5_is_robust_to_outliers` — pre-populated baseline `median_ns: 100`; pass `samples_ns = &[100, 100, 100, 1_000_000_000, 1_000_000_000]` (one-second outliers, but median is still 100); assert `outcome == PerfOutcome::WithinTolerance`. Anti-regression guard: ensures a future contributor cannot silently swap median for mean.
  6. `perf_macro_smoke_writes_initial_baseline` — uses the MACRO (not the impl directly) with a tempdir path AND a trivial `|| {}` empty closure. Assert the test passes on first invocation (missing-baseline-mode is non-fatal per AC2.5); assert the baseline file was written. This is the ONLY macro-level self-test — proves the timing harness compiles, runs, and routes to the impl without crashing.
- All tempdir wiring uses `tempfile::tempdir()` (Story 1.11 already added the workspace-root `tempfile = "3"` discipline in `orgsidian-watcher/Cargo.toml`; Story 1.12 replicates this in `orgsidian-core/Cargo.toml` per AC4).
- **Anti-flakiness discipline**: NONE of the impl self-tests call `std::thread::sleep` or measure wall-clock time. They feed synthetic `samples_ns` slices to the impl. Only `perf_macro_smoke_writes_initial_baseline` measures real wall-clock — and it does so on an empty closure (microseconds at worst). See Dev Notes §3.

**AC8 — Anti-creep scope-fence (out-of-scope items for Story 1.12).**

The following are NOT modified by Story 1.12. Any drift is a review-block:

- **No downstream story perf-AC tests authored.** This story scaffolds the macro + baselines directory + targets doc. The downstream stories (4.3a-g, 6.3, 7.1, 8.1, 8.4, 8.11, 9.1, 10.6, 11.8) author their own `assert_no_perf_regression!` consumers in their own PRs.
- **No pre-written baseline JSON files** under `tests/perf-baselines/` other than nothing (just `.gitkeep` + `README.md`). Owner stories write their own baselines on first CI run (missing-baseline mode is the contract).
- **`.github/workflows/*`: zero touches.** The new `tests/perf_canary.rs` is auto-discovered by the existing `cargo test --workspace --locked` step in [pr.yml#L119-L120](.github/workflows/pr.yml#L119-L120). The Story 1.12 slot reservation at [pr.yml#L170](.github/workflows/pr.yml#L170) (`# Story 1.12: perf snapshot regression macro lands here`) is a stale forward-reference and SHOULD be deleted in this story (per [[feedback_batch_fixes_terse]] no-brainer cleanup) — replaced with a one-line comment `# Story 1.12 perf snapshot regression macro: now wired via `assert_no_perf_regression!` in `crates/orgsidian-core/src/test_support/perf.rs` and consumed by perf-AC stories.`. NO new workflow steps added. The pr.yml comment edit is the SINGLE permitted `.github/workflows/*` touch.
- **No proc-macros, no build.rs, no `criterion` dep.** Declarative `macro_rules!` only. `criterion` is statistical benchmarking; we do NOT need its full machinery for ±20% median-of-5 (and `criterion`'s 100+ transitive deps + Apache-2.0 license churn is supply-chain noise we avoid).
- **No `orgsidian-bench` crate.** The macro lives inside `orgsidian-core` test_support per the explicit epic AC ("`crates/orgsidian-core/src/test_support/perf.rs` exposes …"). Do NOT introduce a new crate.
- **`tests/traceability.rs`: do NOT create.** Owned by Epic 2+ (per [architecture.md#L1081](_bmad-output/planning-artifacts/architecture.md#L1081)). Same anti-creep as Story 1.11 AC6.
- **`crates/test-plugin-panic/`: do NOT create.** LD-38 chaos plugin lands in a later story (same as Story 1.11).
- **`docs/perf/large-vault-scaling.md` (LD-42): do NOT create.** That is owned by a future story per [architecture.md#L1216](_bmad-output/planning-artifacts/architecture.md#L1216). Only `docs/perf/targets.md` lands in Story 1.12.
- **`docs/perf/memory-soak-reports/` (LD-43): do NOT create.** Owned by Story 4.9.
- **README.md / ARCHITECTURE.md / CONTRIBUTING.md: do NOT add backlinks** to `docs/perf/targets.md` in this story. Discoverability follow-up is deferred (same discipline as Story 1.11 AC6).
- **No `cargo bench` invocations** in CI or local discipline. `cargo bench` is a future-tooling concern.
- **No `dhat` heap-profiler wiring** (LD-43 triage tooling). That belongs to Story 4.9.
- **`fixtures/` at workspace root: do NOT create.** Story 1.12 has no fixture-corpus need.
- **Strict-mode env-var hooks**: NONE. Unlike Story 1.11's `ORGSIDIAN_FAILURE_MODE_STRICT=1`, the perf macro has no advisory/strict bifurcation — it is always-on the moment a consumer story commits a baseline. The missing-baseline mode is the only soft-pass path, and it self-resolves on the second CI run.

**AC9 — Dev-box verification matrix.**

All commands run on macOS-arm64 (dev) from the workspace root unless noted. Every cell MUST succeed before the story moves to `review`:

| Command | Expected |
|---|---|
| `ls crates/orgsidian-core/src/test_support/perf.rs` | present |
| `ls crates/orgsidian-core/tests/perf_canary.rs` | present |
| `ls tests/perf-baselines/.gitkeep tests/perf-baselines/README.md` | both present |
| `ls docs/perf/targets.md` | present |
| `grep -c '^pub fn assert_no_perf_regression_impl' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output `1` |
| `grep -c 'macro_rules! assert_no_perf_regression' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output `1` |
| `grep -c '#\[macro_export\]' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output `1` |
| `grep -c 'pub enum PerfOutcome' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output `1` |
| `grep -c 'TOLERANCE_PCT.*= 20' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output ≥`1` (literal 20% constant present — case-corrected post Story 1.12 review P8) |
| `grep -c 'runner_class' crates/orgsidian-core/src/test_support/perf.rs` | exit 0; output ≥`3` (struct field + derivation + comparison) |
| `grep -c 'pub mod perf' crates/orgsidian-core/src/test_support/mod.rs` | exit 0; output `1` |
| `grep -c 'fn perf_impl_writes_baseline_on_first_run\|fn perf_impl_passes_within_tolerance\|fn perf_impl_flags_regression_beyond_tolerance\|fn perf_impl_treats_missing_runner_class_as_first_run\|fn perf_impl_median_of_5_is_robust_to_outliers\|fn perf_macro_smoke_writes_initial_baseline' crates/orgsidian-core/tests/perf_canary.rs` | exit 0; output `6` |
| `grep -c 'PerfOutcome::BaselineWritten\|PerfOutcome::WithinTolerance\|PerfOutcome::Regressed' crates/orgsidian-core/tests/perf_canary.rs` | exit 0; output ≥`3` (one per outcome variant asserted) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo build --workspace --locked` | exit 0 |
| `cargo test --workspace --locked` | exit 0; all 6 canary tests pass |
| `cargo test -p orgsidian-core --test perf_canary --locked --features test-support` | exit 0; 6/6 pass |
| `cargo test -p orgsidian-core --test perf_canary --locked --features test-support -- --nocapture 2>&1 \| grep -c 'perf: writing initial baseline'` | exit 0; output ≥`2` (visible from `perf_impl_writes_baseline_on_first_run` + `perf_impl_treats_missing_runner_class_as_first_run` + `perf_macro_smoke_writes_initial_baseline` — note ≥2 not =3 because Cargo may capture across thread boundaries; ≥2 is the safe floor) |
| `cargo deny --locked check all` | exit 0 (no new advisory/license/source/ban surprises) |
| `cargo audit --deny warnings` (matching pr.yml step) | exit 0 |
| `ls tests/perf-baselines/` | shows ONLY `.gitkeep` + `README.md` (no story-N.M JSON files pre-written) |
| `grep -c '# Story 1.12: perf snapshot regression macro lands here' .github/workflows/pr.yml` | exit 0; output `0` (stale forward-reference removed per AC8) |
| `grep -c 'Story 1.12.*now wired' .github/workflows/pr.yml` | exit 0; output `1` (replacement comment present) |
| `git diff --stat _bmad-output/planning-artifacts/` | empty output (no planning-artifact edits per AC8) |
| `git diff --stat docs/` | shows ONLY `docs/perf/targets.md` (no edits to docs/security/, docs/failure-modes/, etc.) |

If any cell fails on the dev box, the story MUST NOT move to `review`.

**AC10 — Memory-anchored conventions (cross-cutting).**

- **[[feedback_no_co_author_credit]]**: No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on any commit / PR / issue.
- **[[user_contact_email]]**: Authorship attribution uses `tiz.basile@gmail.com` (already pinned in `[workspace.package].authors` at [Cargo.toml#L26](Cargo.toml#L26)). New files do NOT add a personal contact header.
- **[[feedback_version_policy]]**: `tempfile = "3"` is the latest stable major; caret on `"3"` matches the Story 1.11 `fail = "0.5"` precedent. `serde_json` floats on the existing workspace dep. Tauri-ecosystem exact-pin discipline does NOT apply (neither is a Tauri crate).
- **[[feedback_batch_fixes_terse]]**: post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface. Carried decision-grade question (see Dev Notes §10): the AC8 `.github/workflows/pr.yml` stale-comment cleanup — surface for explicit confirmation that this single-line edit is in-scope (epic AC text does NOT mention workflow edits at all).
- **[[project_orgsidian_github_label_scheme]]**: Issue #12 label transitions follow `status:backlog` → `status:in-progress` (at dev-story start) → `status:in-review` (at PR open). Use `status:in-review`, NOT `status:review`.
- **[[project_orgsidian_github_plan]]**: GitHub Free plan = no branch protection enforcement; required-checks list in `pr.yml` is advisory only. The new `perf_canary` `[[test]]` target attaches automatically to that advisory list.
- **[[project_orgsidian_repo_public_during_pre_alpha]]**: repo is public; `docs/perf/targets.md` is visible immediately on merge. Tone accordingly — no internal-only refs.

**Traces:** LD-32 (perf snapshot regression gate — [architecture.md L523](_bmad-output/planning-artifacts/architecture.md#L523)), NFR-1..NFR-7 (PRD §8 absolute targets documented in `docs/perf/targets.md`), NFR-20 (perf snapshot ±10% gate macro discipline — [architecture.md L132](_bmad-output/planning-artifacts/architecture.md#L132)), [test-design.md §6.9 Layer 9](_bmad-output/test-artifacts/test-design.md#L606-L626) (Perf Snapshot mechanism), [test-design.md TC-2](_bmad-output/test-artifacts/test-design.md#L144-L150) (runner_class hardware-scoping addendum), [architecture.md L331](_bmad-output/planning-artifacts/architecture.md#L331) (±20% macro spec verbatim), Process Discipline rule A (red-phase ATDD merge-gate — every downstream perf-AC story's perf test is its red-phase scaffold), Process Discipline rule H (test-design.md as authoritative system-level test strategy).

## Tasks / Subtasks

- [x] **Task 1 — Author `crates/orgsidian-core/src/test_support/perf.rs`** (AC1, AC2, AC3)
  - [x] 1.1 Define `pub enum PerfOutcome { BaselineWritten, WithinTolerance, Regressed }` with `Debug + Clone + PartialEq + Eq` derives (needed for `assert_eq!` in self-tests).
  - [x] 1.2 Define `pub struct PerfReport { pub outcome: PerfOutcome, pub story_id: String, pub runner_class: String, pub measured_ns: u128, pub baseline_ns: Option<u128>, pub samples: usize }` with `Debug + Clone` derives.
  - [x] 1.3 Implement `pub(crate) fn current_runner_class() -> String` returning `format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)` (e.g., `macos-aarch64`, `linux-x86_64`). NO `RUNNER_OS` env var.
  - [x] 1.4 Implement `pub(crate) fn workspace_root() -> PathBuf` per the verbatim snippet in AC3 (walks up from `env!("CARGO_MANIFEST_DIR")` until a `Cargo.toml` containing `[workspace]` is found).
  - [x] 1.5 Implement `pub(crate) fn current_iso8601_utc() -> String` returning a fixed-format `YYYY-MM-DDTHH:MM:SSZ` string for `created_at`. Use `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` → manual format (NOT chrono — keep deps minimal). Format the timestamp manually as `format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", ...)`. See Dev Notes §11.
  - [x] 1.6 Implement `pub fn assert_no_perf_regression_impl(story_id: &str, baseline_path: &Path, samples_ns: &[u128]) -> PerfReport`:
    - Resolve `full_path = workspace_root().join(baseline_path)`.
    - Compute `measured_median = sorted(samples_ns)[2]` (panic if `samples_ns.len() != 5`).
    - Derive `runner_class = current_runner_class()`.
    - Branch:
      - If `full_path` does not exist → create parent dir + write new JSON + `eprintln!` warning + return `BaselineWritten`.
      - If `full_path` exists → parse JSON. If `baselines[runner_class]` is missing → insert + rewrite file + `eprintln!` warning + return `BaselineWritten`. If present → compute `tolerance_threshold = baseline_median * 120 / 100` (u128 arithmetic). If `measured_median <= tolerance_threshold` → return `WithinTolerance`. Else → return `Regressed`.
  - [x] 1.7 Implement `#[macro_export] macro_rules! assert_no_perf_regression`:
    ```rust
    #[macro_export]
    macro_rules! assert_no_perf_regression {
        ($story_id:expr, $baseline_path:expr, $op:expr) => {{
            let mut samples_ns: Vec<u128> = Vec::with_capacity(5);
            for _ in 0..5 {
                let start = ::std::time::Instant::now();
                $op();
                samples_ns.push(start.elapsed().as_nanos());
            }
            let report = $crate::test_support::perf::assert_no_perf_regression_impl(
                $story_id,
                ::std::path::Path::new($baseline_path),
                &samples_ns,
            );
            if matches!(report.outcome, $crate::test_support::perf::PerfOutcome::Regressed) {
                let measured = report.measured_ns;
                let baseline = report.baseline_ns.unwrap_or(0);
                let pct = if baseline > 0 { (measured * 100 / baseline).saturating_sub(100) } else { 0 };
                panic!(
                    "perf regression: {} on {}: measured median {} ns exceeds baseline {} ns by {}% (tolerance: 20%, samples: {})\nBaseline file: {}",
                    report.story_id, report.runner_class, measured, baseline, pct, report.samples, $baseline_path
                );
            }
        }};
    }
    ```
  - [x] 1.8 Add a `pub use` re-export of the macro at the `test_support::perf` module path so [test-design.md §7.3.10](_bmad-output/test-artifacts/test-design.md#L909-L926)'s literal `use orgsidian_core::test_support::perf::assert_no_perf_regression;` line compiles. Per Dev Notes §4, `#[macro_export]` hoists to crate root by default; the `pub use` makes the submodule path equivalent. Pattern:
    ```rust
    // At the top of test_support/perf.rs, after the macro_rules! definition:
    #[doc(inline)]
    pub use crate::assert_no_perf_regression;
    ```
  - [x] 1.9 Module-level `//!` doc-comment references: Story 1.12, LD-32, NFR-20, test-design.md §6.9 + TC-2, and a one-line caveat that this is the canonical perf-gate macro for the whole codebase.

- [x] **Task 2 — Wire the new module + update mod.rs stale comment** (AC1)
  - [x] 2.1 Edit `crates/orgsidian-core/src/test_support/mod.rs`: add `pub mod perf;` after the existing `pub mod clock;`.
  - [x] 2.2 Update the stale forward-reference in the same file's module-level doc-comment (line 4) — change `"Story 1.12 will add `pub mod perf;` alongside it"` to past tense: `"Story 1.12 adds the `perf` submodule alongside it"`.
  - [x] 2.3 Update the stale comment in `crates/orgsidian-core/src/lib.rs:13` (`// "Story 1.12 ... downstream perf-snapshot tests (Story 1.12)"` — phrasing already references this story; leave as-is, the past-tense is already correct context).
  - [x] 2.4 Update `crates/orgsidian-core/src/test_support/clock.rs:4` stale forward-reference (`"(Story 1.12) the perf-snapshot macros"`) to `"the perf-snapshot macros (Story 1.12)"` — no semantic change, just consistency now that the module is real.

- [x] **Task 3 — `Cargo.toml` dependency wiring** (AC4)
  - [x] 3.1 Edit `crates/orgsidian-core/Cargo.toml [features]` block: change `test-support = []` to `test-support = ["dep:serde_json"]` with the 3-line comment header per AC4.
  - [x] 3.2 Append to `[dependencies]`: `serde_json = { workspace = true, optional = true }` with the AC4 verbatim comment.
  - [x] 3.3 Append to `[dev-dependencies]`: `tempfile = "3"` with the AC4 verbatim comment.
  - [x] 3.4 `cargo build --workspace --locked` → success. `Cargo.lock` updates expected (tempfile + transitive); commit the lockfile.
  - [x] 3.5 `cargo deny check --locked` + `cargo audit --deny warnings` → both clean. If either surfaces a new advisory/license, halt + surface as decision-grade question per AC4.

- [x] **Task 4 — Create `tests/perf-baselines/` directory + marker files** (AC3)
  - [x] 4.1 `mkdir -p tests/perf-baselines/`.
  - [x] 4.2 `touch tests/perf-baselines/.gitkeep` (empty file).
  - [x] 4.3 Author `tests/perf-baselines/README.md` per the AC3 required-content spec (purpose, JSON shape, pointers, commit-discipline note). Keep <30 lines.

- [x] **Task 5 — Author `docs/perf/targets.md`** (AC5)
  - [x] 5.1 `mkdir -p docs/perf/`.
  - [x] 5.2 Author `docs/perf/targets.md` with the 5 required sections per AC5 (purpose, target table, gate-vs-target rationale, baseline workflow, footer). Cross-check every row of the target table against the cited [test-design.md / architecture.md / epics.md] anchors before committing.
  - [x] 5.3 `grep -c '| NFR-' docs/perf/targets.md` → ≥`8` (table has all NFR-1..NFR-7 + FR-14/FR-26 rows).

- [x] **Task 6 — Author `crates/orgsidian-core/tests/perf_canary.rs`** (AC7)
  - [x] 6.1 Create the file with module-level `//!` doc-comment referencing Story 1.12 + AC7's canary scope (impl self-tests, NOT consumer-story tests).
  - [x] 6.2 Implement the 6 `#[test]` functions per AC7 exact names + behaviors. Each test uses `tempfile::tempdir()` for an isolated baseline_path. Self-tests target the IMPL (no panics expected); only `perf_macro_smoke_writes_initial_baseline` exercises the macro.
  - [x] 6.3 `cargo test -p orgsidian-core --test perf_canary --locked --features test-support` → 6/6 pass.
  - [x] 6.4 `cargo test --workspace --locked` → 6/6 pass under workspace feature unification (orgsidian-watcher's `features = ["test-support"]` cascades).

- [x] **Task 7 — Cleanup stale `.github/workflows/pr.yml` slot reservation** (AC8)
  - [x] 7.1 Replace the line `# Story 1.12: perf snapshot regression macro lands here` at [pr.yml:170](.github/workflows/pr.yml#L170) with `# Story 1.12 perf snapshot regression macro: now wired via assert_no_perf_regression! in crates/orgsidian-core/src/test_support/perf.rs and consumed by perf-AC stories.`. Single-line cleanup; NO new workflow steps.
  - [x] 7.2 `grep -c '# Story 1.12: perf snapshot regression macro lands here' .github/workflows/pr.yml` → `0`. `grep -c 'Story 1.12 perf snapshot regression macro: now wired' .github/workflows/pr.yml` → `1`.

- [x] **Task 8 — Dev-box verification matrix** (AC9)
  - [x] 8.1 Run every cell in AC9. Record exit codes + outputs in the Dev Agent Record / Debug Log References section.
  - [x] 8.2 Specifically verify the missing-baseline-mode `eprintln!` warning is visible under `--nocapture` (Task 6 + AC9 grep cell).
  - [x] 8.3 Specifically verify the regression panic message format from Task 1.7 by inspection of `perf_macro_smoke_writes_initial_baseline` output (no regression in smoke; the format check is by reading the macro source).

- [x] **Task 9 — Scope-fence audit** (AC8)
  - [x] 9.1 `git status` confirms the in-scope file set:
    - **NEW (5)**: `crates/orgsidian-core/src/test_support/perf.rs`, `crates/orgsidian-core/tests/perf_canary.rs`, `tests/perf-baselines/.gitkeep`, `tests/perf-baselines/README.md`, `docs/perf/targets.md`.
    - **MODIFIED (4 + lockfile)**: `crates/orgsidian-core/Cargo.toml` (feature flag + 2 deps), `crates/orgsidian-core/src/test_support/mod.rs` (1 new line + 1 comment update), `crates/orgsidian-core/src/test_support/clock.rs` (1 stale-comment polish per Task 2.4), `.github/workflows/pr.yml` (1-line comment replacement per AC8/Task 7), `Cargo.lock` (transitive tempfile deps).
    - **Workflow artefacts**: sprint-status.yaml + this story file (the create-story workflow handles these).
  - [x] 9.2 Verified: no `tests/traceability.rs`, no `crates/test-plugin-panic/`, no `docs/perf/large-vault-scaling.md`, no `docs/perf/memory-soak-reports/`, no README.md/ARCHITECTURE.md/CONTRIBUTING.md edits, no `fixtures/` creation, no pre-written `tests/perf-baselines/story-*.json` files, no new workflow steps.
  - [x] 9.3 `Cargo.lock` diff inspection: only new transitive deps of `tempfile` + `serde_json` activation (serde_json itself is pre-locked via other deps). No surprise version bumps to existing deps. If any existing dep is bumped, surface as decision-grade question.

- [x] **Task 10 — GitHub Issue sync (pre-flight)** (AC10)
  - [x] 10.1 Issue #12 label transition: `status:backlog` → `status:in-progress` at dev-story start; → `status:in-review` post-implementation (NOT `status:review`).
  - [x] 10.2 Verify no other label changes needed (`epic:1`, `milestone:v0.1`, `type:story` already correct per `gh issue view 12 --json labels`).

## Dev Notes

### §1 — Why this story lands NOW (Epic 1, after 1.11, before 1.13/14/15)

Per Murat (Party Mode round 2 P0), Story 1.12 is the second of the two architectural test-infra primitives that MUST land in Epic 1 — Story 1.11 covers fault paths (LD-41), Story 1.12 covers perf paths (LD-32 / NFR-20). Together they define the test-discipline contract that every downstream story inherits. Without 1.12, perf-AC stories starting from 4.3a (CodeMirror decorations, Epic 4) would each invent their own `<500ms` or `<2s` absolute-threshold tests, drifting into the GHA-hardware-noise antipattern documented in [test-design.md TC-2](_bmad-output/test-artifacts/test-design.md#L144-L150).

The macro is **forward-compatible with TC-2 from day 1** — the JSON shape includes `runner_class` keying, and the impl scopes comparisons per-class. The first CI runner of each class to see a perf-AC test writes its own baseline entry. This matches Murat's "place the canary before the coal" discipline from Story 1.11 §1.

Story 1.12 lands in Epic 1 (before downstream perf consumers in Epics 4/6/7/8/9/10/11) because once a perf-AC story commits a baseline against the wrong macro shape (e.g., absolute threshold instead of relative), the downstream PR-revert cost is high. Lock the shape now, fill in baselines later.

### §2 — Why the macro lives in `orgsidian-core::test_support::perf` (and not a new crate / not `orgsidian-bench`)

The epic AC literal wording ("`crates/orgsidian-core/src/test_support/perf.rs` exposes …") locks this. The reasoning is identical to Story 1.9's `clock` placement: `orgsidian-core` is the cross-crate integrator ("Core domain orchestrator wiring parser/index/watcher/vault/plugin-api/report" per its [package description](crates/orgsidian-core/Cargo.toml#L2)). Every downstream story has `orgsidian-core` in its dev-deps via the `test-support` feature; the macro is automatically in scope.

A `crates/orgsidian-bench/` crate was considered and rejected: it would force every perf-AC consumer to add a new dev-dep, AND `criterion`-style benchmark crates are statistical-distribution heavy (welch t-tests, throughput regressions) — overkill for a median-of-5 ±20% gate. The `assert_no_perf_regression!` shape is intentionally minimalist.

### §3 — Why impl/macro split is non-negotiable (anti-flakiness)

A naive design wires the 5-sample timing harness inside the macro AND the policy (median + baseline compare + warn) also inside the macro. This makes self-tests impossible without `std::thread::sleep`-based ops, which are inherently flaky on CI under load.

The split moves the policy into a pure `fn` (no I/O timing, accepts pre-computed `samples_ns`). The 5 self-tests in `perf_canary.rs` feed synthetic samples to assert outcome behavior. Only ONE self-test exercises the macro itself (the smoke test) and uses an empty closure → microsecond timing, no flake risk.

This is the same separation-of-concerns discipline as Story 1.11's `failure_modes_coverage.rs` (pure source-text scanner, no I/O) vs `failure_modes.rs` (will eventually exercise real I/O via `fail` crate). See [Story 1.11 Dev Notes §4](_bmad-output/implementation-artifacts/1-11-establish-ld-41-failure-mode-test-harness-party-mode-round-2-p0-murat.md).

### §4 — Macro export discipline: `#[macro_export]` + `pub use` shim

`#[macro_export]` Rust macros are hoisted to the **crate root** namespace, NOT to the module where they are defined. So `orgsidian_core::assert_no_perf_regression!` works but `orgsidian_core::test_support::perf::assert_no_perf_regression!` does NOT by default.

The [test-design.md §7.3.10 scaffold](_bmad-output/test-artifacts/test-design.md#L914) uses the submodule path: `use orgsidian_core::test_support::perf::assert_no_perf_regression;`. To make that import compile, add a `pub use crate::assert_no_perf_regression;` at the top of `test_support/perf.rs`. This is a standard Rust idiom (used by `serde_json::json!` macro re-exports; verified pattern). The `#[doc(inline)]` attribute makes the rustdoc render the macro at the submodule path too.

Side effect: the macro is callable via BOTH paths. The crate-root path is what production consumers use (`use orgsidian_core::assert_no_perf_regression;`); the submodule path is what the scaffold literal expects. Both must work — see AC1 last bullet.

### §5 — Why `env::consts` (compile-time), NOT `RUNNER_OS` (env-var)

`RUNNER_OS` and `RUNNER_ARCH` are set by GitHub Actions only. On a local dev box, they are undefined → `unwrap_or_default()` returns `""`, the runner_class becomes `"-"`, and baselines written locally would never match CI baselines (different `runner_class` keys). Worse: the `BaselineWritten` outcome is the soft-pass — so a developer running `cargo test` locally would silently write a wrong-key baseline entry that pollutes the committed JSON if not noticed.

`std::env::consts::OS` and `std::env::consts::ARCH` are compile-time constants. `macos-aarch64` is the same on a developer's M-series Mac AND on a GHA `macos-14-arm` runner. `linux-x86_64` is the same on a CI `ubuntu-24.04` runner AND a local Linux contributor's box. This is the right primitive for cross-environment baseline-key stability.

The TC-2 ([test-design.md L144-150](_bmad-output/test-artifacts/test-design.md#L144-L150)) addendum recommends `runner_class`-scoping; it does NOT mandate the env-var derivation strategy. Story 1.12 makes the compile-time-constant choice and documents it in Dev Notes for future TC-2 re-evaluation.

Granularity trade-off: `env::consts` gives OS+ARCH but NOT specific CPU model (M1 vs M2 vs M3 are all `macos-aarch64`). For ±20% tolerance this is acceptable. If a future hardware refresh on GHA causes systematic drift, a finer-grained `runner_class` (e.g., reading `/proc/cpuinfo` model name on Linux) becomes a deferred-hardening follow-up — not a Story 1.12 concern.

### §6 — Baseline JSON I/O: rewrite-whole-file, no partial-update gymnastics

The baseline file is small (one struct, max 4-5 `runner_class` entries). The impl reads the whole file, deserializes to a `serde_json::Value` or a dedicated `BaselineFile` struct, mutates the in-memory map, and writes the whole file back. NO append-only / line-by-line tricks.

Concurrency: parallel `cargo test` runs of different stories use DIFFERENT baseline files (one per story_id). Parallel runs of the SAME story (e.g., the same canary test running across multiple `--features` permutations in workspace-unified mode) would race on the same tempdir-scoped file — but each canary test uses its own `tempdir()`, so there is no shared mutable state in the self-tests. For PRODUCTION baselines committed under `tests/perf-baselines/`, only ONE test owns each file (the perf-AC consumer story), so no race.

Atomicity: the impl uses `std::fs::write(full_path, json_string)` which is NOT atomic on Windows (truncate + write happens in two syscalls). This is acceptable because: (a) baseline files are rewritten only during missing-baseline mode, which happens once per `(story_id, runner_class)` over the project lifetime; (b) the LD-8 atomic-write discipline applies to user data (`.org` files), not to test infrastructure. If a baseline write is interrupted, the next test run will write a fresh one (missing-baseline mode is idempotent on a missing file).

### §7 — Why `serde_json` as `optional + dep:` not unconditional

`orgsidian-core` is the cross-crate integration root. Adding `serde_json` to its production `[dependencies]` unconditionally would pull serde_json into every release build of `orgsidian-shell-app` AND `orgsidian-cli` — even though they don't use it (they consume IPC via `tauri-specta` which uses serde + its own JSON path).

The `optional = true` + `test-support = ["dep:serde_json"]` wiring keeps `serde_json` OUT of release builds. The dev-time / test-time path picks it up via the feature flag. Workspace feature unification (`orgsidian-watcher` already enables `orgsidian-core/test-support`) cascades the activation to all crates under `cargo test --workspace`, so the canary test's `serde_json` dep is satisfied without any per-crate Cargo.toml edits in the watcher.

The Story 1.11 `fail` crate uses a different shape (pure `[dev-dependencies]` because it's used ONLY in `tests/failure_modes.rs`, not in any production source). Story 1.12 cannot use that shape because `test_support::perf` is part of the public-via-feature module surface — `[dev-dependencies]` would not be visible to external consumers using `features = ["test-support"]`.

### §8 — Path resolution: workspace root, NOT `CARGO_MANIFEST_DIR`

The most likely dev-agent foot-gun. Default Rust file-I/O is relative to the process CWD, which `cargo test` sets to the package directory (the `crates/<name>/`). If the impl naively passes `baseline_path` to `fs::read_to_string`, a consumer in `crates/orgsidian-index/tests/perf_search.rs` writing baseline `"tests/perf-baselines/story-8.4.json"` would create `crates/orgsidian-index/tests/perf-baselines/story-8.4.json` — wrong location, not under the workspace-root `tests/perf-baselines/` per test-design.md §5.1.

The `workspace_root()` helper walks up from `CARGO_MANIFEST_DIR` (a Cargo-set env var; `env!()` works at compile time even for libraries) until it finds a `Cargo.toml` containing `[workspace]`. The impl then resolves `full_path = workspace_root().join(baseline_path)` so every consumer writes to the same location regardless of which crate hosts the test.

`env!("CARGO_MANIFEST_DIR")` resolves at the point the macro/function is COMPILED, NOT at the call site. For `orgsidian-core` that means the path points to `crates/orgsidian-core/` — walking up one level finds the workspace root. The helper handles deeper nesting safely.

### §9 — Why `crates/orgsidian-core/tests/perf_canary.rs`, NOT `tests/perf_canary.rs` (workspace root)

Story 1.11 placed its harness at `tests/` workspace-root with `[[test]]` wiring in `orgsidian-core/Cargo.toml` (Story 1.11 Dev Notes §2 trade-off). The motivation there was DISCOVERABILITY of the cross-cutting failure-mode catalog at top-level.

Story 1.12's canary self-test does NOT have the same discoverability motivation. It is an internal sanity check on the macro — NOT something a contributor browsing the repo needs to see at top-level. The cross-cutting top-level artifacts for Story 1.12 are `tests/perf-baselines/` (where every story's baseline JSON lands) and `docs/perf/targets.md` (the human-readable target table). The canary lives inside the crate that owns the macro.

This avoids the workspace-virtual-manifest `[[test]]`-declaration gymnastics from Story 1.11 (no `[[test]]` block needed; standard Cargo `tests/<name>.rs` discovery works inside the package). One less moving part.

### §10 — The `.github/workflows/pr.yml` stale-comment cleanup is a decision-grade question

Epic AC for Story 1.12 ([epics.md L601-L616](_bmad-output/planning-artifacts/epics.md#L601-L616)) does NOT mention workflow edits. The current pr.yml has a slot-reservation comment ([pr.yml:170](.github/workflows/pr.yml#L170)) explicitly tagged "Story 1.12: perf snapshot regression macro lands here". Two reasonable interpretations:

1. **Treat as in-scope cleanup**: the comment is a forward-reference scaffold that becomes stale the moment this story lands. Per [[feedback_batch_fixes_terse]], no-brainer cleanup is silent. Replace the comment with a one-line note pointing at the now-real macro location.
2. **Treat as out-of-scope**: AC8 anti-creep on `.github/workflows/*` matches the Story 1.11 discipline. Defer the cleanup to a future docs-tidy story.

This story picks **interpretation 1** (Task 7 / AC8 carve-out) as the default — the slot is explicitly tagged with this story's ID, so it is THIS story's responsibility to retire the marker. The single-line edit is the SMALLEST plausible workflow touch and adds no new CI steps.

Surface for explicit reviewer confirmation per [[feedback_batch_fixes_terse]]. If the reviewer prefers interpretation 2, the AC8 carve-out + Task 7 + the AC9 grep cells are dropped; everything else stands. This is the primary decision-grade question this story carries.

### §11 — ISO-8601 timestamp without `chrono`

The `created_at` field in the baseline JSON is a human-readable timestamp for forensic value (when was this baseline first written?). It is NOT machine-parsed by the regression gate (which only reads `median_ns`). So precision-loss is acceptable.

Hand-roll a `format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", ...)` from `SystemTime::now()`. Compute Y/M/D/H/M/S from `duration_since(UNIX_EPOCH).as_secs()` via a small constant-table day-counter. ~30-40 LOC. Avoids pulling `chrono` (`~80 transitive deps including the `time` crate's notoriously CVE-prone path) into a leaf crate.

Alternative if the LOC concern dominates: pull `time = "0.3"` (the modern `chrono` successor, much smaller). But this adds a new workspace.dependency entry. Default choice: hand-roll. Surface for decision if Task 1.5 ends up >40 LOC.

### §12 — Previous-story intelligence (Story 1.11)

Story 1.11 (just-merged, status: done) established:
- The **two-file harness pattern**: the cross-cutting test file at `tests/` workspace-root + a sibling coverage gate. Story 1.12 inverts this — the canary lives inside the crate (per §9), not at workspace root. The `tests/perf-baselines/` directory IS the cross-cutting top-level artifact for Story 1.12 (analogous to Story 1.11's `tests/failure_modes.rs`).
- The **`[[test]]` declaration in orgsidian-core/Cargo.toml** pattern for workspace-root tests. Story 1.12 does NOT use this — the canary is in `crates/orgsidian-core/tests/perf_canary.rs`, auto-discovered (no `[[test]]` declaration needed).
- The **`required-features = ["test-support"]` discipline** ([crates/orgsidian-core/Cargo.toml#L32-L35](crates/orgsidian-core/Cargo.toml#L32-L35)): canary tests inside the crate inherit this through workspace feature unification (orgsidian-watcher dev-dep already activates `test-support`). The Task 8 dev-box matrix uses `--features test-support` to also exercise the standalone `cargo test -p orgsidian-core --test perf_canary` path.
- The **decision-grade-question carry-forward pattern** ([[feedback_batch_fixes_terse]]): Story 1.11 carried the 9-vs-10 LD-41 categories question. Story 1.12 carries the workflow-cleanup-scope question (§10).
- The **fail crate workspace-dep + crate-level features** wiring pattern ([Cargo.toml#L42-L46](Cargo.toml#L42-L46)). Story 1.12 reuses this for `serde_json` (already pre-existing workspace dep) + `tempfile` (bare in dev-deps per the existing orgsidian-watcher convention).
- The **commit type**: `feat(test):` per Story 1.11's `feat(test): add LD-41 failure-mode harness ...` pattern. Story 1.12's commit message: `feat(test): add LD-32/NFR-20 perf snapshot regression infrastructure (Story 1.12, closes #12)`.
- The **single-PR-per-story** discipline.
- The **review fixup volume** profile: Story 1.11 had 2 patches + 5 deferred (test infrastructure with a surface). Story 1.12 is similar shape (test infrastructure + docs + macro design). Expect 2-4 patches + several deferred.

### §13 — Git-history intelligence (last 5 commits, 2026-05-28)

```
8e1ca95 Merge pull request #126 from orgsidian/story/1.11-ld-41-failure-mode-harness
0ab6316 fix(test): rename failure-mode gate constant (Story 1.11 review)
2bb9b53 feat(test): add LD-41 failure-mode harness + gate + matrix (Story 1.11, closes #11)
f67c4c1 Merge pull request #125 from orgsidian/story/1.10-hygiene-docs
29e43b7 docs: add SECURITY.md / ARCHITECTURE.md / CHANGELOG.md / CONTRIBUTING.md (Story 1.10, closes #10)
```

Patterns to absorb:
- **Story branch naming**: `story/1.12-perf-snapshot-regression-infrastructure` (`story/N.M-<kebab-short-title>`; Story 1.11 used `story/1.11-ld-41-failure-mode-harness`).
- **PR body discipline**: literal `Closes #12` per the Story 1.11 fix-up (PR #126 review pointed out that `Closes #N` was missing from the body until fix-up). Author the PR body with `Closes #12` from the start to avoid the same fix-up.
- **Conventional Commit type**: `feat(test):` (per Story 1.11 + 1.9 precedent for new test infrastructure).
- **`fix(test):`** as the review-fixup type (per Story 1.11's `0ab6316 fix(test): rename failure-mode gate constant`).
- **No co-author trailers, no AI-credit footers** (per [[feedback_no_co_author_credit]]).

### §14 — LLM-dev-agent anti-pattern checklist

Common dev-agent mistakes this story spec intentionally guards against:

1. **DO NOT use mean instead of median.** Median is robust to GHA noise spikes; mean is not. See AC2.2 + AC7 self-test #5.
2. **DO NOT use 10% tolerance.** Epic AC + architecture both lock 20%. The 10% LD-32 headline value is the trend-gate target, NOT the per-invocation macro tolerance.
3. **DO NOT use `RUNNER_OS` / `RUNNER_ARCH` env vars.** Use `std::env::consts::OS / ARCH` (compile-time). See §5.
4. **DO NOT resolve `baseline_path` relative to `CARGO_MANIFEST_DIR` or process CWD.** Resolve via the `workspace_root()` walk-up helper. See §8.
5. **DO NOT panic in the impl.** The impl returns `PerfOutcome::Regressed`; only the MACRO panics. See AC2.7 + §3.
6. **DO NOT use float arithmetic for the tolerance comparison.** Integer `u128`: `measured > baseline * 120 / 100`. Floats drift across platforms and break the deterministic-gate contract.
7. **DO NOT pull `criterion`, `chrono`, `time`, or any "perf benchmarking" crate.** Hand-roll the median + ISO-8601 + JSON paths. Justify any new dep as a decision-grade question.
8. **DO NOT add `serde_json` unconditionally to `[dependencies]`.** Use `optional = true` + `dep:serde_json` activator on the `test-support` feature. See §7.
9. **DO NOT pre-write baseline JSON files** for downstream stories. Owner stories write their own baselines on first CI run (missing-baseline mode). See AC6 + AC8.
10. **DO NOT add a `cargo bench` step to CI.** No bench infrastructure in this story. See AC8.
11. **DO NOT add new `.github/workflows/*` steps.** The ONLY workflow touch is the 1-line stale-comment cleanup at [pr.yml:170](.github/workflows/pr.yml#L170) per AC8/Task 7.
12. **DO NOT create `crates/orgsidian-bench/`.** Macro lives in `orgsidian-core::test_support::perf` per the epic AC literal wording.
13. **DO NOT use `Vec<f64>` for samples.** Use `Vec<u128>` (nanoseconds from `Duration::as_nanos()`). Float-noise is not the problem we're solving here.
14. **DO NOT use `HashMap` for the `baselines` map order.** Use `BTreeMap` (or serde_json's preserve-order feature) so committed JSON files have deterministic key order. Re-running missing-baseline-mode against an existing file should produce a DETERMINISTIC diff (otherwise PR reviews become noisy).
15. **DO NOT silently add `deny.toml [advisories].ignore`** if `cargo deny` surfaces a new advisory on `tempfile` or `serde_json` transitive. Surface as decision-grade question per AC4.
16. **DO NOT bump existing dependencies** (e.g., `tracing`, `thiserror`, `tauri-*`) as a side effect. If `cargo update` produces unrelated bumps, halt + surface.
17. **DO NOT use lowercase / mixed-case directory names.** `docs/perf/` (lowercase + no hyphen needed for single word) follows `docs/security/` and `docs/failure-modes/` convention.
18. **DO NOT add `Co-Authored-By:` trailers or "Generated with Claude Code" footers** to commit / PR / Issue. Per [[feedback_no_co_author_credit]].
19. **DO NOT exceed 5 samples**. The macro hard-codes 5. If a future story needs more, that's a Story-1.12-revision conversation, not an in-test loop.
20. **DO NOT use any kind of warmup**. All 5 samples count. The 20% tolerance absorbs cold-cache effects on the first sample.

### §15 — Cross-platform sanity check

- **Line endings**: repo uses LF. New `.rs` + `.md` + `.json` files MUST be LF (verify via `file <path>`).
- **`workspace_root()` walk-up**: works on all 3 platforms (macOS-arm64, Linux x86_64, Windows). The `Cargo.toml` filename is case-sensitive on Linux ext4 / Windows NTFS-case-sensitive; Windows default NTFS is case-insensitive. The Rust ecosystem convention is `Cargo.toml` (capital C); orgsidian uses this canonical form.
- **`std::env::consts::OS` mapping**: `macos`, `linux`, `windows`, `freebsd`, etc. `ARCH`: `x86_64`, `aarch64`, etc. Stable across Rust versions. Verified against [Rust stdlib docs](https://doc.rust-lang.org/std/env/consts/index.html).
- **JSON file line endings**: `serde_json::to_string_pretty` emits LF by default on all platforms. Confirmed via serde_json 1.0.x docs. The committed baseline files should be LF — verify in the canary test by reading the written file as bytes and asserting no `\r\n`.
- **`tempfile::tempdir()`**: cross-platform; uses OS temp directory conventions (`/tmp` on Unix, `%TEMP%` on Windows). Auto-cleanup on drop. No platform-specific tweaks needed.
- **`Path::join` with relative paths**: cross-platform. `workspace_root().join("tests/perf-baselines/foo.json")` produces forward-slash paths on Unix, backslash on Windows — but Cargo + serde_json + std::fs all accept both. The committed JSON file (`baseline_path` string in source code) MUST use forward slashes for portability — the `Path::new(baseline_path)` constructor normalizes.

### §16 — Architecture decision references (LD anchors)

Critical LD references this story implements / surfaces:
- **LD-32** ([architecture.md#L521-L526](_bmad-output/planning-artifacts/architecture.md#L521-L526)) — CI matrix; per-PR perf snapshot regression gate (±10% headline / ±20% macro tolerance per AC2.3).
- **LD-32 macro spec** ([architecture.md#L331](_bmad-output/planning-artifacts/architecture.md#L331)) — verbatim definition of `assert_no_perf_regression!` macro shape; this story implements it.
- **NFR-20** ([architecture.md#L132](_bmad-output/planning-artifacts/architecture.md#L132)) — Performance regression gate non-functional requirement.
- **NFR-1..NFR-7** (PRD §8) — absolute perf budgets documented in `docs/perf/targets.md` (NOT as the regression gate; see AC5).
- **LD-37** ([architecture.md#L1163-L1170](_bmad-output/planning-artifacts/architecture.md#L1163-L1170)) — Supply-chain gates; both MUST stay green with the new `tempfile` + activated `serde_json` deps (AC4).
- **LD-42** ([architecture.md#L1216](_bmad-output/planning-artifacts/architecture.md#L1216)) — Large-vault scaling; future `docs/perf/large-vault-scaling.md` lives in a later story, NOT here (AC8).
- **LD-43** ([architecture.md#L1220-L1226](_bmad-output/planning-artifacts/architecture.md#L1220-L1226)) — Memory soak regression gate; orthogonal to perf snapshot, owned by Story 4.9.
- **LD-54** ([architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)) — Conventional Commits; `feat(test):` for this story per §13.
- **Process Discipline rule A** ([epics.md#L290-L294](_bmad-output/planning-artifacts/epics.md#L290-L294)) — red-phase ATDD merge-gate; the macro IS the red-phase scaffold mechanism for every downstream perf-AC story.
- **Process Discipline rule H** ([epics.md#L347-L349](_bmad-output/planning-artifacts/epics.md#L347-L349)) — test-design.md as authoritative; §6.9 + TC-2 are the sources of truth for this story's design.

### §17 — Memory-anchored conventions (cross-cutting)

- **[[feedback_no_co_author_credit]]**: No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on commit / PR / Issue.
- **[[user_contact_email]]**: `tiz.basile@gmail.com` (Cargo.toml pin is authoritative); new files do NOT add a personal contact header.
- **[[feedback_version_policy]]**: `tempfile = "3"` is the latest-stable major; caret on `"3"` matches the Story 1.11 precedent. `serde_json` floats on the existing workspace dep. Tauri-exact-pin discipline does NOT apply.
- **[[feedback_batch_fixes_terse]]**: post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface. The workflow-cleanup-scope question (§10) is the primary one this story carries; the hand-rolled-vs-`time`-crate question (§11) is secondary.
- **[[project_orgsidian_github_label_scheme]]**: status label is `status:in-review` (NOT `status:review`).
- **[[project_orgsidian_github_plan]]**: GitHub Free plan = no branch protection enforcement; required-checks list in `pr.yml` is advisory only.
- **[[project_orgsidian_repo_public_during_pre_alpha]]**: repo is already public; `docs/perf/targets.md` is visible immediately on merge. Tone the file accordingly (no internal-only references; PRD/architecture cross-links are OK because those are also visible in the public repo).

### Project Structure Notes

- **New files (5)**: `crates/orgsidian-core/src/test_support/perf.rs`, `crates/orgsidian-core/tests/perf_canary.rs`, `tests/perf-baselines/.gitkeep`, `tests/perf-baselines/README.md`, `docs/perf/targets.md`.
- **New directories (2)**: `tests/perf-baselines/` (at workspace root), `docs/perf/`.
- **Modified files (4 + lockfile)**:
  - `crates/orgsidian-core/Cargo.toml` — `[features]` flag wiring + `serde_json` optional dep + `tempfile` dev-dep.
  - `crates/orgsidian-core/src/test_support/mod.rs` — 1-line addition (`pub mod perf;`) + 1 doc-comment update (stale forward-reference → past tense).
  - `crates/orgsidian-core/src/test_support/clock.rs` — 1-line doc-comment polish per Task 2.4.
  - `.github/workflows/pr.yml` — 1-line stale slot-reservation comment replacement (AC8 / Task 7).
  - `Cargo.lock` — `tempfile` + transitive deps.
- **No new crates**. The macro lives in existing `orgsidian-core::test_support::perf` per the epic AC literal wording.
- **No new workspace members**.
- **No new `.github/workflows/*` steps** — only the 1-line comment replacement.

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L601-L616](_bmad-output/planning-artifacts/epics.md#L601-L616) (Story 1.12 AC verbatim).
- Architecture macro spec: [_bmad-output/planning-artifacts/architecture.md#L331](_bmad-output/planning-artifacts/architecture.md#L331) (the `assert_no_perf_regression!(story-id, baseline_path, || { … })` literal call shape).
- Architecture LD-32 (CI matrix + perf gate headline): [_bmad-output/planning-artifacts/architecture.md#L521-L526](_bmad-output/planning-artifacts/architecture.md#L521-L526).
- Architecture NFR-20: [_bmad-output/planning-artifacts/architecture.md#L132](_bmad-output/planning-artifacts/architecture.md#L132).
- PRD §8 NFR-1..NFR-7 absolute targets (source of `docs/perf/targets.md` table rows): [_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md).
- test-design.md §6.9 Layer 9 (Perf Snapshot mechanism + consumed-by table): [_bmad-output/test-artifacts/test-design.md#L606-L626](_bmad-output/test-artifacts/test-design.md#L606-L626).
- test-design.md §7.3.10 (Perf-AC red-phase scaffold — the import-line the macro must support): [_bmad-output/test-artifacts/test-design.md#L909-L926](_bmad-output/test-artifacts/test-design.md#L909-L926).
- test-design.md TC-2 (runner_class hardware-scoping addendum): [_bmad-output/test-artifacts/test-design.md#L144-L150](_bmad-output/test-artifacts/test-design.md#L144-L150).
- test-design.md §5.1 directory layout (tests/perf-baselines/ placement): [_bmad-output/test-artifacts/test-design.md#L298-L313](_bmad-output/test-artifacts/test-design.md#L298-L313).
- pr.yml stale slot-reservation comment: [.github/workflows/pr.yml#L170](.github/workflows/pr.yml#L170).
- Existing `crates/orgsidian-core/Cargo.toml` (target of feature + deps edits): [crates/orgsidian-core/Cargo.toml](crates/orgsidian-core/Cargo.toml).
- Existing `crates/orgsidian-core/src/test_support/mod.rs` (target of `pub mod perf;` addition): [crates/orgsidian-core/src/test_support/mod.rs](crates/orgsidian-core/src/test_support/mod.rs).
- Existing `crates/orgsidian-core/src/test_support/clock.rs` (pattern reference for the new perf submodule shape): [crates/orgsidian-core/src/test_support/clock.rs](crates/orgsidian-core/src/test_support/clock.rs).
- Existing root workspace Cargo.toml (where `serde_json` is already a workspace dep at L38): [Cargo.toml](Cargo.toml).
- Existing pr.yml workspace test step (auto-discovers the new `tests/perf_canary.rs`): [.github/workflows/pr.yml#L119-L120](.github/workflows/pr.yml#L119-L120).
- Existing `orgsidian-watcher` tempfile dev-dep (convention reference for orgsidian-core's new dev-dep): [crates/orgsidian-watcher/Cargo.toml](crates/orgsidian-watcher/Cargo.toml).
- Existing `cargo deny` ledger (advisory-exceptions surface): [docs/security/advisory-exceptions.md](docs/security/advisory-exceptions.md).
- Previous story (1.11, status: done): [_bmad-output/implementation-artifacts/1-11-establish-ld-41-failure-mode-test-harness-party-mode-round-2-p0-murat.md](_bmad-output/implementation-artifacts/1-11-establish-ld-41-failure-mode-test-harness-party-mode-round-2-p0-murat.md).
- Story 1.9 anchor smoke (pattern reference for `test_support` feature wiring): [_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md](_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md).
- `serde_json` crate (workspace dep, latest stable on 1.x): https://docs.rs/serde_json/latest/serde_json/.
- `tempfile` crate (latest stable on 3.x): https://docs.rs/tempfile/latest/tempfile/.
- Rust `std::env::consts` (`OS` / `ARCH` constants used for `runner_class`): https://doc.rust-lang.org/std/env/consts/index.html.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context) via `bmad-dev-story` skill on 2026-05-28.

### Debug Log References

AC9 dev-box matrix (macOS-arm64, workspace root) — all cells green:

| Cell | Expected | Observed |
|---|---|---|
| `ls crates/orgsidian-core/src/test_support/perf.rs` | present | ✓ |
| `ls crates/orgsidian-core/tests/perf_canary.rs` | present | ✓ |
| `ls tests/perf-baselines/.gitkeep tests/perf-baselines/README.md` | both | ✓ |
| `ls docs/perf/targets.md` | present | ✓ |
| `grep -c '^pub fn assert_no_perf_regression_impl' …perf.rs` | `1` | `1` |
| `grep -c 'macro_rules! assert_no_perf_regression' …perf.rs` | `1` | `1` |
| `grep -c '#\[macro_export\]' …perf.rs` | `1` | `1` (after de-quoting one comment) |
| `grep -c 'pub enum PerfOutcome' …perf.rs` | `1` | `1` |
| `grep -c 'TOLERANCE_PCT.*= 20' …perf.rs` | ≥`1` | `1` (case-corrected post review P8) |
| `grep -c 'runner_class' …perf.rs` | ≥`3` | `18` |
| `grep -c 'pub mod perf' …test_support/mod.rs` | `1` | `1` |
| 6 test fns grep | `6` | `6` |
| 3 outcome variants grep in canary | ≥`3` | `5` |
| `cargo fmt --all -- --check` | exit 0 | ✓ (after one auto-format pass) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | ✓ |
| `cargo build --workspace --locked` | exit 0 | ✓ |
| `cargo test --workspace --locked` | exit 0 | ✓; 6/6 canary tests pass |
| `cargo test -p orgsidian-core --test perf_canary --locked --features test-support` | exit 0 | ✓; 6/6 pass |
| `... -- --nocapture | grep -c 'perf: writing initial baseline'` | ≥`2` | `3` |
| `cargo deny --locked check all` | exit 0 | ✓ (`advisories ok, bans ok, licenses ok, sources ok`; pre-existing `unused-wrapper` warnings only) |
| `cargo audit --deny warnings` (with `.cargo/audit-ignore.txt`) | exit 0 | ✓ (same ignore set as pr.yml; no new advisories) |
| `ls tests/perf-baselines/` | only `.gitkeep` + `README.md` | ✓ |
| `grep -c 'lands here' .github/workflows/pr.yml` (stale) | `0` | `0` |
| `grep -c 'now wired' .github/workflows/pr.yml` | `1` | `1` |
| `git diff --stat _bmad-output/planning-artifacts/` | empty | empty ✓ |
| `git diff --stat docs/` (excluding new docs/perf/) | empty | empty ✓ |

### Completion Notes List

- Built `assert_no_perf_regression!` as the canonical perf gate consumed by every downstream perf-AC story (Stories 4.3a-g, 6.3, 7.1, 8.1, 8.4, 8.11, 9.1, 10.6, 11.8). Macro/impl split is enforced: impl returns `PerfOutcome`, only the macro panics — keeps self-tests sleep-free per Dev Notes §3.
- Hand-rolled `current_iso8601_utc` via Howard Hinnant `civil_from_days` (~25 LOC). No `chrono` / `time` pulled in, consistent with §11 / anti-pattern §14.7.
- `BaselineFile.baselines` uses `BTreeMap<String, BaselineEntry>` so on-disk JSON key order is deterministic (anti-pattern §14.14). Persisted `median_ns` is `u64` (lossless for nanosecond budgets ≪ u64::MAX); in-memory arithmetic stays `u128` per AC2.3.
- `#[macro_export]` hoists to crate root; the `#[doc(inline)] pub use crate::assert_no_perf_regression;` at the foot of `perf.rs` makes the submodule path equivalent — both call sites (crate-root and `test_support::perf::*`) compile cleanly, matching the test-design.md §7.3.10 scaffold's literal import (§4).
- `tests/perf-baselines/.gitkeep` plus `README.md` only — no pre-written story-N.M JSON files (owner stories write their own baselines via missing-baseline mode per AC8 / AC6).
- Decision-grade question §10 (workflow stale-comment cleanup): proceeded with interpretation 1 (single-line replacement at [pr.yml:170](.github/workflows/pr.yml#L170)) as locked by the story spec. Open for reviewer signoff to flip to interpretation 2 if preferred — would revert Task 7 and the AC9 grep cells.
- `cargo audit --deny warnings` run with the pr.yml-equivalent `.cargo/audit-ignore.txt` ignore list — no new advisories surfaced by the `tempfile` + `serde_json`-activation path.
- Issue #12 label: `status:backlog` → `status:in-progress` at start; will move to `status:in-review` at PR open per [[project_orgsidian_github_label_scheme]].

### File List

**New files (5):**
- `crates/orgsidian-core/src/test_support/perf.rs`
- `crates/orgsidian-core/tests/perf_canary.rs`
- `tests/perf-baselines/.gitkeep`
- `tests/perf-baselines/README.md`
- `docs/perf/targets.md`

**Modified files (4 + lockfile):**
- `crates/orgsidian-core/Cargo.toml`
- `crates/orgsidian-core/src/test_support/mod.rs`
- `crates/orgsidian-core/src/test_support/clock.rs`
- `.github/workflows/pr.yml`
- `Cargo.lock`

### Review Findings

Generated by `bmad-code-review` on 2026-05-28. Diff scope: working-tree (pre-PR). Reviewer layers: Blind Hunter + Edge Case Hunter + Acceptance Auditor (all green).

#### Decision-needed (resolve before patches)

- [x] [Review][Decision] **Baseline JSON schema fields (`story_id`, `tolerance_pct`, `samples`) are silently ignored on parse** — impl reads the file then uses the in-source constants for comparison ([perf.rs:207-211](crates/orgsidian-core/src/test_support/perf.rs#L207-L211)). Two reasonable designs: (a) treat on-disk fields as advisory metadata only (drop them from the schema or keep them but document the asymmetry); (b) assert equality on read and surface a clear error / `PerfOutcome::SchemaMismatch` when a hand-edited file disagrees. The spec AC2.5 prescribes the field set but does not specify validation behavior. Source: blind+edge.
- [x] [Review][Decision] **Absolute `$baseline_path` silently bypasses `workspace_root()`** — `workspace_root().join(absolute_path)` returns the absolute path per `Path::join` semantics ([perf.rs:175](crates/orgsidian-core/src/test_support/perf.rs#L175)). The canary tests *rely* on this (they pass tempdir absolute paths and would break under a strict `assert!(baseline_path.is_relative())`). Options: (a) keep current behavior + add a doc-line documenting it; (b) reject absolute paths in production but expose a `*_with_absolute_path` variant for tests; (c) accept silently (status quo). Source: edge.
- [x] [Review][Decision] **Windows line-ending churn on committed baseline JSON files** — `serde_json::to_string_pretty` emits `\n`; Windows checkouts with `core.autocrlf=true` convert to `\r\n`, causing every subsequent write to dirty the file. Options: (a) add `tests/perf-baselines/*.json text eol=lf` to `.gitattributes`; (b) accept the noise (Windows leg is not in v0.1 Alpha CI matrix yet — see Story 1.8); (c) normalize on read. Source: edge.

#### Patch (unambiguous fixes — apply now or leave as action items)

- [x] [Review][Patch] **`workspace_root()` substring scan matches the wrong directory (CRITICAL)** [crates/orgsidian-core/src/test_support/perf.rs:81-98] — `txt.contains("[workspace]")` matches a comment at [crates/orgsidian-core/Cargo.toml:48](crates/orgsidian-core/Cargo.toml#L48) (the literal string `[workspace]` appears inside a comment). The walk-up returns `crates/orgsidian-core` instead of the repo root. Self-tests don't catch it because they pass tempdir absolute paths (which the `join` discards). The very first downstream consumer that follows `docs/perf/targets.md`'s documented pattern (relative path `"tests/perf-baselines/story-X.Y.json"`) will write to `crates/orgsidian-core/tests/perf-baselines/…` instead of `<repo>/tests/perf-baselines/…`. Fix: match a TOML line that *starts with* `[workspace]` after trimming (e.g., `txt.lines().any(|l| l.trim_start().starts_with("[workspace]"))`), or parse the TOML, or additionally require absence of a `[package]` line in the same file. Verified empirically by `grep` on the offending file. Source: blind+edge.
- [x] [Review][Patch] **`perf_canary.rs` does not compile under `cargo test -p orgsidian-core --locked` (CRITICAL)** [crates/orgsidian-core/Cargo.toml] — `tests/perf_canary.rs` uses `serde_json` directly but `serde_json` is gated behind `test-support`. The file is auto-discovered (no `[[test]]` block, no `required-features`), so the standalone per-crate test command emits 6 `E0433` errors. AC9 hides this because every cell that runs the canary explicitly passes `--features test-support` (or relies on workspace feature unification via `orgsidian-watcher`). Fix: declare an explicit `[[test]]` block matching the existing `failure_modes` pattern at [crates/orgsidian-core/Cargo.toml:51-58](crates/orgsidian-core/Cargo.toml#L51-L58): `[[test]] name = "perf_canary" path = "tests/perf_canary.rs" required-features = ["test-support"]`. Verified empirically. Source: edge.
- [x] [Review][Patch] **Macro double-evaluates `$baseline_path`** [crates/orgsidian-core/src/test_support/perf.rs:267-301] — `$baseline_path` is interpolated into the panic format string AND passed to `Path::new(...)`, so a side-effecting expression (e.g., `compute_path()`) executes twice. Fix: bind `let baseline_path = $baseline_path;` at the top of the macro body and use the binding everywhere. Standard macro-hygiene idiom. Source: blind.
- [x] [Review][Patch] **TOCTOU race between `full_path.exists()` and subsequent `read_to_string`** [crates/orgsidian-core/src/test_support/perf.rs:183-208] — parallel `cargo test` runs targeting the same baseline file can pass `exists()` then hit `expect("perf: failed to read existing baseline file")` if the other process deletes/replaces between the calls. Fix: collapse to `match fs::read_to_string(&full_path)` and treat `io::ErrorKind::NotFound` as the first-run branch. Eliminates the probe. Source: blind+edge.
- [x] [Review][Patch] **Macro `$op:expr` must be `Fn` (callable ≥5×), not `FnOnce` — undocumented** [crates/orgsidian-core/src/test_support/perf.rs:249-264] — the rustdoc shows `|| { /* op */ }` without flagging the callability constraint. A consumer passing `move || consume(owned_value)` will hit a confusing compile error on iteration 2. Fix: add one doc-line to the macro rustdoc: `The closure must be re-callable (`Fn`/`FnMut`); a `FnOnce` will fail to compile after the first sample.`. Source: blind+edge.
- [x] [Review][Patch] **Smoke test writes `median_ns: 0` on hardware where `Instant::elapsed()` rounds to 0 ns** [crates/orgsidian-core/src/test_support/perf.rs + crates/orgsidian-core/tests/perf_canary.rs:124-141] — `perf_macro_smoke_writes_initial_baseline` exercises the macro on `|| {}` (empty closure). On systems where the elapsed-ns rounds to 0 the baseline is stored as `median_ns: 0`, after which `threshold = 0 * 120 / 100 = 0` and any non-zero subsequent measurement regresses → permanent first-run lock-in if the file were ever persistent (today it's tempdir-only, so harmless). Fix: in the impl, reject `measured_median == 0` with `panic!("perf: zero-nanosecond median is unrealistic — refusing to write baseline")` (impl-level guard, not a `PerfOutcome` variant); update the smoke test to use a closure that does measurable work (e.g., `|| { std::hint::black_box(42u64.wrapping_mul(7)); }`). Source: blind.
- [x] [Review][Patch] **`tempfile = "3"` is bare in `crates/orgsidian-core/Cargo.toml`, contrary to `[[feedback_version_policy]]` single-source-of-truth** [crates/orgsidian-core/Cargo.toml:43] — Story 1.11 set the precedent with `fail = { workspace = true, features = [...] }`. The current line `tempfile = "3"` does match the existing `orgsidian-watcher` convention but violates the broader single-source pin discipline. Fix: add `tempfile = "3"` to root `[workspace.dependencies]` and change the dev-dep to `tempfile = { workspace = true }`. (Defer if user prefers to bundle with a future workspace-dep tidy.) Source: blind.
- [x] [Review][Patch] **AC9 grep cell mismatch: `tolerance_pct.*20\b` is case-sensitive but the source has `TOLERANCE_PCT: u128 = 20`** [_bmad-output/implementation-artifacts/1-12-….md AC9 row] — the cell as written would output `0` (not `1`); the dev log claims `1`. Spec drift, not source drift. Fix in spec: either `grep -ic` or change the literal to `TOLERANCE_PCT.*= 20`. Semantic invariant (constant 20 present) is satisfied. Source: auditor.

#### Defer (cosmetic / non-correctness)

- [x] [Review][Defer] **Memoize `workspace_root()` via `LazyLock` to avoid repeated filesystem I/O during a `cargo test` run** [crates/orgsidian-core/src/test_support/perf.rs:81-98] — deferred, optimization not correctness (perf-gate I/O could mildly perturb its own samples but the effect is sub-microsecond).
- [x] [Review][Defer] **`current_runner_class()` duplicated in `tests/perf_canary.rs:17-19`** — deferred, forced by `pub(crate)` visibility; promote to `pub` in a follow-up to remove drift risk.
- [x] [Review][Defer] **Panic-message `pct` integer-floor rounding** [crates/orgsidian-core/src/test_support/perf.rs:281-289] — deferred, cosmetic in the diagnostic only; gate decision uses correct arithmetic.
- [x] [Review][Defer] **`current_iso8601_utc()` swallows pre-UNIX clock as `1970-01-01T00:00:00Z`** — deferred, cosmetic; `created_at` is forensic metadata only, not read by the gate.
- [x] [Review][Defer] **Self-test assertions use `assert_eq!(parsed[…], 100u64)` which relies on `serde_json::Value: PartialEq<u64>`** [crates/orgsidian-core/tests/perf_canary.rs:38-40,97-100] — deferred, works today; brittle to a future schema change that serializes as float.

#### Pre-PR gate (deferred until PR open)

- [ ] [Review][PR-Gate] **`Closes #12` must appear in the PR body** — workflow prepend step requires `Closes #<N>` matching the story's GitHub issue (`github_issue: 12` in Metadata). PR is not yet open; surface here so it is not forgotten at PR-create time. Story 1.11 had to fix this up post-open ([commit 0ab6316](https://github.com/orgsidian/orgsidian/commit/0ab6316) in Story 1.11 PR #126).

## Change Log

| Date       | Change                                                                                                  | Author                                |
| ---------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-28 | Story 1.12 contextualized via `bmad-create-story` (ready-for-dev).                                      | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-28 | Story 1.12 implemented end-to-end (perf macro + canary + docs); AC9 matrix green; status → review.       | Amelia (`bmad-dev-story`) for Tiziano |
| 2026-05-28 | `bmad-code-review` run: 2 critical patches (workspace_root substring scan; perf_canary missing required-features), 6 minor patches, 3 decision-needed, 5 deferred, 5 dismissed. | Code reviewer for Tiziano             |
| 2026-05-28 | All 11 patches applied + 7-cell AC9 re-verification green (fmt/clippy/workspace test/canary 7/7/standalone -p core/deny/audit/grep). Story → done. PR still to be opened with `Closes #12`. | Code reviewer for Tiziano             |
