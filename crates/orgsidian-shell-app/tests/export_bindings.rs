//! One-shot binding-export integration test.
//!
//! Doubles as the regen step invoked by `shell-ui` `prebuild` so fresh clones
//! and CI runs can produce `shell-ui/src/lib/tauri.ts` without first running
//! `pnpm tauri dev`. Also serves as a CI assertion that bindings stay
//! exportable (i.e. that no command's type breaks `specta::Type` derivation).

use orgsidian_shell_app_lib::build_specta;
use specta_typescript::Typescript;

#[test]
fn export_bindings() {
    build_specta()
        .export(Typescript::default(), "../../shell-ui/src/lib/tauri.ts")
        .expect("tauri-specta TS client export failed");
}
