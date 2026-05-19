# PRD Quality Review — Orgsidian

## Overall verdict

This PRD holds up well as a solo-dev scope anchor and as a future-self motivational document; it has a real thesis (unification as differentiator), honest non-goals, and FRs with mostly testable consequences. Where it gets weaker is decision-readiness at the architectural fork (parser/stack is gated on Spike 1-2 but the PRD does not say what happens if the spike is inconclusive or both options miss) and Success Metrics that — for the solo author / future-self audience — under-instrument the only thing that actually matters at month 9: whether *the author is still using it and shipping*. The contributor-facing audience is the most underserved of the three: there is no "how to help / where to start / what's already decided vs. open" map.

## Decision-readiness — adequate

The PRD makes real choices (filesystem-native + SQLite-derived index, pseudo-WYSIWYG over true WYSIWYG, internal Plugin Pattern over public API, Plain Mode default, single-Vault) and names what was given up in each. The Non-Goals section is unusually honest — "not a notes-only tool" and "not a planner-only tool" are real positioning fences, not safety. The addendum is doing significant heavy lifting on the parser/stack trade-off.

Where it weakens: the biggest decision in the project (parser + stack) is openly deferred to Spike 1-2 with **no decision rule for what happens if both spikes come in mixed**. OQ-1's "Resolution: addendum §A.2 + Spike 1" reads as resolved but isn't — A.2 explicitly says "Both options are live; Spike 1 + 2 outputs decide." That's fine, but the PRD doesn't say what *triggers* the call (who decides, against what threshold, by when, with what fallback if the spike runs over). For a solo-dev PRD this matters more, not less — there is no PM in the loop to force the call.

Several `[NOTE FOR PM]` callouts sit at safe checkpoints rather than at real tensions. "Workflow Recipes... emotionally load-bearing; revisit if v1.0 timeline permits" is the only one doing real work; the a11y deferral note and the WYSIWYG note read as bookmarks, not decision-forcing flags.

### Findings

- **high** No decision rule on the parser/stack fork (§10 OQ-1, OQ-2; addendum §A.2-A.3) — Spike 1-2 outputs are named as the resolution path, but there's no threshold ("if uniorg gap >5%, drop A"; "if Tauri cold-start >3s on 1k-file vault, drop Tauri"), no time-box ("if spike exceeds 80h, freeze to the safer option"), and no tie-breaker. For a solo dev, an undecidable spike *is* the failure mode. *Fix:* add 3-4 lines to OQ-1 stating the explicit pick rule and a hard deadline ("decision locked by end of Month 2; if both spikes are inconclusive, default to Electron + uniorg as the lower-execution-risk path and accept GPL-3.0").
- **medium** "Project Report" carries far more weight than the FR allocates (§4.3 FR-14) — UJ-3 frames it as the wow demo, §6.2 calls it the wow demo for Beta launch, but the FR is a single bullet list with no spec for "milestone status (Headlines tagged as milestones if convention used)" — what convention? The Glossary doesn't define "milestone." A demo-grade feature needs more decision-readiness than this. *Fix:* either define the milestone convention in Glossary + FR-14 (e.g., `:milestone:` tag, or `MILESTONE` TODO state) or move template/milestone definition to an explicit OQ.
- **medium** "Inline Coaching" (FR-21) is doing strategic work as the onboarding bridge but has no content authored or scoped (§4.5) — for solo-dev planning this is the kind of feature that looks small until you have to write 30 coaching strings in voice. *Fix:* add a consequence "v0.5 ships with N coaching strings for the empty-state surfaces listed in FR-21; copy drafted alongside Starter Vault content" or flag as OQ.
- **low** Several `[NOTE FOR PM]` callouts are bookmarks rather than decisions (§4.1 FR-5 Notes; §8 a11y) — the ProseMirror deferral note doesn't say what would trigger reconsidering; the a11y deferral doesn't say what minimum survives in v1.0. *Fix:* either delete (the Non-Goals + Out of Scope already carry the load) or upgrade to "revisit if X" with a concrete X.

## Substance over theater — strong

This dimension is the PRD's strongest. The Vision statement is not interchangeable with any other PRD in the category — "the integrated planner-and-knowledge desktop app for people who want org-mode without Emacs" is sharp, and the "tasks, time, and notes as peers" framing earns its place by directly informing the Today Dashboard / Agenda-as-front-door decision. Personas are restrained (one primary + JTBD + explicit non-users) — no theater. Differentiation is grounded in real competitor analysis (Organice, Logseq DB rewrite, beorg, Obsidian, NotePlan), not invented novelty. The "Why Now" section is three concrete signals, not boilerplate.

NFRs have product-specific thresholds (300ms first-screen render on 5k-line file, 30ms typing latency, <500MB on 1k-file vault, 200ms search) — not "must be performant."

### Findings

- **low** Mara appears in two UJs (UJ-1, UJ-5) and Tiziano, Sofia, Alex, Riccardo appear in one each — these read as named-walkthroughs rather than personas, which is fine and probably better for a hobby PRD, but the §2.1 "Independent Knowledge Worker" persona never explicitly maps to them. *Fix:* either add one line to §2.4 noting "the journeys below illustrate the primary persona at different points in their use" or drop the persona framing entirely and just call §2 "Target User & Journeys" — the named characters do more work than the abstract persona does.

