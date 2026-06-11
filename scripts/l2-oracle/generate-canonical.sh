#!/usr/bin/env bash
# generate-canonical.sh — regenerate the committed L2 canonical ASTs
# (Story 2.7, LD-45). See docs/parser/l2-oracle.md.
#
# Reads the designated seed list (scripts/l2-oracle/seed-list.txt: one
# corpus-relative path per line, `#` comments allowed), projects each file
# through scripts/l2-oracle/projection.el with a local `emacs`, wraps the
# result with source/schema/deftest metadata (deftest provenance looked up
# in fixtures/full-nightly.json — single source of truth), and writes
# pretty-printed JSON (LF, trailing newline) to
# crates/orgsidian-parser/tests/canonical_ast/{stem}.json.
#
# Idempotent: re-running on an unchanged corpus + unchanged Emacs produces
# byte-identical output. Regenerated files MUST be human-reviewed in the PR
# (LD-45 "peer-reviewed" gate) and the generating Emacs/Org versions quoted
# in the PR body. Record the versions:
#   emacs --version | head -1
#   emacs -Q --batch --eval '(progn (require (quote org)) (princ (org-version)))'
#
# Usage: scripts/l2-oracle/generate-canonical.sh
#   EMACS=/path/to/emacs to override the binary (default: `emacs` on PATH).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
EMACS="${EMACS:-emacs}"
SEED_LIST="$ROOT/scripts/l2-oracle/seed-list.txt"
PROJECTION="$ROOT/scripts/l2-oracle/projection.el"
CORPUS="$ROOT/tests/fixtures/vault-corpus"
MANIFEST="$ROOT/fixtures/full-nightly.json"
OUT_DIR="$ROOT/crates/orgsidian-parser/tests/canonical_ast"

command -v "$EMACS" >/dev/null 2>&1 || {
  echo "error: emacs not found (set EMACS=/path/to/emacs)" >&2
  exit 1
}
command -v python3 >/dev/null 2>&1 || {
  echo "error: python3 not found (needed to wrap projections with manifest metadata)" >&2
  exit 1
}

mkdir -p "$OUT_DIR"

generated=0
while IFS= read -r rel; do
  case "$rel" in '' | '#'*) continue ;; esac
  src="$CORPUS/$rel"
  [ -f "$src" ] || {
    echo "error: seed file not found: $src" >&2
    exit 1
  }
  stem="$(basename "$rel" .org)"
  out="$OUT_DIR/$stem.json"
  "$EMACS" -Q --batch -l "$PROJECTION" --eval "(l2-project-file \"$src\")" |
    python3 -c '
import json, sys

rel, manifest_path = sys.argv[1], sys.argv[2]
headlines = json.load(sys.stdin)
with open(manifest_path, encoding="utf-8") as f:
    manifest = json.load(f)
deftest = next(
    (e["deftest"] for e in manifest["entries"] if e["path"] == rel), None
)
if deftest is None:
    sys.exit(f"error: {rel} not found in {manifest_path}")
obj = {
    "schema": "l2-projection-v1",
    "source": rel,
    "deftest": deftest,
    "headlines": headlines,
}
print(json.dumps(obj, indent=2, ensure_ascii=False))
' "$rel" "$MANIFEST" >"$out"
  echo "wrote ${out#"$ROOT"/}"
  generated=$((generated + 1))
done <"$SEED_LIST"

if [ "$generated" -eq 0 ]; then
  echo "error: no seed entries in $SEED_LIST" >&2
  exit 1
fi
echo "regenerated $generated canonical AST file(s) — review the diff before committing"
