//! LD-43 memory-soak drift math + RSS sampling (Story 4.9).
//!
//! The correctness-critical, unit-tested core of the soak gate lives here:
//!
//! - [`read_rss_bytes`] reads the process's own resident set. On Linux this is
//!   `/proc/self/statm` field 2 (resident pages) × page size, EXACTLY as LD-43
//!   prescribes. Off Linux (a developer's macOS box) it falls back to
//!   `getrusage(RUSAGE_SELF).ru_maxrss` so a local smoke run still executes —
//!   CI is Linux, where the real `/proc/self/statm` path is taken.
//! - [`compute_drift`] is a PURE function over the recorded [`Sample`]s: it
//!   picks the warmup-excluded baseline (first sample at/after `warmup_secs`,
//!   i.e. minute 60) and the window-end sample (last sample at/before
//!   `window_end_secs`, i.e. minute 720), then computes directional GROWTH
//!   `(end - baseline) / baseline` and flags it when it strictly exceeds the
//!   threshold. A memory DECREASE never fails; exactly-threshold passes.

use std::io;

/// LD-43 drift threshold: the soak fails if RSS grows by strictly MORE than
/// this fraction over the measured window. `> 10%` fails; exactly 10% passes.
pub const DRIFT_THRESHOLD: f64 = 0.10;

/// One RSS measurement at a point in the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Seconds elapsed since the session started (0 at the first sample).
    pub elapsed_secs: u64,
    /// Resident set size in bytes at that moment.
    pub rss_bytes: u64,
}

impl Sample {
    /// Construct a sample.
    pub fn new(elapsed_secs: u64, rss_bytes: u64) -> Self {
        Self {
            elapsed_secs,
            rss_bytes,
        }
    }
}

/// The verdict of a soak: which two samples were compared, the directional
/// growth ratio between them, and whether it tripped the threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftReport {
    /// The warmup-excluded baseline sample (minute 60 on the real 12h run).
    pub baseline: Sample,
    /// The window-end sample (minute 720 on the real 12h run).
    pub end: Sample,
    /// Directional growth `(end.rss - baseline.rss) / baseline.rss`. Negative
    /// when memory shrank.
    pub drift_ratio: f64,
    /// The threshold this was evaluated against.
    pub threshold: f64,
    /// `true` when `drift_ratio > threshold` — the fail condition.
    pub exceeded: bool,
}

/// Why a drift computation could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftError {
    /// No sample exists at or after the warmup mark — the run was too short to
    /// establish a post-warmup baseline.
    NoBaseline { warmup_secs: u64 },
    /// No end sample distinct from (and later than) the baseline was found
    /// within the window — nothing to measure drift against.
    NoWindowEnd { window_end_secs: u64 },
    /// The baseline RSS is zero, so a growth ratio is undefined.
    ZeroBaseline,
}

impl std::fmt::Display for DriftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftError::NoBaseline { warmup_secs } => write!(
                f,
                "no RSS sample at or after the warmup mark ({warmup_secs}s) — \
                 the session was too short to establish a post-warmup baseline"
            ),
            DriftError::NoWindowEnd { window_end_secs } => write!(
                f,
                "no RSS sample after the baseline within the window (end {window_end_secs}s) — \
                 nothing to measure drift against"
            ),
            DriftError::ZeroBaseline => {
                write!(f, "baseline RSS is zero — cannot compute a growth ratio")
            }
        }
    }
}

impl std::error::Error for DriftError {}

