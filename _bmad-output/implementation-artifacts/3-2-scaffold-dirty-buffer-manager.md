# Story 3.2: Scaffold Dirty Buffer manager

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Metadata

github_issue: 26

## Story

As the **user editing my `.org` file**,
I want Orgsidian to track which open files have unsaved buffer changes (Dirty Buffer state) keyed by file path,
So that Epic 5 can enforce the Single Writer Rule and Epic 9 can route external writes to the Merge Dialog.

**Traces:** LD-7 (Single Writer Rule + Dirty Buffer + Merge Dialog integrity contract, architecture.md:69), FR-16 (external-write routing: clean → auto-reload, dirty → Merge Dialog), NFR-16 (Single Writer Rule reliability), external-write data-flow (`orgsidian-vault::dirty_buffer checks state for that path`, architecture.md:1131-1133), FR-16 traceability row (`orgsidian-vault/src/dirty_buffer.rs`, architecture.md:1059).

## Scope Fence (read first)

This story is the **Dirty Buffer manager scaffold** inside `crates/orgsidian-vault/`: one new module `src/dirty_buffer.rs` with a thread-safe `DirtyBufferManager` and its lifecycle unit tests. The epic title word is **"scaffold"** — ship the data structure + API surface + tests, nothing that consumes it. It is **NOT**:

- **NOT the watcher / `ConflictStrategy`** (Epic 5, Stories 5.1/5.3/5.4/5.5). No `notify-rs`, no debounce, no `ConflictState` rich struct, no `BlockWithWarning`. This story does not *react* to external writes; it only tracks dirty state. The data-flow arrow `dirty_buffer checks state → CLEAN/DIRTY branch` is Epic 5's wiring — here we ship only the `is_dirty(path)` the branch will call.
- **NOT the Merge Dialog** (Epic 9). No `shell-ui/src/components/merge/`, no `resolveMerge`.
- **NOT Tauri state wiring.** The manager is a plain library type. Registering it into `tauri::State` / a shared app handle is Epic 5/6 turf (whoever first *opens* a Vault). Do **not** add a `#[tauri::command]`, do not touch `orgsidian-shell-app`, do not invent a `Vault` struct to host it (same rule as Story 3.1 AC5).
- **NOT SQLite / index** (Story 3.3+). No `rusqlite`, nothing in `orgsidian-index`.
- **NOT LD-41 harness turf.** The "external delete with Dirty Buffer" LD-41 row needs the watcher — it is Epic 5's placeholder, not this story's. `tests/failure_modes.rs`, `tests/failure_modes_coverage.rs`, and `docs/failure-modes/coverage-matrix.md` stay **byte-untouched** (`EXPECTED_REMAINING_PLACEHOLDERS` stays at 8).
- **NOT path-normalization / canonicalization policy.** Key by the `PathBuf` the caller supplies (Dev Note 3). Case-folding + symlink + relative-vs-absolute normalization is a cross-cutting concern already deferred to the index/Vault-open stories (Story 3.1 deferred the parallel `.org` case-match question). Do not solve it here.
- **NOT sentinel turf.** Byte-untouched: `crates/orgsidian-vault/tests/anchor.rs`, `src/atomic.rs`, `src/error.rs` (the manager does no I/O — it needs no `VaultError` variant), `.github/workflows/*`, `deny.toml` (no new deps → `cargo deny` cannot regress).

The deliverable is exactly: `src/dirty_buffer.rs` (the manager + API + colocated `#[cfg(test)]` lifecycle tests) + a `lib.rs` re-export (AC1). Zero new dependencies (AC5).

## Acceptance Criteria

### AC1 — `src/dirty_buffer.rs` module owns the type; it is importable at the crate root.

