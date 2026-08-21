//! Semantic layer: typed org semantics over the raw syntax tree.
//!
//! Implements FR-1.
//!
//! [`analyze`] parses org source (via [`crate::parse`]) and lifts the raw
//! `tree_sitter::Tree` into owned, typed semantic structs: [`Document`] →
//! [`Headline`] (TODO state, tags, planning timestamps, properties, drawers,
//! clock entries, links) — the stable AST that Story 2.4's serializer and
//! Epic 4's editor build against.
//!
//! Design contract (Story 2.3 source-retention decision): semantic structs
//! **own their data** — extracted `String`s plus `Range<usize>` byte spans
//! into the `analyze()` input. No lifetime parameters in the public surface;
//! the keep-the-source contract of [`crate::ParseTree`] stays internal to
//! [`analyze`]. Every struct that maps to a source region carries its span —
//! load-bearing for round-trip serialization and editor decorations.
//!
//! Lenience contract (LD-41): `analyze()` returns `Ok` for any input
//! `parse()` accepts (which is: everything). Malformed sub-constructs
//! degrade to raw text or `None` fields, never panics or errors.
//!
//! Known shape caveats (documented, not "fixed"): `Headline::properties` is
//! the epic-mandated `HashMap<String, String>` — duplicate keys collapse
//! last-wins and iteration order is unspecified; round-trip fidelity comes
//! from raw spans, never from re-emitting the map. Constructs the grammar
//! does not model (inline markup, inline LaTeX, citations, …) are *not*
//! semantically exposed — see `docs/parser/KNOWN_DIVERGENCES.md`.

mod drawer;
mod headline;
mod link;
mod timestamp;
mod todo;

pub use drawer::{ClockEntry, Drawer, DrawerKind};
pub use headline::{Headline, Tag};
pub use link::{Link, LinkKind};
pub use timestamp::{
    format_planning_timestamp, resolve_date_shortcut, set_planning_timestamp, Delay, DelayKind,
    PlannedStamp, PlanningEdit, PlanningKind, Repeater, RepeaterKind, TimeUnit, Timestamp,
};
pub use todo::{TodoConfig, TodoSequence, TodoState};

use std::ops::Range;

use crate::ParseError;

/// One `#+NAME: value` directive line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Directive {
    /// Directive name without the `#+`/`:` framing (e.g. `TITLE`, `TODO`).
    pub name: String,
    /// The directive's value text, trimmed.
    pub value: String,
    /// Byte range of the whole directive line in the `analyze()` input.
    pub span: Range<usize>,
}

/// Document-level content before the first headline (the zeroth section).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Preamble {
    /// Raw preamble text, exactly as written. On pathological input the
    /// region may extend over adjacent gap-absorbed bytes the grammar left
    /// outside any retained region (root-level `ERROR` nodes) — part of the
    /// Story 2.4 tiling invariant, see [`Headline::raw`].
    pub text: String,
    /// Byte range of [`text`](Self::text) in the `analyze()` input.
    pub span: Range<usize>,
    /// Links found in the preamble (same inline scan as headlines).
    pub links: Vec<Link>,
    /// Directives located in the preamble (e.g. `#+TITLE:`, `#+TODO:`).
    pub directives: Vec<Directive>,
}

/// The semantic view of one org document.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Document {
    /// Top-level headlines, in document order; nesting via
    /// [`Headline::children`].
    pub headlines: Vec<Headline>,
    /// The resolved TODO keyword configuration: in-file `#+TODO:` directives
    /// (accumulated) or the Orgsidian default sequence.
    pub todo_config: TodoConfig,
    /// Content before the first headline, when any.
    pub preamble: Option<Preamble>,
    /// Byte range of the full analyzed source (`0..source.len()`).
    pub span: Range<usize>,
}

