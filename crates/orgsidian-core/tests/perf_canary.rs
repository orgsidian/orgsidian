//! Canary self-tests for the Story 1.12 perf snapshot regression macro.
//!
//! Validates the IMPL (pure-policy `assert_no_perf_regression_impl`) end-to-end
//! across all `PerfOutcome` variants plus a single MACRO smoke test. This file
//! is the only Story 1.12-internal consumer of the macro; downstream perf-AC
//! stories add their own `assert_no_perf_regression!` callers in their own PRs.
//!
//! Anti-flakiness discipline (Story 1.12 Dev Notes §3): self-tests feed
//! synthetic `samples_ns` slices to the IMPL — no `std::thread::sleep`, no
//! wall-clock waits. Only `perf_macro_smoke_writes_initial_baseline` exercises
//! the timing harness, and it does so on an empty closure (microsecond scale).

use orgsidian_core::test_support::perf::{assert_no_perf_regression_impl, PerfOutcome};
use std::path::Path;
use tempfile::tempdir;

fn current_runner_class() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[test]
fn perf_impl_writes_baseline_on_first_run() {
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-first-run.json");

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[100, 100, 100, 100, 100],
    );

    assert_eq!(report.outcome, PerfOutcome::BaselineWritten);
    assert!(baseline_path.exists(), "baseline file must be written");
    let raw = std::fs::read_to_string(&baseline_path).expect("read baseline");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse baseline");
    let class = current_runner_class();
    assert_eq!(
        parsed["baselines"][&class]["median_ns"], 100u64,
        "median_ns for current runner_class must match measured median"
    );
}

#[test]
fn perf_impl_passes_within_tolerance() {
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-within.json");
    seed_baseline(&baseline_path, &current_runner_class(), 100);

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[105, 110, 115, 120, 118],
    );

    assert_eq!(report.outcome, PerfOutcome::WithinTolerance);
    assert_eq!(report.measured_ns, 115);
    assert_eq!(report.baseline_ns, Some(100));
}

#[test]
fn perf_impl_flags_regression_beyond_tolerance() {
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-regress.json");
    seed_baseline(&baseline_path, &current_runner_class(), 100);

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[130, 135, 140, 145, 150],
    );

    assert_eq!(report.outcome, PerfOutcome::Regressed);
    assert_eq!(report.measured_ns, 140);
    assert_eq!(report.baseline_ns, Some(100));
}

#[test]
fn perf_impl_treats_missing_runner_class_as_first_run() {
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-missing-class.json");
    seed_baseline(&baseline_path, "never-matches-anything-xyz", 999_999_999);

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[100, 100, 100, 100, 100],
    );

    assert_eq!(report.outcome, PerfOutcome::BaselineWritten);
    let raw = std::fs::read_to_string(&baseline_path).expect("read baseline");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse baseline");
    let class = current_runner_class();
    assert!(
        parsed["baselines"]["never-matches-anything-xyz"].is_object(),
        "original synthetic runner_class entry must be preserved"
    );
    assert_eq!(
        parsed["baselines"][&class]["median_ns"], 100u64,
        "new entry for current runner_class must be appended"
    );
}

#[test]
fn perf_impl_flags_schema_mismatch_on_disk_drift() {
    // Story 1.12 review P9: on-disk schema fields (story_id, tolerance_pct,
    // samples) must equal the in-source gate parameters or the impl returns
    // PerfOutcome::SchemaMismatch. Hand-craft a file whose story_id disagrees
    // with the in-source story_id argument.
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-schema-mismatch.json");
    let json = r#"{
  "story_id": "story-completely-different-id",
  "tolerance_pct": 20,
  "samples": 5,
  "baselines": {
    "any-runner": { "median_ns": 100, "created_at": "2026-05-28T00:00:00Z" }
  }
}
"#;
    std::fs::write(&baseline_path, json).expect("write seed");

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[100, 100, 100, 100, 100],
    );

    match report.outcome {
        PerfOutcome::SchemaMismatch {
            field,
            on_disk,
            in_source,
        } => {
            assert_eq!(field, "story_id");
            assert_eq!(on_disk, "story-completely-different-id");
            assert_eq!(in_source, "story-1.12-self-test-canary");
        }
        other => panic!("expected SchemaMismatch, got {other:?}"),
    }
}

#[test]
fn perf_impl_median_of_5_is_robust_to_outliers() {
    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-median.json");
    seed_baseline(&baseline_path, &current_runner_class(), 100);

    let report = assert_no_perf_regression_impl(
        "story-1.12-self-test-canary",
        &baseline_path,
        &[100, 100, 100, 1_000_000_000, 1_000_000_000],
    );

    assert_eq!(
        report.outcome,
        PerfOutcome::WithinTolerance,
        "median (100) must dominate over mean — anti-regression guard"
    );
    assert_eq!(report.measured_ns, 100);
}

#[test]
fn perf_macro_smoke_writes_initial_baseline() {
    use orgsidian_core::test_support::perf::assert_no_perf_regression;

    let dir = tempdir().expect("tempdir");
    let baseline_path = dir.path().join("story-canary-macro-smoke.json");
    let baseline_path_str = baseline_path.to_str().expect("utf8 path");

    // Story 1.12 review P6: a `|| {}` empty closure can have `as_nanos() == 0`
    // on systems where Instant's resolution rounds sub-tick elapsed to zero,
    // which the impl now correctly rejects (zero-median lock-in guard). The
    // closure must do enough work to clear the coarsest Instant resolution
    // observed on supported platforms — Windows QueryPerformanceCounter is
    // ~100 ns; the nightly Windows runner regularly returns 0 ns for a single
    // `wrapping_mul` even with `black_box`. A 10k-iteration loop puts the
    // closure at ~5–50 µs on any modern runner (well above QPC tick + below
    // the perf-noise floor) and remains immune to LLVM dead-code elimination
    // via the per-iteration `black_box`.
    assert_no_perf_regression!(
        "story-1.12-self-test-canary-macro",
        baseline_path_str,
        || {
            let mut acc: u64 = 1;
            for i in 0..10_000u64 {
                acc = std::hint::black_box(acc.wrapping_mul(i.wrapping_add(7)));
            }
            std::hint::black_box(acc);
        }
    );

    assert!(
        baseline_path.exists(),
        "macro must have routed through the impl and written the baseline file"
    );
}

fn seed_baseline(path: &Path, runner_class: &str, median_ns: u64) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create tempdir parent");
    }
    let json = format!(
        r#"{{
  "story_id": "story-1.12-self-test-canary",
  "tolerance_pct": 20,
  "samples": 5,
  "baselines": {{
    "{runner_class}": {{
      "median_ns": {median_ns},
      "created_at": "2026-05-28T00:00:00Z"
    }}
  }}
}}
"#
    );
    std::fs::write(path, json).expect("write seed baseline");
}
