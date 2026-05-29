//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! Renders an Issue body from a `parser::Story`. Output MUST match the
//! existing live issue bodies byte-for-byte (modulo the sentinel-line flip
//! to `tools/issues-sync` documented in AC3). Idempotent re-runs depend on
//! the renderer's output being byte-stable.

use crate::milestone_for_epic;
use crate::parser::Story;

/// Render the full Issue body for a story. `branch_for_links` is the git ref
/// used in the `**Source:**` footer URL (typically `main`).
/// `epics_file_relpath` is the path relative to the repo root, e.g.
/// `_bmad-output/planning-artifacts/epics.md`.
pub fn render_body(story: &Story, branch_for_links: &str, epics_file_relpath: &str) -> String {
    let milestone = milestone_for_epic(story.epic);
    // Mirror bash `${body_md%$'\n'}` — strip exactly one trailing newline,
    // not all. Parser preserves the leading + trailing structure so the
    // rendered output is byte-identical to the bash-script-authored bodies
    // currently on GitHub (modulo the sentinel-line flip + the line-number
    // drift discussed in AC3 + AC10 cell 6 caveat).
    let body_md: &str = story.body_raw.strip_suffix('\n').unwrap_or(&story.body_raw);

    format!(
        "> Auto-synced from `{epics}` by `tools/issues-sync`. Manual edits below this line will be **overwritten** on next sync; status label drift is preserved.\n\
\n\
**Epic:** {epic} &middot; **Milestone:** {milestone}\n\
\n\
---\n\
\n\
{body}\n\
\n\
---\n\
\n\
**Source:** [`{epics}` line {line}](https://github.com/orgsidian/orgsidian/blob/{branch}/{epics}#L{line})\n",
        epics = epics_file_relpath,
        epic = story.epic,
        milestone = milestone,
        body = body_md,
        line = story.line_no,
        branch = branch_for_links,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_epics, Story};

    fn synthetic_story() -> Story {
        Story {
            epic: 1,
            num: "1".to_string(),
            title: "Hello".to_string(),
            line_no: 42,
            persona: Some("user".to_string()),
            user_story: None,
            acceptance_criteria: "- AC1".to_string(),
            traces: None,
            microcopy_flag: None,
            body_raw: "Body line one.\nBody line two.".to_string(),
        }
    }

    #[test]
    fn header_sentinel_uses_tools_issues_sync() {
        let body = render_body(
            &synthetic_story(),
            "main",
            "_bmad-output/planning-artifacts/epics.md",
        );
        assert!(
            body.starts_with("> Auto-synced from `_bmad-output/planning-artifacts/epics.md` by `tools/issues-sync`."),
            "header sentinel must reference tools/issues-sync, got: {body:.200}"
        );
    }

    #[test]
    fn body_contains_milestone_mapping_for_epic_1() {
        let body = render_body(
            &synthetic_story(),
            "main",
            "_bmad-output/planning-artifacts/epics.md",
        );
        assert!(body.contains("**Milestone:** v0.1"));
        assert!(body.contains("**Epic:** 1"));
    }

    #[test]
    fn body_contains_source_footer_with_line_anchor() {
        let body = render_body(
            &synthetic_story(),
            "main",
            "_bmad-output/planning-artifacts/epics.md",
        );
        assert!(body.contains("**Source:**"));
        assert!(body.contains("#L42"), "line anchor missing: {body}");
        assert!(body.contains("orgsidian/orgsidian/blob/main/"));
    }

    /// Snapshot test against a checked-in golden file generated from
    /// `gh issue view 1 -R orgsidian/orgsidian --json body --jq .body` with
    /// the sentinel-line substitution applied. The Story 1.1 body in
    /// `epics.md` is the parser input; the golden file is the expected
    /// renderer output. This is the byte-stability contract (AC3).
    #[test]
    fn renders_story_1_1_byte_equal_to_golden() {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../_bmad-output/planning-artifacts/epics.md"
        ));
        let stories = parse_epics(text).unwrap();
        let story_1_1 = stories
            .iter()
            .find(|s| s.epic == 1 && s.num == "1")
            .expect("Story 1.1 missing from epics.md");
        let rendered = render_body(
            story_1_1,
            "main",
            "_bmad-output/planning-artifacts/epics.md",
        );
        let golden = include_str!("../tests/golden/story-1-1-body.md");
        pretty_assertions::assert_eq!(rendered, golden);
    }
}
