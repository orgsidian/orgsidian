//! Implements FR-7 (Story 7.5 saved agenda filter presets).
//!
//! CRUD over the named agenda filter presets held in the per-Vault TOML
//! settings store (`<Vault>/.orgsidian/settings.toml`, `[agenda_presets]`),
//! plus the two default presets seeded on first launch. This is a thin,
//! read-modify-write layer over [`crate::settings::read_vault_settings`] /
//! [`crate::settings::write_vault_settings`] — the settings module owns the
//! file format, atomic write, and default-on-missing contract (LD-40); this
//! module owns only the preset semantics. Modelled on `coaching.rs`, which is
//! the same read-modify-write shape over a different per-Vault store.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::OrgError;
use crate::settings::schema::AgendaPreset;
use crate::settings::{read_vault_settings, write_vault_settings};
use crate::Result as OrgResult;

/// Built-in preset name: DONE headlines completed in the rolling last 7 days.
pub const DONE_THIS_WEEK: &str = "Done This Week";

/// Built-in preset name: DONE headlines completed in the rolling last 30 days.
pub const DONE_THIS_MONTH: &str = "Done This Month";

/// The reserved default preset names — the single source of truth shared by the
/// seeding logic ([`default_agenda_presets`]) and the save-time guard
/// ([`save_agenda_preset`] / [`delete_agenda_preset`]). These two evergreen
/// rolling defaults are protected: a user save may not overwrite one (which
/// would permanently clobber the built-in, since seeding never re-runs once the
/// flag is set), and only deleting one of them suppresses a future re-seed.
pub const RESERVED_PRESET_NAMES: [&str; 2] = [DONE_THIS_WEEK, DONE_THIS_MONTH];

/// Whether `name` is one of the two reserved evergreen default preset names.
fn is_reserved_preset_name(name: &str) -> bool {
    RESERVED_PRESET_NAMES.contains(&name)
}

/// Map a settings-store failure to an `OrgError`. No `From<SettingsError>`
/// exists (the settings error type is private to that module's surface), so —
/// exactly as `coaching.rs` maps its store errors — a preset persistence
/// failure is reported as a disk concern.
fn presets_io(err: impl std::fmt::Display) -> OrgError {
    OrgError::Io {
        reason: format!("agenda-presets settings store: {err}"),
    }
}

/// The two default presets shipped out of the box (FR-7 enhancement,
/// 2026-05-20): `Done This Week` (rolling 7 days) and `Done This Month`
/// (rolling 30 days). Both recall the Custom Agenda in completion mode
/// (`completed = true`, `todo_state = "DONE"`), so they surface headlines
/// whose `CLOSED:` date falls in the rolling window.
pub fn default_agenda_presets() -> BTreeMap<String, AgendaPreset> {
    let done = |rolling_days: u32| AgendaPreset {
        view: "custom".to_string(),
        start: None,
        end: None,
        rolling_days: Some(rolling_days),
        tag: None,
        todo_state: Some("DONE".to_string()),
        file_path_glob: None,
        completed: true,
    };

    let mut presets = BTreeMap::new();
    presets.insert(DONE_THIS_WEEK.to_string(), done(7));
    presets.insert(DONE_THIS_MONTH.to_string(), done(30));
    presets
}

/// List the saved agenda presets for `vault_root`, seeding the built-in
/// defaults on first ever call.
///
/// Seeding is idempotent and delete-respecting: the defaults are inserted only
/// while `agenda_presets_seeded` is still `false`, and each is inserted only
/// if its name is not already present. Once seeded, the flag is set and
/// persisted, so a user who later deletes a default never has it resurrected.
/// The seed write is skipped entirely once the flag is set, so a plain list is
/// read-only after first launch.
///
/// # Errors
///
/// [`OrgError::Io`] if the settings file cannot be read or (on first seed)
/// written.
pub fn list_agenda_presets(vault_root: &Path) -> OrgResult<BTreeMap<String, AgendaPreset>> {
    let mut settings = read_vault_settings(vault_root).map_err(presets_io)?;

    if !settings.agenda_presets_seeded {
        for (name, preset) in default_agenda_presets() {
            settings.agenda_presets.entry(name).or_insert(preset);
        }
        settings.agenda_presets_seeded = true;
        write_vault_settings(vault_root, &settings).map_err(presets_io)?;
    }

    Ok(settings.agenda_presets)
}

