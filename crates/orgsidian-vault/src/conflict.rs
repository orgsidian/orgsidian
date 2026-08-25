//! Conflict model + resolution strategy pattern (FR-16, Party Mode P0 rich form).
//!
//! FR-16 (Single Writer Rule) splits across two epics: Epic 5 ships the v0.1
//! *safe fallback* — an external write onto a file with a Dirty Buffer blocks
//! the save with a conflict warning — and Epic 9 replaces that fallback with a
//! full three-pane Merge Dialog. Party Mode P0 (Winston + Murat consensus) made
//! two structural rulings **binding from day-1**, before either resolution UI
//! exists, so Epic 9 does not have to rewrite the watcher state machine (the
//! "Epic 9 watcher-rewrite trap"):
//!
//! 1. **Rich state, not a boolean.** A conflict is a [`ConflictState`] carrying
//!    the ancestor content hash, the external (on-disk) content, the in-memory
//!    buffer content, and the file path — everything a three-way merge needs —
//!    even though the v0.1 UI consumes only the strategy *choice*.
//! 2. **Resolution is a strategy.** [`ResolveConflict`] is a trait; the active
//!    strategy is injected as a `&dyn ResolveConflict` at startup ([`resolve_with`]).
//!    Swapping [`BlockWithWarning`] (v0.1) for [`ThreePaneMergeDialog`] (Epic 9)
//!    is a one-line change at the injection site — the state machine that calls
//!    `resolve` never changes.
//!
//! # The watcher seam (this is a SEAM, not the wiring)
//!
//! The `orgsidian-watcher` state machine (Stories 5.1/5.2) is a SEPARATE crate,
//! and the LEAF graph rule forbids a vault→watcher dependency edge — so the
//! watcher cannot be wired here. What this module ships is the exact *contract*
//! the watcher's DIRTY branch will consume: build a [`ConflictState`] from the
//! [`crate::dirty_buffer::DirtyBufferManager`] buffer plus the fresh disk
//! content, then call [`resolve_with`] with the injected strategy. Story 5.4
//! (which stacks watcher + vault together) performs that call from inside the
//! state machine. Nothing here reacts to a filesystem event; it only models the
//! conflict and resolves it.
//!
//! # Purity
//!
//! Every type here is pure and in-memory: no I/O, no `Result`, no `tracing`.
//! This mirrors the [`crate::dirty_buffer`] scaffold — the vault owns the
//! conflict data model; reacting to filesystem events (Story 5.4) and shaping
//! the `ConflictDetected` IPC payload — which is where `serde` derives will be
//! added (Story 5.5 / Epic 9) — is the watcher / Tauri layer's job.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::hash::Sha256Hash;

/// A detected external-write conflict, modeled richly (Party Mode P0).
///
/// Fields are private and named exactly per the epic AC; construct with
/// [`ConflictState::new`] and read through the getters. The three-way material
/// a Merge Dialog needs is all here:
///
/// - `ancestor_hash` — SHA-256 of the last content the buffer and disk agreed
///   on (the common ancestor). A hash, not the bytes: the ancestor content
///   itself is recoverable from the index/history when needed. Divergence
///   detection and the three-way merge that consume this hash are Epic 9 — this
///   story ships the rich data, not the merge algorithm (Party Mode P0).
/// - `external_content` — the current on-disk content (the external write).
/// - `buffer_content` — the unsaved in-memory buffer (from
///   [`crate::dirty_buffer::DirtyBufferManager::get_buffer`], cloned by the
///   caller — see that module's borrow contract).
/// - `file_path` — the conflicted file (verbatim, as the Dirty Buffer keys it;
///   path-identity normalization is the Vault-open layer's concern).
pub struct ConflictState {
    ancestor_hash: Sha256Hash,
    external_content: String,
    buffer_content: String,
    file_path: PathBuf,
}

