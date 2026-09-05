use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use orgsidian_core::{
    ConflictNotice, IndexHandle, OrgError, Result as OrgResult, SharedDirtyBuffers,
    SharedPendingConflicts,
};
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, collect_events, Builder, ErrorHandlingMode, Event};

mod editor_prefs;
use editor_prefs::EditorMode;

#[tauri::command]
#[specta::specta]
fn ping() -> OrgResult<String> {
    Ok("pong".to_string())
}

/// Story 3.6 (AC5): the app's FIRST specta event. Emitted from the initial
/// scan's progress callback every LD-42 checkpoint. Fields are single words, so
/// the wire shape is already `{ current, total, errors }`; project-wide
/// camelCase is the specta builder's job — NO per-struct `#[serde(rename_all)]`
/// (architecture.md:868 forbidden anti-pattern).
#[derive(Debug, Clone, serde::Serialize, specta::Type, Event)]
pub struct IndexProgress {
    /// Files processed so far (indexed + skipped + quarantined).
    pub current: u32,
    /// Total `.org` files discovered in the vault.
    pub total: u32,
    /// Files quarantined so far (LD-41 unparseable/unreadable).
    pub errors: u32,
}

/// Story 3.6 (AC5): managed state holding the active vault's index handle (the
/// LD-14 writer + reader pool) and the current scan's cancel flag, plus a
/// designation lock that serializes overlapping [`designate_vault`] calls.
///
/// `index`/`cancel` are `std::sync::Mutex` whose guards are never held across an
/// `.await` (trivial take/store/read). `designating` is an ASYNC mutex, held for
/// the whole of one designation: Tauri runs async commands in parallel and
/// `AppState` is process-wide, so without it two overlapping designations could
/// open two writers on one WAL file and cross-wire the cancel flag (the UI's
/// per-window button-disable is not a backend serialization guarantee).
#[derive(Default)]
pub struct AppState {
    designating: tauri::async_runtime::Mutex<()>,
    index: Mutex<Option<IndexHandle>>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    /// Story 5.5 (LD-7 / FR-16): which open files hold unsaved edits. The
    /// watcher's DIRTY branch reads this to route an external write to the
    /// block-save fallback; a successful [`save_file`] marks the file clean.
    /// `Arc<RwLock<_>>`, so shared by clone with the (deferred) watcher loop.
    dirty_buffers: SharedDirtyBuffers,
    /// Story 5.5 (FR-16 / NFR-16): files whose save is currently BLOCKED by an
    /// unresolved external conflict. [`save_file`] consults it (blocked → refuse
    /// with a `VaultError::ExternalConflict`-backed `OrgError::Vault`); the
    /// "Discard external changes" action ([`discard_external_changes`]) clears it.
    pending_conflicts: SharedPendingConflicts,
}

/// Lock a managed-state mutex, tolerating poisoning. The critical sections are
/// trivial assignments/reads never held across an `.await`, so a poisoned lock
/// (a panic while held, which cannot happen here) still yields usable state
/// rather than propagating a panic out of a `#[tauri::command]` (AC5: no
/// `unwrap`/`expect`/`panic!` in command code).
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    /// The active Vault's canonical root, or `None` when no Vault is designated.
    ///
    /// Clones the path out from under the (never-held-across-`.await`) index
    /// lock so the caller can `.await` store I/O without holding the guard.
    fn current_vault_root(&self) -> Option<PathBuf> {
        lock(&self.index)
            .as_ref()
            .map(|handle| handle.vault_root().to_path_buf())
    }
}

/// The `OrgError::Vault` returned when a Vault-scoped command (editor-mode
/// prefs, LD-40; the Story 6.3 Agenda query) runs before any Vault is active
/// — there is nowhere to persist to / nothing indexed to read yet.
fn no_active_vault() -> OrgError {
    OrgError::Vault {
        reason: "no active vault; designate a vault first".to_string(),
    }
}

/// Designate `path` as the active vault (FR-15): open/guard the derived index,
/// then run the initial scan, emitting [`IndexProgress`] every checkpoint. The
/// handle is kept in managed state for later reads (Epics 7/8).
#[tauri::command]
#[specta::specta]
async fn designate_vault(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> OrgResult<()> {
    designate_vault_impl(&path, &app, &state).await
}

/// The shared designate-then-scan body behind both [`designate_vault`] and
/// Story 6.2's [`generate_starter_vault`] (which runs the Story 6.1 generator
/// FIRST, then designates the now-populated folder exactly the same way a
/// hand-picked existing folder is designated). Factored out so the two
/// commands can never drift on the serialization / previous-handle-shutdown /
/// cancel-flag discipline documented below.
async fn designate_vault_impl(
    path: &str,
    app: &tauri::AppHandle,
    state: &AppState,
) -> OrgResult<()> {
    // Serialize concurrent/overlapping designations: a second invocation waits
    // here until this one has stored (or failed to store) its handle, so the
    // shutdown-of-previous below always sees the real predecessor rather than a
    // still-empty slot mid-scan (which would open a second writer on one WAL).
    let _designating = state.designating.lock().await;

    // Close any previously-designated vault's index BEFORE opening the next one
    // (drain the writer + drop the pool via the async `shutdown`, never a bare
    // drop) so two writers never share one WAL file when the same vault is
    // re-designated, and no connection outlives its handle.
    let previous = lock(&state.index).take();
    if let Some(previous) = previous {
        previous.shutdown().await;
    }

    // A fresh cancel flag for THIS designation, published BEFORE the open+scan
    // so a Cancel click during vault-open (canonicalize/spawn/migrate) is
    // honored by the scan's first checkpoint check rather than lost.
    let cancel = Arc::new(AtomicBool::new(false));
    *lock(&state.cancel) = Some(Arc::clone(&cancel));

    // Run open + scan, then clear the now-finished scan's flag on EVERY exit
    // path (success or error) — the `?` below must not leave a stale flag.
    let outcome = designate_and_scan(path, app, &cancel).await;
    *lock(&state.cancel) = None;

    let handle = outcome?;
    // Retain the handle for later reads.
    *lock(&state.index) = Some(handle);
    Ok(())
}

/// Story 6.2 (FR-18): which built-in Starter Vault the picker's primary cards
/// generate. Mirrors [`orgsidian_core::StarterVaultKind`] but is redeclared
/// here — the core crate carries no `specta` dependency, so (as with
/// [`PlanningKind`] above) the shell owns the wire-typed twin at the IPC
/// boundary. Wire shape `"personalGtd" | "student"`. No `Freelancer` variant:
/// Story 6.1 shipped only Personal GTD + Student (Freelancer needs Story 8.7's
/// BacklinksPanel); the picker's Freelancer card is rendered disabled and never
/// reaches this command (see `StarterVaultPicker.tsx`).
#[derive(Debug, Clone, Copy, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum StarterVaultKind {
    /// Personal GTD (Getting Things Done).
    PersonalGtd,
    /// Student (coursework-shaped).
    Student,
}

