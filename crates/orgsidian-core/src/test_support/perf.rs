//! Perf snapshot regression infrastructure (Story 1.12 — LD-32 / NFR-20).
//!
//! Provides `assert_no_perf_regression!` plus its underlying impl
//! ([`assert_no_perf_regression_impl`]) as the canonical perf-gate mechanism
//! consumed by every perf-AC story from Story 4.3a onwards. Semantics
//! (median-of-5, ±20 % tolerance, missing-baseline bootstrap, `runner_class`
//! scoping per TC-2) are locked in [test-design.md §6.9](
//! ../../../../_bmad-output/test-artifacts/test-design.md) and
//! [architecture.md L331](../../../../_bmad-output/planning-artifacts/architecture.md).
//!
//! Macro / impl are split (impl is pure policy; macro adds the 5-sample timing
//! harness) so self-tests can feed synthetic `samples_ns` slices without
//! sleep-based flakiness — see Story 1.12 Dev Notes §3.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sample count hard-coded by the macro (LD-32 spec — see Story 1.12 AC2.1).
pub const SAMPLES: usize = 5;
/// Regression tolerance percent vs baseline median (architecture.md L331).
pub const TOLERANCE_PCT: u128 = 20;

/// Outcome discriminant returned by [`assert_no_perf_regression_impl`].
///
/// The impl never panics; the macro panics only on [`PerfOutcome::Regressed`]
/// so impl self-tests can assert behavior via `assert_eq!` rather than
/// `#[should_panic]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerfOutcome {
    /// File or `runner_class` entry was missing; the impl wrote it. Non-fatal.
    BaselineWritten,
    /// Measured median is within the ±20 % tolerance vs the recorded baseline.
    WithinTolerance,
    /// Measured median exceeds the tolerance. Macro turns this into a panic.
    Regressed,
    /// Baseline file on disk disagrees with the in-source gate parameters
    /// (`story_id`, `tolerance_pct`, or `samples`). Story 1.12 review P9:
    /// surface the drift instead of silently using the in-source constants
    /// while the file shows different values. Macro panics on this variant.
    SchemaMismatch {
        field: &'static str,
        on_disk: String,
        in_source: String,
    },
}

/// Structured perf result, returned by the impl for self-tests and tooling.
#[derive(Debug, Clone)]
pub struct PerfReport {
    pub outcome: PerfOutcome,
    pub story_id: String,
    pub runner_class: String,
    pub measured_ns: u128,
    pub baseline_ns: Option<u128>,
    pub samples: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineEntry {
    median_ns: u64,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    story_id: String,
    tolerance_pct: u32,
    samples: u32,
    // BTreeMap keeps the on-disk JSON key order deterministic — re-running
    // missing-baseline mode against an existing file produces a stable diff
    // (anti-pattern §14.14).
    baselines: BTreeMap<String, BaselineEntry>,
}

/// Returns the `runner_class` for the current build target, derived from
/// compile-time constants (NOT `RUNNER_OS` / `RUNNER_ARCH` env vars).
/// See Story 1.12 Dev Notes §5 for the rationale.
pub(crate) fn current_runner_class() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Walks up from `CARGO_MANIFEST_DIR` until it finds the first `Cargo.toml`
/// containing a `[workspace]` table. Panics if none is found.
///
/// `baseline_path` is always resolved relative to the returned path so every
/// consumer (regardless of which crate hosts the test) writes to a uniform
/// `<repo>/tests/perf-baselines/` location. See Dev Notes §8.
pub(crate) fn workspace_root() -> PathBuf {
    // Match `[workspace]` as a TOML *section header*, not as a substring of a
    // comment or string. A bare `txt.contains("[workspace]")` falsely matches
    // the literal `[workspace]` inside `#`-comments (e.g. the comment in this
    // very crate's Cargo.toml that documents the virtual-workspace layout) and
    // returns the crate dir instead of the repo root — see Story 1.12 review
    // P1.
    let is_workspace_root = |txt: &str| {
        txt.lines()
            .any(|l| l.trim_start().starts_with("[workspace]"))
    };
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let txt = std::fs::read_to_string(&cargo_toml).unwrap_or_default();
            if is_workspace_root(&txt) {
                return dir;
            }
        }
        if !dir.pop() {
            panic!(
                "perf: could not find workspace root from CARGO_MANIFEST_DIR={}",
                env!("CARGO_MANIFEST_DIR")
            );
        }
    }
}

/// Returns the current UTC instant formatted as `YYYY-MM-DDTHH:MM:SSZ`.
/// Hand-rolled to avoid pulling `chrono` / `time` into a leaf crate (Dev Notes §11).
pub(crate) fn current_iso8601_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = secs_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn secs_to_ymdhms(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    // Howard Hinnant `civil_from_days` (public domain).
    let sec = (secs % 60) as u32;
    let minute = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;
    let days = (secs / 86400) as i64;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32, hour, minute, sec)
}

