//! LD-43 nightly memory-soak harness (Story 4.9).
//!
//! Runs the LD-43 scripted session against the headless `orgsidian-core` in
//! ONE long-lived process — 200 buffer open/close cycles, 50 plugin re-init
//! cycles, 1000 agenda queries — sampling `/proc/self/statm` RSS every 30
//! minutes, then fails (non-zero exit) if RSS grew by more than 10% from the
//! warmup-excluded baseline (minute 60) to the window end (minute 720).
//!
//! The 12h defaults reproduce the AC exactly. The nightly workflow passes
//! small values on `workflow_dispatch` so a manual run validates wiring in
//! minutes instead of blocking the merge-gating nightly for 12 hours.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use orgsidian_memory_soak::{compute_drift, read_rss_bytes, Sample, DRIFT_THRESHOLD};

mod vault;

/// LD-43 memory soak harness.
#[derive(Parser, Debug)]
#[command(
    name = "orgsidian-memory-soak",
    about = "LD-43 nightly memory soak: scripted session + RSS-drift gate (Story 4.9)"
)]
struct Args {
    /// Total wall-clock duration of the soak, in seconds (12h = 43200).
    #[arg(long, default_value_t = 43200)]
    total_seconds: u64,
    /// RSS sample cadence, in seconds (LD-43: every 30 min = 1800).
    #[arg(long, default_value_t = 1800)]
    sample_interval_seconds: u64,
    /// Warmup to exclude before the drift baseline, in seconds (minute 60 =
    /// 3600). The baseline is the first sample at or after this mark.
    #[arg(long, default_value_t = 3600)]
    warmup_seconds: u64,
    /// Drift-window end, in seconds (minute 720 = 43200). The end sample is the
    /// last one at or before this mark.
    #[arg(long, default_value_t = 43200)]
    window_end_seconds: u64,
    /// Buffer open/close cycles over the whole session (LD-43: 200).
    #[arg(long, default_value_t = 200)]
    buffer_cycles: usize,
    /// Plugin re-init cycles over the whole session (LD-43: 50).
    #[arg(long, default_value_t = 50)]
    plugin_reinit_cycles: usize,
    /// Agenda queries over the whole session (LD-43: 1000).
    #[arg(long, default_value_t = 1000)]
    agenda_queries: usize,
    /// Number of synthetic `.org` files in the throwaway vault.
    #[arg(long, default_value_t = 40)]
    vault_files: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(report) => {
            let pct = report.drift_ratio * 100.0;
            let thr = report.threshold * 100.0;
            println!(
                "memory-soak: baseline={} bytes @ {}s, end={} bytes @ {}s, drift={pct:+.2}% (threshold {thr:.1}%)",
                report.baseline.rss_bytes,
                report.baseline.elapsed_secs,
                report.end.rss_bytes,
                report.end.elapsed_secs,
            );
            if report.exceeded {
                eprintln!(
                    "::error::memory-soak FAILED — RSS drift {pct:+.2}% exceeds the {thr:.1}% LD-43 threshold"
                );
                ExitCode::FAILURE
            } else {
                println!("memory-soak: PASS — RSS drift within the LD-43 threshold.");
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("::error::memory-soak harness error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<orgsidian_memory_soak::DriftReport> {
    let total_secs = args.total_seconds;
    let interval = args.sample_interval_seconds.max(1);
    let warmup_secs = args.warmup_seconds;
    let window_end_secs = args.window_end_seconds;

    // Hermetic env: pin the index store and the XDG dirs into tempdirs BEFORE
    // any core call, so nothing touches the runner's real config/data dirs.
    // The TempDirs are kept alive for the whole run via `_guards`.
    let data_dir = tempfile::tempdir().context("create temp data dir")?;
    let config_dir = tempfile::tempdir().context("create temp config dir")?;
    let xdg_data_dir = tempfile::tempdir().context("create temp xdg data dir")?;
    std::env::set_var("ORGSIDIAN_DATA_DIR", data_dir.path());
    std::env::set_var("XDG_CONFIG_HOME", config_dir.path());
    std::env::set_var("XDG_DATA_HOME", xdg_data_dir.path());

    // Throwaway vault of varied org files.
    let vault_dir = tempfile::tempdir().context("create temp vault dir")?;
    vault::synthesize(vault_dir.path(), args.vault_files).context("synthesize vault")?;
    let vault_path = vault_dir.path().to_path_buf();

    println!(
        "memory-soak: starting — total={total_secs}s, interval={interval}s, warmup={warmup_secs}s, \
         window_end={window_end_secs}s, workloads=(buffers {}, reinit {}, queries {}), vault_files={}",
        args.buffer_cycles, args.plugin_reinit_cycles, args.agenda_queries, args.vault_files
    );

    let rt = tokio::runtime::Runtime::new().context("build tokio runtime")?;
    rt.block_on(async move {
        soak(
            args,
            &vault_path,
            total_secs,
            interval,
            warmup_secs,
            window_end_secs,
        )
        .await
    })
}

/// The scripted session: create the index once, then over `num_intervals`
/// ticks perform a proportional slice of each workload, sleeping to the sample
/// cadence and recording RSS at each tick boundary. Returns the drift verdict.
async fn soak(
    args: &Args,
    vault_path: &Path,
    total_secs: u64,
    interval: u64,
    warmup_secs: u64,
    window_end_secs: u64,
) -> Result<orgsidian_memory_soak::DriftReport> {
    let file_paths = org_files(vault_path)?;
    let cancel = AtomicBool::new(false);

    // Initial index build (setup — not one of the 50 re-init cycles).
    orgsidian_core::rebuild_index(vault_path, &cancel, |_| {})
        .await
        .map_err(|e| anyhow::anyhow!("initial index build failed: {e}"))?;

    let num_intervals = (total_secs / interval).max(1);
    let mut samples: Vec<Sample> = Vec::with_capacity(num_intervals as usize + 1);

    let start = Instant::now();
    samples.push(sample_now(&start)?);
    println!(
        "memory-soak: sample @ {}s = {} bytes",
        samples[0].elapsed_secs, samples[0].rss_bytes
    );

    for tick in 1..=num_intervals {
        // Proportional per-tick share so cumulative work hits the totals by the
        // last tick (evenly spread across the window).
        let buffers = span(args.buffer_cycles, tick, num_intervals);
        let reinits = span(args.plugin_reinit_cycles, tick, num_intervals);
        let queries = span(args.agenda_queries, tick, num_intervals);

        for b in 0..buffers {
            if !file_paths.is_empty() {
                let path = &file_paths[(tick as usize + b) % file_paths.len()];
                open_close_buffer(path)?;
            }
        }
        for _ in 0..reinits {
            orgsidian_core::rebuild_index(vault_path, &cancel, |_| {})
                .await
                .map_err(|e| anyhow::anyhow!("plugin re-init (rebuild_index) failed: {e}"))?;
        }
        for _ in 0..queries {
            orgsidian_core::index_stats(vault_path)
                .await
                .map_err(|e| anyhow::anyhow!("agenda query (index_stats) failed: {e}"))?;
        }

        // Sleep so this tick's sample lands on the wall-clock cadence.
        let target = Duration::from_secs(tick * interval);
        let elapsed = start.elapsed();
        if target > elapsed {
            tokio::time::sleep(target - elapsed).await;
        }

        let sample = sample_now(&start)?;
        println!(
            "memory-soak: sample @ {}s = {} bytes (tick {tick}/{num_intervals}, buffers+{buffers} reinit+{reinits} queries+{queries})",
            sample.elapsed_secs, sample.rss_bytes
        );
        samples.push(sample);
    }

    compute_drift(&samples, warmup_secs, window_end_secs, DRIFT_THRESHOLD)
        .context("compute RSS drift from the sampled series")
}

/// Buffer open/close: read the file and run the parser (`parser::analyze`),
/// then drop the document — the headless equivalent of opening a buffer into
/// the editor and closing it.
fn open_close_buffer(path: &Path) -> Result<()> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let document = orgsidian_core::parser::analyze(&source)
        .map_err(|e| anyhow::anyhow!("parse {} failed: {e}", path.display()))?;
    // Explicit drop documents the open→close intent (RSS is what we watch).
    drop(document);
    Ok(())
}

/// Take an RSS sample stamped with elapsed seconds since `start`.
fn sample_now(start: &Instant) -> Result<Sample> {
    let rss = read_rss_bytes().context("read RSS from /proc/self/statm")?;
    Ok(Sample::new(start.elapsed().as_secs(), rss))
}

/// The cumulative-distribution slice of `total` assigned to `tick` of
/// `num_intervals`, so the shares sum to exactly `total`.
fn span(total: usize, tick: u64, num_intervals: u64) -> usize {
    let up_to = total as u64 * tick / num_intervals;
    let before = total as u64 * (tick - 1) / num_intervals;
    (up_to - before) as usize
}

/// Collect the `.org` files in the synthesized vault.
fn org_files(vault_path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(vault_path).context("read vault dir")? {
        let path = entry.context("read vault entry")?.path();
        if path.extension().is_some_and(|ext| ext == "org") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}
