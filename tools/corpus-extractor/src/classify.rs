//! LD-44 construct detection over org text (AC3).
//!
//! Heuristic classification: syntax shapes are borrowed from the parser's
//! `tests/semantic.rs` samples (Story 2.3 AC6) — no novel org syntax is
//! invented here. The SAME classifier feeds selection (`select.rs`) and
//! validation (`validate.rs`), so the subset matrix is internally consistent
//! by construction; the meta-test re-runs it against the committed artifact.

use crate::model::Construct;
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::BTreeSet;

/// Compiled construct detectors. Build once (fallible — regex compilation),
/// classify many. No panics in lib code: callers own the `?`.
pub struct Classifier {
    headline: Regex,
    todo_directive: Regex,
    scheduled: Regex,
    deadline: Regex,
    clock: Regex,
    recurring: Regex,
    drawer_line: Regex,
    markup: Vec<Regex>,
    bracket_link: Regex,
    plain_url: Regex,
    list_item: Regex,
    table_row: Regex,
    block: Regex,
    inline_latex: Regex,
    footnote: Regex,
    citation: Regex,
    tagged_headline: Regex,
}

/// Default org TODO keywords (org-todo-keywords default + the LD-44 row's
/// explicit `NEXT` / `WAITING`).
const DEFAULT_TODO_KEYWORDS: [&str; 4] = ["TODO", "NEXT", "DONE", "WAITING"];