fn median_of_5(samples_ns: &[u128]) -> u128 {
    assert_eq!(
        samples_ns.len(),
        SAMPLES,
        "perf: expected exactly {SAMPLES} samples, got {}",
        samples_ns.len()
    );
    let mut sorted: Vec<u128> = samples_ns.to_vec();
    sorted.sort_unstable();
    sorted[SAMPLES / 2]
}

fn write_baseline_file(
    full_path: &Path,
    story_id: &str,
    baselines: BTreeMap<String, BaselineEntry>,
) -> std::io::Result<()> {
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = BaselineFile {
        story_id: story_id.to_string(),
        tolerance_pct: TOLERANCE_PCT as u32,
        samples: SAMPLES as u32,
        baselines,
    };
    let json = serde_json::to_string_pretty(&payload)
        .expect("perf: BaselineFile serialization is infallible");
    std::fs::write(full_path, format!("{json}\n"))
}

/// Pure policy entrypoint: callers pre-compute the 5 timing samples; this fn
/// handles median, baseline I/O, comparison, warning emission.
///
/// The impl never panics on regression — it returns
/// [`PerfOutcome::Regressed`]. The macro is what consumer stories use, and
/// the macro is what panics. This split makes self-tests deterministic
/// (Dev Notes §3).
pub fn assert_no_perf_regression_impl(
    story_id: &str,
    baseline_path: &Path,
    samples_ns: &[u128],
) -> PerfReport {
    let measured_median = median_of_5(samples_ns);
    // Story 1.12 review P6: a zero-nanosecond median is almost certainly a
    // measurement bug (Instant resolution, empty closure with `as_nanos()`
    // rounding to 0). Writing such a baseline locks the surface forever:
    // threshold = 0 * 120 / 100 = 0, so any non-zero subsequent measurement
    // trips a false regression. Refuse to even reach the I/O path.
    assert!(
        measured_median > 0,
        "perf: zero-nanosecond median for {story_id} — likely a measurement \
         scaffolding bug (empty closure, sub-Instant-resolution op). Refusing \
         to write a degenerate baseline (would lock the surface at threshold 0)."
    );
    let runner_class = current_runner_class();
    let full_path = workspace_root().join(baseline_path);

    let emit_warning = |median_ns: u128| {
        eprintln!(
            "perf: writing initial baseline for {story_id} on {runner_class} ({median_ns} ns) — RE-RUN required for regression gating"
        );
    };

    // Story 1.12 review P4: collapse exists() + read into a single
    // read_to_string + match on NotFound. Eliminates the TOCTOU window in
    // which a parallel test could delete/replace the file between the probe
    // and the read, which would currently panic with a misleading
    // "failed to read existing baseline file" message.
    let raw = match std::fs::read_to_string(&full_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut baselines = BTreeMap::new();
            baselines.insert(
                runner_class.clone(),
                BaselineEntry {
                    median_ns: measured_median as u64,
                    created_at: current_iso8601_utc(),
                },
            );
            write_baseline_file(&full_path, story_id, baselines)
                .expect("perf: failed to write initial baseline file");
            emit_warning(measured_median);
            return PerfReport {
                outcome: PerfOutcome::BaselineWritten,
                story_id: story_id.to_string(),
                runner_class,
                measured_ns: measured_median,
                baseline_ns: None,
                samples: SAMPLES,
            };
        }
        Err(e) => panic!("perf: failed to read baseline file {full_path:?}: {e}"),
    };
    let mut payload: BaselineFile = serde_json::from_str(&raw)
        .expect("perf: failed to parse baseline JSON — file may be corrupt");

    // Story 1.12 review P9 + D1: assert the on-disk schema fields match the
    // in-source gate parameters. Silent drift between disk (advisory metadata)
    // and the actual gate would let hand-edits to `tolerance_pct` look
    // effective while the gate still uses TOLERANCE_PCT, or let two stories
    // collide on the same baseline file without detection.
    if payload.story_id != story_id {
        return PerfReport {
            outcome: PerfOutcome::SchemaMismatch {
                field: "story_id",
                on_disk: payload.story_id.clone(),
                in_source: story_id.to_string(),
            },
            story_id: story_id.to_string(),
            runner_class,
            measured_ns: measured_median,
            baseline_ns: None,
            samples: SAMPLES,
        };
    }
    if u128::from(payload.tolerance_pct) != TOLERANCE_PCT {
        return PerfReport {
            outcome: PerfOutcome::SchemaMismatch {
                field: "tolerance_pct",
                on_disk: payload.tolerance_pct.to_string(),
                in_source: TOLERANCE_PCT.to_string(),
            },
            story_id: story_id.to_string(),
            runner_class,
            measured_ns: measured_median,
            baseline_ns: None,
            samples: SAMPLES,
        };
    }
    if payload.samples as usize != SAMPLES {
        return PerfReport {
            outcome: PerfOutcome::SchemaMismatch {
                field: "samples",
                on_disk: payload.samples.to_string(),
                in_source: SAMPLES.to_string(),
            },
            story_id: story_id.to_string(),
            runner_class,
            measured_ns: measured_median,
            baseline_ns: None,
            samples: SAMPLES,
        };
    }

    if let Some(entry) = payload.baselines.get(&runner_class) {
        let baseline_median = entry.median_ns as u128;
        let threshold = baseline_median * (100 + TOLERANCE_PCT) / 100;
        let outcome = if measured_median <= threshold {
            PerfOutcome::WithinTolerance
        } else {
            PerfOutcome::Regressed
        };
        return PerfReport {
            outcome,
            story_id: story_id.to_string(),
            runner_class,
            measured_ns: measured_median,
            baseline_ns: Some(baseline_median),
            samples: SAMPLES,
        };
    }

    // File exists but lacks current runner_class entry — insert + rewrite.
    payload.baselines.insert(
        runner_class.clone(),
        BaselineEntry {
            median_ns: measured_median as u64,
            created_at: current_iso8601_utc(),
        },
    );
    write_baseline_file(&full_path, story_id, payload.baselines)
        .expect("perf: failed to update baseline file with new runner_class");
    emit_warning(measured_median);
    PerfReport {
        outcome: PerfOutcome::BaselineWritten,
        story_id: story_id.to_string(),
        runner_class,
        measured_ns: measured_median,
        baseline_ns: None,
        samples: SAMPLES,
    }
}

