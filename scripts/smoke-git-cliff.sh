#!/usr/bin/env bash
# smoke-git-cliff.sh — Story 1.15 AC5 LD-54 mapping contract.
#
# Exercises cliff.toml + git-cliff against a self-contained 5-commit fixture
# repository (initialized fresh in mktemp) and asserts that the LD-54
# Conventional-Commits → Keep-a-Changelog mapping is encoded correctly. The
# 9 assertions cover bucket placement (Added / Fixed / Changed), the
# breaking-prefix template branch, chore-exclusion, empty-headings invariant
# (Deprecated / Security), `--include-path` scoping for the plugin-api
# CHANGELOG, and `--prepend` preservation of manual blocks.
#
# Exit 0 = all 9 assertions pass; exit 1 on first failure. Verbose output
# is dumped on any failure so CI logs surface the diagnostic context.
#
# Runnable both locally (developer with git-cliff installed) and inside the
# release-smoke.yml CI workflow.
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Resolve the repo root from this script's location so cliff.toml is found
# regardless of CWD.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLIFF_TOML="${REPO_ROOT}/cliff.toml"

if [ ! -f "${CLIFF_TOML}" ]; then
  echo -e "${RED}FAIL${NC}: ${CLIFF_TOML} not found"
  exit 1
fi

# Verify git-cliff is on PATH (the release-smoke.yml workflow installs it via
# taiki-e/install-action@v2). On dev machines, `cargo install git-cliff
# --version "~2.13" --locked` or `brew install git-cliff`.
if ! command -v git-cliff > /dev/null 2>&1; then
  echo -e "${RED}FAIL${NC}: git-cliff not on PATH. Install via 'cargo install git-cliff --version \"~2.13\" --locked' (or 'brew install git-cliff' on macOS), or rely on CI."
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "${CLIFF_TOML}" "$TMP/cliff.toml"
cd "$TMP"

git init -q
git config user.email "smoke@example.com"
git config user.name "Smoke"
git config commit.gpgsign false

# Pre-create the 4 file paths needed for path-scoped commits (AC4 coverage).
mkdir -p \
  crates/orgsidian-parser/src \
  crates/orgsidian-vault/src \
  crates/orgsidian-index/src \
  crates/orgsidian-plugin-api/src

# Fixture commit 1: feat → Added
printf 'pub fn parser() {}\n' > crates/orgsidian-parser/src/lib.rs
git add crates/orgsidian-parser/src/lib.rs
git commit -q -m "feat(parser): add tree-sitter wrapper"

# Fixture commit 2: fix → Fixed
printf 'pub fn vault() {}\n' > crates/orgsidian-vault/src/lib.rs
git add crates/orgsidian-vault/src/lib.rs
git commit -q -m "fix(vault): handle AV-locked retry"

# Fixture commit 3: perf → Changed
printf 'pub fn index() {}\n' > crates/orgsidian-index/src/lib.rs
git add crates/orgsidian-index/src/lib.rs
git commit -q -m "perf(index): cache FTS5 query plan"

# Fixture commit 4: feat! + BREAKING CHANGE footer → Changed (with prefix)
#   Touches the plugin-api crate to also exercise AC4 path-scoping.
printf 'pub fn plugin_api() {}\n' > crates/orgsidian-plugin-api/src/lib.rs
git add crates/orgsidian-plugin-api/src/lib.rs
git commit -q -m "feat(plugin-api)!: rename Event variant" \
              -m "BREAKING CHANGE: rename Event::FileOpened to Event::FileLoaded"

# Fixture commit 5: chore → excluded
printf '# placeholder Cargo.lock\n' > Cargo.lock
git add Cargo.lock
git commit -q -m "chore: bump Cargo.lock"

# ---------------------------------------------------------------------------
# Invocation 1 — root (unscoped) CHANGELOG generation.
# Capture stdout; do not write to a file (faster than --output + parse).
# ---------------------------------------------------------------------------
OUTPUT="$(git-cliff --unreleased --tag v0.0.1 --config cliff.toml 2>&1)"

assert() {
  local n="$1" desc="$2" cmd="$3"
  if eval "$cmd"; then
    echo -e "${GREEN}PASS${NC} ${n}: ${desc}"
  else
    echo -e "${RED}FAIL${NC} ${n}: ${desc}"
    echo "--- captured output (first 100 lines) ---"
    echo "$OUTPUT" | head -100
    echo "--- end captured output ---"
    exit 1
  fi
}

