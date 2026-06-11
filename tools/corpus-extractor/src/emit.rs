//! Deterministic emission (AC4): `fixtures/subset-pr.json` (self-contained),
//! `fixtures/full-nightly.json` (pointer manifest), and the materialized
//! corpus tree under `tests/fixtures/vault-corpus/`.
//!
//! Determinism contract (AC3): same pin + same extractor code ⇒ byte-identical
//! outputs. All iteration is over sorted/`BTreeMap` collections, JSON is
//! pretty-printed by serde_json (stable), and no timestamps exist beyond the
//! pin header.

use crate::classify::Classifier;
use crate::elisp;
use crate::fetch;
use crate::model::{FullEntry, FullManifest, ManifestHeader, Snippet, SubsetEntry, SubsetManifest};
use crate::select;
use crate::validate;
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Git-LFS pointer-file signature (AC6 graceful degradation).
const LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";

/// True when `bytes` are a git-LFS pointer stub rather than real content.
pub fn is_lfs_pointer(bytes: &[u8]) -> bool {
    bytes.starts_with(LFS_POINTER_PREFIX)
}

/// Output roots — parameterized so the determinism double-run can target a
/// scratch directory instead of the repo.
#[derive(Debug, Clone)]
pub struct OutputPaths {
    /// Root `fixtures/` directory (manifests).
    pub fixtures_dir: PathBuf,
    /// `tests/fixtures/vault-corpus/` (materialized corpus).
    pub vault_dir: PathBuf,
}

impl OutputPaths {
    pub fn for_repo_root(repo_root: &Path) -> Self {
        Self {
            fixtures_dir: repo_root.join("fixtures"),
            vault_dir: repo_root
                .join("tests")
                .join("fixtures")
                .join("vault-corpus"),
        }
    }

    pub fn subset_manifest(&self) -> PathBuf {
        self.fixtures_dir.join("subset-pr.json")
    }

    pub fn full_manifest(&self) -> PathBuf {
        self.fixtures_dir.join("full-nightly.json")
    }
}

/// Counters reported to the CLI / Completion Notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractSummary {
    pub harvested: usize,
    pub subset_members: usize,
    pub corpus_files_written: usize,
    pub vault_bytes: u64,
    pub subset_json_bytes: u64,
    pub full_json_bytes: u64,
}

/// The whole `extract` pipeline over an already-verified upstream source:
/// harvest → classify → select → validate → materialize + emit manifests.
pub fn run_extract(source: &str, out: &OutputPaths) -> Result<ExtractSummary> {
    let classifier = Classifier::new()?;
    let snippets = harvest_snippets(source, &classifier);
    let members = select::select_subset(&snippets, &classifier)?;

    let header = ManifestHeader {
        generator: "orgsidian-corpus-extractor".to_string(),
        extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        org_release_tag: fetch::ORG_RELEASE_TAG.to_string(),
        source_sha256: fetch::SOURCE_SHA256.to_string(),
    };

    let subset = SubsetManifest {
        header: header.clone(),
        entries: members
            .iter()
            .map(|m| SubsetEntry {
                id: m.id.clone(),
                path: format!("{}.org", m.id),
                size_bucket: m.size_bucket,
                byte_len: m.content.len(),
                constructs: m.constructs.iter().copied().collect(),
                edge_buckets: m.edge_buckets.clone(),
                provenance: m.provenance.clone(),
                content: m.content.clone(),
            })
            .collect(),
    };
    let full = FullManifest {
        header,
        entries: snippets
            .iter()
            .map(|s| FullEntry {
                id: s.id.clone(),
                deftest: s.deftest.clone(),
                constructs: s.constructs.iter().copied().collect(),
                path: format!("{}.org", s.id),
                byte_len: s.content.len(),
            })
            .collect(),
    };

    // Validate BEFORE writing anything — an invalid subset never lands on disk
    // (same validator the `verify` subcommand and TC-3 meta-test run).
    let classifier = Classifier::new()?;
    validate::validate_subset(&subset, &classifier)?;
    validate::validate_full(&full)?;

    // Materialize the corpus tree. Generated subtrees are wiped first so
    // regeneration never leaves stale members behind (README.md etc. at the
    // vault root are preserved).
    let mut corpus_files = 0usize;
    let mut vault_bytes = 0u64;
    for sub in ["extracted", "synthesized"] {
        let dir = out.vault_dir.join(sub);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).with_context(|| format!("clearing {}", dir.display()))?;
        }
    }
    for s in &snippets {
        vault_bytes += write_corpus_file(&out.vault_dir, &format!("{}.org", s.id), &s.content)?;
        corpus_files += 1;
    }
    for m in members.iter().filter(|m| m.id.starts_with("synthesized/")) {
        vault_bytes += write_corpus_file(&out.vault_dir, &format!("{}.org", m.id), &m.content)?;
        corpus_files += 1;
    }

    let subset_json_bytes = write_json(&out.subset_manifest(), &subset)?;
    let full_json_bytes = write_json(&out.full_manifest(), &full)?;

    Ok(ExtractSummary {
        harvested: snippets.len(),
        subset_members: members.len(),
        corpus_files_written: corpus_files,
        vault_bytes,
        subset_json_bytes,
        full_json_bytes,
    })
}

