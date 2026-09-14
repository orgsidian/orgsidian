//! Implements FR-6 (Today Dashboard — Story 7.2)
//!
//! Per-section collapsed/expanded persistence for the Today Dashboard's five
//! sections. Story 7.2 ships the [`DashboardSection`] / [`TodayDashboardPrefs`]
//! IPC types plus the store round-trip that backs
//! `commands.setTodayDashboardSectionCollapsed` / `commands.getTodayDashboardPrefs`.
//!
//! Per LD-40 a section's collapse toggle is *ephemeral per-Vault view state*
//! (not authoritative settings, which are TOML) so it is stored via
//! `tauri-plugin-store` at `<Vault>/.orgsidian/today-prefs.json`, keyed by the
//! section's stable identifier. This is a direct sibling of
//! [`crate::editor_prefs`] (per-file editor mode): same store mechanism, same
//! Vault-root-derived path ([`dashboard_prefs_path`]), same `OrgError::Io`
//! failure mapping. An absolute path resolves verbatim under Tauri's
//! `BaseDirectory::AppData` resolver (`PathBuf::push` of an absolute path
//! replaces the base), so the prefs file lands inside the Vault rather than the
//! OS app-data dir.

use std::path::{Path, PathBuf};

use orgsidian_core::{OrgError, Result as OrgResult};
use tauri_plugin_store::StoreExt;

/// The dot-directory that holds per-Vault state, mirroring
/// `orgsidian_core::settings::vault` (`.orgsidian/`).
const VAULT_DOTDIR: &str = ".orgsidian";
/// The `tauri-plugin-store` file name for per-Vault Today Dashboard section
/// preferences (LD-40).
const DASHBOARD_PREFS_FILE: &str = "today-prefs.json";

/// The five Today Dashboard sections (FR-6). Serialized as camelCase string
/// literals on the wire (`"scheduled" | "deadline" | "todayTag" |
/// "inboxPreview" | "activeClock"`) — the same `#[serde]` deviation used by
/// [`crate::editor_prefs::EditorMode`] (specta `rename_all` on containers is
/// rejected by the pinned specta; the serde attribute drives both the JSON
/// shape and the generated TS union via specta-serde Format symmetry). The
/// string form is also the stable store key ([`DashboardSection::key`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DashboardSection {
    /// SCHEDULED-for-today headlines.
    Scheduled,
    /// DEADLINE due-or-overdue headlines.
    Deadline,
    /// Headlines carrying the today-tag.
    TodayTag,
    /// The Inbox preview (first N `inbox.org` headlines).
    InboxPreview,
    /// The single running clock.
    ActiveClock,
}

impl DashboardSection {
    /// Every section, in dashboard render order. Used by the tests to guard the
    /// key/wire-form contract across all variants; [`read_all_collapsed`]
    /// enumerates the fields directly (a struct literal, not an iterator).
    #[cfg(test)]
    const ALL: [DashboardSection; 5] = [
        DashboardSection::Scheduled,
        DashboardSection::Deadline,
        DashboardSection::TodayTag,
        DashboardSection::InboxPreview,
        DashboardSection::ActiveClock,
    ];

    /// The section's stable store key — identical to its camelCase wire form, so
    /// a persisted key survives a rename of neither the enum variant nor the
    /// serde repr silently (the [`section_keys_are_stable`] test pins them).
    pub fn key(self) -> &'static str {
        match self {
            DashboardSection::Scheduled => "scheduled",
            DashboardSection::Deadline => "deadline",
            DashboardSection::TodayTag => "todayTag",
            DashboardSection::InboxPreview => "inboxPreview",
            DashboardSection::ActiveClock => "activeClock",
        }
    }
}

/// The collapsed state of every Today Dashboard section, as read back for the
/// frontend to seed each section's initial toggle. `true` = collapsed, `false` =
/// expanded (the default). Missing/malformed store entries self-heal to the
/// expanded default (see [`read_all_collapsed`]).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "camelCase")]
pub struct TodayDashboardPrefs {
    /// The Scheduled section is collapsed.
    pub scheduled: bool,
    /// The Deadline section is collapsed.
    pub deadline: bool,
    /// The Today-Tag section is collapsed.
    pub today_tag: bool,
    /// The Inbox Preview section is collapsed.
    pub inbox_preview: bool,
    /// The Active Clock section is collapsed.
    pub active_clock: bool,
}

