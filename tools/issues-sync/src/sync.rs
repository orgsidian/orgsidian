//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! Idempotency contract (AC5 + AC8):
//! - No duplicate issues created (lookup by exact title `[Story N.M] <title>`).
//! - No label thrash (set-equality diff on the non-status partition).
//! - **`status:*` labels are NEVER touched on existing issues** — this is the
//!   highest-blast-radius invariant; codified by [`partition_labels`] + tested
//!   by [`tests::status_drift_preserved`] + the wiremock-backed integration
//!   test in `tests/sync_smoke.rs`.
//! - No Project board re-shuffle (pre-fetch existing items by issue number).

use std::collections::HashSet;

use anyhow::Result;

use crate::github::IssueSnapshot;
use crate::parser::Story;
use crate::{milestone_for_epic, render, SyncOpts, SyncReport};

/// Split `labels` into `(status_labels, non_status_labels)`. The contract is
/// `name.starts_with("status:")` — matches `status:backlog`, `status:in-progress`,
/// `status:in-review`, `status:done` per
/// [[project_orgsidian_github_label_scheme]].
pub fn partition_labels(labels: &HashSet<String>) -> (HashSet<String>, HashSet<String>) {
    let mut status = HashSet::new();
    let mut non_status = HashSet::new();
    for l in labels {
        if l.starts_with("status:") {
            status.insert(l.clone());
        } else {
            non_status.insert(l.clone());
        }
    }
    (status, non_status)
}

/// Compute the diff between `expected` and `actual` non-status label sets.
/// Returns `(to_add, to_remove)`. **CRITICAL**: callers MUST partition both
/// sides before invoking — this function does NOT defend against status
/// labels leaking in.
pub fn label_diff(
    expected_non_status: &HashSet<String>,
    actual_non_status: &HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let to_add: Vec<String> = expected_non_status
        .difference(actual_non_status)
        .cloned()
        .collect();
    let to_remove: Vec<String> = actual_non_status
        .difference(expected_non_status)
        .cloned()
        .collect();
    (to_add, to_remove)
}

/// Expected non-status label set for a story.
pub fn expected_labels_for_story(story: &Story) -> HashSet<String> {
    let mut s = HashSet::new();
    s.insert(format!("epic:{}", story.epic));
    s.insert(format!("milestone:{}", milestone_for_epic(story.epic)));
    s.insert("type:story".to_string());
    s
}

/// Decide whether an issue (already in the index) needs project-board placement.
pub fn should_add_to_project(issue_number: u64, on_board: &HashSet<u64>) -> bool {
    !on_board.contains(&issue_number)
}

/// Format the canonical title for a story.
pub fn title_for_story(story: &Story) -> String {
    format!("[Story {}.{}] {}", story.epic, story.num, story.title)
}

/// Main sync routine. Serial REST/GraphQL calls (per AC4 concurrency note).
/// Builds the client from `GITHUB_TOKEN` and delegates to [`sync_with_client`].
pub async fn sync(stories: &[Story], opts: &SyncOpts) -> Result<SyncReport> {
    let client = crate::github::build_client()?;
    sync_with_client(stories, opts, &client).await
}

