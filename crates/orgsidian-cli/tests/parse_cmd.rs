//! Integration tests for `orgsidian parse <file> [--json]` (Story 2.8,
//! LD-27 first CLI command) via `assert_cmd` against the real binary.
//!
//! Posture: human-readable output is asserted with loose substrings (the
//! rendered format is explicitly NOT a contract); JSON output is asserted
//! through parsed `serde_json::Value` field access, never full-document
//! string equality (`Headline::properties` is an unordered `HashMap`).

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// Absolute path to the co-located fixture, resolved against this crate's
/// manifest dir so the tests pass from any working directory.
fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parse_cmd.org")
}

/// The `orgsidian` binary (the bin name, not the crate name).
fn orgsidian() -> Command {
    Command::cargo_bin("orgsidian").expect("orgsidian binary builds")
}

/// AC3: human mode exits 0 and surfaces title, TODO keyword, tags, child
/// headline, and the SCHEDULED stamp (loose substrings, not a golden file).
#[test]
fn human_mode_renders_title_todo_tags_and_children() {
    let assert = orgsidian()
        .arg("parse")
        .arg(fixture_path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    for needle in [
        "First headline",
        "TODO",
        "work",
        "urgent",
        "Child headline",
        "<2026-06-10 Wed 10:00>",
    ] {
        assert!(
            stdout.contains(needle),
            "stdout missing {needle:?}:\n{stdout}"
        );
    }
}

/// AC4: `--json` exits 0 and stdout is parseable JSON with real structure —
/// headline title, camelCase `todoState`, tags, nested child — and nothing
/// else on stdout (scripting-clean).
#[test]
fn json_mode_emits_camel_case_ast() {
    let assert = orgsidian()
        .arg("parse")
        .arg(fixture_path())
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout is valid JSON and nothing else");

    let headline = &doc["headlines"][0];
    assert_eq!(headline["title"], "First headline");
    assert_eq!(
        headline["todoState"]["keyword"], "TODO",
        "camelCase `todoState` key per the project-wide wire convention"
    );
    assert!(
        headline.get("todo_state").is_none(),
        "snake_case keys must not leak into the JSON output"
    );
    assert_eq!(headline["tags"][0]["name"], "work");
    assert_eq!(headline["children"][0]["title"], "Child headline");
    assert_eq!(headline["scheduled"]["date"], "2026-06-10");
    assert_eq!(doc["preamble"]["directives"][0]["name"], "TITLE");
}

/// AC3 error posture: unreadable file → non-zero exit, message on stderr,
/// empty stdout (distinct from clap's own usage-error exit 2).
#[test]
fn missing_file_fails_on_stderr_with_empty_stdout() {
    let assert = orgsidian()
        .arg("parse")
        .arg("tests/fixtures/does-not-exist.org")
        .assert()
        .failure();
    let output = assert.get_output();
    assert!(output.stdout.is_empty(), "stdout must stay empty on error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does-not-exist.org"),
        "stderr names the unreadable file:\n{stderr}"
    );
    assert_ne!(
        output.status.code(),
        Some(2),
        "I/O failure is not a usage error"
    );
}

/// AC5 anti-placebo guard (Story 1.9 discipline): the build.rs-generated man
/// page exists in-tree, is non-empty, and names the `parse` subcommand — a
/// future build.rs regression cannot silently stop generating.
#[test]
fn man_page_is_generated_and_names_the_subcommand() {
    let man_page = Path::new(env!("CARGO_MANIFEST_DIR")).join("man/orgsidian-parse.1");
    let contents = std::fs::read_to_string(&man_page)
        .unwrap_or_else(|err| panic!("man page missing at {}: {err}", man_page.display()));
    assert!(!contents.is_empty(), "man page must be non-empty");
    assert!(
        contents.contains("parse"),
        "man page must name the `parse` subcommand"
    );
}

/// AC6 recommended cross-check (FR-2 through the public façade): analyzing
/// the fixture and serializing it back reproduces the fixture bytes.
#[test]
fn fixture_round_trips_through_the_core_facade() {
    let source = std::fs::read_to_string(fixture_path()).expect("fixture readable");
    let document = orgsidian_core::parser::analyze(&source).expect("analyze is total");
    assert_eq!(
        orgsidian_core::parser::serialize_document(&document),
        source,
        "serialize_document must be byte-identical to the analyzed source"
    );
}
