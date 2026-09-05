---
title: 'Publish v0.1 Alpha: README, landing page, announcement'
type: 'docs'
created: '2026-09-05'
status: 'review'
baseline_commit: 'dec070f'
review_loop_iteration: 0
github_issue: 61
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** With Stories 6.1-6.9 merged, v0.1 Alpha is feature-complete (Starter Vaults, Today/Week Agenda, the CM6 editor, external-edit safety, dark/light themes, first-run coaching, and signed macOS DMG + Homebrew cask + Linux AppImage packaging) but the project's public-facing surface hasn't caught up: the root `README.md` still describes the project as pre-Alpha with the repository private (stale — the repo has been public since 2026-05-25), there's no landing page to point prospective adopters at, and there's no announcement text drafted for the eventual HN/r/orgmode post that SM-1 (50 technical comments + 10 early adopters) depends on.

**Approach:** This is a docs-only story — no application code changes.

1. Rewrite root `README.md`: correct the stale "repository is private" line, restate the vision, add an **Install** section covering the three v0.1 distribution paths (macOS DMG, Homebrew cask, Linux AppImage) that cross-references `docs/releasing.md` for the full signing/verification detail, add a **v0.1 Alpha feature summary** built strictly from what Epics 1-6 actually shipped (verified against each Story 6.1-6.9 implementation artifact rather than the epic summary's aspirational wording — see Design Notes on the Freelancer starter and the "closed-by-release via Epic 8" wording, both of which do not apply to this baseline), and add a **How to contribute** section pointing at `CONTRIBUTING.md`, the Issues/label scheme, and `SECURITY.md`.
2. Add a minimal, dependency-free static landing page at `docs/landing/index.html` — a single self-contained file (inline CSS, `prefers-color-scheme`-aware light/dark palette, no JS, no external requests) that restates the vision, the v0.1 feature list, and the same three install paths, linking to the GitHub Releases page (generically — no specific tag is referenced, since no tag has been pushed by this story).
3. Commit an announcement draft at `docs/announcements/v0.1-alpha.md` with two variants (Hacker News "Show HN" + Reddit r/orgmode), both explicitly marked as drafts not yet posted, with placeholders for the concrete release/landing-page URLs to fill in once a real tag is published.

**What this story deliberately does NOT do** (per explicit task instruction): tag or publish a release, flip repository visibility (already public — verified pre-existing fact, not an action this story performs), or post either announcement anywhere. Those remain the maintainer's actions once Stories 6.8/6.9's signing secrets are configured in the real repository and a `v0.1.0-alpha.*` tag is pushed.

## Boundaries & Constraints

**Always:**
- Every feature claimed in `README.md` and `docs/landing/index.html` must trace to something that actually shipped in Epics 1-6 (verified against the Story 6.1-6.9 implementation artifacts, not just the epic's summary prose) — no capture/search/backlinks/graph-view claims (those are Epic 8, not in v0.1), and no Freelancer-starter claim (Story 6.1 explicitly deferred it — see Design Notes).
- Role-agnostic phrasing for any maintainer reference ("the current lead maintainer"), per the project's naming convention — no hard-coded person's name.
- No AI-credit language anywhere (no "Generated with", no Co-Authored-By, no attribution line) in any file this story touches.
- `docs/landing/index.html` is a single self-contained file: inline `<style>`, no external script/stylesheet requests, no build step.
- Every relative link added to `README.md` must resolve to a real path in this repo.

**Ask First:**
- Whether/when to actually post either announcement variant — explicitly out of scope; the file says so in its own header.

**Never:**
- Do not run any `gh api` repository-visibility mutation — the repo is already public.
- Do not tag, build, or publish an actual GitHub Release.
- Do not modify `sprint-status.yaml`.
- Do not commit, push, or open a PR — the orchestrator handles git/PR for this worktree.

## Code Map

- `README.md` -- REWRITTEN. Corrects the stale "repository is private during pre-Alpha" line (§ Status), rewrites the vision (§ Why), adds an **Install** section (macOS DMG / Homebrew cask / Linux AppImage, cross-referencing `docs/releasing.md`), adds a **v0.1 Alpha feature summary** section, adds a **How to contribute** section, and keeps (lightly updated) the existing Planning Artifacts / Roadmap / Development / License sections.
- `docs/landing/index.html` -- NEW. Self-contained static landing page: hero (name, tagline, download/repo CTAs), a feature-card grid matching the README's v0.1 feature summary, an install table, and a footer linking to source/issues/contributing. Inline CSS only, `prefers-color-scheme` dark/light palette, no external requests.
- `docs/announcements/v0.1-alpha.md` -- NEW. Draft-only announcement (HN + r/orgmode variants), explicitly marked not-posted, with `<...>` placeholders for the release/landing URLs.

No application code (Rust or TypeScript) is touched by this story.

## Tasks & Acceptance

**Execution:**
- [x] `README.md` rewritten: vision, install paths, v0.1 feature summary, "How to contribute" section; stale private-repo wording corrected.
- [x] `docs/landing/index.html` created: self-contained, dependency-free, theme-aware, points at the GitHub Releases page.
- [x] `docs/announcements/v0.1-alpha.md` committed as a draft (HN + r/orgmode variants), explicitly not posted.
- [x] Every `README.md` relative link verified to resolve to a real file in the repo.
- [x] `docs/landing/index.html` verified well-formed via `html.parser`.
- [x] `cargo build --workspace --offline` reconfirmed clean (docs-only change).

**Acceptance Criteria (from epics.md Story 6.10, adjusted for the project facts below):**
- Given Stories 6.7 + 6.8, when v0.1 Alpha ships, then root `README.md` is rewritten with vision, install paths (DMG/Homebrew/AppImage), feature summary, and a "How to contribute" section. *(Done — see § Install, § v0.1 Alpha feature summary, § How to contribute in `README.md`.)*
- And a minimal landing page exists at `docs/landing/index.html` pointing to the GitHub Release. *(Done — links to `https://github.com/orgsidian/orgsidian/releases/latest` and the repo; no specific tag referenced since none has been pushed yet.)*
- ~~And the `orgsidian/orgsidian` repository visibility is flipped from private to public before the announcement post is published~~ — **stale, already satisfied.** The repository has been public since 2026-05-25 (verified fact recorded ahead of this story); this story performs no visibility mutation and instead corrects the README/architecture-adjacent stale "private during pre-Alpha" wording that assumed the flip was still pending.
- And an announcement draft for HN + Reddit r/orgmode is committed at `docs/announcements/v0.1-alpha.md` (timing/posting at author's discretion). *(Done — two variants, both headed with an explicit "draft, not posted" status line.)*

## Design Notes

- **Why the feature summary omits the Freelancer starter.** Epic 6's summary paragraph in `epics.md` lists "Personal GTD + Student + Freelancer" starters, but Story 6.1's own implementation artifact records a locked scope decision: Freelancer is deferred because its AC depends on the Backlinks panel (Story 8.7, not yet built) to show "≥1 backlink visible" on first launch. `docs/user-guide/starter-vaults.md` confirms only Personal GTD and Student ship in v0.1. The README and landing page reflect that shipped reality, not the epic's original (later-revised) framing.
- **Why the feature summary omits capture/search/backlinks/graph.** Epic 6's own prose contains a forward-reference ("FR-26 Graph View — via Epic 8 stories... closed-by-release") describing a hypothetical execution order where Epics 7+8 land before Epic 6 closes. That reordering did not happen in this baseline: `git log` shows no Epic 7 or Epic 8 story commits on `main` as of `dec070f`, and this worktree was scoped explicitly to Epic 6 Stories 6.1-6.9 only. Claiming those features would misrepresent what v0.1 Alpha adopters actually get, so the README instead lists them under "Not in v0.1 Alpha" pointing at the Roadmap.
- **Why the README's status badge changed from "in progress" to "code-complete."** Stories 6.1-6.9 are all merged (`git log` on `main`), but no `v0.1.0-alpha.*` tag has been pushed (`CHANGELOG.md` still shows only `[Unreleased]`, `release.toml`'s own header notes the runbook is "deferred... likely tied to Story 6.10"). "Code-complete, release tag pending" is the accurate state; claiming a release exists would be false, and leaving "in progress" would understate that Epic 6 is actually done.
- **Why the landing page has no JavaScript.** The task calls for "minimal" and "dependency-light"; a static page whose only job is to restate the pitch and link out to GitHub/Homebrew/the AppImage doesn't need any client-side behavior, so there is none — reduces the audit surface to zero scripts.
- **Why the announcement draft is two variants in one file rather than two files.** Both are the same underlying pitch reshaped for each forum's norms (HN: terse, technical, link-forward; r/orgmode: a little more context and org-mode-specific framing, since that audience already knows what org-mode is and is being asked to evaluate a non-Emacs alternative to it). Keeping them in one file makes it easy for the maintainer to review and keep both in sync before either is posted.
- **Role-agnostic maintainer reference.** "the current lead maintainer" is used in `README.md`'s "How to contribute" section rather than naming a person, per the project's established documentation convention (role-agnostic naming in CONTRIBUTING/MAINTAINERS-class docs).

## Verification

**Commands:**
- `git submodule update --init --recursive` -- expected: `tree-sitter-org` submodule present. Ran clean (submodule was not yet initialized in this fresh worktree; now checked out at `219c0b27fdb2c0aeb43841f23f03d6f54657f288`).
- `python3 -c "import html.parser; html.parser.HTMLParser().feed(open('docs/landing/index.html').read())"` -- expected: no exception. Passed.
- README relative-link sweep (regex-extract every `](...)` target that isn't `http`/`#`, verify each resolves via `os.path.exists`, stripping a leading `./`) -- expected: zero unresolved links. Result: 19 relative links checked, 0 unresolved.
- `cargo build --workspace --offline` -- expected: builds clean (docs-only change, confirms nothing else broke). Passed — `Finished` dev profile, all 9 workspace crates + `orgsidian-cli`/`orgsidian-report` compiled, no warnings surfaced beyond normal dependency-compile noise.

**Result (2026-09-05):** All four checks passed in this worktree. No Rust or TypeScript source was touched, so no test suite beyond the build was run.

## Spec Change Log

- 2026-09-05 — Implemented. `README.md` rewritten (vision, install paths, v0.1 feature summary scoped strictly to what Epics 1-6 shipped, "How to contribute" section, stale private-repo wording corrected). `docs/landing/index.html` added (self-contained, dependency-free, theme-aware). `docs/announcements/v0.1-alpha.md` added (HN + r/orgmode drafts, explicitly not posted). Feature list cross-checked against each Story 6.1-6.9 implementation artifact — Freelancer starter and capture/search/backlinks/graph-view claims excluded as not-yet-shipped, correcting two places where `epics.md`'s summary prose no longer matches the actual v0.1 baseline. Status → review.

</frozen-after-approval>
