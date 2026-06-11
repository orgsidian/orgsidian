# ADR 0001 — Corpus subset selection (LD-44) and upstream acquisition

- **Status:** Accepted
- **Date:** 2026-06-11
- **Story:** 2.5 (`tools/corpus-extractor` + fixture governance)
- **Traces:** LD-44, LD-32, LD-37, OD-1, R-009, test-design.md §5 / TC-3

## Context

FR-2 (byte-faithful round-trip) is enforced by two CI gates split per LD-32:
an L0 per-PR gate over a ~100-file subset (Story 2.6, <60s) and a nightly
full-corpus + L2 Emacs-oracle gate (Story 2.7). Both need a corpus that is
representative, reproducible, license-clean, and self-verifying. The richest
available population of org syntax — including deliberately malformed inputs —
is the GNU org-mode test suite's `testing/lisp/test-org-element.el`
(GPL-3.0-or-later, ~thousands of `ert-deftest` assertions).

## Decision

### 1. Pinned fetch, never vendored (licensing + supply chain)

The `.el` file is **fetched at extraction time** into a gitignored cache
(`tools/corpus-extractor/cache/`) and is never committed. The pin is a triple
hard-coded in `tools/corpus-extractor/src/fetch.rs`:

- org-mode **release tag** (`ORG_RELEASE_TAG`, latest stable at impl time),
- the canonical **upstream URL** (GNU Savannah cgit `plain` view at that tag;
  GitHub mirror `bzg/org-mode` raw URL as fallback),
- the file's **SHA-256** (`SOURCE_SHA256`). `fetch` fails hard on mismatch and
  reports the observed digest so a deliberate pin bump is a one-line reviewed
  PR (LD-48-style discipline).

Rationale: the repo is MIT (LD-1) with a GPL-contagion posture (R-009, LD-37);
vendoring ~10k lines of GPL elisp would sit invisible to `cargo deny` (which
only sees Cargo deps). The **extracted snippets** are committed: short
org-syntax examples carried with provenance attribution (manifest headers +
`tests/fixtures/vault-corpus/README.md`), used as **test data only** and never
linked into or distributed with MIT binaries. Extraction is a maintainer
operation — no network in CI or in any test.

### 2. Harvesting scanner (pragmatic, not a full elisp reader)

A line/state scanner finds `(ert-deftest test-org-element/… )` blocks and
lexes the string literal passed to `org-test-with-temp-text` /
`org-test-with-temp-text-in-file`. Escape handling: `\"`, `\\`, `\n`, `\t`
(+ `\r`); `<point>` caret markers are stripped. **Known limits (by design):**

- snippets built via `concat`/`format`/variables (non-literal first argument)
  are not harvested;
- `\u…`/`\x…` codepoint escapes are not decoded (the escaped character is
  kept verbatim);
- assertions outside any `ert-deftest` are skipped.

Coverage of the *literal* population is what LD-44 needs; the epic's "~2000
assertions" is a target, not a contract — the meta-test asserts a floor set at
~75% of the observed harvest (`validate::FULL_CORPUS_FLOOR`), because a floor
of 0 would be a placebo.

### 3. Construct classification

`classify.rs` detects the 15 LD-44 constructs (architecture.md:1228–1245, the
Story 2.3 AC6 enumeration) with regex heuristics whose syntax shapes are
borrowed from `crates/orgsidian-parser/tests/semantic.rs` — no novel org
syntax. Classification is EOL-insensitive (detection normalizes CRLF; corpus
bytes keep it). TODO-keyword detection unions `#+TODO:`-family declarations
with the default set — replacement semantics belong to the parser, not to this
coverage heuristic. The SAME classifier runs at selection time, at `verify`
time, and in the TC-3 meta-test, so the committed manifest is self-verifying.

### 4. LD-44 selection algorithm

1. **Matrix first** — greedy-fill construct coverage (≥3 files per construct)
   from harvested <1KB snippets, preferring multi-construct snippets; then
   fill the small bucket to exactly 30 in stable id order.