/// Upsert a preset under `name` (replacing any existing preset of the same
/// name) and persist.
///
/// Rejects a save whose `name` matches one of the two reserved evergreen
/// default names ([`RESERVED_PRESET_NAMES`], case-sensitive exact match): those
/// built-in rolling defaults must survive, and once `agenda_presets_seeded` is
/// set they are never re-seeded, so a plain overwrite would clobber the default
/// permanently. The rejection is an [`OrgError::Vault`] — the same user-facing
/// channel every preset command already reports through (e.g. the no-active-vault
/// error), so the sidebar surfaces its message inline with no new error kind.
///
/// Deliberately does NOT touch `agenda_presets_seeded`: seeding is owned by
/// [`list_agenda_presets`]. That way saving a preset before the first `list`
/// (an unusual order, but possible for a non-UI caller) still lets the built-in
/// defaults seed on the next `list` alongside the user's preset, rather than
/// suppressing them.
///
/// # Errors
///
/// [`OrgError::Vault`] if `name` is a reserved default preset name;
/// [`OrgError::Io`] if the settings file cannot be read or written.
pub fn save_agenda_preset(vault_root: &Path, name: &str, preset: AgendaPreset) -> OrgResult<()> {
    if is_reserved_preset_name(name) {
        return Err(OrgError::Vault {
            reason: format!(
                "\"{name}\" is a reserved default preset name and cannot be overwritten; \
                 choose a different name"
            ),
        });
    }
    let mut settings = read_vault_settings(vault_root).map_err(presets_io)?;
    settings.agenda_presets.insert(name.to_string(), preset);
    write_vault_settings(vault_root, &settings).map_err(presets_io)?;
    Ok(())
}

