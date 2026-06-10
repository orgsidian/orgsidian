# Story 2.2: Implement `orgsidian-parser` grammar wrapper

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 18

## Story

As the **author / contributor**,
I want the vendored `tree-sitter-org` grammar (SHA-pinned at [`crates/orgsidian-parser/grammar/`](crates/orgsidian-parser/grammar/) by Story 2.1) wired into a real `parse()` body in [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs) — replacing the Story 1.9 stub with a `tree_sitter::Parser` that loads `grammar::language()` and produces a `tree_sitter::Tree` for any `.org` source, exposed through the existing `ParseTree` newtype (which now wraps the real `tree_sitter::Tree` instead of `_private: ()`) plus a `root_node()` accessor so callers can inspect the syntax tree — with the Story 2.1 `_language_for_smoke` shim deleted, the `tests/grammar_link.rs` FFI smoke replaced by a real `tests/grammar.rs` that asserts the parsed root node is `document` for a 10-line org sample, and the Story 1.9 `tests/anchor.rs` left byte-for-byte unchanged,
So that the semantic layer (Story 2.3) and the round-trip serializer (Story 2.4) can be built on top of a stable raw-syntax-tree API — and FR-1 ("open and parse `.org` files") has its first executable parser, not a stub.

**Traces:** FR-1, LD-3, LD-48.

## Scope Fence (read first)

This is the **grammar-wrapper** story: it turns raw `.org` text into a raw `tree_sitter::Tree`. It is **not**:

- **NOT the semantic layer (Story 2.3).** Do **not** introduce `Headline`, `TodoState`, `Tag`, `Timestamp`, `Drawer`, link-type variants, `#+TODO:` cycling, or any `src/semantic/` module. Those are Story 2.3's surface and depend on this story's `tree_sitter::Tree`. If a task seems to require interpreting node *meaning* (TODO state, drawer kind, timestamp parsing), STOP — that is Story 2.3.
- **NOT the serializer (Story 2.4).** No `serialize()`, no round-trip property, no `proptest`. Story 2.4.
- **NOT the CLI (Story 2.8).** No `orgsidian parse <file>` command. Story 2.8.
- **NOT corpus/fixtures (Story 2.5).** No `tools/corpus-extractor`, no `fixtures/subset-pr.json`. A single small in-repo sample fixture is all this story needs.

The deliverable is exactly: a working `parse(&str) -> Result<ParseTree, ParseError>` whose `ParseTree` carries a real `tree_sitter::Tree`, with a `root_node()` accessor, proven by a test that the root node kind is `document`.

## Acceptance Criteria

### AC1 — Replace the `parse()` stub body with a real tree-sitter-org-backed parser; preserve the `parse(&str) -> Result<ParseTree, ParseError>` signature (anchor-sentinel discipline).

**Given** Story 2.1 vendored the grammar and `grammar::language()` returns a working `tree_sitter::Language`,
**When** the `parse()` body in [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs) is rewritten,
**Then**:

- `parse(source: &str) -> Result<ParseTree, ParseError>` constructs a `tree_sitter::Parser::new()`, calls `parser.set_language(&grammar::language())`, then `parser.parse(source, None)`, and wraps the resulting `Tree` in `ParseTree`.
- The **public signature is preserved verbatim** — `parse(&str) -> Result<ParseTree, ParseError>` — per the anchor-sentinel discipline promised in the Story 1.9 / 2.1 module header at [`src/lib.rs:3-7`](crates/orgsidian-parser/src/lib.rs#L3-L7). This is **non-negotiable**: [`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) calls `parse(&source).expect(...)` and MUST keep compiling and passing with **zero edits to that file**.
- **Variance resolution (epic AC text says `pub fn parse(source: &str) -> tree_sitter::Tree`):** the epic prose at [epics.md:772](_bmad-output/planning-artifacts/epics.md#L772) loosely names `tree_sitter::Tree` as the return. The codebase has **already locked** the real shape via the anchor sentinel: `parse(&str) -> Result<ParseTree, ParseError>` with `ParseTree` as the public newtype. Implement per the *intent* (a callable that yields a syntax tree) while preserving the *committed signature*: `ParseTree` now **wraps** `tree_sitter::Tree` and exposes it via accessors (AC2). Do **not** change the return type to a bare `tree_sitter::Tree`. Record this as a variance in Completion Notes (mirrors the Story 2.1 "tree-sitter-cli build hooks" wording variance — epics.md is the GitHub-issues sync-source; do **not** edit it mid-epic).

### AC2 — `ParseTree` wraps the real `tree_sitter::Tree` and exposes it through accessors.

**Given** the parse body produces a `tree_sitter::Tree`,
**When** `ParseTree` is redefined,
**Then**:

- `ParseTree`'s `_private: ()` field (the Story 1.9 placeholder, [`src/lib.rs:13-18`](crates/orgsidian-parser/src/lib.rs#L13-L18)) is replaced with `tree: tree_sitter::Tree`.
- `ParseTree` exposes **at minimum** `pub fn root_node(&self) -> tree_sitter::Node<'_>` (returns `self.tree.root_node()`) so callers can inspect the tree without the field being public.
- `ParseTree` also exposes `pub fn tree(&self) -> &tree_sitter::Tree` so Story 2.3's semantic layer can walk / cursor / query the tree directly. Keep the inner field **private** — accessors only (the newtype is the stable API surface; the wrapped `Tree` is an implementation detail consumers reach through methods).
- `ParseTree` keeps `#[derive(Debug)]` if `tree_sitter::Tree: Debug` (it is, in 0.26.x — verify; if a derive issue arises, a hand-written `Debug` that prints `root_node().to_sexp()` or the root kind is acceptable, documented in Dev Notes).
- The `tree_sitter::Tree` type **must** become reachable from the public API. Add `pub use tree_sitter;` re-export **OR** keep the raw `Tree` out of the public signature by only exposing `Node`/sexp via accessors — choose the minimal surface. **Recommendation:** expose `pub fn tree(&self) -> &tree_sitter::Tree` + add `pub use tree_sitter;` at the crate root so downstream crates (Story 2.3 in `orgsidian-core`'s consumption path) name `orgsidian_parser::tree_sitter::Node` without taking their own direct `tree-sitter` dep. This keeps the version pin single-sourced through `orgsidian-parser`.

### AC3 — Parsing a representative org file returns a syntax tree without error; the root node type is asserted.

**Given** AC1 + AC2,
**When** a representative ≥10-line org sample (headlines, TODO states, a drawer, links) is parsed,
**Then**:

- `parse(sample)` returns `Ok(ParseTree)` (no `Err`) for the representative sample.
- The parsed tree's **root node kind is `"document"`** — this is the tree-sitter-org grammar's root rule (verified: `grammar.json` first rule = `document`; `node-types.json` contains `document`). Assert exactly `parse_tree.root_node().kind() == "document"`.
- The sample is committed as a fixture (see AC5) and is ≥10 lines covering: heading levels with TODO states, a `:PROPERTIES:` drawer, a `SCHEDULED:`/`DEADLINE:` line, inline markup, and at least one link. The parse must **not** require the tree to be error-free at the *node* level — tree-sitter-org may emit `ERROR`/`MISSING` nodes for constructs it doesn't model, and that is acceptable for the *wrapper* (the L0 round-trip gate of Story 2.6 is what enforces fidelity later). AC3 asserts only: parse returns `Ok` **and** root kind is `document`.

### AC4 — `ParseError` reflects the real failure surface; empty input behavior is decided and documented.

**Given** tree-sitter `parser.parse(...)` returns `Option<Tree>` and `set_language` returns `Result<(), LanguageError>`,
**When** `ParseError` is updated,
**Then**:

- The Story 1.9 `ParseError::Empty` variant is **removed**. Empty input (`""`) is **valid org** — it parses to a `document` node with zero children. `parse("")` returns `Ok`, not `Err`. This is an intentional behavior change from the Story 1.9 stub (which returned `Err(Empty)`); it is safe because (a) no workspace crate consumes `parse()` yet (verified: `orgsidian-core` has no `orgsidian-parser` dep; grep across `crates/ shell-ui/ tools/` finds zero `ParseError`/`ParseTree`/`orgsidian_parser` references outside the parser crate), and (b) no test asserts the `Empty` path ([`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) uses the non-empty `* TODO Hello\n` fixture only). Document the behavior change in Completion Notes.
- `ParseError` gains a variant for the genuine failure mode: `set_language` ABI failure and the theoretical `parser.parse()` → `None`. Recommended shape:
  ```rust
  #[derive(Debug, Error)]
  pub enum ParseError {
      /// The vendored grammar's ABI is incompatible with the host `tree-sitter`
      /// crate. In a correctly built crate this is unreachable (Story 2.1's
      /// `grammar_link` smoke proved ABI-compat at the pinned SHA); surfaced as
      /// an error rather than a panic to keep the parser panic-free.
      #[error("failed to load tree-sitter-org grammar: {0}")]
      Grammar(#[from] tree_sitter::LanguageError),
      /// `tree_sitter::Parser::parse` returned `None`. Only happens on a
      /// cancellation flag / timeout, neither of which this wrapper sets, so
      /// this is defensive — but mapping it keeps `parse()` total.
      #[error("tree-sitter returned no tree")]
      NoTree,
  }
  ```
  Keep `#[derive(Debug, Error)]` (`thiserror`, already a dep). `parse()` is **panic-free**: propagate `set_language` failure via `?`/`map_err` into `ParseError::Grammar`, map `None` into `ParseError::NoTree`. Do **not** `.expect()`/`.unwrap()` in the library body.

### AC5 — Add `tests/grammar.rs`; replace the Story 2.1 `tests/grammar_link.rs`; keep `tests/anchor.rs` untouched.

**Given** AC1–AC4,
**When** the test surface is updated,
**Then**:

- **NEW** [`crates/orgsidian-parser/tests/grammar.rs`](crates/orgsidian-parser/tests/grammar.rs) — the epic-named integration test. Asserts the tree has the expected root node type for a ≥10-line org sample. Verbatim intent:
  ```rust
  //! Story 2.2 — grammar wrapper test (FR-1). Replaces the Story 2.1
  //! `tests/grammar_link.rs` FFI-link smoke: the real `parse()` call now
  //! exercises the same cc-compile + extern-"C" symbol-link path end-to-end
  //! (set_language + parse), so the dedicated link smoke is subsumed.

  /// A ≥10-line representative org sample: heading levels with TODO states,
  /// a PROPERTIES drawer, SCHEDULED/DEADLINE, inline markup, and a link.
  const SAMPLE: &str = "\
  #+TITLE: Sample
  * TODO Top heading :work:
  SCHEDULED: <2026-06-10 Wed>
  :PROPERTIES:
  :ID: abc-123
  :END:
  Some *bold* and /italic/ text with a [[id:abc-123][link]].
  ** DONE Sub heading
  DEADLINE: <2026-06-12 Fri>
  - [ ] a checkbox item
  - [X] a done item
  ";

  #[test]
  fn parse_returns_document_root() {
      let tree = orgsidian_parser::parse(SAMPLE).expect("representative sample must parse");
      assert_eq!(
          tree.root_node().kind(),
          "document",
          "tree-sitter-org root node must be `document`"
      );
  }

  #[test]
  fn parse_empty_input_is_ok() {
      // Story 2.2 behavior change: empty `.org` is valid, parses to an empty
      // `document` (Story 1.9 stub returned Err(Empty); see AC4).
      let tree = orgsidian_parser::parse("").expect("empty source is a valid empty document");
      assert_eq!(tree.root_node().kind(), "document");
  }
  ```
  (Adjust the `SAMPLE` literal's leading-whitespace/`\` continuation as needed so the lines are column-0 org — the dev verifies the exact string parses; the *content* must hit the AC3 construct list. Prefer reading the sample from a committed fixture file if the inline literal gets unwieldy — see fixture note below.)
- **DELETE** [`crates/orgsidian-parser/tests/grammar_link.rs`](crates/orgsidian-parser/tests/grammar_link.rs). Story 2.1's [Dev Note + the file's own header](crates/orgsidian-parser/tests/grammar_link.rs#L8) explicitly state Story 2.2 replaces this smoke with the real `parse()` body test. The real `parse()` now calls `set_language(&language())` + `parse(...)`, exercising the identical FFI-link + ABI-compat path the smoke guarded — so deleting it loses no coverage. (If you prefer belt-and-suspenders, you may instead keep a single ABI assertion *inside* `tests/grammar.rs`, but the cleaner end-state is one file: `grammar.rs`.)
- **DO NOT TOUCH** [`crates/orgsidian-parser/tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) **or** [`crates/orgsidian-parser/tests/fixtures/anchor.org`](crates/orgsidian-parser/tests/fixtures/anchor.org). The anchor smoke is the cross-story sentinel — it must pass unchanged, proving `parse(&str) -> Result<…>` survived the body swap. Run `cargo test -p orgsidian-parser --test anchor --locked` and confirm green with no diff to the file.
- **Fixture (optional but recommended):** if the inline `SAMPLE` literal is awkward, add [`crates/orgsidian-parser/tests/fixtures/grammar-sample.org`](crates/orgsidian-parser/tests/fixtures/grammar-sample.org) (≥10 lines, same construct coverage) and read it via `env!("CARGO_MANIFEST_DIR")` like `anchor.rs` does. Co-located fixture per the [CONTRIBUTING fixture-placement rule](architecture.md#L1011) (promote to root `fixtures/` only when ≥2 crates consume — not here). Do **not** introduce the corpus machinery (Story 2.5).

### AC6 — Delete the Story 2.1 `_language_for_smoke` shim; `grammar::language()` stays `pub(crate)` and is consumed directly by `parse()`.

**Given** `parse()` now consumes `grammar::language()` internally,
**When** the Story 2.1 anti-placebo shim is retired,
**Then**:

- **DELETE** the `#[doc(hidden)] pub fn _language_for_smoke()` function at [`src/lib.rs:43-53`](crates/orgsidian-parser/src/lib.rs#L43-L53) **and its leading comment block**. Story 2.1's [deferred-work stanza](_bmad-output/implementation-artifacts/deferred-work.md#L134) + [code-review finding](_bmad-output/implementation-artifacts/deferred-work.md#L141) explicitly assign this deletion to Story 2.2 (it leaked `tree_sitter::Language` into the stable public API via an un-gated `pub fn`). Removing it is the intended cleanup, not a breaking-change concern (no consumer references it; the `_` prefix + `#[doc(hidden)]` marked it internal-only).
- **`grammar::language()` stays `pub(crate)`** (do **not** promote to `pub`). Story 2.1's design note + deferred item ([deferred-work.md:133](_bmad-output/implementation-artifacts/deferred-work.md#L133)) state the promotion happens "only if Story 2.2's design needs raw `Language` access." It does **not**: `parse()` consumes `language()` internally, and the public surface is `ParseTree` + accessors (AC2). Keep the FFI accessor crate-private. The `grammar/mod.rs` doc-comment that says "Story 2.2 consumes `language()`" is now accurate; the "promotes to `pub`" clause can be tightened to past tense ("Story 2.2 consumes `language()` internally; stays `pub(crate)`") — a one-line doc edit, optional.

### AC7 — Traceability + docs hygiene; module header rewritten to reflect the real implementation.

**Given** the stub is gone,
**When** doc-comments are updated,
**Then**:

- The crate-root `//!` header in [`src/lib.rs:1-7`](crates/orgsidian-parser/src/lib.rs#L1-L7) is rewritten: drop the "Story 1.9 ships the anchor-smoke surface only — `parse()` is a stub…" prose (now false) and replace with a description of the real wrapper. **Keep an `//! Implements FR-1` line** (per [FR Traceability Discipline, CONTRIBUTING §4](CONTRIBUTING.md#L109-L124) — the `grep -r "Implements FR-"` live-mapping + the future `tests/traceability.rs` gate depend on it). The header already names FR-1, FR-2; keep FR-1 explicit (FR-2/serializer is Story 2.4 — you may keep the "+ serializer (FR-2)" forward-reference in the one-line crate summary, but the `Implements FR-` marker this story is accountable for is **FR-1**). `grammar/mod.rs` already carries `Implements FR-1` — leave it.
- `cargo doc -p orgsidian-parser` succeeds and the module shows the `Implements FR-1` doc-comment (epic AC). Verify locally: `cargo doc -p orgsidian-parser --no-deps`.
- **No new `grep-smoke`/traceability test required** — the existing FR-mapping pipeline (CONTRIBUTING §4) covers it; this story adds no new FR-bearing module family.

### AC8 — Build, test, and supply-chain gates stay green.

**Given** all the above,
**When** the gates run,
**Then**:

- `cargo build -p orgsidian-parser --locked` succeeds on macOS-arm64 + Ubuntu-LTS (the existing per-PR matrix; submodule already `recursive` since Story 2.1 AC5).
- `cargo test -p orgsidian-parser --locked` passes: `anchor` (unchanged, 1 test) + `grammar` (NEW, 2 tests). `grammar_link` is gone. Report the parser-crate test-count delta in Completion Notes (expected net: was 2 → now 3, i.e. anchor 1 + grammar 2; grammar_link −1, grammar +2).
- `cargo test --workspace --locked` stays green (Story 1.18 settings round-trip, perf-canary smokes, etc. — no regressions).
- `cargo clippy --workspace --all-targets --locked` clean (no new warnings; the dead-code warning Story 2.1 carried on `language()` via the shim is now resolved differently — `language()` is reachable from `parse()`, so no `#[allow(dead_code)]` is needed; **remove** any such allow if one was added).
- `cargo fmt --check` clean.
- `cargo deny check licenses bans advisories` + `cargo audit`: **no new** exceptions/advisories. This story adds **no new dependencies** (`tree-sitter` + `cc` already wired by Story 2.1; `thiserror` already present). If anything new surfaces (it should not), STOP and surface a decision-grade question per [[feedback_batch_fixes_terse]] before editing `deny.toml`.

## Tasks / Subtasks

- [x] **T1** — Rewrite `ParseTree` in [`src/lib.rs`](crates/orgsidian-parser/src/lib.rs): replace `_private: ()` with `tree: tree_sitter::Tree` (private field); add `pub fn root_node(&self) -> tree_sitter::Node<'_>` + `pub fn tree(&self) -> &tree_sitter::Tree`; keep `#[derive(Debug)]` (verify `Tree: Debug` in 0.26.x; hand-write `Debug` only if the derive fails). (AC2)
- [x] **T2** — Update `ParseError`: remove `Empty`; add `Grammar(#[from] tree_sitter::LanguageError)` + `NoTree` per AC4 verbatim shape. (AC4)
- [x] **T3** — Rewrite the `parse()` body: `Parser::new()` → `set_language(&grammar::language())?` (map err → `ParseError::Grammar` via `#[from]`/`?`) → `parser.parse(source, None).ok_or(ParseError::NoTree)?` → `Ok(ParseTree { tree })`. Panic-free (no `unwrap`/`expect`). Preserve the `parse(&str) -> Result<ParseTree, ParseError>` signature exactly. (AC1, AC3, AC4)
- [x] **T4** — Add `pub use tree_sitter;` at the crate root (single-source the version pin for downstream Story 2.3 consumption). Rewrite the crate-root `//!` header: drop the stub prose, keep an `//! Implements FR-1` line. (AC2, AC7)
- [x] **T5** — DELETE the `#[doc(hidden)] pub fn _language_for_smoke()` shim + its comment block from `src/lib.rs`. Confirm `grammar::language()` stays `pub(crate)` and is now reached via `parse()` (no dead-code warning → remove any stale `#[allow(dead_code)]`). Optionally tighten the `grammar/mod.rs` doc-comment to past tense. (AC6)
- [x] **T6** — Create [`crates/orgsidian-parser/tests/grammar.rs`](crates/orgsidian-parser/tests/grammar.rs) per AC5: `parse_returns_document_root` (root kind == `"document"`) + `parse_empty_input_is_ok`. Use the ≥10-line construct-covering sample (inline literal OR a committed `tests/fixtures/grammar-sample.org` read via `CARGO_MANIFEST_DIR`). (AC3, AC5)
- [x] **T7** — DELETE [`crates/orgsidian-parser/tests/grammar_link.rs`](crates/orgsidian-parser/tests/grammar_link.rs). (AC5)
- [x] **T8** — Verify [`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) + [`tests/fixtures/anchor.org`](crates/orgsidian-parser/tests/fixtures/anchor.org) are byte-for-byte unchanged. Run `cargo test -p orgsidian-parser --test anchor --locked` — `parse_anchor_fixture_succeeds` GREEN. (AC1, AC5)
- [x] **T9** — Run `cargo build -p orgsidian-parser --locked` + `cargo test -p orgsidian-parser --locked`. Both green; `anchor` (1) + `grammar` (2) = 3 tests. (AC8)
- [x] **T10** — Run `cargo doc -p orgsidian-parser --no-deps` — succeeds, `Implements FR-1` visible. (AC7)
- [x] **T11** — Run `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked` + `cargo fmt --check`. All green/clean; report workspace test-count delta in Completion Notes. (AC8)
- [x] **T12** — Run `cargo deny check licenses bans advisories` + `cargo audit`. Confirm NO new dep / advisory / exception (none expected — zero new deps). STOP-and-ask if anything new surfaces. (AC8)
- [x] **T13** — Append a pre-seeded `## Deferred from: code review of story-2.2 (YYYY-MM-DD)` stanza to [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) (mirror the Story 2.1 format at [line 131](_bmad-output/implementation-artifacts/deferred-work.md#L131)). Pre-seed candidates: parser-per-call allocation (no pooled/thread-local `Parser` reuse — fine for the wrapper, revisit if Story 2.3+ profiling shows hot-path cost); `ERROR`/`MISSING` node tolerance is intentional at the wrapper layer (fidelity enforced by the Story 2.6 L0 gate, not here). (process hygiene)
- [x] **T14** — Commit + open PR. Commit title: `feat(parser): implement tree-sitter-org grammar wrapper (Story 2.2, closes #18)` — Conventional Commits scope `parser` per [CONTRIBUTING §2](CONTRIBUTING.md#L53). **NO** `Co-Authored-By` trailer, **NO** "Generated with Claude Code" footer per [[feedback_no_co_author_credit]]. PR body: (a) confirm anchor-smoke unchanged + green, (b) confirm root-node `document` assertion, (c) note the empty-input behavior change, (d) confirm zero new deps. (process)

### Review Findings (code review 2026-06-10)

- [x] [Review][Patch] Document the keep-the-source contract on `ParseTree`/`parse()` — node byte-ranges resolve only against the exact `source` passed to `parse()`, kept alive and byte-identical; a normalized/re-read copy yields garbage spans or out-of-bounds panics (resolved from [Review][Decision], 2026-06-10) [crates/orgsidian-parser/src/lib.rs:24-44] (blind+edge)
- [x] [Review][Defer] `ParseTree` source-retention design (owned `String` field vs `ParseTree<'a>` borrow vs caller-carried) — deferred, reason: zero consumers today and the anchor sentinel locks the `parse()` signature, not `ParseTree` internals; the owned-vs-borrowed choice belongs to Story 2.3's real consumer (Scope Fence: don't pre-design the semantic layer's surface). Doc-contract patch above defuses the foot-gun meanwhile. [crates/orgsidian-parser/src/lib.rs:24-44]
- [x] [Review][Patch] `parse_returns_document_root` asserts only the root kind — passes even if the grammar lexes the whole sample into a single `ERROR` child or stops consuming at byte 0; add `child_count() > 0`, `end_byte() == SAMPLE.len()`, and `!has_error()` (sample is well-formed) [crates/orgsidian-parser/tests/grammar.rs:22-30]
- [x] [Review][Patch] `parse_empty_input_is_ok` doesn't verify the documented "zero children" contract — add `assert_eq!(child_count(), 0)` [crates/orgsidian-parser/tests/grammar.rs:32-38]
- [x] [Review][Patch] Completion Notes workspace-test baseline off by one — "48 passed (was 46)" is internally inconsistent with the crate delta net +1; baseline on main is 47 [2-2-implement-orgsidian-parser-grammar-wrapper.md, Completion Notes]
- [x] [Review][Patch] `ParseTree` doc claims "opaque … implementation detail consumers reach through accessors only" while `tree()` hands back the raw `&Tree` and `pub use tree_sitter` re-exports the full crate — rewrite to state the real contract (the raw tree IS the API; tree-sitter semver is part of this crate's public surface) [crates/orgsidian-parser/src/lib.rs:24-26]
- [x] [Review][Patch] `NoTree` variant doc enumerates `parse() → None` causes incompletely — omits "no language set" (unreachable here, but a trap for future refactors that hoist parser construction) [crates/orgsidian-parser/src/lib.rs:54-56]
- [x] [Review][Patch] `Grammar` variant doc cites the `grammar_link` smoke as ABI-compat proof — that test is deleted in this same change; reword to the live `parse()` path [crates/orgsidian-parser/src/lib.rs:48-51]
- [x] [Review][Patch] `ParseError` Display/thiserror wiring never executed by any test — add a cheap assertion on `ParseError::NoTree` display output [crates/orgsidian-parser/tests/grammar.rs]
- [x] [Review][Patch] `ParseTree` thread-safety (auto-traits) undocumented — Dev Note 3 claims `Tree` is `Send + !Sync`; verify the actual auto-traits at tree-sitter 0.26.9 and document (or compile-assert) so a future bump changing them is caught deliberately [crates/orgsidian-parser/src/lib.rs:24-30]
- [x] [Review][Patch] `parse()` doc doesn't state tree-sitter's u32 byte-addressing limit — source > 4 GiB is silently truncated to a prefix tree (`Ok` with partial coverage); add a doc note [crates/orgsidian-parser/src/lib.rs:61-66]
- [x] [Review][Defer] Vendored `scanner.c` serializes list-indent state as signed `char` — valid org whose cumulative indent crosses 128 columns misparses into `ERROR` nodes (empirically verified: indent 127 ok, 128 errors; ≈64 nesting levels at 2-space steps). Upstream bug at the pinned SHA; LD-48 vendoring discipline forbids local edits — deferred, pre-existing (Story 2.1 vendoring) [crates/orgsidian-parser/grammar/src/scanner.c:75-101]
- [x] [Review][Defer] No incremental-reparse or cancellation path is reachable through the public surface — `parse(source, None)` hardcodes a full reparse and `grammar::language()` stays `pub(crate)`, so callers cannot build their own configured `Parser` (~1.4 s uncancelable block measured on a 10 MB file) — deferred, future scope (Story 2.3+ editor loop) [crates/orgsidian-parser/src/lib.rs:67-72]

## Dev Notes

### Critical context the dev agent must internalize

1. **Signature is locked by the anchor sentinel — do not "fix" it to match epic prose.** The epic AC literally writes `pub fn parse(source: &str) -> tree_sitter::Tree`. The codebase already committed to `parse(&str) -> Result<ParseTree, ParseError>` across Story 1.9 → 2.1, with the module header at [`src/lib.rs:3-7`](crates/orgsidian-parser/src/lib.rs#L3-L7) and [`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs) (which calls `.expect()` on the result) as the enforcement mechanism. If you change `parse` to return a bare `Tree`, `anchor.rs` stops compiling and you've broken the sentinel discipline. **Preserve the signature; wrap the `Tree` in `ParseTree`.** This is the same "epic prose vs real mechanism" variance Story 2.1 navigated (it documented the "tree-sitter-cli build hooks" wording divergence and did NOT edit epics.md, which is the GitHub-issues sync-source).

2. **This is a `Result`-returning wrapper, but tree-sitter parsing essentially never fails on valid UTF-8 with a language set.** `parser.parse(text, None)` returns `Option<Tree>` and only yields `None` on a cancellation flag / timeout (neither of which this wrapper sets). `set_language` returns `Result` and only fails on ABI mismatch — which Story 2.1's `grammar_link` smoke already proved impossible at the pinned SHA (`219c0b27…`, tree-sitter 0.26.9). So both `Err` arms are defensive. The `Result` exists to honor the preserved signature and to stay panic-free (a library parser should not `unwrap`). Org's grammar is permissive: malformed constructs produce `ERROR`/`MISSING` *nodes inside* a valid `document` tree, **not** a parse failure. AC3 therefore asserts only `Ok` + root kind `document`, not node-level error-freeness.

3. **`tree_sitter::Tree` is `!Sync` and the `Parser` is `Send + !Sync`.** *(Review correction 2026-06-10: wrong — at tree-sitter 0.26.9 `Tree`, `Parser`, and `Language` all carry `unsafe impl Send + Sync` in `binding_rust/lib.rs:3887-3909`. The guidance below still stands: parsing requires `&mut Parser`, so a fresh per-call `Parser` remains the correct simple choice.)* Creating a fresh `Parser` per `parse()` call is the correct, simple choice for the wrapper — no shared mutable state, no `Send`/`Sync` headaches. Do **not** introduce a global/static or pooled parser in this story. If Story 2.3+ profiling shows per-call `Parser` allocation is a hot-path cost (it won't be for typical file sizes), a thread-local pool is a later optimization — pre-seed it as a deferred item (T13), don't build it now.

4. **`grammar::language()` stays `pub(crate)`.** Story 2.1 deliberately kept it crate-private (option (a) in its Dev Note 5) and deferred promotion to "only if Story 2.2 needs raw `Language` access." It does not — `parse()` consumes it internally, and consumers get `ParseTree` + accessors. Promoting to `pub` would re-leak the raw FFI handle the `_language_for_smoke` cleanup is removing. Keep it private. (See [deferred-work.md:133](_bmad-output/implementation-artifacts/deferred-work.md#L133).)

5. **Deleting `_language_for_smoke` is the assigned cleanup, not collateral damage.** Story 2.1 introduced the `#[doc(hidden)] pub fn _language_for_smoke()` shim purely as an anti-placebo-green compromise to let `grammar_link.rs` exercise the FFI link without a real `parse()` body. Story 2.1's deferred-work + code-review both name Story 2.2 as the deleter ([deferred-work.md:134, 141](_bmad-output/implementation-artifacts/deferred-work.md#L134)). The real `parse()` body now exercises the identical `set_language` + `parse` FFI path, so the shim and its `grammar_link.rs` test are fully subsumed. Removing an un-gated `pub fn` is technically a breaking change in the abstract — but it's `#[doc(hidden)]`, `_`-prefixed, and referenced by exactly one (deleted) test; treat it as internal cleanup.

6. **`ParseTree` field stays private; expose via accessors.** The newtype is the stable API barrier (LD-5 crate-API-barrier philosophy). Story 2.3 will walk the tree via `parse_tree.tree()` / `root_node()` and build the semantic AST on top — it should not reach a `pub` field. Add `pub use tree_sitter;` so Story 2.3 (consumed through `orgsidian-core`) can name `orgsidian_parser::tree_sitter::Node` without its own `tree-sitter` dep, single-sourcing the version pin through this leaf crate (consistent with the LEAF graph rule at [deny.toml:174-176](deny.toml#L174-L176) — `orgsidian-parser` is the only parser-touching dep `orgsidian-core` takes).

7. **Empty-input behavior change is safe and intentional.** Story 1.9's stub returned `Err(ParseError::Empty)` for `""`. A real org parser must accept empty files (`""` → empty `document`). Removing `Empty` and returning `Ok` is correct. It's safe because no workspace crate consumes `parse()` yet (verified grep: zero `orgsidian_parser`/`ParseError`/`ParseTree` references outside the parser crate; `orgsidian-core` has no `orgsidian-parser` dependency) and no test asserts the `Empty` path. Document it in Completion Notes so the change is traceable.

8. **Zero new dependencies.** `tree-sitter = "0.26"` (resolved 0.26.9) + `cc` (build-dep) + `thiserror` are all already in [`crates/orgsidian-parser/Cargo.toml`](crates/orgsidian-parser/Cargo.toml). This story touches no `Cargo.toml` dependency lists and no root `[workspace.dependencies]`. `cargo deny`/`audit` should be a no-op delta. If a transitive surfaces, STOP per [[feedback_batch_fixes_terse]].

9. **tree-sitter 0.26.x API specifics** (verify at impl time per [[feedback_version_policy]]; confirmed against Story 2.1's working `grammar_link.rs` which already uses this surface):
   - `tree_sitter::Parser::new() -> Parser`
   - `parser.set_language(&Language) -> Result<(), LanguageError>` — takes `&Language` (0.25+ borrowed form; Story 2.1's smoke calls `set_language(&language)`).
   - `parser.parse(text: impl AsRef<[u8]>, old_tree: Option<&Tree>) -> Option<Tree>` — pass `None` for `old_tree` (no incremental reparse in this story).
   - `tree.root_node() -> Node` ; `node.kind() -> &str` ; `node.to_sexp() -> String` (handy for debugging).
   - `Language::abi_version()` (renamed from `version()` in 0.25+ — see Story 2.1 Debug Log). Not needed in `parse()` but relevant if you keep any ABI assertion.

### Project Structure Notes

**Alignment with unified project structure:**
- `crates/orgsidian-parser/src/lib.rs` — UPDATE (parse body + ParseTree + ParseError + header). Matches the [architecture crate role](architecture.md#L913): "tree-sitter-org wrapper + semantic AST + serializer (FR-1, FR-2)". This story delivers the *wrapper* third of that line. ✓
- `crates/orgsidian-parser/tests/grammar.rs` — NEW; the epic-named test ([epics.md:774](_bmad-output/planning-artifacts/epics.md#L774)). ✓
- `crates/orgsidian-parser/tests/grammar_link.rs` — DELETE (Story 2.1 transitional smoke, explicitly slated for replacement). ✓
- `crates/orgsidian-parser/grammar/` — submodule, READ-ONLY (LD-48 vendoring discipline: never edit vendored grammar sources locally). This story consumes it; it does not touch it. ✓

**Detected conflicts / variances (with rationale):**
- **Epic AC return-type `tree_sitter::Tree` vs committed `Result<ParseTree, ParseError>`** — resolved in AC1: preserve the sentinel signature, wrap the `Tree`. Record in Completion Notes; do NOT edit epics.md (sync-source; mid-epic rewording churns GitHub issues — same rule Story 2.1 followed for the "tree-sitter-cli build hooks" wording).
- **`grammar/mod.rs` doc-comment "Story 2.2 promotes to `pub`"** — now inaccurate (we keep `pub(crate)`). Optional one-line doc tightening (T5); not a behavior change.
- **LD-3 names the semantic layer at `@orgsidian/core/src/parser/semantic/`** ([architecture.md:65](architecture.md#L65)) — stale monorepo-era path. The real location is `crates/orgsidian-parser/src/semantic/` per epics.md (Story 2.3). NOT this story's concern; flagged only so the dev doesn't chase the old path. No edit (architecture.md is archival per [architecture.md:1010](architecture.md#L1010)).

### Testing Standards Summary

- **Integration tests (Cargo)** under `crates/orgsidian-parser/tests/*.rs`, auto-discovered. Post-Story-2.2: `anchor.rs` (1 test, unchanged) + `grammar.rs` (2 tests, NEW). `grammar_link.rs` deleted. Net parser-crate test count: 2 → 3.
- **Anchor sentinel:** `cargo test -p orgsidian-parser --test anchor --locked` MUST stay green with the file byte-unchanged — the proof that the public `parse()` signature survived the body swap.
- **Runtime budget:** `cargo test -p orgsidian-parser --locked` < 5s warm (Story 2.1 baseline ~0.7s; the real parse adds negligible time — tree-sitter parses a 10-line sample in microseconds).
- **CI matrix:** macOS-arm64 + Ubuntu-LTS per PR ([pr.yml](.github/workflows/pr.yml)); + Windows + Arch nightly ([nightly.yml](.github/workflows/nightly.yml)). Submodule checkout already `recursive` (Story 2.1 AC5) — no CI-config change in this story.
- **No `build.rs` change.** The `cc` compile of `parser.c`/`scanner.c` (Story 2.1) is unchanged; this story only *consumes* the compiled `tree_sitter_org()` symbol via `grammar::language()`.

### References

- Source story: [`epics.md:763-775`](_bmad-output/planning-artifacts/epics.md#L763-L775) — Story 2.2 user-story + AC.
- Previous story (Story 2.1 — vendoring): [`2-1-vendor-tree-sitter-org-as-sha-pinned-git-submodule.md`](_bmad-output/implementation-artifacts/2-1-vendor-tree-sitter-org-as-sha-pinned-git-submodule.md) — `grammar::language()`, the `_language_for_smoke` shim, `grammar_link.rs`, the anchor-sentinel discipline, the 0.26.x API notes (Debug Log: `version()`→`abi_version()`, `set_language(&lang)` borrowed form).
- Next stories that build on this: [`epics.md:777-811`](_bmad-output/planning-artifacts/epics.md#L777-L811) (Story 2.3 semantic layer — consumes `parse_tree.tree()`), [`epics.md:813-826`](_bmad-output/planning-artifacts/epics.md#L813) (Story 2.4 serializer — FR-2).
- Files to modify: [`src/lib.rs`](crates/orgsidian-parser/src/lib.rs) (parse body, ParseTree, ParseError, header, delete shim), [`tests/grammar.rs`](crates/orgsidian-parser/tests/grammar.rs) (NEW), [`tests/grammar_link.rs`](crates/orgsidian-parser/tests/grammar_link.rs) (DELETE), optionally [`src/grammar/mod.rs`](crates/orgsidian-parser/src/grammar/mod.rs) (doc tighten).
- Files that MUST stay unchanged: [`tests/anchor.rs`](crates/orgsidian-parser/tests/anchor.rs), [`tests/fixtures/anchor.org`](crates/orgsidian-parser/tests/fixtures/anchor.org), [`build.rs`](crates/orgsidian-parser/build.rs), [`grammar/`](crates/orgsidian-parser/grammar/) (submodule).
- Architecture (LD-3 parser selection): [`architecture.md:65`](_bmad-output/planning-artifacts/architecture.md#L65).
- Architecture (LD-48 vendoring discipline — grammar is READ-ONLY): [`architecture.md:1276-1281`](_bmad-output/planning-artifacts/architecture.md#L1276-L1281).
- Architecture (LEAF graph rule): [`deny.toml:174-176`](deny.toml#L174-L176) — `orgsidian-parser` may only be a direct dep of `orgsidian-core`.
- Architecture (stack version): [`architecture.md:185`](_bmad-output/planning-artifacts/architecture.md#L185) — `tree-sitter` latest stable; resolved 0.26.9 in [`Cargo.lock`](Cargo.lock).
- PRD (FR-1): [`prd.md:142-148`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L142) — open/parse `.org`; fall back to plain-text on parse error (the wrapper's `Ok`+ERROR-nodes posture supports this).
- FR Traceability Discipline: [`CONTRIBUTING.md:109-124`](CONTRIBUTING.md#L109-L124) — `//! Implements FR-1` is non-negotiable.
- Conventional Commits scope `parser`: [`CONTRIBUTING.md:53`](CONTRIBUTING.md#L53).
- `tree-sitter` Rust crate docs: <https://docs.rs/tree-sitter/0.26.9/tree_sitter/> — `Parser`, `Language`, `Tree`, `Node`.
- tree-sitter-org root rule: `grammar/src/grammar.json` first rule `document`; `grammar/src/node-types.json` contains `document`.
- [[feedback_version_policy]] — latest-stable pin discipline.
- [[feedback_no_co_author_credit]] — commit/PR hygiene.
- [[feedback_batch_fixes_terse]] — STOP-and-ask threshold; apply no-brainer fixes silently.
- [[feedback_role_agnostic_naming_in_docs]] — role-agnostic phrasing in any doc touched.

### Previous Story Intelligence (from Story 2.1)

- **Anchor-sentinel discipline is the whole reason `parse()` returns `Result<ParseTree, ParseError>`.** Story 2.1's scope-fence (Dev Note 1) explicitly said "Story 2.2 is the story that replaces the stub body" and "the public signature is preserved across that replacement." Honor it literally.
- **0.26.x API gotchas Story 2.1 already hit:** `Language::version()` was renamed `abi_version()`; `set_language` takes a borrowed `&Language`. The working `grammar_link.rs` is your reference for the exact call shape.
- **Cold-build cost** is paid by Story 2.1's `cc` compile (5.15s macOS-arm64 cold; ~0.2s warm). This story adds no compile-time cost beyond the trivial new test. `Swatinem/rust-cache@v2` keeps CI warm for the unchanged grammar SHA.
- **Deferred-work convention:** pre-seed the `## Deferred from: code review of story-2.2` stanza at impl time (T13) so the code-review pass appends rather than re-derives context — exactly as Story 2.1 did.
- **`cargo deny`/`audit` posture:** 18 pre-existing gtk-rs/Tauri unmaintained advisories are the known baseline; this story introduces no new dep, so the count must not move.
- **Commit hygiene:** scope `parser`, `closes #18`, no AI-credit trailers/footers.

### Git Intelligence Summary

`git log --oneline -5` at story-write:
- **`8201dad`** — Merge PR #138 (Story 2.1). Vendored the grammar; landed `grammar::language()`, `_language_for_smoke`, `grammar_link.rs`, `build.rs`. This story consumes #138's output and retires its two transitional artifacts (shim + link smoke).
- **`f58c5f3`** — Story 2.1 code-review fixes (`Parser::set_language` guard in the smoke, `build.rs` rerun-if-changed gaps, CONTRIBUTING §8 anchors).
- **`4580060`** — Story 2.1 core impl.
- The only prior `crates/orgsidian-parser/` touches are Story 1.9 (anchor stub) and Story 2.1 (vendoring). Story 2.2 is the first to write a *real* `parse()` body — no other in-flight branch touches this crate.

### Latest Technical Information

- **`tree-sitter` Rust crate**: resolved **0.26.9** in [`Cargo.lock`](Cargo.lock) (pinned `"0.26"` at workspace by Story 2.1). MIT. Surface used: `Parser::new`, `Parser::set_language(&Language)`, `Parser::parse(text, None) -> Option<Tree>`, `Tree::root_node`, `Node::kind`. Verify against <https://docs.rs/tree-sitter/0.26.9/> at impl time.
- **`nvim-orgmode/tree-sitter-org`**: SHA-pinned at `219c0b27fdb2c0aeb43841f23f03d6f54657f288` (Story 2.1). Root grammar rule = `document` (the value AC3 asserts). Grammar name `org` → FFI symbol `tree_sitter_org()` (already bound in `grammar/mod.rs`).
- **No version bumps in this story.** Pure consumption of the Story 2.1 wiring.

### Project Context Reference

- [`prds/prd-orgsidian-2026-05-19/prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) — FR-1 (§4.1).
- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-3 (parser selection), LD-5 (crate API barriers), LD-48 (vendoring discipline), crate dependency graph (LEAF rule), stack-versions table.
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (Stories 2.1 → 2.8).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §2 commit scope `parser`, §4 FR traceability, §8 parser ownership (LD-48, added Story 2.1).
- [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) — Story 2.1 stanza (lines 131-143) names the `_language_for_smoke` deletion + `grammar_link.rs` replacement as Story 2.2 work.
- [[feedback_version_policy]], [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]], [[feedback_role_agnostic_naming_in_docs]].

## Dev Agent Record

### Agent Model Used

claude-fable-5[1m] (Fable 5)

### Debug Log References

- RED phase: `cargo test -p orgsidian-parser --test grammar` failed as expected against the Story 1.9 stub (E0599 `root_node` not found on `ParseTree`; `parse("")` returned `Err(Empty)`), then GREEN after the body swap — both new tests pass.
- tree-sitter 0.26.9 API confirmed at impl time: `set_language(&Language)` borrowed form, `parse(text, None) -> Option<Tree>`, `Tree: Debug` holds (plain `#[derive(Debug)]` on `ParseTree` compiles; no hand-written impl needed).
- Anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green; `git status` confirms `tests/anchor.rs`, `tests/fixtures/anchor.org`, `build.rs`, and the `grammar/` submodule are byte-for-byte untouched.
- Gates: `cargo build -p orgsidian-parser --locked` ok; `cargo test --workspace --locked` 48 passed / 0 failed / 11 ignored; `cargo clippy --workspace --all-targets --locked` clean (no stale `#[allow(dead_code)]` existed to remove — Story 2.1 never added one); `cargo fmt --check` clean; `cargo doc -p orgsidian-parser --no-deps` ok; `cargo deny check licenses bans advisories` ok; `cargo audit` 18 allowed warnings (known gtk-rs/Tauri baseline, count unchanged — zero new deps).

### Completion Notes List

- **Variance (AC1, recorded as instructed):** epic prose at epics.md:772 names `pub fn parse(source: &str) -> tree_sitter::Tree`; implemented per intent while preserving the committed anchor-sentinel signature `parse(&str) -> Result<ParseTree, ParseError>` — `ParseTree` wraps `tree_sitter::Tree`, exposed via `root_node()` / `tree()` accessors. epics.md NOT edited (GitHub-issues sync-source), mirroring the Story 2.1 wording-variance handling.
- **Behavior change (AC4):** `ParseError::Empty` removed; `parse("")` now returns `Ok` with an empty `document` root (empty input is valid org). Safe: no workspace crate consumes `parse()` yet and no test asserted the `Empty` path. Covered by the new `parse_empty_input_is_ok` test.
- `ParseError` now models the real failure surface: `Grammar(#[from] tree_sitter::LanguageError)` (ABI mismatch, defensive) + `NoTree` (cancellation/timeout, neither set by this wrapper). `parse()` is panic-free — no `unwrap`/`expect` in the library body.
- `pub use tree_sitter;` re-export added at the crate root so Story 2.3 can name `orgsidian_parser::tree_sitter::Node` without its own `tree-sitter` dep (version pin single-sourced through this leaf crate).
- `_language_for_smoke` shim + comment block deleted (Story 2.1 deferred item); `grammar::language()` stays `pub(crate)`, now consumed by `parse()` directly. `grammar/mod.rs` doc-comments tightened to past tense.
- `tests/grammar_link.rs` deleted — the real `parse()` body exercises the identical cc-compile + extern-"C" link + `set_language` ABI path end-to-end; `tests/grammar.rs` (inline ≥10-line SAMPLE covering headings/TODO/DONE, `:PROPERTIES:` drawer, SCHEDULED/DEADLINE, bold/italic markup, id-link, checkboxes) replaces it.
- **Test-count delta:** parser crate 2 → 3 (anchor 1 unchanged + grammar 2 new; grammar_link −1). Workspace total 48 passed (was 47; the "(was 46)" originally reported here was an off-by-one bookkeeping error, corrected in code review 2026-06-10). Post-review: parser crate 3 → 5 (grammar.rs +2 tests: `ParseError` Display, `Send + Sync` compile guard; plus strengthened assertions inside the existing two); workspace 50 passed / 0 failed / 11 ignored (verified).
- **Zero new dependencies:** no `Cargo.toml`/`Cargo.lock` change; deny/audit deltas are nil.
- Pre-seeded the `## Deferred from: code review of story-2.2 (2026-06-10)` stanza in deferred-work.md (parser-per-call allocation; intentional ERROR/MISSING tolerance at the wrapper layer).

### File List

- `crates/orgsidian-parser/src/lib.rs` — modified (real `parse()` body, `ParseTree` wraps `Tree` + accessors, `ParseError` reshaped, `pub use tree_sitter;`, crate header rewritten with `Implements FR-1`, `_language_for_smoke` shim deleted)
- `crates/orgsidian-parser/src/grammar/mod.rs` — modified (doc-comments tightened to past tense; `language()` stays `pub(crate)`)
- `crates/orgsidian-parser/tests/grammar.rs` — new (root-kind `document` assertion + empty-input behavior test)
- `crates/orgsidian-parser/tests/grammar_link.rs` — deleted (subsumed by the real `parse()` path in `tests/grammar.rs`)
- `_bmad-output/implementation-artifacts/deferred-work.md` — modified (story-2.2 pre-seeded stanza)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — modified (story 2.2 status transitions)
- `_bmad-output/implementation-artifacts/2-2-implement-orgsidian-parser-grammar-wrapper.md` — modified (this file: checkboxes, Dev Agent Record, File List, Change Log, Status)

## Change Log

- 2026-06-10 — Story 2.2 implemented: tree-sitter-org grammar wrapper replaces the Story 1.9 stub (`ParseTree` wraps real `Tree`, `ParseError` reshaped, empty-input now `Ok`, shim + link-smoke retired, `tests/grammar.rs` added). All gates green; status → review.
