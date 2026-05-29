#!/usr/bin/env bash
# smoke-git-cliff.sh — Story 1.15 AC5 LD-54 mapping contract.
#
# Exercises cliff.toml + git-cliff against self-contained fixture repos
# (initialized fresh in mktemp) and asserts that the LD-54
# Conventional-Commits → Keep-a-Changelog mapping is encoded correctly.
# The assertions cover:
#   - Bucket placement (Added / Fixed / Changed) via awk-bounded ranges
#     so a commit cannot satisfy a bucket assertion by leaking into an
#     adjacent bucket's grep window.
#   - Breaking-prefix template branch — including the CC v1.0.0 scoped
#     form `feat(scope)!:` WITHOUT a `BREAKING CHANGE:` footer (verifies
#     the scope-aware parser regex in cliff.toml).
#   - chore-exclusion.
#   - Empty-headings invariant (Deprecated / Security) when there ARE
#     commits routed to other buckets (AC6).
#   - `--include-path` scoping for the plugin-api CHANGELOG, including
#     the zero-match case (no commits touch the scoped path).
#   - Chore-only release: the `{% if commits | length > 0 %}` template
#     guard omits a phantom version block when all commits are skipped.
#   - Bare-version path (`--tag 0.0.1`, no `v` prefix) — what real
#     `cargo release` passes via `--tag "v$NEW_VERSION"` after the
#     env-var prefix wrap in release.toml.
#   - `--prepend` preservation of manual blocks AND interaction with a
#     pre-existing `## [Unreleased]` heading (documents the observed
#     behavior; see Review Findings D2 in the story file).
#
# Exit 0 = all assertions pass; exit 1 on first failure. Verbose output
# is dumped on any failure so CI logs surface the diagnostic context.
#
# Runnable both locally (developer with git-cliff installed) and inside
# the release-smoke.yml CI workflow.
set -euo pipefail

# Defensive: some git-cliff versions force ANSI color codes on stderr
# when stdout is a pipe; setting NO_COLOR=1 ensures captured `$OUTPUT`
# is escape-sequence-free so `grep -F` literal matches on `⚠ BREAKING:`
# stay deterministic across versions and locales.
export NO_COLOR=1

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
TMP_CHORE="$(mktemp -d)"
TMP_UNREL="$(mktemp -d)"
trap 'rm -rf "$TMP" "$TMP_CHORE" "$TMP_UNREL"' EXIT

cp "${CLIFF_TOML}" "$TMP/cliff.toml"
cp "${CLIFF_TOML}" "$TMP_CHORE/cliff.toml"
cp "${CLIFF_TOML}" "$TMP_UNREL/cliff.toml"

# ===========================================================================
# Fixture 1 — six commits exercising the LD-54 mapping table.
# ===========================================================================
cd "$TMP"

git init -q
git config user.email "smoke@example.com"
git config user.name "Smoke"
git config commit.gpgsign false

# Pre-create the file paths needed for path-scoped commits (AC4 coverage).
mkdir -p \
  crates/orgsidian-parser/src \
  crates/orgsidian-vault/src \
  crates/orgsidian-index/src \
  crates/orgsidian-plugin-api/src \
  crates/orgsidian-search/src

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

# Fixture commit 4: feat(scope)! WITH BREAKING CHANGE footer → Changed
#   Touches the plugin-api crate to also exercise AC4 path-scoping.
#   Routing happens via the `^BREAKING CHANGE` footer parser AND the
#   `^feat(\(.+\))?!` message parser — either alone would route it to
#   `Changed`. The footer parser path is the older redundant safety net.
printf 'pub fn plugin_api() {}\n' > crates/orgsidian-plugin-api/src/lib.rs
git add crates/orgsidian-plugin-api/src/lib.rs
git commit -q -m "feat(plugin-api)!: rename Event variant" \
              -m "BREAKING CHANGE: rename Event::FileOpened to Event::FileLoaded"

# Fixture commit 5: chore → excluded
printf '# placeholder Cargo.lock\n' > Cargo.lock
git add Cargo.lock
git commit -q -m "chore: bump Cargo.lock"

