//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! One-way idempotent sync `epics.md` → `orgsidian/orgsidian` GitHub Issues +
//! Project v2 board placement. The crate lives OUTSIDE `[workspace.members]`
//! (LD-5 leaf-isolation) — mirrors `tools/corpus-extractor/`.

use std::path::PathBuf;

use anyhow::Result;

pub mod github;
pub mod parser;
pub mod render;
pub mod sync;

#[derive(Debug, Clone)]
pub struct SyncOpts {
    pub owner: String,
    pub repo: String,
    pub project_node_id: String,
    pub epics_path: PathBuf,
    pub branch_for_links: String,
    pub dry_run: bool,
    pub render_only: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub issues_created: u32,
    pub issues_body_updated: u32,
    pub issues_labels_updated: u32,
    pub issues_milestone_updated: u32,
    pub project_items_added: u32,
    pub milestones_created: u32,
    pub stories_total: u32,
    pub skipped_no_change: u32,
}

/// Map epic number → milestone title. LD-55 + bash-script:38-44 parity.
///
/// `epic ≤ 6 → v0.1`, `7..=12 → v0.5`, `≥ 13 → v1.0`. Epic 0 or `> 13`
/// is a parser regression (out-of-range).
pub fn milestone_for_epic(epic: u8) -> &'static str {
    match epic {
        1..=6 => "v0.1",
        7..=12 => "v0.5",
        _ => {
            debug_assert!(epic >= 13, "epic 0 is invalid");
            debug_assert!(epic <= 13, "epic > 13 unmapped — extend milestone_for_epic");
            "v1.0"
        }
    }
}

/// Entry-point: parse `epics.md`, then either render a single story (when
/// `opts.render_only` is set) or run the full sync.
pub async fn run(opts: SyncOpts) -> Result<SyncReport> {
    let epics_text = std::fs::read_to_string(&opts.epics_path)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", opts.epics_path.display()))?;
    let stories = parser::parse_epics(&epics_text)?;

    // Render-only short-circuit: print the body for a single story and exit.
    // Used by AC10 cell 6 (byte-stability diff against `gh issue view 1`).
    if let Some(num) = &opts.render_only {
        let epics_rel = opts
            .epics_path
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string();
        let story = stories
            .iter()
            .find(|s| format!("{}.{}", s.epic, s.num) == *num)
            .ok_or_else(|| anyhow::anyhow!("story {num} not found in epics.md"))?;
        let body = render::render_body(story, &opts.branch_for_links, &epics_rel);
        print!("{body}");
        return Ok(SyncReport {
            stories_total: stories.len() as u32,
            ..Default::default()
        });
    }

    sync::sync(&stories, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_for_epic_v01_low() {
        assert_eq!(milestone_for_epic(1), "v0.1");
    }

    #[test]
    fn milestone_for_epic_v01_high() {
        assert_eq!(milestone_for_epic(6), "v0.1");
    }

    #[test]
    fn milestone_for_epic_v05_low() {
        assert_eq!(milestone_for_epic(7), "v0.5");
    }

    #[test]
    fn milestone_for_epic_v05_high() {
        assert_eq!(milestone_for_epic(12), "v0.5");
    }

    #[test]
    fn milestone_for_epic_v10() {
        assert_eq!(milestone_for_epic(13), "v1.0");
    }
}
