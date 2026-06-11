//! Pragmatic assertion scanner over `test-org-element.el` (AC3).
//!
//! Finds `(ert-deftest test-org-element/… )` blocks and harvests the org-text
//! string literals passed to `org-test-with-temp-text` /
//! `org-test-with-temp-text-in-file`. This is a line/state scanner with an
//! elisp *string lexer* — explicitly NOT a full elisp reader (mandated scope,
//! Dev Notes §3). Known unharvested forms (documented in ADR 0001):
//! snippets built via `concat` / `format` / variables, i.e. any call whose
//! first argument is not a plain string literal.

/// One harvested org-text assertion, attributed to its `ert-deftest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Harvested {
    /// Full deftest name, e.g. `test-org-element/headline-parser`.
    pub deftest: String,
    /// 0-based occurrence index within the deftest (one deftest often carries
    /// several `org-test-with-temp-text` assertions).
    pub index: usize,
    /// Decoded org text: elisp escapes resolved, `<point>` markers stripped.
    pub content: String,
}

const DEFTEST_OPEN: &str = "(ert-deftest ";
const TEMP_TEXT: &str = "org-test-with-temp-text";
const IN_FILE_SUFFIX: &str = "-in-file";
/// The org-test caret convention — must be stripped or it pollutes the corpus
/// with literal `<point>` text (AC3).
const POINT_MARKER: &str = "<point>";

/// Scan the full `.el` source and harvest every string-literal assertion in
/// file order. Deterministic: output order is occurrence order.
pub fn harvest(source: &str) -> Vec<Harvested> {
    // Pass 1: deftest positions + names, in file order.
    let deftests = scan_deftests(source);

    // Pass 2: org-test-with-temp-text occurrences with literal string args.
    let mut out = Vec::new();
    let mut per_deftest_counter: Option<(usize, usize)> = None; // (deftest idx, next index)
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(TEMP_TEXT) {
        let token_start = search_from + rel;
        let mut after = token_start + TEMP_TEXT.len();
        if source[after..].starts_with(IN_FILE_SUFFIX) {
            after += IN_FILE_SUFFIX.len();
        }
        search_from = after;

        // Token boundary: a longer identifier (future macro variant) is skipped.
        if source[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            continue;
        }

        // The harvestable form has a plain string literal as first argument.
        let arg_start = after + leading_whitespace_len(&source[after..]);
        if !source[arg_start..].starts_with('"') {
            continue; // non-literal argument (concat/format/variable) — documented limit
        }
        let Some((decoded, _end)) = lex_elisp_string(source, arg_start) else {
            continue; // unterminated string — malformed source, skip defensively
        };
        let content = decoded.replace(POINT_MARKER, "");

        // Attribute to the nearest preceding deftest.
        let Some(deftest_idx) = nearest_deftest(&deftests, token_start) else {
            continue; // outside any deftest (e.g. helper macros) — skip
        };
        let index = match per_deftest_counter {
            Some((idx, next)) if idx == deftest_idx => next,
            _ => 0,
        };
        per_deftest_counter = Some((deftest_idx, index + 1));

        out.push(Harvested {
            deftest: deftests[deftest_idx].1.clone(),
            index,
            content,
        });
    }
    out
}

/// All `(ert-deftest NAME` occurrences as `(position, name)`, file order.
fn scan_deftests(source: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(DEFTEST_OPEN) {
        let pos = search_from + rel;
        let name_start = pos + DEFTEST_OPEN.len();
        let name: String = source[name_start..]
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '(' && *c != ')')
            .collect();
        if !name.is_empty() {
            out.push((pos, name));
        }
        search_from = name_start;
    }
    out
}

/// Index of the last deftest starting before `pos`, if any.
fn nearest_deftest(deftests: &[(usize, String)], pos: usize) -> Option<usize> {
    match deftests.partition_point(|(p, _)| *p < pos) {
        0 => None,
        n => Some(n - 1),
    }
}

