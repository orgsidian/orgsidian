//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! Wraps the GitHub REST issues API (via octocrab) + Projects v2 GraphQL
//! (via raw `octocrab.graphql(...)` string queries — see AC4 rationale for
//! why we don't pull `graphql_client`). Milestones use raw `crab.get` /
//! `crab.post` because octocrab 0.51 IssueHandler does not expose typed
//! milestone helpers (the typed surface in 0.51 is REST-issues only).
//!
//! Authentication: `GITHUB_TOKEN` env var. In CI this is `secrets.PROJECTS_PAT`
//! (a fine-grained PAT with org-level `Projects: Read+Write`) — NOT the
//! built-in `secrets.GITHUB_TOKEN`, which cannot access org-level Projects v2.

use anyhow::{anyhow, Context, Result};
use octocrab::models::issues::Issue;
use octocrab::params::State;
use octocrab::Octocrab;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Build an Octocrab client from `GITHUB_TOKEN` env var.
pub fn build_client() -> Result<Octocrab> {
    build_client_with_base_uri(None)
}

/// Build an Octocrab client with an optional base URI override. The override
/// path is used by the `tests/sync_smoke.rs` wiremock-backed integration
/// tests (per AC6) — pointing octocrab at a local `MockServer` lets the
/// idempotency contract be exercised without touching the live GitHub API.
///
/// Rejects an empty / whitespace-only token. Reason: `secrets.PROJECTS_PAT`
/// in GitHub Actions evaluates to `""` when the secret is unset, which
/// `std::env::var` returns as `Ok("")` (not `Err`). Without this guard the
/// workflow would build for ~5 min then 401 on the first API call.
pub fn build_client_with_base_uri(base_uri: Option<&str>) -> Result<Octocrab> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
        anyhow!(
            "GITHUB_TOKEN env var missing (use the fine-grained PROJECTS_PAT in CI; \
             `gh auth token` locally)"
        )
    })?;
    if token.trim().is_empty() {
        return Err(anyhow!(
            "GITHUB_TOKEN env var is empty (the CI secret PROJECTS_PAT is likely unset \
             — `gh secret list -R orgsidian/orgsidian` should show it)"
        ));
    }
    let mut builder = Octocrab::builder().personal_token(token);
    if let Some(uri) = base_uri {
        builder = builder.base_uri(uri).context("setting base_uri")?;
    }
    builder.build().context("building octocrab client")
}

/// Snapshot of a live GitHub issue, indexed by title.
#[derive(Debug, Clone)]
pub struct IssueSnapshot {
    pub number: u64,
    pub node_id: String,
    pub title: String,
    pub body: String,
    pub labels: HashSet<String>,
    pub state: octocrab::models::IssueState,
    pub milestone: Option<String>,
}

impl From<Issue> for IssueSnapshot {
    fn from(i: Issue) -> Self {
        let labels = i.labels.iter().map(|l| l.name.clone()).collect();
        Self {
            number: i.number,
            node_id: i.node_id,
            title: i.title,
            body: i.body.unwrap_or_default(),
            labels,
            state: i.state,
            milestone: i.milestone.map(|m| m.title),
        }
    }
}

/// List all issues (open + closed) paginated, indexed by title.
///
/// Filters out pull requests. The GitHub REST `/repos/{owner}/{repo}/issues`
/// endpoint returns BOTH Issues and PRs (the retired
/// `scripts/sync-epics-to-github.sh` filtered via `select(.pull_request == null)`).
/// Without this filter, a PR titled like `[Story 1.1] X` would collide with the
/// title-keyed index → `reconcile_existing` would attempt to PATCH the PR body.
pub async fn list_all_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<HashMap<String, IssueSnapshot>> {
    let mut index = HashMap::new();
    let page = retry_on_throttle("list_issues_page_1", || async {
        client
            .issues(owner, repo)
            .list()
            .state(State::All)
            .per_page(100)
            .send()
            .await
            .context("listing issues page 1")
    })
    .await?;
    let mut current = Some(page);
    while let Some(p) = current {
        for issue in &p.items {
            if issue.pull_request.is_some() {
                continue;
            }
            let snap: IssueSnapshot = issue.clone().into();
            index.insert(snap.title.clone(), snap);
        }
        current = retry_on_throttle("list_issues_next_page", || async {
            client
                .get_page::<Issue>(&p.next)
                .await
                .context("paginating issues")
        })
        .await?;
    }
    Ok(index)
}

