---
title: Sprint Change Proposal — Three New Constraints (GitHub hosting & sync, Conventional Commits enforcement, test-design.md authority)
date: 2026-05-19
author: Tiziano (via bmad-correct-course)
status: draft (awaiting approval)
scope: moderate (cross-document propagation; no fundamental replan)
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/addendum.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/test-artifacts/test-design.md
---

# Sprint Change Proposal — 2026-05-19

## 1. Issue Summary

Three new cross-cutting development-infrastructure constraints have been added after PRD/Architecture/Epics finalization (2026-05-19) and after the test-design.md production step. They require propagation across planning documents without re-starting any workflow.

1. **Hosting & GitHub sync.** Monorepo lives in a **private** repo `orgsidian/orgsidian` under a new GitHub organization `orgsidian`. All epics and stories synced as **GitHub Issues** (1 issue per story); **labels** per epic + per milestone (v0.1 / v0.5 / v1.0) + status; **GitHub Project** kanban board added.
2. **Conventional Commits binding.** Every commit, PR title, and CHANGELOG entry follows [conventionalcommits.org/v1.0.0/](https://www.conventionalcommits.org/en/v1.0.0/). Architecture LD-33 already names CC as the changelog substrate but stops short of enforcement. Required: `commitlint` config, `husky` `commit-msg` hook (LD-2 stack already includes husky for pre-commit per architecture.md:785), CI gate, `CONTRIBUTING.md` section, Story 1.10 AC update, CC type → Keep-a-Changelog bucket mapping.
3. **Testing strategy authority.** The newly-produced `_bmad-output/test-artifacts/test-design.md` is the system-level test strategy and must be referenced as the **authoritative input** in: architecture testing sections (Cross-Cutting Concerns, CI LD-32, LD-41/43/44/45), `CONTRIBUTING.md`, and epics.md preamble.

These are **infrastructure-layer constraints, not product-layer**. The PRD body (the product contract with the reader) needs essentially zero edits; the work concentrates in architecture (one update + two new LDs) and epics (four new Epic-1 stories + one AC update + one duplicate cleanup).

## 2. Impact Analysis

### 2.1 PRD impact — minimal

- **§7.1 Privacy / §7.2 Data sovereignty:** untouched. The "no cloud account ever / no telemetry" commitments are runtime guarantees of the **shipped product**; the **source repository hosting** on GitHub is a development-infrastructure choice that has no observable effect on user data. The two commitments do not conflict.
- **§6.1 In Scope — v0.1 Alpha:** the bullet "Public repository, README, landing page, basic documentation" is preserved verbatim. The repository becomes public **at the v0.1 Alpha release tag** (SM-1 announcement); it remains private during pre-Alpha development (Months 1-6). This is consistent with §6.1 but warrants a one-line clarification.
- **§7.3 Cost:** untouched. "Free, open-source, forever / MIT" stands; private-during-development does not contradict OSS.
- **§10 Open Questions:** no new OQs needed (these are resolved decisions, not open questions).

The PRD body therefore takes **one revision-array entry only**, no inline body edits. Status `final` preserved.

### 2.2 Architecture impact — one LD update, two new LDs

- **LD-5 (Monorepo).** Existing text: "GitHub organization `orgsidian` hosting the monorepo; org reserved as namespace for v2+ ancillary repos." Add: repo visibility = **private during pre-Alpha (Months 1 to v0.1 Alpha release tag); flipped to public at v0.1 Alpha announcement (SM-1)**.
- **LD-33 (Release automation).** Existing text already mentions "Conventional commits enable semi-automated changelog generation" but lacks enforcement. Update to: name the **enforcement chain** (`commitlint` + `husky` commit-msg + CI gate), the **CHANGELOG-generation tool** (`git-cliff`), and the **CC → Keep-a-Changelog mapping** (see §3.2 below). Cross-reference new LD-54.
- **NEW LD-54: Conventional Commits enforcement + CHANGELOG mapping.** Full enforcement chain spec.
- **NEW LD-55: GitHub Issues sync + label scheme + Project board.** One issue per story, label taxonomy, Project board config, sync-automation strategy.
- **Cross-Cutting Concerns + testing-related LDs (Concerns #1-5, LD-32, LD-38 chaos, LD-41, LD-43, LD-44, LD-45):** add a single authoritative pointer to `_bmad-output/test-artifacts/test-design.md` near the top of the testing concerns; existing LD bodies stand unmodified (test-design.md is a *consolidation* of the strategy these LDs already encode, not a *change* to them).
- **Project Tree Amendment** (already present at architecture.md:1236): add `commitlint.config.cjs`, `cliff.toml`, `.github/ISSUE_TEMPLATE/story.md`, `.github/labels.yml`, `.github/workflows/sync-issues.yml`, `.github/workflows/commitlint.yml` (or fold into existing `pr.yml`).

Status `complete` preserved with incremental updates + justification line per status discipline.

### 2.3 Epics impact — four new Epic-1 stories, one AC update, one cleanup

Epic 1 (Foundation & CI Baseline) is the natural home for all four new stories. It currently contains Stories 1.1 through 1.12 (per architecture.md:343-369 first-implementation preview) — adding four foundation stories does **not** violate the Process Discipline A.4 sizing rule ("target 5-10 stories per epic, ~7-15h each") because Epic 1 is explicitly a scaffold epic where story count is bound by setup-task count, not by feature decomposition.

**New stories (insertion order at end of Epic 1, before the Epic 2 boundary):**

- **Story 1.13: Bootstrap GitHub organization + private repo + label scheme + Project board.** Creates the `orgsidian` org via `gh api`, creates `orgsidian/orgsidian` private repo, applies `.github/labels.yml` (epic / milestone / status / type), creates the GitHub Project v2 board with 4 columns (Backlog / In Progress / Review / Done) and 2 views (filtered by milestone v0.1, by epic).
- **Story 1.14: Configure commitlint + husky commit-msg hook + CI gate.** Installs `@commitlint/cli` + `@commitlint/config-conventional`, adds `commitlint.config.cjs`, wires `husky` `commit-msg` hook, adds CI job that runs `commitlint --from origin/main --to HEAD` on PRs, adds PR-title check via GitHub Action.
- **Story 1.15: Configure `git-cliff` for CC → CHANGELOG generation.** Installs `git-cliff`, adds `cliff.toml` encoding the CC-type → Keep-a-Changelog-bucket mapping from §3.2 below, wires release pipeline to regenerate `CHANGELOG.md` (root + `crates/orgsidian-plugin-api/CHANGELOG.md`) on `cargo-release` (LD-33).
- **Story 1.16: GitHub Issues sync — one issue per story.** Adds a `.github/workflows/sync-issues.yml` workflow that on changes to `epics.md` runs a script (`tools/issues-sync/`) extracting Story N.M → ensure an open Issue exists per story with: title `[Story N.M] <title>`, body with the user story + ACs + Traces, labels (`epic:N`, `milestone:v0.X`, `status:backlog`), Project board placement (Backlog column). Idempotent — re-running converges. Reverse direction (Issue closure → epics.md status) deferred; this is one-way push v0.1.

**AC update on existing Story 1.10** (CONTRIBUTING.md scope): add an AC for Conventional Commits section + an AC pointing to `_bmad-output/test-artifacts/test-design.md` as authoritative testing strategy.

**Preamble update** (Process Discipline section, around line 278): add one paragraph after rule G referencing test-design.md as the binding testing strategy.

**Existing duplication cleanup** (no-brainer fixup, mentioned for transparency): Story 1.10 currently appears **twice** verbatim in `epics.md` (lines 543-556 and 592-603). The second copy is removed in the same change set.

**Repo visibility flip** at v0.1 Alpha: add a single AC line to **Story 6.6** (or Story 6 release artifact story — pick the natural anchor in Epic 6) that flips repo visibility from private → public before SM-1 announcement. Listed under §4.4 below — flagged for confirmation since I don't have line-numbers for the exact Story 6.x anchor without re-reading Epic 6 in full.

Status `complete` preserved with incremental updates + justification line.

### 2.4 Test-design.md impact — none

`test-design.md` is referenced as authoritative *input* in the other docs. It is not edited by this proposal. Status `workflowStatus: completed` preserved.

### 2.5 Technical / code impact

No production-code changes required by this proposal. All four new stories ship config + workflow files only:

- `commitlint.config.cjs` (root)
- `cliff.toml` (root)
- `.github/labels.yml`
- `.github/ISSUE_TEMPLATE/story.md`
- `.github/workflows/sync-issues.yml`
- `.github/workflows/commitlint.yml` (or fold into `pr.yml`)
- `tools/issues-sync/` (Rust binary outside `[workspace.members]`, consistent with the `tools/corpus-extractor/` precedent)
- `.husky/commit-msg`
- `CONTRIBUTING.md` (new sections — file was already in Story 1.10 scope)

---

## 3. Decisions & Rationale

### 3.1 GitHub Project board — DECIDED: YES, simple kanban

- **Decision:** add a GitHub Project v2 board (org-level project `orgsidian/projects/1`) with 4 columns (Backlog / In Progress / Review / Done) and 2 saved views (filtered by milestone v0.1, by epic).
- **Rationale:** 104 stories × 18 months exceeds the threshold at which linear scanning becomes painful even for a solo dev. The Project board adds ~1 hour of setup (Story 1.13) and gives at-a-glance status; the cost is negligible against the discipline payoff. Labels + milestones alone would work but require manual queries; the board makes the next-action discoverable in one glance.
- **Solo-dev discipline guard:** no swim lanes, no custom fields, no automation rules beyond the issue-sync workflow placing new issues in Backlog. Keep it boring.

### 3.2 Conventional Commits → CHANGELOG bucket mapping — DECIDED

Keep a Changelog buckets: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.

| CC type / footer | CHANGELOG bucket | Notes |
|---|---|---|
| `feat` | **Added** | Default for new functionality |
| `fix` | **Fixed** | Default for bug fixes |
| `perf` | **Changed** | Performance improvements; user-visible |
| `refactor` | **Changed** *(only if user-visible)* | Default excluded; opt-in via `refactor!` or scope `refactor(public-api):` |
| `revert` | **Changed** | Plus a "Reverts #PR" footer in entry text |
| `feat!` / `fix!` / `BREAKING CHANGE:` footer | **Changed** | Prefix entry text with `⚠ BREAKING:` |
| `docs` | *(excluded)* | Internal — not user-facing |
| `style` | *(excluded)* | Internal — not user-facing |
| `test` | *(excluded)* | Internal — not user-facing |
| `build` | *(excluded)* | Internal — not user-facing |
| `ci` | *(excluded)* | Internal — not user-facing |
| `chore` | *(excluded)* | Internal — not user-facing |
| `Deprecated` / `Security` | *(manual entries)* | No CC type maps 1:1; deliberate manual entries on `cargo release` |

**Tool:** `git-cliff` (Rust, integrates with `cargo-release` per LD-33). Config in `cliff.toml` at repo root encoding the table above as `[changelog.body]` + `[git.commit_parsers]` groupings.

**Scope discipline:** CC `scope` (parentheses) is **optional but encouraged**; recommended scopes are crate names (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) or `shell-ui`, `docs`, `ci`. No hard validation on scope values in commitlint config — would create false-positive friction.

### 3.3 Repo visibility timing — DECIDED with flag

- **Decision:** repo is **private** from creation (Story 1.13) through pre-Alpha development; flipped to **public** at the v0.1 Alpha release tag (Story 6.x — see flag in §4.4 below).
- **Rationale:** SM-1 ("Announcement post on HN/Reddit r/orgmode gathers 50+ technical comments + 10+ early adopters") is the moment public visibility creates value, not earlier. Public-from-day-1 invites premature observers and forces premature polish; private-during-dev keeps the focus on shipping the spike output and v0.1 features.
- **Constraint:** Story 13.x and any release-pipeline stories that reference "GitHub Releases" must work *after* the flip (most stories already do — `gh release create` works on private repos via authenticated `gh`).

### 3.4 Test-design.md reference strategy — DECIDED

- **Decision:** treat `_bmad-output/test-artifacts/test-design.md` as the **authoritative system-level testing strategy** referenced by architecture and epics. The existing LD-32/41/43/44/45 entries in architecture.md remain unchanged (they encode discrete decisions; test-design.md *consolidates* them into a strategy document but does not contradict them).
- **Pointer location:** one paragraph at the head of `### Cross-Cutting Concerns` in architecture.md and one paragraph in the Process Discipline section of epics.md.
- **No content duplication:** the architecture and epics never restate test-design.md content; they link to it.

---

## 4. Detailed Change Proposals

### 4.1 PRD — `_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md`

**Edit 1 — `revisions:` array entry (frontmatter, line 6-10 region).**

```diff
 revisions:
   - date: 2026-05-19
     summary: PRD reconciliation post-architecture (LD-46). §7.3, §10 OQ-1/OQ-2/OQ-8, addendum §A.2 and §A.3 updated to reflect MIT (LD-1) + tree-sitter-org + custom semantic layer (LD-3) + Tauri 2.x (LD-1..LD-10).
   - date: 2026-05-19
     summary: PRD reconciliation wave 2. §7.3 + §10 OQ-1 + addendum §A.2 thread in `tree-sitter-org` vendoring & maintenance contingency (architecture LD-48). §8 names i18n library and translator-facing catalog format per architecture LD-52 (Lingui v6.x; `.po` Gettext). §10 OQ-6 customization-template language updated to reflect LD-53 (Typst `.typ` for PDF path, HTML/CSS for HTML path; `sys.inputs` schema generated from `ReportData`).
+  - date: 2026-05-19
+    summary: PRD body unchanged. Development-infrastructure constraints (private GitHub repo `orgsidian/orgsidian` flipped to public at v0.1 Alpha tag; Conventional Commits enforcement; system-level test strategy at `_bmad-output/test-artifacts/test-design.md` as authoritative) absorbed by architecture LD-5/LD-33/LD-54/LD-55 and epics Stories 1.13-1.16. PRD §6.1 "Public repository" bullet now explicitly anchored at v0.1 Alpha release tag (no inline edit needed; aligned by construction). §7.1/§7.2 commitments untouched (runtime privacy ≠ source-host privacy).
```

**Edit 2 — `updated:` field bump.** Set to today's date if it changed; otherwise leave.

**No PRD body edits.**

---

### 4.2 Architecture — `_bmad-output/planning-artifacts/architecture.md`

**Edit 1 — LD-5 (line 64).** Add visibility-timing clause at the end of the existing bullet:

```diff
-- **LD-5. Monorepo: `@orgsidian/core` (pure logic) + `@orgsidian/shell` (Tauri app) + `@orgsidian/cli` (headless CLI, reopened per Party Mode).** In-process boundary between core and shell. CLI consumes `core` only — no shell dependency. GitHub organization `orgsidian` hosting the monorepo; org reserved as namespace for v2+ ancillary repos. Boundary enforcement via `eslint-plugin-boundaries` equivalent for Rust (workspace member visibility rules) + CI checks for cyclic dependencies.
+- **LD-5. Monorepo: `@orgsidian/core` (pure logic) + `@orgsidian/shell` (Tauri app) + `@orgsidian/cli` (headless CLI, reopened per Party Mode).** In-process boundary between core and shell. CLI consumes `core` only — no shell dependency. GitHub organization `orgsidian` (newly created — Story 1.13) hosting the monorepo at `orgsidian/orgsidian`; **repo is private during pre-Alpha development and flipped to public at the v0.1 Alpha release tag** (Story 6.x release artifact, before SM-1 announcement). Org reserved as namespace for v2+ ancillary repos. Boundary enforcement via `eslint-plugin-boundaries` equivalent for Rust (workspace member visibility rules) + CI checks for cyclic dependencies.
```

**Edit 2 — LD-33 (line 522).** Replace existing one-liner about conventional commits with the full enforcement spec, and add cross-references to LD-54/LD-55:

```diff
-**LD-33. Release automation.** **`cargo-release`** for the Rust workspace (workspace-aware versioning). All Rust crates (including `orgsidian-plugin-api`) share the app version with tag scheme `v*` during v0.1 → v1.4; `orgsidian-plugin-api` is internal to the monorepo and not published to crates.io until v1.5+. At v1.5+, `orgsidian-plugin-api` separates with its own SemVer cadence and tag scheme `plugin-api-v*` when external publication begins. JS `shell-ui` version-synced with `shell-app`. CHANGELOG.md per crate + project root. Conventional commits enable semi-automated changelog generation.
+**LD-33. Release automation.** **`cargo-release`** for the Rust workspace (workspace-aware versioning). All Rust crates (including `orgsidian-plugin-api`) share the app version with tag scheme `v*` during v0.1 → v1.4; `orgsidian-plugin-api` is internal to the monorepo and not published to crates.io until v1.5+. At v1.5+, `orgsidian-plugin-api` separates with its own SemVer cadence and tag scheme `plugin-api-v*` when external publication begins. JS `shell-ui` version-synced with `shell-app`. CHANGELOG.md per crate + project root. CHANGELOG generation is fully automated via **`git-cliff`** (`cliff.toml` at repo root) consuming Conventional Commits (see LD-54) on every `cargo release`. CHANGELOG manual entries (`Deprecated` / `Security`) inserted before tag in `cargo release` hook. See LD-54 (commit enforcement chain) and LD-55 (GitHub Issues sync + Project board) for the surrounding workflow.
```

**Edit 3 — NEW LD-54** (insert after LD-53, before §Decision Impact Analysis at line 581):

```markdown
**LD-54. Conventional Commits enforcement + CHANGELOG mapping.**

**Specification.** All commits, PR titles, and CHANGELOG entries follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/). Type vocabulary: `feat`, `fix`, `perf`, `refactor`, `revert`, `docs`, `style`, `test`, `build`, `ci`, `chore`. Breaking changes signalled by `!` (e.g., `feat!:`) or `BREAKING CHANGE:` footer. Scope is optional but recommended; canonical scopes are crate names (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) or `shell-ui` / `docs` / `ci`.

**Enforcement chain (Story 1.14):**

- `commitlint.config.cjs` extends `@commitlint/config-conventional` (no scope-value validation — encouraged not required).
- `husky` `commit-msg` hook runs `commitlint --edit "$1"` (in addition to existing pre-commit hook per Linting & Formatting section at line 785).
- `.github/workflows/pr.yml` (or dedicated `commitlint.yml`) runs `commitlint --from origin/main --to HEAD` on every PR.
- PR title check via `amannn/action-semantic-pull-request` (or equivalent) on `pull_request_target` events.

**CHANGELOG mapping** (encoded in `cliff.toml` `[git.commit_parsers]`):

| CC type / footer | Keep-a-Changelog bucket | Notes |
|---|---|---|
| `feat` | **Added** | |
| `fix` | **Fixed** | |
| `perf` | **Changed** | User-visible improvement |
| `refactor` | **Changed** | Only if user-visible (`refactor!` or scope `public-api`/crate-public-surface) |
| `revert` | **Changed** | Entry text includes "Reverts #N" |
| `feat!` / `fix!` / `BREAKING CHANGE:` | **Changed** | Entry prefixed with `⚠ BREAKING:` |
| `docs` / `style` / `test` / `build` / `ci` / `chore` | *(excluded)* | Internal commits |
| `Deprecated` / `Security` (no CC type) | *(manual entries)* | Inserted before `cargo release` tag |

**Generation tool:** `git-cliff` invoked by `cargo release` pre-tag hook; output overwrites `Unreleased` section of `CHANGELOG.md` (root) and bumps to versioned heading at release time. `crates/orgsidian-plugin-api/CHANGELOG.md` follows the same flow but scoped to commits touching `crates/orgsidian-plugin-api/**` (per LD-33 separation policy at v1.5+).

**CONTRIBUTING.md section** (Story 1.10 AC update) documents the CC vocabulary, scope discipline, examples per type, and the mapping table above.
```

**Edit 4 — NEW LD-55** (insert after LD-54):

```markdown
**LD-55. GitHub Issues sync + label scheme + Project board.**

**Specification.** Every Story N.M in `epics.md` is mirrored as a GitHub Issue in `orgsidian/orgsidian` (one issue per story). Issues, labels, and a single org-level GitHub Project v2 board form the work-tracking surface.

**Label scheme** (`.github/labels.yml`, applied by `actions/github-script` or equivalent at Story 1.13):

- **Epic labels:** `epic:1` … `epic:13` (one per epic).
- **Milestone labels:** `milestone:v0.1`, `milestone:v0.5`, `milestone:v1.0` (in addition to native GitHub milestones for date tracking).
- **Status labels:** `status:backlog`, `status:in-progress`, `status:review`, `status:blocked`, `status:done`.
- **Type labels:** `type:story`, `type:bug`, `type:spike`, `type:chore`, `type:docs`, `type:security`.
- **Priority labels** (used sparingly): `priority:p0`, `priority:p1`.

**Issue body template** (`.github/ISSUE_TEMPLATE/story.md`) renders: persona, user-story, AC list, `Traces:` line, `Microcopy` flag, link back to `epics.md#story-N-M`.

**Project board** (Story 1.13): org-level Project v2 at `orgsidian/projects/1`, name "Orgsidian Roadmap". Columns: **Backlog** / **In Progress** / **Review** / **Done**. Two saved views: filtered by `milestone:v0.X`, grouped by `epic:N`. No swim lanes, no custom fields, no automation rules beyond the issue-sync workflow placing new issues in Backlog. Solo-dev discipline guard: do not add complexity unless a pain-point in v0.1 demonstrates a need.

**Sync automation** (Story 1.16): `.github/workflows/sync-issues.yml` runs on push to `main` when `_bmad-output/planning-artifacts/epics.md` changes. A small Rust binary at `tools/issues-sync/` (outside `[workspace.members]`, same convention as `tools/corpus-extractor/`) parses epics.md, extracts Story headers + bodies + `Traces:`, and uses the GitHub REST API (`gh api` or `octocrab`) to:

1. Ensure an Issue exists per Story with title `[Story N.M] <story title>` and body per template.
2. Apply labels (`epic:N`, `milestone:v0.X` derived from epic-to-milestone mapping in `epics.md` §Epic List, `status:backlog` default, `type:story`).
3. Add the issue to the GitHub Project board (Backlog column) if not already present.
4. Idempotent: re-running converges; closed issues stay closed; status-label drift not corrected (manual is authoritative once an issue is open).

**Direction:** one-way push (epics.md → Issues) in v0.1. Reverse direction (Issue closure → epics.md `status: done` annotation) deferred — likely never needed for a solo workflow.

**Repo visibility:** the org and repo are created **private** at Story 1.13 and remain private through pre-Alpha; flip to public is part of the v0.1 Alpha release artifact stories (Epic 6, anchored before SM-1 announcement). See LD-5.
```

**Edit 5 — Cross-Cutting Concerns header (around line 78).** Add a one-paragraph pointer before the numbered list:

```diff
 ### Cross-Cutting Concerns

+The system-level testing strategy consolidating Concerns #1-7 below, plus the risk-prioritized coverage plan for v0.1 → v1.0, is authored as a standalone artifact at **`_bmad-output/test-artifacts/test-design.md`** (TEA workflow, 2026-05-19). That document is the binding strategy for every story's red-phase scaffold (Process Discipline rule A); the LD entries below are referenced by it (not superseded). Implementing AI agents follow `test-design.md` § per-story-type scaffolds + this section's LD constraints jointly.
+
 1. **Round-trip fidelity** — three-level test oracle (Murat): L0 byte-identical save-no-op (CI gate hard), L1 semantic-preserving surgical edit (property-based with `proptest` or `fast-check`), L2 Emacs ground-truth via `emacs --batch` AST comparison on a subset corpus.
```

**Edit 6 — Project Tree Amendment (line 1236 region).** Append rows for:

```
├── commitlint.config.cjs                   # LD-54 enforcement
├── cliff.toml                              # LD-54 CHANGELOG generation
├── .husky/
│   └── commit-msg                          # LD-54 client-side gate
├── .github/
│   ├── labels.yml                          # LD-55 label scheme
│   ├── ISSUE_TEMPLATE/
│   │   └── story.md                        # LD-55 issue template
│   └── workflows/
│       ├── commitlint.yml                  # LD-54 CI gate (or folded into pr.yml)
│       └── sync-issues.yml                 # LD-55 epics.md → Issues
└── tools/
    └── issues-sync/                        # LD-55 sync binary (outside [workspace.members])
```

(Place these alongside the existing `tools/corpus-extractor/` line and the existing `.github/workflows/` block.)

**Frontmatter status update.** Add an entry under a new `revisions:` array (architecture currently has no such field — adopt the same pattern as PRD):

```diff
 ---
 stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
 inputDocuments:
   - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
   - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/addendum.md
   - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/reconcile-brainstorming.md
   - _bmad-output/brainstorming/brainstorming-session-2026-05-18-1613.md
 workflowType: 'architecture'
 project_name: 'orgsidian'
 user_name: 'Tiziano'
 date: '2026-05-19'
 lastStep: 8
 status: 'complete'
 completedAt: '2026-05-19'
+revisions:
+  - date: 2026-05-19
+    summary: Sprint Change Proposal (correct-course) absorbed. LD-5 amended with repo-visibility timing. LD-33 updated with full CC-enforcement + git-cliff chain. NEW LD-54 (Conventional Commits + CHANGELOG mapping). NEW LD-55 (GitHub Issues sync + Project board). Cross-Cutting Concerns header pointer to `_bmad-output/test-artifacts/test-design.md`. Project Tree amended with commitlint/cliff/husky/sync-issues config. No body content of LD-1..LD-53 modified.
 ---
```

---

### 4.3 Epics — `_bmad-output/planning-artifacts/epics.md`

**Edit 1 — Process Discipline section (after rule G, around line 339).** Add new rule H:

```markdown
### H. System-Level Testing Strategy (LD-54..LD-55 context)

The binding system-level test strategy lives at **`_bmad-output/test-artifacts/test-design.md`** (TEA workflow, 2026-05-19). Per-story red-phase scaffolds (rule A) instantiate the per-story-type scaffolds defined in §7.3 of that document. Coverage targets (§8) and quality gates (§10) of `test-design.md` are CI-enforced via the LD-32 matrix + Story 1.11 failure-mode harness + Story 1.12 perf snapshot infrastructure. Stories below do not duplicate test-design.md content; they reference it by section number where relevant.
```

**Edit 2 — Story 1.10 (line 543-556).** Update AC to add Conventional Commits + test-strategy references:

```diff
 ### Story 1.10: Add `SECURITY.md` + `ARCHITECTURE.md` + `CHANGELOG.md` + `CONTRIBUTING.md`

 As the **author / contributor**,
 I want root-level project hygiene docs in place,
 So that contributors and security researchers have a navigable map from day-1 of the public repository.

 **Acceptance Criteria:**

 **Given** Story 1.1 scaffold,
 **When** the docs are committed,
 **Then** root `SECURITY.md` declares a 14-day patch SLA + GitHub Security Advisories reporting channel + 90-day coordinated disclosure default per LD-37
 **And** root `ARCHITECTURE.md` contains a high-level summary + Mermaid crate dependency graph + link to `docs/architecture.md`
 **And** root `CHANGELOG.md` is initialized in Keep-a-Changelog format with an `Unreleased` heading
-**And** root `CONTRIBUTING.md` documents the development setup, fixture placement rule (co-located by default; promoted to root `fixtures/` only when ≥2 crates consume), and the FR traceability discipline (`Implements FR-NN` doc-comment header).
+**And** root `CONTRIBUTING.md` documents the development setup, fixture placement rule (co-located by default; promoted to root `fixtures/` only when ≥2 crates consume), the FR traceability discipline (`Implements FR-NN` doc-comment header), the Conventional Commits vocabulary + scope discipline + CHANGELOG mapping table per LD-54, and a "Testing strategy" section pointing to `_bmad-output/test-artifacts/test-design.md` as the authoritative system-level test strategy.
+
+**Traces:** LD-37 (SECURITY.md), LD-54 (CONTRIBUTING.md CC section).
```

**Edit 3 — Remove duplicate Story 1.10 block (lines 592-603).** The block at 592-603 is a verbatim duplicate of 543-556 — delete it cleanly. No replacement.

**Edit 4 — NEW Story 1.13** (insert before the Epic 2 boundary at line 605, after Story 1.12):

```markdown
### Story 1.13: Bootstrap GitHub organization + private repo + label scheme + Project board

As the **author / contributor**,
I want the `orgsidian` GitHub organization created with a private `orgsidian/orgsidian` repo, a normalized label scheme, and a single Project v2 kanban board,
So that work tracking is in place before Epic 2 begins.

**Acceptance Criteria:**

**Given** an authenticated `gh` CLI with org-creation privileges,
**When** Story 1.13 is executed,
**Then** the `orgsidian` GitHub organization exists (created via `gh api orgs` or web UI; idempotent if pre-existing)
**And** `orgsidian/orgsidian` private repo exists with default branch `main` and the local Story 1.1 scaffold pushed
**And** `.github/labels.yml` declares the LD-55 label scheme (`epic:1..13`, `milestone:v0.1|v0.5|v1.0`, `status:backlog|in-progress|review|blocked|done`, `type:story|bug|spike|chore|docs|security`, `priority:p0|p1`)
**And** a labels-sync workflow (`actions/github-script` or `crazy-max/ghaction-github-labeler`) applies `.github/labels.yml` on push to `main`
**And** GitHub Project v2 `orgsidian/projects/1` exists with name "Orgsidian Roadmap" and 4 columns (Backlog / In Progress / Review / Done)
**And** the Project has two saved views: "By Milestone v0.1" (filter `milestone:v0.1`) and "By Epic" (group by `epic:N` label)
**And** `.github/ISSUE_TEMPLATE/story.md` exists with the LD-55 template fields (persona, user story, AC list, `Traces:` line, `Microcopy` flag, link to epics.md anchor).

**Traces:** LD-5 (repo location + visibility), LD-55 (label scheme + Project board).
```

**Edit 5 — NEW Story 1.14**:

```markdown
### Story 1.14: Configure commitlint + husky commit-msg hook + CI gate

As the **author / contributor**,
I want commitlint enforcing Conventional Commits v1.0.0 locally (husky `commit-msg`) and on CI (per-PR job + PR-title check),
So that every commit and PR title qualifies for `git-cliff` CHANGELOG ingestion per LD-54.

**Acceptance Criteria:**

**Given** Story 1.3 frontend setup (husky already on disk per pre-commit hook),
**When** Story 1.14 is executed,
**Then** `package.json` lists `@commitlint/cli` and `@commitlint/config-conventional` at latest stable
**And** `commitlint.config.cjs` at repo root declares `module.exports = { extends: ['@commitlint/config-conventional'] }` with no scope-value enum (encouraged not required)
**And** `.husky/commit-msg` runs `pnpm commitlint --edit "$1"` and fails the commit on a non-conforming message
**And** `.github/workflows/pr.yml` (or a dedicated `commitlint.yml`) adds a step that runs `pnpm commitlint --from origin/main --to HEAD` and fails the PR on any non-conforming commit
**And** an additional CI step using `amannn/action-semantic-pull-request@v5` validates the PR title itself against Conventional Commits
**And** a smoke test confirms that a deliberately-malformed local commit (`git commit -m "broken message"`) is rejected by the `commit-msg` hook
**And** a smoke test confirms that a deliberately-malformed PR title triggers the CI title check failure.

**Traces:** LD-54 (enforcement chain).
```

**Edit 6 — NEW Story 1.15**:

```markdown
### Story 1.15: Configure `git-cliff` for CC → CHANGELOG generation

As the **author / contributor**,
I want `git-cliff` invoked by `cargo release` to regenerate `CHANGELOG.md` from Conventional Commits per the LD-54 mapping table,
So that every release ships an accurate, automation-generated user-facing changelog without manual curation of `feat`/`fix`/`perf` entries.

**Acceptance Criteria:**

**Given** Story 1.14 (commitlint live) and LD-33 release automation context,
**When** Story 1.15 is executed,
**Then** `git-cliff` is installed as a `cargo install` step in the release pipeline (or pinned in `Cargo.toml` `[workspace.metadata]` for reproducibility)
**And** `cliff.toml` at repo root encodes the LD-54 mapping table as `[git.commit_parsers]` (per-CC-type group assignment) + `[changelog.body]` template producing Keep-a-Changelog format with `Added`/`Changed`/`Deprecated`/`Removed`/`Fixed`/`Security` headings
**And** `cargo release` `[hooks.pre-release]` invokes `git-cliff --unreleased --tag <version> --prepend CHANGELOG.md`
**And** a second `git-cliff` invocation scoped to `crates/orgsidian-plugin-api/**` paths regenerates `crates/orgsidian-plugin-api/CHANGELOG.md` (LD-33 separate-changelog discipline)
**And** a smoke test runs `git-cliff --unreleased` against a fixture branch with one `feat:`, one `fix:`, one `perf:`, one `feat!:`, one `chore:` commit and asserts: the `chore:` is excluded, the `feat:` lands under Added, the `fix:` under Fixed, the `perf:` under Changed, and the `feat!:` lands under Changed with a `⚠ BREAKING:` prefix in its entry text
**And** the `Deprecated` and `Security` headings remain present-but-empty when no manual entries exist (template allows empty sections).

**Traces:** LD-33 (release automation), LD-54 (CHANGELOG mapping).
```

**Edit 7 — NEW Story 1.16**:

```markdown
### Story 1.16: GitHub Issues sync — one issue per story

As the **author / contributor**,
I want a one-way sync from `_bmad-output/planning-artifacts/epics.md` to GitHub Issues in `orgsidian/orgsidian` (one issue per Story N.M, idempotent re-runs),
So that the Project board (Story 1.13) and Issue search become navigable surfaces over the 104-story roadmap without manual re-typing.

**Acceptance Criteria:**

**Given** Stories 1.13 (org/repo/labels/Project exist) and 1.10 (CONTRIBUTING.md docs the sync),
**When** Story 1.16 is executed,
**Then** `tools/issues-sync/` exists as a Rust binary (Cargo.toml with `publish = false`, outside `[workspace.members]` per LD-5 convention for `tools/corpus-extractor/`)
**And** the binary parses `epics.md` and extracts each `### Story N.M: <title>` block including persona, user-story, AC list, `Traces:` line, and any flags (`[Microcopy: draft|final]`)
**And** the binary uses `octocrab` (or `gh api` via `std::process::Command` wrapper) to ensure-exists each Issue with title `[Story N.M] <title>`, body rendered per `.github/ISSUE_TEMPLATE/story.md`, labels (`epic:N`, `milestone:v0.X` derived from §Epic List milestone-to-epic mapping, `status:backlog` if new, `type:story`)
**And** the binary places each newly-created Issue into the GitHub Project v2 Backlog column (using Projects v2 GraphQL `addProjectV2ItemById`)
**And** re-running the binary on the same `epics.md` is idempotent — no duplicate issues created, no label thrash, no Project board re-shuffle
**And** `.github/workflows/sync-issues.yml` runs the binary on push-to-main when `_bmad-output/planning-artifacts/epics.md` changes (path filter), with `GITHUB_TOKEN` scoped to issues+projects write
**And** a smoke test against a 2-story fixture `epics-fixture.md` creates 2 issues with correct labels and project placement; a second smoke run with the same fixture creates 0 new issues
**And** a deliberate `status:` label drift (e.g., manually changing an issue to `status:in-progress`) is NOT reset by the sync — manual is authoritative once an issue is open
**And** the workflow is documented in CONTRIBUTING.md alongside the LD-55 reference.

**Traces:** LD-55 (Issues sync + Project board).
```

**Edit 8 — Repo visibility flip (Epic 6)** — FLAGGED for confirmation. Add a single AC line to the Epic 6 story that owns the v0.1 Alpha release artifact (likely **Story 6.6** "Publish v0.1 Alpha + announcement", or its current equivalent — I did not enumerate Epic 6 stories in this proposal pass; user to confirm exact anchor). Suggested AC text:

```markdown
**And** the `orgsidian/orgsidian` repository visibility is flipped from private to public before the SM-1 announcement post is published (`gh api -X PATCH /repos/orgsidian/orgsidian -f visibility=public` or web UI), and a smoke check confirms the public README + LICENSE render at the public URL.
```

**Edit 9 — Frontmatter `revisions:` array** (epics.md currently has no `revisions:` field — adopt the same pattern as PRD):

```diff
 status: complete
 completedAt: '2026-05-19'
+revisions:
+  - date: 2026-05-19
+    summary: Sprint Change Proposal (correct-course) absorbed. NEW Stories 1.13-1.16 (GitHub org + Project board + commitlint/husky + git-cliff + Issues sync). Story 1.10 AC extended (CC section + test-strategy pointer in CONTRIBUTING.md). Duplicate Story 1.10 block at former lines 592-603 removed. Process Discipline rule H added pointing to `_bmad-output/test-artifacts/test-design.md`. Epic 6 release-artifact story to be updated with repo-visibility flip AC (anchor flagged for confirmation). No other story content modified.
 partyModeRounds: 2
```

---

## 5. Implementation Handoff

**Scope classification:** **Moderate.** No fundamental replan; cross-document propagation of three named constraints into one already-final PRD (revision-array only), one already-complete architecture (one LD update + two new LDs + cross-cutting pointer + tree amendment), and one already-complete epics (four new Epic-1 stories + one AC update + one duplicate cleanup + one preamble paragraph). No production code yet — these stories ship config/workflow files.

**Recipients & responsibilities:**

- **Tiziano (Product + Architect + QA seat):** approve the proposal, resolve the Epic 6 anchor flag for the repo-visibility-flip AC (§4.3 Edit 8), then route to bmad-quick-dev or bmad-dev-story for Stories 1.13-1.16 implementation when Epic 1 sequencing reaches them.
- **bmad-create-story (downstream):** consume Stories 1.13-1.16 as written; bmad-testarch-atdd then scaffolds red-phase tests per rule A.

**Success criteria:**

1. PRD revisions array contains the 2026-05-19 entry; PRD body unchanged.
2. Architecture LD-5 carries the repo-visibility timing clause; LD-33 carries the CC + git-cliff enforcement chain; LD-54 and LD-55 exist with full bodies; Cross-Cutting Concerns header points to test-design.md; project tree amended.
3. Epics carries Stories 1.13-1.16 with passing red-phase ATDD scaffolds (deferred to per-story bmad-testarch-atdd runs); Story 1.10 AC extended; duplicate block removed; rule H in place.
4. `git-cliff` smoke fixture (Story 1.15) demonstrates the CC → CHANGELOG mapping table is faithfully encoded.
5. Issue sync smoke fixture (Story 1.16) demonstrates idempotent re-run with zero duplicates.

**Out of scope (deliberately not addressed in this correct-course pass):**

- Reverse-direction sync (Issue closure → epics.md `status: done`). Deferred per LD-55; revisit if v0.5 Beta usage demonstrates a need.
- Web-UI for the Project board beyond the 2 default views. Solo-dev discipline guard.
- Migration to bidirectional `git-cliff` (changelog drift detection). Out of scope; current flow is one-shot regeneration at release time.

---

## 6. Decisions Flagged for Explicit Confirmation

These are decisions the proposal makes autonomously but flags here so Tiziano can override before approval:

1. **GitHub Project board: kanban with 4 columns + 2 saved views.** Alternative would be labels + milestones only (no board). My decision: include the board (§3.1 rationale). **Override?** y/n.
2. **CC → CHANGELOG mapping table (§3.2).** Alternative mappings exist (e.g., `refactor` always under Changed; `perf` separate from Changed). My decision: the table in §3.2. **Override?** y/n.
3. **Repo visibility flip at v0.1 Alpha release tag.** Alternative: public from day 1 (Story 1.13), or public at a later milestone (v0.5). My decision: flip at v0.1 (§3.3). **Override?** y/n.
4. **Epic 6 anchor for the visibility-flip AC.** I did not enumerate Epic 6 stories in this pass. Likely target: the story owning the v0.1 Alpha release artifact + announcement (your Epic 6 ends with `Closes SM-1`). **Confirm anchor?** Provide Story 6.X identifier or instruct me to re-scan Epic 6.
5. **Tool: `git-cliff` for CHANGELOG.** Alternative: `cocogitto`, `release-please`, or manual. My decision: `git-cliff` (Rust-native, integrates cleanly with `cargo-release` per LD-33). **Override?** y/n.
6. **`commitlint` scope-value enum: off (encouraged, not required).** Alternative: strict enum on crate names + `shell-ui`/`docs`/`ci`. My decision: off to reduce false-positive friction. **Override?** y/n.

---

*End of Sprint Change Proposal. Awaiting approval per workflow Step 5.*