/// Remove the preset named `name` and persist. A missing name is a no-op
/// (still `Ok`).
///
/// Only deleting one of the two reserved default presets sets
/// `agenda_presets_seeded`, so that a removed default is not resurrected by the
/// next `list` re-seeding it. Deleting a *user* preset leaves the flag untouched
/// — mirroring the same narrow-scoping the hardening pass applied to
/// [`save_agenda_preset`]. The previous blanket "set the flag on any delete"
/// was over-broad: deleting a normal preset before the first `list` would
/// permanently suppress both built-in defaults.
///
/// # Errors
///
/// [`OrgError::Io`] if the settings file cannot be read or written.
pub fn delete_agenda_preset(vault_root: &Path, name: &str) -> OrgResult<()> {
    let mut settings = read_vault_settings(vault_root).map_err(presets_io)?;
    if settings.agenda_presets.remove(name).is_some() {
        // Only removing a built-in default must suppress a future re-seed; a
        // user preset's deletion must not touch the flag (that would clobber the
        // defaults for a vault that has not seeded them yet).
        if is_reserved_preset_name(name) {
            settings.agenda_presets_seeded = true;
        }
        write_vault_settings(vault_root, &settings).map_err(presets_io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_are_the_two_named_done_presets() {
        let defaults = default_agenda_presets();
        assert_eq!(defaults.len(), 2);
        let week = &defaults[DONE_THIS_WEEK];
        assert_eq!(week.rolling_days, Some(7));
        assert_eq!(week.todo_state.as_deref(), Some("DONE"));
        assert!(week.completed);
        assert_eq!(defaults[DONE_THIS_MONTH].rolling_days, Some(30));
    }

    #[test]
    fn first_list_seeds_defaults_and_persists() {
        let dir = tempdir().expect("tempdir");
        let presets = list_agenda_presets(dir.path()).expect("list seeds");
        assert!(presets.contains_key(DONE_THIS_WEEK));
        assert!(presets.contains_key(DONE_THIS_MONTH));

        // Persisted: the flag is now set on disk.
        let settings = read_vault_settings(dir.path()).expect("read back");
        assert!(settings.agenda_presets_seeded);
        assert_eq!(settings.agenda_presets.len(), 2);
    }

    #[test]
    fn deleted_default_is_not_resurrected_on_next_list() {
        let dir = tempdir().expect("tempdir");
        list_agenda_presets(dir.path()).expect("first list seeds");
        delete_agenda_preset(dir.path(), DONE_THIS_WEEK).expect("delete a default");

        let presets = list_agenda_presets(dir.path()).expect("second list");
        assert!(!presets.contains_key(DONE_THIS_WEEK));
        assert!(presets.contains_key(DONE_THIS_MONTH));
    }

    #[test]
    fn save_then_list_round_trips_a_custom_preset() {
        let dir = tempdir().expect("tempdir");
        let preset = AgendaPreset {
            view: "custom".to_string(),
            start: Some("2026-09-01".to_string()),
            end: Some("2026-09-30".to_string()),
            rolling_days: None,
            tag: Some("home".to_string()),
            todo_state: Some("NEXT".to_string()),
            file_path_glob: Some("projects/*".to_string()),
            completed: false,
        };
        save_agenda_preset(dir.path(), "@home this month", preset.clone()).expect("save");

        let presets = list_agenda_presets(dir.path()).expect("list");
        assert_eq!(presets.get("@home this month"), Some(&preset));
    }

    #[test]
    fn saving_before_the_first_list_still_seeds_the_defaults() {
        let dir = tempdir().expect("tempdir");
        // A user (or non-UI caller) saves a preset before ever listing.
        save_agenda_preset(dir.path(), "My preset", AgendaPreset::default()).expect("save");

        // The next list still seeds the built-in defaults alongside it, rather
        // than suppressing them.
        let presets = list_agenda_presets(dir.path()).expect("list");
        assert!(presets.contains_key("My preset"));
        assert!(presets.contains_key(DONE_THIS_WEEK));
        assert!(presets.contains_key(DONE_THIS_MONTH));
    }

    #[test]
    fn deleting_a_missing_preset_is_ok() {
        let dir = tempdir().expect("tempdir");
        list_agenda_presets(dir.path()).expect("seed");
        delete_agenda_preset(dir.path(), "does-not-exist").expect("no-op delete is Ok");
    }

    #[test]
    fn saving_a_reserved_default_name_is_rejected_and_leaves_the_default_intact() {
        let dir = tempdir().expect("tempdir");
        // Seed the built-in defaults, then capture the real `Done This Week`.
        let seeded = list_agenda_presets(dir.path()).expect("seed");
        let original = seeded
            .get(DONE_THIS_WEEK)
            .cloned()
            .expect("default is seeded");

        // Attempt to overwrite it with an entirely different (bogus) preset.
        let usurper = AgendaPreset {
            view: "custom".to_string(),
            start: Some("2000-01-01".to_string()),
            end: Some("2000-01-02".to_string()),
            rolling_days: None,
            tag: Some("nope".to_string()),
            todo_state: None,
            file_path_glob: None,
            completed: false,
        };
        let err = save_agenda_preset(dir.path(), DONE_THIS_WEEK, usurper)
            .expect_err("saving a reserved default name must be rejected");
        assert!(
            matches!(err, OrgError::Vault { .. }),
            "expected OrgError::Vault, got {err:?}"
        );

        // The real default is untouched on the next list.
        let after = list_agenda_presets(dir.path()).expect("list after rejected save");
        assert_eq!(
            after.get(DONE_THIS_WEEK),
            Some(&original),
            "the reserved default must survive a rejected overwrite"
        );
    }

    #[test]
    fn both_reserved_default_names_are_rejected() {
        let dir = tempdir().expect("tempdir");
        for name in RESERVED_PRESET_NAMES {
            let err = save_agenda_preset(dir.path(), name, AgendaPreset::default())
                .expect_err("reserved name rejected");
            assert!(matches!(err, OrgError::Vault { .. }));
        }
    }

    #[test]
    fn saving_a_normal_name_still_works() {
        let dir = tempdir().expect("tempdir");
        save_agenda_preset(dir.path(), "Done Yesterday", AgendaPreset::default())
            .expect("a non-reserved name saves fine");
        let presets = list_agenda_presets(dir.path()).expect("list");
        assert!(presets.contains_key("Done Yesterday"));
    }

    #[test]
    fn deleting_a_non_default_before_first_list_does_not_suppress_defaults() {
        let dir = tempdir().expect("tempdir");
        // A user preset saved and then deleted, both BEFORE any `list` seeds the
        // defaults. The delete must NOT set the seeded flag.
        save_agenda_preset(dir.path(), "My preset", AgendaPreset::default()).expect("save");
        delete_agenda_preset(dir.path(), "My preset").expect("delete a non-default");

        // The next list still seeds the two built-in defaults.
        let presets = list_agenda_presets(dir.path()).expect("list");
        assert!(presets.contains_key(DONE_THIS_WEEK));
        assert!(presets.contains_key(DONE_THIS_MONTH));
        assert!(!presets.contains_key("My preset"));

        // And the flag was only set by the seeding `list` above — not the delete.
        let settings = read_vault_settings(dir.path()).expect("read back");
        assert!(settings.agenda_presets_seeded);
    }
}
