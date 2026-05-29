#!/usr/bin/env bash
# smoke-commitlint-title.sh — exercises the LD-54 PR-title lint surface
# (engine layer) with known-bad and known-good title fixtures. Invoked
# manually (`bash scripts/smoke-commitlint-title.sh`) and from CI as the
# AC7 smoke gate. The end-to-end action integration is verified on the
# story PR via the manual title-flip in Task 6. Exit 0 = both cases
# behaved correctly; non-zero = drift.
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; NC='\033[0m'

# Case 1: malformed PR title MUST fail.
if echo "Fix bug in parser" | pnpm exec commitlint > /dev/null 2>&1; then
  echo -e "${RED}FAIL${NC}: malformed title 'Fix bug in parser' was accepted"
  exit 1
fi
echo -e "${GREEN}PASS${NC}: malformed PR title correctly rejected"

# Case 2: well-formed PR title MUST pass.
if ! echo "fix(parser): handle empty buffer edge case" | pnpm exec commitlint > /dev/null 2>&1; then
  echo -e "${RED}FAIL${NC}: well-formed title was rejected"
  exit 1
fi
echo -e "${GREEN}PASS${NC}: well-formed PR title correctly accepted"

echo "smoke-commitlint-title.sh: OK"
