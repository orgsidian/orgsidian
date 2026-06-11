//! Human-readable AST rendering for `orgsidian parse` (Story 2.8).
//!
//! CLI-side presentation ONLY — rendering never lives in the parser. The
//! output is best-effort presentation, explicitly NOT a stability contract
//! (`--json` is the scripting surface). It is, however, deterministic by
//! contract: the one unordered source (`Headline::properties`, a `HashMap`)
//! is sorted by key before printing; everything else renders in document
//! order from `Vec`s.

use orgsidian_core::parser::semantic::{Document, Headline};

/// Render the whole document as an indented headline tree, preamble summary
/// first. Returns the text WITHOUT a trailing newline (the caller's
/// `println!` adds it).
pub(crate) fn render_document(document: &Document) -> String {
    let mut lines = Vec::new();
    if let Some(preamble) = &document.preamble {
        lines.push(format!("preamble ({} bytes)", preamble.text.len()));
        for directive in &preamble.directives {
            lines.push(format!(
                "  directive #+{}: {}",
                directive.name, directive.value
            ));
        }
        for link in &preamble.links {
            lines.push(format!("  link: {}", link.target));
        }
    }
    if document.headlines.is_empty() {
        lines.push("(no headlines)".to_string());
    } else {
        for headline in &document.headlines {
            render_headline(headline, &mut lines);
        }
    }
    lines.join("\n")
}

/// Render one headline (org-style star prefix, TODO keyword, title, tags),
/// its detail lines (planning timestamps, sorted properties, drawers, clock
/// count, links), then its children, depth-first in document order.
fn render_headline(headline: &Headline, lines: &mut Vec<String>) {
    // `level` 0 is a degenerate ERROR-region sentinel — render at least one
    // star so the line stays recognizable.
    let depth = usize::from(headline.level.max(1));
    let mut line = "*".repeat(depth);
    if let Some(todo) = &headline.todo_state {
        line.push(' ');
        line.push_str(&todo.keyword);
    }
    if !headline.title.is_empty() {
        line.push(' ');
        line.push_str(&headline.title);
    }
    if !headline.tags.is_empty() {
        line.push_str(" :");
        for tag in &headline.tags {
            line.push_str(&tag.name);
            line.push(':');
        }
    }
    lines.push(line);

    let indent = "  ".repeat(depth);
    for (label, stamp) in [
        ("SCHEDULED", &headline.scheduled),
        ("DEADLINE", &headline.deadline),
        ("CLOSED", &headline.closed),
    ] {
        if let Some(timestamp) = stamp {
            lines.push(format!("{indent}{label}: {}", timestamp.raw));
        }
    }
    // Determinism: `properties` is a HashMap — ALWAYS sort before printing.
    let mut properties: Vec<_> = headline.properties.iter().collect();
    properties.sort();
    for (key, value) in properties {
        lines.push(format!("{indent}property {key} = {value}"));
    }
    for drawer in &headline.drawers {
        lines.push(format!(
            "{indent}drawer :{}: ({} bytes)",
            drawer.name,
            drawer.contents.len()
        ));
    }
    if !headline.clocks.is_empty() {
        lines.push(format!("{indent}clocks: {}", headline.clocks.len()));
    }
    for link in &headline.links {
        lines.push(format!("{indent}link: {}", link.target));
    }
    for child in &headline.children {
        render_headline(child, lines);
    }
}
