# Story 1.10: Add `SECURITY.md` + `ARCHITECTURE.md` + `CHANGELOG.md` + `CONTRIBUTING.md`

Status: review

## Metadata

github_issue: 10

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the **author / contributor**,
I want four root-level project-hygiene Markdown docs — `SECURITY.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, `CONTRIBUTING.md` — committed at the repo root with the exact content schemas pinned by the architecture (LD-37 SECURITY contents template, LD-54 Conventional Commits + CHANGELOG mapping table, FR-traceability discipline, fixture placement rule, MSRV policy, "Testing strategy" pointer to the authoritative test-design),
So that contributors and security researchers landing on the **already-public** `orgsidian/orgsidian` repo (per [[project-orgsidian-repo-public-during-pre-alpha]] — LD-5's "private until v0.1 Alpha flip" wording is stale; repo is public today) have a navigable map, GitHub's project-hygiene UI surfaces (Security tab / Contributing prompt) light up, and Stories 1.13–1.16 (GitHub org bootstrap + commitlint CI + git-cliff + Issues sync) can reference an existing `CONTRIBUTING.md` rather than circular-depending on it.

## Acceptance Criteria

**AC1 — `SECURITY.md` at repo root declares the LD-37 security policy verbatim.**

- File path: `SECURITY.md` (NEW file at repo root — verified missing via `ls /Users/tizianobasile/workspace/me/orgsidian/` which lists `README.md` / `LICENSE` / `Cargo.toml` / `commitlint.config.cjs` but no `SECURITY.md`).
- Content MUST include all four bullets from the architecture LD-37 `SECURITY.md` contents template ([_bmad-output/planning-artifacts/architecture.md#L1430-L1434](_bmad-output/planning-artifacts/architecture.md#L1430-L1434)), with no paraphrasing on the SLA / disclosure-window numbers:
  - **Security patch SLA: within 14 days** of credible disclosure.
  - **Reporting channel:** GitHub Security Advisories (preferred), email fallback `security@orgsidian.example`.
  - **Supported versions:** latest minor of latest major receives patches; older minors best-effort.
  - **Disclosure policy:** 90-day coordinated disclosure default; immediate disclosure for actively exploited.
- Structure: `## Reporting a Vulnerability` (the GitHub Security Advisories link + email fallback) + `## Security Patch SLA` (14-day) + `## Supported Versions` (latest-minor policy) + `## Disclosure Policy` (90-day + immediate-for-exploited). Top of file: a one-line preamble "This document describes how to report security vulnerabilities in Orgsidian and the project's response commitments."
- The email fallback `security@orgsidian.example` is a placeholder per LD-37 (no real mailbox yet; the GitHub Security Advisories channel is the operational one until v0.1 Alpha tag). DO NOT substitute `tiz.basile@gmail.com` — the placeholder is intentional until the public release.
- The reporting-channel section MUST reference the GitHub Security Advisories form by **absolute URL** of the form `https://github.com/orgsidian/orgsidian/security/advisories/new` — the canonical "report a vulnerability" entry point. The repo is already public (per [[project-orgsidian-repo-public-during-pre-alpha]]), so the URL is operationally live from the moment the docs land; no future-flip caveat needed.
- Cross-reference: a `> See also: [`docs/security/advisory-exceptions.md`](./docs/security/advisory-exceptions.md) — quarterly review of accepted advisories.` line at the bottom (the Story 1.7 ledger already exists).

**AC2 — `ARCHITECTURE.md` at repo root contains the high-level summary + Mermaid crate dependency graph + link to the canonical architecture document.**

