//! Shared data model: LD-44 constructs, size/edge buckets, and the manifest
//! JSON schema. The manifest shapes are a **public contract** consumed by
//! Story 2.6's `round_trip_subset` PR gate (Dev Notes §6) — keep them flat,
//! boring, and serde-friendly. Documented in `docs/adr/0001-corpus-subset-selection.md`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Small bucket: files strictly under 1 KiB (LD-44 rule 2).
pub const SMALL_MAX_BYTES: usize = 1024;
/// Medium bucket: 1–50 KiB inclusive (LD-44 rule 2).
pub const MEDIUM_MAX_BYTES: usize = 50 * 1024;

/// Subset shape mandated by LD-44 rule 2: exactly 30 small / 50 medium / 20 large.
pub const SMALL_TARGET: usize = 30;
pub const MEDIUM_TARGET: usize = 50;
pub const LARGE_TARGET: usize = 20;

/// LD-44 rule 1: every construct appears at least 3 times across the subset.
pub const MIN_CONSTRUCT_OCCURRENCES: usize = 3;
/// LD-44 rule 3: at least 5 members per edge-case bucket.
pub const MIN_EDGE_OCCURRENCES: usize = 5;

/// The 15 LD-44 syntax constructs (architecture.md:1228-1245 — the same
/// enumeration as Story 2.3's AC6 table). The kebab-case serde ids are part
/// of the manifest contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Construct {
    /// Heading levels 1-6 with TODO states (default + `#+TODO:` custom).
    HeadingTodo,
    /// `SCHEDULED:` planning lines (active `<…>` + inactive `[…]`).
    Scheduled,
    /// `DEADLINE:` planning lines (active + inactive).
    Deadline,
    /// `CLOCK:` lines (open + closed + ranged).
    Clock,
    /// Repeater cookies on timestamps (`+1w`, `++1m`, `.+1y`, …).
    RecurringTimestamp,
    /// Drawers: `:PROPERTIES:`, `:LOGBOOK:`, custom types.
    Drawer,
    /// Inline markup: `*bold*` `/italic/` `=verbatim=` `~code~` `+strike+` `_underline_`.
    InlineMarkup,
    /// Links: `[[id:…]]`, `[[wiki]]`, `[[file://…]]`, plain `http(s)://…`.
    Link,
    /// Lists: `-` / `+` / numbered / checkbox items.
    List,
    /// Tables (simple + separator row + `#+TBLFM:`).
    Table,
    /// Block elements: `#+BEGIN_SRC` / `QUOTE` / `EXAMPLE` / `VERSE`.
    Block,
    /// Inline LaTeX: `$…$`, `\(…\)`, `\[…\]`.
    InlineLatex,
    /// Footnotes: `[fn:N]` definitions + `[fn::inline]` references.
    Footnote,
    /// Citations (org-cite): `[cite:@key]`.
    Citation,
    /// Headline tags: `:tag:`, `:tag1:tag2:`.
    Tag,
}

impl Construct {
    /// All 15 constructs in declaration (epic-bullet) order.
    pub const ALL: [Construct; 15] = [
        Construct::HeadingTodo,
        Construct::Scheduled,
        Construct::Deadline,
        Construct::Clock,
        Construct::RecurringTimestamp,
        Construct::Drawer,
        Construct::InlineMarkup,
        Construct::Link,
        Construct::List,
        Construct::Table,
        Construct::Block,
        Construct::InlineLatex,
        Construct::Footnote,
        Construct::Citation,
        Construct::Tag,
    ];
}

/// LD-44 rule 2 size buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeBucket {
    Small,
    Medium,
    Large,
}

impl SizeBucket {
    /// Bucket from a file's byte length: `<1KB` small, `1-50KB` medium, `>50KB` large.
    pub fn from_byte_len(len: usize) -> Self {
        if len < SMALL_MAX_BYTES {
            SizeBucket::Small
        } else if len <= MEDIUM_MAX_BYTES {
            SizeBucket::Medium
        } else {
            SizeBucket::Large
        }
    }
}

/// LD-44 rule 3 edge-case buckets. Edge files are members of the 100 and
/// count inside their size buckets (interpretation recorded in ADR 0001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeBucket {
    /// Content carrying Arabic / Hebrew / CJK codepoints.
    UnicodeRtl,
    /// CRLF or mixed line endings.
    UnusualEol,
    /// Malformed-but-valid org (over-indented drawers, trailing whitespace in headlines).
    MalformedValid,
}

