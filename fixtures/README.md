# `fixtures/` — shared corpus manifests

First promotion of a fixture set to repo root (CONTRIBUTING §5: a fixture is
promoted out of its crate only when a second consumer appears, and the
promotion PR must name the consumers).

## Contents

| File | What it is | Consumers |
|---|---|---|
| `subset-pr.json` | Self-contained L0 round-trip subset (LD-44): 100 files with **embedded** org content, size/edge buckets, constructs, provenance. | Story 2.6's per-PR `round_trip_subset` gate (`cargo test -p orgsidian-parser`), `tools/corpus-extractor` `verify` subcommand, the TC-3 `matrix_coverage` meta-test, and the round-trip preflight test. |
| `full-nightly.json` | Pointer manifest over the full nightly corpus (~one entry per harvested `test-org-element.el` assertion; paths into `tests/fixtures/vault-corpus/`). | The `round_trip_full` nightly gate (named step on all four OSes; also rides `cargo test --workspace` — Story 2.7), the L2 oracle seed designation + canonical-AST regeneration (`scripts/l2-oracle/`), and `verify`. |
| `fixtures.toml` | Per-epic fixture ownership declaration (Murat P1, test-design.md §5). Hand-maintained. | Maintainers reviewing fixture-mutation PRs. |

## Rules

- `subset-pr.json` and `full-nightly.json` are **generated** by
  `tools/corpus-extractor` — never hand-edit them. Regenerate via
  `fetch` → `extract` → `verify` and quote the invocation + org-mode pin in
  the PR (commit tag `[fixture:epic-2]`).
- Both manifests are **regular git files, not LFS** — the per-PR gate must
  work on a checkout without git-LFS. Only `tests/fixtures/vault-corpus/`
  is meant to go through LFS (currently committed raw per the Story 2.5 AC6
  fallback — see the `FOLLOWUP(LFS-migration)` marker in `.gitattributes`).
- Selection algorithm, licensing posture, and regeneration procedure:
  `docs/adr/0001-corpus-subset-selection.md`.
