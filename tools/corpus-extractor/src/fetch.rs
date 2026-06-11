//! Pinned fetch of the upstream `test-org-element.el` (AC2).
//!
//! The `.el` file is GPL-3.0-or-later and is **never vendored** into this MIT
//! repo (R-009 / LD-37 posture; rationale in ADR 0001). It is fetched at
//! extraction time into a gitignored cache and verified against a hard-coded
//! SHA-256. Pin bumps land only through a reviewed PR (LD-48-style
//! discipline): update the three constants below together.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Pinned org-mode release tag (latest stable at implementation time,
/// 2026-06-11; verified against the upstream tag list via `git ls-remote`).
pub const ORG_RELEASE_TAG: &str = "release_9.8.5";

/// SHA-256 of `testing/lisp/test-org-element.el` at [`ORG_RELEASE_TAG`].
pub const SOURCE_SHA256: &str = "f3065e65d71adc0c124da87d72366c6618fc042517f10fdf65490c04e1dfce6d";

/// File name inside the cache directory.
pub const CACHE_FILE: &str = "test-org-element.el";

/// Cache directory name under the tool root (gitignored at repo root).
pub const CACHE_DIR: &str = "cache";

/// Canonical upstream URL (GNU Savannah cgit raw view at the pinned tag).
pub fn upstream_url() -> String {
    format!(
        "https://git.savannah.gnu.org/cgit/emacs/org-mode.git/plain/testing/lisp/test-org-element.el?id={ORG_RELEASE_TAG}"
    )
}

/// Fallback mirror URL (GitHub `bzg/org-mode` raw view at the pinned tag).
pub fn fallback_url() -> String {
    format!(
        "https://raw.githubusercontent.com/bzg/org-mode/{ORG_RELEASE_TAG}/testing/lisp/test-org-element.el"
    )
}

/// Lower-case hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        // Writing to a String cannot fail; ignore the Infallible result
        // rather than unwrap (lib code carries no panics).
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Absolute path of the cached upstream file under `tool_root`.
pub fn cache_file_path(tool_root: &Path) -> PathBuf {
    tool_root.join(CACHE_DIR).join(CACHE_FILE)
}

/// Download the pinned file (canonical URL first, GitHub mirror as fallback),
/// verify its SHA-256 against [`SOURCE_SHA256`], and write it to the cache.
/// Fails hard on checksum mismatch (AC2) — the observed digest is reported so
/// a deliberate pin bump can copy it into [`SOURCE_SHA256`] via PR.
pub fn fetch_to_cache(tool_root: &Path) -> Result<PathBuf> {
    let primary = upstream_url();
    let bytes = match download(&primary) {
        Ok(bytes) => bytes,
        Err(primary_err) => {
            let fallback = fallback_url();
            download(&fallback).map_err(|fallback_err| {
                anyhow!(
                    "both upstream URLs failed.\n  canonical {primary}: {primary_err:#}\n  fallback {fallback}: {fallback_err:#}"
                )
            })?
        }
    };

    let observed = sha256_hex(&bytes);
    if observed != SOURCE_SHA256 {
        bail!(
            "checksum mismatch for {CACHE_FILE} at tag {ORG_RELEASE_TAG}:\n  expected (pinned): {SOURCE_SHA256}\n  observed (download): {observed}\nRefusing to cache. If this is a deliberate pin bump, update fetch::SOURCE_SHA256 together with fetch::ORG_RELEASE_TAG in a reviewed PR."
        );
    }

    let path = cache_file_path(tool_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating cache dir {}", parent.display()))?;
    }
    std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Read the cached upstream file, re-verifying the checksum. `extract` reads
/// ONLY this cache — a missing or stale cache is an actionable error pointing
/// the user at `fetch` (AC2; no network outside the `fetch` subcommand).
pub fn read_cached(tool_root: &Path) -> Result<String> {
    let path = cache_file_path(tool_root);
    let bytes = std::fs::read(&path).with_context(|| {
        format!(
            "no cached {CACHE_FILE} at {} — run `cargo run --manifest-path tools/corpus-extractor/Cargo.toml -- fetch` first",
            path.display()
        )
    })?;
    let observed = sha256_hex(&bytes);
    if observed != SOURCE_SHA256 {
        bail!(
            "cached {} does not match the pinned SHA-256 (expected {SOURCE_SHA256}, observed {observed}) — re-run `fetch` to refresh the cache",
            path.display()
        );
    }
    String::from_utf8(bytes).context("cached test-org-element.el is not valid UTF-8")
}

fn download(url: &str) -> Result<Vec<u8>> {
    let mut response = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let bytes = response
        .body_mut()
        .with_config()
        // test-org-element.el is ~0.7 MB; 20 MiB is a generous safety bound.
        .limit(20 * 1024 * 1024)
        .read_to_vec()
        .with_context(|| format!("reading body of {url}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // No network in any test (AC2): only the pure helpers are unit-tested.

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA-256("abc") — FIPS 180-2 test vector.
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn pin_triple_is_well_formed() {
        assert!(ORG_RELEASE_TAG.starts_with("release_"));
        assert_eq!(SOURCE_SHA256.len(), 64);
        assert!(SOURCE_SHA256.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(upstream_url().contains(ORG_RELEASE_TAG));
        assert!(fallback_url().contains(ORG_RELEASE_TAG));
    }

    #[test]
    fn read_cached_missing_cache_names_fetch() {
        let dir = std::env::temp_dir().join("corpus-extractor-test-no-cache");
        let err = read_cached(&dir).expect_err("missing cache must error");
        assert!(format!("{err:#}").contains("fetch"), "{err:#}");
    }
}