impl Classifier {
    pub fn new() -> Result<Self> {
        let compile = |pattern: &str| -> Result<Regex> {
            Regex::new(pattern).with_context(|| format!("compiling classifier regex {pattern}"))
        };
        // Inline markup: marker-delimited run on one line, borders non-space,
        // org-style PRE/POST context. A post-match check requires at least one
        // alphanumeric inside to dodge table separators like `|---+---|`.
        let markup = ['*', '/', '=', '~', '+', '_']
            .iter()
            .map(|marker| {
                let m = regex::escape(&marker.to_string());
                compile(&format!(
                    r#"(?m)(?:^|[\s({{'"]){m}([^\s{m}\n](?:[^{m}\n]*[^\s{m}\n])?){m}(?:[\s)}}'".,;:!?]|$)"#
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            headline: compile(r"(?m)^\*{1,6} +(\S+)")?,
            todo_directive: compile(r"(?mi)^#\+(?:TODO|SEQ_TODO|TYP_TODO): *(.+)$")?,
            scheduled: compile(r"SCHEDULED: *[<\[]")?,
            deadline: compile(r"DEADLINE: *[<\[]")?,
            clock: compile(r"(?m)^[ \t]*CLOCK: *\[")?,
            recurring: compile(r"[<\[][^<>\[\]\n]*[.+]?\+\d+[hdwmy][^<>\[\]\n]*[>\]]")?,
            drawer_line: compile(r"(?m)^[ \t]*:([A-Za-z0-9_@#%-]+):[ \t]*$")?,
            markup,
            bracket_link: compile(r"\[\[[^\[\]\n]+\]")?,
            plain_url: compile(r"https?://[^\s\]>]+")?,
            list_item: compile(r"(?m)^[ \t]*(?:[-+] |\d+[.)][ \t])")?,
            table_row: compile(r"(?m)^[ \t]*\|")?,
            block: compile(r"(?mi)^[ \t]*#\+BEGIN_(?:SRC|QUOTE|EXAMPLE|VERSE)\b")?,
            inline_latex: compile(r"\$[^$\n]+\$|\\\(|\\\[")?,
            footnote: compile(r"\[fn:")?,
            citation: compile(r"\[cite[:/]")?,
            tagged_headline: compile(
                r"(?m)^\*{1,6} [^\n]*[ \t]:[A-Za-z0-9_@#%]+(?::[A-Za-z0-9_@#%]+)*:[ \t]*$",
            )?,
        })
    }

    /// Detect the LD-44 constructs present in `content`. Line endings are
    /// normalized for *detection only* (the corpus bytes keep their CRLF —
    /// classification must see the same constructs before and after the
    /// unusual-EOL edge transforms).
    pub fn classify(&self, content: &str) -> BTreeSet<Construct> {
        let normalized: std::borrow::Cow<'_, str> = if content.contains('\r') {
            std::borrow::Cow::Owned(content.replace("\r\n", "\n").replace('\r', "\n"))
        } else {
            std::borrow::Cow::Borrowed(content)
        };
        let text = normalized.as_ref();
        let mut out = BTreeSet::new();

        if self.has_todo_headline(text) {
            out.insert(Construct::HeadingTodo);
        }
        if self.scheduled.is_match(text) {
            out.insert(Construct::Scheduled);
        }
        if self.deadline.is_match(text) {
            out.insert(Construct::Deadline);
        }
        if self.clock.is_match(text) {
            out.insert(Construct::Clock);
        }
        if self.recurring.is_match(text) {
            out.insert(Construct::RecurringTimestamp);
        }
        if self.has_drawer(text) {
            out.insert(Construct::Drawer);
        }
        if self.has_inline_markup(text) {
            out.insert(Construct::InlineMarkup);
        }
        if self.bracket_link.is_match(text) || self.plain_url.is_match(text) {
            out.insert(Construct::Link);
        }
        if self.list_item.is_match(text) {
            out.insert(Construct::List);
        }
        if self.table_row.is_match(text) {
            out.insert(Construct::Table);
        }
        if self.block.is_match(text) {
            out.insert(Construct::Block);
        }
        if self.inline_latex.is_match(text) {
            out.insert(Construct::InlineLatex);
        }
        if self.footnote.is_match(text) {
            out.insert(Construct::Footnote);
        }
        if self.citation.is_match(text) {
            out.insert(Construct::Citation);
        }
        if self.tagged_headline.is_match(text) {
            out.insert(Construct::Tag);
        }
        out
    }

    /// A headline whose first word is a TODO keyword — default set plus any
    /// keywords declared by `#+TODO:`-family directives in the same text
    /// (union for classification purposes; replacement semantics belong to
    /// the parser, not this coverage heuristic — ADR 0001).
    fn has_todo_headline(&self, text: &str) -> bool {
        let mut keywords: BTreeSet<String> = DEFAULT_TODO_KEYWORDS
            .iter()
            .map(|k| (*k).to_string())
            .collect();
        for caps in self.todo_directive.captures_iter(text) {
            if let Some(spec) = caps.get(1) {
                for word in spec.as_str().split_whitespace() {
                    if word == "|" {
                        continue;
                    }
                    // Strip org fast-access suffixes like `DONE(d)`.
                    let bare = word.split('(').next().unwrap_or(word);
                    if !bare.is_empty()
                        && bare.chars().all(|c| {
                            c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-' || c == '_'
                        })
                    {
                        keywords.insert(bare.to_string());
                    }
                }
            }
        }
        self.headline
            .captures_iter(text)
            .filter_map(|caps| caps.get(1))
            .any(|word| keywords.contains(word.as_str()))
    }

    /// A drawer requires an opening `:NAME:` line (non-END) and an `:END:` line.
    fn has_drawer(&self, text: &str) -> bool {
        let mut has_open = false;
        let mut has_end = false;
        for caps in self.drawer_line.captures_iter(text) {
            if let Some(name) = caps.get(1) {
                if name.as_str().eq_ignore_ascii_case("END") {
                    has_end = true;
                } else {
                    has_open = true;
                }
            }
        }
        has_open && has_end
    }

    fn has_inline_markup(&self, text: &str) -> bool {
        self.markup.iter().any(|re| {
            re.captures_iter(text)
                .filter_map(|caps| caps.get(1))
                .any(|inner| inner.as_str().chars().any(|c| c.is_ascii_alphanumeric()))
        })
    }
}

/// True when `s` carries Unicode/RTL content per the LD-44 edge-bucket reading
/// recorded in the story (Arabic, Hebrew, CJK ideographs, kana).
pub fn has_unicode_rtl(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
            '\u{0590}'..='\u{05FF}' // Hebrew
            | '\u{0600}'..='\u{06FF}' // Arabic
            | '\u{3040}'..='\u{30FF}' // Hiragana + Katakana
            | '\u{4E00}'..='\u{9FFF}' // CJK unified ideographs
        )
    })
}

/// True when `s` uses CRLF or mixed line endings (LD-44 unusual-EOL bucket).
pub fn has_unusual_eol(s: &str) -> bool {
    s.contains('\r')
}

/// True when `s` carries the malformed-but-valid markers this corpus
/// synthesizes (LD-44 rule 3): drawer-ish lines over-indented by ≥8 columns,
/// or trailing whitespace on a headline line. Heuristic by design — the same
/// detector runs at selection AND validation time, so the committed manifest
/// stays self-verifying (TC-3).
pub fn has_malformed_valid_marks(s: &str) -> bool {
    let normalized = if s.contains('\r') {
        std::borrow::Cow::Owned(s.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(s)
    };
    normalized.lines().any(|line| {
        let overindented_drawer =
            line.len() >= 8 && line.starts_with("        ") && is_drawer_shape(line.trim_start());
        let trailing_ws_headline = line.starts_with('*')
            && line.trim_start_matches('*').starts_with(' ')
            && line.ends_with([' ', '\t']);
        overindented_drawer || trailing_ws_headline
    })
}

/// `:NAME:`-shaped line body (drawer delimiter or property key line).
fn is_drawer_shape(body: &str) -> bool {
    let Some(rest) = body.strip_prefix(':') else {
        return false;
    };
    let Some(colon) = rest.find(':') else {
        return false;
    };
    colon > 0
        && rest[..colon]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '@' | '#' | '%' | '-'))
}

