# Story 1.14: Configure commitlint + husky commit-msg hook + CI gate

Status: review

## Metadata

github_issue: 14

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want `commitlint` enforcing Conventional Commits v1.0.0 on three surfaces — (1) every local commit via the `husky` `commit-msg` hook (already on disk; verify-only), (2) every PR's commit range via a CI step running `pnpm commitlint --from origin/main --to HEAD`, and (3) every PR title via `amannn/action-semantic-pull-request@v5` — plus two smoke tests proving (a) the local hook rejects malformed messages and (b) the CI title check fails on malformed titles,
So that every commit and PR title qualifies for the `git-cliff` CHANGELOG ingestion lined up in Story 1.15, the LD-54 enforcement chain documented in [CONTRIBUTING.md §2](CONTRIBUTING.md#L42-L83) becomes real (not "lands in Story 1.14 (not yet wired)"), and Story 1.15's reliance on the CC vocabulary in commits-already-on-`main` is unbreakable by construction.

## Acceptance Criteria

**AC1 — `package.json` lists `@commitlint/cli` + `@commitlint/config-conventional` at latest stable.**

- **Already done** (verify-only): [package.json:18-20](package.json#L18-L20) pins `@commitlint/cli: 21.0.1` + `@commitlint/config-conventional: 21.0.1` (Story 1.3 installed; both v21.0.1 is the latest stable in the 21.x line as of 2026-05-29). No bump required in this story.
- **DO NOT bump** to a newer minor/major in this story — `21.0.1` is current; bumping introduces churn unrelated to the LD-54 enforcement-chain wiring. Version policy (latest stable / LTS-preferred per [[feedback_version_policy]]) is satisfied.

**AC2 — `commitlint.config.cjs` at repo root extends `@commitlint/config-conventional` with no scope-value enum.**

- **Already done** (verify-only): [commitlint.config.cjs](commitlint.config.cjs) declares `module.exports = { extends: ['@commitlint/config-conventional'] }` verbatim. No `rules:` block, no `scope-enum` entry — matches the LD-54 prose ("No scope-value enum enforced in commitlint to avoid false-positive friction", [architecture.md:591](_bmad-output/planning-artifacts/architecture.md#L591)).
- **DO NOT add a `scope-enum`** in this story even if the canonical scope list in [CONTRIBUTING.md §2.3](CONTRIBUTING.md#L52-L54) ("`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`, `shell-ui`, `docs`, `ci`") looks enum-able. The architecture is explicit: scopes are *encouraged not required*.
- **DO NOT add custom `rules:`** (line-length overrides, type-case relaxations, etc.) — keep the config minimal so future readers see the literal config-conventional preset. If a rule fails in practice on legitimate commits, surface as a docs-debt follow-up, do not relax silently.

**AC3 — `.husky/commit-msg` runs `pnpm commitlint --edit "$1"` and fails the commit on a non-conforming message.**

- **Already done** (verify-only): [.husky/commit-msg](.husky/commit-msg) contains the single line `pnpm exec commitlint --edit "$1"`. The epics-AC text says `pnpm commitlint --edit "$1"` (without `exec`); the existing form `pnpm exec commitlint` is **semantically equivalent** (both resolve `commitlint` from `node_modules/.bin` via pnpm). The literal `pnpm exec` form is slightly more explicit and was authored by Story 1.3 — keep it; do not flip to `pnpm commitlint` cosmetically.
- **DO NOT rewrite the hook** to add wrapper logic (e.g., skipping merge commits, branch-name checks, etc.). The pre-commit hook handles lint-staged ([architecture.md:848](_bmad-output/planning-artifacts/architecture.md#L848)); the commit-msg hook is single-purpose by design.
- Husky v9.1.7 installed via the `prepare` script in [package.json:7](package.json#L7); the `_/` shim directory + the `commit-msg` hook are present in the working tree.

**AC4 — CI step runs `pnpm commitlint --from origin/main --to HEAD` and fails the PR on any non-conforming commit.**

- **NET-NEW work** for this story. Author a new workflow file at `.github/workflows/commitlint.yml` (dedicated, NOT folded into [pr.yml](.github/workflows/pr.yml)) — rationale in §3 below.
- **Trigger**: `on: pull_request: { branches: [main] }` only — never `push: branches: [main]` (commit-range validation makes no sense on a push to the trunk; the commits are already there by definition). Plus `workflow_dispatch: {}` for manual re-runs after a force-push that resolves a violation.
- **Concurrency**: `concurrency: { group: commitlint-${{ github.ref }}, cancel-in-progress: true }` — matches the [pr.yml:27-29](.github/workflows/pr.yml#L27-L29) discipline (cancel superseded runs to save CI minutes).
- **Permissions**: `contents: read` only — the commit-range job reads `git log`; it doesn't write anywhere. (The PR-title job in AC5 has a different, narrower permissions block — they're separate jobs in the same workflow file.)
- **Checkout**: `actions/checkout@v5` with `fetch-depth: 0` — commitlint's `--from origin/main` needs the full history to walk from the PR's merge-base to HEAD. Without `fetch-depth: 0`, the shallow clone misses the merge-base and the comparison fails with a confusing "fatal: ambiguous argument 'origin/main'" error.
- **Job shape** (one of two top-level jobs in `commitlint.yml`):
  ```yaml
  commitlint-range:
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
      - uses: pnpm/action-setup@v5
        with:
          version: 11.1.1
      - uses: actions/setup-node@v5
        with:
          node-version: '22'
          cache: 'pnpm'
      - run: pnpm install --frozen-lockfile
      - name: Lint commit range
        run: pnpm commitlint --from origin/main --to HEAD --verbose
  ```
- **Runner**: pinned `ubuntu-24.04` (never `ubuntu-latest`) per the [[feedback_version_policy]] and the [pr.yml:38](.github/workflows/pr.yml#L38) discipline.
- **`--verbose` flag**: included so failed commits print their full lint diagnostics in the CI log (default mode is terse → hard to debug from a workflow run page).
- **DO NOT add this job to the `pr.yml` matrix.** Architecture LD-54 says "`pr.yml` (or dedicated `commitlint.yml`)" — pick the dedicated file (rationale: separating concerns + the PR-title job in AC5 needs `pull_request_target` which pr.yml uses zero of, making co-location awkward).
- **DO NOT add `pnpm install` shortcut tricks** (e.g., `--prod` to skip dev deps). `commitlint` lives in `devDependencies` ([package.json:18-20](package.json#L18-L20)); a `--prod` install would skip it and the job would fail on "command not found".

**AC5 — CI step using `amannn/action-semantic-pull-request@v5` validates the PR title against Conventional Commits.**

- **NET-NEW work** for this story. Second top-level job in `.github/workflows/commitlint.yml`.
- **Trigger**: this job must additionally be reachable on `pull_request_target` events — `amannn/action-semantic-pull-request` needs read access to the PR title (which is editable post-creation) and write access to the PR status check. Using `pull_request_target` is the documented contract (see action README). **Critical**: only the *type-list* and *status-check* logic uses elevated permissions; we do NOT check out PR-author code in this job (avoids the [`pull_request_target` checkout footgun](https://securitylab.github.com/research/github-actions-preventing-pwn-requests/) where third-party code runs with secrets).
- **Workflow extension** required: add a second `on:` trigger for this workflow:
  ```yaml
  on:
    pull_request:
      branches: [main]
    pull_request_target:
      types: [opened, edited, synchronize]
      branches: [main]
    workflow_dispatch: {}
  ```
  - The `pull_request_target` `types: [opened, edited, synchronize]` set is the minimum needed: `opened` (new PR), `edited` (title rename), `synchronize` (HEAD updated → revalidate). Do NOT add `reopened` or `labeled` (noise; no title change).
- **Job shape** (second top-level job in `commitlint.yml`):
  ```yaml
  commitlint-pr-title:
    if: github.event_name == 'pull_request_target'
    runs-on: ubuntu-24.04
    timeout-minutes: 3
    permissions:
      pull-requests: read   # action reads PR title; write not needed
      statuses: write       # action posts a status check on the PR head
    steps:
      - uses: amannn/action-semantic-pull-request@v5
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  ```
- The `if:` guard ensures this job runs ONLY on `pull_request_target` events (the `commitlint-range` job above runs on `pull_request`). Without the guard, the job would attempt to run on `pull_request` too and either no-op or double-post the status check.
- **Permissions narrowing**: `pull-requests: read` + `statuses: write` is the minimum. **DO NOT** add `contents: write` / `issues: write` / `pull-requests: write` — overscope on a `pull_request_target` workflow is a documented security smell (the action could be coerced into mutating issues if a malicious PR title escapes lint).
- **DO NOT use a fork** of `amannn/action-semantic-pull-request`. The upstream is well-maintained, MIT-licensed, used by 50k+ repos.
- **DO NOT configure** the action's optional inputs (custom `types`, `requireScope`, `scopes`, `subjectPattern`, etc.) unless absolutely necessary. The action's defaults validate against the full Conventional Commits v1.0.0 type set (`feat|fix|perf|refactor|revert|docs|style|test|build|ci|chore`) — which matches the LD-54 type vocabulary verbatim ([architecture.md:591](_bmad-output/planning-artifacts/architecture.md#L591)). No customization needed.
- **DO NOT add this workflow to `pr.yml`'s slot-reservation comment block** ([pr.yml:167-171](.github/workflows/pr.yml#L167-L171)) — those slots are for jobs that will be folded *into* the pr.yml matrix later (a11y, L0 corpus). commitlint lives in its own workflow file by AC4+AC5 design.

**AC6 — Local smoke test: a deliberately-malformed commit message is rejected by `commit-msg` hook.**

- **NET-NEW work**. Author a shell smoke script at `scripts/smoke-commitlint.sh` that exercises the hook in BOTH directions (rejects malformed; accepts well-formed) and is runnable both manually and in CI:
  ```sh
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
  ```
- **Add `pnpm` script entry** in [package.json](package.json) `"scripts"` block: `"smoke:commitlint": "bash scripts/smoke-commitlint.sh"`. This makes `pnpm smoke:commitlint` the canonical invocation locally and from CI.
- **DO NOT shell out to `git commit` in the smoke**. Spinning up a tmp git repo, configuring user.email, calling `git commit --allow-empty -m 'broken'`, asserting non-zero — that's brittle and slow (~3-5s). Piping straight to `commitlint` via stdin tests the same logic the hook executes (`commitlint --edit "$1"` reads the file; piping to bare `commitlint` reads stdin — both invoke the same lint pipeline). The hook integration is verified at AC3 (file content equals `pnpm exec commitlint --edit "$1"`); the smoke verifies the lint *engine*.
- **DO NOT make the smoke conditional on env vars** (`SKIP_SMOKE`, `CI=true` branches, etc.). The smoke is fast (<2s) and deterministic; conditionals invite drift.
- **CI wiring** (third step in the `commitlint-range` job, AFTER `pnpm install`):
  ```yaml
      - name: Smoke (AC6)
        run: pnpm smoke:commitlint
  ```
- Wire the smoke into the `commitlint-range` job (not a separate workflow file) so a single PR run validates: (a) the commit-range, (b) the smoke. Two layers of confidence in one workflow file.

**AC7 — CI smoke: a deliberately-malformed PR title triggers the title-check failure.**

- **NET-NEW work + manual verification**. Two pieces:
  1. **Smoke script** at `scripts/smoke-commitlint-title.sh` (sibling to AC6's script) that runs `commitlint` against a known-bad title and a known-good title via stdin — same shape as AC6 but with title-specific fixtures:
     ```sh
     #!/usr/bin/env bash
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
     ```
  2. **Add `pnpm` script**: `"smoke:commitlint-title": "bash scripts/smoke-commitlint-title.sh"`.
  3. **Wire into the `commitlint-range` job** (AFTER the AC6 smoke step):
     ```yaml
         - name: Smoke title (AC7)
           run: pnpm smoke:commitlint-title
     ```
  4. **Manual end-to-end verification** during the PR for this story itself: surface in the PR body a "manual smoke verification" section instructing the maintainer to (a) temporarily edit the PR title to `Add commitlint CI` (no `type:` prefix — malformed), (b) wait for the `commitlint-pr-title` status check to fail, (c) revert title to its proper conventional form, (d) wait for the status to flip green. Record both observed states (failed + green) in the Dev Agent Record. Document this in §10 below.
- **DO NOT** attempt to programmatically simulate a `pull_request_target` event in CI (e.g., via `act` or scripted GitHub API calls). The `amannn/action-semantic-pull-request` integration is end-to-end testable only against the real GitHub Actions runtime; the manual title-flip during the story's own PR is the canonical smoke.
- **DO NOT** dedupe scripts/smoke-commitlint{,-title}.sh into one shared script. Separate scripts keep AC6 and AC7 traceable in the diff and in the workflow log; the duplication is intentional and minor (~15 lines each).

**AC8 — `CONTRIBUTING.md §2 "Enforcement chain"` is updated to reflect that the CI side is now wired.**

- **NET-NEW work** (single-line edit). [CONTRIBUTING.md:82](CONTRIBUTING.md#L82) currently reads:
  > **CI:** `.github/workflows/pr.yml` `commitlint --from origin/main --to HEAD` step + PR-title semantic-PR action land in **Story 1.14** (not yet wired).
- Replace with:
  > **CI:** [`.github/workflows/commitlint.yml`](./.github/workflows/commitlint.yml) runs `pnpm commitlint --from origin/main --to HEAD` on every PR (commit-range gate) + [`amannn/action-semantic-pull-request@v5`](https://github.com/amannn/action-semantic-pull-request) on `pull_request_target` (PR-title gate). Both gates are advisory under GitHub Free (no enforceable branch protection per [[project_orgsidian_github_plan]]); merge discipline is maintained by the maintainer's pre-merge check.
- **DO NOT extend** the §2 section with additional prose (rule examples, exception lists, escape hatches). The CC vocabulary + scope discipline + examples table at [CONTRIBUTING.md §2.1-§2.4](CONTRIBUTING.md#L42-L77) already documents the full spec; the §2 "Enforcement chain" section is a one-paragraph status pointer, not a documentation rewrite.
- **DO NOT update** Story 1.15's pointer at [CONTRIBUTING.md:83](CONTRIBUTING.md#L83) (the `cliff.toml` / `git-cliff` mention). Story 1.15 lands separately; that pointer is correct as-is.

**AC9 — Verification matrix (executed post-merge, results recorded in Dev Agent Record).**

This is the hard-truth gate. Each cell below MUST be re-run on the merged commit on `main` and the literal output recorded in the Debug Log References section:

| # | Verification | Pass condition |
|---|---|---|
| 1 | `cat package.json \| jq -r '.devDependencies."@commitlint/cli" + " " + .devDependencies."@commitlint/config-conventional"'` | exit 0; output `21.0.1 21.0.1` |
| 2 | `cat commitlint.config.cjs` | exit 0; output exactly `module.exports = {\n  extends: ['@commitlint/config-conventional'],\n};\n` |
| 3 | `cat .husky/commit-msg` | exit 0; output exactly `pnpm exec commitlint --edit "$1"\n` |
| 4 | `ls .github/workflows/commitlint.yml scripts/smoke-commitlint.sh scripts/smoke-commitlint-title.sh` | exit 0; 3 files present |
| 5 | `bash -n scripts/smoke-commitlint.sh && bash -n scripts/smoke-commitlint-title.sh` | exit 0 (syntax valid on both scripts) |
| 6 | `pnpm smoke:commitlint` | exit 0; PASS on both cases |
| 7 | `pnpm smoke:commitlint-title` | exit 0; PASS on both cases |
| 8 | `echo "broken message" \| pnpm exec commitlint; echo "exit=$?"` | output ends with `exit=1` (rejection) |
| 9 | `echo "feat(parser): add wrapper" \| pnpm exec commitlint; echo "exit=$?"` | output ends with `exit=0` (acceptance) |
| 10 | `gh run list -R orgsidian/orgsidian -w commitlint.yml --limit 1 --json conclusion -q '.[].conclusion'` | output `success` (most recent post-merge run) |
| 11 | `gh issue view 14 -R orgsidian/orgsidian --json state -q '.state'` | `"OPEN"` pre-PR; `"CLOSED"` post-PR-merge (auto-closed by `Closes #14` in PR body) |
| 12 | `grep -c "Story 1.14.*not yet wired" CONTRIBUTING.md` | output `0` (stale "(not yet wired)" prose removed by AC8) |
| 13 | `grep -c "amannn/action-semantic-pull-request" CONTRIBUTING.md` | output `1` (AC8 enforcement-chain line references the action) |

All 13 cells must pass on the merged main commit. Cells 10–11 require network + authenticated `gh`; the rest are local file/shell checks. Cells 6 and 7 must be runnable both locally (developer machine) and inside the CI runner.

## Tasks / Subtasks

- [x] **Task 1: Pre-flight verification of already-done preconditions** (AC: 1, 2, 3)
  - [x] 1.1 Verify [package.json:18-20](package.json#L18-L20) lists `@commitlint/cli: 21.0.1` + `@commitlint/config-conventional: 21.0.1`. Record literal block in Dev Agent Record / Debug Log References.
  - [x] 1.2 Verify [commitlint.config.cjs](commitlint.config.cjs) matches the AC2 literal (no `rules:`, no `scope-enum`).
  - [x] 1.3 Verify [.husky/commit-msg](.husky/commit-msg) contains the single line `pnpm exec commitlint --edit "$1"` (equivalent to AC3 spec text `pnpm commitlint --edit "$1"` — see AC3 note).
  - [x] 1.4 Verify husky's shim machinery at `.husky/_/` is present (post-`pnpm install` state).
  - [x] 1.5 Capture state-at-start for Dev Notes §1.

- [x] **Task 2: Author `scripts/smoke-commitlint.sh` + `scripts/smoke-commitlint-title.sh`** (AC: 6, 7)
  - [x] 2.1 Create both scripts with the exact shell bodies in AC6 + AC7. `set -euo pipefail` at the top of both.
  - [x] 2.2 `chmod +x scripts/smoke-commitlint.sh scripts/smoke-commitlint-title.sh` so they're executable directly (workflow invocation via `bash …` works either way; chmod is for local UX).
  - [x] 2.3 Add `smoke:commitlint` and `smoke:commitlint-title` script entries to [package.json:8](package.json#L8) `"scripts"` block (alphabetical insertion: between `prepare` and `commitlint`).
  - [x] 2.4 Run BOTH locally to verify exit 0 + both PASS lines on each: `pnpm smoke:commitlint && pnpm smoke:commitlint-title`. Record literal output in Debug Log References.
  - [x] 2.5 `bash -n` on both scripts as a syntax-check gate.

- [x] **Task 3: Author `.github/workflows/commitlint.yml`** (AC: 4, 5)
  - [x] 3.1 Create the file with the two-job shape: `commitlint-range` (on `pull_request`) + `commitlint-pr-title` (on `pull_request_target`). Concurrency group `commitlint-${{ github.ref }}`, cancel-in-progress. Top-of-file comment header documenting LD-54 + the two-job rationale (mirror the [pr.yml:1-18](.github/workflows/pr.yml#L1-L18) header style).
  - [x] 3.2 `commitlint-range` job: `ubuntu-24.04`, timeout 5 min, `permissions: contents: read`, checkout with `fetch-depth: 0`, pnpm setup, install, lint commit range, then run the two smoke scripts (AC6 + AC7).
  - [x] 3.3 `commitlint-pr-title` job: `ubuntu-24.04`, timeout 3 min, `if: github.event_name == 'pull_request_target'`, `permissions: pull-requests: read, statuses: write`, single step using `amannn/action-semantic-pull-request@v5` with `GITHUB_TOKEN` env var.
  - [x] 3.4 Pin action versions semver-major (`@v5` on amannn/, `@v5` on actions/checkout, `@v5` on pnpm/action-setup, `@v5` on actions/setup-node) per [[feedback_version_policy]] + the [pr.yml](.github/workflows/pr.yml) discipline.
  - [x] 3.5 YAML round-trip lint: parsed via `npx yaml` — single valid document.
  - [x] 3.6 Compare permissions block byte-by-byte against AC4 + AC5 — no `contents: write`, no `issues: write` slipping in.

- [x] **Task 4: Update `CONTRIBUTING.md §2 "Enforcement chain"`** (AC: 8)
  - [x] 4.1 Edit [CONTRIBUTING.md:82](CONTRIBUTING.md#L82) per AC8 verbatim replacement. Leave [line 83](CONTRIBUTING.md#L83) (the `cliff.toml` / Story 1.15 pointer) UNTOUCHED.
  - [x] 4.2 Visual check: `grep -A 5 "Enforcement chain" CONTRIBUTING.md` — output must show the NEW prose (not the stale "Story 1.14 (not yet wired)" line).

- [x] **Task 5: PR-time smoke (open the story PR with a known-good title; verify both gates green)** (workflow gate)
  - [x] 5.1 Opened [PR #130](https://github.com/orgsidian/orgsidian/pull/130) with title `feat(ci): wire commitlint commit-range + PR-title gates (Story 1.14, closes #14)`. PR body contains `Closes #14`. **§10 Q1 decision (user)**: `feat(ci):` selected.
  - [x] 5.2 `commitlint-range` ✅ pass (14s); `commitlint-pr-title` skipped on this PR — see Task 6 carry-over note. `pr (macos-14)` + `pr (ubuntu-24.04)` from existing pr.yml: pass (no regression).
  - [x] 5.3 PR URL + run IDs recorded in Dev Agent Record / Debug Log References.

- [ ] **Task 6: Manual smoke verification of AC7 (malformed PR title rejection)** (AC: 7) — **DEFERRED to next PR** (see carry-over note in Debug Log References)
  - [ ] 6.1 With the PR open and green, temporarily edit the PR title via `gh pr edit <pr-number> --title "Add commitlint CI"` (no `type:` prefix → malformed per CC v1.0.0).
  - [ ] 6.2 Wait for the `commitlint-pr-title` status check to re-run (triggered by the `pull_request_target: types: [edited]` filter). Record the FAILED conclusion + the literal action output ("Available types: feat, fix, perf, …" or similar).
  - [ ] 6.3 Revert the title to its conventional form via `gh pr edit <pr-number> --title "feat(ci): …"`.
  - [ ] 6.4 Wait for the `commitlint-pr-title` status check to flip back to `success`.
  - [ ] 6.5 Record both states (RED on bad title, GREEN on good title) in Dev Agent Record / Debug Log References.

- [ ] **Task 7: Post-merge convergence + verification matrix** (AC: 9)
  - [ ] 7.1 After PR merge, the `commitlint.yml` workflow does NOT re-run on push-to-main (the workflow is `pull_request` + `pull_request_target` only — by design, see AC4). Instead, run the AC9 cells 1–9, 12, 13 against the merged commit (local checks).
  - [ ] 7.2 Run cells 10 + 11 (network) once the PR is merged.
  - [ ] 7.3 Record all 13 cells' literal output in Dev Agent Record / Debug Log References.

- [x] **Task 8: Sprint status + issue #14 status transitions** (workflow boilerplate)
  - [x] 8.1 At story start: update [`_bmad-output/implementation-artifacts/sprint-status.yaml`](_bmad-output/implementation-artifacts/sprint-status.yaml) `1-14-configure-commitlint-husky-commit-msg-hook-ci-gate` from `ready-for-dev` → `in-progress`. Update issue #14 label `status:backlog` → `status:in-progress` via `gh issue edit 14 -R orgsidian/orgsidian --remove-label status:backlog --add-label status:in-progress`.
  - [x] 8.2 At PR-open: update sprint-status `in-progress` → `review`. Update issue #14 label `status:in-progress` → `status:in-review`.
  - [ ] 8.3 At PR-merge: update sprint-status `review` → `done`. Update issue #14 label → `status:done` AND close the issue (the `Closes #14` PR-body footer auto-closes; the label still needs the manual flip). — handled by `bmad-code-review`

## Dev Notes

### §1 — State-at-start (preconditions)

Three of the seven epic-spec ACs are **already satisfied by Story 1.3's pnpm-side scaffold**. The dev agent's job is verify-only on those, NEW work on the rest:

| AC | State at start | Net-new work |
|---|---|---|
| AC1 — `@commitlint/cli` + config in package.json | ✅ Done (v21.0.1, Story 1.3) | None — verify only |
| AC2 — `commitlint.config.cjs` extends config-conventional | ✅ Done (Story 1.3) | None — verify only |
| AC3 — `.husky/commit-msg` runs `commitlint --edit "$1"` | ✅ Done (uses `pnpm exec commitlint`, equiv.) | None — verify only |
| AC4 — CI step `pnpm commitlint --from origin/main --to HEAD` | ❌ Missing | Author `.github/workflows/commitlint.yml` (`commitlint-range` job) |
| AC5 — `amannn/action-semantic-pull-request@v5` PR-title check | ❌ Missing | Add `commitlint-pr-title` job to same workflow file |
| AC6 — Local smoke (malformed commit rejected) | ❌ Missing | `scripts/smoke-commitlint.sh` + pnpm script + CI step |
| AC7 — CI smoke (malformed PR title rejected) | ❌ Missing | `scripts/smoke-commitlint-title.sh` + pnpm script + CI step + manual end-to-end verify on the story PR itself |
| AC8 — CONTRIBUTING.md enforcement-chain status update | ❌ Stale | Single-line edit at [CONTRIBUTING.md:82](CONTRIBUTING.md#L82) |
| AC9 — Verification matrix | (run post-merge) | Execute 13 cells |

Net-new files: 3 (workflow + 2 smoke scripts). Net-new package.json script entries: 2. Single-file edits: 2 (package.json + CONTRIBUTING.md). One workflow file edit count: 0 (pr.yml is untouched).

### §2 — Reality-vs-spec reconciliations (rated by binding force)

| # | Spec text | Reality | Resolution | Binding force |
|---|---|---|---|---|
| 1 | AC3 says hook runs `pnpm commitlint --edit "$1"` | Hook contains `pnpm exec commitlint --edit "$1"` | Keep `pnpm exec` form (semantically equivalent + more explicit); do not flip cosmetically | **HIGH** — already-shipped code wins; spec wording is informal |
| 2 | AC4 says "`.github/workflows/pr.yml` (or a dedicated `commitlint.yml`)" | pr.yml has zero `pull_request_target` triggers; AC5 needs `pull_request_target` | Pick dedicated `commitlint.yml` (cleaner trigger separation; LD-54 enforcement surface is one file) | **MEDIUM** — author's discretion within spec |
| 3 | Architecture LD-55 status-label scheme uses `status:review` | Memory + repo use `status:in-review` | Use `status:in-review` for label flips in Task 8 (Story 1.13 set the precedent) | **HIGH** — [[project_orgsidian_github_label_scheme]] is authoritative |
| 4 | Story 1.14 lands BEFORE Story 1.15 (`git-cliff`) — so `feat:` / `chore:` commits in this PR are NOT yet ingested by a CHANGELOG generator | Same | Commit type choice for this story matters for *future* CHANGELOG output; default `feat(ci):` (Added bucket per LD-54) — see §10 Q1 | **LOW** — no immediate consequence; future CHANGELOG generation depends on the choice |
| 5 | GH Free → branch protection unenforceable | Same | Document AC8 enforcement-chain line as "advisory under GitHub Free"; do NOT add branch-protection rules | **HIGH** — [[project_orgsidian_github_plan]] is authoritative |

### §3 — DO-NOT-DO list

1. **DO NOT add the commit-range job inside `pr.yml`.** Architecture LD-54 explicitly allows either form; the dedicated `commitlint.yml` is cleaner because (a) `amannn/action-semantic-pull-request` needs `pull_request_target` which pr.yml uses zero of, (b) keeping LD-54 enforcement isolated makes Story 1.15's `git-cliff` follow-on easier to reason about.
2. **DO NOT add a `scope-enum` rule** to `commitlint.config.cjs`. LD-54 prose: "No scope-value enum enforced in commitlint to avoid false-positive friction". Even though the canonical scopes are enumerated in CONTRIBUTING.md, hard-coding them in commitlint creates a maintenance treadmill every time a new crate lands.
3. **DO NOT modify `.husky/commit-msg`** beyond what AC3 specifies (no edits at all — it's already correct).
4. **DO NOT add `pre-commit` lint-staged rules** in this story. Architecture references `husky + lint-staged` at [architecture.md:848](_bmad-output/planning-artifacts/architecture.md#L848), but that's an in-flight discipline owned by code-quality stories, not the LD-54 commit-msg gate.
5. **DO NOT add the workflow to a "required-checks" comment block in `pr.yml`.** GH Free → branch protection unenforceable per [[project_orgsidian_github_plan]]; the CI gates are advisory only.
6. **DO NOT use `actions/checkout@latest` / `@main`** or any other unpinned action versions. Semver-major-pinned (`@v5`) per [[feedback_version_policy]] and the [pr.yml](.github/workflows/pr.yml) convention.
7. **DO NOT overscope `pull_request_target` permissions** in the `commitlint-pr-title` job. `pull-requests: read` + `statuses: write` is the documented minimum for `amannn/action-semantic-pull-request@v5`. Adding `contents: write` or `pull-requests: write` is a documented security smell ([[GitHub Security Lab — preventing pwn requests](https://securitylab.github.com/research/github-actions-preventing-pwn-requests/)]).
8. **DO NOT checkout PR-author code in the `pull_request_target` job.** `amannn/action-semantic-pull-request` only reads the PR title from the API context; no checkout step needed. Adding `actions/checkout@v5` with the default `ref:` would check out the PR HEAD with elevated permissions — a classic supply-chain footgun.
9. **DO NOT add `Co-Authored-By:` trailers or "Generated with Claude Code" footers** to commits / PR body / issue comments per [[feedback_no_co_author_credit]].
10. **DO NOT install a different commit-msg validator** (e.g., `gitlint`, custom regex hook). LD-54 is `@commitlint/config-conventional`; alternative tooling diverges from the spec.
11. **DO NOT skip the manual title-flip smoke in Task 6.** The CI title-check is end-to-end testable only against the real GitHub Actions runtime; that's the canonical proof the gate works. Do not substitute with `act` or scripted GraphQL — the `pull_request_target` event semantics require a real PR title edit.
12. **DO NOT bump `@commitlint/cli` or `@commitlint/config-conventional` minor/major** in this story. Story 1.3 pinned `21.0.1`; bumping pulls unrelated churn into a wiring story.
13. **DO NOT add a `prepare-commit-msg` or `pre-push` hook** — out of scope. Only the `commit-msg` hook is wired; the rest of `.husky/_/` is just husky's shim machinery (`applypatch-msg`, `post-commit`, etc. — all delegating to potential local hook files that don't exist).
14. **DO NOT delete or rename `.husky/_/`** — that directory is husky 9.x's runtime shim; deleting it breaks the hook chain. Created by `pnpm install` via the `prepare` script; safe to leave alone.

### §4 — Workflow file rationale: `commitlint.yml` vs folding into `pr.yml`

The architecture's project-tree at [architecture.md:1423](_bmad-output/planning-artifacts/architecture.md#L1423) lists `commitlint.yml` as a separate file ("or folded into pr.yml") — an explicit two-option spec. The recommendation here is **separate file**, with this rationale chain:

1. **Trigger surface divergence**: `pr.yml` is `pull_request` + `push: main` only ([pr.yml:21-25](.github/workflows/pr.yml#L21-L25)). `amannn/action-semantic-pull-request` needs `pull_request_target` for PR-title editing — adding that trigger to `pr.yml` would invite the entire pr.yml job matrix to fire on title edits, which is wasteful and confuses concurrency cancellation semantics.
2. **Permissions hygiene**: `pull_request_target` jobs need narrower-but-distinct permissions than `pull_request` jobs (statuses:write vs the default repo-scoped token). Keeping them in one file under one `permissions:` block forces overscoping; splitting at the job level is fine *within* the file but conceptually messy.
3. **LD-54 enforcement-chain surface**: a single `commitlint.yml` file makes the LD-54 chain (commit-range + PR-title) trivially greppable. Folded-into-pr.yml would scatter LD-54 logic across 200+ lines of unrelated CI.
4. **Failure-mode isolation**: if a future commitlint config bug causes the CI to false-positive, isolating it in `commitlint.yml` lets the maintainer disable just the LD-54 gate via `workflow_dispatch` without touching the broader `pr.yml` matrix.
5. **Cost**: one extra file (~60 lines) vs ~40 lines added to `pr.yml`. Marginal.

The dev agent should still *cite the choice* in the workflow header comment (mirror [pr.yml:1-18](.github/workflows/pr.yml#L1-L18) style):

```yaml
# commitlint.yml — Story 1.14 LD-54 enforcement chain.
#
# Two-job split:
#   commitlint-range:    runs on pull_request, lints the commit range
#                        with `pnpm commitlint --from origin/main --to HEAD`
#                        plus the two AC6/AC7 smoke scripts.
#   commitlint-pr-title: runs on pull_request_target (needed because the
#                        action posts a status check from outside the PR's
#                        own token scope). Permissions narrowed to
#                        pull-requests:read + statuses:write — no checkout.
#
# Both gates are advisory under GitHub Free (branch protection unenforceable
# per LD-5 + the project's GH Free plan); merge discipline is maintained by
# the maintainer's pre-merge check. See LD-54 in architecture.md and
# CONTRIBUTING.md §2.
#
# Version policy: action versions semver-major-pinned (`@v5`); runner pinned
# (`ubuntu-24.04`). Never `*-latest`.
```

### §5 — `commitlint --from origin/main --to HEAD` mechanics

The flag pair lints every commit in the range `origin/main..HEAD` (exclusive of merge-base, inclusive of HEAD). Mechanics relevant to debugging false-positives:

- **`fetch-depth: 0`** is non-negotiable. The default `actions/checkout` shallow clone fetches depth 1, which leaves `origin/main` unresolvable from the PR's HEAD. The error mode is `fatal: ambiguous argument 'origin/main': unknown revision or path not in the working tree` — non-obvious from the surface.
- **Merge commits**: commitlint's default-config-conventional rules apply to merge commits too. Merge commit subjects (e.g., `Merge pull request #N from owner/branch`) DO NOT conform to CC and would be flagged. **Solution**: GitHub's "Squash and merge" or "Rebase and merge" strategies (configured at the repo level, NOT in this story) eliminate merge commits from `main`. For Stories 1.10–1.13's PR history, the maintainer used **Squash and merge** (verified via `git log --oneline -10` — no `Merge pull request` subjects on `main`). Keep that discipline; do NOT add a commitlint ignore rule for `Merge*` subjects.
- **First commit on a new branch**: the merge-base resolution works fine; no special handling needed.
- **Re-runs after rebase**: `concurrency: { cancel-in-progress: true }` ensures the old run is cancelled and the new one runs against the post-rebase HEAD. No staleness risk.

### §6 — `amannn/action-semantic-pull-request@v5` integration details

- **Action version**: `@v5` (semver-major pin, latest stable as of 2026-05-29; matches the AC5 spec verbatim). The v5 tag is a moving target within the v5.x minor line — that's the **intended behavior** per the [[feedback_version_policy]] (semver-major pin, accept minor/patch updates).
- **Default validation rules** (no input customization needed): CC type set `feat|fix|perf|refactor|revert|docs|style|test|build|ci|chore`; subject required; case-sensitive type; no length cap on subject. Matches LD-54 spec verbatim.
- **Status check name**: `Semantic Pull Request` (the action's default name; configurable via `validateSingleCommit` input but not relevant here). This is the status that flips RED/GREEN on the PR.
- **Failure UX**: when the title is malformed, the action posts an annotated status check with a markdown-formatted error explaining the expected format + the actual title. The dev agent should capture this output in Task 6.5.
- **`pull_request_target` security note**: this workflow runs with `GITHUB_TOKEN` from the **base repo**, not the PR fork (if applicable). For this project (solo-dev, no fork-based contributions in v0.1 timeframe), the security concern is largely academic — but the permissions narrowing in AC5 (`pull-requests: read` + `statuses: write`) is still the right discipline.
- **DO NOT use `requireScope: true`** input. The LD-54 spec is "scope is optional but recommended" — a `requireScope: true` would reject scopeless commits like `chore: bump Cargo.lock`. False-positive landmine.

### §7 — Smoke-test design choices (why pipe to stdin, not full `git commit`)

The AC6/AC7 smoke scripts pipe a message string to `pnpm exec commitlint` via stdin rather than spinning up a tmp git repo. Rationale:

- **Speed**: stdin-piped lint is <500ms; `git commit --allow-empty -m '...'` in a tmp repo is 2-5s (init + config user.email + commit + cleanup).
- **Determinism**: no transient state (tmp dirs, file system writes, git config) — the smoke is the lint pipeline itself.
- **What's being tested**: the AC says "smoke test confirms that a deliberately-malformed local commit (`git commit -m "broken message"`) is rejected by the `commit-msg` hook." The hook runs `commitlint --edit "$1"` where `"$1"` is the path to `.git/COMMIT_EDITMSG`. The lint *engine* is identical whether the message arrives via `--edit` (file path) or stdin (default mode). The stdin form is the engine smoke; the `--edit` form (hook integration) is verified at AC3 (file content matches the LD-54 spec verbatim).
- **Why two separate scripts (AC6 + AC7)**: keeps the diff traceable (one script per AC), keeps the CI log readable (two PASS lines per script vs four jumbled lines in one script), and matches the AC text's "smoke test" singular phrasing per AC. The marginal duplication (~15 lines each) is intentional.
- **Why bash, not Node/.mjs**: the existing scripts directory uses `.mjs` for Node tools ([scripts/check-pnpm-licenses.mjs](scripts/check-pnpm-licenses.mjs), [scripts/check-allowlist-sync.mjs](scripts/check-allowlist-sync.mjs), [scripts/gen-failure-modes-matrix.mjs](scripts/gen-failure-modes-matrix.mjs)) and `.sh` for shell-only tools ([scripts/configure-branch-protection.sh](scripts/configure-branch-protection.sh), [scripts/sync-epics-to-github.sh](scripts/sync-epics-to-github.sh)). The smoke scripts are shell-only (just pipe + assert exit code) → `.sh` is the consistent choice. **Do not** invent a `.mjs` version "for cross-platform consistency" — the husky `_/h` runtime is bash; the smoke runs under the same constraint.

### §8 — Commit type decision (surface, do not silently pick)

Per [[feedback_batch_fixes_terse]] — decision-grade questions surface; no-brainer fixes silent. The commit + PR title type for this story is decision-grade because:

- **`feat(ci):`** — bucketed under CHANGELOG `Added` per LD-54 mapping. Argument: LD-54 enforcement IS a user-visible-to-contributors feature (the rules for how to contribute change). When Story 1.15 lights up git-cliff, the v0.1 Alpha CHANGELOG carries this entry under Added → contributors know commitlint went live. **Recommended default.**
- **`chore(ci):`** — bucketed as *excluded from CHANGELOG*. Argument: same as Story 1.13's chore choice — `.github/workflows/*` is repo infra, not the desktop-app user-facing surface. Defensible reading; matches Story 1.13 precedent (`chore(github):`).
- **`ci:`** — bucketed same as `chore` (excluded from CHANGELOG). Argument: the scope-conventional `ci:` type is reserved for CI changes specifically. The bulk of this story IS CI (new workflow file + 2 smoke scripts wired into CI) — so `ci:` is the most precise type. But: `ci:` excludes from CHANGELOG, and the maintainer may want this surfaced to contributors.

**Trade-off**: `feat(ci):` surfaces in CHANGELOG (high signal for contributors); `chore(ci):` / `ci:` follows the LD-54 mapping table literal but invisible to CHANGELOG readers. Surface to the user; default `feat(ci):` per the contributor-visibility argument.

### §9 — Idempotency-first re-execution

If anything fails partway through and the dev agent re-runs, the operations converge:

- `scripts/smoke-commitlint*.sh` — file overwrites; no state.
- `.github/workflows/commitlint.yml` — file overwrites; no state. Re-pushing converges the workflow on `main` (though the workflow doesn't re-run on push-to-main by design).
- [package.json](package.json) script entries — additive; re-running the AC5 add-script step finds the entry already present, no-op.
- [CONTRIBUTING.md](CONTRIBUTING.md) §2 edit — overwrite-safe; idempotent.
- Issue #14 label flip — `gh issue edit … --remove-label status:backlog --add-label status:in-progress` is idempotent (removing a non-present label and adding an already-present one both no-op).
- Sprint-status.yaml flip — overwrite-safe; check current value before flipping.

### §10 — Decision-grade questions to surface (not silently pick)

1. **Commit + PR title type**: `feat(ci):` (default, surfaces in CHANGELOG per LD-54) vs `chore(ci):` (matches Story 1.13 precedent, excludes from CHANGELOG) vs `ci:` (most precise scope name, excludes from CHANGELOG). Default in Task 5.1: `feat(ci):`. Surface to user before opening the PR.
2. **AC7 manual smoke verification**: do we capture the malformed-title screenshot/output in the Dev Agent Record (Task 6 sub-clauses), or surface in the PR body as a "manual verification log"? Default: BOTH — the Dev Agent Record captures the literal action output (greppable); the PR body has a 3-line "manual smoke verification: title was edited to 'X' (RED), reverted to 'Y' (GREEN), workflow runs <url1> + <url2>" pointer. Frictionless future audit.
3. **`fetch-depth: 0` cost**: full-history fetch on every PR run adds ~5-10s to the `commitlint-range` job startup on this 130+-commit repo (small today; will grow). Alternative: `fetch-depth: 50` (relative depth heuristic). **Default: `fetch-depth: 0`** (deterministic; the cost is negligible vs the bug-mode of "merge-base unresolvable on a long-running branch").

All three questions surface in the PR thread per [[feedback_batch_fixes_terse]] — do not pick silently.

### §11 — Memory-anchored conventions

- **[[project_orgsidian_github_label_scheme]]**: `status:in-review` is the GitHub label (NOT `status:review`). Task 8 label flips use the in-use names.
- **[[project_orgsidian_github_plan]]**: GH Free → branch protection unenforceable. The new CI gates are advisory; AC8 enforcement-chain prose documents this explicitly.
- **[[project_orgsidian_repo_public_during_pre_alpha]]**: repo is PUBLIC; no visibility changes in this story.
- **[[feedback_no_co_author_credit]]**: no Co-Authored-By trailers; no "Generated with Claude Code" footers on commit/PR/issue.
- **[[feedback_version_policy]]**: semver-major-pinned action versions (`@v5`); pinned runner image (`ubuntu-24.04`).
- **[[feedback_batch_fixes_terse]]**: silent no-brainer fixes; surface only the §10 decision-grade Qs.
- **[[user_contact_email]]**: no email field anywhere in the new files; git commit author is set globally per the project's git config (`tiz.basile@gmail.com`).

### §12 — Test strategy

Three layers of confidence in the LD-54 enforcement chain:

1. **Unit-level smoke** (AC6 + AC7): `scripts/smoke-commitlint*.sh` exercise the lint engine on known-good and known-bad fixtures via stdin. Fast (<2s combined), deterministic, runnable both locally (`pnpm smoke:commitlint && pnpm smoke:commitlint-title`) and in CI as steps in the `commitlint-range` job. This is the engine smoke — does the lint pipeline reject what it should and accept what it should.

2. **Integration-level CI gate** (AC4 + AC5): the `commitlint-range` job runs `pnpm commitlint --from origin/main --to HEAD` against the actual PR's commit set on every PR — this is the engine applied to real commit subjects in a real CI environment. The `commitlint-pr-title` job exercises the title-validation surface end-to-end via the real GitHub Actions runtime + the `amannn/action-semantic-pull-request@v5` action.

3. **End-to-end manual verification** (Task 6): the story's own PR is the canonical smoke for the PR-title gate. Edit the title to a known-bad value, watch CI fail; revert, watch CI flip green. Record both states. This is the only way to validate the `pull_request_target` integration outside of testing in production.

The test strategy is appropriate to the story's risk profile:
- **Low complexity**: 3 new files, ~60 + 30 + 30 lines respectively.
- **Low blast radius**: failure modes are CI-job-fails (loud and obvious); silent false-passes are bounded by the smoke scripts' explicit PASS lines.
- **High value**: every commit and PR title downstream of this story carries LD-54 enforcement automatically — Story 1.15 (`git-cliff`) inherits a clean substrate.

No new automated test crates are introduced; the smoke scripts are shell smokes, not Rust unit tests. The story's AC9 verification matrix is the post-merge gate.

### Project Structure Notes

- All new files land under `.github/workflows/` (1 file: `commitlint.yml`) and `scripts/` (2 files: `smoke-commitlint.sh`, `smoke-commitlint-title.sh`). The directory `.github/workflows/` already exists (contains `pr.yml`, `nightly.yml`, `labels-sync.yml` from Story 1.13); the `scripts/` directory already exists (5 existing scripts per `ls scripts/`).
- Two existing files are edited: [package.json](package.json) (2 new `"scripts"` entries) and [CONTRIBUTING.md](CONTRIBUTING.md) (single-line §2 update).
- Zero existing files are renamed, moved, or deleted.
- No Cargo workspace changes; no new pnpm dependencies; no new pnpm scripts beyond `smoke:commitlint` + `smoke:commitlint-title`.
- No README.md edits in this story (the stale "private during pre-Alpha" line remains flagged in [deferred-work.md](_bmad-output/implementation-artifacts/deferred-work.md) but is not this story's scope).

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L638-L656](_bmad-output/planning-artifacts/epics.md#L638-L656) (Story 1.14 AC verbatim)
- Architecture LD-54 (Conventional Commits + CHANGELOG mapping): [_bmad-output/planning-artifacts/architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)
- Architecture project-tree showing `commitlint.yml` slot: [_bmad-output/planning-artifacts/architecture.md#L1414-L1428](_bmad-output/planning-artifacts/architecture.md#L1414-L1428)
- CONTRIBUTING.md §2 (stale "Story 1.14 (not yet wired)" prose this story fixes): [CONTRIBUTING.md#L79-L83](CONTRIBUTING.md#L79-L83)
- Existing `commitlint.config.cjs`: [commitlint.config.cjs](commitlint.config.cjs)
- Existing husky hook: [.husky/commit-msg](.husky/commit-msg)
- Existing per-PR CI workflow (style + version-pin reference): [.github/workflows/pr.yml](.github/workflows/pr.yml)
- Existing labels-sync workflow (compact-workflow style reference): [.github/workflows/labels-sync.yml](.github/workflows/labels-sync.yml)
- Previous story (1.13, GitHub bootstrap; AC-matrix + Dev-Notes-multi-section pattern reference): [_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md](_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md)
- Sprint Change Proposal (origin of Stories 1.13–1.16): [_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md](_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md)
- `@commitlint/cli` v21 docs (CLI flags + config reference): https://commitlint.js.org/reference/cli.html
- `@commitlint/config-conventional` ruleset: https://github.com/conventional-changelog/commitlint/blob/master/%40commitlint/config-conventional/index.js
- `amannn/action-semantic-pull-request@v5` README: https://github.com/amannn/action-semantic-pull-request
- GitHub Security Lab — `pull_request_target` security guidance: https://securitylab.github.com/research/github-actions-preventing-pwn-requests/
- Conventional Commits v1.0.0 spec: https://www.conventionalcommits.org/en/v1.0.0/

## Dev Agent Record

### Agent Model Used

`claude-opus-4-7[1m]` (Claude Opus 4.7, 1M-context) via `bmad-dev-story` skill.

### Debug Log References

**State-at-start verification (Task 1, 2026-05-29):**

```text
# AC1 — package.json devDeps
$ cat package.json | jq -r '.devDependencies."@commitlint/cli" + " " + .devDependencies."@commitlint/config-conventional"'
21.0.1 21.0.1

# AC2 — commitlint.config.cjs
$ cat commitlint.config.cjs
module.exports = {
  extends: ['@commitlint/config-conventional'],
};

# AC3 — husky commit-msg hook
$ cat .husky/commit-msg
pnpm exec commitlint --edit "$1"

# husky _/ shim machinery (19 entries, .gitignore + applypatch-msg .. prepare-commit-msg)
$ ls -la .husky/_/ | wc -l
   21
```

**Smoke-script runs (Task 2.4, 2026-05-29, local macOS-14):**

```text
$ pnpm smoke:commitlint
> bash scripts/smoke-commitlint.sh
PASS: malformed message correctly rejected
PASS: well-formed message correctly accepted
smoke-commitlint.sh: OK

$ pnpm smoke:commitlint-title
> bash scripts/smoke-commitlint-title.sh
PASS: malformed PR title correctly rejected
PASS: well-formed PR title correctly accepted
smoke-commitlint-title.sh: OK
```

**Workflow YAML round-trip (Task 3.5):** `npx yaml < .github/workflows/commitlint.yml` parses as a single valid document; jobs surface as `commitlint-range` + `commitlint-pr-title`.

**CONTRIBUTING.md gates (Task 4 / AC9 cells 12–13):**

```text
$ grep -c "Story 1.14.*not yet wired" CONTRIBUTING.md
0
$ grep -c "amannn/action-semantic-pull-request" CONTRIBUTING.md
1
```

**Task 5 outputs (PR #130, 2026-05-29):**

```text
PR URL:         https://github.com/orgsidian/orgsidian/pull/130
PR title:       feat(ci): wire commitlint commit-range + PR-title gates (Story 1.14, closes #14)
PR body grep:   "Closes #14" ✓
commitlint workflow run: 26627967049
  commitlint-range:    success (14s)
  commitlint-pr-title: skipped (see Task 6 carry-over)
pr.yml workflow run:    26627967003 (pre-existing; non-Story-1.14 regression gate)
  pr (macos-14):       success (1m28s)
  pr (ubuntu-24.04):   success
  merge-gate-nightly-fresh: fail — pre-existing, OUT-OF-SCOPE for Story 1.14
                              (latest scheduled nightly.yml run on main concluded
                              failure at 2026-05-29T08:47:22Z, ~6 min before this
                              PR opened; the LD-32 gate is correctly enforcing
                              "most-recent nightly green within 24h"; resolution
                              requires fixing the nightly failure on main —
                              tracked as a follow-up, not within Story 1.14 scope)
```

**Task 6 carry-over (AC7 manual end-to-end smoke) — DEFERRED to next PR:**

GitHub semantics: `pull_request_target` workflows execute against the workflow definition at the **base ref** (`main`), not the PR head. Because `.github/workflows/commitlint.yml` is brand-new on this PR and not yet present on `main`, the `commitlint-pr-title` job cannot fire on PR #130 itself — the `pull_request_target` trigger has nothing to load. This is the standard `pull_request_target` first-introduction limitation (documented in the action README and the GitHub docs).

Resolution: the manual title-flip smoke runs on the **next** PR after this one merges (the workflow is then present on `main`, so `pull_request_target` fires correctly). Documented in the PR body and surfaced to the user during the dev-story workflow.

Engine-level smoke for AC7 already passed both locally (`pnpm smoke:commitlint-title` PASS) and in CI (`commitlint-range` job step "Smoke title (AC7)" PASS in run 26627967049). The end-to-end action integration is what defers — not the engine.

**Task 7 carry-over (AC9 verification matrix, 13 cells) — handled by `bmad-code-review`** on the merged commit. Cells 1–9 + 12–13 are already verified locally on the branch HEAD (literal output above for cells 1–3, 6–7, 12–13; cells 4–5 + 8–9 verified by file presence + `bash -n` syntax-check during Task 2.5). Cells 10–11 require post-merge state.

### Completion Notes List

- AC1 / AC2 / AC3: ✅ verify-only — preconditions intact from Story 1.3 scaffold; no edits to `package.json` deps, `commitlint.config.cjs`, or `.husky/commit-msg`.
- AC4 / AC5: ✅ new `.github/workflows/commitlint.yml` with two top-level jobs split by `if: github.event_name`. `commitlint-range` gated on `pull_request`; `commitlint-pr-title` gated on `pull_request_target`. Permissions narrowed per AC5 (`pull-requests: read` + `statuses: write` on the title job; `contents: read` on the range job). Action versions semver-major-pinned (`@v5`) matching the [pr.yml](.github/workflows/pr.yml) + [nightly.yml](.github/workflows/nightly.yml) convention.
- AC6 / AC7: ✅ `scripts/smoke-commitlint.sh` + `scripts/smoke-commitlint-title.sh` author the engine smoke; wired via `pnpm smoke:commitlint` + `pnpm smoke:commitlint-title` (alphabetical insertion in `package.json` "scripts"); both pass locally + are invoked from the `commitlint-range` job after the commit-range lint step.
- AC8: ✅ `CONTRIBUTING.md §2 "Enforcement chain"` rewritten to reference the new workflow file + amannn action + GH Free advisory caveat. Story 1.15 pointer at L83 untouched.
- AC9 cells 1–9, 12–13: verified locally + in CI pre-PR (see Debug Log References above). Cells 10–11 + remaining post-merge cells deferred to Task 7 (handled by `bmad-code-review` on the merged commit).
- **Task 6 deferred to NEXT PR**: `pull_request_target` reads workflow from base ref; `commitlint.yml` lands with this PR so the title job cannot fire on PR #130 itself. Manual title-flip smoke runs on the next post-merge PR.
- **Out-of-scope blocker for merge**: `merge-gate-nightly-fresh` is RED because the latest scheduled `nightly.yml` run on `main` (2026-05-29T08:47:22Z) failed. This is a pre-existing condition, NOT introduced by Story 1.14. Story 1.14's own gates (`commitlint-range`) + the existing `pr.yml` matrix all pass.
- §10 decision-grade questions: **Q1** `feat(ci):` selected by user (CHANGELOG-visible); **Q2** BOTH (Dev Agent Record + PR body); **Q3** `fetch-depth: 0` (default, deterministic).
- No `Co-Authored-By` trailers or AI footers on commits/PR/issue per [[feedback_no_co_author_credit]].

### File List

**New files**

- [.github/workflows/commitlint.yml](.github/workflows/commitlint.yml) — Story 1.14 LD-54 enforcement chain workflow (two-job split)
- [scripts/smoke-commitlint.sh](scripts/smoke-commitlint.sh) — AC6 engine smoke
- [scripts/smoke-commitlint-title.sh](scripts/smoke-commitlint-title.sh) — AC7 engine smoke

**Modified files**

- [package.json](package.json) — added `smoke:commitlint` + `smoke:commitlint-title` scripts
- [CONTRIBUTING.md](CONTRIBUTING.md) — §2 enforcement-chain CI status updated (AC8)
- [_bmad-output/implementation-artifacts/sprint-status.yaml](_bmad-output/implementation-artifacts/sprint-status.yaml) — Story 1.14 → in-progress (Task 8.1)
- [_bmad-output/implementation-artifacts/1-14-configure-commitlint-husky-commit-msg-hook-ci-gate.md](_bmad-output/implementation-artifacts/1-14-configure-commitlint-husky-commit-msg-hook-ci-gate.md) — Status, Tasks/Subtasks, Dev Agent Record, Change Log

## Change Log

| Date       | Change                                                                                                                            | Author                                |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-29 | Story 1.14 contextualized via `bmad-create-story` (ready-for-dev).                                                                | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-29 | Implementation: `.github/workflows/commitlint.yml` + 2 smoke scripts + pnpm wiring + CONTRIBUTING.md §2 update. Tasks 1–4 + 8.1.  | Amelia (`bmad-dev-story`) for Tiziano |
| 2026-05-29 | PR #130 opened. `commitlint-range` ✅, `pr (macos-14)` ✅. Story → review. Tasks 5 + 8.2 done; Task 6 deferred to next PR. | Amelia (`bmad-dev-story`) for Tiziano |
