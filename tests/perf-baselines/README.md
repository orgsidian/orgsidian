# `tests/perf-baselines/`

Per-story performance baselines consumed by `assert_no_perf_regression!` (Story 1.12).

Each baseline file is named `story-<N.M>-<surface>.json` and follows this shape
(per Story 1.12 AC2.5):

```json
{
  "story_id": "story-1.12-self-test-canary",
  "tolerance_pct": 20,
  "samples": 5,
  "baselines": {
    "macos-aarch64": {
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

Baselines are auto-written on first run (missing-baseline mode). Commit the JSON
file alongside the perf-AC story PR that introduced it.

For absolute PRD §8 NFR targets (informational, distinct from the regression
gate), see [`docs/perf/targets.md`](../../docs/perf/targets.md). The macro
mechanism is specified in [test-design.md §6.9](../../_bmad-output/test-artifacts/test-design.md).