fn leading_whitespace_len(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// Lex one elisp string literal starting at the opening quote (`start` must
/// index a `"`). Returns the decoded content and the byte index just past the
/// closing quote. Escape handling per the mandated pragmatic set: `\"`, `\\`,
/// `\n`, `\t` (+ `\r` for completeness); any other escape keeps the escaped
/// character verbatim (documented limit — `\u`/`\x` codepoint escapes are not
/// decoded; ADR 0001).
fn lex_elisp_string(source: &str, start: usize) -> Option<(String, usize)> {
    debug_assert!(source[start..].starts_with('"'));
    let mut out = String::new();
    let mut chars = source[start + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, start + 1 + i + 1)),
            '\\' => {
                let (_, escaped) = chars.next()?;
                match escaped {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    // `\"` and `\\` push the literal char; unknown escapes
                    // degrade to the escaped char itself (pragmatic scanner).
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    None // unterminated
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn harvests_simple_literal_with_deftest_attribution() {
        let el = r#"
(ert-deftest test-org-element/headline ()
  "Test headline parsing."
  (should (org-test-with-temp-text "* Headline"
            (org-element-type (org-element-at-point)))))
"#;
        let got = harvest(el);
        assert_eq!(
            got,
            vec![Harvested {
                deftest: "test-org-element/headline".to_string(),
                index: 0,
                content: "* Headline".to_string(),
            }]
        );
    }

    #[test]
    fn decodes_escapes_and_strips_point_marker() {
        let el = "(ert-deftest test-org-element/esc ()\n  (org-test-with-temp-text \"* H\\n<point>Quoted \\\"text\\\" tab\\there \\\\(rx\\\\)\"))\n";
        let got = harvest(el);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content, "* H\nQuoted \"text\" tab\there \\(rx\\)");
    }

    #[test]
    fn strips_every_point_marker_occurrence() {
        let el = "(ert-deftest test-org-element/p ()\n  (org-test-with-temp-text \"a<point>b<point>c\"))\n";
        assert_eq!(harvest(el)[0].content, "abc");
    }

    #[test]
    fn handles_multi_line_snippet() {
        let el = "(ert-deftest test-org-element/multi ()\n  (org-test-with-temp-text\n      \"* TODO Task\\nSCHEDULED: <2026-06-10 Wed>\\n:PROPERTIES:\\n:ID: x\\n:END:\\n\"))\n";
        let got = harvest(el);
        assert_eq!(
            got[0].content,
            "* TODO Task\nSCHEDULED: <2026-06-10 Wed>\n:PROPERTIES:\n:ID: x\n:END:\n"
        );
    }

    #[test]
    fn harvests_in_file_variant_and_indexes_per_deftest() {
        let el = r#"
(ert-deftest test-org-element/two ()
  (org-test-with-temp-text "first")
  (org-test-with-temp-text-in-file "second"))
(ert-deftest test-org-element/other ()
  (org-test-with-temp-text "third"))
"#;
        let got = harvest(el);
        let summary: Vec<(&str, usize, &str)> = got
            .iter()
            .map(|h| (h.deftest.as_str(), h.index, h.content.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("test-org-element/two", 0, "first"),
                ("test-org-element/two", 1, "second"),
                ("test-org-element/other", 0, "third"),
            ]
        );
    }

    #[test]
    fn skips_non_literal_arguments() {
        let el = r#"
(ert-deftest test-org-element/nonlit ()
  (org-test-with-temp-text (concat "a" "b"))
  (org-test-with-temp-text some-variable)
  (org-test-with-temp-text "literal"))
"#;
        let got = harvest(el);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content, "literal");
    }

    #[test]
    fn skips_occurrences_outside_any_deftest() {
        let el = "(defmacro my-helper () (org-test-with-temp-text \"orphan\"))\n";
        assert!(harvest(el).is_empty());
    }

    #[test]
    fn skips_longer_identifiers() {
        let el = "(ert-deftest test-org-element/x ()\n  (org-test-with-temp-text-in-file-extra \"nope\")\n  (org-test-with-temp-text \"yes\"))\n";
        let got = harvest(el);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content, "yes");
    }

    #[test]
    fn unterminated_string_is_skipped_not_panicked() {
        let el = "(ert-deftest test-org-element/bad ()\n  (org-test-with-temp-text \"never closed";
        assert!(harvest(el).is_empty());
    }
}
