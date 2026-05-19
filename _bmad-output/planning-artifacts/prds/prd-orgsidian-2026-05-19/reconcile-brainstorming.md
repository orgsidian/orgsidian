# Reconciliation: Brainstorming 2026-05-18 vs PRD + Addendum 2026-05-19

**Input:** `_bmad-output/brainstorming/brainstorming-session-2026-05-18-1613.md`
**Artifacts reviewed:** `prd.md`, `addendum.md`
**Mode:** Fast-path PRD — known risk of silent compression of qualitative/principled material.

This document itemises material in the brainstorming that is either **missing**, **distorted**, or **silently re-scoped** in the PRD+addendum pair. Each item names the brainstorming source, the gap, and a concrete proposed PRD location to remedy it.

The PRD is overall strong on FRs, scope, and roadmap math. Most gaps are qualitative — principles, decision identity, persona reasoning, and the relationship between brainstorming Action Plans and PRD milestones — rather than missing features.

---

## A. Qualitative principles and product tone (highest priority)

### A1. "Smart Defaults, User in Control" is not surfaced as a documented design principle

- **Brainstorming reference:** Phase 1 Key Breakthroughs ("Pattern 'Smart Defaults, User in Control' emerso trasversalmente come principio cardine del prodotto"); Theme 4 Onboarding [Principio #1]; Session Reflections ("merita essere principio di design documentato (riapparirà in decine di decisioni UX micro)").
- **Status in PRD:** The pattern manifests implicitly (Plain/Power Mode, Today Dashboard configurable to last-open-file, Settings toggles everywhere) but is **never named or stated as a cross-cutting principle**. The PRD has no "Design Principles" section.
- **Why it matters:** The brainstorming explicitly flags this as a principle that "will reappear in dozens of micro UX decisions." Without it being named in the PRD, downstream artifacts (architecture, stories) will re-litigate every "should this be a default or a toggle?" decision instead of inheriting a stance.
- **Proposed PRD location:** Add a new section §3.5 or §7.5 **"Design Principles"** (between Glossary and Features, or inside Constraints & Guardrails) capturing: Smart Defaults / User in Control; Workflow-first over syntax-first; Non-lossy round-trip as trust contract; Filesystem-native as data sovereignty; Pseudo-WYSIWYG as deliberate trade-off.

### A2. "Workflow over syntax" is stated as a feature description, not elevated as a principle

- **Brainstorming reference:** Phase 3 Candidate C — "il vero blocco non è la sintassi ma il workflow"; Session Reflections ("Onboarding workflow-first è probabilmente la decisione di prodotto più sottovalutata — vale la pena renderla esplicita nel PRD"); Breakthrough Moments #5.
- **Status in PRD:** Captured in §4.5 Onboarding description but as feature-specific framing. The brainstorming explicitly recommends making it explicit at PRD level.
- **Proposed PRD location:** Same Design Principles section as A1, OR an explicit subsection in §1 Vision restating the principle so the Tutorial, Starter Vault, and Inline Coaching FRs all derive visibly from it.

### A3. The "task-first marketing, integrated peer-level product" duality is collapsed

- **Brainstorming reference:** Theme 2 [Posizionamento #1] and [Raffinamento posizionamento]: "task come wedge di marketing per entrare in un mercato non saturo, ma il prodotto è integrazione peer-level di task/time/knowledge." Phase 3 Candidate A: "'task-first' è un wedge di marketing, non architettura di prodotto."
- **Status in PRD:** §1 Vision lands on "the unification itself is the differentiator" — which preserves the *product* half but loses the explicit **marketing-wedge vs product-integration duality**. The brainstorming's insight is that these are intentionally different surfaces.
- **Why it matters:** A.6 in the addendum touches it ("Lead with 'desktop-native, cross-platform, OSS, faithful org-mode without Emacs' — fidelity + access. The planner positioning is a secondary hook for the broader audience"). But the brainstorming's framing is the opposite: planner/task **is** the marketing wedge, integration **is** the product truth. The addendum has silently inverted the rhetorical strategy.
- **Proposed PRD location:** §1 Vision — add a 2-sentence paragraph naming the marketing/product split explicitly. Reconcile with addendum §A.6 (which currently contradicts the brainstorming's wedge strategy).

---

## B. Decisions that lost identity in feature grouping

### B1. "Decision #1: Zero Emacs dependency" is implicit but never named as a numbered decision

- **Brainstorming reference:** Theme 1 [Decisione #1]: "Zero dipendenza da Emacs — parser org-mode nativo, no batch mode Emacs."
- **Status in PRD:** Captured in §5 Non-Goals ("Not an Emacs replacement"). Stated as non-goal, not as an architectural decision with positive intent ("parse natively").
- **Proposed PRD location:** §10 Open Questions OQ-1 already touches it via parser choice; add an explicit "Architectural Decision: no Emacs runtime dependency" callout in §4.1 Editor & Org-mode Fidelity description, or in addendum §A.2 as **AD-1** alongside the parser analysis.

### B2. "Decision #2: NO CRDT/multi-device sync built-in" — captured as non-goal, but loses the scope-protection framing

- **Brainstorming reference:** Theme 1 [Decisione #2]; Phase 1 Key Breakthroughs ("Cancellazione esplicita di scope creep pericolosi: no Emacs backend, no CRDT real-time, no sync server in v1").
- **Status in PRD:** Listed in §5 Non-Goals. Adequate, but the brainstorming framed it as "scope creep cancellato" — a deliberate protective act. PRD treats it as routine non-goal among many.
- **Proposed PRD location:** §5 Non-Goals — consider a brief preface noting that the first four non-goals (Emacs, real-time, mobile, sync service) were the **explicitly cancelled scope creeps** flagged in brainstorming Phase 1, not arbitrary exclusions. This preserves the scope-discipline rationale.

### B3. Decisione architettura #5 (Plugin Pattern internal from v1) — preserved but loses the v1.5+ public-exposure timing rationale

- **Brainstorming reference:** Theme 1 [Decisione architettura #5]: "codice interno usa già hooks/registry, esposizione pubblica in v1.5+."
- **Status in PRD:** FR-24 captures the architectural commitment cleanly. The **rationale for why public exposure is v1.5+** (stability, contributor trust) is in SM-C2 counter-metric, which is reasonable. Mild distortion only.
- **Proposed PRD location:** FR-24 Notes — add a one-liner linking to SM-C2 so the v1.5+ rationale is co-located.

### B4. The "monorepo Core + Shell" decision (Breakthrough #1) is entirely absent from the PRD body

- **Brainstorming reference:** Theme 1 [Architettura #1] "Monorepo 'Core + Shell' — engine package + desktop package, in-process per evitare IPC ma confine API pulito"; Phase 1 User Creative Strengths ("proponendo subito una terza via — monorepo con package separati"); Breakthrough Moments #1.
- **Status in PRD:** Not mentioned in the PRD body. The addendum §A.3 implicitly assumes a single-binary deployment per stack; the monorepo/two-package structure is **silently dropped**.
- **Why it matters:** This was flagged as the user's most creative contribution in the session — "the third way" resolving an apparent engine-vs-monolith dichotomy. It is also load-bearing for the Plugin Pattern (FR-24) and for future engine-reuse (CLI tooling, batch operations). Losing it foregrounds a v1.5+ rewrite risk.
- **Proposed PRD location:** Add to addendum §A.3 Stack Rationale (subsection on packaging structure), OR add a brief callout in §4.6 Customization & Extensibility describing Core/Shell as the architectural shape that makes the internal Plugin Pattern coherent. Strongly recommend addendum because this is architecture, not user-facing.

### B5. Storage architecture decisions #1 and #2 (filesystem-native, SQLite index) are present but lose the "org-roam pattern" precedent

- **Brainstorming reference:** Theme 1 [Storage #1] and [Storage #2]: "SQLite come indice — alla org-roam, cache rigenerabile dai file"; Breakthrough Moments #3.
- **Status in PRD:** §4.4 captures both decisions correctly. The **"this is the org-roam pattern, validated by an existing trusted tool"** trust-building rationale is missing.
- **Proposed PRD location:** §4.4 Description — add a sentence: "This is the pattern proven by org-roam in the Emacs ecosystem (filesystem-of-`.org`-files + derived SQLite index), chosen here for the same robustness reasons."

---

## C. Roadmap / Action Plan reconciliation

### C1. Spike content drift between Action Plan 1 and §6.1 / addendum §A.7

- **Brainstorming reference:** Action Plan 1 specifies **four concrete spikes**: (1) parser+render minimal prototype ~30h; (2) file watcher + Single Writer Rule cross-platform ~20h; (3) SQLite index benchmark on 1000-file vault ~10h; (4) plugin pattern interno design ~20h.
- **Status in PRD:** §6.1 mentions Months 1-2 as "Pre-MVP" without enumerating spikes. Addendum §A.2 and §A.3 reference "Spike 1" and "Spike 2" — but the brainstorming Spike 3 (SQLite benchmark) and Spike 4 (plugin pattern design) are silently absent from the addendum's spike framing. OQ-2/OQ-3/OQ-4 partially absorb them but lose the budgeted hours and the discrete spike identity.
- **Proposed PRD location:** Add to §6 a **§6.0 "Pre-MVP Spikes (Months 1-2)"** subsection listing the four named spikes with their hour budgets, mirroring Action Plan 1 verbatim. Or expand addendum §A.7 to enumerate all four spikes explicitly.

### C2. v0.1 Alpha hour budget is described as ~160h in PRD §6.1 — brainstorming Action Plan 2 itemises ~410-580h of work then says "~160h budget"

- **Brainstorming reference:** Action Plan 2 lists six work items totalling roughly 290-420h (parser wrapper 80-120h, editor Raw 60-80h, agenda 80-120h, theming 30-40h, packaging 40-60h, landing). Then says "Resources Needed: ~160h" — **the brainstorming itself has a budget inconsistency** (240h ≈ 6 months × 10h/week × 4 weeks).
- **Status in PRD:** §6.1 says "Months 3-6 (~160h budget)." This is a **direct inheritance of a brainstorming arithmetic ambiguity** — neither document resolves whether v0.1 fits in 160h, 240h, or 290-420h.
- **Proposed PRD location:** §6.1 — flag this as `[ASSUMPTION/OPEN]: v0.1 Alpha budget reconciliation. Brainstorming Action Plan 2 lists item-level estimates totalling 290-420h within a 160-240h calendar budget; either scope must shrink or schedule must stretch. Resolve during architecture pass.]` This is currently a silent risk.

### C3. v0.5 Beta budget — same pattern

- **Brainstorming reference:** Action Plan 3 itemises 8 items totalling 370-630h within a stated ~240h budget. Brainstorming acknowledges "possibile lieve overflow, accettabile" but the gap is 50-150% over budget, not "lieve."
- **Status in PRD:** §6.1 inherits "~240h budget" without flagging the overflow.
- **Proposed PRD location:** Same as C2 — add an explicit `[ASSUMPTION]` to §6.2 noting the brainstorming itemised-vs-budget overrun and the need to reconcile during architecture pass.

### C4. "Time tracking moved from v0.5 to v1.0" decision is captured but loses the rationale

- **Brainstorming reference:** Phase 4 User Decisions: "Time tracking spostato da v0.5 a v1.0 (richiede UX curata)"; Theme 6 [Roadmap swap finale].
- **Status in PRD:** §6.3 lists "Time Tracking UI polish" in v1.0 but FR-8 (Clock in/out/resume) is in **v0.5 Beta scope** (§6.2). This is a **silent re-scoping**: the brainstorming moved time tracking to v1.0; the PRD has put the functional clocking in v0.5 and only the UI polish in v1.0.
- **Why it matters:** The rationale in brainstorming was "richiede UX curata" — i.e., clocking is product-defining enough that it deserves the v1.0 polish budget, not a v0.5 rush. The PRD has split the difference unilaterally.
- **Proposed PRD location:** §6.2 — either move FR-8 to §6.3 v1.0 (consistent with brainstorming intent) or explicitly justify the split: e.g., `[ASSUMPTION: clocking functional core in v0.5 Beta to enable the integrated-planner proposition; UI polish (persistent status bar, time editing affordance) deferred to v1.0 per brainstorming Phase 4 rationale that clocking "richiede UX curata".]`

### C5. "Project Report anticipated from v1.0 to v0.5" — preserved, but losing the "wow demo for public launch" framing in the FR

- **Brainstorming reference:** Phase 4 User Decisions: "Project Report anticipato da v1.0 a v0.5 (wow demo per il lancio pubblico)."
- **Status in PRD:** §6.2 mentions "Project Report export (wow demo for Beta launch)" — preserved correctly. ✅ No gap; flagged here as a positive check.

### C6. Action Plan 1 Success Indicators are not carried into the PRD

- **Brainstorming reference:** Action Plan 1 Success Indicators: "Decisione stack pubblicata; prototipo end-to-end funzionante su un file `.org`; benchmark SQLite documentato."
- **Status in PRD:** §11 Success Metrics SM-1/SM-2/SM-3 cover v0.1, v0.5, v1.0. **There is no SM for the Pre-MVP spike phase** — Action Plan 1's success indicators are silently absent.
- **Proposed PRD location:** §11 — add **SM-0: Pre-MVP spike completion** with the three brainstorming indicators (stack decision published; end-to-end prototype works on one `.org` file; SQLite benchmark documented).

---

## D. Implied user journeys not surfaced as UJs

### D1. Daily/weekly review workflow is implied but not a UJ

- **Brainstorming reference:** Theme 3 [UX #3] "Today Dashboard iniettato — daily note auto-popolata con agenda + clocked tasks + inbox + log settimanale"; Onboarding #6 Workflow Recipes "(GTD, PARA, Zettelkasten, Weekly Review, OKR)."
- **Status in PRD:** Today Dashboard (FR-6) is present. The "log settimanale" / weekly review surface is not in any FR or UJ. UJ-1 covers "opening her day"; no UJ covers "reviewing her week."
- **Proposed PRD location:** §2.4 — add **UJ-7: Mara runs her weekly review** showing Saturday morning workflow: open Agenda in Week view, review last week's clocked time, triage Inbox accumulations, schedule next week. This grounds the Custom Agenda view (FR-7) and exercises Clock totals.

### D2. Configuring the Vault and Settings is implied but not a UJ

- **Brainstorming reference:** OQ-7 (in PRD) names "Settings UI vs. config file" as an open question, recognizing org-mode users expect a text config. Brainstorming Theme 3/4 implies progressive disclosure (Plain → Power).
- **Status in PRD:** FR-20 Plain/Power Mode and FR-23 keybinding remapping exist as FRs but **no UJ exercises configuration**. Users moving from Plain to Power Mode is a key adoption moment with no narrative.
- **Proposed PRD location:** §2.4 — add **UJ-8: Alex graduates from Plain to Power Mode** (months in, hits the toggle, discovers the advanced commands, optionally edits the config file).

### D3. Failed launch / corrupted-vault recovery is implied by FRs but not a UJ

- **Brainstorming reference:** Phase 3 Candidate B included Single Writer Rule + Dirty Buffer + Merge Dialog; addendum §A.5 lists six edge cases. These are reliability promises but with no narrative.
- **Status in PRD:** UJ-5 covers external write to a clean buffer. No UJ covers: orphan-clock-on-startup recovery (covered in FR-8 consequences but not narrated); index corruption / rebuild; vault folder moved or unmounted.
- **Proposed PRD location:** §2.4 — add **UJ-9 (optional): Mara recovers from a corrupt index** showing the rebuild-from-files behaviour (validates FR-17 trust contract). Lower priority than D1/D2.

---

## E. Stakeholder reasoning / motivation context the PRD treats as obvious

### E1. Author's motivation (dogfooding as design discipline) is touched but understated

- **Brainstorming reference:** Phase 2/3 implicit; PRD §0 mentions "The author (solo developer)" and "Future-self … re-grounds the *why*"; SM-2 mentions "Author uses Orgsidian as their daily driver."
- **Status in PRD:** Present but dispersed. The brainstorming framed dogfooding as the **primary validation mechanism** ("Validation key: tu stesso lo usi al posto di Emacs/Obsidian?"). The PRD lists it as one of three audiences in §0 and one criterion in SM-2.
- **Proposed PRD location:** §0 or §1 Vision — a sentence: "The author is the first user. Dogfooding is not a release-readiness check; it is the design discipline. If the author does not reach for Orgsidian first thing in the morning, no feature is finished."

### E2. The "small audience, small but real demand" honest sizing is in the addendum, not the PRD

- **Brainstorming reference:** Phase 2 Tema 2 [Target #1] "Audience ampia, freelancer come persona faro." Addendum §A.6 expands with sizing (5k-20k primary).
- **Status in PRD:** §2 Target User is confident; §9 Why Now mentions "small but real demand" once. The realistic sizing (5k-20k) lives only in addendum §A.6, which is downstream.
- **Proposed PRD location:** §2 or §9 — a one-line acknowledgement in the PRD body that the primary addressable audience is 5k-20k globally per addendum §A.6, so SM-3's 1,000 downloads is a 5-20% capture rate (ambitious but credible). This calibrates SM-3 from "obvious target" to "stretch but defensible."

### E3. "Why now" point #1 (Logseq's DB version dropping org-mode) is captured but loses the user-experience colour

- **Brainstorming reference:** Implicit in the diaspora reasoning; addendum §A.6 names it explicitly.
- **Status in PRD:** §9 Why Now point 1 captures it cleanly. ✅ No gap.

### E4. Author's "preferenza Rust, decisione finale con architetto" is preserved as stack preference but loses the "decision deferred to next workflow" intent

- **Brainstorming reference:** Phase 4 User Decisions: "Stack: preferenza Rust, decisione finale con architetto."
- **Status in PRD:** §10 OQ-2 captures the decision-deferral cleanly. ✅ Minor preservation.

### E5. The Karl Voit / org-purist hostility signal is in addendum §A.6 but not in PRD risks

- **Brainstorming reference:** Not explicit in brainstorming (post-brainstorming Discovery research); however brainstorming Theme 2 implicitly positions Orgsidian as **not for Emacs loyalists**.
- **Status in PRD:** §2.3 Non-Users lists Emacs power users. The **active hostility risk** (Voit-style critique of non-Emacs org tools) is only in addendum §A.6 and is not flagged as a launch risk in PRD §9 or anywhere.
- **Proposed PRD location:** §9 Why Now or a new §9.5 "Known risks at launch" — one-liner: "Expect public critique from the Emacs-loyalist segment of the org community (canonical example: Voit-style 'org-mode without Emacs misses the point'). The product position acknowledges this; launch comms should not engage." Cross-ref addendum §A.6.

---

## F. Brainstorming ideas not lifted into PRD or addendum

### F1. Git integration native (Feature #2 / candidate) — present in brainstorming, completely absent in PRD+addendum

- **Brainstorming reference:** Theme 5 [Feature #2 / candidate]: "Git integration nativa — history, diff, branch per scenari di pianificazione."
- **Status in PRD:** Not present anywhere. Addendum §A.1 source map says "Ideas not lifted (e.g., specific git-integration UI sketches, AI/LLM hook locations) remain in the brainstorming for future reference" — acknowledging the drop, but the brainstorming flagged Git integration as **a candidate Feature**, not a UI sketch. The "candidate" qualifier suggests intent to revisit.
- **Why it matters:** Git is the brainstorming's recommended sync mechanism (§4.4 in PRD also mentions it). A native Git integration (timeline view of headline history, diff for planning scenarios) is a natural and high-leverage addition that the brainstorming explicitly nominated.
- **Proposed PRD location:** §6.4 Out of Scope — add a line "Native Git integration (history, diff, branch-as-planning-scenario) — v1.5+ candidate feature, brainstorming Theme 5." This preserves the idea's identity without committing to it.

### F2. Side-by-side Source/Render (Onboarding #4) — partially absorbed but loses its onboarding-pedagogical role

- **Brainstorming reference:** Theme 4 [Onboarding #4]: "Side-by-side Source/Render (opzionale) — vista split per imparare la sintassi guardando il rendering."
- **Status in PRD:** FR-3 lists Split as an Editor Mode. ✅ Preserved as feature. The **onboarding pedagogy** rationale ("imparare la sintassi guardando il rendering") is not connected to FR-3 or to §4.5.
- **Proposed PRD location:** §4.5 Onboarding description — add a sentence linking Split mode to onboarding ("Split mode (FR-3) doubles as a syntax-learning aid: users can write in Raw and watch the Pseudo-WYSIWYG side update").

### F3. Workflow Recipes (Onboarding #6) — correctly deferred to v1.5+, but the "emotionally load-bearing" flag is honest

- **Brainstorming reference:** Theme 4 [Onboarding #6]: "Workflow Recipes (post-MVP, v1.5+) — gallery di workflow pre-pacchettati."
- **Status in PRD:** §6.4 lists "Workflow Recipes gallery — v1.5+. `[NOTE FOR PM]: emotionally load-bearing; revisit if v1.0 timeline permits.]`" ✅ Preserved with honest framing.

### F4. Themes — CSS-based with dark+light defaults — clean

- **Brainstorming reference:** Theme 5 [Feature #6].
- **Status in PRD:** FR-22 captures it. ✅ No gap.

---

## G. Distortions (small)

### G1. "Filesystem-native + indice SQLite" framed as architecture; brainstorming framed it as both architecture AND product-trust promise

- **Brainstorming reference:** Phase 1 Key Breakthroughs ("Decisione netta su filesystem-native + indice SQLite … come spina dorsale dello storage"); Phase 3 Candidate B refinements.
- **Status in PRD:** §4.4 description includes "This is the trust contract with the user" — ✅ trust framing preserved. Minor point.

### G2. "10h/settimana" working constraint is in addendum §A.7 but not in PRD §7 Constraints

- **Brainstorming reference:** Phase 4 explicit constraint; Phase 4 Action Plans all calibrate to it.
- **Status in PRD:** §7 Constraints does not mention author capacity. Addendum §A.7 mentions "10h/week ≈ 720h available."
- **Proposed PRD location:** §7 or §0 — a one-liner about author capacity (10h/week sustained) as the underpinning constraint for the 18-month roadmap math. This is the constraint that makes every other scope decision honest.

### G3. "Open question architettura: CodeMirror 6 compatibility con stack" was Phase 1 open; resolved partially in addendum but not in OQ list

- **Brainstorming reference:** Theme 1 [Open question architettura].
- **Status in PRD:** OQ-2 covers Tauri vs Electron; the **specific sub-question "does CodeMirror 6 work consistently in all three webviews under load"** is in addendum §A.3 as "Decision criteria for Spike 1-2 point 2." Not in OQ list explicitly.
- **Proposed PRD location:** §10 OQ-2 — add a sub-bullet pointing to the CodeMirror-6-cross-webview test as a sub-question.

---

## Summary of recommended PRD edits (prioritised)

| Priority | Item | Action |
|---|---|---|
| P1 | A1 | Add Design Principles section naming "Smart Defaults, User in Control" |
| P1 | A3 | Restore marketing-wedge vs product-integration duality in §1 Vision |
| P1 | C4 | Resolve time-tracking v0.5/v1.0 split inconsistency vs brainstorming intent |
| P1 | C2/C3 | Flag the brainstorming hour-budget vs item-estimate overrun as ASSUMPTION |
| P2 | A2 | Elevate "workflow over syntax" as a named principle |
| P2 | B4 | Document monorepo Core+Shell architecture in addendum §A.3 |
| P2 | C1 | Enumerate Pre-MVP spikes (4) with hour budgets in §6.0 or addendum §A.7 |
| P2 | C6 | Add SM-0 for Pre-MVP spike phase |
| P2 | D1 | Add UJ-7 weekly review |
| P3 | A1/A2 absorption | Cross-link new principles section from §4.5 Onboarding |
| P3 | B2 | Brief preface to §5 Non-Goals naming the cancelled-scope-creep four |
| P3 | E1 | Strengthen dogfooding-as-design-discipline statement in §1 |
| P3 | E2 | Mention 5k-20k addressable sizing in §2 or §9 |
| P3 | E5 | Note Voit-style critique risk near §9 |
| P3 | F1 | Add Git integration to §6.4 Out of Scope (v1.5+ candidate) |
| P3 | D2 | Add UJ-8 Plain → Power graduation |
| P3 | G2 | State 10h/week author capacity constraint in §7 |
| P4 | B1 | Surface "no Emacs runtime" as positive architectural decision, not only as non-goal |
| P4 | B5 | Add org-roam precedent to §4.4 description |
| P4 | F2 | Link Split mode to onboarding pedagogy in §4.5 |
| P4 | D3 | Optional UJ-9 index rebuild |
| P4 | G3 | Sub-bullet in OQ-2 on CodeMirror cross-webview |

---

## What the PRD got right (notable preservation wins)

- All 6 Phase 1 breakthroughs are at least partially reflected.
- Non-lossy round-trip as trust contract (FR-2) is correctly elevated to integrity-contract status.
- Pseudo-WYSIWYG trade-off (Breakthrough #6) is faithfully preserved in §4.1 notes and addendum §A.4 with full cost rationale.
- Single Writer Rule + Dirty Buffer + Merge Dialog (Phase 3 Candidate B refinement) is correctly surfaced as FR-16 and expanded in addendum §A.5.
- Counter-metrics SM-C1/C2/C3 are a strong addition not directly in brainstorming but consistent with its scope-protection spirit.
- Project Report as v0.5 wow demo (Phase 4 swap) preserved correctly.
- The brainstorming source map in addendum §A.1 is honest and explicit.

The PRD is not bad. It is a fast-path that has done well on FRs and roadmap mechanics; the gaps are concentrated in **principles, decision identity, and the brainstorming's qualitative wisdom that does not fit FR/feature shape.**
