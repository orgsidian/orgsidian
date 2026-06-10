# Story 2.3: Implement semantic layer (TODO cycling, drawer types, timestamps, link types)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 19

## Story

As the **user editing my `.org` vault**,
I want the parser to expose semantic types (TODO state, drawer kind, scheduled/deadline timestamps, link type) beyond raw syntax nodes,
So that FR-1 renders correctly and Epic 4+ can build TODO badges, timestamp pickers, and clickable links against a stable AST.

**Traces:** FR-1, LD-44 (subset corpus syntax-feature matrix).

## Scope Fence (read first)

This is the **semantic-layer** story: it turns the raw `tree_sitter::Tree` (Story 2.2) into typed, owned semantic structs (`Document`, `Headline`, `TodoState`, `Timestamp`, `Drawer`, `Link`, …) under `crates/orgsidian-parser/src/semantic/`. It is **not**:

- **NOT the serializer (Story 2.4).** No `serialize()`, no round-trip property tests, no `proptest`. The semantic structs MUST carry byte spans into the source (see AC1) so Story 2.4 *can* be round-trip-faithful, but emitting org text is out of scope.
- **NOT corpus/fixtures (Story 2.5).** No `tools/corpus-extractor`, no `fixtures/subset-pr.json`, no git-LFS corpus. Small in-repo samples (inline literals or co-located fixture files) are all this story needs.
- **NOT the CI gates (Stories 2.6/2.7).** No `pr.yml`/`nightly.yml` edits, no Emacs oracle. `KNOWN_DIVERGENCES.md` is *initialized* here (AC7); the LD-45 triage workflow that *feeds* it operationally arrives with Story 2.7.
- **NOT the CLI (Story 2.8).** No `orgsidian parse <file>` command, no `clap` work.
- **NOT editor/UI work (Epic 4+).** No rendering, no widgets, no incremental-reparse API, no cancellation API. The deferred-work item "no incremental-reparse path" stays deferred — this story's `analyze()` is a batch consumer; do **not** extend the `parse()` wrapper surface (no promotion of `grammar::language()` to `pub`, no `Parser` pooling).
- **NOT index work (Epic 3).** No SQLite, no `orgsidian-index` types. Other crates still take **zero** new deps on `orgsidian-parser` in this story.

The deliverable is exactly: a public semantic API in `crates/orgsidian-parser/src/semantic/` reachable from the crate root, the 15 LD-44 construct tests in `tests/semantic.rs`, and `docs/parser/KNOWN_DIVERGENCES.md` initialized with the verified grammar coverage gaps.

## Acceptance Criteria

### AC1 — Semantic module + public `Headline` API per the epic's named shape; owned data; spans kept.

**Given** Story 2.2's `parse(&str) -> Result<ParseTree, ParseError>` and `ParseTree::tree()` / `root_node()`,
**When** the semantic layer in [`crates/orgsidian-parser/src/semantic/`](crates/orgsidian-parser/src/semantic/) is implemented,
**Then**:

- The public API exposes `Headline` with **at minimum** the epic-named fields (epics.md:787): `todo_state: Option<TodoState>`, `tags: Vec<Tag>`, `scheduled: Option<Timestamp>`, `deadline: Option<Timestamp>`, `properties: HashMap<String, String>` — the epic's trailing `...` explicitly permits additional fields. Add at least: `level: u8` (stars count), `title: String` (item text minus the TODO keyword), `span: Range<usize>` (byte range of the headline's section in the source), and `children: Vec<Headline>` (nested per the grammar's `section`/`subsection` nesting). Recommended extras: `closed: Option<Timestamp>`, `drawers: Vec<Drawer>`, `clocks: Vec<ClockEntry>`, `links: Vec<Link>`.
- The entry point is `pub fn analyze(source: &str) -> Result<Document, ParseError>` in the semantic module (re-exported at the crate root), where `Document` carries at least `headlines: Vec<Headline>` (top-level only; nesting via `children`), the resolved `todo_config: TodoConfig` (AC2), and the document-level preamble (zeroth-section) content the dev judges useful. `analyze()` calls the existing `parse()` internally — callers hand over `&str` once; no second public path that takes a `ParseTree` is required (adding one as `pub(crate)` plumbing is fine).
- **Source-retention decision (resolves the deferred-work item assigned to this story):** semantic structs **own their data** (extracted `String`s + `Range<usize>` spans). No lifetime parameters in the public semantic surface; `ParseTree` internals stay untouched. Rationale: lifetimes would infect every downstream API (index, IPC, editor); the memory cost of owned extraction is bounded by file size and acceptable per the LD-42 large-vault posture. Record this decision in Completion Notes and tick the corresponding deferred-work entry (see T13).
- Every semantic struct that maps to a source region carries its byte span (`Range<usize>` into the `analyze()` input). This is load-bearing for Story 2.4 (serializer) and Epic 4 (editor decorations); do not skip spans because the AC field list doesn't name them.
- `HashMap<String, String>` for `properties` is the epic-mandated public shape. Known caveats — document both in the module docs, do not "fix" by changing the type: (a) duplicate property keys collapse (last-wins; note it), (b) iteration order is unspecified — Story 2.4 round-trips from raw spans, never by re-emitting the `HashMap`.

### AC2 — TODO state cycling parameterized by in-file `#+TODO:` directive.

**Given** AC1,
**When** TODO handling is implemented,
**Then**:

- `TodoState` models the keyword + its class: recommended shape `TodoState { keyword: String, done: bool }` (or equivalent enum-with-data; keep `PascalCase` type, owned `String`).
- A `TodoConfig` holds the **ordered** keyword sequence split into active/done sets. **Default (no directive):** active `TODO`, `NEXT`; done `DONE`, `WAITING` — cycling order `TODO → NEXT → DONE → WAITING → TODO` (wrap), exactly as the epic AC states.
- An in-file `#+TODO:` directive **replaces** the default for that document: parse the directive's value with org's pipe convention — keywords before `|` are active, after `|` are done; if no `|` is present, the **last** keyword is the done set (org-mode convention). Multiple `#+TODO:` lines accumulate (org allows several sequences); at minimum the single-directive case must work and multi-directive behavior must be decided + documented (accumulating is the org-faithful choice).
- Cycling is exposed as a pure function on the config, e.g. `TodoConfig::next(&self, current: Option<&str>) -> Option<&str>` (or equivalent): advances through the configured sequence in declaration order and wraps from the last keyword back to the first. No UI, no state — Epic 4 wires the keybinding.
- Headline TODO detection: the grammar does **not** split the TODO keyword from the title — `item` is a sequence of `expr` nodes (verified, see Dev Notes). The semantic layer takes the **first `expr` token of `item`** and matches it (case-sensitively, exact match) against the resolved `TodoConfig` keyword set; on match it becomes `todo_state` and is excluded from `title`. A first word like `Todo` or an unconfigured keyword is title text, not a state.
- `#+TODO:` directives are read from `directive` nodes (fields `name`/`value`, verified). Only directives named `TODO` (also accept `SEQ_TODO` and `TYP_TODO` aliases, which share the syntax) feed `TodoConfig`.

### AC3 — Drawer types distinguished: PROPERTIES, LOGBOOK, custom.

**Given** AC1,
**When** drawer handling is implemented,
**Then**:

- A `DrawerKind` (or equivalent) distinguishes `Properties`, `Logbook`, and `Custom(String)` — the epic's three classes.
- `:PROPERTIES:` drawers arrive as the grammar's dedicated `property_drawer` node with structured `property` children (fields `name`/`value`, verified) — they feed `Headline::properties` directly.
- `:LOGBOOK:` and custom drawers arrive as the generic `drawer` node (field `name`, contents = **unstructured `expr` soup** — verified). The semantic layer classifies by drawer name (case-insensitive `LOGBOOK` → `Logbook`; anything else → `Custom(name)`), and exposes the drawer's raw contents + span.
- CLOCK lines inside `:LOGBOOK:` are **not** structured by the grammar (verified: drawer contents are raw `expr` tokens). The semantic layer parses `CLOCK:` lines **itself** from the drawer contents text: `CLOCK: [ts]` (open), `CLOCK: [ts]--[ts] => H:MM` (closed, with duration). Expose `ClockEntry { start, end: Option<…>, duration: Option<…>, span }`. A malformed CLOCK line is not an error — leave it as raw drawer content (LD-41 posture: never crash on weird-but-real org).

### AC4 — Link types parsed into variants: `id:`, `[[wiki]]`, `[[file://]]`, `[[http://]]`, plain URLs.

**Given** AC1,
**When** link handling is implemented,
**Then**:

- A `Link` enum (or struct + `LinkType` enum) distinguishes at minimum: `Id` (`[[id:abc]]`), `Wiki` (`[[wiki-link]]`, `[[wiki-link][description]]` — optional description), `File` (`[[file://path]]` / `[[file:path]]`), `Url` (`[[http://…]]` / `[[https://…]]` bracketed), and plain in-text `http(s)://…` URLs. Each variant carries target, optional description, and span.
- **The grammar has no `link` node** (verified at the pinned SHA: bracketed links and bare URLs are `expr` soup inside `paragraph`). The semantic layer implements its **own inline scanner** over paragraph/section text: find `[[target]]` / `[[target][description]]` spans, classify by target prefix (`id:` → Id; `file:` → File; `http:`/`https:` → Url; no scheme → Wiki), plus a plain-URL scan for bare `http://`/`https://` runs. A hand-rolled scanner over the source slice is fine; if a regex crate seems tempting — STOP, it is a new dependency (see AC9); the bracket grammar is simple enough to hand-roll.
- Escaping/edge posture (document, don't over-engineer): nested `]` inside targets and zero-width cases follow the simplest correct reading (`]]` terminates; `][` splits target/description); divergences from org-element's full link grammar (e.g., angle links `<http://…>`, `~/` file expansion, link abbreviations) are **out of scope** — list them in `KNOWN_DIVERGENCES.md` (AC7).
- Links are collected per headline (`Headline::links` covering the headline's section) and for the document preamble — wherever the dev anchors them, the 15-construct tests (AC6) must be able to find `[[id:abc]]` etc. through the public API.

### AC5 — Timestamps: active/inactive, scheduled/deadline, recurring; chrono-backed.

**Given** AC1,
**When** timestamp handling is implemented,
**Then**:

- `Timestamp` carries at minimum: `active: bool` (`<…>` vs `[…]` — the grammar does **not** distinguish them structurally; check the first byte of the node span, verified), `date: chrono::NaiveDate`, `time: Option<chrono::NaiveTime>`, optional end time (for `<… 10:00-11:00>` — surfaces as the grammar's `duration` field, verified) and/or end date (for `<…>--<…>` ranges), `repeater: Option<Repeater>` (`+1w`, `++1m`, `.+1d` — kind, value, unit; the grammar exposes a `repeat` node, verified), optional `delay` (grammar `delay` node exists; parse if present, else record as gap), and `span`/raw text.
- `SCHEDULED:` / `DEADLINE:` (and `CLOSED:`) come from the grammar's `plan` → `entry` nodes (fields `name: entry_name`, `timestamp`, verified): match `entry_name` text to route into `Headline::scheduled` / `deadline` / `closed`. Both active and inactive forms parse into the same `entry` shape (verified) — the active/inactive flag comes from the delimiter.
- Unparseable date/time text inside an otherwise-shaped timestamp (e.g., month 13) must not panic and must not abort `analyze()` — skip the field (e.g., leave `scheduled: None`) and keep going. Tests may pin the exact lenient behavior chosen; document it.
- **`chrono` (workspace-pinned `0.4`, resolves 0.4.44 — already in `Cargo.lock` as a transitive dep, so zero new crates enter the tree) is added as a direct dependency** of `orgsidian-parser` via `[workspace.dependencies]`, per the architecture date/time convention ("Org file: native `<YYYY-MM-DD Day HH:MM>` — only parser/serializer touch it"; Rust-internal dates are chrono). Use `default-features = false, features = ["std"]` (NaiveDate/NaiveTime need no clock/locale features). This is the **only** new dependency this story may add — see AC9.

### AC6 — `tests/semantic.rs` covers every enumerated LD-44 construct with `semantic_*` tests.

**Given** AC1–AC5,
**When** the test surface is written,
**Then** [`crates/orgsidian-parser/tests/semantic.rs`](crates/orgsidian-parser/tests/semantic.rs) contains ≥1 test per enumerated construct, named `semantic_{construct}` (**variance, resolve as follows:** the epic writes `semantic_{construct_kebab}`, but kebab-case is not a valid Rust identifier — use the snake_case equivalent; record in Completion Notes). The epic's preamble says "14 LD-44 syntax constructs" but enumerates **15 bullets** — cover **all 15** (the count label is the variance, the bullet list is normative; do NOT edit epics.md, it is the GitHub-issues sync-source). Required tests, one per bullet, in epic order:

| # | Test name | Construct (epics.md:793-807) | Must assert (through the public semantic API) |
|---|---|---|---|
| 1 | `semantic_heading_levels_with_todo_states` | Heading levels 1-6 with `TODO`/`NEXT`/`DONE`/`WAITING` + custom via `#+TODO:` | `level` 1..=6 correct; each default keyword → `todo_state` with right `done` class; a `#+TODO: DRAFT \| PUBLISHED` doc recognizes `DRAFT`, demotes `TODO`-the-word to title text; cycling `next()` wraps per AC2 |
| 2 | `semantic_scheduled_timestamp` | `SCHEDULED:`; active + inactive | `scheduled` populated; `active` true for `<…>`, false for `[…]`; date/time values correct |
| 3 | `semantic_deadline_timestamp` | `DEADLINE:`; active + inactive | same as #2 on `deadline` |
| 4 | `semantic_clock_entries` | `CLOCK: [start]--[end] => HH:MM`; open + closed + ranged | closed entry has start+end+duration; open entry has start only; entries surface from `:LOGBOOK:` |
| 5 | `semantic_recurring_timestamps` | `<2026-05-19 Mon +1w>`, `+1d`, `+1m`, `+1y` | `repeater` kind/value/unit parsed on a SCHEDULED/plan-position timestamp |
| 6 | `semantic_drawer_types` | `:PROPERTIES:`; `:LOGBOOK:`; custom | `properties` map filled; `Logbook` vs `Custom("MYDRAWER")` classification |
| 7 | `semantic_inline_markup` | `*bold*` `/italic/` `=verbatim=` `~code~` `+strike+` `_underline_` | sample containing all six analyzes without error; headline/paragraph structure intact (markup is NOT semantically modeled — assert non-crash + documented gap; see AC7) |
| 8 | `semantic_links` | `[[id:abc]]`, `[[wiki-link]]`, `[[wiki-link][description]]`, `[[file://path]]`, plain `http://…` | each form classified into its variant; description captured; plain URL found |
| 9 | `semantic_lists` | `-`, `+`, `1.`, `- [ ]` / `- [X]` | sample analyzes without error and without tree `ERROR` nodes (lists are grammar-modeled but outside this story's semantic surface — assert non-crash; checkbox semantics are a documented gap) |
| 10 | `semantic_tables` | simple + separator row + `#+TBLFM:` | analyzes without error; formula line present in source span (tables grammar-modeled; outside semantic surface — non-crash) |
| 11 | `semantic_block_elements` | `#+BEGIN_SRC`, `#+BEGIN_QUOTE`, `#+BEGIN_EXAMPLE`, `#+BEGIN_VERSE` | all four analyze without error (blocks grammar-modeled; outside semantic surface — non-crash) |
| 12 | `semantic_inline_latex` | `$…$`, `\(…\)`, `\[…\]` | analyzes without error (NOT grammar-modeled — expr soup; non-crash + AC7 gap entry) |
| 13 | `semantic_footnotes` | `[fn:N]`, `[fn::inline]` | definition + inline ref analyze without error (`fndef` is grammar-modeled; inline refs are not — AC7 gap) |
| 14 | `semantic_citations` | `[cite:@key]` | analyzes without error (NOT grammar-modeled; AC7 gap) |
| 15 | `semantic_tags` | `:tag:`, `:tag1:tag2:` | `tags` vec correct for single + multi; tag text without colons |

- "Analyzes without error" means `analyze(sample)` returns `Ok` and the surrounding headline structure (level/title) is still correct — these tests are regression tripwires for the semantic walker against constructs it must *tolerate*, not placebo greens: each must also assert at least one real property (headline title intact, construct text present in the right span, etc.). For constructs marked non-crash, additionally assert the documented gap is real where cheap (e.g., links test already proves the scanner works — markup test may assert the raw markers survive in `title`/body text).
- Samples: inline `&str` literals per test (preferred — each is small), or co-located `tests/fixtures/*.org` read via `env!("CARGO_MANIFEST_DIR")` if any literal gets unwieldy. No corpus machinery.
- `tests/anchor.rs`, `tests/fixtures/anchor.org`, and `tests/grammar.rs` stay **byte-for-byte unchanged** (anchor is the cross-story sentinel; grammar.rs is Story 2.2's surface — this story adds files, it does not edit them).

### AC7 — `docs/parser/KNOWN_DIVERGENCES.md` initialized with verified coverage gaps.

**Given** the constructs above,
**When** the doc is created (NEW directory `docs/parser/`),
**Then** it contains at minimum, each with construct, expected org-mode/Emacs behavior, observed tree-sitter-org behavior at the pinned SHA (`219c0b27…`), Orgsidian's chosen behavior, and status/owner:

1. **Links not modeled as named nodes** — bracketed links + bare URLs are `expr` soup; Orgsidian's semantic layer does its own inline scan (this story). Divergence risk vs org-element's full link grammar (angle links, abbreviations) noted.
2. **Inline markup not modeled** (`*bold*` etc. → `expr` soup) — semantic layer does not expose emphasis; Epic 4 rendering will need its own inline pass; round-trip unaffected (raw text preserved).
3. **Inline LaTeX not modeled** (`$…$`, `\(…\)`, `\[…\]`); only `\begin{…}` environments get `latex_env` nodes.
4. **Citations (`[cite:@key]`) not modeled.**
5. **Inline footnote references not modeled** (only line-start `[fn:N]` definitions get `fndef`).
6. **CLOCK lines inside drawers unstructured** — generic `drawer` contents are raw `expr` tokens; Orgsidian parses CLOCK lines textually (this story).
7. **Timestamps in body paragraphs are not `timestamp` nodes** — only plan-position (line after headline) and entry contexts produce structured timestamps (verified); body timestamps are `expr` soup.
8. **Vendored `scanner.c` signed-char list-indent bug** — cumulative indent ≥128 columns misparses into `ERROR` nodes (carried over from the Story 2.2 review; deferred-work.md has the breadcrumb; LD-48 forbids local grammar edits).

Plus anything new the dev discovers while implementing (each AC6 sample that produces unexpected node shapes is a candidate). The doc states its LD-45 role: from Story 2.7 onward, Emacs-oracle divergences land here per the triage workflow.

### AC8 — Traceability + docs hygiene.

**Given** the new module family,
**When** doc-comments are written,
**Then**:

- `src/semantic/mod.rs` carries a module header with **`//! Implements FR-1`** (semantic layer share of FR-1) per FR Traceability Discipline (CONTRIBUTING §4). The crate-root header in `lib.rs` is updated to drop "Story 2.3 builds on this" future-tense prose if present and mention the semantic module; keep its existing `Implements FR-1` line.
- Public semantic items carry `///` doc comments (this is a public API surface other crates will consume; the architecture requires docs "encouraged" for non-plugin crates — for this surface treat them as required: every `pub` type + method).
- `cargo doc -p orgsidian-parser --no-deps` succeeds without warnings.

### AC9 — Build, test, and supply-chain gates stay green.

**Given** all the above,
**When** the gates run,
**Then**:

- `cargo build -p orgsidian-parser --locked` + `cargo test -p orgsidian-parser --locked` green. Parser-crate test delta: anchor (1) + grammar (4) unchanged; `semantic.rs` adds ≥15. Report the exact count in Completion Notes.
- `cargo test --workspace --locked` green (baseline: 50 passed / 11 ignored post-Story-2.2 review — no regressions).
- `cargo clippy --workspace --all-targets --locked` clean; `cargo fmt --check` clean.
- No `unwrap()`/`expect()` in library code (tests may); no `println!` in committed code.
- `cargo deny check licenses bans advisories` + `cargo audit`: the **only** acceptable delta is `chrono` becoming a direct dependency (already in-tree transitively at 0.4.44; MIT/Apache-2.0 — no new license exception expected). If deny/audit flags anything beyond that, STOP and surface a decision-grade question; do not edit `deny.toml` silently.
- `Cargo.lock` delta limited to dependency-edge bookkeeping for chrono (no new crate versions expected; `default-features = false` may *drop* transitive features — fine).

## Tasks / Subtasks

- [x] **T1** — Add `chrono = { version = "0.4", default-features = false, features = ["std"] }` to root `[workspace.dependencies]` with a Story-2.3 comment (mirror the existing dependency comment style); add `chrono = { workspace = true }` to `crates/orgsidian-parser/Cargo.toml`. Verify `cargo deny`/`audit` delta is nil beyond the new direct edge. (AC5, AC9)
- [x] **T2** — Scaffold `src/semantic/` (one concern per file, ~400-line rule): `mod.rs` (module header `//! Implements FR-1`, re-exports, `analyze()` entry), `headline.rs` (`Headline`, walker over `section`/`headline` nodes: stars→level, first-item-expr→TODO match, remaining item text→title, `tag_list`→tags), `todo.rs` (`TodoState`, `TodoConfig`, default set, `#+TODO:` directive parsing with pipe convention, `next()` cycling), `timestamp.rs` (`Timestamp`, `Repeater`; plan/entry routing for SCHEDULED/DEADLINE/CLOSED; active/inactive from delimiter byte; chrono date/time parsing — **the FR-9 mapping names this exact file**), `drawer.rs` (`DrawerKind`, `Drawer`, property_drawer→properties map, CLOCK-line parsing from LOGBOOK contents into `ClockEntry`), `link.rs` (`Link` variants + the inline `[[…]]`/plain-URL scanner). Wire `pub mod semantic;` in `lib.rs` (lib.rs stays re-export-only — no logic). (AC1–AC5)
- [x] **T3** — Implement the tree walk: `analyze(source)` → `parse(source)?` → walk `root_node()` with a `TreeCursor` (`orgsidian_parser` re-exports `tree_sitter` — internal code uses `crate::tree_sitter` types directly); extract text via `node.utf8_text(source.as_bytes())` (source stays alive inside `analyze` — the owned-extraction design makes the keep-the-source contract internal). Build owned structs with spans. Panic-free: malformed sub-constructs degrade to raw text/`None`, never `Err`/panic (LD-41 posture; the only `Err` path is `parse()`'s own defensive errors). (AC1)
- [x] **T4** — Implement TODO config + cycling per AC2 (default `TODO NEXT | DONE WAITING`; in-file directive replaces; pipe convention; no-pipe → last keyword done; `next()` wraps). (AC2)
- [x] **T5** — Implement drawer classification + CLOCK-line parsing per AC3 (`CLOCK: [ts]` open; `CLOCK: [ts]--[ts] => H:MM` closed; tolerate malformed lines as raw content). (AC3)
- [x] **T6** — Implement the link scanner per AC4 (bracket scan + prefix classification + plain-URL scan; hand-rolled, no regex dep). (AC4)
- [x] **T7** — Implement `Timestamp` per AC5 (active flag from first byte; chrono `NaiveDate`/`NaiveTime`; repeater `+`/`++`/`.+` with unit `h/d/w/m/y`; time-range via grammar `duration` field; lenient on unparseable values). (AC5)
- [x] **T8** — Write `tests/semantic.rs`: the 15 `semantic_*` tests per the AC6 table, each with a small inline sample and real assertions (no placebo greens). Do NOT touch `tests/anchor.rs` / `tests/grammar.rs` / `tests/fixtures/anchor.org`. (AC6)
- [x] **T9** — Create `docs/parser/KNOWN_DIVERGENCES.md` with the 8 verified entries (AC7) + any new findings; state its LD-45 role. (AC7)
- [x] **T10** — Docs hygiene: `///` on all public semantic items; update `lib.rs` crate header prose; `cargo doc -p orgsidian-parser --no-deps` clean. (AC8)
- [x] **T11** — Gates: `cargo build -p orgsidian-parser --locked`, `cargo test -p orgsidian-parser --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked`, `cargo fmt --check`, `cargo deny check licenses bans advisories`, `cargo audit`. Report test-count delta + chrono deny/audit posture in Completion Notes. (AC9)
- [x] **T12** — Verify anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green, `git status` shows `tests/anchor.rs`, `tests/grammar.rs`, `tests/fixtures/anchor.org`, `build.rs`, `grammar/` untouched. (AC6)
- [x] **T13** — deferred-work.md: append the pre-seeded `## Deferred from: code review of story-2.3 (YYYY-MM-DD)` stanza (candidates: inline-markup/citation/LaTeX semantic modeling → Epic 4 consumer decides; link-grammar completeness vs org-element — angle links, abbreviations; multi-`#+TODO:`-sequence cycling edge semantics; body-paragraph timestamp extraction). Also **annotate the two Story-2.2 deferred items this story resolves/decides**: `ParseTree` source-retention (resolved: owned semantic structs — AC1) and incremental-reparse (explicitly NOT taken: no editor consumer yet — Scope Fence). (process hygiene)
- [x] **T14** — Commit + open PR. Commit title: `feat(parser): implement semantic layer (Story 2.3, closes #19)` — Conventional Commits scope `parser` per CONTRIBUTING §2. **NO** `Co-Authored-By` trailer, **NO** "Generated with Claude Code" footer, no AI-credit lines. PR body: (a) anchor + grammar tests unchanged + green, (b) 15/15 construct tests present, (c) chrono direct-dep rationale + deny/audit nil delta, (d) KNOWN_DIVERGENCES.md initialized, (e) source-retention decision recorded. (process)

### Review Findings (adversarial code review, 2026-06-10)

Three parallel layers (Blind Hunter — diff only; Edge Case Hunter — diff + read access; Acceptance Auditor — diff + spec + context docs). 26 raw findings → deduplicated and triaged: 0 decision-needed, 18 patch (all applied), 2 defer, 4 dismissed as noise/false-positive. Auditor verdict: AC1–AC9 all PASS pre-fix; gates re-executed by the review session (see notes below).

- [x] [Review][Patch] Multi-pipe `#+TODO: A | B | C` registered a literal `|` done-keyword reachable via classify/keywords/next [src/semantic/todo.rs:151] — stray `|` tokens now dropped; pinned by `extra_pipes_are_not_keywords`
- [x] [Review][Patch] Plain-URL scan had no left word boundary (`xhttp://foo` matched mid-word) [src/semantic/link.rs:68] — boundary guard (start-of-text or non-alphanumeric predecessor); pinned by `plain_url_requires_word_boundary`
- [x] [Review][Patch] Degenerate `http://` (empty remainder after scheme, e.g. `http://.` post-trim) reported as a Plain link [src/semantic/link.rs:160] — scheme-only URLs rejected; pinned by `scheme_alone_is_not_a_link`
- [x] [Review][Patch] An unterminated `[[` swallowed following lines/paragraphs into one link span [src/semantic/link.rs:96] — newline now aborts the bracket candidate (multi-line links documented out of scope); pinned by `bracket_links_do_not_cross_newlines`
- [x] [Review][Patch] `CLOCK: [ts]--garbage` was misread as an open/running entry [src/semantic/drawer.rs:97] — a `--` with unparseable right side now makes the whole line malformed (stays raw content); pinned by `range_with_unparseable_end_is_malformed_not_open`
- [x] [Review][Patch] Durations accepted signed/out-of-range components (`=> -1:30` → negative TimeDelta; `1:-5` → 55 min; `1:99` → 159 min) [src/semantic/drawer.rs:133] — unsigned parse + minutes ≤ 59; pinned by `nonsense_durations_are_rejected`
- [x] [Review][Patch] Zero-value repeater/delay (`+0d`) parsed into a `Repeater` that would loop any downstream repeat-advance math [src/semantic/timestamp.rs:252] — zero intervals rejected as stray text; pinned by `zero_value_intervals_are_stray_text`
- [x] [Review][Patch] Range second half parsed with throwaway `offset 0` (garbage span trap) [src/semantic/timestamp.rs:121] — real offset now passed
- [x] [Review][Patch] Epic-enumerated repeater literals `+1m`/`+1y` never tested verbatim (test substituted `++1m`/`.+1y`) [tests/semantic.rs `semantic_recurring_timestamps`] — test now covers all four epic literals plus the `++`/`.+` kinds (6 cases)
- [x] [Review][Patch] `CLOSED:` plan-entry routing never integration-tested [tests/semantic.rs] — added `semantic_closed_timestamp`
- [x] [Review][Patch] Tautological assertion in `semantic_preamble_and_empty_document` (`src[span] == text` true by construction) [tests/semantic.rs] — replaced with independent literal assertions
- [x] [Review][Patch] Lowercase `#+todo:` directive-name acceptance (recorded AC2 extension) unpinned by tests [tests/semantic.rs] — added `semantic_lowercase_todo_directive_name`
- [x] [Review][Patch] Directive inertness inside block/drawer contents unpinned (hijack false-positive, see dismissed) [tests/semantic.rs] — added `semantic_directives_inside_blocks_and_drawers_are_inert`
- [x] [Review][Patch] `Drawer::contents` doc contradicted the Properties-drawer construction; empty-drawer `contents_span` fallback (empty range at drawer end) undocumented [src/semantic/drawer.rs:36-49] — both contracts now documented
- [x] [Review][Patch] `Headline::level` sentinels (0 = missing `stars` in ERROR regions; >255 saturates) undocumented [src/semantic/headline.rs:37-42] — documented
- [x] [Review][Patch] `end_time` precedence on degenerate mixed ranges + silent degrade of unparseable `--` tails undocumented [src/semantic/timestamp.rs] — documented on the field and `parse_at`; degrade pinned by `unparseable_range_tail_degrades_to_first_half`
- [x] [Review][Patch] Link-scheme case-sensitivity posture (org-faithful lowercase types) undocumented [src/semantic/link.rs module docs + KNOWN_DIVERGENCES #1] — documented; pinned by `scheme_matching_is_case_sensitive`
- [x] [Review][Patch] Link-shaped text inside verbatim contexts (SRC/EXAMPLE blocks, drawer contents, property values) reported as links with no documentation [src/semantic/headline.rs `links` + KNOWN_DIVERGENCES #1] — documented (structural exclusion deferred, see below)
- [x] [Review][Defer] Link scan does not exclude verbatim regions (structural fix: subtract block/drawer node ranges) [src/semantic/headline.rs:244-253] — deferred to Epic 4 link navigation; harmless to round-trip
- [x] [Review][Defer] `Timestamp::end_time` single-field shape conflates time-ranges and date-range second halves [src/semantic/timestamp.rs] — API-shape decision deferred to Story 2.4+/Epic 4 consumer; precedence documented, `raw` carries fidelity

Dismissed (4): TODO-config hijack via directives inside quoted/example blocks or drawers — **false positive**, empirically verified at the pinned SHA (block/drawer contents are `expr` soup, no `directive` nodes; now pinned by the inertness regression test); plain-URL `)` trimming heuristic — documented posture (KNOWN_DIVERGENCES #1); T7 "grammar `duration` field" letter-vs-implementation mismatch — disclosed in Completion Notes, behavior-equivalent (text parser route); auditor's non-re-executed gates — re-executed green by the review session.

Post-fix gates (review session, 2026-06-10): `cargo test -p orgsidian-parser --locked` 59 passed (1 anchor + 4 grammar + 24 semantic + 30 unit; +12 review tests); `cargo test --workspace --locked` 104 passed / 11 ignored; clippy clean; `cargo fmt --check` clean; `cargo doc -p orgsidian-parser --no-deps` no warnings; `cargo deny check licenses bans advisories` ok/ok/ok; `cargo audit` 18 allowed warnings (baseline unchanged); anchor/grammar/build.rs/grammar/ still byte-untouched.

## Dev Notes

### Critical context the dev agent must internalize

1. **The grammar gives you structure, not semantics — and far less structure than you'd hope.** Empirically verified at the pinned SHA (`219c0b27…`, tree-sitter 0.26.9) by dumping s-expressions for every LD-44 construct:
   - `headline` → fields `stars`, `item` (a flat sequence of `expr` tokens — **the TODO keyword is NOT split out**), `tags: tag_list` (with `tag` children — these ARE structured).
   - `plan` (the line(s) right after a headline) → `entry` nodes with fields `name: entry_name` (`SCHEDULED`, `DEADLINE`, `CLOSED`, anything `\p{L}+`) and `timestamp` (fields `date`, `day`, `time`?, `repeat`, `delay`, `duration`). Both `<active>` and `[inactive]` parse identically — **only the delimiter byte distinguishes them**.
   - `directive` → fields `name` (e.g. `TODO`) and `value` (sequence of `expr` tokens incl. the `|` pipe). `#+TODO: TODO NEXT | DONE WAITING` parses cleanly this way.
   - `property_drawer` → structured `property` children with `name`/`value` fields. **But** `:LOGBOOK:` and custom drawers → generic `drawer` with `name` + `contents` of raw `expr` soup: **CLOCK lines are unstructured text** — you parse them yourself.
   - `list`/`listitem` (fields `bullet`, `checkbox` with `status`), `table`/`row`/`cell` + `formula` (`#+TBLFM:`), `block` (fields `name`, `parameter`, `contents`, `end_name`), `latex_env`, `fndef` (fields `label`, `description`) — all modeled; all **outside** this story's semantic surface (tolerate + non-crash tests).
   - **NOT modeled at all** (pure `expr` soup inside `paragraph`): bracketed links, bare URLs, inline markup (all six kinds), inline LaTeX (`$…$`, `\(…\)`, `\[…\]`), citations `[cite:@key]`, inline footnote references. A standalone timestamp in body text is also just exprs — `timestamp` nodes appear only in plan/entry context.
   The complete named-node inventory (98 kinds) is in `grammar/src/node-types.json`; rule shapes in `grammar/src/grammar.json`. When in doubt, dump `node.to_sexp()` on a scratch sample *before* writing walker code — the shapes above were verified that way and it is the cheapest debugging tool you have.
2. **Consume the Story 2.2 surface exactly as designed for you.** `parse(&str) -> Result<ParseTree, ParseError>` + `ParseTree::tree()` / `root_node()` + the crate-root `pub use tree_sitter;` re-export exist precisely so this story takes **no direct `tree-sitter` dependency** and walks `tree_sitter::Node`/`TreeCursor` through the re-export. Do not touch `parse()`'s signature (anchor sentinel — `tests/anchor.rs` byte-unchanged is non-negotiable), do not promote `grammar::language()` to `pub`, do not add incremental/cancellation APIs (deferred until a real editor consumer exists — explicitly out of scope per the deferred-work owner notes).
3. **Node text resolves only against the exact source passed to `parse()`** (`node.utf8_text(source.as_bytes())`, byte offsets) — the documented keep-the-source contract. The owned-extraction design (AC1) keeps this contract *internal to `analyze()`*: by the time `Document` is returned, every string is owned and every span indexes the caller's original `source`. This is the Story-2.3-assigned resolution of the source-retention deferred item — record it.
4. **TODO keyword matching is config-driven and case-sensitive.** Org keywords are conventionally uppercase; `Todo` as a first word is title text. The default set is the epic-mandated `TODO NEXT | DONE WAITING` (note: richer than vanilla org's `TODO | DONE` — this is Orgsidian's deliberate default). An in-file `#+TODO:` replaces it. Cycling order = declaration order with wrap.
5. **`HashMap` caveat is documented, not fixed.** The epic's `properties: HashMap<String, String>` loses duplicate keys and order. Story 2.4 round-trips from raw spans, so this is acceptable — but say so in the module docs so nobody "helpfully" rebuilds property drawers from the map.
6. **Panic-free, lenient analysis.** `analyze()` must return `Ok` for any input `parse()` accepts (which is: everything). Malformed constructs degrade gracefully (raw text, `None` fields). No `unwrap`/`expect` in library code; clippy + the AI-agent rules treat violations as review blocks. ERROR/MISSING nodes in the tree are tolerated (the wrapper's documented posture); the walker must not assume well-formedness.
7. **chrono is the single permitted new dependency.** Already in `Cargo.lock` (0.4.44, transitive) so the supply-chain delta is one new *edge*, zero new *crates*. `default-features = false, features = ["std"]` gives `NaiveDate`/`NaiveTime` without clock/wasm baggage. Anything else that looks needed (regex? STOP — hand-roll the link scanner; rstest? not in the workspace — plain `#[test]` functions; proptest? Story 2.4's tool) is a decision-grade stop-and-ask.
8. **File-size discipline:** one concern per file, split at ~400 lines. The suggested `semantic/{mod,headline,todo,timestamp,drawer,link}.rs` layout maps one file per AC concern; `timestamp.rs` is the architecture-mandated name (FR-9 mapping: `orgsidian-parser/src/semantic/timestamp.rs`). `lib.rs` stays re-export-only per the crate-organization convention.
9. **Don't chase stale paths.** LD-3 names the semantic layer at `@orgsidian/core/src/parser/semantic/` — stale monorepo-era path; the real location is `crates/orgsidian-parser/src/semantic/` (epics.md is authoritative here; same variance Story 2.2 flagged). The FR-1 mapping row naming `orgsidian-parser/src/grammar.rs` is similarly approximate (it's `src/grammar/mod.rs`). architecture.md is archival — do not edit it.
10. **The 15-vs-14 count and the kebab-case test names are wording variances, not decisions to escalate.** Cover all 15 bullets; use snake_case `semantic_*` names; record both variances in Completion Notes; do not edit epics.md mid-epic (GitHub-issues sync-source — established Story 2.1/2.2 rule).

### Project Structure Notes

**Alignment with unified project structure:**
- `crates/orgsidian-parser/src/semantic/` — NEW module family (mod, headline, todo, timestamp, drawer, link). Matches the architecture crate role "tree-sitter-org wrapper + **semantic AST** + serializer (FR-1, FR-2)" — this story delivers the middle third. ✓
- `crates/orgsidian-parser/src/lib.rs` — UPDATE (add `pub mod semantic;` + re-exports + header prose touch-up). Currently 92 lines: crate header (`Implements FR-1`), `pub use tree_sitter;`, `ParseTree { tree }` + `root_node()`/`tree()` accessors, `ParseError { Grammar, NoTree }`, `parse()`. **What must be preserved:** the `parse()` signature and behavior verbatim, the keep-the-source doc contract, the re-export. **What changes:** module wiring + header prose only — lib.rs stays logic-free. ✓
- `crates/orgsidian-parser/Cargo.toml` — UPDATE (add `chrono = { workspace = true }` with a Story-2.3 comment). Root `Cargo.toml` — UPDATE (`[workspace.dependencies]` chrono pin). ✓
- `crates/orgsidian-parser/tests/semantic.rs` — NEW; the epic-named test file. ✓
- `docs/parser/KNOWN_DIVERGENCES.md` — NEW (new `docs/parser/` directory; the LD-45 path is exactly `docs/parser/KNOWN_DIVERGENCES.md`). ✓
- READ-ONLY / MUST NOT CHANGE: `tests/anchor.rs`, `tests/fixtures/anchor.org`, `tests/grammar.rs`, `build.rs`, `src/grammar/` (wrapper module), `grammar/` (vendored submodule — LD-48 forbids local edits). ✓

**Detected conflicts / variances (with rationale):**
- Epic "14 constructs" label vs 15 enumerated bullets — bullets normative; cover all 15 (AC6). No epics.md edit.
- Epic `semantic_{construct_kebab}` naming vs Rust identifier rules — snake_case equivalents (AC6). No epics.md edit.
- Epic `Headline { …, ... }` ellipsis — extra fields (level/title/span/children/…) are explicitly permitted and required for downstream stories (AC1).
- LD-3 stale path + FR-1 mapping `grammar.rs` approximation — noted above; archival, no edit.
- Epic says "unit tests in `tests/semantic.rs`" — in cargo terms `tests/` holds *integration* tests; the epic's file path wins (matches the project convention `crates/<crate>/tests/<topic>.rs`). Co-located `#[cfg(test)]` unit tests inside `src/semantic/*.rs` are additionally welcome for scanner internals.

### Testing Standards Summary

- Integration tests under `crates/orgsidian-parser/tests/*.rs`, auto-discovered. Post-2.3 expected: `anchor.rs` (1, unchanged) + `grammar.rs` (4, unchanged) + `semantic.rs` (≥15, NEW).
- Anchor sentinel: `cargo test -p orgsidian-parser --test anchor --locked` green with the file byte-unchanged.
- Every test asserts at least one real semantic property — no placebo greens (Story 1.9 discipline). Non-crash tests still pin headline structure or construct presence.
- Runtime budget: `cargo test -p orgsidian-parser --locked` < 5s warm (samples are tiny; tree-sitter parses them in microseconds).
- Workspace baseline: 50 passed / 11 ignored (post-2.2-review). CI matrix: macOS-arm64 + Ubuntu-LTS per PR; Windows + Arch nightly. No CI-config change in this story.

### Previous Story Intelligence (from Story 2.2)

- **The consumption path was pre-built for you:** `ParseTree::tree()`, `root_node()`, and `pub use tree_sitter;` were added in 2.2 *specifically* so 2.3 needs no own tree-sitter dep — the review even reworded docs to clarify "the raw tree IS the API."
- **Review corrections worth knowing:** `Tree`/`Parser`/`Language` are all `Send + Sync` at 0.26.9 (a compile-guard test in `grammar.rs` pins this); `parse()` is total — both `ParseError` arms are defensive and never fire in practice.
- **Two deferred items have this story's name on them:** source-retention design (AC1 resolves it: owned structs) and incremental-reparse (explicitly NOT taken — no editor consumer yet; leave deferred). A third (parser-per-call allocation) stays deferred unless profiling screams.
- **Known grammar landmine:** `scanner.c` signed-char list-indent bug — cumulative indent ≥128 columns yields ERROR nodes on valid org. Don't burn time "fixing" walker bugs that are actually this; it belongs in KNOWN_DIVERGENCES.md (AC7 entry 8).
- **Process patterns that worked:** RED-first test writing (2.2 wrote `tests/grammar.rs` against the stub, watched it fail, then implemented); s-expression dumping for grammar archaeology; pre-seeding the deferred-work stanza at impl time; variance-recording instead of spec-editing; zero-new-deps as default posture with STOP-and-ask on surprises.
- **Hygiene:** scope `parser`, `closes #19`, no AI-credit trailers/footers; baseline 18 allowed cargo-audit warnings (gtk-rs/Tauri unmaintained baseline) — count must not move.

### Git Intelligence Summary

`git log --oneline -5` at story-write: `2f93b5d` Merge PR #139 (Story 2.2) ← `9c3b9c8` review fixes ← `6424a05` Story 2.2 impl ← `8201dad` Merge PR #138 (Story 2.1) ← `f58c5f3` 2.1 review fixes. Pattern: one impl commit + one review-fixes commit per story, PR-merged; only Stories 1.9/2.1/2.2 ever touched `crates/orgsidian-parser/` — no in-flight branch conflicts. This branch: `story/2.3-semantic-layer`.

### Latest Technical Information

- **`tree-sitter` 0.26.9** (pinned via workspace `"0.26"`; consumed through the `orgsidian_parser::tree_sitter` re-export). Walker surface: `Node::kind()`, `Node::child_by_field_name()`, `Node::children_by_field_name()`, `Node::utf8_text(&[u8])`, `Node::byte_range()`, `Tree::walk()` → `TreeCursor` (`goto_first_child`/`goto_next_sibling`/`goto_parent`). Docs: <https://docs.rs/tree-sitter/0.26.9/>.
- **`chrono` 0.4.44** (in `Cargo.lock` already): `NaiveDate::from_ymd_opt(y, m, d) -> Option<NaiveDate>`, `NaiveTime::from_hms_opt(h, m, 0) -> Option<NaiveTime>` — the `_opt` forms are the panic-free constructors this story must use (the non-`_opt` ones panic and are deprecated). MIT/Apache-2.0. Docs: <https://docs.rs/chrono/0.4.44/>.
- **`nvim-orgmode/tree-sitter-org`** at `219c0b27…`: 98 named node kinds (`node-types.json`); the verified construct→node-shape map is in Dev Note 1. Upstream code-quiet ~15 months (LD-48 watch item; not this story's concern).
- No version bumps anywhere; [[feedback_version_policy]] satisfied by pinning chrono to the already-resolved 0.4.x line.

### References

- Source story: [`epics.md:777-811`](_bmad-output/planning-artifacts/epics.md#L777-L811) — Story 2.3 user-story + AC + the 15-bullet LD-44 construct list (lines 793-807).
- Previous story: [`2-2-implement-orgsidian-parser-grammar-wrapper.md`](_bmad-output/implementation-artifacts/2-2-implement-orgsidian-parser-grammar-wrapper.md) — the wrapper surface, review findings, deferred items addressed here.
- Next stories consuming this: Story 2.4 serializer ([`epics.md:813-826`](_bmad-output/planning-artifacts/epics.md#L813) — `serialize(headlines: &[Headline])`, why spans matter), Story 2.8 CLI ([`epics.md:873-886`](_bmad-output/planning-artifacts/epics.md#L873) — prints this AST).
- Architecture: LD-3 parser + semantic layer ([`architecture.md:65`](_bmad-output/planning-artifacts/architecture.md#L65)); LD-44 construct matrix ([`architecture.md:1228-1245`](_bmad-output/planning-artifacts/architecture.md#L1228)); LD-45 divergence triage + `docs/parser/KNOWN_DIVERGENCES.md` path ([`architecture.md:1247-1254`](_bmad-output/planning-artifacts/architecture.md#L1247)); LD-48 vendoring (grammar READ-ONLY) ([`architecture.md:1276-1281`](_bmad-output/planning-artifacts/architecture.md#L1276)); date/time conventions ([`architecture.md:757-761`](_bmad-output/planning-artifacts/architecture.md#L757)); naming + crate-organization + AI-agent rules ([`architecture.md:676-742`](_bmad-output/planning-artifacts/architecture.md#L676), [`architecture.md:850-876`](_bmad-output/planning-artifacts/architecture.md#L850)); FR-9 mapping naming `semantic/timestamp.rs` ([`architecture.md:1048`](_bmad-output/planning-artifacts/architecture.md#L1048)).
- PRD FR-1: [`prd.md:141-148`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md#L141) — render TODO states/tags/timestamps/drawers/links correctly; plain-text fallback posture.
- Deferred-work items owned/decided here: [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) story-2.2 stanza (source-retention → **Story 2.3 API design**; incremental-reparse → Story 2.3+ *only with a real consumer*; scanner.c indent bug breadcrumb).
- CONTRIBUTING: §2 commit scope `parser` ([`CONTRIBUTING.md:48`](CONTRIBUTING.md#L48)); §4 FR traceability `//! Implements FR-1` ([`CONTRIBUTING.md:109-124`](CONTRIBUTING.md#L109)).
- Grammar ground truth: `crates/orgsidian-parser/grammar/src/node-types.json` + `grammar.json` (pinned SHA `219c0b27…`).
- [[feedback_version_policy]], [[feedback_no_co_author_credit]], [[feedback_batch_fixes_terse]], [[feedback_role_agnostic_naming_in_docs]].

### Project Context Reference

- [`prds/prd-orgsidian-2026-05-19/prd.md`](_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md) — FR-1 (§4.1).
- [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — LD-3, LD-41 (failure-mode posture), LD-44, LD-45, LD-48; implementation patterns + AI-agent rules.
- [`epics.md`](_bmad-output/planning-artifacts/epics.md) — Epic 2 (Stories 2.1 → 2.8).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — §2 commit scopes, §4 FR traceability, §8 parser ownership.
- [`deferred-work.md`](_bmad-output/implementation-artifacts/deferred-work.md) — story-2.1 + story-2.2 stanzas.

## Dev Agent Record

### Agent Model Used

claude-fable-5[1m] (Fable 5, Claude Code)

### Debug Log References

- Grammar archaeology (pre-implementation, throwaway `tests/zz_scratch.rs`, deleted before commit): dumped s-expressions + node texts for every construct the walker consumes. Confirmed Dev Note 1 shapes and added: (a) time-ranges `10:00-11:00` surface as the grammar's `duration` field; (b) `<a>--<b>` date ranges are ONE `timestamp` node with two `date`/`day` field pairs; (c) the `plan` node covers only the first line after the headline — `SCHEDULED:`/`DEADLINE:` on separate following lines are paragraph soup (matches org-mode's own planning-line rule; one-line samples used in tests); (d) a directive directly above a paragraph attaches to that paragraph node (`directive` field) rather than to `body` — drove the position-independent directive collection and KNOWN_DIVERGENCES entry 9; (e) `tag`/`property`-name/`entry_name`/drawer-name node texts are bare (no colons).
- RED→GREEN: integration suite written against a stub `analyze()` first — 20/21 semantic tests failed (RED), anchor + grammar stayed green; real walker landed, all green. One real bug caught by the unit suite during RED: the plain-URL scanner sliced `text[i..]` mid-codepoint on multibyte input — fixed with a byte-level prefix check (`bytes[i..].starts_with(b"http://")`), pinned by `multibyte_text_around_links_is_safe`.

### Completion Notes List

- **Public surface** (`crates/orgsidian-parser/src/semantic/`, re-exported flat from `semantic`, `analyze` also at crate root): `analyze(&str) -> Result<Document, ParseError>`; `Document { headlines, todo_config, preamble, span }`; `Headline` with the epic-named fields (`todo_state`, `tags`, `scheduled`, `deadline`, `properties`) plus `level`, `title`, `span`, `children` and the recommended extras `closed`, `drawers`, `clocks`, `links`; `TodoState`/`TodoConfig`/`TodoSequence`; `Timestamp`/`Repeater`/`RepeaterKind`/`Delay`/`DelayKind`/`TimeUnit`; `Drawer`/`DrawerKind`/`ClockEntry`; `Link`/`LinkKind`; `Tag`; `Preamble`/`Directive`. Every source-mapped struct carries its `Range<usize>` byte span. `pub use chrono;` added at crate root (mirrors the `tree_sitter` re-export: single-sourced pin, downstream can name `NaiveDate`).
- **Source-retention decision (resolves the Story-2.2 deferred item):** semantic structs OWN their data (extracted `String`s + spans); no lifetimes in the public surface; `ParseTree` internals untouched. Recorded in deferred-work.md (story-2.2 stanza annotated). Incremental-reparse explicitly NOT taken (second annotation; no editor consumer yet).
- **AC2 decisions:** default set `TODO NEXT | DONE WAITING`; in-file `#+TODO:`/`#+SEQ_TODO:`/`#+TYP_TODO:` (names matched case-insensitively, org-style) REPLACE the default; multiple directives ACCUMULATE (org-faithful choice, documented + tested); no-pipe → last keyword done; `next()` wraps within the keyword's own sequence, `next(None)` starts the cycle, unknown keyword → `None`; fast-access suffixes (`TODO(t!)`) stripped to bare keywords. Keyword match on headlines is case-sensitive exact (first `item` expr only).
- **AC3 decisions:** `DrawerKind::{Properties, Logbook, Custom(String)}`; LOGBOOK matched case-insensitively; the property drawer also appears in `Headline::drawers` (kind `Properties`) so the drawers vec is complete; CLOCK lines parsed textually from LOGBOOK contents only (custom-drawer clocks deferred — stanza note); malformed CLOCK lines stay raw content; an unparseable `=> …` duration keeps the entry and drops the field; duration is overflow-safe `chrono::TimeDelta` via `try_minutes` + checked arithmetic.
- **AC4 decisions:** hand-rolled scanner (no regex dep); `][` splits, `]]` terminates, empty target is not a link; `target` keeps its scheme prefix verbatim (`id:abc`, `file://path`); bracketed links win over plain-URL detection inside their span; plain URLs trim trailing `.,;:!?)'"`; links collected per headline over its own region (headline line + body, children excluded — they collect their own) and for the preamble.
- **AC5 decisions:** one text-based timestamp parser shared by plan entries and CLOCK lines (the grammar's plan `timestamp` node text is fed through it; active/inactive from the delimiter byte); `_opt` chrono constructors only (panic-free); month-13-style garbage → whole timestamp `None`, `analyze()` continues (pinned by `semantic_lenient_unparseable_date`); repeaters `+`/`++`/`.+` with units `h/d/w/m/y`; delays `-`/`--` parsed (grammar `delay` node honored, not a gap); date ranges → `end_date`/`end_time`; time ranges → `end_time`. Plan-entry names matched exact-uppercase (`SCHEDULED`/`DEADLINE`/`CLOSED`), org-style; custom entry names dropped (stanza note).
- **AC6 variances (recorded, not escalated):** epic says "14 constructs" but enumerates 15 bullets → all 15 covered, bullets normative; epic's `semantic_{construct_kebab}` → snake_case `semantic_*` (kebab is not a valid Rust identifier). 21 tests in `tests/semantic.rs` = 15 construct tests + 6 extra regression tests (lenient bad date, no-pipe directive, multi-directive accumulation, duplicate-property last-wins, preamble + empty doc, timestamp ranges). All non-crash tests assert real properties (headline structure + construct text in span); no placebo greens.
- **Test counts:** parser crate 47 (anchor 1 unchanged + grammar 4 unchanged + semantic 21 NEW + unit 21 NEW in `src/semantic/*`); delta +42. Workspace `cargo test --workspace --locked`: 92 passed / 11 ignored (baseline 50/11 + the 42 new) — zero regressions. Parser suite runs in <1s warm.
- **chrono posture (AC9):** direct dep added via `[workspace.dependencies]` (`0.4`, `default-features = false, features = ["std"]`, resolves 0.4.44 already in-tree). `Cargo.lock` delta: exactly one line (the new edge). `cargo deny check licenses bans advisories`: ok/ok/ok. `cargo audit`: 18 allowed warnings — baseline unchanged. Zero other new dependencies (link scanner hand-rolled per AC4).
- **Gates:** `cargo build -p orgsidian-parser --locked` ✓; `cargo test -p orgsidian-parser --locked` ✓ (47); `cargo test --workspace --locked` ✓ (92/11); `cargo clippy --workspace --all-targets --locked` clean; `cargo fmt --check` clean; `cargo doc -p orgsidian-parser --no-deps` no warnings; no `unwrap()`/`expect()` in library code (tests only); no `println!`. Anchor sentinel: `tests/anchor.rs`, `tests/grammar.rs`, `tests/fixtures/anchor.org`, `build.rs`, `grammar/` all byte-untouched (verified via `git status`/`git diff`).
- **KNOWN_DIVERGENCES.md:** initialized at `docs/parser/KNOWN_DIVERGENCES.md` with the 8 verified entries + 1 new finding (directive-to-paragraph attachment) + the LD-45 triage-role statement.
- **File-size discipline:** all `src/semantic/*.rs` files ≤ 317 lines (one concern per file); `lib.rs` stays re-export-only (module wiring + `pub use` only).

### File List

- `Cargo.toml` (M — workspace chrono pin, Story-2.3 comment)
- `Cargo.lock` (M — one new dependency edge: orgsidian-parser → chrono)
- `crates/orgsidian-parser/Cargo.toml` (M — `chrono = { workspace = true }`)
- `crates/orgsidian-parser/src/lib.rs` (M — `pub mod semantic;`, `pub use semantic::analyze;`, `pub use chrono;`, header prose updated)
- `crates/orgsidian-parser/src/semantic/mod.rs` (NEW — module header `Implements FR-1`, `analyze()`, `Document`/`Preamble`/`Directive`, directive collection)
- `crates/orgsidian-parser/src/semantic/headline.rs` (NEW — `Headline`, `Tag`, section walker)
- `crates/orgsidian-parser/src/semantic/todo.rs` (NEW — `TodoState`, `TodoConfig`, `TodoSequence`, directive parsing, cycling)
- `crates/orgsidian-parser/src/semantic/timestamp.rs` (NEW — `Timestamp`, `Repeater`, `Delay`, `TimeUnit`, text parser)
- `crates/orgsidian-parser/src/semantic/drawer.rs` (NEW — `Drawer`, `DrawerKind`, `ClockEntry`, CLOCK-line parser)
- `crates/orgsidian-parser/src/semantic/link.rs` (NEW — `Link`, `LinkKind`, inline scanner)
- `crates/orgsidian-parser/tests/semantic.rs` (NEW — 21 `semantic_*` integration tests)
- `docs/parser/KNOWN_DIVERGENCES.md` (NEW — 9 entries + LD-45 role)
- `_bmad-output/implementation-artifacts/deferred-work.md` (M — story-2.3 stanza appended; two story-2.2 items annotated resolved/decided)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (M — story 2.3 → in-progress → review)
- `_bmad-output/implementation-artifacts/2-3-implement-semantic-layer-todo-cycling-drawer-types-timestamps-link-types.md` (M — this file: status, checkboxes, Dev Agent Record)

## Change Log

- 2026-06-10 — Story created (ultimate context engine analysis completed — comprehensive developer guide created). Status: ready-for-dev.
- 2026-06-10 — Story 2.3 implemented (semantic layer: `analyze()` → `Document`/`Headline`, TODO cycling config, drawer classification + CLOCK parsing, timestamp model, link scanner; 42 new tests, all gates green; KNOWN_DIVERGENCES.md initialized; source-retention deferred item resolved as owned-data). Status: review.
- 2026-06-10 — Adversarial code review (Blind Hunter + Edge Case Hunter + Acceptance Auditor): 18 patches applied (8 code — multi-pipe `#+TODO:`, plain-URL word boundary + empty-host, bracket-link newline guard, CLOCK `--garbage` malformed, duration bounds, zero-interval repeaters, range second-half offset; 5 tests; 5 docs), 2 deferred (verbatim-region link scan, `end_time` field shape), 4 dismissed (incl. empirically-disproven directive-hijack). Parser suite 47 → 59 tests, workspace 92 → 104, all gates green. Status: done.