/// Harvest + id assignment + classification, in deterministic file order.
pub fn harvest_snippets(source: &str, classifier: &Classifier) -> Vec<Snippet> {
    elisp::harvest(source)
        .into_iter()
        .enumerate()
        .map(|(seq, h)| Snippet {
            id: snippet_id(seq, &h.deftest, h.index),
            constructs: classifier.classify(&h.content),
            deftest: h.deftest,
            content: h.content,
        })
        .collect()
}

/// Stable human-meaningful snippet id, e.g. `extracted/0042_headline-parser`
/// (with `-NN` suffix for repeat assertions within one deftest). The id is
/// also the vault-corpus path stem — Story 2.6 uses it as its failure label.
fn snippet_id(seq: usize, deftest: &str, index: usize) -> String {
    let short = deftest.strip_prefix("test-org-element/").unwrap_or(deftest);
    let slug: String = short
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if index == 0 {
        format!("extracted/{seq:04}_{slug}")
    } else {
        format!("extracted/{seq:04}_{slug}-{index:02}")
    }
}

/// Write one corpus file with exact bytes (no EOL translation), creating
/// parent dirs. Returns the byte count written.
fn write_corpus_file(vault_dir: &Path, rel_path: &str, content: &str) -> Result<u64> {
    let path = vault_dir.join(rel_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, content.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(content.len() as u64)
}

/// Deterministic pretty JSON + trailing newline. Returns bytes written.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<u64> {
    let mut json = serde_json::to_string_pretty(value).context("serializing manifest")?;
    json.push('\n');
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, json.as_bytes()).with_context(|| format!("writing {}", path.display()))?;
    Ok(json.len() as u64)
}

/// Load the committed subset manifest from `fixtures/subset-pr.json`.
pub fn load_subset_manifest(path: &Path) -> Result<SubsetManifest> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Load the committed full manifest from `fixtures/full-nightly.json`.
pub fn load_full_manifest(path: &Path) -> Result<FullManifest> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Ids must be unique across a manifest (helper shared with validate).
pub fn duplicate_ids<'a, I: Iterator<Item = &'a str>>(ids: I) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut dupes = Vec::new();
    for id in ids {
        if !seen.insert(id) {
            dupes.push(id.to_string());
        }
    }
    dupes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfs_pointer_detection() {
        assert!(is_lfs_pointer(
            b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 42\n"
        ));
        assert!(!is_lfs_pointer(b"* A real org headline\n"));
        assert!(!is_lfs_pointer(b""));
    }

    #[test]
    fn snippet_ids_are_stable_and_slugged() {
        assert_eq!(
            snippet_id(42, "test-org-element/headline-parser", 0),
            "extracted/0042_headline-parser"
        );
        assert_eq!(
            snippet_id(7, "test-org-element/timestamp/diary", 2),
            "extracted/0007_timestamp-diary-02"
        );
        assert_eq!(snippet_id(0, "other-name", 0), "extracted/0000_other-name");
    }

    #[test]
    fn duplicate_id_detection() {
        assert!(duplicate_ids(["a", "b"].into_iter()).is_empty());
        assert_eq!(duplicate_ids(["a", "b", "a"].into_iter()), vec!["a"]);
    }
}
