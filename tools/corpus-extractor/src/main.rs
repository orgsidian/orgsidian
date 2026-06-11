//! CLI shell for the corpus extractor (Story 2.5). All logic lives in the
//! library (`orgsidian_corpus_extractor`); this binary is the ONLY layer that
//! prints (AC9: `println!` stays out of lib logic).

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use orgsidian_corpus_extractor::{classify::Classifier, emit, fetch, validate};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "orgsidian-corpus-extractor",
    about = "Extracts the LD-44 round-trip corpus from org-mode's test-org-element.el (Story 2.5)",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Download the pinned test-org-element.el into the gitignored cache,
    /// verifying its SHA-256 (maintainer operation — the only networked step).
    Fetch,
    /// Cached .el -> fixtures/{subset-pr,full-nightly}.json + materialized
    /// corpus under tests/fixtures/vault-corpus/ (no network).
    Extract,
    /// Run the LD-44 matrix validator against the COMMITTED manifests (same
    /// code path the matrix_coverage meta-test calls).
    Verify,
}

fn main() -> Result<()> {
    // The tool always operates on its own checkout: tool root is baked in at
    // compile time (maintainer tool, run via --manifest-path per CONTRIBUTING §3).
    let tool_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = tool_root
        .ancestors()
        .nth(2)
        .context("resolving repo root from tool root")?
        .to_path_buf();

    match Cli::parse().cmd {
        Cmd::Fetch => run_fetch(&tool_root),
        Cmd::Extract => run_extract(&tool_root, &repo_root),
        Cmd::Verify => run_verify(&repo_root),
    }
}

fn run_fetch(tool_root: &Path) -> Result<()> {
    println!(
        "fetching test-org-element.el @ {} (pinned SHA-256 {}…)",
        fetch::ORG_RELEASE_TAG,
        &fetch::SOURCE_SHA256[..12]
    );
    let path = fetch::fetch_to_cache(tool_root)?;
    println!("cached + checksum-verified: {}", path.display());
    Ok(())
}

fn run_extract(tool_root: &Path, repo_root: &Path) -> Result<()> {
    let source = fetch::read_cached(tool_root)?;
    let out = emit::OutputPaths::for_repo_root(repo_root);
    let summary = emit::run_extract(&source, &out)?;
    println!("harvested assertions : {}", summary.harvested);
    println!("subset members       : {}", summary.subset_members);
    println!("corpus files written : {}", summary.corpus_files_written);
    println!("vault-corpus bytes   : {}", summary.vault_bytes);
    println!(
        "subset-pr.json bytes : {} ({})",
        summary.subset_json_bytes,
        out.subset_manifest().display()
    );
    println!(
        "full-nightly.json    : {} ({})",
        summary.full_json_bytes,
        out.full_manifest().display()
    );
    Ok(())
}

fn run_verify(repo_root: &Path) -> Result<()> {
    let out = emit::OutputPaths::for_repo_root(repo_root);
    let classifier = Classifier::new()?;

    let subset = emit::load_subset_manifest(&out.subset_manifest())?;
    validate::validate_subset(&subset, &classifier)?;
    println!(
        "subset-pr.json OK: {} entries, LD-44 matrix satisfied",
        subset.entries.len()
    );

    let full = emit::load_full_manifest(&out.full_manifest())?;
    validate::validate_full(&full)?;
    println!(
        "full-nightly.json OK: {} assertions (floor {})",
        full.entries.len(),
        validate::FULL_CORPUS_FLOOR
    );

    // Spot-check: embedded subset content must byte-match its materialized
    // twin under vault-corpus. Needs real corpus bytes — a git-LFS pointer
    // stub here is an actionable setup problem, not a parse error (AC6).
    let vault_root = repo_root
        .join("tests")
        .join("fixtures")
        .join("vault-corpus");
    let mut checked = 0usize;
    for entry in subset.entries.iter().take(10) {
        let path = vault_root.join(&entry.path);
        let bytes = std::fs::read(&path)
            .with_context(|| format!("reading materialized twin {}", path.display()))?;
        if emit::is_lfs_pointer(&bytes) {
            bail!(
                "{} is a git-LFS pointer stub, not corpus bytes.\nRun: git lfs install && git lfs pull\n(The per-PR workflow does NOT need this — the subset is embedded in fixtures/subset-pr.json; only nightly/L2 work and corpus regeneration read vault-corpus.)",
                path.display()
            );
        }
        if bytes != entry.content.as_bytes() {
            bail!(
                "{}: materialized twin diverges from the embedded content (regenerate via extract)",
                entry.id
            );
        }
        checked += 1;
    }
    println!("twin spot-check OK: {checked} files byte-identical to embedded content");
    Ok(())
}
