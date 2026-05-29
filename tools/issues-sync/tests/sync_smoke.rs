//! Implements LD-55 (GitHub Issues sync + Project board placement) — Story 1.16.
//!
//! Wiremock-backed integration test for the idempotency contract (AC6 + AC8).
//! Three scenarios:
//!
//! 1. **First run** against empty repo state — asserts 2 issues created + 2
//!    project items added + 3 milestones created.
//! 2. **Second run** against the state from the first run — asserts ZERO
//!    issues created + ZERO project items added + ZERO label edits + ZERO
//!    body updates. The idempotency contract.
//! 3. **Drift preservation** — an existing issue carries `status:in-progress`
//!    (instead of the expected `status:backlog`). Sync MUST leave the label
//!    untouched. This is the highest-blast-radius invariant (AC8).
//!
//! Why wiremock and not live GitHub: per AC6 §5 — pollutes the real issue
//! tracker with smoke noise, requires PAT in test context, adds flaky
//! external dependency. Wiremock-rs gives 99% of the confidence at 0% of
//! the cost.

use std::path::PathBuf;
use std::sync::OnceLock;

use octocrab::Octocrab;
use orgsidian_issues_sync::parser::parse_epics;
use orgsidian_issues_sync::{github, sync, SyncOpts};
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Process-wide GITHUB_TOKEN init for wiremock tests. `#[tokio::test]` fns
/// run in parallel by default; without serialization, three concurrent
/// `std::env::set_var("GITHUB_TOKEN", …)` calls race the global env table.
/// `OnceLock` guarantees the env var is set exactly once across all tests.
fn ensure_test_env() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        std::env::set_var("GITHUB_TOKEN", "wiremock-test-token");
    });
}

fn fixture_text() -> &'static str {
    include_str!("fixtures/epics-fixture.md")
}

fn make_opts(_server_uri: &str) -> SyncOpts {
    SyncOpts {
        owner: "orgsidian".to_string(),
        repo: "orgsidian".to_string(),
        project_node_id: "PVT_TEST_NODE_ID".to_string(),
        epics_path: PathBuf::from("tools/issues-sync/tests/fixtures/epics-fixture.md"),
        branch_for_links: "main".to_string(),
        dry_run: false,
        render_only: None,
    }
}

fn make_client(server_uri: &str) -> Octocrab {
    // The wiremock MockServer URI is set as the octocrab base_uri so REST +
    // GraphQL calls hit the mock instead of api.github.com.
    ensure_test_env();
    github::build_client_with_base_uri(Some(server_uri)).expect("build wiremock client")
}

/// Helper: JSON shape of a milestone row (subset used by octocrab REST).
fn milestone_json(number: u64, title: &str) -> serde_json::Value {
    json!({
        "url": format!("https://example.invalid/milestone/{number}"),
        "html_url": format!("https://example.invalid/milestone/{number}"),
        "labels_url": format!("https://example.invalid/milestone/{number}/labels"),
        "id": number,
        "node_id": format!("MS_{number}"),
        "number": number,
        "title": title,
        "description": serde_json::Value::Null,
        "state": "open",
        "open_issues": 0,
        "closed_issues": 0,
        "created_at": "2026-05-29T00:00:00Z",
        "updated_at": "2026-05-29T00:00:00Z",
        "due_on": serde_json::Value::Null,
        "closed_at": serde_json::Value::Null,
    })
}

/// Helper: JSON shape of an Issue row (subset used by IssueSnapshot::from).
fn issue_json(number: u64, title: &str, body: &str, labels: &[&str]) -> serde_json::Value {
    let labels_json: Vec<serde_json::Value> = labels
        .iter()
        .map(|name| {
            json!({
                "id": 1u64,
                "node_id": format!("LBL_{name}"),
                "url": "https://example.invalid/label",
                "name": name,
                "description": serde_json::Value::Null,
                "color": "ededed",
                "default": false,
            })
        })
        .collect();
    json!({
        "id": number,
        "node_id": format!("I_{number}"),
        "url": format!("https://example.invalid/issue/{number}"),
        "repository_url": "https://example.invalid/repo",
        "labels_url": "https://example.invalid/issue/labels",
        "comments_url": "https://example.invalid/issue/comments",
        "events_url": "https://example.invalid/issue/events",
        "html_url": format!("https://example.invalid/issue/{number}"),
        "number": number,
        "title": title,
        "state": "open",
        "labels": labels_json,
        "assignees": [],
        "locked": false,
        "comments": 0,
        "created_at": "2026-05-29T00:00:00Z",
        "updated_at": "2026-05-29T00:00:00Z",
        "author_association": "OWNER",
        "body": body,
        "user": {
            "login": "test-user",
            "id": 1u64,
            "node_id": "U_1",
            "avatar_url": "https://example.invalid/avatar",
            "gravatar_id": "",
            "url": "https://example.invalid/user/1",
            "html_url": "https://example.invalid/user/1",
            "followers_url": "https://example.invalid/user/1/followers",
            "following_url": "https://example.invalid/user/1/following",
            "gists_url": "https://example.invalid/user/1/gists",
            "starred_url": "https://example.invalid/user/1/starred",
            "subscriptions_url": "https://example.invalid/user/1/subscriptions",
            "organizations_url": "https://example.invalid/user/1/orgs",
            "repos_url": "https://example.invalid/user/1/repos",
            "events_url": "https://example.invalid/user/1/events",
            "received_events_url": "https://example.invalid/user/1/received_events",
            "type": "User",
            "site_admin": false,
        },
    })
}

