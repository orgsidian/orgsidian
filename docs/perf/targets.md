# Performance Targets — Orgsidian

This document records the **absolute performance budgets** from PRD §8 NFRs
(calibrated to 2020+ M1 / equivalent x86_64 hardware). They are documented here
as **design targets** — they are NOT the regression gate.

The per-PR regression gate is `assert_no_perf_regression!` from
[`orgsidian_core::test_support::perf`](../../crates/orgsidian-core/src/test_support/perf.rs)
which compares the median of 5 samples against a per-story / per-`runner_class`
baseline JSON committed under [`tests/perf-baselines/`](../../tests/perf-baselines/),
allowing ±20 %.

The relationship: **absolute targets** define "is this story's baseline
acceptable at all"; the **regression gate** defines "did the next change break
it." See [architecture.md L331](../../_bmad-output/planning-artifacts/architecture.md)
for the macro spec.

## Targets

| NFR | Surface | Target | Calibration | Owner | Anchor |
|---|---|---|---|---|---|
| NFR-1 | App startup (cold) | <2 s | 1000-file vault | Story 13.x polish | architecture.md L45 |
| NFR-2 | Editor typing latency | <30 ms | n/a | Story 4.3a–g | architecture.md L45 |
| NFR-3 | Agenda recompute (incremental) | <100 ms | 1000-file vault | Story 7.1 + 7.4 | architecture.md L45 |
| NFR-4 (split per PRD §4.3 FR-12) | FTS5 search — first 10 results | <100 ms | 1000-file vault | Story 8.4 | epics.md L1687 |
| NFR-4 (split) | FTS5 search — full 50 results | <200 ms | 1000-file vault | Story 8.4 | epics.md L1687 |
| NFR-5 | Quick Capture end-to-end (hotkey → persist) | <1 s | n/a | Story 8.1 | epics.md L1639 |
| NFR-6 | Editor open 5000-line `.org` | <300 ms | n/a | Story 4.3g | test-design.md §3.1 R-015 |
| NFR-7 | Resident memory | <500 MB | 1000-file vault | Story 4.9 (soak) | architecture.md L45 |
| FR-14 | Project Report typical scope | <5 s | 50 headlines, 4 weeks | Story 10.6 | epics.md L2005 |
| FR-26 / LD-56 | Graph View 5k-node force-directed render | ≤2 s | 5000 nodes | Story 8.11 | architecture.md §LD-56 |
| FR-26 / LD-56 | Graph View steady-state frame | ≤500 ms | 5000 nodes | Story 8.11 | architecture.md §LD-56 |
| — | Merge Dialog open | ≤2 s | conflict-detection-trigger | Story 9.1 | epics.md L1849 |

## Gate vs Target — why ±20 % relative, not absolute thresholds

GitHub Actions runners are heterogeneous (M-series, Intel, AMD, varying I/O
contention). A test that asserts "≤500 ms absolute" is flaky on noisy CI even
when the code is correct, because the threshold collides with normal hardware
variance. Conversely, a hardcoded threshold that's loose enough never to be
flaky is too loose to catch real regressions.

The `assert_no_perf_regression!` macro avoids this trap. It compares each PR's
measurement to the committed baseline FOR THE SAME `runner_class` (`{os}-{arch}`,
e.g., `macos-aarch64`, `linux-x86_64`) — never across hardware classes. The
±20 % tolerance is generous enough that GHA noise rarely tips a passing test
into red, yet tight enough that a real regression (typically ≥30 %) is caught.
This is the TC-2 hardware-heterogeneity argument from
[test-design.md L201-L218](../../_bmad-output/test-artifacts/test-design.md).

Absolute NFR targets (this table) remain the **design contract**: an owner
story may not commit a first baseline whose median is meaningfully worse than
the NFR target without explicit reviewer signoff. Once committed, the
regression gate keeps the surface honest against its own baseline.

## Baseline workflow

1. Author the perf-AC test with `assert_no_perf_regression!`, passing a unique
   `story_id` (e.g., `"story-8.4-search-10results"`) and a relative
   `baseline_path` (e.g., `"tests/perf-baselines/story-8.4-search-10results.json"`).
2. First CI run on a fresh `runner_class` writes the baseline JSON file and
   emits a one-line `eprintln!` warning — the test passes.
3. Commit the baseline JSON alongside the story PR. Cross-class entries are
   added by subsequent first-runs of additional CI matrix legs (each writes
   its own `runner_class` entry into the same file).
4. Subsequent PRs run the same test against the committed baseline. Median of
   5 samples within ±20 % of the recorded median passes silently.
5. When a deliberate perf change shifts a baseline, update the JSON in the
   same PR with a `perf:` Conventional Commit type (per
   [architecture.md L606](../../_bmad-output/planning-artifacts/architecture.md) /
   LD-54). Reviewers see the delta directly in the diff.

`runner_class` derivation uses `std::env::consts::OS` + `ARCH` (compile-time
constants), NOT `RUNNER_OS` / `RUNNER_ARCH` env vars, so a developer's local
`cargo test` on the same hardware class as a CI runner produces a comparable
baseline.

<!-- Regeneration: this file is hand-curated; update on any PRD §8 NFR change or addition of a perf-AC story. -->
