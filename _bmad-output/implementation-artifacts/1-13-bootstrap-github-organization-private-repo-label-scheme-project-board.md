# Story 1.13: Bootstrap GitHub organization + private repo + label scheme + Project board

Status: done

## Metadata

github_issue: 13

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want the `orgsidian` GitHub organization, the `orgsidian/orgsidian` repo, the LD-55 label scheme (declared in a canonical `.github/labels.yml` + applied by a labels-sync workflow), a story-typed Issue template, and the org-level GitHub Project v2 board ("Orgsidian Roadmap" — 4 columns, 2 saved views) bootstrapped idempotently — accepting that the org + repo already exist and the repo is already PUBLIC (per [[project_orgsidian_repo_public_during_pre_alpha]] — LD-5's "private during pre-Alpha" wording is stale; do NOT flip back),
So that work tracking has its durable surface in place before Epic 2 begins, and Stories 1.14 (commitlint CI gate), 1.15 (`git-cliff` CHANGELOG), and 1.16 (epics.md → Issues sync Rust binary) inherit a normalized label/Project ground truth instead of authoring it ad-hoc.

## Acceptance Criteria

**AC1 — `orgsidian` org + `orgsidian/orgsidian` repo idempotency assertion (no creation work; verify-only).**

- The org already exists on **GitHub Free** (`gh api orgs/orgsidian --jq '.plan.name'` → `"free"`; 1 filled seat; 10000 private-repo allowance; verified 2026-05-28). The repo `orgsidian/orgsidian` already exists with default branch `main` and visibility **PUBLIC** (`gh api repos/orgsidian/orgsidian --jq '.visibility'` → `"PUBLIC"`; verified 2026-05-28). The Story 1.1 scaffold is already pushed (latest commit `0c9ea84` Merge of Story 1.12 PR #127 lives on `main`).
- **DO NOT recreate, rename, or flip visibility.** No `gh repo create`, no `gh api orgs` POST, no `gh repo edit --visibility private`. The repo stays public per [[project_orgsidian_repo_public_during_pre_alpha]]. The architecture-AC text "create private + flip at v0.1 Alpha" is stale framing absorbed during the 2026-05-19 sprint-change-proposal but never enforced (the user went public earlier, around 2026-05-25 per memory). LD-5 and README.md "Repository is private during pre-Alpha" are stale prose — Story 1.13 does **not** fix LD-5 or README.md (out of scope; flag as a docs-debt follow-up in §11).
- **Pre-flight verification step** (REQUIRED at task start): the dev agent runs `gh api orgs/orgsidian --jq '{login,plan_name:.plan.name,seats:.plan.filled_seats}'` and `gh api repos/orgsidian/orgsidian --jq '{visibility,default_branch,name}'` and records the literal JSON in the Dev Agent Record / Debug Log References block (proof of state-at-start). If either call returns non-200 or unexpected JSON, HALT with a clear "org/repo state diverged — investigate before proceeding" message rather than silently retrying.

**AC2 — `.github/labels.yml` declares the LD-55 label scheme as the canonical source of truth.**

- File path: `.github/labels.yml` (NEW file — verified missing via `ls /Users/tizianobasile/workspace/me/orgsidian/.github/` which lists only `workflows/`). This file replaces the hand-coded `ensure_label` calls inside [scripts/sync-epics-to-github.sh:45-176](scripts/sync-epics-to-github.sh#L45-L176) as the authoritative label declaration; the shell script is decommissioned by Story 1.16 anyway, but its embedded label colors/descriptions MUST be preserved verbatim in `.github/labels.yml` to keep existing issues' label rendering unchanged.
- **Format**: the [crazy-max/ghaction-github-labeler](https://github.com/crazy-max/ghaction-github-labeler) schema — a YAML list of `{ name, color (6-hex no '#'), description }` objects. The action defaults to a "merge" semantic (additive — does not delete labels not present in the file unless `skip-delete: false` is set, which we explicitly enable; see AC3). The schema accepts an optional `from` rename field; we don't use it (no rename migration needed today).
- **Required entries** (29 total — epic labels + milestones + statuses + types + priorities):
  - **Epic labels (13)**: `epic:1` … `epic:13`, color `0e8a16`, description `Epic <N>`. Existing labels in the repo already use this color/desc — keep verbatim.
  - **Milestone labels (3)**: `milestone:v0.1` color `1d76db` desc `Milestone v0.1 Alpha`; `milestone:v0.5` color `5319e7` desc `Milestone v0.5 Beta`; `milestone:v1.0` color `b60205` desc `Milestone v1.0`.
  - **Status labels (5)** — **NAMING RECONCILIATION**: the epics-AC text at [_bmad-output/planning-artifacts/epics.md:630](_bmad-output/planning-artifacts/epics.md#L630) says `status:review`, but the in-use repo scheme is `status:in-review` per [[project_orgsidian_github_label_scheme]] (Story `review` state → GitHub label `status:in-review`). The memory + existing repo state win; the architecture LD-55 prose at [_bmad-output/planning-artifacts/architecture.md:625](_bmad-output/planning-artifacts/architecture.md#L625) is the same stale `status:review` framing as the epics text. Use the in-use names:
    - `status:backlog` color `ededed` desc `Status: backlog (default for synced stories)` *(exists)*
    - `status:in-progress` color `fbca04` desc `Status: actively being implemented` *(exists)*
    - `status:in-review` color `0e8a16` desc `Status: PR open, awaiting review/merge` *(exists)*
    - `status:blocked` color `b60205` desc `Status: blocked on external dependency` *(NEW — missing today)*
    - `status:done` color `5319e7` desc `Status: completed and merged` *(exists)*
  - **Type labels (6)** — `type:story` exists; the remaining 5 are NEW:
    - `type:story` color `c5def5` desc `Story (epic decomposition)` *(exists)*
    - `type:bug` color `d73a4a` desc `Bug — defect against documented behavior` *(NEW)*
    - `type:spike` color `fbca04` desc `Spike — time-boxed investigation, deliverable is a decision` *(NEW)*
    - `type:chore` color `cfd3d7` desc `Chore — repo / infra / housekeeping (no user-visible change)` *(NEW)*
    - `type:docs` color `0075ca` desc `Docs — documentation-only change` *(NEW)*
    - `type:security` color `b60205` desc `Security — CVE / dependency / surface-hardening change` *(NEW)*
  - **Priority labels (2)** — both NEW (used sparingly per LD-55 prose):
    - `priority:p0` color `b60205` desc `Priority P0 — drop everything (release-blocker, regression, CVE)` *(NEW)*
    - `priority:p1` color `fbca04` desc `Priority P1 — current-sprint commitment` *(NEW)*
- **DO NOT include the GitHub-default labels** (`bug`, `documentation`, `duplicate`, `enhancement`, `good first issue`, `help wanted`, `invalid`, `question`, `wontfix`) in `.github/labels.yml`. They are present on the repo today (auto-created) and the labeler will delete them on first sync run (see AC3 `skip-delete: false`). The unprefixed `bug` / `documentation` clash semantically with `type:bug` / `type:docs`; cleaning them up is the point. Issue #120 currently has the unprefixed `bug` label — after Story 1.13's first labels-sync run, that label is gone; **the dev agent MUST manually re-label issue #120 with `type:bug` BEFORE landing the PR** (verification: `gh issue view 120 -R orgsidian/orgsidian --json labels`). Document this one-time migration in §11.
- File comment header (top of `.github/labels.yml`): a single `# LD-55 canonical label scheme — see _bmad-output/planning-artifacts/architecture.md#L617-L631. Synced by .github/workflows/labels-sync.yml. Renames/removals require deliberate edit; the labeler skip-delete is OFF.` line.

**AC3 — `.github/workflows/labels-sync.yml` applies `.github/labels.yml` on push to main.**

- File path: `.github/workflows/labels-sync.yml` (NEW file). Lives alongside the existing `pr.yml` + `nightly.yml`.
- **Action choice**: `crazy-max/ghaction-github-labeler@v5` (semver-major-pinned per [[feedback_version_policy]]; MIT; latest stable v5.x as of 2026-05-28). Rejected alternative: `actions/github-script` (would force us to author + maintain a labels-sync JavaScript script ourselves — unnecessary when a dedicated action exists with the exact merge/replace semantics we need).
- **Trigger**: `on: { push: { branches: [main], paths: ['.github/labels.yml'] }, workflow_dispatch: {} }` — runs only when the labels file itself changes (avoids burning CI minutes on every commit to main) plus a manual `workflow_dispatch` for re-runs.
- **Job shape**:
  ```yaml
  name: labels-sync
  on:
    push:
      branches: [main]
      paths: ['.github/labels.yml']
    workflow_dispatch: {}
  permissions:
    issues: write   # the action needs issues:write to create/update labels
  jobs:
    sync:
      runs-on: ubuntu-24.04
      timeout-minutes: 5
      steps:
        - uses: actions/checkout@v5
        - uses: crazy-max/ghaction-github-labeler@v5
          with:
            github-token: ${{ secrets.GITHUB_TOKEN }}
            yaml-file: .github/labels.yml
            skip-delete: false   # we WANT default-label cleanup (see AC2)
            dry-run: ${{ github.event_name == 'pull_request' }}
  ```
  - Pin the `ubuntu-24.04` runner image (never `ubuntu-latest`) per the Story 1.8 `pr.yml` discipline ([.github/workflows/pr.yml:38](.github/workflows/pr.yml#L38)).
  - `actions/checkout@v5` semver-major-pinned matches the convention in `pr.yml` ([.github/workflows/pr.yml:43](.github/workflows/pr.yml#L43)).
  - `permissions: issues: write` is the minimum scope — do NOT add `contents: write` / `pull-requests: write` / `repo` (overscope is a security smell; the action only touches `/repos/{owner}/{repo}/labels`).
- **DO NOT add the labels-sync workflow as a required status check on PRs.** GitHub Free → branch protection is unenforceable anyway per [[project_orgsidian_github_plan]]; even if it were, this workflow runs on push-to-main, not on PR, so it is structurally not a PR gate.
- **DO NOT add this workflow to the `pr.yml` required-checks comment block.** The labels-sync is a post-merge convergence, not a per-PR validation.

**AC4 — `.github/ISSUE_TEMPLATE/story.md` renders the LD-55 issue-body template.**

- File path: `.github/ISSUE_TEMPLATE/story.md` (NEW file — verified missing; the `.github/` folder today contains only `workflows/`).
- **Format**: a [GitHub Markdown issue template](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository#legacy-issue-templates) (legacy `.md` form, not the YAML `.yml` form — chosen because (a) the body is free-form prose with multiple section headers, not a fixed-field survey; (b) it matches the LD-55 prose at [_bmad-output/planning-artifacts/architecture.md:629](_bmad-output/planning-artifacts/architecture.md#L629) which says "Issue body template (`.github/ISSUE_TEMPLATE/story.md`)" — note the `.md` suffix; the architecture pre-resolves the choice).
- **Required frontmatter** (verbatim):
  ```yaml
  ---
  name: Story
  about: Mirror a Story N.M from _bmad-output/planning-artifacts/epics.md into an Issue
  title: '[Story N.M] <title>'
  labels: ['type:story', 'status:backlog']
  assignees: []
  ---
  ```
  - `name: Story` is the picker-list label users see at `https://github.com/orgsidian/orgsidian/issues/new/choose`.
  - `labels` pre-applies `type:story` + `status:backlog` (every synced story starts in backlog per LD-55).
  - DO NOT pre-apply an `epic:N` or `milestone:v0.X` label — those are story-specific and the issue author (or Story 1.16's Rust binary) fills them in.
- **Required body sections** (in order):
  1. `## Persona` — one line: `<persona name from epics.md "As a/the <persona>" line>`.
  2. `## User Story` — the three-line `As a … / I want … / so that …` block from the epics.md Story header.
  3. `## Acceptance Criteria` — the AC list verbatim (BDD-formatted Given/When/Then blocks per LD-7+LD-37 style).
  4. `## Traces` — the `Traces:` line from the epics.md story (e.g., `LD-5 (repo location + visibility), LD-55 (label scheme + Project board).`).
  5. `## Microcopy` — flag-only line: `[Microcopy: draft|final|n/a]` — picked from the epics.md `[Microcopy: …]` annotation if present, else `n/a`.
  6. `## Source` — last section: `Source: [epics.md#story-N-M](https://github.com/orgsidian/orgsidian/blob/main/_bmad-output/planning-artifacts/epics.md)` — an absolute link to the epics.md anchor. The `#story-N-M` anchor is a placeholder that the GitHub markdown renderer resolves to the heading slug per its h-tag autolinking; if the actual rendered anchor differs (GitHub slugifies "Story 1.13: Bootstrap GitHub organization + private repo + label scheme + Project board" as `story-113-bootstrap-github-organization--private-repo--label-scheme--project-board` — with double dashes from `+` chars), this is acceptable degraded behavior — the file path is the durable reference; the anchor is best-effort.
- **DO NOT add a "Definition of Done" section.** LD-55 prose lists exactly the 6 sections above; adding a 7th drifts from the spec and creates a maintenance burden on the Story 1.16 Rust binary that has to render this template programmatically.
- **DO NOT add a `config.yml`** at `.github/ISSUE_TEMPLATE/config.yml`. The default behavior (allow blank issues + auto-pick the story.md template when only one template exists) is fine for the v0.1 surface. Story 1.13 is the *story* template; a future Story 1.14+ adding bug/spike/chore templates is when `config.yml` becomes necessary.

**AC5 — GitHub Project v2 board "Orgsidian Roadmap" exists at `orgsidian/projects/1` with 4 columns + 2 saved views.**

- **Pre-flight token scope** (REQUIRED before any Project v2 API call): the current `gh auth status` shows scopes `'admin:public_key', 'gist', 'read:org', 'repo'` — **Project v2 needs `project` scope** (read+write) which is NOT present. The dev agent MUST run `gh auth refresh -h github.com -s project,read:project` (interactive — opens browser; surface this as the FIRST step in the §8 task list with a clear "this requires user interaction" callout). Do NOT attempt to use a PAT or `secrets.GITHUB_TOKEN` substitute — refreshing the existing CLI auth is the lowest-friction path. If the user is on a corp network that blocks the browser flow, fall back to a manual PAT with the `project` scope (the user creates it at https://github.com/settings/tokens) and `gh auth login --with-token < pat.txt`.
- **Verify-or-create**: after scope refresh, the dev agent runs `gh api graphql -f query='query{organization(login:"orgsidian"){projectsV2(first:10){nodes{number title}}}}'` to enumerate existing projects.
  - If a project with title `"Orgsidian Roadmap"` exists at any number, **reuse it** — record the actual number in the Dev Agent Record; do NOT create a duplicate.
  - If no project with that title exists, **create one** via `gh api graphql -f query='mutation{createProjectV2(input:{ownerId:"<org-node-id>",title:"Orgsidian Roadmap"}){projectV2{id number url}}}'`. The `<org-node-id>` is obtained from `gh api graphql -f query='{organization(login:"orgsidian"){id}}'` (cache it; you'll need it once).
- **Columns** (LD-55 + epics AC: 4 columns named **Backlog**, **In Progress**, **Review**, **Done**). Project v2 uses a single-select "Status" field for columns; the default field is auto-created with options `Todo`, `In Progress`, `Done`. The dev agent MUST:
  - Rename the default `Todo` option to `Backlog` (preserve any pre-existing items in that column);
  - Keep `In Progress` as-is;
  - Add a new `Review` option **between** `In Progress` and `Done` (Projects v2 GraphQL `updateProjectV2Field` with positioned options);
  - Keep `Done` as-is.
  - **Naming note**: the column option name is `Review` (matching LD-55 column prose), NOT `In Review` — column option names are user-facing on the board; the label/issue-status naming reconciliation (AC2: `status:in-review` not `status:review`) is independent of column naming. Document this divergence in §11.
- **Two saved views** (LD-55: "By Milestone v0.1" + "By Epic"):
  - View 1: title `By Milestone v0.1` — filter `label:"milestone:v0.1"`, default layout `Board` (Status column), no group.
  - View 2: title `By Epic` — filter (none), layout `Board` (Status column), **group by `Labels` containing `epic:`** (Project v2 doesn't natively group by label prefix; achieve this by adding a new single-select "Epic" field with options `epic:1` … `epic:13` and a per-item assignment workflow OR — simpler — by creating 13 separate filter-only views, one per epic, each titled `Epic <N>` with filter `label:"epic:<N>"`). **Decision-grade question** (surface to user — see §10): the LD-55 prose says "grouped by `epic:N` label" which implies the second approach (13 views) since Projects v2 cannot natively group-by-label-substring; the simpler one-view-with-Epic-field approach diverges from the spec literal. **Default if user does not respond: ship the 13-views variant** (one filter-only view per epic) — closer to the LD-55 literal, no new field to maintain in lockstep with `.github/labels.yml`, and "By Epic" becomes a folder of 13 views in the Projects sidebar.
- **DO NOT add automation rules, swim lanes, or custom fields beyond what AC5 specifies.** LD-55 prose: "No swim lanes, no custom fields, no automation rules beyond the issue-sync workflow placing new issues in Backlog. Solo-dev discipline guard: do not add complexity unless a pain-point in v0.1 demonstrates a need."
- **DO NOT enable Projects v2 built-in workflows** (auto-add issues, auto-archive, etc.). Story 1.16's Rust binary owns issue→project insertion via explicit `addProjectV2ItemById` GraphQL calls; competing automation produces races.

**AC6 — `scripts/sync-epics-to-github.sh` hand-coded `ensure_label` calls are removed (canonical source now `.github/labels.yml`).**

- File: [scripts/sync-epics-to-github.sh](scripts/sync-epics-to-github.sh) — the existing bootstrap shell version of the epics-sync (lines 45-176 contain the hand-coded label declarations).
- **Edit scope**: delete the `ensure_label` invocations at [scripts/sync-epics-to-github.sh:165-176](scripts/sync-epics-to-github.sh#L165-L176) (the 12-ish `ensure_label "<name>" "<color>" "<desc>"` lines + the `for n in $(seq 1 13)` loop above). Replace with a one-line comment: `# Labels are managed by .github/labels.yml (Story 1.13) + .github/workflows/labels-sync.yml. The bootstrap label-ensure block was removed; running this script no longer drifts the label scheme.`
- **DO NOT delete the `ensure_label` function definition itself** at [scripts/sync-epics-to-github.sh:45-51](scripts/sync-epics-to-github.sh#L45-L51) — leaving the function defined-but-unused is acceptable for the few weeks until Story 1.16 retires the whole shell script in favor of `tools/issues-sync/` (Rust). The Story 1.16 retirement is the right place to delete the function; doing it now creates churn for no benefit. The post-edit shell script still functions for issue creation (its primary purpose); it just no longer mutates labels.
- **DO NOT delete `scripts/sync-epics-to-github.sh` itself** — it remains the operational issue-sync tool until Story 1.16 lands. Story 1.13 is label-scheme bootstrap; issue-sync retirement is Story 1.16's job.
- **Smoke verification** post-edit: `bash -n scripts/sync-epics-to-github.sh` (syntax check), then `DRY_RUN=1 bash scripts/sync-epics-to-github.sh 2>&1 | head -20` (dry-run sanity — should still parse epics.md without error). Record both in the Debug Log References.

**AC7 — Issue #120 is re-labeled to use the new `type:bug` label.**

- Issue #120 (`windows nightly: export_bindings exits STATUS_ENTRYPOINT_NOT_FOUND (Story 1.4 deferred-work)`) currently carries the unprefixed `bug` label (plus `milestone:v0.1`). After AC3's first labels-sync run, the unprefixed `bug` label is deleted from the repo.
- **Required action** (one-off migration, executed manually by the dev agent — NOT automated in a script): `gh issue edit 120 -R orgsidian/orgsidian --remove-label bug --add-label type:bug` — but run this **AFTER** the labels-sync workflow has actually applied `.github/labels.yml` (so `type:bug` exists as a defined label).
- **Sequencing**: the dev agent's task order is: (1) author `.github/labels.yml` + `labels-sync.yml` + `story.md` + Project work; (2) open PR; (3) merge PR → labels-sync workflow fires on push-to-main → `type:bug` is created + unprefixed `bug` is deleted; (4) only THEN run the `gh issue edit 120 …` migration. Pre-merge, `type:bug` doesn't exist yet so the `--add-label` step would fail. **Document this post-merge step in the Story 1.13 PR body** as an explicit "post-merge manual cleanup" instruction; the maintainer (Tiziano) is the one who will run it after merging.
- **DO NOT** attempt to re-label issue #120 *before* merging the PR. The `bug` → `type:bug` migration is a post-merge convergence step; pre-merge ordering breaks the labels-sync semantics (the action would see the old `bug` label still in use and might error out depending on whether `skip-delete` reads usage counts).

**AC8 — Pre-existing issues #1-#16 + #80-#120 are not retroactively re-labeled with the new type/priority taxonomy.**

- The 16 + 41 pre-existing issues (created by the `scripts/sync-epics-to-github.sh` bootstrap pass) all carry the original 4-label set: `epic:N` + `milestone:v0.X` + `type:story` + `status:backlog`. They are NOT retroactively edited to add `priority:p0|p1` annotations.
- **Rationale**: priority labels are LD-55 "used sparingly" — they apply only to in-flight work where the maintainer has elected to surface a P0/P1 signal. Retroactively applying them to 100+ backlog issues defeats the "used sparingly" semantics. Story 1.16's Rust binary will not auto-apply priority labels either; manual is authoritative.
- **What IS automatic post-merge**: the unprefixed default `bug` / `documentation` / `enhancement` / etc. labels get *deleted* from the repo by the labels-sync action (because `skip-delete: false`). Existing issues that carried those labels (only #120 today carries unprefixed `bug` — `gh issue list -R orgsidian/orgsidian --state all --label bug --json number` confirms) lose them. AC7 handles the #120 migration; no other issues are affected.

**AC9 — Verification matrix (executed post-merge, results recorded in Dev Agent Record).**

This is the hard-truth gate. Each cell below MUST be re-run on the merged commit on `main` and the literal output recorded in the Debug Log References section:

| # | Verification | Pass condition |
|---|---|---|
| 1 | `gh api orgs/orgsidian --jq '.plan.name'` | exit 0, output `"free"` |
| 2 | `gh api repos/orgsidian/orgsidian --jq '{visibility,default_branch}'` | exit 0, output `{"visibility":"public","default_branch":"main"}` — note: GitHub REST returns lowercase `"public"` (verified 2026-05-28); memory: stays public, do not flip |
| 3 | `ls .github/labels.yml .github/workflows/labels-sync.yml .github/ISSUE_TEMPLATE/story.md` | exit 0, 3 files present |
| 4 | `gh label list -R orgsidian/orgsidian --limit 200 --json name -q '.[].name' \| wc -l` | output ≥ 29 (13 epic + 3 milestone + 5 status + 6 type + 2 priority = 29) — `--json name -q '.[].name'` makes the count deterministic vs TTY-formatted output |
| 5 | `gh label list -R orgsidian/orgsidian --search "priority:" --json name -q '.[].name'` | output contains both `priority:p0` and `priority:p1` |
| 6 | `gh label list -R orgsidian/orgsidian --search "type:" --json name -q '.[].name'` | output contains all of `type:story`, `type:bug`, `type:spike`, `type:chore`, `type:docs`, `type:security` |
| 7 | `gh label list -R orgsidian/orgsidian --search "status:blocked" --json name -q '.[].name'` | output `status:blocked` |
| 8 | `gh label list -R orgsidian/orgsidian --search "bug" --json name -q '.[].name' \| awk '!/^type:bug$/{n++}END{print n+0}'` | output `0` (no labels match `bug` other than `type:bug`; unprefixed `bug` is deleted). `awk` form avoids the `grep -v` exit-code-1-on-no-match brittleness under `set -e` |
| 9 | `gh issue view 120 -R orgsidian/orgsidian --json labels -q '.labels[].name' \| awk '/^type:bug$/{n++}END{print n+0}'` | output `1` (post-AC7 migration). `awk` form avoids the `grep -c` exit-code-1-on-no-match brittleness |
| 10 | `gh api graphql -f query='query{organization(login:"orgsidian"){projectsV2(query:"Orgsidian Roadmap",first:1){nodes{number title url}}}}'` | exit 0, returns the project node with title `"Orgsidian Roadmap"` |
| 11 | `gh issue view 13 -R orgsidian/orgsidian --json state -q '.state'` | output `"OPEN"` pre-PR, `"CLOSED"` post-PR-merge (`Closes #13` in PR body) |
| 12 | `head -3 .github/ISSUE_TEMPLATE/story.md` | output starts with `---` then `name: Story` then `about: ...` |
| 13 | `bash -n scripts/sync-epics-to-github.sh` | exit 0 (syntax still valid after AC6 edit) |
| 14 | `DRY_RUN=1 bash scripts/sync-epics-to-github.sh 2>&1 \| tail -5` | exit 0; output shows the script parses epics.md without referencing the removed `ensure_label` calls |

All 14 cells must pass on the merged main commit. Cells 1, 2, 10, 11 require network + authenticated `gh`; the others are local file/shell checks.

## Tasks / Subtasks

- [x] **Task 1: Pre-flight verification + token scope refresh** (AC: 1, 5)
  - [x] 1.1 Run `gh api orgs/orgsidian --jq '{login,plan_name:.plan.name,seats:.plan.filled_seats}'` and `gh api repos/orgsidian/orgsidian --jq '{visibility,default_branch,name}'`; record literal JSON in `Debug Log References`.
  - [x] 1.2 Confirm `gh auth status` includes `project` scope; if not, run `gh auth refresh -h github.com -s project,read:project` (interactive — open browser). Surface this to the user explicitly if the dev-story is running unattended.
  - [x] 1.3 Capture the org node ID once: `gh api graphql -f query='{organization(login:"orgsidian"){id}}'` → cache for AC5.
- [x] **Task 2: Author `.github/labels.yml`** (AC: 2)
  - [x] 2.1 Create the file with the 29 entries listed in AC2; preserve existing colors/descriptions for already-present labels (epic:1-13, milestone:v0.1/v0.5/v1.0, status:backlog/in-progress/in-review/done, type:story).
  - [x] 2.2 Add the file-level `# LD-55 canonical label scheme …` comment header.
  - [x] 2.3 Lint via `python3 -c "import yaml; yaml.safe_load(open('.github/labels.yml'))"` or equivalent — file must parse as a list of dicts with the three keys.
- [x] **Task 3: Author `.github/workflows/labels-sync.yml`** (AC: 3)
  - [x] 3.1 Create the workflow file with the exact YAML shape in AC3.
  - [x] 3.2 Action version pinned to `crazy-max/ghaction-github-labeler@v5`; runner pinned to `ubuntu-24.04`.
  - [x] 3.3 `permissions: issues: write` only — no other scopes.
  - [x] 3.4 Lint: `pnpm exec prettier --check .github/workflows/labels-sync.yml` (if prettier is wired to YAML) OR a YAML round-trip via Python.
- [x] **Task 4: Author `.github/ISSUE_TEMPLATE/story.md`** (AC: 4)
  - [x] 4.1 Create the file with the frontmatter block + the 6 body sections (Persona → User Story → Acceptance Criteria → Traces → Microcopy → Source).
  - [x] 4.2 Body sections use placeholder copy ("As a …, I want …, so that …" etc.) — this is a TEMPLATE that humans (or Story 1.16's Rust binary) fill in at issue-create time.
  - [x] 4.3 No `config.yml` accompanies it.
- [x] **Task 5: Verify-or-create the "Orgsidian Roadmap" Project v2 + configure columns + saved views** (AC: 5) — **partial**: views deferred to follow-up #128 (API gap)
  - [x] 5.1 Enumerate existing projects via GraphQL `projectsV2(first:10)`. If found, record its number/URL. Else `createProjectV2` mutation.
  - [x] 5.2 Rename `Status` field's `Todo` option to `Backlog`; insert `Review` between `In Progress` and `Done`. Use Projects v2 `updateProjectV2Field` GraphQL.
  - [ ] 5.3 ~~Create saved view `By Milestone v0.1`~~ — **DEFERRED to #128**: GitHub GraphQL API has no `createProjectV2View` / `updateProjectV2View` mutation (Dev Notes §5 URL was misleading; verified via schema introspection 2026-05-28). Saved views are UI-only.
  - [ ] 5.4 ~~Create saved view(s) `By Epic`~~ — **DEFERRED to #128** for the same reason.
  - [x] 5.5 Record the project URL + the column-config GraphQL query results in `Debug Log References`.
- [x] **Task 6: Decommission the hand-coded label block in `scripts/sync-epics-to-github.sh`** (AC: 6)
  - [x] 6.1 Delete lines 165-176 (the `ensure_label` invocations + the `for n in $(seq 1 13)` loop).
  - [x] 6.2 Replace with a single-line comment per AC6.
  - [x] 6.3 Leave the `ensure_label` function definition intact.
  - [x] 6.4 `bash -n scripts/sync-epics-to-github.sh` to verify.
  - [x] 6.5 `DRY_RUN=1 bash scripts/sync-epics-to-github.sh` to dry-run.
- [ ] **Task 7: Commit + open PR** (workflow gate)
  - [ ] 7.1 Single commit titled per LD-54 — recommended: `chore(github): bootstrap LD-55 label scheme + Issue template + Project board config (Story 1.13, closes #13)`. The `chore` scope reflects "no user-visible change"; alternatively `feat(github): …` per the "first-time bootstrap deserves feat" reading. **Decision-grade question** (surface to user per [[feedback_batch_fixes_terse]]): `chore` vs `feat` for first-time infra bootstrap. **Default: `chore`** (this is repo bootstrapping, not a desktop-app user-visible change; LD-54 mapping table buckets `chore` as "excluded from CHANGELOG" which is correct for `.github/` infra). **Resolved 2026-05-28: `chore` per user.**
  - [ ] 7.2 PR body must contain `Closes #13` (workflow gate per Story 1.12's review-findings PR-Gate entry).
  - [ ] 7.3 No Co-Authored-By trailers per [[feedback_no_co_author_credit]].
- [ ] **Task 8: Post-merge convergence steps** (AC: 7, 9)
  - [ ] 8.1 After PR merge, the labels-sync workflow fires on push-to-main and converges the label set. Watch the workflow run; record its conclusion.
  - [ ] 8.2 Run `gh issue edit 120 -R orgsidian/orgsidian --remove-label bug --add-label type:bug` (AC7 one-off migration).
  - [ ] 8.3 Run all 14 AC9 verification cells; record literal output in `Debug Log References`.
- [ ] **Task 9: Sprint status + issue #13 status transitions** (workflow boilerplate)
  - [x] 9.1 At story start: update `_bmad-output/implementation-artifacts/sprint-status.yaml` `1-13-bootstrap-…` from `ready-for-dev` → `in-progress`. Update issue #13 label `status:backlog` → `status:in-progress`.
  - [ ] 9.2 At PR-open: update sprint-status `in-progress` → `review`. Update issue #13 label `status:in-progress` → `status:in-review`.
  - [ ] 9.3 At PR-merge: update sprint-status `review` → `done`. Update issue #13 label → `status:done` AND close the issue (the `Closes #13` PR body footer auto-closes; the label still needs the manual flip).

## Review Findings (bmad-code-review, 2026-05-28)

Layers: Blind Hunter ✓ · Edge Case Hunter ✓ · Acceptance Auditor ✓ (no AC violations). Counts: 6 patch, 6 defer, 29 dismissed (incl. 2 decision-needed resolved as keep-as-is).

### Decision-needed (resolved 2026-05-28)

- [x] [Review][Decision] `dry-run` conditional is dead code — resolved: keep as-is (faithful to AC3 literal; dead-but-harmless; future-proof if `pull_request` trigger is ever added).
- [x] [Review][Decision] No `from:` rename for `bug` → `type:bug` migration — resolved: keep current AC7 manual flow (explicit two-step post-merge; AC7 stays).

### Patch (applied 2026-05-29)

- [x] [Review][Patch] Missing `contents: read` in workflow permissions [[.github/workflows/labels-sync.yml](.github/workflows/labels-sync.yml)] — added `contents: read` alongside `issues: write`. Header comment updated to explain why explicit permissions must list contents:read.
- [x] [Review][Patch] No `concurrency:` guard on labels-sync workflow [[.github/workflows/labels-sync.yml](.github/workflows/labels-sync.yml)] — added `concurrency: { group: labels-sync, cancel-in-progress: false }`.
- [x] [Review][Patch] `paths:` filter excludes the workflow file itself [[.github/workflows/labels-sync.yml](.github/workflows/labels-sync.yml)] — added `.github/workflows/labels-sync.yml` to the `paths:` list (now multi-line form).
- [x] [Review][Patch] AC9 cell 2 expects uppercase `"PUBLIC"` but API returns lowercase `"public"` — updated AC9 row 2 to lowercase canonical form.
- [x] [Review][Patch] AC9 cell 4 brittle `wc -l` on TTY-formatted output — updated AC9 row 4 to `--json name -q '.[].name' \| wc -l`.
- [x] [Review][Patch] AC9 cells 8 & 9 use `grep -v` / `grep -c` exit-code-fragile under `set -e` — updated AC9 rows 8 & 9 to `awk` form.

### Deferred (logged in deferred-work.md)

- [x] [Review][Defer] Issue template label drift if `labels.yml` renames `status:backlog`/`type:story` without updating template frontmatter [[.github/ISSUE_TEMPLATE/story.md:5](.github/ISSUE_TEMPLATE/story.md#L5)] — legacy `.md` template has no drift-detection mechanism.
- [x] [Review][Defer] Label-state inconsistency on PR-close (Task 9.3 requires manual flip; `Closes #N` auto-close keeps stale `status:in-progress`) — process gap; revisit in Story 1.16 (Rust sync binary).
- [x] [Review][Defer] Project v2 board state not reproducible from repo — Board #1 lives in GitHub state only; no Makefile/script encapsulates it. Acceptable solo-dev posture per LD-55 "no automation rules".
- [x] [Review][Defer] No `status:*` mutual-exclusion enforcement — issues can carry multiple status labels. Process discipline issue; out of LD-55 scope.
- [x] [Review][Defer] `scripts/sync-epics-to-github.sh` lacks preflight check that required labels exist [[scripts/sync-epics-to-github.sh:163](scripts/sync-epics-to-github.sh#L163)] — fresh-clone runs before `labels-sync` would 422 per issue; mitigated because Story 1.16 retires the script.
- [x] [Review][Defer] No `priority:p2`/`priority:p3` fallback label — LD-55 "used sparingly" design choice; not in this story's scope.

## Dev Notes

### §1 — State-at-start (verified 2026-05-28)

- **Org**: `orgsidian` exists; plan `free`; 1 filled seat; 10000 private-repo allowance (theoretical — GH Free still gates branch protection per [[project_orgsidian_github_plan]]).
- **Repo**: `orgsidian/orgsidian` exists; visibility `PUBLIC` (already flipped, per [[project_orgsidian_repo_public_during_pre_alpha]]); default branch `main`; latest commit `0c9ea84` (Merge PR #127 of Story 1.12).
- **Labels existing today** (from `gh label list -R orgsidian/orgsidian`):
  - Default GitHub labels (DELETED by labels-sync first-run): `bug`, `documentation`, `duplicate`, `enhancement`, `good first issue`, `help wanted`, `invalid`, `question`, `wontfix`.
  - Already conformant: `epic:1` … `epic:13` (13), `milestone:v0.1`, `milestone:v0.5`, `milestone:v1.0`, `status:backlog`, `status:in-progress`, `status:in-review`, `status:done`, `type:story` (21 conformant).
  - **MISSING** (NEW labels added by `.github/labels.yml`): `status:blocked`, `type:bug`, `type:spike`, `type:chore`, `type:docs`, `type:security`, `priority:p0`, `priority:p1` (8 NEW).
- **Issues existing today**: 16 epic-stories (#1–#16) + ~104 stories across epics 2–13 (#17 onward) + 1 deferred-work issue (#120). Created by the bootstrap [scripts/sync-epics-to-github.sh](scripts/sync-epics-to-github.sh) per the LD-55-foreshadowing convention. Story 1.16 retires that shell script in favor of the `tools/issues-sync/` Rust binary.
- **`.github/` folder**: contains ONLY `workflows/{nightly.yml,pr.yml}` (verified). Missing today: `labels.yml`, `workflows/labels-sync.yml`, `ISSUE_TEMPLATE/`, `PULL_REQUEST_TEMPLATE.md`.
- **Project v2**: not enumerable with the current token scope (`read:org`+`repo` — missing `read:project`). The dev agent's first task is to refresh scopes; reality of project existence is then a `gh api graphql` enumeration away.

### §2 — Reality-vs-spec reconciliations (rated by binding force)

| # | Spec text | Reality | Resolution | Binding force |
|---|---|---|---|---|
| 1 | Epics AC + LD-5 say "private repo" | Repo is PUBLIC since ~2026-05-25 | Stay public; do not flip; document in §11 | **HIGH** — [[project_orgsidian_repo_public_during_pre_alpha]] is authoritative |
| 2 | Epics AC + LD-55 say `status:review` | Existing label is `status:in-review` | Use `status:in-review`; document divergence in §11 | **HIGH** — [[project_orgsidian_github_label_scheme]] is authoritative |
| 3 | Architecture talks about "branch protection on `main`" implicitly | GH Free → branch protection unenforceable | Story 1.13 does NOT add branch-protection config | **HIGH** — [[project_orgsidian_github_plan]] is authoritative |
| 4 | LD-55 prose says Project board "grouped by `epic:N` label" | Projects v2 cannot group by label-substring | Use 13 filter-only views; surface decision-grade Q | **MEDIUM** — see §10 |
| 5 | LD-55 prose: "Issue body template" + 6 sections (persona/user-story/AC/Traces/Microcopy/anchor) | None of these exist today | Author the file per AC4 | **HIGH** — literal LD-55 spec |
| 6 | Architecture says "actions/github-script OR crazy-max/ghaction-github-labeler" | Neither is wired | Pick `crazy-max/ghaction-github-labeler@v5` per AC3 rationale | **LOW** — author's discretion |
| 7 | Epics AC says "Project board ... 4 columns (Backlog / In Progress / Review / Done)" | Projects v2 default Status field has `Todo`/`In Progress`/`Done` | Rename `Todo`→`Backlog`, add `Review` between `In Progress` and `Done` | **MEDIUM** — LD-55 literal |

### §3 — DO-NOT-DO list

1. **DO NOT flip repo visibility back to private.** [[project_orgsidian_repo_public_during_pre_alpha]] is authoritative; LD-5 + README.md private-during-pre-Alpha framing is stale.
2. **DO NOT rename `status:in-review` to `status:review`.** [[project_orgsidian_github_label_scheme]] + the existing PR-status discipline are authoritative.
3. **DO NOT add a "labels-sync" job to `pr.yml`'s required-checks list.** It's a post-merge convergence; PR gates run pre-merge.
4. **DO NOT add branch protection rules** via `scripts/configure-branch-protection.sh` or any other channel — GH Free → unenforceable per [[project_orgsidian_github_plan]]; the existing script is a documented no-op.
5. **DO NOT delete `scripts/sync-epics-to-github.sh`** — Story 1.16 owns its retirement.
6. **DO NOT add `Co-Authored-By:` trailers or "Generated with Claude Code" footers** to commits / PRs / Issues per [[feedback_no_co_author_credit]].
7. **DO NOT auto-add Projects v2 built-in workflows** (auto-add issues, auto-archive, etc.). Story 1.16 owns project insertion explicitly.
8. **DO NOT use unpinned action versions** (`@latest`, `@main`). Semver-major-pinned (`@v5`) per [[feedback_version_policy]] and the existing `pr.yml` convention.
9. **DO NOT add a `priority:p0` / `priority:p1` label to existing backlog issues retroactively.** LD-55 "used sparingly" semantics demand manual-only assignment.
10. **DO NOT use `actions/labeler@v5`** (a different action that does PR-content-based labeling). The action we want is `crazy-max/ghaction-github-labeler@v5` (repo-wide label scheme sync). Easy name-collision trap.
11. **DO NOT add `PULL_REQUEST_TEMPLATE.md`** in this story. It's a reasonable future addition but is not in any AC; Story 1.10 already documents PR shape in CONTRIBUTING.md.
12. **DO NOT switch the existing `pr.yml` permissions block** to add anything for labels-sync.

### §4 — Token scope mechanics

The interactive `gh auth refresh -h github.com -s project,read:project` flow is the canonical path. It:
1. Opens a browser to https://github.com/settings/connections/applications/178c6fc6912e29d6d7b that prompts for the additional scopes;
2. Once approved, `gh` re-fetches a token with the unioned scope set;
3. The keyring entry (Mac keychain in this case — confirmed via the keyring backend visible in `gh auth status`) is updated in place.
After refresh, `gh auth status` should show `'admin:public_key', 'gist', 'project', 'read:org', 'read:project', 'repo'` (alphabetical).

### §5 — Project v2 GraphQL idiosyncrasies

- The single-select "Status" field's option IDs are needed for renames; obtain via:
  ```graphql
  query {
    node(id: "<project-id>") {
      ... on ProjectV2 {
        field(name: "Status") {
          ... on ProjectV2SingleSelectField {
            id
            options { id name }
          }
        }
      }
    }
  }
  ```
- Renaming an option preserves all items currently assigned to it (in-place rename, not delete-and-recreate).
- Inserting a new option between two existing ones uses `updateProjectV2Field` with the full `options` array re-ordered; the API replaces the list, so include all existing options + the new one in the desired order.
- Saved views in Projects v2 are scoped to a `ProjectV2View` node; creation is `createProjectV2View(input:{projectId, name, layout: BOARD_LAYOUT})`. Filters and group-by are then set via `updateProjectV2View` mutations. The full mutation surface is documented at https://docs.github.com/en/graphql/reference/mutations#createprojectv2view.

### §6 — `.github/labels.yml` color discipline

- Re-use the existing colors for already-defined labels (verified via `gh label list`); the labeler will no-op on identical entries (idempotent merge).
- New label colors picked above are GitHub standard palette entries:
  - `b60205` (red) for blocked / security / p0;
  - `fbca04` (yellow) for in-progress / spike / p1;
  - `d73a4a` (light red) for type:bug;
  - `cfd3d7` (gray) for type:chore;
  - `0075ca` (blue) for type:docs;
- Color clashes between `priority:p0`/`type:security`/`status:blocked` (all `b60205`) are intentional — the textual prefix is the disambiguator; the shared visual signal "this is serious" reinforces the semantics.

### §7 — Microcopy flag semantics (for AC4)

The LD-55 prose mentions a `Microcopy: draft|final` flag on issue templates. This is a hold-over from the UX-design-specification reconciliation (2026-05-20). For Story 1.13 the template literal carries `[Microcopy: draft|final|n/a]` as a placeholder line — the issue author picks one when filing. Story 1.16's Rust binary will parse the epics.md `[Microcopy: …]` annotation (when present) and substitute the value at sync time. No action needed in Story 1.13 beyond shipping the placeholder line.

### §8 — Commit type decision (surface, do not silently pick)

Per [[feedback_batch_fixes_terse]] — decision-grade questions surface; no-brainer fixes silent. The `chore:` vs `feat:` vs `ci:` question for this story's commit is decision-grade because:
- `chore:` — bucketed as *excluded from CHANGELOG* per LD-54 (correct for `.github/` infra changes; the user-facing app is unaffected);
- `ci:` — same CHANGELOG bucket as `chore:` per LD-54, but `ci:` is reserved by convention for `.github/workflows/` changes specifically. The labels-sync workflow IS a `.github/workflows/` change, but the bulk of the diff is the labels.yml + ISSUE_TEMPLATE + Project board work, not CI logic;
- `feat:` — bucketed under CHANGELOG `Added`. Wrong here because there's no user-facing feature.

**Default: `chore(github): bootstrap LD-55 label scheme + Issue template + Project board config (Story 1.13, closes #13)`.** Surface the alternative `ci:` to the user if they prefer it for the workflow-creation aspect; do not silently pick `feat:`.

### §9 — Idempotency-first re-execution

If anything fails partway through and the dev agent re-runs, the operations are designed to converge:
- `.github/labels.yml` edits — file content is the source of truth; re-running labels-sync converges.
- ISSUE_TEMPLATE/story.md — overwrite-safe; no state.
- Project v2 — the verify-or-create branch handles existing-project re-runs without dupes.
- `scripts/sync-epics-to-github.sh` decommission — the `ensure_label` deletion is idempotent (deleted-then-deleted is fine).
- Issue #120 migration — `gh issue edit … --remove-label bug --add-label type:bug` is idempotent (removing a non-present label and adding an already-present one both no-op).

### §10 — Decision-grade questions to surface (not silently pick)

1. **`By Epic` view: 13 filter-only views (LD-55 literal) vs 1 view with new `Epic` single-select field (simpler).** Default in AC5: 13 views. Ask if user prefers the field-based simplification.
2. **Commit type: `chore:` vs `ci:`.** Default in Task 7.1: `chore:`.

Both questions surface in the PR thread per [[feedback_batch_fixes_terse]] — do not pick silently.

### §11 — Memory-anchored conventions + flagged docs-debt

- **[[project_orgsidian_repo_public_during_pre_alpha]]**: stay public; do not flip. README.md "Repository is private during pre-Alpha" line is stale → flagged docs-debt follow-up (NOT this story's scope — Story 1.10 also flagged this).
- **[[project_orgsidian_github_label_scheme]]**: `status:in-review` is the canonical name; `status:review` in epics.md + architecture is stale → docs-debt follow-up to amend LD-55 prose in architecture.md (NOT this story's scope; the *labels* are what we ship now, the *docs* catch up later).
- **[[project_orgsidian_github_plan]]**: GH Free → branch protection unenforceable; `scripts/configure-branch-protection.sh` is a no-op. Story 1.13 does not touch it.
- **[[feedback_no_co_author_credit]]**: no Co-Authored-By trailers; no "Generated with Claude Code" footers on commit/PR/Issue.
- **[[feedback_version_policy]]**: semver-major-pinned action versions (`@v5`); never `@latest` / `@main`.
- **[[feedback_batch_fixes_terse]]**: silent no-brainer fixes; surface only decision-grade Qs (§8 commit type + §10 By-Epic view shape are the two examples that should surface).
- **Dev-Notes §5 docs-debt (NEW 2026-05-28)**: the §5 prose cites `createProjectV2View` / `updateProjectV2View` mutations + the URL https://docs.github.com/en/graphql/reference/mutations#createprojectv2view — neither mutation exists in the GitHub GraphQL schema (verified via `__type(name:"Mutation"){fields{name}}` introspection on 2026-05-28). Saved views in Projects v2 are UI-only. AC5 sub-clauses 5.3/5.4 (saved views) were deferred to follow-up issue **#128**. The Dev-Notes §5 update is out-of-scope for Story 1.13; flag for whoever amends the Dev-Notes template next.
- **[[user_contact_email]]**: no email field anywhere in the new files — `.github/labels.yml` / `labels-sync.yml` / `story.md` carry no author metadata; this is intentional. The git commit author is set globally per the rust-toolchain / Cargo.toml authorship pin (`tiz.basile@gmail.com` per [[user_contact_email]]).

### §12 — Test strategy

No new automated tests are introduced by this story. AC9 is the verification matrix — 14 shell commands executed post-merge with literal-output recording. This is appropriate because:
- The artifacts are config files (.yml/.md) parsed by GitHub / `crazy-max/ghaction-github-labeler@v5` — those parsers are out-of-tree; we don't unit-test them.
- The Project v2 mutations are one-shot state changes — re-runnable but not unit-testable from this repo without mocking the GraphQL endpoint.
- The labels-sync workflow IS its own integration test: a single push-to-main run validates that `labels.yml` parses, labels converge, and `skip-delete: false` deletes the defaults. Watching that workflow's conclusion (cell 4-8 in AC9) is the de-facto smoke test.

The Story 1.10 pattern of an `AC8 verification matrix` is the reference; this story's AC9 follows the same shape with 14 cells (vs 1.10's 13).

### Project Structure Notes

- All new files land in `.github/` (the directory itself already exists with `workflows/`): `.github/labels.yml`, `.github/workflows/labels-sync.yml`, `.github/ISSUE_TEMPLATE/story.md` (creates the `ISSUE_TEMPLATE/` subdirectory).
- The single existing-file edit is `scripts/sync-epics-to-github.sh` (label-block decommission per AC6).
- No new Rust crates, no new pnpm dependencies, no new pnpm scripts, no Cargo workspace changes.
- No README.md edits in this story (stale "private during pre-Alpha" line flagged but not fixed — see §11).
- No new directories outside `.github/ISSUE_TEMPLATE/`.

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L618-L636](_bmad-output/planning-artifacts/epics.md#L618-L636) (Story 1.13 AC verbatim)
- Architecture LD-5 (org/repo location): [_bmad-output/planning-artifacts/architecture.md#L67](_bmad-output/planning-artifacts/architecture.md#L67) — note: "private" framing is stale per §11.
- Architecture LD-55 (label + Issue template + Project board): [_bmad-output/planning-artifacts/architecture.md#L617-L642](_bmad-output/planning-artifacts/architecture.md#L617-L642)
- Architecture LD-54 (Conventional Commits — informs §8 commit-type decision): [_bmad-output/planning-artifacts/architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)
- Sprint Change Proposal (NEW Stories 1.13-1.16 origin): [_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md#L55-L101](_bmad-output/planning-artifacts/sprint-change-proposal-2026-05-19.md#L55-L101)
- Existing labels bootstrap shell (decommissioned in this story): [scripts/sync-epics-to-github.sh:45-176](scripts/sync-epics-to-github.sh#L45-L176)
- Existing branch-protection script (no-op per memory; untouched by this story): [scripts/configure-branch-protection.sh](scripts/configure-branch-protection.sh)
- Existing `pr.yml` (action version + runner pin discipline reference): [.github/workflows/pr.yml:38-43](.github/workflows/pr.yml#L38-L43)
- Existing `nightly.yml` (workflow style reference): [.github/workflows/nightly.yml](.github/workflows/nightly.yml)
- Previous story (1.12, perf snapshot infra; AC-matrix style reference): [_bmad-output/implementation-artifacts/1-12-establish-perf-snapshot-regression-infrastructure-party-mode-round-2-p0-murat.md](_bmad-output/implementation-artifacts/1-12-establish-perf-snapshot-regression-infrastructure-party-mode-round-2-p0-murat.md)
- Reference story (1.10, hygiene docs; multi-file root-level pattern): [_bmad-output/implementation-artifacts/1-10-add-security-md-architecture-md-changelog-md-contributing-md.md](_bmad-output/implementation-artifacts/1-10-add-security-md-architecture-md-changelog-md-contributing-md.md)
- `crazy-max/ghaction-github-labeler` v5 README (action input/output reference): https://github.com/crazy-max/ghaction-github-labeler
- GitHub Projects v2 GraphQL reference: https://docs.github.com/en/graphql/reference/objects#projectv2 + https://docs.github.com/en/graphql/reference/mutations#createprojectv2

## Dev Agent Record

### Agent Model Used

`claude-opus-4-7[1m]` (Claude Opus 4.7, 1M-context) via `bmad-dev-story` skill.

### Debug Log References

**§1 Pre-flight (2026-05-28, pre-edit on main @ `0c9ea84`)**

```
$ gh api orgs/orgsidian --jq '{login,plan_name:.plan.name,seats:.plan.filled_seats}'
{"login":"orgsidian","plan_name":"free","seats":1}

$ gh api repos/orgsidian/orgsidian --jq '{visibility,default_branch,name}'
{"default_branch":"main","name":"orgsidian","visibility":"public"}
```

> Note: GitHub API returns `visibility: "public"` (lowercase). AC9 cell 2 spec expects `"PUBLIC"` (uppercase) — this is a literal mismatch in the AC text; the actual API response is canonically lowercase. Recording observed output.

**§4 Token-scope refresh** (Task 1.2)

User executed `gh auth refresh -h github.com -s project,read:project` interactively (browser). Post-refresh:

```
$ gh auth status
github.com
  ✓ Logged in to github.com account basteez (keyring)
  - Active account: true
  - Git operations protocol: ssh
  - Token: gho_************************************
  - Token scopes: 'admin:public_key', 'gist', 'project', 'read:org', 'repo'
```

(`read:project` collapses into the broader `project` scope, which is read+write — expected behavior.)

**Org node ID cached** (Task 1.3): `O_kgDOEQxtTQ`.

**Existing labels at state-at-start** (30 total — 21 conformant + 9 GitHub defaults):

```
bug | documentation | duplicate | enhancement | good first issue | help wanted | invalid | question | wontfix      ← 9 GH defaults (DELETED post-merge by labels-sync)
epic:1..epic:13                                                                                                   ← 13 conformant
milestone:v0.1 | milestone:v0.5 | milestone:v1.0                                                                  ← 3 conformant
status:backlog | status:in-progress | status:in-review | status:done                                              ← 4 conformant
type:story                                                                                                        ← 1 conformant
```

Missing post-Story-1.13: `status:blocked`, `type:bug`, `type:spike`, `type:chore`, `type:docs`, `type:security`, `priority:p0`, `priority:p1` (8 NEW).

**§5 Project v2 GraphQL trace**

Enumeration (pre-create):
```
$ gh api graphql -f query='query{organization(login:"orgsidian"){projectsV2(first:20){nodes{id number title url closed}}}}'
{"data":{"organization":{"projectsV2":{"nodes":[]}}}}
```

Creation:
```
$ gh api graphql -f query='mutation{createProjectV2(input:{ownerId:"O_kgDOEQxtTQ",title:"Orgsidian Roadmap"}){projectV2{id number url title}}}'
{"data":{"createProjectV2":{"projectV2":{
  "id":"PVT_kwDOEQxtTc4BZBHy",
  "number":1,
  "url":"https://github.com/orgs/orgsidian/projects/1",
  "title":"Orgsidian Roadmap"
}}}}
```

Status field (default `Todo | In Progress | Done`) reconfigured via `updateProjectV2Field`:
```
{"data":{"updateProjectV2Field":{"projectV2Field":{
  "id":"PVTSSF_lADOEQxtTc4BZBHyzhUCzKg",
  "name":"Status",
  "options":[
    {"id":"1f7f341c","name":"Backlog","color":"GRAY"},
    {"id":"6af5f793","name":"In Progress","color":"YELLOW"},
    {"id":"c91a5bb6","name":"Review","color":"GREEN"},
    {"id":"8942ba09","name":"Done","color":"PURPLE"}
  ]
}}}}
```

Saved-views API gap (Task 5.3/5.4 deferred):

```
$ gh api graphql -f query='query{__type(name:"Mutation"){fields{name}}}' --jq '.data.__type.fields[].name' | grep -i 'projectv2view'
(no output)

$ gh api graphql -f query='mutation{createProjectV2View(input:{projectId:"PVT_kwDOEQxtTc4BZBHy",name:"By Milestone v0.1",layout:BOARD_LAYOUT}){projectV2View{id name}}}'
gh: Field 'createProjectV2View' doesn't exist on type 'Mutation'
```

→ Follow-up issue **#128** opened to track manual UI work.

**Task 6 smoke verification (modified `scripts/sync-epics-to-github.sh`)**

```
$ bash -n scripts/sync-epics-to-github.sh
(exit 0 — syntax OK)

$ DRY_RUN=1 bash scripts/sync-epics-to-github.sh 2>&1 | tail -5
  ~ #107  [Story 13.6] (update)
[dry-run] gh issue edit 107 --body-file - --add-label epic:13,milestone:v1.0,type:story --milestone v1.0
  ~ #108  [Story 13.7] (update)
[dry-run] gh issue edit 108 --body-file - --add-label epic:13,milestone:v1.0,type:story --milestone v1.0
==> Done. Processed 117 stories.
```

→ No `==> Ensuring base label set` output — label-ensure block successfully removed. Script still processes 117 stories correctly.

**AC9 verification matrix (cells 1-14)**: deferred to post-merge per the AC9 spec ("executed post-merge, results recorded in Dev Agent Record"). Will be appended by the maintainer (Tiziano) after merging the PR and watching the labels-sync workflow run.

### Completion Notes List

**Delivered**:
- `.github/labels.yml` — 29-entry canonical LD-55 label scheme (13 epic + 3 milestone + 5 status + 6 type + 2 priority); preserves existing colors/descriptions for the 21 already-conformant labels; adds 8 missing labels.
- `.github/workflows/labels-sync.yml` — push-to-main + workflow_dispatch trigger; `crazy-max/ghaction-github-labeler@v5`; `skip-delete: false` (cleans up the 9 default GH labels post-merge); `permissions: issues: write` only; runner pinned to `ubuntu-24.04`.
- `.github/ISSUE_TEMPLATE/story.md` — LD-55 6-section template (Persona → User Story → Acceptance Criteria → Traces → Microcopy → Source); legacy `.md` format; `type:story` + `status:backlog` pre-applied via frontmatter.
- `scripts/sync-epics-to-github.sh` — `ensure_label` invocation block (lines 167-175 in the pre-edit file) removed; replaced with a one-line pointer comment; `ensure_label` function definition left intact per AC6; dry-run smoke OK.
- Project v2 board **#1** ("Orgsidian Roadmap") created at https://github.com/orgs/orgsidian/projects/1; Status field reconfigured `Backlog → In Progress → Review → Done`.

**Deferred (with explicit follow-up)**:
- AC5 sub-clauses 5.3/5.4 (saved views): GitHub GraphQL API has no `createProjectV2View` / `updateProjectV2View` mutation — the Dev-Notes §5 URL was misleading. Follow-up issue **#128** tracks the manual UI work (1 `By Milestone v0.1` view + the 13-Epic-filter-only variant per the §10 decision-grade default).
- AC7 (#120 `bug` → `type:bug` re-label) — post-merge manual step; cannot run pre-merge because `type:bug` doesn't exist as a label until labels-sync fires on push-to-main.
- AC9 verification matrix (14 cells) — post-merge per AC text.

**Decision-grade resolutions** (per [[feedback_batch_fixes_terse]]):
- §8 commit type: `chore` (user chose, default).
- §10 By-Epic view shape: `13 filter-only views` (user chose, default) — moot now since views API-gap deferred to #128.

**Notable observations**:
- AC9 cell 2 expects `visibility: "PUBLIC"` (uppercase) but the GitHub REST API returns lowercase `"public"`. Minor literal mismatch in the AC text; observed output recorded above.
- The `read:project` scope collapses into the broader `project` scope after refresh; `gh auth status` shows `project` only (not `read:project,project`).

### File List

**New files**:
- `.github/labels.yml`
- `.github/workflows/labels-sync.yml`
- `.github/ISSUE_TEMPLATE/story.md`

**Modified files**:
- `scripts/sync-epics-to-github.sh` — AC6 decommission of the `ensure_label` invocation block.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — Story 1.13 status transition (`ready-for-dev` → `in-progress` → `review` at PR-open).
- `_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md` — Tasks checkboxes, Status, Dev Agent Record, §11 docs-debt, Change Log.

**External state changes** (not files):
- GitHub org `orgsidian` → no change (verify-only).
- GitHub repo `orgsidian/orgsidian` → no change (verify-only; stays PUBLIC).
- GitHub issue **#13** → label `status:backlog` → `status:in-progress` (story start); will go → `status:in-review` at PR-open and → `status:done` at PR-merge.
- GitHub issue **#128** (NEW) → saved-views follow-up tracker.
- GitHub Project v2 **#1** "Orgsidian Roadmap" → created; Status field options reconfigured (4 columns: Backlog/In Progress/Review/Done).

## Change Log

| Date       | Change                                                                  | Author                                |
| ---------- | ----------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-28 | Story 1.13 contextualized via `bmad-create-story` (ready-for-dev).      | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-28 | Story 1.13 implementation: labels.yml + labels-sync.yml + story.md ISSUE_TEMPLATE + Project v2 board + Status field reconfig + `scripts/sync-epics-to-github.sh` ensure_label decommission. Saved views deferred to follow-up #128 (GitHub GraphQL API gap). Status → review at PR-open. | Amelia (`bmad-dev-story`) for Tiziano |
