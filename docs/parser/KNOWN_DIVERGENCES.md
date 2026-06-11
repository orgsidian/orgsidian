# Known Divergences — orgsidian-parser vs org-mode/Emacs

This document tracks verified divergences between Orgsidian's parser stack
(the vendored `nvim-orgmode/tree-sitter-org` grammar at the SHA-pinned
submodule `crates/orgsidian-parser/grammar/`, commit
`219c0b27fdb2c0aeb43841f23f03d6f54657f288`, plus the Story 2.3 semantic
layer) and org-mode's reference behavior (Emacs `org-element`).

**LD-45 role:** this file is the landing zone of the divergence-triage
workflow. From Story 2.7 onward, every nightly Emacs-oracle mismatch is
triaged into an entry here (construct, expected, observed, chosen behavior,
status/owner). Until then it records the gaps verified during Stories
2.2/2.3 implementation. The grammar itself is READ-ONLY per LD-48 — local
grammar edits are never an acceptable fix.

Entry format: **Construct** / Expected (org-mode/Emacs) / Observed
(tree-sitter-org @ pinned SHA) / Orgsidian behavior / Status & owner.

---

## 1. Links are not modeled as named nodes

- **Construct:** `[[target]]`, `[[target][description]]`, bare `http(s)://…`.
- **Expected (org-mode):** `org-element` produces structured `link` objects
  with type, path, and description, covering bracket links, angle links
  (`<http://…>`), plain links, link abbreviations, and `~/` file expansion.
- **Observed (tree-sitter-org):** no `link` node exists; bracketed links and
  bare URLs are `expr` token soup inside `paragraph`.
- **Orgsidian behavior:** the semantic layer (Story 2.3,
  `src/semantic/link.rs`) hand-rolls an inline scanner over the source
  slice: `[[…]]`/`[[…][…]]` spans classified by target prefix (`id:` →
  `Id`, `file:` → `File`, `http(s)://` → `Url`, no scheme → `Wiki`) plus a
  plain-`http(s)://` scan. Simplest correct bracket reading: `][` splits
  target/description, `]]` terminates, empty targets are not links, and a
  candidate that hits a newline before terminating is abandoned (review
  posture 2026-06-10: an unterminated `[[` must not swallow following
  paragraphs). Plain URLs require a word boundary (start of text or a
  preceding non-alphanumeric byte) and a non-empty remainder after the
  scheme.
- **Divergence risk:** angle links, link abbreviations (`#+LINK:`), `~/`
  expansion, and other `org-element` link types are NOT recognized — they
  stay raw text. Multi-line (wrapped) bracket links, which org-element
  accepts, are not recognized either. Scheme matching is case-sensitive
  (org-faithful: link types are lowercase — `HTTP://x` / `File:x` classify
  as wiki targets; bare `HTTP://x` is plain text). Trailing-punctuation
  trimming on plain URLs is heuristic (`.,;:!?)'"` stripped — a legitimate
  trailing `)` as in `wiki/Foo_(bar)` is lost from the target; raw text
  unaffected). The scan is purely textual over each headline's region:
  link-shaped text inside verbatim contexts (`#+BEGIN_SRC` /
  `#+BEGIN_EXAMPLE` blocks, drawer contents, property values) is also
  reported as links — org-element would not. Epic 4 decides whether
  verbatim regions get excluded (tracked in deferred-work, story-2.3
  review stanza).
- **Status/owner:** accepted gap for v0.1; revisit when Epic 4 link
  navigation needs the full org link grammar. Round-trip unaffected (raw
  text preserved; spans carried).

## 2. Inline markup is not modeled

- **Construct:** `*bold*`, `/italic/`, `=verbatim=`, `~code~`, `+strike+`,
  `_underline_`.
- **Expected (org-mode):** `org-element` emphasis objects.
- **Observed (tree-sitter-org):** `expr` soup inside `paragraph`; no
  emphasis nodes.
- **Orgsidian behavior:** the semantic layer does not expose emphasis; raw
  text (and spans) are preserved.
- **Status/owner:** Epic 4 rendering needs its own inline pass; round-trip
  unaffected. Tracked in deferred-work (story-2.3 stanza).

## 3. Inline LaTeX is not modeled

- **Construct:** `$…$`, `\(…\)`, `\[…\]`.
- **Expected (org-mode):** latex-fragment objects.
- **Observed (tree-sitter-org):** `expr` soup; only `\begin{…}…\end{…}`
  environments get `latex_env` nodes.
- **Orgsidian behavior:** tolerated as raw text; non-crash covered by
  `semantic_inline_latex`.
- **Status/owner:** Epic 4 consumer decides; raw text preserved.

## 4. Citations are not modeled

