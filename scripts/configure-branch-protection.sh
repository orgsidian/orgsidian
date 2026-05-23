#!/usr/bin/env bash
# scripts/configure-branch-protection.sh — Story 1.8 AC5
# (LD-32 merge-gate-on-nightly anti-atrophy posture).
#
# Idempotently applies the branch protection rule on
# orgsidian/orgsidian:main:
#   - Required status checks: pr (macos-14), pr (ubuntu-24.04),
#     merge-gate-nightly-fresh
#   - Required reviews: 0 (solo-dev posture per
#     [[feedback_spec_driven_not_solo_dev_bandwidth]] — review gate is the
#     bmad-code-review workflow run on each PR, not GitHub's enforced count).
#   - enforce_admins: false (lets the maintainer push hotfixes if CI is
#     genuinely broken; CI greenness is the durable gate).
#
# Run from the repo root, one-shot, by the maintainer before merging
# Story 1.8 to main. The script logs the resulting JSON to stdout for audit.
# Re-running on an already-protected branch updates the rule rather than
# erroring.

set -euo pipefail

REPO="${REPO:-orgsidian/orgsidian}"
BRANCH="${BRANCH:-main}"

# --- Pre-flight: gh CLI must be authenticated against the right org. -------
if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: gh CLI not found on PATH. Install from https://cli.github.com/" >&2
  exit 1
fi
if ! gh auth status >/dev/null 2>&1; then
  echo "ERROR: gh CLI is not authenticated. Run: gh auth login" >&2
  exit 1
fi
# Sanity-check we can read the target repo (early-fail if scopes are wrong).
if ! gh api "repos/${REPO}" >/dev/null 2>&1; then
  echo "ERROR: gh CLI cannot read ${REPO}. Check token scopes (needs 'repo' or 'public_repo' + admin:repo for branch protection)." >&2
  exit 1
fi

echo "Configuring branch protection on ${REPO}:${BRANCH}..."

# Idempotent PUT — GitHub branch protection API supports replace semantics.
# - required_status_checks.strict=false: do NOT require branches to be
#   up-to-date with main before merge (avoids the rebase-loop trap on
#   parallel PRs; LD-32 fast-merge cadence).
# - required_status_checks.contexts: the 3 checks introduced by Story 1.8.
# - required_pull_request_reviews=null: 0 enforced reviewers.
# - enforce_admins=false: maintainer can bypass for genuine emergencies.
# - restrictions=null: no push-restriction allowlist (open contributor model).
# - allow_force_pushes=false, allow_deletions=false: standard hardening.
RESPONSE="$(gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "repos/${REPO}/branches/${BRANCH}/protection" \
  -f required_status_checks[strict]=false \
  -f 'required_status_checks[contexts][]=pr (macos-14)' \
  -f 'required_status_checks[contexts][]=pr (ubuntu-24.04)' \
  -f 'required_status_checks[contexts][]=merge-gate-nightly-fresh' \
  -F enforce_admins=false \
  -f required_pull_request_reviews= \
  -f restrictions= \
  -F allow_force_pushes=false \
  -F allow_deletions=false)"

echo "$RESPONSE" | jq .
echo ""
echo "Branch protection applied. Required checks:"
echo "  - pr (macos-14)"
echo "  - pr (ubuntu-24.04)"
echo "  - merge-gate-nightly-fresh"
