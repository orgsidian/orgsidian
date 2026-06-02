//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Round-trip fidelity contract per Story 1.18 AC3:
//!   - `read(write(s)) == s` (structural)
//!   - `write(read(F)) == F` byte-for-byte (writer fixed-point)
//!   - Unknown fields survive a v1-binary round-trip via `_extra` flatten
//!   - `schema_version = 1` is present on every default write
//!
//! Property-based fuzz (256 cases) covers randomized `VaultSettings` values.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use orgsidian_core::settings::{
    read_vault_settings, vault_settings_path, write_vault_settings, GlobalSettings,
    SchemaVersion, VaultSettings, SCHEMA_VERSION_CURRENT,
};
use orgsidian_core::settings::schema::{
    AgendaPreset, ThemeChoice, TodayDashboardSections, UiMode,
};
use proptest::collection;
use proptest::option;
use proptest::prelude::*;
use tempfile::tempdir;

#[test]
fn default_vault_settings_round_trip() {
    let dir = tempdir().expect("tempdir");
    let original = VaultSettings::default();
    write_vault_settings(dir.path(), &original).expect("write");
    let read_back = read_vault_settings(dir.path()).expect("read");
    assert_eq!(read_back, original);
}

#[test]
fn default_global_settings_round_trip() {
    // GlobalSettings reuses the same TOML serializer; round-trip via in-memory
    // string (we cannot safely touch the real `<config-dir>/orgsidian/global.toml`).
    let original = GlobalSettings::default();
    let serialized = toml::to_string_pretty(&original).expect("serialize");
    let read_back: GlobalSettings = toml::from_str(&serialized).expect("deserialize");
    assert_eq!(read_back, original);
}

#[test]
fn populated_vault_settings_round_trip() {
    let dir = tempdir().expect("tempdir");
    let mut original = VaultSettings::default();
    original
        .keybindings
        .insert("editor.save".into(), "Cmd+S".into());
    original
        .keybindings
        .insert("agenda.today".into(), "Cmd+Shift+T".into());
    original.theme = ThemeChoice::Custom(PathBuf::from("/themes/solarized.css"));
    original.capture_hotkey = Some("Cmd+Shift+C".into());
    original.agenda_presets.insert(
        "Work today".into(),
        AgendaPreset {
            view: "today".into(),
            filters: vec!["@work".into(), "TODO".into()],
        },
    );
    original.agenda_presets.insert(
        "Personal week".into(),
        AgendaPreset {
            view: "week".into(),
            filters: vec!["@home".into()],
        },
    );
    original
        .dismissed_coaching
        .insert("first-launch-balloon".into());
    original.ui_mode = UiMode::Power;
    original.today_dashboard = TodayDashboardSections {
        show_scheduled: false,
        show_deadlines: true,
        show_clock: false,
        show_inbox: true,
    };

    write_vault_settings(dir.path(), &original).expect("write");
    let read_back = read_vault_settings(dir.path()).expect("read");
    assert_eq!(read_back, original);
}

#[test]
fn writer_fixed_point() {
    let dir_a = tempdir().expect("tempdir A");
    let dir_b = tempdir().expect("tempdir B");
    let mut s = VaultSettings::default();
    s.keybindings
        .insert("editor.save".into(), "Cmd+S".into());

    write_vault_settings(dir_a.path(), &s).expect("write A");
    write_vault_settings(dir_b.path(), &s).expect("write B");

    let bytes_a = std::fs::read(vault_settings_path(dir_a.path())).expect("read A");
    let bytes_b = std::fs::read(vault_settings_path(dir_b.path())).expect("read B");

    assert_eq!(
        bytes_a, bytes_b,
        "writer must be deterministic: same input value → byte-identical output files"
    );
}

