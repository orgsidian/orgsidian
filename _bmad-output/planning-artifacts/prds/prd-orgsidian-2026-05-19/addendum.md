# Orgsidian — PRD Addendum

Material from the brainstorming and the Discovery research that belongs downstream of the PRD body — architecture decisions, parser license analysis, stack rationale, alternative-options matrices, in-depth persona context, and the rejected-alternative reasoning. This document is the bridge from the PRD's *what* to `bmad-create-architecture`'s *how*.

The PRD body is the contract with the reader. This addendum is the working memory for the next workflow.

---

## A.1 Brainstorming source map

The PRD body is derived from `_bmad-output/brainstorming/brainstorming-session-2026-05-18-1613.md`. Quick cross-reference for traceability:

| PRD section | Brainstorming source |
|---|---|
| §1 Vision | Phase 1 — What If Scenarios; Phase 3 — First Principles Candidate A (Positioning) |
| §2 Target User | Phase 2 — Mind Mapping Tema 2 (Posizionamento & Target); Action Plan 1-4 |
| §4.1 Editor & Org-mode Fidelity | Phase 3 — Candidate B (Architettura); Theme 1 — Architettura |
| §4.2 Planner Core | Phase 1 — UX/Editor cluster; Phase 3 — Candidate A refinement |
| §4.3 Quick Capture / Search / Project Report | Phase 2 — Theme 5 Features |
| §4.4 Storage & Index | Phase 1 — Storage decisions; Phase 3 — Candidate B refinements (Single Writer Rule, Dirty Buffer) |
| §4.5 Onboarding | Phase 3 — Candidate C (Onboarding workflow-first inversion) |
| §4.6 Customization & Extensibility | Phase 2 — Theme 5 (Plugin pattern); Phase 1 — Plugin pattern interno |
| §6 MVP Scope | Phase 4 — Resource Constraints; Action Plans 1-4 |
| §10 Open Questions | Phase 3 — Candidate B (3 hidden assumptions); brainstorming §Open Questions |
| §11 Success Metrics | Action Plans 1-4 success indicators |

The brainstorming retains 35 structured ideas; the PRD body lifts the ones that materially shape v1.0 scope. Ideas not lifted (e.g., specific git-integration UI sketches, AI/LLM hook locations) remain in the brainstorming for future reference.

---

## A.2 Parser choice — license analysis (resolves OQ-1)

> **Resolution 2026-05-19 (architecture LD-1, LD-3, LD-48).** **Option B selected**: `nvim-orgmode/tree-sitter-org` (MIT, active fork, last push 2026-05-05) + custom Rust semantic layer in `@orgsidian/core/src/parser/semantic/`. Option A rejected (GPL-3.0 contagion incompatible with the Plugin-API strategy in FR-24 / v1.5+). Option C deferred as fallback if upstream coverage proves inadequate. The coverage measurement work originally scoped as Spike 1 is reframed as a side-task of OD-1 in the architecture document; the 8-week Spike 1-2 time-box no longer applies. **Vendoring & maintenance contingency (LD-48):** the grammar lives at `crates/orgsidian-parser/grammar/` as a SHA-pinned git submodule (no auto-bump; SHA review on every upgrade). A named parser-owner role maintains familiarity with grammar source. v0.3 reserves 2 weeks for a fork-and-maintain dry run (checkout upstream, build from source, ship a trivial real-issue fix against the parser test corpus) to confirm the team can sustain the dependency if upstream stalls. Trigger for an in-house fork to `orgsidian-org/tree-sitter-org` (kept MIT): no upstream commits for >6 months at any v* milestone. Historical analysis below retained for context.

This is the single most consequential architectural decision in v1.0 because it cascades to product license, contribution model, and parser feature completeness. The research subagent's finding that uniorg is GPL-3.0-or-later changes the trade-off space from the brainstorming's framing.

### Option A: uniorg (Node.js / TypeScript)

