//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! `SettingsError` is the failure type for the settings store. Variants carry
//! the offending path where possible so the shell-app's LD-41 fallback handler
//! (Story 6.7 wires it) can rename `<file>.broken-{timestamp}` and surface a
//! warn-banner. Mirrors the `OrgError` precedent at [`crate::error`].

use std::io;
use std::path::PathBuf;

/// Convenience alias mirroring [`crate::Result`].
pub type SettingsResult<T> = Result<T, SettingsError>;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// `dirs::config_dir()` returned `None` — extremely rare; degraded-OS-environment guard.
    #[error("OS config directory unavailable (dirs::config_dir() returned None)")]
    ConfigDirUnavailable,

    /// Filesystem I/O failure (read, create_dir_all, atomic-write).
    #[error("settings I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// TOML parse failure (file exists but malformed).
    ///
    /// Caller (shell-app bootstrap) handles the LD-41 fallback (backup
    /// `<file>.broken-{timestamp}` + warn banner). Out of Story 1.18 scope.
    // TODO(Story-6.7): wire LD-41 fallback handler at the call site.
    #[error("failed to parse settings TOML at {path}: {source}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// TOML serialize failure — practically only fires on schema-author bugs
    /// (e.g., a non-string `HashMap` key). Surfaced for completeness.
    #[error("failed to serialize settings TOML: {source}")]
    SerializeFailed {
        #[source]
        source: toml::ser::Error,
    },

    /// Forward-compat refusal: file declares `schema_version = N` where
    /// `N > SCHEMA_VERSION_CURRENT`. A v1 binary cannot safely interpret a v2
    /// file because v2's semantic changes are unknown.
    #[error("settings schema_version {found} is newer than supported version {supported}")]
    SchemaVersionUnsupported { found: u32, supported: u32 },
}
