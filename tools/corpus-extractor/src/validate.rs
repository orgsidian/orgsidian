//! LD-44 matrix validator (AC8, TC-3). ONE validator, TWO entry points: the
//! `verify` subcommand and `tests/matrix_coverage.rs` both call into here, so
//! the committed `fixtures/subset-pr.json` is what gets checked — artifact rot
//! (hand-edits, stale regeneration, EOL mangling) is exactly what this catches.

use crate::classify::{self, Classifier};
use crate::emit;
use crate::fetch;
use crate::model::{
    Construct, EdgeBucket, FullManifest, SizeBucket, SubsetManifest, LARGE_TARGET, MEDIUM_TARGET,
    MIN_CONSTRUCT_OCCURRENCES, MIN_EDGE_OCCURRENCES, SMALL_TARGET,
};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// Anti-placebo floor for the full nightly corpus (AC3/AC8): 75% of the
/// harvest observed from the pinned `test-org-element.el` (the epic's "~2000"
/// is a target, not a contract — rationale in ADR 0001; observed harvest at
/// release_9.8.5 is 569 assertions). A floor of 0 would be a placebo; this
/// trips if a regression ever guts the scanner or the pin.
pub const FULL_CORPUS_FLOOR: usize = 425;

/// Validate the self-contained subset manifest against every LD-44 rule.
/// Re-classifies the EMBEDDED content (not the stored metadata), so stored
/// constructs/edge-buckets/byte-lengths are cross-checked too.
pub fn validate_subset(manifest: &SubsetManifest, classifier: &Classifier) -> Result<()> {
    validate_header(
        &manifest.header.org_release_tag,
        &manifest.header.source_sha256,
    )?;

    let dupes = emit::duplicate_ids(manifest.entries.iter().map(|e| e.id.as_str()));
    if !dupes.is_empty() {
        bail!("duplicate subset ids: {dupes:?}");
    }

    let mut bucket_counts: BTreeMap<SizeBucket, usize> = BTreeMap::new();
    let mut construct_files: BTreeMap<Construct, usize> = BTreeMap::new();
    let mut edge_files: BTreeMap<EdgeBucket, usize> = BTreeMap::new();

    for entry in &manifest.entries {
        if entry.byte_len != entry.content.len() {
            bail!(
                "{}: stored byte_len {} != embedded content length {}",
                entry.id,
                entry.byte_len,
                entry.content.len()
            );
        }
        let bucket = SizeBucket::from_byte_len(entry.content.len());
        if bucket != entry.size_bucket {
            bail!(
                "{}: stored size bucket {:?} but {} bytes is {:?}",
                entry.id,
                entry.size_bucket,
                entry.content.len(),
                bucket
            );
        }
        let detected: Vec<Construct> = classifier.classify(&entry.content).into_iter().collect();
        if detected != entry.constructs {
            bail!(
                "{}: stored constructs {:?} do not match re-classified {:?} (artifact rot?)",
                entry.id,
                entry.constructs,
                detected
            );
        }
        let edges = classify::detect_edges(&entry.content);
        if edges != entry.edge_buckets {
            bail!(
                "{}: stored edge buckets {:?} do not match re-detected {:?}",
                entry.id,
                entry.edge_buckets,
                edges
            );
        }
        validate_twin_path(&entry.id, &entry.path)?;

        *bucket_counts.entry(bucket).or_default() += 1;
        for c in &entry.constructs {
            *construct_files.entry(*c).or_default() += 1;
        }
        for e in &entry.edge_buckets {
            *edge_files.entry(*e).or_default() += 1;
        }
    }

    // Aggregate gates run AFTER per-entry integrity, so a tampered entry is
    // reported precisely before any count mismatch noise.
    if manifest.entries.len() != SMALL_TARGET + MEDIUM_TARGET + LARGE_TARGET {
        bail!(
            "subset has {} entries, expected exactly {}",
            manifest.entries.len(),
            SMALL_TARGET + MEDIUM_TARGET + LARGE_TARGET
        );
    }
    for (bucket, expected) in [
        (SizeBucket::Small, SMALL_TARGET),
        (SizeBucket::Medium, MEDIUM_TARGET),
        (SizeBucket::Large, LARGE_TARGET),
    ] {
        let got = bucket_counts.get(&bucket).copied().unwrap_or(0);
        if got != expected {
            bail!("size bucket {bucket:?}: {got} files, LD-44 requires exactly {expected}");
        }
    }
    for construct in Construct::ALL {
        let got = construct_files.get(&construct).copied().unwrap_or(0);
        if got < MIN_CONSTRUCT_OCCURRENCES {
            bail!(
                "construct {construct:?}: appears in {got} subset files, LD-44 requires >= {MIN_CONSTRUCT_OCCURRENCES}"
            );
        }
    }
    for edge in EdgeBucket::ALL {
        let got = edge_files.get(&edge).copied().unwrap_or(0);
        if got < MIN_EDGE_OCCURRENCES {
            bail!(
                "edge bucket {edge:?}: {got} subset files, LD-44 requires >= {MIN_EDGE_OCCURRENCES}"
            );
        }
    }
    Ok(())
}

