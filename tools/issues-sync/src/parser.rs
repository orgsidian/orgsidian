//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! Parses `_bmad-output/planning-artifacts/epics.md` into per-story records.
//! State machine reproduces the pre-Story-1.16 bash-bootstrap regex
//! semantics, while authoritatively skipping the `## Epic List` overview
//! section before consuming `### Story N.M[a-z]?:` headings inside each
//! `## Epic <N>:` deep section.

use anyhow::Result;
use regex::Regex;
use std::sync::OnceLock;

/// One story extracted from `epics.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    /// Epic ordinal (e.g. 1 for `## Epic 1: …`).
    pub epic: u8,
    /// Story number suffix as a string (e.g. `"3a"` for `### Story 4.3a:`).
    pub num: String,
    /// Story title (text after the colon on the `### Story` heading line).
    pub title: String,
    /// 1-indexed line number of the `### Story` heading (per `wc -l` convention).
    pub line_no: u32,
    /// Persona literal extracted from the body, e.g. `"author / contributor"`.
    pub persona: Option<String>,
    /// Structured `As a … / I want … / so that …` triple.
    pub user_story: Option<UserStory>,
    /// AC block verbatim (preserves Markdown bold + bullets + code fences).
    pub acceptance_criteria: String,
    /// Verbatim `**Traces:**` line, if present.
    pub traces: Option<String>,
    /// First `[Microcopy: draft|final|n/a]` flag found in the body.
    pub microcopy_flag: Option<MicrocopyFlag>,
    /// Verbatim body (everything between the heading and the next flush
    /// boundary). Stored for downstream renderer use.
    pub body_raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStory {
    pub raw: String,
    pub persona: String,
    pub capability: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrocopyFlag {
    Draft,
    Final,
    Na,
}

static STORY_HEADING: OnceLock<Regex> = OnceLock::new();
static EPIC_HEADING: OnceLock<Regex> = OnceLock::new();
static H2_HEADING: OnceLock<Regex> = OnceLock::new();
static PERSONA_RE: OnceLock<Regex> = OnceLock::new();
static USERSTORY_RE: OnceLock<Regex> = OnceLock::new();
static MICROCOPY_RE: OnceLock<Regex> = OnceLock::new();

fn story_heading() -> &'static Regex {
    STORY_HEADING.get_or_init(|| {
        // Mirror the pre-Story-1.16 bash bootstrap's heading regex —
        // letter suffix optional (e.g., `### Story 4.3a:` is distinct from
        // `### Story 4.3:` and from `### Story 4.3b:`).
        Regex::new(r"^###\s+Story\s+([0-9]+)\.([0-9]+[a-z]?):\s+(.+)$").unwrap()
    })
}

fn epic_heading() -> &'static Regex {
    EPIC_HEADING.get_or_init(|| Regex::new(r"^##\s+Epic\s+([0-9]+):\s+(.+)$").unwrap())
}

fn h2_heading() -> &'static Regex {
    H2_HEADING.get_or_init(|| Regex::new(r"^##\s+(.+)$").unwrap())
}

fn persona_re() -> &'static Regex {
    PERSONA_RE.get_or_init(|| Regex::new(r"^As\s+(?:the|a|an)\s+\*\*([^*]+?)\*\*,").unwrap())
}

fn userstory_re() -> &'static Regex {
    USERSTORY_RE.get_or_init(|| {
        Regex::new(
            r"(?s)As\s+(?:the|a|an)\s+\*\*(?P<persona>[^*]+?)\*\*,\s*\n\s*I\s+want\s+(?P<capability>.+?),\s*\n\s*[Ss]o\s+that\s+(?P<outcome>.+?)\.",
        )
        .unwrap()
    })
}

fn microcopy_re() -> &'static Regex {
    MICROCOPY_RE.get_or_init(|| Regex::new(r"\[Microcopy:\s*(draft|final|n/a)\]").unwrap())
}