## Strategic coherence — strong

The PRD has a thesis: *the unification of tasks/time/notes inside a desktop-native, cross-platform, OSS, org-faithful tool is the unclaimed differentiator.* That thesis visibly drives feature prioritization — Agenda is the front door (§4.2), Project Report is the Beta wow demo because it's the most visible proof of the integration, Starter Vaults are workflow-first because the brainstorming surfaced workflow as the real org-mode barrier. The milestone ordering (Alpha = fidelity + agenda; Beta = the integration; v1.0 = polish + Windows + community-readiness) reflects the thesis, not feature-set convenience.

Counter-metrics are real and pointed (SM-C1 feature count, SM-C2 Plugin API timeline, SM-C3 Emacs-user conversion) — these are the kind of counter-metrics that protect a solo-dev project from drift, which is exactly the right calibration.

### Findings

- **medium** SM-2 measures author-as-daily-driver but the thesis is "tasks, time, and notes as peers" — the metric doesn't validate the thesis, only that the tool is usable (§11) — an author can daily-drive a notes-only or planner-only tool. *Fix:* add a sub-criterion to SM-2: "during the 4-week author-daily-driver window, the author logs clocked time on tasks linked to notes in at least 3 distinct projects" — i.e., observe the *integration* actually being used, not just the app being opened.

## Done-ness clarity — adequate

Most FRs have testable consequences with bounds, not adjectives — "under 300ms," "byte-identical," "within 5 seconds," "under 200ms for first 50 results." This is well above the median for solo-dev PRDs. The Pseudo-WYSIWYG FR-4 explicitly names the hard part ("cursor placement, copy-paste, and find/replace operate on source positions, not rendered positions") which is the kind of detail that prevents future-self from re-deriving the requirement.

Where it slips: a handful of feature consequences hand-wave on the load-bearing part.

### Findings

- **high** FR-14 Project Report "linked notes summarized (titles and snippets)" — "summarized" is undefined and the Project Report is positioned as the Beta wow demo (§4.3) — at solo-dev stakes, "summarized" could mean anything from "first 200 chars of body" to "LLM-generated," and Non-Goals rules out LLMs. *Fix:* "linked notes listed by title with the first N characters of body as snippet; no AI summarization in v1.0."
- **high** FR-16 Merge Dialog "manually merge in editor" (§4.4) is the most product-defining behavior in the storage feature, and "manually merge" is one phrase — does this open a 3-way merge view? A side-by-side with copy-across affordance? Drop them into a temp file and let the user resolve? *Fix:* spec the merge surface at least at the level "side-by-side text panes with line-level copy-to-left/right affordance, output is the user's saved result; no auto-merge attempted in v1.0."
- **medium** FR-1 "rendered correctly per the org-mode syntax conventions supported by Orgsidian (subset documented; see §10 OQ-5)" — the consequence rides on a subset that OQ-5 defers to v0.1 README — that's a real circular dependency where "done" for FR-1 cannot be assessed until OQ-5 lands. *Fix:* either commit a minimum subset inline in FR-1 (headlines, TODO states with cycling, scheduled, deadline, drawers, inline `*bold*`/`/italic/`/`=verbatim=`/`~code~`, `[[link]]` and `id:` links, checkboxes) or upgrade OQ-5's resolution from "shipped with v0.1 README" to "drafted in Spike 1 outputs."
- **medium** FR-21 Inline Coaching consequences only cover dismissibility and a reset action — not what the coaching actually *does* — *Fix:* add a consequence enumerating the surfaces ("coaching strings exist for at minimum: empty Today Dashboard, empty Inbox, first-time agenda view, never-clocked-in headline view, search with zero results, Plain Mode → Power Mode toggle hint").
- **medium** FR-15 atomic write on Windows is `[ASSUMPTION]` in §4.4 NFR but OQ-4 names "fall back to documented write-through with structured warning" as the contingency — these contradict: the NFR claims atomic writes cross-platform, OQ-4 says Windows might not. *Fix:* relax the NFR to "atomic write on macOS/Linux; Windows uses temp-file-and-replace where available, with documented fallback per OQ-4."

## Scope honesty — strong

Non-Goals is the strongest section of the PRD. It rules out things by *name* and *competitor* ("Not a notes-only tool. Notes without the planner integration is Obsidian's space; we do not compete there") rather than by category. Out-of-scope-for-MVP lists explicit hour deltas where known. The Assumptions Index is comprehensive — every inline `[ASSUMPTION]` is indexed. Open Questions are real questions with named resolution paths, not rhetorical.

Open-items density: 8 OQs + 12 indexed assumptions + 4 `[NOTE FOR PM]` callouts against a 1.5-year solo-dev hobby roadmap is appropriate — high enough to signal honesty, not so high it indicates the PRD is premature.

### Findings

