# Contributing to Orgsidian

Thank you for your interest in contributing to Orgsidian. This document is the entry-point reference for setting up the project locally, the commit conventions, and the documentation/testing disciplines the codebase relies on. Each section below points to the canonical source rather than restating it — the deep content lives in the architecture document and the test-design document.

## 1. Development setup

### Toolchain prerequisites

- **Rust:** stable toolchain pinned via [`rust-toolchain.toml`](./rust-toolchain.toml). `rustup` will auto-install the pinned channel on the first `cargo` invocation. Required components: `rustfmt`, `clippy`.
- **Node.js:** 20.x LTS or later. The Lingui v6.x SWC plugin requires Node 18+ minimum; the project pins **LTS-preferred** per the version policy.
- **pnpm:** 9.x. Install via `npm i -g pnpm@9` or use Corepack (`corepack enable && corepack prepare pnpm@9 --activate`).
- **Tauri prerequisites (platform-specific):** see <https://tauri.app/v2/guides/prerequisites/> for the macOS / Linux / Windows native dependency tables. Summary: Xcode Command Line Tools on macOS; `webkit2gtk-4.1-dev` + `libsoup-3.0` on Ubuntu/Debian; MSVC build tools + WebView2 on Windows.

### First build

```sh
git clone https://github.com/orgsidian/orgsidian.git
cd orgsidian
pnpm install                          # commitlint + husky + shell-ui deps
cargo build --workspace --locked
pnpm tauri dev                        # launches the Tauri window
```

### CI parity check

Run this one-liner locally to exercise the exact per-PR gate set (Story 1.8 `pr.yml`):

```sh
cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked && cargo test --manifest-path tools/issues-sync/Cargo.toml --locked && cargo test --manifest-path tools/corpus-extractor/Cargo.toml --locked && pnpm typecheck && pnpm test && pnpm a11y
```

If this command passes locally, the per-PR CI matrix on macOS-arm64 + Ubuntu-LTS will pass too (modulo platform-specific differences caught by the nightly Windows + Arch sweep). (The L0 round-trip subset gate — the dedicated `L0 round-trip subset gate (LD-32/LD-44, <60s)` step in `pr.yml` — is a filtered subset of `cargo test --workspace`, so parity-wise the one-liner already covers it.)

## 2. Conventional Commits (LD-54)

All commits, PR titles, and CHANGELOG entries follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/).

### Type vocabulary

`feat`, `fix`, `perf`, `refactor`, `revert`, `docs`, `style`, `test`, `build`, `ci`, `chore`.

### Breaking changes

Signalled by `!` after the type (e.g., `feat!:`) or by a `BREAKING CHANGE:` footer.

### Scope discipline

Scope is optional but recommended. Canonical scopes are crate names (`parser`, `index`, `watcher`, `vault`, `plugin-api`, `report`, `core`, `cli`, `shell-app`) or `shell-ui` / `docs` / `ci`. No scope-value enum is enforced in commitlint — this avoids false-positive friction while keeping the convention discoverable.

### Examples per type

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

### CHANGELOG mapping

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

### Enforcement chain