2. **Buckets second** — harvested snippets are nearly all <1KB, so the 50
   medium (1–50KB) and 20 large (>50KB) members are **deterministically
   composed**: seeded selection of snippets joined under generated
   `* Section k` headings until a per-file byte target is crossed (medium
   targets spread 1.5–30KB, large 52–96KB). Coverage is *guaranteed* by
   force-including carrier snippets for every construct into three distinct
   medium compositions.
3. **Edge bucket third** — designated medium compositions receive mechanical
   transforms: CRLF ×3 + mixed-EOL ×2 (unusual-EOL ≥5), over-indented drawers
   ×3 + trailing-whitespace headlines ×2 (malformed-but-valid ≥5, the interim
   corpus shapes 14/15), Unicode/RTL ×5 (harvested RTL/CJK carriers first; a
   single documented salt line covers any gap — the one exception to
   "harvested material plus mechanical transforms only").

**Edge-bucket interpretation (recorded per story):** edge files are members of
the 100 and count inside their size buckets — 30+50+20 leaves no room for
extras.

**Determinism is a hard requirement:** sorted inputs, `BTreeMap` iteration,
fixed literal seed (`synth::SYNTH_SEED`) with per-composition derived seeds,
stable human-meaningful ids (`extracted/NNNN_<deftest-slug>`,
`synthesized/NNN_<bucket>-<recipe>`), no timestamps beyond the pin header.
Same pin + same extractor code ⇒ byte-identical outputs (double-extract diff
is the enforcement). Every synthesized member records its recipe (transforms,
seed, source snippet ids) in the manifest.

### 5. Embedded subset vs pointer manifest

- `fixtures/subset-pr.json` is **self-contained** (embedded org content): the
  Story 2.6 per-PR gate must work on a checkout **without git-LFS** (no
  GitHub-Free LFS bandwidth burn per PR), and JSON escaping makes embedded
  CRLF/trailing-whitespace bytes immune to EOL mangling.
- `fixtures/full-nightly.json` is a **pointer manifest** into
  `tests/fixtures/vault-corpus/` (architecture.md:995) — embedding ~2k
  assertions would bloat a file the PR gate never reads.
- Both JSONs are **regular git files**; only `vault-corpus/**/*.org` goes
  through git-LFS. (Variance from test-design §5.3 rule 3 — recorded, not
  spec-edited; epics.md AC is the sync-source.)
- Subset members also exist materialized under vault-corpus (byte-identical
  twins, same emission pass) so nightly/L2 tooling reuses the same bytes.

### 6. Supply chain note

`tools/corpus-extractor` is outside the Cargo workspace (root `Cargo.toml`
`exclude`), so its standalone `Cargo.lock` is **outside `cargo deny`'s root
scope**. Scanning that lockfile is named deferred work (Story 2.5 stanza in
`deferred-work.md`), alongside a CI build/test step for the tool itself.

## Regeneration procedure

```sh
cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- fetch
cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- extract
cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- verify
```

PR with commit tag `[fixture:epic-2]`, quoting the invocation and the pin
(tag + SHA-256). Pin bumps update `ORG_RELEASE_TAG` + `SOURCE_SHA256` together
in `fetch.rs`; `fetch` prints the observed digest on mismatch.

## Consequences

- The PR gate consumes one boring JSON contract (`header` + `entries[]`);
  Story 2.6 repoints the existing harness without parser-crate changes.
- The matrix is enforced twice (extract-time validation + committed-artifact
  meta-test), so artifact rot or a stale pin fails fast.
- Snippets reachable only through non-literal elisp forms are invisible to the
  corpus until the scanner grows; acceptable because the literal population
  already exceeds the LD-44 requirements by an order of magnitude.
- The org-mode pin needs a bump cadence (paired with the LD-48 parser-owner
  review) — named deferred work.