/// Redacting `Debug`: prints the path, the ancestor hash, and content
/// *byte-lengths* — never the content itself.
///
/// `external_content` and `buffer_content` are the user's notes (one of them
/// unsaved). A derived `Debug` would spill them verbatim into any enclosing
/// `{:?}`, panic backtrace, or log line — the same privacy leak
/// [`crate::dirty_buffer::DirtyBufferManager`] guards against. The hash is a
/// digest, not content, so it is shown in full.
impl fmt::Debug for ConflictState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConflictState")
            .field("file_path", &self.file_path)
            .field("ancestor_hash", &self.ancestor_hash)
            .field("external_content_len", &self.external_content.len())
            .field("buffer_content_len", &self.buffer_content.len())
            .finish()
    }
}

impl ConflictState {
    /// Assemble a conflict from its parts (the watcher's DIRTY branch does this
    /// once it has the fresh disk content and the buffer).
    pub fn new(
        ancestor_hash: Sha256Hash,
        external_content: impl Into<String>,
        buffer_content: impl Into<String>,
        file_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            ancestor_hash,
            external_content: external_content.into(),
            buffer_content: buffer_content.into(),
            file_path: file_path.into(),
        }
    }

    /// SHA-256 of the common-ancestor content.
    pub fn ancestor_hash(&self) -> Sha256Hash {
        self.ancestor_hash
    }

    /// The current on-disk (external) content.
    pub fn external_content(&self) -> &str {
        &self.external_content
    }

    /// The unsaved in-memory buffer content.
    pub fn buffer_content(&self) -> &str {
        &self.buffer_content
    }

    /// The conflicted file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

/// The outcome of resolving a conflict — the contract every [`ResolveConflict`]
/// strategy returns and every caller (the watcher state machine) branches on.
///
/// The three variants are the fixed invariants the strategy pattern is built
/// around: a strategy may block, write a merged result, or cancel — nothing
/// else. Adding a variant is a deliberate contract change (hence
/// `#[non_exhaustive]`, matching the crate's forward-compatible enum
/// convention), so downstream `match`es stay honest.
///
/// `#[must_use]`: a dropped `Resolution` silently discards the conflict decision
/// — no block, no write, no preserve — the exact bug the strategy pattern exists
/// to prevent.
#[must_use = "a Resolution is the conflict decision (block / write / preserve) and must be acted on"]
#[non_exhaustive]
pub enum Resolution {
    /// v0.1 [`BlockWithWarning`] outcome: refuse the save and surface the
    /// conflict on `path`. The buffer is left untouched (nothing is written);
    /// the user resolves manually. Carries the path so the caller can emit the
    /// `ConflictDetected { path }` event without re-deriving it.
    Block { path: PathBuf },
    /// [`ThreePaneMergeDialog`] accept: atomically write `merged_content` to
    /// `path` (via the Story 3.1 `atomic_write`), then mark the buffer clean.
    WriteMerged {
        path: PathBuf,
        merged_content: String,
    },
    /// The user cancelled resolution: write nothing and PRESERVE the Dirty
    /// Buffer (the caller must NOT `mark_clean`). Carries no payload — the caller
    /// still holds the path it built the [`ConflictState`] from, so a
    /// payload-free "leave everything as it was" outcome loses nothing.
    Cancel,
}

/// Redacting `Debug`: `WriteMerged.merged_content` is user note text, so print
/// its byte-length, not its bytes (same rationale as [`ConflictState`]'s
/// `Debug`). `Block`/`Cancel` carry no sensitive content.
impl fmt::Debug for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::Block { path } => f.debug_struct("Block").field("path", path).finish(),
            Resolution::WriteMerged {
                path,
                merged_content,
            } => f
                .debug_struct("WriteMerged")
                .field("path", path)
                .field("merged_content_len", &merged_content.len())
                .finish(),
            Resolution::Cancel => f.write_str("Cancel"),
        }
    }
}