/// Inner sync routine. Accepts a pre-built `Octocrab` client — used by the
/// wiremock-backed integration tests at `tests/sync_smoke.rs` (AC6).
pub async fn sync_with_client(
    stories: &[Story],
    opts: &SyncOpts,
    client: &octocrab::Octocrab,
) -> Result<SyncReport> {
    let mut report = SyncReport {
        stories_total: stories.len() as u32,
        ..Default::default()
    };

    if opts.dry_run {
        eprintln!(
            "[dry-run] {} stories parsed from {}",
            stories.len(),
            opts.epics_path.display()
        );
    }

    // Step 0 — milestones.
    let milestones = if opts.dry_run {
        eprintln!("[dry-run] would ensure milestones v0.1 / v0.5 / v1.0");
        std::collections::HashMap::new()
    } else {
        let before = std::collections::HashMap::<String, u64>::new();
        let m = crate::github::ensure_milestones(client, &opts.owner, &opts.repo).await?;
        report.milestones_created = (m.len() as i64 - before.len() as i64).max(0) as u32;
        m
    };

    // Step 1 — pre-fetch issue index + project items.
    let issue_index = if opts.dry_run {
        eprintln!("[dry-run] would list all issues");
        std::collections::HashMap::new()
    } else {
        crate::github::list_all_issues(client, &opts.owner, &opts.repo).await?
    };
    let on_board = if opts.dry_run {
        eprintln!(
            "[dry-run] would enumerate project items for {}",
            opts.project_node_id
        );
        HashSet::new()
    } else {
        crate::github::project_existing_issue_numbers(client, &opts.project_node_id).await?
    };

    let epics_rel = opts
        .epics_path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();

    for story in stories {
        let title = title_for_story(story);
        let expected_body = render::render_body(story, &opts.branch_for_links, &epics_rel);
        let expected_labels = expected_labels_for_story(story);
        let expected_milestone_title = milestone_for_epic(story.epic);
        let expected_milestone_num = milestones.get(expected_milestone_title).copied();

        match issue_index.get(&title) {
            Some(existing) => {
                reconcile_existing(
                    client,
                    opts,
                    existing,
                    &expected_body,
                    &expected_labels,
                    expected_milestone_title,
                    expected_milestone_num,
                    &on_board,
                    &mut report,
                )
                .await?;
            }
            None => {
                create_new(
                    client,
                    opts,
                    story,
                    &title,
                    &expected_body,
                    &expected_labels,
                    expected_milestone_num,
                    &mut report,
                )
                .await?;
            }
        }
    }

    Ok(report)
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_existing(
    client: &octocrab::Octocrab,
    opts: &SyncOpts,
    existing: &IssueSnapshot,
    expected_body: &str,
    expected_non_status_labels: &HashSet<String>,
    expected_milestone_title: &str,
    expected_milestone_num: Option<u64>,
    on_board: &HashSet<u64>,
    report: &mut SyncReport,
) -> Result<()> {
    // Body diff.
    if existing.body != expected_body {
        if opts.dry_run {
            eprintln!("[dry-run] would update body of issue #{}", existing.number);
        } else {
            crate::github::update_body(
                client,
                &opts.owner,
                &opts.repo,
                existing.number,
                expected_body,
            )
            .await?;
        }
        report.issues_body_updated += 1;
    } else {
        report.skipped_no_change += 1;
    }

    // Label diff — partition both sides; ONLY touch non-status.
    let (_status_existing, non_status_existing) = partition_labels(&existing.labels);
    let (to_add, to_remove) = label_diff(expected_non_status_labels, &non_status_existing);
    if !to_add.is_empty() {
        if opts.dry_run {
            eprintln!(
                "[dry-run] would add labels {:?} to issue #{}",
                to_add, existing.number
            );
        } else {
            crate::github::add_labels(client, &opts.owner, &opts.repo, existing.number, &to_add)
                .await?;
        }
        report.issues_labels_updated += 1;
    }
    for label in &to_remove {
        debug_assert!(
            !label.starts_with("status:"),
            "label_diff returned a status:* label — partition_labels invariant broken"
        );
        if opts.dry_run {
            eprintln!(
                "[dry-run] would remove label {label} from issue #{}",
                existing.number
            );
        } else {
            crate::github::remove_label(client, &opts.owner, &opts.repo, existing.number, label)
                .await?;
        }
        report.issues_labels_updated += 1;
    }

    // Milestone reconciliation.
    if existing.milestone.as_deref() != Some(expected_milestone_title) {
        if let Some(num) = expected_milestone_num {
            if opts.dry_run {
                eprintln!(
                    "[dry-run] would set milestone of issue #{} to {expected_milestone_title}",
                    existing.number
                );
            } else {
                crate::github::set_milestone(client, &opts.owner, &opts.repo, existing.number, num)
                    .await?;
            }
            report.issues_milestone_updated += 1;
        }
    }

    // Project board placement.
    if should_add_to_project(existing.number, on_board) {
        if opts.dry_run {
            eprintln!(
                "[dry-run] would add issue #{} to project board",
                existing.number
            );
        } else {
            crate::github::add_issue_to_project(client, &opts.project_node_id, &existing.node_id)
                .await?;
        }
        report.project_items_added += 1;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_new(
    client: &octocrab::Octocrab,
    opts: &SyncOpts,
    _story: &Story,
    title: &str,
    expected_body: &str,
    expected_non_status_labels: &HashSet<String>,
    expected_milestone_num: Option<u64>,
    report: &mut SyncReport,
) -> Result<()> {
    let mut labels: Vec<String> = expected_non_status_labels.iter().cloned().collect();
    labels.sort();
    labels.push("status:backlog".to_string());

    if opts.dry_run {
        eprintln!("[dry-run] would create issue: {title} (labels: {labels:?})");
        report.issues_created += 1;
        report.project_items_added += 1;
        return Ok(());
    }

    let created = crate::github::create_issue(
        client,
        &opts.owner,
        &opts.repo,
        title,
        expected_body,
        labels,
        expected_milestone_num,
    )
    .await?;
    report.issues_created += 1;

    // Add to project board immediately.
    crate::github::add_issue_to_project(client, &opts.project_node_id, &created.node_id).await?;
    report.project_items_added += 1;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> HashSet<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn partition_separates_status_and_non_status() {
        let labels = s(&[
            "epic:1",
            "milestone:v0.1",
            "type:story",
            "status:in-progress",
        ]);
        let (status, non_status) = partition_labels(&labels);
        assert_eq!(status, s(&["status:in-progress"]));
        assert_eq!(non_status, s(&["epic:1", "milestone:v0.1", "type:story"]));
    }

    #[test]
    fn label_diff_set_equality_yields_empty() {
        let want = s(&["epic:1", "milestone:v0.1", "type:story"]);
        let have = s(&["type:story", "milestone:v0.1", "epic:1"]);
        let (add, rem) = label_diff(&want, &have);
        assert!(add.is_empty());
        assert!(rem.is_empty());
    }

    #[test]
    fn label_diff_reports_missing_and_extra() {
        let want = s(&["epic:1", "milestone:v0.1", "type:story"]);
        let have = s(&["epic:1", "milestone:v0.5"]); // wrong milestone, missing type:story
        let (mut add, mut rem) = label_diff(&want, &have);
        add.sort();
        rem.sort();
        assert_eq!(
            add,
            vec!["milestone:v0.1".to_string(), "type:story".to_string()]
        );
        assert_eq!(rem, vec!["milestone:v0.5".to_string()]);
    }

    /// AC8 invariant: status:in-progress on an existing issue MUST NOT be
    /// touched by the diff path. Verified by partitioning + diffing only
    /// the non-status set.
    #[test]
    fn status_drift_preserved() {
        let actual = s(&[
            "epic:1",
            "milestone:v0.1",
            "type:story",
            "status:in-progress",
        ]);
        let (_status, non_status) = partition_labels(&actual);
        let expected = s(&["epic:1", "milestone:v0.1", "type:story"]);
        let (add, rem) = label_diff(&expected, &non_status);
        assert!(add.is_empty(), "no labels should be added: {add:?}");
        assert!(rem.is_empty(), "no labels should be removed: {rem:?}");
    }

    /// Defense-in-depth: even if a future regression caused the expected set
    /// to include `status:backlog`, the partition-then-diff pipeline still
    /// must report no changes when the existing issue has any `status:*`
    /// label. The caller (reconcile_existing) does the partitioning; this
    /// test simulates the contract.
    #[test]
    fn status_drift_preserved_even_if_expected_leaks_status() {
        let actual = s(&[
            "epic:1",
            "milestone:v0.1",
            "type:story",
            "status:in-progress",
        ]);
        let (_status_actual, non_status_actual) = partition_labels(&actual);
        // Caller MUST partition the expected set too, even if some future bug
        // emits a status label into it.
        let expected_leaked = s(&["epic:1", "milestone:v0.1", "type:story", "status:backlog"]);
        let (_status_expected, non_status_expected) = partition_labels(&expected_leaked);
        let (add, rem) = label_diff(&non_status_expected, &non_status_actual);
        assert!(
            add.is_empty(),
            "regression: status:backlog leaked into add list: {add:?}"
        );
        assert!(
            rem.is_empty(),
            "regression: status:in-progress leaked into remove list: {rem:?}"
        );
    }

    #[test]
    fn project_placement_skipped_when_already_on_board() {
        let on_board: HashSet<u64> = [1u64, 2, 3].into_iter().collect();
        assert!(!should_add_to_project(2, &on_board));
        assert!(should_add_to_project(4, &on_board));
    }

    #[test]
    fn title_format_matches_bash_script() {
        let story = Story {
            epic: 1,
            num: "16".to_string(),
            title: "GitHub Issues sync — one issue per story".to_string(),
            line_no: 1,
            persona: None,
            user_story: None,
            acceptance_criteria: String::new(),
            traces: None,
            microcopy_flag: None,
            body_raw: String::new(),
        };
        assert_eq!(
            title_for_story(&story),
            "[Story 1.16] GitHub Issues sync — one issue per story"
        );
    }
}
