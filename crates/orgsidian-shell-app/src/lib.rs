use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use orgsidian_core::{IndexHandle, Result as OrgResult};
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, collect_events, Builder, ErrorHandlingMode, Event};

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
    let outcome = designate_and_scan(&path, &app, &cancel).await;
    *lock(&state.cancel) = None;

    let handle = outcome?;
    // Retain the handle for later reads.
    *lock(&state.index) = Some(handle);
    Ok(())
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
        .commands(collect_commands![ping, designate_vault, cancel_index_scan])
        // Story 3.6: the app's first declared event lights up the `events`
        // object in the generated `tauri.ts`.
        .events(collect_events![IndexProgress])
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
