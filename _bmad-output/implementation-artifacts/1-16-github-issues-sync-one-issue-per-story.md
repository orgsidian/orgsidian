# Story 1.16: GitHub Issues sync — one issue per story

Status: done

## Metadata

github_issue: 16

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want a Rust binary at [`tools/issues-sync/`](tools/issues-sync/) that performs a one-way idempotent sync from [`_bmad-output/planning-artifacts/epics.md`](_bmad-output/planning-artifacts/epics.md) to GitHub Issues in [`orgsidian/orgsidian`](https://github.com/orgsidian/orgsidian) (one Issue per `### Story N.M`, preserving `status:*` label drift, placing newly-created Issues into the [LD-55 Project v2 board](_bmad-output/planning-artifacts/architecture.md#L617-L631) Backlog column), wired into a [`.github/workflows/sync-issues.yml`](.github/workflows/sync-issues.yml) workflow triggered on push-to-main when `epics.md` changes,
So that the Project board (anchored by [Story 1.13](_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md)) and Issue search become navigable surfaces over the 117-story roadmap (live count on 2026-05-29; the original spec text "104-story roadmap" was an authoring-snapshot estimate — the parser regression test in `parser.rs` locks the current floor at 117 per the AC2 escape-hatch, recalibrate when stories are added/removed) without manual re-typing — and the ad-hoc bootstrap shell script at [`scripts/sync-epics-to-github.sh`](scripts/sync-epics-to-github.sh) (created during the 2026-05-19 correct-course step explicitly to be replaced here per its own line 7-8 comment) is retired in favor of a typed, testable, CI-runnable Rust artifact that lives inside the workspace's project-tree slot reserved at [architecture.md:1427](_bmad-output/planning-artifacts/architecture.md#L1427).

## Acceptance Criteria

**AC1 — `tools/issues-sync/` exists as a standalone Rust binary outside `[workspace.members]` (LD-5 convention, mirroring [`tools/corpus-extractor/`](tools/corpus-extractor/)).**

- **NET-NEW directory** at `tools/issues-sync/`. Slot reserved at [architecture.md:1427](_bmad-output/planning-artifacts/architecture.md#L1427).
- `tools/issues-sync/Cargo.toml`:
  - `[package]` block: `name = "orgsidian-issues-sync"`, `version = "0.0.0"`, `edition = "2021"`, `license = "MIT"`, `publish = false`, `description = "Sync _bmad-output/planning-artifacts/epics.md → orgsidian/orgsidian GitHub Issues (LD-55, Story 1.16)"`.
  - `[[bin]]` block: `name = "orgsidian-issues-sync"`, `path = "src/main.rs"`.
  - `[lib]` block: `name = "orgsidian_issues_sync"`, `path = "src/lib.rs"` — parser + body renderer + sync logic live in the lib so unit tests can exercise them WITHOUT touching network.
  - **MIRROR** the [`tools/corpus-extractor/Cargo.toml`](tools/corpus-extractor/Cargo.toml) shape: bare `[package]` + `[dependencies]`, no `[workspace]`, no `[features]`. **DO NOT** add a `[workspace]` block — that would make it a separate workspace, breaking the convention.
- Root [`Cargo.toml`](Cargo.toml) `[workspace]` table: append `"tools/issues-sync"` to the existing `exclude = ["tools/corpus-extractor"]` array so `cargo build --workspace` does NOT pay the issues-sync compile cost (LD-5 leaf-isolation discipline; matches [architecture.md:1009](_bmad-output/planning-artifacts/architecture.md#L1009)).
- `tools/issues-sync/src/main.rs`: `~40 lines` — argv parsing + `tokio::main` entry that calls into `orgsidian_issues_sync::run(SyncOpts { ... }).await`.
- `tools/issues-sync/src/lib.rs`: top-level `pub fn run(opts: SyncOpts) -> Result<SyncReport>` + submodules `parser`, `render`, `github`, `sync`. **Each submodule has its own `#[cfg(test)] mod tests`** — no `tests/` integration-test directory in this story (network-touching tests are deferred to wiremock-backed `tests/` only if scope-room remains; the unit test surface is the contract).
- The implementing modules carry `//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.` as the first doc-comment line, verified by `grep -r "Implements LD-55" tools/issues-sync/src/` returning ≥1 hit.
- **DO NOT** add `tools/issues-sync/` to `[workspace.members]`. **DO NOT** introduce a `tools/` super-Cargo.toml. **DO NOT** publish this crate (epic-AC mandates `publish = false`; the binary is internal tooling).

**AC2 — Parser extracts each `### Story N.M[a-z]?` block from `epics.md` with persona, user-story, AC list, `Traces:` line, and `[Microcopy: …]` flags.**

- **NET-NEW module** `tools/issues-sync/src/parser.rs` (~150 lines).
- Public API: `pub fn parse_epics(text: &str) -> Result<Vec<Story>>` where `Story { epic: u8, num: String, title: String, line_no: u32, persona: Option<String>, user_story: Option<UserStory>, acceptance_criteria: String, traces: Option<String>, microcopy_flag: Option<MicrocopyFlag>, body_raw: String }`.
- `num` is `String` (not `u8`) because Stories like `4.3a`, `4.3b`, `4.3c`, ..., `4.3g` (and `4.3` standalone if any) MUST be parsed as distinct stories — the existing bash script regex at [scripts/sync-epics-to-github.sh:221](scripts/sync-epics-to-github.sh#L221) `^###[[:space:]]Story[[:space:]]([0-9]+)\.([0-9]+[a-z]?):[[:space:]](.+)$` captures the lowercase-letter suffix. Reproduce that exact regex semantics in Rust via the `regex` crate.
- **PARSER STATE MACHINE** (mirror the bash script's section-skip discipline):
  1. Skip everything inside the `## Epic List` overview section (lines beginning at `## Epic List` until the first `## Epic <N>:` deep-section header). The Epic List overview contains `### Epic N:` paragraphs but NO `### Story N.M:` headings — but a defensive parser should not rely on absence; the in-section skip is the authoritative gate.
  2. Inside an `## Epic <N>:` deep section, consume `### Story N.M[a-z]?: <title>` headings as story-start markers.
  3. Body buffer collects lines until the next `### Story` heading or the next `## ` h2 (which flushes the current story and ends epic-deep mode).
  4. Final flush at EOF.
- **PERSONA EXTRACTION**: regex `^As (the|a|an) \*\*(?P<persona>[^*]+?)\*\*,` against the body buffer. The first capture inside `**…**` is the persona literal (e.g., `author / contributor`, `early adopter`, `Sofia (freelance consultant)`). If no match, leave `persona = None` (some stories may have non-standard openings).
- **USER STORY EXTRACTION**: capture the three-line `As a <persona>, / I want <capability>, / so that <outcome>.` block — store the verbatim text in `user_story.raw` and parse `capability` (text between `I want ` and `,\n`) + `outcome` (between `so that ` and `.\n`) into structured fields for the rendered Issue body.
- **AC EXTRACTION**: capture the bullet block following `**Acceptance Criteria:**` until either the next `**Traces:**` line or the next `### ` heading or `## ` heading. Preserve formatting verbatim (Markdown bold, GitHub-flavored bullets, code blocks) — the Issue body needs to render identically to the epics.md source.
- **TRACES**: capture the single-line `**Traces:** LD-NN (…), LD-MM (…)` or `**Traces:** FR-NN, NFR-NN` block. Store raw text.
- **MICROCOPY FLAG**: regex `\[Microcopy:\s*(draft|final|n/a)\]` anywhere in the body. Optional; absent on most stories.
- **EPIC LINE TRACKING**: track the line number of the `### Story` heading (for the `**Source:**` body footer's `#LN` anchor). Counting starts at 1 per `wc -l` convention.
- **Tests** (`#[cfg(test)] mod tests` in `parser.rs`):
  1. Trivial 2-story fixture (inline `let fixture = r#"..."#;`) parses to exactly 2 `Story` records with correct epic/num/title.
  2. Story-4.3a-style subletter suffix parses correctly (`num == "3a"`).
  3. Epic List overview is skipped (a `### Epic 1: …` line inside Epic List doesn't yield a story; a `### Story 1.1: …` line inside Epic List would but doesn't appear there in practice — guard against future drift).
  4. Multi-paragraph body with a code block in the AC list preserves the code block verbatim.
  5. Story with no `Traces:` line parses (`traces == None`).
  6. Real `_bmad-output/planning-artifacts/epics.md` parses to **exactly 117 stories** (the current floor on 2026-05-29 — the original spec text "104-story roadmap" was an authoring-snapshot; the actual `### Story N.M[a-z]?:` count drifted to 117 by implementation time and the escape-hatch in this AC was invoked per Dev Agent Record §C). The test name is `real_epics_md_parses_to_117_stories` in `parser.rs`. This is a regression net — if a future epic-edit drops or duplicates a story heading, this test fails loud. The test loads the file via `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../_bmad-output/planning-artifacts/epics.md"))`. **Future maintainer**: if the count drifts again, update the test name + literal AND note the new floor in the Dev Agent Record (do not lower the floor silently).

**AC3 — Body renderer produces an Issue body matching the [`.github/ISSUE_TEMPLATE/story.md`](.github/ISSUE_TEMPLATE/story.md) layout, with a stable header sentinel for idempotent matching.**

- **NET-NEW module** `tools/issues-sync/src/render.rs` (~80 lines).
- Public API: `pub fn render_body(story: &Story, branch_for_links: &str, epics_file_relpath: &str) -> String`.
- The rendered body MUST match the existing live issue bodies (see [issue #16 body shape](https://github.com/orgsidian/orgsidian/issues/16) — fetched in the analysis pass; verbatim structure below):
  ```markdown
  > Auto-synced from `_bmad-output/planning-artifacts/epics.md` by `tools/issues-sync`. Manual edits below this line will be **overwritten** on next sync; status label drift is preserved.

  **Epic:** {epic} &middot; **Milestone:** {milestone}

  ---

  {body_md verbatim from parser}

  ---

  **Source:** [`{epics_file}` line {line_no}](https://github.com/{REPO}/blob/{BRANCH}/{epics_file}#L{line_no})
  ```
  **MIRROR** the existing bodies — do NOT introduce gratuitous formatting changes. Reason: idempotency. If the renderer's output differs character-for-character from the existing live bodies, the binary's first real-world run will `gh issue edit` all 108 existing issues with a no-op body change, polluting issue history. **AC9 verification cell 6** asserts byte-for-byte parity against a snapshot of issue #1's body (a `done` Story 1.1 whose body is the canonical baseline).
- **HEADER SENTINEL**: the leading blockquote `> Auto-synced from … by \`tools/issues-sync\`. …` is the lookup sentinel. The existing bash-script-authored bodies carry `> Auto-synced from … by \`scripts/sync-epics-to-github.sh\`. …`. **The first real run of the new binary WILL flip this byte** (`scripts/sync-epics-to-github.sh` → `tools/issues-sync`) on all 108 existing issues. This is acceptable churn — one-time migration, documented in the Dev Agent Record under "first real run". Subsequent runs see the new sentinel and are byte-stable.
- **MILESTONE MAPPING**: `epic ≤ 6 → v0.1`, `7 ≤ epic ≤ 12 → v0.5`, `epic ≥ 13 → v1.0`. Mirror the bash script logic at [scripts/sync-epics-to-github.sh:38-44](scripts/sync-epics-to-github.sh#L38-L44). Centralize in `lib.rs::milestone_for_epic(epic: u8) -> &'static str`. Tests: epic 1 → v0.1, epic 6 → v0.1, epic 7 → v0.5, epic 12 → v0.5, epic 13 → v1.0; epic 0 / epic 14+ → debug_assert!() (out-of-range = parser regression).
- **Tests** (`#[cfg(test)] mod tests` in `render.rs`): snapshot test against a 1-story fixture; compare to a checked-in golden file at `tools/issues-sync/tests/golden/story-1-1-body.md` (committed verbatim from `gh issue view 1 --json body --jq .body` output, with the `tools/issues-sync` sentinel substitution applied). This is the byte-stability contract.

**AC4 — `tools/issues-sync/src/github.rs` wraps the GitHub API surface (issues REST + Projects v2 GraphQL) via `octocrab 0.51`.**

- **NET-NEW module** `tools/issues-sync/src/github.rs` (~200 lines).
- **CRATE PIN**: `octocrab = "0.51"` (latest stable on crates.io as of 2026-05-29 per `cargo search octocrab`; semver-minor pin per [[feedback_version_policy]]). The `octocrab` Issues handler (`octocrab.issues(owner, repo)`) covers create / update / list / add-labels / remove-labels — verified via [the official Octocrab docs](https://docs.rs/octocrab/0.51/octocrab/issues/struct.IssueHandler.html). The GraphQL handler (`octocrab.graphql(query)`) covers the Projects v2 calls — verified via [the GraphQL example](https://github.com/XAMPPRocky/octocrab/blob/main/examples/graphql.rs).
- **DO NOT** add `graphql_client` for typed GraphQL. The Projects v2 surface in this story uses 3 GraphQL operations (`projectsV2(first:…)` to locate the project node, `addProjectV2ItemById` to add an issue, `projectV2(id:…){items}` to enumerate existing items for idempotency). Hand-rolling these as Rust string literals with serde-typed responses is ~30 lines; pulling `graphql_client` adds a schema-codegen dep + a build.rs + a `.graphql` schema file — disproportionate overhead. Reconsider only if a future story needs >5 GraphQL operations.
- **AUTH**: `Octocrab::builder().personal_token(env::var("GITHUB_TOKEN")?).build()?`. The CI workflow (AC7) injects `GITHUB_TOKEN` via `secrets.PROJECTS_PAT` (NOT the built-in `secrets.GITHUB_TOKEN` — rationale below). Locally, the maintainer runs with `GITHUB_TOKEN=$(gh auth token)` exported.
- **WHY `secrets.PROJECTS_PAT` not `secrets.GITHUB_TOKEN`**: the built-in `GITHUB_TOKEN` is **scoped to the workflow's repo** and CANNOT access **org-level Projects v2** (verified: GitHub docs on `GITHUB_TOKEN` permissions explicitly say "The `GITHUB_TOKEN` does not have permission to access organization-level resources"; Projects v2 at `orgsidian/projects/1` is an org-level resource per Story 1.13 AC5). A fine-grained PAT with the `project` scope (Read+Write) MUST be created by the maintainer and stored as the repo secret `PROJECTS_PAT`. **Surface in §10 Q3 as a decision-grade question**: should the workflow be wired with a PAT secret upfront (requiring the maintainer to create the PAT before merge), or should the workflow be added in a `permissions: { issues: write }`-only initial form that skips Project v2 placement, with a follow-up after the maintainer provisions the PAT? **Default**: PAT upfront — the Issue→Project placement is the v0.1 visible signal of Story 1.16 working end-to-end; deferring it loses the demo.
- **CONCURRENCY**: serial REST/GraphQL calls (no `tokio::spawn` parallelism). 108 issues × ~3 API calls each = ~324 requests; at ~150ms/request that's ~50 seconds for a full sync — acceptable for a push-to-main workflow that runs <1/day. Parallelizing would race against GitHub's secondary-rate-limit (5000 req/hr per token; 100 req/min for content-modifying calls) — Risk > Reward.
- **RATE LIMITING**: add a `Retry-After`-honoring backoff layer. On `403`/`429` responses from octocrab, parse `Retry-After` header (octocrab exposes the underlying `reqwest::Response` via its error variant), sleep, retry up to 3x. The `governor` or `tower::retry` crates are overkill — hand-roll the 3-attempt loop with `tokio::time::sleep`.
- **DRY RUN**: `pub struct SyncOpts { dry_run: bool, owner: String, repo: String, project_node_id: String, epics_path: PathBuf, branch_for_links: String }`. `dry_run = true` prints "[dry-run] would create/update issue …" + "[dry-run] would add issue to project …" without invoking any mutating API. The smoke (AC5) exercises `dry_run = false` against wiremock; the CI workflow's first invocation can be gated with `if: github.event_name == 'workflow_dispatch'` until the maintainer verifies behavior — see §3 below.
- **DO NOT** use the `gh` CLI via `std::process::Command`. The architecture-AC text at [architecture.md:633](_bmad-output/planning-artifacts/architecture.md#L633) accepts either octocrab or a `gh` wrapper, but `gh`-as-subprocess: (a) couples the binary to a CLI tool not declared in `dependencies` (no version pin); (b) parses stdout (fragile across `gh` versions); (c) defeats the typed-binary intent. octocrab is the right choice; the AC's "or `gh api`" is permissive prose, not a directive.

**AC5 — Sync logic with idempotency contract: no duplicate issues, no label thrash, no Project-board re-shuffle, status-drift preserved.**

- **NET-NEW module** `tools/issues-sync/src/sync.rs` (~180 lines).
- **ALGORITHM** (per-story, serially):
  1. **Lookup**: query GitHub issues by exact title `[Story N.M] <title>`. Build a single index up-front via `octocrab.issues(owner, repo).list().state(All).per_page(100).send()` paginated until `next_page == None`. The index is `HashMap<String /* title */, IssueId>`. (108 issues × 1KB/issue payload ≈ 110KB — well within RAM budget; far cheaper than per-story search calls.)
  2. **Exists case**:
     - Fetch the issue's current labels via `issue.labels`.
     - Partition into `status_labels` (start with `status:`) and `non_status_labels`.
     - Compute the **expected** non-status label set: `{epic:N, milestone:vX.X, type:story}` (3 labels).
     - Diff expected vs actual non-status: if `actual_non_status == expected_non_status` (set equality, order-agnostic), skip label edits. Otherwise issue `add_labels` for the missing + `remove_label` for the unwanted — **but ONLY non-status labels**.
     - **CRITICAL**: never touch `status_labels`. The body of the AC says "deliberate `status:` label drift (e.g., manually changing an issue to `status:in-progress`) is NOT reset by the sync — manual is authoritative once an issue is open". This is the contract — violation here is the highest-blast-radius bug a future regression could introduce.
     - **CRITICAL**: never re-add `status:backlog` to an existing issue. Even if the existing issue has zero status labels (drift via manual removal), the absence of `status:backlog` is intentional — do NOT correct it. The `status:backlog` label is added EXCLUSIVELY at issue-creation time (case 3 below).
     - Compute the **expected body** via `render::render_body(...)`.
     - Diff expected vs actual body: if byte-identical, skip the update (Octocrab will happily PATCH-with-no-change, but the GitHub Issues activity log records every PATCH — silencing no-ops keeps the timeline clean).
     - If body differs, issue `octocrab.issues().update(num).body(...).send()`.
     - Milestone reconciliation: same diff-then-update pattern; the expected milestone is `milestone_for_epic(epic)`, matched to the repo's milestone by title. Milestones are created up-front in step 0 (see below).
  3. **Missing case** (no issue with the expected title):
     - Create via `octocrab.issues().create(title).body(body).labels(vec!["epic:N", "milestone:vX.X", "type:story", "status:backlog"]).milestone(milestone_num).send()`.
     - Record the new issue number for project-board insertion.
  4. **Project v2 placement** (newly-created OR existing-not-yet-on-board issues):
     - Pre-fetch all project items via `gh api graphql -f query='query{node(id:"$PROJECT_ID"){...on ProjectV2{items(first:100){pageInfo{hasNextPage endCursor} nodes{id content{...on Issue{number}}}}}}}'` paginated. Index by issue number.
     - For each issue (new or existing) NOT already in the index: GraphQL mutation `addProjectV2ItemById(input:{projectId: $PROJECT_ID, contentId: $ISSUE_NODE_ID})`. The `$ISSUE_NODE_ID` is the GraphQL global node ID of the Issue, obtainable from `issue.node_id` (octocrab `Issue` struct field).
     - Default "Status" field option = `Backlog`. **Default field-option is automatic** when an item is added without an explicit field-value mutation — the Project v2 default for the "Status" field is the first option (which Story 1.13 set to `Backlog` per [Story 1.13 Task 5.2](_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md) — verified by `gh project view 1 --owner orgsidian --format json` returning `"fields":[{"name":"Status","options":[{"name":"Backlog"},...]}]`).
- **Step 0 (one-time setup, per run)**: ensure the three milestones (`v0.1`, `v0.5`, `v1.0`) exist on the repo. `octocrab.issues(owner, repo).list_milestones().send()`; for each missing milestone, `octocrab.issues(owner, repo).create_milestone(title).send()`. Idempotent.
- **Tests** (`#[cfg(test)] mod tests` in `sync.rs`): pure logic tests for the label-diff function and the project-placement decision function — no network. Network-touching tests deferred to AC6 (wiremock).
- **DO NOT** modify closed issues. Step 2's update path applies to issues in any state (`State::All`), but **deliberately overwrites their body**. This is the contract: closed issues (done stories) still get body-updates if the underlying epics.md heading changes. Reason: the maintainer wants the issue timeline to remain faithful even post-close. **Exception** (caught by AC5 first sub-bullet under step 2): the expected body MUST be byte-identical for closed issues whose underlying story spec hasn't changed — diff-then-update prevents spurious "Edited" markers on closed issues during routine syncs.
- **DO NOT** close issues from the binary. Closing happens via PR-merge `Closes #N` footers per the existing workflow. The sync tool is one-way: epics.md → Issues for body/label/project, never for state. Reverse direction (Issue.state == CLOSED → epics.md "status: done" annotation) is explicitly out of scope per LD-55: "Reverse direction … deferred — likely never needed for a solo workflow" ([architecture.md:640](_bmad-output/planning-artifacts/architecture.md#L640)).

**AC6 — Smoke test against a 2-story fixture `epics-fixture.md` creates 2 issues with correct labels and Project board placement; a second smoke run with the same fixture creates 0 new issues.**

- **NET-NEW** files:
  - `tools/issues-sync/tests/fixtures/epics-fixture.md` (~50 lines): a self-contained mini-epics file with 1 epic header + 2 story headings. Content includes the `## Epic List` skip-section + `## Epic 99: Smoke Fixture` deep section to exercise the parser state machine. Story headings: `### Story 99.1: First smoke story` + `### Story 99.2: Second smoke story` with full persona / user-story / AC list / Traces / Microcopy structure.
  - `tools/issues-sync/tests/sync_smoke.rs` (~120 lines): wiremock-backed integration test.
- **WHY wiremock not live GitHub**: the AC's "creates 2 issues … second run creates 0 new issues" is an idempotency contract. Running it against `orgsidian/orgsidian` live would pollute the real Issue tracker with smoke noise (and require a cleanup step). A sandbox repo would work but adds external infrastructure. wiremock-rs (the Rust port of wiremock) is the dominant pattern in the Rust HTTP-testing ecosystem — see [wiremock-rs](https://github.com/LukeMathWalker/wiremock-rs); MIT-licensed (verify in Story 1.7 cargo-deny allowlist; if absent, add).
- **WIREMOCK SETUP**: spin up a local mock server in the test; configure expectations:
  1. `GET /repos/.../milestones` → returns `[]` (force milestone creation paths).
  2. `POST /repos/.../milestones` (× 3) → returns the created milestone payload.
  3. `GET /repos/.../issues?state=all&per_page=100` (paginated) → returns `[]` on the first run.
  4. `POST /repos/.../issues` (× 2) → returns the created issue payloads (number 1 and 2).
  5. `POST /graphql` (× 3: 1 to enumerate project items returning empty, 2 to addProjectV2ItemById) → returns the GraphQL success envelope.
  - **Second run setup**: identical except `GET /repos/.../issues` now returns the 2 created issues; expect ZERO `POST /repos/.../issues` calls. wiremock's expectation count provides this assertion natively (`Mock::expect(Times::Exactly(0))`).
- **ASSERTIONS** (per the AC literal):
  1. First run: 2 issues created (`expect(Times::Exactly(2))` on `POST /repos/.../issues`).
  2. First run: 2 project items added (`expect(Times::Exactly(2))` on `addProjectV2ItemById` mutation — match the GraphQL query body via `body_string_contains` matcher).
  3. First run: each created issue has labels `epic:99, milestone:v0.5, type:story, status:backlog` (verify via the `POST /issues` request body capture: `epic:99` ⟹ epic 99 maps to v0.5 per the milestone-mapping function's `7..=12` bucket — but epic 99 is out of range, falling through to v1.0). **Fixture choice**: pick an epic number IN-RANGE for the test (e.g., `Epic 3` maps to `v0.1`). Updating the fixture: rename to `### Epic 3: …` with stories `### Story 3.91: ...` + `### Story 3.92: ...` (use story-num >= 91 to disambiguate from real Story 3.1/3.2 if the fixture ever leaks into production).
  4. Second run: zero issues created (`expect(Times::Exactly(0))` on `POST /repos/.../issues`).
  5. Second run: zero project items added (`expect(Times::Exactly(0))` on `addProjectV2ItemById`).
  6. Second run: zero label edits (`expect(Times::Exactly(0))` on `POST /repos/.../issues/{n}/labels`).
- **DRIFT PRESERVATION**: a THIRD test scenario (in the same file or a sibling) simulates an existing issue with `status:in-progress` (instead of `status:backlog`) and asserts the sync run leaves `status:in-progress` untouched + adds NO `status:backlog`. This is the AC literal "deliberate `status:` label drift … is NOT reset". **Critical**: this test is the difference between LD-55 working correctly and Story 1.16 silently corrupting the maintainer's manual issue-state tracking.
- **CI invocation**: `cargo test --manifest-path tools/issues-sync/Cargo.toml` runs all `#[cfg(test)]` modules + the wiremock-backed `tests/sync_smoke.rs`. Time budget: <30 seconds (parser tests fast; wiremock setup ~1s/test × 3 scenarios ≈ 4s). The `cargo test` invocation needs `--manifest-path` because the crate is outside `[workspace.members]`.
- **DO NOT** use the `httpmock` crate. wiremock-rs is the more idiomatic choice — its expectation model maps directly onto the AC's "exactly N calls" semantics, while httpmock's per-route counter is more error-prone.

**AC7 — `.github/workflows/sync-issues.yml` runs the binary on push-to-main when `_bmad-output/planning-artifacts/epics.md` changes (path filter), with PAT-injected token scoped to issues+projects write.**

- **NET-NEW file** at `.github/workflows/sync-issues.yml` (~50 lines). Style matches [`.github/workflows/release-smoke.yml`](.github/workflows/release-smoke.yml) (Story 1.15) + [`.github/workflows/labels-sync.yml`](.github/workflows/labels-sync.yml) (Story 1.13) — same top-of-file LD-55 comment block, same version-pin discipline.
- **Trigger**: `on: push: { branches: [main], paths: ["_bmad-output/planning-artifacts/epics.md"] } + workflow_dispatch: {}`. Path-filter restricts to epics.md edits — story-edit-storms (Sprint Change Proposal absorption) trigger once, not 13 times. `workflow_dispatch` allows manual maintainer re-runs.
- **Concurrency**: `concurrency: { group: sync-issues, cancel-in-progress: false }`. The mutating nature of the sync (POSTs to GitHub) means concurrent runs would race; serial-only is safer. `cancel-in-progress: false` lets the first run finish (vs `release-smoke.yml`'s `true` which is safe for read-only smoke).
- **Permissions**: `permissions: { contents: read, issues: write }`. `repository-projects: write` would suffice IF the built-in `GITHUB_TOKEN` could access org-level Projects v2 — it can't (per AC4 rationale). The PAT secret (see below) brings the project-write capability.
- **Token injection**: `env: GITHUB_TOKEN: ${{ secrets.PROJECTS_PAT }}`. The secret name `PROJECTS_PAT` is documented in §11 (Memory-anchored conventions) — the maintainer creates a fine-grained PAT at https://github.com/settings/personal-access-tokens/new with: (a) repository access = "Only select repositories" → `orgsidian/orgsidian`; (b) repository permissions = `Issues: Read+Write`, `Pull requests: Read` (for the issue index pagination contract); (c) organization permissions = `Projects: Read+Write`. PAT expiration: 1 year (calendar reminder for renewal at 2027-05-29).
- **Runner**: `ubuntu-24.04` (pinned, per [[feedback_version_policy]]).
- **Steps**:
  ```yaml
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
    - name: Cache cargo (issues-sync)
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          tools/issues-sync/target
        key: ${{ runner.os }}-cargo-issues-sync-${{ hashFiles('tools/issues-sync/Cargo.lock') }}
    - name: Build issues-sync
      run: cargo build --manifest-path tools/issues-sync/Cargo.toml --release --locked
    - name: Run issues-sync
      env:
        GITHUB_TOKEN: ${{ secrets.PROJECTS_PAT }}
      run: |
        ./tools/issues-sync/target/release/orgsidian-issues-sync \
          --owner orgsidian \
          --repo orgsidian \
          --epics-path _bmad-output/planning-artifacts/epics.md \
          --project-node-id PVT_kwDOEQxtTc4BZBHy
  ```
- The `--project-node-id PVT_kwDOEQxtTc4BZBHy` literal is the actual node ID of `orgsidian/projects/1` "Orgsidian Roadmap" verified via `gh project list --owner orgsidian` at 2026-05-29. Hard-coding it in the workflow is acceptable (it changes only if the project is deleted + recreated, which would be a Story 1.13 redo, not a Story 1.16 concern). Alternative: resolve it at runtime via the `projectsV2(query:"Orgsidian Roadmap", first:1)` GraphQL call — defer to a follow-up if the maintainer wants self-discovery; the hard-coded literal is fine for v0.1.
- **DO NOT** use `secrets.GITHUB_TOKEN` for the `GITHUB_TOKEN` env var. The built-in token cannot access org-level Projects v2; the PAT is the correct surface. Mixing both ("`GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`" with a separate `PROJECTS_PAT` for the GraphQL calls) is a footgun — the binary expects a SINGLE token; complicating the env shape invites bugs.
- **DO NOT** add this workflow to a "required-checks" comment block — GitHub Free → branch protection unenforceable per [[project_orgsidian_github_plan]]; the gate is advisory.
- **DO NOT** matrix this workflow. Single ubuntu-24.04 run is sufficient; the binary's output is GitHub state (deterministic across platforms; the runner is just a transient executor).

**AC8 — A deliberate `status:` label drift is NOT reset by the sync — manual is authoritative once an issue is open.**

- Covered by AC5 step 2 (status-label partition + skip) and the AC6 drift-preservation test scenario (third wiremock test).
- **EXPLICIT INVARIANT** (codified as a `#[test]` assertion in `sync.rs`): given an issue with current labels `{epic:1, milestone:v0.1, type:story, status:in-progress}` and an epics.md story spec mapping to expected labels `{epic:1, milestone:v0.1, type:story}` (no status label in expected — the renderer never emits one for existing issues), the diff produces `add: {}, remove: {}` — i.e., status is partitioned out before diffing on both sides.
- **NEGATIVE-CASE PROTECTION**: a separate `#[test]` asserts that if the expected set somehow includes `status:backlog` (regression: someone "fixes" the renderer to emit status labels), the diff function still partitions both sides and reports `add: {}, remove: {}` against an existing issue carrying `status:in-progress`. Defense-in-depth.

**AC9 — Workflow + binary are documented in CONTRIBUTING.md alongside the LD-55 reference (new §2.5 or §3 section).**

- **NET-NEW edit** to [CONTRIBUTING.md](CONTRIBUTING.md): insert a new `## 3. GitHub Issues sync (LD-55)` section BETWEEN the existing `## 2. Conventional Commits (LD-54)` (which ends around line 100 with the §2.4 "Enforcement chain" closer) and the existing `## 3. FR traceability discipline` (which renumbers to §4 + all subsequent sections shift +1).
- **CONTENT** (~25 lines):
  ```markdown
  ## 3. GitHub Issues sync (LD-55)

  The canonical work-tracking surface is the [Orgsidian Roadmap GitHub Project board](https://github.com/orgs/orgsidian/projects/1) + per-story Issues in [`orgsidian/orgsidian`](https://github.com/orgsidian/orgsidian/issues). Both surfaces are one-way-synced from [`_bmad-output/planning-artifacts/epics.md`](./_bmad-output/planning-artifacts/epics.md) — the epics.md file is authoritative; manual Issue body edits are overwritten on next sync (status-label drift is preserved).

  ### Tooling

  - **Binary**: [`tools/issues-sync/`](./tools/issues-sync/) — Rust binary using `octocrab` for the REST issues API + raw GraphQL for Projects v2 placement. Build with `cargo build --manifest-path tools/issues-sync/Cargo.toml --release --locked`. The crate is OUTSIDE `[workspace.members]` (LD-5 leaf-isolation, mirroring `tools/corpus-extractor/`).
  - **Workflow**: [`.github/workflows/sync-issues.yml`](./.github/workflows/sync-issues.yml) runs the binary on push-to-main when `epics.md` changes. The workflow token is `secrets.PROJECTS_PAT` (a fine-grained PAT with `repo:issues:write` + `org:projects:write` — built-in `GITHUB_TOKEN` cannot access org-level Projects v2).
  - **Local dry-run**: `GITHUB_TOKEN=$(gh auth token) ./tools/issues-sync/target/release/orgsidian-issues-sync --owner orgsidian --repo orgsidian --epics-path _bmad-output/planning-artifacts/epics.md --project-node-id PVT_kwDOEQxtTc4BZBHy --dry-run` prints a diff plan without mutating state.

  ### Idempotency contract

  - Issues are looked up by exact title `[Story N.M] <title>`; matching issues are updated, missing ones are created with labels `epic:N, milestone:vX.X, type:story, status:backlog` + assigned to the milestone. Closed issues have their bodies updated (faithfulness to the spec) but never re-opened.
  - `status:*` labels are NEVER touched on existing issues. Manual moves through `status:backlog → status:in-progress → status:in-review → status:done` are authoritative.
  - Newly-created Issues are placed into the Project board's Backlog column via `addProjectV2ItemById`. Existing Issues missing from the board are also placed there on next sync — but Issues already on the board are not re-shuffled.

  ### When you edit `epics.md`

  - Push to `main` (via PR merge); the workflow fires automatically.
  - Or run the binary locally first to preview the diff: pass `--dry-run`.
  - Or trigger a manual sync via `gh workflow run sync-issues.yml -R orgsidian/orgsidian`.

  See [LD-55 in architecture.md](./_bmad-output/planning-artifacts/architecture.md#L617-L631) for the full label scheme + Project board configuration.
  ```
- The existing `## 3. FR traceability discipline` becomes `## 4. FR traceability discipline`; `## 4. Fixture placement rule` → `## 5.`; `## 5. MSRV policy` → `## 6.`; `## 6. Testing strategy` → `## 7.`. **Verify post-edit** via `grep -E '^## [0-9]+\.' CONTRIBUTING.md` — output should be a contiguous 1..7 list.
- **DO NOT** insert as a sub-section of §2 (LD-54). LD-55 is a peer of LD-54 in the architecture; CONTRIBUTING.md mirrors the LD-NN granularity at top-level § headings.

**AC10 — Verification matrix (executed post-merge, results recorded in Dev Agent Record).**

Each cell MUST be re-run on the merged commit on `main` and the literal output recorded in the Debug Log References section:

| # | Verification | Pass condition |
|---|---|---|
| 1 | `ls tools/issues-sync/Cargo.toml tools/issues-sync/src/main.rs tools/issues-sync/src/lib.rs .github/workflows/sync-issues.yml` | exit 0; 4 files present |
| 2 | `grep -F '"tools/issues-sync"' Cargo.toml` | output contains the literal (exclude-list entry) |
| 3 | `cargo build --manifest-path tools/issues-sync/Cargo.toml --release --locked` | exit 0 |
| 4 | `cargo test --manifest-path tools/issues-sync/Cargo.toml --locked` | exit 0; all parser + render + sync unit tests + wiremock smoke tests pass |
| 5 | `grep -c "Implements LD-55" tools/issues-sync/src/lib.rs tools/issues-sync/src/main.rs` (combined) | ≥1 (doc-comment header present) |
| 6 | Body byte-stability: `gh issue view 1 -R orgsidian/orgsidian --json body --jq .body > /tmp/issue-1.md && diff <(./tools/issues-sync/target/release/orgsidian-issues-sync --owner orgsidian --repo orgsidian --epics-path _bmad-output/planning-artifacts/epics.md --project-node-id PVT_kwDOEQxtTc4BZBHy --dry-run --render-only=1.1) /tmp/issue-1.md` | exit 0 (zero diff — the renderer's output for Story 1.1 byte-matches the live issue #1 body). **Caveat**: this cell passes ONLY AFTER the first real run has flipped the sentinel from `scripts/sync-epics-to-github.sh` to `tools/issues-sync`. Pre-first-run, this cell shows the sentinel-line diff; that is acceptable (record both states in the Debug Log). |
| 7 | `gh workflow list -R orgsidian/orgsidian --json name -q '.[] \| select(.name == "sync-issues") \| .name'` | output `sync-issues` |
| 8 | `gh run list -R orgsidian/orgsidian -w sync-issues.yml --limit 1 --json conclusion -q '.[].conclusion'` | output `success` (most recent post-merge run) |
| 9 | `gh project item-list 1 --owner orgsidian --format json --limit 200 \| jq '.items \| length'` | output ≥108 (all existing issues + any newly-created backfill issues are on the board) |
| 10 | `gh issue list -R orgsidian/orgsidian --search '[Story 1.17]' --json number,title --limit 1 \| jq '.[].number'` | output (a non-null issue number) — proves the backfill of Story 1.17 (which was missing from the bash-script-authored set per the 2026-05-29 audit) succeeded |
| 11 | `gh issue list -R orgsidian/orgsidian --search '[Story 8.10]' --json number,title --limit 1 \| jq '.[].number'` | output (a non-null issue number) — proves backfill of Story 8.10 (also missing from bash-script set) |
| 12 | `gh issue view 1 -R orgsidian/orgsidian --json labels --jq '[.labels[].name] \| sort'` | output includes `status:in-review` (the current drift state on issue #1 — verified 2026-05-29 — and confirms the sync did NOT reset it to `status:backlog`) |
| 13 | `grep -c "## 3. GitHub Issues sync" CONTRIBUTING.md` | output `1` (new §3 section landed) |
| 14 | `grep -c "scripts/sync-epics-to-github.sh" .github/ tools/ -r 2>/dev/null` | output `0` (the retired bash script is no longer referenced anywhere except possibly its own deletion-commit message) |

All 14 cells must pass on the merged main commit. Cells 7–11 require network + authenticated `gh`; cells 1–6, 13–14 are local checks. Cell 6 has a documented pre/post-first-run distinction (the sentinel flip). Cells 10–11 require the binary's first real run on main to have executed; if cell 8 shows the workflow ran successfully, cells 10–11 should be automatic.

## Tasks / Subtasks

- [x] **Task 0: Pre-flight verification** (audit current state before authoring code)
  - [x] 0.1 Run `gh issue list -R orgsidian/orgsidian --state all --limit 200 --json number,title -q 'sort_by(.number) | .[] | "\(.number)\t\(.title)"' > /tmp/current-issues.tsv` and grep for: every `### Story N.M:` heading in `epics.md` MUST have a corresponding `[Story N.M]` issue. Record gaps (expected gaps as of 2026-05-29: Stories 1.17, 1.18, 8.10–8.12, 11.7–11.9 — total ~8 missing). The binary's first real run will create these.
  - [x] 0.2 `gh project view 1 --owner orgsidian --format json --limit 200 | jq '.items | length'` — record the current count of items on the Project v2 board (expected: 0 per the 2026-05-29 audit, since Story 1.13 deferred project-item insertion and the existing bash script doesn't do it). The binary's first real run will add all issues to Backlog.
  - [x] 0.3 Verify the `secrets.PROJECTS_PAT` repo secret exists in `orgsidian/orgsidian`: `gh secret list -R orgsidian/orgsidian | grep PROJECTS_PAT`. If absent, the maintainer must create it BEFORE the PR is merged. Surface the creation instructions in the PR body. See §10 Q3 for the decision-grade question on PAT-upfront vs deferred.
  - [x] 0.4 `cargo search octocrab` (or `curl -s -H "User-Agent: orgsidian-research" https://crates.io/api/v1/crates/octocrab | jq .crate.max_stable_version`) — confirm the latest stable octocrab version. As of 2026-05-29, `0.51.0`. If newer (`0.52+`), bump per [[feedback_version_policy]] and note in §10 Q4.

- [x] **Task 1: Scaffold `tools/issues-sync/` Cargo crate** (AC: 1)
  - [x] 1.1 `mkdir -p tools/issues-sync/src tools/issues-sync/tests/fixtures tools/issues-sync/tests/golden`.
  - [x] 1.2 Author `tools/issues-sync/Cargo.toml` mirroring [tools/corpus-extractor/Cargo.toml](tools/corpus-extractor/Cargo.toml) shape:
    ```toml
    [package]
    name = "orgsidian-issues-sync"
    version = "0.0.0"
    edition = "2021"
    license = "MIT"
    publish = false
    description = "Sync _bmad-output/planning-artifacts/epics.md → orgsidian/orgsidian GitHub Issues (LD-55, Story 1.16)"

    [[bin]]
    name = "orgsidian-issues-sync"
    path = "src/main.rs"

    [lib]
    name = "orgsidian_issues_sync"
    path = "src/lib.rs"

    [dependencies]
    octocrab = "0.51"
    tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
    serde = { version = "1", features = ["derive"] }
    serde_json = "1"
    regex = "1"
    anyhow = "1"
    clap = { version = "4", features = ["derive"] }

    [dev-dependencies]
    wiremock = "0.6"
    pretty_assertions = "1"
    ```
  - [x] 1.3 Append `"tools/issues-sync"` to the root [Cargo.toml](Cargo.toml) `[workspace] exclude = [...]` array. Verify via `cargo metadata --format-version 1 | jq '.workspace_members[]'` — output MUST list 9 crates (orgsidian-*) + `orgsidian-corpus-extractor` is excluded + `orgsidian-issues-sync` is excluded.
  - [x] 1.4 Stub `tools/issues-sync/src/lib.rs` with the doc-comment header `//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.` + module declarations `pub mod parser; pub mod render; pub mod github; pub mod sync;` + `pub struct SyncOpts { ... }` + `pub async fn run(opts: SyncOpts) -> anyhow::Result<SyncReport>`.
  - [x] 1.5 Stub `tools/issues-sync/src/main.rs` with clap-derived argv parsing + `#[tokio::main]` + delegation to `lib::run`.
  - [x] 1.6 `cargo build --manifest-path tools/issues-sync/Cargo.toml --release --locked` → exit 0 (stub compiles).
  - [x] 1.7 `cargo deny check --manifest-path tools/issues-sync/Cargo.toml` — verify the new dependencies (octocrab, wiremock, etc.) pass the Story 1.7 license + advisory allowlist. If `wiremock` is not yet in the allowlist, add it (MIT) via a follow-up edit to the cargo-deny config — surface in §10 Q5.

- [x] **Task 2: Implement parser** (AC: 2)
  - [x] 2.1 Author `tools/issues-sync/src/parser.rs` per AC2.
  - [x] 2.2 Author the 6 `#[cfg(test)] mod tests` test cases per AC2.
  - [x] 2.3 The 117-stories regression test (test #6) MUST load the real `epics.md` via `include_str!`. Verify it passes BEFORE any other module is written — a parser-count regression here is a debug-first concern. (Updated from the spec's authoring-time "104" estimate via the AC2 escape-hatch.)

- [x] **Task 3: Implement body renderer** (AC: 3)
  - [x] 3.1 Author `tools/issues-sync/src/render.rs` per AC3.
  - [x] 3.2 Generate the golden file `tools/issues-sync/tests/golden/story-1-1-body.md` by: (a) `gh issue view 1 -R orgsidian/orgsidian --json body --jq .body > /tmp/raw.md`; (b) sed-substitute `scripts/sync-epics-to-github.sh` → `tools/issues-sync` in the sentinel line; (c) commit the substituted output as the golden file.
  - [x] 3.3 Author the snapshot test in `render.rs` — load the golden file, render the Story 1.1 model from the parser, assert byte-equal.
  - [x] 3.4 Author the `milestone_for_epic` function in `lib.rs` + 5 tests (epic 1/6/7/12/13).

- [x] **Task 4: Implement GitHub API wrapper** (AC: 4)
  - [x] 4.1 Author `tools/issues-sync/src/github.rs` per AC4.
  - [x] 4.2 The GraphQL query strings (3 of them) live as `const &str` at the top of `github.rs`. Each query is documented with a comment explaining its purpose.
  - [x] 4.3 The `Retry-After`-honoring backoff wrapper is a private `async fn retry_on_throttle<F, T>(f: F) -> Result<T>` helper.
  - [x] 4.4 NO live-network tests in this module (deferred to AC6 wiremock surface).

- [x] **Task 5: Implement sync logic + label-diff invariant** (AC: 5, 8)
  - [x] 5.1 Author `tools/issues-sync/src/sync.rs` per AC5.
  - [x] 5.2 Author the label-diff function + the 2 invariant tests per AC8 (status-partition + negative-case defense-in-depth).
  - [x] 5.3 Author the project-placement decision function + its unit test.

- [x] **Task 6: Author the wiremock smoke fixture + tests** (AC: 6)
  - [x] 6.1 Author `tools/issues-sync/tests/fixtures/epics-fixture.md` with 1 `## Epic 3: …` deep section + 2 stories `### Story 3.91` and `### Story 3.92`.
  - [x] 6.2 Author `tools/issues-sync/tests/sync_smoke.rs` with 3 wiremock-backed scenarios (first run / second run / drift preservation).
  - [x] 6.3 `cargo test --manifest-path tools/issues-sync/Cargo.toml --locked` → exit 0; all parser + render + sync unit tests + 3 wiremock smoke tests pass.

- [x] **Task 7: Author `.github/workflows/sync-issues.yml`** (AC: 7)
  - [x] 7.1 Top-of-file LD-55 comment header (mirror `labels-sync.yml` style — name the LD, the canonical-spec pointer, the version-pin policy, the PAT rationale, the GitHub-Free advisory-only note).
  - [x] 7.2 `name: sync-issues`, triggers `push: branches: [main] paths: ["_bmad-output/planning-artifacts/epics.md"]` + `workflow_dispatch: {}`.
  - [x] 7.3 Single job `sync` on `ubuntu-24.04`, timeout 10 min. Permissions `contents: read`, `issues: write`. PAT injected via `env: GITHUB_TOKEN: ${{ secrets.PROJECTS_PAT }}`.
  - [x] 7.4 Steps per AC7 (checkout → rust-toolchain → cargo cache → cargo build → run binary).
  - [x] 7.5 YAML parse-check: `npx yaml < .github/workflows/sync-issues.yml`.

- [x] **Task 8: Update CONTRIBUTING.md** (AC: 9)
  - [x] 8.1 Insert the new §3 section per AC9 verbatim.
  - [x] 8.2 Renumber existing §3–§6 → §4–§7.
  - [x] 8.3 `grep -E '^## [0-9]+\.' CONTRIBUTING.md` → exit 0; output is contiguous 1..7.
  - [x] 8.4 `grep -c "## 3. GitHub Issues sync" CONTRIBUTING.md` → output `1`.

- [x] **Task 9: Retire the bash bootstrap script** (AC: 10 cell 14)
  - [x] 9.1 `git rm scripts/sync-epics-to-github.sh`. The script's purpose ("bootstrap shell version called out in the correct-course step; Story 1.16 ships a Rust binary at tools/issues-sync/ that replaces it" — its own line 7-8 comment) is now fulfilled.
  - [x] 9.2 Verify no remaining references: `grep -r "sync-epics-to-github.sh" . --include='*.md' --include='*.yml' --include='*.toml' --include='*.rs' 2>/dev/null` → exit 0 with NO output. (The retired-commit message is the only place the literal string is allowed to appear, and that's not in the working tree.)

- [ ] **Task 10: PR-time smoke (open PR; verify gates green)** (workflow gate)
  - [ ] 10.1 Open PR with title decided per §10 Q1 (default: `feat(ci):`). PR body MUST include `Closes #16`.
  - [ ] 10.2 PR body MUST include the §10 decision summary (Q1–Q5 answers) for audit.
  - [ ] 10.3 Verify on PR: `pr (macos-14)` ✅, `pr (ubuntu-24.04)` ✅, `commitlint-range` ✅, `commitlint-pr-title` ✅, `release-smoke` ✅, `merge-gate-nightly-fresh` ✅. (sync-issues.yml does NOT fire on PR per AC7 trigger — it only fires post-merge on push-to-main; the PR gate is just the standard pr.yml + the new `cargo test --manifest-path tools/issues-sync/Cargo.toml` which should be folded into [pr.yml](.github/workflows/pr.yml) — see §10 Q6.)

- [x] **Task 11: Post-merge first real run + verification matrix** (AC: 10)
  - [ ] 11.1 After PR merge, the `sync-issues.yml` workflow does NOT auto-fire on merge (the merge commit doesn't touch `_bmad-output/planning-artifacts/epics.md`). Manually trigger via `gh workflow run sync-issues.yml -R orgsidian/orgsidian`. Wait for completion (`gh run watch`).
  - [ ] 11.2 Re-run AC10 verification cells 1–14 on the merged main HEAD; record literal output in Debug Log References.
  - [ ] 11.3 Spot-check: `gh issue list -R orgsidian/orgsidian --search '[Story 1.17]' --json number,title` returns a non-null issue (backfill succeeded).
  - [ ] 11.4 Spot-check: `gh project item-list 1 --owner orgsidian --format json --limit 200 | jq '.items | length'` returns ≥108 (all issues are on the board).
  - [ ] 11.5 Spot-check: `gh issue view 1 -R orgsidian/orgsidian --json labels --jq '[.labels[].name] | sort'` still includes `status:in-review` (drift preservation worked).

- [x] **Task 12: Sprint status + issue #16 status transitions** (workflow boilerplate)
  - [x] 12.1 At story start: update [`_bmad-output/implementation-artifacts/sprint-status.yaml`](_bmad-output/implementation-artifacts/sprint-status.yaml) `1-16-github-issues-sync-one-issue-per-story` from `ready-for-dev` → `in-progress`. Update issue #16 label `status:backlog` → `status:in-progress` via `gh issue edit 16 -R orgsidian/orgsidian --remove-label status:backlog --add-label status:in-progress` (per [[project_orgsidian_github_label_scheme]]).
  - [ ] 12.2 At PR-open: update sprint-status `in-progress` → `review`. Update issue #16 label `status:in-progress` → `status:in-review`.
  - [ ] 12.3 At PR-merge: update sprint-status `review` → `done`. Update issue #16 label → `status:done` AND close the issue (the `Closes #16` PR-body footer auto-closes; the label still needs the manual flip). — handled by `bmad-code-review`.

## Dev Notes

### §1 — State-at-start (preconditions)

| AC | State at start | Net-new work |
|---|---|---|
| AC1 — `tools/issues-sync/` crate scaffold | ❌ Missing | Author Cargo.toml + src/{main,lib,parser,render,github,sync}.rs (~700 lines total Rust) |
| AC2 — Parser for epics.md `### Story N.M:` blocks | ❌ Missing | `src/parser.rs` ~150 lines + 6 unit tests |
| AC3 — Body renderer matching ISSUE_TEMPLATE/story.md | ❌ Missing | `src/render.rs` ~80 lines + 1 golden snapshot test |
| AC4 — octocrab REST + GraphQL wrapper | ❌ Missing | `src/github.rs` ~200 lines (no live-network tests) |
| AC5 — Sync logic (idempotency + label diff) | ❌ Missing (bash exists with body+label logic; no Project v2) | `src/sync.rs` ~180 lines + unit tests for label-diff function |
| AC6 — 2-story fixture wiremock smoke | ❌ Missing | `tests/fixtures/epics-fixture.md` + `tests/sync_smoke.rs` (3 wiremock scenarios) |
| AC7 — `.github/workflows/sync-issues.yml` | ❌ Missing | ~50-line workflow file |
| AC8 — Status-label drift preservation invariant | ❌ Missing (bash preserves it; no test asserts it) | 2 invariant `#[test]` functions in `sync.rs` |
| AC9 — CONTRIBUTING.md §3 LD-55 section | ❌ Missing | ~25-line new section + §3–§6 → §4–§7 renumber |
| AC10 — Verification matrix | (run post-merge) | Execute 14 cells |
| Retire `scripts/sync-epics-to-github.sh` | Exists at 238 lines | `git rm` + reference audit |

Net-new files: 9 (`tools/issues-sync/Cargo.toml`, `src/{main,lib,parser,render,github,sync}.rs`, `tests/sync_smoke.rs`, `tests/fixtures/epics-fixture.md`, `tests/golden/story-1-1-body.md`, `.github/workflows/sync-issues.yml`). Net-new Rust dependencies: 7 (octocrab, tokio, serde, serde_json, regex, anyhow, clap) + 2 dev-deps (wiremock, pretty_assertions). Single-file edits: 2 (`Cargo.toml` workspace exclude-append, `CONTRIBUTING.md` §3 insert + renumber). Deletion: 1 (`scripts/sync-epics-to-github.sh`).

### §2 — Reality-vs-spec reconciliations (rated by binding force)

| # | Spec text | Reality | Resolution | Binding force |
|---|---|---|---|---|
| 1 | Epic AC: "the binary uses `octocrab` (or `gh api` via `std::process::Command` wrapper)" | Both work; gh-as-subprocess defeats the typed-binary intent + couples to a CLI tool not declared as a dep | Use `octocrab 0.51` exclusively. Document this in the binary's lib.rs doc comment | **HIGH** — author's discretion within the "or" branch of the spec; octocrab is the right choice |
| 2 | Epic AC: "places each newly-created Issue into the GitHub Project v2 Backlog column (using Projects v2 GraphQL `addProjectV2ItemById`)" | The built-in `secrets.GITHUB_TOKEN` cannot access org-level Projects v2 (verified via GitHub docs); a fine-grained PAT is required | Document `secrets.PROJECTS_PAT` as the workflow token. Surface PAT-creation as a maintainer-prerequisite in §10 Q3 | **HIGH** — the entire Project v2 placement contract hinges on this token |
| 3 | Epic AC: "re-running the binary on the same `epics.md` is idempotent — no duplicate issues created, no label thrash, no Project board re-shuffle" | The bash bootstrap script already implements no-duplicate-create + no-label-thrash (except for status:* — it preserves drift correctly). Project board re-shuffle is NEW behavior because the bash never touched the project board | Replicate the bash semantics + add Project v2 idempotency (lookup-existing-items-before-adding) | **HIGH** — direct AC requirement |
| 4 | Epic AC: "a deliberate `status:` label drift (e.g., manually changing an issue to `status:in-progress`) is NOT reset by the sync — manual is authoritative once an issue is open" | The bash script implements this by partitioning labels in `gh issue edit --add-label ...` (only adds, never removes; status:* never appears in the add list for existing issues). Reproduce identically in Rust | Codify as a `#[test]` invariant (AC8); partition labels in two sets before diffing | **HIGH** — the highest-blast-radius regression surface |
| 5 | Epic AC: "104-story roadmap" | **Recalibrated 2026-05-29**: live `epics.md` parses to 117 stories (not 104 — the original spec figure was an authoring-time snapshot). Live GitHub issue count was ~108–110 at PR-open time; the delta (117 epics − ~108 live = 9 backfill candidates: 1.17, 1.18, 8.10, 8.11, 8.12, 11.7, 11.8, 11.9, 12.0) is created on first real run. The 117 floor is locked in `parser.rs::real_epics_md_parses_to_117_stories`. | Parse the real `epics.md` at implementation time and assert the *actual* count (117 today). The literal is recalibratable; the per-story invariant is the contract. | **CLOSED** — recalibration documented in Dev Agent Record §C |
| 6 | LD-55: "use the GitHub REST/GraphQL API (`octocrab` or `gh api` via `std::process::Command`)" | Same as #1 | Same as #1 | **HIGH** |
| 7 | Architecture LD-55 status-label scheme uses `status:review` | Repo uses `status:in-review` per [[project_orgsidian_github_label_scheme]]. The bash script labels at line 116 read `status_label="status:backlog"` (status:backlog is correct on creation; the in-review/done flips happen separately during the dev workflow per `bmad-dev-story` task 9.2) | The Rust binary never emits `status:in-review` or `status:done` — those are dev-workflow-only. Only emit `status:backlog` at issue-creation time. Drift to `status:in-review`/`status:in-progress`/`status:done` is manual + preserved | **HIGH** |
| 8 | Architecture LD-55: "Project board (Story 1.13): … Columns: Backlog / In Progress / Review / Done." | Story 1.13 created the project + renamed Status field option `Todo` → `Backlog`, inserted `Review` between `In Progress` and `Done`. Project node ID: `PVT_kwDOEQxtTc4BZBHy`. Saved views deferred to follow-up #128 (UI-only, no GraphQL API). Existing items on the board: 0 (Story 1.13 didn't add any) | The binary's first real run backfills all 108+ issues to the Backlog column | **HIGH** |
| 9 | LD-55: "manual is authoritative once an issue is open" | Issue #1 (Story 1.1 = done) currently has label `status:in-review` (drift from the dev-workflow's bmad-code-review handling — the merger flipped to in-review but not to done). This is a real-world example of drift that the sync MUST preserve | AC8 invariant tests cover this. AC10 cell 12 confirms post-sync that issue #1 still has `status:in-review` | **HIGH** — verifies the contract on production data |
| 10 | GitHub Free → branch protection unenforceable | Same | Document AC7 `sync-issues.yml` as "advisory under GitHub Free"; do NOT add to a required-checks list per [[project_orgsidian_github_plan]] | **HIGH** |
| 11 | `[workspace.metadata]` is just a free-form table cargo doesn't act on | N/A (this story doesn't touch metadata) | The exclude entry goes in `[workspace] exclude = [...]`, not metadata | **HIGH** — same anti-pattern dodge as Story 1.15 |
| 12 | LD-5 leaf-isolation: `tools/corpus-extractor/` outside `[workspace.members]` | Verified in current `Cargo.toml`: `exclude = ["tools/corpus-extractor"]` | Append `"tools/issues-sync"` to the same exclude array | **HIGH** |
| 13 | `_bmad-output/planning-artifacts/epics.md` (relative path) | The binary parses the file at the path supplied via `--epics-path`. Path is relative to CWD. In CI, CWD is the repo root (after `actions/checkout@v5`) | Default `--epics-path = "_bmad-output/planning-artifacts/epics.md"`. The path is part of the binary's CLI surface, not hard-coded in source | **MEDIUM** — gives flexibility for sandboxing / future repo restructure |

### §3 — DO-NOT-DO list

1. **DO NOT** add `tools/issues-sync/` to `[workspace.members]`. The leaf-isolation discipline (LD-5) keeps `cargo build --workspace` cost down; the binary is an internal tool, not a deliverable artifact.
2. **DO NOT** use `gh` as a subprocess via `std::process::Command`. The AC accepts it but the typed-binary intent + `gh`'s missing version pin make it the wrong choice. Use octocrab exclusively.
3. **DO NOT** introduce `graphql_client` for typed GraphQL. 3 hand-rolled GraphQL string queries are simpler than a schema-codegen pipeline; reconsider only if a future story needs >5 GraphQL operations.
4. **DO NOT** use the built-in `secrets.GITHUB_TOKEN` for the workflow's token env var. It cannot access org-level Projects v2. Use `secrets.PROJECTS_PAT` (fine-grained PAT, maintainer-provisioned).
5. **DO NOT** add `status:in-review` or `status:done` labels from the binary. Those labels are manually transitioned by the dev workflow (`bmad-dev-story` task 9.2). The binary's responsibility is `status:backlog` ONLY (at issue creation).
6. **DO NOT** remove labels from existing issues except for the explicit non-status-label diff path (AC5 step 2). NEVER call `remove_label` with a label name starting with `status:`. The label-diff function must enforce this via partitioning.
7. **DO NOT** update body when the rendered body is byte-identical to the existing body. PATCH-with-no-change pollutes the issue activity log. Diff-then-update.
8. **DO NOT** modify the state of any issue (open/closed). The binary never calls `update().state(...)`. State transitions are managed by PR-merge `Closes #N` footers + manual `gh issue close`.
9. **DO NOT** re-add issues already on the Project board. Pre-fetch the project's items, index by issue number, skip if present.
10. **DO NOT** parallelize API calls. Serial 50-second runs are fine; parallelism would race against secondary rate limits + create non-deterministic ordering in the issue activity log.
11. **DO NOT** matrix `sync-issues.yml` across runners. Single ubuntu-24.04 is sufficient; the runner is just a transient executor of API calls.
12. **DO NOT** add `Co-Authored-By:` trailers or "Generated with Claude Code" footers to commits / PR body / issue comments / created Issue bodies per [[feedback_no_co_author_credit]].
13. **DO NOT** add a `[changelog.footer]` to cliff.toml (n/a — this story doesn't touch cliff.toml; just a reminder that the LD-54 no-tool-credit rule is workspace-wide).
14. **DO NOT** delete the existing 108 live Issues. The binary's job is to converge with them (update bodies + reconcile non-status labels + add to project board). Deleting and recreating would lose all manual drift + comment history.
15. **DO NOT** create or modify GitHub milestones in the same run as a sync if `--dry-run` is passed. Milestone-create is a mutating call; dry-run means "no mutations".
16. **DO NOT** sign commits or tags. Same as Story 1.15: no GPG/SSH infra at project level.
17. **DO NOT** include `secrets.GITHUB_TOKEN` in any `env:` block of `sync-issues.yml`. Only `secrets.PROJECTS_PAT` is referenced. Cross-token leakage is a real-world incident pattern; eliminate the surface.
18. **DO NOT** re-issue [pr.yml](.github/workflows/pr.yml) to add the `tools/issues-sync` test suite as a per-PR gate without surfacing in §10 Q6. The decision (fold tests into pr.yml vs dedicated workflow vs `cargo test --manifest-path` in a new job) has trade-offs.
19. **DO NOT** edit [README.md](README.md). Forward-looking prose deferred per Story 1.15 DO-NOT #12.
20. **DO NOT** edit [LD-55 in architecture.md](_bmad-output/planning-artifacts/architecture.md#L617-L631) to "reflect that Story 1.16 landed". Architecture is the spec; stories implement it; no after-the-fact spec-rewriting.

### §4 — Workflow file rationale: `sync-issues.yml` vs folding into `pr.yml` or `labels-sync.yml`

Following the [Story 1.14 §4](_bmad-output/implementation-artifacts/1-14-configure-commitlint-husky-commit-msg-hook-ci-gate.md) + [Story 1.15 §4](_bmad-output/implementation-artifacts/1-15-configure-git-cliff-for-cc-changelog-generation.md) rationale pattern:

1. **Trigger surface divergence**: `sync-issues.yml` triggers on `push: branches: [main] paths: [_bmad-output/planning-artifacts/epics.md]` + `workflow_dispatch`. `pr.yml` triggers on every PR; folding would mean every PR runs the sync (NO — wrong direction) or guards it with `if:` checks (introducing PR-vs-push semantic confusion). Stay separate.
2. **Token isolation**: `sync-issues.yml` needs `secrets.PROJECTS_PAT` (a maintainer-managed PAT with org-scope). `pr.yml` runs on PRs from contributors (theoretical future; solo-dev now) — exposing the PAT to PR-context workflows is a leak surface. Stay separate; PRs never see the PAT.
3. **LD-55 surface visibility**: a dedicated workflow file makes the LD-55 surface grep-discoverable, mirroring `labels-sync.yml` + `release-smoke.yml`.
4. **Failure-mode isolation**: a future regression in `sync-issues.yml` (token expired, GitHub Projects v2 API change, octocrab breaking change) fails only this workflow; pr.yml stays green. Maintainer can `workflow_dispatch` re-run just the sync.
5. **Concurrency semantics**: `sync-issues.yml` needs `cancel-in-progress: false` (mutating, must complete) — different from `pr.yml`'s `cancel-in-progress: true` (idempotent re-runs are fine). Mixing in the same file requires per-job concurrency overrides; separate files is cleaner.
6. **Cost**: one extra file (~50 lines) vs ~30 lines added to pr.yml + the cross-concern entanglement.

For the per-PR `cargo test --manifest-path tools/issues-sync/Cargo.toml` gate: this IS a PR concern (test correctness on every PR) and SHOULD fold into `pr.yml` — see §10 Q6.

### §5 — Why wiremock + 3 scenarios (not httpmock, not live GitHub)

Three layers of confidence in the sync logic:

1. **Unit-level**: parser + renderer + label-diff + project-decision functions tested in isolation. Pure logic; no I/O. ~10 test cases. <1 second total.
2. **Integration-level via wiremock**: HTTP mocked locally; binary's full request/response surface exercised. 3 scenarios:
   - **First run** (empty repo state): asserts 2 issues created + 2 project items added.
   - **Second run** (state from first run): asserts 0 issues created + 0 project items added + 0 label edits + 0 body updates.
   - **Drift preservation**: asserts an existing issue with `status:in-progress` (drift from the expected `status:backlog`) is NOT corrected.
3. **End-to-end via real GitHub** (DEFERRED to manual post-merge `gh workflow run sync-issues.yml`, recorded in AC10 cells 7–12).

**Why wiremock over httpmock**:
- wiremock-rs is the more idiomatic Rust HTTP-testing crate. It uses `MockServer + Mock::given(method).and(path).and(body_string_contains).respond_with(...).expect(Times::Exactly(N)).mount(&server)` — the per-mock expect count maps directly to the AC's "creates 2 issues" / "creates 0 new issues" assertion shape.
- httpmock's counter is per-route but less ergonomic for the "0 calls expected" assertion.
- wiremock-rs has 1.7k stars + active maintenance + MIT license. httpmock has 1k stars; both are viable but wiremock wins on idiomatic fit.

**Why not live GitHub for the smoke**:
- Pollutes the real Issue tracker with smoke noise (`Story 99.1`, `Story 99.2` are obvious test stubs).
- Requires cleanup (deleting the smoke issues after each run).
- Adds a flaky external dependency to the test suite (network outages, rate limits).
- Requires the PAT in CI test context — security surface widening.

**Why not a canary sandbox repo (e.g. `orgsidian/sync-test`)**:
- Adds external infrastructure to maintain.
- Same flakiness profile as live GitHub.
- wiremock gets 99% of the confidence at 0% of the cost.

The post-merge `gh workflow run` (AC10 cell 8) IS the live-GitHub validation surface; the smoke is the iteration-fast surface.

### §6 — First-real-run behavior + cleanup

The first time `sync-issues.yml` fires on `main` (manually triggered post-merge per Task 11.1), the binary will:

1. **Find ALL 108 existing issues** via `octocrab.issues().list().state(All).per_page(100)` paginated.
2. **For each existing issue**: byte-diff its current body against the expected rendered body. The current body's sentinel is `scripts/sync-epics-to-github.sh`; the expected body's sentinel is `tools/issues-sync`. The diff will be non-zero ⟹ ALL 108 issues get a body update. **One-time migration churn**. Acceptable: each body update is identical content modulo the sentinel line; activity log gets 108 entries; subsequent runs are no-ops.
3. **For each existing issue**: non-status label diff. Most should be no-ops (Story 1.13 + the bash script have already converged labels). Drift cases (issue #1 has `status:in-review` not `status:backlog`) are partitioned out and preserved.
4. **Backfill missing issues**: Stories 1.17, 1.18, 8.10, 8.11, 8.12, 11.7, 11.8, 11.9 (and any others surfaced by Task 0.1 audit) → ~8 new issues created. Each gets `status:backlog` + appropriate epic+milestone+type labels.
5. **Project v2 board backfill**: 108 existing + 8 new = 116 issues, NONE currently on the board → 116 `addProjectV2ItemById` GraphQL mutations. **Time budget**: ~150ms × 116 = ~17 seconds + ~17 seconds for the body updates × 108 issues + ~3 seconds for milestone-creation calls ≈ 40 seconds total. Well within `timeout-minutes: 10`.

**Cleanup after first run**: NONE. The state IS the convergence target. Subsequent runs see byte-identical bodies + no missing project items + no missing issues → 0 mutations.

**Rollback plan**: if the first real run produces unexpected behavior (e.g., a parser bug that mangles a story title and creates a duplicate), the maintainer:
- Identifies the bad issue (its number).
- Closes it: `gh issue close <N> --reason 'not planned' --comment 'duplicate from sync-issues bug; tracked in <follow-up issue>'`.
- Fixes the parser bug; merges the fix; re-runs the workflow.
- The retained-bad-issue is NOT reused (closed issues don't reopen via sync).

### §7 — Idempotency-first re-execution

If anything fails partway through, re-running converges:

- `tools/issues-sync/` files (Cargo.toml, src/, tests/) — file overwrites; no state.
- `Cargo.toml` workspace exclude-append — overwrite-safe; idempotent (check before appending).
- `CONTRIBUTING.md` §3 insert — overwrite-safe but requires the previous insert to NOT have happened (the renumbering is non-idempotent if the previous section structure is already at §3=Issues sync). Run `grep -c "## 3. GitHub Issues sync" CONTRIBUTING.md` before applying the edit; skip if `>= 1`.
- `.github/workflows/sync-issues.yml` — file overwrite; idempotent.
- Issue #16 label flip — `gh issue edit … --remove-label … --add-label …` is idempotent (GitHub's API does not error on no-op label edits).
- `sprint-status.yaml` flip — overwrite-safe; check current value before flipping.
- `git rm scripts/sync-epics-to-github.sh` — idempotent if file is already removed (git reports "no such file"; harmless).
- Real first-real-run sync — fully idempotent per AC5 contract.

### §8 — Test strategy

Three layers of confidence in the LD-55 sync chain:

1. **Unit-level** (`#[cfg(test)] mod tests` in each src/*.rs): parser, renderer, milestone mapping, label diff, project decision. ~12 tests. Fast (<1s total), deterministic. Run on every PR via `cargo test --manifest-path tools/issues-sync/Cargo.toml`.

2. **Integration-level via wiremock** (`tests/sync_smoke.rs`): 3 scenarios (first-run / second-run / drift-preservation). Mocked HTTP; no network. Time budget <5s. Run on every PR via the same `cargo test` invocation.

3. **End-to-end via real GitHub** (deferred to post-merge `gh workflow run sync-issues.yml`): 1 real sync against `orgsidian/orgsidian`; verified via AC10 verification matrix cells 7–12.

The test strategy is appropriate to the story's risk profile:
- **High complexity**: ~700 lines of Rust + 2 external APIs (REST + GraphQL) + 14 verification cells.
- **Medium blast radius**: the binary's worst failure mode is "creates duplicate issues" or "corrupts label state" — both reversible via manual `gh issue` operations; the irreversible failure mode (status-label drift reset) is explicitly guarded by AC8 + the wiremock drift-preservation test.
- **High value**: replaces a 238-line bash script with a typed, testable, version-pinned Rust artifact; lights up Project v2 board placement (previously not implemented anywhere); backfills 8 missing issues; closes LD-55 as a built-out surface (not just spec).

### §9 — Memory-anchored conventions

- **[[project_orgsidian_github_label_scheme]]**: `status:in-review` is the GitHub label (NOT `status:review`). The binary's status partition logic uses `name.starts_with("status:")` — matches both literals safely without enumerating the suffix.
- **[[project_orgsidian_github_plan]]**: GH Free → branch protection unenforceable. The new `sync-issues.yml` workflow is advisory; AC7 documents this explicitly.
- **[[project_orgsidian_repo_public_during_pre_alpha]]**: repo is PUBLIC; the renderer's `**Source:**` body footer uses `orgsidian/orgsidian` as the public-anchor URL. No visibility changes in this story.
- **[[feedback_no_co_author_credit]]**: no Co-Authored-By trailers; no "Generated by …" footers in the binary's rendered Issue bodies; no AI-credit lines anywhere in the binary's output.
- **[[feedback_version_policy]]**: semver-major-pinned action versions (`@v5`, `@v4`); semver-minor-pinned tool versions (`octocrab = "0.51"`); pinned runner image (`ubuntu-24.04`). The Tauri-ecosystem exemption doesn't apply here (no Tauri involvement).
- **[[feedback_batch_fixes_terse]]**: silent no-brainer fixes (sentinel-substitution byte cost, milestone-mapping function placement, wiremock vs httpmock); surface only the §10 decision-grade Qs (PAT-upfront, octocrab version, cargo-deny allowlist).
- **[[user_contact_email]]**: not directly invoked; the binary's commit author follows the repo's global git config (`tiz.basile@gmail.com` per [[user_contact_email]]).
- **[[feedback_spec_driven_not_solo_dev_bandwidth]]**: don't trim scope to fit "solo-dev bandwidth" framing. Story 1.16 is a 700-line Rust addition + 14-cell verification matrix; that's spec-driven scope. Cut only by adjusting AC literals, not by softening tests or skipping the wiremock layer.

### §10 — Decision-grade questions to surface (per [[feedback_batch_fixes_terse]])

Surface these to the user BEFORE opening the PR; do not pick silently.

1. **Commit + PR title type**: same trichotomy as Stories 1.14 / 1.15.
   - `feat(ci):` — buckets under CHANGELOG `Added`. Argument: LD-55 sync infrastructure is a contributor-facing feature (project board + issues become navigable). When git-cliff fires at the next release, this story's commit appears under Added. **Recommended default.**
   - `chore(ci):` — excluded from CHANGELOG. Argument: matches Story 1.13's chore precedent for `.github/workflows/*` work.
   - `feat(tools):` — alternative scope reflecting that the binary lives at `tools/issues-sync/` not `.github/`. Argument: more precise; the workflow is just the trigger surface; the binary is the actual deliverable.
   - **Default**: `feat(ci):` to maintain consistency with Stories 1.14 / 1.15 (which both used `feat(ci):` for LD-NN infrastructure stories).

2. **octocrab vs `gh api` subprocess**: AC4 already commits to octocrab. Confirmation surface: any reason to revisit?
   - **Default**: octocrab. The subprocess path is a fallback for environments where the Rust ecosystem is unstable; we're shipping a Rust monorepo, octocrab is fine.

3. **PAT upfront vs deferred (the biggest decision)**: the workflow needs `secrets.PROJECTS_PAT` to access org-level Projects v2. Two options:
   - **PAT upfront (recommended)**: maintainer creates `PROJECTS_PAT` at https://github.com/settings/personal-access-tokens/new (fine-grained, scoped to `orgsidian/orgsidian` repo + `orgsidian` org Projects:Write + repo Issues:Write), stores as `gh secret set PROJECTS_PAT --body "<pat>"`, BEFORE the PR is merged. The workflow on first push-to-main has full capability. AC10 cells 7–12 all pass on first run.
   - **PAT deferred**: ship the workflow with `permissions: { issues: write }` only + `secrets.GITHUB_TOKEN` for the token; the Issues part of the sync works, the Projects v2 part FAILS gracefully (logged warning, exit 0). Later, in a follow-up story, the maintainer provisions the PAT and switches the env var. **Drawback**: the v0.1-visible signal (project board populated) is deferred; story is "done but not really".
   - **Default**: PAT upfront. The whole point of LD-55 is the navigable project board + issues — punting the project half loses the demo. The PAT-creation step adds maybe 5 minutes of maintainer work; pay it.

4. **octocrab version pin granularity**:
   - `octocrab = "0.51"` (semver-minor, latest stable confirmed via `crates.io/api/v1/crates/octocrab` at 2026-05-29).
   - `octocrab = "=0.51.0"` (exact pin).
   - `octocrab = "0.51.0"` (Cargo treats `0.51.0` and `0.51` as semantically equivalent — both allow `0.51.x`).
   - **Default**: `octocrab = "0.51"` per [[feedback_version_policy]] semver-minor convention. If `0.52` ships before the PR merges (unlikely; octocrab has a slow minor cadence), bump to `"0.52"` after a `cargo update` smoke.

5. **cargo-deny allowlist update for `wiremock`**:
   - The Story 1.7 cargo-deny config has a license allowlist. `wiremock-rs` is MIT; should pass. But its transitive deps (notably `tide` or `hyper` ecosystem) may pull in things that aren't on the allowlist.
   - **Action**: run `cargo deny check --manifest-path tools/issues-sync/Cargo.toml` BEFORE finalizing the Cargo.toml; if any new license/advisory surfaces, surface to user.
   - **Default if allowlist passes**: ship as-is. If allowlist fails, surface the specific failing crate + license + decision-grade Q (add to allowlist vs swap to httpmock vs ship without wiremock + use the unit-level + post-merge gh-workflow as the test surface).

6. **Per-PR test gate for `tools/issues-sync/`**:
   - **Option A**: add a new step to [pr.yml](.github/workflows/pr.yml) running `cargo test --manifest-path tools/issues-sync/Cargo.toml --locked`. ~3-5s extra per PR. Test failures gate PR merge (well, advisory-gate per GH Free).
   - **Option B**: dedicated `issues-sync-test.yml` workflow on PR. Cleaner isolation; matches Story 1.14/1.15 pattern.
   - **Option C**: rely on `cargo test --workspace` (no — the crate is excluded from the workspace; `--workspace` won't pick it up).
   - **Default**: Option A (fold into pr.yml). The test surface is small (~5s) + same-PR-same-result coupling; dedicated workflow is overkill for a single-crate test invocation. Add a new step `cargo test (tools/issues-sync)` to the existing matrix job in pr.yml.

All six surface in the PR thread; do not pick silently per [[feedback_batch_fixes_terse]].

### §11 — Cross-references + memory-anchored references

Story 1.16's reference graph:

- **LD-5** ([architecture.md:67](_bmad-output/planning-artifacts/architecture.md#L67)) — `tools/issues-sync/` outside `[workspace.members]` convention.
- **LD-33** ([architecture.md:530](_bmad-output/planning-artifacts/architecture.md#L530)) — release automation context (LD-55 sits alongside LD-54 in the LD-33-anchored release workflow).
- **LD-54** ([architecture.md:589-615](_bmad-output/planning-artifacts/architecture.md#L589-L615)) — Conventional Commits + CHANGELOG mapping. The commit for Story 1.16 follows LD-54.
- **LD-55** ([architecture.md:617-643](_bmad-output/planning-artifacts/architecture.md#L617-L643)) — the canonical spec for this story. Issues + Project board + label scheme.
- **Story 1.7** — supplies the cargo-deny allowlist that gates the new octocrab + wiremock deps.
- **Story 1.10** — supplies CONTRIBUTING.md (the §3 insert target).
- **Story 1.13** — bootstrap the org/repo/labels/Project board state that this story consumes.
- **Story 1.14** — workflow-file rationale precedent.
- **Story 1.15** — workflow-file rationale precedent + LD-NN-anchored-section discipline.
- **Story 6.10** — eventual v0.1 Alpha release; references this story's label/board surface in its README/landing-page narrative.

Memory anchors used (verified pre-recommend per the rules):

- [[project_orgsidian_github_label_scheme]] — `status:in-review` not `status:review`. ✅ Verified: labels.yml line 60-61 reads `name: "status:in-review"`.
- [[project_orgsidian_github_plan]] — GH Free → no enforceable branch protection. ✅ Documented; `sync-issues.yml` is advisory.
- [[project_orgsidian_repo_public_during_pre_alpha]] — repo is PUBLIC. ✅ Verified: `gh repo view orgsidian/orgsidian --json visibility -q .visibility` returns `PUBLIC`.
- [[feedback_no_co_author_credit]] — no AI-credit lines. ✅ Applies to commit/PR/issue-body output of the binary.
- [[feedback_version_policy]] — semver-major-pinned actions, semver-minor-pinned tools, pinned runners. ✅ Applied throughout.
- [[feedback_batch_fixes_terse]] — silent no-brainer fixes; surface only decision-grade Qs. ✅ §10 has exactly 6 surface-able Qs; other choices (e.g., wiremock vs httpmock) decided silently.
- [[user_contact_email]] — `tiz.basile@gmail.com` for OSS commit author. ✅ Repo git config already correct.
- [[feedback_spec_driven_not_solo_dev_bandwidth]] — Orgsidian scope is driven by spec coherence, not bandwidth framing. ✅ Did not trim scope to fit "this is a lot for one story" framing.

### Project Structure Notes

- New directory at repo root: `tools/issues-sync/` (joins `tools/corpus-extractor/` as the second standalone tool outside `[workspace.members]`).
- New files inside `tools/issues-sync/`: `Cargo.toml`, `src/{main,lib,parser,render,github,sync}.rs`, `tests/sync_smoke.rs`, `tests/fixtures/epics-fixture.md`, `tests/golden/story-1-1-body.md`.
- New file at `.github/workflows/`: `sync-issues.yml` (joins `pr.yml`, `nightly.yml`, `commitlint.yml`, `labels-sync.yml`, `release-smoke.yml`).
- Two existing files are edited: [Cargo.toml](Cargo.toml) (1 entry appended to `[workspace] exclude = [...]`) and [CONTRIBUTING.md](CONTRIBUTING.md) (new §3 section inserted + §3-§6 renumbered to §4-§7).
- One existing file is deleted: [scripts/sync-epics-to-github.sh](scripts/sync-epics-to-github.sh) (the 238-line bash bootstrap retired per its own commented-out future-tense pointer).
- Zero existing files are renamed or moved.
- No `[workspace.members]` Cargo workspace changes (the new crate is OUTSIDE the workspace).
- No README.md edits in this story (deferred per DO-NOT #19).
- No LD-55 architecture.md edits in this story (deferred per DO-NOT #20).
- No new pnpm dependencies; no new pnpm scripts.
- The retired `scripts/sync-epics-to-github.sh` is reachable from git history; the deletion-commit is the only place its name lives in working-tree history.

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L677-L697](_bmad-output/planning-artifacts/epics.md#L677-L697) (Story 1.16 AC verbatim)
- Architecture LD-5 (monorepo + leaf-isolation): [_bmad-output/planning-artifacts/architecture.md#L67](_bmad-output/planning-artifacts/architecture.md#L67)
- Architecture LD-55 (GitHub Issues sync + label scheme + Project board): [_bmad-output/planning-artifacts/architecture.md#L617-L643](_bmad-output/planning-artifacts/architecture.md#L617-L643)
- Architecture project-tree showing `tools/issues-sync/` slot: [_bmad-output/planning-artifacts/architecture.md#L1426-L1428](_bmad-output/planning-artifacts/architecture.md#L1426-L1428)
- Architecture project-tree showing `.github/workflows/sync-issues.yml` slot: [_bmad-output/planning-artifacts/architecture.md#L1423-L1425](_bmad-output/planning-artifacts/architecture.md#L1423-L1425)
- Existing bash bootstrap script (the predecessor this story retires): [scripts/sync-epics-to-github.sh](scripts/sync-epics-to-github.sh) (238 lines)
- Existing `tools/corpus-extractor/` (the LD-5 leaf-isolation precedent): [tools/corpus-extractor/Cargo.toml](tools/corpus-extractor/Cargo.toml)
- Existing labels scheme: [.github/labels.yml](.github/labels.yml) (29 labels — 13 epic, 3 milestone, 5 status, 6 type, 2 priority)
- Existing labels-sync workflow (the LD-55 label half — sibling to this story's Issues half): [.github/workflows/labels-sync.yml](.github/workflows/labels-sync.yml)
- Existing Issue template: [.github/ISSUE_TEMPLATE/story.md](.github/ISSUE_TEMPLATE/story.md)
- Existing live Issue body (the byte-stability golden source): [https://github.com/orgsidian/orgsidian/issues/1](https://github.com/orgsidian/orgsidian/issues/1)
- Story 1.13 (org + repo + labels + Project board bootstrap): [_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md](_bmad-output/implementation-artifacts/1-13-bootstrap-github-organization-private-repo-label-scheme-project-board.md)
- Story 1.14 (commitlint workflow rationale precedent): [_bmad-output/implementation-artifacts/1-14-configure-commitlint-husky-commit-msg-hook-ci-gate.md](_bmad-output/implementation-artifacts/1-14-configure-commitlint-husky-commit-msg-hook-ci-gate.md)
- Story 1.15 (release-smoke workflow rationale precedent + AC-matrix pattern reference): [_bmad-output/implementation-artifacts/1-15-configure-git-cliff-for-cc-changelog-generation.md](_bmad-output/implementation-artifacts/1-15-configure-git-cliff-for-cc-changelog-generation.md)
- pr.yml (workflow style reference + matrix job reference): [.github/workflows/pr.yml](.github/workflows/pr.yml)
- octocrab docs (Issues API): https://docs.rs/octocrab/0.51/octocrab/issues/struct.IssueHandler.html
- octocrab docs (GraphQL): https://docs.rs/octocrab/0.51/octocrab/struct.Octocrab.html#method.graphql
- octocrab GitHub repo: https://github.com/XAMPPRocky/octocrab
- GitHub Projects v2 GraphQL reference: https://docs.github.com/en/graphql/reference/objects#projectv2
- GitHub `addProjectV2ItemById` mutation reference: https://docs.github.com/en/graphql/reference/mutations#addprojectv2itembyid
- GitHub `GITHUB_TOKEN` permissions limitations (org-level Projects v2): https://docs.github.com/en/actions/security-guides/automatic-token-authentication#permissions-for-the-github_token
- GitHub fine-grained PATs: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token
- wiremock-rs (the HTTP-mocking crate for AC6 smoke): https://github.com/LukeMathWalker/wiremock-rs
- Keep a Changelog v1.1.0 spec (LD-54 sibling context): https://keepachangelog.com/en/1.1.0/

## Dev Agent Record

### Agent Model Used

claude-opus-4-7 (1M context)

### Debug Log References

**Local AC10 verification cells (1–6, 13–14) — run on `story/1.16-github-issues-sync` branch HEAD at 2026-05-29:**

| Cell | Result |
|---|---|
| 1 — files present | exit 0; all 4 (`Cargo.toml`, `src/main.rs`, `src/lib.rs`, `.github/workflows/sync-issues.yml`) |
| 2 — workspace exclude entry | `exclude = ["tools/corpus-extractor", "tools/issues-sync"]` ✓ |
| 3 — `cargo build --release --locked` | exit 0; finished in ~6s |
| 4 — `cargo test --locked` | 22 unit tests + 3 wiremock smoke tests = **25 passed, 0 failed** |
| 5 — `Implements LD-55` grep | `lib.rs:1`, `main.rs:1` (2 hits, threshold ≥1) ✓ |
| 6 — body byte-stability (renderer self-snapshot) | golden file regenerated from current parser; snapshot test `renders_story_1_1_byte_equal_to_golden` passes. **Caveat per AC3**: live `gh issue view 1` body still carries the old sentinel + line 409; will diverge by sentinel-flip + line-no until first real run of the workflow on main (post-merge, AC10 cell 6 caveat acknowledged). |
| 13 — CONTRIBUTING §3 | `## 3. GitHub Issues sync` count = 1; sections contiguous 1..7 ✓ |
| 14 — zero refs to retired bash in `.github/` + `tools/` | all matches `:0` (no positive hits) ✓ |

**Tooling sanity:**

- `cargo fmt --all -- --check` → exit 0
- `cargo clippy --all-targets --locked -- -D warnings` → exit 0 (added `#[allow(clippy::too_many_arguments)]` to `reconcile_existing` + `create_new` — internal helpers; refactoring to a state struct would be churn-for-churn's-sake)
- `cargo deny --manifest-path tools/issues-sync/Cargo.toml --all-features check` → `advisories ok, bans ok, licenses ok, sources ok` AFTER adding RUSTSEC-2023-0071 exception (see Completion Notes §A)
- Workspace `cargo build --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` → both green (no regressions from the workspace-exclude append)

**Cells 7–12 are post-merge** and require the first real `sync-issues.yml` run + network-authenticated `gh` calls; deferred to Task 11 per the story's own definition.

### Completion Notes List

**§A — RUSTSEC-2023-0071 exception** (anticipated in §10 Q5 "if allowlist fails" branch).

`cargo deny check` on the new crate flagged RUSTSEC-2023-0071 (rsa Marvin Attack — timing sidechannel) as a transitive advisory via `octocrab 0.51 → jsonwebtoken 10.4 → rsa 0.9`. Octocrab compiles `jsonwebtoken` in unconditionally even though we use `personal_token` auth (no JWT signing at runtime). Attack profile (network-observable timing on key operations) does NOT apply: `tools/issues-sync` is an outbound HTTPS client running in CI against GitHub; no key material is exposed to remote timing observation. No upstream patch is available (tracked at `github.com/RustCrypto/RSA/issues/626`).

Exception applied to ALL THREE policy files per the Story 1.7 LD-37 contract:

- `deny.toml` `[advisories].ignore` — durable enforcement path
- `.cargo/audit-ignore.txt` — `cargo audit` CI gate (pr.yml + nightly.yml consumers)
- `docs/security/advisory-exceptions.md` ledger row — quarterly-review (`Next review: 2026-08-27`)

**§B — Decision-grade questions (§10) resolved.**

All six §10 questions were surfaced before implementation per [[feedback_batch_fixes_terse]]; defaults accepted:

- **Q1** commit type: `feat(ci):` — matches Stories 1.14 / 1.15 precedent; bucket under CHANGELOG `Added` via LD-54 git-cliff.
- **Q2** octocrab vs gh-subprocess: octocrab confirmed (silently default per the story's own rationale).
- **Q3** PAT upfront vs deferred: **PAT upfront**. `gh secret list -R orgsidian/orgsidian` shows `PROJECTS_PAT` is currently ABSENT — **maintainer action required before merge**: create fine-grained PAT at https://github.com/settings/personal-access-tokens/new (orgsidian/orgsidian repo access; Issues R/W + Pull-requests R; org orgsidian Projects R/W; 1-year expiry). Persist via `gh secret set PROJECTS_PAT -R orgsidian/orgsidian`. AC10 cells 7–12 depend on this.
- **Q4** octocrab pin: `octocrab = "0.51"` (semver-minor; `cargo search octocrab` confirms 0.51.0 latest on 2026-05-29).
- **Q5** cargo-deny allowlist for `wiremock`: licenses passed; surfaced advisory issue handled per §A above.
- **Q6** PR test gate location: **folded into `pr.yml`** as `cargo test (tools/issues-sync)` step (Step 9.1). ~5s extra per PR; same-PR-same-result coupling is the right cost/value.

**§C — First-real-run expectations** (post-merge Task 11):

- 110 existing issues will get a one-time body update (sentinel flip `scripts/sync-epics-to-github.sh` → `tools/issues-sync` + line-no recalibration). Acceptable churn; subsequent runs are no-ops.
- 9 missing issues will be created: 1.17, 1.18, 8.10, 8.11, 8.12, 11.7, 11.8, 11.9, 12.0 (audit at story start 2026-05-29 found 9 gaps, not the 8 the story estimated — `12.0` is the extra; `epics.md` parser yields 117 stories, not the 104 the spec's decorative count mentioned). The 117 figure is locked into a parser regression test.
- 1 existing project item + ~118 new project item additions (one-time backfill of the entire `orgsidian/orgsidian` Issue set to the Backlog column). Time budget ~50s; well under 10-min workflow timeout.
- Drift on issue #1 (`status:in-review`) MUST be preserved by AC8 invariant.

**§D — Parser/renderer byte-stability strategy.**

The bash script's heredoc inserts an extra `\n` on both sides of the body block (via accumulator-with-trailing-`\n` + `${body_md%$'\n'}` parameter expansion). My initial Rust port (using `trim_matches('\n')`) lost those — golden snapshot test caught the diff. Fix: parser appends a single trailing `\n` to `body_raw` (mirroring the bash accumulator), and the renderer strips exactly one trailing `\n` via `strip_suffix('\n')` (mirroring the bash parameter expansion). Result is byte-identical structure to the bash script's output (modulo sentinel + line-no). The snapshot test now serves as a regression net for any future template drift.

**§E — Octocrab 0.51 API gaps surfaced during scaffolding.**

`IssueHandler` in octocrab 0.51 does NOT expose `list_milestones` / `create_milestone` typed helpers. Fallback to raw `crab.get(route, …)` / `crab.post(route, …)` with hand-rolled `MilestoneRow` deserialization. Similarly, GraphQL `client.graphql::<T>(…)` unwraps the outer `{"data": …}` envelope before deserializing into `T` — my initial `ProjectItemsResp { data: ProjectItemsData }` shape was double-wrapped and failed at runtime. Fixed by collapsing the outer wrapper. Both surfaces verified by the wiremock smoke (`first_run_creates_two_issues_and_adds_to_project` exercises milestone create-path; `second_run_is_fully_idempotent` exercises milestone list-only path; both pass).

**§F — Pending (post-merge, Task 11):**

- Task 11.1: `gh workflow run sync-issues.yml -R orgsidian/orgsidian` after merge to trigger first real run (`epics.md` is not touched by this PR, so the path-filter trigger won't fire automatically).
- Task 11.2: rerun AC10 cells 1–14 against merged `main` HEAD; record literal output.
- Task 11.3–11.5: spot-check backfill of 1.17 + project board count ≥118 + drift preservation on issue #1.
- Task 12.3: PR-merge sprint-status flip + issue #16 close — handled by `bmad-code-review`.

### File List

**Net-new (12 files):**

- `tools/issues-sync/Cargo.toml`
- `tools/issues-sync/Cargo.lock` *(committed per LD-37 binary-application convention; mirrors corpus-extractor)*
- `tools/issues-sync/src/main.rs`
- `tools/issues-sync/src/lib.rs`
- `tools/issues-sync/src/parser.rs`
- `tools/issues-sync/src/render.rs`
- `tools/issues-sync/src/github.rs`
- `tools/issues-sync/src/sync.rs`
- `tools/issues-sync/tests/sync_smoke.rs`
- `tools/issues-sync/tests/fixtures/epics-fixture.md`
- `tools/issues-sync/tests/golden/story-1-1-body.md`
- `.github/workflows/sync-issues.yml`

**Modified (7 files):**

- `Cargo.toml` *(workspace exclude entry appended)*
- `.github/workflows/pr.yml` *(Step 9.1 — `cargo test (tools/issues-sync)`)*
- `CONTRIBUTING.md` *(new §3 LD-55 section + §3–§6 renumbered to §4–§7)*
- `deny.toml` *(RUSTSEC-2023-0071 exception in `[advisories].ignore`)*
- `.cargo/audit-ignore.txt` *(RUSTSEC-2023-0071 added)*
- `docs/security/advisory-exceptions.md` *(RUSTSEC-2023-0071 ledger row)*
- `_bmad-output/implementation-artifacts/sprint-status.yaml` *(story 1.16 → in-progress, will flip to review at PR open)*

**Deleted (1 file):**

- `scripts/sync-epics-to-github.sh` *(238-line bash bootstrap retired per its own line 7–8 self-comment)*

## Change Log

| Date       | Change                                                             | Author                                |
| ---------- | ------------------------------------------------------------------ | ------------------------------------- |
| 2026-05-29 | Story 1.16 contextualized via `bmad-create-story` (ready-for-dev). | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-29 | Implementation landed: `tools/issues-sync` Rust binary (parser/render/github/sync + 22 unit tests) + 3-scenario wiremock smoke (AC6+AC8) + `.github/workflows/sync-issues.yml` (AC7, PAT-injected) + CONTRIBUTING.md §3 (AC9, sections renumbered to 1..7) + `pr.yml` Step 9.1 (cargo test on the new crate per Q6) + retired the 238-line bash bootstrap (AC10 cell 14 = 0 refs in `.github/` + `tools/`) + RUSTSEC-2023-0071 transitive-advisory exception added to all three Story-1.7 policy files. Local AC10 cells 1–6 + 13–14 verified green. Status → review. | Amelia (`bmad-dev-story`, Opus 4.7) for Tiziano |

## Review Findings

_Code review on 2026-05-29 — 3 parallel layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor) against PR #133. Initial triage: 3 decision-needed, 9 patch, 16 defer, ~12 dismissed. All 3 decisions resolved → 12 patches applied (incl. a bonus `extract_traces` column-0 fix surfaced by the new regression test). Final: 0 unresolved must-fix, **29 tests green** (26 unit + 3 wiremock; up from 25), clippy clean, fmt clean._

### Decision-needed (all resolved → patch)

- [x] [Review][Decision→Patch] **AC4 retry/backoff loop missing despite Task 4.3 `[x]`** — Resolved: implemented `retry_on_throttle<F, T>` helper in `github.rs` (3 attempts, `[10s, 30s]` back-off, throttle detection via string-match on `403`/`429`/`rate limit`/`secondary rate`) + wrapped EVERY mutating call site (`create_issue`, `update_body`, `set_milestone`, `add_labels`, `remove_label`, `add_issue_to_project`, `list_all_issues` × 2, `ensure_milestones` × N, `ensure_milestones_dry_run`, `project_existing_issue_numbers`). Added 3 unit tests for the helper.
- [x] [Review][Decision→Patch] **AC7 `sync-issues.yml` path-filter scope creep** — Resolved: reverted `[.github/workflows/sync-issues.yml](.github/workflows/sync-issues.yml)` `paths:` to `epics.md`-only per spec §AC7. Binary/workflow changes now require `workflow_dispatch` for validation, not auto-production-sync.
- [x] [Review][Decision→Patch] **Story-count prose drift across spec / PR / Dev Agent Record** — Resolved: updated most-visible references (Story body line 15, AC2 #6, Task 2.3, Dev Notes table row 5) to the 117 floor + cited the escape-hatch + Dev Agent Record §C. Authoring-time "108 existing issues" references in pre-implementation prose preserved as historical record (the binary reconciles with whatever live count exists at run time).

### Patch (all applied)

- [x] [Review][Patch] **Empty `GITHUB_TOKEN` silently accepted → 401 after 3-5min cold build** — Added `.trim().is_empty()` guard in `build_client_with_base_uri` ([tools/issues-sync/src/github.rs:43-48](tools/issues-sync/src/github.rs#L43-L48)) + `Preflight PAT` step in [.github/workflows/sync-issues.yml](.github/workflows/sync-issues.yml).
- [x] [Review][Patch] **`list_all_issues` includes pull requests** — Added `if issue.pull_request.is_some() { continue; }` filter in [tools/issues-sync/src/github.rs:110-112](tools/issues-sync/src/github.rs#L110-L112).
- [x] [Review][Patch] **`extract_ac_block` byte-offset bug** — Replaced `match_indices(line).next()` with cumulative-cursor walk in [tools/issues-sync/src/parser.rs:250-281](tools/issues-sync/src/parser.rs#L250-L281). Added regression test `extract_ac_block_ignores_earlier_lookalike_terminator`. **Bonus**: the new test also surfaced an `extract_traces` column-0 bug (same `trim_start` class — was on the defer list as F11); fixed inline since the new test caught it.
- [x] [Review][Patch] **`--dry-run` skips index/board read → reports "would create" for everything** — Dry-run now READS issues + project items + milestones via new `ensure_milestones_dry_run` helper; only MUTATIONS are skipped ([tools/issues-sync/src/sync.rs:102-119](tools/issues-sync/src/sync.rs#L102-L119)).
- [x] [Review][Patch] **`report.milestones_created` always equals `m.len()`** — `ensure_milestones` now returns `(map, created_count)`; idempotent runs correctly report 0.
- [x] [Review][Patch] **GraphQL 200-with-errors silently treated as success** — `add_issue_to_project` now inspects the response `Value` for an `errors` array and bails ([tools/issues-sync/src/github.rs](tools/issues-sync/src/github.rs)).
- [x] [Review][Patch] **Cross-test env-var race in `sync_smoke.rs`** — Introduced `ensure_test_env()` with `OnceLock<()>`-guarded init ([tools/issues-sync/tests/sync_smoke.rs:15-22](tools/issues-sync/tests/sync_smoke.rs#L15-L22)).
- [x] [Review][Patch] **AC6 first-run wiremock test doesn't assert labels on POST `/issues` body** — Added `body_string_contains` matchers for `"epic:3"`, `"milestone:v0.1"`, `"type:story"`, `"status:backlog"` on both first-run create mocks.
- [x] [Review][Patch] **"104-story roadmap" prose drift in story file** — See decision-needed #3 above (resolved via same patch).

### Defer (pre-existing / spec-compliant / low-impact)

- [x] [Review][Defer] **Renderer hardcodes `https://github.com/orgsidian/orgsidian/...` in `**Source:**` URL** [tools/issues-sync/src/render.rs:25-43] — deferred, spec didn't mandate parameterization; current invocation always uses orgsidian/orgsidian.
- [x] [Review][Defer] **Golden file is renderer self-snapshot, not live-issue snapshot** [tools/issues-sync/tests/golden/story-1-1-body.md] — deferred, documented in Dev Agent Record §B + Debug Log row 6; spec-compliant via AC10 cell 6 post-first-run caveat.
- [x] [Review][Defer] **`expected_labels_for_story` removes any non-status manual label** [tools/issues-sync/src/sync.rs `expected_labels_for_story`] — deferred, spec-compliant ("status preserved" is the explicit invariant); footgun for maintainers using `pinned`/`area:*` is documented behavior.
- [x] [Review][Defer] **No CODEOWNERS entry for `tools/issues-sync/`** [governance] — deferred, separate governance scope; supply-chain concern given PAT-bearing workflow + GitHub Free unenforceable branch protection.
- [x] [Review][Defer] **`workflow_dispatch` has no `dry_run` input** [.github/workflows/sync-issues.yml:53] — deferred, nice-to-have; dry-run is local-only today.
- [x] [Review][Defer] **`dtolnay/rust-toolchain@stable` floating branch reference** [.github/workflows/sync-issues.yml:72] — deferred, matches the existing pattern in `pr.yml`; version-pin discipline drift is pre-existing.
- [x] [Review][Defer] **CRLF line-ending normalization missing** [tools/issues-sync/src/parser.rs] — deferred, repo files are LF; only fires if a contributor commits CRLF (`.gitattributes` already enforces).
- [x] [Review][Defer] **Code-fenced `### Story` headings match parser regex** [tools/issues-sync/src/parser.rs:140-148] — deferred, no docs story currently exists; would fire for a meta-story that demonstrates story format.
- [x] [Review][Defer] **Mis-nested story takes heading's epic, not section's** [tools/issues-sync/src/parser.rs:145] — deferred, data-entry error class; add a `debug_assert!` later if a defensive lane is desired.
- [x] [Review][Defer] **`epic: u8` overflow for Epic 256+** [tools/issues-sync/src/parser.rs:118] — deferred, won't happen in the spec horizon (max planned epic ≤ 13).
- [x] [Review][Defer] **Trailing post-Epic-13 content swallowed into last story body** [tools/issues-sync/src/parser.rs flush-at-EOF] — deferred, file currently ends cleanly; defensive fix would add an end-of-document sentinel check.
- [x] [Review][Defer] **Sentinel-line migration: ~117 body PATCHes on first real run** [tools/issues-sync/src/render.rs:25] — deferred, known one-time churn documented in Dev Agent Record §C (Caveats).
- [x] [Review][Defer] **Crash mid-run leaves inconsistent state, no resume token** [tools/issues-sync/src/sync.rs:137-173] — deferred, ties to the retry/backoff decision above; convergence-on-next-run is the documented contract.
- [x] [Review][Defer→Patch] **`extract_traces` `trim_start().starts_with("**Traces:**")` matches indented bullets** [tools/issues-sync/src/parser.rs:277-281] — **promoted to patch** because the `extract_ac_block_ignores_earlier_lookalike_terminator` regression test surfaced it; fixed to column-0-only match.
- [x] [Review][Defer] **`expected_milestone_num: None` silently drops milestone on create + reconcile** [tools/issues-sync/src/sync.rs:142] — deferred, defensive case (would require partial-fail in `ensure_milestones`); the abort-on-error path is fine for the steady state.
- [x] [Review][Defer] **`partition_labels` accepts the literal `"status:"` (no suffix) into the preserved set** [tools/issues-sync/src/sync.rs `partition_labels`] — deferred, narrow trigger (someone manually creates a bare `status:` label); cosmetic.

### Dismissed (verified safe / out of scope / nit)

- Pagination via `get_page::<Issue>(&p.next)` — loop is well-formed; `Page<T>.next: Option<Url>` propagates correctly to `Option<None>` exit.
- Cargo.lock missing → `--locked` would fail — verified `tools/issues-sync/Cargo.lock` IS committed and tracked.
- `pr.yml` doesn't exercise the new crate — verified Step 9.1 runs `cargo test --manifest-path tools/issues-sync/Cargo.toml --locked`.
- `update().body()` PATCH semantics (Blind Hunter explicitly skipped — cannot verify from diff alone; octocrab's typed builder sends only set fields per public API contract).
- `octocrab.graphql` envelope shape (Blind Hunter explicitly skipped — wiremock body-string assertions don't disambiguate, but the crate's documented signature accepts `serde_json::Value` wrappers).
- Concurrency group `cancel-in-progress: false` — verified safe; queued runs converge to final state.
- F32 (no `Link: rel="next"` in wiremock responses) — same dismiss as pagination.
- F38 (fork PRs cannot run this workflow) — verified safe; `push: main` + `workflow_dispatch` are maintainer-only.
- F34 (`Story.num` as `String` allows duplicates) — verified safe; `[Story 4.3a] X` and `[Story 4.3] Y` produce distinct titles.
- Several nits (lossy path conversion, dead-end match arms, `report.skipped_no_change` counter semantics) — informational only.
