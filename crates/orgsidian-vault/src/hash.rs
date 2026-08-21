//! `Sha256Hash`: a minimal 32-byte SHA-256 digest newtype (Story 5.3).
//!
//! The workspace had no content-hash type when Story 5.3 landed, and the rich
//! [`ConflictState`](crate::conflict::ConflictState) needs one for its
//! `ancestor_hash` field — the digest of the common-ancestor content that the
//! external write and the in-memory buffer both diverged from (FR-16). Rather
//! than reach for a bare `[u8; 32]` (which loses the "this is a SHA-256"
//! meaning and invites mixing with unrelated byte arrays), the model gets a
//! named newtype.
//!
//! This type is deliberately **not** conflict-specific: a SHA-256 of file
//! content is a general primitive. A future consumer (e.g. an index
//! content-hash for incremental re-sync) may promote it to a shared location;
//! it lives in the vault LEAF for now because that is where the first consumer
//! (the conflict model) landed.
//!
//! Backed by the `sha2` crate (already in `Cargo.lock` transitively via Tauri —
//! Story 5.3 adds a dependency *edge*, no new crates). The digest is not secret,
//! so `Debug`/`Display` render the full lowercase hex — unlike buffer content,
//! which the conflict types redact.

use std::fmt;

use sha2::{Digest, Sha256};

/// A 32-byte SHA-256 digest.
///
/// `Copy` (32 bytes is cheap to pass by value) and `Hash`/`Eq` so it can key a
/// map or be compared directly. Construct with [`Sha256Hash::of`] (hash some
/// bytes) or [`Sha256Hash::from_bytes`] (wrap a digest computed elsewhere).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Hash([u8; 32]);

impl Sha256Hash {
    /// Compute the SHA-256 of `bytes`.
    ///
    /// Deterministic: identical input always yields an identical hash, and any
    /// difference in input yields a different hash (with cryptographic
    /// probability). This is the constructor the conflict model uses to stamp
    /// an ancestor's content.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        // `Sha256::digest` returns a `GenericArray<u8, U32>`; copy its 32 bytes
        // into the owned array. `.into()` on the fixed-size array is infallible.
        Self(digest.into())
    }

    /// Wrap 32 digest bytes computed elsewhere (e.g. read back from the index).
    ///
    /// No hashing happens — the caller asserts these bytes ARE a SHA-256.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw 32 digest bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Lowercase hex, e.g. `e3b0c44298fc1c14...`. The digest is not sensitive, so
/// the full value is shown (contrast the redacting `Debug` on the conflict
/// types, which hide user note content).
impl fmt::Display for Sha256Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// `Sha256Hash(e3b0c442…)` — the hex digest wrapped so it is legible in a
/// `{:?}` on enclosing state.
impl fmt::Debug for Sha256Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Hash({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SHA-256 of the empty input is a well-known constant — pins the
    /// backing algorithm so a future swap of the hashing crate cannot silently
    /// change the digest.
    #[test]
    fn empty_input_matches_known_vector() {
        let hash = Sha256Hash::of(b"");
        assert_eq!(
            hash.to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn of_is_deterministic_and_input_sensitive() {
        let a = Sha256Hash::of(b"* TODO ancestor\n");
        let b = Sha256Hash::of(b"* TODO ancestor\n");
        let c = Sha256Hash::of(b"* DONE ancestor\n");

        assert_eq!(a, b, "same bytes hash equal");
        assert_ne!(a, c, "different bytes hash differ");
    }

    #[test]
    fn from_bytes_round_trips() {
        let hash = Sha256Hash::of(b"round trip");
        let bytes = *hash.as_bytes();
        assert_eq!(Sha256Hash::from_bytes(bytes), hash);
    }

    #[test]
    fn display_is_64_hex_chars() {
        let rendered = Sha256Hash::of(b"anything").to_string();
        assert_eq!(rendered.len(), 64, "32 bytes → 64 hex nibbles");
        assert!(
            rendered.bytes().all(|b| b.is_ascii_hexdigit()),
            "{rendered}"
        );
    }

    #[test]
    fn debug_wraps_the_hex() {
        let hash = Sha256Hash::of(b"");
        assert_eq!(
            format!("{hash:?}"),
            "Sha256Hash(e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855)"
        );
    }
}
