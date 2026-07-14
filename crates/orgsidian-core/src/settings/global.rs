//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! Global settings reader / writer. Path = `<config-dir>/orgsidian/global.toml`
//! where `<config-dir>` is resolved via [`dirs::config_dir`] — `~/.config` on
//! Linux/BSD, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows.

use std::fs;
use std::path::PathBuf;

use orgsidian_vault::atomic_write;

use super::error::{SettingsError, SettingsResult};
use super::meta::FILE_HEADER;
use super::schema::GlobalSettings;

const GLOBAL_DOTDIR: &str = "orgsidian";
const GLOBAL_SETTINGS_FILE: &str = "global.toml";

/// Resolve the OS-conventional global settings file path. Returns
/// [`SettingsError::ConfigDirUnavailable`] only on extremely degraded
/// environments (no HOME variable on Unix; no `%APPDATA%` on Windows).
pub fn global_settings_path() -> SettingsResult<PathBuf> {
    dirs::config_dir()
        .map(|cfg| cfg.join(GLOBAL_DOTDIR).join(GLOBAL_SETTINGS_FILE))
        .ok_or(SettingsError::ConfigDirUnavailable)
}

/// Read global settings. See [`super::vault::read_vault_settings`] for read
/// semantics — same contract (default-on-missing, parse-error-otherwise).
pub fn read_global_settings() -> SettingsResult<GlobalSettings> {
    let path = global_settings_path()?;
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GlobalSettings::default());
        }
        Err(source) => return Err(SettingsError::Io { path, source }),
    };
    toml::from_str(&raw).map_err(|source| SettingsError::ParseFailed { path, source })
}

/// Atomically write global settings. Creates `<config-dir>/orgsidian/` if absent.
pub fn write_global_settings(settings: &GlobalSettings) -> SettingsResult<()> {
    let path = global_settings_path()?;
    let dir = path
        .parent()
        .expect("global config path always has a parent");
    fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let body = toml::to_string_pretty(settings)
        .map_err(|source| SettingsError::SerializeFailed { source })?;
    let mut contents = String::with_capacity(FILE_HEADER.len() + body.len());
    contents.push_str(FILE_HEADER);
    contents.push_str(&body);

    // Story 3.1: atomic_write returns VaultError; SettingsError keeps its
    // io::Error-sourced shape via the into_io() escape hatch.
    atomic_write(&path, contents.as_bytes()).map_err(|e| SettingsError::Io {
        path,
        source: e.into_io(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_settings_path_resolves_under_config_dir() {
        // Test fixture: `dirs::config_dir()` is OS-dependent but always returns
        // a path containing "orgsidian/global.toml" when present. On the CI
        // matrix (macOS + Ubuntu + Windows nightly) this resolves cleanly; if
        // ever `None` (degraded environment) the function returns
        // ConfigDirUnavailable, which we tolerate here.
        match global_settings_path() {
            Ok(p) => {
                let s = p.to_string_lossy();
                assert!(
                    s.ends_with("orgsidian/global.toml") || s.ends_with("orgsidian\\global.toml"),
                    "unexpected path tail: {s}"
                );
            }
            Err(SettingsError::ConfigDirUnavailable) => {
                // Tolerated — CI environment without HOME. The variant is
                // tested by construction (it's the only path to this branch).
            }
            Err(e) => panic!("unexpected error variant: {e:?}"),
        }
    }

    #[test]
    fn read_returns_default_when_global_file_missing() {
        // We can't reliably control whether the global file is missing in CI
        // (a developer might have one). Skip the assertion when the file
        // happens to exist; otherwise verify the default-on-missing semantic.
        let Ok(path) = global_settings_path() else {
            return;
        };
        if path.exists() {
            // Developer has a real global.toml — skip rather than mutate it.
            return;
        }
        let result = read_global_settings().expect("missing global → Ok(default)");
        assert_eq!(result, GlobalSettings::default());
    }
}