# Fixture commit 6: feat(scope)! WITHOUT footer → Changed (regex-only path).
#   This is the canary that exposes the scope-aware regex requirement in
#   cliff.toml. With a `^feat!` regex (without `(\(.+\))?`), this commit
#   would fall through to the non-breaking `^feat` parser and land in
#   `Added` instead of `Changed`, losing the `⚠ BREAKING:` prefix.
printf 'pub fn search() {}\n' > crates/orgsidian-search/src/lib.rs
git add crates/orgsidian-search/src/lib.rs
git commit -q -m "feat(search)!: drop legacy query syntax"

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
    echo "--- captured output (first 120 lines) ---"
    echo "$OUTPUT" | head -120
    echo "--- end captured output ---"
    exit 1
  fi
}

# Awk range pattern bounds each bucket window strictly between its own
# `### Heading` and the NEXT `### ` heading — so a bucket-mixing regression
# (e.g., a `feat` commit landing under `Fixed`) cannot satisfy the wrong
# bucket's assertion via grep's `-A N` window straddling.

# Assertion 1 — feat → Added
#   The `upper_first` Tera filter capitalises the first word ("Add ...");
#   case-insensitive grep keeps the contract verb-agnostic.
assert 1 "feat → Added bucket" \
  'echo "$OUTPUT" | awk "/^### Added/{f=1;next} /^### /{f=0} f" | grep -Fiq "add tree-sitter wrapper"'

# Assertion 2 — fix → Fixed
assert 2 "fix → Fixed bucket" \
  'echo "$OUTPUT" | awk "/^### Fixed/{f=1;next} /^### /{f=0} f" | grep -Fiq "handle AV-locked retry"'

# Assertion 3 — perf → Changed
assert 3 "perf → Changed bucket" \
  'echo "$OUTPUT" | awk "/^### Changed/{f=1;next} /^### /{f=0} f" | grep -Fiq "cache FTS5 query plan"'

# Assertion 4 — feat(scope)! WITH footer → Changed with `⚠ BREAKING:` prefix
assert 4 "feat(scope)! [with footer] → Changed with ⚠ BREAKING: prefix" \
  'echo "$OUTPUT" | awk "/^### Changed/{f=1;next} /^### /{f=0} f" | grep -F "⚠ BREAKING:" | grep -Fiq "rename Event variant"'

# Assertion 5 — chore excluded
assert 5 "chore excluded from output" \
  '! echo "$OUTPUT" | grep -Fiq "bump Cargo.lock"'

# Assertion 6 — empty Deprecated + Security headings emitted (AC6)
#   Only meaningful when commits ARE present (the new template guard wraps
#   the body in `{% if commits | length > 0 %}`). Fixture 1 has commits, so
#   all six KAC buckets are emitted.
assert 6 "empty Deprecated + Security headings emitted (AC6)" \
  'echo "$OUTPUT" | grep -q "^### Deprecated" && echo "$OUTPUT" | grep -q "^### Security"'

# Assertion 7 — feat(scope)! WITHOUT footer → Changed with prefix
#   Verifies the scope-aware `^feat(\(.+\))?!` parser regex routes correctly
#   on the message marker alone. If the regex regresses to `^feat!`, this
#   commit lands in `Added` and assertion 7 fails.
assert 7 "feat(scope)! [no footer] → Changed with ⚠ BREAKING: prefix" \
  'echo "$OUTPUT" | awk "/^### Changed/{f=1;next} /^### /{f=0} f" | grep -F "⚠ BREAKING:" | grep -Fiq "drop legacy query syntax"'

# Assertion 7b — same commit does NOT appear in Added (no double-routing)
assert 7b "feat(scope)! [no footer] does NOT appear in Added bucket" \
  '! echo "$OUTPUT" | awk "/^### Added/{f=1;next} /^### /{f=0} f" | grep -Fiq "drop legacy query syntax"'

# ---------------------------------------------------------------------------
# Invocation 2 — scoped (plugin-api only) CHANGELOG generation.
# Capture to a separate variable; assert path-scoping is correct.
# ---------------------------------------------------------------------------
SCOPED_OUTPUT="$(git-cliff --include-path 'crates/orgsidian-plugin-api/**' \
                            --unreleased --tag v0.0.1 \
                            --config cliff.toml 2>&1)"

