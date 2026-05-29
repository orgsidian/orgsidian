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
cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked && pnpm typecheck && pnpm test && pnpm a11y
```

If this command passes locally, the per-PR CI matrix on macOS-arm64 + Ubuntu-LTS will pass too (modulo platform-specific differences caught by the nightly Windows + Arch sweep).

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
- **CHANGELOG generation:** `cliff.toml` + `git-cliff` (invoked by `cargo release` pre-tag hook) lands in **Story 1.15** (not yet created). Until then, the root [`CHANGELOG.md`](./CHANGELOG.md) carries an empty `[Unreleased]` heading by intent.

## 3. FR traceability discipline

Every module that implements a functional requirement (FR-NN, defined in the PRD) carries a doc-comment header naming the FR it implements. Concrete example using FR-12 (full-text search via SQLite FTS5):

```rust
//! Implements FR-12 (full-text search via SQLite FTS5).
```

**Live mapping.** The mapping from FR to module is reproducible at any time:

```sh
grep -r "Implements FR-" crates/ shell-ui/src/
```

**CI gate.** `tests/traceability.rs` at workspace root will (post-Story 2.x, once any FR-bearing module exists) parse the PRD's FR-NN enumeration and fail if any FR has no `Implements FR-NN` match in the codebase. The doc-comment is **not** aspirational documentation — when an FR-bearing story lands, the header is non-negotiable.

## 4. Fixture placement rule

**Default: co-located per crate.** Test fixtures live alongside the consuming crate, e.g., `crates/orgsidian-parser/tests/fixtures/anchor.org` (the Story 1.9 anchor fixture). One crate consumes → fixture is per-crate.

**Promotion to root `fixtures/`: only when ≥2 crates consume the same fixture.** The root `fixtures/` directory does not exist yet — the first promotion will create it. When promoting, add a short `README.md` inside the promoted folder naming the consumers so a future contributor can see why it's shared.

Solo fixtures stay per-crate; cross-crate fixtures only at root.

## 5. MSRV policy

**Toolchain pin.** Orgsidian uses **stable Rust**, pinned via [`rust-toolchain.toml`](./rust-toolchain.toml). The workspace does **not** declare a `rust-version` field in [`Cargo.toml`](./Cargo.toml) because Orgsidian is a binary application (not a library published to crates.io) — the `rust-toolchain.toml` channel pin is the operational MSRV.

**Update cadence.** The pinned channel is bumped when a stable feature the project adopts requires it. Updates land via a `chore` commit touching `rust-toolchain.toml` and must pass the CI matrix (macOS-arm64 + Ubuntu-LTS per-PR; nightly Windows + Arch full sweep, per Story 1.8).

**`orgsidian-plugin-api` divergence (v1.5+).** When `orgsidian-plugin-api` publishes to crates.io (post v1.5 per LD-33), that crate **will** carry a `rust-version` field — at that point it becomes a library and MSRV becomes a public contract. The workspace MSRV otherwise tracks `rust-toolchain.toml`.

## 6. Testing strategy

The system-level testing strategy is owned by [`_bmad-output/test-artifacts/test-design.md`](./_bmad-output/test-artifacts/test-design.md) (TEA workflow, 2026-05-19).

That document defines: the three-level round-trip oracle (L0 per-PR / L1 nightly / L2 Emacs oracle), the anchor-smoke layer (§6.1), per-story-type red-phase scaffold templates (§7.3), the risk-prioritized coverage plan v0.1 → v1.0, and the failure-mode catalog mapping (LD-41 + Story 1.11).

Per the architecture's [Cross-Cutting Concerns header](./_bmad-output/planning-artifacts/architecture.md), `test-design.md` is the binding strategy for every story's red-phase scaffold (Process Discipline rule A); architecture LD-32 / LD-37 / LD-41 / LD-43 / LD-44 / LD-45 are referenced by it, not superseded.

Story 1.11 implements the LD-41 failure-mode harness; Story 1.12 implements the perf-snapshot regression macro consumed across the epics — both reference `test-design.md` as the source spec.
