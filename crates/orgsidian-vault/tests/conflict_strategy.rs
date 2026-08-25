//! Story 5.3 AC: a SINGLE test suite parameterized over both conflict-resolution
//! strategies, asserting the [`Resolution`] contract invariants (`Block`,
//! `WriteMerged`, `Cancel`) reached through the `&dyn ResolveConflict` injection
//! seam — the exact shape the Epic 5 watcher state machine consumes.
//!
//! An INTEGRATION test (not colocated) on purpose: it exercises the crate's
//! PUBLIC seam (`orgsidian_vault::{resolve_with, ConflictState, …}`) exactly as
//! the watcher will, so it also witnesses that the seam is reachable from
//! outside the crate.

use std::path::{Path, PathBuf};

use orgsidian_vault::{
    resolve_with, BlockWithWarning, ConflictState, ConflictStrategy, Resolution, ResolveConflict,
    Sha256Hash, ThreePaneMergeDialog,
};

const FILE: &str = "notes.org";

/// The invariant each case expects `resolve` to produce. Kept separate from the
/// `Resolution` value so the parameterized body asserts on the *variant*
/// (contract shape) rather than re-deriving payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedOutcome {
    Block,
    WriteMerged,
    Cancel,
}

/// One row of the parameterized suite: a human name, the active strategy as the
/// injected trait object, and the contract invariant it must honor.
struct Case {
    name: &'static str,
    strategy: Box<dyn ResolveConflict>,
    expected: ExpectedOutcome,
}

/// Both strategies, parameterized. `ThreePaneMergeDialog` appears with each of
/// its two decisions so the single suite reaches all three `Resolution`
/// invariants without a bespoke test per outcome.
fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "BlockWithWarning",
            strategy: Box::new(BlockWithWarning),
            expected: ExpectedOutcome::Block,
        },
        Case {
            name: "ThreePaneMergeDialog::accept",
            strategy: Box::new(ThreePaneMergeDialog::accept("* TODO merged\n")),
            expected: ExpectedOutcome::WriteMerged,
        },
        Case {
            name: "ThreePaneMergeDialog::cancel",
            strategy: Box::new(ThreePaneMergeDialog::cancel()),
            expected: ExpectedOutcome::Cancel,
        },
    ]
}

fn conflict() -> ConflictState {
    ConflictState::new(
        Sha256Hash::of(b"* TODO ancestor\n"),
        "* TODO external\n",
        "* TODO buffer\n",
        FILE,
    )
}

fn outcome_of(resolution: &Resolution) -> ExpectedOutcome {
    match resolution {
        Resolution::Block { .. } => ExpectedOutcome::Block,
        Resolution::WriteMerged { .. } => ExpectedOutcome::WriteMerged,
        Resolution::Cancel => ExpectedOutcome::Cancel,
        // `Resolution` is `#[non_exhaustive]`; a future variant must extend this
        // suite deliberately rather than pass by silent default.
        other => panic!("unhandled Resolution variant: {other:?}"),
    }
}

/// The single parameterized body: every strategy, injected as `&dyn
/// ResolveConflict` and resolved through `resolve_with`, honors its declared
/// contract invariant.
#[test]
fn strategy_honors_its_resolution_contract() {
    for case in cases() {
        let resolution = resolve_with(case.strategy.as_ref(), conflict());
        assert_eq!(
            outcome_of(&resolution),
            case.expected,
            "strategy `{}` produced the wrong Resolution variant",
            case.name
        );
    }
}

/// All three contract invariants are actually reachable across the suite — the
/// AC names `Block`, `WriteMerged`, and `Cancel` explicitly, so pin that the
/// parameter set covers each exactly.
#[test]
fn suite_covers_all_three_invariants() {
    let mut seen: Vec<ExpectedOutcome> = cases()
        .iter()
        .map(|c| {
            // Re-resolve through the seam so coverage is asserted on real output,
            // not on the declared `expected` alone.
            outcome_of(&resolve_with(c.strategy.as_ref(), conflict()))
        })
        .collect();
    seen.sort_by_key(|o| format!("{o:?}"));
    seen.dedup();
    assert_eq!(
        seen.len(),
        3,
        "the suite must reach Block, WriteMerged, and Cancel: {seen:?}"
    );
}

/// `Block` and `WriteMerged` carry the conflicted path through to the caller so
/// the watcher can emit its event / atomic-write without re-deriving it.
#[test]
fn payloads_carry_the_conflicted_path() {
    match resolve_with(&BlockWithWarning, conflict()) {
        Resolution::Block { path } => assert_eq!(path, Path::new(FILE)),
        other => panic!("expected Block, got {other:?}"),
    }

    match resolve_with(&ThreePaneMergeDialog::accept("merged body"), conflict()) {
        Resolution::WriteMerged {
            path,
            merged_content,
        } => {
            assert_eq!(path, PathBuf::from(FILE));
            assert_eq!(merged_content, "merged body");
        }
        other => panic!("expected WriteMerged, got {other:?}"),
    }
}

/// The `ConflictStrategy` selector enum — the value held at startup — resolves
/// identically to its wrapped strategy, and swapping the variant swaps the
/// outcome with no change to the resolving call site (AC5).
#[test]
fn selector_enum_swaps_without_touching_the_call_site() {
    fn resolve_active(active: &ConflictStrategy) -> Resolution {
        // Stands in for the watcher state machine: it holds the injected
        // resolver and never changes when the active strategy is swapped.
        resolve_with(active.as_resolver(), conflict())
    }

    let v01 = ConflictStrategy::BlockWithWarning(BlockWithWarning);
    assert_eq!(outcome_of(&resolve_active(&v01)), ExpectedOutcome::Block);

    let epic9 = ConflictStrategy::ThreePaneMergeDialog(ThreePaneMergeDialog::accept("m"));
    assert_eq!(
        outcome_of(&resolve_active(&epic9)),
        ExpectedOutcome::WriteMerged
    );
}