/// Compute LD-43 RSS drift from an ordered (ascending `elapsed_secs`) sample
/// series.
///
/// - **Baseline** (warmup excluded): the FIRST sample with
///   `elapsed_secs >= warmup_secs` — minute 60 on the real run.
/// - **End**: the LAST sample with `elapsed_secs <= window_end_secs` whose
///   `elapsed_secs` is strictly greater than the baseline's — minute 720.
/// - **Drift**: `(end.rss - baseline.rss) / baseline.rss`, a signed GROWTH
///   ratio. `exceeded` is `drift_ratio > threshold` (strict — exactly the
///   threshold passes; a shrink is always fine).
///
/// # Errors
///
/// [`DriftError`] when the series lacks a post-warmup baseline, a distinct
/// later end sample within the window, or has a zero baseline.
pub fn compute_drift(
    samples: &[Sample],
    warmup_secs: u64,
    window_end_secs: u64,
    threshold: f64,
) -> Result<DriftReport, DriftError> {
    let baseline = *samples
        .iter()
        .find(|s| s.elapsed_secs >= warmup_secs)
        .ok_or(DriftError::NoBaseline { warmup_secs })?;

    let end = *samples
        .iter()
        .rfind(|s| s.elapsed_secs <= window_end_secs && s.elapsed_secs > baseline.elapsed_secs)
        .ok_or(DriftError::NoWindowEnd { window_end_secs })?;

    if baseline.rss_bytes == 0 {
        return Err(DriftError::ZeroBaseline);
    }

    let drift_ratio =
        (end.rss_bytes as f64 - baseline.rss_bytes as f64) / baseline.rss_bytes as f64;

    Ok(DriftReport {
        baseline,
        end,
        drift_ratio,
        threshold,
        exceeded: drift_ratio > threshold,
    })
}

/// Read this process's current resident set size, in bytes.
///
/// Linux: `/proc/self/statm` field 2 (resident pages) × `sysconf(_SC_PAGESIZE)`
/// — the exact source LD-43 prescribes. Other targets: `getrusage(RUSAGE_SELF)`
/// `ru_maxrss` (a peak, not instantaneous — approximate, for local smoke only;
/// the units differ by platform, handled below).
///
/// # Errors
///
/// I/O or parse failure reading `/proc/self/statm` on Linux.
pub fn read_rss_bytes() -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        read_rss_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(read_rss_getrusage())
    }
}

#[cfg(target_os = "linux")]
fn read_rss_linux() -> io::Result<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm")?;
    // Fields: size resident shared text lib data dt (in pages). Field 2 is the
    // resident set.
    let resident_pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "/proc/self/statm too short"))?
        .parse()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("/proc/self/statm resident field not an integer: {e}"),
            )
        })?;
    // SAFETY: sysconf is a pure libc query with no memory effects.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page_size = if page_size > 0 {
        page_size as u64
    } else {
        4096
    };
    Ok(resident_pages * page_size)
}