impl StarterVaultKind {
    /// Map the wire enum to its [`orgsidian_core::StarterVaultKind`] twin.
    /// Named + unit-tested (rather than an inline `match` in the command body)
    /// so a swapped arm — a Student pick generating a Personal GTD Vault, or
    /// vice-versa — fails a test instead of silently shipping the wrong
    /// Starter Vault; the core generator's own tests use the core enum
    /// directly and never cross this IPC-boundary translation.
    fn to_core(self) -> orgsidian_core::StarterVaultKind {
        match self {
            StarterVaultKind::PersonalGtd => orgsidian_core::StarterVaultKind::PersonalGtd,
            StarterVaultKind::Student => orgsidian_core::StarterVaultKind::Student,
        }
    }
}

/// Story 6.2 (FR-18): generate the chosen built-in Starter Vault's `.org`
/// files into `path` (Story 6.1's [`orgsidian_core::generate_starter_vault`]),
/// then designate + scan the freshly-populated folder — the picker's primary
/// cards are a generate-then-designate compound action from the caller's
/// perspective, sharing [`designate_vault_impl`] with [`designate_vault`] so
/// the newly-generated Vault immediately shows non-empty Today/Week Agenda
/// content once Stories 6.3/6.4 land.
///
/// `today` is the frontend's local calendar day (`YYYY-MM-DD`, the same
/// convention [`set_scheduled`] uses) so every generated SCHEDULED/DEADLINE
/// timestamp anchors to the user's actual "today" rather than a server-side
/// clock/timezone guess.
///
/// Hardening (NFR-16 spirit "never silently overwrite user data"): this is a
/// first-launch onboarding action, so `path` may be a folder the user picked
/// without realizing it already holds their own `.org` files (e.g. their
/// Documents folder) — [`orgsidian_core::generate_starter_vault`] would
/// happily overwrite same-named files. [`ensure_target_has_no_org_files`]
/// refuses BEFORE generating when that's the case; it never prompts-and-
/// continues.
#[tauri::command]
#[specta::specta]
async fn generate_starter_vault(
    kind: StarterVaultKind,
    path: String,
    today: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> OrgResult<()> {
    let today = resolve_today(&today)?;
    ensure_target_has_no_org_files(&path).await?;
    orgsidian_core::generate_starter_vault(kind.to_core(), Path::new(&path), today)?;
    designate_vault_impl(&path, &app, &state).await
}

/// Story 6.2 hardening (NFR-16 spirit): refuse [`generate_starter_vault`] when
/// `path` already holds a top-level `.org` file — generating into it would
/// silently overwrite same-named starter files, a real risk on first launch
/// when a user might point the picker at a populated folder (their Documents,
/// say) rather than an empty one.
///
/// Deliberately shallow: a top-level `read_dir`, not a recursive walk — this
/// is a fast pre-flight safety check, not a Vault contents audit (the real
/// recursive scan is [`orgsidian_vault::scan_org_files`], run later by
/// [`designate_vault_impl`]). A `path` that doesn't exist yet (the common
/// case — the generator's own `create_dir_all` creates it) is empty by
/// definition and passes.
///
/// # Errors
///
/// [`OrgError::Vault`] (worded to steer the user to an empty folder, or to
/// "Use my own folder" for an existing populated Vault) when a top-level
/// `.org` file is found; [`OrgError::Io`] if `path` exists but cannot be
/// listed (e.g. permission denied).
async fn ensure_target_has_no_org_files(path: &str) -> OrgResult<()> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        // Doesn't exist yet — nothing to check; the generator creates it.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(OrgError::Io {
                reason: format!("failed to read target folder {path}: {err}"),
            })
        }
    };

    while let Some(entry) = entries.next_entry().await.map_err(|err| OrgError::Io {
        reason: format!("failed to read target folder {path}: {err}"),
    })? {
        let is_org_file = entry
            .file_type()
            .await
            .is_ok_and(|file_type| file_type.is_file())
            && entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("org"));

        if is_org_file {
            return Err(OrgError::Vault {
                reason: format!(
                    "{path} already contains .org files; pick an empty folder for a \
                     Starter Vault, or use \"Use my own folder\" to designate this \
                     folder as your existing Vault instead"
                ),
            });
        }
    }
    Ok(())
}

/// Story 6.2 (FR-18): has a Vault ever been configured? The `/today` route's
/// onboarding gate calls this on mount to decide between rendering the
/// [`StarterVaultPicker`] onboarding surface and the app's normal Vault-scoped
/// content. `true` when a Vault is already designated THIS session, or (LD-40)
/// `GlobalSettings.recent_vaults` is non-empty from a previous launch — the
/// picker is a first-launch surface, not a re-ask on every relaunch.
///
/// [`StarterVaultPicker`]: (frontend) shell-ui/src/components/onboarding/StarterVaultPicker.tsx
#[tauri::command]
#[specta::specta]
fn has_configured_vault(state: tauri::State<'_, AppState>) -> OrgResult<bool> {
    if state.current_vault_root().is_some() {
        return Ok(true);
    }
    let global =
        orgsidian_core::settings::read_global_settings().map_err(|source| OrgError::Io {
            reason: format!("failed to read global settings: {source}"),
        })?;
    Ok(!global.recent_vaults.is_empty())
}

