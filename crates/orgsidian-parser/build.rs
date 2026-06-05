// Story 2.1 (LD-48): compile the vendored nvim-orgmode/tree-sitter-org
// grammar (git submodule at grammar/) into orgsidian-parser via cc.
//
// This standalone build script is deliberate: we do NOT depend on the
// upstream `bindings/rust/build.rs` (a) to keep `tree-sitter-org` a
// filesystem-only submodule rather than a Cargo path-dep — which would
// otherwise pull it in as a workspace-member candidate and violate the
// LD-5 LEAF graph rule — and (b) so our build surface is ~30 lines of
// obvious code instead of an upstream file that could change shape.

fn main() {
    let grammar_src = std::path::Path::new("grammar").join("src");
    let parser_c = grammar_src.join("parser.c");
    let scanner_c = grammar_src.join("scanner.c");
    let headers_dir = grammar_src.join("tree_sitter");

    // Emit `rerun-if-changed` BEFORE the existence check so that cargo
    // re-runs build.rs after `git submodule update --init --recursive`
    // (otherwise a fresh-clone panic leaves cargo with no watched path and
    // recovery requires `cargo clean`). Watching the headers dir
    // (`tree_sitter/parser.h` etc., included by parser.c + scanner.c)
    // ensures SHA bumps that only touch headers also trigger a rebuild.
    println!("cargo:rerun-if-changed={}", parser_c.display());
    println!("cargo:rerun-if-changed={}", scanner_c.display());
    println!("cargo:rerun-if-changed={}", headers_dir.display());

    // Anti-footgun: hard-fail with a parser-owner-readable message if the
    // submodule has not been initialized (fresh clone without
    // `git submodule update --init --recursive` — extremely common).
    if !parser_c.exists() {
        panic!(
            "tree-sitter-org submodule not initialized. \
             Run `git submodule update --init --recursive` from the repo root. \
             (LD-48: grammar is a SHA-pinned submodule, not a cargo git-dep.) \
             Missing file: {}",
            parser_c.display()
        );
    }

    let mut c_config = cc::Build::new();
    c_config.include(&grammar_src);
    c_config
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs");
    #[cfg(target_env = "msvc")]
    c_config.flag("-utf-8");

    c_config.file(&parser_c);

    if scanner_c.exists() {
        c_config.file(&scanner_c);
    }

    c_config.compile("tree_sitter_org");
}
