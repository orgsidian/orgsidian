//! Integration tests for `orgsidian index {init|rebuild|stats|integrity}`
//! (Story 3.7, LD-27 / LD-49) via `assert_cmd` against the real binary.
//!
//! Posture (mirrors `parse_cmd.rs`): human output is asserted with loose
//! substrings (the rendered format is NOT a contract); `--json` output is
//! asserted through parsed `serde_json::Value` field access, never golden
//! strings. Every run is hermetic — each gets its own `ORGSIDIAN_DATA_DIR`
//! (and an isolated HOME / XDG config so the vault-designation `recent_vaults`
//! write never touches the developer's real settings) via a fresh `TempDir`,
//! so the index store lives entirely under temp and the committed fixture
//! vault is only ever read.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// The committed fixture vault (valid `notes.org` + `tasks.org` plus a
/// deliberately non-UTF-8 `malformed.org` that must quarantine).
fn vault_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault")
}

/// The `orgsidian` binary, wired to a hermetic, per-test data + config store.
fn orgsidian(data_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("orgsidian").expect("orgsidian binary builds");
    // The index lives under this temp dir (the ORGSIDIAN_DATA_DIR override).
    cmd.env("ORGSIDIAN_DATA_DIR", data_dir.path());
    // Best-effort isolation of the `recent_vaults` settings write that vault
    // designation performs (init/rebuild): redirect HOME + XDG config/data so
    // no run pollutes real user settings. The index isolation above is what the
    // assertions depend on; this only keeps the suite a good citizen.
    cmd.env("HOME", data_dir.path());
    cmd.env("XDG_CONFIG_HOME", data_dir.path().join("config"));
    cmd.env("XDG_DATA_HOME", data_dir.path().join("data"));
    cmd
}

/// Parse the whole stdout as a single JSON object — fails if any progress or
/// log line leaked onto stdout alongside it (the `--json` scripting-clean
/// contract).
fn stdout_json(output: &[u8]) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(output);
    serde_json::from_str(&stdout)
        .expect("stdout is a single parseable JSON object and nothing else")
}

/// AC1: `init` on a fresh vault creates a stamped index and indexes every
/// `.org` file, verified by a following `stats --json`.
#[test]
fn init_then_stats_reports_every_indexed_file() {
    let data = TempDir::new().expect("temp data dir");

    let init = orgsidian(&data)
        .arg("index")
        .arg("init")
        .arg(vault_path())
        .assert()
        .success();
    let init_out = String::from_utf8_lossy(&init.get_output().stdout);
    assert!(
        init_out.contains("init complete") && init_out.contains("indexed"),
        "init prints a human summary:\n{init_out}"
    );

    let stats = orgsidian(&data)
        .arg("index")
        .arg("stats")
        .arg(vault_path())
        .arg("--json")
        .assert()
        .success();
    let json = stdout_json(&stats.get_output().stdout);

    // notes.org + tasks.org index cleanly; malformed.org quarantines (still a
    // `files` row) — so 3 files, 1 quarantined.
    assert_eq!(json["fileCount"], 3, "all three .org files get a files row");
    assert_eq!(
        json["quarantinedCount"], 1,
        "the non-UTF-8 file is quarantined"
    );
    assert!(
        json["headlineCount"].as_i64().unwrap() > 0,
        "headlines were indexed"
    );
    assert!(
        json["ftsDocCount"].as_i64().unwrap() > 0,
        "the FTS corpus is populated"
    );
    assert_eq!(json["schemaVersion"], 1, "schema version 1 is stamped");
    assert!(
        json["schemaAppliedAt"].as_str().is_some(),
        "schema applied_at is present"
    );
    assert!(
        json["lastIndexedAt"].as_str().is_some(),
        "a populated index has a last-indexed timestamp"
    );
}

/// AC1 re-run: a second `init` over an unchanged vault skips every file (the
/// incremental fast path) and still exits 0.
#[test]
fn init_rerun_skips_unchanged_files() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();

    let rerun = orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .success();
    let json = stdout_json(&rerun.get_output().stdout);
    assert_eq!(
        json["indexed"], 0,
        "nothing re-indexed on an unchanged re-run"
    );
    assert!(
        json["skipped"].as_i64().unwrap() >= 2,
        "the two clean files are skipped"
    );
}

