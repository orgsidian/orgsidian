// Clap derive definitions for the `orgsidian` CLI (Story 2.8, LD-27 command
// tree — first command: `parse`). Clap annotations are the PRIMARY CLI
// documentation (architecture CLI Documentation Strategy): `--help` is the
// user manual, and build.rs renders these same definitions into man pages.
//
// SELF-CONTAINED BY CONTRACT: this file may `use` only `clap` and `std`
// items — it is `include!`-shared with `build.rs` (the crate is not built
// yet at build-script time), so anything else here breaks the build script.

use clap::{Args, Parser, Subcommand};
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
    /// Create, rebuild, inspect, or verify a vault's derived SQLite index.
    #[command(long_about = "Manage a vault's derived SQLite index (LD-49 \
        rebuild-index, LD-27 CLI-as-integration-surface).\n\n\
        `init` creates and populates the index (incremental on re-run); \
        `rebuild` drops it and re-scans from scratch; `stats` prints counts, \
        schema version, and the last-indexed timestamp; `integrity` runs the \
        SQLite and FTS5 consistency checks and exits non-zero on any failure. \
        The index lives outside the vault, under the OS data dir (or the \
        ORGSIDIAN_DATA_DIR override). Each subcommand takes a <vault> path and \
        an optional --json flag that emits a single JSON object with no \
        progress noise.")]
    Index {
        /// The index operation to run.
        #[command(subcommand)]
        action: IndexAction,
    },
}

/// The `orgsidian index` operations (Story 3.7). Each carries the shared
/// [`IndexArgs`] (`<vault>` positional + `--json`).
#[derive(Subcommand)]
pub enum IndexAction {
    /// Create + populate the index for a vault (incremental on re-run).
    Init(IndexArgs),
    /// Drop the index and fully rebuild it from scratch.
    Rebuild(IndexArgs),
    /// Print index statistics: counts, schema version, last-indexed time.
    Stats(IndexArgs),
    /// Verify index integrity (SQLite + FTS5 consistency checks).
    Integrity(IndexArgs),
}

/// Shared arguments for every `orgsidian index` subcommand.
#[derive(Args)]
pub struct IndexArgs {
    /// Path to the vault root directory to index.
    pub vault: PathBuf,
    /// Emit a single JSON object to stdout instead of human-readable text
    /// (no progress lines).
    #[arg(long)]
    pub json: bool,
}