- **low** Non-Goals says "No iOS or Android app in v1" but Out-of-scope-for-MVP says "Mobile app (iOS/Android) — v2+" — small contradiction in the deferral horizon (v1.5+ vs. v2+) — *Fix:* align on v2+ in both places (more honest given the solo-dev budget reality acknowledged in addendum §A.7).
- **low** Non-Goals "Not a true WYSIWYG editor in v1" — "v1" here is ambiguous between "v1.0" and "the v1 milestone series including v1.5" — given §6.4 puts WYSIWYG at "v1.5+" the Non-Goal should say "in v1.0; deferred to v1.5+." *Fix:* s/v1/v1.0/ throughout §5 for consistency.

## Downstream usability — adequate

The PRD is explicitly the upstream of `bmad-create-architecture` and `bmad-create-epics-and-stories` (stated in §0), so this dimension matters. Glossary is present, terms are used consistently across FRs and UJs (Vault, Headline, Agenda, Today Dashboard, Quick Capture, Backlink all hold). FR / UJ / SM IDs are contiguous and unique. Cross-references are via Glossary terms.

What's missing for the *contributor* audience (audience 2 of 3 named in §0): there's no map of "where could someone help / what's already decided / what's still live." A contributor reading this PRD knows the vision but not how to help — the PRD as-written is read-only documentation, not invitation-shaped.

### Findings

- **medium** No contributor-facing entry point for the second audience named in §0 — the PRD describes the product but doesn't tell a would-be contributor where to plug in, which is the only thing the "potential contributors" audience is reading the document to learn. *Fix:* add a §13 "How to help" or expand §0 with two lines pointing to the issue tracker labels, the spikes (which are concrete pre-MVP work a contributor could pick up), and the Plugin Pattern internal interface (FR-24) as a contribution surface even pre-API. Alternative: own that the PRD is author-anchor + future-self only, and move "contributor-facing" out of §0's stated audiences (more honest if you'd rather defer the contribution scaffolding).
- **low** UJ names (Mara, Tiziano, Sofia, Alex, Riccardo) don't all link back to §2.1's "Independent Knowledge Worker" persona by exact label — minor for downstream tooling but the rubric's "UJs each name a persona from §2 by exact label" is technically unmet. *Fix:* either tag each UJ with "(Independent Knowledge Worker)" once, or accept that the named-character pattern is intentional and skip the formal linkage (this is a hobby PRD; box-checking would be theater).

## Shape fit — strong

This is a hobby / solo PRD with launch stakes — rigor light, substance bar still applies. The PRD nails this calibration: it has the structure a chain-top PRD needs (it does feed UX → architecture → stories) without over-formalizing. Personas are right-sized (one primary, JTBD, explicit non-users) for a consumer product. UJs are load-bearing (the planner+PKM proposition is fundamentally a UX claim; the journeys carry the proof). NFRs have product-specific thresholds. The addendum exists precisely because the body needs to stay readable for the three audiences while architectural depth still has somewhere to live.

The triple-audience framing in §0 (solo-dev anchor / contributor-facing / future-self motivational) is the right frame and *almost* delivered, but the contributor-facing layer is thin (see Downstream usability finding above) and the future-self motivational layer is implicit — it's in the Vision and Why Now but never explicitly addressed.

### Findings

- **medium** "Future-self motivational" is named as audience #3 in §0 but the PRD contains no element that addresses it specifically — the Vision is sharp but it's pitched at the *reader*, not at month-9-Tiziano who has lost momentum — *Fix:* add a short §1.1 "Why this is worth finishing" or a one-paragraph future-self letter at the top of §1, anchored on the specific moments that should re-motivate ("when you stall at month 9: the diaspora hasn't moved; Logseq hasn't come back; the desktop-native gap is still open; ship v0.5 Beta even if v1.0 slips"). This is the kind of move that fits a hobby PRD and would actually serve audience #3, vs. naming the audience and then not writing for them. Alternatively, drop audience #3 from §0 if the Vision and Why Now are meant to carry that load implicitly.

## Mechanical notes

- **Glossary drift:** none significant. "Active Clock," "Dirty Buffer," "Single Writer Rule" all hold across §4.1, §4.2, §4.4. "Milestone" used in FR-14 without Glossary entry — see Done-ness FR-14 finding.
- **ID continuity:** FR-1 through FR-24 contiguous and unique. UJ-1 through UJ-6 contiguous. SM-1, SM-2, SM-3 primary + SM-4, SM-5 secondary + SM-C1, SM-C2, SM-C3 counter — clear scheme, no gaps. OQ-1 through OQ-8 contiguous.
- **Assumptions Index roundtrip:** spot-checked all 12 entries against inline `[ASSUMPTION]` tags — all present inline; index complete. Good.
- **Cross-references:** "§4.4 invariant" referenced from §4.4 description (self-reference, fine). "§5 Non-Goals" referenced consistently. "Addendum §A.X" references match A.1 through A.8.
- **UJ persona linkage:** see Downstream usability finding (low).
- **Required sections:** Vision / Target User / Glossary / Features / Non-Goals / MVP Scope / Constraints / NFRs / Why Now / Open Questions / Success Metrics / Assumptions Index — all present and earned.
