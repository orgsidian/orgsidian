# Story 2.5: Build `tools/corpus-extractor` + fixture governance

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 21

## Story

As the **author / contributor**,
I want `tools/corpus-extractor/` extracting the L0 subset (~100 files) and the full nightly corpus (~2000 assertions) from `org-mode/testing/lisp/test-org-element.el`,
So that LD-44 subset criteria are enforceable and Murat's P1 fixture-governance discipline is in place.

**Traces:** LD-44 (subset selection criteria), LD-32 (per-PR vs nightly gate split), OD-1 (corpus extracted from `test-org-element.el`, reusable extractor), test-design.md §5 (fixture architecture, Murat P1), §2 item 6 (Epic-2 prerequisite), TC-3 (self-verifying subset matrix). Enables FR-2 enforcement (Stories 2.6/2.7); the FR-2 trace itself stays owned by `crates/orgsidian-parser/src/serializer.rs` — the extractor is a tool, **no `//! Implements FR-` header** (single trace owner per FR, CONTRIBUTING §4).

## Scope Fence (read first)

This is the **corpus + governance** story: it turns the Story 1.2-era stub at `tools/corpus-extractor/` into a real extractor, commits the generated corpus + manifests, and establishes fixture ownership. It is **not**:

- **NOT a parser-crate change.** `crates/orgsidian-parser/` stays byte-for-byte untouched: `tests/round_trip.rs` (incl. its interim 18-file corpus at `tests/fixtures/round_trip/`), `tests/{anchor,grammar,semantic}.rs`, `src/`, `build.rs`, `grammar/` (LD-48) are all sentinels. Repointing/extending the directory-driven `round_trip_subset` harness to consume `fixtures/subset-pr.json` is **Story 2.6's** explicitly-stated job (epics.md:854). The optional preflight check in AC8 consumes the parser as a dependency — it does not modify it.
- **NOT the CI gates (Stories 2.6/2.7).** No `.github/workflows/*` edits. The nightly placeholder comment at `nightly.yml:160` ("Story 2.7: nightly L0 full corpus + L2 Emacs oracle gates land here") stays. CI build/test coverage for the extractor itself is pre-seeded as deferred work (AC10).
- **NOT the L2 oracle (Story 2.7).** No `crates/orgsidian-parser/tests/canonical_ast/`, no Emacs invocation, no divergence triage.
- **NOT Epic-3+ fixtures.** No `synthetic-10k/25k/50k` vault generation (LD-42, epic-3-owned), no golden traces (epic-5), no UJ vaults. `fixtures.toml` declares only fixtures that exist after this story.
- **NOT a workspace-membership change.** `Cargo.toml` `exclude = ["tools/corpus-extractor", "tools/issues-sync"]` already exists (verified) — root `Cargo.toml`, `Cargo.lock`, `deny.toml` stay untouched. The extractor has its own standalone `Cargo.toml` + `Cargo.lock`.

Deliverables: the extractor (lib + bin, subcommands), `fixtures/subset-pr.json` + `fixtures/full-nightly.json` + `fixtures/fixtures.toml` (+ `fixtures/README.md`), `tests/fixtures/vault-corpus/` (git-LFS, + provenance README), `docs/adr/0001-corpus-subset-selection.md`, governance docs (CONTRIBUTING subsection + `.gitattributes` LFS stanza + `.gitignore` cache entry), extractor meta-tests, deferred-work annotations.

## Acceptance Criteria

### AC1 — Real standalone extractor replaces the stub (lib + bin, subcommand CLI).

**Given** the existing stub (`tools/corpus-extractor/Cargo.toml` `publish = false` + 3-line `main.rs`, already outside `[workspace.members]`),
**When** the extractor is implemented,
**Then**:

- `tools/corpus-extractor/` keeps package name `orgsidian-corpus-extractor`, `publish = false`, edition 2021, MIT, `version = "0.0.0"`. Follow the `tools/issues-sync` house pattern: explicit `[[bin]]` + `[lib]` targets so logic is unit-testable; CLI via `clap` derive; errors via `anyhow`.
- Subcommands: `fetch` (acquire + checksum-verify the upstream `.el` into a gitignored cache, AC2), `extract` (cached `.el` → corpus files + both JSON manifests, AC3/AC4), `verify` (run the LD-44 matrix validator against the **committed** `fixtures/subset-pr.json` — same code path the AC8 meta-test calls).
- Builds and tests via `cargo build --manifest-path tools/corpus-extractor/Cargo.toml --locked` / `cargo test --manifest-path tools/corpus-extractor/Cargo.toml --locked` (the documented issues-sync invocation style, CONTRIBUTING §3). Its own `Cargo.lock` is committed (LD-37 reproducibility convention).
- One concern per file (~400-line rule): suggested split `main.rs` (CLI shell) / `fetch.rs` (pin + download + sha256) / `elisp.rs` (assertion scanning) / `classify.rs` (construct detection) / `select.rs` (LD-44 algorithm) / `synth.rs` (size/edge-bucket synthesis) / `emit.rs` (deterministic JSON + corpus materialization) / `validate.rs` (matrix validator, shared by `verify` + meta-test).

