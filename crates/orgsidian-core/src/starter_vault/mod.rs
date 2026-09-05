//! Implements FR-18 (Personal GTD + Student starters; Freelancer + Empty deferred)
//!
//! A first-time user who picks a Starter Vault should see *their workflow*,
//! not a syntax reference, in the first five minutes (UJ-3, UJ-4). This
//! module is the generator: given a chosen [`StarterVaultKind`] and a
//! caller-designated target folder, it writes a small, realistic set of
//! `.org` files — one project (or course), an `inbox.org` at the vault root,
//! a journal, and a someday/maybe list — with agenda-bearing `SCHEDULED`/
//! `DEADLINE` timestamps anchored to a caller-supplied "today" so that
//! opening the freshly-generated Vault immediately shows non-empty Today and
//! Week Agenda content (Stories 6.3/6.4 consume the files this module ships;
//! this story only shapes their content).
//!
//! **Scope (locked 2026-09-05):** only [`StarterVaultKind::PersonalGtd`] and
//! [`StarterVaultKind::Student`] ship here. The Freelancer starter's ≥1
//! project / ≥3 milestones / ≥1 clocked-task / ≥1 backlink AC depends on
//! Story 8.7's BacklinksPanel (not yet built) and is deferred — see
//! `_bmad-output/implementation-artifacts/deferred-work.md`. The Empty
//! starter ships in Story 11.1 (v0.5 Beta).
//!
//! `today` is dependency-injected (never read from the wall clock in this
//! module), mirroring the `test_support::Clock` precedent: the (future)
//! Story 6.2 caller resolves the real first-launch date and passes it in,
//! which keeps every generated file byte-for-byte deterministic in tests.

mod personal_gtd;
mod student;

use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::error::OrgError;
use crate::index::vault_err;

/// Which built-in Starter Vault content set to generate (FR-18).
///
/// `Freelancer` and `Empty` are intentionally absent from this enum — see the
/// module doc-comment above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterVaultKind {
    /// Personal GTD (Getting Things Done): Inbox / Projects / Someday-Maybe,
    /// per David Allen's method — the everyday-life lighthouse persona flow.
    PersonalGtd,
    /// Student: Inbox / Courses (assignments + readings) / Someday, shaped
    /// around a term's coursework rhythm.
    Student,
}

impl StarterVaultKind {
    /// A short machine-stable slug, useful for logging/telemetry callers.
    pub fn slug(self) -> &'static str {
        match self {
            StarterVaultKind::PersonalGtd => "personal-gtd",
            StarterVaultKind::Student => "student",
        }
    }
}

/// Generate the chosen starter's `.org` files into `vault_root`.
///
/// `vault_root` is created (`create_dir_all`) if it does not already exist;
/// each file is written via the Story 3.1 [`orgsidian_vault::atomic_write`]
/// subsystem (temp-file-and-rename, AV-aware retry) at `vault_root/<name>`,
/// overwriting any file of the same name — the (future) Story 6.2 caller is
/// expected to invoke this only against a folder the user just designated as
/// a fresh Starter Vault target.
///
/// `today` anchors every `SCHEDULED`/`DEADLINE`/`CLOSED` timestamp in the
/// generated content: at least one Headline is scheduled for `today` itself
/// (Today Agenda) and others fall within the following 7 days (Week Agenda),
/// satisfying the "opening the Vault immediately shows non-empty Today/Week
/// Agenda content" AC once Stories 6.3/6.4's queries read this Vault.
///
/// # Errors
///
/// [`OrgError::Io`] if `vault_root` cannot be created; [`OrgError::Vault`] if
/// any file write fails (mapped from [`orgsidian_vault::VaultError`]).
pub fn generate_starter_vault(
    kind: StarterVaultKind,
    vault_root: &Path,
    today: NaiveDate,
) -> Result<(), OrgError> {
    std::fs::create_dir_all(vault_root).map_err(|source| OrgError::Io {
        reason: format!(
            "failed to create Starter Vault folder {}: {source}",
            vault_root.display()
        ),
    })?;

    for (name, content) in files_for(kind, today) {
        let path: PathBuf = vault_root.join(name);
        orgsidian_vault::atomic_write(&path, content.as_bytes()).map_err(vault_err)?;
    }
    Ok(())
}

/// The `(name, content)` file set for `kind`, anchored to `today`.
///
/// Single dispatch point shared by [`generate_starter_vault`] and the tests so
/// the two can never drift; adding a `StarterVaultKind` variant makes this
/// `match` a compiler-guided one-line edit.
fn files_for(kind: StarterVaultKind, today: NaiveDate) -> Vec<(&'static str, String)> {
    match kind {
        StarterVaultKind::PersonalGtd => personal_gtd::files(today),
        StarterVaultKind::Student => student::files(today),
    }
}

/// Render an active (`<...>`) org timestamp for `date` — no time-of-day, the
/// form every Starter Vault `SCHEDULED:`/`DEADLINE:` line uses.
pub(crate) fn active_timestamp(date: NaiveDate) -> String {
    date.format("<%Y-%m-%d %a>").to_string()
}

