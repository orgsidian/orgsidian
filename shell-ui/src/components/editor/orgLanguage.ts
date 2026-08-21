// Implements FR-3 — org-mode-aware syntax highlighting for Raw editor mode.
//
// Story 4.2: Raw mode shows plain `.org` source with *syntax-highlight tokens
// only* — no Pseudo-WYSIWYG decorations or widgets. This module is that token
// layer: a CodeMirror 6 `StreamLanguage` (LD-6) that tokenizes the org
// constructs the AC enumerates (headline asterisks, TODO/DONE state keywords,
// tags, active/inactive timestamps) plus a `HighlightStyle` that tags each with
// a stable `cm-org-*` class. Colors are applied externally (see
// `styles/editor.css`) through the `--org-*` token vocabulary; this file emits
// classes only, so the styling contract stays in CSS.
//
// It intentionally does NOT touch the buffer: highlighting is presentational,
// the source stays byte-faithful (the FR-2 round-trip contract).

import {
  HighlightStyle,
  StreamLanguage,
  syntaxHighlighting,
  type StreamParser,
} from "@codemirror/language";
import { type Extension } from "@codemirror/state";
import { Tag } from "@lezer/highlight";

/**
 * Stable CSS classes emitted on the highlighted token spans. Exported so tests
 * assert token presence by class and `styles/editor.css` styles them via the
 * `--org-*` vocabulary. Keys are the org construct; values are the DOM class.
 */
export const ORG_TOKEN_CLASS = {
  headingStars: "cm-org-heading-stars",
  todoKeyword: "cm-org-todo",
  doneKeyword: "cm-org-done",
  tag: "cm-org-tag",
  timestampActive: "cm-org-ts-active",
  timestampInactive: "cm-org-ts-inactive",
} as const;

// One private highlight `Tag` per construct. The tokenizer emits a token-name
// string, the `tokenTable` maps that name to the Tag, and the `HighlightStyle`
// maps the Tag to a `cm-org-*` class.
const orgTag = {
  headingStars: Tag.define(),
  todoKeyword: Tag.define(),
  doneKeyword: Tag.define(),
  tag: Tag.define(),
  timestampActive: Tag.define(),
  timestampInactive: Tag.define(),
};

// Token-name strings shared between `token()` and `tokenTable` (must match).
// Every name is `org`-prefixed so none collides with a built-in CodeMirror
// CM5-style token name (e.g. a bare `"tag"` resolves to the built-in tag style
// and never reaches our `tokenTable`).
const TOKEN = {
  headingStars: "orgHeadingStars",
  todoKeyword: "orgTodoKeyword",
  doneKeyword: "orgDoneKeyword",
  tag: "orgTag",
  timestampActive: "orgTimestampActive",
  timestampInactive: "orgTimestampInactive",
} as const;

// `StringStream.match(regex)` only matches at the current position (it returns
// null when the match index is > 0), so these patterns tokenize left-to-right.
const HEADLINE_STARS = /^\*+(?=\s)/;
// The org default TODO sequence keywords (LD-6). A full `#+TODO:`-configurable
// sequence is Story 4.3b; Raw highlighting recognizes the day-1 defaults.
const TODO_KEYWORD = /^(?:TODO|NEXT|WAITING)(?=\s|$)/;
const DONE_KEYWORD = /^DONE(?=\s|$)/;
// `:tag:` or `:tag1:tag2:` — org tag characters are word chars plus @ # % _.
const TAGS = /^:[A-Za-z0-9_@%#]+(?::[A-Za-z0-9_@%#]+)*:/;
// Active `<YYYY-MM-DD ...>` vs inactive `[YYYY-MM-DD ...]` timestamps.
const TIMESTAMP_ACTIVE = /^<\d{4}-\d{2}-\d{2}[^>\n]*>/;
const TIMESTAMP_INACTIVE = /^\[\d{4}-\d{2}-\d{2}[^\]\n]*\]/;

interface OrgHighlightState {
  /** True once the current line's leading `*+` stars were consumed. */
  inHeadline: boolean;
  /** True for exactly the first non-space token after the stars (keyword slot). */
  keywordEligible: boolean;
}

const orgParser: StreamParser<OrgHighlightState> = {
  name: "orgsidian-org",
  startState: () => ({ inHeadline: false, keywordEligible: false }),
  copyState: (state) => ({ ...state }),
  token(stream, state) {
    if (stream.sol()) {
      // A new line resets headline context; only a leading `*+ ` opens one.
      state.inHeadline = false;
      state.keywordEligible = false;
      if (stream.match(HEADLINE_STARS)) {
        state.inHeadline = true;
        state.keywordEligible = true;
        return TOKEN.headingStars;
      }
    }

    if (state.keywordEligible) {
      // Skip the whitespace between the stars and the first word (keeps the
      // keyword slot eligible for the next call).
      if (stream.eatSpace()) return null;
      // The keyword slot is the FIRST word only — consume the eligibility here
      // so a bare "TODO" later in the title is not mis-highlighted.
      state.keywordEligible = false;
      if (stream.match(TODO_KEYWORD)) return TOKEN.todoKeyword;
      if (stream.match(DONE_KEYWORD)) return TOKEN.doneKeyword;
      // Not a state keyword — fall through to the general matchers below.
    }

    if (stream.match(TAGS)) return TOKEN.tag;
    if (stream.match(TIMESTAMP_ACTIVE)) return TOKEN.timestampActive;
    if (stream.match(TIMESTAMP_INACTIVE)) return TOKEN.timestampInactive;

    // No token here: advance one char as unstyled text.
    stream.next();
    return null;
  },
  tokenTable: {
    [TOKEN.headingStars]: orgTag.headingStars,
    [TOKEN.todoKeyword]: orgTag.todoKeyword,
    [TOKEN.doneKeyword]: orgTag.doneKeyword,
    [TOKEN.tag]: orgTag.tag,
    [TOKEN.timestampActive]: orgTag.timestampActive,
    [TOKEN.timestampInactive]: orgTag.timestampInactive,
  },
};

const orgHighlightStyle = HighlightStyle.define([
  { tag: orgTag.headingStars, class: ORG_TOKEN_CLASS.headingStars },
  { tag: orgTag.todoKeyword, class: ORG_TOKEN_CLASS.todoKeyword },
  { tag: orgTag.doneKeyword, class: ORG_TOKEN_CLASS.doneKeyword },
  { tag: orgTag.tag, class: ORG_TOKEN_CLASS.tag },
  { tag: orgTag.timestampActive, class: ORG_TOKEN_CLASS.timestampActive },
  { tag: orgTag.timestampInactive, class: ORG_TOKEN_CLASS.timestampInactive },
]);

const orgLanguage = StreamLanguage.define(orgParser);

/**
 * The org syntax-highlight extension: the org `StreamLanguage` plus its
 * `HighlightStyle`. Present in every Editor Mode; in Raw mode it is the ONLY
 * presentational layer (no decorations/widgets).
 */
export function orgSyntaxHighlight(): Extension {
  return [orgLanguage, syntaxHighlighting(orgHighlightStyle)];
}
