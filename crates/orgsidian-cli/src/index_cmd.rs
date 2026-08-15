//! `orgsidian index {init|rebuild|stats|integrity}` handlers (Story 3.7).
//!
//! Thin dispatch + rendering only — mirrors `run_parse` in `main.rs`. The index
//! is reached EXCLUSIVELY through the `orgsidian-core` façade (deny.toml LEAF
//! graph rule); no `orgsidian-index`/`-vault` edge exists here. Each handler
//! builds a Tokio runtime, `block_on`s the relevant async core function, and
//! renders the result to stdout — human text by default, or exactly one
//! `serde_json` object under `--json` (no progress noise). Errors always print
//! to stderr and exit non-zero (distinct from clap's usage-error exit 2);
//! `integrity` additionally exits non-zero on any failing check.

use std::io::Write as _;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use orgsidian_core::{IndexStats, IntegrityReport, OrgError, ScanOutcome, ScanProgress};

use crate::cli::{IndexAction, IndexArgs};

/// Dispatch an `orgsidian index` subcommand to its handler.
pub(crate) fn run(action: IndexAction) -> ExitCode {
    match action {
        IndexAction::Init(args) => run_init(&args),
        IndexAction::Rebuild(args) => run_rebuild(&args),
        IndexAction::Stats(args) => run_stats(&args),
        IndexAction::Integrity(args) => run_integrity(&args),
    }
}

/// `index init`: designate the vault (create/open + stamp the index) and run a
/// full incremental scan, then print the scan summary.
fn run_init(args: &IndexArgs) -> ExitCode {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(code) => return code,
    };
    let cancel = AtomicBool::new(false);
    let result: Result<ScanOutcome, OrgError> = rt.block_on(async {
        let handle = orgsidian_core::designate_vault(&args.vault).await?;
        let outcome =
            orgsidian_core::scan_vault(&handle, &cancel, progress_printer(args.json)).await;
        // Shut the index down cleanly before surfacing the outcome, so no
        // connection outlives the handle regardless of success or failure.
        handle.shutdown().await;
        outcome
    });
    match result {
        Ok(outcome) => emit_scan(args, &outcome, "init"),
        Err(err) => fail(&err),
    }
}

/// `index rebuild`: drop the DB and re-scan from scratch, then print the
/// summary.
fn run_rebuild(args: &IndexArgs) -> ExitCode {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(code) => return code,
    };
    let cancel = AtomicBool::new(false);
    let result = rt.block_on(orgsidian_core::rebuild_index(
        &args.vault,
        &cancel,
        progress_printer(args.json),
    ));
    match result {
        Ok(outcome) => emit_scan(args, &outcome, "rebuild"),
        Err(err) => fail(&err),
    }
}

/// `index stats`: print the read-only aggregate counts + schema/last-indexed.
fn run_stats(args: &IndexArgs) -> ExitCode {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(code) => return code,
    };
    match rt.block_on(orgsidian_core::index_stats(&args.vault)) {
        Ok(stats) if args.json => print_json(serde_json::to_string_pretty(&stats)),
        Ok(stats) => print_line(&render_stats(&stats)),
        Err(err) => fail(&err),
    }
}

