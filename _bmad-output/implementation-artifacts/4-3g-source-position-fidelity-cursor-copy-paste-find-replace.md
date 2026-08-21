---
title: 'Source-position fidelity (cursor, copy-paste, find/replace)'
type: 'feature'
created: '2026-08-21'
status: 'review'
baseline_commit: 'cdb8daf'
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/implementation-artifacts/epic-4-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Stories 4.3a–4.3f layered six decoration types onto Pseudo-WYSIWYG mode — heading line decorations, TODO-state replace-widgets, tag-pill replace-widgets, timestamp replace-widgets, checkbox replace-widgets, and link mark/replace decorations. Several of these are `Decoration.replace({ widget })`, the one CM6 construct that can, if mis-ranged, leak *rendered* text or *visual* positions into copy/paste, selection, and find/replace. FR-3 / FR-4 require the opposite: cursor placement, copy, paste, and find/replace must read/write **source** character offsets, not rendered positions, no matter how many decorations are in view — the product's trust contract ("show `.org` source, don't hide it").

**Approach:** This is primarily a **verification + hardening** story. It (a) adds `@codemirror/search` (the `search` member of the LD-6 editor stack, not previously wired) so find/replace is genuinely user-invokable and provably operates on `EditorState.doc` (source), bundled behind a new mode-independent `sourceFidelity()` extension wired once into the `Editor` host; (b) proves the whole source-fidelity contract with an integration suite that mounts **all** 4.3a–4.3f decorations together and exercises real copy (clipboard event), paste (transaction), find, and replace through source offsets; (c) supplies the Story-4.3g perf gate. No decoration behavior is changed — every 4.3 widget already ranges over exact source offsets; the tests lock that in.

## Boundaries & Constraints

**Always:**
- Copy/cut/paste and find/replace operate on `EditorState.doc` — CM6's clipboard serialization is `state.sliceDoc()` over selection ranges and search runs over the document `Text`; decorations are presentational and never enter these paths.
- Every 4.3 decoration's range is the exact **source** span (`line.from + offset`, `base + match.index`, …). This story asserts that invariant; it does not alter any decoration's ranges.
- `sourceFidelity()` is mode-independent — find/replace works in Raw, Pseudo-WYSIWYG, and Split alike, so it lands in the `Editor` host's base extensions, not in the Pseudo-WYSIWYG-only decoration set.
- No `atomicRanges` are introduced: skipping collapsed ranges would make source offsets *un*-addressable by the cursor, the opposite of the AC. Every source character stays reachable.

**Ask First:**
- Wiring a search *keymap* that could collide with Story 4.6 default keybindings — the bundle uses CM6's `searchKeymap`, which binds `Mod-f`/`Mod-Alt-g` etc. that `defaultKeymap` does not, so there is no present conflict; 4.6 can layer its map on top (CM6 precedence by order).