/// Open + guard the derived index for `path` and run the initial scan, emitting
/// [`IndexProgress`] every checkpoint. Returns the handle on success; the caller
/// owns cancel-flag lifecycle and managed-state storage.
async fn designate_and_scan(
    path: &str,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> OrgResult<IndexHandle> {
    let handle = orgsidian_core::designate_vault(Path::new(path)).await?;

    orgsidian_core::scan_vault(&handle, cancel, |progress| {
        // Emitting is best-effort: a failed emit (no listener / window gone)
        // must not abort the scan.
        let _ = IndexProgress {
            current: progress.current,
            total: progress.total,
            errors: progress.errors,
        }
        .emit(app);
    })
    .await?;

    Ok(handle)
}

/// Story 6.3 code review follow-up: resolve the `open_file`/`openFile` incoming
/// `path` against the active Vault root.
///
/// Agenda rows (Story 6.3's `agenda_today`) carry `files.path` — the
/// vault-relative, `/`-normalized `rel_path` — NOT an absolute path, so a
/// packaged app (cwd != vault root) failed to open the clicked file. A
/// relative `path` is joined onto `vault_root`; an absolute `path` is returned
/// unchanged (back-compat with any caller that already has a full path). A
/// relative `path` with no Vault designated has nowhere to resolve against, so
/// it errors with the same [`no_active_vault`] `OrgError::Vault` the other
/// Vault-scoped commands (`agenda_today`, editor-mode prefs) use.
///
/// TODO(vault-scoping): this only joins the path — it does NOT canonicalize or
/// reject `..`/symlink vault-escape. That hardening is deferred to the
/// dedicated vault-root-scoping story (see `deferred-work.md`); this fix's
/// scope is click-to-open working at runtime, not scoping enforcement.
fn resolve_open_path(path: &str, vault_root: Option<&Path>) -> OrgResult<PathBuf> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Ok(candidate.to_path_buf());
    }
    let vault_root = vault_root.ok_or_else(no_active_vault)?;
    Ok(vault_root.join(candidate))
}

/// Story 4.1: read a file's full UTF-8 source text (a `.org` file in normal
/// use) for the CodeMirror 6 editor host. A relative `path` (the vault-relative
/// `rel_path` agenda rows carry) is resolved against the active Vault root by
/// [`resolve_open_path`] before reading; an absolute `path` is read as-is. This
/// command does no extension check nor Vault-escape scoping yet; that
/// hardening is deferred (see `resolve_open_path`'s `TODO(vault-scoping)` and
/// `deferred-work.md`). Both IO failures (missing path, permission denied) and
/// invalid-UTF-8 content collapse to [`OrgError::Io`]: `read_to_string` already
/// surfaces non-UTF-8 bytes as an `InvalidData` IO error, so one mapping covers
/// the whole matrix. The returned buffer is byte-faithful — CM6 owns it; it is
/// never duplicated into state nor persisted apart from the `.org` file.
#[tauri::command]
#[specta::specta]
async fn open_file(path: String, state: tauri::State<'_, AppState>) -> OrgResult<String> {
    open_file_at(&path, state.current_vault_root().as_deref()).await
}

/// The testable body of [`open_file`]: resolve `path` against `vault_root`
/// (see [`resolve_open_path`]) then read it. Split out so unit tests can drive
/// the resolution + read without needing a live `tauri::State`.
async fn open_file_at(path: &str, vault_root: Option<&Path>) -> OrgResult<String> {
    let resolved = resolve_open_path(path, vault_root)?;
    tokio::fs::read_to_string(&resolved)
        .await
        .map_err(|err| OrgError::Io {
            reason: format!("failed to read {}: {err}", resolved.display()),
        })
}

/// Story 5.5 (FR-16 / NFR-16): the redaction-safe `state` carried on the
/// [`ConflictDetected`] event. The banner renders only the `path`; these fields
/// are diagnostic metadata projected from the core [`ConflictNotice`] — content
/// byte *lengths* and the ancestor hash, NEVER the user's note text (the
/// conflict types' redaction contract extended across the IPC boundary).
///
/// These are the first multi-word event fields in the app, so — like `OrgError`
/// and for the same reason (the architecture's project-wide specta rename is not
/// available in the pinned `tauri-specta =2.0.0-rc.25`, and `#[specta(rename_all)]`
/// is rejected) — `#[serde(rename_all = "camelCase")]` is the working way to
/// honor the mandated camelCase IPC wire (`{ ancestorHash, externalLen,
/// bufferLen }`). specta-serde's Format symmetry keeps the generated TS in step.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSummary {
    /// Hex SHA-256 of the ancestor content (a digest, safe to surface).
    pub ancestor_hash: String,
    /// Byte length of the external (on-disk) content — never the content.
    pub external_len: u32,
    /// Byte length of the unsaved buffer content — never the content.
    pub buffer_len: u32,
}

/// Story 5.5 (FR-16 v0.1 fallback): emitted when an external write lands on a
/// file with a Dirty Buffer and the active `BlockWithWarning` strategy blocks
/// the save. The frontend renders the calm, non-modal conflict banner in the
/// editor surface and offers "Discard external changes" / "View file in default
/// editor". Wire name `conflict-detected`, accessor `events.conflictDetected`.
#[derive(Debug, Clone, serde::Serialize, specta::Type, Event)]
pub struct ConflictDetected {
    /// The conflicted file (verbatim, as the Dirty Buffer keys it) — the id the
    /// banner shows and the two actions round-trip back to the block/discard
    /// commands.
    pub path: String,
    /// Redaction-safe conflict metadata (no note content).
    pub state: ConflictSummary,
}

impl ConflictDetected {
    /// Project a core [`ConflictNotice`] into the wire event — the ONLY crossing
    /// of the conflict data over IPC, and it carries no note content.
    fn from_notice(notice: &ConflictNotice) -> Self {
        ConflictDetected {
            path: notice.path().to_string_lossy().into_owned(),
            state: ConflictSummary {
                ancestor_hash: notice.ancestor_hash().to_string(),
                // A `.org` buffer never approaches 4 GiB; `as` is safe here.
                external_len: notice.external_len() as u32,
                buffer_len: notice.buffer_len() as u32,
            },
        }
    }
}

/// Story 5.5 (FR-16): push a [`ConflictDetected`] to the window. Best-effort —
/// a failed emit (no listener / window gone) must not abort the reconcile, the
/// same discipline [`IndexProgress`] emission follows.
///
/// Called by the watcher event-consumption loop's DIRTY branch after
/// [`orgsidian_core::resolve_dirty_conflict`] records the block and returns the
/// notice. That loop (draining the `WatcherFacade`'s `Receiver<FileChanged>` per
/// designated vault) is the shared Epic-5 seam still deferred from Story 5.4;
/// this helper is the emit half it calls, kept here so the payload projection is
/// reviewed and unit-tested now (see `tests`).
pub fn emit_conflict_detected(app: &tauri::AppHandle, notice: &ConflictNotice) {
    let _ = ConflictDetected::from_notice(notice).emit(app);
}