# Assertion 8 — scoped: plugin-api commit captured
assert 8 "scoped --include-path captures plugin-api commit" \
  'echo "$SCOPED_OUTPUT" | grep -Fiq "rename Event variant"'

# Assertion 9 — scoped: non-plugin-api commits excluded
assert 9 "scoped --include-path excludes non-plugin-api commits" \
  '! echo "$SCOPED_OUTPUT" | grep -Fiq "add tree-sitter wrapper" \
   && ! echo "$SCOPED_OUTPUT" | grep -Fiq "handle AV-locked retry" \
   && ! echo "$SCOPED_OUTPUT" | grep -Fiq "cache FTS5 query plan" \
   && ! echo "$SCOPED_OUTPUT" | grep -Fiq "drop legacy query syntax"'

# ---------------------------------------------------------------------------
# Invocation 3 — `--prepend` preserves manual blocks in the existing file.
# Seed plugin-api CHANGELOG.md with a manual `## [0.0.0]` block, then run
# scoped git-cliff with --prepend and verify both `## [0.0.1]` and
# `## [0.0.0]` headings are present in the resulting file.
# ---------------------------------------------------------------------------
PLUGIN_CHANGELOG="crates/orgsidian-plugin-api/CHANGELOG.md"
cat > "$PLUGIN_CHANGELOG" <<'EOF'
## [0.0.0] - 2026-05-22

### Added
- Seed entry
EOF

# Capture stderr so a non-zero exit surfaces a diagnostic rather than dying
# silently under `set -euo pipefail`.
PREPEND_OUTPUT="$(git-cliff --include-path 'crates/orgsidian-plugin-api/**' \
                            --unreleased --tag v0.0.1 \
                            --config cliff.toml \
                            --prepend "$PLUGIN_CHANGELOG" 2>&1)" || {
  echo -e "${RED}FAIL${NC} 10: --prepend invocation exited non-zero"
  echo "--- prepend output ---"
  echo "$PREPEND_OUTPUT"
  echo "--- file contents ---"
  cat "$PLUGIN_CHANGELOG" || true
  exit 1
}

# Assertion 10 — both headings present after --prepend
#   The template strips the leading `v` so the rendered heading is
#   `## [0.0.1]` (not `## [v0.0.1]`).
assert 10 "--prepend preserves manual [0.0.0] block alongside new [0.0.1]" \
  'grep -q "^## \[0\.0\.1\]" "$PLUGIN_CHANGELOG" && grep -q "^## \[0\.0\.0\]" "$PLUGIN_CHANGELOG"'

# ---------------------------------------------------------------------------
# Invocation 4 — bare-version path (`--tag 0.0.1`, no `v` prefix).
# Documents what cargo-release actually passes BEFORE the `v` wrap in
# release.toml. Even with a bare version, the template's
# `trim_start_matches(pat="v")` is a no-op (nothing to strip), so the
# heading still renders cleanly as `## [0.0.1]`. release.toml wraps the
# env var as `--tag "v$NEW_VERSION"` to satisfy `tag_pattern`, so this
# assertion pins the template's tolerance of either path.
# ---------------------------------------------------------------------------
BARE_OUTPUT="$(git-cliff --unreleased --tag 0.0.1 --config cliff.toml 2>&1)" || {
  echo -e "${RED}FAIL${NC} 11: bare-version invocation exited non-zero"
  echo "--- bare output ---"
  echo "$BARE_OUTPUT"
  exit 1
}

assert 11 "bare --tag (no v prefix) renders [0.0.1] heading correctly" \
  'echo "$BARE_OUTPUT" | grep -q "^## \[0\.0\.1\]"'

# ---------------------------------------------------------------------------
# Invocation 5 — `--include-path` zero-match: scoped invocation against a
# crate path that NO commit in the fixture touches. Verifies the template
# guard `{% if commits | length > 0 %}` omits the version block entirely
# instead of emitting a phantom heading with six empty sections. This
# protects the chained `pre-release-hook` in release.toml from producing
# a malformed CHANGELOG entry when a release window has no commits in the
# scoped sub-tree.
# ---------------------------------------------------------------------------
ZERO_INCLUDE_OUTPUT="$(git-cliff --include-path 'crates/orgsidian-nonexistent/**' \
                                 --unreleased --tag v0.0.1 \
                                 --config cliff.toml 2>&1)" || {
  echo -e "${RED}FAIL${NC} 12: zero-include-path invocation exited non-zero"
  echo "--- zero-include output ---"
  echo "$ZERO_INCLUDE_OUTPUT"
  exit 1
}

