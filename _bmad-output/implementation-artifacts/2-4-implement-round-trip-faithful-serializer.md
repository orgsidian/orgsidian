# Story 2.4: Implement round-trip-faithful serializer

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 20

## Story

As the **user editing my `.org` vault**,
I want files saved by Orgsidian without user-visible edits to be byte-identical to their on-disk version (FR-2),
So that the trust contract with the org community is honored.

**Traces:** FR-2 (PRD §4.1 — "the trust contract"), LD-32 (CI gate budgets), LD-44 (subset corpus criteria), architecture FR-2 mapping row (`orgsidian-parser/src/serializer.rs` + `tests/round_trip.rs`).

## Scope Fence (read first)

This is the **serializer** story: it makes `analyze()`'s semantic structs carry enough raw source to emit org text back, byte-identical, and proves it with a round-trip test surface. It is **not**:

- **NOT a field-driven org renderer.** Do NOT build code that re-renders a `Headline` from its semantic fields (`level` stars + `todo_state` + `title` + tags + re-emitted `properties` map…). That path is structurally lossy — `title` is trimmed, `properties` is an unordered last-wins `HashMap`, drawer contents lose framing — and FR-2 is byte-fidelity, not semantic re-rendering. The serializer is a **raw-region passthrough** (see AC1/AC2). A mutation-aware emit path ("change `todo_state`, serialize the edit") is Epic 4's edit-application concern, explicitly out of scope here.
- **NOT corpus/fixtures machinery (Story 2.5).** No `tools/corpus-extractor`, no `fixtures/subset-pr.json`, no git-LFS vault corpus, no `fixtures.toml`, no ADR. This story ships a small **interim** in-repo corpus (AC3) sized for its own test surface.
- **NOT the CI gates (Stories 2.6/2.7).** No `pr.yml`/`nightly.yml` edits, no Emacs oracle, no canonical-AST JSON. But the corpus test function MUST be named `round_trip_subset` — Story 2.6's gate invokes `cargo test -p orgsidian-parser round_trip_subset` verbatim (epics.md:854) and must not need a rename.
- **NOT the CLI (Story 2.8)** and **NOT the save path (Epic 3)** — no `clap`, no atomic-write wiring, no `orgsidian-core` orchestrator. Other crates take **zero** new deps on `orgsidian-parser` in this story.
- **NOT a `parse()`/wrapper change.** `tests/anchor.rs`, `tests/grammar.rs`, `tests/fixtures/anchor.org`, `build.rs`, `grammar/` (vendored submodule, LD-48) stay byte-for-byte untouched. `tests/semantic.rs` should also stay byte-unchanged (verified: it contains no struct literals, only `analyze()` consumers — the new `raw` field cannot break it).

The deliverable is exactly: `crates/orgsidian-parser/src/serializer.rs` (`//! Implements FR-2`) with `serialize` + a document-level entry point, raw-region retention added to the semantic layer, `tests/round_trip.rs` (corpus + proptest properties), the interim round-trip fixture corpus, and deferred-work annotations.

## Acceptance Criteria

### AC1 — Serializer module with the epic-named signature + a document-level entry point.

**Given** Story 2.3's `analyze(&str) -> Result<Document, ParseError>` (owned structs, byte spans),
**When** the serializer is implemented in [`crates/orgsidian-parser/src/serializer.rs`](crates/orgsidian-parser/src/serializer.rs) (NEW top-level module — the epic and the architecture FR-2 mapping row both name this exact path; it is NOT under `semantic/`),
**Then**:

- `pub fn serialize(headlines: &[Headline]) -> String` exists with the epic-verbatim signature (epics.md:822). It emits each headline's retained raw own-region text followed by its `children`, recursively, concatenated in document order.
- **Interpretation (record as variance, do not edit epics.md):** `serialize(headlines)` alone cannot reproduce a file's preamble (zeroth section: `#+TITLE:`, directives, intro text) because `&[Headline]` doesn't carry it. Add `pub fn serialize_document(document: &Document) -> String` = preamble text (when present) + `serialize(&document.headlines)`. The byte-identity AC ("opening **any file** … byte-identical") is proven against `serialize_document`; for files with no preamble the two are identical. Both functions re-exported at the crate root (mirror the `analyze` re-export pattern in `lib.rs`).
- Serialization is **infallible and pure** (`-> String`, no `Result`, no I/O) and never re-emits semantic fields: no `HashMap` iteration, no `title` reconstruction, no timestamp formatting. Raw text in, raw text out.

