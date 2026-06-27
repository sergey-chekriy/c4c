use crate::{
    compiler::Workspace,
    diagnostic::render_all,
    handwritten_parser, lexer,
    source::{SourceId, SourceMap},
    tree_sitter_parser,
};
use tree_sitter::Tree;

pub fn parse(
    sources: &SourceMap,
    source_id: SourceId,
    identifiers: &str,
) -> Result<Workspace, String> {
    let source = sources.get(source_id);
    let syntax = tree_sitter_parser::parse(source);
    let tokens = lexer::lex(source).map_err(|diagnostics| render_all(&diagnostics, sources))?;
    let workspace = handwritten_parser::parse(sources, source_id, tokens, identifiers)?;
    match syntax {
        Ok(tree) => adapt_cst(tree, workspace),
        Err(diagnostic) => Err(diagnostic.render(sources)),
    }
}

fn adapt_cst(tree: Tree, workspace: Workspace) -> Result<Workspace, String> {
    if tree.root_node().kind() != "source_file" {
        return Err("Tree-sitter returned an unexpected root node".into());
    }
    // ponytail: reuse the proven semantic builder until direct CST mapping has fixture parity.
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_preserves_legacy_semantic_model_counts() {
        for source in [
            include_str!("../examples/internet-banking.dsl"),
            include_str!("../tests/fixtures/m3-core.dsl"),
            include_str!("../tests/fixtures/m4-views.dsl"),
            include_str!("../tests/fixtures/m5-styles.dsl"),
            include_str!("../tests/fixtures/m8-exporters.dsl"),
            include_str!("../tests/fixtures/m83-archimate-profile.dsl"),
            include_str!("../tests/fixtures/archi-native/m86-mini-expected.dsl"),
            include_str!("../tests/fixtures/m87-archimate-full-vocabulary.dsl"),
            include_str!("../tests/fixtures/m88-archimate-32-full-vocabulary.dsl"),
        ] {
            let (sources, source_id) = SourceMap::from_text("parity.dsl", source);
            let tokens = lexer::lex(sources.get(source_id)).unwrap();
            let legacy = handwritten_parser::parse(&sources, source_id, tokens, "flat").unwrap();
            let current = parse(&sources, source_id, "flat").unwrap();
            assert_eq!(
                (
                    current.elements.len(),
                    current.relationships.len(),
                    current.views.len(),
                    current.warnings.len(),
                ),
                (
                    legacy.elements.len(),
                    legacy.relationships.len(),
                    legacy.views.len(),
                    legacy.warnings.len(),
                )
            );
        }
    }
}
