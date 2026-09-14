//! Story 7.5 (FR-7) end-to-end: each shipped Starter Vault's REAL generated
//! `.org` content, once parsed + indexed through the production scan pipeline,
//! must surface non-empty results in the two default presets on first launch.
//!
//! This chains real starter-vault org text → `parser::analyze` → indexer
//! (`scan_vault`) → the `completed`-mode agenda query (`Done This Week` /
//! `Done This Month`), rather than asserting against the fixture strings or the
//! `HeadlineInput` structs directly — closing the gap flagged in review that no
//! test proved the AC's "each starter vault returns ≥1 result from fixtures in
//! the default presets" through the actual query path. Hermetic: `open_index`
//! takes the DB path directly, so it touches neither the OS data dir nor
//! `global.toml` (same discipline as `tests/scan.rs`).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use chrono::{Duration, NaiveDate};
use orgsidian_core::{generate_starter_vault, open_index, scan_vault, StarterVaultKind};
use orgsidian_index::query::agenda::{custom, CustomAgendaQuery};
use tempfile::TempDir;

/// The fixed "today" every Starter Vault fixture is anchored to in its own unit
/// tests — reused here so the rolling windows line up with the CLOSED stamps the
/// generators emit.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 5).expect("valid date")
}

fn iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// The completion-mode query the `Done This …` default presets resolve to at
/// recall time: the rolling window `[today-(n-1), today]`, filtered to DONE.
/// `CustomAgendaQuery` is `#[non_exhaustive]`, so it is built via `Default` then
/// mutated (the same shape the shell-app DTO uses).
fn done_in_rolling(rolling_days: i64) -> CustomAgendaQuery {
    let mut query = CustomAgendaQuery::default();
    query.start_date = iso(today() - Duration::days(rolling_days - 1));
    query.end_date = iso(today());
    query.todo_state = Some("DONE".to_string());
    query.completed_in_range = true;
    query
}

/// Generate `kind` into a temp vault, index it with the real scan pipeline, and
/// return the temp dirs (kept alive) plus the index DB path.
async fn index_starter(kind: StarterVaultKind) -> (TempDir, TempDir, PathBuf) {
    let vault = TempDir::new().expect("vault tempdir");
    let index = TempDir::new().expect("index tempdir");
    let db_path = index.path().join("index.sqlite3");

    generate_starter_vault(kind, vault.path(), today()).expect("generate starter vault");

    let handle = open_index(vault.path(), &db_path)
        .await
        .expect("open index");
    let cancel = AtomicBool::new(false);
    let outcome = scan_vault(&handle, &cancel, |_| {})
        .await
        .expect("scan starter vault");
    assert_eq!(
        outcome.errors, 0,
        "{kind:?} starter vault indexed with errors"
    );
    // Drain the writer + close pool connections so a fresh read connection sees
    // every committed frame.
    handle.shutdown().await;

    (vault, index, db_path)
}

/// Prove both default presets are non-empty (the AC pins ≥2 DONE fixtures inside
/// the rolling-7 window, which is a subset of the rolling-30 window).
async fn assert_default_presets_non_empty(kind: StarterVaultKind) {
    let (_vault, _index, db_path) = index_starter(kind).await;
    let conn = orgsidian_index::open(&db_path).expect("open index for read");

    let week_query = done_in_rolling(7);
    let week = custom(&conn, &week_query).expect("Done This Week query");
    assert!(
        week.len() >= 2,
        "{kind:?} `Done This Week` must surface ≥2 DONE rows from fixtures, got {}",
        week.len()
    );
    for item in &week {
        assert_eq!(
            item.todo_keyword.as_deref(),
            Some("DONE"),
            "completion-mode rows must be DONE headlines"
        );
        // The completion path groups by the CLOSED date (`agenda_date`); it must
        // land inside the queried rolling window.
        assert!(
            item.agenda_date.as_str() >= week_query.start_date.as_str()
                && item.agenda_date.as_str() <= week_query.end_date.as_str(),
            "closed date {} outside the rolling-7 window [{}, {}]",
            item.agenda_date,
            week_query.start_date,
            week_query.end_date
        );
    }

    let month = custom(&conn, &done_in_rolling(30)).expect("Done This Month query");
    assert!(
        month.len() >= 2,
        "{kind:?} `Done This Month` must surface ≥2 DONE rows from fixtures, got {}",
        month.len()
    );
    // The rolling-7 hits are a subset of the rolling-30 hits.
    assert!(
        month.len() >= week.len(),
        "{kind:?} the 30-day window must include at least the 7-day window's rows"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn personal_gtd_default_presets_return_done_fixtures() {
    assert_default_presets_non_empty(StarterVaultKind::PersonalGtd).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn student_default_presets_return_done_fixtures() {
    assert_default_presets_non_empty(StarterVaultKind::Student).await;
}
