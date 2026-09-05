//! Implements FR-21 (partial) / FR-18 / UJ-4 (Story 6.6).
//!
//! Hardcoded UJ-4 first-run coaching-balloon dismissal persistence:
//! `<Vault>/.orgsidian/coaching-dismissed.json`, keyed by two hardcoded
//! coaching IDs ([`UJ4_TODAY_INTRO`], [`UJ4_CAPTURE_INTRO`]).
//!
//! Deliberately NOT the `VaultSettings.dismissed_coaching` TOML field
//! (`settings::schema::VaultSettings`) — that field is Story 11.5's home for
//! the FR-21 registry-driven `CoachingSlot` dismissal mechanism (v0.5 Beta).
//! Story 6.6's balloons are an explicitly disposable v0.1 stand-in: per the
//! epic AC, Story 11.4 REMOVES (not migrates) this module wholesale when the
//! real registry ships, importing only the two ID constants below so existing
//! dismissals keep working across the cutover. Keeping the storage format and
//! file separate from `VaultSettings` means this whole module — and only this
//! module — is deleted in that one PR, without touching the locked settings
//! schema.
//!
//! Format: a plain JSON array of dismissed coaching-id strings, not wrapped in
//! `tauri-plugin-store`'s envelope (contrast `editor_prefs.rs` in the shell-app
//! crate, which uses that store for genuinely ephemeral per-file view state) —
//! inspectable, and the simplest shape a future importer reads without any
//! Orgsidian-specific tooling. Written via the Story 3.1 atomic-write path
//! (LD-8), the same discipline `settings::vault` uses for `settings.toml`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use orgsidian_vault::atomic_write;

use crate::error::OrgError;
use crate::Result as OrgResult;

const VAULT_DOTDIR: &str = ".orgsidian";
const COACHING_DISMISSED_FILE: &str = "coaching-dismissed.json";

/// Story 6.6 (UJ-4): the "this is your day" balloon anchored to the first
/// Today Agenda item (`shell-ui/src/components/agenda/AgendaToday.tsx`).
pub const UJ4_TODAY_INTRO: &str = "UJ4_TODAY_INTRO";

/// Story 6.6 (UJ-4): the Quick Capture nudge balloon. v0.1 anchor: a calm
/// top-of-route placement on `/today` (see the Story 6.6 story file's Design
/// Notes for why — the Epic 7 Inbox preview surface this balloon's copy
/// references does not exist yet).
pub const UJ4_CAPTURE_INTRO: &str = "UJ4_CAPTURE_INTRO";

/// Resolve the coaching-dismissed store path:
/// `<vault>/.orgsidian/coaching-dismissed.json`. Pure path arithmetic — no I/O.
pub fn coaching_dismissed_path(vault_root: &Path) -> PathBuf {
    vault_root.join(VAULT_DOTDIR).join(COACHING_DISMISSED_FILE)
}

/// Map any I/O or (de)serialization failure to `OrgError::Io` — a coaching
/// dismissal write is a disk concern, same mapping convention
/// `editor_prefs.rs` uses for its store.
fn coaching_io(err: impl std::fmt::Display) -> OrgError {
    OrgError::Io {
        reason: format!("coaching-dismissed store: {err}"),
    }
}

/// Read the set of dismissed coaching ids for `vault_root`. Returns an empty
/// set when the file does not exist yet (no balloon dismissed in this Vault)
/// — never an error, mirroring `read_vault_settings`'s default-on-missing
/// contract.
///
/// Self-heals a malformed file (post-review fix, 2026-09-05): if the file
/// exists but fails to parse as JSON (hand-edited, truncated by a crash
/// mid-write before the atomic rename landed elsewhere, etc.), this also
/// returns an empty set rather than `Err` — a permanent parse error here
/// would otherwise trap whichever balloon(s) the corrupt file was guarding
/// into resurfacing on every launch with no way to dismiss them (dismissing
/// would call `dismiss_coaching`, which itself reads via this function and
/// would also fail). A subsequent `dismiss_coaching` call overwrites the
/// corrupt file with valid content, so the corruption self-heals the moment
/// the user next dismisses a balloon. Non-parse I/O errors (permission
/// denied, etc.) are unaffected — those still propagate as `Err`.
pub fn read_dismissed_coaching(vault_root: &Path) -> OrgResult<BTreeSet<String>> {
    let path = coaching_dismissed_path(vault_root);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(source) => return Err(coaching_io(source)),
    };
    match serde_json::from_str(&raw) {
        Ok(dismissed) => Ok(dismissed),
        Err(_parse_err) => Ok(BTreeSet::new()),
    }
}