impl EdgeBucket {
    pub const ALL: [EdgeBucket; 3] = [
        EdgeBucket::UnicodeRtl,
        EdgeBucket::UnusualEol,
        EdgeBucket::MalformedValid,
    ];
}

/// One harvested org-text assertion from `test-org-element.el`.
#[derive(Debug, Clone)]
pub struct Snippet {
    /// Stable human-meaningful id, e.g. `extracted/0042_headline-todo-keyword`.
    pub id: String,
    /// Provenance: the `ert-deftest` name the snippet was harvested from.
    pub deftest: String,
    /// Org text with elisp escapes decoded and `<point>` markers stripped.
    pub content: String,
    /// Detected LD-44 constructs.
    pub constructs: BTreeSet<Construct>,
}

/// How a subset member came to be (manifest contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Provenance {
    /// Harvested verbatim from an `ert-deftest` assertion.
    Extracted { deftest: String },
    /// Deterministically composed/transformed from harvested snippets.
    Synthesized { recipe: Recipe },
}

/// Deterministic synthesis recipe: replaying it over the same pinned source
/// yields byte-identical output (AC3 determinism requirement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    /// Applied transforms in order, e.g. `["compose"]`, `["compose", "crlf"]`.
    pub transforms: Vec<String>,
    /// Fixed seed feeding the composition RNG.
    pub seed: u64,
    /// Ids of the source snippets, in composition order.
    pub sources: Vec<String>,
}

/// Provenance header carried by both manifests (AC2): org-mode release tag,
/// upstream file SHA-256, and the extractor version that emitted them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestHeader {
    pub generator: String,
    pub extractor_version: String,
    pub org_release_tag: String,
    pub source_sha256: String,
}

/// One member of the L0 per-PR subset. `content` is embedded so the Story 2.6
/// PR gate works on an LFS-free checkout (AC4; rationale in ADR 0001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsetEntry {
    pub id: String,
    /// Relative path of the materialized twin under `tests/fixtures/vault-corpus/`.
    pub path: String,
    pub size_bucket: SizeBucket,
    pub byte_len: usize,
    pub constructs: Vec<Construct>,
    pub edge_buckets: Vec<EdgeBucket>,
    pub provenance: Provenance,
    /// Embedded org content (JSON escaping keeps CRLF/trailing-whitespace
    /// bytes immune to EOL mangling).
    pub content: String,
}

/// `fixtures/subset-pr.json` — self-contained (AC4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsetManifest {
    pub header: ManifestHeader,
    pub entries: Vec<SubsetEntry>,
}

/// One assertion in the full nightly corpus — a *pointer* (no embedded content).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullEntry {
    pub id: String,
    pub deftest: String,
    pub constructs: Vec<Construct>,
    /// Relative path under `tests/fixtures/vault-corpus/`.
    pub path: String,
    pub byte_len: usize,
}

/// `fixtures/full-nightly.json` — pointer manifest (AC4, architecture.md:995).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullManifest {
    pub header: ManifestHeader,
    pub entries: Vec<FullEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_bucket_boundaries_match_ld44() {
        assert_eq!(SizeBucket::from_byte_len(0), SizeBucket::Small);
        assert_eq!(SizeBucket::from_byte_len(1023), SizeBucket::Small);
        assert_eq!(SizeBucket::from_byte_len(1024), SizeBucket::Medium);
        assert_eq!(SizeBucket::from_byte_len(50 * 1024), SizeBucket::Medium);
        assert_eq!(SizeBucket::from_byte_len(50 * 1024 + 1), SizeBucket::Large);
    }

    #[test]
    fn construct_ids_are_kebab_case_contract() {
        let json = serde_json::to_string(&Construct::HeadingTodo).expect("serialize");
        assert_eq!(json, "\"heading-todo\"");
        let json = serde_json::to_string(&Construct::RecurringTimestamp).expect("serialize");
        assert_eq!(json, "\"recurring-timestamp\"");
        let back: Construct = serde_json::from_str("\"inline-latex\"").expect("deserialize");
        assert_eq!(back, Construct::InlineLatex);
    }

    #[test]
    fn buckets_sum_to_one_hundred() {
        assert_eq!(SMALL_TARGET + MEDIUM_TARGET + LARGE_TARGET, 100);
    }
}