#[derive(Debug, Deserialize)]
struct MilestoneRow {
    number: u64,
    title: String,
}

/// Ensure the three milestones (`v0.1`, `v0.5`, `v1.0`) exist on the repo.
/// Returns `(title → milestone number map, created_count)`. Idempotent. Uses
/// raw REST because octocrab 0.51 doesn't expose typed milestone helpers.
pub async fn ensure_milestones(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<(HashMap<String, u64>, u32)> {
    let want: [&str; 3] = ["v0.1", "v0.5", "v1.0"];
    let list_route = format!("/repos/{owner}/{repo}/milestones?state=all&per_page=100");
    let existing: Vec<MilestoneRow> = retry_on_throttle("milestones_list", || async {
        client
            .get(&list_route, None::<&()>)
            .await
            .context("listing milestones")
    })
    .await?;
    let mut by_title: HashMap<String, u64> =
        existing.into_iter().map(|m| (m.title, m.number)).collect();
    let mut created_count: u32 = 0;
    for title in want {
        if !by_title.contains_key(title) {
            let create_route = format!("/repos/{owner}/{repo}/milestones");
            let body = json!({"title": title});
            let created: MilestoneRow =
                retry_on_throttle(&format!("milestones_create_{title}"), || async {
                    client
                        .post(&create_route, Some(&body))
                        .await
                        .with_context(|| format!("creating milestone {title}"))
                })
                .await?;
            by_title.insert(created.title, created.number);
            created_count += 1;
        }
    }
    Ok((by_title, created_count))
}

/// Dry-run twin of [`ensure_milestones`]: LISTs existing milestones but does
/// NOT create missing ones. Returns `(existing_map, would_create_count)`.
pub async fn ensure_milestones_dry_run(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<(HashMap<String, u64>, u32)> {
    let want: [&str; 3] = ["v0.1", "v0.5", "v1.0"];
    let list_route = format!("/repos/{owner}/{repo}/milestones?state=all&per_page=100");
    let existing: Vec<MilestoneRow> = retry_on_throttle("milestones_list_dry_run", || async {
        client
            .get(&list_route, None::<&()>)
            .await
            .context("listing milestones (dry-run)")
    })
    .await?;
    let by_title: HashMap<String, u64> =
        existing.into_iter().map(|m| (m.title, m.number)).collect();
    let would_create = want.iter().filter(|t| !by_title.contains_key(**t)).count() as u32;
    Ok((by_title, would_create))
}

/// Create a fresh issue. Returns the new issue.
pub async fn create_issue(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    labels: Vec<String>,
    milestone_num: Option<u64>,
) -> Result<Issue> {
    retry_on_throttle(&format!("create_issue:{title}"), || {
        let labels = labels.clone();
        async move {
            let handler = client.issues(owner, repo);
            let mut builder = handler
                .create(title)
                .body(body.to_string())
                .labels(Some(labels));
            if let Some(m) = milestone_num {
                builder = builder.milestone(m);
            }
            builder
                .send()
                .await
                .with_context(|| format!("creating issue: {title}"))
        }
    })
    .await
}

/// Patch an issue's body (only — do not touch state/title/labels here).
pub async fn update_body(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    body: &str,
) -> Result<()> {
    retry_on_throttle(&format!("update_body:#{number}"), || async {
        let handler = client.issues(owner, repo);
        handler
            .update(number)
            .body(body)
            .send()
            .await
            .with_context(|| format!("updating body of issue #{number}"))?;
        Ok(())
    })
    .await
}

/// Replace the milestone of an existing issue.
pub async fn set_milestone(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    milestone_num: u64,
) -> Result<()> {
    retry_on_throttle(&format!("set_milestone:#{number}"), || async {
        let handler = client.issues(owner, repo);
        handler
            .update(number)
            .milestone(milestone_num)
            .send()
            .await
            .with_context(|| format!("setting milestone on issue #{number}"))?;
        Ok(())
    })
    .await
}

/// Add labels to an existing issue (non-destructive — does NOT remove others).
pub async fn add_labels(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    labels: &[String],
) -> Result<()> {
    if labels.is_empty() {
        return Ok(());
    }
    retry_on_throttle(&format!("add_labels:#{number}"), || async {
        client
            .issues(owner, repo)
            .add_labels(number, labels)
            .await
            .with_context(|| format!("add_labels on issue #{number}"))?;
        Ok(())
    })
    .await
}

/// Remove a label from an existing issue.
pub async fn remove_label(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    label: &str,
) -> Result<()> {
    retry_on_throttle(&format!("remove_label:#{number}/{label}"), || async {
        client
            .issues(owner, repo)
            .remove_label(number, label)
            .await
            .with_context(|| format!("remove_label '{label}' on issue #{number}"))?;
        Ok(())
    })
    .await
}

/// GraphQL response for the project-items enumeration query. Note that
/// `octocrab.graphql::<T>(...)` already unwraps the outer `{"data": ...}`
/// envelope before deserializing into `T` — so `ProjectItemsResp` is the
/// shape AFTER that strip (i.e., what was inside `data`).
#[derive(Debug, Deserialize)]
struct ProjectItemsResp {
    node: Option<ProjectNode>,
}
#[derive(Debug, Deserialize)]
struct ProjectNode {
    items: ProjectItems,
}
#[derive(Debug, Deserialize)]
struct ProjectItems {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    nodes: Vec<ProjectItemNode>,
}
#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ProjectItemNode {
    content: Option<ProjectItemContent>,
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProjectItemContent {
    Issue { number: u64 },
    Other {},
}

/// GraphQL query: enumerate items already on a Project v2 board.
const Q_PROJECT_ITEMS: &str = r#"
query($projectId: ID!, $after: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      items(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          content {
            ... on Issue { number }
          }
        }
      }
    }
  }
}
"#;

