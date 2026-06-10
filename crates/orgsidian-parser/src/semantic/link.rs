//! Inline link scanning and classification (FR-1).
//!
//! The grammar has **no** `link` node at the pinned SHA — bracketed links
//! and bare URLs are token soup inside paragraphs. This module hand-rolls
//! the inline scan (deliberately: no regex dependency) over a source slice:
//! `[[target]]` / `[[target][description]]` spans plus a plain
//! `http(s)://…` scan, classified by target prefix.
//!
//! Edge posture (documented, not over-engineered — see
//! `docs/parser/KNOWN_DIVERGENCES.md`): `][` splits target/description,
//! `]]` terminates; an empty target (`[[]]`) is not a link; angle links
//! (`<http://…>`), link abbreviations, and `~/` expansion are out of scope.

use std::ops::Range;

/// Link classification by target prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// `[[id:abc]]` — org ID link.
    Id,
    /// `[[file://path]]` / `[[file:path]]` — file link.
    File,
    /// `[[http://…]]` / `[[https://…]]` — bracketed URL.
    Url,
    /// `[[target]]` with no recognized scheme — wiki-style internal link.
    Wiki,
    /// Bare in-text `http://…` / `https://…` run (not bracketed).
    Plain,
}

/// One link found by the inline scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Classification by target prefix (see [`LinkKind`]).
    pub kind: LinkKind,
    /// The raw target text, scheme prefix retained (`"id:abc"`,
    /// `"file://path"`, `"wiki page"`).
    pub target: String,
    /// The `[[target][description]]` description, when present.
    pub description: Option<String>,
    /// Byte range of the whole link (brackets included; for plain URLs the
    /// URL itself) in the `analyze()` input.
    pub span: Range<usize>,
}

/// Scan `text` for bracketed links and plain URLs. `offset` is the byte
/// position of `text[0]` in the original source. Bracketed links win over
/// plain-URL detection inside their own span (a URL used as a bracket target
/// or description is not double-reported).
pub(crate) fn scan_links(text: &str, offset: usize) -> Vec<Link> {
    let bytes = text.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' && bytes.get(i + 1) == Some(&b'[') {
            if let Some((link, end)) = parse_bracket_link(text, i, offset) {
                links.push(link);
                i = end;
                continue;
            }
        }
        if bytes[i..].starts_with(b"http://") || bytes[i..].starts_with(b"https://") {
            let (link, end) = parse_plain_url(text, i, offset);
            links.push(link);
            i = end;
            continue;
        }
        // Advance one byte: all scan anchors (`[[`, `http`) are ASCII, so a
        // mid-codepoint position can never match and slicing only happens at
        // match positions (always char boundaries).
        i += 1;
    }
    links
}

/// Parse `[[…]]` starting at byte `start` (which holds `[[`). Returns the
/// link and the byte index just past the closing `]]`, or `None` when the
/// link never terminates or has an empty target.
fn parse_bracket_link(text: &str, start: usize, offset: usize) -> Option<(Link, usize)> {
    let inner_start = start + 2;
    let rest = &text[inner_start..];

    // Find the first `][` (target/description split) or `]]` (terminator),
    // whichever comes first — the simplest correct reading of org brackets.
    let mut target_end = None;
    let mut split = None;
    let rest_bytes = rest.as_bytes();
    for (pos, window) in rest_bytes.windows(2).enumerate() {
        match window {
            b"]]" => {
                target_end = Some(pos);
                break;
            }
            b"][" => {
                split = Some(pos);
                break;
            }
            _ => {}
        }
    }

    let (target, description, inner_len) = match (split, target_end) {
        (Some(split), _) => {
            let desc_rest = &rest[split + 2..];
            let desc_end = desc_rest.find("]]")?;
            (
                &rest[..split],
                Some(desc_rest[..desc_end].to_string()),
                split + 2 + desc_end,
            )
        }
        (None, Some(end)) => (&rest[..end], None, end),
        (None, None) => return None,
    };
    if target.is_empty() {
        return None;
    }

    let end = inner_start + inner_len + 2;
    let kind = classify_target(target);
    let link = Link {
        kind,
        target: target.to_string(),
        description,
        span: offset + start..offset + end,
    };
    Some((link, end))
}

/// Classify a bracket target by scheme prefix.
fn classify_target(target: &str) -> LinkKind {
    if target.starts_with("id:") {
        LinkKind::Id
    } else if target.starts_with("file:") {
        LinkKind::File
    } else if target.starts_with("http://") || target.starts_with("https://") {
        LinkKind::Url
    } else {
        LinkKind::Wiki
    }
}

/// Parse a bare URL starting at byte `start` (which holds `http`). The URL
/// runs to the first whitespace or bracket-ish delimiter; trailing sentence
/// punctuation is trimmed (simple posture, documented).
fn parse_plain_url(text: &str, start: usize, offset: usize) -> (Link, usize) {
    let rest = &text[start..];
    let len = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '<' | '>'))
        .unwrap_or(rest.len());
    let mut url = &rest[..len];
    while let Some(stripped) = url.strip_suffix(['.', ',', ';', ':', '!', '?', ')', '\'', '"']) {
        url = stripped;
    }
    let end = start + url.len();
    let link = Link {
        kind: LinkKind::Plain,
        target: url.to_string(),
        description: None,
        span: offset + start..offset + end,
    };
    (link, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_all_bracket_forms() {
        let text = "[[id:abc]] [[file:notes.org]] [[https://x.dev][X]] [[wiki page]]";
        let links = scan_links(text, 0);
        assert_eq!(links.len(), 4);
        assert_eq!(links[0].kind, LinkKind::Id);
        assert_eq!(links[1].kind, LinkKind::File);
        assert_eq!(links[2].kind, LinkKind::Url);
        assert_eq!(links[2].description.as_deref(), Some("X"));
        assert_eq!(links[3].kind, LinkKind::Wiki);
        assert_eq!(links[3].target, "wiki page");
    }

    #[test]
    fn bracket_url_is_not_double_reported_as_plain() {
        let links = scan_links("[[https://x.dev][see https://x.dev]]", 0);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::Url);
    }

    #[test]
    fn plain_url_trims_trailing_punctuation() {
        let links = scan_links("see https://example.com/a, then http://b.io.", 10);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "https://example.com/a");
        assert_eq!(links[1].target, "http://b.io");
        assert_eq!(links[1].span, 10 + 32..10 + 43);
    }

    #[test]
    fn degenerate_brackets_are_not_links() {
        assert!(scan_links("[[]]", 0).is_empty(), "empty target");
        assert!(scan_links("[[never closed", 0).is_empty(), "unterminated");
        assert!(
            scan_links("[fn:1] [cite:@k] [x]", 0).is_empty(),
            "single brackets"
        );
        // `][` without a terminating `]]` is not a link either.
        assert!(scan_links("[[a][b", 0).is_empty());
    }

    #[test]
    fn multibyte_text_around_links_is_safe() {
        let text = "приветствие [[id:é]] café http://x.io/ñ done";
        let links = scan_links(text, 0);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "id:é");
        assert_eq!(links[1].target, "http://x.io/ñ");
    }
}
