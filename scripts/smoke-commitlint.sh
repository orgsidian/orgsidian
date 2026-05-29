#!/usr/bin/env bash
# smoke-commitlint.sh — exercises the LD-54 commit-msg hook in both directions.
# Invoked manually (`bash scripts/smoke-commitlint.sh`) and from CI as the
# AC6 smoke gate. Exit 0 = both cases behaved correctly; non-zero = drift.
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; NC='\033[0m'

# Case 1: malformed message MUST be rejected (exit code 1 from commitlint).
if echo "broken message" | pnpm exec commitlint > /dev/null 2>&1; then
  echo -e "${RED}FAIL${NC}: malformed message 'broken message' was accepted (expected rejection)"
  exit 1
fi
echo -e "${GREEN}PASS${NC}: malformed message correctly rejected"

# Case 2: well-formed message MUST be accepted (exit code 0).
if ! echo "feat(parser): add tree-sitter wrapper" | pnpm exec commitlint > /dev/null 2>&1; then
  echo -e "${RED}FAIL${NC}: well-formed message was rejected (expected accept)"
  exit 1
fi
echo -e "${GREEN}PASS${NC}: well-formed message correctly accepted"

echo "smoke-commitlint.sh: OK"
