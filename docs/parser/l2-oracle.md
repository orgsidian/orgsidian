# L2 Emacs Oracle — runbook (Story 2.7, LD-45)

The L2 oracle pins Orgsidian's semantic layer and Emacs `org-element` to a
shared, committed ground truth: the canonical ASTs under
`crates/orgsidian-parser/tests/canonical_ast/`. Three legs:

| Leg | Compares | Runs | Blocking |
|---|---|---|---|
| `round_trip_full` | corpus bytes ↔ `serialize(analyze())` | workspace tests everywhere + named nightly step on macOS/Ubuntu/Arch/Windows | red nightly → `merge-gate-nightly-fresh` |
| `l2_canonical_concordance` | Orgsidian projection ↔ canonical | `cargo test --workspace` (per-PR + nightly) | red per-PR = direct PR block |
| `l2-emacs-oracle` job | emacs 29.4 ↔ emacs 30.2 ↔ canonical | nightly, Linux, docker | FAIL/WARN per the triage table below |

"Both Emacs concordant against Orgsidian" (LD-45 case 1) is decidable
without ever diffing Orgsidian against Emacs directly: the Rust leg pins
Orgsidian to the canonical per-PR; the nightly job pins both Emacs
versions to the same canonical.

## Schema `l2-projection-v1`

Per headline, in document order, nested: `level` (int), `todo`
(string|null), `title` (string — org-element `:raw-value`: stars, TODO
keyword, and trailing tags stripped; everything else verbatim), `tags`
(string array — `[]`, never null), `scheduled`/`deadline`/`closed`
(string|null — planning timestamp `:raw-value`, source bytes verbatim),
`children` (recursive). Nothing else in v1 — this is the honest
intersection of Orgsidian's Story-2.3 semantic surface and `org-element`.
Raw org-element dumps are NOT version-stable (Org 9.6 vs 9.7
`:standard-properties`, buffer positions) — never compare them.

Comparison is **structural** (canonicalized JSON values, both in
`tests/l2_canonical.rs` and `scripts/l2-oracle/compare.py`): formatting
and key order can never produce false divergence.

## Seed selection

Documented in full in `crates/orgsidian-parser/tests/canonical_ast/README.md`:
≥1 file per manifest construct kind (15 kinds at release_9.8.5) preferring
the smallest representative, plus ≥1 structure-only file; target 12–20
files; every candidate must pass the concordance pre-flight on both legs
before being committed. Seed paths: `scripts/l2-oracle/seed-list.txt`.
Current seed: 17 files.

## Regenerating canonical ASTs

```sh
scripts/l2-oracle/generate-canonical.sh        # local `emacs` on PATH
EMACS=/path/to/emacs scripts/l2-oracle/generate-canonical.sh
```

Idempotent on an unchanged corpus + unchanged Emacs. Regenerated files
**must be human-reviewed in the PR** (LD-45 peer-review gate) and the
generating Emacs/Org versions quoted in the PR body (committed set:
Emacs 30.2 / Org 9.7.11). The canonical dir is governed fixture data —
`fixtures/fixtures.toml` `[oracle.canonical-ast]`.

## Triage decision table (nightly `l2-emacs-oracle` job)

Implemented by `scripts/l2-oracle/compare.py`; per seed file, with
`e29` = silex/emacs:29.4 output, `e30` = silex/emacs:30.2 output:

| Condition | Verdict | Meaning / action |
|---|---|---|
| `e29 == e30 == canonical` | **OK** | Oracle healthy. The Orgsidian leg is enforced by `l2_canonical_concordance` in the same nightly's test jobs. |
| `e29 == e30 != canonical` | **FAIL** (`::error`, job red) | Both Emacs agree against the canonical. If the Rust leg is **green**: "both Emacs concordant against Orgsidian" → Orgsidian bug → PR-blocking via the red nightly. If the Rust leg is **red**: canonical/oracle drift after an image bump — regenerate + review the canonical. Either way: act today. |
| `e29 != e30` | **WARN** (`::warning`, job stays green) | The Emacs versions disagree with each other (covers both LD-45 "discordant from each other" and the mixed one-concordant case): human review, defer, do NOT block. Log an entry in `docs/parser/KNOWN_DIVERGENCES.md` (construct / expected / observed / chosen behavior / owner). |
| docker pull error, emacs crash, missing canonical | **FAIL** | An oracle that cannot run is a broken gate, not a skipped one. |

The job evaluates ALL seed files, then exits non-zero if any FAIL-class
divergence was seen — full triage picture per run.

## Image bumps (R-028)

Images are pinned by exact-version tag (`silex/emacs:29.4`, `silex/emacs:30.2`
— no official `emacs` repo exists on Docker Hub; silex is the de-facto
maintained multi-version image). Tags rebuild on a rolling basis, so the
job logs each image's digest at runtime — the R-028 drift audit trail.
To bump (e.g. a new Emacs 30.x point release):

1. Update the `EMACS_29_IMAGE`/`EMACS_30_IMAGE` tags in
   `.github/workflows/nightly.yml` (latest stable of each pinned line).
2. Regenerate the canonical ASTs locally with a matching Emacs and review
   the diff (`generate-canonical.sh` — any change is a real
   org-element-behavior change and deserves scrutiny).
3. Land both in one PR; the next nightly validates the new images against
   the reviewed canonical (the LD-45 meta-test).

## Running locally

With docker (mirrors CI exactly; run from the repo root):

```sh
docker run --rm -v "$PWD:/work" -w /work silex/emacs:29.4 \
  emacs -Q --batch -l scripts/l2-oracle/projection.el \
  --eval '(l2-project-file "tests/fixtures/vault-corpus/extracted/0372_planning-parser-02.org")'
# same with silex/emacs:30.2
```

With a local emacs (≥27 for the builtin `json-serialize`):

```sh
emacs -Q --batch -l scripts/l2-oracle/projection.el \
  --eval '(l2-project-file "tests/fixtures/vault-corpus/extracted/0372_planning-parser-02.org")'
```

Then compare a full local sweep with the comparator:

```sh
python3 scripts/l2-oracle/compare.py \
  --canonical-dir crates/orgsidian-parser/tests/canonical_ast \
  --e29-dir <dir-of-29-outputs> --e30-dir <dir-of-30-outputs>
```

The Rust leg alone: `cargo test -p orgsidian-parser --test l2_canonical --locked`.
