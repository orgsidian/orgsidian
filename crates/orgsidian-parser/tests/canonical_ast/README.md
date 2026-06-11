# L2 canonical ASTs (Story 2.7, LD-45)

One `{stem}.json` per designated L2 seed file — the ground-truth meeting
point of the three-legged oracle (`docs/parser/l2-oracle.md`):

- `tests/l2_canonical.rs` pins Orgsidian's semantic projection to these
  files on every `cargo test --workspace` run (per-PR and nightly).
- The nightly `l2-emacs-oracle` job pins pinned Emacs 29.4 + 30.2
  `org-element` projections to the same files (LD-45 triage).

Files are **script-generated and human-reviewed** (LD-45 "peer-reviewed"
gate): regenerate via `scripts/l2-oracle/generate-canonical.sh` (local
Emacs; the committed set was generated with Emacs 30.2 / Org 9.7.11),
then review the diff in the PR. Never hand-edit; mutation requires PR
review (`fixtures/fixtures.toml` `[oracle.canonical-ast]`).

## Schema `l2-projection-v1`

Top level: `schema` (this string), `source` (corpus-relative path under
`tests/fixtures/vault-corpus/`), `deftest` (upstream ERT provenance from
`fixtures/full-nightly.json`), `headlines` (array). Per headline, in
document order with nesting:

| key         | type           | meaning                                              |
| ----------- | -------------- | ----------------------------------------------------- |
| `level`     | int            | number of stars                                        |
| `todo`      | string\|null   | recognized TODO keyword                                |
| `title`     | string         | org-element `:raw-value` (stars/keyword/tags stripped) |
| `tags`      | string array   | trailing tags in order, no colons (`[]`, never null)   |
| `scheduled` | string\|null   | planning timestamp `:raw-value` (source bytes)         |
| `deadline`  | string\|null   | planning timestamp `:raw-value`                        |
| `closed`    | string\|null   | planning timestamp `:raw-value`                        |
| `children`  | array          | recursive                                              |

Nothing else in v1: this is the honest intersection of Orgsidian's
Story-2.3 semantic surface and `org-element`'s headline properties.
Comparison is **structural** (canonicalized JSON values) on both legs, so
pretty-printing and key order never produce false divergence. Deepening
the projection (properties, drawers, body elements) is deferred work, not
a README edit.

## Seed selection rule

From the `fixtures/full-nightly.json` pointer manifest (per-entry
`constructs`/`deftest` provenance — consume, never reshape):

1. **≥1 file per construct kind present in the manifest** (15 kinds at
   release_9.8.5: inline-markup, drawer, block, list, citation, table,
   footnote, inline-latex, link, tag, deadline, recurring-timestamp,
   clock, heading-todo, scheduled), preferring the smallest representative
   per construct for peer-reviewability.
2. **≥1 structure-only file** (headline tree, no manifest constructs).
3. Target 12–20 files. Every candidate must pass the concordance
   pre-flight on BOTH oracle legs (Rust leg + local-Emacs leg) before
   being committed; a candidate exposing a real Orgsidian-vs-org-element
   divergence is recorded in `docs/parser/KNOWN_DIVERGENCES.md` and a
   concordant sibling is picked instead — the seed must be green on day 1.

Current seed: **17 files** (15 construct picks + 2 structure-only),
listed with corpus-relative paths in `scripts/l2-oracle/seed-list.txt`.
All 15 construct kinds are covered; no candidate failed the pre-flight,
so seeding produced no new KNOWN_DIVERGENCES entries. Construct → file:

| construct kind      | seed file                                  |
| ------------------- | ------------------------------------------ |
| block + drawer      | `0214_headline-properties-03.json`         |
| citation            | `0079_citation-reference-parser-02.json`   |
| clock               | `0092_clock-parser.json`                   |
| deadline            | `0371_planning-parser-01.json`             |
| drawer              | `0383_property-drawer-parser-07.json`      |
| footnote            | `0173_footnote-definition-parser-09.json`  |
| heading-todo        | `0185_headline-todo-keyword.json`          |
| inline-latex        | `0293_latex-fragment-parser.json`          |
| inline-markup       | `0517_context-15.json`                     |
| link                | `0338_link-parser-23.json`                 |
| list                | `0369_plain-list-parser-05.json`           |
| recurring-timestamp | `0440_timestamp-parser-08.json`            |
| scheduled           | `0372_planning-parser-02.json`             |
| table               | `0429_table-cell-parser-01.json`           |
| tag                 | `0194_headline-tags-02.json`               |
| (structure-only)    | `0562_cache-headline-01.json`, `0566_cache-headline-05.json` |

The `NNNN_` stem prefix guarantees filename uniqueness in this flat dir.
