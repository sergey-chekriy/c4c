use crate::{
    diagnostic::{render_all, Diagnostic},
    lexer, parser,
    source::{SourceMap, Span},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub name: Option<String>,
    pub description: Option<String>,
    pub elements: Vec<Element>,
    pub relationships: Vec<Relationship>,
    pub views: Vec<View>,
    pub identifiers: String,
    pub span: Span,
    pub source_map: SourceMap,
}

impl Workspace {
    pub(crate) fn new(source_map: SourceMap, span: Span) -> Self {
        Self {
            name: None,
            description: None,
            elements: Vec::new(),
            relationships: Vec::new(),
            views: Vec::new(),
            identifiers: "flat".into(),
            span,
            source_map,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub kind: ElementKind,
    pub name: String,
    pub description: Option<String>,
    pub technology: Option<String>,
    pub parent: Option<String>,
    pub tags: Vec<String>,
    pub span: Span,
    pub id_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    Person,
    SoftwareSystem,
    Container,
    Component,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub source: String,
    pub destination: String,
    pub description: Option<String>,
    pub technology: Option<String>,
    pub tags: Vec<String>,
    pub span: Span,
    pub source_span: Span,
    pub destination_span: Span,
}

#[derive(Debug, Clone)]
pub struct View {
    pub kind: ViewKind,
    pub scope: Option<String>,
    pub key: Option<String>,
    pub description: Option<String>,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub auto_layout: Option<String>,
    pub title: Option<String>,
    pub span: Span,
    pub scope_span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewKind {
    SystemContext,
    Container,
}

pub fn compile_file(path: &str) -> Result<Workspace, String> {
    let (sources, source_id) = SourceMap::load(path)?;
    compile_sources(sources, source_id)
}

#[cfg(test)]
pub fn compile(source: &str) -> Result<Workspace, String> {
    let (sources, source_id) = SourceMap::from_text("<memory>", source);
    compile_sources(sources, source_id)
}

fn compile_sources(
    sources: SourceMap,
    source_id: crate::source::SourceId,
) -> Result<Workspace, String> {
    let tokens = lexer::lex(sources.get(source_id))
        .map_err(|diagnostics| render_all(&diagnostics, &sources))?;
    parser::parse(sources, source_id, tokens)
}

pub fn validate(workspace: &Workspace) -> Result<(), String> {
    let mut diagnostics = Vec::new();
    let mut identifiers = HashMap::new();
    for element in &workspace.elements {
        if let Some(original) = identifiers.insert(element.id.as_str(), element) {
            diagnostics.push(
                Diagnostic::error(
                    element.id_span,
                    format!("duplicate identifier '{}'", element.id),
                )
                .with_help(format!(
                    "rename this element; '{}' was first defined at byte {}",
                    original.id, original.id_span.start
                )),
            );
        }
        match element.kind {
            ElementKind::Container => require_parent(
                workspace,
                element,
                ElementKind::SoftwareSystem,
                "container",
                &mut diagnostics,
            ),
            ElementKind::Component => require_parent(
                workspace,
                element,
                ElementKind::Container,
                "component",
                &mut diagnostics,
            ),
            _ => {}
        }
    }
    for relationship in &workspace.relationships {
        debug_assert!(relationship.span.start <= relationship.source_span.start);
        debug_assert!(relationship.span.end >= relationship.destination_span.end);
        if find(workspace, &relationship.source).is_none() {
            diagnostics.push(
                Diagnostic::error(
                    relationship.source_span,
                    format!(
                        "relationship source '{}' is not defined",
                        relationship.source
                    ),
                )
                .with_help(format!(
                    "define '{}' before this relationship, or check the identifier",
                    relationship.source
                )),
            );
        }
        if find(workspace, &relationship.destination).is_none() {
            diagnostics.push(
                Diagnostic::error(
                    relationship.destination_span,
                    format!(
                        "relationship destination '{}' is not defined",
                        relationship.destination
                    ),
                )
                .with_help(format!(
                    "define '{}' before this relationship, or check the identifier",
                    relationship.destination
                )),
            );
        }
    }
    for view in &workspace.views {
        match view.kind {
            ViewKind::SystemContext => require_kind(
                workspace,
                view,
                ElementKind::SoftwareSystem,
                "systemContext",
                &mut diagnostics,
            ),
            ViewKind::Container => require_kind(
                workspace,
                view,
                ElementKind::SoftwareSystem,
                "container view",
                &mut diagnostics,
            ),
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(render_all(&diagnostics, &workspace.source_map))
    }
}

pub fn inspect(workspace: &Workspace) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "workspace: {}\n",
        workspace.name.as_deref().unwrap_or("<unnamed>")
    ));
    output.push_str("elements:\n");
    for element in &workspace.elements {
        output.push_str(&format!(
            "  {} {:?} '{}' desc={:?} tech={:?} parent={:?} tags={:?}\n",
            element.id,
            element.kind,
            element.name,
            element.description,
            element.technology,
            element.parent,
            element.tags
        ));
    }
    output.push_str("relationships:\n");
    for relationship in &workspace.relationships {
        output.push_str(&format!(
            "  {} -> {} desc={:?} tech={:?} tags={:?}\n",
            relationship.source,
            relationship.destination,
            relationship.description,
            relationship.technology,
            relationship.tags
        ));
    }
    output.push_str("views:\n");
    for view in &workspace.views {
        output.push_str(&format!(
            "  {:?} scope={:?} key={:?} desc={:?}\n",
            view.kind, view.scope, view.key, view.description
        ));
    }
    output
}

pub fn export_mermaid(workspace: &Workspace, output: &Path) -> Result<(), String> {
    for view in &workspace.views {
        let key = view.key.clone().unwrap_or_else(|| match view.kind {
            ViewKind::SystemContext => "system-context".into(),
            ViewKind::Container => "container".into(),
        });
        fs::write(output.join(format!("{key}.mmd")), mermaid(workspace, view))
            .map_err(|error| format!("cannot write {key}.mmd: {error}"))?;
    }
    Ok(())
}

fn mermaid(workspace: &Workspace, view: &View) -> String {
    let identifiers = view_element_ids(workspace, view);
    let mut output = String::from("flowchart LR\n");
    for element in &workspace.elements {
        if !identifiers.contains(&element.id) {
            continue;
        }
        let label = format!("{}\\n{}", escape(&element.name), kind_label(&element.kind));
        output.push_str(&format!("  {}[\"{}\"]\n", node_id(&element.id), label));
    }
    let mut emitted = HashSet::new();
    for relationship in &workspace.relationships {
        let source = view_endpoint(workspace, view, &relationship.source);
        let destination = view_endpoint(workspace, view, &relationship.destination);
        if source == destination
            || !identifiers.contains(&source)
            || !identifiers.contains(&destination)
        {
            continue;
        }
        let key = format!(
            "{}->{}:{}",
            source,
            destination,
            relationship.description.as_deref().unwrap_or("")
        );
        if emitted.insert(key) {
            output.push_str(&format!(
                "  {} -->|{}| {}\n",
                node_id(&source),
                escape(relationship.description.as_deref().unwrap_or("")),
                node_id(&destination)
            ));
        }
    }
    output
}

fn view_element_ids(workspace: &Workspace, view: &View) -> HashSet<String> {
    if !view.includes.iter().any(|include| include == "*") {
        return view.includes.iter().cloned().collect();
    }
    let mut identifiers = HashSet::new();
    match view.kind {
        ViewKind::SystemContext => {
            if let Some(scope) = &view.scope {
                identifiers.insert(scope.clone());
                for relationship in &workspace.relationships {
                    let source = view_endpoint(workspace, view, &relationship.source);
                    let destination = view_endpoint(workspace, view, &relationship.destination);
                    if source == *scope {
                        identifiers.insert(destination.clone());
                    }
                    if destination == *scope {
                        identifiers.insert(source);
                    }
                }
            }
        }
        ViewKind::Container => {
            if let Some(scope) = &view.scope {
                for element in &workspace.elements {
                    if element.parent.as_deref() == Some(scope) || element.id == *scope {
                        identifiers.insert(element.id.clone());
                    }
                }
                for relationship in &workspace.relationships {
                    if identifiers.contains(&relationship.source)
                        || identifiers.contains(&relationship.destination)
                    {
                        identifiers.insert(relationship.source.clone());
                        identifiers.insert(relationship.destination.clone());
                    }
                }
            }
        }
    }
    for exclude in &view.excludes {
        identifiers.remove(exclude);
    }
    identifiers
}

fn view_endpoint(workspace: &Workspace, view: &View, identifier: &str) -> String {
    match (&view.kind, view.scope.as_deref()) {
        (ViewKind::SystemContext, Some(scope))
            if is_descendant_of(workspace, identifier, scope) =>
        {
            scope.to_string()
        }
        _ => identifier.to_string(),
    }
}

fn is_descendant_of(workspace: &Workspace, identifier: &str, ancestor: &str) -> bool {
    if identifier == ancestor {
        return true;
    }
    let mut current = find(workspace, identifier).and_then(|element| element.parent.as_deref());
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = find(workspace, parent).and_then(|element| element.parent.as_deref());
    }
    false
}

