# Epic 5 Context: External-Edits Co-existence (Safe Fallback)

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Connect the filesystem watcher to the Dirty Buffer to enforce the v0.1 form of the Single Writer Rule (LD-7). When an external tool writes a file whose buffer is clean, Orgsidian auto-reloads it and incrementally re-indexes it. When the file has unsaved (Dirty Buffer) changes, Orgsidian does not silently overwrite and does not yet show a Merge Dialog (that lands in Epic 9); instead it blocks the save and surfaces a calm conflict warning. This is FR-16's v0.1 fallback only. Crucially, the machinery is built day-1 in its full shape — a `ConflictStrategy` pattern with `BlockWithWarning` as one variant, and a rich `ConflictState` struct rather than a boolean — so Epic 9 swaps in the three-pane merge strategy without rewriting the watcher state machine.

## Stories

- Story 5.1: notify-rs filesystem watcher with debounce
- Story 5.2: Golden-trace fixtures from vim / VS Code / Emacs saves
- Story 5.3: ConflictState struct + ConflictStrategy pattern
- Story 5.4: Clean-buffer auto-reload + re-index
- Story 5.5: Dirty-Buffer block-save fallback with conflict warning UI

## Requirements & Constraints

- FR-16 (v0.1 fallback only): detect external writes; clean buffer → auto-reload + re-index; dirty buffer → block save with warning. Full three-pane Merge Dialog is explicitly out of scope (Epic 9).
- NFR-16 (Single Writer Rule): while Orgsidian holds a Dirty Buffer for a file, it is the sole writer; external writes to dirty files must never silently overwrite. Race-condition surface must be tested deterministically (injected clock + synthetic external-write events).
- Detection NFR: external writes detected within 5 seconds on macOS, Linux, and Windows.
- Never-silent-overwrite is a core trust contract; the app must never lose or clobber unsaved user work.
- Network mounts and case-folding filesystems are documented as v0.1 **unsupported** configurations (record in `docs/architecture/resilience.md`). Do not attempt to support them here.
- Depends on Epic 3 (atomic-write subsystem + Dirty Buffer manager) being closed. Story 5.3 also depends on Epic 3's content-hashing (Story 3.2).

## Technical Decisions

- **Watcher (LD-9):** `notify-rs` wrapped in `orgsidian-watcher`. A watcher abstraction layer (`WatcherFacade`) in core allows deterministic fakes for unit tests; integration tests replay golden traces from real editors rather than touching a real filesystem.
- **Debounce (OD-3):** atomic-save sequences from vim/VS Code/Emacs emit 3-12 filesystem events per save (delete + create + modify). A 250ms debounce window coalesces each burst into exactly one `FileChanged { path }` event. Golden-trace fixtures (per-editor JSON) drive calibration and are asserted to yield exactly one event per save.
- **Writer-ID suppress tokens:** Orgsidian's own atomic writes trip the watcher. The save cycle emits a writer-ID/suppress token so the watcher ignores self-writes and does not spuriously enter conflict/reload flows. This mechanism already exists in the save-cycle data flow; Epic 5 relies on it so external-vs-self writes are distinguishable.
- **Conflict model (Party Mode P0 — Winston + Murat):** `ConflictState` is a rich struct (`ancestor_hash`, `external_content`, `buffer_content`, `file_path`), never a boolean. `ConflictStrategy` is a pattern with variants `BlockWithWarning | ThreePaneMergeDialog`, each implementing a `ResolveConflict` trait returning a `Resolution` (`Block` / `WriteMerged` / `Cancel`). The watcher state machine consumes `&dyn ResolveConflict` with the active strategy injected at startup. Tests parameterize one suite over both strategies. Goal: Epic 9 changes only the injected strategy, not the state machine.
- **Clean-buffer reload:** on external write with `is_dirty(path) == false`, refresh the in-memory buffer from disk, incrementally re-sync the SQLite index for that file (`orgsidian-index::sync::incremental`), and preserve cursor position if its source line is unchanged (otherwise reset to top).
- **Dirty-buffer block:** when active strategy is `BlockWithWarning` and an external write hits a Dirty Buffer, emit a `ConflictDetected { path, state }` Tauri event; `saveFile` returns `Err(OrgError::Vault(VaultError::ExternalConflict { path }))`. A "Discard external changes" action lets the subsequent save proceed (still atomic-write).
- **Async:** watcher/indexer paths use `tokio::fs`; CPU-bound parsing via `spawn_blocking` (LD-16).

## UX & Interaction Patterns

- Clean-buffer auto-reload is fully automatic and silent — no dialog. A non-modal status note ("file reloaded from disk", ~3s) is announced via `aria-live="polite"`. Never use `assertive` for this.
- The dirty-buffer conflict is a calm, inline banner in the editor surface, not a modal (the modal Merge Dialog is Epic 9). Copy is plain and direct, no exclamation marks and no warning colors — this is crisis UX dialed *down*, designed to make the user feel held. Banner offers "Discard external changes" and "View file in default editor".

## Cross-Story Dependencies

- 5.2 depends on 5.1 (watcher); its recorded traces are reused by Epic 9 Merge Dialog tests.
- 5.3 depends on 5.1 + 5.2 + Epic 3 Story 3.2 (content hashing).
- 5.4 depends on 5.1 + 5.2 + 5.3.
- 5.5 depends on 5.3 + 5.4.
