---
workflowStatus: 'completed'
workflowType: 'testarch-test-design'
mode: 'system-level'
totalSteps: 5
stepsCompleted:
  - step-01-detect-mode
  - step-02-load-context
  - step-03-risk-and-testability
  - step-04-coverage-plan
  - step-05-generate-output
lastSaved: '2026-05-19'
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md
  - _bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/addendum.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
---

# Test Design: orgsidian

**Date:** 2026-05-19
**Author:** Tiziano (TEA — Master Test Architect)
**Status:** System-Level Test Design — v1.0 baseline
**Project:** orgsidian
**PRD Reference:** `_bmad-output/planning-artifacts/prds/prd-orgsidian-2026-05-19/prd.md`
**Architecture Reference:** `_bmad-output/planning-artifacts/architecture.md` (53 Locked Decisions: LD-1..LD-53)
**Epics Reference:** `_bmad-output/planning-artifacts/epics.md` (104 stories across 13 epics)
**Regime:** spec-driven AI-agent implementation (PRD addendum §A.7 reframing — correctness over velocity)

---

## 0. Document Purpose & Scope

This is the **system-level testing strategy** for orgsidian — the full picture across PRD (24 FRs + 21 NFRs), Architecture (53 Locked Decisions), and Epics (104 stories, 13 epics, ~18-month v0.1 → v1.0 trajectory).

Single output file at `_bmad-output/test-artifacts/test-design.md` (consolidating the architecture-vs-QA split the system-level template would normally produce — appropriate here because the project is a solo-spec-driven AI-agent build with one human in both the architect and QA seats).

The document is **risk-prioritized**, **evidence-backed** against locked decisions, and authored to be consumed by:

1. The implementing AI agents (Story 1.9 anchor smoke discipline, Story 1.11 failure-mode harness, Story 1.12 perf snapshot infra, Process Discipline rule A merge-gate).
2. The author (Tiziano) — to verify the strategy still holds after each epic closes.
3. Downstream BMad workflows — `bmad-testarch-atdd` (red-phase scaffolds per story), `bmad-testarch-ci` (pipeline staging), `bmad-testarch-trace` (FR → test coverage), `bmad-testarch-nfr` (NFR thresholds and review).

What this document is **not**: a tactical test-case catalog. Per-story test cases live in the stories themselves (Process Discipline A — ATDD red-phase tests scaffolded via `bmad-testarch-atdd` before production code). This document is the **strategy that the per-story tests instantiate**.

---

## 1. Executive Summary

**Scope:** Cross-platform desktop org-mode editor (Tauri 2.x + Rust + React 19 + CodeMirror 6 + SQLite FTS5). 9-crate Cargo workspace + `shell-ui/` React app. Three milestones: v0.1 Alpha (Months 3-6, macOS+Linux), v0.5 Beta (Months 7-12), v1.0 (Months 13-18, +Windows).

**Risk profile:** **30 risks** identified.
- **15 high-priority (score ≥6)** requiring immediate or sustained mitigation.
- **3 score-9 risks** that dominate the strategy: R-011 (CM6 memory leak), R-017 (round-trip violation on real-world `.org`), R-023 (ATDD red-phase collapse under AI-agent velocity).
- Categories with most weight: TECH (parser + WebView + CodeMirror cross-platform), DATA (round-trip fidelity + Single Writer Rule + atomic writes), PERF (memory soak + Windows Quick Capture latency + editor-open).

**Coverage strategy:** ~9-layer test pyramid mapping to the 13 epics:
1. **Anchor smoke** (Story 1.9) — anti-placebo-green discipline.
2. **Rust unit per crate** — `#[cfg(test)] mod tests` co-located + rstest + insta.
3. **Integration via CLI** (LD-27 — `assert_cmd`) — primary cross-crate surface.
4. **React component** — Vitest + happy-dom co-located.
5. **E2E** — Playwright + Tauri WebDriver (UJ-3 spine Story 10.7, UJ-6 spine Story 8.8, a11y Story 13.5, others).
6. **Property-based** — `proptest` Rust + `fast-check` TS.
7. **Chaos / fault injection** — `fail` crate + `tests/failure_modes.rs` (Story 1.11) + `test-plugin-panic` chaos crate (LD-38).
8. **Memory soak** — nightly 12h, <10% RSS drift (LD-43, activated Epic 4 Story 4.9).
9. **Perf snapshot** — `assert_no_perf_regression!` shared macro (Story 1.12), ±20% on median of 5 runs.

**Three-level round-trip oracle** (LD-44/LD-45) is the trust-contract spine:
- **L0 byte-identical save-no-op** — per-PR subset (~100 files, <60s) [Story 2.6].
- **L1 property-based** — `proptest` randomized headlines → serialize → parse → serialize.
- **L2 Emacs ground-truth** — nightly via `emacs:29.x` + `emacs:30.x` with hand-written canonical AST (LD-45) [Story 2.7].

**Execution model:** Per-PR / Nightly / Weekly, with **stale-nightly merge-gate** (LD-32) — PR cannot merge unless per-PR is green AND last nightly is green within 24h.

**ATDD red-phase enforcement** (Process Discipline rule A) is the merge gate: every story requires a committed red-phase test before production code lands; PR cannot merge unless the same PR series flips that test red → green.

**Estimated test-strategy effort:** ~120-220h spread across the 18-month roadmap (≈10-15% of project budget), front-loaded into Epic 1 (Stories 1.5/1.7/1.8/1.9/1.11/1.12) and Epic 2 (Stories 2.5/2.6/2.7).

---

## 2. Quick Guide

### 🚨 BLOCKERS — Foundation Must Land Before Any Feature Story

These are non-negotiable Epic 1 + Epic 2 prerequisites — without them, every downstream story inherits a tainted CI baseline ("placebo green").

1. **[Story 1.7] `cargo-deny` + `cargo audit` supply-chain gates online** before any new dependency lands. License allowlist (LD-37) is the GPL-3.0 transitive-contagion catch.
2. **[Story 1.8] CI matrix with `panic = "unwind"` + `invoke_plugin_hook!` stub** — LD-38 plugin panic isolation depends on it.
3. **[Story 1.9] 3 anchor smoke tests committed** — `crates/orgsidian-parser/tests/anchor.rs`, `crates/orgsidian-vault/tests/anchor.rs`, `crates/orgsidian-watcher/tests/anchor.rs`. Anti-placebo-green discipline (Party Mode P2 — Murat).
4. **[Story 1.11] LD-41 failure-mode harness skeleton** — `tests/failure_modes.rs` enumerates all 9 categories with `#[ignore]` placeholders; coverage gate fires from v0.5 Beta onward.
5. **[Story 1.12] `assert_no_perf_regression!` macro published** — every subsequent perf-AC story (4.3a-g, 6.3, 7.1, 8.1, 8.4, 9.1, 10.6) consumes it. Absolute perf numbers from PRD §8 are baselines, not gates.
6. **[Story 2.5] `tools/corpus-extractor` + `fixtures/fixtures.toml`** — corpus governance with per-epic ownership and git-LFS versioning.
7. **[Story 2.6] L0 round-trip gate live on every PR** — `<60s` on ~100-file subset. FR-2 trust contract.
8. **[Story 2.7] L2 Emacs oracle gate live nightly** — `emacs:29.x` + `emacs:30.x` pinned; canonical AST committed.

**Until items 1–8 are green, no v0.1 Alpha feature story should land.**

### ⚠️ HIGH PRIORITY — Mitigations for Score-≥6 Risks

| Risk | Score | Mitigation Owner / Location |
|---|---|---|
| R-011 CM6 memory leak | **9** | LD-43 nightly memory soak gate, activated Epic 4 Story 4.9 (anticipated from Epic 6 per Party Mode P1) |
| R-017 Round-trip violation real-world | **9** | LD-44 subset corpus criteria + LD-45 Emacs oracle + L0/L1/L2 three-level oracle |
| R-023 ATDD red-phase collapse | **9** | Process Discipline A merge-gate + tooling support via `bmad-testarch-atdd` skill |
| R-001 `tree-sitter-org` upstream stall | 6 | LD-48 SHA-pinned submodule + parser-owner role + v0.3 fork-and-maintain dry run |
| R-002 CM6 widget edge cases | 6 | LD-6 mandatory recipes (`WidgetType.eq()`, `Transaction.userEvent`, no dispatch in `view.composing`, `widget.ignoreEvent() === false`) + Vitest+happy-dom component tests |
| R-004 WebView consistency | 6 | LD-32 nightly cross-webview matrix (WebKit/WebView2/WebKitGTK) + LD-44 subset feeds same fixture into editor component tests |
| R-009 Transitive GPL dependency | 6 | LD-37 `cargo deny check licenses` allowlist; first-run sweep on typst transitive closure (~150 crates) is the bar |
| R-013 Quick Capture Windows latency | 6 | LD-28 separate `quick-capture.html` Vite bundle + nightly Windows perf snapshot |
| R-015 Editor open <300ms regression | 6 | Story 1.12 `assert_no_perf_regression!` consumed by Story 4.3a-g (decoration stack) |
| R-018 Atomic write Windows | 6 | LD-8 AV-retry wrapper + property tests in `crates/orgsidian-vault/tests/atomic.rs` |
| R-019 External-write race | 6 | LD-7 ConflictState rich struct + LD-9 watcher abstraction with deterministic fakes + golden traces (Story 5.2) |
| R-022 Multi-instance race | 6 | LD-39 lockfile with heartbeat + dialog; CI test spawning 2 processes (Story per LD-39 acceptance) |
| R-024 Placebo green CI | 6 | Story 1.9 anchor smoke tests — explicit anti-pattern test |
| R-025 Round-trip gate not live v0.1 | 6 | Block v0.1 Alpha tag on Stories 2.6 + 2.7 (per Epic 6 Story 6.5 freeze gate) |
| R-027 Nightly merge-gate flakiness | 6 | LD-32 24h staleness window + nightly retry logic + dedicated Linux soak runner |

### 📋 INFO — Strategy Decisions Already Solved

1. **Stack tooling locked**: Vitest 2.1 + happy-dom (CM6 `getComputedStyle`); Playwright + Tauri WebDriver E2E; `rstest` + `proptest` + `insta`; `assert_cmd` for CLI integration; `axe-core` a11y; `cargo audit` + `cargo deny` + `cargo-semver-checks` supply-chain.
2. **Three-level round-trip oracle** (L0 byte-identical / L1 property / L2 Emacs ground-truth) — codified in LD-44 + LD-45, mechanized by `tools/corpus-extractor`.
3. **Fixture governance** — `tests/fixtures/vault-corpus/` git-LFS + `fixtures.toml` per-epic ownership (Story 2.5).
4. **Per-PR vs nightly split** with merge-gate on nightly staleness (LD-32) — established pattern from rust-analyzer/biomejs/ruff.
5. **CLI as primary integration surface** (LD-27) — cheaper than Playwright/Tauri WebDriver for cross-crate flows per Murat.

---

## 3. Testability Review (System-Level)

Per Step 3 protocol: actionable concerns first, FYI strengths after.

### 3.1 🚨 Actionable Testability Concerns

#### TC-1. ATDD red-phase **CI verification** is implicit, not explicit

Process Discipline rule A states the merge gate requires the red-phase test to predate the green commit. There is no specified CI gate that *verifies* this temporal ordering. Under AI-agent velocity pressure, this collapses to a verbal contract (R-023 score 9).

- **What architecture must provide:** CI gate that parses the commit history of the PR series and asserts that a test marked with story-id existed in the failing-fixture state in commit N before the production code in commit N+k.
- **Concrete recipe:** GitHub Actions workflow scanning commits for the story-id tag in test-file diff vs `Cargo.toml`/`shell-ui/src/` diff. Tooling support via `bmad-testarch-atdd` skill.
- **Owner:** author/contributor (CI infra).
- **Timeline:** must land in Epic 1 (alongside Story 1.8 CI matrix).

#### TC-2. `assert_no_perf_regression!` baselines tied to runner hardware

LD-32 nightly runs on the GitHub Actions free tier (heterogeneous CPU). Story 1.12 establishes ±20% on median of 5 runs vs `tests/perf-baselines/{story_id}.json`. Without runner-pinning, baseline drift across GHA hardware refreshes can mask real regressions or fail-false-positive on transient noise.

- **What architecture must provide:** Baseline files include `runner_class` field; comparison only valid within the same class. Or: track baselines per `(story_id, runner_class)` tuple.
- **Owner:** author/contributor (Story 1.12 author).
- **Timeline:** Epic 1 (Story 1.12 design pass).

#### TC-3. LD-44 subset coverage matrix is documented but not **self-verifying**

The LD-44 algorithm requires every documented org-mode syntax construct ≥3 times, size buckets (30 small + 50 medium + 20 large), edge-case bucket (≥5 Unicode/RTL/CRLF/malformed-valid). `tools/corpus-extractor/` generates the subset, but no test asserts the **generated subset itself** still satisfies the matrix after corpus changes.

- **What architecture must provide:** A meta-test inside `tools/corpus-extractor/` itself that runs the matrix-coverage validator on the generated `subset-pr.json` and fails if any construct count falls below the threshold.
- **Owner:** author/contributor (Story 2.5).
- **Timeline:** Epic 2 (Story 2.5 acceptance criteria addendum).

#### TC-4. Memory soak gate (LD-43) runs only on Linux — CM6 leaks are likely platform-divergent

LD-43 mandates a Linux-only nightly soak runner. CM6 + WebView leaks differ materially across WebKit (macOS), WebView2 (Windows), and WebKitGTK (Linux). A leak that surfaces only on WebView2 will not be caught until v1.0 + Windows users complain.

- **What architecture must provide:** Either extend the soak to a tri-platform matrix from v1.0 (Epic 13), or document an explicit known-gap with a v1.5+ resolution date.
- **Owner:** author/contributor (Epic 13 planning).
- **Timeline:** v1.0 milestone — decide before Story 13.1 (Windows MSI packaging) lands.

#### TC-5. Tauri WebDriver maturity on Windows is the E2E spine risk for v1.0

Stories 8.8 (UJ-6 spine) and 10.7 (UJ-3 spine) target Playwright + Tauri WebDriver. The integration is documented as macOS-arm64 + Ubuntu-LTS per PR; Windows runs only nightly. Tauri 2.x WebDriver coverage on Windows is the weakest link in the matrix (verified-as-of architecture LD-32 wording).

- **What architecture must provide:** Either an early Windows spine pilot in Epic 8 (during v0.5 Beta sprints), or an explicit Windows-spine fallback strategy using direct `webdriver-bidi` against WebView2 if Tauri WebDriver proves blocking.
- **Owner:** author/contributor.
- **Timeline:** Epic 8 (Story 8.8 retrospective) — earliest Windows-spine data point.

#### TC-6. Plugin chaos test (LD-38) covers `panic!`, not memory exhaustion or infinite loops

`test-plugin-panic` deterministically panics in every hook. `std::panic::catch_unwind` does NOT catch infinite loops, deadlocks, or unbounded memory growth — these failure modes will hang the host without disabling the plugin.

- **What architecture must provide:** A second chaos plugin `test-plugin-exhaust` (or extend `test-plugin-panic`) with:
  - Infinite loop in `on_event` — host must detect via timeout watchdog.
  - Unbounded `Vec` allocation in `on_save_before` — host must detect via memory ceiling.
- **Owner:** author/contributor.
- **Timeline:** Epic 12 (Story 12.4 LD-50 plugin event surface review) — surfaces as a v1.0 hardening item.

#### TC-7. CLI as "primary integration surface" (LD-27) is undermined by the **frontend-driven** UJs

UJs 1, 2, 4, 5 are all UI-driven (Today Dashboard, Quick Capture, Starter Vault picker, Merge Dialog). The CLI exercises `core`, but the **user contract** lives in the UI layer. Over-relying on CLI integration risks missing regressions in IPC-bound flows (LD-24 `tauri-specta` codegen drift, R-030).

- **What architecture must provide:** CLI integration tests are necessary but not sufficient; complement with Vitest component tests for each UI surface AND at least one E2E spine per UJ (already partially solved: Stories 8.8 + 10.7 spine; missing UJ-1, UJ-2, UJ-4, UJ-5 spines).
- **Recommendation:** add UJ-1 (Today Dashboard launch), UJ-2 (Quick Capture round-trip ≤3s — partially covered by Story 8.1 return-focus AC), UJ-4 (Starter Vault first-launch), UJ-5 (Merge Dialog conflict) as e2e spines in Epics 7, 8, 6, 9 respectively.
- **Owner:** author/contributor.
- **Timeline:** spec-amendment to Epics 6, 7, 8, 9 — flag during epic-start.

