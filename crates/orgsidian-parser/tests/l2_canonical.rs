//! Story 2.7 — L2 canonical-AST concordance leg (LD-45).
//!
//! Pins Orgsidian's semantic layer to the committed canonical ASTs under
//! `tests/canonical_ast/` (script-generated from Emacs `org-element` via
//! `scripts/l2-oracle/`, human-reviewed). The nightly `l2-emacs-oracle`
//! job pins both pinned Emacs versions to the same canonical files, so
//! "both Emacs concordant against Orgsidian" (LD-45 triage case 1 —
//! PR-blocking) is decidable without ever diffing Orgsidian against Emacs
//! directly. This test rides inside `cargo test --workspace`, making case
//! 1 directly PR-blocking through the canonical proxy.
//!
//! The projection here is test-local over the public semantic API (no
//! `src/` involvement): schema "l2-projection-v1" — per headline `level`,
//! `todo`, `title`, `tags`, `scheduled`/`deadline`/`closed` (timestamp raw
//! source text), `children` (recursive). Elisp mirror:
//! `scripts/l2-oracle/projection.el`. Schema/regeneration/triage docs:
//! `docs/parser/l2-oracle.md`.
//!
//! Naming discipline: the test name must NOT contain `round_trip` — the
//! AC1/2.6 cargo filters (`round_trip_full`, `round_trip_subset`) stay
//! surgical.

use std::fs;
use std::path::PathBuf;

use orgsidian_parser::analyze;
use orgsidian_parser::semantic::Headline;
use serde_json::{json, Value};

/// The committed canonical ASTs (one `{stem}.json` per designated L2 seed
/// file; regenerate via `scripts/l2-oracle/generate-canonical.sh`).
fn canonical_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("canonical_ast")
}

/// The materialized corpus root that canonical `source` paths resolve
/// against (same `../..` hop as `tests/round_trip.rs`).
fn vault_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("vault-corpus")
}

/// git-LFS pointer-stub signature — same prefix check as
/// `tools/corpus-extractor/src/emit.rs::is_lfs_pointer`.
const LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";

/// Project one [`Headline`] onto schema l2-projection-v1. Key order matches
/// the elisp projection for reviewability; comparison is structural, so
/// order never affects the verdict.
fn project_headline(headline: &Headline) -> Value {
    json!({
        "level": headline.level,
        "todo": headline.todo_state.as_ref().map(|t| t.keyword.clone()),
        "title": headline.title,
        "tags": headline.tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
        "scheduled": headline.scheduled.as_ref().map(|t| t.raw.clone()),
        "deadline": headline.deadline.as_ref().map(|t| t.raw.clone()),
        "closed": headline.closed.as_ref().map(|t| t.raw.clone()),
        "children": headline.children.iter().map(project_headline).collect::<Vec<_>>(),
    })
}

/// First differing JSON path between two values, with both sides' values —
/// "headlines differ somewhere in 3KB of JSON" is useless diagnostics.
fn first_diff(ours: &Value, canonical: &Value, path: &str) -> Option<(String, Value, Value)> {
    match (ours, canonical) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let sub_path = format!("{path}.{key}");
                match (a.get(key), b.get(key)) {
                    (Some(av), Some(bv)) => {
                        if let Some(diff) = first_diff(av, bv, &sub_path) {
                            return Some(diff);
                        }
                    }
                    (Some(av), None) => {
                        return Some((sub_path, av.clone(), Value::Null));
                    }
                    (None, Some(bv)) => {
                        return Some((sub_path, Value::Null, bv.clone()));
                    }
                    (None, None) => unreachable!("key came from one of the maps"),
                }
            }
            None
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some((
                    format!("{path}.length"),
                    Value::from(a.len()),
                    Value::from(b.len()),
                ));
            }
            for (i, (av, bv)) in a.iter().zip(b.iter()).enumerate() {
                if let Some(diff) = first_diff(av, bv, &format!("{path}[{i}]")) {
                    return Some(diff);
                }
            }
            None
        }
        (a, b) => {
            if a == b {
                None
            } else {
                Some((path.to_string(), a.clone(), b.clone()))
            }
        }
    }
}

#[test]
fn l2_canonical_concordance() {
    let dir = canonical_dir();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "canonical AST dir {} unreadable: {e}\n\
                 regenerate via scripts/l2-oracle/generate-canonical.sh \
                 (docs/parser/l2-oracle.md)",
                dir.display()
            )
        })
        .map(|entry| entry.expect("canonical dir entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();

    // Anti-placebo: a wiped canonical dir must not pass vacuously. The seed
    // is 12-20 files by the AC3 selection rule; 10 is the tripwire floor.
    assert!(
        files.len() >= 10,
        "{}: only {} canonical AST file(s) — seed wiped or partially \
         deleted? regenerate via scripts/l2-oracle/generate-canonical.sh \
         (docs/parser/l2-oracle.md)",
        dir.display(),
        files.len()
    );

    let corpus_root = vault_corpus_dir();
    for file in &files {
        let label = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.display().to_string());
        let raw = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("{label}: cannot read {}: {e}", file.display()));
        let canonical: Value = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!(
                "{label}: malformed JSON: {e}\n\
                 regenerate via scripts/l2-oracle/generate-canonical.sh"
            )
        });

        let schema = canonical.get("schema").and_then(Value::as_str);
        assert_eq!(
            schema,
            Some("l2-projection-v1"),
            "{label}: unknown canonical schema {schema:?} — this test \
             implements l2-projection-v1 only (docs/parser/l2-oracle.md)"
        );

        let source = canonical
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                panic!(
                    "{label}: missing string `source` — regenerate via \
                     scripts/l2-oracle/generate-canonical.sh"
                )
            });

        // Source-exists + LFS-stub + readability guards (as in
        // round_trip_full): a canonical file pointing at nothing is a
        // broken gate, not a skipped one.
        let source_path = corpus_root.join(source);
        let bytes = fs::read(&source_path).unwrap_or_else(|e| {
            panic!(
                "{label}: cannot read seed source {}: {e}\n\
                 the canonical AST references a corpus file that must exist \
                 — stale seed after corpus regeneration? re-run \
                 scripts/l2-oracle/generate-canonical.sh and review \
                 (docs/parser/l2-oracle.md)",
                source_path.display()
            )
        });
        assert!(
            !bytes.starts_with(LFS_POINTER_PREFIX),
            "{label}: {} is a git-LFS pointer stub, not org content — \
             run `git lfs install && git lfs pull` to materialize the corpus",
            source_path.display()
        );
        let src = String::from_utf8(bytes).unwrap_or_else(|e| {
            panic!("{label}: {} is not valid UTF-8: {e}", source_path.display())
        });

        let doc = analyze(&src).unwrap_or_else(|e| panic!("{label}: analyze failed: {e}"));
        let ours = Value::Array(doc.headlines.iter().map(project_headline).collect());
        let expected = canonical.get("headlines").unwrap_or_else(|| {
            panic!(
                "{label}: missing `headlines` — regenerate via \
                 scripts/l2-oracle/generate-canonical.sh"
            )
        });

        if let Some((path, our_value, canonical_value)) = first_diff(&ours, expected, "headlines") {
            panic!(
                "{label}: Orgsidian diverges from the canonical AST \
                 (source {source}) at {path}\n  orgsidian: {our_value}\n  \
                 canonical: {canonical_value}\n\
                 LD-45 triage: if the nightly l2-emacs-oracle job agrees \
                 with the canonical value, this is an Orgsidian bug \
                 (PR-blocking); see docs/parser/l2-oracle.md"
            );
        }
    }
}
