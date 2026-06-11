#!/usr/bin/env python3
"""LD-45 triage comparator for the nightly l2-emacs-oracle job (Story 2.7).

For every committed canonical AST (crates/orgsidian-parser/tests/
canonical_ast/*.json) compare three structurally-canonicalized JSON values
(parsed values, never bytes — pretty-printing and key order can not
produce false divergence):

  canonical  the committed, human-reviewed ground truth (`headlines` key)
  e29        the silex/emacs:29.4 projection output (bare headlines array)
  e30        the silex/emacs:30.2 projection output (bare headlines array)

Triage per file (docs/parser/l2-oracle.md mirrors this table):

  e29 == canonical and e30 == canonical
      OK — oracle healthy. (The Orgsidian leg is enforced separately by
      tests/l2_canonical.rs inside the same nightly's cargo test jobs.)
  e29 == e30 != canonical
      FAIL (::error) — both Emacs agree against the canonical. With the
      Rust leg green this is "both Emacs concordant against Orgsidian"
      (LD-45 case 1): Orgsidian bug, PR-blocking via the red nightly.
      With the Rust leg red it is canonical/oracle drift after an image
      bump. Either way: maintainer action required.
  e29 != e30
      WARN (::warning) — the Emacs versions disagree with each other
      (covers LD-45 "both discordant" AND the mixed one-concordant case:
      human review, defer, do not block). Log the divergence in
      docs/parser/KNOWN_DIVERGENCES.md (construct / expected / observed /
      chosen behavior / owner).
  missing or unparsable input
      FAIL — an oracle that cannot run is a broken gate, not a skipped one.

Evaluates ALL files, then exits non-zero if any FAIL-class divergence was
seen (full triage picture per run; nightly fail-fast: false philosophy).

Usage:
  python3 scripts/l2-oracle/compare.py \
      --canonical-dir crates/orgsidian-parser/tests/canonical_ast \
      --e29-dir /tmp/l2-out/e29 --e30-dir /tmp/l2-out/e30
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

MIN_SEED = 10  # same anti-placebo floor as tests/l2_canonical.rs


def load(path: Path) -> tuple[object, str | None]:
    """Parsed JSON value, or (None, reason) on failure."""
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f), None
    except FileNotFoundError:
        return None, f"missing file {path}"
    except (json.JSONDecodeError, OSError) as exc:
        return None, f"unreadable/unparsable {path}: {exc}"


def triage(stem: str, canonical_file: Path, e29_dir: Path, e30_dir: Path) -> str:
    """Return verdict 'ok' | 'warn' | 'fail' for one seed file, printing
    GitHub annotations as side effects."""
    canonical_doc, err = load(canonical_file)
    if err is None:
        canonical = canonical_doc.get("headlines") if isinstance(canonical_doc, dict) else None
        if canonical is None:
            err = f"{canonical_file} has no `headlines` key"
    e29, err29 = load(e29_dir / f"{stem}.json")
    e30, err30 = load(e30_dir / f"{stem}.json")
    problems = [e for e in (err, err29, err30) if e]
    if problems:
        print(f"::error::{stem}: oracle could not run — {'; '.join(problems)}. "
              "An oracle that cannot run is a broken gate, not a skipped one "
              "(docker pull / emacs crash / missing canonical).")
        return "fail"

    if e29 == canonical and e30 == canonical:
        print(f"OK    {stem}: e29 == e30 == canonical")
        return "ok"

    if e29 == e30:  # != canonical
        print(f"::error::{stem}: BOTH Emacs (29.4 + 30.2) agree against the "
              f"canonical AST (LD-45). If the Rust leg (l2_canonical_concordance) "
              f"is green this is an Orgsidian bug — PR-blocking; if red, it is "
              f"canonical/oracle drift after an image bump. Triage: "
              f"docs/parser/l2-oracle.md. emacs={json.dumps(e29)} "
              f"canonical={json.dumps(canonical)}")
        return "fail"

    print(f"::warning::{stem}: Emacs 29.4 and 30.2 disagree with each other "
          f"(LD-45: human review; defer, do NOT block). Log an entry in "
          f"docs/parser/KNOWN_DIVERGENCES.md (construct / expected / observed / "
          f"chosen behavior / owner). e29={json.dumps(e29)} e30={json.dumps(e30)} "
          f"canonical={json.dumps(canonical)}")
    return "warn"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canonical-dir", required=True, type=Path)
    parser.add_argument("--e29-dir", required=True, type=Path)
    parser.add_argument("--e30-dir", required=True, type=Path)
    args = parser.parse_args()

    canonical_files = sorted(args.canonical_dir.glob("*.json"))
    if len(canonical_files) < MIN_SEED:
        print(f"::error::only {len(canonical_files)} canonical AST file(s) under "
              f"{args.canonical_dir} — seed wiped or wrong checkout? "
              f"(anti-placebo floor: {MIN_SEED})")
        return 1

    counts = {"ok": 0, "warn": 0, "fail": 0}
    for canonical_file in canonical_files:
        verdict = triage(canonical_file.stem, canonical_file, args.e29_dir, args.e30_dir)
        counts[verdict] += 1

    print(f"\nL2 oracle triage: {counts['ok']} OK, {counts['warn']} WARN, "
          f"{counts['fail']} FAIL over {len(canonical_files)} seed file(s)")
    return 1 if counts["fail"] else 0


if __name__ == "__main__":
    sys.exit(main())
