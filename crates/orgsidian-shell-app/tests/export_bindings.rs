//! One-shot binding-export integration test.
//!
//! Doubles as the regen step invoked by `shell-ui` `prebuild` so fresh clones
//! and CI runs can produce `shell-ui/src/lib/tauri.ts` without first running
//! `pnpm tauri dev`. Also serves as a CI assertion that bindings stay
//! exportable (i.e. that no command's type breaks `specta::Type` derivation).
//!
//! Story 1.8 (AC7, closes Story 1.4 deferred-work): the test now asserts on
//! generated content. A regression dropping `OrgError`, the `kind`
//! discriminator, or the `ping` command from the bindings will fail loudly.
//!
//! Windows skip (see issue #120). The test exe links transitively against
//! `webview2-com-sys 0.38.2` via `Builder<tauri::Wry>` from `build_specta()`;
//! on `windows-2022` the OS loader fails resolving a webview2 import at process
//! startup with `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), BEFORE `main()` runs.
//! `#[ignore]` filters test execution but cannot bypass load-time failure — the
//! whole module is conditionally compiled out on Windows so Cargo produces an
//! inert test binary with no `tauri::Wry` linkage. macOS + Ubuntu are unaffected
//! (different webview backend). Proper fix is to extract `build_specta()` into
//! an IPC-contract crate that doesn't depend on `tauri::Wry`; tracked at #120.
#![cfg(not(target_os = "windows"))]

use std::fs;
use std::path::PathBuf;

use orgsidian_shell_app_lib::build_specta;
use specta_typescript::Typescript;

#[test]
fn export_bindings() {
    let out: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("shell-ui")
        .join("src")
        .join("lib")
        .join("tauri.ts");

    build_specta()
        .export(Typescript::default(), &out)
        .expect("tauri-specta TS client export failed");

    // Fresh-checkout fallback: the export() above writes the file, so it
    // MUST exist now. But if a future refactor moves the output, surface a
    // clear skip rather than a panic with a confusing path error.
    if !out.exists() {
        eprintln!(
            "export_bindings: skipped — {} does not exist. Run `pnpm --filter shell-ui build` first.",
            out.display()
        );
        return;
    }

    let contents = fs::read_to_string(&out).unwrap_or_else(|e| {
        panic!(
            "failed to read generated bindings at {}: {e}",
            out.display()
        )
    });

    // Story 1.4 deferred-work substance: a regression that drops any of these
    // anchors from the bindings would have shipped green before today. Story 3.6
    // extends the set with the new commands + the FIRST specta event surface
    // (`events.indexProgress` on the `index-progress` wire name), so a
    // regression dropping the designation command or the progress event fails
    // loudly here.
    for anchor in [
        "export const commands",
        "ping",
        "OrgError",
        "kind",
        "designateVault",
        "cancelIndexScan",
        "openFile",
        "setEditorMode",
        "getEditorMode",
        "EditorMode",
        // Story 5.5: the dirty-buffer block-save fallback command surface + the
        // conflict-banner event. A regression dropping any fails loudly here.
        "saveFile",
        "discardExternalChanges",
        "openInDefaultEditor",
        // Story 6.3 (FR-7): the Today Agenda query surface.
        "agendaToday",
        "AgendaItemDto",
        // Story 6.2: the Starter Vault picker's generate-then-designate command
        // + the onboarding-gate query. A regression dropping either fails
        // loudly here.
        "generateStarterVault",
        "hasConfiguredVault",
        "StarterVaultKind",
        // Story 6.4 (FR-7): the Week Agenda query surface + the new
        // `AgendaItemDto` grouping field (a serde-rename/field-drop regression
        // on it would otherwise ship green — `AgendaItemDto` alone is anchored
        // above but its field list is not).
        "agendaWeek",
        "agendaDate",
        // Story 6.6 (FR-21 partial / UJ-4): the hardcoded coaching-balloon
        // dismissal read/write commands. A regression dropping either fails
        // loudly here.
        "getDismissedCoaching",
        "dismissCoaching",
        "export const events",
        "indexProgress",
        "index-progress",
        "IndexProgress",
        "conflictDetected",
        "conflict-detected",
        "ConflictDetected",
    ] {
        assert!(
            contents.contains(anchor),
            "generated tauri.ts missing anchor `{anchor}` — likely IPC regression. File: {}",
            out.display()
        );
    }
}