/// `index integrity`: run the consistency checks and exit non-zero on any
/// failure (regardless of `--json`).
fn run_integrity(args: &IndexArgs) -> ExitCode {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(code) => return code,
    };
    match rt.block_on(orgsidian_core::index_integrity(&args.vault)) {
        Ok(report) => {
            let rendered = if args.json {
                print_json(serde_json::to_string_pretty(&report))
            } else {
                print_line(&render_integrity(&report))
            };
            // A failing check is a non-zero exit even when rendering succeeded —
            // the scriptable CI gate. A render failure is already non-zero.
            if report.ok {
                rendered
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => fail(&err),
    }
}

/// Build a Tokio runtime for `block_on` (the writer task + reader pool + the
/// scan's `spawn_blocking` all need one). A build failure is a non-zero exit
/// with a stderr diagnostic.
fn runtime() -> Result<tokio::runtime::Runtime, ExitCode> {
    tokio::runtime::Runtime::new().map_err(|err| {
        eprintln!("error: cannot start the async runtime: {err}");
        ExitCode::FAILURE
    })
}

/// A scan progress callback: prints one checkpoint line to stdout in human
/// mode, and is a no-op under `--json` so stdout stays exactly one object.
/// Progress write failures are ignored (a closed pipe on progress is not a
/// command failure; the final summary write reports any real stdout error).
fn progress_printer(json: bool) -> impl FnMut(ScanProgress) {
    move |progress: ScanProgress| {
        if json {
            return;
        }
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(
            stdout,
            "  indexed {} of {} ({} quarantined)",
            progress.current, progress.total, progress.errors
        );
    }
}

/// Render + emit a completed scan's summary (`init`/`rebuild`), as human text
/// or one JSON object.
fn emit_scan(args: &IndexArgs, outcome: &ScanOutcome, verb: &str) -> ExitCode {
    if args.json {
        return print_json(serde_json::to_string_pretty(&serde_json::json!({
            "indexed": outcome.indexed,
            "skipped": outcome.skipped,
            "errors": outcome.errors,
            "cancelled": outcome.cancelled,
        })));
    }
    let summary = format!(
        "{verb} complete: {} indexed, {} skipped, {} quarantined{}",
        outcome.indexed,
        outcome.skipped,
        outcome.errors,
        if outcome.cancelled {
            " (cancelled)"
        } else {
            ""
        }
    );
    print_line(&summary)
}

/// Human-readable `index stats` table.
fn render_stats(stats: &IndexStats) -> String {
    let last = stats.last_indexed_at.as_deref().unwrap_or("never");
    format!(
        "files:        {}\n\
         quarantined:  {}\n\
         headlines:    {}\n\
         tags:         {}\n\
         links:        {}\n\
         fts docs:     {}\n\
         schema:       v{} (applied {})\n\
         last indexed: {}",
        stats.file_count,
        stats.quarantined_count,
        stats.headline_count,
        stats.tag_count,
        stats.link_count,
        stats.fts_doc_count,
        stats.schema_version,
        stats.schema_applied_at,
        last,
    )
}

/// Human-readable `index integrity` report: one line per check, then a verdict.
fn render_integrity(report: &IntegrityReport) -> String {
    let mut lines = Vec::with_capacity(report.checks.len() + 2);
    for check in &report.checks {
        let status = if check.ok { "OK  " } else { "FAIL" };
        match &check.detail {
            Some(detail) if !check.ok => {
                lines.push(format!("{status} {}: {detail}", check.name));
            }
            _ => lines.push(format!("{status} {}", check.name)),
        }
    }
    lines.push(String::new());
    lines.push(
        if report.ok {
            "integrity: OK"
        } else {
            "integrity: FAILED"
        }
        .to_string(),
    );
    lines.join("\n")
}

/// Write an already-serialized JSON payload to stdout as one object. Takes the
/// `serde_json::to_string_pretty` result (computed at the call site, where the
/// concrete `Serialize` type is in scope) so this crate needs no direct `serde`
/// dependency.
fn print_json(rendered: serde_json::Result<String>) -> ExitCode {
    match rendered {
        Ok(text) => print_line(&text),
        Err(err) => {
            eprintln!("error: cannot serialize the output as JSON: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Write one line to stdout with the `run_parse` broken-pipe posture: a closed
/// reader is a normal scripting event (non-zero, quiet); any other write error
/// is reported on stderr.
fn print_line(text: &str) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match writeln!(stdout, "{text}").and_then(|()| stdout.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: cannot write to stdout: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Report a core failure on stderr and exit non-zero (never exit 2 — that is
/// clap's usage-error code).
fn fail(err: &OrgError) -> ExitCode {
    eprintln!("error: {err}");
    ExitCode::FAILURE
}
