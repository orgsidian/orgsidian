//! `orgsidian` — Orgsidian's headless CLI binary.
//!
//! First command of the LD-27 command tree: `orgsidian parse <file>
//! [--json]` — the early public artifact for testing Orgsidian's org-mode
//! parsing fidelity before any GUI ships. This file is thin dispatch only:
//! clap definitions live in `cli.rs` (`include!`-shared with `build.rs` for
//! man-page generation), human-readable rendering in `render.rs`. The
//! parser is consumed through the `orgsidian-core` façade exclusively
//! (deny.toml LEAF graph rule).

mod cli;
mod render;

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser as _;

fn main() -> ExitCode {
    match cli::Cli::parse().command {
        cli::Command::Parse { file, json } => run_parse(&file, json),
    }
}

/// Run `orgsidian parse`: read the file, analyze it through the core
/// façade, print the AST to stdout (human-readable tree by default, pretty
/// camelCase JSON with `--json`).
///
/// Error posture: I/O failures (and the parser's defensive, in-practice
/// unreachable errors) print to stderr and exit non-zero — distinct from
/// clap's own usage-error exit 2. A failed stdout write also exits non-zero
/// (quietly for a closed pipe — a normal scripting event, never a panic).
/// Org *content* never fails: malformed constructs degrade leniently inside
/// the AST (LD-41).
fn run_parse(file: &Path, json: bool) -> ExitCode {
    let source = match std::fs::read_to_string(file) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let document = match orgsidian_core::parser::analyze(&source) {
        Ok(document) => document,
        Err(err) => {
            eprintln!("error: cannot analyze {}: {err}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let rendered = if json {
        match serde_json::to_string_pretty(&document) {
            Ok(rendered) => rendered,
            Err(err) => {
                eprintln!("error: cannot serialize the AST as JSON: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        render::render_document(&document)
    };
    // Review fix (Story 2.8): write explicitly instead of `println!` — the
    // macro panics on write failure, and a closed pipe (`orgsidian parse
    // f.org --json | head`) is a normal scripting event, not a bug (Rust
    // ignores SIGPIPE). Broken pipe exits non-zero quietly (the reader went
    // away by design); any other write failure is reported on stderr.
    let mut stdout = std::io::stdout().lock();
    match writeln!(stdout, "{rendered}").and_then(|()| stdout.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: cannot write to stdout: {err}");
            ExitCode::FAILURE
        }
    }
}