/// Validate the pointer manifest: floor, unique ids, id-consistent paths.
pub fn validate_full(manifest: &FullManifest) -> Result<()> {
    validate_header(
        &manifest.header.org_release_tag,
        &manifest.header.source_sha256,
    )?;
    if manifest.entries.len() < FULL_CORPUS_FLOOR {
        bail!(
            "full corpus has {} assertions, below the anti-placebo floor of {FULL_CORPUS_FLOOR}",
            manifest.entries.len()
        );
    }
    let dupes = emit::duplicate_ids(manifest.entries.iter().map(|e| e.id.as_str()));
    if !dupes.is_empty() {
        bail!("duplicate full-corpus ids: {dupes:?}");
    }
    for entry in &manifest.entries {
        validate_twin_path(&entry.id, &entry.path)?;
        if entry.deftest.is_empty() {
            bail!("{}: empty deftest provenance", entry.id);
        }
    }
    Ok(())
}

/// Manifest paths are joined under `tests/fixtures/vault-corpus/` by `verify`
/// and the preflight twin check — reject traversal/absolute paths, and pin the
/// path to its id-derived form (`<id>.org`, the emission invariant), so a
/// tampered manifest cannot silently point the twin checks elsewhere.
fn validate_twin_path(id: &str, path: &str) -> Result<()> {
    if !path.ends_with(".org") || path.starts_with('/') || path.contains("..") {
        bail!("{id}: malformed corpus path {path:?}");
    }
    if path != format!("{id}.org") {
        bail!("{id}: corpus path {path:?} does not match the id-derived twin path \"{id}.org\"");
    }
    Ok(())
}