/// The strategy pattern's core trait: resolve a [`ConflictState`] into a
/// [`Resolution`].
///
/// Object-safe by construction (one method, `&self` + owned `state`, concrete
/// return) so the active strategy travels as a `&dyn ResolveConflict` — the
/// injection seam ([`resolve_with`]) the watcher state machine consumes.
///
/// **`Send + Sync` supertrait.** The `orgsidian-watcher` state machine runs the
/// conflict path off a filesystem-event thread and holds the injected strategy
/// in shared state, so the trait object must cross thread boundaries. Requiring
/// `Send + Sync` here makes that a contract guarantee rather than a per-call-site
/// worry; every strategy in this module already satisfies it.
///
/// **`state` by value.** Resolving *consumes* the conflict — a resolved conflict
/// is spent — and by-value lets a strategy move the state's owned data into the
/// `Resolution` instead of cloning where its outcome allows it (e.g.
/// [`BlockWithWarning`] moves the path out of `state`).
///
/// **Sync + infallible by design (day-1 seam).** A genuine three-pane merge is
/// an async, interactive user flow, yet this trait is deliberately synchronous
/// and infallible: the day-1 model runs that interaction *upstream* — the dialog
/// produces a [`MergeDecision`] and `resolve` only *applies* it. If Epic 9 finds
/// it needs async or fallible resolution inside the strategy, evolving this
/// signature is the sanctioned seam change (recorded in `deferred-work.md`); the
/// state machine still consumes whatever `resolve_with` exposes.
pub trait ResolveConflict: Send + Sync {
    fn resolve(&self, state: ConflictState) -> Resolution;
}

/// The v0.1 Alpha strategy (Story 5.5): block the save and warn.
///
/// Zero-sized — it needs no configuration; every conflict resolves to
/// [`Resolution::Block`]. Epic 9 RETIRES this in favor of [`ThreePaneMergeDialog`].
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockWithWarning;

impl ResolveConflict for BlockWithWarning {
    fn resolve(&self, state: ConflictState) -> Resolution {
        Resolution::Block {
            path: state.file_path,
        }
    }
}

/// The decision a three-pane Merge Dialog interaction produces — the day-1
/// stand-in for real UI.
///
/// A genuine 3-pane merge (Epic 9) is driven by the user selecting hunks and
/// editing the merged pane; a pure `resolve(state) -> Resolution` cannot
/// conjure that interaction. So the day-1 [`ThreePaneMergeDialog`] is
/// *configured* with the decision the eventual dialog would return, which keeps
/// the strategy deterministic and testable and marks the precise point Epic 9
/// replaces (the dialog computes this decision live instead of receiving it).
#[derive(Clone)]
pub enum MergeDecision {
    /// The user accepted a merged result — write `merged_content`.
    Accept { merged_content: String },
    /// The user cancelled — preserve the buffer, write nothing.
    Cancel,
}

/// Redacting `Debug`: `Accept.merged_content` is user note text, so print its
/// byte-length, never its bytes. This is load-bearing for the whole module's
/// redaction guarantee — the derived `Debug` on [`ThreePaneMergeDialog`] and
/// [`ConflictStrategy`] both format a `MergeDecision` field, so redacting HERE
/// is what keeps `{:?}` on the startup strategy value from leaking merged text.
impl fmt::Debug for MergeDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeDecision::Accept { merged_content } => f
                .debug_struct("Accept")
                .field("merged_content_len", &merged_content.len())
                .finish(),
            MergeDecision::Cancel => f.write_str("Cancel"),
        }
    }
}

/// The Epic 9 strategy (day-1 placeholder): resolve via a three-pane Merge
/// Dialog.
///
/// Holds the [`MergeDecision`] stand-in described above. `accept`/`cancel`
/// constructors keep call sites readable.
#[derive(Debug, Clone)]
pub struct ThreePaneMergeDialog {
    decision: MergeDecision,
}

impl ThreePaneMergeDialog {
    /// Build the strategy from a concrete decision.
    pub fn new(decision: MergeDecision) -> Self {
        Self { decision }
    }

    /// Convenience: an accept-with-merged-content decision.
    pub fn accept(merged_content: impl Into<String>) -> Self {
        Self::new(MergeDecision::Accept {
            merged_content: merged_content.into(),
        })
    }

    /// Convenience: a cancel decision.
    pub fn cancel() -> Self {
        Self::new(MergeDecision::Cancel)
    }
}

impl ResolveConflict for ThreePaneMergeDialog {
    fn resolve(&self, state: ConflictState) -> Resolution {
        match &self.decision {
            MergeDecision::Accept { merged_content } => Resolution::WriteMerged {
                path: state.file_path,
                merged_content: merged_content.clone(),
            },
            MergeDecision::Cancel => Resolution::Cancel,
        }
    }
}