/// Render an inactive (`[...]`) org timestamp for `date` — the form
/// `CLOSED:` lines use (inactive so it never re-enters the Agenda).
pub(crate) fn inactive_timestamp(date: NaiveDate) -> String {
    date.format("[%Y-%m-%d %a]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::collections::HashSet;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 5).expect("valid date")
    }

    #[test]
    fn active_timestamp_matches_org_shape() {
        // 2026-09-05 is a Saturday.
        assert_eq!(active_timestamp(today()), "<2026-09-05 Sat>");
    }

    #[test]
    fn inactive_timestamp_matches_org_shape() {
        assert_eq!(inactive_timestamp(today()), "[2026-09-05 Sat]");
    }

    #[test]
    fn generate_personal_gtd_writes_inbox_at_vault_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        generate_starter_vault(StarterVaultKind::PersonalGtd, dir.path(), today())
            .expect("generate");
        assert!(dir.path().join("inbox.org").is_file());
    }

    #[test]
    fn generate_student_writes_inbox_at_vault_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        generate_starter_vault(StarterVaultKind::Student, dir.path(), today()).expect("generate");
        assert!(dir.path().join("inbox.org").is_file());
    }

    #[test]
    fn generate_creates_missing_vault_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("brand-new-vault");
        assert!(!nested.exists());
        generate_starter_vault(StarterVaultKind::PersonalGtd, &nested, today()).expect("generate");
        assert!(nested.join("inbox.org").is_file());
    }

    #[test]
    fn generated_file_names_are_unique_per_starter() {
        for kind in [StarterVaultKind::PersonalGtd, StarterVaultKind::Student] {
            let files = files_for(kind, today());
            let names: HashSet<_> = files.iter().map(|(name, _)| *name).collect();
            assert_eq!(
                names.len(),
                files.len(),
                "{:?} has duplicate filenames",
                kind
            );
            assert!(names.contains("inbox.org"), "{:?} missing inbox.org", kind);
        }
    }

    #[test]
    fn every_generated_file_parses_without_panicking() {
        for kind in [StarterVaultKind::PersonalGtd, StarterVaultKind::Student] {
            for (name, content) in files_for(kind, today()) {
                let doc = orgsidian_parser::analyze(&content).expect("parse never fails (LD-41)");
                assert!(
                    !doc.headlines.is_empty(),
                    "{:?}/{name} produced no headlines",
                    kind
                );
            }
        }
    }

    #[test]
    fn generate_writes_every_file_of_the_starter_to_disk() {
        // Not just inbox.org — a regression dropping files after the first must
        // be caught at the end-to-end layer.
        for kind in [StarterVaultKind::PersonalGtd, StarterVaultKind::Student] {
            let dir = tempfile::tempdir().expect("tempdir");
            generate_starter_vault(kind, dir.path(), today()).expect("generate");
            for (name, _) in files_for(kind, today()) {
                assert!(
                    dir.path().join(name).is_file(),
                    "{kind:?} did not write {name} to disk"
                );
            }
        }
    }

    #[test]
    fn generate_overwrites_a_preexisting_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let inbox = dir.path().join("inbox.org");
        std::fs::write(&inbox, b"STALE CONTENT").expect("seed stale file");
        generate_starter_vault(StarterVaultKind::PersonalGtd, dir.path(), today())
            .expect("generate");
        let written = std::fs::read_to_string(&inbox).expect("read back");
        assert_ne!(written, "STALE CONTENT", "inbox.org should be overwritten");
        assert!(written.contains("#+TITLE: Inbox"));
    }

    #[test]
    fn generate_errors_when_vault_root_is_an_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let as_file = dir.path().join("not-a-dir");
        std::fs::write(&as_file, b"i am a file").expect("seed file");
        let err = generate_starter_vault(StarterVaultKind::PersonalGtd, &as_file, today())
            .expect_err("create_dir_all over a regular file must fail");
        assert!(
            matches!(err, OrgError::Io { .. }),
            "expected OrgError::Io, got {err:?}"
        );
    }

    #[test]
    fn slugs_are_stable() {
        // `slug()` is a machine-stable contract for logging/telemetry callers.
        assert_eq!(StarterVaultKind::PersonalGtd.slug(), "personal-gtd");
        assert_eq!(StarterVaultKind::Student.slug(), "student");
    }

    #[test]
    fn generation_is_byte_for_byte_deterministic() {
        for kind in [StarterVaultKind::PersonalGtd, StarterVaultKind::Student] {
            assert_eq!(
                files_for(kind, today()),
                files_for(kind, today()),
                "{kind:?} content must be deterministic for a fixed today"
            );
        }
    }

    #[test]
    fn timestamps_render_correctly_across_calendar_boundaries() {
        // Year-end rollover and a leap day exercise the chrono date arithmetic
        // and the `%a` weekday recompute beyond the mid-month fixture above.
        let year_end = NaiveDate::from_ymd_opt(2026, 12, 31).expect("valid date");
        assert_eq!(active_timestamp(year_end), "<2026-12-31 Thu>");
        assert_eq!(
            active_timestamp(year_end + Duration::days(5)),
            "<2027-01-05 Tue>"
        );
        let leap = NaiveDate::from_ymd_opt(2028, 2, 28).expect("valid date");
        assert_eq!(
            inactive_timestamp(leap + Duration::days(1)),
            "[2028-02-29 Tue]"
        );
    }
}
