# Story 2.7: Light up nightly full-corpus + L2 Emacs oracle gates

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 23

## Story

As the **author / contributor**,
I want the full ~2000-assertion corpus + L2 Emacs ground-truth oracle running nightly on pinned `emacs:29.x` + `emacs:30.x`,
So that LD-45 divergence triage workflow is operational and parser drift from Emacs is caught within 24h.

**Traces:** FR-2 (trust-contract enforcement, nightly half — trace *owner* stays `crates/orgsidian-parser/src/serializer.rs`, single owner per CONTRIBUTING §4 — **no new `//! Implements FR-` header anywhere in this story**), LD-32 (nightly full-corpus gate + stale-nightly merge gate), LD-44 (full corpus consumed), LD-45 (L2 oracle pinning + canonical AST + divergence triage), LD-37 (`--locked` everywhere), LD-41 (`analyze` is total), test-design §6.10 L2 row + §7.3.13 (CI-gate scaffold), R-017 (round-trip violation), R-025 (v0.1 Alpha tag blocked unless 2.6+2.7 green), R-028 (Emacs image drift).

## Scope Fence (read first)

This is the **nightly CI-wiring + oracle-bootstrap** story: it adds the `round_trip_full` harness over the pointer manifest, wires the nightly full-corpus gate on all four OSes, designates the L2 seed subset, establishes the canonical-AST pipeline (committed elisp projection + generation script + Rust concordance leg), and lights the L2 Emacs oracle job with LD-45 triage semantics. It is **not**:

- **NOT a parser implementation change.** Nothing under `crates/orgsidian-parser/src/` changes; `build.rs`, `grammar/` (LD-48), `tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/{anchor.org,round_trip/}` are byte-for-byte sentinels. Parser-crate edits: `tests/round_trip.rs` (new fn + doc-comment), the new `tests/l2_canonical.rs`, and the new `tests/canonical_ast/` directory. **Zero `Cargo.toml`/`Cargo.lock` deltas expected** (`serde_json` dev-dep already present since 2.6).
- **NOT corpus regeneration.** `fixtures/*.json` and `tests/fixtures/vault-corpus/**` are generated artifacts — byte-untouched (CONTRIBUTING §5). No `tools/corpus-extractor/src/**` changes: the L2 pick is made *from* the manifest, not *by* the extractor (the deferred item says "the manifest schema already carries per-entry constructs/provenance to support the pick" — consume it).
- **NOT a pr.yml change.** The per-PR workflow stays byte-untouched. The new tests ride along in the existing `cargo test --workspace` step for free (Dev Notes §4); no new per-PR steps, no budget renegotiation.
- **NOT a divergence-fixing story.** If L2 seeding exposes an Orgsidian-vs-Emacs divergence, the LD-45 outcome is a `KNOWN_DIVERGENCES.md` entry (or seed-file swap) — never a `src/` patch, never a grammar edit (LD-48).
- **NOT the memory soak (Story 4.9), graph perf (8.12), or LD-41 nightly steps.** Their placeholder comments in `nightly.yml` stay.
- **NOT branch protection.** GitHub Free cannot enforce required checks ([[project_orgsidian_github_plan]]); "PR-blocking" is realized by the existing `merge-gate-nightly-fresh` job in pr.yml reading the most recent nightly conclusion — a red nightly blocks merges by convention + that job. Nothing to configure.

Deliverables: `round_trip_full` harness, nightly.yml full-corpus steps (hosted×3 + arch), L2 seed designation + `canonical_ast/` + README, `scripts/l2-oracle/` (projection.el + generate + compare), `l2-emacs-oracle` nightly job, Rust concordance test, runbook doc, fixtures.toml/KNOWN_DIVERGENCES/CONTRIBUTING touch-ups, deferred-work hygiene.

## Acceptance Criteria

### AC1 — `round_trip_full` consumes `fixtures/full-nightly.json` (epic-core).

**Given** the pointer manifest (569 entries; schema in Dev Notes §2) and the materialized corpus at `tests/fixtures/vault-corpus/`,
**When** [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs) gains a new test fn,
**Then**:

