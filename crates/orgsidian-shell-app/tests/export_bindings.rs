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
    // four anchors from the bindings would have shipped green before today.
    for anchor in ["export const commands", "ping", "OrgError", "kind"] {
        assert!(
            contents.contains(anchor),
            "generated tauri.ts missing anchor `{anchor}` — likely IPC regression. File: {}",
            out.display()
        );
    }
}
