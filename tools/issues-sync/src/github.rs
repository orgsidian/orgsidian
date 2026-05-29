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
pub fn build_client_with_base_uri(base_uri: Option<&str>) -> Result<Octocrab> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
        anyhow!(
            "GITHUB_TOKEN env var missing (use the fine-grained PROJECTS_PAT in CI; \
             `gh auth token` locally)"
        )
    })?;
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
pub async fn list_all_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<HashMap<String, IssueSnapshot>> {
    let mut index = HashMap::new();
    let page = client
        .issues(owner, repo)
        .list()
        .state(State::All)
        .per_page(100)
        .send()
        .await
        .context("listing issues page 1")?;
    let mut current = Some(page);
    while let Some(p) = current {
        for issue in &p.items {
            let snap: IssueSnapshot = issue.clone().into();
            index.insert(snap.title.clone(), snap);
        }
        current = client
            .get_page::<Issue>(&p.next)
            .await
            .context("paginating issues")?;
    }
    Ok(index)
}

#[derive(Debug, Deserialize)]
struct MilestoneRow {
    number: u64,
    title: String,
}

/// Ensure the three milestones (`v0.1`, `v0.5`, `v1.0`) exist on the repo.
/// Returns `(title → milestone number)` map. Idempotent. Uses raw REST
/// because octocrab 0.51 doesn't expose typed milestone helpers.
pub async fn ensure_milestones(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<HashMap<String, u64>> {
    let want: [&str; 3] = ["v0.1", "v0.5", "v1.0"];
    let list_route = format!("/repos/{owner}/{repo}/milestones?state=all&per_page=100");
    let existing: Vec<MilestoneRow> = client
        .get(&list_route, None::<&()>)
        .await
        .context("listing milestones")?;
    let mut by_title: HashMap<String, u64> =
        existing.into_iter().map(|m| (m.title, m.number)).collect();
    for title in want {
        if !by_title.contains_key(title) {
            let create_route = format!("/repos/{owner}/{repo}/milestones");
            let body = json!({"title": title});
            let created: MilestoneRow = client
                .post(&create_route, Some(&body))
                .await
                .with_context(|| format!("creating milestone {title}"))?;
            by_title.insert(created.title, created.number);
        }
    }
    Ok(by_title)
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

/// Patch an issue's body (only — do not touch state/title/labels here).
pub async fn update_body(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    body: &str,
) -> Result<()> {
    let handler = client.issues(owner, repo);
    handler
        .update(number)
        .body(body)
        .send()
        .await
        .with_context(|| format!("updating body of issue #{number}"))?;
    Ok(())
}

/// Replace the milestone of an existing issue.
pub async fn set_milestone(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    milestone_num: u64,
) -> Result<()> {
    let handler = client.issues(owner, repo);
    handler
        .update(number)
        .milestone(milestone_num)
        .send()
        .await
        .with_context(|| format!("setting milestone on issue #{number}"))?;
    Ok(())
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
    client
        .issues(owner, repo)
        .add_labels(number, labels)
        .await
        .with_context(|| format!("add_labels on issue #{number}"))?;
    Ok(())
}

/// Remove a label from an existing issue.
pub async fn remove_label(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    label: &str,
) -> Result<()> {
    client
        .issues(owner, repo)
        .remove_label(number, label)
        .await
        .with_context(|| format!("remove_label '{label}' on issue #{number}"))?;
    Ok(())
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
        let resp: ProjectItemsResp = client
            .graphql(&json!({"query": Q_PROJECT_ITEMS, "variables": vars}))
            .await
            .context("graphql: project items")?;
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
pub async fn add_issue_to_project(
    client: &Octocrab,
    project_node_id: &str,
    issue_node_id: &str,
) -> Result<()> {
    let vars = json!({"projectId": project_node_id, "contentId": issue_node_id});
    let _: Value = client
        .graphql(&json!({"query": M_ADD_PROJECT_ITEM, "variables": vars}))
        .await
        .with_context(|| format!("graphql: addProjectV2ItemById({issue_node_id})"))?;
    Ok(())
}

/// Sleep helper used by callers wanting to honor `Retry-After`. Hand-rolled
/// 3-attempt backoff is implemented inline at call sites (per AC4: governor +
/// tower::retry are overkill).
pub async fn sleep_secs(s: u64) {
    tokio::time::sleep(Duration::from_secs(s)).await;
}