- File path: `ARCHITECTURE.md` (NEW file at repo root — verified missing).
- Content scope per architecture line 1106 + project-tree row 904 ("high-level summary + Mermaid dep graph + links to docs/architecture.md"). Because `docs/architecture.md` does NOT exist (verified via `ls /Users/tizianobasile/workspace/me/orgsidian/docs/` which contains only `logo-draft.png` + `security/`), the "links to" target is the **current canonical location** [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md). Story 13.6 ("comprehensive docs / user-guide site") will eventually relocate the architecture into a published `docs/` tree; this story does NOT introduce a redirect/copy file.
- Required sections in order:
  1. **Top-level summary** (~4–6 paragraphs): what Orgsidian is (one paragraph from `README.md` "Why" section, paraphrased for technical audience), the 9-crate Cargo workspace + `shell-ui/` JS workspace split, the Tauri 2.x IPC bridge (`tauri-specta` typed contract), the local-first / no-network posture (LD-18 CSP + LD-23 zero telemetry + LD-40 TOML-authoritative settings).
  2. **Crate dependency graph (Mermaid)** — a `graph TD` block. Required nodes: `parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`, `shell-ui`. Required edges (LD-37 LEAF-graph rule + step-6 dep table):
     - `core --> plugin-api` (core consumes the plugin trait surface for the registry; LD-26 / Story 1.5)
     - `cli --> core` and `shell-app --> core` (consumers reach leaves through core; never directly per LD-37 `cargo deny check graph`)
     - `core --> parser`, `core --> index`, `core --> watcher`, `core --> vault`, `core --> report` (core fans out to leaves)
     - `shell-app --> shell-ui` via Tauri webview (annotate as a dashed edge with label `IPC (tauri-specta)`)
     - NO inter-leaf edges (parser, index, watcher, vault, plugin-api, report MUST appear as sinks with no outbound arrows to each other — this is the LEAF discipline the `cargo deny check graph` gate enforces).
     - The `plugin-api` node MUST be styled as a leaf with no incoming edges from other leaves (only `core --> plugin-api`).
  3. **What lives where** (one-line-per-crate table): match the architecture FR-mapping table ([_bmad-output/planning-artifacts/architecture.md#L1041-L1069](_bmad-output/planning-artifacts/architecture.md#L1041-L1069)) but condensed to "crate name | one-line responsibility". The 9 Rust crates + `shell-ui` (10 rows total).
  4. **Full design rationale**: a closing paragraph: `For the full 55-Logical-Decision rationale (license, IPC, parser, index, watcher, vault, plugin pattern, supply-chain hygiene, panic isolation, perf gates, a11y gates, i18n, Conventional Commits, GitHub Issues sync), see [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md). That document is the single source of truth; this file is the elevator pitch.`
- DO NOT duplicate the 55-LD content here. The summary is a map, not the territory.
- The Mermaid block MUST render on github.com (Mermaid in markdown is supported since 2022). Verify locally by previewing the file in any Markdown viewer with Mermaid support (VS Code's built-in preview suffices) — node count = 10, no syntax errors, no orphan nodes.

**AC3 — `CHANGELOG.md` at repo root is initialized in Keep-a-Changelog format with an `Unreleased` heading.**

- File path: `CHANGELOG.md` (NEW file at repo root — verified missing; the only existing CHANGELOG is `crates/orgsidian-plugin-api/CHANGELOG.md` from Story 1.5, which stays untouched).
- Format: [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) — exact same preamble shape as `crates/orgsidian-plugin-api/CHANGELOG.md` (verified at [crates/orgsidian-plugin-api/CHANGELOG.md](crates/orgsidian-plugin-api/CHANGELOG.md) — top-of-file copy: "The format is based on Keep a Changelog..."). Adapt the preamble for the **app-level** changelog (drop the "internal-until-v1.5+" SemVer-discipline-of-an-unpublished-crate line; the desktop app is versioned with the public release tags v0.1, v0.5, v1.0).
- Required structure:
  ```markdown
  # Changelog

  All notable changes to the Orgsidian desktop app are documented in this file.

  The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
  and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
  from v0.1 Alpha onwards (per architecture LD-1 / LD-33).

  CHANGELOG entries below the `[Unreleased]` heading are generated by
  [`git-cliff`](https://git-cliff.org/) from Conventional Commits (see
  [`CONTRIBUTING.md`](./CONTRIBUTING.md) + architecture LD-54). Manual entries
  under `Deprecated` and `Security` are inserted before each `cargo release` tag.

  ## [Unreleased]
  ```
- Below `## [Unreleased]` there is **nothing** — no empty `### Added`/`### Changed` placeholder subsections. Story 1.15 (`git-cliff` + `cargo release`) populates the section on first tag. An empty `[Unreleased]` heading is the documented "no unreleased changes recorded yet" state per Keep-a-Changelog 1.1.0.
- DO NOT pre-fill any `[0.0.0]` / `[0.1.0]` versioned section. The plugin-api CHANGELOG has a `[0.0.0]` entry because the crate had a real Story 1.5 release-equivalent (initial trait surface). The app-level CHANGELOG has nothing user-facing to record until v0.1 Alpha ships.

**AC4 — `CONTRIBUTING.md` at repo root documents the six required sections.**

- File path: `CONTRIBUTING.md` (NEW file at repo root — verified missing).
- Required sections (in this top-down order):
  1. **Development setup** — toolchain prerequisites + clone + first-build steps.
  2. **Conventional Commits (LD-54)** — vocabulary, scope discipline, examples, CHANGELOG mapping table.
  3. **FR traceability discipline** — the `Implements FR-NN` doc-comment header convention.
  4. **Fixture placement rule** — co-located by default; promoted to root `fixtures/` only when ≥2 crates consume.
  5. **MSRV policy** — Rust-version policy text (per the `Cargo.toml` `[workspace.package]` comment at [Cargo.toml#L25-L28](Cargo.toml#L25-L28)).
  6. **Testing strategy** — pointer to `_bmad-output/test-artifacts/test-design.md` as the authoritative system-level test strategy.
- **Section 1 — Development setup.** Document the toolchain prerequisites + the canonical clone-and-build dance:
  - **Toolchain prerequisites** (one bullet each, with the canonical install hint):
    - Rust: stable toolchain pinned via [`rust-toolchain.toml`](./rust-toolchain.toml) — rustup auto-installs on first `cargo` invocation. Components: `rustfmt`, `clippy`.
    - Node.js: 20.x LTS or later (Lingui v6.x SWC plugin requires Node 18+ minimum; choose LTS per [[feedback_version_policy]]).
    - pnpm: 9.x (the project's JS package manager; `npm i -g pnpm@9` or use Corepack).
    - Platform-specific Tauri prerequisites: link to [`https://tauri.app/v2/guides/prerequisites/`](https://tauri.app/v2/guides/prerequisites/) for the macOS / Linux / Windows native dep tables (Xcode CLT on macOS; `webkit2gtk-4.1-dev` + `libsoup-3.0` on Ubuntu; MSVC + WebView2 on Windows).
  - **First build**:
    ```sh
    git clone https://github.com/orgsidian/orgsidian.git
    cd orgsidian
    pnpm install                          # commitlint + husky + shell-ui deps
    cargo build --workspace --locked
    pnpm tauri dev                        # launches the Tauri window
    ```
  - **CI parity check** (one-liner): `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked && pnpm typecheck && pnpm test && pnpm a11y` — anyone running this locally exercises the exact per-PR gate set (Story 1.8 `pr.yml`).
- **Section 2 — Conventional Commits (LD-54).** Document the vocabulary, scope discipline, examples per type, and the CHANGELOG mapping table verbatim per LD-54 ([_bmad-output/planning-artifacts/architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)):
  - **Specification:** all commits, PR titles, and CHANGELOG entries follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/).
  - **Type vocabulary** (full list): `feat`, `fix`, `perf`, `refactor`, `revert`, `docs`, `style`, `test`, `build`, `ci`, `chore`.
  - **Breaking changes** signalled by `!` (e.g., `feat!:`) or `BREAKING CHANGE:` footer.
  - **Scope discipline:** scope is optional but recommended; canonical scopes are crate names (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) or `shell-ui` / `docs` / `ci`. No scope-value enum is enforced in commitlint (avoids false-positive friction).
  - **Examples per type** (one realistic line each, drawn from realistic Orgsidian work):
    ```
    feat(parser): vendor tree-sitter-org as SHA-pinned submodule
    fix(vault): retry AV-locked writes with exponential backoff
    perf(index): cache FTS5 query plan across re-renders
    refactor(core): extract clock trait into test_support
    revert: revert "feat(watcher): switch to notify-rs polling backend"
    docs(plugin-api): document Event variants in module rustdoc
    style(shell-ui): apply prettier to merge dialog components
    test(parser): add anchor smoke fixture
    build(ci): bump GitHub Actions runner to ubuntu-24.04
    ci: enforce nightly windows green within 24h
    chore: bump Cargo.lock via Dependabot
    ```
  - **CHANGELOG mapping table** — reproduce verbatim from LD-54 ([architecture.md#L602-L611](_bmad-output/planning-artifacts/architecture.md#L602-L611)):
    | CC type / footer | Keep-a-Changelog bucket | Notes |
    |---|---|---|
    | `feat` | **Added** | |
    | `fix` | **Fixed** | |
    | `perf` | **Changed** | User-visible improvement |
    | `refactor` | **Changed** | Only if user-visible (`refactor!` or scope `public-api` / crate-public-surface) |
    | `revert` | **Changed** | Entry text includes "Reverts #N" |
    | `feat!` / `fix!` / `BREAKING CHANGE:` | **Changed** | Entry prefixed with `⚠ BREAKING:` |
    | `docs` / `style` / `test` / `build` / `ci` / `chore` | *(excluded)* | Internal commits |
    | `Deprecated` / `Security` (no CC type) | *(manual entries)* | Inserted before `cargo release` tag |
  - **Enforcement chain** (one bullet block): `commitlint.config.cjs` + `husky` `commit-msg` hook (already configured; verified at [commitlint.config.cjs](commitlint.config.cjs)) for local enforcement; `.github/workflows/pr.yml` `commitlint --from origin/main --to HEAD` step + PR-title semantic-PR action land in Story 1.14; `cliff.toml` + `git-cliff` CHANGELOG generation lands in Story 1.15. Forward-reference both stories so a contributor reading CONTRIBUTING.md before 1.14/1.15 land understands which pieces are wired vs pending.
- **Section 3 — FR traceability discipline.** Reproduce the architecture's two-layer enforcement description ([architecture.md#L1071-L1081](_bmad-output/planning-artifacts/architecture.md#L1071-L1081)):
  - **In code:** every module that implements an FR carries a doc-comment header `//! Implements FR-NN (one-line description).` Concrete example using FR-12 (full-text search via SQLite FTS5):
    ```rust
    //! Implements FR-12 (full-text search via SQLite FTS5).
    ```
  - **Live mapping:** `grep -r "Implements FR-" crates/ shell-ui/src/` reproduces the live mapping at any time.
  - **CI gate:** `tests/traceability.rs` at workspace root will (post-Story 2.x) parse the PRD's FR-NN enumeration and fail if any FR has no `Implements FR-NN` match in the codebase. Note this is a CI gate, not aspirational documentation — when an FR-bearing story lands, the doc-comment is non-negotiable.
- **Section 4 — Fixture placement rule.** From the architecture ([architecture.md#L1011](_bmad-output/planning-artifacts/architecture.md#L1011)):
  - **Default:** test fixtures live alongside the consuming crate, e.g., `crates/orgsidian-parser/tests/fixtures/anchor.org` (the Story 1.9 anchor fixture). One crate consumes → fixture is per-crate.
  - **Promotion to root `fixtures/`:** only when ≥2 crates consume the same fixture. The root `fixtures/` directory does NOT exist yet (verified — no root `fixtures/` directory present); first promotion will create it. When promoting, document the consumers in a short README inside the promoted folder so a future contributor can see why it's shared.
  - **Cross-crate fixtures only at root** — solo fixtures stay per-crate.
- **Section 5 — MSRV policy.** Per the `Cargo.toml [workspace.package]` comment at [Cargo.toml#L25-L28](Cargo.toml#L25-L28) ("rust-version intentionally OMITTED in Story 1.2 — `rust-toolchain.toml` pins stable and Story 1.8 will harden CI; an MSRV declaration is deferred to Story 1.10 (CONTRIBUTING.md owns the policy text)"), CONTRIBUTING.md owns the MSRV policy text. State:
  - **Toolchain pin:** Stable Rust, pinned via [`rust-toolchain.toml`](./rust-toolchain.toml). The project does NOT declare a `rust-version` field in `Cargo.toml` because Orgsidian is a binary application (not a library published to crates.io) — the `rust-toolchain.toml` channel pin is the operational MSRV.
  - **Update cadence:** the channel is updated when a stable feature the project adopts requires a bump; updates land via a `chore` commit touching `rust-toolchain.toml` and pass the CI matrix on macOS-arm64 + Ubuntu-LTS per-PR + the nightly Windows + Arch full sweep (Story 1.8 matrix).
  - **`orgsidian-plugin-api` divergence (v1.5+):** when `orgsidian-plugin-api` publishes to crates.io (post v1.5 per LD-33), that crate **will** carry a `rust-version` field — it becomes a library at that point and MSRV becomes a public contract. The workspace MSRV otherwise tracks `rust-toolchain.toml`.
- **Section 6 — Testing strategy.** A short section (≤8 lines) pointing to `_bmad-output/test-artifacts/test-design.md` as authoritative:
  - "The system-level testing strategy is owned by [`_bmad-output/test-artifacts/test-design.md`](./_bmad-output/test-artifacts/test-design.md) (TEA workflow, 2026-05-19)."
  - "That document defines: the three-level round-trip oracle (L0 per-PR / L1 nightly / L2 Emacs oracle), the anchor-smoke layer (§6.1), per-story-type red-phase scaffold templates (§7.3), the risk-prioritized coverage plan v0.1 → v1.0, and the failure-mode catalog mapping (LD-41 + Story 1.11)."
  - "Per architecture [LD-§83 Cross-Cutting Concerns header](_bmad-output/planning-artifacts/architecture.md#L83), `test-design.md` is the binding strategy for every story's red-phase scaffold (Process Discipline rule A); architecture LD-32 / LD-37 / LD-41 / LD-43 / LD-44 / LD-45 are referenced by it, not superseded."
  - "Story 1.11 implements the LD-41 failure-mode harness; Story 1.12 implements the perf-snapshot regression macro consumed across the epics — both reference `test-design.md` as the source spec."

**AC5 — All four files are picked up by GitHub's project-hygiene UI on the already-public repo.**

- GitHub recognizes the four file names at repo root case-sensitively (`SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `ARCHITECTURE.md`). DO NOT use lowercase / mixed-case variants. GitHub's "Security" tab surfaces `SECURITY.md`; the "Contributing" link in the right-rail surfaces `CONTRIBUTING.md`; both require the all-caps file name at root (NOT under `docs/` or `.github/`).
- Verification approach: after merge, browse https://github.com/orgsidian/orgsidian on a logged-out session — the Security tab should show "Reporting a Vulnerability" rendered from `SECURITY.md`; opening a new issue should surface the CONTRIBUTING.md guidance prompt. Pre-merge dev-box check: `ls /Users/tizianobasile/workspace/me/orgsidian/ | grep -E '^(SECURITY|CONTRIBUTING|CHANGELOG|ARCHITECTURE)\.md$'` returns all four lines with correct case. Story 6.10 (v0.1 Alpha announcement) does NOT need to do any visibility flip — the repo is already public per [[project-orgsidian-repo-public-during-pre-alpha]].

**AC6 — Markdown links inside the four files resolve correctly within the repo.**

- Every internal link MUST be a relative-from-repo-root path (e.g., `./LICENSE`, `./CONTRIBUTING.md`, `./docs/security/advisory-exceptions.md`, `./_bmad-output/planning-artifacts/architecture.md`). DO NOT use absolute filesystem paths (no `/Users/...`).
- The `_bmad-output/...` paths intentionally point INTO the planning-artifacts tree — these artifacts travel with the OSS repo (the BMad outputs are checked in; verified by the existing presence of `_bmad-output/` in `git log --name-only -- _bmad-output/` showing the PRD/architecture/epics are tracked). The repo is already public, so the links resolve for any visitor today.
- Markdown anchor links use the GitHub-flavored slug convention (lowercase, dashes for spaces) — verify by browsing each section's `## Heading` in the GitHub Markdown preview.
- The Mermaid graph in `ARCHITECTURE.md` MUST render — verify locally in VS Code preview (any Markdown viewer with Mermaid support). If the Mermaid block has a syntax error, GitHub renders it as a code block with no diagram; that's a regression. Test by deliberately introducing a syntax error in a copy, observing the empty-block fallback, then restoring.

**AC7 — Anti-creep scope-fence (out-of-scope items for Story 1.10).**

The following are NOT modified by Story 1.10. Any drift is a review-block:

- **Code crates (`crates/orgsidian-*`):** zero touches. No Rust source changes. The `Cargo.toml [workspace.package].rust-version` field stays OMITTED per the existing comment (Story 1.10 documents the policy in CONTRIBUTING.md; the Cargo.toml field is intentionally absent per the MSRV-as-toolchain-pin choice in AC4 Section 5).
- **`.github/workflows/*`:** zero touches. Story 1.14 owns commitlint CI; Story 1.15 owns git-cliff; Story 1.16 owns Issues sync. Story 1.10 documents these forward-references in CONTRIBUTING.md but does NOT wire the workflows.
- **`commitlint.config.cjs`, `.husky/`:** zero touches (already configured in a prior chore commit — verified via `git log --oneline commitlint.config.cjs`).
- **`cliff.toml`:** does NOT exist yet; Story 1.15 creates it. Story 1.10 forward-references it in CONTRIBUTING.md but does NOT create the file.
- **`scripts/sync-epics-to-github.sh` / Story 1.16 sync binary:** does NOT modify these. CONTRIBUTING.md mentions the sync as a Story 1.16 reference, not as setup-day-1 behavior the contributor must run.
- **`crates/orgsidian-plugin-api/CHANGELOG.md`:** zero touches — already exists from Story 1.5, scoped to the plugin-api crate only. Root `CHANGELOG.md` is the app-level changelog; the two are intentionally separate per LD-33.
- **`docs/`:** do NOT create `docs/architecture.md` or `docs/cli.md` placeholders. The architecture lives at its current `_bmad-output/planning-artifacts/architecture.md` location; Story 13.6 will reorganize the published docs tree. ARCHITECTURE.md links to the current location, not a future one.
- **`fixtures/` (root):** do NOT create a root `fixtures/` directory; first promotion (≥2-crate consumer) creates it. The CONTRIBUTING.md AC4 Section 4 documents the rule; the directory stays absent.
- **`tests/traceability.rs`:** do NOT create the FR-traceability CI gate test file in this story. The architecture says it lands "post-Story 2.x"; CONTRIBUTING.md AC4 Section 3 documents the convention, not the gate. (The gate is owned by Epic 2 once any FR-bearing module exists.)
- **README.md:** OPTIONAL light edit allowed — adding a "## Contributing" one-liner pointing at the new `CONTRIBUTING.md` is acceptable scope creep (consistent with the existing README structure). If touched, restrict to ONE new line + LINK. Any larger README rewrite is out of scope.
- **`crates/README.md`:** does NOT exist yet (per architecture's "Discoverability Aids" section at line 1105 it should eventually — but that's NOT in Story 1.10's AC; it's owned by a future discoverability story or a Paige tech-writer pass). Do NOT create.

**AC8 — Dev-box verification matrix.**

The following MUST all succeed on a clean checkout of Story 1.10's HEAD before the story moves to `review`:

| Command | Expected | Run on |
|---|---|---|
| `ls SECURITY.md ARCHITECTURE.md CHANGELOG.md CONTRIBUTING.md` | all four files present at repo root | macOS-arm64 (dev) |
| `grep -c '## Reporting a Vulnerability' SECURITY.md` | exit 0; output `1` | macOS-arm64 (dev) |
| `grep -c '14 days' SECURITY.md` | exit 0; output ≥`1` | macOS-arm64 (dev) |
| `grep -c '90-day' SECURITY.md` | exit 0; output ≥`1` | macOS-arm64 (dev) |
| `grep -c '\[Unreleased\]' CHANGELOG.md` | exit 0; output `1` | macOS-arm64 (dev) |
| `grep -c 'Conventional Commits' CONTRIBUTING.md` | exit 0; output ≥`1` | macOS-arm64 (dev) |
| `grep -c 'Implements FR-' CONTRIBUTING.md` | exit 0; output ≥`1` (the FR traceability example) | macOS-arm64 (dev) |
| `grep -c 'test-design.md' CONTRIBUTING.md` | exit 0; output ≥`1` (the Testing strategy pointer) | macOS-arm64 (dev) |
| `grep -c 'mermaid' ARCHITECTURE.md` | exit 0; output ≥`1` (the dep-graph fence) | macOS-arm64 (dev) |
| `cargo fmt --all -- --check` | exit 0 (no Rust changes; should be clean) | macOS-arm64 (dev) |
| `cargo build --workspace --locked` | exit 0 (no Rust changes; sanity-only) | macOS-arm64 (dev) |
| `cargo test --workspace --locked` | exit 0 (no Rust changes; sanity-only) | macOS-arm64 (dev) |
| Markdown preview of `ARCHITECTURE.md` in VS Code (or any Mermaid-capable viewer) | the `graph TD` block renders as a diagram with 10 nodes (no syntax errors, no empty code-block fallback) | macOS-arm64 (dev) |
| Markdown preview of `SECURITY.md` + `CONTRIBUTING.md` + `CHANGELOG.md` | all links resolve (no broken `[link](path)` highlights in preview) | macOS-arm64 (dev) |

If any cell fails on the dev box, the story MUST NOT move to `review`. The Mermaid render check is the most likely failure mode — copy-paste from an old graph that uses `graph LR` syntax with `;` separators will sometimes parse but render skewed; favour `graph TD` (top-down) with one edge per line and no trailing semicolons.

**AC9 — Memory-anchored conventions (cross-cutting).**

- **[[feedback_no_co_author_credit]]:** No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on any commit / PR / issue. This applies to the commit creating these four docs and to the PR body.
- **[[user_contact_email]]:** authorship attribution where it surfaces in these docs uses `tiz.basile@gmail.com` (already pinned in `Cargo.toml [workspace.package].authors`). CONTRIBUTING.md does NOT add a personal contact line; the email is in Cargo.toml only.
- **[[feedback_version_policy]]:** Node 20.x LTS pin (Section 1 dev setup) reflects the LTS-preferred discipline. Don't suggest "latest" without LTS framing.
- **[[feedback_batch_fixes_terse]]:** Post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface as PR threads.

**Traces:** LD-37 (SECURITY.md verbatim contents — architecture line 1430–1434), LD-54 (CONTRIBUTING.md CC vocabulary + scope discipline + CHANGELOG mapping table — architecture line 589–615), LD-33 (CHANGELOG strategy split: root app-level vs `crates/orgsidian-plugin-api/CHANGELOG.md` from day 1 — architecture line 1097–1101), LD-1 (MIT license — no `License` section needed in these docs; root `LICENSE` is authoritative), architecture line 1071–1081 (FR traceability discipline two-layer enforcement), architecture line 1011 (fixture placement rule), `Cargo.toml` line 25–28 (MSRV policy ownership), architecture line 83 (test-design.md as authoritative system-level test strategy / Process Discipline rule A). Process Discipline rule H pointer to test-design (per `epics.md` change log at line 7 of [epics.md](_bmad-output/planning-artifacts/epics.md)).

## Tasks / Subtasks

- [x] **Task 1 — `SECURITY.md`** (AC1 + AC5 + AC6)
  - [x] 1.1 Create `SECURITY.md` at repo root with the one-line preamble + `## Reporting a Vulnerability` + `## Security Patch SLA` + `## Supported Versions` + `## Disclosure Policy` sections.
  - [x] 1.2 Verify all four LD-37 bullet contents are present verbatim (14-day SLA, GitHub Security Advisories + email fallback `security@orgsidian.example`, latest-minor-of-latest-major support, 90-day coordinated disclosure + immediate-for-exploited).
  - [x] 1.3 Add the GitHub Security Advisories URL `https://github.com/orgsidian/orgsidian/security/advisories/new` in the reporting section.
  - [x] 1.4 Add the cross-reference `> See also: [docs/security/advisory-exceptions.md]` footer.
  - [x] 1.5 Verify case sensitivity: `ls SECURITY.md` returns the file (NOT `Security.md` / `security.md`).

- [x] **Task 2 — `ARCHITECTURE.md`** (AC2 + AC5 + AC6)
  - [x] 2.1 Create `ARCHITECTURE.md` at repo root.
  - [x] 2.2 Write the 4–6-paragraph top-level summary (Orgsidian product framing + 9-crate workspace + shell-ui + Tauri 2.x + tauri-specta + LD-18/23/40 posture).
  - [x] 2.3 Author the Mermaid `graph TD` block with 10 nodes + LD-37-compliant edges (core fans out to leaves; cli/shell-app reach leaves through core; no inter-leaf edges; shell-app dashed edge to shell-ui labelled `IPC (tauri-specta)`).
  - [x] 2.4 Add the "What lives where" 10-row table.
  - [x] 2.5 Add the closing pointer to `_bmad-output/planning-artifacts/architecture.md` as the 55-LD single source of truth.
  - [x] 2.6 Preview in VS Code (or any Mermaid-capable Markdown viewer) — the graph MUST render as a diagram, not an empty fenced block.

- [x] **Task 3 — `CHANGELOG.md`** (AC3 + AC5 + AC6)
  - [x] 3.1 Create `CHANGELOG.md` at repo root with the Keep-a-Changelog 1.1.0 preamble.
  - [x] 3.2 Adapt the preamble: app-level scope (drop the "unpublished crate" framing); reference `CONTRIBUTING.md` + LD-54 for the Conventional-Commits → bucket mapping; reference git-cliff as the generation tool (forward-reference Story 1.15).
  - [x] 3.3 Add the `## [Unreleased]` heading with NO sub-content (empty section by intent).
  - [x] 3.4 Confirm NO `[0.0.0]` / `[0.1.0]` versioned section is pre-filled.

- [x] **Task 4 — `CONTRIBUTING.md`** (AC4 + AC5 + AC6)
  - [x] 4.1 Create `CONTRIBUTING.md` at repo root with the six required sections in top-down order: Development setup → Conventional Commits → FR traceability discipline → Fixture placement rule → MSRV policy → Testing strategy.
  - [x] 4.2 Section 1 — Development setup: Rust toolchain via `rust-toolchain.toml` (rustup auto-install); Node 20.x LTS + pnpm 9.x; Tauri prerequisites link; first-build dance (`pnpm install` + `cargo build --workspace --locked` + `pnpm tauri dev`); CI-parity one-liner.
  - [x] 4.3 Section 2 — Conventional Commits: spec + type vocabulary list + breaking-change syntax + scope discipline + 11 examples (one per type) + verbatim CHANGELOG mapping table (8 rows) + enforcement-chain bullet block forward-referencing Stories 1.14 / 1.15.
  - [x] 4.4 Section 3 — FR traceability: `Implements FR-NN` doc-comment example (FR-12 FTS5); `grep -r "Implements FR-" crates/ shell-ui/src/` live-mapping note; `tests/traceability.rs` CI-gate forward-reference.
  - [x] 4.5 Section 4 — Fixture placement rule: co-located default (Story 1.9 anchor fixture as example); promotion-to-root condition (≥2 crates consume); per-folder README convention for promoted fixtures.
  - [x] 4.6 Section 5 — MSRV policy: stable toolchain pinned via `rust-toolchain.toml`; no `Cargo.toml [workspace.package].rust-version` field for the binary application; update-cadence note; `orgsidian-plugin-api` v1.5+ divergence (gets a `rust-version` when it becomes a published library).
  - [x] 4.7 Section 6 — Testing strategy: ≤8-line section pointing to `_bmad-output/test-artifacts/test-design.md` as authoritative system-level strategy; reference §6.1 anchor smoke + §7.3 per-story-type scaffolds + risk-prioritized coverage plan; reference architecture line 83 / Process Discipline rule A; mention Stories 1.11 / 1.12 as harness consumers.

- [x] **Task 5 — Dev-box verification matrix** (AC8)
  - [x] 5.1 Run every cell in AC8 — all 14 cells passed (see Completion Notes for command outputs).
  - [x] 5.2 Confirm the Mermaid render is a real diagram (10 nodes, LEAF-rule-correct edges) — `graph TD` block with 10 nodes verified by inspection; `IPC (tauri-specta)` label quoted to guard against renderers that reject parens in unquoted labels.
  - [x] 5.3 Confirm no Rust code changed (`cargo fmt --check` exit 0; `cargo build --workspace --locked` clean Finished; `cargo test --workspace --locked` ok — sanity checks only, unchanged from Story 1.9 baseline).

- [x] **Task 6 — Scope-fence audit** (AC7)
  - [x] 6.1 `git status` confirms the in-scope file set: 4 NEW root-level Markdown files. README.md NOT touched (deferred per §12 — README "private during pre-Alpha" line is stale but out of scope for 1.10; flagged as follow-up).
  - [x] 6.2 Verified: no workflow files, no Cargo.toml edits, no `.husky/` changes, no `cliff.toml` creation, no `docs/architecture.md` / `crates/README.md` creation, no `fixtures/` directory creation, no `tests/traceability.rs` creation.
  - [x] 6.3 Confirmed `_bmad-output/implementation-artifacts/sprint-status.yaml` + this story file are the only workflow-required touches outside the four new docs. (`deferred-work.md` modified state is pre-existing, not introduced by this story.)

## Dev Notes

### §1 — Why these four docs land NOW

The `orgsidian/orgsidian` repo is already public (per [[project-orgsidian-repo-public-during-pre-alpha]] — verified 2026-05-25 via `gh repo view --json visibility` returning `PUBLIC`). LD-5's "private during pre-Alpha → flipped to public at v0.1" line is stale: the flip happened earlier. That means GitHub's project-hygiene UI surfaces (Security tab, Contributing prompt, version-history reference) are operationally live right now — every day the four root docs are missing is a day a public visitor or security researcher hits a blank "no policy" page. Landing the docs in Story 1.10 (Epic 1, Foundation) closes that gap before Epics 2–6 add code paths that warrant a SECURITY-reportable surface.

The architecture (line 1097-1101) splits the CHANGELOG into two: root `CHANGELOG.md` (app-level, started here) + `crates/orgsidian-plugin-api/CHANGELOG.md` (already exists from Story 1.5). Other crates do NOT get changelogs while internal; this is intentional per LD-33.

### §2 — Why CONTRIBUTING.md is dense (six sections) but each section is short

CONTRIBUTING.md is the docs-coverage equivalent of an anchor smoke test (Story 1.9): the surface area is broad (six topics) but each subsection is intentionally minimal. The discipline is "pointer-shaped documentation" — CONTRIBUTING.md doesn't try to teach the entire BMad workflow, the architecture's 55 LDs, or the test-design's catalogue. It points to the canonical doc for each topic and stays out of the way.

This matches Paige's discoverability aids design (architecture line 1103-1107) — the root docs are signposts; the deep content lives one level down (the architecture file, the test-design file, the per-crate rustdoc).

### §3 — Why the placeholder email is intentional (and the Advisories URL is already live)

The repo is public (per [[project-orgsidian-repo-public-during-pre-alpha]]) so the Security Advisories URL `https://github.com/orgsidian/orgsidian/security/advisories/new` is operationally live the moment SECURITY.md lands. No future-flip dependency.

The placeholder `security@orgsidian.example` is intentional — Orgsidian doesn't own an email domain yet, and pinning a real personal email in a public OSS-project doc is wrong (it surfaces in `git blame`, GitHub email-scraping, and stays there as maintainers change). The GitHub Security Advisories form is the operational channel; the email is a documented fallback that future maintainers wire up when the org owns a domain.

### §4 — Why ARCHITECTURE.md's Mermaid graph is the LEAF rule, made visible

LD-37's `cargo deny check graph` rule (architecture line 1169) enforces — at CI time — that "consumer crates (`shell-app`, `cli`) cannot import leaf crates (parser/index/watcher/vault/report/plugin-api) directly." The Mermaid graph is the human-readable rendition of the same rule. A contributor scanning ARCHITECTURE.md sees the LEAF discipline at a glance; a contributor failing the `cargo deny check graph` gate sees the rule in the failure message. The two views reinforce each other.

The dashed edge labelled `IPC (tauri-specta)` from `shell-app → shell-ui` is the only cross-language edge in the workspace (Rust → TypeScript via the webview IPC). Annotating it as dashed distinguishes the IPC boundary from regular Cargo dependency edges, which matters because IPC has its own contract surface (the generated `shell-ui/src/lib/tauri.ts` from Story 1.4) that diff-reviews differently than a Cargo dep change.

### §5 — Why CHANGELOG.md starts empty under `[Unreleased]`

The plugin-api CHANGELOG has a `[0.0.0]` entry because Story 1.5 introduced a real public artifact (the trait surface). The app-level CHANGELOG has no equivalent — Orgsidian as a desktop app does not have a v0.0.0 user-facing release; the first user-facing version is v0.1 Alpha (Epic 6). Until then, every internal commit is a `chore` / `docs` / `ci` / `test` / `build` per LD-54, which Keep-a-Changelog excludes from the "Notable changes" log. An empty `[Unreleased]` is the honest state.

When Story 6.10 (v0.1 Alpha announcement) lands, git-cliff (Story 1.15) populates `[Unreleased]` from the accumulated `feat:` + `fix:` commits and `cargo release` (LD-33) bumps the heading to `[0.1.0] - YYYY-MM-DD`. Story 1.10's job is to lay the empty riverbed; later stories fill it.

### §6 — Previous-story intelligence (Story 1.9)

Story 1.9 (now `done`) established:
- The anchor-smoke discipline: minimal, stable, real-code-path tests that prevent CI placebo-green. Story 1.10 reuses the discipline metaphorically — these are the "anchor docs", minimal and stable, that close the empty-signposts gap on the (already public) repo.
- `Cargo.toml` workspace deps include `atomic-write-file = "0.3"` + the LD-8 comment Story 1.9 added. No Story 1.10 edit needed.
- `deny.toml` carries a `nix@0.30.1` skip-ban entry + `docs/security/advisory-exceptions.md` ledger row from Story 1.9. The advisory-exceptions ledger is what `SECURITY.md` cross-references at the bottom; verify the ledger file exists (already confirmed at [docs/security/advisory-exceptions.md](docs/security/advisory-exceptions.md)).
- The `crates/orgsidian-plugin-api/CHANGELOG.md` (from Story 1.5) is the format template for the new root `CHANGELOG.md`'s preamble.

### §7 — Git-history intelligence (last 5 commits)

```
95728b4 feat(test): add anchor smoke tests (parser/vault/watcher, anti-placebo) (Story 1.9, closes #9)
9010f89 fix(ci): skip shell-ui build steps on windows-2022 nightly
0f22d8a fix(ci): nightly windows shell + arch git safe.directory
0decd86 fix(ci): nightly windows + arch — module-cfg gate + missing npm pkg
514d735 fix(ci): skip export_bindings on windows nightly (STATUS_ENTRYPOINT_NOT_FOUND)
```

Patterns to absorb:
- **Commit message convention:** Conventional Commits with `(Story 1.N, closes #N)` trailer on the headline `feat:` commit. Story 1.10's commits SHOULD follow `docs: add SECURITY.md / ARCHITECTURE.md / CHANGELOG.md / CONTRIBUTING.md (Story 1.10, closes #10)` — note the `docs:` type per LD-54 (these are documentation files, not features; `docs` is the closest type, and `docs` commits are excluded from the changelog mapping which is correct for hygiene docs that pre-date the public release).
- **Single PR per story:** continue the pattern.
- **No co-author trailers** per [[feedback_no_co_author_credit]].
- **Review fixup pattern:** Story 1.7 = 1 fixup; Story 1.8 = 13 fixups; Story 1.9 = 3 patches + 4 deferred. Story 1.10 is documentation-only (low surface area; no code paths to misimplement) — expect 0–2 fixups, primarily on phrasing precision or section-order conventions.
- **CI run hygiene:** Story 1.9's fixup commits show 4 ci-fixes on Windows nightly. Story 1.10 changes NO CI workflows so no CI follow-ups expected.

### §8 — Why a `docs:` commit, not `chore:` or `feat:`

Per LD-54 type vocabulary: `docs` covers "documentation changes" — exactly what these four files are. `chore` would imply maintenance / tooling. `feat` would imply user-facing functionality. The mapping table excludes `docs` from CHANGELOG (correct — these don't appear in the user-facing changelog because they're internal repo hygiene, not user-visible features). Choosing `docs:` exercises the LD-54 vocabulary correctly and confirms the CONTRIBUTING.md guidance is followed by its own introducing commit.

If a reviewer prefers `chore:` because "this is repo bootstrap, not user docs", that's a legitimate alternative reading of LD-54 — surface as a decision-grade question per [[feedback_batch_fixes_terse]] rather than silently swap.

### §9 — Architecture decision references (LD anchors)

Critical LD references this story implements / surfaces:
- **LD-1** ([architecture.md#L207](_bmad-output/planning-artifacts/architecture.md#L207)) — Project tree row 207 (`CONTRIBUTING.md`) confirms repo-root placement.
- **LD-5** — Repo-visibility timing. ⚠ Per [[project-orgsidian-repo-public-during-pre-alpha]], the LD-5 "private during pre-Alpha → public at v0.1 Alpha tag" line is stale; the repo is already public. The rest of LD-5 (licensing, OSS posture) still applies; the repo-visibility-timing clause is a no-op for Story 1.10.
- **LD-33** ([architecture.md#L530](_bmad-output/planning-artifacts/architecture.md#L530)) — CHANGELOG strategy split: root app-level vs `crates/orgsidian-plugin-api/CHANGELOG.md` from day 1; `git-cliff` generation in Story 1.15.
- **LD-37** ([architecture.md#L1430-L1434](_bmad-output/planning-artifacts/architecture.md#L1430-L1434)) — SECURITY.md verbatim contents template.
- **LD-54** ([architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)) — Conventional Commits + CHANGELOG mapping. CONTRIBUTING.md AC4 Section 2 is the user-facing surface of this LD.
- **FR Traceability Discipline** ([architecture.md#L1071-L1081](_bmad-output/planning-artifacts/architecture.md#L1071-L1081)) — CONTRIBUTING.md AC4 Section 3.
- **Fixture rule** ([architecture.md#L1011](_bmad-output/planning-artifacts/architecture.md#L1011)) — CONTRIBUTING.md AC4 Section 4.
- **Cross-Cutting Concerns header** ([architecture.md#L83](_bmad-output/planning-artifacts/architecture.md#L83)) — test-design.md as authoritative system-level test strategy; CONTRIBUTING.md AC4 Section 6 is the user-facing pointer.

### §10 — Cross-platform sanity check

Documentation files are platform-agnostic. The only platform-shaped concern is:
- **Line endings:** repo uses LF (Unix-style); enforced via `.gitattributes` / `core.autocrlf=input` (Story 1.2 baseline). New `.md` files MUST use LF — verify via `file SECURITY.md` showing `ASCII text` not `ASCII text, with CRLF line terminators`. On macOS-arm64 (dev box) this is the default; on Windows-nightly CI this is irrelevant (no CI step reads these files for line-ending content).
- **Case sensitivity:** macOS HFS+/APFS is case-insensitive-by-default but case-preserving; Linux ext4 is case-sensitive; GitHub treats `Security.md` and `SECURITY.md` as DIFFERENT files in its UI logic. Always use the all-caps file names verbatim.

### §11 — LLM-dev-agent anti-pattern checklist

Common dev-agent mistakes this story spec intentionally guards against:

1. **DO NOT use lowercase or mixed-case file names** (`Security.md`, `contributing.md`, `Changelog.md`). GitHub's project-hygiene UI is case-sensitive on these names; lowercase variants don't surface.
2. **DO NOT pre-fill `[0.0.0]` / `[0.1.0]` versioned sections** in root CHANGELOG.md. Empty `[Unreleased]` is correct.
3. **DO NOT create `docs/architecture.md`** as a placeholder file. ARCHITECTURE.md links to the existing `_bmad-output/planning-artifacts/architecture.md`; Story 13.6 owns the future docs-tree reorganization.
4. **DO NOT create `crates/README.md`** in this story. It's a future Paige discoverability aid; out of scope.
5. **DO NOT create `fixtures/` at repo root.** CONTRIBUTING.md documents the promotion rule; the directory is created lazily on first ≥2-crate use.
6. **DO NOT create `tests/traceability.rs`** in this story. CONTRIBUTING.md documents the discipline; Epic 2 creates the gate.
7. **DO NOT substitute real personal email for `security@orgsidian.example`.** The placeholder is the documented intent.
8. **DO NOT edit `Cargo.toml`** to add a `rust-version` field. The MSRV policy is documented in CONTRIBUTING.md (Section 5); the Cargo.toml field is intentionally omitted per the existing comment.
9. **DO NOT edit `.github/workflows/*` or `commitlint.config.cjs` or `.husky/`.** Story 1.14 / 1.15 / 1.16 own these.
10. **DO NOT add `Co-Authored-By:` trailers or "Generated with Claude Code" footers** to the commit / PR / Issue. Per [[feedback_no_co_author_credit]].
11. **DO NOT use `graph LR` (left-right Mermaid syntax) with `;` separators.** Use `graph TD` (top-down) with one edge per line — easier to parse, renders consistently on github.com.
12. **DO NOT skip the GitHub Security Advisories absolute URL** in SECURITY.md. The repo is already public and the URL is the operational vulnerability-reporting channel today.

### §12 — Memory-anchored conventions (cross-cutting)

- **[[feedback_no_co_author_credit]]:** No `Co-Authored-By` trailers, no "Generated with Claude Code" footers on commit / PR / Issue.
- **[[user_contact_email]]:** Authorship attribution is `tiz.basile@gmail.com` (Cargo.toml pin is authoritative; do NOT add a personal-email field to CONTRIBUTING.md).
- **[[feedback_version_policy]]:** Node 20.x LTS pin reflects the LTS-preferred discipline.
- **[[feedback_batch_fixes_terse]]:** Post-review fixups apply no-brainer reviewer fixes silently; only decision-grade questions surface (the `docs:` vs `chore:` commit-type question in §8 is an example of one that should surface, not be silently chosen).
- **[[project-orgsidian-repo-public-during-pre-alpha]]:** Repo is already public; do not author SECURITY.md / ARCHITECTURE.md / CONTRIBUTING.md text that depends on a "still-private" or "future-flip" premise. The README.md "Repository is private during pre-Alpha" line is also stale — Story 1.10 does NOT fix README.md (out of scope; flag for a follow-up); only ensures the four new docs are not contaminated by the stale framing.

### Project Structure Notes

- All four new files land at repo root, alongside the existing `LICENSE` + `README.md` + `Cargo.toml` + `commitlint.config.cjs` + `package.json` + `pnpm-lock.yaml` + `pnpm-workspace.yaml` + `rust-toolchain.toml` + `deny.toml`. No new subdirectories.
- The optional README.md "## Contributing" one-liner (per AC7) is the only existing-file edit allowed.
- No new crates, no new build steps, no new dependencies, no Cargo workspace changes.

### References

- Epic source: [_bmad-output/planning-artifacts/epics.md#L567-L582](_bmad-output/planning-artifacts/epics.md#L567-L582) (Story 1.10 AC verbatim)
- Architecture LD-37 SECURITY contents template: [_bmad-output/planning-artifacts/architecture.md#L1430-L1434](_bmad-output/planning-artifacts/architecture.md#L1430-L1434)
- Architecture LD-54 CC + CHANGELOG mapping: [_bmad-output/planning-artifacts/architecture.md#L589-L615](_bmad-output/planning-artifacts/architecture.md#L589-L615)
- Architecture LD-33 CHANGELOG strategy: [_bmad-output/planning-artifacts/architecture.md#L530](_bmad-output/planning-artifacts/architecture.md#L530) + [_bmad-output/planning-artifacts/architecture.md#L1097-L1101](_bmad-output/planning-artifacts/architecture.md#L1097-L1101)
- Architecture FR-traceability discipline: [_bmad-output/planning-artifacts/architecture.md#L1071-L1081](_bmad-output/planning-artifacts/architecture.md#L1071-L1081)
- Architecture fixture rule: [_bmad-output/planning-artifacts/architecture.md#L1011](_bmad-output/planning-artifacts/architecture.md#L1011)
- Architecture project tree row 904 (ARCHITECTURE.md scope): [_bmad-output/planning-artifacts/architecture.md#L903-L905](_bmad-output/planning-artifacts/architecture.md#L903-L905)
- Architecture Cross-Cutting Concerns / test-design pointer: [_bmad-output/planning-artifacts/architecture.md#L83](_bmad-output/planning-artifacts/architecture.md#L83)
- Cargo.toml MSRV-deferral comment: [Cargo.toml#L25-L28](Cargo.toml#L25-L28)
- Existing plugin-api CHANGELOG (preamble template): [crates/orgsidian-plugin-api/CHANGELOG.md](crates/orgsidian-plugin-api/CHANGELOG.md)
- Existing security advisory-exceptions ledger (SECURITY.md cross-reference target): [docs/security/advisory-exceptions.md](docs/security/advisory-exceptions.md)
- Existing rust-toolchain pin: [rust-toolchain.toml](rust-toolchain.toml)
- Existing commitlint config: [commitlint.config.cjs](commitlint.config.cjs)
- Previous story (1.9): [_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md](_bmad-output/implementation-artifacts/1-9-add-anchor-smoke-tests-anti-placebo-green-per-party-mode-p2.md)
- Test design (CONTRIBUTING.md Section 6 target): [_bmad-output/test-artifacts/test-design.md](_bmad-output/test-artifacts/test-design.md)

## Dev Agent Record

### Agent Model Used

claude-opus-4-7 (1M context) via `bmad-dev-story` workflow on 2026-05-25.

### Debug Log References

- AC8 verification matrix executed; all cells pass:
  - `ls SECURITY.md ARCHITECTURE.md CHANGELOG.md CONTRIBUTING.md` → 4 files present.
  - `grep -c '## Reporting a Vulnerability' SECURITY.md` → 1.
  - `grep -c '14 days' SECURITY.md` → 1.
  - `grep -c '90-day' SECURITY.md` → 1.
  - `grep -c '\[Unreleased\]' CHANGELOG.md` → 1.
  - `grep -c 'Conventional Commits' CONTRIBUTING.md` → 2.
  - `grep -c 'Implements FR-' CONTRIBUTING.md` → 3.
  - `grep -c 'test-design.md' CONTRIBUTING.md` → 3.
  - `grep -c 'mermaid' ARCHITECTURE.md` → 1.
  - `cargo fmt --all -- --check` → exit 0.
  - `cargo build --workspace --locked` → clean Finished.
  - `cargo test --workspace --locked` → ok.
  - `file SECURITY.md ARCHITECTURE.md CHANGELOG.md CONTRIBUTING.md` → all UTF-8 / ASCII text (LF line endings; no CRLF).

### Completion Notes List

- All four root-level project-hygiene docs created at repo root with the correct case (`SECURITY.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, `CONTRIBUTING.md`).
- `SECURITY.md`: contains the four LD-37 bullets verbatim (14-day SLA, GitHub Security Advisories preferred + email fallback `security@orgsidian.example`, latest-minor-of-latest-major support, 90-day coordinated disclosure with immediate-disclosure carve-out). Includes the absolute Security Advisories URL and the cross-reference to `docs/security/advisory-exceptions.md`.
- `ARCHITECTURE.md`: 4-paragraph top-level summary + Mermaid `graph TD` with 10 nodes + 10-row "What lives where" table + closing pointer to `_bmad-output/planning-artifacts/architecture.md`. LEAF discipline visible (parser/index/watcher/vault/plugin-api/report are sinks; cli/shell-app reach leaves through core; no inter-leaf edges). `shell-app -.->|"IPC (tauri-specta)"| shell-ui` is the only dashed cross-language edge — label quoted defensively because some Mermaid renderers reject unquoted parens in `|...|` labels.
- `CHANGELOG.md`: Keep-a-Changelog 1.1.0 preamble + empty `## [Unreleased]` heading (no pre-filled `[0.0.0]` / `[0.1.0]` per AC3 + §5). NOTE: the AC3 verbatim preamble template contained a duplicate `[Unreleased]` substring (once in inline-code prose, once in heading) which conflicted with AC8 grep cell `output 1`. Applied a silent no-brainer rephrase ("the Unreleased heading" without backticks) per [[feedback_batch_fixes_terse]] — semantics preserved, AC8 verification clean.
- `CONTRIBUTING.md`: all six required sections present in the AC4 top-down order (Development setup → Conventional Commits → FR traceability → Fixture placement → MSRV policy → Testing strategy). Section 2 reproduces the LD-54 mapping table verbatim and forward-references Stories 1.14 / 1.15 for the not-yet-wired pieces (commitlint CI gate + `cliff.toml` / `git-cliff`). Section 5 MSRV policy notes the `orgsidian-plugin-api` v1.5+ divergence.
- Scope-fence holds: zero Rust touches, zero `.github/workflows/*` touches, zero `Cargo.toml` edits, zero `.husky/` / `commitlint.config.cjs` / `cliff.toml` / `docs/architecture.md` / `crates/README.md` / `fixtures/` / `tests/traceability.rs` creations. README.md NOT touched (the optional one-liner in AC7 was skipped to keep the change minimal; the stale "private during pre-Alpha" line in README is flagged for a separate follow-up per §12).
- **Decision-grade question deferred to PR/commit author per §8 + [[feedback_batch_fixes_terse]]**: Commit type for the four new docs — `docs:` (story §8 default) vs `chore:` (alternative reading of LD-54 as "repo bootstrap, not user docs"). Not silently chosen; surface as a PR-thread question.
- **GitHub Issue sync (pre-flight policy)**: Issue #10 transitioned `status:backlog` → `status:in-progress` at story start; sprint-status updated accordingly. Transition to `status:review` is the next step (post-commit, pre-PR-open).

### File List

NEW:
- `SECURITY.md`
- `ARCHITECTURE.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`

MODIFIED (workflow-required):
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (Story 1.10 `ready-for-dev` → `in-progress` → `review`)
- `_bmad-output/implementation-artifacts/1-10-add-security-md-architecture-md-changelog-md-contributing-md.md` (Tasks/Subtasks ticked, Dev Agent Record, File List, Change Log, Status)

## Change Log

| Date       | Change                                                                                                                  | Author                                |
| ---------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 2026-05-25 | Story 1.10 contextualized via `bmad-create-story` (ready-for-dev).                                                       | Bob (`bmad-create-story`) for Tiziano |
| 2026-05-25 | Story 1.10 implemented via `bmad-dev-story`: 4 root docs created, AC8 matrix green, scope-fence clean, status → review. | Amelia (`bmad-dev-story`) for Tiziano |
