// Implements FR-3 — Split editor mode (two-view, shared-buffer surface).
//
// Story 4.4: Split mode presents the SAME buffer in a 50/50 surface — a Raw
// source view (left, syntax-highlight only) beside a Pseudo-WYSIWYG view
// (right, the full decoration layer). Both panes edit ONE logical buffer.
//
// CM6 has no single `EditorState` object literally shared by two views (each
// view owns its state field), so the canonical CM6 "split view" recipe realizes
// the shared buffer by giving both views the same initial document and
// FORWARDING every change transaction from whichever view originates it to its
// sibling (tagged with a sync annotation so the forward never echoes back into
// an infinite loop). The two states therefore hold byte-identical documents at
// all times: an edit in either pane updates the underlying buffer atomically
// for both. Selection/cursor is intentionally NOT forwarded — each pane keeps
// its own caret. See https://codemirror.net/examples/split/.
//
// Scroll position is mirrored between panes with an equality-guarded DOM scroll
// listener (setting the sibling only when it differs, which self-terminates the
// feedback the mirrored write would otherwise cause).
//
// This is a plain imperative factory rather than a React component on purpose:
// the `Editor` host already owns the single StrictMode-safe async load chain and
// the `EditorHandle`; it drives this factory inside that one `useEffect` and
// calls `destroy()` from the effect cleanup, so both views are torn down exactly
// once per creation (no leak across a StrictMode double-mount).

import {
  Annotation,
  type Extension,
  type Text,
  Transaction,
} from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import { modeExtensions } from "./editorMode";

/**
 * Marks a transaction as the forwarded (mirror) copy of a change that already
 * happened in the sibling view, so the sync handler applies it without
 * forwarding it a second time — the loop-breaker for the two-way sync.
 */
const syncAnnotation = Annotation.define<boolean>();

/**
 * A live Split surface: the two synced views plus an idempotent teardown.
 * `primaryView` (the left / Raw pane) is the view the `EditorHandle` exposes and
 * focuses — it is the conventional "source of truth" pane for cursor/focus.
 */
export interface SplitSurface {
  /** Left pane — Raw source (syntax highlight only). The primary/handle view. */
  primaryView: EditorView;
  /** Right pane — Pseudo-WYSIWYG (the full decoration/widget layer). */
  secondaryView: EditorView;
  /**
   * Destroy both views and remove the surface wrapper from the parent. Safe to
   * call more than once (idempotent): a second call is a no-op.
   */
  destroy: () => void;
}

/** Options for {@link createSplitEditor}. */
export interface SplitEditorOptions {
  /** Element the split surface is appended to (the `Editor` host container). */
  parent: HTMLElement;
  /**
   * The shared buffer seed. A `Text` (handed over live from a previous surface
   * so a mode switch neither reloads from disk nor drops unsaved edits) or a
   * plain string on first open.
   */
  doc: string | Text;
  /**
   * Extensions common to both panes (editable flag, base keymap, editor face).
   * The panes differ ONLY by their mode-specific extension set on top of this.
   */
  baseExtensions: Extension;
}

/**
 * Apply `trs` to `self`, then forward each non-empty, not-already-forwarded
 * change to `other` so both buffers stay identical. The forwarded transaction
 * carries only the changes (never the selection) plus the sync annotation and
 * the original `userEvent` tag — preserving round-trip/plugin-surface semantics
 * (LD-26) while keeping each pane's caret independent.
 */
function syncDispatch(
  trs: readonly Transaction[],
  self: EditorView,
  other: () => EditorView,
): void {
  self.update(trs);
  for (const tr of trs) {
    if (tr.changes.empty || tr.annotation(syncAnnotation) !== undefined) {
      continue;
    }
    const annotations: Annotation<unknown>[] = [syncAnnotation.of(true)];
    const userEvent = tr.annotation(Transaction.userEvent);
    if (userEvent !== undefined) {
      annotations.push(Transaction.userEvent.of(userEvent));
    }
    other().dispatch({ changes: tr.changes, annotations });
  }
}

/**
 * Keep `to`'s scroll offset matched to `from`'s. The equality guard both avoids
 * redundant writes and self-terminates the sync: when `from` scrolls we set
 * `to`; `to`'s own listener then fires, finds the offsets already equal, and
 * writes nothing back — so there is no feedback loop and no reentrancy flag to
 * manage. Returns the bound listener so teardown can detach it.
 */
function bindScrollSync(from: EditorView, to: EditorView): () => void {
  const listener = () => {
    if (to.scrollDOM.scrollTop !== from.scrollDOM.scrollTop) {
      to.scrollDOM.scrollTop = from.scrollDOM.scrollTop;
    }
    if (to.scrollDOM.scrollLeft !== from.scrollDOM.scrollLeft) {
      to.scrollDOM.scrollLeft = from.scrollDOM.scrollLeft;
    }
  };
  from.scrollDOM.addEventListener("scroll", listener);
  return listener;
}

/**
 * Build the 50/50 Split surface into `parent`.
 *
 * Left pane = `modeExtensions("raw")` (highlight only); right pane =
 * `modeExtensions("pseudoWysiwyg")` (decoration layer). Both seed from the same
 * `doc`, and each view's `dispatchTransactions` forwards changes to the other so
 * the buffer stays shared and edits are atomic across panes.
 */
export function createSplitEditor(options: SplitEditorOptions): SplitSurface {
  const { parent, doc, baseExtensions } = options;

  // Imperative DOM (mirrors how CM6 appends its own `.cm-editor`): a flex row of
  // two equal, independently scrolling panes with a token-colored divider.
  const wrapper = document.createElement("div");
  wrapper.className = "flex h-full w-full";
  wrapper.dataset.orgSplit = "true";

  const leftPane = document.createElement("div");
  leftPane.className = "h-full min-w-0 flex-1 overflow-auto";
  leftPane.dataset.orgSplitPane = "raw";

  const rightPane = document.createElement("div");
  rightPane.className =
    "h-full min-w-0 flex-1 overflow-auto border-l border-[var(--org-border-default)]";
  rightPane.dataset.orgSplitPane = "pseudoWysiwyg";

  wrapper.append(leftPane, rightPane);
  parent.append(wrapper);

  // `let` + getters break the definition cycle: each view's dispatch must be
  // able to reach the other, but both are still being constructed here.
  let leftView: EditorView | undefined;
  let rightView: EditorView | undefined;

  leftView = new EditorView({
    parent: leftPane,
    doc,
    extensions: [baseExtensions, modeExtensions("raw")],
    dispatchTransactions: (trs) =>
      syncDispatch(trs, leftView as EditorView, () => rightView as EditorView),
  });

  // The right pane reuses the SAME document object, so no second copy is made.
  rightView = new EditorView({
    parent: rightPane,
    doc: leftView.state.doc,
    extensions: [baseExtensions, modeExtensions("pseudoWysiwyg")],
    dispatchTransactions: (trs) =>
      syncDispatch(trs, rightView as EditorView, () => leftView as EditorView),
  });

  const leftScrollListener = bindScrollSync(leftView, rightView);
  const rightScrollListener = bindScrollSync(rightView, leftView);

  let destroyed = false;
  const destroy = () => {
    if (destroyed) return;
    destroyed = true;
    leftView?.scrollDOM.removeEventListener("scroll", leftScrollListener);
    rightView?.scrollDOM.removeEventListener("scroll", rightScrollListener);
    leftView?.destroy();
    rightView?.destroy();
    wrapper.remove();
  };

  return { primaryView: leftView, secondaryView: rightView, destroy };
}