- **Local:** [`commitlint.config.cjs`](./commitlint.config.cjs) + `husky` `commit-msg` hook (already configured) — every commit is validated as you type.
- **CI:** [`.github/workflows/commitlint.yml`](./.github/workflows/commitlint.yml) runs `pnpm commitlint --from origin/main --to HEAD` on every PR (commit-range gate) + [`amannn/action-semantic-pull-request@v5`](https://github.com/amannn/action-semantic-pull-request) on `pull_request_target` (PR-title gate). Both gates are advisory under GitHub Free (no enforceable branch protection); merge discipline is maintained by the maintainer's pre-merge check.
- **CHANGELOG generation:** [`cliff.toml`](./cliff.toml) + [`git-cliff`](https://git-cliff.org/) regenerates [`CHANGELOG.md`](./CHANGELOG.md) and [`crates/orgsidian-plugin-api/CHANGELOG.md`](./crates/orgsidian-plugin-api/CHANGELOG.md) from Conventional Commits on every `cargo release` (configured via [`release.toml`](./release.toml) `pre-release-hook`). Manual `### Deprecated` / `### Security` entries are inserted into `CHANGELOG.md` before tagging per LD-54. The mapping is smoke-tested by [`.github/workflows/release-smoke.yml`](./.github/workflows/release-smoke.yml) against a 5-commit fixture on every PR.

## 3. GitHub Issues sync (LD-55)

The canonical work-tracking surface is the [Orgsidian Roadmap GitHub Project board](https://github.com/orgs/orgsidian/projects/1) + per-story Issues in [`orgsidian/orgsidian`](https://github.com/orgsidian/orgsidian/issues). Both surfaces are one-way-synced from [`_bmad-output/planning-artifacts/epics.md`](./_bmad-output/planning-artifacts/epics.md) — the epics.md file is authoritative; manual Issue body edits are overwritten on next sync (status-label drift is preserved).

### Tooling

- **Binary:** [`tools/issues-sync/`](./tools/issues-sync/) — Rust binary using `octocrab` for the REST issues API + raw GraphQL for Projects v2 placement. Build with `cargo build --manifest-path tools/issues-sync/Cargo.toml --release --locked`. The crate is OUTSIDE `[workspace.members]` (LD-5 leaf-isolation, mirroring `tools/corpus-extractor/`).
- **Workflow:** [`.github/workflows/sync-issues.yml`](./.github/workflows/sync-issues.yml) runs the binary on push-to-main when `epics.md` changes. The workflow token is `secrets.PROJECTS_PAT` (a fine-grained PAT with `repo:issues:write` + `org:projects:write` — built-in `GITHUB_TOKEN` cannot access org-level Projects v2).
- **Local dry-run:** `GITHUB_TOKEN=$(gh auth token) ./tools/issues-sync/target/release/orgsidian-issues-sync --owner orgsidian --repo orgsidian --epics-path _bmad-output/planning-artifacts/epics.md --project-node-id PVT_kwDOEQxtTc4BZBHy --dry-run` prints a diff plan without mutating state.

### Idempotency contract

- Issues are looked up by exact title `[Story N.M] <title>`; matching issues are updated, missing ones are created with labels `epic:N, milestone:vX.X, type:story, status:backlog` + assigned to the milestone. Closed issues have their bodies updated (faithfulness to the spec) but never re-opened.
- `status:*` labels are NEVER touched on existing issues. Manual moves through `status:backlog → status:in-progress → status:in-review → status:done` are authoritative.
- Newly-created Issues are placed into the Project board's Backlog column via `addProjectV2ItemById`. Existing Issues missing from the board are also placed there on next sync — but Issues already on the board are not re-shuffled.

### When you edit `epics.md`

- Push to `main` (via PR merge); the workflow fires automatically.
- Or run the binary locally first to preview the diff: pass `--dry-run`.
- Or trigger a manual sync via `gh workflow run sync-issues.yml -R orgsidian/orgsidian`.

See [LD-55 in architecture.md](./_bmad-output/planning-artifacts/architecture.md#L617-L631) for the full label scheme + Project board configuration.

## 4. FR traceability discipline

Every module that implements a functional requirement (FR-NN, defined in the PRD) carries a doc-comment header naming the FR it implements. Concrete example using FR-12 (full-text search via SQLite FTS5):

```rust
//! Implements FR-12 (full-text search via SQLite FTS5).
```

**Live mapping.** The mapping from FR to module is reproducible at any time:

```sh
grep -r "Implements FR-" crates/ shell-ui/src/
```

**CI gate.** `tests/traceability.rs` at workspace root will (post-Story 2.x, once any FR-bearing module exists) parse the PRD's FR-NN enumeration and fail if any FR has no `Implements FR-NN` match in the codebase. The doc-comment is **not** aspirational documentation — when an FR-bearing story lands, the header is non-negotiable.

## 5. Fixture placement rule

**Default: co-located per crate.** Test fixtures live alongside the consuming crate, e.g., `crates/orgsidian-parser/tests/fixtures/anchor.org` (the Story 1.9 anchor fixture). One crate consumes → fixture is per-crate.

**Promotion to root `fixtures/`: only when ≥2 crates consume the same fixture.** The first promotion (Story 2.5: the LD-44 corpus manifests) created the root `fixtures/` directory. When promoting, add a short `README.md` inside the promoted folder naming the consumers so a future contributor can see why it's shared.

Solo fixtures stay per-crate; cross-crate fixtures only at root.

### Fixture governance (LD-44 / test-design §5)

Every fixture set is declared in [`fixtures/fixtures.toml`](./fixtures/fixtures.toml) and **owned by exactly one epic**. The rules:

- **Mutation requires PR review naming the owning epic.** Tag the commit message `[fixture:epic-N]` (e.g. `[fixture:epic-2]` for corpus changes). On GitHub Free, branch protection is unenforceable, so this is a documented convention checked by the maintainer pre-merge — the same advisory posture as the commitlint gates (§2).
- **Generated fixtures are never hand-edited.** `fixtures/subset-pr.json`, `fixtures/full-nightly.json`, and everything under `tests/fixtures/vault-corpus/{extracted,synthesized}/` are emitted by `tools/corpus-extractor`. Regeneration PRs must quote the generator invocation and the org-mode pin (tag + SHA-256):

  ```sh
  cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- fetch
  cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- extract
  cargo run --manifest-path tools/corpus-extractor/Cargo.toml --locked -- verify
  ```

- New fixture sets are added to `fixtures.toml` by the story that creates them; do not pre-declare paths that don't exist.

### git-LFS setup (vault corpus)

`tests/fixtures/vault-corpus/**/*.org` is designed to be versioned through git-LFS.

> **Current state (Story 2.5 fallback):** git-lfs was unavailable on the machine that generated the corpus, so the ~2.3 MB corpus is committed as **raw git objects** for now — the LFS stanza in `.gitattributes` is commented out with a `FOLLOWUP(LFS-migration)` marker (a scoped `-text` rule keeps the EOL-sensitive bytes intact), and the migration is tracked in deferred-work (owner: the first maintainer machine with git-lfs, after the Epic-2 story stack merges — no history rewrite while stacked PRs are open). Until that migration lands, no LFS setup is needed to read the corpus.

Once the migration lands, the one-time setup (only if you work on nightly/L2 gates or corpus regeneration) is:

```sh
git lfs install   # once per machine
git lfs pull      # materialize the corpus files
```

**The per-PR workflow does not require LFS.** The L0 subset is embedded in `fixtures/subset-pr.json` (regular git), so PR checkouts and the Story 2.6 gate never smudge LFS content. Tooling that does need real corpus bytes detects LFS pointer stubs and reports these setup steps instead of failing with a parse error.

## 6. MSRV policy

**Toolchain pin.** Orgsidian uses **stable Rust**, pinned via [`rust-toolchain.toml`](./rust-toolchain.toml). The workspace does **not** declare a `rust-version` field in [`Cargo.toml`](./Cargo.toml) because Orgsidian is a binary application (not a library published to crates.io) — the `rust-toolchain.toml` channel pin is the operational MSRV.

**Update cadence.** The pinned channel is bumped when a stable feature the project adopts requires it. Updates land via a `chore` commit touching `rust-toolchain.toml` and must pass the CI matrix (macOS-arm64 + Ubuntu-LTS per-PR; nightly Windows + Arch full sweep, per Story 1.8).

**`orgsidian-plugin-api` divergence (v1.5+).** When `orgsidian-plugin-api` publishes to crates.io (post v1.5 per LD-33), that crate **will** carry a `rust-version` field — at that point it becomes a library and MSRV becomes a public contract. The workspace MSRV otherwise tracks `rust-toolchain.toml`.

## 7. Testing strategy

The system-level testing strategy is owned by [`_bmad-output/test-artifacts/test-design.md`](./_bmad-output/test-artifacts/test-design.md) (TEA workflow, 2026-05-19).

That document defines: the three-level round-trip oracle (L0 per-PR / L1 nightly / L2 Emacs oracle), the anchor-smoke layer (§6.1), per-story-type red-phase scaffold templates (§7.3), the risk-prioritized coverage plan v0.1 → v1.0, and the failure-mode catalog mapping (LD-41 + Story 1.11).

Per the architecture's [Cross-Cutting Concerns header](./_bmad-output/planning-artifacts/architecture.md), `test-design.md` is the binding strategy for every story's red-phase scaffold (Process Discipline rule A); architecture LD-32 / LD-37 / LD-41 / LD-43 / LD-44 / LD-45 are referenced by it, not superseded.

Story 1.11 implements the LD-41 failure-mode harness; Story 1.12 implements the perf-snapshot regression macro consumed across the epics — both reference `test-design.md` as the source spec.

## 8. Parser ownership (LD-48)

The org-mode grammar at [`crates/orgsidian-parser/grammar/`](./crates/orgsidian-parser/grammar/) is a **SHA-pinned git submodule** vendoring [`nvim-orgmode/tree-sitter-org`](https://github.com/nvim-orgmode/tree-sitter-org). The vendoring discipline, the parser-owner role, and the upgrade process are mandated by [LD-48 in architecture.md](./_bmad-output/planning-artifacts/architecture.md#L1276). This section is the human-readable contract; the machine-readable pin is whatever SHA `git ls-tree HEAD crates/orgsidian-parser/grammar` reports.

### Role: parser owner

A single **parser owner** holds working familiarity with the `tree-sitter-org` grammar source and signs off on every submodule SHA bump. The role MUST exist at all times per LD-48; the identity of the holder is tracked outside this document (the GitHub org's maintainer list, or `MAINTAINERS.md` if/when it lands). For v0.1 Alpha the **current lead maintainer** fills the role by default; the wording stays role-agnostic so it remains accurate as the team grows. No separate ceremony is required until a second parser-touching contributor appears.

### SHA-pin discipline

The submodule is pinned to a specific commit SHA — **not** to a branch. The `.gitmodules` entry intentionally omits any `branch =` key. Running `git submodule update --remote` as a workflow shortcut is **forbidden**: it would fast-forward the local checkout to upstream HEAD and silently change the parser behaviour. Bumps land **only** through a reviewed PR (see below). The recorded gitlink in the parent-repo index (`git ls-tree HEAD crates/orgsidian-parser/grammar`) is the authoritative pin.

### Upgrade process (the LD-48 contingency mechanism)

When the parser owner wants to bump the pin:

1. **Enumerate** upstream commits since the last pin:
   ```sh
   git -C crates/orgsidian-parser/grammar log <current-SHA>..origin/main --oneline
   ```
2. **Review** every commit in the range for:
   - Grammar correctness regressions (any `grammar.js` change).
   - Test corpus changes (any `test/corpus/*.txt` change).
   - `scanner.c` edits — external-token logic, the highest-risk surface.
   - Node-type renames — the Story 2.2 wrapper depends on these strings.
3. **Open a PR** titled `chore(parser): bump tree-sitter-org to <SHA>`. The PR description includes:
   - The upstream commit-range diff link.
   - Which node-type strings were added / renamed / removed.
   - Whether the L0 round-trip subset (the `L0 round-trip subset gate` step in `pr.yml`) still passes.
4. **Sign-off**: the bump lands only after the parser owner's explicit approval in the PR. **No auto-bump** — Dependabot / Renovate MUST NOT be configured for this submodule.

### Fork-and-maintain dry run

[LD-48 reserves 2 weeks at the v0.3 milestone](./_bmad-output/planning-artifacts/architecture.md#L1276) for a fork-and-maintain dry run: the parser owner checks out upstream, builds from source, fixes a trivial issue, and runs the full parser test corpus. This section only **documents** the cadence; the dry-run itself is a v0.3-milestone task, not Story 2.1 scope.

### In-house fork trigger

Per [LD-48](./_bmad-output/planning-artifacts/architecture.md#L1276): if at any `v*` milestone upstream `nvim-orgmode/tree-sitter-org` has had no commits for more than 6 months, fork to `orgsidian-org/tree-sitter-org` and maintain in-house under MIT. The current parser-owner SHA-review log is the input to that decision.