/// Parse the full `epics.md` text into a vector of `Story` records, in
/// source order.
pub fn parse_epics(text: &str) -> Result<Vec<Story>> {
    enum Section {
        Outside,      // pre-`## Epic List` or post-final-flush
        EpicListSkip, // inside `## Epic List` — skip everything
        EpicDeep(u8), // inside `## Epic <N>:` — collect stories
    }

    let mut section = Section::Outside;
    let mut stories: Vec<Story> = Vec::new();
    let mut cur: Option<PendingStory> = None;

    for (idx, line) in text.lines().enumerate() {
        let line_no = (idx + 1) as u32;

        // Heading-driven section transitions. Check h2-class first because a
        // `## Epic <N>:` line is also matched by the generic `## …` regex.
        if let Some(caps) = epic_heading().captures(line) {
            // Flush any in-flight story before swapping section state.
            if let Some(p) = cur.take() {
                stories.push(p.finalize());
            }
            let epic: u8 = caps[1].parse().unwrap_or(0);
            section = Section::EpicDeep(epic);
            continue;
        }
        if let Some(caps) = h2_heading().captures(line) {
            let title = caps[1].trim();
            if let Some(p) = cur.take() {
                stories.push(p.finalize());
            }
            section = if title.eq_ignore_ascii_case("Epic List") {
                Section::EpicListSkip
            } else {
                Section::Outside
            };
            continue;
        }

        match section {
            Section::Outside | Section::EpicListSkip => {
                // Skip entirely.
            }
            Section::EpicDeep(epic) => {
                if let Some(caps) = story_heading().captures(line) {
                    // Flush previous story (if any) before starting a new one.
                    if let Some(p) = cur.take() {
                        stories.push(p.finalize());
                    }
                    let story_epic: u8 = caps[1].parse().unwrap_or(epic);
                    let num = caps[2].to_string();
                    let title = caps[3].trim().to_string();
                    cur = Some(PendingStory::new(story_epic, num, title, line_no));
                } else if let Some(ref mut p) = cur {
                    p.push_line(line);
                }
            }
        }
    }

    // EOF flush.
    if let Some(p) = cur.take() {
        stories.push(p.finalize());
    }

    Ok(stories)
}

struct PendingStory {
    epic: u8,
    num: String,
    title: String,
    line_no: u32,
    body_lines: Vec<String>,
}

impl PendingStory {
    fn new(epic: u8, num: String, title: String, line_no: u32) -> Self {
        Self {
            epic,
            num,
            title,
            line_no,
            body_lines: Vec::new(),
        }
    }

    fn push_line(&mut self, line: &str) {
        self.body_lines.push(line.to_string());
    }

    fn finalize(self) -> Story {
        // Mirror the pre-Story-1.16 bash bootstrap's accumulator: each line
        // carries a trailing `\n` in the bash version. Equivalent here: join
        // with `\n` AND append a trailing `\n` so body_raw matches that
        // accumulator byte-for-byte. The renderer then strips exactly one
        // trailing `\n` before splicing into the template (mirror of the
        // bash `${body_md%$'\n'}` parameter expansion).
        let mut body_raw = self.body_lines.join("\n");
        if !self.body_lines.is_empty() {
            body_raw.push('\n');
        }

        let persona = persona_re()
            .captures(&body_raw)
            .map(|c| c[1].trim().to_string());

        let user_story = userstory_re().captures(&body_raw).map(|c| {
            let persona = c.name("persona").unwrap().as_str().trim().to_string();
            let capability = c
                .name("capability")
                .unwrap()
                .as_str()
                .trim()
                .replace(['\n'], " ");
            let outcome = c
                .name("outcome")
                .unwrap()
                .as_str()
                .trim()
                .replace(['\n'], " ");
            let raw = c.get(0).unwrap().as_str().to_string();
            UserStory {
                raw,
                persona,
                capability,
                outcome,
            }
        });

        let microcopy_flag = microcopy_re().captures(&body_raw).map(|c| match &c[1] {
            "draft" => MicrocopyFlag::Draft,
            "final" => MicrocopyFlag::Final,
            _ => MicrocopyFlag::Na,
        });

        let acceptance_criteria = extract_ac_block(&body_raw);
        let traces = extract_traces(&body_raw);

        Story {
            epic: self.epic,
            num: self.num,
            title: self.title,
            line_no: self.line_no,
            persona,
            user_story,
            acceptance_criteria,
            traces,
            microcopy_flag,
            body_raw,
        }
    }
}

fn extract_ac_block(body: &str) -> String {
    // Lookup `**Acceptance Criteria:**` marker; capture verbatim everything
    // until the next `**Traces:**`, the next `### `/`## ` heading, or EOF.
    let needle = "**Acceptance Criteria:**";
    let Some(start) = body.find(needle) else {
        return String::new();
    };
    let after = &body[start + needle.len()..];
    let mut end = after.len();
    for (i, line) in after.split_inclusive('\n').enumerate() {
        if i == 0 {
            continue;
        }
        if line.starts_with("**Traces:**") || line.starts_with("### ") || line.starts_with("## ") {
            // Byte offset of the start of this line within `after`.
            let off = after
                .match_indices(line)
                .next()
                .map(|(b, _)| b)
                .unwrap_or(after.len());
            end = off;
            break;
        }
    }
    after[..end].trim_start_matches('\n').trim_end().to_string()
}