**Never:**
- Mutate the buffer during copy/paste/find (only an explicit replace is a source edit, tagged as a user event by CM6's own replace command).
- Refactor 4.3a–4.3f decoration internals beyond a minimal, source-range fix if one is found (none was).
- A barrel `decorations/index.ts`.

## Perf-harness decision (explicit)

The `assert_no_perf_regression!("story-4.3g-source-fidelity", …)` macro (Story 1.12) is **Rust-only** (`crates/orgsidian-core/src/test_support/perf.rs`): it times a Rust closure, medians 5 samples, and gates ±20% against a per-`runner_class` baseline JSON. Source-fidelity ops (copy/paste/find/replace over a CM6 `EditorView`) are **pure-TS/CM6** and cannot be exercised from a Rust test without reimplementing CM6. Per the story's own guidance ("add the closest faithful equivalent the repo supports and document the decision"), the gate is implemented as a **Vitest perf test** (`sourceFidelity.perf.test.tsx`) that mirrors the macro's semantics faithfully:
- **median-of-5** samples per op (same as `SAMPLES = 5`);
- **±20% tolerance** (same as `TOLERANCE_PCT`);
- a **self-calibrating baseline** rather than a machine-absolute number: the decorated-editor op is compared against the *same op on an undecorated (Raw) editor of the identical document*. This makes the gate machine-independent (the reason the Rust harness commits per-runner baselines) and answers the exact AC question — "do the 4.3 decorations regress source-fidelity op latency > 20%?" The Rust harness's committed-baseline model is unavailable cross-runner in CI for a TS microbench, so the raw-mode op is the baseline.

The test id string `story-4.3g-source-fidelity` is preserved in the test name/comment for traceability to the AC.

## I/O & Edge-Case Matrix

| Scenario | Input / State (all decorations active) | Expected |
|----------|----------------------------------------|----------|
| copy heading line | selection over `** Heading` (heading line-deco) | clipboard text `** Heading` (source, incl. stars) |
| copy TODO line | selection over `* TODO task` (replace-widget badge) | clipboard text contains `TODO` keyword, not badge label |
| copy across widgets | select-all of a doc with every decoration | clipboard === full source byte-for-byte |
| find `TODO` | search query `TODO`, badge rendered | match at source offset of the keyword span |
| find heading source | query `** ` | matches the source stars, not rendered text |
| find timestamp raw | query `2026-05-19` | matches inside the rendered timestamp's source |
| replace `TODO`→`DONE` | replaceNext over the badge | doc source updated; rest byte-identical; badge re-renders `DONE` |
| paste at offset | insert `X` at a source offset inside a decorated line | inserted at that source offset; doc correct |
| round-trip | mount all decorations, no edit | `sliceDoc()` byte-identical to source |
| perf | 5× copy/find/replace, decorated vs raw | decorated median ≤ raw median × 1.20 |

</frozen-after-approval>

## Code Map

- `shell-ui/src/components/editor/decorations/sourceFidelity.ts` — NEW. `// Implements FR-3, FR-4`. `sourceFidelity()` extension bundling `@codemirror/search`'s `search()` + `keymap.of(searchKeymap)` so find/replace is user-invokable and source-based in every mode. Re-exports the search primitives (`SearchQuery`, `SearchCursor`, `findNext`, `replaceNext`, `replaceAll`, `setSearchQuery`) the tests and future stories use, keeping the CM6 search import surface in one place.
- `shell-ui/src/components/editor/decorations/sourceFidelity.test.tsx` — NEW. Vitest + happy-dom. Mounts a full editor (`modeExtensions("pseudoWysiwyg")` + `sourceFidelity()`) over a doc with every 4.3 decoration and asserts copy (real `copy` ClipboardEvent), select-all round-trip, find, replace, and paste all hit source offsets.
- `shell-ui/src/components/editor/decorations/sourceFidelity.perf.test.tsx` — NEW. The Story-4.3g perf gate (see decision above): median-of-5, ±20%, decorated-vs-raw baseline.
- `shell-ui/src/components/editor/Editor.tsx` — EDIT (minimal): one import + one entry in the base extensions array (`sourceFidelity()`), so find/replace is available in all modes.
- `shell-ui/package.json` / `pnpm-lock.yaml` — add `@codemirror/search` (LD-6 stack member), pinned exact like the sibling `@codemirror/*` deps.

## Tasks & Acceptance

**Execution:**
- [x] Add `@codemirror/search` dependency (pinned exact).
- [x] `sourceFidelity.ts` — `sourceFidelity()` bundle + search re-exports.
- [x] `Editor.tsx` — wire `sourceFidelity()` into base extensions.
- [x] `sourceFidelity.test.tsx` — copy / round-trip / find / replace / paste fidelity with all decorations active.
- [x] `sourceFidelity.perf.test.tsx` — perf gate (decorated ≤ raw × 1.20, median-of-5).

**Acceptance Criteria:**
- Given all 4.3a–4.3f decorations active, when copy/paste/find/replace run, then source character offsets are read/written — verified.
- And copying a heading line copies `** Heading` (source) — verified.
- And find/replace on `TODO` finds the source keyword, not the rendered badge — verified.
- And the perf gate confirms op latency does not regress > 20% from baseline — verified via the documented TS-side faithful equivalent.

## Verification

See PR description / final report for exact numbers.

## Design Notes

- **Why CM6 gives source fidelity structurally:** the `EditorState.doc` (`Text`) *is* the source. Copy/cut serialize `state.sliceDoc()` over the selection; paste dispatches an insert transaction at the selection; `@codemirror/search` runs its cursor over the document `Text`. Decorations (`mark`, `line`, `replace`, widgets) only change what is *painted* — they never participate in clipboard or search. So the fidelity guarantee is a property of the architecture, and every 4.3 widget already keys its decoration to the exact source range; this story's job is to prove it stays true with all of them layered together, and to make find/replace real.
- **No atomicRanges by design:** adding replace ranges to `EditorView.atomicRanges` would make the cursor *skip* the source characters under a widget, i.e. make some source offsets unreachable — the exact opposite of "operate on source character offsets". The timestamp layer already *reveals* its source when the selection touches it (4.3d); the other widgets keep their source reachable char-by-char. Left unchanged.
- **Perf-gate faithful equivalent:** documented above under "Perf-harness decision".
</content>
</invoke>
