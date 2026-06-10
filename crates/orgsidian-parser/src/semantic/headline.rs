//! `Headline` — the central semantic struct — and the section walker (FR-1).
//!
//! A grammar `section` node maps to one [`Headline`]: stars → level, the
//! first `item` token → TODO state (config-driven; the grammar does not
//! split the keyword out), the remaining item text → title, `tag_list` →
//! tags, `plan` entries → scheduled/deadline/closed, `property_drawer` →
//! properties, body `drawer` nodes → drawers (+ CLOCK entries from
//! `:LOGBOOK:`), and nested sections → children. The walker never panics:
//! malformed sub-constructs degrade to raw text or `None` (LD-41 posture).

use std::collections::HashMap;
use std::ops::Range;

use crate::tree_sitter::Node;

use super::drawer::{self, ClockEntry, Drawer, DrawerKind};
use super::link::{self, Link};
use super::timestamp::{self, Timestamp};
use super::todo::{TodoConfig, TodoState};

/// One headline tag (`:work:` → name `work`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Tag text without the surrounding colons.
    pub name: String,
    /// Byte range of the bare tag name in the `analyze()` input.
    pub span: Range<usize>,
}

/// One org headline with its section content and nested children.
///
/// Field shape per the Epic 2 contract (`todo_state`, `tags`, `scheduled`,
/// `deadline`, `properties`, …) plus the structural fields downstream
/// stories need (`level`, `title`, `span`, `children`).
#[derive(Debug, Clone, PartialEq)]
pub struct Headline {
    /// Heading depth = number of leading stars (1-based).
    pub level: u8,
    /// The recognized TODO keyword, when the first title word matches the
    /// document's [`TodoConfig`] (case-sensitive). An unconfigured first
    /// word stays in [`title`](Self::title).
    pub todo_state: Option<TodoState>,
    /// Headline text minus stars, TODO keyword, and trailing tag list.
    pub title: String,
    /// Trailing `:tag1:tag2:` tags, in order, names without colons.
    pub tags: Vec<Tag>,
    /// `SCHEDULED:` timestamp from the planning line, when present/parseable.
    pub scheduled: Option<Timestamp>,
    /// `DEADLINE:` timestamp from the planning line, when present/parseable.
    pub deadline: Option<Timestamp>,
    /// `CLOSED:` timestamp from the planning line, when present/parseable.
    pub closed: Option<Timestamp>,
    /// `:PROPERTIES:` drawer key/value pairs.
    ///
    /// **Caveats (epic-mandated shape, documented not "fixed"):** duplicate
    /// keys collapse last-wins, and iteration order is unspecified — the
    /// Story 2.4 serializer round-trips from raw spans, never by re-emitting
    /// this map.
    pub properties: HashMap<String, String>,
    /// All drawers in this headline's section (including the property
    /// drawer), classified by [`DrawerKind`].
    pub drawers: Vec<Drawer>,
    /// `CLOCK:` entries parsed from this headline's `:LOGBOOK:` drawer(s).
    pub clocks: Vec<ClockEntry>,
    /// Links found in this headline's own region (headline line + body,
    /// excluding child sections — children collect their own).
    pub links: Vec<Link>,
    /// Byte range of the headline's whole section (headline line through the
    /// end of its last child) in the `analyze()` input.
    pub span: Range<usize>,
    /// Nested subsections, in document order.
    pub children: Vec<Headline>,
}

/// Node text against the exact source passed to `parse()` — the documented
/// keep-the-source contract, kept internal to `analyze()` by the owned
/// extraction design. Defensive `""` on any (unexpected) invalid slice.
fn node_text<'s>(node: Node<'_>, source: &'s str) -> &'s str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