/// Derive the per-Vault dashboard-prefs store path:
/// `<vault>/.orgsidian/today-prefs.json`.
///
/// Pure — no I/O, no directory creation. `tauri-plugin-store`'s `save()` creates
/// the parent `.orgsidian/` lazily.
pub fn dashboard_prefs_path(vault_root: &Path) -> PathBuf {
    vault_root.join(VAULT_DOTDIR).join(DASHBOARD_PREFS_FILE)
}

/// Map any store / serialization failure to `OrgError::Io` — a prefs write is a
/// disk concern, and the failure catalog (LD-41) treats config-file trouble as
/// non-fatal (the caller falls back to the expanded default).
fn prefs_io(err: impl std::fmt::Display) -> OrgError {
    OrgError::Io {
        reason: format!("today-prefs store: {err}"),
    }
}

/// Persist `collapsed` for `section` in the active Vault's dashboard-prefs store
/// (LD-40). Overwrites any previous choice for that section.
pub fn persist_collapsed(
    app: &tauri::AppHandle,
    vault_root: &Path,
    section: DashboardSection,
    collapsed: bool,
) -> OrgResult<()> {
    let store = app
        .store(dashboard_prefs_path(vault_root))
        .map_err(prefs_io)?;
    store.set(
        section.key().to_string(),
        serde_json::Value::Bool(collapsed),
    );
    store.save().map_err(prefs_io)?;
    Ok(())
}

/// Read every section's collapsed state from the active Vault's dashboard-prefs
/// store. A section with no stored entry — or a malformed (non-boolean) one —
/// self-heals to the expanded default (`false`), so a hand-edited or
/// partially-written `today-prefs.json` never fails the dashboard load.
pub fn read_all_collapsed(
    app: &tauri::AppHandle,
    vault_root: &Path,
) -> OrgResult<TodayDashboardPrefs> {
    let store = app
        .store(dashboard_prefs_path(vault_root))
        .map_err(prefs_io)?;
    // Missing key OR a non-boolean value → expanded default. This is a wider
    // tolerance than `editor_prefs::read_mode` (which errors on a malformed
    // value) because a bad view-state toggle must never block the whole
    // dashboard from rendering.
    let collapsed = |section: DashboardSection| -> bool {
        store
            .get(section.key())
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    };
    Ok(TodayDashboardPrefs {
        scheduled: collapsed(DashboardSection::Scheduled),
        deadline: collapsed(DashboardSection::Deadline),
        today_tag: collapsed(DashboardSection::TodayTag),
        inbox_preview: collapsed(DashboardSection::InboxPreview),
        active_clock: collapsed(DashboardSection::ActiveClock),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_path_lands_in_vault_dotdir() {
        let p = dashboard_prefs_path(Path::new("/vaults/work"));
        assert_eq!(
            p,
            Path::new("/vaults/work/.orgsidian/today-prefs.json"),
            "today-prefs must live inside the Vault's .orgsidian dir (LD-40)"
        );
    }

    #[test]
    fn section_keys_are_stable() {
        // Persisted keys are a storage contract: a rename here silently orphans
        // every user's saved toggles, so pin the exact strings.
        assert_eq!(DashboardSection::Scheduled.key(), "scheduled");
        assert_eq!(DashboardSection::Deadline.key(), "deadline");
        assert_eq!(DashboardSection::TodayTag.key(), "todayTag");
        assert_eq!(DashboardSection::InboxPreview.key(), "inboxPreview");
        assert_eq!(DashboardSection::ActiveClock.key(), "activeClock");
        // Every variant is covered by ALL (guards a future added section from
        // being dropped out of `read_all_collapsed`).
        assert_eq!(DashboardSection::ALL.len(), 5);
    }

    #[test]
    fn section_key_matches_camelcase_wire_form() {
        // The store key and the specta/serde wire form MUST agree so the
        // frontend can key on the same string it sends to
        // `setTodayDashboardSectionCollapsed`.
        for section in DashboardSection::ALL {
            let wire = serde_json::to_value(section).unwrap();
            assert_eq!(wire, serde_json::json!(section.key()));
        }
    }

    #[test]
    fn prefs_default_is_all_expanded() {
        assert_eq!(
            TodayDashboardPrefs::default(),
            TodayDashboardPrefs {
                scheduled: false,
                deadline: false,
                today_tag: false,
                inbox_preview: false,
                active_clock: false,
            }
        );
    }

    #[test]
    fn section_round_trips_through_json() {
        for section in DashboardSection::ALL {
            let value = serde_json::to_value(section).unwrap();
            let back: DashboardSection = serde_json::from_value(value).unwrap();
            assert_eq!(back, section);
        }
    }
}