- `crates/orgsidian-vault/src/dirty_buffer.rs` exists with a module doc header naming the LD/FR traces (LD-7 + FR-16 + NFR-16), matching the `atomic.rs`/`error.rs` grep-smoke precedent.
- `lib.rs` gains `pub mod dirty_buffer;` and `pub use dirty_buffer::DirtyBufferManager;` so `orgsidian_vault::DirtyBufferManager` resolves. Update the `lib.rs` crate doc: the existing forward-looking sentence "Story 3.2 adds the Dirty Buffer module alongside" becomes a present-tense description of what shipped.
- `pub struct DirtyBufferManager` with `#[derive(Debug, Default)]` and a `pub fn new() -> Self` (idiomatic even with `Default`).
- `src/atomic.rs`, `src/error.rs`, and `tests/anchor.rs` are **byte-untouched** (sentinel discipline — see Dev Note 1). `lib.rs` is the only pre-existing file modified.

### AC2 — Public API surface exactly matches the epic.

- `pub fn mark_dirty(&mut self, path: impl Into<PathBuf>, content: impl Into<String>)` — records/overwrites the unsaved buffer content for `path`. Re-marking an already-dirty path replaces its stored content.
- `pub fn mark_clean(&mut self, path: &Path)` — clears dirty state for `path`. Calling it on a path that is not tracked is a **no-op** (does not panic).
- `pub fn is_dirty(&self, path: &Path) -> bool`.
- `pub fn get_buffer(&self, path: &Path) -> Option<&str>` — the currently-buffered content for a dirty path, or `None` when clean/untracked. Signature is literal `Option<&str>` per the epic (see Dev Note 2 for why this drives the storage/locking choice).
- Content is stored as owned UTF-8 `String` keyed by `PathBuf` in a `std::collections::HashMap<PathBuf, String>`. (`.org` files are UTF-8 text; `String` is the correct buffer type and makes `get_buffer -> Option<&str>` a free borrow.)
- No I/O in this type — it is a pure in-memory registry. It therefore returns no `Result` and touches no `VaultError`.

### AC3 — Thread-safe, `Send + Sync`, with the sharing pattern documented.

- `DirtyBufferManager` must be `Send + Sync` (a `HashMap<PathBuf, String>` is — assert it with a compile-time `fn _assert_send_sync<T: Send + Sync>() {}` witness in the test module).
- **Chosen concurrency shape (Dev Note 2):** the manager is a *plain* struct — `&self` for reads (`is_dirty`, `get_buffer`), `&mut self` for mutations (`mark_dirty`, `mark_clean`) — and thread-safety is delivered by the documented shared handle `Arc<RwLock<DirtyBufferManager>>`. This keeps `get_buffer -> Option<&str>` honest (a borrow through a `RwLockReadGuard` is legal at the call site) and adds **zero dependencies**. `RwLock` (not `Mutex`) because the watcher will hammer `is_dirty` on the read path (many readers, rare writers).
- The module doc states the intended shared handle explicitly. A convenience alias `pub type SharedDirtyBuffers = Arc<RwLock<DirtyBufferManager>>` is encouraged (discoverability for the Epic 5/6 consumer) but optional.
- **Permitted deviation, disclose if taken:** if the dev instead chooses interior locking (`Arc<RwLock<HashMap<…>>>` *inside* the struct), then `get_buffer` must return `Option<String>` (a clone — you cannot return a borrow outliving the internal guard). That is an epic-signature variance to record in Completion Notes + deferred-work. The plain-struct shape above avoids the variance and is the recommended default.

### AC4 — Unit tests cover the full lifecycle and edges (colocated `#[cfg(test)] mod tests`).

