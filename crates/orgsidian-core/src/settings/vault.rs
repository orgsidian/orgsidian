//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Per-Vault settings reader / writer. Path = `<vault>/.orgsidian/settings.toml`.
//! Writes are atomic via [`orgsidian_vault::atomic_write`] (LD-8); the writer
//! creates `.orgsidian/` as needed via `std::fs::create_dir_all`.
//
// FOLLOWUP(Story-5.4): watcher reload hook lands here per LD-7 Single Writer Rule.
// FOLLOWUP(Story-12.3): swap to toml_edit for format-preserving GUI round-trip.

use std::fs;
use std::path::{Path, PathBuf};

use orgsidian_vault::atomic_write;

use super::error::{SettingsError, SettingsResult};
use super::meta::FILE_HEADER;
use super::schema::VaultSettings;

const VAULT_DOTDIR: &str = ".orgsidian";
const VAULT_SETTINGS_FILE: &str = "settings.toml";

/// Resolve the canonical per-Vault settings file path. Pure path arithmetic —
/// no I/O, no directory creation. The writer creates `.orgsidian/` lazily.
pub fn vault_settings_path(vault_path: &Path) -> PathBuf {
    vault_path.join(VAULT_DOTDIR).join(VAULT_SETTINGS_FILE)
}

/// Read per-Vault settings. Returns `VaultSettings::default()` when the file is
/// absent (first-launch path). Returns [`SettingsError::ParseFailed`] when the
/// file exists but cannot be parsed; the caller (shell-app bootstrap) is
/// responsible for the LD-41 backup-and-warn fallback (Story 6.7+ scope).
pub fn read_vault_settings(vault_path: &Path) -> SettingsResult<VaultSettings> {
    let path = vault_settings_path(vault_path);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(VaultSettings::default());
        }
        Err(source) => return Err(SettingsError::Io { path, source }),
    };
    toml::from_str(&raw).map_err(|source| SettingsError::ParseFailed { path, source })
}

/// Atomically write per-Vault settings. Creates `<vault>/.orgsidian/` if absent.
/// Serialization is deterministic (pretty TOML with declaration-order keys);
/// see the writer fixed-point property in `tests/settings_round_trip.rs`.
pub fn write_vault_settings(vault_path: &Path, settings: &VaultSettings) -> SettingsResult<()> {
    let path = vault_settings_path(vault_path);
    let dir = path.parent().expect(".orgsidian path always has a parent");
    fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let body = toml::to_string_pretty(settings)
        .map_err(|source| SettingsError::SerializeFailed { source })?;
    let mut contents = String::with_capacity(FILE_HEADER.len() + body.len());
    contents.push_str(FILE_HEADER);
    contents.push_str(&body);

    atomic_write(&path, contents.as_bytes()).map_err(|source| SettingsError::Io { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn vault_settings_path_joins_dotorgsidian() {
        let p = vault_settings_path(Path::new("/vaults/work"));
        assert_eq!(p, Path::new("/vaults/work/.orgsidian/settings.toml"));
    }

    #[test]
    fn read_returns_default_when_file_missing() {
        let dir = tempdir().expect("tempdir");
        let result = read_vault_settings(dir.path()).expect("missing file → Ok(default)");
        assert_eq!(result, VaultSettings::default());
    }

    #[test]
    fn write_creates_dotorgsidian_dir() {
        let dir = tempdir().expect("tempdir");
        let settings = VaultSettings::default();
        write_vault_settings(dir.path(), &settings).expect("write succeeds");
        let on_disk = vault_settings_path(dir.path());
        assert!(on_disk.exists(), "settings.toml should exist at {on_disk:?}");
        assert!(
            on_disk.parent().unwrap().is_dir(),
            ".orgsidian/ directory should be created"
        );
    }

    #[test]
    fn parse_failure_surfaces_parse_failed_variant() {
        let dir = tempdir().expect("tempdir");
        // Create .orgsidian/settings.toml with deliberately malformed TOML.
        let settings_path = vault_settings_path(dir.path());
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::write(&settings_path, "schema_version = \"not-a-number-and-no-closing-quote")
            .expect("seed malformed file");
        let err = read_vault_settings(dir.path()).expect_err("malformed TOML must error");
        assert!(matches!(err, SettingsError::ParseFailed { .. }));
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempdir().expect("tempdir");
        let mut original = VaultSettings::default();
        original
            .keybindings
            .insert("editor.save".into(), "Cmd+S".into());
        write_vault_settings(dir.path(), &original).expect("write");
        let read_back = read_vault_settings(dir.path()).expect("read");
        assert_eq!(read_back, original);
    }
}