/// AC2 (rebuild-identity): `rebuild` drops the DB and rebuilds it, and its
/// `stats` counts equal a from-scratch `init`.
#[test]
fn rebuild_matches_init_counts() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();
    let after_init = stats_json(&data);

    let rebuild = orgsidian(&data)
        .args(["index", "rebuild"])
        .arg(vault_path())
        .assert()
        .success();
    let rebuild_out = String::from_utf8_lossy(&rebuild.get_output().stdout);
    assert!(
        rebuild_out.contains("rebuild complete"),
        "rebuild prints its summary:\n{rebuild_out}"
    );

    let after_rebuild = stats_json(&data);
    for key in [
        "fileCount",
        "quarantinedCount",
        "headlineCount",
        "tagCount",
        "linkCount",
        "ftsDocCount",
    ] {
        assert_eq!(
            after_init[key], after_rebuild[key],
            "rebuild-identity: {key} must match a from-scratch init"
        );
    }
}

/// Helper: run `stats --json` and return the parsed object.
fn stats_json(data: &TempDir) -> serde_json::Value {
    let stats = orgsidian(data)
        .args(["index", "stats"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .success();
    stdout_json(&stats.get_output().stdout)
}

/// Helper: locate the single `index-*.sqlite3` the override placed under the
/// data dir (so a test can open it directly with rusqlite to inject corruption
/// or stale rows).
fn locate_index_db(data: &TempDir) -> PathBuf {
    std::fs::read_dir(data.path())
        .expect("data dir readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("index-") && name.ends_with(".sqlite3"))
        })
        .expect("init created an index database")
}

/// AC4: `integrity` on a healthy index passes every check and exits 0 (text
/// and `--json`).
#[test]
fn integrity_healthy_exits_zero() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();

    let human = orgsidian(&data)
        .args(["index", "integrity"])
        .arg(vault_path())
        .assert()
        .success();
    let human_out = String::from_utf8_lossy(&human.get_output().stdout);
    assert!(
        human_out.contains("integrity: OK"),
        "healthy integrity verdict:\n{human_out}"
    );

    let json = orgsidian(&data)
        .args(["index", "integrity"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .success();
    let report = stdout_json(&json.get_output().stdout);
    assert_eq!(report["ok"], true, "every check passed");
    assert!(
        report["checks"].as_array().unwrap().len() >= 4,
        "integrity_check + foreign_key_check + both FTS tables"
    );
}

/// AC6: with no index for the target vault, `stats` errors on stderr and exits
/// 1 (never 2, never a panic, never a created DB *or* base directory), with
/// empty stdout. The index store override points at a directory that does NOT
/// yet exist, so the read-only path is proven to provision nothing on refusal
/// (the `resolve_index_db_path` "creates nothing" contract).
#[test]
fn stats_without_index_errors_and_never_creates_a_db() {
    let data = TempDir::new().expect("temp data dir");
    // A base dir that does not exist yet — refusing to find an index must not
    // bring it into being.
    let store = data.path().join("index-store-not-yet-created");
    assert!(!store.exists(), "precondition: the store dir is absent");

    let assert = orgsidian(&data)
        .env("ORGSIDIAN_DATA_DIR", &store)
        .args(["index", "stats"])
        .arg(vault_path())
        .assert()
        .failure();
    let output = assert.get_output();
    assert!(output.stdout.is_empty(), "stdout stays empty on error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("index init"),
        "the error points at `index init`:\n{stderr}"
    );
    assert_ne!(
        output.status.code(),
        Some(2),
        "a missing index is not a usage error"
    );

    // Neither a DB nor its base directory was created by the read-only refusal.
    assert!(
        !store.exists(),
        "stats must not create the index base directory on refusal"
    );
}

/// AC4 (corrupt path, I/O matrix): an inconsistent index makes `integrity`
/// report each failing check and exit non-zero (never 2). Corruption is
/// deterministic and portable — after a healthy `init`, open the on-disk index
/// directly and insert an orphan `tags` row (foreign_keys OFF) so
/// `PRAGMA foreign_key_check` finds a violation.
#[test]
fn integrity_corrupt_index_exits_nonzero() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();

    // Locate the single `index-*.sqlite3` the override placed under the data dir.
    let db_path = std::fs::read_dir(data.path())
        .expect("data dir readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("index-") && n.ends_with(".sqlite3"))
        })
        .expect("init created an index database");

    // Corrupt: an orphan FK (headline_id with no matching headline).
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open index db");
        conn.pragma_update(None, "foreign_keys", "OFF")
            .expect("disable FK enforcement for the insert");
        conn.execute(
            "INSERT INTO tags (headline_id, tag, position) VALUES (999999, 'orphan', 0)",
            [],
        )
        .expect("insert orphan tag row");
    }

    let assert = orgsidian(&data)
        .args(["index", "integrity"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .failure();
    let output = assert.get_output();
    assert_ne!(
        output.status.code(),
        Some(2),
        "an integrity failure is not a usage error"
    );
    let report = stdout_json(&output.stdout);
    assert_eq!(report["ok"], false, "the corrupt index fails integrity");
    let failed: Vec<&serde_json::Value> = report["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .filter(|c| c["ok"] == false)
        .collect();
    assert!(
        failed.iter().any(|c| c["name"] == "foreign_key_check"),
        "the orphan FK is reported as a failing check:\n{report}"
    );
}

/// AC6: `integrity` with no index likewise errors and exits non-zero.
#[test]
fn integrity_without_index_errors() {
    let data = TempDir::new().expect("temp data dir");
    let assert = orgsidian(&data)
        .args(["index", "integrity"])
        .arg(vault_path())
        .assert()
        .failure();
    assert!(
        assert.get_output().stdout.is_empty(),
        "stdout stays empty on error"
    );
}

/// Edge: a missing `<vault>` positional is a clap usage error — exit 2.
#[test]
fn missing_vault_is_a_usage_error_exit_two() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "stats"])
        .assert()
        .failure()
        .code(2);
}