/// Story 5.5 (FR-16 / NFR-16 Single Writer Rule): save `content` to `path`, or
/// REFUSE when an external write conflict is unresolved.
///
/// The v0.1 save gate: if `path` has a pending external conflict, the save is
/// blocked and returns `OrgError::Vault` (from `VaultError::ExternalConflict`) —
/// Orgsidian never silently overwrites unsaved work. Otherwise the buffer is
/// written via the Story 3.1 atomic write (temp-file + rename) and marked clean.
/// The `path` is taken verbatim (same keying as `open_file` and the Dirty
/// Buffer); Vault-root scoping is deferred with `open_file`'s (see
/// `deferred-work.md`).
#[tauri::command]
#[specta::specta]
async fn save_file(
    path: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> OrgResult<()> {
    orgsidian_core::save_buffer(
        &state.pending_conflicts,
        &state.dirty_buffers,
        Path::new(&path),
        &content,
    )
    .await
}

/// Story 5.5 (FR-16): "Discard external changes" — clear the pending external
/// conflict on `path` so the next [`save_file`] overwrites the external write
/// via the normal atomic path. Idempotent; a no-op when `path` is not blocked.
#[tauri::command]
#[specta::specta]
fn discard_external_changes(path: String, state: tauri::State<'_, AppState>) -> OrgResult<()> {
    orgsidian_core::discard_external_changes(&state.pending_conflicts, Path::new(&path));
    Ok(())
}

/// Story 5.5 (FR-16): "View file in default editor" — open the conflicted file
/// in the OS default application via `tauri-plugin-opener`, so the user can
/// inspect the external write before deciding to discard. Read-only from
/// Orgsidian's side; it never touches the Dirty Buffer or the block state.
#[tauri::command]
#[specta::specta]
fn open_in_default_editor(path: String, app: tauri::AppHandle) -> OrgResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|err| OrgError::Io {
            reason: format!("failed to open {path} in the default editor: {err}"),
        })
}

/// Story 4.2 (FR-3): persist the per-file Editor Mode choice for `file_path`
/// via `tauri-plugin-store` at `<Vault>/.orgsidian/editor-prefs.json` (LD-40).
/// Errors with `OrgError::Vault` when no Vault is active (nowhere to store).
#[tauri::command]
#[specta::specta]
async fn set_editor_mode(
    mode: EditorMode,
    file_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> OrgResult<()> {
    let vault_root = state.current_vault_root().ok_or_else(no_active_vault)?;
    editor_prefs::persist_mode(&app, &vault_root, &file_path, mode)
}

/// Story 4.2 (FR-3): read the persisted Editor Mode for `file_path`, or `None`
/// when the file has no stored choice. Errors with `OrgError::Vault` when no
/// Vault is active.
#[tauri::command]
#[specta::specta]
async fn get_editor_mode(
    file_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> OrgResult<Option<EditorMode>> {
    let vault_root = state.current_vault_root().ok_or_else(no_active_vault)?;
    editor_prefs::read_mode(&app, &vault_root, &file_path)
}

/// Story 4.8 (FR-9): which planning keyword a `set_scheduled` write targets.
/// Wire shape `"scheduled" | "deadline"` (camelCase per the project convention),
/// mapped to the parser's `PlanningKind` inside the command.
#[derive(Debug, Clone, Copy, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PlanningKind {
    /// `SCHEDULED:` planning entry.
    Scheduled,
    /// `DEADLINE:` planning entry.
    Deadline,
}

/// Story 4.8 (FR-9): the date/time the picker (or a typed shortcut) commits.
/// `date` is either a literal `YYYY-MM-DD` or a relative shortcut (`today`,
/// `+1d`, `+1w`, …) resolved against `today`; `time` is an optional `HH:MM`.
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
pub struct TimestampInput {
    /// Literal `YYYY-MM-DD` or a relative shortcut token.
    pub date: String,
    /// Optional clock time `HH:MM`.
    pub time: Option<String>,
}

/// Story 4.8 (FR-9): a byte-faithful buffer edit — replace `from..to` with
/// `insert` — the frontend applies as ONE CM6 transaction (LD-26 shared
/// surface). Offsets are `u32` for the JS wire; a `.org` buffer never
/// approaches 4 GiB.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct PlanningEdit {
    /// Byte offset where the replaced region begins.
    pub from: u32,
    /// Byte offset where the replaced region ends.
    pub to: u32,
    /// Replacement text.
    pub insert: String,
}

/// Map a date/time parse failure (malformed `today`/literal/`HH:MM`) to a
/// diagnostic `OrgError`. Unreachable from the picker (it sends valid ISO); a
/// guard against a buggy or hand-crafted caller.
fn bad_timestamp(reason: String) -> OrgError {
    OrgError::Parse {
        file: "<editor-buffer>".to_string(),
        reason,
    }
}

/// Story 6.2: parse the frontend-supplied local `today` (`YYYY-MM-DD`) into a
/// `NaiveDate` for [`generate_starter_vault`] — same literal-date parse +
/// error-mapping convention [`resolve_planned`] uses for `set_scheduled`.
fn resolve_today(today: &str) -> OrgResult<orgsidian_core::parser::chrono::NaiveDate> {
    orgsidian_core::parser::chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .map_err(|err| bad_timestamp(format!("invalid `today` {today:?}: {err}")))
}

/// Resolve a [`TimestampInput`] into the parser's concrete `PlannedStamp`,
/// treating `date` as a relative shortcut first (pure-Rust resolver) and
/// falling back to a literal `YYYY-MM-DD`. `today` (supplied by the frontend as
/// the user's local date) anchors relative shortcuts, so no server-side clock /
/// timezone assumption is needed.
fn resolve_planned(
    input: &TimestampInput,
    today: &str,
) -> OrgResult<orgsidian_core::parser::semantic::PlannedStamp> {
    use orgsidian_core::parser::chrono::{NaiveDate, NaiveTime};
    use orgsidian_core::parser::semantic::{resolve_date_shortcut, PlannedStamp};

    let today = NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .map_err(|err| bad_timestamp(format!("invalid `today` {today:?}: {err}")))?;
    let date = resolve_date_shortcut(&input.date, today)
        .or_else(|| NaiveDate::parse_from_str(&input.date, "%Y-%m-%d").ok())
        .ok_or_else(|| bad_timestamp(format!("unrecognized date {:?}", input.date)))?;
    let time = match input.time.as_deref() {
        Some(text) if !text.is_empty() => Some(
            NaiveTime::parse_from_str(text, "%H:%M")
                .map_err(|err| bad_timestamp(format!("invalid time {text:?}: {err}")))?,
        ),
        _ => None,
    };
    Ok(PlannedStamp { date, time })
}

