use crate::{
    diagnostic::Diagnostic,
    source::{SourceFile, Span},
};
use tree_sitter::{Language, Node, Parser, Tree};
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_structurizr_dsl() -> *const ();
}

const LANGUAGE_FN: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_structurizr_dsl) };

pub fn language() -> Language {
    LANGUAGE_FN.into()
}

pub fn parse(source: &SourceFile) -> Result<Tree, Diagnostic> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .map_err(|error| Diagnostic::error(source_span(source), error.to_string()))?;
    let tree = parser.parse(&source.text, None).ok_or_else(|| {
        Diagnostic::error(source_span(source), "Tree-sitter parser was cancelled")
    })?;
    if let Some(node) = first_error(tree.root_node()) {
        let span = Span::new(source.id, node.start_byte(), node.end_byte());
        let message = if node.is_missing() {
            format!("missing syntax near {}", node.kind())
        } else {
            "invalid Structurizr DSL syntax".into()
        };
        return Err(Diagnostic::error(span, message)
            .with_help("check this statement against the supported M1-M7 grammar"));
    }
    Ok(tree)
}

fn first_error(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let error = node.children(&mut cursor).find_map(first_error);
    error
}

fn source_span(source: &SourceFile) -> Span {
    Span::new(source.id, 0, source.text.len().min(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceMap;
    use tree_sitter::Query;

    const FIXTURES: &[(&str, &str)] = &[
        (
            "examples/internet-banking.dsl",
            include_str!("../examples/internet-banking.dsl"),
        ),
        (
            "tests/fixtures/invalid-relationship.dsl",
            include_str!("../tests/fixtures/invalid-relationship.dsl"),
        ),
        (
            "tests/fixtures/m3-core.dsl",
            include_str!("../tests/fixtures/m3-core.dsl"),
        ),
        (
            "tests/fixtures/m3-deployment.dsl",
            include_str!("../tests/fixtures/m3-deployment.dsl"),
        ),
        (
            "tests/fixtures/m3-extension-base.dsl",
            include_str!("../tests/fixtures/m3-extension-base.dsl"),
        ),
        (
            "tests/fixtures/m3-extension.dsl",
            include_str!("../tests/fixtures/m3-extension.dsl"),
        ),
        (
            "tests/fixtures/m3-json-extension.dsl",
            include_str!("../tests/fixtures/m3-json-extension.dsl"),
        ),
        (
            "tests/fixtures/m3-remove-relationship.dsl",
            include_str!("../tests/fixtures/m3-remove-relationship.dsl"),
        ),
        (
            "tests/fixtures/m3-unsafe.dsl",
            include_str!("../tests/fixtures/m3-unsafe.dsl"),
        ),
        (
            "tests/fixtures/m3-url-extension.dsl",
            include_str!("../tests/fixtures/m3-url-extension.dsl"),
        ),
        (
            "tests/fixtures/m4-deployment.dsl",
            include_str!("../tests/fixtures/m4-deployment.dsl"),
        ),
        (
            "tests/fixtures/m4-remote-image.dsl",
            include_str!("../tests/fixtures/m4-remote-image.dsl"),
        ),
        (
            "tests/fixtures/m4-views.dsl",
            include_str!("../tests/fixtures/m4-views.dsl"),
        ),
        (
            "tests/fixtures/m5-styles.dsl",
            include_str!("../tests/fixtures/m5-styles.dsl"),
        ),
        (
            "tests/fixtures/m5-remote.dsl",
            include_str!("../tests/fixtures/m5-remote.dsl"),
        ),
        (
            "tests/fixtures/m5-invalid.dsl",
            include_str!("../tests/fixtures/m5-invalid.dsl"),
        ),
        (
            "tests/fixtures/m6-preprocessing.dsl",
            include_str!("../tests/fixtures/m6-preprocessing.dsl"),
        ),
        (
            "tests/fixtures/m6-remote.dsl",
            include_str!("../tests/fixtures/m6-remote.dsl"),
        ),
        (
            "tests/fixtures/m6-unsafe.dsl",
            include_str!("../tests/fixtures/m6-unsafe.dsl"),
        ),
        (
            "tests/fixtures/m7-docs.dsl",
            include_str!("../tests/fixtures/m7-docs.dsl"),
        ),
        (
            "tests/fixtures/m7-custom-importer.dsl",
            include_str!("../tests/fixtures/m7-custom-importer.dsl"),
        ),
        (
            "tests/fixtures/m7-invalid-paths.dsl",
            include_str!("../tests/fixtures/m7-invalid-paths.dsl"),
        ),
    ];

    #[test]
    fn parses_all_m1_to_m7_fixtures_without_error_or_missing_nodes() {
        for (path, text) in FIXTURES {
            let (sources, source_id) = SourceMap::from_text(*path, *text);
            let tree = parse(sources.get(source_id)).unwrap_or_else(|error| {
                panic!("{path}: {}", error.render(&sources));
            });
            assert!(!tree.root_node().has_error(), "{path}");
        }
    }

    #[test]
    fn editor_queries_compile() {
        for query in [
            include_str!("../tree-sitter-structurizr-dsl/queries/highlights.scm"),
            include_str!("../tree-sitter-structurizr-dsl/queries/folds.scm"),
            include_str!("../tree-sitter-structurizr-dsl/queries/locals.scm"),
        ] {
            Query::new(&language(), query).unwrap();
        }
    }

    #[test]
    fn malformed_cst_reports_source_context() {
        let (sources, source_id) =
            SourceMap::from_text("tests/tree-error.dsl", "workspace {\n  model {\n}\n");
        let error = parse(sources.get(source_id)).unwrap_err().render(&sources);
        assert!(error.contains("tests/tree-error.dsl:"));
        assert!(error.contains("error:"));
        assert!(error.contains('|'));
        assert!(error.contains('^'));
        assert!(error.contains("help:"));
    }
}
