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
/// name) and persist. Also marks the defaults as seeded so a first-ever save
/// does not later trigger a surprise re-seed of the built-ins on the next
/// `list`.
///
/// # Errors
///
/// [`OrgError::Io`] if the settings file cannot be read or written.
pub fn save_agenda_preset(vault_root: &Path, name: &str, preset: AgendaPreset) -> OrgResult<()> {
    let mut settings = read_vault_settings(vault_root).map_err(presets_io)?;
    settings.agenda_presets.insert(name.to_string(), preset);
    settings.agenda_presets_seeded = true;
    write_vault_settings(vault_root, &settings).map_err(presets_io)?;
    Ok(())
}

/// Remove the preset named `name` and persist. A missing name is a no-op
/// (still `Ok`); the seeded flag is preserved so deleting a default does not
/// re-arm seeding.
///
/// # Errors
///
/// [`OrgError::Io`] if the settings file cannot be read or written.
pub fn delete_agenda_preset(vault_root: &Path, name: &str) -> OrgResult<()> {
    let mut settings = read_vault_settings(vault_root).map_err(presets_io)?;
    if settings.agenda_presets.remove(name).is_some() {
        // Ensure the flag is set even if the vault predates seeding, so the
        // deletion is not undone by a subsequent list re-seeding the defaults.
        settings.agenda_presets_seeded = true;
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
    fn deleting_a_missing_preset_is_ok() {
        let dir = tempdir().expect("tempdir");
        list_agenda_presets(dir.path()).expect("seed");
        delete_agenda_preset(dir.path(), "does-not-exist").expect("no-op delete is Ok");
    }
}