/// Manifests must carry the same pin the extractor source carries — a manifest
/// regenerated against a different pin without a source bump (or vice versa)
/// fails here.
fn validate_header(tag: &str, sha256: &str) -> Result<()> {
    if tag != fetch::ORG_RELEASE_TAG {
        bail!(
            "manifest pinned to org-mode tag {tag:?} but the extractor pin is {:?} — regenerate (fetch + extract)",
            fetch::ORG_RELEASE_TAG
        );
    }
    if sha256 != fetch::SOURCE_SHA256 {
        bail!(
            "manifest source SHA-256 {sha256} does not match the extractor pin {} — regenerate (fetch + extract)",
            fetch::SOURCE_SHA256
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ManifestHeader, Provenance, SubsetEntry};

    fn header() -> ManifestHeader {
        ManifestHeader {
            generator: "orgsidian-corpus-extractor".to_string(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
            org_release_tag: fetch::ORG_RELEASE_TAG.to_string(),
            source_sha256: fetch::SOURCE_SHA256.to_string(),
        }
    }

    fn entry(id: &str, content: &str, classifier: &Classifier) -> SubsetEntry {
        SubsetEntry {
            id: id.to_string(),
            path: format!("{id}.org"),
            size_bucket: SizeBucket::from_byte_len(content.len()),
            byte_len: content.len(),
            constructs: classifier.classify(content).into_iter().collect(),
            edge_buckets: classify::detect_edges(content),
            provenance: Provenance::Extracted {
                deftest: "test-org-element/x".to_string(),
            },
            content: content.to_string(),
        }
    }

    #[test]
    fn rejects_wrong_entry_count() {
        let classifier = Classifier::new().expect("classifier");
        let manifest = SubsetManifest {
            header: header(),
            entries: vec![entry("extracted/0000_a", "* H\n", &classifier)],
        };
        let err = validate_subset(&manifest, &classifier).expect_err("must fail");
        assert!(
            format!("{err:#}").contains("expected exactly 100"),
            "{err:#}"
        );
    }

    #[test]
    fn rejects_tampered_content() {
        let classifier = Classifier::new().expect("classifier");
        let mut e = entry("extracted/0000_a", "* TODO H\n", &classifier);
        // Simulate artifact rot: content edited after generation.
        e.content = "* H\n".to_string();
        e.byte_len = e.content.len();
        let manifest = SubsetManifest {
            header: header(),
            entries: vec![e],
        };
        let err = validate_subset(&manifest, &classifier).expect_err("must fail");
        assert!(format!("{err:#}").contains("re-classified"), "{err:#}");
    }

    #[test]
    fn rejects_byte_len_mismatch() {
        let classifier = Classifier::new().expect("classifier");
        let mut e = entry("extracted/0000_a", "* H\n", &classifier);
        e.byte_len += 1;
        let manifest = SubsetManifest {
            header: header(),
            entries: vec![e],
        };
        let err = validate_subset(&manifest, &classifier).expect_err("must fail");
        assert!(format!("{err:#}").contains("byte_len"), "{err:#}");
    }

    #[test]
    fn rejects_stale_pin_header() {
        let classifier = Classifier::new().expect("classifier");
        let mut h = header();
        h.org_release_tag = "release_0.0.0".to_string();
        let manifest = SubsetManifest {
            header: h,
            entries: vec![],
        };
        let err = validate_subset(&manifest, &classifier).expect_err("must fail");
        assert!(format!("{err:#}").contains("regenerate"), "{err:#}");
    }

    #[test]
    fn full_floor_is_not_a_placebo() {
        // 75% of the 569-assertion harvest observed at release_9.8.5; a floor
        // below ~400 would stop tripping on a gutted scanner.
        const { assert!(FULL_CORPUS_FLOOR >= 400, "floor must stay decision-grade") };
        let manifest = FullManifest {
            header: header(),
            entries: vec![],
        };
        let err = validate_full(&manifest).expect_err("must fail");
        assert!(format!("{err:#}").contains("floor"), "{err:#}");
    }

    #[test]
    fn rejects_twin_path_id_mismatch() {
        // Subset side: a path pointing at a different (well-formed) twin.
        let classifier = Classifier::new().expect("classifier");
        let mut e = entry("extracted/0000_a", "* H\n", &classifier);
        e.path = "extracted/0001_b.org".to_string();
        let manifest = SubsetManifest {
            header: header(),
            entries: vec![e],
        };
        let err = validate_subset(&manifest, &classifier).expect_err("must fail");
        assert!(format!("{err:#}").contains("id-derived"), "{err:#}");

        // Full side: same tamper, same shared check.
        let mut manifest = FullManifest {
            header: header(),
            entries: (0..FULL_CORPUS_FLOOR)
                .map(|i| crate::model::FullEntry {
                    id: format!("extracted/{i:04}_x"),
                    deftest: "test-org-element/x".to_string(),
                    constructs: vec![],
                    path: format!("extracted/{i:04}_x.org"),
                    byte_len: 4,
                })
                .collect(),
        };
        manifest.entries[1].path = "extracted/0000_x.org".to_string();
        let err = validate_full(&manifest).expect_err("must fail");
        assert!(format!("{err:#}").contains("id-derived"), "{err:#}");
    }

    #[test]
    fn full_rejects_malformed_paths() {
        let classifier = Classifier::new().expect("classifier");
        let _ = classifier; // full validation does not need the classifier
        let mut manifest = FullManifest {
            header: header(),
            entries: (0..FULL_CORPUS_FLOOR)
                .map(|i| crate::model::FullEntry {
                    id: format!("extracted/{i:04}_x"),
                    deftest: "test-org-element/x".to_string(),
                    constructs: vec![],
                    path: format!("extracted/{i:04}_x.org"),
                    byte_len: 4,
                })
                .collect(),
        };
        validate_full(&manifest).expect("well-formed manifest passes");
        manifest.entries[0].path = "../escape.org".to_string();
        let err = validate_full(&manifest).expect_err("must fail");
        assert!(
            format!("{err:#}").contains("malformed corpus path"),
            "{err:#}"
        );
    }
}
