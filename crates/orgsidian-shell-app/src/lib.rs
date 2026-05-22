use orgsidian_core::Result as OrgResult;
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

#[tauri::command]
#[specta::specta]
fn ping() -> OrgResult<String> {
    Ok("pong".to_string())
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
        .commands(collect_commands![ping])
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../shell-ui/src/lib/tauri.ts"),
        )
        .expect("tauri-specta TS client export failed");

    let tauri_builder = tauri::Builder::default()
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
            Ok(())
        })
        .run(tauri::generate_context!())
}
