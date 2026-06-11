# Story 2.6: Light up L0 round-trip CI gate (per-PR ~100 files <60s)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 22

## Story

As the **author / contributor**,
I want the L0 byte-identical round-trip gate running on every PR against the ~100-file subset in <60s wall-clock,
So that FR-2 is enforced as a hard CI contract from v0.1 Alpha onwards per LD-32.

**Traces:** FR-2 (the trust contract — enforcement step; the FR-2 trace *owner* stays `crates/orgsidian-parser/src/serializer.rs`, single trace owner per CONTRIBUTING §4 — **no new `//! Implements FR-` header anywhere in this story**), LD-32 (per-PR subset gate, <60s, macOS-arm64 + Ubuntu-LTS), LD-44 (the subset being consumed), LD-37 (`--locked` on every cargo invocation), test-design.md §6.10 L0 row + §7.3.13 (CI-gate story scaffold), R-025 (v0.1 Alpha tag blocked unless 2.6+2.7 green).

## Scope Fence (read first)

This is the **CI-wiring** story: it repoints the existing `round_trip_subset` harness at the real LD-44 subset (`fixtures/subset-pr.json`) and lights the per-PR gate in `pr.yml`, closing the deferred-work items pre-assigned to 2.6 along the way. It is **not**:

- **NOT a parser implementation change.** Nothing under `crates/orgsidian-parser/src/` changes; `build.rs`, `grammar/` (LD-48), `tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/{anchor.org,round_trip/}` are byte-for-byte sentinels. The only parser-crate edits are `tests/round_trip.rs` (harness extension) and `Cargo.toml` (`serde_json` dev-dep edge).
- **NOT corpus regeneration.** `fixtures/subset-pr.json`, `fixtures/full-nightly.json`, `tests/fixtures/vault-corpus/**` are **generated artifacts — never hand-edit** (CONTRIBUTING §5). No `tools/corpus-extractor/src/**` changes either: the extractor is consumed (built + tested in CI), not modified.
- **NOT the nightly/L2 gates (Story 2.7).** `nightly.yml` stays byte-untouched — its Story-2.7 placeholder comment (line ~160) stays. No Emacs, no `canonical_ast/`, no full-corpus gate, no L2-subset designation.
- **NOT a git-LFS history rewrite.** The `FOLLOWUP(Story-2.6)` LFS-migration marker is *dispositioned* here (AC6), but `git lfs migrate import` rewrites history and is **forbidden** while the 2.3→2.6 stacked branches/PRs are unmerged. git-lfs is verified absent in this environment anyway (story-creation check: `git lfs version` → "not a git command") — the expected outcome is a documented re-deferral.
- **NOT a branch-protection change.** GitHub Free cannot enforce required checks ([[project_orgsidian_github_plan]]) — the gate is "hard" by convention + merge-gate job, same as every existing gate. No org/repo settings work.

Deliverables: harness repoint (`tests/round_trip.rs` + dev-dep edge), three `pr.yml` additions (L0 gate step, extractor test step, extractor lockfile audit) + rust-cache workspaces extension, LFS-followup disposition (docs + markers), CONTRIBUTING/fixtures.toml doc touch-ups, deferred-work hygiene.

## Acceptance Criteria

### AC1 — `round_trip_subset` consumes `fixtures/subset-pr.json` (epic-core).

**Given** Story 2.5's committed manifest (100 entries, embedded content — schema in Dev Notes §2) and Story 2.4's directory-driven harness,
**When** [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs) is extended,
**Then**:

- The test named **`round_trip_subset`** (public name contract — locked since 2.4, invoked verbatim by AC2's step) loads `fixtures/subset-pr.json` from `CARGO_MANIFEST_DIR/../../fixtures/subset-pr.json` and asserts `serialize_document(&analyze(content)?) == content` byte-for-byte for **every** entry's embedded `content`, using the entry `id` as the failure label through the existing `assert_round_trip` diagnostics (first divergent byte offset + ±20-byte context windows). Failure on any entry fails the test → fails the PR.
- **Anti-placebo:** assert the manifest has **exactly 100 entries** (LD-44: 30+50+20 by construction) and a non-empty `header` (presence check is enough — pin-value sync is owned by the extractor's `validate.rs`, don't duplicate the matrix validator here).
- **Extend, don't replace:** the interim 18-file directory iteration over `tests/fixtures/round_trip/` stays inside the gate (those handcrafted shapes + the byte-sensitivity tripwires are not in the extracted subset). One `#[test] fn round_trip_subset` covering both sources is the recommended shape; a split into two fns is acceptable ONLY if both names contain the substring `round_trip_subset` (cargo's filter is substring-based — `corpus_retains_byte_sensitive_fixtures` and the inline/proptest tests must stay NON-matching so the gate step runs exactly the corpus tests).
- Manifest parsing via `serde_json` added to parser `[dev-dependencies]` as `{ workspace = true }` (root pin `serde_json = "1"` exists, Cargo.toml:35; already resolved in Cargo.lock via orgsidian-core → **dependency edge only, zero new crates**). `serde_json::Value` access is sufficient; a typed struct with a `serde` derive dev-dep edge is also fine (also zero new crates). Any other lockfile delta: STOP, decision-grade question.
- Test code may use `unwrap()/expect()` per house rules, but failures a maintainer will actually see (missing/unreadable manifest, malformed JSON, wrong entry count) must panic with actionable messages naming the file and the regeneration pointer (CONTRIBUTING §5), not a bare `Option::unwrap` backtrace.

### AC2 — `pr.yml` L0 gate step: epic-verbatim invocation, <60s enforced, both matrix OSes.

**Given** [.github/workflows/pr.yml](.github/workflows/pr.yml) (single `pr` job, matrix `macos-14` + `ubuntu-24.04` — epic AC "macOS-arm64 + Ubuntu-LTS" is satisfied by placement inside the existing matrix job, no new job),
**When** the gate step is added,
**Then**:

- A named step (suggested: `L0 round-trip subset gate (LD-32/LD-44, <60s)`) runs `cargo test -p orgsidian-parser round_trip_subset --locked -- --test-threads=4`. The invocation is epic-verbatim (epics.md:854) **plus `--locked`** per the LD-37 house rule on every cargo invocation in this workflow — record as variance, don't drop either. (`--test-threads=4` is a no-op for a single test fn; it's part of the published contract, keep it.)
- The step carries **`timeout-minutes: 1`** — this is the <60s budget made *enforcing*, exactly the test-design §7.3.13 scaffold. It is safe because the step reuses binaries already compiled by Step 9 (`cargo test --workspace`) in the same job — the step's wall-clock is pure test runtime (measured 1.75s for all 100 entries locally, debug profile; Dev Notes §3). If you deviate from `timeout-minutes: 1`, you must add an equivalent in-step runtime assertion.
- Placement: in the Rust section right after Step 9.1 (`cargo test (tools/issues-sync)`) — binaries warm, early signal, does not disturb the load-bearing Step 14→16 ordering (deferred-work story-1.17 note). Add a short comment block in the existing house style (LD references + rationale). Update the slot-reservation comment block (line ~180): remove/annotate the `Story 2.6:` line.
- Note in the step comment that Step 9's `cargo test --workspace` already executes `round_trip_subset` once — the dedicated step is the *named, budgeted* contract (visible step timing in the Actions UI + the timeout enforcement), not redundant coverage by accident.
- `nightly.yml` untouched. No new top-level job (step-level checks aren't branch-protection contexts, and GitHub Free can't require them anyway — the matrix `pr` job stays the unit of greenness).

