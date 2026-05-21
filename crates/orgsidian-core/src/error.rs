//! `OrgError`: project-wide IPC error type.
//!
//! Variants are struct-shaped so each error category carries diagnostic detail
//! (file/path/reason) without needing a separate context layer. The
//! discriminator on the wire is `kind` (internally-tagged) which TypeScript
//! consumers can narrow on directly.

pub type Result<T> = std::result::Result<T, OrgError>;

// `#[specta(rename_all = "camelCase")]` is rejected by specta `=2.0.0-rc.25`
// (compiler error: "no longer supported on containers. Use #[serde(rename_all
// = ...)] instead."). The `#[serde]` attribute alone covers both the JSON wire
// format and the generated TS shape via specta-serde's Format symmetry. See
// Dev Notes §Casing for the deviation.
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OrgError {
    #[error("parse error in {file}: {reason}")]
    Parse { file: String, reason: String },
    #[error("io error: {reason}")]
    Io { reason: String },
    #[error("index error: {reason}")]
    Index { reason: String },
    #[error("vault error: {reason}")]
    Vault { reason: String },
}