fn find<'a>(workspace: &'a Workspace, identifier: &str) -> Option<&'a Element> {
    workspace
        .elements
        .iter()
        .find(|element| element.id == identifier)
}

fn require_parent(
    workspace: &Workspace,
    element: &Element,
    kind: ElementKind,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(
        element
            .parent
            .as_deref()
            .and_then(|parent| find(workspace, parent)),
        Some(parent) if parent.kind == kind
    ) {
        diagnostics.push(
            Diagnostic::error(
                element.span,
                format!("{label} '{}' has invalid parent", element.id),
            )
            .with_help(format!(
                "define this {label} inside the required parent block"
            )),
        );
    }
}

fn require_kind(
    workspace: &Workspace,
    view: &View,
    kind: ElementKind,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let result = view
        .scope
        .as_deref()
        .and_then(|scope| find(workspace, scope));
    match result {
        Some(element) if element.kind == kind => {}
        Some(element) => diagnostics.push(
            Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!(
                    "{label} scope '{}' has wrong kind {:?}",
                    element.id, element.kind
                ),
            )
            .with_help("use a software system identifier as the view scope"),
        ),
        None => diagnostics.push(
            Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!("{label} scope is missing or undefined"),
            )
            .with_help("define the software system before this view"),
        ),
    }
}

fn node_id(identifier: &str) -> String {
    identifier.replace(['.', '-'], "_")
}