/// Persist `id` as dismissed. Idempotent (dismissing an already-dismissed id
/// re-writes the same content) and additive: any other id dismissed earlier
/// is preserved. Creates `<vault>/.orgsidian/` if absent. Returns the
/// resulting full dismissed set.
pub fn dismiss_coaching(vault_root: &Path, id: &str) -> OrgResult<BTreeSet<String>> {
    let path = coaching_dismissed_path(vault_root);
    let dir = path.parent().expect(".orgsidian path always has a parent");
    fs::create_dir_all(dir).map_err(coaching_io)?;

    let mut dismissed = read_dismissed_coaching(vault_root)?;
    dismissed.insert(id.to_string());

    let body = serde_json::to_string_pretty(&dismissed).map_err(coaching_io)?;
    atomic_write(&path, body.as_bytes()).map_err(coaching_io)?;
    Ok(dismissed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn coaching_dismissed_path_joins_dotorgsidian() {
        let p = coaching_dismissed_path(Path::new("/vaults/work"));
        assert_eq!(
            p,
            Path::new("/vaults/work/.orgsidian/coaching-dismissed.json")
        );
    }

    #[test]
    fn read_returns_empty_set_when_file_missing() {
        let dir = tempdir().expect("tempdir");
        let result = read_dismissed_coaching(dir.path()).expect("missing file → Ok(empty set)");
        assert!(result.is_empty());
    }

    #[test]
    fn dismiss_creates_dotorgsidian_dir_and_persists() {
        let dir = tempdir().expect("tempdir");
        let result = dismiss_coaching(dir.path(), UJ4_TODAY_INTRO).expect("dismiss succeeds");
        assert!(result.contains(UJ4_TODAY_INTRO));

        let on_disk = coaching_dismissed_path(dir.path());
        assert!(on_disk.exists());
        assert!(on_disk.parent().unwrap().is_dir());

        let read_back = read_dismissed_coaching(dir.path()).expect("read back");
        assert_eq!(read_back, result);
    }

    #[test]
    fn dismissing_two_ids_preserves_both() {
        let dir = tempdir().expect("tempdir");
        dismiss_coaching(dir.path(), UJ4_TODAY_INTRO).expect("first dismiss");
        let result = dismiss_coaching(dir.path(), UJ4_CAPTURE_INTRO).expect("second dismiss");

        assert!(result.contains(UJ4_TODAY_INTRO));
        assert!(result.contains(UJ4_CAPTURE_INTRO));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn dismissing_the_same_id_twice_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        dismiss_coaching(dir.path(), UJ4_TODAY_INTRO).expect("first dismiss");
        let result =
            dismiss_coaching(dir.path(), UJ4_TODAY_INTRO).expect("second dismiss, same id");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn malformed_json_self_heals_to_empty_set_rather_than_erroring() {
        let dir = tempdir().expect("tempdir");
        let path = coaching_dismissed_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();

        let result = read_dismissed_coaching(dir.path())
            .expect("malformed JSON self-heals to Ok(empty set), never Err");
        assert!(result.is_empty());
    }

    #[test]
    fn dismiss_after_malformed_json_overwrites_it_with_a_valid_file() {
        let dir = tempdir().expect("tempdir");
        let path = coaching_dismissed_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();

        let result = dismiss_coaching(dir.path(), UJ4_TODAY_INTRO)
            .expect("dismiss succeeds even though the prior file was corrupt");
        assert_eq!(result.len(), 1);
        assert!(result.contains(UJ4_TODAY_INTRO));

        let read_back = read_dismissed_coaching(dir.path()).expect("file is valid JSON now");
        assert_eq!(read_back, result);
    }
}