### 3.2 ✅ Testability Strengths (FYI)

The architecture is **testability-aware** at an unusual depth for a v1.0 plan:

- **Story 1.9 anchor smoke tests** — explicit anti-placebo-green discipline (Party Mode P2 — Murat). Three anchor tests prove parser+vault+watcher actually exercise real code paths.
- **Story 1.11 LD-41 failure-mode harness** — single cross-cutting harness enumerating all 9 failure modes with fault-injection via `fail` crate. Coverage gate fires from v0.5 Beta onward.
- **Story 1.12 perf snapshot infra** — shared `assert_no_perf_regression!` macro eliminates per-story baseline drift and the "<500ms" CI-gate antipattern.
- **LD-44 subset corpus selection** — algorithmic (syntax-feature matrix + size buckets + edge-case bucket), regeneratable, auditable.
- **LD-45 Emacs oracle pinning** — two pinned Emacs versions + canonical AST committed + divergence triage workflow.
- **Watcher abstraction with deterministic fakes** (LD-9) — race conditions exercised deterministically via injected clock + synthetic events.
- **Golden-trace fixtures** (Story 5.2) — vim/VS Code/Emacs save sequences recorded once; debounce calibration is data-driven.
- **`ConflictState` rich struct + `ConflictStrategy` pattern** (Story 5.3, Party Mode P0) — Epic 9 swaps strategy variant without rewriting watcher state machine. Avoids the "Epic 9 watcher rewrite trap."
- **Story 6.5 `cargo-semver-checks` automation** replaces the manual `IndexQuery` freeze gate.
- **LD-43 memory soak gate** — 12h scripted session, <10% RSS drift, blocks PR merge via nightly merge-gate.
- **LD-51 CSS token snapshot test** — `--org-*` vocabulary locked as public theme API contract.
- **`Implements FR-NN` doc-comment + `tests/traceability.rs`** — bidirectional FR ↔ module-doc enforcement.

### 3.3 Architecturally Significant Requirements (ASRs)

| ASR | Category | Source | Status |
|---|---|---|---|
| Round-trip preservation (FR-2) — byte-identical save-no-op enforced by CI | ACTIONABLE | PRD §1.5, §4.1 FR-2; LD-32 | Designed (Stories 2.4–2.7); gate live from Epic 2 |
| Single Writer Rule + Dirty Buffer + Merge Dialog | ACTIONABLE | PRD §4.4 FR-16; LD-7 | v0.1 BlockWithWarning (Epic 5); v0.5 ThreePaneMergeDialog (Epic 9) |
| Performance budgets (startup <2s, typing <30ms, agenda <100ms, search <200ms, capture <1s, mem <500MB, editor open <300ms) | ACTIONABLE | PRD §8 NFR-1..7 | Designed via Story 1.12 `assert_no_perf_regression!` |
| Memory <10% RSS drift over 11h | ACTIONABLE | LD-43; NFR-21 | Nightly gate from Epic 4 (Story 4.9, anticipated per Party Mode P1) |
| WCAG 2.1 AA + keyboard nav | ACTIONABLE | PRD §8 NFR-9 | Story 13.5 axe-core 0-serious/critical + manual qualitative sign-off |
| Cross-platform parity (Windows added v1.0) | ACTIONABLE | PRD §8 NFR-8 | LD-32 CI matrix + Epic 13 Windows hardening |
| Internal Plugin Pattern stable contract from day-1 | ACTIONABLE | FR-24; LD-10, LD-26, LD-50 | `orgsidian-plugin-api` internal; Story 12.4 LD-50 surface review; semver from day-1 |
| Zero network calls in core paths | ACTIONABLE | PRD §7.1; NFR-12 | CI verifies via network-namespace sandbox |
| Atomic writes on all platforms | ACTIONABLE | PRD §7.4, FR-15 NFR; LD-8, NFR-15 | `atomic-write-file` + 3-retry; property tests + chaos (Story 3.1 + Story 1.11) |
| Supply-chain hygiene (licenses, advisories, leaf-crate) | FYI | LD-37 | Per-PR `cargo audit` + `cargo deny`; quarterly review |
| i18n catalog drift (Lingui) | FYI | LD-52, NFR-10 | `lingui extract --clean && git diff --exit-code` CI gate |
| CSS token public API (LD-51) | FYI | FR-22, LD-51 | `tokens.test.ts` Vitest snapshot test |
| Plugin panic isolation under static linking | ACTIONABLE | LD-38 | `panic = "unwind"` + `invoke_plugin_hook!` macro + `test-plugin-panic` chaos crate |

---

## 4. Risk Assessment Matrix

Probability and Impact scored 1–3 each; score = P × I. Risks ≥6 are high-priority. Categories per `risk-governance.md` (TECH / SEC / PERF / DATA / BUS / OPS).

### 4.1 High-Priority Risks (Score ≥6) — Immediate Mitigation