- **Lifecycle clean → dirty → save → clean** (the epic's mandated case): a fresh path `is_dirty == false`; after `mark_dirty` it is `true` and `get_buffer` returns the content; after `mark_clean` it is `false` and `get_buffer` returns `None`.
- `get_buffer` returns `None` for an untracked path and after `mark_clean`.
- Re-`mark_dirty` on an already-dirty path **replaces** the content (assert the second value wins).
- `mark_clean` on an untracked path is a no-op (no panic; other entries unaffected).
- Two distinct paths are tracked independently (marking one clean leaves the other dirty).
- Concurrency smoke test: wrap in `Arc<RwLock<DirtyBufferManager>>`, spawn N threads doing interleaved `mark_dirty`/`is_dirty`, join, assert a consistent final state — proves the shared handle compiles and is race-free (deterministic assertion on final state, not on timing).
- `_assert_send_sync::<DirtyBufferManager>()` compile-time witness (AC3).

### AC5 — Gates stay green; zero dependency delta.

- `cargo test --workspace` green (in particular `-p orgsidian-vault`); `cargo clippy -- -D warnings` + `cargo fmt --check` clean on the touched crate; `cargo deny check` + `cargo audit` green (they cannot regress — no dependency change).
- **Dependency delta is exactly zero.** No new entries in `orgsidian-vault/Cargo.toml` (`std::collections::HashMap`, `std::sync::{Arc, RwLock}`, `std::path::{Path, PathBuf}` cover everything). No workspace-level changes.
- No `unwrap`/`expect`/`panic!` in committed non-test code (Story 2.8 discipline).
- Commitlint-conformant conventional commit; plain message, no AI-credit trailers.

## Tasks / Subtasks

- [x] Task 1 — Module + type + re-export (AC1, AC2)
  - [x] Create `src/dirty_buffer.rs` with module doc header (LD-7 + FR-16 + NFR-16 traces)
  - [x] `#[derive(Debug, Default)] pub struct DirtyBufferManager { buffers: HashMap<PathBuf, String> }` + `pub fn new()`
  - [x] `mark_dirty` / `mark_clean` / `is_dirty` / `get_buffer` per AC2 signatures
  - [x] `lib.rs`: add `pub mod dirty_buffer;` + `pub use dirty_buffer::DirtyBufferManager;`; rewrite the forward-looking crate-doc sentence to present tense
- [x] Task 2 — Thread-safety shape + sharing docs (AC3)
  - [x] Plain-struct shape (`&self` reads / `&mut self` mutations); `RwLock` rationale in module doc
  - [x] Document the `Arc<RwLock<DirtyBufferManager>>` shared handle; optional `SharedDirtyBuffers` alias
- [x] Task 3 — Tests (AC4)
  - [x] Colocated `#[cfg(test)] mod tests`: lifecycle, get_buffer None-cases, re-mark replaces, mark_clean no-op, independent paths
  - [x] Concurrency smoke test through `Arc<RwLock<_>>`; `_assert_send_sync` witness
- [x] Task 4 — Gates (AC5)
  - [x] `cargo test --workspace`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny check`, `cargo audit`
  - [x] Confirm zero dependency delta; no `unwrap`/`expect`/`panic!` in non-test code

## Dev Notes

### 1. Sentinel + scope discipline — one module, one re-export, nothing else

`crates/orgsidian-vault/tests/anchor.rs` (Story 1.9 anti-placebo sentinel) stays byte-untouched — it exercises the atomic-write path, which this story does not touch. `src/atomic.rs` and `src/error.rs` are likewise untouched: the manager does **no I/O**, so it needs no new `VaultError` variant and no `Result` return. The *only* pre-existing file you edit is `src/lib.rs` (module decl + re-export + the one doc-sentence tense fix). If you find yourself editing `atomic.rs`, `error.rs`, the watcher/index crates, or the LD-41 harness, stop — you have left the scope fence.

### 2. Why `get_buffer -> Option<&str>` forces the concurrency shape (the one real design decision)

The epic pins two things that pull against each other: `get_buffer(path) -> Option<&str>` **and** "thread-safe (`Arc<Mutex<…>>` or `Arc<RwLock<…>>`)". If the lock lives *inside* the struct, `get_buffer` cannot return `&str` — the borrow would have to outlive the `RwLockReadGuard` that drops at the end of the method, which does not compile; you'd be forced to `Option<String>` (clone), silently deviating from the epic signature.

**Resolution (already decided — implement this, do not relitigate):** make `DirtyBufferManager` a *plain* struct holding `HashMap<PathBuf, String>`, with `&self` read methods and `&mut self` mutators. Thread-safety is the caller's `Arc<RwLock<DirtyBufferManager>>`: `manager.read().unwrap().get_buffer(p)` returns `Option<&str>` borrowing through the guard, which is legal at the call site because the guard is alive there. This (a) keeps the epic signature literal, (b) adds zero deps, (c) gives many-reader concurrency for the hot `is_dirty` path the Epic 5 watcher will call on every debounced FS event. `RwLock` over `Mutex` for exactly that read-heavy asymmetry. The epic's "per implementation choice" clause is satisfied — we chose `Arc<RwLock<…>>`. The interior-locking alternative + its `Option<String>` cost is documented in AC3 as a disclosed fallback only.

### 3. Path keying — take paths verbatim, defer normalization

Key the map on the `PathBuf` the caller hands in. Do **not** canonicalize, lowercase, or resolve symlinks: (a) canonicalization is I/O and fallible, which would drag `VaultError` and a `Result` into a type that should be pure and infallible; (b) path-identity policy (case-folding on macOS/Windows, symlinks, relative-vs-absolute) is a cross-cutting Vault concern already deferred to the index/Vault-open stories — Story 3.1 deferred the sibling `.org` case-sensitivity question for the same reason. The contract this story ships is "same `PathBuf` in → same entry"; making callers pass consistent (canonical) paths is the Vault-open layer's job in Epic 5/6. Add a one-line deferred-work note flagging "DirtyBufferManager keys on raw PathBuf; path-identity normalization owned by Vault-open (Epic 5/6)" so the consumer wires it knowingly.

### 4. Downstream consumers — build the API they will actually call

- **Epic 5 watcher (Story 5.4/5.5):** on a debounced external write it calls `is_dirty(path)` to branch CLEAN (auto-reload) vs DIRTY (block/merge). This is the hot read path — hence `RwLock`.
- **Epic 5 `ConflictState` (Story 5.3):** the rich struct carries `buffer_content` — sourced from `get_buffer(path)`. In practice the consumer clones it out to build `ConflictState`, so `&str` (borrow, caller clones if needed) is the right primitive — do not pre-clone inside the manager.
- **Epic 9 Merge Dialog / `resolveMerge` (Story 9.3):** on accept, atomic-writes the merged content (Story 3.1 `atomic_write`) then `mark_clean(path)`. On cancel, the Dirty Buffer is *preserved* — i.e. nothing calls `mark_clean`. Your `mark_clean` semantics (clear-if-present, no-op otherwise) support both without special-casing.

Ship the four methods and stop — resist adding `all_dirty() -> Vec<…>`, event emission, or persistence. Those are unrequested and would widen the `#[non_exhaustive]`-free surface before a real consumer defines the need.

### 5. Code conventions (established — follow exactly)

- Module doc header naming LD/FR traces (Story 1.17/1.18/3.1 precedent): LD-7, FR-16, NFR-16.
- Doc comments on public items encouraged (vault is not `plugin-api`; `cargo doc` cleanliness is encouraged-not-gated).
- No `unwrap`/`expect`/`panic!` in committed non-test code (Story 2.8). Tests may `.unwrap()` the `RwLock` guards freely — a poisoned lock in a test *should* fail the test.
- Rust unit tests: `#[cfg(test)] mod tests` at the bottom of the source file, colocated (architecture.md:724). No separate `tests/dirty_buffer.rs` needed — the type has no I/O fixtures, so the colocated module is the right home (unlike `atomic.rs`, which earned a separate integration test for its trait-fake fault injection).

### Project Structure Notes

- `crates/orgsidian-vault/src/` goes from `lib.rs + atomic.rs + error.rs` to `+ dirty_buffer.rs`. `lib.rs` stays thin (decls + re-exports) — Story 3.1 deliberately kept it that way so this module lands cleanly.
- No new `tests/` file (tests are colocated per Dev Note 5). Existing `tests/{anchor,atomic,orphan_cleanup}.rs` are untouched.
- `Cargo.toml`: **no delta** (std-only). Root workspace file untouched.
- Branch per repo convention: `story/3.2-dirty-buffer-manager` off `main`; conventional commits (commitlint gate); plain commit messages, no AI-credit trailers.

### Testing Standards Summary

- Unit tests colocated (`#[cfg(test)] mod tests`), deterministic — the concurrency test asserts on **final state after join**, never on timing/ordering (Story 3.1 lesson: never depend on wall-clock or scheduling in tests).
- No filesystem fixtures needed (pure in-memory type) — no `tempfile`, no real paths on disk; construct `PathBuf::from("a.org")` literals.
- `_assert_send_sync::<DirtyBufferManager>()` is a zero-cost compile-time contract test — cheaper and stronger than any runtime assertion for the `Send + Sync` guarantee Epic 5 relies on when it puts the manager in `tauri::State`.

### Previous Story Intelligence (from Story 3.1)

- **3.1 established the crate shape you extend:** `lib.rs` = decls + re-exports; module doc headers carry LD/FR traces; `#[non_exhaustive]` on the error enum for forward growth (you add no variant, so no concern here). The `lib.rs` crate doc already contains the sentence "Story 3.2 adds the Dirty Buffer module alongside" — rewrite it to present tense, don't just append.
- **3.1 review lessons that apply:** (a) the reviewer flagged unrequested surface and residual-risk decisions — keep the API to the four mandated methods (Dev Note 4); (b) disclose any signature variance in Completion Notes (the `Option<&str>` vs `Option<String>` fork is the only candidate — the plain-struct choice avoids it). (c) 3.1's supply-chain churn came from a *new dep edge*; this story has none, so `deny.toml`/audit ledger stay untouched.
- **LEAF rule (Story 1.18/2.8/3.1):** `orgsidian-vault` is a LEAF crate; `deny.toml` allows only `orgsidian-core` as its consumer. You add no dependency edges in either direction, so the graph is unchanged.

### Git Intelligence Summary

Working tree at story-creation: on `story/3.1-atomic-write-av-retry`, clean, HEAD `a8d5a13` (3.1 code-review hardening). Recent flow: story-branch PRs merged via `gh pr merge --admin` (branch protection needs 1 review, unsatisfiable solo); `chore:` for cross-branch merges (commitlint forbids `merge:`); `status:in-review` label during CR. Story 3.1 landed the whole vault atomic-write subsystem in `a31cddb..a8d5a13`.

### Latest Technical Information

- No external crates involved — std-only (`HashMap`, `Arc`, `RwLock`, `Path`, `PathBuf`). No version research applies. `std::sync::RwLock` is the correct primitive (no `parking_lot` in-tree, and none warranted for this read-heavy-but-tiny map).
- Rust edition/toolchain: workspace `edition` + latest-stable toolchain with `rustfmt` + `clippy` enforced (architecture.md:172, rust-toolchain.toml).

### References

- Epic AC source: `_bmad-output/planning-artifacts/epics.md` §Epic 3 → Story 3.2 (lines 907-919)
- LD-7 (Single Writer Rule + Dirty Buffer): `architecture.md:69`; Single Writer / Dirty Buffer subsystem listing: `architecture.md:104`; crate layout: `architecture.md:205-220`, `architecture.md:916`
- External-write data flow (`dirty_buffer checks state`): `architecture.md:1127-1140`
- FR-16 traceability (`orgsidian-vault/src/dirty_buffer.rs`): `architecture.md:1059`; NFR-16 (Single Writer reliability): `epics.md:124`
- Downstream consumers: Story 5.3 (`ConflictState` rich struct, `epics.md:1277`+), Story 5.5 (`BlockWithWarning`, `epics.md:1301`+), Story 9.3 (`resolveMerge` → atomic-write + clear Dirty, `epics.md:1873`+), Story 11.8 (Refile both-clean precondition, `epics.md:2155`+)
- Crate to extend: `crates/orgsidian-vault/src/lib.rs` (re-export site), `src/atomic.rs`/`src/error.rs` (conventions to mirror; do not edit)
- Testing convention (colocated unit tests): `architecture.md:724`

### Project Context Reference

No `project-context.md` exists in the repo at story-creation (checked 2026-07-22). Architecture + this story file are the authoritative context.

## Dev Agent Record

### Agent Model Used

claude-opus-5[1m] (Claude Opus 5, 1M context) — `bmad-dev-story` workflow.

### Debug Log References

Red-green-refactor, single pass:

- **RED** — wrote the full colocated test module (7 tests) plus the bare `DirtyBufferManager` struct with no `impl` block, then `cargo test -p orgsidian-vault`: 13 compile errors, all `E0599 no method/associated function found` (`new` ×6, `is_dirty` ×3, `get_buffer` ×2, `mark_dirty` ×1, `mark_clean` ×1). Confirms the tests genuinely exercise the AC2 surface rather than passing vacuously.
- **GREEN** — added the `impl` block (`new` + the four mandated methods). All 7 `dirty_buffer::tests` pass; the 4 pre-existing `atomic::tests` unaffected (11 lib tests total).
- **REFACTOR** — `cargo fmt` reformatted one multi-line `assert!` in `distinct_paths_tracked_independently`; no logic change. `cargo clippy --workspace --all-targets -- -D warnings` exit 0 with no lint on the new module (no `new_without_default` warning — `Default` is derived *and* `new()` exists, which is the idiom clippy wants).

### Completion Notes List

**What shipped.** One new module, `crates/orgsidian-vault/src/dirty_buffer.rs`: `DirtyBufferManager` (a `HashMap<PathBuf, String>` newtype-ish registry) with exactly the four epic-mandated methods, plus `new()` and a `SharedDirtyBuffers = Arc<RwLock<DirtyBufferManager>>` alias. `lib.rs` gained the module decl, the `DirtyBufferManager` re-export, and a present-tense rewrite of the forward-looking Story-3.2 doc sentence. No other pre-existing file touched.

**Epic signature honored — no variance to disclose.** `get_buffer` returns `Option<&str>` literally, per AC2/AC3's recommended default. The plain-struct + caller-owned-lock shape (Dev Note 2) was implemented as pre-decided; the interior-locking fallback and its `Option<String>` clone cost were **not** taken, so the AC3 "permitted deviation" disclosure does not apply.

**AC3 `RwLock` choice.** `std::sync::RwLock`, not `Mutex`, and not `parking_lot` (no such dep in-tree, none warranted for a small read-skewed map). Rationale is in the module doc: the Epic 5 watcher hammers `is_dirty` on every debounced FS event, mutations only at keystroke-batch/save boundaries.

**API surface held to four methods.** No `all_dirty()`, no event emission, no persistence, no `#[tauri::command]`, no `Vault` host struct — per Dev Note 4 and the Story-3.1 review lesson about unrequested surface.

**Judgment call worth a reviewer's eye — `.unwrap()` inside the module-doc example.** AC3 requires the module doc to state the shared handle explicitly; I made that an *executable* doctest (```` ``` ```` fenced, runs under `cargo test --doc`, passing) rather than an un-compiled snippet, so the documented pattern cannot rot. It contains two `RwLock` guard `.unwrap()` calls at lines 31-32 — idiomatic for a Rust doc example, and doctests are test code (Dev Note 5: "Tests may `.unwrap()` the `RwLock` guards freely"). Verified there is **no CI grep gate** on `unwrap`/`expect` in `.github/workflows/` — the Story-2.8 no-panic rule is review-enforced convention — and confirmed zero `unwrap`/`expect`/`panic!`/`todo!` in the actual non-test code above the `#[cfg(test)]` boundary. Flagging it because these are the first `//!`-comment `unwrap()`s in the tree (no precedent to point at). Reject and downgrade to a non-running ```` ```no_run ````/```` ```text ```` block if the convention is meant to read literally.

**Concurrency test is deterministic.** `shared_handle_is_race_free` spawns 8 threads × 50 interleaved `mark_dirty`/`is_dirty` cycles through the shared handle; half the threads end by saving. Assertions run *after* join and only on final state (even-numbered threads clean + `get_buffer == None`; odd-numbered dirty with last-write-wins content) — never on timing or interleaving order, per the Story-3.1 lesson.

**Gates (all green, verified locally).**

- `cargo test --workspace` — 0 failures across all crates. Vault: 11 lib tests (7 new), 5 `tests/atomic.rs`, 6 `tests/orphan_cleanup.rs`, 1 doctest (new).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo fmt --check` — clean.
- `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok. (Pre-existing unrelated note: the `RUSTSEC-2026-0195` ignore entry in `deny.toml:65` matches no crate — untouched by this story.)
- `cargo audit` — 18 allowed warnings, the unchanged pre-existing baseline.
- **Dependency delta exactly zero** — `git diff HEAD -- crates/orgsidian-vault/Cargo.toml Cargo.lock` is 0 lines. std-only: `HashMap`, `Arc`, `RwLock`, `Path`, `PathBuf`.

**Sentinel discipline verified byte-exact.** `git diff --name-only HEAD` over `tests/anchor.rs`, `src/atomic.rs`, `src/error.rs`, `crates/orgsidian-vault/Cargo.toml`, root `Cargo.toml`, `deny.toml`, `.github/`, `crates/orgsidian-core/tests/failure_modes.rs`, `docs/failure-modes/` returned **empty**. LD-41 harness still reports `2 passed; 8 ignored` — `EXPECTED_REMAINING_PLACEHOLDERS` stays at 8, unchanged.

**Deferred work recorded.** One MED item appended to `deferred-work.md` (§story-3.2 implementation): the raw-`PathBuf` keying means two spellings of the same file register as independent entries, so `is_dirty` can answer `false` for a genuinely-dirty file and FR-16 would route an external write to auto-reload and clobber the buffer. Ownership assigned to the Epic 5/6 Vault-open story, which must canonicalize once at open. This is the pre-decided AC2/Dev-Note-3 tradeoff, not a defect — but it is a real correctness obligation on the consumer, so it is written down rather than left implicit.

### File List

- `crates/orgsidian-vault/src/dirty_buffer.rs` — **NEW**: `DirtyBufferManager` + `SharedDirtyBuffers` alias + colocated `#[cfg(test)] mod tests` (7 tests).
- `crates/orgsidian-vault/src/lib.rs` — MODIFIED: `pub mod dirty_buffer;`, `pub use dirty_buffer::DirtyBufferManager;`, crate-doc sentence rewritten to present tense.
- `_bmad-output/implementation-artifacts/deferred-work.md` — MODIFIED: appended §"Deferred from: story-3.2 implementation (2026-08-01)".
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — MODIFIED: `3-2-…` ready-for-dev → in-progress → review; `last_updated` bumped.
- `_bmad-output/implementation-artifacts/3-2-scaffold-dirty-buffer-manager.md` — MODIFIED: task checkboxes, Dev Agent Record, File List, Change Log, Status.

## Change Log

- 2026-07-22 — Story created (create-story workflow); status ready-for-dev; github issue #26 pre-existing via Story 1.16 sync.
- 2026-08-01 — Implemented (dev-story workflow): `dirty_buffer` module + 7 colocated tests + `lib.rs` re-export. All 5 ACs satisfied, zero dependency delta, all gates green, sentinels byte-untouched. Status → review; issue #26 label → `status:in-review`.