/// The set of available conflict-resolution strategies — the value held at
/// startup and the thing Epic 9 "swaps".
///
/// Each variant wraps its concrete strategy (both `impl ResolveConflict`), so
/// the app constructs ONE `ConflictStrategy` and hands the watcher its trait
/// object via [`ConflictStrategy::as_resolver`]. The enum itself also
/// `impl ResolveConflict`, so `&ConflictStrategy` is directly usable as a
/// `&dyn ResolveConflict`. Selecting a different variant at the injection site
/// is the entire "swap the strategy" operation (AC5).
#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    /// v0.1 Alpha: [`BlockWithWarning`].
    BlockWithWarning(BlockWithWarning),
    /// Epic 9: [`ThreePaneMergeDialog`].
    ThreePaneMergeDialog(ThreePaneMergeDialog),
}

impl ConflictStrategy {
    /// Borrow the active strategy as the `&dyn ResolveConflict` trait object the
    /// watcher state machine is injected with.
    pub fn as_resolver(&self) -> &dyn ResolveConflict {
        match self {
            ConflictStrategy::BlockWithWarning(strategy) => strategy,
            ConflictStrategy::ThreePaneMergeDialog(strategy) => strategy,
        }
    }
}

impl ResolveConflict for ConflictStrategy {
    fn resolve(&self, state: ConflictState) -> Resolution {
        self.as_resolver().resolve(state)
    }
}

