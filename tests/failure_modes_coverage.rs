//! Story 1.11 — LD-41 failure-mode coverage gate.
//!
//! Default mode (v0.1 → v0.5 Beta): advisory; logs unimplemented categories
//! and passes. Strict mode (`ORGSIDIAN_FAILURE_MODE_STRICT=1`): fails CI if
//! any LD-41 category has only `#[ignore]` placeholders. The strict-mode
//! flip is a v0.5-Beta release-prep story owned, NOT Story 1.11.

const HARNESS_SRC: &str = include_str!("./failure_modes.rs");

/// Expected remaining `#[ignore]` placeholder count in `tests/failure_modes.rs`.
///
/// Starts at the LD-41 catalog size (10 rows — architecture.md L1196-L1209) and
/// MUST be decremented by 1 in each downstream LD-41 implementation story as it
/// removes the matching `#[ignore = "implemented in Epic N"]` attribute from a
/// placeholder fn. Reaches 0 at the v0.5-Beta release-prep cutoff alongside the
/// strict-mode gate flip.
///
/// Coordinated-update touchpoints: this constant + the matching `#[ignore]`
/// removal in `tests/failure_modes.rs` + regen of `docs/failure-modes/coverage-matrix.md`.
/// A catalog growth (new LD-41 row) instead INCREMENTS this AND requires
/// adding a placeholder fn AND updating architecture.md L1196-L1209.
const EXPECTED_REMAINING_PLACEHOLDERS: usize = 10;

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
    let (placeholders, _) = scan_categories();
    assert_eq!(
        placeholders.len(),
        EXPECTED_REMAINING_PLACEHOLDERS,
        "LD-41 placeholder drift: tests/failure_modes.rs has {} remaining \
         #[ignore] placeholders, EXPECTED_REMAINING_PLACEHOLDERS is {}. \
         If you removed an #[ignore] (implementing a real test), decrement \
         the constant. If you added a new LD-41 category, increment it AND \
         update architecture.md L1196-L1209 + docs/failure-modes/coverage-matrix.md.",
        placeholders.len(),
        EXPECTED_REMAINING_PLACEHOLDERS,
    );
}