/// Story 4.8 (FR-9): compute the byte-faithful planning-line edit that sets (or
/// removes, when `timestamp` is `null`) the SCHEDULED/DEADLINE timestamp on the
/// headline whose line starts at `headline_id` (its byte offset in `source`).
///
/// This is a PURE text transformation: the org-structural work lives in the
/// parser's `set_planning_timestamp` (byte-faithful, recurring-cookie
/// preserving, unit-tested there); the command only resolves the wire input and
/// widens offsets. CM6 remains the buffer owner — the frontend applies the
/// returned [`PlanningEdit`] as one tagged transaction rather than the backend
/// mutating any state. Recurring cookies (`+1w`) survive because an existing
/// same-kind stamp's repeater/delay is carried onto the re-picked date.
///
/// The `headlineId`/`timestamp` pair is the epic-4-context contract; `source`,
/// `kind`, and `today` are the parameters that contract needs in practice (the
/// backend holds no editor buffer, and relative shortcuts need a reference
/// date). Never a raw `invoke` — the typed client is the only caller.
#[tauri::command]
#[specta::specta]
async fn set_scheduled(
    source: String,
    headline_id: u32,
    kind: PlanningKind,
    timestamp: Option<TimestampInput>,
    today: String,
) -> OrgResult<PlanningEdit> {
    use orgsidian_core::parser::semantic::{self, set_planning_timestamp};

    let kind = match kind {
        PlanningKind::Scheduled => semantic::PlanningKind::Scheduled,
        PlanningKind::Deadline => semantic::PlanningKind::Deadline,
    };
    let planned = match timestamp {
        Some(input) => Some(resolve_planned(&input, &today)?),
        None => None,
    };
    let edit = set_planning_timestamp(&source, headline_id as usize, kind, planned);
    Ok(PlanningEdit {
        from: edit.from as u32,
        to: edit.to as u32,
        insert: edit.insert,
    })
}

/// Implements FR-7 (Story 6.3 v0.1 Today Agenda subset): wire projection of
/// `orgsidian_core::AgendaItem` for `commands.agendaToday`. Multi-word fields
/// need the explicit camelCase rename — same reason as [`ConflictSummary`]:
/// the pinned `tauri-specta =2.0.0-rc.25` has no project-wide rename, and
/// `#[specta(rename_all)]` is rejected.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgendaItemDto {
    /// `headlines.id` — the click-to-open target's identity (route
    /// `$headlineId`). Narrowed from the index's `i64` rowid to `u32`:
    /// specta-typescript forbids exporting `i64`/`u64` (BigInt precision
    /// loss in JS `number`), and a Vault's headline count never approaches
    /// 4 billion — the same "narrow at the IPC boundary" call
    /// `ConflictSummary`'s byte lengths already make.
    pub headline_id: u32,
    /// Source file path — the grouping key and the other click-to-open
    /// target (route `$filePath`).
    pub file_path: String,
    /// Headline title, stars/keyword/tags already stripped.
    pub title: String,
    /// The Headline's byte offset in its file's source — the optional
    /// `byteStart` search param the editor route uses to place the cursor at
    /// the Headline itself, not just open the file. Same `i64` → `u32`
    /// narrowing rationale as `headline_id`.
    pub byte_start: u32,
    /// TODO keyword text, when the headline carries one.
    pub todo_keyword: Option<String>,
    /// `SCHEDULED:` date, when this row matched via the Scheduled-today leg.
    pub scheduled_date: Option<String>,
    /// `SCHEDULED:` time, when the timestamp carries one.
    pub scheduled_time: Option<String>,
    /// `DEADLINE:` date, when this row matched via the Deadline leg.
    pub deadline_date: Option<String>,
    /// `DEADLINE:` time, when the timestamp carries one.
    pub deadline_time: Option<String>,
    /// `true` when the Deadline is strictly before the query's anchor date
    /// (`today` for `commands.agendaToday`, `startDate` for
    /// `commands.agendaWeek`) — overdue, as opposed to due today/in-window.
    pub overdue: bool,
    /// The calendar day (`YYYY-MM-DD`) this row is grouped under — see
    /// `orgsidian_core::AgendaItem::agenda_date`'s docs (Story 6.4). Always
    /// present, including for `commands.agendaToday` rows (trivially `today`
    /// itself).
    pub agenda_date: String,
}

impl From<orgsidian_core::AgendaItem> for AgendaItemDto {
    fn from(item: orgsidian_core::AgendaItem) -> Self {
        AgendaItemDto {
            headline_id: item.headline_id as u32,
            file_path: item.file_path,
            title: item.title,
            byte_start: item.byte_start as u32,
            todo_keyword: item.todo_keyword,
            scheduled_date: item.scheduled_date,
            scheduled_time: item.scheduled_time,
            deadline_date: item.deadline_date,
            deadline_time: item.deadline_time,
            overdue: item.overdue,
            agenda_date: item.agenda_date,
        }
    }
}

/// Story 6.3 (FR-7): the `/today` route's data source —
/// `shell-ui/src/components/agenda/AgendaToday.tsx` calls this once per
/// mount. `today` is the frontend's local calendar day (`YYYY-MM-DD`), the
/// same convention `set_scheduled` (Story 4.8) established — never a
/// server-side clock read (see `orgsidian_core::agenda_today`'s docs).
/// Errors with `OrgError::Vault` when no Vault is active.
#[tauri::command]
#[specta::specta]
async fn agenda_today(
    today: String,
    state: tauri::State<'_, AppState>,
) -> OrgResult<Vec<AgendaItemDto>> {
    let vault_root = state.current_vault_root().ok_or_else(no_active_vault)?;
    let items = orgsidian_core::agenda_today(&vault_root, &today).await?;
    Ok(items.into_iter().map(AgendaItemDto::from).collect())
}