/// Detect every LD-44 edge bucket `content` belongs to (sorted, stable).
pub fn detect_edges(content: &str) -> Vec<crate::model::EdgeBucket> {
    use crate::model::EdgeBucket;
    let mut out = Vec::new();
    if has_unicode_rtl(content) {
        out.push(EdgeBucket::UnicodeRtl);
    }
    if has_unusual_eol(content) {
        out.push(EdgeBucket::UnusualEol);
    }
    if has_malformed_valid_marks(content) {
        out.push(EdgeBucket::MalformedValid);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Construct as C;

    fn classifier() -> Classifier {
        Classifier::new().expect("static patterns compile")
    }

    // One positive case per LD-44 construct — shapes from tests/semantic.rs.

    #[test]
    fn detects_heading_todo_default_keywords() {
        let got = classifier().classify("* TODO One\n** NEXT Two\n*** DONE Three\n");
        assert!(got.contains(&C::HeadingTodo), "{got:?}");
    }

    #[test]
    fn detects_heading_todo_custom_directive_keywords() {
        let got = classifier().classify("#+TODO: DRAFT | PUBLISHED\n* DRAFT Post\n");
        assert!(got.contains(&C::HeadingTodo), "{got:?}");
        // Without the directive, DRAFT is just title text.
        let got = classifier().classify("* DRAFT Post\n");
        assert!(!got.contains(&C::HeadingTodo), "{got:?}");
    }

    #[test]
    fn detects_scheduled() {
        let got = classifier().classify("* Active\nSCHEDULED: <2026-06-10 Wed 10:00>\n");
        assert!(got.contains(&C::Scheduled), "{got:?}");
        let got = classifier().classify("* Inactive\nSCHEDULED: [2026-06-11 Thu]\n");
        assert!(got.contains(&C::Scheduled), "{got:?}");
    }

    #[test]
    fn detects_deadline() {
        let got = classifier().classify("* H\nDEADLINE: <2026-06-12 Fri 09:30>\n");
        assert!(got.contains(&C::Deadline), "{got:?}");
        assert!(!got.contains(&C::Scheduled), "{got:?}");
    }

    #[test]
    fn detects_clock_entries() {
        let got = classifier().classify(
            "* Clocked\n:LOGBOOK:\nCLOCK: [2026-06-09 Tue 10:00]--[2026-06-09 Tue 11:30] =>  1:30\n:END:\n",
        );
        assert!(got.contains(&C::Clock), "{got:?}");
    }

    #[test]
    fn detects_recurring_repeaters() {
        for stamp in [
            "<2026-06-10 Wed +1w>",
            "<2026-06-10 Wed ++1m>",
            "<2026-06-10 Wed .+1y>",
        ] {
            let text = format!("* H\nSCHEDULED: {stamp}\n");
            let got = classifier().classify(&text);
            assert!(got.contains(&C::RecurringTimestamp), "{stamp}: {got:?}");
        }
        let got = classifier().classify("* H\nSCHEDULED: <2026-06-10 Wed>\n");
        assert!(!got.contains(&C::RecurringTimestamp), "{got:?}");
    }

    #[test]
    fn detects_drawers() {
        let got = classifier().classify("* H\n:PROPERTIES:\n:ID: abc\n:END:\n");
        assert!(got.contains(&C::Drawer), "{got:?}");
        let got = classifier().classify("* H\n:MYDRAWER:\nfree text\n:END:\n");
        assert!(got.contains(&C::Drawer), "{got:?}");
        // :END: alone (or none) is not a drawer.
        let got = classifier().classify("* H\n:END:\n");
        assert!(!got.contains(&C::Drawer), "{got:?}");
    }

    #[test]
    fn detects_inline_markup_all_six() {
        for sample in [
            "*bold*",
            "/italic/",
            "=verbatim=",
            "~code~",
            "+strike+",
            "_underline_",
        ] {
            let text = format!("Text with {sample} inside.\n");
            let got = classifier().classify(&text);
            assert!(got.contains(&C::InlineMarkup), "{sample}: {got:?}");
        }
    }

    #[test]
    fn table_separator_is_not_markup() {
        let got = classifier().classify("| a | b |\n|---+---|\n| 1 | 2 |\n");
        assert!(!got.contains(&C::InlineMarkup), "{got:?}");
        assert!(got.contains(&C::Table), "{got:?}");
    }

    #[test]
    fn detects_links() {
        for sample in [
            "[[id:abc]]",
            "[[wiki page][the docs]]",
            "[[file://notes/x.org]]",
            "plain http://example.com here",
        ] {
            let text = format!("See {sample}.\n");
            let got = classifier().classify(&text);
            assert!(got.contains(&C::Link), "{sample}: {got:?}");
        }
        // Footnote/citation brackets are not links.
        let got = classifier().classify("Text[fn:1] and [cite:@k].\n");
        assert!(!got.contains(&C::Link), "{got:?}");
    }

    #[test]
    fn detects_lists() {
        let got = classifier().classify("- one\n+ two\n1. three\n- [ ] open\n- [X] done\n");
        assert!(got.contains(&C::List), "{got:?}");
        // A headline is not a list item.
        let got = classifier().classify("* Heading only\n");
        assert!(!got.contains(&C::List), "{got:?}");
    }

    #[test]
    fn detects_tables() {
        let got = classifier().classify("| a | b |\n|---+---|\n| 1 | 2 |\n#+TBLFM: $2=$1*2\n");
        assert!(got.contains(&C::Table), "{got:?}");
    }

    #[test]
    fn detects_blocks() {
        for kind in ["SRC rust", "QUOTE", "EXAMPLE", "VERSE"] {
            let text = format!(
                "#+BEGIN_{kind}\nx\n#+END_{}\n",
                kind.split(' ').next().unwrap()
            );
            let got = classifier().classify(&text);
            assert!(got.contains(&C::Block), "{kind}: {got:?}");
        }
    }

    #[test]
    fn detects_inline_latex() {
        for sample in ["$x^2$", "\\(a+b\\)", "\\[c\\]"] {
            let text = format!("Inline {sample} math.\n");
            let got = classifier().classify(&text);
            assert!(got.contains(&C::InlineLatex), "{sample}: {got:?}");
        }
    }

    #[test]
    fn detects_footnotes() {
        let got = classifier().classify("Text[fn:1] and inline[fn::note].\n\n[fn:1] Def.\n");
        assert!(got.contains(&C::Footnote), "{got:?}");
    }

    #[test]
    fn detects_citations() {
        let got = classifier().classify("Claim [cite:@key2026].\n");
        assert!(got.contains(&C::Citation), "{got:?}");
        let got = classifier().classify("Styled [cite/text:@key].\n");
        assert!(got.contains(&C::Citation), "{got:?}");
    }

    #[test]
    fn detects_tags() {
        let got = classifier().classify("* One :tag:\n* Two :tag1:tag2:\n");
        assert!(got.contains(&C::Tag), "{got:?}");
        let got = classifier().classify("* Not a tag: trailing colon text\n");
        assert!(!got.contains(&C::Tag), "{got:?}");
    }

    #[test]
    fn classification_is_eol_insensitive() {
        let lf = "* TODO Task\nSCHEDULED: <2026-06-10 Wed>\n- item\n";
        let crlf = lf.replace('\n', "\r\n");
        let c = classifier();
        assert_eq!(c.classify(lf), c.classify(&crlf));
    }

    #[test]
    fn unusual_eol_detection() {
        assert!(has_unusual_eol("* H\r\nbody\r\n"));
        assert!(has_unusual_eol("a\nb\r\nc\n")); // mixed
        assert!(!has_unusual_eol("* H\nbody\n"));
    }

    #[test]
    fn malformed_valid_detection() {
        // Over-indented drawer lines (≥8 columns) — interim-corpus shape 14.
        assert!(has_malformed_valid_marks(
            "* H\n          :PROPERTIES:\n          :ID: x\n          :END:\n"
        ));
        // Trailing whitespace on a headline — interim-corpus shape 15.
        assert!(has_malformed_valid_marks("* Headline   \nbody\n"));
        // CRLF text with trailing-ws headline still detected after normalization.
        assert!(has_malformed_valid_marks("* Headline   \r\nbody\r\n"));
        // Clean org — including a normally-indented drawer — is NOT malformed.
        assert!(!has_malformed_valid_marks(
            "* H\n  :PROPERTIES:\n  :ID: x\n  :END:\nbody\n"
        ));
        // Headline without trailing ws, bold-start body line: not malformed.
        assert!(!has_malformed_valid_marks("* H\n*bold* text\n"));
    }

    #[test]
    fn detect_edges_is_sorted_and_complete() {
        use crate::model::EdgeBucket as E;
        let all = "مرحبا   \r\n          :PROPERTIES:\r\n          :END:\r\n* H   \r\n";
        assert_eq!(
            detect_edges(all),
            vec![E::UnicodeRtl, E::UnusualEol, E::MalformedValid]
        );
        assert!(detect_edges("plain\n").is_empty());
    }

    #[test]
    fn unicode_rtl_detection() {
        assert!(has_unicode_rtl("مرحبا"));
        assert!(has_unicode_rtl("שלום"));
        assert!(has_unicode_rtl("こんにちは"));
        assert!(has_unicode_rtl("漢字"));
        assert!(!has_unicode_rtl("plain ascii — café")); // Latin-1 accents are not the RTL/CJK bucket
    }
}