assert 12 "--include-path zero-match: no version block emitted" \
  '! echo "$ZERO_INCLUDE_OUTPUT" | grep -q "^## \["'

# ===========================================================================
# Fixture 2 — chore-only repo: all commits are routed to `skip = true`
# parsers, so the template's `commits` array is empty and the guard
# omits the version block.
# ===========================================================================
cd "$TMP_CHORE"
git init -q
git config user.email "smoke@example.com"
git config user.name "Smoke"
git config commit.gpgsign false

mkdir -p ci
printf 'placeholder\n' > ci/cfg.yml
git add ci/cfg.yml
git commit -q -m "ci: pin runner image"

printf '# placeholder\n' > Cargo.lock
git add Cargo.lock
git commit -q -m "chore: bump Cargo.lock"

printf 'placeholder\n' > README.md
git add README.md
git commit -q -m "docs: tweak README phrasing"

CHORE_OUTPUT="$(git-cliff --unreleased --tag v0.0.2 --config cliff.toml 2>&1)" || {
  echo -e "${RED}FAIL${NC} 13: chore-only invocation exited non-zero"
  echo "--- chore-only output ---"
  echo "$CHORE_OUTPUT"
  exit 1
}

# Assertion 13 — chore-only release: template guard omits the version block.
#   With the guard, the entire body (heading + six buckets) is suppressed
#   when `commits | length == 0`. Output may contain the `[changelog]`
#   header preamble (without --prepend) but MUST NOT contain a `## [`
#   version heading.
assert 13 "chore-only release: no version block emitted (template guard)" \
  '! echo "$CHORE_OUTPUT" | grep -q "^## \["'

# ===========================================================================
# Fixture 3 — pre-seeded `## [Unreleased]` heading: documents the observed
# `git-cliff --prepend` behavior against the real CHANGELOG.md layout.
# Per Review Findings D2 in the story file, `--prepend` literally inserts
# above existing content WITHOUT consuming the `[Unreleased]` heading —
# so the resulting file has BOTH the new version block AND the original
# `[Unreleased]` heading, in that order. A follow-up release-runbook
# story may decide to consume `[Unreleased]` via a wrapper script or
# `pre-release-replacements`. This assertion pins the current contract
# so a behavior change in git-cliff is caught by the smoke.
# ===========================================================================
cd "$TMP_UNREL"
git init -q
git config user.email "smoke@example.com"
git config user.name "Smoke"
git config commit.gpgsign false

mkdir -p crates/orgsidian-parser/src
printf 'pub fn parser() {}\n' > crates/orgsidian-parser/src/lib.rs
git add crates/orgsidian-parser/src/lib.rs
git commit -q -m "feat(parser): add tree-sitter wrapper"

UNREL_CHANGELOG="CHANGELOG.md"
cat > "$UNREL_CHANGELOG" <<'EOF'
# Changelog

Real-layout fixture mirroring CHANGELOG.md L1-L13.

## [Unreleased]

EOF

UNREL_PREPEND_OUTPUT="$(git-cliff --unreleased --tag v0.1.0 \
                                  --config cliff.toml \
                                  --prepend "$UNREL_CHANGELOG" 2>&1)" || {
  echo -e "${RED}FAIL${NC} 14: --prepend with [Unreleased] seed exited non-zero"
  echo "--- prepend output ---"
  echo "$UNREL_PREPEND_OUTPUT"
  echo "--- file contents ---"
  cat "$UNREL_CHANGELOG" || true
  exit 1
}

# Assertion 14 — new version block AND pre-existing [Unreleased] both
# present after --prepend. Pins current git-cliff behavior; a future fix
# (wrapper script that consumes [Unreleased]) would update this assertion.
assert 14 "--prepend against [Unreleased]: both new [0.1.0] and [Unreleased] survive" \
  'grep -q "^## \[0\.1\.0\]" "$UNREL_CHANGELOG" && grep -q "^## \[Unreleased\]" "$UNREL_CHANGELOG"'

echo "smoke-git-cliff.sh: OK"