/// Analyze org source into a semantic [`Document`].
///
/// Calls [`crate::parse`] internally — hand the source over once; all
/// returned data is owned and all spans index this `source` argument.
/// Returns `Err` only on [`crate::parse`]'s own defensive errors (which
/// never fire in practice); every parseable input — that is, every input —
/// analyzes to `Ok` (LD-41 lenient posture).
pub fn analyze(source: &str) -> Result<Document, ParseError> {
    let tree = crate::parse(source)?;
    let root = tree.root_node();

    // TODO config: `#+TODO:`-family directives anywhere in the file apply
    // file-wide (org semantics), so directives are collected over the whole
    // tree, in document order.
    let directives = collect_directives(root, source);
    let todo_values = directives
        .iter()
        .filter(|d| {
            d.name.eq_ignore_ascii_case("TODO")
                || d.name.eq_ignore_ascii_case("SEQ_TODO")
                || d.name.eq_ignore_ascii_case("TYP_TODO")
        })
        .map(|d| d.value.as_str());
    let todo_config = TodoConfig::from_directive_values(todo_values);

    // Top-level coverage cursor (Story 2.4 tiling invariant): preamble +
    // headline `raw` regions must tile the source 0..len byte-for-byte.
    // Bytes the grammar leaves outside retained regions (root-level `ERROR`
    // nodes, `section`s without a `headline` field) are gap-absorbed into
    // the nearest retained region — never dropped.
    let mut pos = 0usize;

    // Preamble: the document's zeroth-section `body` field, when present —
    // extended backward over any uncovered prefix (e.g. a root-level `ERROR`
    // node preceding the body).
    let mut preamble = root.child_by_field_name("body").map(|body| {
        let node_span = body.byte_range();
        let span = pos.min(node_span.start)..node_span.end;
        let text = source.get(span.clone()).unwrap_or("").to_string();
        let links = link::scan_links(&text, span.start);
        let directives = directives
            .iter()
            .filter(|d| d.span.start >= span.start && d.span.end <= span.end)
            .cloned()
            .collect();
        pos = span.end;
        Preamble {
            text,
            span,
            links,
            directives,
        }
    });

    // Top-level sections → headlines; nesting via `children`. Uncovered
    // bytes before a section are prepended to its `raw`.
    let mut headlines: Vec<Headline> = Vec::new();
    let mut cursor = root.walk();
    for section in root.children_by_field_name("subsection", &mut cursor) {
        let range = section.byte_range();
        if let Some(mut h) = headline::build_section(section, source, &todo_config) {
            if range.start > pos {
                h.raw
                    .insert_str(0, source.get(pos..range.start).unwrap_or(""));
            }
            pos = pos.max(range.end);
            headlines.push(h);
        }
    }

    // Trailing uncovered bytes: append after the last emitted region. When
    // nothing was retained at all (pathological all-`ERROR` input), the
    // whole source becomes the preamble — it *is* content before the first
    // headline, there being none.
    if pos < source.len() {
        let tail = source.get(pos..).unwrap_or("");
        if let Some(last) = headlines.last_mut() {
            headline::absorb_trailing(last, tail);
        } else if let Some(p) = preamble.as_mut() {
            p.text.push_str(tail);
            p.span.end = source.len();
        } else {
            let span = pos..source.len();
            preamble = Some(Preamble {
                text: tail.to_string(),
                links: link::scan_links(tail, span.start),
                directives: directives
                    .iter()
                    .filter(|d| d.span.start >= span.start && d.span.end <= span.end)
                    .cloned()
                    .collect(),
                span,
            });
        }
    }

    Ok(Document {
        headlines,
        todo_config,
        preamble,
        span: 0..source.len(),
    })
}

/// Collect every `directive` node in the tree, in document order, with an
/// iterative pre-order cursor walk (robust against `ERROR`-region nesting).
fn collect_directives(root: crate::tree_sitter::Node<'_>, source: &str) -> Vec<Directive> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    'walk: loop {
        let node = cursor.node();
        if node.kind() == "directive" {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                let value = node
                    .child_by_field_name("value")
                    .and_then(|v| v.utf8_text(source.as_bytes()).ok())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                out.push(Directive {
                    name,
                    value,
                    span: node.byte_range(),
                });
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                continue 'walk;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    out
}
