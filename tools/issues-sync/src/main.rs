//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use orgsidian_issues_sync::{run, SyncOpts};

#[derive(Parser, Debug)]
#[command(name = "orgsidian-issues-sync", version, about)]
struct Cli {
    #[arg(long)]
    owner: String,

    #[arg(long)]
    repo: String,

    #[arg(long, default_value = "_bmad-output/planning-artifacts/epics.md")]
    epics_path: PathBuf,

    #[arg(long)]
    project_node_id: String,

    #[arg(long, default_value = "main")]
    branch_for_links: String,

    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Render-only: emit the body for a single story (e.g. "1.1") and exit.
    /// Used by AC10 cell 6 byte-stability diff.
    #[arg(long)]
    render_only: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let opts = SyncOpts {
        owner: cli.owner,
        repo: cli.repo,
        epics_path: cli.epics_path,
        project_node_id: cli.project_node_id,
        branch_for_links: cli.branch_for_links,
        dry_run: cli.dry_run,
        render_only: cli.render_only,
    };
    let report = run(opts).await?;
    eprintln!("{report:#?}");
    Ok(())
}