fn extract_traces(body: &str) -> Option<String> {
    body.lines()
        .find(|l| l.trim_start().starts_with("**Traces:**"))
        .map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_2_stories() -> &'static str {
        r#"
# Some preamble.

## Epic List

Some overview.

### Epic 1: Foo (this should be skipped — Epic List)

## Epic 1: Foundation

### Story 1.1: First story title

As the **author**,
I want to do thing one,
so that outcome one.

**Acceptance Criteria:**

- AC1 bullet one
- AC2 bullet two

**Traces:** LD-1, LD-2

### Story 1.2: Second story title

As a **user**,
I want to do thing two,
so that outcome two.

**Acceptance Criteria:**

- AC1 only

## Epic 2: Next section flushes story 1.2
"#
    }

    #[test]
    fn parses_two_stories() {
        let stories = parse_epics(fixture_2_stories()).unwrap();
        assert_eq!(stories.len(), 2);
        assert_eq!(stories[0].epic, 1);
        assert_eq!(stories[0].num, "1");
        assert_eq!(stories[0].title, "First story title");
        assert_eq!(stories[1].num, "2");
    }

    #[test]
    fn skips_epic_list_overview() {
        // The `### Epic 1: Foo (this should be skipped — Epic List)` line is
        // matched by epic_heading() but appears INSIDE `## Epic List`. The
        // parser's section gate must prevent it from being treated as a
        // legitimate Epic-deep transition. (In practice, real epics.md only
        // has `### Epic N:` paragraphs inside the overview, never `### Story
        // N.M:`. The defensive check is the in-section gate, not the
        // assumption.)
        // Note: `### Epic 1:` is a level-3 heading, not level-2, so the
        // `## …` regex doesn't match it. Sanity-check that the parser sees
        // exactly 2 stories (story headings only fire inside `## Epic <N>:`).
        let stories = parse_epics(fixture_2_stories()).unwrap();
        assert_eq!(stories.len(), 2, "Epic List h3 must not yield a story");
    }

    #[test]
    fn parses_letter_suffix() {
        let fixture = r#"
## Epic 4: Editor

### Story 4.3a: Letter-suffix story

As a **user**,
I want X,
so that Y.

**Acceptance Criteria:**

- AC

## Epic 5: Next
"#;
        let stories = parse_epics(fixture).unwrap();
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].num, "3a");
    }

    #[test]
    fn preserves_code_block_in_ac() {
        let fixture = r#"
## Epic 1: Foundation

### Story 1.1: Code block

As a **user**,
I want X,
so that Y.

**Acceptance Criteria:**

- Item one with code:
  ```rust
  fn foo() {}
  ```
- Item two

**Traces:** LD-1

## Epic 2: Next
"#;
        let stories = parse_epics(fixture).unwrap();
        assert_eq!(stories.len(), 1);
        assert!(
            stories[0].acceptance_criteria.contains("```rust"),
            "code fence must be preserved verbatim in AC block: got {:?}",
            stories[0].acceptance_criteria
        );
        assert!(stories[0].acceptance_criteria.contains("fn foo()"));
    }

    #[test]
    fn parses_story_without_traces() {
        let fixture = r#"
## Epic 1: Foundation

### Story 1.1: No traces line

As a **user**,
I want X,
so that Y.

**Acceptance Criteria:**

- AC

## Epic 2: Next
"#;
        let stories = parse_epics(fixture).unwrap();
        assert_eq!(stories.len(), 1);
        assert!(stories[0].traces.is_none());
    }

    /// Regression net: parsing the live `_bmad-output/planning-artifacts/
    /// epics.md` must yield the expected story count. The story spec said
    /// "104-story roadmap" but the live file as of 2026-05-29 has 117
    /// `### Story N.M[a-z]?:` headings. The 117 floor is the contract going
    /// forward — adding/removing stories without updating this assertion is
    /// a parser regression that this test catches loud.
    #[test]
    fn real_epics_md_parses_to_117_stories() {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../_bmad-output/planning-artifacts/epics.md"
        ));
        let stories = parse_epics(text).unwrap();
        assert_eq!(
            stories.len(),
            117,
            "epics.md story-count drift: expected 117, got {}",
            stories.len()
        );
    }
}