fn escape(value: &str) -> String {
    value.replace('"', "'").replace('|', "/")
}

fn kind_label(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::SoftwareSystem => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM_CONTEXT: &str = "flowchart LR\n  customer[\"Customer\\nPerson\"]\n  bank[\"Internet Banking System\\nSoftware System\"]\n  customer -->|Uses| bank\n";
    const CONTAINER: &str = "flowchart LR\n  customer[\"Customer\\nPerson\"]\n  bank[\"Internet Banking System\\nSoftware System\"]\n  bank_web[\"Web Application\\nContainer\"]\n  bank_api[\"API Application\\nContainer\"]\n  bank_db[\"Database\\nContainer\"]\n  customer -->|Uses| bank_web\n  bank_web -->|Calls| bank_api\n  bank_api -->|Reads/writes| bank_db\n";

    fn compile_named(path: &str, source: &str) -> Result<Workspace, String> {
        let (sources, source_id) = SourceMap::from_text(path, source);
        compile_sources(sources, source_id)
    }

    #[test]
    fn preserves_m1_workspace_and_mermaid() {
        let workspace = compile(include_str!("../examples/internet-banking.dsl")).unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.elements.len(), 5);
        assert_eq!(workspace.relationships.len(), 3);
        assert_eq!(workspace.views.len(), 2);
        assert_eq!(mermaid(&workspace, &workspace.views[0]), SYSTEM_CONTEXT);
        assert_eq!(mermaid(&workspace, &workspace.views[1]), CONTAINER);
    }

    #[test]
    fn reports_duplicate_and_missing_relationship_endpoints_with_source() {
        let workspace = compile_named(
            "tests/bad.dsl",
            "workspace {\n  model {\n    p = person User\n    p = person Other\n    missing -> nowhere Uses\n  }\n}\n",
        )
        .unwrap();
        let error = validate(&workspace).unwrap_err();
        assert!(error.contains("tests/bad.dsl:4:5: error: duplicate identifier 'p'"));
        assert!(error.contains("relationship source 'missing' is not defined"));
        assert!(error.contains("relationship destination 'nowhere' is not defined"));
        assert!(error.contains("|     ^^^^^^^"));
        assert!(error.contains("help:"));
    }

    #[test]
    fn reports_unterminated_string() {
        let error = compile_named("tests/string.dsl", "workspace \"broken {\n").unwrap_err();
        assert!(error.contains("tests/string.dsl:1:11: error: unterminated string"));
        assert!(error.contains("^^^^^^^^^"));
    }

    #[test]
    fn reports_missing_closing_braces() {
        let error = compile("workspace {\n  model {\n    p = person User\n").unwrap_err();
        assert!(error.contains("missing closing '}' for model"));
        assert!(error.contains("missing closing '}' for workspace"));
    }

    #[test]
    fn accepts_case_insensitive_keywords_and_comments() {
        let workspace = compile(
            "WORKSPACE {\n  # workspace comment\n  !IDENTIFIERS HIERARCHICAL\n  MODEL {\n    // model comment\n    bank = SOFTWARESYSTEM Bank\n  }\n  VIEWS {\n    SYSTEMCONTEXT bank context {\n      INCLUDE *\n      AUTOLAYOUT LR\n    }\n  }\n}\n",
        )
        .unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.identifiers, "hierarchical");
        assert_eq!(workspace.views[0].auto_layout.as_deref(), Some("LR"));
    }

    #[test]
    fn accepts_line_continuation_and_empty_placeholder() {
        let workspace = compile(
            "workspace \\\n  Continued {\n  !identifiers hierarchical\n  model {\n    bank = softwareSystem Bank {\n      api = container API \"\" Rust\n    }\n  }\n}\n",
        )
        .unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.name.as_deref(), Some("Continued"));
        assert_eq!(workspace.elements[1].description, None);
        assert_eq!(workspace.elements[1].technology.as_deref(), Some("Rust"));
    }

    #[test]
    fn enforces_brace_line_rules() {
        let opening = compile("workspace\n{\n}\n").unwrap_err();
        assert!(opening.contains("opening '{' for workspace must be on the same line"));
        let closing = compile("workspace {\n} trailing\n").unwrap_err();
        assert!(closing.contains("closing '}' for workspace must be on a line of its own"));
    }
}