- **Construct:** `[cite:@key]`.
- **Expected (org-mode):** citation objects (org-cite).
- **Observed (tree-sitter-org):** `expr` soup.
- **Orgsidian behavior:** raw text; the link scanner explicitly does not
  treat single-bracket forms as links (covered by `semantic_citations`).
- **Status/owner:** out of v0.1 scope; revisit with a citations feature.

## 5. Inline footnote references are not modeled

- **Construct:** `[fn:1]` / `[fn::inline]` referenced mid-paragraph.
- **Expected (org-mode):** footnote-reference objects.
- **Observed (tree-sitter-org):** only line-start `[fn:N]` definitions get
  `fndef` nodes (fields `label`/`description`); inline references are
  `expr` soup.
- **Orgsidian behavior:** raw text; non-crash covered by
  `semantic_footnotes`.
- **Status/owner:** Epic 4 consumer decides.

## 6. CLOCK lines inside drawers are unstructured

- **Construct:** `CLOCK: [ts]` / `CLOCK: [ts]--[ts] => H:MM` inside
  `:LOGBOOK:`.
- **Expected (org-mode):** clock elements with timestamps and duration.
- **Observed (tree-sitter-org):** generic `drawer` contents are raw `expr`
  tokens — CLOCK lines have no structure.
- **Orgsidian behavior:** the semantic layer parses CLOCK lines textually
  from `:LOGBOOK:` contents (`src/semantic/drawer.rs`) into `ClockEntry`
  (start, optional end, optional duration, span). Malformed CLOCK lines are
  not errors — they stay raw drawer content (LD-41 posture).
- **Status/owner:** working as designed for v0.1; clock lines in custom
  (non-LOGBOOK) drawers are not scanned.

## 7. Timestamps in body paragraphs are not `timestamp` nodes

- **Construct:** `<2026-06-10 Wed>` standalone in body text.
- **Expected (org-mode):** timestamp objects anywhere in paragraph text.
- **Observed (tree-sitter-org):** `timestamp` nodes appear only in
  plan/entry context (the line right after a headline); body timestamps are
  `expr` soup. Also verified: the `plan` node covers only that first line —
  a `SCHEDULED:` keyword on any later line is plain paragraph text (this
  half matches org-mode, which also recognizes planning only on the line
  immediately following the headline).
- **Orgsidian behavior:** body timestamps stay raw text; only plan-position
  `SCHEDULED:`/`DEADLINE:`/`CLOSED:` route into `Headline`
  scheduled/deadline/closed.
- **Status/owner:** body-paragraph timestamp extraction deferred (tracked in
  deferred-work, story-2.3 stanza); agenda features (Epic 5+) will decide.

## 8. Vendored `scanner.c` signed-char list-indent bug

- **Construct:** deeply indented lists — cumulative indent ≥ 128 columns
  (≈64 nesting levels at 2-space steps).
- **Expected (org-mode):** parses fine; indentation depth is unbounded.
- **Observed (tree-sitter-org):** the external scanner serializes its
  list-indent stack through signed `char`
  (`grammar/src/scanner.c:75-101`); indent 128+ corrupts scanner state on
  serialize/deserialize and yields `ERROR` nodes on valid org (verified at
  the pinned SHA during the Story 2.2 review: indent 127 → clean tree,
  indent 128 → `has_error = true`).
- **Orgsidian behavior:** `parse()` returns `Ok` with `ERROR` nodes inside
  (documented wrapper posture); the semantic walker tolerates the ERROR
  region and degrades gracefully. Silent fidelity loss — the Story 2.6 L0
  round-trip gate will catch affected files.
- **Status/owner:** LD-48 forbids local grammar edits; owner is an upstream
  PR / the v0.3 fork-and-maintain dry run. Breadcrumb in deferred-work
  (story-2.2 stanza).

## 9. Directives bind to the following paragraph node

- **Construct:** `#+TITLE: …` (any `#+NAME: value` directive) immediately
  followed by a paragraph.
- **Expected (org-mode):** keywords are standalone elements; a following
  paragraph is a separate element.
- **Observed (tree-sitter-org):** a directive directly above a paragraph
  attaches to that paragraph node (field `directive`); standalone directives
  attach to the enclosing `body`. Either way the directive's own
  `name`/`value` fields are intact (verified during Story 2.3
  implementation).
- **Orgsidian behavior:** the semantic layer collects `directive` nodes
  position-independently over the whole tree, so `#+TODO:` configuration and
  preamble directives are unaffected by the attachment quirk. Paragraph
  spans include their attached directive lines.
- **Status/owner:** informational; matters only to consumers that reason
  about paragraph boundaries (Story 2.4 serializer round-trips from raw
  spans and is unaffected).
