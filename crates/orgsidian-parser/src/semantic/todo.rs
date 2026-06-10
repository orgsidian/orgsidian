//! TODO keyword state and document-level cycling configuration (FR-1).
//!
//! Org TODO keywords are *configuration*, not syntax: the grammar leaves the
//! keyword inside the headline's `item` text, and this module decides which
//! first word counts as a state. The decision is driven by [`TodoConfig`] —
//! either the Orgsidian default sequence or the document's own `#+TODO:`
//! directives (also accepted under their `#+SEQ_TODO:` / `#+TYP_TODO:`
//! aliases, which share the same value syntax).

use std::ops::Range;

/// A recognized TODO keyword on a headline, with its done-class.
///
/// `keyword` is the exact source text (matching is case-sensitive: `Todo` as
/// a first word is title text, not a state). `done` is the class resolved
/// from the document's [`TodoConfig`] — `true` for keywords after the `|`
/// in the configured sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoState {
    /// The keyword text exactly as written in the source (e.g. `"TODO"`).
    pub keyword: String,
    /// `true` if the keyword belongs to the done set of its sequence.
    pub done: bool,
    /// Byte range of the keyword in the `analyze()` input.
    pub span: Range<usize>,
}

/// One ordered TODO keyword sequence: active keywords, then done keywords.
///
/// Mirrors one `#+TODO:` line (org allows several per document). Cycling
/// advances through `active` then `done` in declaration order and wraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoSequence {
    /// Keywords before the `|` separator (not-yet-done states), in order.
    pub active: Vec<String>,
    /// Keywords after the `|` separator (done states), in order.
    pub done: Vec<String>,
}

impl TodoSequence {
    /// All keywords of this sequence in declaration (cycling) order.
    pub fn keywords(&self) -> impl Iterator<Item = &str> {
        self.active
            .iter()
            .chain(self.done.iter())
            .map(String::as_str)
    }
}

/// The resolved TODO keyword configuration for one document.
///
/// **Default (no `#+TODO:` directive):** active `TODO`, `NEXT`; done `DONE`,
/// `WAITING` — cycling `TODO → NEXT → DONE → WAITING → TODO` (wrap). This is
/// Orgsidian's deliberate default, richer than vanilla org's `TODO | DONE`.
///
/// In-file `#+TODO:` directives **replace** the default for that document.
/// Multiple directive lines accumulate into multiple sequences (the
/// org-faithful choice); a keyword cycles within the sequence it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoConfig {
    /// The configured sequences, in directive declaration order.
    pub sequences: Vec<TodoSequence>,
}

impl Default for TodoConfig {
    fn default() -> Self {
        TodoConfig {
            sequences: vec![TodoSequence {
                active: vec!["TODO".to_string(), "NEXT".to_string()],
                done: vec!["DONE".to_string(), "WAITING".to_string()],
            }],
        }
    }
}

impl TodoConfig {
    /// Build a config from the `#+TODO:`-family directive values found in a
    /// document, in order. Returns the default config when no directive
    /// yields any keyword (an empty or all-whitespace directive is ignored
    /// rather than wiping the keyword set).
    pub fn from_directive_values<'a, I>(values: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let sequences: Vec<TodoSequence> = values
            .into_iter()
            .filter_map(parse_directive_value)
            .collect();
        if sequences.is_empty() {
            TodoConfig::default()
        } else {
            TodoConfig { sequences }
        }
    }

    /// Look up a keyword (case-sensitive, exact). Returns its done-class, or
    /// `None` when the word is not a configured keyword (i.e. title text).
    pub fn classify(&self, keyword: &str) -> Option<bool> {
        for seq in &self.sequences {
            if seq.active.iter().any(|k| k == keyword) {
                return Some(false);
            }
            if seq.done.iter().any(|k| k == keyword) {
                return Some(true);
            }
        }
        None
    }

    /// Advance one step through the cycling order — the pure-function core
    /// Epic 4 will wire to a keybinding.
    ///
    /// - `next(None)` starts the cycle: the first keyword of the first
    ///   sequence.
    /// - `next(Some(k))` advances within the sequence containing `k`, in
    ///   declaration order, wrapping from the last keyword back to the first.
    /// - An unconfigured keyword yields `None` (the caller decides what an
    ///   unknown state cycles to; the config cannot).
    pub fn next(&self, current: Option<&str>) -> Option<&str> {
        let current = match current {
            None => {
                return self.sequences.first().and_then(|s| s.keywords().next());
            }
            Some(c) => c,
        };
        for seq in &self.sequences {
            let keywords: Vec<&str> = seq.keywords().collect();
            if let Some(pos) = keywords.iter().position(|k| *k == current) {
                return keywords.get((pos + 1) % keywords.len()).copied();
            }
        }
        None
    }

    /// All configured keywords across all sequences, in declaration order.
    pub fn keywords(&self) -> impl Iterator<Item = &str> {
        self.sequences.iter().flat_map(TodoSequence::keywords)
    }
}

