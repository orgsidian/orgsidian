//! LD-44 subset selection (AC3): matrix-greedy small-bucket fill, then
//! deterministic medium/large composition with edge-bucket designations.
//!
//! Algorithm (documented in `docs/adr/0001-corpus-subset-selection.md`):
//! 1. **Matrix first** — greedy-fill construct coverage from harvested <1KB
//!    snippets, preferring multi-construct snippets for economy.
//! 2. **Buckets second** — harvested snippets are nearly all <1KB, so the 50
//!    medium and 20 large members are composed deterministically from the
//!    harvest pool (`synth::compose`), with per-composition seeded RNG.
//! 3. **Edge bucket third** — designated medium compositions receive
//!    mechanical transforms (CRLF / mixed-EOL / over-indent / trailing-ws /
//!    Unicode salting); edge files are members of the 100 and count inside
//!    their size buckets.
//!
//! Construct coverage (≥3 files per construct) is *guaranteed* by force-
//! including carrier snippets into three distinct compositions per construct;
//! the final `constructs`/`edge_buckets` recorded per member always come from
//! re-running the classifier/edge detectors over the final bytes, so the
//! emitted manifest is self-verifying (TC-3).

use crate::classify::{self, Classifier};
use crate::model::{
    Construct, EdgeBucket, Provenance, Recipe, SizeBucket, Snippet, LARGE_TARGET, MEDIUM_TARGET,
    MIN_CONSTRUCT_OCCURRENCES, SMALL_MAX_BYTES, SMALL_TARGET,
};
use crate::synth::{self, Rng, SYNTH_SEED, UNICODE_SALT_LINE};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};

/// A fully-formed subset member, ready for emission.
#[derive(Debug, Clone)]
pub struct Member {
    /// Human-meaningful stable id (doubles as the vault-corpus path stem).
    pub id: String,
    pub content: String,
    pub size_bucket: SizeBucket,
    pub edge_buckets: Vec<EdgeBucket>,
    pub constructs: BTreeSet<Construct>,
    pub provenance: Provenance,
}

/// Medium-bucket byte targets: deterministic spread 1.5KB → 30KB. Composition
/// stops right after crossing the target, so the band stays inside 1–50KB.
fn medium_target(i: usize) -> usize {
    1536 + i * (30 * 1024 - 1536) / (MEDIUM_TARGET - 1)
}

/// Large-bucket byte targets: deterministic spread 52KB → 96KB (>50KB floor
/// with headroom; keeps the committed corpus in the low-single-digit-MB zone).
fn large_target(i: usize) -> usize {
    52 * 1024 + i * (44 * 1024) / (LARGE_TARGET - 1)
}