/// Story 6.4 (FR-7): the `/agenda/week` route's data source —
/// `shell-ui/src/components/agenda/AgendaWeek.tsx` calls this once per mount.
/// `start_date` is the frontend's local calendar day (`YYYY-MM-DD`), the
/// window's first ("current") day — same convention `agenda_today` uses (see
/// `orgsidian_core::agenda_week`'s docs). Errors with `OrgError::Vault` when
/// no Vault is active.
#[tauri::command]
#[specta::specta]
async fn agenda_week(
    start_date: String,
    state: tauri::State<'_, AppState>,
) -> OrgResult<Vec<AgendaItemDto>> {
    let vault_root = state.current_vault_root().ok_or_else(no_active_vault)?;
    let items = orgsidian_core::agenda_week(&vault_root, &start_date).await?;
    Ok(items.into_iter().map(AgendaItemDto::from).collect())
}

/// Request cancellation of the in-flight scan (LD-42 cancellable + partial
/// retained). A no-op when no scan is running.
#[tauri::command]
#[specta::specta]
fn cancel_index_scan(state: tauri::State<'_, AppState>) -> OrgResult<()> {
    if let Some(flag) = lock(&state.cancel).as_ref() {
        flag.store(true, Ordering::Release);
    }
    Ok(())
}