/// Parse one `#+TODO:` directive value with org's pipe convention: keywords
/// before `|` are active, after it are done; with no `|` the **last** keyword
/// is the done set. Org fast-access suffixes (`TODO(t!)`) are stripped to the
/// bare keyword. Returns `None` when the value contains no keywords.
fn parse_directive_value(value: &str) -> Option<TodoSequence> {
    let words =
        |s: &str| -> Vec<String> { s.split_whitespace().filter_map(strip_fast_access).collect() };
    let (active, done) = match value.split_once('|') {
        Some((left, right)) => (words(left), words(right)),
        None => {
            let mut all = words(value);
            if all.is_empty() {
                return None;
            }
            // No pipe: org convention — the last keyword is the done set.
            let last = all.split_off(all.len() - 1);
            (all, last)
        }
    };
    if active.is_empty() && done.is_empty() {
        return None;
    }
    Some(TodoSequence { active, done })
}

/// Strip an org fast-access suffix: `TODO(t)` / `DONE(d!)` → bare keyword.
/// A token that is *only* a suffix (or empty after stripping) is dropped.
fn strip_fast_access(token: &str) -> Option<String> {
    let bare = match token.find('(') {
        Some(idx) if token.ends_with(')') => &token[..idx],
        _ => token,
    };
    if bare.is_empty() {
        None
    } else {
        Some(bare.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_splits_active_and_done() {
        let seq = parse_directive_value("DRAFT REVIEW | PUBLISHED").expect("keywords");
        assert_eq!(seq.active, ["DRAFT", "REVIEW"]);
        assert_eq!(seq.done, ["PUBLISHED"]);
    }

    #[test]
    fn no_pipe_makes_last_keyword_done() {
        let seq = parse_directive_value("ONE TWO THREE").expect("keywords");
        assert_eq!(seq.active, ["ONE", "TWO"]);
        assert_eq!(seq.done, ["THREE"]);
    }

    #[test]
    fn single_keyword_without_pipe_is_done_only() {
        let seq = parse_directive_value("DONE").expect("keywords");
        assert!(seq.active.is_empty());
        assert_eq!(seq.done, ["DONE"]);
    }

    #[test]
    fn fast_access_suffixes_are_stripped() {
        let seq = parse_directive_value("TODO(t) WAIT(w@/!) | DONE(d!)").expect("keywords");
        assert_eq!(seq.active, ["TODO", "WAIT"]);
        assert_eq!(seq.done, ["DONE"]);
    }

    #[test]
    fn empty_value_is_ignored_and_default_kept() {
        assert_eq!(parse_directive_value("   "), None);
        let cfg = TodoConfig::from_directive_values(["   "]);
        assert_eq!(cfg, TodoConfig::default());
    }

    #[test]
    fn classify_is_case_sensitive() {
        let cfg = TodoConfig::default();
        assert_eq!(cfg.classify("TODO"), Some(false));
        assert_eq!(cfg.classify("WAITING"), Some(true));
        assert_eq!(cfg.classify("Todo"), None);
    }

    #[test]
    fn next_cycles_within_owning_sequence() {
        let cfg = TodoConfig::from_directive_values(["A | B", "X Y | Z"]);
        assert_eq!(cfg.next(None), Some("A"));
        assert_eq!(cfg.next(Some("B")), Some("A"));
        assert_eq!(cfg.next(Some("Z")), Some("X"));
        assert_eq!(cfg.next(Some("missing")), None);
    }
}
