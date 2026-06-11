// Clap derive definitions for the `orgsidian` CLI (Story 2.8, LD-27 command
// tree — first command: `parse`). Clap annotations are the PRIMARY CLI
// documentation (architecture CLI Documentation Strategy): `--help` is the
// user manual, and build.rs renders these same definitions into man pages.
//
// SELF-CONTAINED BY CONTRACT: this file may `use` only `clap` and `std`
// items — it is `include!`-shared with `build.rs` (the crate is not built
// yet at build-script time), so anything else here breaks the build script.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top-level `orgsidian` CLI arguments.
#[derive(Parser)]
#[command(
    name = "orgsidian",
    version,
    about = "Headless CLI for Orgsidian, the org-mode desktop app",
    long_about = "Headless CLI for Orgsidian (LD-27 command tree). The first \
                  public artifact: test Orgsidian's org-mode parsing fidelity \
                  on your own .org files before any GUI ships."
)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The LD-27 command tree (one command today; `index`, `query`,
/// `validate-plugin`, and `vault` slot in here in later epics).
#[derive(Subcommand)]
pub enum Command {
    /// Parse an org file and print its AST.
    #[command(long_about = "Parse an org file and print its semantic AST.\n\n\
        By default the AST is rendered as a human-readable headline tree \
        (best-effort presentation — the exact format is not a stability \
        contract). With --json the full semantic document is printed as \
        pretty JSON with camelCase keys; the JSON shape mirrors the current \
        semantic types and is NOT yet a schema-stability contract (the \
        stable wire surface arrives with the GUI IPC layer).")]
    Parse {
        /// Path to the `.org` file to parse.
        file: PathBuf,
        /// Print the AST as pretty JSON (camelCase keys) instead of the
        /// human-readable tree.
        #[arg(long)]
        json: bool,
    },
}