/// GraphQL mutation: add an Issue (by node ID) to a Project v2 board.
const M_ADD_PROJECT_ITEM: &str = r#"
mutation($projectId: ID!, $contentId: ID!) {
  addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId}) {
    item { id }
  }
}
"#;

/// Pre-fetch all issue numbers already on the Project v2 board.
pub async fn project_existing_issue_numbers(
    client: &Octocrab,
    project_node_id: &str,
) -> Result<HashSet<u64>> {
    let mut numbers = HashSet::new();
    let mut after: Option<String> = None;
    loop {
        let vars = json!({"projectId": project_node_id, "after": after});
        let resp: ProjectItemsResp = retry_on_throttle("graphql:project_items", || async {
            client
                .graphql(&json!({"query": Q_PROJECT_ITEMS, "variables": vars}))
                .await
                .context("graphql: project items")
        })
        .await?;
        let node = resp
            .node
            .ok_or_else(|| anyhow!("project node id {project_node_id} not found"))?;
        for it in node.items.nodes {
            if let Some(ProjectItemContent::Issue { number }) = it.content {
                numbers.insert(number);
            }
        }
        if !node.items.page_info.has_next_page {
            break;
        }
        after = node.items.page_info.end_cursor;
        if after.is_none() {
            break;
        }
    }
    Ok(numbers)
}

