//! Story 6.4 (FR-7) perf-AC gate: view-switching from `/today` to
//! `/agenda/week` on a 1000-file Vault.
//!
//! The AC ("view-switching...completes in <200 ms on a 1000-file Vault") is
//! the design contract; THIS test is the Story 1.12 regression gate that keeps
//! the committed baseline honest across PRs (±20 % tolerance,
//! `runner_class`-scoped — see `docs/perf/targets.md` for the full rationale).
//! It measures `orgsidian_index::query::agenda::week` directly against the
//! same synthetic 1000-file/5-headline-per-file in-memory index Story 6.3's
//! gate uses — the query (plus its Rust-side stable sort by `agenda_date`) is
//! the whole cost of `/agenda/week`'s render on the read side
//! (`AgendaWeek.tsx` does no further backend round-trip), so this is a
//! faithful proxy for the AC, exactly as Story 6.3's gate is for `/today`.
//!
//! Requires the `test-support` feature (perf baseline JSON I/O; see the
//! `[[test]]` block in `Cargo.toml`):
//! `cargo test -p orgsidian-core --features test-support --test story_6_4_agenda_week_perf`

use orgsidian_core::test_support::perf::assert_no_perf_regression;
use orgsidian_index::query::agenda;
use orgsidian_index::{apply_schema, upsert_file, FileIndexInput, HeadlineInput};
use rusqlite::Connection;

const FILE_COUNT: usize = 1000;
const HEADLINES_PER_FILE: usize = 5;
const START_DATE: &str = "2026-09-05";

/// One synthetic headline. Every 5th headline (index 0 of each file) is
/// Scheduled on the window's start day so the query has real rows to
/// assemble, not an empty scan — the perf-relevant cost is
/// building/sorting/mapping ~1000 result rows, which an all-empty index would
/// not exercise.
fn headline(position: i64) -> HeadlineInput {
    let scheduled = position == 0;
    HeadlineInput {
        level: 1,
        position,
        byte_start: 0,
        byte_end: 10,
        todo_keyword: Some("TODO".to_string()),
        todo_done: Some(false),
        title: format!("Headline {position}"),
        body: "Some body text for realism.".to_string(),
        scheduled_date: if scheduled {
            Some(START_DATE.to_string())
        } else {
            None
        },
        scheduled_time: None,
        deadline_date: None,
        deadline_time: None,
        closed_date: None,
        closed_time: None,
        tags: vec!["work".to_string()],
        properties: Vec::new(),
        clock_entries: Vec::new(),
        links: Vec::new(),
        children: Vec::new(),
    }
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

    conn
}

#[test]
fn agenda_week_query_stays_within_perf_baseline_on_1000_file_vault() {
    let conn = build_synthetic_index();

    // Sanity: the synthetic Vault actually has agenda rows (one per file) —
    // otherwise this would silently benchmark an empty-result fast path.
    let items = agenda::week(&conn, START_DATE).expect("query must succeed");
    assert_eq!(
        items.len(),
        FILE_COUNT,
        "one Scheduled-on-start-day row per file"
    );

    assert_no_perf_regression!(
        "story-6.4-agenda-week",
        "tests/perf-baselines/story-6.4-agenda-week.json",
        || {
            let items = agenda::week(&conn, START_DATE).expect("query must succeed");
            std::hint::black_box(items);
        }
    );
}