/// **AC6 scenario 1 — first run against empty repo state.**
#[tokio::test]
async fn first_run_creates_two_issues_and_adds_to_project() {
    let server = MockServer::start().await;

    // Step 0 — milestones: list returns empty, then 3 creates.
    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/milestones"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::Value::Array(vec![])))
        .expect(1)
        .mount(&server)
        .await;
    for (idx, title) in ["v0.1", "v0.5", "v1.0"].iter().enumerate() {
        Mock::given(method("POST"))
            .and(path("/repos/orgsidian/orgsidian/milestones"))
            .and(body_string_contains(format!("\"title\":\"{title}\"")))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(milestone_json((idx + 1) as u64, title)),
            )
            .expect(1)
            .mount(&server)
            .await;
    }

    // Step 1 — list issues: returns empty array (no existing issues).
    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .and(query_param("state", "all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::Value::Array(vec![])))
        .expect(1)
        .mount(&server)
        .await;

    // GraphQL — project items query returns empty + addProjectV2ItemById (×2).
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("addProjectV2ItemById"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "addProjectV2ItemById": { "item": { "id": "PVI_NEW" } } }
        })))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("ProjectV2"))
        .and(body_string_contains("items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "node": { "items": {
                "pageInfo": { "hasNextPage": false, "endCursor": null },
                "nodes": []
            } } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Step 2 — create issues (×2). Per AC6 §3: assert label-set IS in the
    // outgoing POST /issues body (not just the title) — the wire-level
    // contract is what catches a regression where someone changes the
    // expected-labels set without noticing.
    Mock::given(method("POST"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .and(body_string_contains("Story 3.91"))
        .and(body_string_contains("\"epic:3\""))
        .and(body_string_contains("\"milestone:v0.1\""))
        .and(body_string_contains("\"type:story\""))
        .and(body_string_contains("\"status:backlog\""))
        .respond_with(ResponseTemplate::new(201).set_body_json(issue_json(
            101,
            "[Story 3.91] First smoke story",
            "body-91",
            &["epic:3", "milestone:v0.1", "type:story", "status:backlog"],
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .and(body_string_contains("Story 3.92"))
        .and(body_string_contains("\"epic:3\""))
        .and(body_string_contains("\"milestone:v0.1\""))
        .and(body_string_contains("\"type:story\""))
        .and(body_string_contains("\"status:backlog\""))
        .respond_with(ResponseTemplate::new(201).set_body_json(issue_json(
            102,
            "[Story 3.92] Second smoke story",
            "body-92",
            &["epic:3", "milestone:v0.1", "type:story", "status:backlog"],
        )))
        .expect(1)
        .mount(&server)
        .await;

    let client = make_client(&server.uri());
    let stories = parse_epics(fixture_text()).expect("fixture parses");
    assert_eq!(stories.len(), 2, "fixture should produce exactly 2 stories");

    let report = sync::sync_with_client(&stories, &make_opts(&server.uri()), &client)
        .await
        .expect("sync should succeed");
    assert_eq!(report.issues_created, 2);
    assert_eq!(report.project_items_added, 2);
    assert_eq!(report.stories_total, 2);

    server.verify().await;
}

/// **AC6 scenario 2 — second run against existing state.** Zero new issues,
/// zero project items added, zero label edits, zero body updates.
#[tokio::test]
async fn second_run_is_fully_idempotent() {
    let server = MockServer::start().await;

    // Milestones already exist — list returns all 3.
    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/milestones"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            milestone_json(1, "v0.1"),
            milestone_json(2, "v0.5"),
            milestone_json(3, "v1.0"),
        ])))
        .expect(1)
        .mount(&server)
        .await;
    // ZERO milestone creates this run.
    Mock::given(method("POST"))
        .and(path("/repos/orgsidian/orgsidian/milestones"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let stories = parse_epics(fixture_text()).expect("fixture parses");
    let opts = make_opts(&server.uri());
    let epics_rel = opts
        .epics_path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();
    // Pre-render the expected bodies so the mock returns BYTE-IDENTICAL
    // bodies — the diff-then-update path then short-circuits with zero PATCHes.
    let body_91 = orgsidian_issues_sync::render::render_body(&stories[0], "main", &epics_rel);
    let body_92 = orgsidian_issues_sync::render::render_body(&stories[1], "main", &epics_rel);

    let existing_labels = &["epic:3", "milestone:v0.1", "type:story", "status:backlog"];
    let mut issue_91 = issue_json(
        101,
        "[Story 3.91] First smoke story",
        &body_91,
        existing_labels,
    );
    issue_91["milestone"] = milestone_json(1, "v0.1");
    let mut issue_92 = issue_json(
        102,
        "[Story 3.92] Second smoke story",
        &body_92,
        existing_labels,
    );
    issue_92["milestone"] = milestone_json(1, "v0.1");
    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .and(query_param("state", "all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([issue_91, issue_92])))
        .expect(1)
        .mount(&server)
        .await;

    // GraphQL project items: both issues already on board (numbers 101, 102).
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("ProjectV2"))
        .and(body_string_contains("items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "node": { "items": {
                "pageInfo": { "hasNextPage": false, "endCursor": null },
                "nodes": [
                    { "content": { "number": 101u64 } },
                    { "content": { "number": 102u64 } }
                ]
            } } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // ZERO mutations expected.
    Mock::given(method("POST"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/repos/orgsidian/orgsidian/issues/\d+/labels$",
        ))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("addProjectV2ItemById"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    // Issue PATCH (body/milestone update path).
    Mock::given(method("PATCH"))
        .and(path_regex(r"^/repos/orgsidian/orgsidian/issues/\d+$"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let client = make_client(&server.uri());
    let report = sync::sync_with_client(&stories, &opts, &client)
        .await
        .expect("sync should succeed");
    assert_eq!(
        report.issues_created, 0,
        "no new issues expected on second run"
    );
    assert_eq!(
        report.project_items_added, 0,
        "no project additions expected on second run"
    );
    assert_eq!(report.issues_body_updated, 0, "no body updates expected");
    assert_eq!(report.issues_labels_updated, 0, "no label edits expected");
    assert_eq!(report.skipped_no_change, 2);

    server.verify().await;
}

/// **AC6 scenario 3 — drift preservation.** An existing issue's status label
/// has drifted to `status:in-progress` from the expected `status:backlog`.
/// The sync MUST leave the status label untouched (AC8 invariant).
#[tokio::test]
async fn drift_status_label_is_preserved() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/milestones"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            milestone_json(1, "v0.1"),
            milestone_json(2, "v0.5"),
            milestone_json(3, "v1.0"),
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let stories = parse_epics(fixture_text()).expect("fixture parses");
    let opts = make_opts(&server.uri());
    let epics_rel = opts
        .epics_path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();
    let body_91 = orgsidian_issues_sync::render::render_body(&stories[0], "main", &epics_rel);
    let body_92 = orgsidian_issues_sync::render::render_body(&stories[1], "main", &epics_rel);

    // Issue 101 has DRIFT: status:in-progress instead of status:backlog.
    let mut issue_91 = issue_json(
        101,
        "[Story 3.91] First smoke story",
        &body_91,
        &[
            "epic:3",
            "milestone:v0.1",
            "type:story",
            "status:in-progress",
        ],
    );
    issue_91["milestone"] = milestone_json(1, "v0.1");
    // Issue 102: regular state with status:backlog.
    let mut issue_92 = issue_json(
        102,
        "[Story 3.92] Second smoke story",
        &body_92,
        &["epic:3", "milestone:v0.1", "type:story", "status:backlog"],
    );
    issue_92["milestone"] = milestone_json(1, "v0.1");
    Mock::given(method("GET"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .and(query_param("state", "all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([issue_91, issue_92])))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("ProjectV2"))
        .and(body_string_contains("items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "node": { "items": {
                "pageInfo": { "hasNextPage": false, "endCursor": null },
                "nodes": [
                    { "content": { "number": 101u64 } },
                    { "content": { "number": 102u64 } }
                ]
            } } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // CRITICAL ASSERTIONS:
    //   - ZERO label additions (no `add_labels`).
    //   - ZERO `status:*` removals — codified by the path-regex matcher
    //     covering `/issues/{n}/labels/status%3A...` paths.
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/repos/orgsidian/orgsidian/issues/\d+/labels$",
        ))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path_regex(
            r"^/repos/orgsidian/orgsidian/issues/\d+/labels/status.*$",
        ))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    // Catch-all: any other unmocked POST → 500 to make spurious calls visible.
    Mock::given(method("POST"))
        .and(path("/repos/orgsidian/orgsidian/issues"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let client = make_client(&server.uri());
    let report = sync::sync_with_client(&stories, &opts, &client)
        .await
        .expect("sync should succeed");
    assert_eq!(
        report.issues_labels_updated, 0,
        "status drift must not trigger label edits"
    );
    assert_eq!(report.issues_created, 0, "no new issues on drift run");
    assert_eq!(
        report.project_items_added, 0,
        "no project additions on drift run"
    );

    server.verify().await;
}
