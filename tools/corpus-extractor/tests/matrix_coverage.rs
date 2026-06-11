//! TC-3 meta-test (AC8): the COMMITTED `fixtures/subset-pr.json` must satisfy
//! every LD-44 rule. This consumes the same `validate` module the `verify`
//! subcommand uses (one validator, two entry points) and runs against the
//! committed artifact — NOT an in-memory regeneration — so artifact rot
//! (hand-edits, stale pins, EOL mangling) is exactly what it catches.

use orgsidian_corpus_extractor::classify::Classifier;
use orgsidian_corpus_extractor::emit;
use orgsidian_corpus_extractor::model::{
    Construct, EdgeBucket, SizeBucket, LARGE_TARGET, MEDIUM_TARGET, MIN_CONSTRUCT_OCCURRENCES,
    MIN_EDGE_OCCURRENCES, SMALL_TARGET,
};
use orgsidian_corpus_extractor::validate;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    // tools/corpus-extractor/ -> repo root -> fixtures/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
}

#[test]
fn committed_subset_satisfies_ld44_matrix() {
    let path = fixtures_dir().join("subset-pr.json");
    let manifest = emit::load_subset_manifest(&path).expect("committed subset-pr.json loads");
    let classifier = Classifier::new().expect("classifier");

    // The shared validator is the gate…
    validate::validate_subset(&manifest, &classifier).expect("LD-44 matrix validation");

    // …and the headline numbers are re-asserted explicitly here (anti-placebo:
    // this test fails loudly even if the validator is ever gutted).
    assert_eq!(manifest.entries.len(), 100);
    let count = |b: SizeBucket| {
        manifest
            .entries
            .iter()
            .filter(|e| e.size_bucket == b)
            .count()
    };
    assert_eq!(count(SizeBucket::Small), SMALL_TARGET, "small bucket");
    assert_eq!(count(SizeBucket::Medium), MEDIUM_TARGET, "medium bucket");
    assert_eq!(count(SizeBucket::Large), LARGE_TARGET, "large bucket");

    for construct in Construct::ALL {
        let files = manifest
            .entries
            .iter()
            .filter(|e| e.constructs.contains(&construct))
            .count();
        assert!(
            files >= MIN_CONSTRUCT_OCCURRENCES,
            "construct {construct:?} appears in only {files} subset files (LD-44 requires >= {MIN_CONSTRUCT_OCCURRENCES})"
        );
    }
    for edge in EdgeBucket::ALL {
        let files = manifest
            .entries
            .iter()
            .filter(|e| e.edge_buckets.contains(&edge))
            .count();
        assert!(
            files >= MIN_EDGE_OCCURRENCES,
            "edge bucket {edge:?} has only {files} subset files (LD-44 requires >= {MIN_EDGE_OCCURRENCES})"
        );
    }
}

#[test]
fn committed_full_manifest_clears_assertion_floor() {
    let path = fixtures_dir().join("full-nightly.json");
    let manifest = emit::load_full_manifest(&path).expect("committed full-nightly.json loads");
    validate::validate_full(&manifest).expect("full-corpus validation");
    assert!(
        manifest.entries.len() >= validate::FULL_CORPUS_FLOOR,
        "full corpus regressed below the anti-placebo floor: {} < {}",
        manifest.entries.len(),
        validate::FULL_CORPUS_FLOOR
    );
}

#[test]
fn manifests_share_one_pin() {
    let subset =
        emit::load_subset_manifest(&fixtures_dir().join("subset-pr.json")).expect("subset loads");
    let full =
        emit::load_full_manifest(&fixtures_dir().join("full-nightly.json")).expect("full loads");
    assert_eq!(
        subset.header, full.header,
        "both manifests carry the same provenance header"
    );
}
