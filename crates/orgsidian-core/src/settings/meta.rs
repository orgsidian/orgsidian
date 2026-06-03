//! Implements LD-40 + FR-23 settings store (OQ-7 dual-surface).
//!
//! File-level metadata constants and header text shared between the vault and
//! global writers. Kept as a separate module so the boundary doc + grep-smoke
//! traceability test can target a stable surface even if the header copy
//! evolves.

/// Leading comment block prepended to every authoritative TOML file by the
/// writers. Provides human-readable framing for the v1 schema.
pub const FILE_HEADER: &str = "\
# === Orgsidian settings — schema v1 (LD-40) ===
# Edit by hand if you like; the Settings GUI is a thin round-trip editor over this file.

";
