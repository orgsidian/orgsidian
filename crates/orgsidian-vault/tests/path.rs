//! Story 3.6 (AC3): `.org` discovery, root canonicalization, and the
//! vault-relative path form — driven against real temp directories.

use std::fs;
use std::path::Path;

use orgsidian_vault::{canonicalize_vault_root, open_vault_root, scan_org_files, to_rel_path};
use tempfile::TempDir;

/// Create `root/rel` (parents included) with `contents`.
fn write_file(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).expect("create parent dirs");
    fs::write(&path, contents).expect("write file");
}

/// The relative, `/`-normalized forms of every discovered file, sorted.
fn rel_paths(root: &Path) -> Vec<String> {
    scan_org_files(root)
        .expect("scan")
        .iter()
        .filter_map(|f| to_rel_path(root, f))
        .collect()
}

#[test]
fn scan_finds_org_files_across_nested_dirs_deterministically() {
    let dir = TempDir::new().expect("tempdir");
    let root = canonicalize_vault_root(dir.path()).expect("canonicalize");

    write_file(&root, "top.org", "* a");
    write_file(&root, "sub/nested.org", "* b");
    write_file(&root, "sub/deep/leaf.org", "* c");

    let found = rel_paths(&root);
    assert_eq!(
        found,
        vec!["sub/deep/leaf.org", "sub/nested.org", "top.org"]
    );

    // Deterministic: a second scan yields the identical order.
    assert_eq!(rel_paths(&root), found);
}

#[test]
fn scan_skips_orgsidian_config_dir_and_dotdirs() {
    let dir = TempDir::new().expect("tempdir");
    let root = canonicalize_vault_root(dir.path()).expect("canonicalize");

    write_file(&root, "keep.org", "* keep");
    write_file(&root, ".orgsidian/settings.org", "* hidden config");
    write_file(&root, ".git/hooks/note.org", "* hidden vcs");
    write_file(&root, ".hidden/inside.org", "* hidden dir");

    assert_eq!(rel_paths(&root), vec!["keep.org"]);
}

#[test]
fn scan_ignores_non_org_files_and_matches_extension_case_insensitively() {
    let dir = TempDir::new().expect("tempdir");
    let root = canonicalize_vault_root(dir.path()).expect("canonicalize");

    write_file(&root, "notes.org", "* org");
    write_file(&root, "UPPER.ORG", "* also org");
    write_file(&root, "readme.md", "not org");
    write_file(&root, "data.json", "{}");
    write_file(&root, "orglike.organizer", "not org either");

    assert_eq!(rel_paths(&root), vec!["UPPER.ORG", "notes.org"]);
}

#[test]
fn to_rel_path_normalizes_and_rejects_escapes() {
    let dir = TempDir::new().expect("tempdir");
    let root = canonicalize_vault_root(dir.path()).expect("canonicalize");
    write_file(&root, "sub/note.org", "* x");

    let file = root.join("sub").join("note.org");
    assert_eq!(to_rel_path(&root, &file).as_deref(), Some("sub/note.org"));

    // A path outside the root is refused.
    assert_eq!(to_rel_path(&root, Path::new("/etc/passwd")), None);
    // The root itself is not a file-relative path.
    assert_eq!(to_rel_path(&root, &root), None);
}

#[cfg(unix)]
#[test]
fn canonicalize_resolves_a_symlinked_root() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new().expect("tempdir");
    let real = dir.path().join("real-vault");
    fs::create_dir(&real).expect("mkdir real");
    write_file(&real, "note.org", "* x");

    let link = dir.path().join("link-vault");
    symlink(&real, &link).expect("symlink");

    // Canonicalizing the symlink yields the real path; scanning through either
    // spelling produces the same vault-relative key (the two-spellings collapse
    // the path-identity policy exists to guarantee).
    let via_link = canonicalize_vault_root(&link).expect("canon link");
    let via_real = canonicalize_vault_root(&real).expect("canon real");
    assert_eq!(via_link, via_real);
    assert_eq!(rel_paths(&via_link), vec!["note.org"]);
}

#[test]
fn open_vault_root_canonicalizes_and_is_non_fatal_on_clean() {
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "note.org", "* x");

    let canonical = open_vault_root(dir.path()).expect("open vault root");
    let expected = canonicalize_vault_root(dir.path()).expect("canonicalize");
    assert_eq!(canonical, expected);
    // The vault still opened and its files are discoverable.
    assert_eq!(rel_paths(&canonical), vec!["note.org"]);
}

#[test]
fn canonicalize_errors_on_missing_root() {
    let dir = TempDir::new().expect("tempdir");
    let missing = dir.path().join("does-not-exist");
    assert!(canonicalize_vault_root(&missing).is_err());
}