### AC3 — Extractor CI build/test step (closes deferred item, story-2.5 stanza, MED).

**Given** the deferred item "CI build/test step for `tools/corpus-extractor`" (owner: "Story 2.6's pr.yml edit") and the Step 9.1 issues-sync precedent,
**When** the step is added,
**Then**:

- A step `cargo test (tools/corpus-extractor)` runs `cargo test --manifest-path tools/corpus-extractor/Cargo.toml --locked` (the documented CONTRIBUTING §3 invocation style), placed adjacent to Step 9.1. This compiles the extractor for the first time in CI and runs its 65-test suite — notably the **TC-3 matrix meta-test** (committed `subset-pr.json` still satisfies the LD-44 matrix → fixture-rot tripwire now fires per-PR) and the **round-trip preflight twin** (all 100 entries through the parser path-dep — a second, independent execution of the L0 property).
- **Cache the tool target dirs:** extend the existing `Swatinem/rust-cache@v2` step with a `workspaces` input covering the root workspace AND both tool crates (`.`, `tools/corpus-extractor`, `tools/issues-sync` — one per line; each defaults to its `target` dir). Without this, the extractor's parser path-dep (tree-sitter + cc + chrono) recompiles cold on every PR (~2-4 min/cell); with it, warm runs are seconds. This also fixes the pre-existing uncached issues-sync build — note it in Completion Notes.
- Build prerequisites already satisfied: `submodules: recursive` checkout (the parser path-dep needs the grammar submodule) and the Linux native-deps step are already in the job. The extractor's `ureq` is vendored-TLS — no extra system packages expected; if a system-lib surprise appears on either OS, that's decision-grade, report it.
- No test of this story may touch the network: the extractor suite is already network-free by design (fetch is a maintainer operation) — do not add `fetch`/`extract` invocations to CI.

### AC4 — Scoped supply-chain scan of the extractor lockfile (closes deferred item, story-2.5 stanza, LOW).

**Given** the deferred item "Supply-chain scanning of the extractor's standalone lockfile" (ADR 0001 §6) and the existing Step 11 `cargo audit` pattern,
**When** the scan is wired,
**Then**:

- Step 11 is extended (or a sibling step added) to also run `cargo audit --deny warnings $IGNORES --file tools/corpus-extractor/Cargo.lock` — cargo-audit reads a lockfile directly via `--file`, no manifest dance. Reuse the same `$IGNORES` expansion from `.cargo/audit-ignore.txt` (single source of truth; the extractor lockfile shouldn't trip the gtk-era ignores, but lockstep beats divergence).
- If the extractor lockfile surfaces a NEW advisory requiring an exception: **STOP, decision-grade** — no silent `.cargo/audit-ignore.txt` or `docs/security/advisory-exceptions.md` edits.
- **License-side scoping decision (pre-made):** `cargo deny` is workspace-config-bound and the extractor's `ureq` TLS stack (rustls/ring/webpki-roots family) likely carries licenses outside the root allowlist. The deferred item's "(or audit)" branch is the sanctioned closure: audit-only. If a scoped `cargo deny --manifest-path` invocation works without ANY `deny.toml`/allowlist change, you may add it as a bonus; otherwise re-defer the license-scan half explicitly in the AC8 stanza (one line, named rationale). Do not edit `deny.toml`.

### AC5 — Gate runtime evidence (<60s AC made falsifiable).

**Given** the epic AC "total round-trip subset gate runtime is <60s on the GitHub Actions runner" cannot be *observed* until the PR runs,
**When** the story completes,
**Then**:

- Local measurements recorded in Completion Notes: `time cargo test -p orgsidian-parser round_trip_subset --locked -- --test-threads=4` (warm build) — story-creation baselines to beat/confirm: interim-18-files 0.01s; all-100-embedded 1.75s via the preflight twin (Dev Notes §3).
- The `timeout-minutes: 1` step config (AC2) is the in-CI enforcement mechanism from the first run onward.
- The PR-body note (for the pipeline step that opens the PR) must ask the merger to confirm the step duration on both matrix cells in the first Actions run — record as a process item in Completion Notes; you cannot watch CI from this session.

### AC6 — git-LFS `FOLLOWUP(Story-2.6)` disposition (closes/redirects deferred item, story-2.5 stanza, MED).

**Given** the marker in [.gitattributes](.gitattributes), the CONTRIBUTING §5 "Current state (Story 2.5 fallback)" note, the `fixtures.toml` vault-corpus note, and the deferred-work item ("history rewrite — coordinate before any corpus growth"),
**When** the disposition is made,
**Then**:

- Run `git lfs version` first (story-creation check says NOT installed here; re-verify). **Expected path — re-deferral, properly documented:** (a) the deferred-work item is annotated: 2.6 checked, git-lfs still unavailable, ownership re-assigned to "first maintainer machine with git-lfs, after the Epic-2 story stack merges"; (b) the `.gitattributes` `FOLLOWUP(Story-2.6)` marker text is updated to the new owner and gains the no-history-rewrite caveat; (c) the CONTRIBUTING §5 current-state note stops naming Story 2.6 as the migration owner (one-line edit). The commented-out LFS stanza itself and the active `-text` rule stay exactly as they are.
- **Hard guard either way:** `git lfs migrate import` (the command quoted in the deferred item) **rewrites history** — running it now would orphan the stacked 2.3→2.6 branches and any open PRs. It must NOT run in this story. If git-lfs IS unexpectedly available, the only sanctioned in-story action is still the documentation re-deferral above — optionally noting the non-rewriting alternative for the future owner: uncomment the stanza + `git lfs install` + `git add --renormalize tests/fixtures/vault-corpus` in a normal commit (old raw blobs stay in history, ~2.3 MB, acceptable; future versions go through LFS).
- The corpus files themselves stay byte-untouched (generated fixtures; `[fixture:epic-2]` governance does not fire — no fixture mutation happens in this story).

### AC7 — Doc touch-ups kept honest.

**Given** docs that describe the CI gate set and fixture consumers,
**When** the wiring lands,
**Then**:

- CONTRIBUTING §1 "CI parity check" one-liner is extended to cover the new per-PR surface — minimally: append the two `--manifest-path` tool-suite invocations (issues-sync was already missing from the parity line — pre-existing gap, fix it in the same stroke and note it). Do not bloat: the L0 gate is a subset of `cargo test --workspace`, parity-wise it's already covered; a parenthetical naming the dedicated gate step is enough.
- `fixtures/fixtures.toml` `parser.round-trip-interim` note updated: "Story 2.6 repoints/extends the harness" → present tense (harness now consumes `fixtures/subset-pr.json` + the interim dir). `corpus.subset-pr` note already names the 2.6 consumers — verify, touch only if stale. (fixtures.toml is hand-maintained — notes edits are legal; the `[fixture:epic-N]` tag is NOT needed since no fixture *content* mutates.)
- `fixtures/README.md` consumers table: verify rows still accurate (they already name the 2.6 gate) — expected zero-edit.
- No edits to `epics.md` / `architecture.md` / `test-design.md` / PRD (variance-recording instead, Dev Notes §8). `docs/parser/KNOWN_DIVERGENCES.md` untouched (no new divergences can appear — the corpus already round-trips, see Dev Notes §3).

### AC8 — Gates stay green; sentinels untouched; deferred-work hygiene.

**Given** all the above,
**When** the gates run,
**Then**:

- `cargo test -p orgsidian-parser --locked` green (baseline 77 tests; report the delta — extending the existing fn keeps the count, added unit/shape tests are welcome and reported). `cargo test --workspace --locked` green (baseline **122 passed / 0 failed / 11 ignored**; workspace runtime grows by ~2s from the 100-entry loop — fine). Extractor suite via `--manifest-path` green (baseline **65 tests**: 60 lib + 3 matrix + 2 preflight).
- `cargo clippy --workspace --all-targets --locked` clean; `cargo fmt --all -- --check` clean; `cargo deny --locked check all` ok; `cargo audit` at the 18-allowed-warnings baseline; the new extractor-lockfile audit (AC4) green.
- Root `Cargo.lock` delta: exactly the `serde_json` (± `serde`) dev-dep edge(s) on orgsidian-parser, **zero new crates**. `deny.toml`, `.cargo/audit-ignore.txt`, `docs/security/advisory-exceptions.md` untouched.
- Workflow lint: `pr.yml` must stay parseable — sanity-check YAML (e.g. `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/pr.yml'))"`); there is no actionlint in the repo, eyeball indentation against the existing steps.
- Sentinels via `git status`: nothing under `crates/orgsidian-parser/src/`, `crates/orgsidian-parser/tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/round_trip/`, `grammar/`, `build.rs`; `tools/corpus-extractor/**` untouched; `fixtures/*.json` + `tests/fixtures/vault-corpus/**` byte-untouched; `nightly.yml` untouched.
- deferred-work.md: story-2.5 stanza items annotated (CI step → closed by 2.6; supply-chain → audit closed + license-half disposition; LFS → re-deferred with new owner); a `## Deferred from: code review of story-2.6 (YYYY-MM-DD)` stanza pre-seeded at impl time. Known candidates: LFS migration (re-deferred, AC6); license-scan of the extractor lockfile (if AC4's bonus didn't fit); first-CI-run timing confirmation (process); `dtolnay/rust-toolchain@stable` floating-ref hardening (pre-existing, story-1.16 stanza — do NOT fix here).

## Tasks / Subtasks

- [x] **T1** — Harness repoint: add `serde_json = { workspace = true }` dev-dep (Story-2.6 comment); extend `round_trip_subset` to load + iterate `fixtures/subset-pr.json` (exactly-100 assert, id-labeled `assert_round_trip`, actionable failure messages) while keeping the interim-dir iteration. RED-first: point the loader at the manifest with a deliberately wrong count assert (or temp-tamper a content string in memory) to watch the diagnostics fire, then finalize. (AC1)
- [x] **T2** — Measure: `time cargo test -p orgsidian-parser round_trip_subset --locked -- --test-threads=4` warm; record numbers for Completion Notes (expect low single-digit seconds). (AC5)
- [x] **T3** — `pr.yml`: L0 gate step after Step 9.1 (epic-verbatim invocation + `--locked`, `timeout-minutes: 1`, house-style comment) + slot-reservation comment update. (AC2)
- [x] **T4** — `pr.yml`: extractor test step (`--manifest-path`, `--locked`) + `rust-cache` `workspaces` extension (root + both tools). (AC3)
- [x] **T5** — `pr.yml`: extractor lockfile audit (`cargo audit ... --file tools/corpus-extractor/Cargo.lock`, shared `$IGNORES`); decide the deny-licenses bonus per AC4's pre-made scoping. (AC4)
- [x] **T6** — LFS disposition: `git lfs version` check; deferred-work annotation + `.gitattributes` marker re-point + CONTRIBUTING §5 one-liner per AC6. NO `git lfs migrate import` under any circumstance. (AC6)
- [x] **T7** — Doc touch-ups: CONTRIBUTING §1 parity line; `fixtures/fixtures.toml` interim note present-tense; verify `fixtures/README.md` rows. (AC7)
- [x] **T8** — deferred-work.md: annotate the three consumed 2.5 items; pre-seed the story-2.6 stanza. (AC8)
- [x] **T9** — Gates: parser suite, workspace suite, extractor suite, clippy/fmt/deny/audit (+ new scoped audit), YAML sanity, `git status` sentinel sweep. Report exact counts + lockfile delta in Completion Notes. (AC8)
- [x] **T10** — Commit. Suggested title: `feat(ci): light up L0 round-trip per-PR gate (Story 2.6, closes #22)` (scope `ci` per the Story-1.16 precedent; enum advisory, CONTRIBUTING §2). **NO** Co-Authored-By trailer, **NO** "Generated with Claude Code" footer, no AI-credit lines. PR body (when the pipeline's PR step runs): gate invocation + measured runtimes + first-run timing confirmation ask + deferred-items closure list + `Closes #22`. (process)

### Review Findings

Adversarial code review of commit `3834fc8` vs `2af292c` (2026-06-11, three layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor — run sequentially in-session, no subagent dispatch available in this environment; layer evidence boundaries respected).

- [x] [Review][Patch] Stale `FOLLOWUP(Story-2.6)` marker reference survived the AC6/AC7 honesty pass [tests/fixtures/vault-corpus/README.md:36] — the hand-written provenance README (explicitly preserved by the extractor's regeneration wipe, `emit.rs:120`; not generated content, so the edit is legal under the same precedent as this story's `fixtures/README.md` honesty edit) still named the old marker and the `git lfs migrate import` follow-up without the re-deferral. FIXED: marker renamed to `FOLLOWUP(LFS-migration)` + re-deferral wording aligned with CONTRIBUTING §5. No corpus *content* touched — `[fixture:epic-2]` governance does not fire.
- [x] [Review][Patch] CONTRIBUTING §8 pin-bump checklist still flags the L0 gate as "future" [CONTRIBUTING.md:209] — "(Story 2.6 — flag as 'future' until that story ships)" became false the moment this story landed. FIXED: parenthetical now names the live `pr.yml` gate step.
- [x] [Review][Defer] PR body `Closes #22` gate (project review customization) — no PR exists yet for `story/2.6-l0-round-trip-ci-gate` (stacked pre-PR review; this run is forbidden from creating/editing PRs and issues). The pipeline step that opens the PR MUST include `Closes #22` in the body (and the `status:done` label flip on issue #22 is owned by the orchestrator per the same constraint) — already captured as the T10/AC5 process items; recorded in the deferred-work story-2.6 stanza.
- [x] [Review][Defer] `dtolnay/rust-toolchain@stable` floating-ref hardening [.github/workflows/pr.yml] — deferred, pre-existing (story-1.16 stanza; already cross-referenced in the story-2.6 stanza — no new entry).

Dismissed as noise (4): `timeout-minutes: 1` recompile risk from `-p` vs `--workspace` feature unification — **empirically disproved** (`cargo test -p orgsidian-parser --no-run` after a workspace build finishes in 0.25s with zero recompilation; a future unification break would fail loudly via the timeout, not silently); duplicate manifest `id`s unguarded in the gate (owned by the extractor's `validate.rs` + TC-3, which runs in the same PR via Step 9.3); rust-cache entry growth + one-time cache bust from the new `workspaces` input (by design, documented in Completion Notes); unquoted `$IGNORES` in the audit step (intentional word-splitting, pre-existing house pattern).

Review verification (post-fix, all green): gate invocation `cargo test -p orgsidian-parser round_trip_subset --locked -- --test-threads=4` → 1 passed, test body **1.63s** (filter matches exactly 1 test, verified via `--list`); parser suite **77/0**; workspace **122 passed / 0 failed / 11 ignored**; extractor suite **65/0**; `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all -- --check` clean; pr.yml YAML-valid (ruby); epic invocation verified verbatim against epics.md:854 (+`--locked`, recorded variance); `timeout-minutes: 1` matches test-design §7.3.13; root Cargo.lock delta = 1 line (`serde_json` dev-dep edge on orgsidian-parser, zero new crates); sentinel sweep clean (diff touches exactly the declared File List). Review fixes are doc-only — no code or workflow-step changes.

## Dev Notes

### 1. Current state (verified at story-creation, branch `story/2.6-l0-round-trip-ci-gate` @ `edc568a`, 2026-06-11)

- **Harness:** `crates/orgsidian-parser/tests/round_trip.rs` — `round_trip_subset` iterates `tests/fixtures/round_trip/*.org` (18 interim files, ≥10 floor assert), with `assert_round_trip(label, src)` providing first-divergent-byte + context-window diagnostics. The file's own doc-comment says the test NAME is the public contract and "Story 2.5's full ~100-file subset plugs into this same test". 14 tests total in the file; the substring `round_trip_subset` matches exactly one fn today.
- **Manifest:** `fixtures/subset-pr.json` (5.1 MB, regular git, `text eol=lf` protected) — 100 entries, embedded content totaling ~2.33 MB (min 3 B, max 96 KB). Committed by 2.5's orchestrator run; extractor `verify` + TC-3 meta-test green against it.
- **pr.yml:** single matrix job (`macos-14`, `ubuntu-24.04`), steps 1→16: checkout (submodules recursive) → Linux deps → rust toolchain + `Swatinem/rust-cache@v2` (shared-key `pr-${{ matrix.os }}`, NO `workspaces` input today) → pnpm/node → `cargo fmt/clippy/build/test --workspace` → Step 9.1 issues-sync `--manifest-path` test → deny → audit (with `$IGNORES` from `.cargo/audit-ignore.txt`) → pnpm audit/licenses/allowlist-sync → shell-ui build → i18n drift → slot-reservation comment block (line ~180, carries the `Story 2.6:` line) → a11y block (Step 14→16 ordering is load-bearing — do not reorder). Plus the separate `merge-gate-nightly-fresh` job (don't touch).
- **Deferred items pre-assigned to 2.6** (deferred-work.md, story-2.5 stanza): extractor CI step [MED], extractor-lockfile supply-chain scan [LOW], vault-corpus git-LFS migration [MED, FOLLOWUP marker in `.gitattributes`].
- **git-lfs:** NOT installed in this environment (`git lfs version` → "git: 'lfs' is not a git command").
- **GitHub issue #22** verified OPEN: "[Story 2.6] Light up L0 round-trip CI gate (per-PR ~100 files <60s)" (labels epic:2 / type:story / status:backlog / milestone:v0.1).
- **Branch stack:** `story/2.6-l0-round-trip-ci-gate` sits on completed 2.5 (`edc568a`) ← 2.4 ← 2.3 ← main(2.2). The 2.3/2.4/2.5 commits are local to this lineage; PR base may need their merges first — flag in PR description (established pattern).

### 2. Manifest schema (the contract Story 2.5 shipped — consume, don't reshape)

```json
{
  "header": { "generator": "orgsidian-corpus-extractor", "extractor_version": "0.0.0",
               "org_release_tag": "release_9.8.5", "source_sha256": "f3065e65…" },
  "entries": [ {
      "id": "extracted/0076_citation-parser-12",      // ← your failure label
      "path": "extracted/0076_citation-parser-12.org", // vault-corpus twin (NOT read by this gate)
      "size_bucket": "small", "byte_len": 15,
      "constructs": ["table", "citation"], "edge_buckets": [],
      "provenance": { "kind": "extracted", "deftest": "test-org-element/citation-parser" },
      "content": "| [cite:@key] |"                     // ← the bytes you round-trip
  } ] }
```

The gate reads `entries[].id` + `entries[].content` only. Do NOT touch `path` (LFS-pointer territory belongs to the extractor's twin test), do NOT re-validate the LD-44 matrix (extractor `validate.rs` owns it — and AC3 puts that meta-test in the same PR run anyway). `content` is a JSON string — CRLF/trailing-whitespace bytes live inside escapes, immune to checkout EOL mangling by design.

### 3. Runtime evidence (measured at story-creation, this machine, debug profile)

- `cargo test -p orgsidian-parser --locked round_trip_subset`: test body **0.01s** (18 interim files); 8.6s wall including warm-ish build.
- Extractor preflight (`every_subset_entry_round_trips_byte_faithfully` — analyze+serialize over ALL 100 embedded entries, plus the twin check): **1.75s**. This is the closest proxy for the post-repoint gate body: expect the gate step at low single-digit seconds, ~30x inside the 60s budget even on slower GitHub runners.
- Round-trip failures are effectively impossible from corpus *content*: Story 2.4's arbitrary-input identity property (`serialize_document(&analyze(s)?) == s` for ANY string, proptest 256 cases) plus the already-green preflight mean the only realistic gate failures are environmental (missing manifest, JSON corruption, EOL mangling) — which is exactly what the actionable-message requirement in AC1 is for.
- CI compile cost is pre-paid: pr.yml Step 8/9 build the workspace + all test binaries before the gate step runs; the dedicated step recompiles nothing. That is what makes `timeout-minutes: 1` safe.

### 4. Design: extend `round_trip_subset`, don't fork it

- One test fn, two sources (manifest entries + interim dir) is the cleanest honoring of both contracts: epic AC "consuming `fixtures/subset-pr.json`" and the 2.4-era doc-comment "plugs into this same test". The interim corpus stays because its 18 handcrafted shapes (kitchen-sink, deep nesting, preamble-only…) are curated differently from the extracted subset and cost 0.01s.
- Failure semantics: `assert_round_trip` panics on first divergence — that satisfies "failure on any subset file fails the PR". Optionally collect-all-then-panic listing every failing id (nicer triage on a corpus-wide systemic break); dev's choice, diagnostics quality is the non-negotiable.
- Keep `fixtures_dir()`/`assert_round_trip` helpers as-is; add a `manifest_path()` sibling (`CARGO_MANIFEST_DIR/../../fixtures/subset-pr.json`). The extractor already uses the same `../..` hop from its own manifest dir — established pattern.
- `corpus_retains_byte_sensitive_fixtures` (tripwire) and the inline/proptest tests stay byte-unchanged.

### 5. The invocation, dissected (don't "fix" it)

- `cargo test -p orgsidian-parser round_trip_subset -- --test-threads=4` is published in epics.md:854 (the GitHub-issues sync source) and test-design §7.3.13. Keep it verbatim; add `--locked` (LD-37 — every cargo invocation in pr.yml passes it; variance: epic AC omits it, house rule wins, record it).
- `--test-threads=4` parallelizes across test *fns*, not within the corpus loop — with one matching fn it's inert. It stays because the invocation string is the contract; do not satisfy it by splitting the corpus into 4 fns (gratuitous complexity; 1.75s needs no parallelism).
- The substring filter selects every test whose name contains `round_trip_subset` — current count: exactly 1. Guard any new test names in this file against accidental matches (or deliberate ones, per AC1's split option).

### 6. CI cost + cache mechanics (AC3's second half)

- `Swatinem/rust-cache@v2` `workspaces` input: newline-separated `path[ -> target-dir]` entries; default target dir is `target` under each path. Use `.`, `tools/corpus-extractor`, `tools/issues-sync`. The cache key derives from lockfiles under registered workspaces — tool-lockfile bumps will now correctly bust cache. Keep `shared-key: pr-${{ matrix.os }}` as-is.
- First post-merge PR run pays the extractor cold build once per OS cell (parser path-dep compiles tree-sitter + cc + chrono into `tools/corpus-extractor/target`); warm runs are seconds. Mention the one-time cost in the PR body so a slow first run doesn't read as a regression.
- The extractor suite re-runs the preflight (1.75s) and matrix meta-test per PR — that's the *intended* redundancy: gate (parser-side) + TC-3 (corpus-side) + preflight (cross-check) make corpus rot, harness rot, and parser regressions independently detectable.

### 7. LFS: why re-deferral is the correct outcome, not a failure

- The deferred item's own text says "history rewrite — coordinate before any corpus growth". `git lfs migrate import` rewrites every commit touching the corpus — i.e. the 2.5 commits this branch stack sits on. Mid-stack, pre-merge, with PRs pending: forbidden. (The non-rewriting `--renormalize` path exists for the future owner — documented in AC6.)
- git-lfs is absent on this machine anyway (verified). The valuable work 2.6 CAN do is make the marker honest: re-point ownership, encode the no-rewrite caveat, keep CONTRIBUTING accurate. A `FOLLOWUP(Story-2.6)` marker that survives Story 2.6 unchanged would be a lie; an annotated re-deferral is hygiene.
- Nothing in the per-PR gate depends on this either way (the manifest is LFS-free by design — the entire point of the embedded-content decision in 2.5/ADR 0001).

### 8. Variances (record in Completion Notes; no spec edits — epics.md is the issues sync-source)

1. Epic invocation gains `--locked` (LD-37 house rule on every pr.yml cargo invocation).
2. Epic "runs on macOS-arm64 + Ubuntu-LTS per PR" satisfied by the existing matrix job (`macos-14` + `ubuntu-24.04`), not a new job.
3. "<60s on the GitHub Actions runner" enforced via `timeout-minutes: 1` + measured local evidence; actual CI numbers confirmable only on the first Actions run (process note in PR body).
4. Harness *extends* rather than *replaces* the interim-corpus iteration (both sources under the contractual test name) — sanctioned by 2.4's variance #2 and fixtures.toml's "repoints/extends".
5. Deferred-item closures ride along (extractor CI step, lockfile audit, LFS disposition) — pre-assigned to 2.6 by the 2.5 stanza, not in the epic AC text.

### Project Structure Notes

**Alignment with unified project structure:**

- `crates/orgsidian-parser/tests/round_trip.rs` — UPDATE. Current state: Dev Notes §1/§4; what changes: `round_trip_subset` body + one helper + manifest asserts; what must be preserved: test name substring-uniqueness, all other test fns byte-unchanged, the `assert_round_trip` diagnostics shape, the file doc-comment updated to present tense (2.5/2.6 forward references → done). ✓
- `crates/orgsidian-parser/Cargo.toml` — UPDATE: `[dev-dependencies] serde_json = { workspace = true }` (+ optional `serde`) with Story-2.6 comment. ✓
- `.github/workflows/pr.yml` — UPDATE: 3 new steps + `rust-cache` `workspaces` + slot-comment update; matrix/jobs/triggers/concurrency unchanged; `merge-gate-nightly-fresh` untouched. Matches architecture.md:908 (`pr.yml` = the per-PR job, LD-32) and test-design §7.3.13. ✓
- `.gitattributes` — UPDATE: FOLLOWUP marker text only (comment line; the active rules byte-unchanged). ✓
- `CONTRIBUTING.md` — UPDATE: §1 parity one-liner + §5 LFS current-state one-liner. ✓
- `fixtures/fixtures.toml` — UPDATE: notes-only (interim entry present-tense). `fixtures/README.md` — expected zero-edit (verify). ✓
- `_bmad-output/implementation-artifacts/deferred-work.md` — UPDATE (annotations + 2.6 stanza); `sprint-status.yaml` — UPDATE (status transitions). ✓
- READ-ONLY / MUST NOT CHANGE: `crates/orgsidian-parser/src/**`, `build.rs`, `grammar/`, `tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/{anchor.org,round_trip/*.org}`, `tools/corpus-extractor/**`, `tools/issues-sync/**`, `fixtures/*.json`, `tests/fixtures/vault-corpus/**`, `.github/workflows/nightly.yml`, root `Cargo.toml`, `deny.toml`, `.cargo/audit-ignore.txt`, `docs/security/advisory-exceptions.md`, `docs/adr/0001-corpus-subset-selection.md`, `docs/parser/KNOWN_DIVERGENCES.md`, `_bmad-output/planning-artifacts/**`, `_bmad-output/test-artifacts/**`. (Root `Cargo.lock` changes ONLY by the dev-dep edge.) ✓

### Testing Standards Summary

- The story's own test surface IS the gate: AC1's extended `round_trip_subset` (anti-placebo: exact-100 + id labels + the 2.4 diagnostics) + AC3's per-PR execution of the extractor's 65-test suite. No new test files expected; report final counts (parser 77+δ, workspace 122+δ, extractor 65).
- Anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green, file byte-unchanged (Story 1.9 discipline).
- Every assertion must be falsifiable: the exact-100 count fails on a truncated manifest; the timeout fails on a runtime blowup; the TC-3 meta-test fails on corpus rot. Nothing in this story may pass vacuously if `fixtures/subset-pr.json` goes missing — that must be a loud, actionable failure.
- CI verification is two-stage by nature: all local gates in-session; step-timing confirmation on the first real Actions run (process note, AC5).

### Previous Story Intelligence (from Stories 2.5 + 2.4)

- **The schema was designed for you** (2.5 Dev Notes §6): flat header+entries, human-meaningful ids as failure labels, embedded content precisely so this gate needs no LFS and no EOL paranoia. Consume it as-is.
- **The harness was designed for you** (2.4): name-locked `round_trip_subset`, directory-driven "on purpose: Stories 2.5/2.6 extend/repoint... without touching this test", diagnostics already built. Your edit is additive iteration, not redesign.
- **Orchestrator execution reality** (2.5): this pipeline's impl environment denies network and arbitrary process execution but allows the full cargo suite (build/test/fmt/clippy/deny/audit) and file edits; git may be denied — if so, finish all file work and return the exact git commands (pre-authorized fallback, used by 2.5). cwd quirk: run cargo from the worktree root (gix discovery panics outside it).
- **Numbers that must not move:** workspace 122/0/11; extractor 65/65; cargo-audit 18 allowed warnings; deny ok/ok/ok; root-lockfile delta = the declared dev-dep edge only.
- **Process patterns that worked:** RED-first where honest; variance-recording instead of spec-editing; STOP-and-ask on any deny/audit/lockfile surprise; pre-seeded deferred-work stanza; exact counts in Completion Notes; no AI-credit lines anywhere.

### Git Intelligence Summary

`git log --oneline` at story-write: `edc568a` 2.5 review fixes ← `23caf32` 2.5 impl (closes #21) ← `699e90e` 2.5 story file ← `8a0bc92`/`e7c449b`/`5629312` (2.4 trio) ← `733a9f3`/`1b26a79`/`7f308d8` (2.3 trio) ← `2f93b5d` Merge PR #139 (2.2). Branch `story/2.6-l0-round-trip-ci-gate` stacked on completed 2.5; worktree clean at story-creation. Per-story commit pattern: story-file commit → impl commit → review-fixes commit. CI-touching precedents: Story 1.8 (`pr.yml` author), 1.16 (`feat(ci)` scope + Step 9.1 `--manifest-path` pattern), 1.17 (step-ordering note). A concurrent pipeline may work Story 2.8 in a separate worktree — disjoint paths (`crates/orgsidian-cli`), no overlap.

### Latest Technical Information

- **No new crates.** `serde_json` 1.x already resolved in root `Cargo.lock` (orgsidian-core consumer since Story 1.12/1.18); the parser dev-dep is an edge.
- **Swatinem/rust-cache@v2** `workspaces` input: newline-separated `path -> target` lines (target defaults to `target`); cache key includes lockfiles of registered workspaces. Semver-major pin `@v2` matches the existing usage ([[feedback_version_policy]]).
- **cargo-audit 0.22** (pinned in Step 5): `--file <path/to/Cargo.lock>` audits an arbitrary lockfile; combine with the existing `--deny warnings $IGNORES` expansion.
- **GitHub Actions** `timeout-minutes` is per-step here (test-design §7.3.13 uses exactly this); on timeout the step fails the job — that's the enforcement.
- **Runners:** `macos-14` = arm64 (M-series), `ubuntu-24.04` = current LTS image — already pinned in the matrix; both comfortably parse 2.33 MB of org in seconds (Dev Notes §3).

### References

- Source story: [`epics.md:844-857`](_bmad-output/planning-artifacts/epics.md#L844-L857) — user story + 4 ACs (invocation at :854). Upstream deps: [`epics.md:813-826`](_bmad-output/planning-artifacts/epics.md#L813) (2.4), [`epics.md:828-842`](_bmad-output/planning-artifacts/epics.md#L828) (2.5). Downstream: [`epics.md:859-871`](_bmad-output/planning-artifacts/epics.md#L859) (2.7 nightly — untouched here).
- Architecture: LD-32 gate split + <60s budget + atrophy rationale [`architecture.md:521-528`](_bmad-output/planning-artifacts/architecture.md#L521); LD-44 [`architecture.md:1228-1245`](_bmad-output/planning-artifacts/architecture.md#L1228); FR-2 mapping row (serializer + tests + "CI gate (LD-32)") [`architecture.md:1045`](_bmad-output/planning-artifacts/architecture.md#L1045); `subset-pr.json` structure row [`architecture.md:994`](_bmad-output/planning-artifacts/architecture.md#L994).
- Test design: L0 row [`test-design.md` §6.10/§6.11]; §7.3.13 CI-gate scaffold (the `timeout-minutes: 1` step, verbatim); TC-3; R-017/R-025.
- Previous stories: [`2-5-build-tools-corpus-extractor-fixture-governance.md`](_bmad-output/implementation-artifacts/2-5-build-tools-corpus-extractor-fixture-governance.md) (schema contract §6, orchestrator record, environment facts), [`2-4-implement-round-trip-faithful-serializer.md`](_bmad-output/implementation-artifacts/2-4-implement-round-trip-faithful-serializer.md) (harness design, name contract §4).
- Real code this story modifies: [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs), [`crates/orgsidian-parser/Cargo.toml`](crates/orgsidian-parser/Cargo.toml), [`.github/workflows/pr.yml`](.github/workflows/pr.yml), [`.gitattributes`](.gitattributes), [`CONTRIBUTING.md`](CONTRIBUTING.md) §1/§5, [`fixtures/fixtures.toml`](fixtures/fixtures.toml).
- Real code consumed read-only: [`fixtures/subset-pr.json`](fixtures/subset-pr.json), [`tools/corpus-extractor/tests/round_trip_preflight.rs`](tools/corpus-extractor/tests/round_trip_preflight.rs) + [`tests/matrix_coverage.rs`](tools/corpus-extractor/tests/matrix_coverage.rs), [`.cargo/audit-ignore.txt`](.cargo/audit-ignore.txt).
- Deferred-work items consumed: [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) story-2.5 stanza (all three 2.6-assigned items); story-1.17 stanza (Step 14→16 ordering note — context only).
- Memory anchors: [[project_orgsidian_github_plan]] (Free plan: advisory checks, LFS quota), [[project_orgsidian_github_label_scheme]] (issue label flips at later pipeline steps), [[feedback_version_policy]], [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]].

### Project Context Reference

- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-32 (gate matrix), LD-37 (--locked/supply-chain), LD-41 (analyze is total), LD-44/LD-45 (corpus/oracle), LD-48 (grammar READ-ONLY).
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (2.1 → 2.8); Process Discipline rules.
- [`test-design.md`](_bmad-output/test-artifacts/test-design.md) — §6.10 three-level oracle, §7.3.13 scaffold, TC-3, risk register.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §1 CI parity, §2 commit conventions, §3 `--manifest-path` precedent, §5 fixture governance + LFS state, §7 testing strategy.
- [`docs/adr/0001-corpus-subset-selection.md`](docs/adr/0001-corpus-subset-selection.md), [`fixtures/README.md`](fixtures/README.md), [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md).

## Dev Agent Record

### Agent Model Used

claude-fable-5 (Fable 5) — BMad dev-story pipeline, fully autonomous orchestrator run, 2026-06-11.

### Debug Log References

- RED run (T1): exactly-100 assert deliberately set to 101 → fired with the actionable message (`left: 100, right: 101`, names the manifest path + CONTRIBUTING §5 regeneration pointer). Note: the AC's alternative RED path (in-memory content tamper) cannot fire by design — Story 2.4's arbitrary-input identity property means ANY string round-trips, so the count-assert RED is the honest falsifiability check.
- GREEN run: `cargo test -p orgsidian-parser round_trip_subset --locked -- --test-threads=4` → 1 passed; test body **1.66s** (100 embedded entries + 18 interim files, debug profile); ~3.0s wall warm. Substring filter matches exactly 1 test fn (verified via `--list`).
- First `--locked` run failed until the lockfile registered the dev-dep edge: resolved offline via `cargo metadata --offline` — Cargo.lock delta is exactly 1 line (`serde_json` added to orgsidian-parser's dep list), zero new crates.
- AC4 bonus probe: `cargo deny --locked --manifest-path tools/corpus-extractor/Cargo.toml check all` → advisories ok, **bans FAILED, licenses FAILED** → bonus NOT taken per the pre-made scoping; license half re-deferred (story-2.6 deferred-work stanza).
- Scoped audit probe: `cargo audit --deny warnings $IGNORES --file tools/corpus-extractor/Cargo.lock` → exit 0, 97 crate dependencies scanned, zero NEW advisories (no STOP condition).
- `git lfs version` re-verified → "git: 'lfs' is not a git command" (expected path: documented re-deferral).
- pr.yml YAML validated via ruby YAML load + structural asserts (gate step directly after Step 9.1, `timeout-minutes: 1`, workspaces = `.` + both tool crates, audit step carries both invocations). PyYAML unavailable on this machine; ruby stdlib used instead.

### Completion Notes List

- **AC1** — `round_trip_subset` extended (not forked): one test fn, two sources. Source 1: `fixtures/subset-pr.json` via `manifest_path()` (`CARGO_MANIFEST_DIR/../..` hop, extractor-established pattern), `serde_json::Value` access, anti-placebo exactly-100 assert + non-empty `header` presence check, per-entry `id`-labeled `assert_round_trip` (2.4 diagnostics untouched), all maintainer-visible failures panic with actionable messages naming the file + the CONTRIBUTING §5 regeneration pointer. Source 2: the interim 18-file dir iteration kept byte-equivalent (≥10 floor assert intact). File doc-comment updated to present tense. `corpus_retains_byte_sensitive_fixtures`, inline, and proptest tests byte-unchanged (only `cargo fmt` rewrapped one new statement).
- **AC2** — pr.yml Step 9.2 `L0 round-trip subset gate (LD-32/LD-44, <60s)`: epic-verbatim invocation (epics.md:854) **plus `--locked`** (variance 1), `timeout-minutes: 1` (test-design §7.3.13 scaffold), placed right after Step 9.1, house-style comment block explaining the named-budgeted-contract rationale vs Step 9's workspace run. Slot-reservation `Story 2.6:` line annotated as landed. `nightly.yml` byte-untouched; no new top-level job (variance 2).
- **AC3** — Step 9.3 `cargo test (tools/corpus-extractor)` (`--manifest-path` + `--locked`, Step 9.1 precedent): first CI compilation of the extractor + its 65-test suite per PR (TC-3 matrix meta-test + round-trip preflight twin now fire per-PR). `Swatinem/rust-cache@v2` extended with `workspaces: .` + `tools/corpus-extractor` + `tools/issues-sync` — **also fixes the pre-existing uncached issues-sync build** (noted per AC3). No system-lib surprise locally; CI test runs are network-free by design (no fetch/extract added).
- **AC4** — Step 11 extended: second `cargo audit --deny warnings $IGNORES --file tools/corpus-extractor/Cargo.lock` invocation sharing the `.cargo/audit-ignore.txt` expansion (single source of truth). Green at zero new advisories — no exception needed, no STOP. License half: scoped `cargo deny` fails without allowlist edits → audit-only closure per the pre-made decision; re-deferred explicitly in the story-2.6 deferred-work stanza. `deny.toml` untouched.
- **AC5** — Measured (this machine, debug, warm build): gate test body **1.66s** (1.74s in the full-suite run) for all 100 embedded entries + 18 interim files; ~3.0s wall including cargo overhead — ~35x inside the 60s budget; interim-only baseline was 0.01s, story-creation preflight proxy 1.75s (confirmed). In-CI enforcement = `timeout-minutes: 1` from the first run. **PROCESS ITEM for the PR step:** PR body must ask the merger to confirm the gate step duration on BOTH matrix cells (macos-14 + ubuntu-24.04) in the first Actions run, and note the one-time extractor cold build (~2-4 min/cell) until the extended rust-cache warms.
- **AC6** — git-lfs re-verified absent → documented re-deferral (the expected path): deferred-work story-2.5 LFS item annotated (2.6 checked; new owner: first maintainer machine WITH git-lfs, after the Epic-2 story stack merges); `.gitattributes` marker re-pointed to `FOLLOWUP(LFS-migration)` with the no-history-rewrite caveat + the non-rewriting `git add --renormalize` alternative; CONTRIBUTING §5 current-state note no longer names Story 2.6 as owner. `git lfs migrate import` NOT run. Commented-out LFS stanza + active `-text` rules byte-unchanged; corpus files byte-untouched (no `[fixture:epic-2]` tag needed — no fixture content mutated).
- **AC7** — CONTRIBUTING §1 parity one-liner now includes both `--manifest-path` tool-suite invocations (issues-sync was a pre-existing gap, fixed in the same stroke as noted) + a parenthetical naming the dedicated gate step (L0 gate is parity-covered by `cargo test --workspace`). `fixtures/fixtures.toml` `parser.round-trip-interim` note made present-tense; `corpus.subset-pr` note verified accurate (no edit). **Deviation (honesty edit):** `fixtures/README.md` was expected zero-edit, but its LFS bullet referenced the old `FOLLOWUP(Story-2.6)` marker name — one line updated to the new marker name; consumers table rows verified accurate and unchanged. The `vault-corpus` note in fixtures.toml got the same marker-name honesty touch (notes edits are legal — hand-maintained file). No epics/architecture/test-design/PRD/KNOWN_DIVERGENCES edits (variance-recording instead).
- **AC8 (gates, exact counts)** — parser suite **77/0** (34 lib + 1 anchor + 4 grammar + 14 round_trip + 24 semantic; no count delta — existing fn extended); workspace **122 passed / 0 failed / 11 ignored** (baseline exact); extractor **65/0** via `--manifest-path`. `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all -- --check` clean; `cargo deny --locked check all` → advisories/bans/licenses/sources all ok; root `cargo audit` exit 0 at the baseline ignore list; scoped extractor audit exit 0. pr.yml parses (ruby YAML) with structure asserted. Sentinel sweep: `git status` shows ONLY the declared files; nothing under `crates/orgsidian-parser/src/`, test sentinels, `grammar/`, `build.rs`, `tools/**` sources, `fixtures/*.json`, `vault-corpus/**`, `nightly.yml`, `deny.toml`, `.cargo/audit-ignore.txt`, `docs/security/**`. Root `Cargo.lock` delta = the single `serde_json` dev-dep edge, zero new crates. deferred-work.md: three story-2.5 items annotated + story-2.6 stanza pre-seeded (LFS re-deferral, license-scan half, first-CI-run timing, dtolnay floating-ref cross-ref to story-1.16 — not fixed here).
- **Variances recorded (Dev Notes §8, no spec edits):** (1) epic invocation gains `--locked` (LD-37 house rule); (2) macOS-arm64+Ubuntu-LTS satisfied by the existing matrix job, no new job; (3) <60s enforced via `timeout-minutes: 1` + local evidence, CI numbers confirmable only on first Actions run (process note); (4) harness extends rather than replaces the interim corpus (both sources under the contractual name); (5) deferred-item closures ride along per the 2.5 stanza pre-assignment.
- **Process note (orchestrator constraint):** GitHub issue #22 verified OPEN with `status:backlog`; this run is forbidden from editing issues/PRs — the `status:*` label flips (backlog → in-progress → review) and the PR `Closes #22` body line are owned by the pipeline's later steps per the project's customization (`on_complete` checklist deferred to the orchestrator).

### File List

- `crates/orgsidian-parser/Cargo.toml` — modified (serde_json `{ workspace = true }` dev-dep with Story-2.6 comment)
- `crates/orgsidian-parser/tests/round_trip.rs` — modified (manifest source in `round_trip_subset`, `manifest_path()` helper, doc-comment present tense)
- `Cargo.lock` — modified (1 line: serde_json dep edge on orgsidian-parser)
- `.github/workflows/pr.yml` — modified (Step 9.2 L0 gate, Step 9.3 extractor tests, rust-cache `workspaces`, Step 11 scoped audit, slot-comment annotation)
- `.gitattributes` — modified (FOLLOWUP marker re-pointed: new owner + no-rewrite caveat; active rules byte-unchanged)
- `CONTRIBUTING.md` — modified (§1 parity line + parenthetical; §5 LFS current-state note)
- `fixtures/fixtures.toml` — modified (notes only: round-trip-interim present tense; vault-corpus marker name)
- `fixtures/README.md` — modified (1 line: FOLLOWUP marker name reference — honesty edit, consumers table unchanged)
- `_bmad-output/implementation-artifacts/deferred-work.md` — modified (three story-2.5 items annotated; story-2.6 stanza pre-seeded)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — modified (2-6 status transitions)
- `_bmad-output/implementation-artifacts/2-6-light-up-l0-round-trip-ci-gate-per-pr-100-files-60s.md` — modified (this file: tasks, status, dev record)

## Change Log

- 2026-06-11 — Adversarial code review (3 layers) of commit 3834fc8: 2 patch findings (both doc-honesty one-liners: stale `FOLLOWUP(Story-2.6)` marker in `tests/fixtures/vault-corpus/README.md`; stale "future" flag in CONTRIBUTING §8) fixed in-review; 2 deferred (PR `Closes #22` body line + issue label flip → pipeline PR step; dtolnay floating ref → pre-existing story-1.16 item); 4 dismissed (incl. the `timeout-minutes: 1` recompile concern, empirically disproved). All gates re-verified green at baseline counts. Status: done.
- 2026-06-11 — Story implemented (all 10 tasks complete; gate live in pr.yml as Step 9.2 with timeout-minutes: 1; harness consumes fixtures/subset-pr.json + interim dir under the contractual test name; extractor CI step + scoped lockfile audit close the two pre-assigned deferred items; LFS followup re-deferred with documented owner + no-rewrite caveat; all gates green at baseline counts — parser 77, workspace 122/0/11, extractor 65; Cargo.lock delta = serde_json dev-dep edge only). Status: review.
- 2026-06-11 — Story created (ultimate context engine analysis completed — comprehensive developer guide created; gate runtime grounded empirically: 100-entry round-trip measured at 1.75s vs the 60s budget; LFS re-deferral and audit-only supply-chain scoping pre-decided from specs with variances recorded, not spec-edited). Status: ready-for-dev.
