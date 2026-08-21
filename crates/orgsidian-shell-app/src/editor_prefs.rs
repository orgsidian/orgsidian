//! Implements FR-3
//!
//! Editor Modes (Raw / Pseudo-WYSIWYG / Split) per-file persistence. Story 4.2
//! ships the [`EditorMode`] IPC type plus the store round-trip that backs
//! `commands.setEditorMode` / `commands.getEditorMode`.
//!
//! Per LD-40 the choice is stored as *ephemeral per-Vault view state* (not
//! authoritative settings, which are TOML) via `tauri-plugin-store` at
//! `<Vault>/.orgsidian/editor-prefs.json`, keyed by the file's path. The store
//! path is derived from the active Vault's root ([`editor_prefs_path`]); an
//! absolute path resolves verbatim under Tauri's `BaseDirectory::AppData`
//! resolver (`PathBuf::push` of an absolute path replaces the base), so the
//! prefs file lands inside the Vault rather than the OS app-data dir.

use std::path::{Path, PathBuf};

use orgsidian_core::{OrgError, Result as OrgResult};
use tauri_plugin_store::StoreExt;

/// The dot-directory that holds per-Vault state, mirroring
/// `orgsidian_core::settings::vault` (`.orgsidian/`).
const VAULT_DOTDIR: &str = ".orgsidian";
/// The `tauri-plugin-store` file name for per-file editor preferences (LD-40).
const EDITOR_PREFS_FILE: &str = "editor-prefs.json";

/// The three Editor Modes (FR-3). Serialized as camelCase string literals on
/// the wire (`"raw" | "pseudoWysiwyg" | "split"`) — the same `#[serde]`
/// deviation used by `OrgError` (specta `rename_all` on containers is rejected
/// by the pinned specta; the serde attribute drives both the JSON shape and the
/// generated TS union via specta-serde Format symmetry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum EditorMode {
    /// Plain `.org` source with syntax-highlight tokens only — no decorations.
    Raw,
    /// Inline decorations/widgets over the source buffer (product default).
    PseudoWysiwyg,
    /// Side-by-side source + Pseudo-WYSIWYG panes sharing one `EditorState`.
    Split,
}

/// Derive the per-Vault editor-prefs store path: `<vault>/.orgsidian/editor-prefs.json`.
///
/// Pure — no I/O, no directory creation. `tauri-plugin-store`'s `save()` creates
/// the parent `.orgsidian/` lazily.
pub fn editor_prefs_path(vault_root: &Path) -> PathBuf {
    vault_root.join(VAULT_DOTDIR).join(EDITOR_PREFS_FILE)
}

/// Map any store / serialization failure to `OrgError::Io` — a prefs write is a
/// disk concern, and the failure catalog (LD-41) treats config-file trouble as
/// non-fatal (the caller falls back to the default mode).
fn prefs_io(err: impl std::fmt::Display) -> OrgError {
    OrgError::Io {
        reason: format!("editor-prefs store: {err}"),
    }
}

/// Persist `mode` for `file_path` in the active Vault's editor-prefs store
/// (LD-40). Overwrites any previous choice for that file.
pub fn persist_mode(
    app: &tauri::AppHandle,
    vault_root: &Path,
    file_path: &str,
    mode: EditorMode,
) -> OrgResult<()> {
    let store = app.store(editor_prefs_path(vault_root)).map_err(prefs_io)?;
    let value = serde_json::to_value(mode).map_err(prefs_io)?;
    store.set(file_path.to_string(), value);
    store.save().map_err(prefs_io)?;
    Ok(())
}

/// Read the persisted mode for `file_path`, or `None` when the file has no
/// stored choice yet.
pub fn read_mode(
    app: &tauri::AppHandle,
    vault_root: &Path,
    file_path: &str,
) -> OrgResult<Option<EditorMode>> {
    let store = app.store(editor_prefs_path(vault_root)).map_err(prefs_io)?;
    match store.get(file_path) {
        Some(value) => serde_json::from_value(value).map(Some).map_err(prefs_io),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_path_lands_in_vault_dotdir() {
        let p = editor_prefs_path(Path::new("/vaults/work"));
        assert_eq!(
            p,
            Path::new("/vaults/work/.orgsidian/editor-prefs.json"),
            "editor-prefs must live inside the Vault's .orgsidian dir (LD-40)"
        );
    }

    #[test]
    fn editor_mode_serializes_to_camelcase_string_literals() {
        assert_eq!(
            serde_json::to_value(EditorMode::Raw).unwrap(),
            serde_json::json!("raw")
        );
        assert_eq!(
            serde_json::to_value(EditorMode::PseudoWysiwyg).unwrap(),
            serde_json::json!("pseudoWysiwyg")
        );
        assert_eq!(
            serde_json::to_value(EditorMode::Split).unwrap(),
            serde_json::json!("split")
        );
    }

    #[test]
    fn editor_mode_round_trips_through_json() {
        for mode in [
            EditorMode::Raw,
            EditorMode::PseudoWysiwyg,
            EditorMode::Split,
        ] {
            let value = serde_json::to_value(mode).unwrap();
            let back: EditorMode = serde_json::from_value(value).unwrap();
            assert_eq!(back, mode);
        }
        // The AC's literal `"raw"` deserializes to the Raw variant.
        let raw: EditorMode = serde_json::from_value(serde_json::json!("raw")).unwrap();
        assert_eq!(raw, EditorMode::Raw);
    }

    #[test]
    fn unknown_mode_string_is_rejected() {
        let parsed = serde_json::from_value::<EditorMode>(serde_json::json!("wysiwyg"));
        assert!(parsed.is_err(), "unknown mode strings must not deserialize");
    }
}
