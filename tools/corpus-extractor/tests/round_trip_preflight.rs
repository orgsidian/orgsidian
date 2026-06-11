//! Round-trip preflight (AC8, cheap insurance before Story 2.6 wires the PR
//! gate): every committed subset entry must already satisfy the parser's
//! byte-faithful identity `serialize_document(&analyze(content)) == content`.
//!
//! `orgsidian-parser` is a dev-dependency via path — legal because this crate
//! is OUTSIDE the workspace, so tree-sitter/cc/chrono land in the tool's own
//! lockfile only (AC8). Story 2.4 proved the identity for arbitrary strings
//! (proptest), so a red run here is decision-grade information about the
//! extractor's emission (I/O or encoding bugs), not about the parser.

use orgsidian_corpus_extractor::emit;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn every_subset_entry_round_trips_byte_faithfully() {
    let manifest = emit::load_subset_manifest(&repo_root().join("fixtures").join("subset-pr.json"))
        .expect("committed subset-pr.json loads");
    assert!(!manifest.entries.is_empty());

    let mut failures = Vec::new();
    for entry in &manifest.entries {
        let document = match orgsidian_parser::analyze(&entry.content) {
            Ok(doc) => doc,
            Err(err) => {
                failures.push(format!("{}: analyze failed: {err:#}", entry.id));
                continue;
            }
        };
        let serialized = orgsidian_parser::serialize_document(&document);
        if serialized != entry.content {
            let offset = serialized
                .bytes()
                .zip(entry.content.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| serialized.len().min(entry.content.len()));
            failures.push(format!(
                "{}: round-trip divergence at byte {offset} (in {} bytes)",
                entry.id,
                entry.content.len()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} subset entries failed the round-trip preflight:\n{}",
        failures.len(),
        manifest.entries.len(),
        failures.join("\n")
    );
}

/// Embedded content and the materialized vault-corpus twin must be the same
/// bytes (same emission pass — AC4/Dev Notes §6). Skips silently per file when
/// the twin is a git-LFS pointer stub (LFS-less checkout: the per-PR workflow
/// never needs the smudged files).
#[test]
fn materialized_twins_match_embedded_content_when_present() {
    let root = repo_root();
    let manifest = emit::load_subset_manifest(&root.join("fixtures").join("subset-pr.json"))
        .expect("committed subset-pr.json loads");
    let vault = root.join("tests").join("fixtures").join("vault-corpus");

    let mut compared = 0usize;
    for entry in manifest.entries.iter().take(20) {
        let path = vault.join(&entry.path);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("missing materialized twin {}: {e}", path.display()));
        if emit::is_lfs_pointer(&bytes) {
            continue; // LFS-less checkout — pointer stub, nothing to compare
        }
        assert_eq!(
            bytes,
            entry.content.as_bytes(),
            "{}: materialized twin diverges from embedded content",
            entry.id
        );
        compared += 1;
    }
    // No assertion on `compared` > 0: on an LFS-less checkout every twin is a
    // pointer and this test degrades to twin-existence checking by design.
    let _ = compared;
}