/// Per-composition RNG seed: independent of composition order, fixed literal
/// base seed (determinism is a hard requirement, AC3).
fn comp_rng(index: usize) -> Rng {
    Rng::new(SYNTH_SEED.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// Select the full LD-44 subset: exactly 30 small + 50 medium + 20 large.
pub fn select_subset(snippets: &[Snippet], classifier: &Classifier) -> Result<Vec<Member>> {
    let smalls = select_smalls(snippets)?;
    let (mediums, larges) = compose_buckets(snippets, classifier)?;

    let mut members = Vec::with_capacity(100);
    members.extend(smalls.into_iter().map(|s| Member {
        id: s.id.clone(),
        size_bucket: SizeBucket::Small,
        edge_buckets: classify::detect_edges(&s.content),
        constructs: s.constructs.clone(),
        provenance: Provenance::Extracted {
            deftest: s.deftest.clone(),
        },
        content: s.content,
    }));
    members.extend(mediums);
    members.extend(larges);
    Ok(members)
}

/// Matrix-greedy small-bucket fill (exactly [`SMALL_TARGET`] members).
fn select_smalls(snippets: &[Snippet]) -> Result<Vec<Snippet>> {
    // Candidates: harvested, non-trivial, strictly <1KB (LD-44 small band).
    let candidates: Vec<&Snippet> = snippets
        .iter()
        .filter(|s| !s.content.trim().is_empty() && s.content.len() < SMALL_MAX_BYTES)
        .collect();
    if candidates.len() < SMALL_TARGET {
        bail!(
            "only {} small candidates harvested — cannot fill the {SMALL_TARGET}-file small bucket",
            candidates.len()
        );
    }

    let mut deficits: BTreeMap<Construct, usize> = Construct::ALL
        .iter()
        .map(|c| (*c, MIN_CONSTRUCT_OCCURRENCES))
        .collect();
    let mut selected: Vec<&Snippet> = Vec::with_capacity(SMALL_TARGET);
    let mut used: BTreeSet<&str> = BTreeSet::new();

    // Greedy matrix fill: repeatedly take the candidate covering the most
    // still-deficient constructs (tie-break: id order — deterministic).
    while selected.len() < SMALL_TARGET {
        let best = candidates
            .iter()
            .filter(|s| !used.contains(s.id.as_str()))
            .map(|s| {
                let gain = s
                    .constructs
                    .iter()
                    .filter(|c| deficits.get(c).copied().unwrap_or(0) > 0)
                    .count();
                (gain, *s)
            })
            .max_by(|(ga, sa), (gb, sb)| ga.cmp(gb).then(sb.id.cmp(&sa.id)));
        let Some((gain, snippet)) = best else { break };
        if gain == 0 {
            break; // matrix satisfied (or unreachable from smalls) — fill phase next
        }
        used.insert(snippet.id.as_str());
        for c in &snippet.constructs {
            if let Some(d) = deficits.get_mut(c) {
                *d = d.saturating_sub(1);
            }
        }
        selected.push(snippet);
    }

    // Fill remaining slots in id order (stable, deterministic).
    for s in &candidates {
        if selected.len() >= SMALL_TARGET {
            break;
        }
        if used.insert(s.id.as_str()) {
            selected.push(s);
        }
    }
    Ok(selected.into_iter().cloned().collect())
}

/// Edge-bucket designation for medium composition `i` (recorded in ADR 0001):
/// 0-2 CRLF, 3-4 mixed-EOL (≥5 unusual-EOL); 5-7 over-indent, 8-9 trailing-ws
/// (≥5 malformed-valid); 10-14 Unicode/RTL (≥5).
fn medium_transform(i: usize) -> &'static str {
    match i {
        0..=2 => "crlf",
        3..=4 => "mixed-eol",
        5..=7 => "overindent",
        8..=9 => "trailing-ws",
        10..=14 => "unicode",
        _ => "compose",
    }
}

fn compose_buckets(
    snippets: &[Snippet],
    classifier: &Classifier,
) -> Result<(Vec<Member>, Vec<Member>)> {
    let pool: Vec<Snippet> = snippets
        .iter()
        .filter(|s| !s.content.trim().is_empty())
        .cloned()
        .collect();
    if pool.is_empty() {
        bail!("empty harvest pool — nothing to compose from");
    }

    // Carrier snippets per construct (id order). Coverage guarantee: each
    // construct is force-included in 3 distinct medium compositions, cycling
    // over its carriers when fewer than 3 exist.
    let mut forced: BTreeMap<usize, Vec<&Snippet>> = BTreeMap::new();
    for (ci, construct) in Construct::ALL.iter().enumerate() {
        let carriers: Vec<&Snippet> = pool
            .iter()
            .filter(|s| s.constructs.contains(construct))
            .collect();
        if carriers.is_empty() {
            bail!(
                "no harvested snippet carries construct {construct:?} — the LD-44 matrix cannot be filled from this pin"
            );
        }
        for j in 0..MIN_CONSTRUCT_OCCURRENCES {
            let comp = 5 + ci * MIN_CONSTRUCT_OCCURRENCES + j; // comps 5..=49
            forced
                .entry(comp)
                .or_default()
                .push(carriers[j % carriers.len()]);
        }
    }
    // Over-indent comps (5-7) must contain drawer lines for the transform to
    // bite; force a drawer carrier into each.
    let drawer_carrier = pool
        .iter()
        .find(|s| s.constructs.contains(&Construct::Drawer));
    if let Some(carrier) = drawer_carrier {
        for comp in 5..=7 {
            forced.entry(comp).or_default().push(carrier);
        }
    }
    // Unicode comps (10-14): harvest first — force RTL/CJK carriers when the
    // upstream suite provides them; the salt line covers any remainder.
    let unicode_carriers: Vec<&Snippet> = pool
        .iter()
        .filter(|s| classify::has_unicode_rtl(&s.content))
        .collect();

    let mut mediums = Vec::with_capacity(MEDIUM_TARGET);
    for i in 0..MEDIUM_TARGET {
        let label = medium_transform(i);
        let mut force: Vec<&Snippet> = forced.remove(&i).unwrap_or_default();
        let mut salted = false;
        if label == "unicode" {
            if unicode_carriers.is_empty() {
                salted = true;
            } else {
                force.push(unicode_carriers[(i - 10) % unicode_carriers.len()]);
            }
        }

        let mut rng = comp_rng(i);
        let (mut content, mut sources) = synth::compose(&pool, &force, medium_target(i), &mut rng);
        let mut transforms = vec!["compose".to_string()];
        if salted {
            content.push_str(UNICODE_SALT_LINE);
            transforms.push("unicode-salt".to_string());
            sources.push("salt:unicode".to_string());
        }
        match label {
            "crlf" => {
                content = synth::to_crlf(&content);
                transforms.push("crlf".to_string());
            }
            "mixed-eol" => {
                content = synth::to_mixed_eol(&content);
                transforms.push("mixed-eol".to_string());
            }
            "overindent" => {
                content = synth::overindent_drawers(&content);
                transforms.push("overindent".to_string());
            }
            "trailing-ws" => {
                content = synth::trailing_ws_headlines(&content);
                transforms.push("trailing-ws".to_string());
            }
            _ => {}
        }

        let bucket = SizeBucket::from_byte_len(content.len());
        if bucket != SizeBucket::Medium {
            bail!(
                "medium composition {i} landed at {} bytes ({bucket:?}) — target spread needs retuning",
                content.len()
            );
        }
        mediums.push(Member {
            id: format!("synthesized/{:03}_medium-{label}", i + 1),
            size_bucket: bucket,
            edge_buckets: classify::detect_edges(&content),
            constructs: classifier.classify(&content),
            provenance: Provenance::Synthesized {
                recipe: Recipe {
                    transforms,
                    seed: SYNTH_SEED,
                    sources,
                },
            },
            content,
        });
    }

    let mut larges = Vec::with_capacity(LARGE_TARGET);
    for i in 0..LARGE_TARGET {
        let mut rng = comp_rng(MEDIUM_TARGET + i);
        let (content, sources) = synth::compose(&pool, &[], large_target(i), &mut rng);
        let bucket = SizeBucket::from_byte_len(content.len());
        if bucket != SizeBucket::Large {
            bail!(
                "large composition {i} landed at {} bytes ({bucket:?}) — target spread needs retuning",
                content.len()
            );
        }
        larges.push(Member {
            id: format!("synthesized/{:03}_large-compose", MEDIUM_TARGET + i + 1),
            size_bucket: bucket,
            edge_buckets: classify::detect_edges(&content),
            constructs: classifier.classify(&content),
            provenance: Provenance::Synthesized {
                recipe: Recipe {
                    transforms: vec!["compose".to_string()],
                    seed: SYNTH_SEED,
                    sources,
                },
            },
            content,
        });
    }

    Ok((mediums, larges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::Classifier;
    use crate::model::EdgeBucket;

    /// Build a synthetic harvest pool that carries every LD-44 construct, so
    /// selection invariants are testable without the real upstream file.
    fn pool() -> Vec<Snippet> {
        let classifier = Classifier::new().expect("classifier");
        let samples: Vec<String> = vec![
            "* TODO Task one\n".to_string(),
            "* NEXT Task two\n".to_string(),
            "* DONE Task three\n".to_string(),
            "* H\nSCHEDULED: <2026-06-10 Wed>\n".to_string(),
            "* H\nDEADLINE: <2026-06-12 Fri>\n".to_string(),
            "* H\n:LOGBOOK:\nCLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:00] =>  1:00\n:END:\n".to_string(),
            "* H\nSCHEDULED: <2026-06-10 Wed +1w>\n".to_string(),
            "* H\n:PROPERTIES:\n:ID: abc\n:END:\n".to_string(),
            "Some *bold* and /italic/ text.\n".to_string(),
            "See [[id:abc][the note]] and http://example.com\n".to_string(),
            "- item one\n- [ ] open\n1. numbered\n".to_string(),
            "| a | b |\n|---+---|\n| 1 | 2 |\n".to_string(),
            "#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC\n".to_string(),
            "Inline $x^2$ math and \\(a+b\\).\n".to_string(),
            "Text[fn:1].\n\n[fn:1] A footnote.\n".to_string(),
            "Claim [cite:@key2026].\n".to_string(),
            "* Tagged :work:urgent:\n".to_string(),
            "Plain filler paragraph with nothing special.\n".to_string(),
        ];
        samples
            .into_iter()
            .enumerate()
            .map(|(i, content)| Snippet {
                id: format!("extracted/{i:04}_sample"),
                deftest: format!("test-org-element/sample-{i}"),
                constructs: classifier.classify(&content),
                content,
            })
            .collect()
    }

    /// Inflate the pool so the small bucket can fill 30 distinct members.
    fn big_pool() -> Vec<Snippet> {
        let mut out = pool();
        for k in 0..40 {
            out.push(Snippet {
                id: format!("extracted/9{k:03}_filler"),
                deftest: format!("test-org-element/filler-{k}"),
                content: format!("Filler paragraph number {k}.\n"),
                constructs: BTreeSet::new(),
            });
        }
        out
    }

    #[test]
    fn subset_has_exact_bucket_counts() {
        let classifier = Classifier::new().expect("classifier");
        let members = select_subset(&big_pool(), &classifier).expect("select");
        assert_eq!(members.len(), 100);
        let count = |b: SizeBucket| members.iter().filter(|m| m.size_bucket == b).count();
        assert_eq!(count(SizeBucket::Small), SMALL_TARGET);
        assert_eq!(count(SizeBucket::Medium), MEDIUM_TARGET);
        assert_eq!(count(SizeBucket::Large), LARGE_TARGET);
    }

    #[test]
    fn subset_satisfies_construct_matrix() {
        let classifier = Classifier::new().expect("classifier");
        let members = select_subset(&big_pool(), &classifier).expect("select");
        for construct in Construct::ALL {
            let files = members
                .iter()
                .filter(|m| m.constructs.contains(&construct))
                .count();
            assert!(
                files >= MIN_CONSTRUCT_OCCURRENCES,
                "{construct:?}: only {files} subset files"
            );
        }
    }

    #[test]
    fn subset_satisfies_edge_minimums() {
        let classifier = Classifier::new().expect("classifier");
        let members = select_subset(&big_pool(), &classifier).expect("select");
        for edge in EdgeBucket::ALL {
            let files = members
                .iter()
                .filter(|m| m.edge_buckets.contains(&edge))
                .count();
            assert!(files >= 5, "{edge:?}: only {files} subset files");
        }
    }

    #[test]
    fn selection_is_deterministic() {
        let classifier = Classifier::new().expect("classifier");
        let a = select_subset(&big_pool(), &classifier).expect("select");
        let b = select_subset(&big_pool(), &classifier).expect("select");
        let flat = |ms: &[Member]| -> Vec<(String, String)> {
            ms.iter()
                .map(|m| (m.id.clone(), m.content.clone()))
                .collect()
        };
        assert_eq!(flat(&a), flat(&b));
    }

    #[test]
    fn missing_construct_fails_loudly() {
        let classifier = Classifier::new().expect("classifier");
        // A pool with no citation anywhere cannot fill the matrix.
        let pool: Vec<Snippet> = big_pool()
            .into_iter()
            .filter(|s| !s.constructs.contains(&Construct::Citation))
            .collect();
        let err = select_subset(&pool, &classifier).expect_err("must fail");
        assert!(format!("{err:#}").contains("Citation"), "{err:#}");
    }

    #[test]
    fn synthesized_members_record_recipes() {
        let classifier = Classifier::new().expect("classifier");
        let members = select_subset(&big_pool(), &classifier).expect("select");
        for m in members.iter().filter(|m| m.id.starts_with("synthesized/")) {
            match &m.provenance {
                Provenance::Synthesized { recipe } => {
                    assert!(!recipe.sources.is_empty(), "{}: empty sources", m.id);
                    assert_eq!(recipe.transforms[0], "compose");
                    assert_eq!(recipe.seed, SYNTH_SEED);
                }
                other => panic!("{}: expected synthesized provenance, got {other:?}", m.id),
            }
        }
    }
}