# Assertion 1 — feat → Added
#   The `upper_first` Tera filter capitalises the first word ("Add ...");
#   case-insensitive grep keeps the contract verb-agnostic.
assert 1 "feat → Added bucket" \
  'echo "$OUTPUT" | grep -A 50 "^### Added" | grep -Fiq "add tree-sitter wrapper"'

# Assertion 2 — fix → Fixed
assert 2 "fix → Fixed bucket" \
  'echo "$OUTPUT" | grep -A 50 "^### Fixed" | grep -Fiq "handle AV-locked retry"'

# Assertion 3 — perf → Changed
assert 3 "perf → Changed bucket" \
  'echo "$OUTPUT" | grep -A 50 "^### Changed" | grep -Fiq "cache FTS5 query plan"'

# Assertion 4 — feat! → Changed with `⚠ BREAKING:` prefix
assert 4 "feat! → Changed with ⚠ BREAKING: prefix" \
  'echo "$OUTPUT" | grep -A 50 "^### Changed" | grep -F "⚠ BREAKING:" | grep -Fiq "rename Event variant"'

# Assertion 5 — chore excluded
assert 5 "chore excluded from output" \
  '! echo "$OUTPUT" | grep -Fiq "bump Cargo.lock"'

# Assertion 6 — empty Deprecated + Security headings emitted
assert 6 "empty Deprecated + Security headings emitted (AC6)" \
  'echo "$OUTPUT" | grep -q "^### Deprecated" && echo "$OUTPUT" | grep -q "^### Security"'

# ---------------------------------------------------------------------------
# Invocation 2 — scoped (plugin-api only) CHANGELOG generation.
# Capture to a separate variable; assert path-scoping is correct.
# ---------------------------------------------------------------------------
SCOPED_OUTPUT="$(git-cliff --include-path 'crates/orgsidian-plugin-api/**' \
                            --unreleased --tag v0.0.1 \
                            --config cliff.toml 2>&1)"

# Assertion 7 — scoped: plugin-api commit captured
assert 7 "scoped --include-path captures plugin-api commit" \
  'echo "$SCOPED_OUTPUT" | grep -Fiq "rename Event variant"'

# Assertion 8 — scoped: non-plugin-api commits excluded
assert 8 "scoped --include-path excludes non-plugin-api commits" \
  '! echo "$SCOPED_OUTPUT" | grep -Fiq "add tree-sitter wrapper" \
   && ! echo "$SCOPED_OUTPUT" | grep -Fiq "handle AV-locked retry" \
   && ! echo "$SCOPED_OUTPUT" | grep -Fiq "cache FTS5 query plan"'

# ---------------------------------------------------------------------------
# Invocation 3 — `--prepend` preserves manual blocks in the existing file.
# Seed plugin-api CHANGELOG.md with a manual `## [0.0.0]` block, then run
# scoped git-cliff with --prepend and verify both `## [v0.0.1]`-style and
# `## [0.0.0]` headings are present in the resulting file.
# ---------------------------------------------------------------------------
PLUGIN_CHANGELOG="crates/orgsidian-plugin-api/CHANGELOG.md"
cat > "$PLUGIN_CHANGELOG" <<'EOF'
## [0.0.0] - 2026-05-22

### Added
- Seed entry
EOF

git-cliff --include-path 'crates/orgsidian-plugin-api/**' \
          --unreleased --tag v0.0.1 \
          --config cliff.toml \
          --prepend "$PLUGIN_CHANGELOG" > /dev/null 2>&1

# Assertion 9 — both headings present after --prepend
#   git-cliff strips the `v` prefix in the rendered version per our template
#   (`version | trim_start_matches(pat="v")`), so we look for `## [0.0.1]`.
assert 9 "--prepend preserves manual [0.0.0] block alongside new [0.0.1]" \
  'grep -q "^## \[0\.0\.1\]" "$PLUGIN_CHANGELOG" && grep -q "^## \[0\.0\.0\]" "$PLUGIN_CHANGELOG"'

echo "smoke-git-cliff.sh: OK"