/// Add a single issue to a Project v2 board (by issue node ID).
///
/// GraphQL can return `HTTP 200` with `{"data": null, "errors": [...]}` —
/// notably for permission errors on org-level Projects or content-ID drift.
/// We deserialize as `Value` and explicitly inspect for an `errors` array so
/// a silent failure can't flip `report.project_items_added` past the truth.
pub async fn add_issue_to_project(
    client: &Octocrab,
    project_node_id: &str,
    issue_node_id: &str,
) -> Result<()> {
    let vars = json!({"projectId": project_node_id, "contentId": issue_node_id});
    let label = format!("graphql:addProjectV2ItemById({issue_node_id})");
    let resp: Value = retry_on_throttle(&label, || async {
        client
            .graphql(&json!({"query": M_ADD_PROJECT_ITEM, "variables": vars}))
            .await
            .with_context(|| format!("graphql: addProjectV2ItemById({issue_node_id})"))
    })
    .await?;
    if let Some(errors) = resp.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            return Err(anyhow!(
                "addProjectV2ItemById({issue_node_id}) returned GraphQL errors: {errors:?}"
            ));
        }
    }
    Ok(())
}

/// Sleep helper used by [`retry_on_throttle`]. Kept public for tests + future
/// inline use (per AC4: governor + tower::retry are overkill for a serial
/// CI-only tool).
pub async fn sleep_secs(s: u64) {
    tokio::time::sleep(Duration::from_secs(s)).await;
}

/// Retry an async operation up to 3 attempts on rate-limit / throttle errors,
/// with exponential back-off. Per AC4: hand-rolled, no tower::retry / governor
/// dep.
///
/// Throttle detection inspects the formatted error string for `403`, `429`,
/// or `rate limit` markers — pragmatic because octocrab 0.51's typed
/// `octocrab::Error::GitHub` variant doesn't expose `Retry-After` on the
/// public surface, and a string-match keeps the helper independent of
/// octocrab's internal layout shifts.
///
/// Back-off schedule: 10s after attempt 1, 30s after attempt 2 (GitHub's
/// secondary-rate-limit recommended floor is 60s; we under-shoot by design
/// because consecutive 403s within 40s typically indicate a primary-limit
/// burst that clears within the next minute window).
pub async fn retry_on_throttle<F, Fut, T>(label: &str, mut op: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    const MAX_ATTEMPTS: u32 = 3;
    const BACKOFF_SECS: [u64; 2] = [10, 30];
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt >= MAX_ATTEMPTS || !is_throttle_error(&e) {
                    return Err(e);
                }
                let sleep_s = BACKOFF_SECS[(attempt - 1) as usize];
                eprintln!(
                    "[{label}] throttled (attempt {attempt}/{MAX_ATTEMPTS}) — backing off {sleep_s}s: {e:#}"
                );
                sleep_secs(sleep_s).await;
            }
        }
    }
}

fn is_throttle_error(err: &anyhow::Error) -> bool {
    let s = format!("{err:#}").to_lowercase();
    s.contains("status: 403")
        || s.contains("status: 429")
        || s.contains("status code: 403")
        || s.contains("status code: 429")
        || s.contains("rate limit")
        || s.contains("rate-limit")
        || s.contains("secondary rate")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retry_on_throttle_succeeds_on_first_attempt() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let result: Result<u32> = retry_on_throttle("test_first_attempt", move || {
            let calls_c = calls_c.clone();
            async move {
                calls_c.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_on_throttle_propagates_non_throttle_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let result: Result<u32> = retry_on_throttle("test_non_throttle", move || {
            let calls_c = calls_c.clone();
            async move {
                calls_c.fetch_add(1, Ordering::SeqCst);
                Err(anyhow!("status: 404 Not Found"))
            }
        })
        .await;
        assert!(result.is_err());
        // 404 is not a throttle — no retry.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn is_throttle_error_matches_403_and_429_variants() {
        assert!(is_throttle_error(&anyhow!("status: 403 Forbidden")));
        assert!(is_throttle_error(&anyhow!("status: 429 Too Many Requests")));
        assert!(is_throttle_error(&anyhow!(
            "github: API rate limit exceeded"
        )));
        assert!(is_throttle_error(&anyhow!("secondary rate limit detected")));
        assert!(!is_throttle_error(&anyhow!("status: 404 Not Found")));
        assert!(!is_throttle_error(&anyhow!(
            "network error: connection reset"
        )));
    }
}
