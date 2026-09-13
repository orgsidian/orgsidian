//! Story 7.1 (FR-6) perf-AC gate: the Today Dashboard assembly on a 1000-file
//! Vault.
//!
//! The absolute NFR ("dashboard render under 500 ms on a 1000-file Vault",
//! `docs/perf/targets.md` / Epic 7 context) is the design contract; THIS test
//! is the Story 1.12 regression gate that keeps the committed baseline honest
//! across PRs (±20 % tolerance, `runner_class`-scoped). Per the Story 7.1
//! Design Notes, `assert_no_perf_regression!` is a Rust macro, so it gates the
//! backend dashboard-data assembly (`dashboard::today`) — the measurable proxy
//! for "dashboard render" — not the React render. The synthetic index mirrors
//! `story_6_3_agenda_today_perf.rs`.
//!
//! Requires the `test-support` feature (perf baseline JSON I/O; see the
//! `[[test]]` block in `Cargo.toml`):
//! `cargo test -p orgsidian-core --features test-support --test story_7_1_today_dashboard_perf`

use orgsidian_core::test_support::perf::assert_no_perf_regression;
use orgsidian_index::query::dashboard::{self, DashboardParams};
use orgsidian_index::{apply_schema, upsert_file, FileIndexInput, HeadlineInput};
use rusqlite::Connection;

const FILE_COUNT: usize = 1000;
const HEADLINES_PER_FILE: usize = 5;
const INBOX_ENTRIES: usize = 20;
const TODAY: &str = "2026-09-05";
const TODAY_TAG: &str = "today";
const INBOX_PREVIEW_COUNT: usize = 5;

/// One synthetic headline in a regular Vault file. Headline 0 of each file is
/// Scheduled today AND tagged `today` (so the Scheduled and Today-tag sections
/// each assemble ~1000 rows); headline 1 carries a Deadline due today (so the
/// Deadline section assembles ~1000 rows too). The perf-relevant cost is
/// building/sorting/mapping several thousand result rows across the sections,
/// which an all-empty index would not exercise.
fn headline(position: i64) -> HeadlineInput {
    let scheduled = position == 0;
    let deadline = position == 1;
    HeadlineInput {
        level: 1,
        position,
        byte_start: 0,
        byte_end: 10,
        todo_keyword: Some("TODO".to_string()),
        todo_done: Some(false),
        title: format!("Headline {position}"),
        body: "Some body text for realism.".to_string(),
        scheduled_date: scheduled.then(|| TODAY.to_string()),
        scheduled_time: None,
        deadline_date: deadline.then(|| TODAY.to_string()),
        deadline_time: None,
        closed_date: None,
        closed_time: None,
        tags: if scheduled {
            vec![TODAY_TAG.to_string(), "work".to_string()]
        } else {
            vec!["work".to_string()]
        },
        properties: Vec::new(),
        clock_entries: Vec::new(),
        links: Vec::new(),
        children: Vec::new(),
    }
}

/// A plain Inbox-capture headline (no scheduling/tags), so the Inbox section
/// has real rows to `LIMIT` and map.
fn inbox_headline(position: i64) -> HeadlineInput {
    let mut h = headline(position);
    h.title = format!("Inbox capture {position}");
    h.scheduled_date = None;
    h.deadline_date = None;
    h.tags = Vec::new();
    h
}

fn build_synthetic_index() -> Connection {
    let mut conn = Connection::open_in_memory().expect("open in-memory db");
    apply_schema(&mut conn).expect("apply schema");

    for file_idx in 0..FILE_COUNT {
        let headlines = (0..HEADLINES_PER_FILE as i64).map(headline).collect();
        let input = FileIndexInput {
            rel_path: format!("vault/file-{file_idx:04}.org"),
            mtime_ns: 1,
            size_bytes: 1,
            preamble: None,
            headlines,
        };
        upsert_file(&mut conn, &input).expect("seed synthetic file");
    }

    // A Vault-root inbox.org for the Inbox-preview section.
    let inbox = FileIndexInput {
        rel_path: "inbox.org".to_string(),
        mtime_ns: 1,
        size_bytes: 1,
        preamble: None,
        headlines: (0..INBOX_ENTRIES as i64).map(inbox_headline).collect(),
    };
    upsert_file(&mut conn, &inbox).expect("seed inbox");

    conn
}

fn params() -> DashboardParams {
    DashboardParams {
        today: TODAY.to_string(),
        today_tag: TODAY_TAG.to_string(),
        inbox_preview_count: INBOX_PREVIEW_COUNT,
    }
}

#[test]
fn today_dashboard_assembly_stays_within_perf_baseline_on_1000_file_vault() {
    let conn = build_synthetic_index();

    // Sanity: the synthetic Vault actually populates every section — otherwise
    // this would silently benchmark empty-result fast paths.
    let dash = dashboard::today(&conn, &params()).expect("query must succeed");
    assert_eq!(
        dash.scheduled.len(),
        FILE_COUNT,
        "one Scheduled-today row per file"
    );
    assert_eq!(
        dash.deadlines.len(),
        FILE_COUNT,
        "one Deadline-today row per file"
    );
    assert_eq!(
        dash.today_tag.len(),
        FILE_COUNT,
        "one today-tagged row per file"
    );
    assert_eq!(
        dash.inbox.len(),
        INBOX_PREVIEW_COUNT,
        "the first N inbox entries"
    );

    assert_no_perf_regression!(
        "story-7.1-today-dashboard",
        "tests/perf-baselines/story-7.1.json",
        || {
            let dash = dashboard::today(&conn, &params()).expect("query must succeed");
            std::hint::black_box(dash);
        }
    );
}
