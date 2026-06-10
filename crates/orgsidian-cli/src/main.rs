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
/// clap's own usage-error exit 2. Org *content* never fails: malformed
/// constructs degrade leniently inside the AST (LD-41).
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
    println!("{rendered}");
    ExitCode::SUCCESS
}