/// AC4 (corrupt FTS path): the integrity command's core purpose is catching
/// silent search corruption. Corrupt the FTS5 external-content index directly
/// via its shadow `_data` storage (the schema.rs shadow-table technique):
/// emptying `fts_content_data` breaks the index's internal doclist structure so
/// the FTS `'integrity-check'` command raises `fts5: corruption`. This trips the
/// FTS branch specifically (not `foreign_key_check`), proving
/// `fts_integrity_check` reports rather than swallows the failure — a silent
/// search-corruption regression would otherwise ship undetected.
#[test]
fn integrity_corrupt_fts_exits_nonzero() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();
    let db_path = locate_index_db(&data);

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open index db");
        conn.execute("DELETE FROM fts_content_data", [])
            .expect("empty the FTS content shadow storage to corrupt the index");
    }

    let assert = orgsidian(&data)
        .args(["index", "integrity"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .failure();
    let output = assert.get_output();
    assert_ne!(
        output.status.code(),
        Some(2),
        "an integrity failure is not a usage error"
    );
    let report = stdout_json(&output.stdout);
    assert_eq!(
        report["ok"], false,
        "the FTS-desynced index fails integrity"
    );

    let failed: Vec<String> = report["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .filter(|check| check["ok"] == false)
        .filter_map(|check| check["name"].as_str().map(str::to_owned))
        .collect();
    assert!(
        failed
            .iter()
            .any(|name| name == "fts_headlines" || name == "fts_content"),
        "an FTS consistency check must be among the failures — proves the FTS \
         branch runs, not just FK:\n{report}"
    );
}

/// AC2 anti-placebo (Story 1.9): `rebuild` must DROP + regenerate, not re-scan
/// in place. Inject a stale `files` row for a path that is not in the vault;
/// a true drop discards it. Without the drop (if `remove_index_files` were a
/// no-op) the incremental re-scan would leave the ghost row behind.
#[test]
fn rebuild_drops_stale_rows() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .assert()
        .success();
    let db_path = locate_index_db(&data);
    let ghost = "ghost-not-in-vault.org";

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open index db");
        conn.execute(
            "INSERT INTO files (path, mtime_ns, size_bytes, indexed_at) \
             VALUES (?1, 0, 0, '2000-01-01T00:00:00Z')",
            [ghost],
        )
        .expect("inject a stale files row");
    }
    // Sanity: the ghost is really there before the rebuild (fileCount 3 -> 4).
    assert_eq!(
        stats_json(&data)["fileCount"],
        4,
        "the injected stale row is present before rebuild"
    );

    orgsidian(&data)
        .args(["index", "rebuild"])
        .arg(vault_path())
        .assert()
        .success();

    // The DB was dropped + rebuilt at the same path; the ghost must be gone and
    // the count back to the from-scratch baseline of 3.
    assert_eq!(
        stats_json(&data)["fileCount"],
        3,
        "rebuild dropped the stale row (count returns to the init baseline)"
    );
    let rebuilt = locate_index_db(&data);
    let conn = rusqlite::Connection::open(&rebuilt).expect("open rebuilt index db");
    let ghost_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM files WHERE path = ?1",
            [ghost],
            |row| row.get(0),
        )
        .expect("query ghost row");
    assert_eq!(
        ghost_rows, 0,
        "the stale ghost files row did not survive rebuild"
    );
}

