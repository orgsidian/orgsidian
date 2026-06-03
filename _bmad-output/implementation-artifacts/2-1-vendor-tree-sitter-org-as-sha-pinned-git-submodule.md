# Story 2.1: Vendor `tree-sitter-org` as SHA-pinned git submodule

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 17

## Story

As the **author / contributor**,
I want [`nvim-orgmode/tree-sitter-org`](https://github.com/nvim-orgmode/tree-sitter-org) vendored at [`crates/orgsidian-parser/grammar/`](crates/orgsidian-parser/grammar/) as a SHA-pinned `git submodule` (no auto-bumping, no cargo git-dep) — wired into [`crates/orgsidian-parser/Cargo.toml`](crates/orgsidian-parser/Cargo.toml) via a new `build.rs` that compiles the vendored `grammar/src/parser.c` + `grammar/src/scanner.c` through `cc` (so `cargo build -p orgsidian-parser` succeeds without invoking external tooling), exposed internally via a `pub(crate) fn language() -> tree_sitter::Language` binding for Story 2.2 to consume — with [`CONTRIBUTING.md`](CONTRIBUTING.md) carrying a new **§8 Parser ownership (LD-48)** section that names the parser-owner role and the SHA-review process for upgrades, and the CI `actions/checkout@v5` calls in [`.github/workflows/pr.yml`](.github/workflows/pr.yml) + [`.github/workflows/nightly.yml`](.github/workflows/nightly.yml) flipped from `submodules: false` to `submodules: recursive`,
So that the LD-48 maintenance contingency is in place from day-1 (forkable-on-stall posture, SHA-review discipline, parser-owner role formalized) and Stories 2.2 → 2.8 can build the semantic layer + serializer on a known-good grammar revision without surprise upstream bumps mid-epic.

**Traces:** LD-48, LD-3, FR-1.

## Acceptance Criteria

### AC1 — Add `nvim-orgmode/tree-sitter-org` as a SHA-pinned git submodule at `crates/orgsidian-parser/grammar/`.

- **Add the submodule**: `git submodule add https://github.com/nvim-orgmode/tree-sitter-org.git crates/orgsidian-parser/grammar`. This creates an entry in [`.gitmodules`](.gitmodules) (root-level) AND records the parent-repo's pinned commit-hash inside the index tree at `crates/orgsidian-parser/grammar`.
- **Pin to a specific SHA — DO NOT track a branch.** After `git submodule add`, immediately `cd crates/orgsidian-parser/grammar && git checkout <SHA> && cd - && git add crates/orgsidian-parser/grammar`. Cargo's parent-repo records the gitlink at exactly that commit; subsequent `git submodule update` checks out the recorded SHA — never `HEAD` of any branch.
- **Recommended SHA**: pin to the latest commit on `main` of `nvim-orgmode/tree-sitter-org` reviewed by the parser-owner at impl time. As of story-write (2026-06-03) the head of `main` is [`219c0b27fdb2c0aeb43841f23f03d6f54657f288`](https://github.com/nvim-orgmode/tree-sitter-org/commit/219c0b27fdb2c0aeb43841f23f03d6f54657f288) (2025-02-23, `docs: Add note about PRs and fork information`). Tagged release `v1.3.2` at `911fe2de0d334febe1c9e402aaeba19db3d39a28` is also acceptable. **Verify the chosen SHA still has `bindings/rust/build.rs` + `src/parser.c` + `src/scanner.c` + `grammar.js` files present** (cargo build via `cc` requires these) — `git ls-tree --name-only <SHA>` after checkout.
- **`.gitmodules` contents** (the standard 3-line block; no extra `branch =` key — branch-tracking defeats the SHA-pin discipline):
  ```ini
  [submodule "crates/orgsidian-parser/grammar"]
      path = crates/orgsidian-parser/grammar
      url = https://github.com/nvim-orgmode/tree-sitter-org.git
  ```
- **License sanity**: confirm the submodule HEAD carries the MIT license (per [LD-3](_bmad-output/planning-artifacts/architecture.md#L65) + [LD-48](_bmad-output/planning-artifacts/architecture.md#L1276)). `cat crates/orgsidian-parser/grammar/LICENSE` after checkout — must contain `MIT License`. The MIT-MIT compatibility with LD-1 is the precondition for this vendoring; if the license changed upstream, STOP and surface a decision-grade question.
- **DO NOT add `tree-sitter-org` as a Cargo git-dependency.** The vendoring path is `git submodule + cc::Build::new()` exclusively — per [LD-48](_bmad-output/planning-artifacts/architecture.md#L1278) (vendoring discipline) and [deny.toml `unknown-git = "deny"` at line 203](deny.toml#L203) (which would refuse a cargo git source anyway). The submodule is a filesystem-only artifact; cargo never resolves it.

### AC2 — Document the SHA pin in `.gitmodules` and the parser-owner role + SHA-review process in `CONTRIBUTING.md`.

- **NEW `CONTRIBUTING.md` section** `## 8. Parser ownership (LD-48)` appended after the existing §7 ("Testing strategy"). ~30-50 lines of markdown. Required content:
  1. **Role definition.** Names a **single role** — the **"parser owner"** — as the working-familiarity owner of the `tree-sitter-org` grammar source. v0.1 Alpha assignment: the **current lead maintainer** holds the role; the assignment is intentionally role-agnostic (no hard-coded name) to remain accurate as the team grows. LD-48 mandates the role exists at all times; the *identity* of the holder is tracked outside this doc (`MAINTAINERS.md` if/when it lands, or the GitHub org's maintainer list). Pre-team-growth posture: the solo lead maintainer fills the role by default; no separate ceremony needed until a 2nd parser-touching contributor appears.
  2. **SHA-pin discipline.** Explicit statement that the submodule is pinned to a SHA, not a branch; `git submodule update --remote` is **forbidden** as a workflow shortcut. Bumps land via PR.
  3. **SHA-review process for upgrades** (the LD-48 contingency mechanism):
     - Run `git -C crates/orgsidian-parser/grammar log <current-SHA>..origin/main --oneline` to enumerate upstream commits since the last pin.
     - Parser owner reviews every commit in the range for: grammar correctness regressions (any `grammar.js` change), test corpus changes (any `test/corpus/*.txt` change), scanner.c edits (external-token logic — high-risk surface), and node-type renames (Story 2.2's wrapper depends on these strings).
     - Open a `chore(parser): bump tree-sitter-org to <SHA>` PR. PR description includes: (a) the upstream commit-range diff link, (b) which node-type strings were added/renamed/removed, (c) whether the L0 round-trip subset (Story 2.6, not yet shipped — flag as "future" until live) still passes.
     - Bumps land only after parser-owner sign-off in the PR. No auto-bump (Dependabot/Renovate must NOT be configured for this submodule).
  4. **Fork-and-maintain dry run reminder.** Reference [LD-48](_bmad-output/planning-artifacts/architecture.md#L1280): v0.3 milestone reserves 2 weeks for a fork-and-maintain dry run; parser owner checks out upstream, builds from source, fixes a trivial issue, runs the full parser test corpus. This story only DOCUMENTS the cadence — the dry-run itself is a v0.3-milestone task, not Story 2.1 scope.
  5. **In-house fork trigger.** Reference [LD-48](_bmad-output/planning-artifacts/architecture.md#L1281): if at any v* milestone upstream has had no commits for >6 months, fork to `orgsidian-org/tree-sitter-org` and maintain in-house under MIT.
- **`.gitmodules` is the machine-readable pin record.** The recorded SHA in the parent-repo index (visible via `git ls-tree HEAD crates/orgsidian-parser/grammar`) is the authoritative pin. The CONTRIBUTING.md prose names the *human* process; the index records the *machine* state.
- **Cross-link from `CONTRIBUTING.md` §8 to [LD-48 in architecture.md](_bmad-output/planning-artifacts/architecture.md#L1276)** so a future contributor reading either doc lands on the other.

### AC3 — Wire `crates/orgsidian-parser` to compile the vendored grammar via `build.rs` + `cc`; expose internal binding for Story 2.2.

- **Add workspace dependencies** at root [`Cargo.toml`](Cargo.toml) `[workspace.dependencies]`:
  ```toml
  # Story 2.1 (LD-48): tree-sitter Rust host for the nvim-orgmode/tree-sitter-org
  # vendored grammar at crates/orgsidian-parser/grammar/. Latest stable 0.26.x at
  # impl time. MIT.
  tree-sitter = "0.26"
  # Story 2.1: C compiler driver for build.rs — compiles vendored parser.c +
  # scanner.c through the host's cc/cl/clang. MIT/Apache-2.0. Pre-allowed.
  cc = "1"
  ```
  Pin caret-1.x via `"0.26"` and `"1"` per [[feedback_version_policy]] (latest stable; lockfile records the precise resolved version). Pin `tree-sitter` to `0.26` rather than `>= 0.25, < 0.27` because the upstream `bindings/rust/lib.rs` uses `tree_sitter::Language` from the crate's stable surface; 0.26.x is the latest line as of story-write (verified `0.26.9` on crates.io).
- **`crates/orgsidian-parser/Cargo.toml` changes**:
  - Add `[build-dependencies]` table with `cc = { workspace = true }`.
  - Add `tree-sitter = { workspace = true }` to `[dependencies]` (alongside the existing `thiserror`).
  - Add `build = "build.rs"` to `[package]` (or leave default — `build.rs` at crate root is auto-detected by Cargo; explicit declaration is the safer convention).
  - Keep the existing `description`, `version.workspace = true`, etc.
- **NEW `crates/orgsidian-parser/build.rs`** — mirrors the upstream pattern at `nvim-orgmode/tree-sitter-org/bindings/rust/build.rs` but adapted to point at `grammar/src/` (relative to our crate root, not the submodule root). Verbatim shape:
  ```rust
  // Story 2.1 (LD-48): compile the vendored nvim-orgmode/tree-sitter-org
  // grammar (git submodule at grammar/) into orgsidian-parser via cc.
  // Mirrors the upstream bindings/rust/build.rs pattern but points at our
  // submodule-rooted paths.

  fn main() {
      let grammar_src = std::path::Path::new("grammar").join("src");

      // Anti-footgun: hard-fail with a parser-owner-readable message if the
      // submodule has not been initialized (fresh clone without
      // `git submodule update --init --recursive` — extremely common).
      let parser_c = grammar_src.join("parser.c");
      if !parser_c.exists() {
          panic!(
              "tree-sitter-org submodule not initialized. \
               Run `git submodule update --init --recursive` from the repo root. \
               (LD-48: grammar is a SHA-pinned submodule, not a cargo git-dep.) \
               Missing file: {}",
              parser_c.display()
          );
      }

      let mut c_config = cc::Build::new();
      c_config.include(&grammar_src);
      c_config
          .flag_if_supported("-Wno-unused-parameter")
          .flag_if_supported("-Wno-unused-but-set-variable")
          .flag_if_supported("-Wno-trigraphs");
      #[cfg(target_env = "msvc")]
      c_config.flag("-utf-8");

      c_config.file(&parser_c);
      println!("cargo:rerun-if-changed={}", parser_c.display());

      let scanner_c = grammar_src.join("scanner.c");
      if scanner_c.exists() {
          c_config.file(&scanner_c);
          println!("cargo:rerun-if-changed={}", scanner_c.display());
      }

      c_config.compile("tree_sitter_org");
  }
  ```
  **Note on `scanner.c` existence check**: the upstream pattern unconditionally adds `scanner.c`. Some tree-sitter grammars (those with no external tokens) omit it. The `nvim-orgmode/tree-sitter-org` head DOES ship a `scanner.c` (external-token logic for org-mode block delimiters), so the `if scanner_c.exists()` branch is defensive only. Leave the conditional in — it's free guard against a future grammar that drops external tokens.
- **NEW internal binding module** `crates/orgsidian-parser/src/grammar/mod.rs` (a sub-module of the existing crate root, not a new crate; mirrors the existing flat `src/lib.rs` style). Verbatim shape:
  ```rust
  //! Internal tree-sitter-org grammar binding (Story 2.1, FR-1, LD-48).
  //!
  //! Re-exports the `extern "C" fn tree_sitter_org()` symbol produced by the
  //! `build.rs` `cc` compile of `grammar/src/parser.c`. Story 2.2 consumes
  //! `language()` to wire `parse(&str) -> Tree`. Story 2.1 itself does NOT
  //! call `language()` — the binding is forward-compat only; the
  //! anti-placebo-green smoke at AC4 exercises the symbol-link path.

  unsafe extern "C" {
      fn tree_sitter_org() -> tree_sitter::Language;
  }

  /// Get the tree-sitter [`Language`][tree_sitter::Language] for the vendored
  /// `nvim-orgmode/tree-sitter-org` grammar. Internal; Story 2.2 promotes to
  /// `pub` if the public parse() wrapper needs to expose it (current
  /// expectation is that it does not — `parse()` consumes `language()`
  /// internally only).
  pub(crate) fn language() -> tree_sitter::Language {
      // SAFETY: `tree_sitter_org()` is a thread-safe FFI constructor produced
      // by `tree-sitter generate` (deterministic, no global mutable state);
      // upstream tree-sitter-* crates ship this pattern verbatim.
      unsafe { tree_sitter_org() }
  }
  ```
  Wire `mod grammar;` into [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs) at the top of the file (after the `//!` doc-header). DO NOT make `grammar` a `pub mod` — Story 2.1's scope is purely vendoring; the `pub` surface enters with Story 2.2.
- **DO NOT modify `parse()` in [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs)**. The Story 1.9 stub body must remain functionally identical (returns `Ok(ParseTree)` for non-empty input, `Err(ParseError::Empty)` otherwise) so the [`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) anchor-smoke keeps passing without changes. Story 2.2 is the story that replaces the stub body. The Story 1.9 doc-comment promise ("Story 2.2 wires the real tree-sitter-org grammar and replaces this body; the public signature `parse(&str) -> Result<ParseTree, ParseError>` is preserved across that replacement (anchor sentinel discipline)") is preserved literally — Story 2.1 vendors the *grammar*; Story 2.2 *consumes* it.
- **License verification** (LD-37 hygiene; mirror Story 1.18's protocol):
  - `tree-sitter@0.26.x` license = MIT — pre-allowed in [deny.toml allowlist](deny.toml#L78-L91).
  - `cc@1.x` license = MIT/Apache-2.0 — pre-allowed.
  - The `tree-sitter-org` grammar itself is a git submodule (filesystem-only artifact, NOT a cargo dep) — `cargo deny` never sees it; license verification is manual via `cat crates/orgsidian-parser/grammar/LICENSE` per AC1.
  - Run `cargo deny check licenses bans advisories` post-impl + `cargo audit`; confirm no NEW advisories surface and no new `[bans].skip` entry is needed. The `tree-sitter` 0.26.x dep tree pulls in `regex-syntax` and `streaming-iterator` (both MIT) — if either triggers a new duplicate-version, STOP and surface a decision-grade question (per [[feedback_batch_fixes_terse]]) before adding a `[bans].skip` row.

### AC4 — `cargo build -p orgsidian-parser` compiles the vendored grammar; existing anchor-smoke stays green; add one new smoke that exercises the FFI symbol link.

- **PRIMARY GATE**: `cargo build -p orgsidian-parser` on macOS-arm64 + Ubuntu-LTS succeeds with `--locked`. This is THE Story 2.1 acceptance signal — every other AC supports this gate.
- **EXISTING `tests/anchor.rs` (Story 1.9) MUST stay green** with no modifications. Run `cargo test -p orgsidian-parser --test anchor` post-impl — `parse_anchor_fixture_succeeds` must pass. This is the [anchor sentinel discipline](crates/orgsidian-parser/src/lib.rs#L4-L8) — the public `parse()` signature is preserved across the Story 2.1 vendoring + the (later) Story 2.2 body swap.
- **NEW `crates/orgsidian-parser/tests/grammar_link.rs`** — anti-placebo-green guard. Exercises the FFI symbol so the `cc` compile + link path is end-to-end-tested by `cargo test`, not just `cargo build`. Verbatim shape:
  ```rust
  //! Story 2.1 — grammar FFI link smoke (anti-placebo-green per LD-48 + Party
  //! Mode P2 anchor convention). Without this test, a regression that breaks
  //! the cc compile or the extern "C" symbol-link would still pass
  //! `cargo build -p orgsidian-parser` (compilation runs only if some module
  //! references the symbol; build.rs only emits the object file).
  //!
  //! Story 2.2 replaces this smoke with the real `parse()` body test against
  //! a tree-sitter `Tree`. The internal `grammar::language()` accessor is
  //! `pub(crate)` so this test reaches it via an integration-test
  //! shim — see `crates/orgsidian-parser/src/lib.rs` for the
  //! `#[cfg(test)] pub use grammar::language as _language_for_smoke;`
  //! re-export.

  #[test]
  fn grammar_language_symbol_links() {
      // Resolve the FFI symbol via the test-only re-export. The mere
      // resolution proves: (a) build.rs ran, (b) cc compiled parser.c +
      // scanner.c, (c) the extern "C" symbol is reachable from Rust.
      let language = orgsidian_parser::_language_for_smoke();
      // tree-sitter Language exposes `version()` — calling it confirms the
      // returned pointer is a real Language struct, not a null/garbage value.
      assert!(language.version() > 0, "tree-sitter-org Language must have positive version");
  }
  ```
  Add the `#[cfg(test)] pub use grammar::language as _language_for_smoke;` re-export at the bottom of `crates/orgsidian-parser/src/lib.rs`. The `_` prefix marks it as test-only-internal; the `#[cfg(test)]` gate keeps it out of production builds. Story 2.2 deletes this re-export when `parse()` consumes `language()` directly.
- **CI matrix gate**: the per-PR pipeline ([pr.yml](.github/workflows/pr.yml)) already runs `cargo build --workspace --locked` + `cargo test --workspace --locked` on macOS-arm64 + Ubuntu-LTS. After AC5 flips `submodules: recursive`, the existing CI invocations exercise Story 2.1's compile + link without any new workflow step. **Verify locally first**: `git submodule update --init --recursive && cargo build -p orgsidian-parser --locked && cargo test -p orgsidian-parser --locked` from a fresh clone simulates the CI path.

### AC5 — Flip CI `actions/checkout@v5` calls from `submodules: false` to `submodules: recursive`.

- **EDIT** [`.github/workflows/pr.yml`](.github/workflows/pr.yml):
  - **Line 40-45**: change `submodules: false` to `submodules: recursive` on the primary checkout step. Update the inline comment from `# Step 1 — checkout. submodules: false today; Story 2.1 flips to # \`recursive\` when tree-sitter-org lands as a SHA-pinned submodule.` to `# Step 1 — checkout. submodules: recursive — required for the # tree-sitter-org grammar at crates/orgsidian-parser/grammar/ (LD-48).`.
  - **Line 230**: the `merge-gate-nightly-fresh` job's base-ref checkout (used only to read `nightly.yml` from the PR base) does NOT need submodules. Leave at `submodules: false` — no submodule-touching code runs in that job. Update the inline comment to clarify: `# submodules: false intentional — this job only reads .github/workflows/nightly.yml from base ref.`
- **EDIT** [`.github/workflows/nightly.yml`](.github/workflows/nightly.yml):
  - **Line 54**: change `submodules: false` to `submodules: recursive` on the `hosted` job's primary checkout. Required because nightly runs `cargo test --workspace` which compiles orgsidian-parser.
  - **Line 196**: review context — if this is also a `cargo test`/`cargo build` step's checkout, flip to `recursive`; if it's a non-Rust auxiliary (e.g. doc-link checker), leave as `false`. Story dev reads the surrounding job definition and decides.
- **NO new CI workflow file required.** The submodule-aware checkout is the only CI surface Story 2.1 touches. Caching: `Swatinem/rust-cache@v2` (already wired in pr.yml step 2) caches the cargo registry + target dir. The first run after this story merges pays the full `cc` compile of `parser.c` + `scanner.c` (one-time, ~5-15s); subsequent runs hit the rust-cache and compile-time is amortized to zero for unchanged grammar SHAs.
- **`merge-gate-nightly-fresh` window**: per its existing soft-pass logic, if the PR introducing Story 2.1 hasn't yet propagated nightly.yml to main, the gate soft-passes. No special handling required for the Story 2.1 PR itself.

### AC6 — Update `.gitignore` (if needed) + `architecture.md` cross-link audit.

- **`.gitignore` review** (defensive): the standard gitignore does NOT need to exclude `crates/orgsidian-parser/grammar/` — that path IS the submodule's working tree, and git knows it's a submodule via `.gitmodules`. **Verify** by running `git status` post-`git submodule add` — the submodule should appear as a *committed gitlink*, not as untracked files. If `git status` shows `crates/orgsidian-parser/grammar/` as untracked, the submodule add failed and the dev must redo it.
- **DO NOT add** `crates/orgsidian-parser/grammar/target/` or similar to `.gitignore` — the submodule is a SOURCE-ONLY checkout (no build products go into it). The `cc` compile output lands under `target/` at the workspace root, where the existing top-level `.gitignore` already covers it.
- **Architecture cross-link audit** (low-touch — Story 1.18's "stale row" lesson at [1-18 dev notes](_bmad-output/implementation-artifacts/1-18-toml-settings-authoritative-store-with-hybrid-boundary.md#L294)): grep `_bmad-output/planning-artifacts/architecture.md` for `tree-sitter-org` mentions and verify none contradict LD-48 vendoring. Known matches at [lines 65, 131, 185, 209, 913, 1276-1281, 1308 (force), 1440, 1478, 1533, 1537](_bmad-output/planning-artifacts/architecture.md) — all align with LD-3 + LD-48. No edits required in Story 2.1 scope. If a divergence surfaces, record in the Project Structure Notes section of Completion Notes; do NOT modify architecture.md (it's archival per [architecture.md:1010](_bmad-output/planning-artifacts/architecture.md#L1010)).

### AC7 — Traceability + documentation hygiene.

- **`//! Implements FR-1` doc-comment** on `crates/orgsidian-parser/src/grammar/mod.rs` — joins the existing FR-1 doc-comment on [`crates/orgsidian-parser/src/lib.rs:1`](crates/orgsidian-parser/src/lib.rs#L1) per [FR Traceability Discipline at CONTRIBUTING.md §4 line 113](CONTRIBUTING.md#L113). Story 2.1 prose already includes `LD-48` references; the FR-1 anchor is what the live FR-mapping grep at CONTRIBUTING.md §4 expects.
- **No new `grep-smoke` test required.** The Story 1.17 / 1.18 grep-smoke pattern is for module families with 6+ source files asserting a shared traceability marker. Story 2.1 adds 1 new `.rs` file (`grammar/mod.rs`) — no smoke is warranted; the existing FR-mapping pipeline catches drift.
- **Deferred-work stanza**: at end of dev work, append a `## Deferred from: code review of story-2.1 (YYYY-MM-DD)` stanza to [`_bmad-output/implementation-artifacts/deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md). Pre-seeded with at least:
  - **`grammar::language()` is `pub(crate)`, not `pub`** — Story 2.2 promotes to `pub` when `parse()` body consumes it. The test-only `_language_for_smoke` re-export is the v0.1 substitute.
  - **Submodule init in CI matrix bootstrap docs** — README.md's "First build" / `cargo build --workspace` quickstart should mention `git submodule update --init --recursive`. Out of Story 2.1 scope (docs-sweep story owns the README touch).

## Tasks / Subtasks

- [x] **T1** — Add submodule: `git submodule add https://github.com/nvim-orgmode/tree-sitter-org.git crates/orgsidian-parser/grammar`, then `cd crates/orgsidian-parser/grammar && git checkout <SHA> && cd -`. Verify `.gitmodules` contents + `git ls-tree HEAD crates/orgsidian-parser/grammar` records the pinned SHA. Verify `cat crates/orgsidian-parser/grammar/LICENSE` contains `MIT License`. Verify `grammar/src/parser.c` + `grammar/src/scanner.c` exist. (AC1)
- [x] **T2** — Add `tree-sitter = "0.26"` + `cc = "1"` to root [`Cargo.toml`](Cargo.toml) `[workspace.dependencies]` with the Story-2.1 inline-comment header per [[feedback_version_policy]]. (AC3)
- [x] **T3** — Edit [`crates/orgsidian-parser/Cargo.toml`](crates/orgsidian-parser/Cargo.toml): add `build = "build.rs"` to `[package]`; add `tree-sitter = { workspace = true }` to `[dependencies]`; add `[build-dependencies]\ncc = { workspace = true }`. Remove the Story-1.9 inline `# tree-sitter / tree-sitter-org land in Story 2.1+ ...` comment (it's now resolved). (AC3)
- [x] **T4** — Create [`crates/orgsidian-parser/build.rs`](crates/orgsidian-parser/build.rs) per AC3 verbatim shape. (AC3)
- [x] **T5** — Create [`crates/orgsidian-parser/src/grammar/mod.rs`](crates/orgsidian-parser/src/grammar/mod.rs) per AC3 verbatim shape; wire `mod grammar;` into [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs) (top, after `//!` header). Add the `#[cfg(test)] pub use grammar::language as _language_for_smoke;` re-export at the bottom of `lib.rs`. (AC3, AC4, AC7)
- [x] **T6** — Run `cargo build -p orgsidian-parser --locked` on macOS-arm64. Must succeed. If `cc` fails on `parser.c` (e.g., unsupported flag, missing header), STOP and surface a decision-grade question — do NOT silence with `-Wno-error=*` workarounds. (AC3, AC4)
- [x] **T7** — Run `cargo test -p orgsidian-parser --test anchor --locked`. Existing Story 1.9 anchor-smoke must stay GREEN with zero changes to `parse()` or `tests/anchor.rs`. (AC4)
- [x] **T8** — Create [`crates/orgsidian-parser/tests/grammar_link.rs`](crates/orgsidian-parser/tests/grammar_link.rs) per AC4 verbatim shape. Run `cargo test -p orgsidian-parser --test grammar_link --locked` — `grammar_language_symbol_links` must pass. (AC4)
- [x] **T9** — Edit [`.github/workflows/pr.yml`](.github/workflows/pr.yml) lines 40-45: flip `submodules: false` → `submodules: recursive`, update inline comment. Leave the line-230 `merge-gate-nightly-fresh` checkout at `false` (with clarifying comment). (AC5)
- [x] **T10** — Edit [`.github/workflows/nightly.yml`](.github/workflows/nightly.yml) line 54: flip `submodules: false` → `submodules: recursive`. Review line 196 — flip if it precedes a `cargo` step, leave alone otherwise. (AC5)
- [x] **T11** — Append §8 "Parser ownership (LD-48)" to [`CONTRIBUTING.md`](CONTRIBUTING.md) per AC2 outline (~30-50 lines markdown). Cross-link to LD-48 in architecture.md. (AC2)
- [x] **T12** — Run `cargo deny check licenses bans advisories` + `cargo audit`. Confirm no NEW advisory + no new `[bans].skip` row needed. If new advisory/duplicate surfaces, STOP and surface a decision-grade question. (AC3 license verification)
- [x] **T13** — Run `cargo test --workspace --locked` on macOS-arm64. All tests must remain GREEN (including settings round-trip from Story 1.18, perf-canary smokes, etc.). Report total test-count delta vs main in Completion Notes (expected: +1 test from grammar_link). (AC4)
- [x] **T14** — Append the deferred-work stanza per AC7. (AC7)
- [x] **T15** — Commit + open PR. Commit title: `feat(parser): vendor tree-sitter-org as SHA-pinned submodule (Story 2.1, closes #17)` — mirrors the [CONTRIBUTING.md §2 example at line 53](CONTRIBUTING.md#L53) exactly. NO `Co-Authored-By` trailer, NO "Generated with Claude Code" footer per [[feedback_no_co_author_credit]]. PR description includes: (a) the pinned SHA + reasoning, (b) `cargo build -p orgsidian-parser` timing on cold cache (for the rust-cache budget), (c) confirmation that anchor-smoke + grammar_link both pass. (AC4)

## Review Findings

(empty — populated on code-review)

## Dev Notes

### Critical context the dev agent must internalize

1. **Scope-fence: this is a vendoring story, NOT a parser-implementation story.** Story 2.1 lands the *submodule* + *build wiring* only. The `parse()` body in [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs) stays at the Story 1.9 stub. Story 2.2 is the wrapper-implementation story. If a sub-task seems to require calling `tree_sitter::Parser::new().set_language(&language())…` from `parse()`, STOP — that's Story 2.2's surface. The anchor-smoke discipline ([`src/lib.rs:4-8`](crates/orgsidian-parser/src/lib.rs#L4-L8)) explicitly promises the stub body is preserved across vendoring.

2. **Grammar submodule pin: SHA, never branch.** `git submodule add` records the *commit-hash* of whatever the submodule HEAD points to at add-time. The `.gitmodules` block intentionally omits `branch = main` — including it would tell `git submodule update --remote` to fast-forward to upstream HEAD on every developer machine, defeating LD-48 review discipline. The recorded gitlink in the parent-repo index IS the authoritative pin; the `.gitmodules` URL field is just for `init` to know where to clone from.

3. **`build.rs` runs once per cargo invocation per crate per profile** — anti-footgun panic at the top guards the common "fresh clone forgot `git submodule update --init`" failure mode. Without the guard, the dev sees a cryptic `cargo:rerun-if-changed=grammar/src/parser.c (No such file or directory)` rather than the actionable parser-owner-readable message in the AC3 verbatim shape.

4. **`tree-sitter` crate 0.26 vs older versions.** Upstream `nvim-orgmode/tree-sitter-org/Cargo.toml` declares `tree-sitter = ">= 0.19, < 0.21"` (stale; the binding file itself works with later 0.x). The compiled C symbol `tree_sitter_org()` returns a `tree_sitter::Language`-shaped pointer regardless of host crate version — the binding is ABI-stable across the 0.19 → 0.26 line. Story 2.1 pins `0.26` for forward-compat (latest stable, prep for Story 2.2's `Parser::set_language` call); this works because we *do not consume* the upstream `bindings/rust/lib.rs` — we write our own minimal `grammar/mod.rs` that exposes `extern "C" fn tree_sitter_org() -> Language;` directly. The upstream `bindings/rust/` directory in the submodule is unused by our build (we don't include it as a path dep).

5. **`pub(crate)` on `language()` is deliberate, not a placeholder.** Story 2.2 either (a) leaves it `pub(crate)` if `parse()` is the only consumer and exposes `tree_sitter::Tree` only through the `ParseTree` newtype, or (b) promotes to `pub` if Story 2.2's API surface needs raw Language access. Story 2.1 chooses (a) by default — keeps the public API minimal. The `_language_for_smoke` test re-export is the AC4 anti-placebo-green compromise: lets `grammar_link.rs` exercise the FFI symbol without polluting the public API.

6. **CI rust-cache budget.** The `cc` compile of `parser.c` (typically 100-300 KLOC of tree-sitter generated code) + `scanner.c` takes ~5-15s on a cold cache. [`Swatinem/rust-cache@v2`](https://github.com/Swatinem/rust-cache) caches the resulting object file under the `target/` tree, so subsequent runs with the SAME submodule SHA hit the cache (zero rebuild). If the submodule SHA changes, the cache invalidates and the cost is paid once. Story 2.1's PR description should include the measured cold-cache `cargo build -p orgsidian-parser` time on macos-14 + ubuntu-24.04 for the [perf-canary baseline](_bmad-output/planning-artifacts/architecture.md#L1452) (Story 1.12).

7. **`actions/checkout@v5` `submodules: recursive` behavior.** Flipping to `recursive` makes the checkout step run `git submodule update --init --recursive` after the initial clone. On the GitHub-hosted runners this adds ~2-5s of network I/O (small repo, single submodule). The runner's `GITHUB_TOKEN` has read access to public repos by default — no PAT required since `nvim-orgmode/tree-sitter-org` is public. If the submodule were ever moved to a private repo (LD-48 in-house fork trigger fires), the workflow would need `token: ${{ secrets.SUBMODULE_PAT }}` — leave a forward-compat comment near the `submodules: recursive` line so future-you finds the right knob.

8. **`merge-gate-nightly-fresh` checkout stays at `submodules: false`.** Per `pr.yml:225-230`, that job only `cat`s `nightly.yml` from the PR's base ref. It runs no `cargo` step. Flipping it to `recursive` would add ~3s to every PR for no benefit and slightly increases the failure surface (transient git fetch failure during a metadata-only job). The clarifying comment in T9 is the discoverability fix.

9. **Architecture date mismatch on tree-sitter-org "active fork" claim.** [Architecture LD-3 at line 65](_bmad-output/planning-artifacts/architecture.md#L65) says `nvim-orgmode/tree-sitter-org` had its "last push 2026-05-05". As of story-write (2026-06-03), the GitHub API confirms `pushed_at: 2026-05-05` on the repo metadata, but `commits/main` HEAD is at `219c0b27fd` dated 2025-02-23 — the `pushed_at` reflects tag/branch activity (force-push, tag pushes), not new code commits. Code-wise the upstream has been quiet for ~15 months. This is NOT a Story 2.1 blocker (LD-48's `>6 months no commits` trigger is about milestone-time review, not story-time), but DO flag in Completion Notes so the v0.3 fork-and-maintain dry run picks up the early-warning signal. The parser-owner section of CONTRIBUTING.md should reflect the actual state, not the architecture's optimistic prose.

10. **`tree-sitter` crate 0.26 transitive deps — verify no new `[bans].skip` row needed.** `tree-sitter 0.26` pulls in `regex-syntax 0.8` and `streaming-iterator 0.1` (per crates.io dep tree at story-write time). Both are MIT, single-versioned in current Cargo.lock = no duplicate-version conflict. If `cargo deny check bans` flags a NEW row (e.g., a `regex-syntax 0.8` vs an existing `regex-syntax 0.7` from the Tauri chain), STOP and surface a decision-grade question per [[feedback_batch_fixes_terse]] before editing `deny.toml`. Adding `[bans].skip` entries is a policy decision (LD-37).

11. **The submodule's `bindings/rust/build.rs` is NOT consumed by our build.** Our `crates/orgsidian-parser/build.rs` is a STANDALONE Rust build script that uses `cc` directly against `grammar/src/`. The submodule's own `bindings/rust/build.rs` is for *grammar consumers who depend on tree-sitter-org as a Cargo path-dep*. We deliberately don't take that path because (a) Cargo would treat the submodule as a workspace member candidate (LD-5 LEAF graph rule violation), (b) our build.rs is ~30 lines of obvious code and is easier to maintain than depending on an upstream file that could change shape. Document this choice in build.rs's header comment.

12. **Story 1.18 lessons applicable to Story 2.1**:
    - Issue-number metadata: `github_issue: 17` is the correct number for Story 2.1 ([gh issue list confirmation at story-write time](https://github.com/orgsidian/orgsidian/issues/17)). Story 1.17's metadata-fix lesson (collision because issues-sync hadn't yet run on the branch) doesn't apply here — #17 is already assigned and visible upstream.
    - Conventional Commits scope `parser` is the correct scope per [CONTRIBUTING.md §2 line 53](CONTRIBUTING.md#L53) (canonical scopes table).
    - Deferred-work stanza format: see [`_bmad-output/implementation-artifacts/deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) for the `## Deferred from: code review of story-1.NN (YYYY-MM-DD)` shape; pre-seed the items in T14.

### Project Structure Notes

**Alignment with unified project structure**:
- `crates/orgsidian-parser/grammar/` — NEW submodule path, matches the [LD-48 line 1278 specification](_bmad-output/planning-artifacts/architecture.md#L1278) verbatim ✓
- `crates/orgsidian-parser/build.rs` — NEW; matches the Cargo convention (crate-root `build.rs` is auto-detected). No precedent in this workspace yet — Story 2.1 is the first crate to use a build script. ✓
- `crates/orgsidian-parser/src/grammar/mod.rs` — NEW sub-module; matches the [`crates/orgsidian-core/src/settings/` pattern from Story 1.18](crates/orgsidian-core/src/settings/) (sub-module as a directory with `mod.rs`) — single-file `grammar.rs` would also work but using the directory shape leaves room for Story 2.2 to add sibling files (e.g., `grammar/queries.rs` for tree-sitter queries) without re-structuring. ✓
- Workspace deps additions (`tree-sitter`, `cc`) follow the Story 1.18 inline-comment convention at [Cargo.toml lines 47-78](Cargo.toml#L47-L78) ✓

**Detected conflicts or variances** (with rationale):
- **Epic AC wording "via `tree-sitter-cli` build hooks" is imprecise.** [epics.md:761](_bmad-output/planning-artifacts/epics.md#L761) literally says "cargo build -p orgsidian-parser compiles the vendored grammar via tree-sitter-cli build hooks." The actual mechanism is `cc::Build::new()` invoked from a Rust `build.rs` — no `tree-sitter-cli` binary executes at build time. The `tree-sitter-cli` is needed only to *regenerate* `parser.c` from `grammar.js` (the upstream maintainers' workflow), which Orgsidian's build never does. Story 2.1 implements per the *intent* of the AC (vendored grammar compiles at `cargo build` time) rather than the *literal text*. Document this variance in Completion Notes. Architecture/epics rewording is out of scope (epic file is sync-source for GitHub issues; mid-epic rewording risks churn).
- **`pub` vs `pub(crate)` on `grammar::language()`.** Epic AC at [epics.md:772](_bmad-output/planning-artifacts/epics.md#L772) (Story 2.2) says `pub fn parse(source: &str) -> tree_sitter::Tree` — implying the public API exposes a `Tree`. The internal `language()` accessor is an implementation detail of the future `parse()` body; Story 2.1 keeps it `pub(crate)` (rationale at Dev Note 5). Story 2.2 may promote if its design requires it — that's a Story 2.2 decision, not a Story 2.1 surface.
- **`scanner.c` conditional inclusion** — `nvim-orgmode/tree-sitter-org` HEAD ships a `scanner.c`. The `if scanner_c.exists()` check in `build.rs` (Dev Note: defensive — upstream could drop external tokens in a future bump) is paranoid. If parser-owner judgment says "this is over-cautious," safe to remove the conditional (defaulting to unconditional `.file(&scanner_c)`). Recorded as a variance only — Story 2.1 keeps the defensive shape per AC3.

### Testing Standards Summary

- **Build test (Cargo)**: `cargo build -p orgsidian-parser --locked` — primary AC4 gate. Runs on macOS-arm64 + Ubuntu-LTS per PR; Windows-2022 nightly. ~5-15s cold cache, <1s warm.
- **Integration tests (Cargo)**: under `crates/orgsidian-parser/tests/*.rs`. Auto-discovered. Story 2.1 adds `tests/grammar_link.rs` (1 test); existing `tests/anchor.rs` stays untouched (1 test, Story 1.9). Total parser-crate test count post-Story-2.1: 2.
- **Test runtime budget**: `cargo test -p orgsidian-parser --locked` should stay <5s wall-clock on warm cache (current Story 1.9 baseline <1s; Story 2.1 adds ~0.5s for the FFI link test).
- **CI matrix**: macOS-arm64 + Ubuntu-LTS per PR via [pr.yml](.github/workflows/pr.yml); macOS + Ubuntu + Windows + Arch nightly. The `submodules: recursive` flip in AC5 is the only CI-config change; no new workflow step.
- **No unit tests required for `build.rs`.** Build scripts are exercised by every `cargo build` — the AC4 gate IS the test.

### References

- Source story: [`epics.md:748-761`](_bmad-output/planning-artifacts/epics.md#L748-L761) — Story 2.1 user-story + AC + Traces.
- Architecture (canonical LD-48): [`architecture.md:1276-1281`](_bmad-output/planning-artifacts/architecture.md#L1276-L1281) — vendoring discipline + parser-owner role + fork triggers.
- Architecture (canonical LD-3): [`architecture.md:65`](_bmad-output/planning-artifacts/architecture.md#L65) — parser selection rationale + license-compatibility chain.
- Architecture (parser crate placement): [`architecture.md:913, 1017-1025`](_bmad-output/planning-artifacts/architecture.md#L913) — orgsidian-parser is a LEAF crate; only orgsidian-core consumes it (deny.toml `[[bans.deny]]` rule at [deny.toml line 175](deny.toml#L175)).
- Architecture (stack-versions): [`architecture.md:185`](_bmad-output/planning-artifacts/architecture.md#L185) — tree-sitter latest stable pin.
- PRD (FR-1 open/parse): [`prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) §4.1 — parser is the FR-1 owner.
- Previous story (Story 1.18 — TOML settings store): [`1-18-toml-settings-authoritative-store-with-hybrid-boundary.md`](_bmad-output/implementation-artifacts/1-18-toml-settings-authoritative-store-with-hybrid-boundary.md) — sub-module-as-directory pattern + inline-comment convention + license-verification protocol.
- Anchor-smoke discipline (Story 1.9): [`crates/orgsidian-parser/src/lib.rs:4-8`](crates/orgsidian-parser/src/lib.rs#L4-L8) — anti-placebo-green sentinel preserved across Story 2.1 vendoring + Story 2.2 body-swap.
- Existing anchor-smoke test: [`crates/orgsidian-parser/tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) — Story 2.1 must NOT modify.
- Upstream `nvim-orgmode/tree-sitter-org` repo: <https://github.com/nvim-orgmode/tree-sitter-org>
- Upstream `bindings/rust/build.rs` reference: <https://github.com/nvim-orgmode/tree-sitter-org/blob/main/bindings/rust/build.rs> (NOT consumed by our build; reference pattern only).
- `tree-sitter` Rust crate docs: <https://docs.rs/tree-sitter/latest/tree_sitter/>
- `cc` crate docs: <https://docs.rs/cc/latest/cc/>
- LEAF graph rule: [`deny.toml:174-176`](deny.toml#L174-L176) — `orgsidian-parser` may only be a direct dep of `orgsidian-core`.
- Conventional Commits parser-scope example: [`CONTRIBUTING.md:53`](CONTRIBUTING.md#L53) — verbatim `feat(parser): vendor tree-sitter-org as SHA-pinned submodule` as the recommended commit title.
- FR Traceability Discipline: [`CONTRIBUTING.md:109-124`](CONTRIBUTING.md#L109-L124).
- Cross-platform `cc` build (`-utf-8` on MSVC, `flag_if_supported` for GCC/Clang warning suppressions): upstream pattern at <https://github.com/nvim-orgmode/tree-sitter-org/blob/main/bindings/rust/build.rs>.

### Previous Story Intelligence (from Story 1.18)

Relevant to Story 2.1:

- **Submodule init in CI** ([Story 1.18 had no submodule touch; this is a new surface for Story 2.1]): the `actions/checkout@v5 with: submodules: recursive` flip is the only CI-config change. Story 1.18's "no new CI step required" pattern repeats here — the existing matrix invocations cover Story 2.1 once submodules are recursive-fetched.
- **Workspace-dep inline-comment convention** ([Story 1.18 Cargo.toml additions at lines 47-78](Cargo.toml#L47-L78)): every new `[workspace.dependencies]` entry gets a `# Story 2.1 (LD-48): <one-line rationale>` header. Story 2.1 follows verbatim.
- **License-verification protocol** ([Story 1.18 AC4 license verification block]): `cargo deny check licenses bans advisories` + `cargo audit` post-impl. No `deny.toml` modification unless an unexpected transitive surfaces. STOP and ask before adding a `[bans].skip` row.
- **Deferred-work stanza convention** ([Story 1.18 final commit at deferred-work.md L125-129](_bmad-output/implementation-artifacts/deferred-work.md#L125)): pre-seed the stanza at impl time so the code-review pass can append findings rather than re-derive context.
- **GitHub issues-sync metadata** ([Story 1.17 → 1.18 metadata-fix lesson]): `github_issue: 17` is correct here (verified `gh issue list` at story-write); Story 1.17's optimistic-overwrite collision does NOT apply because #17 was already claimed by Story 2.1 in the LD-55 sync.
- **Commit message convention**: `feat(parser): vendor tree-sitter-org as SHA-pinned submodule (Story 2.1, closes #17)` — Conventional Commits scope `parser`, no Co-Authored-By trailer, no AI-credit footer per [[feedback_no_co_author_credit]].

### Git Intelligence Summary

Recent commits relevant to Story 2.1 (`git log --oneline -10` at story-write):

- **`eea6341`** (Merge PR #137 — Story 1.18): TOML settings authoritative store. Demonstrates the sub-module-as-directory pattern + first cross-crate edge (`orgsidian-core` → `orgsidian-vault`). Story 2.1 reuses the inline-comment convention + license-verification protocol from this commit.
- **`a530a31`** (Merge PR #135 — Story 1.17): WCAG CI gate. Pure CI workflow changes. No relevance to parser internals.
- **`9e2d662`** (Story 1.16): added `tools/issues-sync` Cargo binary (outside workspace). Story 2.1's GitHub issue mapping (#17) is the live output of this sync.
- **No prior commit touches `crates/orgsidian-parser/`** beyond the Story 1.9 anchor-smoke (`tests/anchor.rs` + `src/lib.rs` stub). Story 2.1 is the first substantive `parser/` change.
- **No prior commit uses `git submodule`** anywhere in the workspace. Story 2.1 introduces the workspace's first submodule — `.gitmodules` is a brand-new top-level file.
- **No prior commit uses `cc` or a Cargo `build.rs`.** Story 2.1's `build.rs` is the workspace's first build script — sets the precedent for future stories (e.g., Story 3.x SQLite migrations, if any need build-time code-gen).

### Latest Technical Information

**Verify versions at implementation time** (per [[feedback_version_policy]] — latest stable; LTS preferred):

- **`tree-sitter` Rust crate**: latest stable verified `0.26.9` on crates.io at story-write (2026-06-03). Pin `tree-sitter = "0.26"` at workspace; lockfile resolves to the actual latest 0.26.x. API: `tree_sitter::Language` is the FFI handle type; `Parser::new()` + `Parser::set_language(&lang)` is the Story 2.2 consumer surface. License: MIT.
- **`cc` build-helper crate**: latest stable `1.x` (`1.2.x` verified at story-write). API: `cc::Build::new().include(path).file(path).compile(libname)`. Cross-platform — handles GCC/Clang on Unix, MSVC + MSYS2 on Windows. License: MIT/Apache-2.0.
- **`nvim-orgmode/tree-sitter-org` upstream**:
  - Default branch: `main` (verified via GitHub API at story-write).
  - Latest commit on `main`: `219c0b27fdb2c0aeb43841f23f03d6f54657f288` (2025-02-23, `docs: Add note about PRs and fork information`).
  - Latest tagged release: `v1.3.2` at `911fe2de0d334febe1c9e402aaeba19db3d39a28` (no `v1.3.3` / `v1.3.4` on the nvim-orgmode fork — those exist on the upstream `milisims` repo only, which is archived per [architecture.md LD-3 line 65](_bmad-output/planning-artifacts/architecture.md#L65)).
  - Repository license: MIT (verified via GitHub API).
  - Repo `pushed_at`: `2026-05-05T17:53:16Z` (tag/branch activity, not new code commits — see Dev Note 9).
  - Parser-owner SHA-review checklist target: pin to either the latest `main` SHA or `v1.3.2` tagged release. Default recommendation: latest `main` SHA (`219c0b27fdb2c0aeb43841f23f03d6f54657f288`). The dev's SHA choice is recorded in `.gitmodules` (via the parent index gitlink) at impl time.
- **GitHub Actions `actions/checkout@v5`**: `submodules: recursive` is the documented option that does `git submodule update --init --recursive` after the initial clone. No PAT required for public submodules. Pinned semver-major `@v5` per [[feedback_version_policy]] (matches existing pr.yml convention).

### Project Context Reference

The repository's project context lives across:
- [`_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) — PRD (§4.1 FR-1).
- [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md) — Architecture (LD-3 parser selection, LD-37 supply-chain hygiene, LD-44 round-trip subset criteria, LD-45 Emacs oracle, LD-48 vendoring + maintenance contingency, stack-versions table).
- [`_bmad-output/planning-artifacts/epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 spec (Stories 2.1 → 2.8).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §2 commit message scope `parser`, §4 FR traceability, §7 testing strategy (Story 2.1 appends §8 "Parser ownership (LD-48)").
- [[feedback_version_policy]] — latest-stable pin discipline.
- [[feedback_no_co_author_credit]] — commit/PR/issue hygiene.
- [[feedback_batch_fixes_terse]] — STOP-and-ask threshold for policy decisions.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7 (1M context) via bmad-dev-story workflow

### Debug Log References

- AC4 spec text `#[cfg(test)] pub use grammar::language as _language_for_smoke;` does NOT compile: (a) `cfg(test)` is not set when the lib is compiled for integration tests (they link against the non-test build), and (b) `pub use` of a `pub(crate)` item is forbidden by E0364. Resolved by pivoting to a `#[doc(hidden)] pub fn _language_for_smoke() -> tree_sitter::Language` shim that preserves the AC4 intent (FFI symbol exercised end-to-end at `cargo test` time, no stable public surface for the raw `Language`). Story 2.2 deletes the shim when `parse()` consumes `language()` directly. Same item logged in deferred-work.md.
- `tree_sitter::Language::version()` was renamed to `abi_version()` in 0.25+; the AC4 verbatim shape uses the old name. Updated `tests/grammar_link.rs` to call `abi_version()` against the pinned 0.26.x crate.

### Completion Notes List

- **Pinned SHA**: `219c0b27fdb2c0aeb43841f23f03d6f54657f288` (nvim-orgmode/tree-sitter-org, 2025-02-23 — head of `main` at story-write per the story's recommendation). License: MIT confirmed (`crates/orgsidian-parser/grammar/LICENSE`). Submodule is a gitlink (mode 160000), not a path-dep.
- **Cold-cache `cargo build -p orgsidian-parser --locked` (macOS-arm64, local)**: 5.15s. Includes `tree-sitter 0.26.x` + transitive deps compile + cc compile of `grammar/src/parser.c` (~470 KLOC generated) + `grammar/src/scanner.c`. Warm cache: ~0.2s.
- **Parser-crate test count delta**: anchor (1, Story 1.9, unchanged) + grammar_link (1, NEW Story 2.1) = 2 total. Workspace-wide `cargo test --locked` all green.
- **`cargo deny check licenses bans advisories`**: PASS. No new bans, no new license exceptions, no new advisories. `tree-sitter 0.26.x` transitive deps (`streaming-iterator`, `regex`, `regex-automata`, `regex-syntax`, `tree-sitter-language`) all single-versioned in Cargo.lock and license-compatible (MIT/Apache-2.0).
- **`cargo audit`**: PASS. The 18 pre-existing warnings (gtk-rs/Tauri chain unmaintained advisories) are unchanged; no Story-2.1-introduced advisory.
- **Anchor-smoke** (`tests/anchor.rs`, Story 1.9): UNCHANGED, still green. The `parse()` stub body is preserved verbatim per the Story 2.1 scope-fence.
- **CI flip**: `pr.yml` primary checkout + both `nightly.yml` checkouts now use `submodules: recursive`. The `merge-gate-nightly-fresh` checkout intentionally stays `false` (it reads only `nightly.yml` from base ref, no cargo work).
- **`scanner.c` upstream `-Wsign-compare` warnings (5 total)**: emitted by `cc` but not suppressed; upstream code, not in Story 2.1 scope to patch. Recorded in deferred-work.md as a LOW-priority upstream-PR follow-up.
- **Architecture/epic variance** flagged in Project Structure Notes: epics.md AC text says "via tree-sitter-cli build hooks" — actual mechanism is `cc::Build` from `build.rs`. Per the story's Dev Note 12 + the recurring "epics.md is sync-source for issues" rule, no architecture/epic edit was made. Recorded in deferred-work.md as a NIT.
- **Architecture LD-3 `pushed_at` vs commit-HEAD divergence**: confirmed upstream `pushed_at: 2026-05-05` masks code-quiet ~15 months. Flagged for v0.3 fork-and-maintain dry run (LD-48); not a Story 2.1 blocker.

### File List

**Added:**
- `.gitmodules` (NEW, root-level; first submodule in the workspace)
- `crates/orgsidian-parser/grammar` (NEW gitlink, mode 160000 → SHA `219c0b27fdb2c0aeb43841f23f03d6f54657f288`)
- `crates/orgsidian-parser/build.rs`
- `crates/orgsidian-parser/src/grammar/mod.rs`
- `crates/orgsidian-parser/tests/grammar_link.rs`

**Modified:**
- `Cargo.toml` (added `tree-sitter = "0.26"` + `cc = "1"` under `[workspace.dependencies]`)
- `Cargo.lock` (auto: tree-sitter chain resolution)
- `crates/orgsidian-parser/Cargo.toml` (added `build = "build.rs"`, `tree-sitter = { workspace = true }`, `[build-dependencies] cc = { workspace = true }`)
- `crates/orgsidian-parser/src/lib.rs` (added `mod grammar;` + `#[doc(hidden)] pub fn _language_for_smoke()` shim)
- `.github/workflows/pr.yml` (primary checkout flipped to `submodules: recursive`; merge-gate-nightly-fresh kept at `false` with clarifying comment)
- `.github/workflows/nightly.yml` (hosted + Arch container checkouts both flipped to `submodules: recursive`)
- `CONTRIBUTING.md` (appended §8 "Parser ownership (LD-48)")
- `_bmad-output/implementation-artifacts/deferred-work.md` (appended Story 2.1 stanza)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status flipped to in-progress → review; last_updated bumped)
- `_bmad-output/implementation-artifacts/2-1-vendor-tree-sitter-org-as-sha-pinned-git-submodule.md` (this file; Dev Agent Record, File List, Change Log, Status)

## Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-06-03 | bmad-create-story | Created story spec for Story 2.1 |
| 2026-06-03 | bmad-dev-story | Implemented Story 2.1 (T1–T15); SHA pinned 219c0b27, cold build 5.15s, all gates green |
