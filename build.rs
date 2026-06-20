use std::path::Path;

fn main() {
    let grammar = "tree-sitter-structurizr-dsl/src";
    let parser = format!("{grammar}/parser.c");
    let header = format!("{grammar}/tree_sitter/parser.h");
    for artifact in [&parser, &header] {
        if !Path::new(artifact).is_file() {
            panic!(
                "Tree-sitter generated parser artifact is missing: {artifact}\nRun: make grammar"
            );
        }
        println!("cargo:rerun-if-changed={artifact}");
    }
    cc::Build::new()
        .include(grammar)
        .file(parser)
        .warnings(false)
        .compile("tree-sitter-structurizr-dsl");
}
