// Story 2.8 (LD-27 / architecture CLI Documentation Strategy): build-time
// man-page generation. `build.rs` cannot `use orgsidian_cli::…` (the crate
// is not built yet), so the clap derive definitions are `include!`-shared
// from `src/cli.rs` (kept clap+std-only by contract) and rendered via
// `clap_mangen::generate_to`, which emits the root page plus one hyphenated
// page per subcommand (`orgsidian.1`, `orgsidian-parse.1`).
//
// VARIANCE (recorded in the story file): writing into the source tree from
// build.rs is non-idiomatic Cargo (OUT_DIR is the convention) but the
// architecture mandates the in-tree `man/` dir; the generated pages are
// committed as a reviewable artifact and bundled by the release pipeline
// later. The dir is resolved via CARGO_MANIFEST_DIR — never cwd-relative.
// Build scripts may `expect()` (build tool, not library code).

use clap::CommandFactory as _;

include!("src/cli.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli.rs");

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo always sets CARGO_MANIFEST_DIR");
    let man_dir = std::path::Path::new(&manifest_dir).join("man");
    std::fs::create_dir_all(&man_dir).expect("create the in-tree man/ dir");
    clap_mangen::generate_to(Cli::command(), &man_dir).expect("render man pages into man/");
}