### AC2 — Raw-region retention in the semantic layer; tiling invariant; no byte ever dropped.

**Given** the current model retains only spans (`Range<usize>`) — `Document` does NOT keep the source and `Headline` has no raw text, so byte-faithful emission is impossible today (verified against `src/semantic/{mod,headline}.rs` at story-creation time),
**When** raw retention is added,
**Then**:

- `Headline` gains `raw: String` — the source slice of the headline's **own region**: section start through the start of its first child section, or section end when childless. This is exactly the region `build_section` already computes for the links scan (`headline.rs:254-259`); populate `raw` from the same boundaries so the two never drift.
- **Tiling invariant (the load-bearing contract):** for every analyzed document, `preamble.text` + the recursive concatenation of all `raw` fields reproduces the input **byte-for-byte, 0..len, nothing dropped, nothing duplicated**. Empirically verified at the pinned grammar SHA (`219c0b27…`) during story creation — see Dev Notes §1 for the probe data: preamble `body` + top-level `subsection` spans tile to EOF exactly in every probed case (blank lines between sections, CRLF, leading/trailing blank lines, missing trailing newline, empty file), and within a section the named children tile contiguously.
- **Defensive gap-absorption:** the grammar *can* produce trees where naive span concatenation drops bytes — `build_section` returns `None` for a `section` with no `headline` field (ERROR regions), and ERROR nodes at root level are not `subsection` fields, so `children_by_field_name("subsection")` skips them. The implementation MUST NOT lose those bytes: track a position cursor during `analyze` and fold any uncovered gap into an adjacent retained region (absorb into the next headline's `raw`, the preamble, or a document-level trailing field — dev's choice; the invariant is what's non-negotiable, the mechanism is not). The AC4 arbitrary-input property is the enforcement mechanism: if any input drops bytes, proptest finds it.
- Adding a public field to `Headline` is a struct-literal-breaking change with **zero downstream consumers** (verified: no crate depends on `orgsidian-parser`; `tests/semantic.rs` constructs nothing). Document the field (`///`) including the own-region definition and the tiling invariant.

### AC3 — `tests/round_trip.rs` proves byte-identity on an interim LD-44-representative corpus.

**Given** AC1 + AC2,
**When** the round-trip test surface is written,
**Then**:

- [`crates/orgsidian-parser/tests/round_trip.rs`](crates/orgsidian-parser/tests/round_trip.rs) (NEW — the epic-named file) contains a corpus-driven test **named `round_trip_subset`** that iterates every `.org` file under `tests/fixtures/round_trip/` and asserts `serialize_document(&analyze(&src)?) == src` byte-for-byte, reporting the failing filename + first divergent byte offset on mismatch (a bare `assert_eq!` on two multi-KB strings is useless diagnostics).
- `tests/fixtures/round_trip/` (NEW) holds a handcrafted interim corpus: every one of the 15 LD-44 constructs from Story 2.3's AC6 table appears in at least one file (reuse/adapt the inline samples from `tests/semantic.rs` — do not invent new syntax), plus LD-44 edge-bucket cases: Unicode/RTL content, over-indented property drawers, trailing whitespace in headlines, a file with no trailing newline, a file that is only a preamble (no headlines), a deeply nested (level 1-6) file. Keep the corpus small (~10-20 files) — this is the harness's own test diet, not Story 2.5's ~100-file subset.
- **Line-ending-sensitive cases live as inline `&str` literals in the test file, not fixtures:** CRLF, mixed endings, and missing-trailing-newline bytes are not safe to commit as `.org` files without EOL protection. Additionally add `crates/orgsidian-parser/tests/fixtures/round_trip/*.org -text` to the root `.gitattributes` (mirror the narrow-scoping comment style of the existing `tests/perf-baselines/*.json` entry) so a future Windows checkout with `autocrlf` cannot silently rewrite fixture bytes and turn the gate into a liar.
- **Variance (record, do not edit epics.md):** the epic's "the full subset corpus" forward-references `fixtures/subset-pr.json`, which is Story 2.5's deliverable (its AC is *Given Story 2.4* — the corpus cannot exist yet). This story ships the harness + interim corpus; the full ~100-file subset plugs into the same `round_trip_subset` test in Stories 2.5/2.6. Write the corpus iteration so the fixture directory is easy to repoint/extend (directory-driven, not a hardcoded filename list).

### AC4 — proptest properties: second-serialization idempotence (epic) + arbitrary-input identity (stronger, recommended).

**Given** AC1-AC3 and `proptest` already workspace-pinned (`proptest = "1"`, resolves 1.11.0, in-tree since Story 1.18 via `orgsidian-core`),
**When** the property tests are written (in `tests/round_trip.rs`),
**Then**:

- **Epic property (mandatory):** strategies generate randomized headline content — levels 1..=6, optional TODO keywords (default config set), titles, tags, planning lines with random dates, property drawers, body text, nesting — **rendered to org text, then turned into `Headline`s via `analyze()`** (the only honest constructor: a hand-built `Headline` would need a hand-built `raw`, which is the strategy's render output anyway). Then: `serialize` → `analyze` the output → `serialize` again → assert the **second serialization is byte-identical to the first**. This is the epic's exact property (epics.md:825).
- **Arbitrary-input identity (recommended, stronger — implement unless it reveals a grammar-level blocker, in which case STOP and document):** for **any** `String` (proptest `".*"`-class strategy plus a `\PC*` Unicode-heavy variant), `serialize_document(&analyze(&s)?) == s`. `analyze` accepts every input (LD-41 lenience), so this property has no precondition — it enforces the AC2 tiling invariant against ERROR-region pathologies far beyond what handcrafted fixtures reach. If a counterexample surfaces from the vendored `scanner.c` signed-char indent bug (cumulative list indent ≥128 — `KNOWN_DIVERGENCES.md` entry 8), note that even ERROR trees must round-trip byte-faithfully under gap-absorption; the bug corrupts the *tree shape*, not the byte coverage.
- Follow the existing proptest house pattern from [`crates/orgsidian-core/tests/settings_round_trip.rs`](crates/orgsidian-core/tests/settings_round_trip.rs): `proptest!` macro, explicit `ProptestConfig { cases: 256, .. }`, named strategies. Budget: the whole parser test suite stays **<10s warm** (currently <1s with 59 tests; 256 cases × small docs parse in microseconds).
- `proptest = { workspace = true }` is added to `[dev-dependencies]` of `crates/orgsidian-parser/Cargo.toml` with a Story-2.4 comment — a dependency **edge** only, zero new crates in `Cargo.lock` (proptest 1.11.0 already resolved). This is the only dependency change this story may make.

### AC5 — Traceability + docs hygiene; zero-normalization posture documented.

**Given** the new module,
**When** doc-comments are written,
**Then**:

- `src/serializer.rs` carries **`//! Implements FR-2`** as the first doc-comment line (CONTRIBUTING §4 FR Traceability Discipline — the grep gate is `Implements FR-`).
- Module docs state the design contract: raw-region passthrough; the tiling invariant; why field-driven rendering is forbidden (lossy `title`/`properties`); and the **zero-normalization posture** — FR-2's "modulo trailing-newline normalization, documented" allowance is **not exercised**: serialization is exact, including trailing bytes (verified — trailing blank lines are inside the last section's span). Record as variance: PRD FR-2 says "documented in Settings"; no Settings surface exists and nothing needs documenting there since no normalization occurs — the module doc is the documentation.
- `lib.rs`: add `mod serializer;` + `pub use serializer::{serialize, serialize_document};` (lib.rs stays re-export-only, no logic); update the crate-header prose — "the round-trip serializer (Story 2.4, FR-2) builds on both surfaces" future-tense line becomes present-tense. Keep `//! Implements FR-1` line intact (FR-1 is the parse side; serializer.rs owns the FR-2 trace).
- Every new `pub` item carries `///` docs. `cargo doc -p orgsidian-parser --no-deps` succeeds without warnings.

### AC6 — Build, test, and supply-chain gates stay green.

**Given** all the above,
**When** the gates run,
**Then**:

- `cargo build -p orgsidian-parser --locked` + `cargo test -p orgsidian-parser --locked` green. Parser-crate baseline: 59 tests (1 anchor + 4 grammar + 24 semantic + 30 unit) — all unchanged and green; `round_trip.rs` adds the corpus test + ≥2 properties + any unit tests. Report exact counts in Completion Notes.
- `cargo test --workspace --locked` green (baseline: 104 passed / 11 ignored post-Story-2.3 review — no regressions).
- `cargo clippy --workspace --all-targets --locked` clean; `cargo fmt --check` clean; no `unwrap()`/`expect()` in library code (tests may); no `println!` in committed code.
- `cargo deny check licenses bans advisories` ok/ok/ok and `cargo audit` at the 18-allowed-warnings baseline: the only acceptable `Cargo.lock` delta is the proptest dev-dep edge (zero new crate versions). Anything beyond that: STOP and surface a decision-grade question; do not edit `deny.toml` silently.
- Sentinel check: `git status` shows `tests/anchor.rs`, `tests/grammar.rs`, `tests/semantic.rs`, `tests/fixtures/anchor.org`, `build.rs`, `grammar/` untouched.

### AC7 — Deferred-work hygiene.

**Given** the Story 2.3 stanza in [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md),
**When** this story completes,
**Then**:

- The **`Timestamp::end_time` single-field shape** item (owner "Story 2.4+") is annotated with this story's decision: the raw-passthrough serializer **never consumes `Timestamp` fields**, so the field split is NOT forced by 2.4; `raw` carries fidelity; the API-shape decision stays deferred to the Epic 4 timestamp-picker consumer.
- A `## Deferred from: code review of story-2.4 (YYYY-MM-DD)` stanza is pre-seeded at impl time (established 2.2/2.3 process pattern). Known candidates: mutation-aware serialization (edit one field → emit) → Epic 4 edit-application; corpus expansion → Story 2.5; any tiling edge the arbitrary-input property surfaces but is judged grammar-bug territory.

## Tasks / Subtasks

- [ ] **T1** — Re-verify the tiling probe data from Dev Notes §1 with a throwaway scratch test (the 2.2/2.3 `tests/zz_scratch.rs` pattern, deleted before commit): preamble/section tiling on blank-line, CRLF, no-trailing-newline, empty, ERROR-region inputs. Confirms nothing changed under you; costs minutes; the probe code is reproduced in Dev Notes §1. (AC2)
- [ ] **T2** — Add `raw: String` to `Headline` in `src/semantic/headline.rs`: populate in `build_section` from the existing own-region boundaries (links-scan region, lines 254-259); document field + invariant. Implement gap-absorption in `src/semantic/mod.rs` `analyze()` (position cursor over top-level pieces) and in the child walk, so `None`-section and ERROR-node bytes are never dropped. (AC2)
- [ ] **T3** — Create `src/serializer.rs`: `//! Implements FR-2` header, `serialize(headlines: &[Headline]) -> String` (recursive raw concat), `serialize_document(&Document) -> String` (preamble + headlines). Wire `lib.rs` (`mod serializer;` + re-exports + header prose present-tense). (AC1, AC5)
- [ ] **T4** — Build the interim corpus `tests/fixtures/round_trip/*.org` (~10-20 files: 15 LD-44 constructs + edge bucket per AC3); add the `.gitattributes` `-text` rule with scoping comment. (AC3)
- [ ] **T5** — Write `tests/round_trip.rs`: `round_trip_subset` corpus test (directory-driven, divergence diagnostics) + inline-literal cases for CRLF / mixed endings / missing trailing newline / empty input. (AC3)
- [ ] **T6** — Add `proptest = { workspace = true }` to parser `[dev-dependencies]` (Story-2.4 comment); write the epic idempotence property (generated org content → `analyze` → `serialize` → `analyze` → `serialize`, second == first) + the arbitrary-input identity property (`serialize_document(analyze(s)) == s` for any `String`), 256 cases each, settings_round_trip.rs house style. (AC4)
- [ ] **T7** — Docs hygiene: `///` on all new pub items; module-doc design contract incl. zero-normalization posture + variances; `cargo doc -p orgsidian-parser --no-deps` clean. (AC5)
- [ ] **T8** — Gates: `cargo build -p orgsidian-parser --locked`, `cargo test -p orgsidian-parser --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked`, `cargo fmt --check`, `cargo deny check licenses bans advisories`, `cargo audit`. Verify sentinel files untouched via `git status`. Report test-count delta + lockfile delta in Completion Notes. (AC6)
- [ ] **T9** — deferred-work.md: annotate the `Timestamp::end_time` item with the 2.4 decision; pre-seed the story-2.4 stanza. (AC7)
- [ ] **T10** — Commit + open PR. Commit title: `feat(parser): implement round-trip-faithful serializer (Story 2.4, closes #20)` — Conventional Commits scope `parser` per CONTRIBUTING §2. **NO** `Co-Authored-By` trailer, **NO** "Generated with Claude Code" footer, no AI-credit lines. PR body: (a) anchor + grammar + semantic tests byte-unchanged + green, (b) corpus + both properties present, `round_trip_subset` name locked for Story 2.6, (c) proptest dev-dep edge rationale + deny/audit nil delta, (d) tiling invariant + gap-absorption summary, (e) variances recorded (serialize_document, interim corpus, zero-normalization). (process)

## Dev Notes

### 1. Empirical ground truth: spans tile (verified 2026-06-10 at the pinned SHA, tree-sitter 0.26.9)

A scratch probe (throwaway `tests/zz_scratch.rs`, the 2.2/2.3 archaeology pattern) dumped `body`/`subsection` byte ranges for: simple two-section files, preamble + blank lines, missing trailing newline, trailing blank lines, CRLF, preamble-only, nested sections, blank lines between sections, the empty file, and leading blank lines. **Result: every case tiles 0..len exactly — zero gaps, zero tail bytes.** Sample: `"#+TITLE: t\n\nintro text\n\n* One\n\nbody\n\n\n* Two\n"` → `body 0..24`, `section0 24..38`, `section1 38..44` = len 44. Trailing blank lines land *inside* the last section's span; leading blank lines land in `body`; CRLF bytes are plain content. Within a section, named children also tile contiguously (`headline 0..4`, `plan 4..32`, `property_drawer 32..58`, `body 58..68`, child `section`s 68..86). Consequences:

- **Full byte-identity is achievable with zero trailing-newline normalization.** The FR-2 "modulo" allowance is a fallback you will not need on well-formed input. Do not normalize anything.
- The headline own-region (`section.start..first_child.start | section.end`) plus recursive children covers the section span with no gaps — `raw` + children concat = section bytes.
- The only byte-drop hazards are **pathological trees**: `build_section → None` (section without `headline` field inside ERROR regions) and root-level ERROR nodes that aren't `subsection` fields. Hence AC2's gap-absorption + AC4's arbitrary-input property as the enforcement mechanism. Re-run the probe yourself (T1) — `node.to_sexp()` + `byte_range()` dumping is the cheapest debugging tool you have.

### 2. Why raw-passthrough is the only design that can pass FR-2 (do not relitigate)

- `Headline::title` is **trimmed** (`headline.rs:128-132`) — trailing whitespace in headlines is an LD-44 edge-bucket case and would be destroyed by re-rendering.
- `Headline::properties` is the epic-mandated `HashMap<String, String>`: duplicate keys collapse last-wins, order unspecified. Story 2.3's module docs already warn: "round-trip fidelity comes from raw spans, never from re-emitting the map" (`semantic/mod.rs:22-27`). This story is where that promise is kept.
- Timestamps re-rendered from `chrono` fields would lose the exact day-name text, spacing, and the degenerate-range forms documented on `Timestamp::end_time`.
- The user story is "saved **without user-visible edits** → byte-identical" — there is no mutation in scope, so there is nothing a field-driven renderer could express that passthrough cannot.

### 3. Where the raw field goes and what it costs

- Populate `raw` in `build_section` (`src/semantic/headline.rs`) using the **same** own-region boundaries as the links scan — single source of truth for the region, no drift. The `source.get(..)` + `unwrap_or("")` defensive style is established in that file; follow it.
- Memory: `raw` roughly doubles owned-string cost per headline. The owned-data posture (and its LD-42 bounded-by-file-size rationale) was decided in Story 2.3's source-retention resolution — this is a continuation, not a new decision. Note it in Completion Notes; don't redesign.
- `Document` itself does NOT need to retain the full source: `preamble.text` (already owned, `mod.rs:110-125`) + headline `raw`s cover everything once gap-absorption exists. Avoid adding a `source: String` to `Document` — it would make `serialize_document` a placebo (returning a stored copy proves nothing).

### 4. Test-surface contracts that protect other stories

- **`round_trip_subset` is a public name contract:** Story 2.6 wires `cargo test -p orgsidian-parser round_trip_subset -- --test-threads=4` into `pr.yml` (epics.md:854). Cargo's filter is substring-based — the corpus test must carry that name today.
- `tests/anchor.rs` / `tests/grammar.rs` / `tests/semantic.rs` / `tests/fixtures/anchor.org` byte-unchanged. `semantic.rs` was verified free of struct literals at story-creation; if adding `raw` somehow breaks it, that's a design smell — stop and re-check, don't edit the sentinel.
- Every test asserts a real property (Story 1.9 anti-placebo discipline). The corpus test's failure output must localize the divergence (filename + byte offset + a short context window), or debugging a 50KB fixture mismatch becomes archaeology.

### 5. proptest specifics

- Workspace pin exists: root `Cargo.toml` line 78 (`proptest = "1"`, comment "First proptest use in the workspace. MIT/Apache-2.0."), resolved 1.11.0 in `Cargo.lock`, consumed by `orgsidian-core` dev-deps since Story 1.18. Your change is one `{ workspace = true }` dev-dep line — `cargo deny`/`audit` delta must be nil.
- House style reference: `crates/orgsidian-core/tests/settings_round_trip.rs` (`proptest::collection`, `proptest::option`, `proptest!` with `ProptestConfig { cases: 256, .. }`).
- For the idempotence property, generate **org text**, not structs: compose random headline lines (`"*".repeat(level) + " " + keyword? + title + tags?`), planning lines from random `NaiveDate`-valid dates, drawers, body paragraphs; join; `analyze()`; take `document.headlines`. Generating `Headline` literals directly would force you to hand-synthesize `raw` — which is the same render, with extra steps and a fake invariant.
- Keep generated text away from the scanner.c indent bug zone (don't generate >100 columns of cumulative list indent) for the *idempotence* property; the *identity* property intentionally has no such guard — byte coverage must survive ERROR trees.
- proptest writes failure-persistence files (`proptest-regressions/`) next to the test on failure. If one appears during development, commit it only if it pins a real fixed bug (house precedent: orgsidian-core has none committed).

### 6. Module/docs conventions (established, follow exactly)

- One concern per file, ~400-line rule; `lib.rs` stays re-export-only. `serializer.rs` should be small (~100-150 lines + unit tests) — recursion over `raw`/`children` plus docs.
- Doc-comment trace header is greppable: first line exactly `//! Implements FR-2`. CONTRIBUTING §4's future `tests/traceability.rs` gate parses for `Implements FR-NN` — this story makes FR-2 findable; do not also claim FR-2 in other files' headers (single trace owner per FR, matching the FR-2 mapping row).
- Internal tree walking uses `crate::tree_sitter` re-export types; `node.utf8_text(source.as_bytes())`; no new `tree-sitter` dep anywhere.

### 7. Variances (record in Completion Notes; do NOT edit epics.md / architecture.md / prd.md — epics.md is the GitHub-issues sync-source)

1. Epic signature `serialize(headlines: &[Headline]) -> String` kept verbatim, but byte-identity on whole files requires the preamble → `serialize_document(&Document)` added as the document-level entry (AC1).
2. Epic "the full subset corpus" forward-references Story 2.5's `fixtures/subset-pr.json` (whose own AC is *Given Story 2.4*) → interim in-repo corpus + the `round_trip_subset` harness now; full subset lands via 2.5/2.6 (AC3).
3. FR-2's "modulo trailing-newline normalization, documented (in Settings)" → normalization not exercised (exact serialization, verified); no Settings surface exists; posture documented in serializer module docs instead (AC5).
4. Epic "proptest strategies generate randomized headlines" → headlines generated via rendered-org-text + `analyze()`, the only constructor that produces a valid `raw` (AC4).

### Project Structure Notes

**Alignment with unified project structure:**

- `crates/orgsidian-parser/src/serializer.rs` — NEW; matches the architecture FR-2 mapping row verbatim (architecture.md:1045) and completes the crate role "tree-sitter-org wrapper + semantic AST + **serializer** (FR-1, FR-2)" (architecture.md:913). ✓
- `crates/orgsidian-parser/src/semantic/headline.rs` — UPDATE: `raw` field + population in `build_section`. Current state: 287 lines; `Headline` struct (lines 36-83) with 12 fields, all spans-based; own-region already computed for links (lines 254-259). What must be preserved: every existing field, the walker's panic-free lenience, the links-scan behavior. ✓
- `crates/orgsidian-parser/src/semantic/mod.rs` — UPDATE: gap-absorption in `analyze()`'s top-level walk (lines 127-132 today). What must be preserved: `analyze` signature, directive collection, preamble construction, `TodoConfig` resolution. ✓
- `crates/orgsidian-parser/src/lib.rs` — UPDATE: `mod serializer;` + re-exports + present-tense header prose. Preserve: `parse()` verbatim (anchor sentinel), `tree_sitter`/`chrono` re-exports, `Implements FR-1` line. ✓
- `crates/orgsidian-parser/Cargo.toml` — UPDATE: `[dev-dependencies] proptest = { workspace = true }`. Root `Cargo.toml`/workspace pins: NO change (proptest pin exists). ✓
- `crates/orgsidian-parser/tests/round_trip.rs` + `tests/fixtures/round_trip/` — NEW (epic-named test file). `.gitattributes` — UPDATE (one scoped `-text` rule). ✓
- READ-ONLY / MUST NOT CHANGE: `tests/anchor.rs`, `tests/grammar.rs`, `tests/semantic.rs`, `tests/fixtures/anchor.org`, `build.rs`, `src/grammar/`, `grammar/` (LD-48), `.github/workflows/*` (Story 2.6's turf), `docs/parser/KNOWN_DIVERGENCES.md` (append-only if a genuinely new divergence is found; none expected). ✓

**Detected conflicts / variances:** see Dev Notes §7. Additionally: LD-3's monorepo-era path framing is stale (established 2.2/2.3 finding — epics.md paths win); architecture.md is archival, do not edit.

### Testing Standards Summary

- Integration tests in `crates/orgsidian-parser/tests/*.rs`, auto-discovered. Post-2.4 expected: anchor (1) + grammar (4) + semantic (24) unchanged; `round_trip.rs` NEW (corpus test + ≥2 proptest properties + edge-case literals); unit tests in `src/serializer.rs`/`headline.rs` welcome for region math.
- Anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green, file byte-unchanged.
- Budget: parser suite <10s warm (proptest 256-case properties on tiny docs are sub-second; corpus files are small). Workspace baseline 104 passed / 11 ignored — must not regress.
- CI: macOS-arm64 + Ubuntu-LTS per PR, Windows + Arch nightly — no CI-config change in this story (the L0 gate is Story 2.6).

### Previous Story Intelligence (from Story 2.3)

- **The spans-for-2.4 promise was kept:** every semantic struct carries `Range<usize>` into the `analyze()` input; `Preamble.text` is already owned raw text. What 2.3 did NOT leave you: any raw text on `Headline` (spans only) — that's this story's AC2, anticipated by 2.3's AC1 note "load-bearing for Story 2.4."
- **Lenience contract:** `analyze()` returns `Ok` for any input — your arbitrary-input property leans on this; no preconditions needed.
- **Review-hardened edges you inherit for free:** multi-pipe `#+TODO:`, link word-boundaries, CLOCK malformed-line handling — none of it matters to the serializer (raw passthrough), which is exactly the point.
- **Process patterns that worked:** RED-first (write `round_trip.rs` against a stub `serialize`, watch it fail, then implement); s-expression/byte-range dumping before walker changes; pre-seeded deferred-work stanza; variance-recording instead of spec-editing; zero-new-deps default with STOP-and-ask.
- **Hygiene:** commit scope `parser`; `closes #20`; no AI-credit lines; cargo-audit baseline is 18 allowed warnings (gtk-rs/Tauri unmaintained set) — the count must not move.

### Git Intelligence Summary

`git log --oneline -6` at story-write: `733a9f3` 2.3 review fixes ← `1b26a79` 2.3 impl (closes #19) ← `7f308d8` 2.3 story file ← `2f93b5d` Merge PR #139 (2.2). Branch: `story/2.4-round-trip-serializer`, **stacked on the completed 2.3 branch** (2.3 commits are local to this lineage; the PR base may need 2.3's merge first — flag in the PR description if 2.3 isn't on main yet). Pattern per story: one story-file commit + one impl commit + one review-fixes commit, PR-merged. Only Stories 1.9/2.1/2.2/2.3 ever touched `crates/orgsidian-parser/` — no conflicting in-flight work.

### Latest Technical Information

- **proptest 1.11.0** (workspace-pinned `"1"`, already in `Cargo.lock` via orgsidian-core — zero new crates). Key API: `proptest!` macro, `ProptestConfig`, `prop::string::string_regex`, `prop::collection::vec`, `Strategy::prop_map`/`prop_flat_map` for composing the org-text generator. MIT/Apache-2.0. Docs: <https://docs.rs/proptest/1.11.0/>.
- **tree-sitter 0.26.9** via the crate re-export; relevant here: `Node::byte_range()`, `children_by_field_name`, `TreeCursor`. No version bumps anywhere ([[feedback_version_policy]]: latest-stable pins already satisfied; Tauri ecosystem untouched).
- **chrono 0.4.44** — read-only context: the serializer never formats chrono values (Dev Notes §2).
- Grammar pinned at `219c0b27…` (`nvim-orgmode/tree-sitter-org`); tiling behavior verified at this exact SHA (Dev Notes §1). Any future SHA bump re-runs the tiling probes (parser-owner SHA-review process, CONTRIBUTING §8).

### References

- Source story: [`epics.md:813-826`](_bmad-output/planning-artifacts/epics.md#L813-L826) — Story 2.4 user story + 4 ACs. Downstream name contract: [`epics.md:844-855`](_bmad-output/planning-artifacts/epics.md#L844) (Story 2.6 `round_trip_subset` invocation); corpus sequencing: [`epics.md:828-841`](_bmad-output/planning-artifacts/epics.md#L828) (Story 2.5 *Given Story 2.4*).
- Previous story: [`2-3-implement-semantic-layer-todo-cycling-drawer-types-timestamps-link-types.md`](_bmad-output/implementation-artifacts/2-3-implement-semantic-layer-todo-cycling-drawer-types-timestamps-link-types.md) — semantic API shape, spans contract, review findings, completion notes.
- PRD FR-2: [`prd.md:150-156`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L150) (byte-identical, "modulo trailing-newline normalization, documented in Settings"); trust contract framing [`prd.md:50`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L50); SM-4 corpus success metric [`prd.md:602`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L602).
- Architecture: FR-2 mapping row [`architecture.md:1045`](_bmad-output/planning-artifacts/architecture.md#L1045); crate role [`architecture.md:913`](_bmad-output/planning-artifacts/architecture.md#L913); three-level round-trip oracle (L0/L1 proptest/L2) [`architecture.md:85`](_bmad-output/planning-artifacts/architecture.md#L85); CI budgets [`architecture.md:523-524`](_bmad-output/planning-artifacts/architecture.md#L523); LD-44 subset criteria [`architecture.md:1228-1245`](_bmad-output/planning-artifacts/architecture.md#L1228); proptest "latest stable" [`architecture.md:188`](_bmad-output/planning-artifacts/architecture.md#L188); date/time convention ("only parser/serializer touch" the native form) [`architecture.md:761`](_bmad-output/planning-artifacts/architecture.md#L761).
- Real code this story modifies: [`crates/orgsidian-parser/src/semantic/headline.rs`](crates/orgsidian-parser/src/semantic/headline.rs) (own-region at 254-259, `Headline` at 36-83), [`crates/orgsidian-parser/src/semantic/mod.rs`](crates/orgsidian-parser/src/semantic/mod.rs) (`analyze` at 91-140), [`crates/orgsidian-parser/src/lib.rs`](crates/orgsidian-parser/src/lib.rs).
- proptest house pattern: [`crates/orgsidian-core/tests/settings_round_trip.rs`](crates/orgsidian-core/tests/settings_round_trip.rs); workspace pin: root [`Cargo.toml:77-78`](Cargo.toml#L77).
- Deferred-work items owned/decided here: [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) story-2.3 stanza — `Timestamp::end_time` (owner "Story 2.4+"); scanner.c indent bug breadcrumb (story-2.2 stanza).
- CONTRIBUTING: §2 commit scope `parser`, §4 FR traceability (`Implements FR-` grep gate + future `tests/traceability.rs`), §8 parser-owner SHA review.
- `.gitattributes` scoping precedent: root [`.gitattributes`](.gitattributes) (perf-baselines LF rule with narrow-scope rationale comment).
- [[feedback_version_policy]], [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]].

### Project Context Reference

- [`prds/prd-orgsidian-2026-05-19/prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) — FR-2 (§4.1), §3 trust-contract principles.
- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-32 (CI gates), LD-41 (lenience posture), LD-42 (large-vault memory posture), LD-44/LD-45 (corpus + oracle), LD-48 (vendoring, grammar READ-ONLY); implementation patterns + AI-agent rules.
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (Stories 2.1 → 2.8).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §2 commit scopes, §4 FR traceability, §8 parser ownership.
- [`docs/parser/KNOWN_DIVERGENCES.md`](docs/parser/KNOWN_DIVERGENCES.md) — 9 entries incl. the scanner.c indent bug (entry 8).
- [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) — story-2.2 + story-2.3 stanzas.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-06-10 — Story created (ultimate context engine analysis completed — comprehensive developer guide created; serializer design grounded in empirical span-tiling verification at the pinned grammar SHA). Status: ready-for-dev.