| Risk ID | Category | Description | P | I | Score | Mitigation | Owner | Timeline |
|---|---|---|---|---|---|---|---|---|
| **R-011** | **PERF** | Memory leak from CM6 decorations/widgets surfaces only after sustained editing; silently degrades v0.5/v1.0 daily-driver experience | 3 | 3 | **9** | LD-43 nightly memory soak gate (Story 4.9, anticipated Epic 4); `dhat` heap profiler in `docs/perf/memory-soak-reports/` for triage | Author | Epic 4 (anticipated per Party Mode P1) |
| **R-017** | **DATA** | Round-trip violation on real-world `.org` file outside subset corpus breaches FR-2 trust contract (Logseq-lossy reputational risk) | 3 | 3 | **9** | LD-44 subset criteria + LD-45 L2 Emacs oracle nightly + L0 per-PR gate + L1 property tests (`proptest`); `KNOWN_DIVERGENCES.md` for accepted gaps | Author | Epic 2 (Stories 2.5–2.7) |
| **R-023** | **BUS** | ATDD red-phase enforcement collapses under AI-agent velocity pressure; "implement then maybe test" reverts | 3 | 3 | **9** | Process Discipline A merge-gate + commit-history verification CI gate (TC-1) + `bmad-testarch-atdd` skill scaffolds | Author / CI infra | Epic 1 (Story 1.8) |
| **R-001** | **TECH** | `nvim-orgmode/tree-sitter-org` upstream stalls; coverage gaps emerge that block parser stories | 2 | 3 | 6 | LD-48 SHA-pinned submodule; named parser-owner role; v0.3 budget for 2-week fork-and-maintain dry run; in-house fork at `orgsidian-org/tree-sitter-org` if no commits >6 months | Author (parser-owner) | v0.3 milestone |
| **R-002** | **TECH** | CM6 widget+multi-cursor interaction edge cases (CM discuss #6504, codemirror/dev #111) degrade Pseudo-WYSIWYG UX | 3 | 2 | 6 | LD-6 mandatory recipes; Vitest+happy-dom component tests for Stories 4.3a-g; multi-cursor + widget interaction is a documented v0.1 limitation | Author | Epic 4 |
| **R-004** | **TECH** | WebView consistency (WebKit / WebView2 / WebKitGTK) breaks CM6 decoration parity across platforms — silent rendering divergence on Windows | 3 | 2 | 6 | LD-32 nightly cross-webview matrix; LD-44 subset feeds same fixtures into editor component tests; WebKitGTK version pin documented; Story 13.5 a11y axe-core on all surfaces | Author | Continuous (nightly from Epic 4) |
| **R-009** | **SEC** | Transitive GPL-3.0 dependency surfaces in dep graph (e.g., typst transitive closure ~150 crates) — blocks LD-1 MIT contract | 2 | 3 | 6 | LD-37 `cargo deny check licenses` allowlist per-PR; first-run sweep on Story 10.1 (typst integration) is the bar; advisory exception process in `docs/security/advisory-exceptions.md` | Author | Per-PR (Story 1.7); intensive first-run on Story 10.1 |
| **R-013** | **PERF** | Quick Capture end-to-end >1s on Windows (separate Tauri window + global hotkey + WebView2 cold-start) — breaches NFR-5 | 3 | 2 | 6 | LD-28 separate `quick-capture.html` Vite bundle (small, minimal deps); Story 8.1 `assert_no_perf_regression!`; nightly Windows perf snapshot; Lingui 3kB runtime per LD-52 supports the budget | Author | Story 8.1; nightly Windows monitoring from Epic 13 |
| **R-015** | **PERF** | Editor opens 5000-line `.org` file in >300ms with full FR-4 decoration stack (heading + TODO + tag + timestamp + checkbox + link) active — breaches NFR-6 | 3 | 2 | 6 | Story 1.12 `assert_no_perf_regression!` consumed by Stories 4.3a-g; CM6 viewport-based decoration rendering (LD-6); incremental parse via tree-sitter (LD-3) | Author | Story 4.3g (last decoration story) |
| **R-018** | **DATA** | Atomic write fails to provide power-loss-safety on Windows (MoveFileExW edge cases, AV interference, Search-indexer lock) | 2 | 3 | 6 | LD-8 `atomic-write-file` + 3-retry exponential backoff wrapper; property tests in `crates/orgsidian-vault/tests/atomic.rs` (Story 3.1); LD-41 failure-mode catalog `Disk full / ENOSPC`, `.tmp orphans` covered | Author | Story 3.1 + Story 1.11 |
| **R-019** | **DATA** | External-write race condition during atomic save corrupts Single Writer Rule invariant (FR-16) | 2 | 3 | 6 | LD-7 ConflictState rich struct + LD-9 watcher abstraction with deterministic fakes; Story 5.2 golden traces (vim/VS Code/Emacs); Story 5.3 `ResolveConflict` trait parametrized test suite; LD-41 "External delete with Dirty Buffer" entry | Author | Epic 5 + Epic 9 |
| **R-022** | **DATA** | Multi-instance race writes same vault concurrently, corrupting index or `.org` files | 2 | 3 | 6 | LD-39 `.orgsidian/instance.lock` with PID + heartbeat + 5min orphan threshold; CI test spawning 2 processes against same fixture vault | Author | LD-39 acceptance (Epic 3 or Epic 6) |
| **R-024** | **BUS** | CI placebo green — tests pass but don't exercise real paths (the "merge-pressure atrophy" failure mode) | 2 | 3 | 6 | Story 1.9 anchor smoke tests (Party Mode P2 — Murat); each anchor exercises a real code path end-to-end (parse + write+read+verify + watcher detect) | Author | Story 1.9 (Epic 1) |
| **R-025** | **BUS** | v0.1 Alpha ships without round-trip CI gate live, breaching the trust contract before public eyes see it | 2 | 3 | 6 | Epic 2 close gate: v0.1 Alpha tag blocked unless Stories 2.6 (L0 per-PR) + 2.7 (L2 nightly) are green; documented in Epic 6 Story 6.5 (`IndexQuery` freeze) flow | Author | Block v0.1 Alpha tag |
| **R-027** | **OPS** | Nightly merge-gate (LD-32 — PR cannot merge if last nightly fails >24h) creates extended merge windows when nightly flakes | 3 | 2 | 6 | LD-32 24h staleness window; nightly retry logic + dedicated soak runner; failure triage workflow documented (parser oracle, memory soak, perf trend, large-vault scaling each have separate ownership) | Author | Continuous (CI hygiene) |

### 4.2 Medium-Priority Risks (Score 3–5)

| Risk ID | Category | Description | P | I | Score | Mitigation |
|---|---|---|---|---|---|---|
| R-003 | TECH | Tauri ecosystem breaking change (6-8 week cadence) breaks build at unexpected milestone | 2 | 2 | 4 | LD-47 exact-pin (`=2.X.Y`) + quarterly Tauri-sync slot at v0.2/v0.3/v0.4; v0.4 reserves 2-3 weeks for migration |
| R-005 | TECH | SQLite FTS5 sync drift between `headlines` table and `fts_headlines` virtual table (application-managed sync, no triggers) | 2 | 2 | 4 | LD-4 application-managed sync invariant tests; `tests/fts_sync.rs` asserts consistency after every index mutation in Story 3.5 |
| R-006 | TECH | notify-rs golden-trace coverage gap for new editor (e.g., Sublime Text) atomic-save sequence | 2 | 2 | 4 | Story 5.2 fixture set is extensible; `fixtures.toml` ownership clear; community contribution path documented |
| R-012 | PERF | Agenda recompute >100ms on 1k-file vault under widget-rendering load (NFR-3) | 2 | 2 | 4 | LD-30 `@tanstack/react-virtual` for Agenda + LD-14 reader pool; Story 7.1 perf snapshot |
| R-014 | PERF | SQLite FTS5 latency >200ms on 1k-file vault with Unicode-heavy corpus (NFR-4) | 2 | 2 | 4 | LD-4 FTS5 PRAGMAs locked + `unicode61 remove_diacritics 2` + `porter` tokenizer; Story 8.4 perf snapshot |
| R-016 | PERF | Large-vault scaling (10k+ files) — LD-42 soft targets not enforced as gates | 2 | 2 | 4 | LD-42 nightly synthetic vaults at 10k/25k/50k; baselines in `docs/perf/large-vault-scaling.md`; not PR-blocking but trend-monitored |
| R-026 | BUS | Plugin API breaking change pre-v1.5 publication (internal SemVer drift) — surfaces only at LD-50 review | 2 | 2 | 4 | LD-50 dedicated event-surface review at v0.5 milestone (Story 12.4) + LD-26 `#[non_exhaustive]` Event enum; semver-checks (Story 6.5) from v0.5 onward |
| R-028 | OPS | Emacs oracle pinning (LD-45) drift if `emacs:29.x` / `emacs:30.x` Docker images go stale | 2 | 2 | 4 | LD-45 canonical AST committed for the L2 subset; meta-test verifies Emacs' output matches canonical (if Emacs diverges from canonical, oracle is broken, not Orgsidian) |
| R-030 | OPS | `tauri-specta` codegen drift between Rust IPC commands and TypeScript client (LD-24, LD-31) | 2 | 2 | 4 | `pnpm tauri build` regenerates `shell-ui/src/lib/tauri.ts` as pre-step; `git diff --exit-code` CI gate post-codegen catches drift |
| R-007 | SEC | User CSS data exfiltration via `url()` on hover/load — CSP `connect-src 'self'` is the only mitigation | 1 | 3 | 3 | LD-18 CSP locked; LD-22 documented threat model; Story 12.1 user CSS loader includes a CSP-violation logging test fixture |
| R-008 | SEC | Plugin (v1.5+) escape from `wasmtime` sandbox — not v1.0 but architecture chosen now | 1 | 3 | 3 | LD-25 + LD-26 designed for WASM message-passing semantics from day 1; LD-50 review precedes external publication; deferred risk register entry for v1.5+ |
| R-010 | SEC | Code-signing key compromise on GitHub Actions secret | 1 | 3 | 3 | LD-19 signing keys as GHA secrets; future hardening (HSM-backed signing) tracked in v1.5+ backlog |

### 4.3 Low-Priority Risks (Score 1–2)

| Risk ID | Category | Description | P | I | Score | Action |
|---|---|---|---|---|---|---|
| R-020 | DATA | SQLite index corruption with WAL journal lost mid-write | 1 | 2 | 2 | Monitor; LD-13 rebuild policy handles recovery; PRAGMA integrity_check on startup |
| R-021 | DATA | Vault deletion runtime — app must enter read-only mode | 1 | 2 | 2 | LD-41 entry covered; CI integration test `rmrf` mid-session |
| R-029 | OPS | Apple notarization service outage blocks macOS release | 1 | 2 | 2 | Monitor; LD-19 release pipeline; manual notarization fallback documented |

### 4.4 Risk Category Legend

- **TECH** — Technical/Architecture (flaws, integration, scalability, cross-platform divergence)
- **SEC** — Security (sandboxing, supply-chain, signing, data exfiltration)
- **PERF** — Performance (SLA violations, regressions, memory leaks)
- **DATA** — Data Integrity (round-trip, atomic writes, single-writer, concurrency)
- **BUS** — Business Impact (trust contract, discipline collapse, scope drift)
- **OPS** — Operations (CI, release pipelines, oracle drift, codegen)

---

## 5. Fixture Architecture

Per Murat (Party Mode P1): fixture governance is a load-bearing element of the strategy. Without ownership, fixtures rot.

### 5.1 Directory Layout

```
orgsidian/
├── tests/
│   ├── failure_modes.rs                 # LD-41 cross-cutting harness (Story 1.11)
│   ├── failure_modes_coverage.rs        # CI gate — LD-41 categories must not be #[ignore] past v0.5
│   ├── traceability.rs                  # FR ↔ //! Implements FR-NN bidirectional gate
│   └── perf-baselines/
│       └── {story_id}.json              # Story 1.12 baselines (per-runner-class scoped — TC-2)
│
├── tests/fixtures/                      # PROJECT-WIDE FIXTURES (git-LFS versioned)
│   └── vault-corpus/                    # shared corpus consumed across crates
│       ├── subset-pr.json               # ~100 files, LD-44 selection algorithm
│       ├── full-nightly.json            # ~2000 assertions from test-org-element.el
│       ├── synthetic-10k/               # LD-42 large-vault scaling
│       ├── synthetic-25k/
│       └── synthetic-50k/
│
├── fixtures/
│   └── fixtures.toml                    # PER-EPIC OWNERSHIP DECLARATION (Story 2.5)
│
├── tools/corpus-extractor/              # outside [workspace.members], publish=false
│   ├── src/
│   │   ├── main.rs                      # extract subset + full-nightly from test-org-element.el
│   │   └── validator.rs                 # meta-test: subset satisfies LD-44 matrix (TC-3)
│   └── tests/
│       └── matrix_coverage.rs           # asserts generated subset still meets LD-44 criteria
│
├── crates/orgsidian-parser/
│   ├── grammar/                         # tree-sitter-org SHA-pinned submodule (LD-48)
│   └── tests/
│       ├── fixtures/                    # CRATE-LOCAL fixtures (anchor sample, semantic samples)
│       │   ├── anchor.org               # Story 1.9 anchor smoke
│       │   ├── semantic-heading-1.org   # one fixture per LD-44 syntax construct (Story 2.3)
│       │   └── ...
│       ├── canonical_ast/               # LD-45 L2 oracle canonical ASTs
│       │   └── {file}.json              # hand-written, peer-reviewed
│       └── anchor.rs                    # Story 1.9 anchor smoke
│
├── crates/orgsidian-watcher/tests/
│   ├── golden_traces/                   # Story 5.2 (owner = epic-5)
│   │   ├── vim.json
│   │   ├── vscode.json
│   │   └── emacs.json
│   └── anchor.rs                        # Story 1.9 anchor smoke
│
├── crates/orgsidian-vault/tests/
│   ├── fixtures/                        # AV-lock / power-loss simulation traces
│   └── anchor.rs                        # Story 1.9 anchor smoke
│
├── crates/orgsidian-index/tests/
│   └── fixtures/                        # FTS5 corpus + multi-year cross-reference vault (UJ-6 Story 8.8)
│
├── crates/orgsidian-report/tests/
│   └── fixtures/                        # 4-week project with milestones + open-clock (UJ-3 Story 10.7)
│
├── crates/test-plugin-panic/            # LD-38 chaos test plugin (workspace member, dev-only)
└── shell-ui/
    ├── src/themes/__snapshots__/        # LD-51 tokens snapshot
    │   └── tokens.snap
    └── e2e/fixtures/                    # E2E spine vault fixtures (UJ-3, UJ-6)
```

### 5.2 `fixtures.toml` — Per-Epic Ownership

```toml
# Format established by Story 2.5; Party Mode P1 — Murat

[corpus.subset-pr]
path = "tests/fixtures/vault-corpus/subset-pr.json"
owner = "epic-2"
regenerated_by = "tools/corpus-extractor"
ld_reference = "LD-44"
notes = "~100 files, per-PR L0 round-trip gate, <60s budget"

[corpus.full-nightly]
path = "tests/fixtures/vault-corpus/full-nightly.json"
owner = "epic-2"
regenerated_by = "tools/corpus-extractor"
ld_reference = "LD-44, LD-45"
notes = "~2000 assertions, nightly full-corpus + L2 Emacs oracle"

[corpus.synthetic-10k]
path = "tests/fixtures/vault-corpus/synthetic-10k"
owner = "epic-3"
ld_reference = "LD-42"
notes = "Large-vault scaling soft target <5min"

[traces.vim]
path = "crates/orgsidian-watcher/tests/golden_traces/vim.json"
owner = "epic-5"
ld_reference = "LD-9, OD-3"
notes = "vim swap+rename atomic-save sequence; debounce calibration"

[traces.vscode]
path = "crates/orgsidian-watcher/tests/golden_traces/vscode.json"
owner = "epic-5"
ld_reference = "LD-9, OD-3"

[traces.emacs]
path = "crates/orgsidian-watcher/tests/golden_traces/emacs.json"
owner = "epic-5"
ld_reference = "LD-9, OD-3"

[vault.uj6-multi-year]
path = "crates/orgsidian-index/tests/fixtures/uj6-multi-year/"
owner = "epic-8"
notes = "≥2 years of dated .org files with id: cross-references for Story 8.8 UJ-6 spine"

[vault.uj3-project-report]
path = "crates/orgsidian-report/tests/fixtures/uj3-project-report/"
owner = "epic-10"
notes = "4-week project, ≥3 milestones, ≥10 clocked tasks, ≥5 linked notes, 1 open clock for Story 10.7 UJ-3 spine"

[oracle.canonical-ast]
path = "crates/orgsidian-parser/tests/canonical_ast/"
owner = "epic-2"
ld_reference = "LD-45"
notes = "Hand-written, peer-reviewed JSON ASTs for L2 subset; Emacs oracle compared against this canonical"

[chaos.test-plugin-panic]
path = "crates/test-plugin-panic/"
owner = "epic-1"
ld_reference = "LD-38"
notes = "Chaos plugin deterministically panics in every hook point"
```

### 5.3 Governance Rules

1. **Every fixture file is owned by exactly one epic.** Cross-epic consumption is allowed but ownership of mutation remains with the listed owner.
2. **Mutation requires PR review by the named owner.** Owners are tracked at the epic-team level; for solo-dev this is the author with explicit commit-message tag (`[fixture:epic-N]`).
3. **Generated fixtures** (`subset-pr.json`, `full-nightly.json`) are regenerated by `tools/corpus-extractor` and committed as binary blobs via git-LFS. Manual editing forbidden; PR comment must reference the generator invocation.
4. **Crate-local fixtures** (e.g., `crates/orgsidian-parser/tests/fixtures/semantic-*.org`) are co-located by default per CONTRIBUTING.md; promoted to root `fixtures/` only when ≥2 crates consume them.
5. **Anchor fixtures** (Story 1.9) are deliberately minimal and stable — should not change after Epic 1 closes.

---

## 6. Test Pyramid — Levels and Mechanisms

The pyramid below maps test levels to mechanisms and primary owners (epic/crate). Layers are listed bottom-up.

### 6.1 Layer 1 — Anchor Smoke (Story 1.9, Anti-Placebo-Green)

**Mechanism:** Three end-to-end-light Rust tests covering parser+vault+watcher.

| Test | Location | Asserts |
|---|---|---|
| `parser/anchor.rs` | `crates/orgsidian-parser/tests/anchor.rs` | `* TODO Hello\n` parses without error |
| `vault/anchor.rs` | `crates/orgsidian-vault/tests/anchor.rs` | atomic_write + read-back byte-identical |
| `watcher/anchor.rs` | `crates/orgsidian-watcher/tests/anchor.rs` | Detects 1 fs event within 5s (deterministic fake clock) |

**Execution:** Per-PR on macOS + Ubuntu.
**Failure mode:** If green, real code paths are exercised — protects against CI-config-only "compiled OK" placebo.

### 6.2 Layer 2 — Rust Unit (Per Crate, Co-Located)

**Mechanism:** `#[cfg(test)] mod tests` at bottom of source files + crate-level `tests/<topic>.rs` for integration-within-crate.

**Tooling:**
- `rstest` for parameterized tests (e.g., semantic layer per LD-44 construct).
- `proptest` for property-based generation (Round-trip L1 — Story 2.4 randomized headlines).
- `insta` for snapshot tests (parser AST, SQL schema fingerprint, CLI stdout).

**Coverage by crate:**

| Crate | Unit Test Focus | Property Tests | Snapshot Tests |
|---|---|---|---|
| `orgsidian-parser` | Per-construct semantic layer (14 LD-44 constructs enumerated in Story 2.3) | Serialize→parse→serialize idempotence (Story 2.4) | AST shape (insta) |
| `orgsidian-index` | Schema invariants, FTS5 sync, migration runner | Query input fuzzing | Schema fingerprint (insta) |
| `orgsidian-watcher` | Debounce window, event coalescing | Golden-trace replay | — |
| `orgsidian-vault` | Atomic write retry, Dirty Buffer state machine | Byte-corruption recovery | — |
| `orgsidian-plugin-api` | Trait dispatch, `HookOutcome` semantics, `Event` non-exhaustive | Hook priority ordering invariants | `cargo doc` surface (cargo-semver-checks) |
| `orgsidian-core` | Façade orchestration, integration glue | — | — |
| `orgsidian-report` | `ReportData` serde wiring, template rendering | — | Generated PDF page count + key strings (insta) |

**Execution:** Per-PR on macOS + Ubuntu (Rust unit tests run as part of `cargo test --workspace`).

### 6.3 Layer 3 — CLI Integration (Cross-Crate, LD-27 Primary Surface)

**Mechanism:** `assert_cmd` against `orgsidian-cli` from `crates/orgsidian-cli/tests/<topic>.rs`.

**Per Murat (architecture §Cross-Cutting Concerns 8):** CLI is cheaper than Playwright/Tauri WebDriver for cross-crate integration and exercises `core` directly. Every new core feature should have a CLI command exercising it before shell integration.

**Coverage matrix:**

| CLI Command | Story | Test File | Asserts |
|---|---|---|---|
| `orgsidian parse <file>` | 2.8 | `parse_cmd.rs` | AST output matches insta snapshot |
| `orgsidian index init` | 3.7 | `index_cmd.rs` | DB created, schema version =1 |
| `orgsidian index rebuild` | 3.7 / LD-49 | `index_cmd.rs` | DB dropped + rebuilt; progress to stdout matches UI cadence |
| `orgsidian index stats` | 3.7 | `index_cmd.rs` | Stats output contains headline/file/FTS5 counts |
| `orgsidian index integrity` | 3.7 | `index_cmd.rs` | Exits non-zero on pre-corrupted fixture DB |
| `orgsidian query agenda <range>` | (Epic 6/7) | `query_agenda_cmd.rs` | Today/Week/Custom range output |
| `orgsidian query search <query>` | (Epic 8) | `query_search_cmd.rs` | FTS5 results match expected |
| `orgsidian query backlinks <id>` | (Epic 8) | `query_backlinks_cmd.rs` | Backlinks output for known headline |
| `orgsidian validate-plugin <path>` | (Epic 1/12) | `validate_plugin_cmd.rs` | Contract test runner for plugin authors |
| `orgsidian vault info` / `init` | (Epic 3/6) | `vault_cmd.rs` | Vault metadata output |

**`--json` flag** on every command — exercised in parallel test cases for scriptability.

**Execution:** Per-PR on macOS + Ubuntu.

### 6.4 Layer 4 — React Component (Vitest + happy-dom)

**Mechanism:** Vitest 2.1 + happy-dom (CM6 requires `getComputedStyle` support — Bun test rejected on `vi.mock` ESM hoisting parity per architecture step 3).

**Co-location rule:** `Component.test.tsx` next to `Component.tsx`; `Surface/SurfaceName.test.tsx` next to `index.tsx`.

**Coverage focus:**

| Surface / Component | Story | Key Assertions |
|---|---|---|
| `Editor.tsx` CM6 host | 4.1 | StrictMode-safe lifecycle; `EditorView` destroyed on unmount (spy) |
| `Heading Decoration` | 4.3a | `getComputedStyle(...).fontSize` monotonically decreasing h1→h6 |
| `TodoStateCycler` | 4.3b | Click cycles state; source mutated atomically; `WidgetType.eq()` shallow-equal preserves widget |
| `Tag Pill` | 4.3c | Colons hidden visually but preserved in source |
| `Timestamp Decoration` | 4.3d | Hover>300ms displays raw source tooltip |
| `Checkbox Widget` | 4.3e | Click toggles `- [ ]` ↔ `- [X]`; `widget.ignoreEvent() === false` |
| `Link Decoration` | 4.3f | Bracket markers hidden when cursor off line |
| `ModeSwitcher` | 4.5 | Cycle Raw → Pseudo-WYSIWYG → Split; preference persists per file |
| `Today Dashboard` | 7.1 | Sections render; empty-states; collapse persistence |
| `Backlinks Panel` | 8.7 | Updates <100ms on cursor move |
| `MergeDialog` | 9.1 | 3-pane render; hunk selection; focus management |
| `QuickCapture` (separate window) | 8.1 | Multi-line input; submit appends to inbox; return-focus AC (UJ-2 ≤3s) |
| `CoachingSlot` registry | 11.4 | Dismissal persists; reset action restores all |
| `ReportExport` settings | 10.6 | Scope + range + format pickers; Generate triggers `commands.generateReport` |
| `ConflictBanner` (v0.1 fallback) | 5.5 | Block-save with warning visible |
| `tokens.test.ts` LD-51 snapshot | 12.2 | `--org-*` variable set matches `tokens.snap` |

**Execution:** Per-PR via `pnpm test` on macOS + Ubuntu.

### 6.5 Layer 5 — E2E (Playwright + Tauri WebDriver)

**Mechanism:** Playwright (latest stable) with Tauri WebDriver integration. Tests in `shell-ui/e2e/*.spec.ts`.

**Spine integration tests** (Party Mode round 2 P0 — Sally — UJ-driven coherent journeys, not fragmented unit assertions):

| Spine | UJ | Story | Recommended Addition (TC-7) |
|---|---|---|---|
| `uj1-today-launch.spec.ts` | UJ-1 | — (recommend new) | Open laptop → Today Dashboard renders today's Scheduled + Deadline + Inbox + Active Clock within FR-6 NFR |
| `uj2-quick-capture-roundtrip.spec.ts` | UJ-2 | partial via 8.1 | Global hotkey → dialog → submit → return-focus to prior app ≤3s |
| `uj3-report-spine.spec.ts` | UJ-3 | 10.7 | 4-week project → Project Report → date range → PDF → assert open-clock ⚠ flag visible page 1 |
| `uj4-starter-vault-first-launch.spec.ts` | UJ-4 | — (recommend new) | First-launch → Starter Vault picker → Personal GTD/Student → Today Dashboard non-empty + coaching balloons (Story 6.6) |
| `uj5-merge-dialog.spec.ts` | UJ-5 | — (recommend new for Epic 9) | External write on Dirty Buffer → Merge Dialog 3-pane → hunk selection → atomic save |
| `uj6-search-spine.spec.ts` | UJ-6 | 8.8 | `Cmd+P` → "kubernetes ingress" → grouped results → click → editor at headline → backlinks sidebar |
| `a11y-keyboard.spec.ts` | NFR-9 | 13.5 | axe-core 0 serious/critical; focus rings; Tab order |

**Execution:**
- Per-PR on macOS-arm64 + Ubuntu-LTS for **named spine specs** + a11y (selective).
- Nightly: full E2E suite on macOS + Ubuntu + Arch + Windows.
- Per LD-32: Windows-spine pilot during Epic 8 (Story 8.8 retrospective) to de-risk TC-7.

### 6.6 Layer 6 — Property-Based (proptest + fast-check)

**Mechanism:**
- Rust: `proptest` for randomized input generation; bounded shrinking on failure.
- TypeScript: `fast-check` for JS-side invariant tests (e.g., specta IPC payload round-trip).

**Coverage:**

| Property | Crate / Layer | Story | Invariant |
|---|---|---|---|
| Round-trip L1 (semantic-preserving) | `orgsidian-parser` | 2.4 | `serialize(parse(s)) == serialize(parse(serialize(parse(s))))` for randomized headlines |
| FTS5 query syntax fuzzing | `orgsidian-index` | (Epic 8) | No panic on malformed queries; results stable across equivalent inputs |
| `ConflictState` hash invariants | `orgsidian-vault` | 5.3 | `ancestor_hash` deterministic across re-merges |
| Atomic-write corruption recovery | `orgsidian-vault` | 3.1 | `proptest` with random byte corruption → recovery via `.tmp.<pid>` orphan cleanup |
| Config corruption (LD-41) | (top-level) | 1.11 | Random byte corruption of `<Vault>/.orgsidian/settings.json` → backup + fall back to defaults |
| `tauri-specta` IPC payload | (frontend) | 1.4 | `fast-check` randomized Rust struct → JSON → TS round-trip; null/undefined invariant per architecture step 5 |

**Execution:** Per-PR on macOS + Ubuntu, time-bounded (`proptest!` config `cases: 256`; nightly raises to `cases: 4096`).

### 6.7 Layer 7 — Chaos / Fault Injection (`fail` crate + LD-41 harness)

**Mechanism:** `tests/failure_modes.rs` cross-cutting harness (Story 1.11) using the `fail` crate for fault-injection at specific code sites. `crates/test-plugin-panic/` chaos crate (LD-38).

**Per Murat (Story 1.11 acceptance):** all 9 LD-41 failure-mode categories enumerated as placeholder `#[ignore = "implemented in Epic N"]` tests at v0.1 Alpha; each ignore is progressively replaced by real implementation; `tests/failure_modes_coverage.rs` fails CI if any category has only `#[ignore]` past v0.5 Beta release tag.

**LD-41 Coverage Matrix:**

| Failure Mode | Owning Epic | Mechanism |
|---|---|---|
| Malformed `.org` in vault | Epic 2 | Fixture corpus with deliberately broken files |
| Disk full ENOSPC during atomic write | Epic 3 | `fail::cfg("atomic-write::after-tmp-rename", "panic")` |
| Config corruption | Epic 3/6 | `proptest` random byte corruption of `settings.json` |
| Vault folder deleted runtime | Epic 5 | CI integration test `rmrf` mid-session |
| Plugin `init()` panic | Epic 1 | `crates/test-plugin-panic-init` chaos plugin |
| Plugin `on_event` / hook panic | Epic 1 | `crates/test-plugin-panic-runtime` chaos plugin |
| SQLite index corruption | Epic 3 | Fixture: pre-corrupted `.db` file; `PRAGMA integrity_check` on startup |
| `.tmp` orphan files from crash | Epic 3 | Test: `kill -9` mid-write fixture; restart cleans orphans |
| External delete with Dirty Buffer | Epic 5 | Integration test with watcher harness |

**Plus TC-6 hardening (Epic 12 v1.0):**
- Plugin infinite loop (timeout watchdog)
- Plugin unbounded memory (memory ceiling)

**Execution:** Per-PR on macOS + Ubuntu; nightly extends to Windows.

### 6.8 Layer 8 — Memory Soak (LD-43, Nightly)

**Mechanism:** Dedicated Linux runner job runs a 12-hour scripted session: 200 buffer open/close cycles + 50 plugin re-init cycles + 1000 agenda queries. RSS sampled every 30 min via `/proc/self/statm`. Fails if drift >10% over 11h (warmup excluded, minute 60 → minute 720).

**Activation timeline:** Story 4.9 (Epic 4) — anticipated from Epic 6 per Party Mode P1 because CM6 decorations are the most likely leak source.

**Triage:** `dhat` heap profiler attached to a separate diagnostic run on demand; reports committed to `docs/perf/memory-soak-reports/`.

**Cross-platform extension (TC-4):** Linux-only soak is a known v1.0 gap; v1.5+ extension to tri-platform considered.

**Execution:** Nightly only (12h budget). Failure blocks all PR merges via stale-nightly merge-gate (LD-32).

### 6.9 Layer 9 — Perf Snapshot (Story 1.12 shared infrastructure)

**Mechanism:** `assert_no_perf_regression!(story_id, baseline_path, || { … })` macro in `crates/orgsidian-core/src/test_support/perf.rs`. Runs `op` 5 times, computes median, compares against `tests/perf-baselines/{story_id}.json`. Fails if median exceeds baseline by >20%. Missing-baseline mode (first run) writes baseline + emits non-fatal warning.

**Consumed by:**

| Story | Perf Target (baseline) | NFR |
|---|---|---|
| 4.3g (source-position fidelity) | Read/write source offset operation | — |
| 6.3 (Today Agenda) | Today render <500ms on 1k-file vault | FR-6 |
| 7.1 (Today Dashboard full) | <500ms on 1k-file vault | FR-6 |
| 8.1 (Quick Capture roundtrip) | hotkey → submit → return-focus ≤3s; ≤1s end-to-end persist | FR-10, NFR-5 |
| 8.4 (FTS5 search) | <200ms first 50 results, 1k-file vault | NFR-4 |
| 9.1 (Merge Dialog open) | ≤2s from conflict detection event | — |
| 10.6 (Project Report typical scope) | <5s for 50 headlines, 4 weeks | FR-14 |
| Editor open 5000-line | <300ms first screen | NFR-6 |
| Agenda recompute after edit | <100ms (incremental) | NFR-3 |

**Per TC-2:** baseline files should include `runner_class` field to scope comparisons; this is a Story 1.12 design addendum.

**Execution:** Per-PR on macOS + Ubuntu.

### 6.10 Layer 10 — Three-Level Round-Trip Oracle (LD-44 / LD-45)

The **spine of the FR-2 trust contract**. Distinct from the "test pyramid" — this is the per-PR + nightly gate set.

| Level | Mechanism | Corpus | Cadence | Story |
|---|---|---|---|---|
| **L0** | Byte-identical save-no-op | LD-44 subset (~100 files, syntax-feature matrix + size buckets + edge-case bucket) | Per-PR <60s | 2.6 |
| **L1** | Property-based round-trip (proptest randomized) | Generated (no fixture) | Per-PR, bounded shrinking | 2.4 |
| **L2** | Emacs ground-truth oracle | LD-45 subset against canonical AST | Nightly | 2.7 |

**LD-45 divergence triage:**
- Both Emacs versions concordant against Orgsidian → Orgsidian bug (PR-blocking).
- Both Emacs versions discordant from each other → log in `KNOWN_DIVERGENCES.md`; not PR-blocking.
- One concordant, one discordant → human review; defer decision.

### 6.11 Layer 11 — Cross-Platform Matrix (LD-32)

**Per-PR matrix (`<90s` wall-clock total target):**
- macOS-arm64 + Ubuntu-LTS
- `cargo build/test/clippy -- -D warnings/fmt --check`
- `pnpm typecheck/test`
- L0 round-trip subset gate (~100 files <60s)
- Perf snapshot regression (±20% median of 5)
- Anchor smoke tests (Story 1.9)
- Cargo-deny + cargo-audit
- Cargo-semver-checks (from v0.5 onward)
- Lingui catalog drift gate (LD-52)
- LD-51 tokens snapshot test
- `tests/traceability.rs` FR ↔ doc-comment gate
- Selective E2E spines (UJ-3, UJ-6) + a11y axe-core

**Nightly (full matrix):**
- macOS-arm64 + Ubuntu-LTS + Arch Linux + Windows (Windows added from Epic 1, feature-equivalent from Epic 13)
- Full round-trip corpus (~2000 assertions from `test-org-element.el`)
- L2 Emacs oracle (`emacs:29.x` + `emacs:30.x`)
- Memory soak 12h (Linux)
- Large-vault scaling 10k/25k/50k synthetic vaults (LD-42)
- Perf trend dashboard
- Full E2E suite cross-platform
- LD-41 failure-mode harness full
- LD-38 chaos test plugin matrix

**Merge gate (LD-32):**
- PR can only merge if per-PR job is green AND most recent nightly is green within last 24h.
- Stale-nightly (>24h failing) blocks all merges to `main`.

**Release pipeline (triggered by `v*` tag):**
- Build per platform → sign per LD-19 → publish to GitHub Releases + auto-update endpoint (LD-20).

**Windows-specific failure modes** (Epic 13 hardening):
- ReadDirectoryChangesW edge cases (renames, network mounts, case-folding)
- MoveFileExW atomic semantics + AV/Search-indexer transient locks
- MSI signing + EV cert upgrade decision
- WebView2 quirks vs WebKit / WebKitGTK
- Quick Capture cold-start latency on Windows (R-013)

---

## 7. ATDD Red-Phase Enforcement (Process Discipline Rule A)

**Per Murat (Party Mode round 1 P0):** Epic-level granularity is necessary but not sufficient for spec-driven AI-agent implementation. Story-level enforcement is required to avoid "implement then maybe write tests" collapse.

### 7.1 The Merge Gate (Process Discipline A)

A PR cannot merge unless:

1. **(a)** the story has acceptance criteria authored via `bmad-create-story` skill;
2. **(b)** a red-phase test exists, was committed first in the PR series, and references the story-id;
3. **(c)** the test transitions red → green via the production code in the same PR series.

**CI verification (TC-1 — must land in Epic 1):** GitHub Actions workflow scans commits in the PR for a test file marked with the story-id; asserts the test existed in failing state at commit N before production code in commit N+k.

### 7.2 Story Sizing (Process Discipline A.4)

- Target 5–10 stories per epic, ~7–15h each.
- Epics flagged for sharding during epic-planning if they exceed 12 stories.

### 7.3 Red-Phase Scaffolds Expected Per Story Type

Each story below is dev-loop instantiated via `bmad-testarch-atdd`. The red-phase scaffold is part of the story spec — committed before any production code.

#### 7.3.1 Parser / Serializer Story (Stories 2.2, 2.3, 2.4)

```rust
// crates/orgsidian-parser/tests/semantic.rs
// Red-phase scaffold for Story 2.3 (semantic layer, construct: heading_with_todo)
#[test]
fn semantic_heading_with_todo() {
    let source = "* TODO Buy milk :grocery:";
    let ast = orgsidian_parser::parse_semantic(source);  // FAILS — function doesn't exist yet

    let headline = ast.headlines().next().unwrap();
    assert_eq!(headline.todo_state(), Some(TodoState::Todo));
    assert_eq!(headline.tags(), &["grocery"]);
    assert_eq!(headline.title(), "Buy milk");
}
```

**Plus:** module carries `//! Implements FR-1` as first doc-comment line — verified by `tests/traceability.rs`.

#### 7.3.2 Round-Trip Story (Story 2.4)

```rust
// crates/orgsidian-parser/tests/round_trip.rs
// Red-phase scaffold for Story 2.4 (round-trip serializer)
use proptest::prelude::*;

proptest! {
    #[test]
    fn round_trip_is_byte_identical(headlines in arb_headlines()) {
        let source = orgsidian_parser::serialize(&headlines);
        let reparsed = orgsidian_parser::parse_semantic(&source);
        let reserialized = orgsidian_parser::serialize(&reparsed.headlines());
        prop_assert_eq!(source, reserialized);  // FAILS — serializer not implemented
    }
}

#[test]
fn round_trip_subset_corpus() {
    let subset = load_subset_pr_corpus();  // fixtures/subset-pr.json
    for file in subset.files() {
        let source = std::fs::read_to_string(&file.path).unwrap();
        let ast = orgsidian_parser::parse_semantic(&source);
        let serialized = orgsidian_parser::serialize(&ast.headlines());
        assert_eq!(source, serialized, "byte-identical violation in {}", file.path.display());
    }
}
```

#### 7.3.3 Index / Query Story (Stories 3.3, 3.5, 6.5, 8.4)

```rust
// crates/orgsidian-index/tests/queries.rs
// Red-phase scaffold for Story 8.4 (FTS5 search)
#[tokio::test]
async fn search_fts5_unicode() {
    let pool = test_pool().await;
    seed_fixture_vault(&pool, "unicode-heavy").await;

    let results = pool.query()
        .search("kubernetes ingress")  // FAILS — method doesn't exist
        .limit(50)
        .execute()
        .await
        .unwrap();

    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.matched_line.contains("kubernetes")));
    // Perf: bounded by Story 1.12 assert_no_perf_regression elsewhere
}
```

#### 7.3.4 Watcher Story (Stories 5.1, 5.2, 5.3)

```rust
// crates/orgsidian-watcher/tests/debounce.rs
// Red-phase scaffold for Story 5.1 (debounce coalescing)
#[test]
fn vim_golden_trace_coalesces_to_one_event() {
    let trace = load_golden_trace("vim");  // tests/golden_traces/vim.json
    let mut watcher = test_watcher_with_deterministic_clock();

    let emitted = watcher.replay(trace);
    assert_eq!(emitted.len(), 1, "vim atomic-save must coalesce to one FileChanged event");
    assert_eq!(emitted[0].path, "test.org");
}
```

#### 7.3.5 Vault / Atomic-Write Story (Stories 3.1, 3.2)

```rust
// crates/orgsidian-vault/tests/atomic.rs
// Red-phase scaffold for Story 3.1 (AV-retry wrapper)
#[test]
fn atomic_write_retries_on_av_lock() {
    let fs = FaultInjectingFs::new();
    fs.inject(FaultPattern::PermissionDenied { attempts: 2 });  // first 2 attempts fail

    let result = atomic_write_with_retry(&fs, "test.org", b"content");
    assert!(result.is_ok());
    assert_eq!(fs.attempts(), 3);  // 2 failures + 1 success
    // Note: with FAILS until retry wrapper is implemented
}
```

#### 7.3.6 Plugin API Story (Stories 1.5, 6.5, 8.9, 9.5, 12.4)

```rust
// crates/orgsidian-plugin-api/tests/contract.rs
// Red-phase scaffold for Story 1.5 (trait surface)
struct TestPlugin;

impl OrgsidianPlugin for TestPlugin {
    fn metadata(&self) -> PluginMetadata { /* ... */ }
    fn init(&mut self, _ctx: PluginContext) -> Result<()> { Ok(()) }
    fn shutdown(&mut self) -> Result<()> { Ok(()) }
    fn on_save_before(&mut self, _ctx: &HookContext, content: &str) -> Result<HookOutcome<String>> {
        Ok(HookOutcome::Replace(format!("# Modified\n{}", content)))
    }
}

#[test]
fn hook_outcome_replace_modifies_content() {
    let mut plugin = TestPlugin;
    let ctx = HookContext::test();  // FAILS — HookContext doesn't exist yet
    let outcome = plugin.on_save_before(&ctx, "hello").unwrap();
    assert!(matches!(outcome, HookOutcome::Replace(c) if c.starts_with("# Modified")));
}
```

#### 7.3.7 CodeMirror Decoration Story (Stories 4.1, 4.3a-g, 4.4)

```typescript
// shell-ui/src/components/editor/decorations/Heading.test.tsx
// Red-phase scaffold for Story 4.3a (heading hierarchy decorations)
import { render } from '@testing-library/react';
import { Editor } from '../Editor';
import { expect, test } from 'vitest';

test('Story 4.3a: heading levels render with monotonically decreasing font-size', async () => {
  const source = "* H1\n** H2\n*** H3\n**** H4\n***** H5\n****** H6";
  const { container } = render(<Editor mode="pseudo-wysiwyg" initialContent={source} />);

  const headingElements = Array.from(container.querySelectorAll('.cm-line.org-heading'));
  expect(headingElements).toHaveLength(6);  // FAILS — decoration not implemented

  const fontSizes = headingElements.map(el => parseFloat(getComputedStyle(el).fontSize));
  for (let i = 1; i < fontSizes.length; i++) {
    expect(fontSizes[i]).toBeLessThan(fontSizes[i - 1]);
  }
});
```

#### 7.3.8 UI Surface Story (Stories 6.2, 6.6, 7.1, 11.3, 11.4)

```typescript
// shell-ui/src/components/today/TodayDashboard.test.tsx
// Red-phase scaffold for Story 7.1 (Today Dashboard surface)
test('Story 7.1: Today Dashboard renders scheduled + deadline + inbox + active clock sections', async () => {
  const fixture = mockFixtureVault({
    scheduled: ['Buy milk', 'Client call'],
    deadlines: ['Pay rent'],
    inbox: ['Random thought 1'],
    activeClock: { headline: 'Deep work session' },
  });

  const { getByRole } = render(<TodayDashboard fixture={fixture} />);

  expect(getByRole('region', { name: /scheduled today/i })).toBeInTheDocument();  // FAILS
  expect(getByRole('region', { name: /deadlines/i })).toBeInTheDocument();
  expect(getByRole('region', { name: /inbox/i })).toBeInTheDocument();
  expect(getByRole('status', { name: /active clock/i })).toHaveTextContent('Deep work session');
});
```

#### 7.3.9 CLI Command Story (Stories 2.8, 3.7, 6.5)

```rust
// crates/orgsidian-cli/tests/index_cmd.rs
// Red-phase scaffold for Story 3.7 (`orgsidian index rebuild`)
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn index_rebuild_drops_and_recreates_db() {
    let vault = tempdir_with_fixture_vault();

    Command::cargo_bin("orgsidian").unwrap()
        .args(["index", "rebuild", vault.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("rebuilt"));  // FAILS

    Command::cargo_bin("orgsidian").unwrap()
        .args(["index", "stats", vault.path().to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\":1"));
}
```

#### 7.3.10 Perf-AC Story (Stories 4.3a-g, 6.3, 7.1, 8.1, 8.4, 9.1, 10.6)

```rust
// crates/orgsidian-index/tests/perf_search.rs
// Red-phase scaffold for Story 8.4 perf gate
use orgsidian_core::test_support::perf::assert_no_perf_regression;

#[test]
fn search_perf_snapshot() {
    let pool = pool_with_1k_file_vault();
    assert_no_perf_regression!(
        "story-8.4-fts5-search-1k-vault",
        "tests/perf-baselines/story-8.4.json",
        || {
            let _results = pool.query().search("kubernetes").limit(50).execute();
        }
    );  // First run: writes baseline + warning. Subsequent runs: ±20% median-of-5 gate.
}
```

#### 7.3.11 E2E Spine Story (Stories 8.8, 10.7, plus recommended UJ-1/2/4/5)

```typescript
// shell-ui/e2e/uj3-report-spine.spec.ts
// Red-phase scaffold for Story 10.7 (UJ-3 spine)
import { test, expect } from '@playwright/test';

test('Story 10.7: Sofia ships a client report with open-clock warning', async ({ page }) => {
  await openFixtureVault(page, 'uj3-project-report');

  await page.click('text=My Client Engagement');  // open project file
  await page.keyboard.press('Control+Shift+R');   // Project Report action — FAILS if not bound

  await page.click('text=Last 4 weeks');
  await page.click('text=PDF');
  await page.click('text=Generate');

  // Wait for save dialog; capture path
  const downloadPromise = page.waitForEvent('download');
  await page.click('text=Save');
  const download = await downloadPromise;

  const pdfPath = await download.path();
  const pdf = await readPdf(pdfPath!);

  expect(pdf.totalPages).toBeGreaterThanOrEqual(1);
  // Open-clock warning must appear on page 1 (not buried below the fold)
  const page1Text = pdf.pages[0].text;
  expect(page1Text).toMatch(/⚠.*Deep work session.*Clock running/);
});
```

#### 7.3.12 A11y Story (Story 13.5)

```typescript
// shell-ui/e2e/a11y-keyboard.spec.ts
// Red-phase scaffold for Story 13.5 (axe-core gate + keyboard navigation)
import { test, expect } from '@playwright/test';
import { injectAxe, checkA11y } from '@axe-core/playwright';

const surfaces = ['/today', '/agenda/today', '/editor', '/settings/general'];

for (const surface of surfaces) {
  test(`Story 13.5: ${surface} has 0 serious/critical a11y violations`, async ({ page }) => {
    await page.goto(surface);
    await injectAxe(page);
    await checkA11y(page, undefined, {
      detailedReport: true,
      detailedReportOptions: { html: false },
      axeOptions: { resultTypes: ['violations'] },
      includedImpacts: ['serious', 'critical'],
    });  // FAILS until all serious/critical issues resolved
  });
}
```

#### 7.3.13 Round-Trip CI Gate Story (Stories 2.6, 2.7)

```yaml
# .github/workflows/pr.yml — Red-phase scaffold for Story 2.6
# Workflow references the test before the test exists; CI fails until Story 2.4 lands the implementation
- name: L0 round-trip subset gate (<60s)
  run: |
    cargo test -p orgsidian-parser round_trip_subset -- --test-threads=4
  timeout-minutes: 1
```

#### 7.3.14 Failure-Mode Harness Story (Story 1.11)

```rust
// tests/failure_modes.rs — Red-phase scaffold for Story 1.11
// Enumerate all 9 LD-41 categories with #[ignore] placeholders

#[test]
#[ignore = "implemented in Epic 2"]
fn malformed_org_file_quarantined() {
    let vault = fixture_vault_with_malformed("broken-headline.org");
    let result = orgsidian_core::open_vault(vault.path());
    assert!(result.is_ok());
    let stats = result.unwrap().vault_meta().get_stats();
    assert!(stats.contains("broken-headline.org"), "malformed file must be marked quarantined");
}

#[test]
#[ignore = "implemented in Epic 3"]
fn disk_full_atomic_write() {
    fail::cfg("atomic-write::after-tmp-rename", "panic").unwrap();
    let vault = test_vault();
    let result = vault.save_file("test.org", "content");
    assert!(result.is_err());
    // No partial write must remain
    assert!(!vault.path().join("test.org").exists());
    fail::remove("atomic-write::after-tmp-rename");
}

// ... (all 9 LD-41 categories enumerated similarly)
```

**Plus:** `tests/failure_modes_coverage.rs` fails CI if any LD-41 category has only `#[ignore]` placeholders past v0.5 Beta release tag.

---

## 8. Coverage Matrix — FR / NFR → Test Levels → Priorities

Priorities P0–P3 use `test-priorities-matrix.md` criteria:
- **P0:** Blocks core functionality + High risk (≥6) + No workaround
- **P1:** Critical paths + Medium/high risk
- **P2:** Secondary flows + low/medium risk
- **P3:** Nice-to-have, exploratory, benchmarks

### 8.1 P0 — Critical

**Criteria:** Blocks core functionality + High risk + No workaround + Affects majority of users.

| Test ID | FR/NFR | Test Level | Risk Link | Story / Location | Notes |
|---|---|---|---|---|---|
| **P0-001** | FR-2 round-trip L0 | Rust Integration (Round-Trip Oracle L0) | R-017 | Story 2.6 | ~100 files subset, <60s per PR |
| **P0-002** | FR-2 round-trip L2 | Rust Integration (Emacs Oracle L2) | R-017 | Story 2.7 | `emacs:29.x` + `emacs:30.x`, nightly |
| **P0-003** | FR-2 round-trip L1 | Property (proptest) | R-017 | Story 2.4 | Randomized headlines round-trip |
| **P0-004** | FR-1 parse `.org` | Rust Unit + Anchor | R-001 | Stories 1.9, 2.2, 2.3 | Anchor + 14 LD-44 constructs |
| **P0-005** | FR-15 atomic write | Rust Unit + Chaos | R-018 | Stories 3.1, 1.11 | AV-retry + power-loss fault injection |
| **P0-006** | FR-16 Single Writer Rule (v0.1) | Rust Integration + Watcher Trace | R-019 | Stories 5.1–5.5 | Golden traces + ConflictStrategy contract |
| **P0-007** | FR-16 Merge Dialog (v0.5) | Rust Integration + Component + E2E | R-019 | Stories 9.1–9.4, UJ-5 spine | Three-pane diff + hunk selection + atomic save |
| **P0-008** | NFR-15 atomic write reliability | Chaos + Property | R-018 | Stories 1.11, 3.1 | Power loss + AV interference fault injection |
| **P0-009** | NFR-16 Single Writer Rule integrity | Rust Integration | R-019 | Stories 5.3, 9.4 | ConflictState rich struct invariants |
| **P0-010** | NFR-19 round-trip CI gate live | CI Workflow | R-025 | Stories 2.6, 2.7 | Block v0.1 Alpha tag without these |
| **P0-011** | NFR-21 memory soak <10% RSS drift | Memory Soak (nightly) | R-011 | Story 4.9 | LD-43; activated Epic 4 |
| **P0-012** | Anchor smoke (parser/vault/watcher) | Rust Integration | R-024 | Story 1.9 | Anti-placebo-green |
| **P0-013** | LD-41 failure-mode coverage | Cross-Cutting Harness | R-024 | Story 1.11 | Coverage gate fires v0.5 onward |
| **P0-014** | LD-37 supply-chain hygiene | CI Workflow | R-009 | Story 1.7 | cargo-audit + cargo-deny per-PR |
| **P0-015** | LD-38 plugin panic isolation | Chaos | (TC-6) | Story 1.8 + test-plugin-panic | catch_unwind + invoke_plugin_hook macro |
| **P0-016** | ATDD red-phase merge-gate | CI Workflow | R-023 | Epic 1 (TC-1 addendum) | Verifies temporal ordering of test vs production code |
| **P0-017** | FR-22 LD-51 tokens snapshot | Component (Vitest) | — | Story 12.2 | `tokens.test.ts` Vitest snapshot |
| **P0-018** | UJ-3 spine (Project Report + open clock) | E2E (Playwright + Tauri WD) | R-019, R-024 | Story 10.7 | UJ-3 critical edge case (open clock ⚠) |
| **P0-019** | UJ-6 spine (Search + Backlinks) | E2E (Playwright + Tauri WD) | R-024 | Story 8.8 | UJ-6 coherent journey |

**Total P0: ~19 tests**

### 8.2 P1 — High

**Criteria:** Important features + medium/high risk + common workflows + workaround exists but difficult.

| Test ID | FR/NFR | Test Level | Risk Link | Story / Location |
|---|---|---|---|---|
| **P1-001** | FR-3 Editor Modes switch | Component + Rust Integration | R-002 | Stories 4.5 |
| **P1-002** | FR-4 Pseudo-WYSIWYG decorations (7 sub-stories) | Component (Vitest) | R-002 | Stories 4.3a–4.3g |
| **P1-003** | FR-5 default keybindings | Component + E2E | — | Stories 4.6, 4.7 |
| **P1-004** | FR-6 Today Dashboard | Component + E2E (UJ-1 recommended) | — | Story 7.1 |
| **P1-005** | FR-7 Agenda views (Today/Week/Custom + presets) | Component + Rust Integration | R-012 | Stories 7.4, 7.5 |
| **P1-006** | FR-8 Clock in/out/resume + stale-clock prompt | Rust Unit + Component | — | Stories 7.6, 7.7 |
| **P1-007** | FR-9 Schedule/Deadline + recurring timestamps | Component + Rust Unit | — | Story 4.8 |
| **P1-008** | FR-10 Quick Capture latency | E2E (UJ-2 recommended) + Perf | R-013 | Story 8.1 |
| **P1-009** | FR-11 system tray Quick Capture | E2E + Component | — | Story 8.3 |
| **P1-010** | FR-12 FTS5 search latency | Rust Integration + Perf | R-014 | Story 8.4 |
| **P1-011** | FR-13 Backlinks panel | Rust Unit + Component | — | Stories 8.6, 8.7 |
| **P1-012** | FR-14 Project Report PDF generation | Rust Integration + Component | — | Stories 10.1–10.6 |
| **P1-013** | FR-15 Vault designation + initial scan progress | E2E + Rust Integration | — | Story 3.6 |
| **P1-014** | FR-17 SQLite index derived | Rust Integration | R-020 | Stories 3.3, 3.7 |
| **P1-015** | FR-18 Starter Vaults (Personal GTD, Student, Freelancer, Empty) | E2E (UJ-4 recommended) + Component | — | Stories 6.1, 6.2, 11.1, 11.2 |
| **P1-016** | FR-19 Interactive Tutorial | E2E + Component | — | Story 13.3 |
| **P1-017** | FR-20 Plain/Power Mode | Component | — | Story 11.3 |
| **P1-018** | FR-21 Inline Coaching | Component | — | Stories 6.6, 11.4, 11.5 |
| **P1-019** | FR-22 themes + CSS override (LD-51 tokens snapshot) | Component | — | Stories 6.7, 12.1, 12.2 |
| **P1-020** | FR-23 keybinding remapping + conflict detection | Component | — | Story 12.3 |
| **P1-021** | FR-24 Plugin Pattern (semver-checks, surface review) | Rust Integration + CLI | R-026 | Stories 1.5, 6.5, 8.9, 9.5, 12.4 |
| **P1-022** | NFR-1..NFR-7 perf budgets (via Story 1.12) | Perf Snapshot | R-011, R-013, R-015 | Stories 1.12 + perf-AC consumers |
| **P1-023** | NFR-8 cross-platform parity (v1.0 Windows) | CI Matrix (LD-32) | R-004 | Continuous; Epic 13 |
| **P1-024** | NFR-9 a11y axe-core 0 serious/critical | E2E (axe-core) | — | Story 13.5 |
| **P1-025** | NFR-10 Lingui catalog drift | CI Workflow (lingui extract) | — | Story 1.6 onward |
| **P1-026** | NFR-12 no network calls in core paths | CI Workflow (network namespace sandbox) | — | Continuous |
| **P1-027** | NFR-20 perf snapshot ±20% gate | CI Workflow | R-011, R-015 | Story 1.12 |
| **P1-028** | LD-39 multi-instance lockfile | Rust Integration | R-022 | LD-39 acceptance |
| **P1-029** | LD-40 vault-self-contained state | Rust Integration + E2E | — | Continuous (Epics 6, 11, 12) |
| **P1-030** | LD-42 large-vault scaling soft targets | Memory + Perf (nightly) | R-016 | LD-42; nightly synthetic vaults |
| **P1-031** | LD-44 corpus generator self-validation | Meta-Test in `tools/corpus-extractor` | R-017 | TC-3 addendum to Story 2.5 |
| **P1-032** | LD-45 Emacs oracle canonical AST | Hand-Authored + Meta-Test | R-017 | Story 2.7 |
| **P1-033** | LD-50 Plugin event surface review | Manual Gate + Doc | R-026 | Story 12.4 |
| **P1-034** | `tauri-specta` codegen drift gate | CI Workflow (git diff post-build) | R-030 | Story 1.4 |

**Total P1: ~34 tests**

### 8.3 P2 — Medium

**Criteria:** Secondary features + low/medium risk + edge cases + regression prevention.

| Test ID | FR/NFR | Test Level | Story / Location |
|---|---|---|---|
| **P2-001** | LD-3 parser semantic-layer KNOWN_DIVERGENCES.md curation | Doc + Meta-Test | Story 2.3 |
| **P2-002** | LD-41 disk full / `.tmp` orphan / vault deletion edge cases | Chaos | Story 1.11 |
| **P2-003** | LD-42 large-vault 25k/50k scaling | Nightly Synthetic Vaults | LD-42 |
| **P2-004** | Watcher golden traces for additional editors (Sublime, Helix) | Watcher Trace | (Community) |
| **P2-005** | FR-24 plugin chaos test for infinite loops + memory exhaustion (TC-6) | Chaos | Epic 12 hardening |
| **P2-006** | LD-43 memory soak tri-platform extension (TC-4) | Memory Soak (deferred) | v1.5+ |
| **P2-007** | Recurring timestamp edge cases (`+1y`, `+1m`, leap years) | Rust Unit | Story 4.8 |
| **P2-008** | FTS5 Unicode/RTL/CRLF/case-folded paths | Property + Rust Unit | Story 8.4 |
| **P2-009** | Atomic-write network mount fallback (polling) | Rust Integration | LD-21 |
| **P2-010** | LD-19 code signing + auto-update flow | CI Workflow + Manual | Stories 6.8–6.10, 13.1, 13.2 |
| **P2-011** | LD-22 user CSS data exfiltration via `url()` (CSP) | E2E + Manual Threat Model | Story 12.1 |
| **P2-012** | LD-29 TanStack Router typed search params + loader data | Rust Integration + Component | Continuous |
| **P2-013** | LD-30 react-virtual for 1k+ agenda items | Component + Perf | Story 7.1 |
| **P2-014** | LD-35 logging (frontend bridge + rotation) | Manual + Integration | Continuous |
| **P2-015** | Microcopy registry `[draft]` vs `[final]` lint | CI Workflow | Continuous (Process Discipline G) |
| **P2-016** | UJ-1 today dashboard launch spine (recommended addition) | E2E (Playwright + Tauri WD) | Recommended (Epic 7) |
| **P2-017** | UJ-2 quick capture round-trip spine (recommended addition) | E2E | Recommended (Epic 8) |
| **P2-018** | UJ-4 starter vault first-launch spine (recommended addition) | E2E | Recommended (Epic 6) |
| **P2-019** | UJ-5 merge dialog conflict spine (recommended addition) | E2E | Recommended (Epic 9) |
| **P2-020** | Cross-platform packaging (DMG / AppImage / MSI / Homebrew cask / Flathub) | CI Workflow + Manual | Stories 6.8, 6.9, 13.1 |

**Total P2: ~20 tests**

### 8.4 P3 — Low

**Criteria:** Nice-to-have + exploratory + performance benchmarks + documentation validation.

| Test ID | FR/NFR | Test Level | Notes |
|---|---|---|---|
| **P3-001** | LD-47 Tauri ecosystem sync slot benchmark | Manual quarterly | v0.2/v0.3/v0.4 |
| **P3-002** | LD-48 parser fork-and-maintain dry run | Manual (v0.3 budget) | 2-week exercise |
| **P3-003** | LD-50 plugin event surface review walkthrough | Manual | Story 12.4 |
| **P3-004** | LD-42 large-vault perf trend dashboard | Reporting only | `docs/perf/large-vault-scaling.md` |
| **P3-005** | LD-43 memory soak trend dashboard | Reporting only | `docs/perf/memory-soak-reports/` |
| **P3-006** | `docs/user-guide/` link-checker | CI Workflow | Story 13.6 |
| **P3-007** | `cargo doc --no-deps` warning-free for `orgsidian-plugin-api` | CI Workflow | Continuous (LD-26) |
| **P3-008** | Examples plugins skeleton compiles | CI Workflow | Story 1.5, 13.7 |
| **P3-009** | Onboarding pdf/screenshot golden images for v1.0 docs | Manual | Story 13.6 |
| **P3-010** | Public Plugin API SemVer changelog discipline | CI Workflow | LD-26 / LD-33 |

**Total P3: ~10 tests**

### 8.5 Coverage Summary

| Priority | Count | Notes |
|---|---|---|
| **P0** | ~19 | Core trust contract + foundation discipline |
| **P1** | ~34 | All 24 FRs + key NFRs |
| **P2** | ~20 | Edge cases + recommended UJ spines (UJ-1/2/4/5) + cross-platform packaging |
| **P3** | ~10 | Benchmarks + doc hygiene + ecosystem maintenance |
| **Total** | **~83** | (excluding per-construct unit tests, which are ~50 additional inside Story 2.3 LD-44 enumeration) |

---

## 9. Execution Strategy

**Philosophy** (per `test-quality.md` + Murat — keep execution simple, defer only when infrastructure-expensive):

> Run everything in PRs if it completes in <90s wall-clock total. Defer only memory soak (12h), large-vault scaling (multi-hour synthetic vaults), L2 Emacs oracle (Docker pull + emacs --batch latency), and full E2E suite cross-platform.

### 9.1 Per-PR (target <90s wall-clock total)

**Runners:** macOS-arm64 + Ubuntu-LTS (LD-32).

**Suite:**
- `cargo build --workspace`
- `cargo test --workspace` (unit + integration)
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- `pnpm typecheck`
- `pnpm test` (Vitest unit + component)
- **Anchor smoke** (Story 1.9): parser/vault/watcher
- **L0 round-trip subset** (Story 2.6): ~100 files <60s
- **L1 property round-trip** (Story 2.4): bounded shrinking
- **Perf snapshot regression** (Story 1.12): ±20% on perf-AC stories
- **Selective E2E spines** (UJ-3 Story 10.7, UJ-6 Story 8.8, a11y Story 13.5)
- `cargo audit` — fails on RUSTSEC ≥ medium (LD-37)
- `cargo deny check licenses` (LD-37 allowlist)
- `cargo deny check bans` (no duplicate major versions)
- `cargo deny check graph` (LEAF crate rule)
- `cargo-semver-checks` on `orgsidian-plugin-api` + `IndexQuery` (from v0.5 onward) — Story 6.5
- `lingui extract --clean && git diff --exit-code` (LD-52 catalog drift)
- `pnpm test:tokens-snapshot` (LD-51 — `tokens.test.ts`)
- `cargo test traceability` (FR ↔ doc-comment gate)
- `tauri-specta` IPC codegen drift check (`git diff --exit-code` post `pnpm tauri build` codegen step) — R-030
- **ATDD red-phase merge-gate workflow** (TC-1 addendum)

### 9.2 Nightly (full matrix)

**Runners:** macOS-arm64 + Ubuntu-LTS + Arch Linux + Windows.

**Suite:**
- Full per-PR suite on every runner in matrix
- **Full round-trip corpus** (~2000 assertions from `test-org-element.el`) — Story 2.7
- **L2 Emacs oracle** (`emacs:29.x` + `emacs:30.x`) — Story 2.7 / LD-45
- **Memory soak** (12h on dedicated Linux runner) — LD-43 / Story 4.9
- **Large-vault scaling** (10k/25k/50k synthetic vaults) — LD-42
- **Perf trend dashboard** — track median + p95 over time
- **Full E2E suite cross-platform**
- **LD-41 failure-mode harness full coverage** — Story 1.11
- **LD-38 chaos test plugin matrix** — `test-plugin-panic` across init/shutdown/event/hook
- **`proptest` cases raised to 4096** (vs per-PR 256)

**Merge gate** (LD-32): PR can only merge if (a) per-PR job is green AND (b) most recent nightly is green within last 24h. Stale-nightly (>24h failing) blocks all merges to `main`.

### 9.3 Weekly / Milestone

- **Quarterly Tauri ecosystem sync** at v0.2/v0.3/v0.4 milestones (LD-47).
- **Parser-owner fork-and-maintain dry run** (LD-48) — v0.3 budget, 2 weeks.
- **`cargo-semver-checks` divergence sweep** — at every milestone.
- **Advisory exception review** (LD-37) — quarterly via `docs/security/advisory-exceptions.md`.
- **LD-50 plugin event surface review** — at v0.5 milestone (Story 12.4).
- **A11y manual qualitative sign-off** — Story 13.5 at v1.0.
- **Microcopy registry copy-pass** — at v0.5 Beta and v1.0 (Process Discipline G).

### 9.4 Release

**Triggered by:** `v*` tag (LD-33 `cargo-release` workspace-aware).

**Pipeline:**
- Matrix build per platform
- Sign per LD-19 (macOS Developer ID + notarization; Windows code signing; Linux GPG + AppImage signature)
- Publish to GitHub Releases + auto-update endpoint (LD-20 `tauri-plugin-updater`)
- Update CHANGELOG (Keep-a-Changelog format per Story 1.10)
- Trigger Homebrew cask + Flathub manifest updates (LD-34)

---

## 10. Quality Gates

### 10.1 Per-Priority Pass Rates

- **P0 pass rate = 100%** (no exceptions; blocks release)
- **P1 pass rate ≥ 95%** (waivers require documented rationale + Issue link)
- **P2 pass rate ≥ 90%** (informational; trend-monitored)
- **P3 pass rate ≥ 85%** (informational; trend-monitored)

### 10.2 NFR Thresholds (Gate Conditions)

| NFR | Gate | Mechanism | Story |
|---|---|---|---|
| NFR-1 startup <2s | Baseline + ±20% (Story 1.12) | `assert_no_perf_regression!` | (Epic 13 polish) |
| NFR-2 typing <30ms | Baseline + ±20% | Story 1.12 perf snapshot | (Epic 4) |
| NFR-3 agenda recompute <100ms | Baseline + ±20% | Story 1.12 | Story 7.1 |
| NFR-4 search <200ms | Baseline + ±20% | Story 1.12 | Story 8.4 |
| NFR-5 Quick Capture <1s | Baseline + ±20% | Story 1.12 | Story 8.1 |
| NFR-6 editor open <300ms | Baseline + ±20% | Story 1.12 | Story 4.3g |
| NFR-7 memory <500MB resident | Soft target + LD-43 soak gate | LD-43 (≤10% RSS drift) | Story 4.9 |
| NFR-19 round-trip CI gate | L0 100% per-PR + L2 nightly green within 24h | LD-32 stale-nightly gate | Stories 2.6, 2.7 |
| NFR-20 perf snapshot ±10% | ±20% on median of 5 runs (Story 1.12) | `assert_no_perf_regression!` | Story 1.12 |
| NFR-21 memory soak <10% RSS drift | <10% over 11h | LD-43 nightly | Story 4.9 |
| NFR-9 a11y WCAG 2.1 AA | 0 axe-core serious/critical + manual qualitative sign-off | Story 13.5 | Story 13.5 |
| NFR-12 zero network calls in core | Network-namespace sandbox verifies | CI workflow | Continuous |

### 10.3 Risk Mitigation Completion Gates

| Risk | Gate before release tag |
|---|---|
| R-011 (CM6 memory leak) | LD-43 soak green for 7 consecutive nights pre-tag |
| R-017 (round-trip violation) | L0 + L2 green within 24h of release tag |
| R-023 (ATDD red-phase collapse) | CI workflow verifying temporal ordering active |
| R-001 (parser upstream stall) | LD-48 fork-and-maintain dry run completed at v0.3 |
| R-009 (transitive GPL) | `cargo deny check licenses` clean on full closure |
| R-018 (atomic write Windows) | LD-41 disk-full chaos test + AV-retry property test green on Windows nightly |
| R-019 (external-write race) | Watcher golden-trace suite green + Merge Dialog atomicity test (Story 9.3) |
| R-024 (placebo green) | Anchor smoke tests (Story 1.9) green on every release |

### 10.4 Coverage Targets

- **Critical paths (P0 + P1):** ≥85%
- **Security scenarios:** 100% (zero tolerance for unmitigated SEC risks ≥6)
- **Round-trip L0 subset (LD-44):** 100% per PR
- **LD-44 syntax-feature matrix:** every construct ≥3 times in subset; verified by `tools/corpus-extractor` meta-test
- **LD-41 failure-mode coverage:** every category has a passing test (no `#[ignore]`) by v0.5 Beta tag

### 10.5 Non-Negotiable Pre-Release Requirements

Before any release tag (`v0.1-alpha`, `v0.5-beta`, `v1.0`):

- [ ] All P0 tests pass
- [ ] Anchor smoke tests (Story 1.9) green
- [ ] L0 round-trip subset gate green per PR
- [ ] L2 Emacs oracle nightly green within 24h
- [ ] Memory soak (LD-43) green within 24h (from Epic 4 onward)
- [ ] `cargo audit` + `cargo deny` clean (LD-37)
- [ ] `cargo-semver-checks` clean for `orgsidian-plugin-api` + `IndexQuery` (from v0.5 onward)
- [ ] No high-risk (≥6) items unmitigated
- [ ] `tests/traceability.rs` clean (every FR has `//! Implements FR-NN`)
- [ ] LD-50 plugin event surface review committed (before v1.0)
- [ ] Story 13.5 a11y axe-core 0 serious/critical (before v1.0)
- [ ] Cross-platform matrix green: macOS + Linux (v0.1, v0.5); + Windows (v1.0)

---

## 11. Resource Estimates

**Per `test-quality.md` discipline — ranges only, no false precision.** Effort here covers the **test-strategy author-time** (test design, scaffold authoring, CI configuration, fixture governance, oracle setup). Per-story production effort is in the epic estimates, not duplicated here.

### 11.1 Effort by Priority

| Priority | Count | Effort Range | Notes |
|---|---|---|---|
| **P0** | ~19 | **~40–65 hours** | Round-trip oracle (L0/L1/L2), anchor smoke, failure-mode harness, perf snapshot infra, memory soak setup, supply-chain gates, ATDD merge-gate CI |
| **P1** | ~34 | **~50–90 hours** | Per-FR ATDD scaffolds, spine integration UJ-3/UJ-6, plugin chaos test, axe-core gate, semver-checks, CSS tokens snapshot, Lingui catalog drift |
| **P2** | ~20 | **~25–50 hours** | UJ-1/2/4/5 spine additions, watcher golden traces (additional editors), large-vault scaling fixtures, microcopy registry lint, recurring timestamps |
| **P3** | ~10 | **~5–15 hours** | Tauri WebDriver matrix expansion, doc link-checker, fork-and-maintain dry run scaffold |
| **Total** | **~83** | **~120–220 hours** | Spread across the 18-month roadmap (≈10–15% of project budget) |

### 11.2 Timeline (Concentrated Front-Loading)

| Phase | Months | Test-Strategy Work |
|---|---|---|
| Epic 1 (Months 1–2) | ~25–40h | Stories 1.5/1.7/1.8/1.9/1.11/1.12 — supply-chain + CI matrix + anchor smoke + LD-41 harness + perf snapshot infra + ATDD merge-gate (TC-1) |
| Epic 2 (Months 2–3) | ~20–35h | Stories 2.5/2.6/2.7 — `tools/corpus-extractor` + `fixtures.toml` + L0 per-PR + L2 nightly + Emacs canonical AST |
| Epic 3–5 (Months 3–6) | ~15–30h | Watcher golden traces (Story 5.2) + ConflictStrategy contract (Story 5.3) + atomic-write property tests (Story 3.1) + Story 4.9 memory soak activation |
| Epic 6 (Months 5–6) | ~10–20h | Story 6.5 cargo-semver-checks + UJ-4 spine (recommended) + v0.1 Alpha release pipeline |
| Epic 7–10 (Months 7–10) | ~20–35h | UJ-3 + UJ-6 spines (Stories 8.8, 10.7) + UJ-1/2/5 spines (recommended) + perf snapshots per surface |
| Epic 11–12 (Months 10–12) | ~15–25h | Story 12.2 LD-51 tokens snapshot + Story 12.4 LD-50 plugin event surface review |
| Epic 13 (Months 13–18) | ~15–35h | Story 13.5 a11y axe-core + Windows-specific test hardening (R-013 Quick Capture, R-018 atomic write Windows, R-004 WebView2 consistency) |

### 11.3 Assumptions

- Effort includes test design, fixture authoring, CI configuration, debugging, and oracle maintenance.
- Excludes ongoing maintenance after a story closes (estimated ~10–15% per-story tail effort).
- Assumes test infrastructure (factories, fixtures, perf baselines) is built incrementally per epic, not retrofitted at v1.0.
- Assumes baseline of `~10h/week` author capacity (PRD §7.3) sustained; reductions cascade to roadmap (PRD addendum §A.7).

---

## 12. Risk Mitigation Plans (High-Priority Risks ≥6)

Detailed mitigation strategies for the 15 high-priority risks. Reference the risk table in §4 for scoring rationale.

### R-011: CM6 Memory Leak (Score 9) — CRITICAL

**Mitigation Strategy:**
1. Activate LD-43 nightly memory soak gate at Story 4.9 (Epic 4, anticipated from Epic 6 per Party Mode P1).
2. Scripted session: 200 buffer cycles + 50 plugin re-init + 1000 agenda queries over 12h.
3. RSS measured every 30 min via `/proc/self/statm`; drift <10% threshold.
4. Triage via `dhat` heap profiler on demand; reports in `docs/perf/memory-soak-reports/`.
5. v1.0 hardening (TC-4): consider tri-platform soak extension to catch WebKit/WebView2/WebKitGTK divergence.

**Owner:** Author.
**Timeline:** Story 4.9 (~Month 5).
**Status:** Planned.
**Verification:** 7 consecutive nights green pre-release tag.

### R-017: Round-Trip Violation on Real-World `.org` (Score 9) — CRITICAL

**Mitigation Strategy:**
1. L0 byte-identical save-no-op per-PR (Story 2.6) on LD-44 subset (~100 files, <60s).
2. L1 property-based round-trip (Story 2.4) via `proptest` randomized headlines.
3. L2 Emacs oracle nightly (Story 2.7) on `emacs:29.x` + `emacs:30.x` pinned versions.
4. LD-45 divergence triage workflow (both concordant against Orgsidian → PR-blocking).
5. `KNOWN_DIVERGENCES.md` curated by parser-owner.
6. TC-3: meta-test in `tools/corpus-extractor` validating generated subset satisfies LD-44 matrix.

**Owner:** Author (parser-owner).
**Timeline:** Stories 2.4–2.7 (~Months 2–3).
**Status:** Planned.
**Verification:** L0 + L2 green within 24h of release tag.

### R-023: ATDD Red-Phase Collapse Under AI-Agent Velocity (Score 9) — CRITICAL

**Mitigation Strategy:**
1. Process Discipline rule A merge-gate codified.
2. TC-1 addendum: CI workflow scans PR commit history; asserts test-file diff (with story-id tag) predates production-code diff (`Cargo.toml`/`shell-ui/src/`).
3. `bmad-testarch-atdd` skill scaffolds red-phase tests per story type (see §7.3 catalogue).
4. Story sizing: target 5–10 stories per epic at 7–15h each; epics sharded if >12 stories.

**Owner:** Author / CI infra.
**Timeline:** Epic 1 (Story 1.8 addendum, ~Month 1).
**Status:** Planned.
**Verification:** CI workflow active before Epic 2 begins; sample audit every milestone.

### R-001: `tree-sitter-org` Upstream Stalls (Score 6)

**Mitigation Strategy:**
1. LD-48: SHA-pinned git submodule at `crates/orgsidian-parser/grammar/`; no auto-bump; SHA review per upgrade.
2. Named parser-owner role maintains grammar source familiarity.
3. v0.3 reserves 2 weeks for fork-and-maintain dry run.
4. Trigger: no upstream commits >6 months → fork to `orgsidian-org/tree-sitter-org` under MIT.

**Owner:** Parser-owner.
**Timeline:** v0.3 milestone dry run; trigger any time post-v0.1.
**Status:** Planned.

### R-002: CM6 Widget Edge Cases (Score 6)

**Mitigation Strategy:**
1. LD-6 mandatory recipes baked into Stories 4.3a–g:
   - `WidgetType.eq()` shallow-equal on widget props
   - `Transaction.userEvent` for widget-triggered changes
   - No `view.dispatch` inside `update()` while `view.composing`
   - `widget.ignoreEvent() === false` for interactive widgets
2. Vitest+happy-dom component tests per decoration story.
3. Multi-cursor + widget interaction documented as v0.1 limitation.

**Owner:** Author.
**Timeline:** Stories 4.3a–4.3g.
**Status:** Planned.

### R-004: WebView Consistency Across WebKit / WebView2 / WebKitGTK (Score 6)

**Mitigation Strategy:**
1. LD-32 nightly cross-webview matrix from Epic 4 onward.
2. LD-44 subset feeds same fixtures into editor component tests (Vitest).
3. WebKitGTK version pin documented in `docs/architecture/resilience.md`.
4. Windows-spine pilot in Epic 8 retrospective (TC-7) to de-risk v1.0.

**Owner:** Author.
**Timeline:** Continuous (nightly from Epic 4); concentrated in Epic 13.
**Status:** Planned.

### R-009: Transitive GPL Dependency (Score 6)

**Mitigation Strategy:**
1. LD-37 `cargo deny check licenses` allowlist (MIT/Apache-2.0/BSD-2/3/ISC/Unlicense/Zlib/MPL-2.0); GPL/AGPL/proprietary/unknown fail.
2. First-run sweep is the bar; Story 10.1 (typst integration, ~150 transitive crates) is the highest-risk addition.
3. Quarterly advisory exception review in `docs/security/advisory-exceptions.md`.

**Owner:** Author.
**Timeline:** Per-PR (Story 1.7); intensive on Story 10.1.
**Status:** Planned.

### R-013: Quick Capture Latency >1s on Windows (Score 6)

**Mitigation Strategy:**
1. LD-28 separate `quick-capture.html` Vite bundle (small, minimal deps).
2. Story 8.1 `assert_no_perf_regression!` consumes Story 1.12 infra.
3. Lingui v6 (LD-52) 3 kB runtime supports cold-start budget.
4. Nightly Windows perf snapshot from Epic 13.

**Owner:** Author.
**Timeline:** Story 8.1 + Epic 13 hardening.
**Status:** Planned.

### R-015: Editor Open >300ms on 5000-Line File (Score 6)

**Mitigation Strategy:**
1. Story 1.12 `assert_no_perf_regression!` consumed by Stories 4.3a–g.
2. CM6 viewport-based decoration rendering (LD-6).
3. Incremental parse via tree-sitter (LD-3).
4. Baseline established at Story 4.1; regression gate ±20%.

**Owner:** Author.
**Timeline:** Story 4.3g (last decoration story, ~Month 4).
**Status:** Planned.

### R-018: Atomic Write Windows (Score 6)

**Mitigation Strategy:**
1. LD-8 `atomic-write-file` + 3-retry exponential backoff wrapper for AV/Search-indexer.
2. Property tests in `crates/orgsidian-vault/tests/atomic.rs` (Story 3.1) with fault injection.
3. LD-41 catalogue: disk full ENOSPC + `.tmp` orphan cleanup explicitly tested.

**Owner:** Author.
**Timeline:** Stories 3.1 + 1.11.
**Status:** Planned.

### R-019: External-Write Race (Single Writer Rule) (Score 6)

**Mitigation Strategy:**
1. LD-7 ConflictState rich struct from day 1 (Story 5.3 — Party Mode P0).
2. LD-9 watcher abstraction with deterministic fakes for unit tests.
3. Story 5.2 golden traces (vim/VS Code/Emacs save sequences).
4. Story 5.3 `ResolveConflict` trait parametrized test suite covers both `BlockWithWarning` (v0.1) and `ThreePaneMergeDialog` (v0.5).
5. LD-41 "External delete with Dirty Buffer" entry tested.

**Owner:** Author.
**Timeline:** Epic 5 + Epic 9.
**Status:** Planned.

### R-022: Multi-Instance Race (Score 6)

**Mitigation Strategy:**
1. LD-39 `.orgsidian/instance.lock` JSON with PID + heartbeat (30s) + 5min orphan threshold.
2. Dialog "Open read-only / Force unlock / Cancel" when second instance detected.
3. CI test: spawn 2 `orgsidian` processes against same fixture vault; assert no index corruption.

**Owner:** Author.
**Timeline:** LD-39 acceptance (~Epic 3 or 6).
**Status:** Planned.

### R-024: CI Placebo Green (Score 6)

**Mitigation Strategy:**
1. Story 1.9 anchor smoke tests — three real code-path exercises (parser/vault/watcher).
2. Anchor tests deliberately stable (no Story 1.9 fixture changes after Epic 1 closes).
3. Per-PR + anchor tests run on every release pipeline.

**Owner:** Author.
**Timeline:** Story 1.9 (~Month 1).
**Status:** Planned.

### R-025: v0.1 Alpha Ships Without Round-Trip Gate Live (Score 6)

**Mitigation Strategy:**
1. Epic 2 close gate: v0.1 Alpha tag blocked unless Stories 2.6 (L0 per-PR) + 2.7 (L2 nightly) are green.
2. Documented in Epic 6 Story 6.5 (`IndexQuery` freeze gate) flow.

**Owner:** Author.
**Timeline:** Block v0.1 Alpha tag.
**Status:** Planned.

### R-027: Nightly Merge-Gate Flakiness (Score 6)

**Mitigation Strategy:**
1. LD-32 24h staleness window — buffers transient failures.
2. Nightly retry logic + dedicated soak runner (separate from main CI matrix).
3. Failure triage workflow documented per gate (parser oracle, memory soak, perf trend, large-vault scaling each have separate ownership in `docs/perf/` or `docs/architecture/resilience.md`).

**Owner:** Author.
**Timeline:** Continuous CI hygiene.
**Status:** Planned.

---

## 13. Entry Criteria

**Testing infrastructure cannot proceed without these prerequisites:**

- [ ] **Epic 1 closed** with Stories 1.5/1.7/1.8/1.9/1.11/1.12 — CI matrix + supply-chain + anchor smoke + LD-41 harness + perf snapshot infra + ATDD CI gate (TC-1).
- [ ] **`tools/corpus-extractor` built** (Story 2.5) — subset generator + `fixtures.toml` ownership declarations.
- [ ] **L0 per-PR + L2 nightly gates live** (Stories 2.6 + 2.7) before v0.1 Alpha tag.
- [ ] Test environments provisioned: macOS-arm64 + Ubuntu-LTS for per-PR; +Arch Linux + Windows for nightly.
- [ ] Tauri WebDriver Linux + macOS verified working for E2E spines (TC-7 Windows-spine pilot deferred to Epic 8 retro).
- [ ] `fixtures/vault-corpus/` git-LFS tracking active.
- [ ] `tests/perf-baselines/` directory established; first baselines written via Story 1.12 missing-baseline mode.
- [ ] `docs/perf/`, `docs/security/`, `docs/parser/`, `docs/architecture/`, `docs/plugin-api/` skeletons created (Story 1.10).

---

## 14. Exit Criteria

**Per-Release Exit Criteria:**

### v0.1 Alpha

- [ ] All P0 tests pass
- [ ] Anchor smoke tests green
- [ ] L0 round-trip subset gate green per PR
- [ ] L2 Emacs oracle nightly green within 24h
- [ ] LD-41 failure-mode harness committed with `#[ignore]` placeholders (real coverage by v0.5)
- [ ] `cargo audit` + `cargo deny` clean
- [ ] macOS + Linux cross-platform matrix green
- [ ] Round-trip CI gate documented in README as the FR-2 trust contract
- [ ] No open high-priority (≥6) bugs

### v0.5 Beta

All v0.1 Alpha exit criteria PLUS:
- [ ] All P1 tests ≥95% pass rate
- [ ] LD-41 failure-mode coverage gate green (no `#[ignore]` placeholders)
- [ ] LD-43 memory soak ≥7 consecutive nights green
- [ ] LD-51 tokens snapshot test green (CSS theme API contract locked)
- [ ] LD-50 plugin event surface review document committed (Story 12.4)
- [ ] `cargo-semver-checks` clean on `orgsidian-plugin-api` + `IndexQuery`
- [ ] UJ-3 + UJ-6 spine tests green on macOS + Linux
- [ ] Windows nightly green (still no Windows release artifacts — v1.0)

### v1.0

All v0.5 Beta exit criteria PLUS:
- [ ] All P0 + P1 tests pass on macOS + Linux + Windows
- [ ] Story 13.5 a11y axe-core 0 serious/critical + manual qualitative sign-off
- [ ] All NFR perf budgets within ±20% on baseline median of 5 runs
- [ ] Windows MSI packaging + code signing operational
- [ ] Auto-update via `tauri-plugin-updater` operational across all 3 platforms
- [ ] `examples/plugins/{hello-world, agenda-exporter}/` skeletons compile and validate
- [ ] FR traceability (`tests/traceability.rs`) clean: every FR has `//! Implements FR-NN`

---

## 15. Tooling & Access

| Tool / Service | Purpose | Access Required | Status |
|---|---|---|---|
| `vitest` 2.1.x + `happy-dom` | Frontend unit + component tests | npm install | Locked (architecture step 3) |
| `playwright` (latest stable) | E2E + Tauri WebDriver | npm install | Locked |
| Tauri WebDriver | E2E driver for Tauri 2.x apps | Tauri 2.x release | Locked (LD-2) |
| `rstest` (latest stable) | Rust parameterized tests | cargo dep | Locked |
| `proptest` (latest stable) | Rust property-based tests | cargo dep | Locked |
| `insta` (latest stable) | Rust snapshot testing | cargo dep | Locked |
| `assert_cmd` | Rust CLI integration tests | cargo dep | Locked (LD-27) |
| `fail` crate | Fault injection for LD-41 chaos | cargo dep | Locked (Story 1.11) |
| `cargo audit` | RUSTSEC advisory check | install via cargo | Locked (LD-37) |
| `cargo deny` | License allowlist + ban-duplicates + graph LEAF rule | install via cargo | Locked (LD-37) |
| `cargo-semver-checks` | Plugin API + IndexQuery freeze | install via cargo | Locked (Story 6.5) |
| `axe-core` (@axe-core/playwright) | a11y violations gate | npm install | Locked (Story 13.5) |
| `@lingui/cli` | i18n catalog extract + drift gate | npm install | Locked (LD-52) |
| GitHub Actions | CI/CD runner | Repo Actions tab; signing-key secrets | Repo config |
| `emacs:29.x` Docker image | L2 oracle (LD-45) | Docker Hub | Pinned |
| `emacs:30.x` Docker image | L2 oracle (LD-45) | Docker Hub | Pinned |
| `dhat` heap profiler | Memory leak triage (LD-43) | cargo dep (`dhat`) | Locked |
| git-LFS | Versioning binary fixtures | git-lfs install | Setup at scaffold time |
| Apple Developer ID Application certificate | macOS signing | Apple Developer Program | Required for releases |
| Windows code-signing cert | Windows MSI signing (LD-19) | CA-issued (EV upgrade evaluated v1.0) | Required for v1.0 |
| GPG signing key | Linux AppImage + GPG-signed checksums | Maintainer-managed | Required for releases |
| Tauri updater key pair | LD-20 auto-update signing | Generated at scaffold | Required for releases |

**Access requests (before v0.1 Alpha tag):**
- [ ] Apple Developer ID Application certificate
- [ ] GPG signing key generated and committed (public)
- [ ] Tauri updater key pair generated and embedded
- [ ] Windows code-signing cert procured (target: v0.5 Beta or Q3 Year 1, latest by Epic 13 start)

---

## 16. Interworking & Regression

**Services / components impacted across epics:**

| Service / Component | Impact | Regression Scope | Validation |
|---|---|---|---|
| `orgsidian-parser` | Foundation of FR-1, FR-2, every downstream story | LD-44 subset + LD-45 oracle nightly | L0 + L2 round-trip + per-construct unit tests |
| `orgsidian-index` (schema + FTS5) | All Agenda + Search + Backlinks queries | Schema migration tests + FTS5 sync invariants | Story 3.4 migration test + FTS5 sync tests in Story 3.5 |
| `orgsidian-watcher` | FR-16 + Single Writer Rule + Quick Capture isolation | Golden-trace replay + debounce property tests | Stories 5.1 + 5.2 + 5.3 |
| `orgsidian-vault` (atomic writes + Dirty Buffer) | NFR-15 + NFR-16 + every save path | Property tests + LD-41 chaos | Stories 3.1 + 3.2 + 1.11 |
| `orgsidian-plugin-api` | FR-24 + every v1.0 feature (Capture, Search, Report, Theme) | `cargo-semver-checks` + Story 8.9 + Story 9.5 checkpoints + Story 12.4 LD-50 review | semver-checks + checkpoint docs |
| `tauri-specta` IPC bridge | Every IPC command (~30–50) | Codegen drift gate (`git diff --exit-code` post-build) | Story 1.4 |
| CodeMirror 6 host + decorations | FR-3 + FR-4 + R-002 + R-011 + R-015 | Component tests + memory soak + perf snapshot | Stories 4.1 + 4.3a–g + 4.9 |
| Today Dashboard | UJ-1 + FR-6 + NFR-3 | Component tests + perf snapshot + UJ-1 spine (recommended) | Stories 7.1 + recommended spine |
| Agenda views | UJ-1 + FR-6 + FR-7 | Component tests + perf snapshot | Stories 6.3 + 6.4 + 7.4 + 7.5 |
| Quick Capture (separate Tauri window) | UJ-2 + FR-10 + R-013 | E2E (UJ-2 spine recommended) + perf snapshot | Story 8.1 |
| FTS5 search | UJ-6 + FR-12 + R-014 | Perf snapshot + property tests on query syntax | Story 8.4 + UJ-6 spine Story 8.8 |
| Backlinks engine | UJ-6 + FR-13 | Component test + perf snapshot | Stories 8.6 + 8.7 + UJ-6 spine 8.8 |
| Merge Dialog | UJ-5 + FR-16 + R-019 | E2E + Rust integration + atomicity test | Stories 9.1–9.4 + UJ-5 spine recommended |
| Project Report | UJ-3 + FR-14 + R-009 (typst transitive closure) | E2E (Story 10.7) + supply-chain gate | Stories 10.1–10.7 |
| Starter Vault picker | UJ-4 + FR-18 | E2E (UJ-4 spine recommended) + Component | Stories 6.1 + 6.2 + 11.1 + 11.2 |
| Theme engine + CSS tokens | FR-22 + LD-51 | `tokens.test.ts` Vitest snapshot + WCAG AA contrast | Stories 6.7 + 12.1 + 12.2 |
| CLI (`orgsidian-cli`) | LD-27 primary integration surface | `assert_cmd` test per command + `--json` parity | Stories 2.8 + 3.7 + others |
| `orgsidian-report` (typst) | FR-14 + R-009 | Supply-chain gate + PDF page-count snapshot + UJ-3 spine | Stories 10.1–10.7 |

**Cross-team coordination** (in solo-dev regime, this maps to AI-agent vs author handoff):

- **Parser-owner role** (LD-48) — author maintains grammar-source familiarity; tracked via v0.3 dry-run.
- **Microcopy registry** (Process Discipline G) — `[draft]` vs `[final]` lint enforces UX-copy pass before v1.0.
- **LD-50 plugin event surface review** — author manually conducts at v0.5 milestone (Story 12.4 `[MANUAL-GATE]`).

---

## 17. Assumptions and Dependencies

### 17.1 Assumptions

1. **AI-agent implementation is spec-driven** — per PRD addendum §A.7, correctness over velocity is the binding constraint. ATDD red-phase enforcement is therefore both feasible (no time pressure) and necessary (no "I'll write the test later" human shortcut).
2. **Baseline hardware is 2020+ M1 / equivalent x86_64** — perf budgets calibrated to this (PRD §8). GitHub Actions runners are heterogeneous; per TC-2, runner-class scoping mitigates baseline drift.
3. **`nvim-orgmode/tree-sitter-org` upstream remains active** through v1.0 — LD-48 fork-and-maintain dry run + 6-month-stall trigger protect against regression.
4. **`typst-as-lib` 0.15.x is stable** through v0.5 Beta + v1.0 — LD-53 downgrade contingency to `printpdf` 0.9.x recorded if typst regresses.
5. **Tauri 2.x WebDriver is sufficient on macOS + Linux** for E2E spines per PR — TC-7 flags Windows as the de-risk item (Epic 8 retrospective + v1.0 hardening).
6. **The author's ~10h/week capacity is sustained** (PRD §7.3) — if drops <8h/week sustained, roadmap stretches and test-strategy timeline stretches proportionally.
7. **Process Discipline rule A is enforceable via CI** (TC-1 addendum) — the commit-history scan is the binding mechanism; without it, the merge-gate is a verbal contract.

### 17.2 Dependencies

1. **Epic 1 Stories 1.5/1.7/1.8/1.9/1.11/1.12 land before Epic 2 begins** — without them, every downstream test inherits a tainted baseline.
2. **`tools/corpus-extractor` + LD-44 algorithm** (Story 2.5) is a hard dependency for Stories 2.6 + 2.7 (L0 + L2 gates).
3. **L2 Emacs canonical AST** (LD-45) is hand-authored and peer-reviewed — requires author time at Epic 2; no automation shortcut.
4. **Apple Developer ID + Windows code-signing cert** procured before v0.1 Alpha (macOS) and v1.0 (Windows) respectively.
5. **GitHub Actions free-tier runner availability** — assumed sufficient through v1.0; nightly soak runner may need a self-hosted Linux runner if free tier becomes constrained.
6. **`emacs:29.x` + `emacs:30.x` Docker images** remain available on Docker Hub — LD-45 oracle pinning requires both versions.

### 17.3 Risks to the Test Plan Itself

| Risk | Impact | Contingency |
|---|---|---|
| Author capacity drops <8h/week | All timelines stretch; v1.0 slips beyond Month 18 | Re-prioritize: drop P2 spine recommendations (UJ-1/2/4/5) first; keep P0+P1 |
| Tauri 2.x evolves catastrophically | LD-2 stack invalidated; entire E2E pyramid affected | LD-47 fallback to `wry` direct + custom IPC; ~3 weeks pre-budgeted in `docs/architecture/resilience.md` |
| GitHub Actions free tier becomes unworkable | Nightly + cross-platform matrix infeasible | Self-host Linux nightly runner; reduce macOS/Windows nightly cadence to weekly |
| `nvim-orgmode/tree-sitter-org` stalls >6 months mid-roadmap | LD-48 fork triggered; parser-owner absorbs maintenance burden | v0.3 dry run validates feasibility; in-house fork at `orgsidian-org/tree-sitter-org` |
| `typst-as-lib` introduces regression in 0.14.x→0.15.x | Story 10.6 wow-demo PDF rendering breaks | LD-53 downgrade to `printpdf` 0.9.x; ~3 dev-days |
| Heterogeneous GHA hardware breaks perf baselines (TC-2) | False positives/negatives on Story 1.12 gate | runner_class scoping in baseline files; widen tolerance to ±25% if needed |

---

## 18. Implementation Planning Handoff

Test-strategy work items mapped to epic milestones for `bmad-sprint-planning` consumption.

| Work Item | Owner | Target Milestone | Story Reference |
|---|---|---|---|
| CI matrix + supply-chain gates (cargo-deny + cargo-audit + cargo-semver-checks) | Author | Epic 1 | Stories 1.7, 1.8 |
| Anchor smoke tests (parser/vault/watcher) | Author | Epic 1 | Story 1.9 |
| LD-41 failure-mode harness skeleton + coverage gate | Author | Epic 1 | Story 1.11 |
| `assert_no_perf_regression!` macro + perf baseline infra | Author | Epic 1 | Story 1.12 |
| ATDD red-phase CI gate (TC-1 addendum) | Author | Epic 1 | Addendum to Story 1.8 |
| `tools/corpus-extractor` + `fixtures.toml` per-epic ownership | Author | Epic 2 | Story 2.5 |
| L0 per-PR round-trip gate (~100 files, <60s) | Author | Epic 2 | Story 2.6 |
| L2 nightly Emacs oracle gate (canonical AST committed) | Author | Epic 2 | Story 2.7 |
| `tools/corpus-extractor` matrix self-validation meta-test (TC-3) | Author | Epic 2 | Addendum to Story 2.5 |
| Watcher golden traces (vim/VS Code/Emacs) | Author | Epic 5 | Story 5.2 |
| `ConflictState` rich struct + `ConflictStrategy` pattern + parametrized test suite | Author | Epic 5 | Story 5.3 |
| Memory soak gate activation (LD-43) | Author | Epic 4 | Story 4.9 |
| `cargo-semver-checks` automation on `IndexQuery` + `orgsidian-plugin-api` | Author | Epic 6 | Story 6.5 |
| UJ-6 spine E2E test (search + backlinks coherent journey) | Author | Epic 8 | Story 8.8 |
| Plugin API consistency checkpoint (Epic 8) | Author | Epic 8 | Story 8.9 |
| UJ-1 today dashboard launch spine (recommended addition) | Author | Epic 7 | (Recommended) |
| UJ-2 quick capture round-trip spine (recommended addition) | Author | Epic 8 | (Recommended) |
| UJ-4 starter vault first-launch spine (recommended addition) | Author | Epic 6 | (Recommended) |
| UJ-5 merge dialog conflict spine (recommended addition) | Author | Epic 9 | (Recommended) |
| Plugin API consistency checkpoint (Epic 9) | Author | Epic 9 | Story 9.5 |
| UJ-3 spine E2E test (Project Report + open-clock ⚠) | Author | Epic 10 | Story 10.7 |
| LD-51 `tokens.test.ts` Vitest snapshot | Author | Epic 12 | Story 12.2 |
| LD-50 plugin event surface review sign-off | Author | Epic 12 | Story 12.4 |
| Story 13.5 a11y axe-core 0 serious/critical + manual qualitative | Author | Epic 13 | Story 13.5 |
| Windows-specific perf + reliability hardening (R-004, R-013, R-018) | Author | Epic 13 | Stories 13.1, 13.4 |
| TC-4 tri-platform memory soak extension (deferred decision) | Author | v1.0 → decide before Story 13.1 | (Decision gate) |
| TC-6 plugin chaos hardening (infinite loops + memory exhaustion) | Author | Epic 12 hardening | (v1.0 hardening) |
| TC-7 Windows-spine pilot retrospective | Author | Epic 8 retro | (Pilot decision) |

---

## 19. Appendix A: Knowledge Base References

- **Risk Governance** — `risk-governance.md` (TECH/SEC/PERF/DATA/BUS/OPS classification + P×I scoring)
- **Probability-Impact** — `probability-impact.md` (1–3 scoring criteria)
- **Test Levels Framework** — `test-levels-framework.md` (E2E vs API vs Component vs Unit selection rules)
- **Test Priorities Matrix** — `test-priorities-matrix.md` (P0–P3 classification criteria)
- **Test Quality** — `test-quality.md` (Definition of Done: no hard waits, <300 lines, <1.5min)
- **ADR Quality Readiness Checklist** — `adr-quality-readiness-checklist.md`

---

## 20. Appendix B: Cross-Reference Index

### 20.1 LD → Test Strategy

| LD | Test Strategy Section |
|---|---|
| LD-1 (License MIT) | §4 R-009; §9.1 cargo-deny check licenses |
| LD-2 (Tauri 2.x stack) | §6.5 E2E; TC-7 Windows-spine; R-003 |
| LD-3 (parser) | §10.2 L0/L1/L2 oracle; R-001; R-017 |
| LD-4 (SQLite/FTS5) | §6.2 index unit tests; R-005; R-014 |
| LD-5 (monorepo) | §5 fixture architecture; §6.3 CLI integration |
| LD-6 (CodeMirror 6) | §6.4 component tests; R-002 mandatory recipes |
| LD-7 (Single Writer Rule) | §6.7 chaos; R-019; UJ-5 spine |
| LD-8 (atomic writes) | §6.7 chaos; R-018; Story 3.1 |
| LD-9 (notify-rs watcher) | §5.2 golden traces; R-019; R-006 |
| LD-10 (Plugin API internal until v1.5+) | §6.2 `orgsidian-plugin-api` semver-checks; R-026; LD-50 |
| LD-11..LD-16 (Data Architecture) | §6.2 index unit + §6.3 CLI integration |
| LD-17..LD-23 (Security & Sandboxing) | §6.5 a11y + §10 quality gates + R-007, R-009, R-010 |
| LD-24 (`tauri-specta`) | §9.1 codegen drift gate; R-030 |
| LD-25 (static plugin linking) | LD-38 chaos; TC-6 |
| LD-26 (Plugin API trait) | §6.2 unit + §7.3.6 ATDD scaffold + LD-50 review |
| LD-27 (CLI command tree) | §6.3 primary integration surface |
| LD-28..LD-31 (Frontend) | §6.4 component + §6.5 E2E |
| LD-32 (CI matrix) | §6.11 cross-platform matrix + §9 execution + R-027 |
| LD-33..LD-36 (Infra) | §9.4 release; LD-19 signing |
| LD-37 (Supply-chain) | §9.1 per-PR + §10.4 coverage; R-009 |
| LD-38 (Plugin panic isolation) | §6.7 chaos + `test-plugin-panic`; TC-6 hardening |
| LD-39 (Multi-instance lockfile) | R-022; §8.2 P1-028 |
| LD-40 (Vault-self-contained state) | §6.4 + §8.2 P1-029 |
| LD-41 (Failure mode catalog) | §6.7 chaos harness; Story 1.11 |
| LD-42 (Large-vault indexing UX) | §6.11 nightly + R-016; §8.2 P1-030 |
| LD-43 (Memory soak gate) | §6.8 memory soak; R-011; TC-4 |
| LD-44 (L0 subset corpus criteria) | §5.2 `fixtures.toml` + §6.10 + TC-3 |
| LD-45 (L2 Emacs oracle pinning) | §6.10 + R-017 + R-028 |
| LD-46 (PRD reconciliation TODO) | Resolved 2026-05-19 |
| LD-47 (Tauri ecosystem pinning) | §9.3 quarterly sync + R-003 |
| LD-48 (`tree-sitter-org` vendoring + fork contingency) | R-001 + v0.3 dry run |
| LD-49 (`rebuild-index` first-class) | §6.3 CLI Story 3.7 |
| LD-50 (Plugin event surface review) | §10.5 v1.0 gate + Story 12.4 |
| LD-51 (CSS token snapshot test) | §6.4 + Story 12.2 + §8.1 P0-017 |
| LD-52 (Lingui i18n) | §9.1 catalog drift gate |
| LD-53 (typst PDF rendering) | §8.2 P1-012 + R-009 supply-chain first-run |

### 20.2 Process Discipline Rule → Test Strategy

| Rule | Test Strategy Section |
|---|---|
| A. Story-Level ATDD | §7 ATDD red-phase enforcement; R-023; TC-1 |
| B. Persona Controlled Vocabulary | §20 vocab-linter CI gate (Process Discipline B) |
| C. Traceability Discipline | `tests/traceability.rs` FR ↔ doc-comment gate |
| D. User-Voice in `So that` | (review-level — not test-gate) |
| E. Perf Assertions via Shared Infra | §6.9 perf snapshot + Story 1.12 |
| F. AC Refactor Rule (>4 And chains split) | Story 4.3 → 4.3a–g exemplar |
| G. Microcopy Discipline | §9.3 + Story 10.6 `[microcopy: draft]` + microcopy registry lint |

### 20.3 Story Anchor → Test Strategy Owner

| Story | Test Strategy Role |
|---|---|
| Story 1.9 | Anchor smoke (anti-placebo-green) |
| Story 1.11 | LD-41 failure-mode harness |
| Story 1.12 | `assert_no_perf_regression!` shared infra |
| Story 2.5 | `tools/corpus-extractor` + `fixtures.toml` |
| Story 2.6 | L0 per-PR round-trip gate |
| Story 2.7 | L2 nightly Emacs oracle gate |
| Story 4.9 | Memory soak gate activation |
| Story 5.2 | Watcher golden traces |
| Story 5.3 | `ConflictState` + `ConflictStrategy` pattern |
| Story 6.5 | `cargo-semver-checks` automation |
| Story 8.8 | UJ-6 spine E2E |
| Story 10.7 | UJ-3 spine E2E with open-clock ⚠ assertion |
| Story 12.2 | LD-51 tokens snapshot |
| Story 12.4 | LD-50 plugin event surface review |
| Story 13.5 | Story 13.5 a11y axe-core + manual qualitative |

---

## 21. Completion Report

**Mode:** System-Level Test Design.
**Inputs loaded:** PRD (2026-05-19 final), addendum, architecture (53 Locked Decisions LD-1..LD-53), epics (104 stories across 13 epics).
**Output file:** `_bmad-output/test-artifacts/test-design.md` (single consolidated document per user request, replacing the default two-file architecture+QA split).

**Key risks identified:** 30 total, 15 high-priority (≥6), 3 score-9 (R-011 CM6 memory leak, R-017 round-trip violation, R-023 ATDD red-phase collapse).

**Key strategy elements:**
- Three-level round-trip oracle (L0 per-PR / L1 property / L2 Emacs nightly).
- 9-layer test pyramid mapped to 13 epics + cross-platform matrix.
- ATDD red-phase enforcement codified as CI merge gate (Process Discipline A + TC-1 addendum).
- 14 per-story-type ATDD red-phase scaffold templates (§7.3 catalogue).
- Fixture governance via `fixtures.toml` per-epic ownership + git-LFS versioning.

**Open assumptions / next-step decisions:**
- **TC-1:** ATDD red-phase CI verification workflow needs explicit Story 1.8 addendum.
- **TC-2:** Perf baseline runner-class scoping needs Story 1.12 design addendum.
- **TC-3:** `tools/corpus-extractor` matrix self-validation meta-test needs Story 2.5 acceptance addendum.
- **TC-4:** Tri-platform memory soak extension is a v1.0 decision (before Story 13.1).
- **TC-5:** Tauri WebDriver Windows-spine pilot in Epic 8 retrospective.
- **TC-6:** Plugin chaos hardening (infinite loops + memory exhaustion) is v1.0 hardening.
- **TC-7:** UJ-1/2/4/5 spine E2E tests recommended for Epics 6, 7, 8, 9.

**Quality gate posture:**
- P0 100% pass; P1 ≥95%; round-trip L0 + L2 within 24h; memory soak <10% RSS drift; axe-core 0 serious/critical; `cargo audit` + `cargo deny` + `cargo-semver-checks` clean.

**Recommended next workflows:**
1. `bmad-testarch-atdd` to generate the red-phase scaffolds for Epic 1 P0 stories (1.5, 1.7, 1.8, 1.9, 1.11, 1.12) using the §7.3 templates.
2. `bmad-testarch-ci` to scaffold the GitHub Actions workflows (per-PR + nightly + merge-gate + ATDD verification per TC-1).
3. `bmad-testarch-trace` to set up the FR ↔ test traceability matrix.
4. `bmad-testarch-nfr` to formalize the per-NFR threshold review.

---

**End of Test Design Document.**

**Generated by:** BMad TEA Agent (Master Test Architect)
**Workflow:** `bmad-testarch-test-design`
**Version:** 4.0 (BMad v6, system-level mode)
**Date:** 2026-05-19
