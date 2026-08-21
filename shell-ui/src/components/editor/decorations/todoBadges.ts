// Implements FR-4 — Pseudo-WYSIWYG TODO-state pill badges (click-to-cycle).
//
// Story 4.3b. This is the CM6 decoration layer that, in Pseudo-WYSIWYG (and
// Split) mode, REPLACES each headline's TODO-state keyword with a colored,
// clickable `TodoStateCycler` pill (`components/org/TodoStateCycler.ts`).
// Raw mode never includes this extension, so it stays decoration-free.
//
// Design contract (Epic 4 / LD-6):
//   - `Decoration.replace({ widget })` over the keyword's exact source range —
//     the buffer stays byte-faithful; only the presentation changes.
//   - Clicking a pill cycles to the next state in the configured `#+TODO:`
//     sequence via a single `Transaction` tagged `userEvent="input.cycle-todo"`
//     — the one shared mutation path (FR-24 / LD-26), never a private write.
//   - The cycle is never dispatched while the view is composing (IME) — the
//     LD-6 "never dispatch during composition" recipe.
//   - Colors resolve through the `--org-accent-{todo,next,done,waiting}` tokens
//     (see `styles/editor.css`), falling back to the FR-22 vocabulary until the
//     Story 6.7 palette declares them.

import { RangeSetBuilder, type Extension, type Text } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import {
  TodoStateCycler,
  type TodoStateClass,
} from "@/components/org/TodoStateCycler";

/**
 * The `Transaction.userEvent` tag on every badge-triggered source change. Kept
 * public so the keybinding story (4.6) and tests can assert the shared path.
 */
export const CYCLE_TODO_USER_EVENT = "input.cycle-todo";

/**
 * Day-1 default state sequence used when the document declares no `#+TODO:`
 * directive. Matches the keywords the Raw-mode highlighter recognizes (TODO,
 * NEXT, WAITING as not-done; DONE as done), in cycle order.
 */
export const DEFAULT_TODO_SEQUENCE = [
  "TODO",
  "NEXT",
  "WAITING",
  "DONE",
] as const;

// Headline prefix: one-or-more leading stars followed by whitespace. The
// keyword (if any) is the first whitespace-delimited token after the stars.
const HEADLINE_PREFIX = /^(\*+)\s+/;
// `#+TODO:` / `#+SEQ_TODO:` / `#+TYP_TODO:` directive (case-insensitive), whose
// value is a space-separated keyword list with an optional `|` done-separator.
const TODO_DIRECTIVE = /^\s*#\+(?:SEQ_|TYP_)?TODO:\s*(.+?)\s*$/i;

const STATE_CLASSES: ReadonlySet<TodoStateClass> = new Set([
  "todo",
  "next",
  "done",
  "waiting",
]);

/** Map a keyword to its `--org-accent-*` styling bucket. */
function classFor(keyword: string): TodoStateClass {
  const lower = keyword.toLowerCase() as TodoStateClass;
  return STATE_CLASSES.has(lower) ? lower : "other";
}

/**
 * Resolve the active TODO keyword sequence for a document: the union (in order,
 * de-duplicated) of every `#+TODO:`-family directive's keywords, with the `|`
 * done-separator dropped. Falls back to {@link DEFAULT_TODO_SEQUENCE} when the
 * document declares none.
 */
export function resolveTodoSequence(doc: Text): string[] {
  const keywords: string[] = [];
  const seen = new Set<string>();
  // Directives live near the top of a file, but scanning all lines is cheap and
  // robust; `#+TODO:` anywhere in the buffer is honored.
  for (let i = 1; i <= doc.lines; i += 1) {
    const match = TODO_DIRECTIVE.exec(doc.line(i).text);
    if (!match) continue;
    for (const token of match[1].split(/\s+/)) {
      if (token === "" || token === "|") continue;
      if (!seen.has(token)) {
        seen.add(token);
        keywords.push(token);
      }
    }
  }
  return keywords.length > 0 ? keywords : [...DEFAULT_TODO_SEQUENCE];
}

/** The next keyword after `keyword`, wrapping past the end of the sequence. */
function nextState(sequence: readonly string[], keyword: string): string {
  const index = sequence.indexOf(keyword);
  if (index === -1) return sequence[0];
  return sequence[(index + 1) % sequence.length];
}