/// I/O matrix (error cell): a `<vault>` that does not exist is an
/// `OrgError::Vault` — exit 1 with a stderr message, NOT clap's usage-error
/// exit 2 and never a panic.
#[test]
fn init_with_invalid_vault_errors_not_usage() {
    let data = TempDir::new().expect("temp data dir");
    let missing = data.path().join("no-such-vault");

    let assert = orgsidian(&data)
        .args(["index", "init"])
        .arg(&missing)
        .assert()
        .failure();
    let output = assert.get_output();
    assert_ne!(
        output.status.code(),
        Some(2),
        "a missing vault path is a vault error, not a clap usage error"
    );
    assert!(
        !output.stderr.is_empty(),
        "the vault error is reported on stderr"
    );
}

/// AC5 (`--json` clean during ACTIVE indexing): a fresh-vault `init --json`
/// must parse as a single JSON object — proving the progress callback is a
/// no-op while files are genuinely being indexed (not only on the all-skipped
/// re-run). Asserts `indexed >= 2` so the run really indexed the clean files.
#[test]
fn init_json_is_clean_during_active_indexing() {
    let data = TempDir::new().expect("temp data dir");
    let init = orgsidian(&data)
        .args(["index", "init"])
        .arg(vault_path())
        .arg("--json")
        .assert()
        .success();
    let json = stdout_json(&init.get_output().stdout);
    assert!(
        json["indexed"].as_i64().unwrap() >= 2,
        "a fresh init actively indexes the clean files: {json}"
    );
}

/// `rebuild` with no pre-existing index must succeed — `remove_index_files`
/// tolerates the absent DB (+ `-wal`/`-shm`) and then builds fresh.
#[test]
fn rebuild_without_prior_index_succeeds() {
    let data = TempDir::new().expect("temp data dir");
    orgsidian(&data)
        .args(["index", "rebuild"])
        .arg(vault_path())
        .assert()
        .success();
    // Sanity: the fresh rebuild produced a populated index.
    assert_eq!(
        stats_json(&data)["fileCount"],
        3,
        "rebuild-from-nothing indexes every .org file"
    );
}

/// AC5 anti-placebo guard: the build.rs-generated man page for the `index`
/// subcommand exists in-tree, is non-empty, and names `index`.
#[test]
fn man_page_is_generated_and_names_index() {
    let man_page = Path::new(env!("CARGO_MANIFEST_DIR")).join("man/orgsidian-index.1");
    let contents = std::fs::read_to_string(&man_page)
        .unwrap_or_else(|err| panic!("man page missing at {}: {err}", man_page.display()));
    assert!(!contents.is_empty(), "man page must be non-empty");
    assert!(
        contents.contains("index"),
        "man page must name the `index` subcommand"
    );
}