- A test named **`round_trip_full`** (public name contract — AC2's nightly step invokes `cargo test -p orgsidian-parser round_trip_full --locked` verbatim per epics.md:869 + LD-37) loads `fixtures/full-nightly.json` via the established `CARGO_MANIFEST_DIR/../..` hop, and for **every** entry: resolves `entries[].path` against `tests/fixtures/vault-corpus/`, reads the file, and asserts `serialize_document(&analyze(src)?) == src` byte-for-byte through the existing `assert_round_trip` diagnostics, labeled by `entries[].id`.
- **Anti-placebo:** assert `entries.len() >= 425` (the extractor's `FULL_CORPUS_FLOOR` — exact-count asserts are wrong here, the count floats with the upstream pin; cite the floor's origin in the message) and non-empty `header` (presence only — pin-value sync is owned by the extractor's `validate.rs`).
- **Anti-mangling:** per entry, cross-check on-disk byte length against `entries[].byte_len` (catches EOL rewriting / truncation / partial checkout) and detect git-LFS pointer stubs (`version https://git-lfs.github.com/spec/v1` prefix — same signature as `tools/corpus-extractor/src/emit.rs::is_lfs_pointer`) with an actionable panic naming the file and the `git lfs install && git lfs pull` remedy. Today the corpus is raw git so neither fires; both guards are for the post-LFS-migration future and for corrupted checkouts.
- Maintainer-visible failures (missing manifest, malformed JSON, below-floor count, unreadable/mismatched corpus file) panic with actionable messages naming the path and the regeneration pointer (CONTRIBUTING §5) — never a bare unwrap backtrace.
- Naming discipline: `round_trip_full` must not contain `round_trip_subset` as a substring (it doesn't) and no other test in the crate may contain `round_trip_full` — both cargo filters stay surgical. `round_trip_subset` and all other tests in the file stay byte-unchanged except the module doc-comment (present-tense update: the 2.7 layer exists now).

### AC2 — nightly.yml full-corpus gate: epic-verbatim invocation on all four OSes.

**Given** [.github/workflows/nightly.yml](.github/workflows/nightly.yml) (`hosted` matrix job: macos-14 + ubuntu-24.04 + windows-2022; separate `arch-linux` container job),
**When** the gate steps are added,
**Then**:

- A named step (suggested: `L0 full-corpus round-trip gate (LD-32/LD-44)`) runs `cargo test -p orgsidian-parser round_trip_full --locked` in **both** the `hosted` job (after its `cargo test` step) and the `arch-linux` job (after its `cargo test` step) — that is macOS + Ubuntu + Arch + Windows, the epic's exact OS list. The invocation is epic-verbatim (epics.md:869) **plus `--locked`** (LD-37 house rule; 2.6 precedent — record as variance, don't drop either).
- The step carries **`timeout-minutes: 5`** — generous against the measured 0.18s test body (Dev Notes §3) yet a real tripwire if corpus growth toward the ~2000 target ever blows the budget. Binaries are warm (the jobs' `cargo test --workspace` precedes it), so step wall-clock is pure test runtime.
- House-style comment block on each step (LD references + the named-budgeted-contract rationale: the workspace run already executes `round_trip_full` once; the dedicated step is the visible, budgeted contract — 2.6 AC2 pattern).
- The header comment's LD-44 line and the slot-reservation comment (`# Story 2.7: nightly L0 full corpus + L2 Emacs oracle gates land here`) are updated/annotated as landed. The Story 4.9 and 8.12 placeholders stay.

### AC3 — L2 subset designation + committed canonical ASTs (closes deferred item, story-2.5 stanza).

**Given** the deferred item "L2-subset designation within the corpus" (owner: Story 2.7) and the manifest's per-entry `constructs`/`deftest` provenance,
**When** the seed subset is designated,
**Then**:

- `crates/orgsidian-parser/tests/canonical_ast/` contains one `{stem}.json` per designated corpus file (flat dir; `{stem}` = the corpus file stem, e.g. `extracted/0372_planning-parser-02.org` → `0372_planning-parser-02.json` — the `NNNN_` prefix guarantees uniqueness), satisfying the epic's `canonical_ast/{file}.json` shape.
- **Seed selection rule (documented in `canonical_ast/README.md`):** ≥1 file per construct kind present in the manifest (15 kinds today: inline-markup, drawer, block, list, citation, table, footnote, inline-latex, link, tag, deadline, recurring-timestamp, clock, heading-todo, scheduled — see Dev Notes §2), preferring the smallest representative per construct for peer-reviewability, plus ≥1 structure-only file (headline tree, no constructs). Target **12–20 files**. Each canonical JSON carries `source` (corpus-relative path), `schema: "l2-projection-v1"`, provenance `deftest`, and the projected `headlines` array — self-describing for review.
- **Concordance pre-flight at seeding time:** every candidate must pass BOTH oracle legs locally (Rust leg AC5 + Emacs leg via the AC4 script) before being committed. A candidate that exposes a real Orgsidian-vs-org-element divergence is **not silently swapped out**: record it as a `KNOWN_DIVERGENCES.md` entry (LD-45 landing zone — construct/expected/observed/chosen-behavior/owner format already established) and pick a concordant sibling for the seed. The seed must be green on day 1 — a gate born red is a gate born dead. Known hazards to dodge or document: Orgsidian's default TODO config (`TODO NEXT | DONE WAITING`-family) differs from vanilla org's (`TODO | DONE`) — a bare `NEXT`/`WAITING` headline without a `#+TODO:` directive classifies differently; priority cookies (`[#A]`) stay in Orgsidian's `title` but org-element strips them to `:priority`. (Dev Notes §6.)
- `fixtures/fixtures.toml` gains the `[oracle.canonical-ast]` entry (test-design.md:411-415 sketch: path, `owner = "epic-2"`, LD-45 reference, note "script-generated via scripts/l2-oracle, human-reviewed; mutation requires PR review"). Hand-maintained file — edit is legal; no `[fixture:epic-N]` tag fires (no generated-fixture content mutates).

### AC4 — L2 projection schema + committed generation script.

**Given** that raw `org-element-parse-buffer` dumps are NOT version-stable (Org 9.6 vs 9.7 internal `:standard-properties` differences; buffer positions; bundled-org drift between Emacs 29/30) and Orgsidian's semantic surface is the Story-2.3 headline tree,
**When** the oracle pipeline is built,
**Then**:

- **Projection schema v1** (documented in `canonical_ast/README.md` + the runbook): per headline, in document order with nesting — `level` (int), `todo` (string|null), `title` (string — org-element `:raw-value`), `tags` (string array), `scheduled`/`deadline`/`closed` (string|null — the timestamp's `:raw-value`), `children` (recursive). Nothing else in v1: this is the honest intersection both parsers can produce today (Orgsidian: `Headline{level, todo_state, title, tags, scheduled, deadline, closed, children}`; Emacs: `org-element-property` on `headline`/`timestamp` nodes). Deepening the projection (properties, drawers, body elements) is recorded as deferred work, not smuggled in.
- `scripts/l2-oracle/projection.el` — the committed elisp projection: `org-mode` buffer → `org-element-parse-buffer` → schema-v1 plist tree → `json-serialize` (Emacs ≥27 builtin; key order fixed by construction, `:null-object :null`, strings de-propertized via `substring-no-properties`). Probe-verified shape at story-creation (Dev Notes §3): `{"level":1,"todo":null,"title":"H","tags":[],"scheduled":"<2012-03-29 thu.>"}`. The script must run clean under `emacs --batch` with no user init (`-Q`) and no network.
- `scripts/l2-oracle/generate-canonical.sh` (or `.mjs` — dev's choice, house has both) — regenerates every `canonical_ast/*.json` from the designated seed list using a local `emacs`, pretty-printed (LF, trailing newline) for reviewability. Idempotent: re-running on an unchanged corpus + unchanged Emacs produces byte-identical output.
- **Comparison is structural, not byte-wise:** the CI comparator (AC6) and the Rust leg (AC5) compare canonicalized JSON values, so pretty-printing and key order never produce false divergence.
- Canonical files are generated on this machine with **Emacs 30.2 / Org 9.7.11** (verified present at `/opt/homebrew/bin/emacs`), then human-reviewed in the PR (the LD-45 "peer-reviewed" gate — variance 5: script-generated + reviewed, not literally hand-written). The first nightly run validates them against BOTH pinned images (the LD-45 meta-test); an Emacs-29-only discrepancy surfaces there and routes through triage, not through this story's gates.
- **Environment fallback (pre-authorized, 2.5 precedent):** if the dev session denies `emacs` process execution, finish everything else (script, seed list, Rust leg, workflow) and leave the exact generation commands as a pending-commands block for the orchestrator — mirroring Story 2.5's T7 delegation. Local `emacs --batch` runs are the only step that can need this.

### AC5 — Rust concordance leg: Orgsidian vs canonical (the PR-blocking leg).

**Given** the committed canonical ASTs and the public `analyze()` surface,
**When** `crates/orgsidian-parser/tests/l2_canonical.rs` is added,
**Then**:

- A test (suggested name: `l2_canonical_concordance` — must NOT contain `round_trip` as a substring, keeping the AC1/2.6 cargo filters surgical) iterates every `tests/canonical_ast/*.json`, resolves its `source` against `tests/fixtures/vault-corpus/`, runs `analyze()`, projects the resulting `Document.headlines` to schema v1 (a test-local projection helper over the public semantic API — **no `src/` changes**), and asserts structural equality with the canonical `headlines` value. Divergence diagnostics name the file, the first differing path into the JSON, and both values.
- Anti-placebo: assert the canonical dir is non-empty (≥10 files) so a wiped directory cannot pass vacuously; assert every canonical `source` file exists (LFS-stub + readability guards as in AC1).
- This test runs inside `cargo test --workspace` — per-PR and nightly on every OS cell. That makes LD-45 triage case 1 ("both Emacs concordant against Orgsidian → Orgsidian bug → PR-blocking") **directly** PR-blocking through the canonical proxy, stronger than the merge-gate-only path and fully spec-compatible (Dev Notes §5).

### AC6 — Nightly L2 Emacs oracle job with LD-45 triage semantics.

**Given** that GitHub macOS/Windows runners have no Docker and no pinned Emacs, and that cross-version triage needs both Emacs outputs in one place,
**When** the `l2-emacs-oracle` job is added to nightly.yml,
**Then**:

- A new top-level job `l2-emacs-oracle` runs on `ubuntu-24.04` (no container), checks out WITHOUT submodules (it touches only `scripts/l2-oracle/`, `tests/fixtures/vault-corpus/`, `crates/orgsidian-parser/tests/canonical_ast/` — no cargo, no grammar), and for **each** designated L2 file invokes the projection under both pinned images via `docker run`:
  - `docker run --rm -v "$PWD:/work" -w /work silex/emacs:29.4 emacs -Q --batch -l scripts/l2-oracle/projection.el --eval '(...file...)'`
  - same with `silex/emacs:30.2`
  — preserving the epic's `--batch --eval` invocation shape. Image pinning: exact-version tags `29.4` / `30.2` (both verified existing on Docker Hub 2026-06-11; latest stable of each line per [[feedback_version_policy]]); the job logs each image's digest at runtime for the R-028 drift audit trail. There is no official `emacs` Docker Hub image — `silex/emacs` is the de-facto maintained multi-version image (variance 3).
- A comparator (`scripts/l2-oracle/compare.py` — python3 is on the runner; or .mjs, dev's choice) implements the LD-45 triage per file, comparing canonicalized JSON of `e29`, `e30`, and `canonical`:
  - `e29 == canonical && e30 == canonical` → **OK** (oracle healthy; the Orgsidian leg is enforced by AC5 in the same nightly's test jobs).
  - `e29 == e30 && e29 != canonical` → **FAIL** the job (`::error`). Combined with AC5 green this is exactly "both Emacs concordant against Orgsidian" → Orgsidian bug → PR-blocking via the red nightly; with AC5 red it's canonical/oracle drift after an image bump — either way maintainer action is required (runbook §triage).
  - `e29 != e30` → **WARN** (`::warning`), do NOT fail: "both Emacs discordant from each other → log in KNOWN_DIVERGENCES.md" and the mixed case ("one concordant, one discordant → human review; defer, do not block") — the warning text names the file, both versions' outputs, and instructs the KNOWN_DIVERGENCES entry format.
  - Any execution failure (docker pull error, emacs crash, missing canonical) → **FAIL** with an actionable message (an oracle that cannot run is a broken gate, not a skipped one).
- Job `timeout-minutes: 20` (two image pulls dominate; per-file emacs batch is ~100ms). `fail-fast` posture: evaluate ALL files, then exit non-zero if any FAIL-class divergence was seen (full triage picture per run, matching the nightly `fail-fast: false` philosophy).
- The L2 job is Linux-only (variance 4): the epic binds the 4-OS matrix to `round_trip_full` (AC2 satisfies it), not to the L2 step's placement; org-element ground truth is OS-independent text parsing, and single-job execution is what makes e29-vs-e30 comparison possible at all.

### AC7 — Docs kept honest: runbook + landing-zone + governance.

**Given** the LD-45 triage workflow must be operable by a human at 7am after a red nightly,
**When** the docs land,
**Then**:

- `docs/parser/l2-oracle.md` (new, concise): schema v1 definition, seed-selection rule, how to regenerate canonical ASTs (script invocation + review requirement), the triage decision table (mirroring AC6), the R-028 image-bump procedure (bump tag → regenerate/verify canonical → PR), and the local-run instructions (`docker run` lines; local emacs alternative).
- `docs/parser/KNOWN_DIVERGENCES.md`: header's "From Story 2.7 onward… Until then" framing updated to present tense (the landing zone is now live; entry format unchanged). Plus any entries produced by AC3's seeding pre-flight. No other content edits.
- `fixtures/README.md` consumers table: `full-nightly.json` row to present tense (the 2.7 gate exists now); add the canonical-ast consumer line if the table shape calls for it.
- `tests/fixtures/vault-corpus/README.md`: the "only nightly/L2 work and corpus regeneration read these files" sentence gains the honest addendum that `cargo test --workspace` now also reads them via `round_trip_full`/`l2_canonical` (one line — this matters for the future LFS owner).
- `CONTRIBUTING.md`: one line in §5 (or §7) pointing L2/canonical questions at `docs/parser/l2-oracle.md`; §1 parity line gains nothing (both new tests are inside `cargo test --workspace` — already parity-covered; the Emacs job is intentionally NOT in the local parity contract, the runbook covers it).
- No edits to `epics.md` / `architecture.md` / `test-design.md` / PRD (variance-recording instead, Dev Notes §7).

### AC8 — Gates stay green; sentinels untouched; deferred-work hygiene.

**Given** all the above,
**When** the gates run,
**Then**:

- `cargo test -p orgsidian-parser --locked` green (baseline 77; expect +2: `round_trip_full`, `l2_canonical_concordance` — report exact delta). `cargo test --workspace --locked` green (baseline 122 passed / 0 failed / 11 ignored; +2 and ~+0.5s runtime — fine). Extractor suite untouched at 65 via `--manifest-path`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all -- --check` clean; `cargo deny --locked check all` ok; root + extractor `cargo audit` at the established baselines. **Zero `Cargo.lock` delta** (serde_json dev-dep already present). Any lockfile change: STOP, decision-grade.
- Workflow sanity: `nightly.yml` parses (YAML load — ruby stdlib worked for 2.6, PyYAML absent); eyeball step indentation against existing steps; `pr.yml` byte-untouched (sentinel).
- Sentinels via `git status` / diff: nothing under `crates/orgsidian-parser/src/`, `build.rs`, `grammar/`, `tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/{anchor.org,round_trip/}`; `tools/**` sources untouched; `fixtures/*.json` + `tests/fixtures/vault-corpus/**/*.org` byte-untouched; `.github/workflows/pr.yml`, `deny.toml`, `.cargo/audit-ignore.txt`, `docs/security/**` untouched.
- deferred-work.md: story-2.5 "L2-subset designation" item annotated closed-by-2.7; story-2.6 LFS item annotated with the new disk-reading consumers (post-migration CI checkouts need `lfs: true` — one line); a `## Deferred from: code review of story-2.7 (YYYY-MM-DD)` stanza pre-seeded. Known candidates: projection-schema deepening beyond the Story-2.3 surface; L2 seed expansion cadence (pair with the §8 pin-bump rhythm); silex-image digest-pinning hardening (R-028); corpus growth back toward the ~2000 target on upstream pin bumps; first-nightly-run confirmation (process — Emacs-29 concordance is unverifiable locally).

## Tasks / Subtasks

- [ ] **T1** — `round_trip_full` in `tests/round_trip.rs`: manifest loader (floor ≥425 + header presence), per-entry path resolution + byte_len cross-check + LFS-stub guard + `assert_round_trip` with id labels; doc-comment present-tense. RED-first: tamper the floor (or point at a nonexistent corpus dir) to watch diagnostics fire, then finalize. (AC1)
- [ ] **T2** — Measure: `time cargo test -p orgsidian-parser round_trip_full --locked` warm; record for Completion Notes (story-creation baseline: 569 entries, 0 failures, **0.18s** test body). (AC1/AC2)
- [ ] **T3** — nightly.yml: full-corpus gate step in `hosted` + `arch-linux` jobs (epic-verbatim + `--locked`, `timeout-minutes: 5`, house comments); header LD-44 line + slot-reservation comment updated. (AC2)
- [ ] **T4** — `scripts/l2-oracle/projection.el` + `generate-canonical.sh`: schema-v1 projection, `-Q --batch` clean, deterministic pretty-printed output. Verify against the story-creation probe shape. (AC4)
- [ ] **T5** — Seed designation: pick 12–20 corpus files per the AC3 rule from `fixtures/full-nightly.json` constructs/provenance; run the concordance pre-flight (Rust leg + local emacs 30.2); swap-or-document divergent candidates (KNOWN_DIVERGENCES entries per LD-45); commit `canonical_ast/*.json` + `canonical_ast/README.md`. If emacs exec is denied in-session: pending-commands block for the orchestrator (AC4 fallback). (AC3/AC4)
- [ ] **T6** — `tests/l2_canonical.rs`: test-local schema-v1 projection over the public semantic API; structural compare vs every canonical file; ≥10-file floor + source-exists + LFS guards; actionable diagnostics. RED-first: tamper one canonical value in memory (or a temp copy) to watch the diff diagnostics fire. (AC5)
- [ ] **T7** — nightly.yml `l2-emacs-oracle` job: checkout (no submodules), digest logging, per-file `docker run` × {silex/emacs:29.4, silex/emacs:30.2} with `--batch --eval`, comparator with LD-45 triage exit semantics (OK/FAIL/WARN per AC6), evaluate-all-then-fail, `timeout-minutes: 20`. (AC6)
- [ ] **T8** — `scripts/l2-oracle/compare.py` (or .mjs): canonicalized JSON triple-compare + GitHub annotations; unit-testable pure function if trivially cheap (optional). (AC6)
- [ ] **T9** — Docs: `docs/parser/l2-oracle.md` runbook; KNOWN_DIVERGENCES header present-tense (+ any seeding entries); fixtures/README row; vault-corpus README consumer line; CONTRIBUTING pointer line; fixtures.toml `[oracle.canonical-ast]`. (AC7/AC3)
- [ ] **T10** — deferred-work.md: annotate 2.5's L2-designation item + 2.6's LFS item; pre-seed the story-2.7 stanza. (AC8)
- [ ] **T11** — Gates: parser/workspace/extractor suites, clippy/fmt/deny/audit, YAML sanity, sentinel sweep, zero-lockfile-delta check. Exact counts in Completion Notes. (AC8)
- [ ] **T12** — Commit. Suggested title: `feat(ci): light up nightly full-corpus + L2 Emacs oracle gates (Story 2.7, closes #23)`. **NO** Co-Authored-By trailer, **NO** "Generated with Claude Code" footer, no AI-credit lines. PR body (pipeline's PR step): gate invocations + measured runtimes + first-nightly confirmation ask (Emacs-29 leg + docker-pull timing) + deferred-closure list + `Closes #23`. (process)

## Dev Notes

### 1. Current state (verified at story-creation, branch `story/2.7-nightly-full-corpus-l2-oracle` @ `b3d8ae6`, 2026-06-11)

- **`round_trip_full` does NOT exist** — only `round_trip_subset` (2.6) plus inline/proptest/tripwire tests in `tests/round_trip.rs`. The epic AC names the test as if present; creating it IS this story's first deliverable, name-locked by AC2's invocation.
- **Manifest:** `fixtures/full-nightly.json` — pointer manifest (no embedded content), **569 entries** (epic's "~2000" was a target; observed harvest at release_9.8.5 is 569 — 2.5 Orchestrator Execution Record), header schema identical to subset-pr; per-entry `id`, `deftest`, `constructs`, `path`, `byte_len`. All paths point under `tests/fixtures/vault-corpus/extracted/` (569 files; the 70 `synthesized/` files are subset-only twins, NOT in this manifest).
- **Corpus on disk:** raw git (NOT LFS — `FOLLOWUP(LFS-migration)` marker in `.gitattributes`; `-text` rule protects EOL bytes, so Windows checkouts are byte-exact). 2.2 MB under `extracted/`.
- **Story-creation probe (this machine):** all 569 entries round-trip byte-identical in **0.18s** (debug profile, byte_len cross-check included). The full gate can be born green and the LD-32 "3-5 min per cell" sizing fear does not materialize at this corpus size.
- **nightly.yml:** `hosted` matrix job (macos-14/ubuntu-24.04/windows-2022; `fail-fast: false`; submodules recursive; full cargo+pnpm suite; shell-ui steps skipped on windows per #120) + `arch-linux` container job (archlinux:base-devel; rustup/pacman bootstrap; `safe.directory` fix; same suite). The Story-2.7 slot comment sits after the i18n step in `hosted`; the arch job has no marker (add the step after its `cargo test` anyway — the epic OS list includes Arch). `cron: 0 5 * * *` + `workflow_dispatch`.
- **Emacs locally:** `/opt/homebrew/bin/emacs` = **GNU Emacs 30.2** (Org 9.7.11) — canonical generation is possible on this machine. Docker present at `/usr/local/bin/docker`. Emacs 29.x NOT present locally — the 29-leg is first verified by the first nightly run (process note).
- **Docker images:** no official `emacs` repo on Docker Hub; `silex/emacs` tags `29.4` and `30.2` both verified existing (2026-06-11; tags are rebuilt rolling — log digests at runtime, R-028).
- **KNOWN_DIVERGENCES.md:** 9 entries; header already declares this file the LD-45 landing zone "from Story 2.7 onward".
- **GitHub issue #23** verified OPEN: "[Story 2.7] Light up nightly full-corpus + L2 Emacs oracle gates" (epic:2 / type:story / status:backlog / milestone:v0.1).
- **Branch stack:** `story/2.7-nightly-full-corpus-l2-oracle` on completed 2.6 (`b3d8ae6`) ← 2.5 ← 2.4 ← 2.3 ← main(2.2). Stacked-PR caveats from 2.5/2.6 apply (flag base-ordering in the PR body). A concurrent pipeline may work Story 2.8 in a separate worktree — disjoint paths (`crates/orgsidian-cli`), no overlap.

### 2. Pointer-manifest schema (consume, don't reshape)

```json
{ "header": { "generator": "orgsidian-corpus-extractor", "extractor_version": "0.0.0",
              "org_release_tag": "release_9.8.5", "source_sha256": "f3065e65…" },
  "entries": [ {
      "id": "extracted/0372_planning-parser-02",      // ← failure label
      "deftest": "test-org-element/planning-parser",   // ← provenance (L2 pick + canonical metadata)
      "constructs": ["scheduled"],                      // ← L2 pick driver
      "path": "extracted/0372_planning-parser-02.org",  // ← resolve against tests/fixtures/vault-corpus/
      "byte_len": 32                                    // ← anti-mangling cross-check
  } ] }
```

Construct distribution (story-creation count over 569 entries): inline-markup 58, drawer 40, block 38, list 37, citation 28, table 22, footnote 22, inline-latex 19, link 16, tag 12, deadline 9, recurring-timestamp 5, clock 3, heading-todo 3, scheduled 1. Sparse kinds (scheduled/clock/heading-todo) have few candidates — if a sparse kind's only candidates all fail the concordance pre-flight, document the divergence and note the kind as canonical-uncovered in `canonical_ast/README.md` (decision-grade honesty beats a forced red).

### 3. Story-creation probes (empirical ground truth)

- **Full-corpus round trip:** 569/569 byte-identical, 0.18s test body, byte_len cross-check green on every entry (temp probe, removed). `timeout-minutes: 5` has ~1600x headroom.
- **Emacs batch projection:** `emacs --batch` + `org-element-parse-buffer` + `json-serialize` on a corpus file produced exactly the schema-v1 shape: `{"level":1,"todo":null,"title":"H","tags":[],"scheduled":"<2012-03-29 thu.>"}` (Emacs 30.2 / Org 9.7.11). Pitfalls already hit and solved in the probe: `json-serialize` takes a plist/hash (not a flat vector); titles need `substring-no-properties` (org returns propertized strings); use `:null-object :null` for empty-slot determinism; timestamps' `:raw-value` preserves source bytes verbatim (`thu.` stayed `thu.`).
- **org-element raw dumps are NOT comparable across versions:** Org 9.7 nodes carry `:standard-properties` vectors with buffer references — this is why schema-v1 projection (not sexp dumps) is the only workable canonical format.

### 4. Design: the three-legged oracle, and where each leg runs

| Leg | Compares | Runs | Blocking semantics |
|---|---|---|---|
| `round_trip_full` (AC1) | corpus bytes ↔ serialize(analyze()) | workspace tests everywhere + named nightly step ×4 OS | red nightly → merge gate |
| `l2_canonical_concordance` (AC5) | Orgsidian projection ↔ canonical | workspace tests everywhere (per-PR too) | red per-PR = direct PR block (triage case 1) |
| `l2-emacs-oracle` job (AC6) | emacs29 ↔ emacs30 ↔ canonical | nightly, Linux, docker | FAIL/WARN per LD-45 triage |

- The canonical AST is the meeting point: AC5 pins Orgsidian to it per-PR; AC6 pins both Emacs versions to it nightly. "Both Emacs concordant against Orgsidian" is then decidable without ever diffing Orgsidian against Emacs directly.
- Both new Rust tests ride inside `cargo test --workspace` (pr.yml Step 9 + nightly's cargo test) at +0.2s — no `#[ignore]` games: an `#[ignore]`'d test would make the epic-verbatim nightly invocation a 0-test placebo. The dedicated nightly steps are the named, budgeted contract (2.6 pattern).
- Triage decision table implemented by the comparator (per L2 file): `e29==e30==canon` → OK; `e29==e30≠canon` → FAIL (Orgsidian bug if AC5 green, canonical drift if AC5 red — both demand action); `e29≠e30` → WARN + KNOWN_DIVERGENCES instruction (covers both the "discordant from each other" and the "mixed" LD-45 rows — neither blocks).

### 5. Why the Rust leg per-PR is correct (not scope creep)

LD-45 case 1 says Orgsidian-vs-ground-truth divergence is PR-blocking. The nightly-only realization (red nightly → stale-nightly merge gate) has a 24h detection lag; the canonical files make case 1 testable in pure Rust with zero CI cost (≤20 small files through `analyze()`), so the leg lands in the workspace suite and the lag disappears. The Emacs legs — the parts that genuinely need nightly cadence, docker, and triage nuance — stay nightly. This strengthens the spec without contradicting it; recorded as part of variance 7.

### 6. Seeding hazards (read before picking files)

- **TODO config mismatch:** vanilla org default is `(sequence "TODO" "DONE")`; Orgsidian's default TodoConfig includes NEXT/WAITING-family keywords (Story 2.3). A corpus file with a bare `NEXT` headline and no `#+TODO:` directive projects `todo: "NEXT"` in Orgsidian but `todo: null, title: "NEXT …"` in Emacs. Prefer candidates with explicit directives or without non-vanilla keywords; otherwise it's a legitimate KNOWN_DIVERGENCES entry (chosen behavior: Orgsidian's richer default — document it).
- **Priority cookies:** `[#A]` stays in Orgsidian `title`, stripped by org-element into `:priority`. Dodge in seed v1 or document.
- **`title` normalization:** org-element `:raw-value` strips stars/keyword/tags but keeps everything else verbatim; Orgsidian `title` is "headline text minus stars, TODO keyword, and trailing tag list" — same contract on paper; the pre-flight verifies it holds per candidate in practice.
- **COMMENT headlines, statistics cookies, footnote-section flags:** org-element models them as headline properties outside schema v1 — `:raw-value` keeps them in the title text on both sides, so they're safe; do not add v1 fields for them.
- **Empty-vs-null discipline:** `tags: []` (never null); `todo`/planning slots `null` when absent — fix it in both projections and the README so review diffs stay clean.

### 7. Variances (record in Completion Notes; no spec edits — epics.md is the issues sync-source)

1. **Corpus size:** observed 569 assertions vs epic/LD-32 "~2000" — extraction reality recorded by the 2.5 orchestrator; the test asserts the extractor's floor (425), not an exact count. Growth toward ~2000 rides future upstream pin bumps (deferred-work).
2. Epic invocation gains `--locked` (LD-37 house rule; 2.6 variance-1 precedent).
3. **`emacs:29.x`/`emacs:30.x` realized as `silex/emacs:29.4`/`silex/emacs:30.2`** — no official `emacs` Docker Hub repo exists; silex is the de-facto maintained multi-version image; exact-version tag pin per [[feedback_version_policy]]; runtime digest logging covers R-028 drift.
4. **L2 Emacs legs Linux-only, single job** — docker is unavailable on GitHub macOS/Windows runners; the epic's 4-OS clause binds `round_trip_full` (satisfied on all four); cross-version triage requires co-located outputs.
5. **Canonical ASTs script-generated + human-reviewed**, not literally "hand-written" (LD-45 wording) — deterministic regeneration beats hand-typed JSON for image-bump drift; PR review remains the human gate.
6. **Projection schema v1 = Story-2.3 semantic surface** (headline tree + planning + tags + todo) — raw org-element dumps are version-unstable and Orgsidian doesn't model inline/body elements yet (KNOWN_DIVERGENCES 1-7); deepening deferred explicitly.
7. **Full-corpus + concordance tests ride per-PR** inside `cargo test --workspace` (+0.2s measured) — the nightly named steps remain the budgeted contract; LD-32's nightly-only rationale (minutes-scale corpus) doesn't materialize at 569 entries; the Emacs legs preserve the per-PR/nightly split where it costs real minutes.
8. **Canonical generation env:** Emacs 30.2 / Org 9.7.11 locally (or orchestrator-delegated); the Emacs-29 leg is first exercised by the first nightly (process note in PR body).

### Project Structure Notes

**Alignment with unified project structure** (architecture.md:1396-1400 + test-design.md:332,411-415 pre-declare `canonical_ast/` exactly where this story puts it):

- `crates/orgsidian-parser/tests/round_trip.rs` — UPDATE: add `round_trip_full` + manifest/corpus helpers; doc-comment present-tense. Preserve: all existing test fns byte-unchanged, `assert_round_trip` diagnostics shape, substring-uniqueness of both gate filters.
- `crates/orgsidian-parser/tests/l2_canonical.rs` — NEW (test-local projection; public API only).
- `crates/orgsidian-parser/tests/canonical_ast/` — NEW: `{stem}.json` × 12-20 + `README.md` (selection rule + schema v1 + regeneration pointer).
- `.github/workflows/nightly.yml` — UPDATE: 2 gate steps (hosted + arch) + `l2-emacs-oracle` job + comment updates. Jobs/triggers/concurrency otherwise unchanged; 4.9/8.12 placeholders stay.
- `scripts/l2-oracle/{projection.el, generate-canonical.sh, compare.py}` — NEW (scripts/ is the established home for CI-support tooling: check-pnpm-licenses.mjs precedent).
- `docs/parser/l2-oracle.md` — NEW. `docs/parser/KNOWN_DIVERGENCES.md` — UPDATE (header present-tense + seeding entries only).
- `fixtures/fixtures.toml` — UPDATE (`[oracle.canonical-ast]`); `fixtures/README.md` + `tests/fixtures/vault-corpus/README.md` — UPDATE (one-line consumer honesty each); `CONTRIBUTING.md` — UPDATE (one pointer line).
- `_bmad-output/implementation-artifacts/{deferred-work.md, sprint-status.yaml}` — UPDATE.
- READ-ONLY / MUST NOT CHANGE: `crates/orgsidian-parser/src/**`, `build.rs`, `grammar/`, `tests/{anchor,grammar,semantic}.rs`, `tests/fixtures/{anchor.org,round_trip/}`, `tools/**`, `fixtures/*.json`, `tests/fixtures/vault-corpus/**/*.org`, `.github/workflows/pr.yml`, root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.cargo/audit-ignore.txt`, `docs/security/**`, `docs/adr/**`, `_bmad-output/planning-artifacts/**`, `_bmad-output/test-artifacts/**`.

### Testing Standards Summary

- The story's test surface IS the gate set: `round_trip_full` (floor + byte_len + LFS guards = falsifiable on truncation/mangling/missing corpus), `l2_canonical_concordance` (≥10-file floor = falsifiable on wiped canonical dir), the comparator's FAIL path (falsifiable via a tampered canonical). RED-first runs required on T1/T6.
- Anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green, file byte-unchanged (Story 1.9 discipline).
- Nothing may pass vacuously: empty canonical dir, missing manifest, 0-entry manifest, and unpullable docker images must each be loud, actionable failures.
- CI verification is two-stage: all cargo gates in-session; the Emacs-29 leg, docker-pull timing, and the L2 job's first triage output are confirmable only on the first nightly run (process note for the PR body + workflow_dispatch suggestion to the merger).

### Previous Story Intelligence (Stories 2.5 + 2.6)

- **The manifest was designed for this story** (2.5 Dev Notes + deferred item 175): per-entry `constructs`/`deftest` exist precisely to drive the L2 pick. Consume; never reshape or hand-edit.
- **The harness was designed for extension** (2.4→2.6 lineage): `assert_round_trip` diagnostics + `manifest_path()`-style helpers + the `../..` hop are established; `round_trip_full` is additive iteration, not redesign.
- **2.6's CI-step pattern transfers verbatim:** epic-verbatim invocation + `--locked`, `timeout-minutes` as enforcement, house-style comment naming the redundancy rationale, slot-comment annotation, YAML sanity via ruby when PyYAML is absent.
- **Orchestrator execution reality** (2.5/2.6): the dev environment denies network and may deny arbitrary process execution (`emacs`, `docker`) and git; the full cargo suite + file edits work. Pre-authorized fallbacks: pending-commands block for the orchestrator (2.5 T7 precedent — applies to canonical generation here), exact git commands returned if commit is denied. Run cargo from the worktree root (gix discovery panics outside it).
- **Numbers that must not move:** workspace 122/0/11 (+2 from this story), extractor 65, audit 18-warnings baseline, deny ok×4, zero lockfile delta.
- **Process patterns:** variance-recording over spec-editing; STOP on lockfile/deny/audit surprises; pre-seeded deferred-work stanza; exact counts in Completion Notes; no AI-credit lines; issue labels + PR body owned by later pipeline steps.

### Git Intelligence Summary

`git log --oneline` at story-write: `b3d8ae6` 2.6 review fixes ← `3834fc8` 2.6 impl (closes #22) ← `2af292c` 2.6 story ← `edc568a`/`23caf32`/`699e90e` (2.5 trio) ← 2.4 trio ← 2.3 trio ← `2f93b5d` (2.2 merge). Per-story commit pattern: story-file → impl → review-fixes. CI-precedents: 1.8 authored nightly.yml (hosted+arch shape, container quirks documented in-file: pacman npm gotcha, safe.directory, rust-cache shared keys), 2.6 authored the gate-step house style. Worktree clean at story-creation.

### Latest Technical Information

- **Zero new Rust deps** — `serde_json` dev-dep present since 2.6; the projection helper in tests uses it for `Value` comparison.
- **silex/emacs** (Docker Hub): tags `29.4` + `30.2` verified 2026-06-11; tags rebuild on a rolling basis (same-day `last_updated` observed) — tag-pin + runtime digest logging is the right posture vs digest-pin brittleness (R-028 runbook section owns bumps). Plain tags are Ubuntu-based with emacs on PATH; no extra packages needed for `--batch` org parsing.
- **Emacs `json-serialize`** is builtin ≥27 (no package install in containers); takes plists/hash-tables with keyword keys; `:null-object :null` recommended for schema determinism.
- **Org versions in play:** Emacs 29.4 bundles Org 9.6.x; Emacs 30.2 bundles Org 9.7.x; local = 9.7.11. Cross-version org-element *property* surface for schema v1 (level/todo-keyword/raw-value/tags/planning) is stable across both; internal node format is not — hence projection, never sexp dumps.
- **GitHub runners:** docker preinstalled on `ubuntu-24.04`; absent on `macos-14`/`windows-2022` (why AC6 is Linux-only). `archlinux:base-devel` container job cannot run docker-in-docker — the L2 job is its own top-level job, not an arch step.
- **`timeout-minutes`** per-step (gate steps) and per-job (`l2-emacs-oracle`) both enforce by failing — consistent with test-design §7.3.13.

### References

- Source story: [`epics.md:859-871`](_bmad-output/planning-artifacts/epics.md#L859-L871) — user story + 3 ACs (invocation :869, L2 step :870, triage :871). Upstream: 2.5 [`epics.md:828-842`](_bmad-output/planning-artifacts/epics.md#L828), 2.6 [`epics.md:844-857`](_bmad-output/planning-artifacts/epics.md#L844).
- Architecture: LD-32 nightly clause [`architecture.md:524-526`](_bmad-output/planning-artifacts/architecture.md#L524); LD-44 [`architecture.md:1228-1245`](_bmad-output/planning-artifacts/architecture.md#L1228); **LD-45 (the spec core)** [`architecture.md:1247-1255`](_bmad-output/planning-artifacts/architecture.md#L1247); structure pre-declaration [`architecture.md:1396-1400`](_bmad-output/planning-artifacts/architecture.md#L1396).
- Test design: §6.10 three-level oracle + L2 row; §6.11 nightly matrix; §7.3.13 CI-gate scaffold; fixtures sketch `[oracle.canonical-ast]` [`test-design.md:411-415`]; R-017/R-025/**R-028** (oracle drift).
- Previous stories: [`2-6-…md`](_bmad-output/implementation-artifacts/2-6-light-up-l0-round-trip-ci-gate-per-pr-100-files-60s.md) (gate-step house style, harness design §4-5, orchestrator constraints), [`2-5-…md`](_bmad-output/implementation-artifacts/2-5-build-tools-corpus-extractor-fixture-governance.md) (manifest schema, Orchestrator Execution Record — observed 569, floor 425, pending-commands precedent).
- Real code this story modifies: [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs), [`.github/workflows/nightly.yml`](.github/workflows/nightly.yml), [`fixtures/fixtures.toml`](fixtures/fixtures.toml), [`docs/parser/KNOWN_DIVERGENCES.md`](docs/parser/KNOWN_DIVERGENCES.md), [`CONTRIBUTING.md`](CONTRIBUTING.md), [`fixtures/README.md`](fixtures/README.md), [`tests/fixtures/vault-corpus/README.md`](tests/fixtures/vault-corpus/README.md).
- Real code consumed read-only: [`fixtures/full-nightly.json`](fixtures/full-nightly.json), [`tests/fixtures/vault-corpus/extracted/`](tests/fixtures/vault-corpus/), [`tools/corpus-extractor/src/validate.rs`](tools/corpus-extractor/src/validate.rs) (`FULL_CORPUS_FLOOR = 425`), [`tools/corpus-extractor/src/emit.rs`](tools/corpus-extractor/src/emit.rs) (`is_lfs_pointer` signature), [`crates/orgsidian-parser/src/semantic/headline.rs`](crates/orgsidian-parser/src/semantic/headline.rs) (the projection's Rust source surface), [`.github/workflows/pr.yml`](.github/workflows/pr.yml) (`merge-gate-nightly-fresh` — the blocking mechanism, untouched).
- Deferred-work items consumed: story-2.5 stanza "L2-subset designation" (pre-assigned here); story-2.6 stanza LFS item (gains a consumer note).
- Memory anchors: [[project_orgsidian_github_plan]] (Free plan: merge gate is advisory-by-convention), [[feedback_version_policy]] (exact-tag image pins), [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]], [[project_orgsidian_github_label_scheme]] (label flips owned by later pipeline steps).

### Project Context Reference

- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-32 (matrix + merge gate), LD-37 (--locked/supply chain), LD-41 (analyze total), LD-44/LD-45 (corpus/oracle — this story's spec core), LD-48 (grammar READ-ONLY).
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (2.1→2.8); Process Discipline rules.
- [`test-design.md`](_bmad-output/test-artifacts/test-design.md) — §6.10/§6.11, §7.3.13, risk register (R-017/R-025/R-028).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §1 parity, §2 commits, §5 fixture governance, §8 pin-bump checklist (the L2 seed joins its orbit).
- [`docs/parser/KNOWN_DIVERGENCES.md`](docs/parser/KNOWN_DIVERGENCES.md), [`docs/adr/0001-corpus-subset-selection.md`](docs/adr/0001-corpus-subset-selection.md), [`fixtures/README.md`](fixtures/README.md), [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md).

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-06-11 — Story created (ultimate context engine analysis completed — comprehensive developer guide created; gates grounded empirically: full 569-entry corpus round-trips in 0.18s, emacs-batch schema-v1 projection probe-verified on Emacs 30.2/Org 9.7.11, silex/emacs 29.4+30.2 image tags verified on Docker Hub; canonical-AST pipeline, LD-45 triage exit semantics, and Linux-only L2 placement pre-decided from specs with 8 variances recorded, not spec-edited). Status: ready-for-dev.
