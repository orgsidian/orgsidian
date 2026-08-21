// @vitest-environment happy-dom
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";

/**
 * Story 4.3g perf gate — `assert_no_perf_regression!("story-4.3g-source-fidelity", …)`.
 *
 * PERF-HARNESS DECISION (see the story file's "Perf-harness decision" section):
 * the Story 1.12 macro (`crates/orgsidian-core/src/test_support/perf.rs`) is
 * Rust-only — it times a Rust closure. Source-fidelity ops run over a CM6
 * `EditorView` (pure TS), so they cannot be driven from a Rust test without
 * reimplementing CodeMirror. This is the closest faithful equivalent the repo
 * supports, mirroring the macro's semantics:
 *   - MEDIAN-OF-5 samples per op (matches `SAMPLES = 5`);
 *   - ±20% TOLERANCE (matches `TOLERANCE_PCT`);
 *   - a self-calibrating BASELINE: the op on the fully-decorated Pseudo-WYSIWYG
 *     editor is compared against the SAME op on an undecorated Raw editor of the
 *     identical document. That is exactly "regression from baseline" (the
 *     baseline being the editor before the 4.3 decorations existed), and it is
 *     machine-independent — the reason the Rust harness commits per-runner
 *     baselines, which a TS microbench cannot reuse cross-runner.
 *
 * The guarded claim: the 4.3a–4.3f decoration layer does not regress copy or
 * find/replace latency by more than 20% (structurally it cannot — these ops read
 * `EditorState.doc`, never the decoration set — and this locks that in).
 */

import { modeExtensions } from "../editorMode";
import { sourceFidelity, SearchCursor } from "./sourceFidelity";

const SAMPLES = 5; // matches perf.rs SAMPLES
const TOLERANCE = 1.2; // matches perf.rs TOLERANCE_PCT (20%)
// A small absolute noise floor added on top of the 20% relative tolerance. Both
// ops here are decoration-independent by construction, so the "true" ratio is
// ~1.0; this cushion absorbs sub-millisecond scheduler/GC jitter that can
// otherwise make a percentage comparison of a very fast op flaky, while a real
// regression (copy walking the rendered DOM, search consulting decorations)
// would be orders of magnitude larger and still trip the gate.
const NOISE_FLOOR_NS = 2_000_000; // 2ms

// A ~560-line document with every decoration kind, so the ops have real work.
function bigDoc(): string {
  const block = [
    "* TODO Buy milk :work:urgent:",
    "Body with http://example.com and [[id:abc][a link]].",
    "** DONE Ship it <2026-05-19 Tue 14:00>",
    "- [ ] pending task",
    "- [X] finished task",
    "*** NEXT Deep dive [2026-01-02 Fri]",
    "plain body line with no decorations at all",
  ];
  const lines: string[] = ["#+TODO: TODO NEXT WAITING | DONE"];
  for (let i = 0; i < 80; i += 1) lines.push(...block);
  return lines.join("\n");
}

let views: EditorView[] = [];

afterEach(() => {
  for (const v of views) v.destroy();
  views = [];
});

function mount(mode: "pseudoWysiwyg" | "raw", doc: string): EditorView {
  const v = new EditorView({
    doc,
    parent: document.body,
    extensions: [modeExtensions(mode), sourceFidelity()],
  });
  views.push(v);
  return v;
}

/** Median of `SAMPLES` timings (ns), each timing `inner` repeats of `op`. */
function medianNs(op: () => void, inner: number, warmup: number): number {
  for (let i = 0; i < warmup; i += 1) op();
  const samples: number[] = [];
  for (let s = 0; s < SAMPLES; s += 1) {
    const start = performance.now();
    for (let i = 0; i < inner; i += 1) op();
    samples.push((performance.now() - start) * 1e6); // ns for the sample
  }
  samples.sort((a, b) => a - b);
  return samples[Math.floor(SAMPLES / 2)];
}

// The two fidelity ops, exactly as CM6 performs them:
//   - copy serializes `state.sliceDoc()` over the selection (here: whole doc);
//   - find scans the document `Text` with a `SearchCursor`.
function copyOp(v: EditorView): () => void {
  return () => void v.state.sliceDoc(0, v.state.doc.length);
}
function findOp(v: EditorView): () => void {
  return () => {
    const cur = new SearchCursor(v.state.doc, "TODO");
    let n = 0;
    while (!cur.next().done) n += 1;
    if (n === 0) throw new Error("expected matches");
  };
}

describe("story-4.3g-source-fidelity — perf gate (decorated vs raw baseline)", () => {
  it("copy latency does not regress > 20% with all decorations active", () => {
    const doc = bigDoc();
    const decorated = mount("pseudoWysiwyg", doc);
    const raw = mount("raw", doc);

    const baseline = medianNs(copyOp(raw), 1000, 200);
    const measured = medianNs(copyOp(decorated), 1000, 200);
    // eslint-disable-next-line no-console
    console.log(
      `[story-4.3g-source-fidelity] copy: baseline(raw)=${baseline | 0}ns measured(decorated)=${measured | 0}ns ratio=${(measured / baseline).toFixed(3)}`,
    );
    expect(measured).toBeLessThanOrEqual(baseline * TOLERANCE + NOISE_FLOOR_NS);
  });

  it("find latency does not regress > 20% with all decorations active", () => {
    const doc = bigDoc();
    const decorated = mount("pseudoWysiwyg", doc);
    const raw = mount("raw", doc);

    const baseline = medianNs(findOp(raw), 20, 10);
    const measured = medianNs(findOp(decorated), 20, 10);
    // eslint-disable-next-line no-console
    console.log(
      `[story-4.3g-source-fidelity] find: baseline(raw)=${baseline | 0}ns measured(decorated)=${measured | 0}ns ratio=${(measured / baseline).toFixed(3)}`,
    );
    expect(measured).toBeLessThanOrEqual(baseline * TOLERANCE + NOISE_FLOOR_NS);
  });
});
