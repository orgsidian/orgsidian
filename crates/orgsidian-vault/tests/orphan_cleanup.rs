//! Story 3.1 — orphan temp-file cleanup tests (LD-41 ".tmp orphan files" row,
//! AC5).
//!
//! Plants fixtures matching the real `atomic-write-file` 0.3.0 temp pattern
//! (`.` + `{name}.org` + `.` + 6 alphanumerics — no PID, verified in crate
//! source) and asserts the guarded scan removes only stale orphans whose
//! `{name}.org` target exists: user dotfiles, `.orgsidian/` contents,
//! non-`.org` temps, fresh in-flight temps, and pattern-shaped names with no
//! target (plausible user files) must all survive. The scan is best-effort
//! and never follows symlinks.

use std::fs::{self, File, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

use orgsidian_vault::clean_orphan_temp_files;

/// Backdate a file's mtime so it clears the 60s orphan-age guard.
fn backdate(path: &Path, age: Duration) {
    let file = File::options()
        .write(true)
        .open(path)
        .expect("open fixture for backdating");
    let stale = SystemTime::now() - age;
    file.set_times(FileTimes::new().set_modified(stale))
        .expect("set_times must succeed on fixture");
}

fn plant(path: &Path, content: &[u8]) {
    fs::write(path, content).expect("plant fixture file");
}

#[test]
fn stale_orphan_is_removed_and_reported() {
    let dir = tempfile::TempDir::new().expect("TempDir");
    let target = dir.path().join("notes.org");
    plant(&target, b"* TODO real content\n");
    let orphan = dir.path().join(".notes.org.aB3xY9");
    plant(&orphan, b"partial write from dead writer");
    backdate(&orphan, Duration::from_secs(120));

    let report = clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 1);
    assert_eq!(report.removed, vec![orphan.clone()]);
    assert!(report.errors.is_empty(), "clean scan reports no errors");
    assert!(!orphan.exists(), "stale orphan must be deleted");
    assert!(target.exists(), "the real target file must survive");
}

#[test]
fn stale_orphan_in_subdirectory_is_removed() {
    let dir = tempfile::TempDir::new().expect("TempDir");
    let subdir = dir.path().join("projects");
    fs::create_dir(&subdir).expect("mkdir");
    plant(&subdir.join("plan.org"), b"* real\n");
    let orphan = subdir.join(".plan.org.Zz9Aa1");
    plant(&orphan, b"stale");
    backdate(&orphan, Duration::from_secs(120));

    let report = clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 1, "recursive scan reaches subdirs");
    assert!(!orphan.exists());
}

#[test]
fn pattern_shaped_name_without_target_survives() {
    // Review finding (Story 3.1): "backup" is exactly 6 ASCII alphanumerics,
    // so `.archive.org.backup` matches the crate temp pattern by name alone.
    // Without an `archive.org` target in the same directory it is more
    // plausibly a user file than crate residue — the target-existence guard
    // must leave it alone no matter how old it is.
    let dir = tempfile::TempDir::new().expect("TempDir");
    let lookalike = dir.path().join(".archive.org.backup");
    plant(&lookalike, b"user-made backup, not crate residue");
    backdate(&lookalike, Duration::from_secs(3600));

    let report = clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 0);
    assert!(
        lookalike.exists(),
        "pattern-shaped name with no target must never be deleted"
    );
}

#[test]
fn user_dotfiles_and_fresh_temps_survive() {
    let dir = tempfile::TempDir::new().expect("TempDir");

    // Legitimate vault content.
    let notes = dir.path().join("notes.org");
    plant(&notes, b"* TODO real content\n");

    // User dotfiles — never touched, even when backdated.
    let gitignore = dir.path().join(".gitignore");
    plant(&gitignore, b"target/\n");
    backdate(&gitignore, Duration::from_secs(3600));

    // `.orgsidian/` settings dir + a stale non-.org temp inside it.
    let dot_orgsidian = dir.path().join(".orgsidian");
    fs::create_dir(&dot_orgsidian).expect("mkdir .orgsidian");
    let settings = dot_orgsidian.join("settings.toml");
    plant(&settings, b"schema_version = 1\n");
    let toml_temp = dot_orgsidian.join(".settings.toml.abc123");
    plant(&toml_temp, b"non-org temp");
    backdate(&toml_temp, Duration::from_secs(3600));

    // Fresh in-flight temp (current mtime) — a concurrent writer must never
    // be raced, so the 60s age guard leaves it alone. Its target exists, so
    // the age guard (not the target-existence guard) is what protects it.
    let draft = dir.path().join("draft.org");
    plant(&draft, b"* draft\n");
    let in_flight = dir.path().join(".draft.org.qW4rT7");
    plant(&in_flight, b"in-flight write");

    let report = clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 0, "nothing eligible for removal");
    for survivor in [&notes, &gitignore, &settings, &toml_temp, &in_flight] {
        assert!(survivor.exists(), "{survivor:?} must survive the scan");
    }
}

#[test]
fn hidden_org_target_orphan_is_removed_alongside_survivors() {
    // `.hidden.org.abc123` is the crate temp for target `hidden.org` — it
    // matches the full pattern and must be collected once stale, while the
    // survivors from the same directory stay put.
    let dir = tempfile::TempDir::new().expect("TempDir");
    let orphan = dir.path().join(".hidden.org.abc123");
    plant(&orphan, b"stale");
    backdate(&orphan, Duration::from_secs(120));
    let target = dir.path().join("hidden.org");
    plant(&target, b"* real\n");

    let report = clean_orphan_temp_files(dir.path()).expect("cleanup must succeed");

    assert_eq!(report.removed_count(), 1);
    assert!(!orphan.exists());
    assert!(target.exists(), "the real target file must survive");
}

#[cfg(unix)]
#[test]
fn symlinked_directories_are_not_followed() {
    // Cycle safety: the scan never follows symlinks, so orphans behind a
    // symlinked org folder are out of scope (documented on the API).
    let vault = tempfile::TempDir::new().expect("TempDir");
    let outside = tempfile::TempDir::new().expect("TempDir");
    plant(&outside.path().join("notes.org"), b"* real\n");
    let orphan = outside.path().join(".notes.org.aB3xY9");
    plant(&orphan, b"stale orphan behind a symlink");
    backdate(&orphan, Duration::from_secs(120));
    std::os::unix::fs::symlink(outside.path(), vault.path().join("linked"))
        .expect("symlink fixture");

    let report = clean_orphan_temp_files(vault.path()).expect("cleanup must succeed");

    assert_eq!(
        report.removed_count(),
        0,
        "linked subtree must not be scanned"
    );
    assert!(
        orphan.exists(),
        "orphans behind symlinked dirs are untouched"
    );
}