- **License:** GPL-3.0-or-later. Strong copyleft.
- **Coverage:** High. uniorg is the most spec-faithful non-Emacs parser in the JavaScript ecosystem (used by Logseq among others — though Logseq's lossy issues are at the editor layer, not the parser layer).
- **Stack fit:** Native fit for Electron + TypeScript. In Tauri + Rust, would require an embedded V8/Node runtime or rewriting bindings — non-trivial.
- **License consequence for Orgsidian:** Linking to uniorg makes the combined work GPL-3.0. The author's commitment is OSS so this is acceptable, but it forecloses any future option for a commercial / dual-license / closed-fork variant. Forks and modifications must remain GPL-3.0. This is a public commitment, not a technicality.

### Option B: tree-sitter-org (Rust / portable C)

- **License:** MIT (depending on the specific fork; latillon and milisims forks are MIT; verify the chosen fork at adoption time).
- **Coverage:** Partial. Tree-sitter grammars excel at syntactic structure (headlines, blocks, lists) but miss semantic completeness — TODO state cycling, drawer types, deadline/scheduled semantics, link types, table formulas. The gap is real and would require a custom semantic layer on top of the tree-sitter parse tree, likely several hundred to a few thousand lines of code.
- **Stack fit:** Native fit for Tauri + Rust. Also usable from Node via the tree-sitter Node bindings.
- **License consequence for Orgsidian:** MIT permissive. Combined work can be MIT or Apache-2.0. Preserves future flexibility.

### Option C: write the parser ourselves

Rejected during brainstorming Phase 3 First Principles. The cost (months of work to reach the coverage uniorg already has) is not justified for a solo OSS developer. Mentioned here for completeness.

### Recommendation

The decision deserves Spike 1 (Months 1-2) before being locked. Two specific spike outputs would resolve it:

1. **uniorg spike:** wire uniorg into a minimal Electron + CodeMirror 6 prototype, parse a corpus of 100 real-world org files, log every coverage gap or parse error. If gaps are <5%, Option A is viable.
2. **tree-sitter-org spike:** wire tree-sitter-org into a minimal Tauri + CodeMirror 6 prototype, parse the same corpus, log gaps. Estimate effort to fill the semantic gaps. If <80h additional effort, Option B is viable.

Stack and parser are decided together: Option A pairs naturally with Electron+TS, Option B pairs naturally with Tauri+Rust. Trying to mix (Tauri + uniorg via embedded Node) inherits the worst of both.

### Author preference signal

The brainstorming surfaced a preference for Rust/Tauri (performance, OSS spirit) and a tolerance for OSS-license-locked code. Both options are live; Spike 1 + 2 outputs decide.

---

## A.3 Stack rationale (resolves OQ-2)

> **Resolution 2026-05-19 (architecture LD-1..LD-10).** **Tauri 2.x + Rust confirmed** as the stack, with CodeMirror 6 in webview and the monorepo "Core + Shell" pattern enforced from day one of v0.1 Alpha. Electron + TypeScript rejected as a consequence of OQ-1's Option B path (Tauri + Rust is the natural fit for `tree-sitter-org` + custom Rust semantic layer). The original Spike 2 stack-comparison is reframed as ongoing CI matrix work in OD-2 of the architecture document — cross-webview CodeMirror 6 consistency (WebKit/WebView2/WebKitGTK) is validated continuously, not by a one-shot spike. Tauri ecosystem pinning policy is governed by architecture LD-47 (exact-pin, quarterly review, v0.4 budget). Parser-side dependency sustainability — the natural co-risk of the parser+stack co-decision — is governed by architecture LD-48 (vendored submodule, parser-owner role, v0.3 fork-and-maintain dry run, in-house fork trigger if upstream stalls >6 months); see §A.2 above. Historical comparison below retained for context.

### Why not Qt + C++

Mature, fast, native widgets. Rejected: hostile development experience for a solo developer in 2026; UI building is slow; CodeMirror 6 (the chosen editor surface) doesn't run natively in Qt without a web view, and at that point a webview-shell framework is the better abstraction.

### Why not Flutter desktop

Better than Qt for solo developer UI velocity, but the org-mode community is JavaScript-and-Lisp-heavy and a Dart-based contributor base would be vanishingly thin. Foreclosure of contribution is the dealbreaker.

### Electron + TypeScript

- **Pros:** mature ecosystem; CodeMirror 6 is native; uniorg fits natively; the contributor pool overlaps Obsidian/Logseq users. Lowest activation energy for a v0.1 Alpha.
- **Cons:** memory and disk footprint (200MB+ resident, ~80-150MB disk per platform); start-up time slower than native; org-purist community is allergic to Electron on principle.

### Tauri 2.0 + Rust (brainstorming preference)

- **Pros:** small binary (<30MB), small memory footprint (~60-100MB resident), faster cold start. Webview-based UI so CodeMirror 6 still works. Rust ecosystem is healthy and growing. Aesthetic match with the org-mode "minimal and fast" sensibility.
- **Cons:** smaller contributor pool than Node/TS; Tauri 2.0 desktop maturity is good but Linux distribution still has quirks (AppImage works; flatpak/snap less polished); webview cross-platform consistency is a real ongoing pain point (WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux all behave subtly differently). uniorg integration adds friction.

### Monorepo "Core + Shell" — packaging pattern

Independent of the parser/stack winner, the codebase is organized as a monorepo with two top-level packages (this was the user's most-praised creative contribution from brainstorming Phase 1, Architettura #1):

- **`@orgsidian/core`** — pure logic: parser wrapper, SQLite index, query layer, file watcher, plugin registry, business rules. No UI concerns. Deployable as a library in its own right.
- **`@orgsidian/shell`** — desktop application: editor surface (CodeMirror 6), agenda views, settings UI, packaging, OS integration. Consumes `@orgsidian/core` as a dependency.

The two packages run in-process (no IPC overhead) but have a strict API boundary between them. This resolves the apparent dichotomy "monolith vs. engine+shell" without paying the IPC tax of true process separation. The boundary is enforced by package structure and type/module visibility, not by network/IPC mechanisms.

**Strategic payoff:**

1. **Plugin Pattern (FR-24) plugs into `@orgsidian/core`**, not `@orgsidian/shell`. Public Plugin API in v1.5+ exposes a stable core-package surface; UI-level extension points are a separate later concern. This pre-defines the refactoring path so v1.5+ does not require a rewrite.
2. **Alternative surfaces become viable v2+ paths** without rewriting logic: a web playground for the parser, a future TUI mode, a headless CLI for org analytics, an LSP server. All consume `@orgsidian/core`.
3. **Contributor onboarding splits cleanly.** A contributor interested in parser fidelity touches `@orgsidian/core` only and never learns the UI stack. A contributor focused on UX touches `@orgsidian/shell`.
4. **Refactoring boundaries are pre-defined.** Shell-side changes never modify core internals; core-package changes are versioned with semver discipline starting at v0.1 Alpha.

This pattern must be in place from day one of v0.1 Alpha. Retrofitting it later is expensive.

### Decision criteria for Spike 1-2

Pick the stack that scores highest on a weighted sum of:

1. **Parser path simplicity** (the OQ-1 winner forces this).
2. **CodeMirror 6 cross-platform consistency under load** (test on a 5,000-line org file in all three webviews).
3. **Cold-start time and memory under realistic Vault sizes** (1,000 files, sustained editing for an hour).
4. **Contributor friction** (the "how many minutes from `git clone` to running app" test on a fresh laptop).

Brainstorming sentiment leans Tauri. Spike must validate; the cost of being wrong here is enormous (the whole v1.0 effort).

---

## A.4 Pseudo-WYSIWYG via CodeMirror 6 — the 5x cost savings

This is the single highest-leverage architecture decision in the v1.0 plan and deserves a full record.

Brainstorming Phase 4 surfaced: true WYSIWYG (ProseMirror + custom Org schema) would cost 240-320 hours — roughly 1/5 of the entire v1.0 budget. That budget would buy a feature that visually matches Obsidian's preview mode but cost-displaces Quick Capture, Project Report, or onboarding work.

The pseudo-WYSIWYG approach uses CodeMirror 6's decorator and widget APIs to render syntax with visual richness while keeping the underlying buffer as plain `.org` text:

- Headings styled with hierarchical font sizes and color
- TODO states rendered as colored pill badges (clickable to cycle)
- Tags rendered as pill labels
- Timestamps rendered as human-readable dates with hover-for-source
- Checkboxes rendered as toggle widgets
- Links rendered as clickable underlined text
- Drawer regions visually collapsed by default with an expand affordance
- Inline markup (*bold*, /italic/, =verbatim=, ~code~) rendered with visual styling but markers visible (or visible on focus only — to be decided in v0.5 design pass)

The effect is "the source is the doc and the doc is readable" — the same trick that makes typewriter-mode markdown editors feel good without the format-translation cost. Estimated cost: 60-80h vs. 240-320h for true ProseMirror WYSIWYG. The deferred true WYSIWYG path lives in v1.5+ once the rest of v1.0 is shipped and the budget allows.

The risk: some users will want true WYSIWYG and judge Orgsidian as "still showing me markdown-like syntax." The mitigation is the workflow-first positioning — users come for the planner+PKM integration, not for visual hide-the-markup; users for whom hidden markup is the dealbreaker were going to bounce to Obsidian regardless. Validate via v0.1 Alpha and v0.5 Beta feedback. If the pull is strong, ProseMirror WYSIWYG moves up the v1.5 priority list.

---

## A.5 Single Writer Rule + Dirty Buffer — architectural detail

A first-principles surface in brainstorming Phase 3 identified the file watcher as a fragile-but-load-bearing component. The Single Writer Rule (FR-16) is the integrity contract; the implementation has several subtle pieces worth recording for `bmad-create-architecture`.

### Invariants

1. A file Orgsidian has open with a **Dirty Buffer** is owned by Orgsidian. External writes during this window trigger the Merge Dialog; they never silently overwrite the buffer.
2. A file Orgsidian has open with a **clean buffer** is not owned. External writes trigger an automatic reload.
3. A file Orgsidian does not have open is irrelevant — external writes only trigger an index re-scan for that file path.

### Edge cases to handle in implementation

- **File rename or move on disk.** macOS APFS clones, Linux mv across filesystems (which decomposes to copy+unlink), Windows file lock holds. Each platform's watcher emits these differently. Tactical: treat rename events as delete-then-create-at-new-path and re-key the Dirty Buffer's file binding accordingly.
- **File deleted on disk while open with a Dirty Buffer.** Surface as a banner: "File deleted externally. Save will recreate it." User choice: save (recreate), discard buffer, save-as-different-file.
- **Symlinks within the Vault.** Either follow (recommended default) or treat as opaque; document and provide a setting.
- **Case-folding filesystems (macOS default, Windows default).** Treat path comparisons case-insensitively for these platforms; Linux case-sensitive.
- **Network mounts.** fsevents/inotify do not fire reliably on network-mounted folders. Document and provide a polling-based fallback (configurable interval).
- **Atomic-write artifacts.** Tools like Vim and VS Code write a temp file and rename, generating a delete+create sequence rather than a modify event. The watcher must coalesce these into a single "file was rewritten" event with a short (e.g., 250ms) debounce window.

These are not v0.1 Alpha blockers individually but the v0.1 design must reserve clear extension points so handling can be added incrementally without re-architecting.

---

## A.6 Realistic audience sizing

Brainstorming Phase 2 set target audience as "knowledge worker generic, freelancer as lighthouse persona." The Discovery research surfaced that the org-mode community is small, Emacs-loyalist, and skeptical of non-Emacs org tools (Karl Voit's January 2024 post is the canonical critique-frame).

A more honest sizing for v1.0 success-planning:

- **Primary addressable:** "Org-curious, non-Emacs users." Estimated 5,000-20,000 people globally based on Reddit r/orgmode active membership (~30k), HN org-mode thread engagement, and the visible diaspora from Logseq's DB-version-without-org transition. This is a small audience; SM-3's 1,000 downloads in 30 days against this base is a 5-20% capture rate, which is ambitious but credible if positioning lands.
- **Secondary addressable:** "Freelancers/consultants who want plain-text planning and have heard of org-mode." Larger pool but less specific demand signal; conversion is on the strength of the planner+PKM proposition alone, not org-mode evangelism.
- **Out of reach (correctly):** "Emacs org-mode power users." This audience does not need Orgsidian; they have org-mode in its canonical environment. Optimizing for them would distort the product (FR-5 Emacs keybindings option is a courtesy, not a strategy).
- **Out of reach (intentionally):** "Markdown-native PKM users." Obsidian and Logseq serve them; Orgsidian's `.org` format requirement is a moat with two sides.

This sizing should inform v1.0 launch comms: target the diaspora, not the loyalists. Lead with "desktop-native, cross-platform, OSS, faithful org-mode without Emacs" — fidelity + access. The planner positioning is a secondary hook for the broader audience.

---

## A.7 Roadmap source-of-truth

The brainstorming Phase 4 Action Plans 1-4 contain the canonical hour budgets and feature lists per milestone. The PRD §6 reflects them and the addendum here cross-references for traceability:

- **v0.1 Alpha** — brainstorming Action Plan 2, 160h budget, Months 3-6.
- **v0.5 Beta** — brainstorming Action Plan 3, 240h budget, Months 7-12.
- **v1.0** — brainstorming Action Plan 4, 240h budget, Months 13-18.
- **Pre-MVP spikes** — brainstorming Action Plan 1, 60-80h budget, Months 1-2.

Total v0.1 → v1.0: ~720h over 18 months at 10h/week ≈ 720h available. Realistic but tight. Any slippage in the spike (Months 1-2) shifts everything; any feature added beyond the brainstorming plan must displace something else.

**Item-vs-budget overrun risk (real and chronic).** The brainstorming Action Plans 2-4 itemized features at the sub-FR level, and the itemized hour totals sum higher than the calendar budgets — for example, Action Plan 2 (v0.1 Alpha) lists ~290-420h of itemized work inside a "~160h" calendar budget. This is not a brainstorming error; it is the typical solo-OSS planning pattern where item estimates are generous and the calendar budget represents the *cap* that forces selection among items. The PRD §6 treats the budgets as ceilings explicitly. Discipline: at each milestone start, re-rank items by SM contribution and cut the bottom of the list to fit; do not silently absorb the overrun by quality compression.

**10h/week constraint** is structural — based on Tiziano's available time given other commitments. The roadmap math depends on this number being sustained. If it drops sustainably below 8h/week, the 18-month roadmap stretches accordingly; if it rises sustainably above 12h/week, scope can be re-expanded selectively. This is the leading indicator to watch for re-planning triggers.

The discipline anchor: when the urge arises to add a feature mid-roadmap, re-read brainstorming Phase 4 ("La matematica brutale ha rivelato che Option C completo costa 1250-1850h"). The roadmap is the protection against the option-C death spiral.

---

## A.8 Rejected alternatives — surface notes

For `bmad-create-architecture` and future contributors who will ask "why didn't you...":

- **Emacs backend (org-mode batch mode + RPC).** Rejected: makes Emacs a hard runtime dependency, defeats the entire positioning. Brainstorming Phase 1.
- **CRDT-based real-time sync (Yjs, Automerge).** Rejected: huge architectural complexity, requires a sync server eventually, distracts from v1.0 single-user scope. Brainstorming Phase 1.
- **Built-in Orgsidian sync server.** Rejected: shifts the project from "tool" to "service," opens ops and security surface, contradicts the "no cloud account, ever" commitment. Brainstorming Phase 1; revisit as optional self-hostable in v2+.
- **Mobile app in v1.** Rejected: solo dev cannot ship desktop + mobile in 18 months. beorg/Orgzly are the recommended pairing. Brainstorming Phase 4.
- **AI/LLM features in v1.** Rejected: architecturally premature, distracts from fidelity bar, and the org community is divided on LLM integration. Hooks preserved; user-facing features in v1.5+.
- **Compete with Obsidian on notes-only.** Rejected: positioning is integration, not notes. Notes without the planner is Obsidian's space.
- **Compete with NotePlan on planner-only.** Rejected: similarly, planner without notes is NotePlan's space and they own Apple platforms there.

---

*End of addendum. Living document — extend during v1.0 architecture pass.*
