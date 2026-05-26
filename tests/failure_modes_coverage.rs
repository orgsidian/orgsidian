//! Story 1.11 — LD-41 failure-mode coverage gate.
//!
//! Default mode (v0.1 → v0.5 Beta): advisory; logs unimplemented categories
//! and passes. Strict mode (`ORGSIDIAN_FAILURE_MODE_STRICT=1`): fails CI if
//! any LD-41 category has only `#[ignore]` placeholders. The strict-mode
//! flip is a v0.5-Beta release-prep story owned, NOT Story 1.11.

const HARNESS_SRC: &str = include_str!("./failure_modes.rs");

/// Expected LD-41 category count (architecture.md L1196-L1209 catalog).
/// Pinned here so a future contributor cannot remove a category from the
/// harness without breaking the coverage gate — forces a coordinated update of
/// architecture.md + harness + this constant + docs/failure-modes/coverage-matrix.md.
const EXPECTED_LD_41_CATEGORIES: usize = 10;

/// Returns `(unimplemented_categories, total_categories)` parsed from the
/// `#[ignore = "implemented in Epic N"]` annotations in HARNESS_SRC.
///
/// Iteration order: top-down through the source file. A `Vec` (not a HashMap)
/// preserves source order so downstream consumers (e.g., the matrix generator)
/// can rely on category iteration order being deterministic.
fn scan_categories() -> (Vec<String>, usize) {
    let lines: Vec<&str> = HARNESS_SRC.lines().collect();
    let mut categories: Vec<String> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Only count attribute lines that bear the literal "implemented in Epic"
        // marker; comments and doc-comments are skipped naturally because they
        // start with `//` rather than `#[ignore`.
        if !trimmed.starts_with("#[ignore = \"implemented in Epic ") {
            continue;
        }
        // The owning `fn <name>` is on a subsequent line (allow blank lines
        // between attribute and fn signature even though current harness has none).
        for next in lines.iter().skip(idx + 1) {
            let next_trim = next.trim_start();
            if next_trim.is_empty() {
                continue;
            }
            if let Some(rest) = next_trim.strip_prefix("fn ") {
                if let Some(paren) = rest.find('(') {
                    categories.push(rest[..paren].trim().to_string());
                }
            }
            break;
        }
    }

    let total = categories.len();
    (categories, total)
}

#[test]
fn ld_41_categories_have_real_implementations() {
    let (unimplemented, total) = scan_categories();
    let strict = std::env::var("ORGSIDIAN_FAILURE_MODE_STRICT")
        .map(|v| v == "1")
        .unwrap_or(false);

    if strict {
        assert!(
            unimplemented.is_empty(),
            "LD-41 strict-coverage gate: {} of {} categories still have only \
             #[ignore] placeholders: {:?}. Implement real fault-injection \
             tests in the owning epics before merging post-v0.5 Beta.",
            unimplemented.len(),
            total,
            unimplemented,
        );
    } else {
        eprintln!(
            "LD-41 advisory: {}/{} failure-mode categories still on #[ignore] \
             placeholders: {:?}. Strict-mode CI gate flips post-v0.5 Beta.",
            unimplemented.len(),
            total,
            unimplemented,
        );
    }
}

#[test]
fn failure_mode_count_matches_ld_41_catalog() {
    let (_unimplemented, total) = scan_categories();
    assert_eq!(
        total, EXPECTED_LD_41_CATEGORIES,
        "LD-41 catalog drift: tests/failure_modes.rs has {} categories, \
         architecture.md L1196-L1209 has {}. Update both in lockstep + \
         the EXPECTED_LD_41_CATEGORIES constant + docs/failure-modes/coverage-matrix.md.",
        total, EXPECTED_LD_41_CATEGORIES,
    );
}