/// Build one [`Headline`] from a grammar `section` node. Returns `None` only
/// when the section carries no `headline` field (possible inside `ERROR`
/// regions — the walker must not assume well-formedness).
pub(crate) fn build_section(
    section: Node<'_>,
    source: &str,
    config: &TodoConfig,
) -> Option<Headline> {
    let headline_node = section.child_by_field_name("headline")?;

    // Level: number of stars.
    let level = headline_node
        .child_by_field_name("stars")
        .map(|stars| stars.byte_range().len())
        .map(|n| u8::try_from(n).unwrap_or(u8::MAX))
        .unwrap_or(0);

    // TODO state + title: the grammar leaves the keyword inside `item`; the
    // first `expr` token is matched (case-sensitive, exact) against the
    // resolved TodoConfig. On match it becomes `todo_state` and is excluded
    // from `title`; otherwise it is title text.
    let mut todo_state = None;
    let mut title = String::new();
    if let Some(item) = headline_node.child_by_field_name("item") {
        let mut title_start = item.start_byte();
        if let Some(first) = item.named_child(0) {
            let word = node_text(first, source);
            if let Some(done) = config.classify(word) {
                todo_state = Some(TodoState {
                    keyword: word.to_string(),
                    done,
                    span: first.byte_range(),
                });
                title_start = first.end_byte();
            }
        }
        title = source
            .get(title_start..item.end_byte())
            .unwrap_or("")
            .trim()
            .to_string();
    }

    // Tags: the one headline part the grammar fully structures.
    let mut tags = Vec::new();
    if let Some(tag_list) = headline_node.child_by_field_name("tags") {
        let mut cursor = tag_list.walk();
        for tag in tag_list.children_by_field_name("tag", &mut cursor) {
            tags.push(Tag {
                name: node_text(tag, source).trim_matches(':').to_string(),
                span: tag.byte_range(),
            });
        }
    }

    // Planning line: route entries by name (exact uppercase, org-style).
    let mut scheduled = None;
    let mut deadline = None;
    let mut closed = None;
    if let Some(plan) = section.child_by_field_name("plan") {
        let mut cursor = plan.walk();
        for entry in plan.named_children(&mut cursor) {
            if entry.kind() != "entry" {
                continue;
            }
            let Some(name_node) = entry.child_by_field_name("name") else {
                continue; // bare timestamp on the plan line — out of scope
            };
            let Some(ts_node) = entry.child_by_field_name("timestamp") else {
                continue;
            };
            // Active vs inactive comes from the delimiter byte — the grammar
            // parses both forms identically (verified at the pinned SHA).
            let Some(ts) = timestamp::parse_at(node_text(ts_node, source), ts_node.start_byte())
            else {
                continue; // unparseable values: skip the field, keep going
            };
            match node_text(name_node, source) {
                "SCHEDULED" => scheduled = scheduled.or(Some(ts)),
                "DEADLINE" => deadline = deadline.or(Some(ts)),
                "CLOSED" => closed = closed.or(Some(ts)),
                _ => {} // custom entry names — outside the semantic surface
            }
        }
    }

    // :PROPERTIES: — structured `property` children feed the map directly.
    let mut properties = HashMap::new();
    let mut drawers = Vec::new();
    if let Some(property_drawer) = section.child_by_field_name("property_drawer") {
        let mut first_prop = None;
        let mut last_prop = None;
        let mut cursor = property_drawer.walk();
        for prop in property_drawer.named_children(&mut cursor) {
            if prop.kind() != "property" {
                continue;
            }
            let Some(name_node) = prop.child_by_field_name("name") else {
                continue;
            };
            let name = node_text(name_node, source).to_string();
            if name.is_empty() {
                continue;
            }
            let value = prop
                .child_by_field_name("value")
                .map(|v| node_text(v, source).trim().to_string())
                .unwrap_or_default();
            properties.insert(name, value); // duplicate keys: last wins
            first_prop = first_prop.or(Some(prop.start_byte()));
            last_prop = Some(prop.end_byte());
        }
        let contents_span = match (first_prop, last_prop) {
            (Some(start), Some(end)) => start..end,
            _ => property_drawer.end_byte()..property_drawer.end_byte(),
        };
        drawers.push(Drawer {
            kind: DrawerKind::Properties,
            name: "PROPERTIES".to_string(),
            contents: source.get(contents_span.clone()).unwrap_or("").to_string(),
            span: property_drawer.byte_range(),
            contents_span,
        });
    }

    // Generic drawers in the body: classify by name; CLOCK lines are parsed
    // textually out of :LOGBOOK: contents (unstructured at the pinned SHA).
    let mut clocks = Vec::new();
    if let Some(body) = section.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            if child.kind() != "drawer" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .map(|n| node_text(n, source).to_string())
                .unwrap_or_default();
            let (contents, contents_span) = match child.child_by_field_name("contents") {
                Some(c) => (node_text(c, source).to_string(), c.byte_range()),
                None => (String::new(), child.end_byte()..child.end_byte()),
            };
            let kind = if name.eq_ignore_ascii_case("LOGBOOK") {
                DrawerKind::Logbook
            } else {
                DrawerKind::Custom(name.clone())
            };
            if kind == DrawerKind::Logbook {
                clocks.extend(drawer::parse_clock_lines(&contents, contents_span.start));
            }
            drawers.push(Drawer {
                kind,
                name,
                contents,
                span: child.byte_range(),
                contents_span,
            });
        }
    }

    // Links: inline scan over this headline's own region (headline line +
    // body, up to the first child section — children scan their own).
    let mut cursor = section.walk();
    let own_region_end = section
        .children_by_field_name("subsection", &mut cursor)
        .next()
        .map(|sub| sub.start_byte())
        .unwrap_or_else(|| section.end_byte());
    let links = source
        .get(section.start_byte()..own_region_end)
        .map(|text| link::scan_links(text, section.start_byte()))
        .unwrap_or_default();

    // Children: nested sections, recursively.
    let mut cursor = section.walk();
    let children = section
        .children_by_field_name("subsection", &mut cursor)
        .filter_map(|sub| build_section(sub, source, config))
        .collect();

    Some(Headline {
        level,
        todo_state,
        title,
        tags,
        scheduled,
        deadline,
        closed,
        properties,
        drawers,
        clocks,
        links,
        span: section.byte_range(),
        children,
    })
}