/**
 * Replace the keyword at [`from`, `to`) with `next` atomically, tagged as the
 * shared `input.cycle-todo` user event. Skips dispatch while composing (LD-6).
 * Exported as the single cycle command the widget and future keybindings share.
 */
export function cycleTodoState(
  view: EditorView,
  change: { from: number; to: number; next: string },
): void {
  if (view.composing) return;
  view.dispatch({
    changes: { from: change.from, to: change.to, insert: change.next },
    userEvent: CYCLE_TODO_USER_EVENT,
  });
}

/**
 * Keyboard-driven TODO cycle for the headline at (or above) the main cursor —
 * the command form of the click-to-cycle pill, wired to the default keymap in
 * Story 4.6 (`keybindings/default.ts`). Reuses {@link cycleTodoState} so the
 * keyboard path and the pill share ONE mutation surface (FR-24 / LD-26) and the
 * same `input.cycle-todo` user-event tag.
 *
 * Behavior mirrors the pill: if the headline already carries a keyword from the
 * active sequence it advances to the next state (wrapping past the end); if it
 * carries none, the first keyword is inserted right after the stars. Returns
 * `true` when it acted (so CM6 treats the chord as handled) and `false` when the
 * cursor is not under a headline (the chord falls through).
 */
export function cycleTodoAtCursor(view: EditorView): boolean {
  const { state } = view;
  const cursorLine = state.doc.lineAt(state.selection.main.head);
  // Walk up to the nearest headline at or above the cursor.
  let headline: { from: number; text: string } | null = null;
  for (let n = cursorLine.number; n >= 1; n -= 1) {
    const line = state.doc.line(n);
    if (HEADLINE_PREFIX.test(line.text)) {
      headline = { from: line.from, text: line.text };
      break;
    }
  }
  if (headline === null) return false;

  const prefix = HEADLINE_PREFIX.exec(headline.text);
  if (prefix === null) return false;
  const sequence = resolveTodoSequence(state.doc);
  const kFrom = headline.from + prefix[0].length;
  const rest = headline.text.slice(prefix[0].length);
  const token = /^(\S+)/.exec(rest);

  if (token && sequence.includes(token[1])) {
    // Existing keyword → advance to the next state (wraps), replacing in place.
    cycleTodoState(view, {
      from: kFrom,
      to: kFrom + token[1].length,
      next: nextState(sequence, token[1]),
    });
    return true;
  }
  // No keyword yet → insert the first state keyword after the stars.
  cycleTodoState(view, { from: kFrom, to: kFrom, next: `${sequence[0]} ` });
  return true;
}

/** Build the badge decoration set over the view's visible ranges. */
function buildBadges(view: EditorView): DecorationSet {
  const sequence = resolveTodoSequence(view.state.doc);
  const keywords = new Set(sequence);
  const builder = new RangeSetBuilder<Decoration>();

  for (const { from, to } of view.visibleRanges) {
    let pos = from;
    while (pos <= to) {
      const line = view.state.doc.lineAt(pos);
      const prefix = HEADLINE_PREFIX.exec(line.text);
      if (prefix) {
        const rest = line.text.slice(prefix[0].length);
        const token = /^(\S+)/.exec(rest);
        if (token && keywords.has(token[1])) {
          const keyword = token[1];
          const kFrom = line.from + prefix[0].length;
          const kTo = kFrom + keyword.length;
          builder.add(
            kFrom,
            kTo,
            Decoration.replace({
              widget: new TodoStateCycler(
                {
                  from: kFrom,
                  to: kTo,
                  keyword,
                  next: nextState(sequence, keyword),
                  stateClass: classFor(keyword),
                },
                cycleTodoState,
              ),
            }),
          );
        }
      }
      pos = line.to + 1;
    }
  }

  return builder.finish();
}

const todoBadgePlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildBadges(view);
    }

    update(update: ViewUpdate): void {
      // Rebuild only when the source or the visible viewport changed; the badge
      // set is a pure function of both. No dispatch happens here, so the LD-6
      // "never dispatch inside update() while composing" hazard cannot arise.
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildBadges(update.view);
      }
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
  },
);

/**
 * The Pseudo-WYSIWYG TODO-badge extension. Added to the mode's decoration set
 * in `editorMode.ts`; absent from Raw mode.
 */
export function todoBadges(): Extension {
  return todoBadgePlugin;
}