#[cfg(not(target_os = "linux"))]
fn read_rss_getrusage() -> u64 {
    // SAFETY: getrusage fills the provided struct; RUSAGE_SELF is valid.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if rc != 0 {
        return 0;
    }
    // ru_maxrss units: bytes on macOS/Darwin, kilobytes on Linux (unused here).
    let maxrss = usage.ru_maxrss as u64;
    if cfg!(target_os = "macos") {
        maxrss
    } else {
        maxrss * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(elapsed_secs: u64, rss_bytes: u64) -> Sample {
        Sample::new(elapsed_secs, rss_bytes)
    }

    // Real-run-shaped series: samples every 30 min (1800s) from 0 to 720 min
    // (43200s). warmup 3600s (minute 60), window end 43200s (minute 720).
    #[test]
    fn growth_over_threshold_fails() {
        // baseline at 3600s = 100MB, end at 43200s = 120MB → +20% > 10%.
        let samples = vec![
            s(0, 90_000_000),
            s(1800, 95_000_000),
            s(3600, 100_000_000),
            s(43200, 120_000_000),
        ];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert_eq!(r.baseline, s(3600, 100_000_000));
        assert_eq!(r.end, s(43200, 120_000_000));
        assert!((r.drift_ratio - 0.20).abs() < 1e-9);
        assert!(r.exceeded);
    }

    #[test]
    fn growth_under_threshold_passes() {
        // baseline 100MB, end 105MB → +5% <= 10%.
        let samples = vec![s(3600, 100_000_000), s(43200, 105_000_000)];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert!((r.drift_ratio - 0.05).abs() < 1e-9);
        assert!(!r.exceeded);
    }

    #[test]
    fn exactly_threshold_passes() {
        // baseline 100MB, end 110MB → exactly +10%. Strict `>` means PASS.
        let samples = vec![s(3600, 100_000_000), s(43200, 110_000_000)];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert!((r.drift_ratio - 0.10).abs() < 1e-9);
        assert!(!r.exceeded, "exactly the threshold must pass (strict >)");
    }

    #[test]
    fn memory_decrease_passes() {
        let samples = vec![s(3600, 120_000_000), s(43200, 100_000_000)];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert!(r.drift_ratio < 0.0);
        assert!(!r.exceeded);
    }

    #[test]
    fn warmup_spike_is_excluded() {
        // A huge allocation during warmup (minute 0-30) must NOT become the
        // baseline: baseline is the minute-60 sample, so the post-warmup
        // plateau is what's measured. Warmup peak 500MB; baseline 100MB; end
        // 108MB → +8% passes, even though minute-0→end would look like -78%.
        let samples = vec![
            s(0, 500_000_000),
            s(1800, 200_000_000),
            s(3600, 100_000_000),
            s(43200, 108_000_000),
        ];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert_eq!(r.baseline, s(3600, 100_000_000));
        assert!((r.drift_ratio - 0.08).abs() < 1e-9);
        assert!(!r.exceeded);
    }

    #[test]
    fn baseline_is_first_sample_at_or_after_warmup() {
        // No sample exactly at 3600; the first at/after it (5000s) is baseline.
        let samples = vec![
            s(1000, 100_000_000),
            s(5000, 100_000_000),
            s(43200, 130_000_000),
        ];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert_eq!(r.baseline.elapsed_secs, 5000);
        assert!(r.exceeded); // +30%
    }

    #[test]
    fn end_respects_window_and_ignores_later_samples() {
        // A sample beyond the window (minute 750) must be ignored; end is the
        // last sample at/before the window end (minute 720 = 43200s).
        let samples = vec![
            s(3600, 100_000_000),
            s(43200, 105_000_000),
            s(45000, 200_000_000), // beyond window — ignored
        ];
        let r = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap();
        assert_eq!(r.end.elapsed_secs, 43200);
        assert!(!r.exceeded);
    }

    #[test]
    fn no_post_warmup_sample_errors() {
        let samples = vec![s(0, 100_000_000), s(1800, 100_000_000)];
        let err = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap_err();
        assert_eq!(err, DriftError::NoBaseline { warmup_secs: 3600 });
    }

    #[test]
    fn only_baseline_no_end_errors() {
        // A single post-warmup sample: it becomes the baseline, and there is no
        // strictly-later sample in the window to measure against.
        let samples = vec![s(0, 100_000_000), s(3600, 100_000_000)];
        let err = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap_err();
        assert_eq!(
            err,
            DriftError::NoWindowEnd {
                window_end_secs: 43200
            }
        );
    }

    #[test]
    fn zero_baseline_errors() {
        let samples = vec![s(3600, 0), s(43200, 100)];
        let err = compute_drift(&samples, 3600, 43200, DRIFT_THRESHOLD).unwrap_err();
        assert_eq!(err, DriftError::ZeroBaseline);
    }

    #[test]
    fn smoke_shaped_window_scales() {
        // Smoke params: total 180s, interval 6s, warmup 15s, window end 180s.
        // Baseline = first >= 15s (18s), end = last <= 180s. +4% passes.
        let mut samples = Vec::new();
        for i in 0..=30u64 {
            let elapsed = i * 6;
            let rss = if elapsed < 15 {
                50_000_000
            } else {
                100_000_000
            };
            samples.push(s(elapsed, rss));
        }
        // Bump the final sample to +4% over baseline.
        *samples.last_mut().unwrap() = s(180, 104_000_000);
        let r = compute_drift(&samples, 15, 180, DRIFT_THRESHOLD).unwrap();
        assert_eq!(r.baseline.elapsed_secs, 18);
        assert_eq!(r.end.elapsed_secs, 180);
        assert!(!r.exceeded);
    }

    #[test]
    fn read_rss_is_nonzero() {
        // On any supported target the process has a resident set > 0.
        let rss = read_rss_bytes().expect("read RSS");
        assert!(rss > 0, "expected a non-zero RSS, got {rss}");
    }
}