/// Construct the project's `tauri-specta` builder.
///
/// Shared between `run()` and `tests/export_bindings.rs` so the command list
/// stays in lockstep with the bindings-export test. Story 1.4 deviation from
/// the Dev Notes "don't preempt the v0.5 Beta cleanup" guidance: `pub(crate)
/// fn ping` + `#[tauri::command]` + `#[specta::specta]` triggers an
/// `__cmd__ping` macro name collision under tauri-specta `=2.0.0-rc.25`, so
/// the cleanup path was promoted to the implementation here. `#[doc(hidden)]`
/// because the function must be `pub` for the integration test to import it,
/// but is not part of the crate's intentional public API.
#[doc(hidden)]
pub fn build_specta() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .error_handling(ErrorHandlingMode::Throw)
        .commands(collect_commands![
            ping,
            designate_vault,
            cancel_index_scan,
            open_file,
            save_file,
            discard_external_changes,
            open_in_default_editor,
            set_editor_mode,
            get_editor_mode,
            set_scheduled,
            agenda_today,
            generate_starter_vault,
            has_configured_vault,
            agenda_week
        ])
        // Story 3.6: the app's first declared event lights up the `events`
        // object in the generated `tauri.ts`. Story 5.5 adds the second event —
        // the dirty-buffer conflict banner's trigger.
        .events(collect_events![IndexProgress, ConflictDetected])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let specta_builder = build_specta();

    // Debug builds re-export the typed TS client on every app start. Release
    // builds skip the write — bindings are produced via the
    // `cargo test --test export_bindings` step from `prebuild` for
    // reproducibility.
    #[cfg(debug_assertions)]
    specta_builder
        .export(
            Typescript::default(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../shell-ui/src/lib/tauri.ts"
            ),
        )
        .expect("tauri-specta TS client export failed");

    let tauri_builder = tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build());

    // Story 13.2 activates the updater runtime: generates the signing key,
    // populates `plugins.updater.{pubkey,endpoints}` in tauri.conf.json, and
    // registers `tauri_plugin_updater::Builder::new().build()` here behind
    // `#[cfg(desktop)]`. Story 1.3 ships the Cargo dep, JS binding, and
    // capability permission only — runtime registration without real config
    // fails deserialization at startup.

    tauri_builder
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);

            // Story 1.18 (LD-40): boot-time smoke for the TOML settings store.
            // Proves the wire compiles + reads; full GUI consumption is Story
            // 12.x scope. Failure does NOT abort startup — caller (Story 6.7+)
            // will wire the LD-41 backup-and-warn fallback.
            match orgsidian_core::settings::read_global_settings() {
                Ok(_settings) => tracing::info!(
                    target: "orgsidian::settings",
                    "LD-40 global settings loaded from disk (or default-on-missing)"
                ),
                Err(err) => tracing::warn!(
                    target: "orgsidian::settings",
                    error = %err,
                    "LD-40 global settings read failed; continuing with in-memory defaults (Story 6.7+ wires LD-41 fallback)"
                ),
            }

            Ok(())
        })
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Resolve a shared `.org` fixture (repo-root relative). Same `../..` hop
    /// out of the crate manifest dir the parser tests use.
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("vault-corpus")
            .join("extracted")
            .join(name)
    }

    #[test]
    fn open_file_reads_existing_fixture() {
        let path = fixture("0002_map.org");
        let expected = std::fs::read_to_string(&path).expect("fixture must be readable");

        let got = tauri::async_runtime::block_on(open_file_at(&path.to_string_lossy(), None))
            .expect("open_file should read the fixture");

        // Byte-faithful: the command returns exactly the file's content.
        assert_eq!(got, expected);
        assert!(!got.is_empty(), "fixture should not be empty");
    }

    #[test]
    fn open_file_missing_path_is_io_error() {
        let path = fixture("this-file-does-not-exist.org");

        let err = tauri::async_runtime::block_on(open_file_at(&path.to_string_lossy(), None))
            .expect_err("a missing path must error, not read");

        assert!(
            matches!(err, OrgError::Io { .. }),
            "missing path should map to OrgError::Io, got {err:?}"
        );
    }

    #[test]
    fn open_file_non_utf8_is_io_error() {
        // `read_to_string` surfaces non-UTF-8 bytes as an `InvalidData` IO
        // error, so the whole non-UTF-8 matrix row collapses to `OrgError::Io`.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("invalid.org");
        std::fs::write(&path, [0xFF, 0xFE, 0xFF]).expect("write invalid bytes");

        let err = tauri::async_runtime::block_on(open_file_at(&path.to_string_lossy(), None))
            .expect_err("non-UTF-8 content must error, not partially render");

        assert!(
            matches!(err, OrgError::Io { .. }),
            "non-UTF-8 content should map to OrgError::Io, got {err:?}"
        );
        // `dir` drops here, removing the temp file.
    }

    #[test]
    fn open_file_is_byte_faithful_for_multibyte_utf8() {
        // Prove the "byte-faithful" contract for non-ASCII: multibyte UTF-8
        // (accents, CJK, emoji) and a CR must survive the round-trip unchanged.
        // Writing our own fixture keeps the assertion honest — the shared
        // fixture is pure ASCII, so comparing it against `read_to_string` only
        // proves the function agrees with itself.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("multibyte.org");
        let source = "* Café ☕ 東京\n- naïve façade 🚀\r\nτίτλος\n";
        std::fs::write(&path, source).expect("write multibyte fixture");

        let got = tauri::async_runtime::block_on(open_file_at(&path.to_string_lossy(), None))
            .expect("open_file should read multibyte UTF-8");

        // Byte-for-byte identical — no Unicode normalization, no CRLF rewrite.
        assert_eq!(got, source);
        assert_eq!(got.as_bytes(), source.as_bytes());
    }

    #[test]
    fn resolve_open_path_joins_relative_path_onto_vault_root() {
        // Story 6.3 code review follow-up: an agenda row's `file_path` (the
        // vault-relative `rel_path`) must resolve against the active Vault
        // root, not the process cwd.
        let vault_root = PathBuf::from("/vault/root");
        let resolved = resolve_open_path("sub/notes.org", Some(&vault_root))
            .expect("relative path with a vault root should resolve");
        assert_eq!(resolved, vault_root.join("sub/notes.org"));
    }

    #[test]
    fn resolve_open_path_reads_absolute_path_unchanged() {
        // Back-compat: a caller that already has a full path (e.g. today's
        // direct fixture reads) is unaffected by the vault-root join.
        let path = fixture("0002_map.org");
        assert!(path.is_absolute(), "fixture path must be absolute");

        let resolved = resolve_open_path(&path.to_string_lossy(), Some(Path::new("/some/vault")))
            .expect("absolute path should resolve regardless of vault root");
        assert_eq!(resolved, path);

        // Also true with no vault designated at all.
        let resolved_no_vault = resolve_open_path(&path.to_string_lossy(), None)
            .expect("absolute path should resolve with no vault designated");
        assert_eq!(resolved_no_vault, path);
    }

    #[test]
    fn resolve_open_path_relative_with_no_vault_is_vault_error() {
        // No vault designated + a relative path has nothing to resolve
        // against — the same `no_active_vault` error other Vault-scoped
        // commands (`agenda_today`) already return.
        let err = resolve_open_path("sub/notes.org", None)
            .expect_err("relative path with no vault must error, not read cwd-relative");

        assert!(
            matches!(err, OrgError::Vault { .. }),
            "relative path with no vault should map to OrgError::Vault, got {err:?}"
        );
    }

    #[test]
    fn open_file_at_resolves_relative_path_against_vault_root_end_to_end() {
        // End-to-end through `open_file_at` (the `open_file` command's testable
        // body): a relative path joined onto a real vault root reads the file.
        let vault_root = fixture("0002_map.org")
            .parent()
            .expect("fixture dir has a parent")
            .to_path_buf();

        let got = tauri::async_runtime::block_on(open_file_at("0002_map.org", Some(&vault_root)))
            .expect("relative path should resolve against the vault root and read");

        let expected = std::fs::read_to_string(fixture("0002_map.org")).expect("fixture readable");
        assert_eq!(got, expected);
    }

    /// Apply a [`PlanningEdit`] to `source` the way the CM6 transaction does.
    fn apply(source: &str, edit: &PlanningEdit) -> String {
        let (from, to) = (edit.from as usize, edit.to as usize);
        format!("{}{}{}", &source[..from], edit.insert, &source[to..])
    }

    #[test]
    fn set_scheduled_resolves_relative_shortcut_via_today() {
        // Story 4.8: the `+1w` shortcut resolves through the pure-Rust resolver
        // against the frontend-supplied `today`, then writes a planning line.
        let source = "* Plan the week\nBody\n".to_string();
        let edit = tauri::async_runtime::block_on(set_scheduled(
            source.clone(),
            0,
            PlanningKind::Scheduled,
            Some(TimestampInput {
                date: "+1w".to_string(),
                time: None,
            }),
            "2026-05-19".to_string(), // Tue
        ))
        .expect("set_scheduled should succeed");
        assert_eq!(
            apply(&source, &edit),
            "* Plan the week\nSCHEDULED: <2026-05-26 Tue>\nBody\n"
        );
    }

    #[test]
    fn set_scheduled_writes_literal_deadline_with_time() {
        let source = "* Ship it\n".to_string();
        let edit = tauri::async_runtime::block_on(set_scheduled(
            source.clone(),
            0,
            PlanningKind::Deadline,
            Some(TimestampInput {
                date: "2026-05-19".to_string(),
                time: Some("17:30".to_string()),
            }),
            "2026-05-19".to_string(),
        ))
        .expect("set_scheduled should succeed");
        assert_eq!(
            apply(&source, &edit),
            "* Ship it\nDEADLINE: <2026-05-19 Tue 17:30>\n"
        );
    }

    #[test]
    fn set_scheduled_removes_when_timestamp_null() {
        let source = "* Task\nSCHEDULED: <2026-05-19 Tue>\nBody\n".to_string();
        let edit = tauri::async_runtime::block_on(set_scheduled(
            source.clone(),
            0,
            PlanningKind::Scheduled,
            None,
            "2026-05-19".to_string(),
        ))
        .expect("removal should succeed");
        assert_eq!(apply(&source, &edit), "* Task\nBody\n");
    }

    /// Story 5.5: the `ConflictDetected` event projects a real block into a
    /// redaction-safe payload — path + content byte-lengths + ancestor hash, and
    /// NEVER the user's note text (the redaction contract crossing IPC).
    #[test]
    fn conflict_detected_projects_redacted_notice() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("notes.org");
        std::fs::write(&path, "EXTERNAL DISK\n").expect("write external"); // 14 bytes

        let buffers: SharedDirtyBuffers = Default::default();
        buffers
            .write()
            .expect("lock")
            .mark_dirty(path.clone(), "BUFFER SECRET\n"); // 14 bytes
        let pending: SharedPendingConflicts = Default::default();

        let notice = tauri::async_runtime::block_on(orgsidian_core::resolve_dirty_conflict(
            &orgsidian_core::BlockWithWarning,
            &pending,
            &buffers,
            &path,
        ))
        .expect("resolve")
        .expect("dirty write is blocked");

        let event = ConflictDetected::from_notice(&notice);
        assert_eq!(event.path, path.to_string_lossy());
        assert_eq!(event.state.buffer_len, "BUFFER SECRET\n".len() as u32);
        assert_eq!(event.state.external_len, "EXTERNAL DISK\n".len() as u32);
        // Ancestor hash is a 64-hex-char digest, carried in full.
        assert_eq!(event.state.ancestor_hash.len(), 64);

        // Redaction: no note text anywhere in the event, even under `{:?}`.
        let rendered = format!("{event:?}");
        assert!(!rendered.contains("SECRET"), "{rendered}");
        assert!(!rendered.contains("DISK"), "{rendered}");
    }

    #[test]
    fn set_scheduled_rejects_malformed_date() {
        let err = tauri::async_runtime::block_on(set_scheduled(
            "* Task\n".to_string(),
            0,
            PlanningKind::Scheduled,
            Some(TimestampInput {
                date: "nonsense".to_string(),
                time: None,
            }),
            "2026-05-19".to_string(),
        ))
        .expect_err("a date that is neither a shortcut nor ISO must error");
        assert!(matches!(err, OrgError::Parse { .. }), "got {err:?}");
    }

    /// Story 6.3: `AgendaItemDto::from` carries every field across the IPC
    /// projection unchanged (this is a straight field copy, not a redaction —
    /// unlike `ConflictDetected`, nothing here is sensitive).
    #[test]
    fn agenda_item_dto_projects_every_field() {
        let core_item = orgsidian_core::AgendaItem {
            headline_id: 42,
            file_path: "inbox.org".to_string(),
            title: "Ship v0.1".to_string(),
            byte_start: 128,
            todo_keyword: Some("TODO".to_string()),
            scheduled_date: Some("2026-09-05".to_string()),
            scheduled_time: None,
            deadline_date: Some("2026-09-01".to_string()),
            deadline_time: Some("17:00".to_string()),
            overdue: true,
            agenda_date: "2026-09-05".to_string(),
        };

        let dto = AgendaItemDto::from(core_item.clone());

        assert_eq!(dto.headline_id, core_item.headline_id as u32);
        assert_eq!(dto.file_path, core_item.file_path);
        assert_eq!(dto.title, core_item.title);
        assert_eq!(dto.byte_start, core_item.byte_start as u32);
        assert_eq!(dto.todo_keyword, core_item.todo_keyword);
        assert_eq!(dto.scheduled_date, core_item.scheduled_date);
        assert_eq!(dto.scheduled_time, core_item.scheduled_time);
        assert_eq!(dto.deadline_date, core_item.deadline_date);
        assert_eq!(dto.deadline_time, core_item.deadline_time);
        assert_eq!(dto.overdue, core_item.overdue);
        assert_eq!(dto.agenda_date, core_item.agenda_date);
    }

    /// Story 6.2: `generate_starter_vault`'s `today` parse — a literal
    /// `YYYY-MM-DD` (the frontend's `localTodayIso()`) resolves cleanly.
    #[test]
    fn resolve_today_accepts_iso_date() {
        let date = resolve_today("2026-09-05").expect("valid ISO date");
        assert_eq!(date.to_string(), "2026-09-05");
    }

    /// A malformed `today` (should be unreachable from the picker, which
    /// always sends `localTodayIso()`) errors with `OrgError::Parse` rather
    /// than panicking — same defensive posture as `set_scheduled`'s `today`.
    #[test]
    fn resolve_today_rejects_malformed_date() {
        let err = resolve_today("not-a-date").expect_err("malformed date must error");
        assert!(matches!(err, OrgError::Parse { .. }), "got {err:?}");
    }

    /// Story 6.2: pin the wire→core `StarterVaultKind` mapping. The full
    /// `generate_starter_vault` command needs an `AppHandle`/`State`, but the
    /// arm that can silently ship the WRONG Starter Vault is this pure
    /// translation — assert each variant maps to its matching core twin so a
    /// swapped arm fails here rather than in a user's freshly-generated Vault.
    #[test]
    fn starter_vault_kind_maps_to_matching_core_variant() {
        assert!(matches!(
            StarterVaultKind::PersonalGtd.to_core(),
            orgsidian_core::StarterVaultKind::PersonalGtd
        ));
        assert!(matches!(
            StarterVaultKind::Student.to_core(),
            orgsidian_core::StarterVaultKind::Student
        ));
    }

    /// Story 6.2 hardening: a target folder that already holds a top-level
    /// `.org` file is refused — the same "arm that can silently ship the
    /// wrong thing" the full `generate_starter_vault` command needs an
    /// `AppHandle`/`State` to exercise end-to-end, so (as with
    /// `starter_vault_kind_maps_to_matching_core_variant` above) the guard is
    /// unit-tested directly rather than through the full command.
    #[test]
    fn ensure_target_has_no_org_files_rejects_a_populated_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("notes.org"), "* Pre-existing\n").expect("seed .org file");

        let err = tauri::async_runtime::block_on(ensure_target_has_no_org_files(
            &dir.path().to_string_lossy(),
        ))
        .expect_err("a populated folder must be refused");

        assert!(matches!(err, OrgError::Vault { .. }), "got {err:?}");
        let OrgError::Vault { reason } = err else {
            unreachable!()
        };
        assert!(reason.contains(".org"), "{reason}");
        assert!(
            reason.contains("Use my own folder"),
            "should steer the user toward the existing-Vault path: {reason}"
        );
    }

    /// A folder containing files but no `.org` file (e.g. a stray `.txt`) is
    /// NOT refused — the guard checks for `.org` files specifically, not "any
    /// file at all".
    #[test]
    fn ensure_target_has_no_org_files_allows_folder_with_non_org_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("readme.txt"), "not an org file").expect("seed file");

        tauri::async_runtime::block_on(ensure_target_has_no_org_files(
            &dir.path().to_string_lossy(),
        ))
        .expect("a folder with no .org file must be allowed");
    }

    /// An empty existing folder is allowed (the common "fresh empty folder"
    /// first-launch case).
    #[test]
    fn ensure_target_has_no_org_files_allows_empty_existing_folder() {
        let dir = tempfile::tempdir().expect("tempdir");

        tauri::async_runtime::block_on(ensure_target_has_no_org_files(
            &dir.path().to_string_lossy(),
        ))
        .expect("an empty folder must be allowed");
    }

    /// A folder that doesn't exist yet is allowed — `generate_starter_vault`'s
    /// own `create_dir_all` creates it, so there is nothing to check.
    #[test]
    fn ensure_target_has_no_org_files_allows_missing_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("brand-new-vault");
        assert!(!missing.exists());

        tauri::async_runtime::block_on(ensure_target_has_no_org_files(&missing.to_string_lossy()))
            .expect("a not-yet-created folder must be allowed");
    }
}
