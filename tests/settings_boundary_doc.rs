//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Drift guard for `docs/architecture/settings-boundary.md` per Story 1.18 AC5.
//! Asserts:
//!   (a) all 6 required section headings are present verbatim;
//!   (b) the `tauri-plugin-store` ephemeral allowlist contains exactly 4 entries;
//!   (c) every `VaultSettings` / `GlobalSettings` field name appears at least
//!       once in the doc (catches the case where a future field-add forgets to
//!       update the boundary doc).

use std::fs;
use std::path::PathBuf;

fn read_doc() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // CARGO_MANIFEST_DIR is the test-host crate (orgsidian-core); the doc lives
    // two levels up at the workspace root.
    let path: PathBuf = PathBuf::from(manifest_dir)
        .join("../..")
        .join("docs/architecture/settings-boundary.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn required_section_headings_present() {
    let doc = read_doc();
    let required: &[&str] = &[
        "# Settings Store Boundary (LD-40 + FR-23)",
        "## Authoritative Settings (TOML, OQ-7 dual-surface)",
        "## Ephemeral UI State (`tauri-plugin-store`-allowed)",
        "## Forbidden Patterns",
        "## Adding a New Setting (decision tree)",
        "## References",
    ];
    for heading in required {
        assert!(
            doc.contains(heading),
            "boundary doc missing required heading: {heading:?}"
        );
    }
}

#[test]
fn ephemeral_allowlist_has_exactly_four_entries() {
    let doc = read_doc();
    let expected: &[&str] = &[
        "lastOpenFile",
        "windowGeometry",
        "tutorialProgress",
        "lastVaultPath",
    ];
    for key in expected {
        assert!(
            doc.contains(&format!("`{key}`")),
            "boundary doc missing ephemeral allowlist entry: {key}"
        );
    }
    // Drift guard: surface any new ephemeral key snuck in past review.
    // The four entries above are the LD-40 closed allowlist. Any addition
    // requires a deliberate update to this test + the doc.
    let allowlist_section = doc
        .split("## Ephemeral UI State (`tauri-plugin-store`-allowed)")
        .nth(1)
        .expect("ephemeral section present")
        .split("## Forbidden Patterns")
        .next()
        .expect("forbidden section follows ephemeral section");
    // Count rows of the markdown table by looking for the pipe-bordered lines
    // that start with `| \`` (the type/key column). Header + separator are
    // excluded because they don't start with the backtick.
    let row_count = allowlist_section
        .lines()
        .filter(|l| l.trim_start().starts_with("| `"))
        .count();
    assert_eq!(
        row_count, 4,
        "ephemeral allowlist must have exactly 4 rows; found {row_count} in:\n{allowlist_section}"
    );
}

#[test]
fn schema_field_names_present() {
    let doc = read_doc();
    // Every public field on VaultSettings + GlobalSettings (excluding the
    // `_extra` forward-compat catch-all) must appear at least once in the doc.
    let required_fields: &[&str] = &[
        // VaultSettings
        "schema_version",
        "keybindings",
        "theme",
        "capture_hotkey",
        "agenda_presets",
        "dismissed_coaching",
        "ui_mode",
        "today_dashboard",
        // GlobalSettings
        "recent_vaults",
        "default_language",
        "default_theme",
    ];
    for field in required_fields {
        assert!(
            doc.contains(field),
            "boundary doc missing schema field reference: {field}"
        );
    }
}