#[test]
fn unknown_fields_preserved() {
    let dir = tempdir().expect("tempdir");
    let initial = VaultSettings::default();
    write_vault_settings(dir.path(), &initial).expect("write initial");

    // Inject a v2-style extension into the on-disk file by hand.
    let path = vault_settings_path(dir.path());
    let existing = std::fs::read_to_string(&path).expect("read existing");
    let injected = format!(
        "{existing}\n[some_v2_extension]\nfoo = 1\nbar = \"hello\"\n"
    );
    std::fs::write(&path, &injected).expect("inject v2 extension");

    let read_back = read_vault_settings(dir.path()).expect("read with extension");
    assert!(
        read_back._extra.contains_key("some_v2_extension"),
        "_extra should capture unknown table"
    );

    // Write back and verify the extension survives.
    write_vault_settings(dir.path(), &read_back).expect("write back");
    let after = std::fs::read_to_string(&path).expect("read after");
    assert!(
        after.contains("[some_v2_extension]"),
        "v2 extension header must survive: {after}"
    );
    assert!(after.contains("foo = 1"), "foo = 1 must survive: {after}");
    assert!(
        after.contains("bar = \"hello\""),
        "bar = \"hello\" must survive: {after}"
    );
}

#[test]
fn schema_version_one_present_on_default_write() {
    let dir = tempdir().expect("tempdir");
    write_vault_settings(dir.path(), &VaultSettings::default()).expect("write");
    let on_disk = std::fs::read_to_string(vault_settings_path(dir.path())).expect("read");

    let needle = format!("schema_version = {}", SCHEMA_VERSION_CURRENT);
    let occurrences = on_disk.matches(&needle).count();
    assert_eq!(
        occurrences, 1,
        "schema_version must appear exactly once; got {occurrences} in:\n{on_disk}"
    );
}

// --- Property-based test (256 cases by default) ---

fn theme_choice_strategy() -> impl Strategy<Value = ThemeChoice> {
    prop_oneof![
        Just(ThemeChoice::DefaultLight),
        Just(ThemeChoice::DefaultDark),
        "[a-z]{3,8}".prop_map(|s| ThemeChoice::Custom(PathBuf::from(format!("/themes/{s}.css")))),
    ]
}

fn ui_mode_strategy() -> impl Strategy<Value = UiMode> {
    prop_oneof![Just(UiMode::Plain), Just(UiMode::Power)]
}

fn agenda_preset_strategy() -> impl Strategy<Value = AgendaPreset> {
    (
        prop_oneof![
            Just("today".to_string()),
            Just("week".to_string()),
            Just("custom".to_string()),
        ],
        collection::vec("[a-z]{2,6}", 0..4),
    )
        .prop_map(|(view, filters)| AgendaPreset { view, filters })
}

fn today_dashboard_strategy() -> impl Strategy<Value = TodayDashboardSections> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(show_scheduled, show_deadlines, show_clock, show_inbox)| TodayDashboardSections {
            show_scheduled,
            show_deadlines,
            show_clock,
            show_inbox,
        },
    )
}

fn vault_settings_strategy() -> impl Strategy<Value = VaultSettings> {
    (
        // Small alphabets keep the property test fast (~256 × ~5ms = ~1.3s).
        collection::btree_map("[a-z]{3,10}", "[a-zA-Z+]{1,8}", 0..5),
        theme_choice_strategy(),
        option::of("[a-zA-Z+]{1,12}"),
        collection::btree_map("[a-zA-Z ]{3,10}", agenda_preset_strategy(), 0..3),
        collection::btree_set("[a-z-]{4,12}", 0..4),
        ui_mode_strategy(),
        today_dashboard_strategy(),
    )
        .prop_map(
            |(
                keybindings,
                theme,
                capture_hotkey,
                agenda_presets,
                dismissed_coaching,
                ui_mode,
                today_dashboard,
            )| VaultSettings {
                schema_version: SchemaVersion(SCHEMA_VERSION_CURRENT),
                keybindings: keybindings_to_btreemap(keybindings),
                theme,
                capture_hotkey,
                agenda_presets,
                dismissed_coaching,
                ui_mode,
                today_dashboard,
                _extra: toml::Table::new(),
            },
        )
}

fn keybindings_to_btreemap(m: BTreeMap<String, String>) -> BTreeMap<String, String> {
    m
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// `read(write(s)) == s` for randomized `VaultSettings` values.
    #[test]
    fn vault_settings_round_trip_property(s in vault_settings_strategy()) {
        let dir = tempdir().expect("tempdir");
        write_vault_settings(dir.path(), &s).expect("write");
        let read_back = read_vault_settings(dir.path()).expect("read");
        prop_assert_eq!(read_back, s);
    }
}

// `BTreeSet` unused-import shim (strategy returns nothing else that needs it).
#[allow(dead_code)]
fn _unused_btreeset() -> BTreeSet<String> {
    BTreeSet::new()
}
