//! Story 1.9 — vault anchor smoke (anti-placebo-green per Party Mode P2).
//!
//! Round-trips `* TODO Hello\n` through `orgsidian_vault::atomic_write` and
//! asserts byte-identical read-back. Proves the atomic-write code path moves
//! bytes through the filesystem unchanged (no BOM, no line-ending munging,
//! no truncation) — the byte-identity check is the heart of the anchor.

#[test]
fn atomic_write_anchor_roundtrips_byte_identical() {
    const ANCHOR: &[u8] = b"* TODO Hello\n";

    let dir = tempfile::TempDir::new().expect("anchor TempDir must succeed");
    let target = dir.path().join("anchor.org");

    orgsidian_vault::atomic_write(&target, ANCHOR).expect("anchor atomic_write must succeed");

    let read_back = std::fs::read(&target).expect("read-back must succeed");
    assert_eq!(
        read_back, ANCHOR,
        "anchor.org must be byte-identical after atomic write"
    );
}
