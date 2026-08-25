//! Editor-save calibration (OD-3 / Story 5.2).
//!
//! Story 5.1's [`Debouncer`](crate::watcher::Debouncer) coalesces a burst of raw
//! events *per path* into one [`FileChanged`](crate::watcher::FileChanged). That
//! alone is not "exactly one `FileChanged` per editor save": a real save is
//! multi-*path*, not just multi-event. vim renames the target to a `~` backup and
//! drops a `4913` writability probe plus a `.swp` swap file; VS Code writes a
//! temp file and atomically renames it onto the target; Emacs leaves a `~`
//! backup, a `#…#` autosave, and a `.#…` lock file. Left unfiltered, each of
//! those artifact paths would debounce into its own spurious `FileChanged`,
//! tripping the Single-Writer / merge state machines Epic 5 protects.
//!
//! This module is the calibration seam between the raw event stream and the
//! debouncer: [`save_targets`] keeps only the genuine save-target paths (`.org`
//! files that are not editor artifacts) so one logical save collapses to one
//! target path, which the per-path debouncer then coalesces to one
//! `FileChanged`. The classification is pure and path-only (no I/O), so the
//! golden-trace replay in `tests/debounce.rs` is fully deterministic.
//!
//! The rules are derived from the documented save mechanics of vim, VS Code, and
//! Emacs, and are pinned by the hand-authored fixtures under
//! `tests/golden_traces/` (see `fixtures/fixtures.toml`, `owner = "epic-5"`).

use std::path::{Path, PathBuf};

/// File extension the vault treats as a genuine edit target. Editor artifacts
/// carry other extensions (`org~`, `swp`, `tmp-*`, …) or are otherwise
/// identified by [`is_editor_artifact`], so a real save always leaves exactly
/// one surviving `.org` path.
const TARGET_EXTENSION: &str = "org";

/// vim swap-file extensions (`:h swap-file`). vim writes `.<name>.swp` while a
/// buffer is open and refreshes it on save.
const VIM_SWAP_EXTENSIONS: &[&str] = &["swp", "swo", "swx", "swpx", "swap"];

/// True if `path` names a transient editor artifact that must never surface as a
/// logical save — a swap file, a backup, an autosave, a lock file, or an
/// atomic-write temp/probe file. Classification is by file name only (no I/O).
///
/// Rules (documented vim / VS Code / Emacs save mechanics):
/// - `*~` — vim/Emacs backup of the previous contents.
/// - `.#*` — Emacs lock file (a dangling symlink; note it still ends in `.org`,
///   so the extension check alone would wrongly keep it — this prefix rule is
///   what excludes it).
/// - `#*#` — Emacs autosave buffer.
/// - `.<name>.sw{p,o,x,…}` — vim swap file.
/// - `*.tmp` / `*.tmp-<rand>` — VS Code (and generic) atomic-write temp file.
/// - all-digit, extension-less names (e.g. `4913`) — vim's writability probe.
pub fn is_editor_artifact(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    // Emacs backup / vim backup.
    if name.ends_with('~') {
        return true;
    }
    // Emacs lock file: `.#notes.org` — matched before the extension check
    // because its extension is still `org`.
    if name.starts_with(".#") {
        return true;
    }
    // Emacs autosave: `#notes.org#`.
    if name.starts_with('#') && name.ends_with('#') {
        return true;
    }

    let extension = path.extension().and_then(|e| e.to_str());
    if let Some(ext) = extension {
        // vim swap file: hidden name with a swap extension (`.notes.org.swp`).
        if name.starts_with('.') && VIM_SWAP_EXTENSIONS.contains(&ext) {
            return true;
        }
        // Atomic-write temp file: `notes.org.tmp` or `notes.org.tmp-a1b2c3`.
        if ext == "tmp" || ext.starts_with("tmp-") {
            return true;
        }
    } else {
        // Extension-less, all-digit name: vim's `4913` writability probe.
        if name.bytes().all(|b| b.is_ascii_digit()) {
            return true;
        }
    }

    false
}

/// True if `path` is a genuine save target: an `.org` file that is not an editor
/// artifact. Everything else — swap/backup/autosave/lock/temp files, the parent
/// directory FSEvents reports on macOS, and any non-`.org` sibling — is not a
/// target.
pub fn is_save_target(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some(TARGET_EXTENSION) && !is_editor_artifact(path)
}

/// The genuine save-target paths a raw event touches — the calibration the
/// facade applies before arming the debouncer. An artifact-only event (swap
/// flush, lock-file release, temp-file churn) yields an empty vector and arms
/// nothing, so it never produces a `FileChanged`. An atomic rename whose event
/// lists both `[temp, target]` keeps only `target`.
pub fn save_targets(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|p| is_save_target(p))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn plain_org_file_is_a_save_target() {
        assert!(is_save_target(&p("/vault/notes.org")));
        assert!(!is_editor_artifact(&p("/vault/notes.org")));
    }

    #[test]
    fn vim_artifacts_are_filtered() {
        // swap file, writability probe, backup.
        assert!(is_editor_artifact(&p("/vault/.notes.org.swp")));
        assert!(is_editor_artifact(&p("/vault/.notes.org.swo")));
        assert!(is_editor_artifact(&p("/vault/4913")));
        assert!(is_editor_artifact(&p("/vault/notes.org~")));
        for a in ["/vault/.notes.org.swp", "/vault/4913", "/vault/notes.org~"] {
            assert!(!is_save_target(&p(a)), "{a} must not be a target");
        }
    }

    #[test]
    fn vscode_temp_files_are_filtered() {
        assert!(is_editor_artifact(&p("/vault/notes.org.tmp")));
        assert!(is_editor_artifact(&p("/vault/notes.org.tmp-a1b2c3")));
        assert!(!is_save_target(&p("/vault/notes.org.tmp-a1b2c3")));
    }

    #[test]
    fn emacs_artifacts_are_filtered() {
        // autosave, lock file (still ends in `.org`!), backup.
        assert!(is_editor_artifact(&p("/vault/#notes.org#")));
        assert!(is_editor_artifact(&p("/vault/.#notes.org")));
        assert!(is_editor_artifact(&p("/vault/notes.org~")));
        // The lock file is the tripwire for the extension-only shortcut.
        assert!(
            !is_save_target(&p("/vault/.#notes.org")),
            "the Emacs lock file ends in .org but must never be a target"
        );
    }

    #[test]
    fn non_org_siblings_and_directories_are_not_targets() {
        assert!(!is_save_target(&p("/vault"))); // parent dir (macOS FSEvents)
        assert!(!is_save_target(&p("/vault/notes.txt")));
        assert!(!is_save_target(&p("/vault/image.png")));
    }

    #[test]
    fn save_targets_keeps_only_the_real_target() {
        // An atomic rename event lists both the temp file and the target.
        let out = save_targets(&[p("/vault/notes.org.tmp-a1b2c3"), p("/vault/notes.org")]);
        assert_eq!(out, vec![p("/vault/notes.org")]);
    }

    #[test]
    fn save_targets_is_empty_for_artifact_only_events() {
        let out = save_targets(&[p("/vault/.notes.org.swp"), p("/vault/.#notes.org")]);
        assert!(out.is_empty());
    }
}