/// The injection seam the watcher state machine's DIRTY branch consumes.
///
/// This is the whole point of the strategy pattern: the state machine holds an
/// injected `&dyn ResolveConflict` (chosen once at startup) and calls this to
/// turn a [`ConflictState`] into a [`Resolution`], WITHOUT knowing or caring
/// which concrete strategy it holds. Story 5.4 wires this call in from inside
/// `orgsidian-watcher`; swapping v0.1 [`BlockWithWarning`] for Epic 9
/// [`ThreePaneMergeDialog`] is a one-line change at the injection site and
/// touches no state-machine code (AC3, AC5).
///
/// It is a thin forwarder (`strategy.resolve(state)`) — its value is as the
/// named, documented, test-covered contract point, not as logic.
pub fn resolve_with(strategy: &dyn ResolveConflict, state: ConflictState) -> Resolution {
    strategy.resolve(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time witness that the trait is object-safe — the property the
    /// `&dyn ResolveConflict` injection seam depends on.
    fn _assert_object_safe(_: &dyn ResolveConflict) {}

    /// Compile-time witness that strategies (and the boxed trait object) are
    /// `Send + Sync` — required to inject the active strategy across the
    /// watcher's filesystem-event thread boundary.
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn strategies_are_send_sync_for_the_watcher_thread() {
        _assert_send_sync::<BlockWithWarning>();
        _assert_send_sync::<ThreePaneMergeDialog>();
        _assert_send_sync::<ConflictStrategy>();
        // The injected trait object itself must cross the watcher thread.
        _assert_send_sync::<Box<dyn ResolveConflict>>();
    }

    fn sample_state() -> ConflictState {
        ConflictState::new(
            Sha256Hash::of(b"* TODO ancestor\n"),
            "* TODO external edit\n",
            "* TODO buffer edit\n",
            "notes.org",
        )
    }

    #[test]
    fn conflict_state_exposes_all_rich_fields() {
        let state = sample_state();
        assert_eq!(state.ancestor_hash(), Sha256Hash::of(b"* TODO ancestor\n"));
        assert_eq!(state.external_content(), "* TODO external edit\n");
        assert_eq!(state.buffer_content(), "* TODO buffer edit\n");
        assert_eq!(state.file_path(), Path::new("notes.org"));
    }

    #[test]
    fn block_with_warning_always_blocks_carrying_the_path() {
        let resolution = BlockWithWarning.resolve(sample_state());
        match resolution {
            Resolution::Block { path } => assert_eq!(path, Path::new("notes.org")),
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn merge_dialog_accept_writes_merged_content() {
        let strategy = ThreePaneMergeDialog::accept("* TODO merged\n");
        match strategy.resolve(sample_state()) {
            Resolution::WriteMerged {
                path,
                merged_content,
            } => {
                assert_eq!(path, Path::new("notes.org"));
                assert_eq!(merged_content, "* TODO merged\n");
            }
            other => panic!("expected WriteMerged, got {other:?}"),
        }
    }

    #[test]
    fn merge_dialog_cancel_preserves_the_buffer() {
        let strategy = ThreePaneMergeDialog::cancel();
        assert!(matches!(
            strategy.resolve(sample_state()),
            Resolution::Cancel
        ));
    }

    #[test]
    fn selector_enum_delegates_to_the_wrapped_strategy() {
        let block = ConflictStrategy::BlockWithWarning(BlockWithWarning);
        assert!(matches!(
            block.resolve(sample_state()),
            Resolution::Block { .. }
        ));

        let merge = ConflictStrategy::ThreePaneMergeDialog(ThreePaneMergeDialog::accept("merged"));
        assert!(matches!(
            merge.resolve(sample_state()),
            Resolution::WriteMerged { .. }
        ));

        // `as_resolver` yields the same trait object the watcher is injected with.
        _assert_object_safe(block.as_resolver());
    }

    #[test]
    fn resolve_with_forwards_through_the_dyn_seam() {
        // The watcher holds `&dyn ResolveConflict` and calls `resolve_with`
        // without knowing the concrete strategy — swapping the object swaps the
        // outcome with zero call-site change.
        let strategies: Vec<Box<dyn ResolveConflict>> = vec![
            Box::new(BlockWithWarning),
            Box::new(ThreePaneMergeDialog::accept("merged")),
        ];
        let outcomes: Vec<Resolution> = strategies
            .iter()
            .map(|s| resolve_with(s.as_ref(), sample_state()))
            .collect();
        assert!(matches!(outcomes[0], Resolution::Block { .. }));
        assert!(matches!(outcomes[1], Resolution::WriteMerged { .. }));
    }

    /// `Debug` must never leak the buffer or external note text; it must still
    /// carry the debugging-useful metadata (path, both content lengths, hash).
    #[test]
    fn debug_redacts_conflict_content() {
        let state = ConflictState::new(
            Sha256Hash::of(b"ancestor"),
            "EXTERNAL SECRET NOTE", // 20 bytes
            "BUFFER SECRET NOTE",   // 18 bytes
            "diary.org",
        );
        let rendered = format!("{state:?}");
        assert!(!rendered.contains("SECRET"), "no note text: {rendered}");
        assert!(rendered.contains("diary.org"), "path stays visible");
        assert!(
            rendered.contains("external_content_len: 20"),
            "external length shown: {rendered}"
        );
        assert!(
            rendered.contains("buffer_content_len: 18"),
            "buffer length shown: {rendered}"
        );
        assert!(
            rendered.contains(&Sha256Hash::of(b"ancestor").to_string()),
            "ancestor hash shown: {rendered}"
        );
    }

    /// The strategy value held at startup (`ConflictStrategy`, derived `Debug`)
    /// must not leak the merged note text either — its `Debug` delegates through
    /// `MergeDecision`'s redacting impl.
    #[test]
    fn debug_redacts_strategy_merge_content() {
        let strategy = ConflictStrategy::ThreePaneMergeDialog(ThreePaneMergeDialog::accept(
            "MERGED SECRET NOTE",
        ));
        let rendered = format!("{strategy:?}");
        assert!(!rendered.contains("SECRET"), "{rendered}");
        assert!(
            rendered.contains("merged_content_len"),
            "length shown: {rendered}"
        );
    }

    /// `Resolution::WriteMerged` `Debug` must not leak the merged note text.
    #[test]
    fn debug_redacts_write_merged_content() {
        let resolution = Resolution::WriteMerged {
            path: PathBuf::from("m.org"),
            merged_content: "MERGED SECRET".to_string(),
        };
        let rendered = format!("{resolution:?}");
        assert!(!rendered.contains("SECRET"), "{rendered}");
        assert!(rendered.contains("m.org"));
    }
}