/// Asserts the supplied closure's median runtime stays within ±20 % of the
/// committed baseline for the current `runner_class`.
///
/// Usage:
/// ```ignore
/// use orgsidian_core::test_support::perf::assert_no_perf_regression;
/// assert_no_perf_regression!(
///     "story-1.12-self-test-canary",
///     "tests/perf-baselines/story-1.12-self-test-canary.json",
///     || { /* op under test */ }
/// );
/// ```
///
/// On first run (missing baseline / missing `runner_class` entry) the macro
/// writes the baseline, emits a one-line `eprintln!` warning, and the test
/// passes. Subsequent runs compare against the committed median.
///
/// **The closure must be re-callable** (`Fn`/`FnMut`, not `FnOnce`): the
/// macro invokes it exactly 5 times in the same process. A `move ||
/// consume(owned_value)` closure fails to compile on the second iteration —
/// pass the value by reference instead.
///
/// **`baseline_path` semantics**: a *relative* path is resolved against the
/// workspace root, so production consumers always land in
/// `<repo>/tests/perf-baselines/`. An *absolute* path bypasses the
/// workspace-root join entirely (`Path::join(absolute)` discards the
/// receiver) — this is intentional for self-tests using `tempfile::tempdir()`.
#[macro_export]
macro_rules! assert_no_perf_regression {
    ($story_id:expr, $baseline_path:expr, $op:expr) => {{
        // Story 1.12 review P3: bind `$baseline_path` once so a
        // side-effecting expression (e.g. `compute_path()`) is not evaluated
        // twice (once for the impl call, once for the panic format string).
        let baseline_path = $baseline_path;
        let mut samples_ns: ::std::vec::Vec<u128> = ::std::vec::Vec::with_capacity(5);
        for _ in 0..5 {
            let start = ::std::time::Instant::now();
            $op();
            samples_ns.push(start.elapsed().as_nanos());
        }
        let report = $crate::test_support::perf::assert_no_perf_regression_impl(
            $story_id,
            ::std::path::Path::new(baseline_path),
            &samples_ns,
        );
        match report.outcome {
            $crate::test_support::perf::PerfOutcome::Regressed => {
                let measured = report.measured_ns;
                let baseline = report.baseline_ns.unwrap_or(0);
                let pct = if baseline > 0 {
                    (measured * 100 / baseline).saturating_sub(100)
                } else {
                    0
                };
                panic!(
                    "perf regression: {} on {}: measured median {} ns exceeds baseline {} ns by {}% (tolerance: 20%, samples: {})\nBaseline file: {}",
                    report.story_id,
                    report.runner_class,
                    measured,
                    baseline,
                    pct,
                    report.samples,
                    baseline_path
                );
            }
            $crate::test_support::perf::PerfOutcome::SchemaMismatch {
                field,
                on_disk,
                in_source,
            } => {
                panic!(
                    "perf baseline schema mismatch: {} for {} on disk reads {:?} but in-source gate is {:?}\nBaseline file: {}\nHand-edit the file to realign, or update the in-source constants and commit both together.",
                    field, report.story_id, on_disk, in_source, baseline_path
                );
            }
            _ => {}
        }
    }};
}

// The macro_export attribute above hoists the macro to the crate root by
// default; the submodule re-export below makes
// `orgsidian_core::test_support::perf::assert_no_perf_regression` work as
// well, matching the test-design.md §7.3.10 scaffold's literal import line.
// See Dev Notes §4.
#[doc(inline)]
pub use crate::assert_no_perf_regression;