### AC2 — Upstream acquisition is pinned-fetch, reproducible, and license-clean (decision — see Dev Notes §2).

**Given** the repo is MIT (LD-1) with a GPL-contagion posture (R-009, LD-37) and `test-org-element.el` is GPL-3.0-or-later elisp,
**When** the upstream source is acquired,
**Then**:

- The `.el` file is **fetched at extraction time, never vendored into the repo**. The pin is a triple hard-coded in the extractor source (one const module): org-mode **release tag** (latest stable at impl time — verify, do not guess), the **upstream URL** (canonical: `https://git.savannah.gnu.org/cgit/emacs/org-mode.git/plain/testing/lisp/test-org-element.el?id=<tag>`; document the GitHub mirror `bzg/org-mode` raw URL as fallback), and the file's **SHA-256**. `fetch` fails hard on checksum mismatch.
- Downloads land in `tools/corpus-extractor/cache/` — added to root `.gitignore` (one scoped line + comment, mirroring its existing style). `extract` reads only the cache and refuses to run with a missing/mismatched cache (pointing the user at `fetch`). Extraction is a **maintainer operation**, not a per-PR/CI step — no network in any test.
- Provenance is embedded: both emitted JSONs carry a header object (org-mode tag, source SHA-256, extractor version) and `tests/fixtures/vault-corpus/README.md` states the corpus is derived from the GNU org-mode test suite (GPL-3.0-or-later), is **test data only**, and is never linked into or distributed with MIT binaries. The ADR (AC7) records the licensing rationale.

### AC3 — Extraction + LD-44 selection algorithm (subset ~100 files; full ~2000 assertions; deterministic).

**Given** a checksum-verified `test-org-element.el` in cache,
**When** `extract` runs,
**Then**:

- **Assertion harvesting:** scan for `ert-deftest test-org-element/...` blocks and harvest the org-text string literals (primarily `org-test-with-temp-text` / `org-test-with-temp-text-in-file` arguments). A pragmatic scanner is mandated — handle elisp string escapes (`\"`, `\\`, `\n`, `\t`) and **strip `<point>` markers** (the org-test caret convention); a full elisp reader is explicitly out of scope. Record harvested-assertion count; the epic's "~2000" is a target, the real count is whatever the pinned file yields — assert a floor (AC8) from the observed number, report exact counts in Completion Notes.
- **Full corpus:** every harvested snippet becomes a member of the full nightly corpus, each tagged with its `ert-deftest` name (provenance) and detected constructs.
- **L0 subset (LD-44, architecture.md:1228-1245 — the authoritative matrix):** exactly **30 small (<1KB) + 50 medium (1-50KB) + 20 large (>50KB)** files; every construct in the 15-row LD-44 table (the same enumeration as Story 2.3's AC6) appears **≥3 times** across the subset; edge-case bucket **≥5 Unicode/RTL, ≥5 unusual-EOL (CRLF, mixed), ≥5 malformed-but-valid** (over-indented property drawers, trailing whitespace in headlines). Interpretation (record in ADR): edge-case files are members of the 100 and count inside their size buckets.
- **Synthesis where extraction can't reach:** `.el` snippets are nearly all <1KB, so medium/large files are **deterministically composed** from harvested snippets (e.g. seeded concatenation under generated headings); edge-case files are deterministic transforms (CRLF re-encoding of clean members, RTL/Unicode salting). Every synthesized file records its recipe (source snippet ids + transform) in the manifest. No hand-authored org text — harvested material plus mechanical transforms only.
- **Determinism is a hard requirement:** same pin + same extractor code ⇒ byte-identical outputs. Sorted/`BTreeMap` iteration everywhere, fixed seed, no timestamps beyond the pin header. Running `extract` twice and diffing must be clean (make this a test or document the manual check in Completion Notes).

### AC4 — Emission: two manifests at root `fixtures/` + materialized corpus at `tests/fixtures/vault-corpus/`.

**Given** the selected subset + full corpus,
**When** outputs are written,
**Then**:

- [`fixtures/subset-pr.json`](fixtures/subset-pr.json) is **self-contained**: per entry — id, **embedded org content** (JSON string), size bucket, detected constructs, edge-bucket membership, provenance (deftest name or synthesis recipe), byte length. Rationale (record in ADR + Dev Notes §5): the Story 2.6 per-PR gate must work on a checkout **without git-LFS** (LFS-free PR CI = no GitHub-Free LFS bandwidth burn, [[project_orgsidian_github_plan]]), and JSON escaping makes embedded CRLF/trailing-whitespace bytes immune to EOL mangling.
- [`fixtures/full-nightly.json`](fixtures/full-nightly.json) is a **pointer manifest** (architecture.md:995 calls it exactly that): per assertion — id, deftest provenance, constructs, and the **relative path** of its materialized file under `tests/fixtures/vault-corpus/`. No embedded content.
- `tests/fixtures/vault-corpus/` holds the materialized `.org` files for the **full corpus and the subset** (subset files exist both embedded and on disk — disk is for nightly/L2/tooling reuse; embedded is the PR-gate diet), organized deterministically (e.g. `extracted/NNNN_<slug>.org`, `synthesized/NNN_<bucket>_<slug>.org` — dev's choice, sorted and stable).
- Both JSONs are committed as **regular git files** (NOT LFS); only `tests/fixtures/vault-corpus/` goes through LFS (AC6). This follows the epic AC text verbatim; it varies from test-design.md §5.2's example paths and §5.3 rule 3 ("JSONs as LFS blobs") — record as variance, do not edit test-design.md (Dev Notes §5 has the reconciliation table).
- Total committed corpus size stays modest (target: low single-digit MB) — GitHub Free LFS quota is ~1GiB storage/bandwidth per month; report the final sizes in Completion Notes.

### AC5 — `fixtures/fixtures.toml` per-epic ownership + governance documentation (Murat P1).

**Given** the emitted fixtures and the already-existing fixture sets,
**When** governance is written,
**Then**:

- [`fixtures/fixtures.toml`](fixtures/fixtures.toml) (hand-maintained, NOT generated) declares **every fixture set that exists after this story**, each with `path`, `owner = "epic-N"`, optional `regenerated_by` / `ld_reference` / `notes` — schema per test-design.md §5.2 with corrected real paths. Minimum entries: `corpus.subset-pr` (epic-2, `regenerated_by = "tools/corpus-extractor"`), `corpus.full-nightly` (epic-2, same), `corpus.vault-corpus` (epic-2, same), parser crate-local fixtures (`anchor.org` epic-1; `semantic.rs` samples + `round_trip/` interim corpus epic-2), `tests/perf-baselines/` (epic-1). Future fixtures (golden traces, synthetic vaults, canonical ASTs, UJ vaults) are added by their owning stories — do not pre-declare paths that don't exist.
- Governance rules documented (new CONTRIBUTING subsection extending §5 "Fixture placement rule"): every fixture owned by exactly one epic; **mutation requires PR review** with the owning epic named — on GitHub Free branch protection is unenforceable ([[project_orgsidian_github_plan]]), so the mechanism is the documented convention: commit-message tag `[fixture:epic-N]` (test-design §5.3 rule 2) + maintainer pre-merge check, same advisory posture as the commitlint gates (CONTRIBUTING §2). Generated fixtures are never hand-edited; regeneration PRs must quote the generator invocation + pin.
- [`fixtures/README.md`](fixtures/README.md) — this story creates root `fixtures/` for the first time; per CONTRIBUTING §5 the promotion README names the consumers (parser `round_trip_subset` via Story 2.6, nightly full-corpus via Story 2.7, extractor `verify`/meta-test).

### AC6 — `tests/fixtures/vault-corpus/` versioned via git-LFS, with graceful degradation.

**Given** git-LFS may be absent on any given machine,
**When** LFS versioning is wired,
**Then**:

- Root `.gitattributes` gains a scoped stanza (mirroring the file's existing comment style): `tests/fixtures/vault-corpus/**/*.org filter=lfs diff=lfs merge=lfs -text` (the `-text` rides along — corpus bytes are EOL-sensitive by design). README/`*.md` files inside vault-corpus stay regular git (exclude them from the LFS pattern).
- CONTRIBUTING gains setup docs (in the AC5 subsection): one-time `git lfs install`, `git lfs pull` to materialize, and the explicit statement that **the per-PR workflow does not require LFS** (subset is embedded in `fixtures/subset-pr.json`; only nightly/L2 work and corpus regeneration need the smudged files).
- The extractor degrades gracefully: `extract`/`verify` detect LFS **pointer files** (content starting `version https://git-lfs.github.com/spec/v1`) when they need real corpus bytes and fail with an actionable message naming the setup steps — never a confusing parse error. Nothing in `cargo test --workspace` reads vault-corpus in this story.
- **Impl-time fallback (decision — don't block):** check `git lfs version` first. If git-lfs is genuinely unavailable in the impl environment and cannot be installed, commit the corpus as regular git files WITH the `.gitattributes` stanza left in place but commented out + a `FOLLOWUP(Story-2.6)` marker, record the deviation in Completion Notes, and add a deferred-work item to run `git lfs migrate import` before the corpus grows. A small raw corpus is recoverable; blocking the story is not. (If LFS works — the expected case — none of this applies.)

### AC7 — ADR 0001 documents the algorithm.

**Given** the selection algorithm exists,
**When** [`docs/adr/0001-corpus-subset-selection.md`](docs/adr/0001-corpus-subset-selection.md) is written (NEW dir `docs/adr/` — first ADR; use the conventional Status/Context/Decision/Consequences shape),
**Then** it documents: the pin triple + fetch flow; the harvesting scanner (and its known limits — what elisp forms it does not read); construct classification; the LD-44 selection algorithm (matrix-greedy fill, bucket targets, edge-bucket interpretation as members of the 100); deterministic synthesis recipes; why `subset-pr.json` embeds content while `full-nightly.json` points (LFS-free PR gate, EOL safety); the licensing/provenance posture (AC2); and the regeneration procedure (`fetch` → `extract` → `verify` → PR with `[fixture:epic-2]`).

### AC8 — Self-verifying subset (TC-3 meta-test) + parser preflight.

**Given** test-design TC-3 ("a meta-test inside `tools/corpus-extractor/` itself ... fails if any construct count falls below threshold" — explicitly assigned to Story 2.5),
**When** the extractor test surface is written,
**Then**:

- `tools/corpus-extractor/tests/matrix_coverage.rs` (NEW) loads the **committed** `fixtures/subset-pr.json` (path-relative via `CARGO_MANIFEST_DIR/../..`) and asserts: every LD-44 construct ≥3 occurrences; bucket counts exactly 30/50/20; edge minimums ≥5/≥5/≥5; assertion-count floor for the full manifest (set from the observed harvest, anti-placebo — a floor of 0 is a placebo). It must consume the same `validate.rs` the `verify` subcommand uses (one validator, two entry points).
- Unit tests cover the elisp scanner (escapes, `<point>` stripping, a multi-line snippet) and classifier (one positive case per LD-44 construct — reuse syntax shapes from `crates/orgsidian-parser/tests/semantic.rs`; don't invent novel org syntax).
- **Parser preflight (strongly recommended, cheap insurance):** `tools/corpus-extractor/tests/round_trip_preflight.rs` with `orgsidian-parser = { path = "../../crates/orgsidian-parser" }` as a **dev-dependency** (legal: the extractor is outside the workspace; the dep pulls tree-sitter/cc/chrono into the tool's own lockfile only) asserting `serialize_document(&analyze(content)?) == content` for every subset entry. Story 2.4's arbitrary-input identity property makes failure near-impossible — which is exactly why a red preflight would be decision-grade information *before* Story 2.6 wires the PR gate. If the path dep causes real trouble (e.g. submodule/cc issues in the tool context), drop it, note why, and let 2.6's gate be the first verification.

### AC9 — Workspace gates stay green; sentinels untouched.

**Given** all the above,
**When** the gates run,
**Then**:

- Workspace untouched ⇒ `cargo build --workspace --locked`, `cargo test --workspace --locked` (baseline: **122 passed / 0 failed / 11 ignored** post-2.4), `cargo clippy --workspace --all-targets --locked`, `cargo fmt --check`, `cargo deny check licenses bans advisories` (ok/ok/ok), `cargo audit` (18-allowed-warnings baseline) — all green with **zero root `Cargo.lock` delta**. Anything that forces a root lockfile/deny change: STOP, decision-grade question.
- Extractor gates: `cargo test --manifest-path tools/corpus-extractor/Cargo.toml --locked` green; `cargo fmt --check` + clippy clean for the tool too (run with `--manifest-path`); no `unwrap()`/`expect()` in extractor library code (binary `main` + tests may); `println!` allowed ONLY in the CLI output layer (it's a CLI tool — keep it out of lib logic).
- Sentinel check via `git status`: nothing under `crates/`, `.github/workflows/`, root `Cargo.toml`/`Cargo.lock`/`deny.toml` modified.
- New extractor dependencies: keep to the boring set (suggested: `clap` 4 derive, `anyhow` 1, `serde`+`serde_json` 1, `sha2` 0.10, `ureq` (blocking, small — for `fetch`), optional `regex` 1; `pretty_assertions` dev). Latest stable verified at impl time per [[feedback_version_policy]]; all MIT/Apache-2.0-compatible. The standalone lockfile is outside `cargo deny`'s root scope — note this in the ADR's supply-chain paragraph (deferred CI hardening, AC10).

### AC10 — Deferred-work hygiene.

**Given** [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md),
**When** this story completes,
**Then**:

- The story-2.4 stanza's **"Corpus expansion to the LD-44 ~100-file subset"** item is annotated: corpus + manifests delivered by 2.5; harness repointing remains with Story 2.6.
- A `## Deferred from: code review of story-2.5 (YYYY-MM-DD)` stanza is pre-seeded at impl time. Known candidates: CI build/test step for `tools/corpus-extractor` (mirror the issues-sync precedent; owner Story 2.6's `pr.yml` edit or a CI-hardening pass); supply-chain scanning of the extractor's standalone lockfile; LFS migration follow-up if the AC6 fallback fired; L2-subset designation within the corpus (Story 2.7 picks its oracle files); org-mode pin-bump cadence (pairs with the LD-48 parser-owner review).

## Tasks / Subtasks

- [ ] **T1** — Scaffold the real crate: lib+bin split per issues-sync pattern, clap subcommands (`fetch`/`extract`/`verify`), deps added to the standalone manifest, own `Cargo.lock` regenerated and committed. (AC1, AC9)
- [ ] **T2** — `fetch`: pin-const module (tag + URL + SHA-256 — resolve the latest stable org-mode release tag at impl time), download to `tools/corpus-extractor/cache/`, sha256 verification, `.gitignore` cache entry. (AC2)
- [ ] **T3** — `elisp.rs` scanner: `ert-deftest` blocks → org-text literals; escape handling; `<point>` stripping; unit tests RED-first against hand-built `.el` snippets. (AC3, AC8)
- [ ] **T4** — `classify.rs`: LD-44 15-construct detection over org text (shapes borrowed from `tests/semantic.rs`); unit test per construct. (AC3, AC8)
- [ ] **T5** — `select.rs` + `synth.rs`: matrix-greedy subset fill, 30/50/20 buckets, deterministic medium/large composition + edge-bucket transforms (CRLF/Unicode-RTL/malformed-valid), recipes recorded. (AC3)
- [ ] **T6** — `emit.rs`: deterministic `subset-pr.json` (embedded content + provenance header), `full-nightly.json` (pointer manifest), materialized `tests/fixtures/vault-corpus/` tree; double-run determinism check. (AC3, AC4)
- [ ] **T7** — Run the pipeline for real: `fetch` → `extract`; commit outputs; record harvest/subset/size numbers for Completion Notes. (AC3, AC4)
- [ ] **T8** — git-LFS: check `git lfs version`; `.gitattributes` stanza; track + commit vault-corpus through LFS (or execute the documented AC6 fallback); pointer-file detection in `extract`/`verify`. (AC6)
- [ ] **T9** — Governance: `fixtures/fixtures.toml` (all existing fixture sets), `fixtures/README.md` (promotion rule consumers), `tests/fixtures/vault-corpus/README.md` (provenance/licensing), CONTRIBUTING fixture-governance + LFS-setup subsection. (AC2, AC5, AC6)
- [ ] **T10** — `docs/adr/0001-corpus-subset-selection.md`. (AC7)
- [ ] **T11** — `validate.rs` + `tests/matrix_coverage.rs` (committed-artifact meta-test, shared validator) + `tests/round_trip_preflight.rs` (parser path dev-dep) + `verify` subcommand wiring. (AC8)
- [ ] **T12** — Gates: full workspace suite (zero delta) + extractor suite + fmt/clippy both scopes + deny/audit baseline + `git status` sentinel sweep. (AC9)
- [ ] **T13** — deferred-work.md: annotate 2.4 corpus-expansion item; pre-seed story-2.5 stanza. (AC10)
- [ ] **T14** — Commit. Suggested title: `feat(corpus-extractor): extract LD-44 corpus + establish fixture governance (Story 2.5, closes #21)` (scope enum is advisory, CONTRIBUTING §2). **NO** Co-Authored-By trailer, **NO** "Generated with Claude Code" footer, no AI-credit lines. PR body (when the pipeline's PR step runs): corpus numbers, determinism statement, LFS posture, variance list. (process)

## Dev Notes

### 1. Current state (verified at story-creation, branch `story/2.5-corpus-extractor-fixture-governance` @ `8a0bc92`)

- `tools/corpus-extractor/` exists as a Story 1.2-era stub: `Cargo.toml` (name `orgsidian-corpus-extractor`, `publish = false`, comment "Real deps land in Story 2.5"), `Cargo.lock`, 3-line `main.rs`. Root `Cargo.toml:18` already excludes it. **You are filling in a reserved slot, not creating a project** — keep the package identity.
- Root `fixtures/` and `tests/fixtures/` do **not** exist yet (verified) — this story creates both (first promotion per CONTRIBUTING §5). Root `tests/` does exist (LD-41 harness files + `perf-baselines/`) — `tests/fixtures/vault-corpus/` nests cleanly.
- `docs/adr/` does not exist — first ADR.
- The Story 2.4 harness (`crates/orgsidian-parser/tests/round_trip.rs`) is **directory-driven on purpose** over its interim corpus, with the comment "Stories 2.5/2.6 extend/repoint the corpus without touching this test". Its 18 interim fixtures protected by the `.gitattributes` `-text` rule stay exactly where they are.
- `git lfs` availability in the impl environment is **unverified** (the check was not runnable at story-creation time) — T8 starts with `git lfs version` and the AC6 fallback is pre-authorized.

### 2. Upstream acquisition: pinned fetch, not vendored snapshot (decision — do not relitigate)

Three reasons, in priority order:

1. **License hygiene.** `test-org-element.el` is GPL-3.0-or-later. The repo is MIT (LD-1) with an explicit GPL-contagion risk posture (R-009; LD-37 license allowlist). Vendoring ~10k lines of GPL elisp into an MIT repo is exactly the contamination the gates exist to catch — except `cargo deny` only sees Cargo deps, so it would sit invisible. Keep it out of tree.
2. **Supply-chain posture.** The project's pattern for upstream code is SHA-pinning with human-reviewed bumps (LD-48 submodule; exact-pin Tauri, LD-47). A tag+SHA-256-pinned fetch is the data-file equivalent: reproducible, auditable, bump-by-PR.
3. **The corpus is the artifact.** The committed outputs (JSONs + vault-corpus) are what CI consumes; the `.el` is build-time input for a maintainer operation. There is no runtime or CI need for the file itself.

The **extracted snippets** do get committed — they are short org-syntax examples carried with provenance attribution (AC2's README + JSON headers + ADR). Test-data-only, never compiled into or shipped with the MIT binaries. This is the documented, defensible posture; the ADR is where it lives.

### 3. What's inside `test-org-element.el` (extraction mechanics)

- Thousands of `(ert-deftest test-org-element/<name> ... )` forms, most exercising `org-test-with-temp-text "<org snippet>"` (and the `-in-file` variant). The snippet strings carry elisp escapes and frequently a `<point>` marker (caret position convention) that **must be stripped** or it pollutes the corpus with literal `<point>` text.
- The epic's "~2000 assertions" is an estimate of this population. Do not hard-code 2000 anywhere; harvest, count, report, and set the meta-test floor from observation (e.g. 75% of observed — your call, justify in the ADR).
- A pragmatic line/state scanner is the mandated approach: find `org-test-with-temp-text`, lex the following string literal with escape handling (track `\"` and `\\`; convert `\n`/`\t`), no full sexp reader. Document unharvested forms (snippets built by `concat`/`format`, non-literal arguments) as known limits in the ADR — coverage of the *literal* population is what LD-44 needs.
- Some snippets are deliberately malformed org — that's a **feature** for the full corpus (the parser is total per LD-41, and Story 2.4's serializer round-trips ERROR trees byte-faithfully via gap-absorption), and a candidate source for the malformed-valid edge bucket.

### 4. LD-44 selection + synthesis (the algorithm the ADR documents)

- Matrix first: greedy-fill construct coverage (≥3 each across all 15 constructs) from harvested snippets, preferring multi-construct files for economy.
- Buckets second: harvested snippets are nearly all <1KB → the small bucket is naturally extracted; mediums/larges are composed deterministically (seeded selection of N snippets joined under generated `* Section k` headings until the target byte band is hit). Composition output is org-valid by construction (concatenation of org fragments with headline separators).
- Edge bucket third, as members of the 100 (30+50+20 = 100 leaves no room for extras — record this interpretation): CRLF/mixed-EOL via mechanical re-encoding of clean members; Unicode/RTL via members carrying Arabic/Hebrew/CJK content (harvest first — the upstream suite has some — synthesize the gap); malformed-valid via over-indented drawers / trailing-whitespace headlines (same shapes as the interim corpus files 13-15).
- Determinism: sorted inputs, `BTreeMap`, fixed literal seed, stable ids. The double-extract diff check (T6) is the enforcement.

### 5. Path reconciliation (three documents, one answer)

| Artifact | epics.md AC (sync-source — **wins**) | architecture.md | test-design.md §5 |
|---|---|---|---|
| subset manifest | `fixtures/subset-pr.json` | `fixtures/subset-pr.json` | `tests/fixtures/vault-corpus/subset-pr.json` |
| nightly manifest | `fixtures/full-nightly.json` | `fixtures/full-nightly.json` ("pointer to full corpus") | `tests/fixtures/vault-corpus/full-nightly.json` |
| ownership decl | `fixtures/fixtures.toml` | — | `fixtures/fixtures.toml` |
| corpus files | `tests/fixtures/vault-corpus/` (git-LFS) | — | `tests/fixtures/vault-corpus/` (git-LFS) |

Follow epics.md verbatim (it is also what Story 2.6's AC consumes: "`round_trip_subset` ... consuming `fixtures/subset-pr.json`"). Record the test-design §5.2 path variance and the §5.3-rule-3 variance (JSONs as regular git, not LFS blobs — rationale: the PR gate must not depend on LFS smudging or burn GitHub-Free LFS bandwidth on every PR checkout; pointer-vs-embedded split per architecture.md:994-995). Do **not** edit epics.md / architecture.md / test-design.md.

### 6. Story 2.6 consumer contract (design for it, don't build it)

- 2.6 wires `cargo test -p orgsidian-parser round_trip_subset -- --test-threads=4` consuming `fixtures/subset-pr.json` on macOS-arm64 + Ubuntu-LTS per PR, <60s budget. Your JSON schema is therefore a **public contract**: flat, obvious, serde-friendly (top-level header object + `entries: [...]`), embedded content as plain JSON strings. Keep it boring; document it in the ADR. ~100 files of mostly-small content parses in negligible time — the 60s budget is parser-side, not JSON-side.
- The harness will iterate manifest entries instead of (or in addition to) a fixture directory — your `id` field becomes its failure label, so make ids human-meaningful (`extracted/0042_headline_todo`, not bare integers).
- Subset files ALSO exist materialized under vault-corpus (AC4) so 2.7's nightly + future tooling reuse the same bytes; byte-equality between the embedded copy and the LFS file is enforced by determinism (same emission pass) — the preflight test may cheaply assert a few of them when LFS content is present (skip silently on pointer files).

### 7. Variances (record in Completion Notes; no spec edits)

1. test-design.md §5.2 example paths place both JSONs inside vault-corpus → root `fixtures/` per epics.md AC (sync-source) + architecture.md structure (Dev Notes §5).
2. test-design.md §5.3 rule 3 says generated JSONs committed "as binary blobs via git-LFS" → regular git files; only vault-corpus is LFS (epic AC names only vault-corpus for LFS; PR-gate LFS-independence + Free-plan quota rationale).
3. Epic "~2000 assertions" → observed harvest count from the pinned file, floor-asserted (anti-placebo), exact number reported.
4. PR-review mutation policy (epic AC) → documented-convention enforcement (`[fixture:epic-N]` tag + maintainer pre-merge check); GitHub Free cannot enforce branch protection ([[project_orgsidian_github_plan]]) — same advisory posture as every other gate of this kind in the repo.
5. fixtures.toml declares existing fixture sets only; test-design §5.2's future entries (synthetic vaults, golden traces, canonical ASTs, UJ vaults) land with their owning stories.

### Project Structure Notes

**Alignment with unified project structure:**

- `tools/corpus-extractor/{Cargo.toml,Cargo.lock,src/main.rs}` — UPDATE (stub → real; keep identity). `src/{fetch,elisp,classify,select,synth,emit,validate,lib}.rs`, `tests/{matrix_coverage,round_trip_preflight}.rs` — NEW. Matches architecture.md:996-999 + test-design.md §5.1 (`validator` + `matrix_coverage` named there explicitly). ✓
- `fixtures/{subset-pr.json,full-nightly.json,fixtures.toml,README.md}` — NEW (creates root `fixtures/`; CONTRIBUTING §5 promotion README required). ✓
- `tests/fixtures/vault-corpus/**` + `README.md` — NEW (creates `tests/fixtures/`; LFS-tracked except markdown). ✓
- `docs/adr/0001-corpus-subset-selection.md` — NEW (creates `docs/adr/`; architecture.md:1396 names this exact path). ✓
- `.gitattributes` — UPDATE (LFS stanza, mirror existing comment style). `.gitignore` — UPDATE (extractor cache dir). `CONTRIBUTING.md` — UPDATE (fixture governance + LFS setup subsection extending §5). ✓
- READ-ONLY / MUST NOT CHANGE: everything under `crates/` (incl. the entire parser crate + interim round_trip corpus), `.github/workflows/*`, root `Cargo.toml`/`Cargo.lock`, `deny.toml`, `docs/parser/KNOWN_DIVERGENCES.md`, `_bmad-output/planning-artifacts/*`, `_bmad-output/test-artifacts/*`. ✓

### Testing Standards Summary

- Extractor: unit tests in-module + integration tests in `tools/corpus-extractor/tests/`, run via `--manifest-path`. Every test asserts a real property (Story 1.9 anti-placebo discipline) — the matrix meta-test runs against the **committed** artifact, not an in-memory regeneration, so artifact rot is what it catches. RED-first where it's honest (scanner/classifier against hand-built inputs).
- Workspace: zero test-count delta expected (baseline 122 passed / 11 ignored). The parser suite (77 tests) must pass byte-untouched.
- No CI changes; the extractor's CI integration is a named deferred item (AC10).

### Previous Story Intelligence (from Story 2.4)

- **The harness is waiting for you:** `round_trip_subset` is directory-driven with divergence diagnostics (filename + byte offset + context window); 2.6 repoints it at your JSON. Your only obligation to it is a sane, stable schema (Dev Notes §6).
- **Round-trip safety net:** 2.4 proved `serialize_document(&analyze(s)?) == s` for **arbitrary strings** (proptest, 256 cases, incl. ERROR-region pathologies) — so corpus content cannot make the L0 gate fail for parser reasons; only I/O/encoding bugs in your emission could. That's what the preflight buys you certainty on.
- **EOL discipline:** byte-sensitive fixtures need protection from autocrlf and editor strippers — 2.4 used a scoped `.gitattributes -text` rule + a tripwire test. Your equivalents: LFS (`-text` included) for files, JSON escaping for embedded content.
- **Process patterns that worked:** variance-recording instead of spec-editing; STOP-and-ask on any root lockfile/deny delta; pre-seeded deferred-work stanza; exact counts in Completion Notes; scratch probes deleted before commit.
- **Hygiene:** no AI-credit lines anywhere; cargo-audit baseline 18 allowed warnings — must not move (it won't: zero workspace dep changes).

### Git Intelligence Summary

`git log --oneline` at story-write: `8a0bc92` 2.4 review fixes ← `e7c449b` 2.4 impl (closes #20) ← `5629312` 2.4 story file ← `733a9f3`/`1b26a79`/`7f308d8` (2.3 trio) ← `2f93b5d` Merge PR #139 (2.2). Branch `story/2.5-corpus-extractor-fixture-governance` is **stacked on the completed 2.4 branch** (2.3+2.4 commits local to this lineage; PR base may need their merges first — flag in the PR description). Per-story commit pattern: one story-file commit + one impl commit + one review-fixes commit. `tools/` precedent: Story 1.16 (`feat(ci): wire LD-55 GitHub Issues sync via tools/issues-sync`, scopes `issues-sync`/`ci` used in follow-ups). GitHub issue **#21** verified OPEN: "[Story 2.5] Build `tools/corpus-extractor` + fixture governance". A concurrent pipeline works Story 2.8 in a separate worktree — disjoint paths (`crates/orgsidian-cli`), no overlap with this story's files.

### Latest Technical Information

- Suggested extractor deps (standalone lockfile only; verify latest stable at impl time per [[feedback_version_policy]]): `clap` 4 (derive — issues-sync precedent), `anyhow` 1, `serde` 1 + `serde_json` 1, `sha2` 0.10, `ureq` (small blocking HTTP for `fetch`; avoid pulling tokio for one GET — `reqwest` only if `ureq`'s TLS story is a problem in practice), `regex` 1 if the scanner wants it, `pretty_assertions` dev. All MIT/Apache-2.0-family.
- org-mode upstream: canonical repo `git.savannah.gnu.org/cgit/emacs/org-mode.git` (cgit `/plain/<path>?id=<tag>` serves raw files); GitHub mirror `bzg/org-mode`. Pin the **latest stable release tag at impl time** (verify on the repo — do not trust memory for the version number) and record tag + SHA-256 in the pin module + ADR.
- git-LFS: pattern line `<glob> filter=lfs diff=lfs merge=lfs -text`; pointer files start `version https://git-lfs.github.com/spec/v1`; GitHub Free ≈1GiB storage + 1GiB/month bandwidth — nightly-only LFS checkout of a few MB is comfortably inside budget.
- Parser surface for the preflight: `orgsidian_parser::{analyze, serialize_document}` (re-exported at crate root; `analyze` total per LD-41).

### References

- Source story: [`epics.md:828-842`](_bmad-output/planning-artifacts/epics.md#L828-L842) — user story + 5 ACs. Downstream consumers: [`epics.md:844-857`](_bmad-output/planning-artifacts/epics.md#L844) (Story 2.6 gate), [`epics.md:859-871`](_bmad-output/planning-artifacts/epics.md#L859) (Story 2.7 nightly/L2).
- Architecture: LD-44 matrix [`architecture.md:1228-1245`](_bmad-output/planning-artifacts/architecture.md#L1228); structure rows [`architecture.md:993-999,1009`](_bmad-output/planning-artifacts/architecture.md#L993) (root `fixtures/`, extractor outside workspace, full-nightly "pointer"); OD-1 [`architecture.md:76`](_bmad-output/planning-artifacts/architecture.md#L76); nightly full-corpus [`architecture.md:524`](_bmad-output/planning-artifacts/architecture.md#L524); ADR path [`architecture.md:1396`](_bmad-output/planning-artifacts/architecture.md#L1396); issues-sync "same convention" [`architecture.md:633`](_bmad-output/planning-artifacts/architecture.md#L633).
- Test design (authoritative test strategy per epics.md Process Discipline rule H): [`test-design.md`](_bmad-output/test-artifacts/test-design.md) — §2 item 6 (Epic-2 prerequisite), TC-3 (~L152, meta-test mandate), §5.1-5.3 (fixture architecture + fixtures.toml schema + governance rules), R-017/R-009 risk rows.
- Previous story: [`2-4-implement-round-trip-faithful-serializer.md`](_bmad-output/implementation-artifacts/2-4-implement-round-trip-faithful-serializer.md) — harness contract, EOL discipline, gate baselines, variance pattern.
- Real code: [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs) (directory-driven harness + diagnostics — READ-ONLY), [`tools/corpus-extractor/`](tools/corpus-extractor/) (the stub), [`tools/issues-sync/Cargo.toml`](tools/issues-sync/Cargo.toml) (lib+bin house pattern), [`.gitattributes`](.gitattributes), [`Cargo.toml:18`](Cargo.toml#L18) (exclude list).
- CONTRIBUTING: §2 commit conventions, §3 (`--manifest-path` invocation precedent + "mirroring tools/corpus-extractor"), §4 FR-traceability (why the extractor carries NO FR header), §5 fixture placement (promotion README rule), §8 parser ownership.
- Deferred-work items consumed/annotated: [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) story-2.4 stanza (corpus expansion → this story).
- Memory anchors: [[project_orgsidian_github_plan]] (Free plan — advisory gates, LFS quota), [[feedback_version_policy]], [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]].
- PRD: FR-2 + corpus framing [`prd.md:148-156`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L148); SM-4 [`prd.md:602`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L602).

### Project Context Reference

- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-1 (MIT), LD-32 (gate split), LD-37 (supply-chain), LD-41 (lenience), LD-44/LD-45 (corpus/oracle), LD-47/LD-48 (pinning discipline).
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (Stories 2.1 → 2.8); Process Discipline rules.
- [`test-design.md`](_bmad-output/test-artifacts/test-design.md) — fixture architecture §5; risk register.
- [`CONTRIBUTING.md`](CONTRIBUTING.md), [`docs/parser/KNOWN_DIVERGENCES.md`](docs/parser/KNOWN_DIVERGENCES.md), [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md).

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-06-10 — Story created (ultimate context engine analysis completed — comprehensive developer guide created; upstream-acquisition and fixture-path decisions resolved from specs with variances recorded, not spec-edited). Status: ready-for-dev.
