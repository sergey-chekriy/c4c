use crate::{
    diagnostic::{render_all, Diagnostic},
    documentation, parser,
    preprocessor::{self, IncludeDependency},
    source::{SourceMap, Span},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub name: Option<String>,
    pub description: Option<String>,
    pub elements: Vec<Element>,
    pub relationships: Vec<Relationship>,
    pub views: Vec<View>,
    pub identifiers: String,
    pub identifiers_explicit: bool,
    pub extension: Option<WorkspaceExtension>,
    pub attributes: Vec<Property>,
    pub properties: Vec<Property>,
    pub view_properties: Vec<Property>,
    pub directives: Vec<Directive>,
    pub preserved: Vec<PreservedBlock>,
    pub groups: Vec<Group>,
    pub removed_relationships: Vec<RemovedRelationship>,
    pub implied_relationships: Option<String>,
    pub enterprise: Option<NamedBlock>,
    pub element_styles: Vec<ElementStyle>,
    pub relationship_styles: Vec<RelationshipStyle>,
    pub themes: Vec<ThemeReference>,
    pub branding: Option<Branding>,
    pub terminology: Vec<Property>,
    pub dependencies: Vec<IncludeDependency>,
    pub documentation: Vec<DocumentationSection>,
    pub decisions: Vec<DecisionRecord>,
    pub warnings: Vec<Diagnostic>,
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
            identifiers_explicit: false,
            extension: None,
            attributes: Vec::new(),
            properties: Vec::new(),
            view_properties: Vec::new(),
            directives: Vec::new(),
            preserved: Vec::new(),
            groups: Vec::new(),
            removed_relationships: Vec::new(),
            implied_relationships: None,
            enterprise: None,
            element_styles: Vec::new(),
            relationship_styles: Vec::new(),
            themes: Vec::new(),
            branding: None,
            terminology: Vec::new(),
            dependencies: Vec::new(),
            documentation: Vec::new(),
            decisions: Vec::new(),
            warnings: Vec::new(),
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
    pub group: Option<usize>,
    pub tags: Vec<String>,
    pub url: Option<String>,
    pub attributes: Vec<Property>,
    pub properties: Vec<Property>,
    pub perspectives: Vec<Property>,
    pub instances: Option<String>,
    pub instance_of: Option<Reference>,
    pub reference: Option<Reference>,
    pub deployment_groups: Vec<Reference>,
    pub health_checks: Vec<HealthCheck>,
    pub directives: Vec<Directive>,
    pub element_type: Option<String>,
    pub order: usize,
    pub span: Span,
    pub id_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Person,
    SoftwareSystem,
    Container,
    Component,
    Generic,
    DeploymentEnvironment,
    DeploymentGroup,
    DeploymentNode,
    InfrastructureNode,
    SoftwareSystemInstance,
    ContainerInstance,
    ArchiMate(&'static str),
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub id: Option<String>,
    pub source: String,
    pub destination: String,
    pub description: Option<String>,
    pub technology: Option<String>,
    pub tags: Vec<String>,
    pub url: Option<String>,
    pub attributes: Vec<Property>,
    pub properties: Vec<Property>,
    pub perspectives: Vec<Property>,
    pub order: usize,
    pub span: Span,
    pub id_span: Option<Span>,
    pub source_span: Span,
    pub destination_span: Span,
}

#[derive(Debug, Clone)]
pub struct View {
    pub kind: ViewKind,
    pub scope: Option<String>,
    pub key: Option<String>,
    pub description: Option<String>,
    pub includes: Vec<ViewSelector>,
    pub excludes: Vec<ViewSelector>,
    pub auto_layout: Option<AutoLayout>,
    pub title: Option<String>,
    pub is_default: bool,
    pub animations: Vec<AnimationStep>,
    pub properties: Vec<Property>,
    pub filter: Option<ViewFilter>,
    pub environment: Option<Reference>,
    pub dynamic_relationships: Vec<DynamicRelationship>,
    pub image_sources: Vec<ImageSource>,
    pub order: usize,
    pub span: Span,
    pub scope_span: Option<Span>,
    pub key_span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewKind {
    SystemLandscape,
    SystemContext,
    Container,
    Component,
    Filtered,
    Dynamic,
    Deployment,
    Custom,
    Image,
    ArchiMate,
}

#[derive(Debug, Clone)]
pub struct ViewSelector {
    pub value: String,
    pub span: Span,
    pub expression: bool,
}

#[derive(Debug, Clone)]
pub struct AutoLayout {
    pub direction: String,
    pub rank_separation: Option<String>,
    pub node_separation: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AnimationStep {
    pub elements: Vec<Reference>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterMode {
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct ViewFilter {
    pub base_key: Reference,
    pub mode: FilterMode,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DynamicRelationship {
    pub sequence: Option<String>,
    pub source: Option<Reference>,
    pub destination: Option<Reference>,
    pub relationship: Option<Reference>,
    pub description: Option<String>,
    pub technology: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImageSource {
    pub kind: String,
    pub arguments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub identifier: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Directive {
    pub name: String,
    pub arguments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentationOwner {
    Workspace,
    Element(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentationFormat {
    Markdown,
    AsciiDoc,
}

#[derive(Debug, Clone)]
pub struct DocumentationSection {
    pub owner: DocumentationOwner,
    pub source_path: PathBuf,
    pub title: String,
    pub format: DocumentationFormat,
    pub order: usize,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionFormat {
    AdrTools,
    Madr,
    Log4brains,
    MarkdownGeneric,
}

#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub owner: DocumentationOwner,
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub date: Option<String>,
    pub source_path: PathBuf,
    pub content: String,
    pub format: DecisionFormat,
    pub order: usize,
}

#[derive(Debug, Clone)]
pub struct PreservedBlock {
    pub name: String,
    pub arguments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WorkspaceExtension {
    pub target: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NamedBlock {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub parent: Option<usize>,
    pub owner: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub url: String,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RemovedRelationship {
    pub source: String,
    pub destination: String,
    pub description: Option<String>,
    pub span: Span,
    pub source_span: Span,
    pub destination_span: Span,
    pub order: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleMode {
    Default,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub struct ElementStyle {
    pub tag: String,
    pub mode: StyleMode,
    pub values: Vec<Property>,
    pub properties: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RelationshipStyle {
    pub tag: String,
    pub mode: StyleMode,
    pub values: Vec<Property>,
    pub properties: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ThemeReference {
    pub source: String,
    pub mode: StyleMode,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Branding {
    pub logo: Option<Property>,
    pub fonts: Vec<BrandingFont>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BrandingFont {
    pub name: String,
    pub location: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    pub allow_network: bool,
    pub strict_safe: bool,
    pub strict: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiMateElementType {
    pub keyword: &'static str,
    pub native: &'static str,
    pub folder: &'static str,
    pub layer: &'static str,
    pub role: &'static str,
    pub junction_kind: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiMateRelationshipType {
    pub keyword: &'static str,
    pub native: &'static str,
    pub open_group: &'static str,
    pub family: &'static str,
}

const fn element_type(
    keyword: &'static str,
    native: &'static str,
    folder: &'static str,
    role: &'static str,
) -> ArchiMateElementType {
    element_type_with_layer(keyword, native, folder, folder, role)
}

const fn element_type_with_layer(
    keyword: &'static str,
    native: &'static str,
    folder: &'static str,
    layer: &'static str,
    role: &'static str,
) -> ArchiMateElementType {
    ArchiMateElementType {
        keyword,
        native,
        folder,
        layer,
        role,
        junction_kind: None,
    }
}

const fn junction_type(
    keyword: &'static str,
    junction_kind: Option<&'static str>,
) -> ArchiMateElementType {
    ArchiMateElementType {
        keyword,
        native: "Junction",
        folder: "other",
        layer: "other",
        role: "junction",
        junction_kind,
    }
}

pub const ARCHIMATE_ELEMENT_TYPES: &[ArchiMateElementType] = &[
    element_type("resource", "Resource", "strategy", "active"),
    element_type("capability", "Capability", "strategy", "behavior"),
    element_type("valueStream", "ValueStream", "strategy", "behavior"),
    element_type("courseOfAction", "CourseOfAction", "strategy", "behavior"),
    element_type("businessActor", "BusinessActor", "business", "active"),
    element_type("businessRole", "BusinessRole", "business", "active"),
    element_type(
        "businessCollaboration",
        "BusinessCollaboration",
        "business",
        "active",
    ),
    element_type(
        "businessInterface",
        "BusinessInterface",
        "business",
        "active",
    ),
    element_type("businessProcess", "BusinessProcess", "business", "behavior"),
    element_type(
        "businessFunction",
        "BusinessFunction",
        "business",
        "behavior",
    ),
    element_type(
        "businessInteraction",
        "BusinessInteraction",
        "business",
        "behavior",
    ),
    element_type("businessEvent", "BusinessEvent", "business", "behavior"),
    element_type("businessService", "BusinessService", "business", "behavior"),
    element_type("businessObject", "BusinessObject", "business", "passive"),
    element_type("contract", "Contract", "business", "passive"),
    element_type("representation", "Representation", "business", "passive"),
    element_type("product", "Product", "business", "passive"),
    element_type(
        "applicationComponent",
        "ApplicationComponent",
        "application",
        "active",
    ),
    element_type(
        "applicationCollaboration",
        "ApplicationCollaboration",
        "application",
        "active",
    ),
    element_type(
        "applicationInterface",
        "ApplicationInterface",
        "application",
        "active",
    ),
    element_type(
        "applicationFunction",
        "ApplicationFunction",
        "application",
        "behavior",
    ),
    element_type(
        "applicationInteraction",
        "ApplicationInteraction",
        "application",
        "behavior",
    ),
    element_type(
        "applicationProcess",
        "ApplicationProcess",
        "application",
        "behavior",
    ),
    element_type(
        "applicationEvent",
        "ApplicationEvent",
        "application",
        "behavior",
    ),
    element_type(
        "applicationService",
        "ApplicationService",
        "application",
        "behavior",
    ),
    element_type("dataObject", "DataObject", "application", "passive"),
    element_type("node", "Node", "technology", "active"),
    element_type("device", "Device", "technology", "active"),
    element_type("systemSoftware", "SystemSoftware", "technology", "active"),
    element_type(
        "technologyCollaboration",
        "TechnologyCollaboration",
        "technology",
        "active",
    ),
    element_type(
        "technologyInterface",
        "TechnologyInterface",
        "technology",
        "active",
    ),
    element_type("path", "Path", "technology", "active"),
    element_type(
        "communicationNetwork",
        "CommunicationNetwork",
        "technology",
        "active",
    ),
    element_type(
        "technologyFunction",
        "TechnologyFunction",
        "technology",
        "behavior",
    ),
    element_type(
        "technologyProcess",
        "TechnologyProcess",
        "technology",
        "behavior",
    ),
    element_type(
        "technologyInteraction",
        "TechnologyInteraction",
        "technology",
        "behavior",
    ),
    element_type(
        "technologyEvent",
        "TechnologyEvent",
        "technology",
        "behavior",
    ),
    element_type(
        "technologyService",
        "TechnologyService",
        "technology",
        "behavior",
    ),
    element_type("artifact", "Artifact", "technology", "passive"),
    element_type_with_layer("equipment", "Equipment", "technology", "physical", "active"),
    element_type_with_layer("facility", "Facility", "technology", "physical", "active"),
    element_type_with_layer(
        "distributionNetwork",
        "DistributionNetwork",
        "technology",
        "physical",
        "active",
    ),
    element_type_with_layer("material", "Material", "technology", "physical", "passive"),
    element_type("stakeholder", "Stakeholder", "motivation", "motivation"),
    element_type("driver", "Driver", "motivation", "motivation"),
    element_type("assessment", "Assessment", "motivation", "motivation"),
    element_type("goal", "Goal", "motivation", "motivation"),
    element_type("outcome", "Outcome", "motivation", "motivation"),
    element_type("principle", "Principle", "motivation", "motivation"),
    element_type("requirement", "Requirement", "motivation", "motivation"),
    element_type("constraint", "Constraint", "motivation", "motivation"),
    element_type("meaning", "Meaning", "motivation", "motivation"),
    element_type("value", "Value", "motivation", "motivation"),
    element_type(
        "workPackage",
        "WorkPackage",
        "implementation_migration",
        "implementation",
    ),
    element_type(
        "deliverable",
        "Deliverable",
        "implementation_migration",
        "implementation",
    ),
    element_type(
        "implementationEvent",
        "ImplementationEvent",
        "implementation_migration",
        "implementation",
    ),
    element_type(
        "plateau",
        "Plateau",
        "implementation_migration",
        "implementation",
    ),
    element_type("gap", "Gap", "implementation_migration", "implementation"),
    element_type("grouping", "Grouping", "other", "composite"),
    element_type("location", "Location", "other", "composite"),
    junction_type("junction", None),
    junction_type("andJunction", Some("and")),
    junction_type("orJunction", Some("or")),
];

pub const ARCHIMATE_RELATIONSHIP_TYPES: &[ArchiMateRelationshipType] = &[
    ArchiMateRelationshipType {
        keyword: "composition",
        native: "CompositionRelationship",
        open_group: "Composition",
        family: "structural",
    },
    ArchiMateRelationshipType {
        keyword: "aggregation",
        native: "AggregationRelationship",
        open_group: "Aggregation",
        family: "structural",
    },
    ArchiMateRelationshipType {
        keyword: "assignment",
        native: "AssignmentRelationship",
        open_group: "Assignment",
        family: "structural",
    },
    ArchiMateRelationshipType {
        keyword: "realization",
        native: "RealizationRelationship",
        open_group: "Realization",
        family: "dependency",
    },
    ArchiMateRelationshipType {
        keyword: "serving",
        native: "ServingRelationship",
        open_group: "Serving",
        family: "dependency",
    },
    ArchiMateRelationshipType {
        keyword: "access",
        native: "AccessRelationship",
        open_group: "Access",
        family: "dependency",
    },
    ArchiMateRelationshipType {
        keyword: "influence",
        native: "InfluenceRelationship",
        open_group: "Influence",
        family: "dependency",
    },
    ArchiMateRelationshipType {
        keyword: "triggering",
        native: "TriggeringRelationship",
        open_group: "Triggering",
        family: "dynamic",
    },
    ArchiMateRelationshipType {
        keyword: "flow",
        native: "FlowRelationship",
        open_group: "Flow",
        family: "dynamic",
    },
    ArchiMateRelationshipType {
        keyword: "specialization",
        native: "SpecializationRelationship",
        open_group: "Specialization",
        family: "other",
    },
    ArchiMateRelationshipType {
        keyword: "association",
        native: "AssociationRelationship",
        open_group: "Association",
        family: "other",
    },
];

pub fn archimate_element_kind(keyword: &str) -> Option<ElementKind> {
    archimate_element_type_by_keyword(keyword).map(|element| ElementKind::ArchiMate(element.native))
}

pub fn archimate_element_keyword(native_type: &str) -> Option<&'static str> {
    archimate_element_type_by_native(native_type).map(|element| element.keyword)
}

pub fn archimate_element_folder(native_type: &str) -> Option<&'static str> {
    archimate_element_type_by_native(native_type).map(|element| element.folder)
}

pub fn archimate_element_layer(native_type: &str) -> Option<&'static str> {
    archimate_element_type_by_native(native_type).map(|element| element.layer)
}

pub fn archimate_element_role(native_type: &str) -> Option<&'static str> {
    archimate_element_type_by_native(native_type).map(|element| element.role)
}

pub fn archimate_element_default_color(native_type: &str) -> Option<&'static str> {
    archimate_element_type_by_native(native_type)
        .map(|element| archimate_default_color_for_folder(element.folder))
}

pub fn archimate_element_type_by_keyword(keyword: &str) -> Option<&'static ArchiMateElementType> {
    ARCHIMATE_ELEMENT_TYPES
        .iter()
        .find(|element| keyword.eq_ignore_ascii_case(element.keyword))
}

pub fn archimate_element_type_by_native(
    native_type: &str,
) -> Option<&'static ArchiMateElementType> {
    ARCHIMATE_ELEMENT_TYPES
        .iter()
        .find(|element| native_type.eq_ignore_ascii_case(element.native))
}

fn archimate_default_color_for_folder(folder: &str) -> &'static str {
    match folder {
        "business" => "#ffffb5",
        "technology" => "#c9d9ff",
        "motivation" => "#ccccff",
        "strategy" => "#f5deaa",
        "implementation_migration" => "#ffe0e0",
        "other" => "#eeeeee",
        _ => "#b5ffff",
    }
}

pub fn archimate_relationship_type(value: &str) -> Option<&'static str> {
    let compact = value
        .trim()
        .strip_prefix("archimate:")
        .unwrap_or(value.trim())
        .trim_end_matches("Relationship");
    ARCHIMATE_RELATIONSHIP_TYPES
        .iter()
        .find(|relationship| {
            compact.eq_ignore_ascii_case(relationship.keyword)
                || value.eq_ignore_ascii_case(relationship.native)
                || value.eq_ignore_ascii_case(&relationship.keyword.to_ascii_lowercase())
        })
        .map(|relationship| relationship.native)
}

pub fn archimate_relationship_info(value: &str) -> Option<&'static ArchiMateRelationshipType> {
    archimate_relationship_type(value).and_then(|native| {
        ARCHIMATE_RELATIONSHIP_TYPES
            .iter()
            .find(|relationship| relationship.native == native)
    })
}

pub fn relationship_archimate_type(relationship: &Relationship) -> Option<&str> {
    relationship
        .attributes
        .iter()
        .rev()
        .find(|property| property.key == "type")
        .map(|property| property.value.as_str())
        .or_else(|| {
            relationship
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("archimate:"))
        })
        .and_then(archimate_relationship_type)
}

pub fn relationship_access_direction(relationship: &Relationship) -> Option<&str> {
    relationship
        .attributes
        .iter()
        .rev()
        .find(|property| property.key == "access")
        .map(|property| property.value.as_str())
}

pub fn archimate_relationship_open_group_type(native_type: &str) -> Option<&'static str> {
    archimate_relationship_info(native_type).map(|relationship| relationship.open_group)
}

pub fn compile_file(path: &str) -> Result<Workspace, String> {
    compile_file_with_options(path, CompileOptions::default())
}

pub fn compile_file_with_options(path: &str, options: CompileOptions) -> Result<Workspace, String> {
    let mut sources = SourceMap::new();
    let mut stack = Vec::new();
    let mut workspace = compile_path(Path::new(path), &mut sources, options, &mut stack)?;
    workspace.source_map = sources;
    documentation::import(&mut workspace, options.strict_safe)?;
    apply_archimate_semantic_warnings(&mut workspace);
    if options.strict_safe {
        enforce_strict_safe(&workspace)?;
    }
    if options.strict {
        validate_with_options(
            &workspace,
            ValidationOptions {
                strict_archimate: true,
            },
        )?;
    }
    Ok(workspace)
}

#[cfg(test)]
pub fn compile(source: &str) -> Result<Workspace, String> {
    let mut sources = SourceMap::new();
    let preprocessed = preprocessor::text(Path::new("<memory>"), source, &mut sources, false)?;
    let mut workspace = compile_sources(&sources, preprocessed.source_id)?;
    workspace.dependencies = preprocessed.dependencies;
    workspace.warnings.extend(preprocessed.warnings);
    workspace.source_map = sources;
    apply_archimate_semantic_warnings(&mut workspace);
    Ok(workspace)
}

fn compile_sources(
    sources: &SourceMap,
    source_id: crate::source::SourceId,
) -> Result<Workspace, String> {
    compile_sources_with_identifiers(sources, source_id, "flat")
}

fn compile_sources_with_identifiers(
    sources: &SourceMap,
    source_id: crate::source::SourceId,
    identifiers: &str,
) -> Result<Workspace, String> {
    parser::parse(sources, source_id, identifiers)
}

fn compile_path(
    path: &Path,
    sources: &mut SourceMap,
    options: CompileOptions,
    stack: &mut Vec<PathBuf>,
) -> Result<Workspace, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if stack.contains(&canonical) {
        return Err(format!(
            "workspace extension cycle detected at {}",
            canonical.display()
        ));
    }
    stack.push(canonical.clone());
    let preprocessed = preprocessor::file(
        &canonical,
        sources,
        options.allow_network,
        options.strict_safe,
    )?;
    let source_id = preprocessed.source_id;
    let mut derived = compile_sources(sources, source_id)?;
    derived.dependencies.extend(preprocessed.dependencies);
    derived.warnings.extend(preprocessed.warnings);
    let result = if let Some(extension) = derived.extension.clone() {
        if is_url(&extension.target) {
            let message = if options.allow_network {
                "remote workspace extension is not implemented in M3; no network request was made"
            } else {
                "remote workspace extension is disabled; pass --allow-network when support is added"
            };
            Err(Diagnostic::error(extension.span, message)
                .with_help("extend a local .dsl file in M3")
                .render(sources))
        } else if extension.target.ends_with(".json") {
            Err(Diagnostic::error(
                extension.span,
                "JSON workspace extension is not implemented in M3",
            )
            .with_help("extend a local Structurizr DSL file")
            .render(sources))
        } else {
            let base_path = canonical
                .parent()
                .unwrap_or(Path::new("."))
                .join(&extension.target);
            let base = compile_path(&base_path, sources, options, stack)?;
            if !derived.identifiers_explicit && derived.identifiers != base.identifiers {
                derived = compile_sources_with_identifiers(sources, source_id, &base.identifiers)?;
            }
            Ok(merge_workspaces(base, derived, sources.clone()))
        }
    } else {
        Ok(derived)
    };
    stack.pop();
    result
}

fn merge_workspaces(mut base: Workspace, mut derived: Workspace, sources: SourceMap) -> Workspace {
    let order_offset = base
        .elements
        .iter()
        .map(|element| element.order)
        .chain(
            base.relationships
                .iter()
                .map(|relationship| relationship.order),
        )
        .chain(
            base.removed_relationships
                .iter()
                .map(|relationship| relationship.order),
        )
        .chain(base.views.iter().map(|view| view.order))
        .max()
        .map_or(0, |order| order + 1);
    for element in &mut derived.elements {
        element.order += order_offset;
    }
    for relationship in &mut derived.relationships {
        relationship.order += order_offset;
    }
    for relationship in &mut derived.removed_relationships {
        relationship.order += order_offset;
    }
    for view in &mut derived.views {
        view.order += order_offset;
    }
    let group_offset = base.groups.len();
    for element in &mut derived.elements {
        if let Some(group) = &mut element.group {
            *group += group_offset;
        }
    }
    for group in &mut derived.groups {
        if let Some(parent) = &mut group.parent {
            *parent += group_offset;
        }
    }

    base.name = derived.name.or(base.name);
    base.description = derived.description.or(base.description);
    base.identifiers = derived.identifiers;
    base.identifiers_explicit = derived.identifiers_explicit;
    base.extension = derived.extension;
    base.implied_relationships = derived.implied_relationships.or(base.implied_relationships);
    base.enterprise = derived.enterprise.or(base.enterprise);
    base.branding = derived.branding.or(base.branding);
    base.span = derived.span;
    base.source_map = sources;
    base.elements.extend(derived.elements);
    base.relationships.extend(derived.relationships);
    base.removed_relationships
        .extend(derived.removed_relationships);
    base.views.extend(derived.views);
    base.attributes.extend(derived.attributes);
    base.properties.extend(derived.properties);
    base.view_properties.extend(derived.view_properties);
    base.directives.extend(derived.directives);
    base.preserved.extend(derived.preserved);
    base.groups.extend(derived.groups);
    base.element_styles.extend(derived.element_styles);
    base.relationship_styles.extend(derived.relationship_styles);
    base.themes.extend(derived.themes);
    base.terminology.extend(derived.terminology);
    base.dependencies.extend(derived.dependencies);
    base.documentation.extend(derived.documentation);
    base.decisions.extend(derived.decisions);
    base.warnings.extend(derived.warnings);
    base
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn enforce_strict_safe(workspace: &Workspace) -> Result<(), String> {
    let mut diagnostics = Vec::new();
    for theme in &workspace.themes {
        if is_url(&theme.source) {
            diagnostics.push(Diagnostic::error(
                theme.span,
                "strict-safe mode rejects remote theme references",
            ));
        }
    }
    for style in &workspace.element_styles {
        for property in &style.values {
            if property.key == "icon" && is_url(&property.value) {
                diagnostics.push(Diagnostic::error(
                    property.span,
                    "strict-safe mode rejects remote icon references",
                ));
            }
        }
    }
    if let Some(branding) = &workspace.branding {
        if let Some(logo) = &branding.logo {
            if is_url(&logo.value) {
                diagnostics.push(Diagnostic::error(
                    logo.span,
                    "strict-safe mode rejects remote branding logos",
                ));
            }
        }
        for font in &branding.fonts {
            if font.location.as_deref().is_some_and(is_url) {
                diagnostics.push(Diagnostic::error(
                    font.span,
                    "strict-safe mode rejects remote branding fonts",
                ));
            }
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(render_all(&diagnostics, &workspace.source_map))
    }
}

fn apply_archimate_semantic_warnings(workspace: &mut Workspace) {
    let warnings = archimate_semantic_diagnostics(workspace, false);
    workspace.warnings.extend(warnings);
}

pub fn warnings(workspace: &Workspace) -> Option<String> {
    (!workspace.warnings.is_empty()).then(|| render_all(&workspace.warnings, &workspace.source_map))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidationOptions {
    pub strict_archimate: bool,
}

pub fn validate(workspace: &Workspace) -> Result<(), String> {
    validate_with_options(workspace, ValidationOptions::default())
}

pub fn validate_with_options(
    workspace: &Workspace,
    options: ValidationOptions,
) -> Result<(), String> {
    let mut diagnostics = Vec::new();
    let mut identifiers = HashMap::new();
    validate_property_spans(&workspace.attributes);
    for element in &workspace.elements {
        validate_property_spans(&element.attributes);
        if let Some(original) = identifiers.insert(element.id.as_str(), element) {
            let source = workspace.source_map.get(original.id_span.source_id);
            let (line, column) = source.line_column(original.id_span.start);
            diagnostics.push(
                Diagnostic::error(
                    element.id_span,
                    format!("duplicate identifier '{}'", element.id),
                )
                .with_help(format!(
                    "rename this element; '{}' was first defined at {}:{line}:{column}",
                    original.id,
                    source.path.display()
                )),
            );
        }
        validate_parent(workspace, element, &mut diagnostics);
        validate_group(workspace, element, &mut diagnostics);
        validate_element_references(workspace, element, &mut diagnostics);
    }
    validate_groups(workspace, &mut diagnostics);
    let mut relationship_ids = HashMap::new();
    for relationship in &workspace.relationships {
        validate_property_spans(&relationship.attributes);
        if let Some(id) = relationship.id.as_deref() {
            if let Some(original) = relationship_ids.insert(id, relationship) {
                let source = workspace
                    .source_map
                    .get(original.id_span.unwrap().source_id);
                let (line, column) = source.line_column(original.id_span.unwrap().start);
                diagnostics.push(
                    Diagnostic::error(
                        relationship.id_span.unwrap_or(relationship.span),
                        format!("duplicate relationship identifier '{id}'"),
                    )
                    .with_help(format!(
                        "rename this relationship; '{id}' was first defined at {}:{line}:{column}",
                        source.path.display()
                    )),
                );
            }
        }
        debug_assert!(relationship.span.start <= relationship.source_span.start);
        debug_assert!(relationship.span.end >= relationship.destination_span.end);
        validate_prior_reference(
            workspace,
            &relationship.source,
            relationship.source_span,
            relationship.order,
            "relationship source",
            &mut diagnostics,
        );
        validate_prior_reference(
            workspace,
            &relationship.destination,
            relationship.destination_span,
            relationship.order,
            "relationship destination",
            &mut diagnostics,
        );
    }
    for removed in &workspace.removed_relationships {
        validate_prior_reference(
            workspace,
            &removed.source,
            removed.source_span,
            removed.order,
            "removed relationship source",
            &mut diagnostics,
        );
        validate_prior_reference(
            workspace,
            &removed.destination,
            removed.destination_span,
            removed.order,
            "removed relationship destination",
            &mut diagnostics,
        );
        let matches = workspace.relationships.iter().any(|relationship| {
            relationship.order < removed.order
                && relationship.source == removed.source
                && relationship.destination == removed.destination
                && removed.description.as_ref().is_none_or(|description| {
                    relationship.description.as_ref() == Some(description)
                })
        });
        if !matches {
            diagnostics.push(
                Diagnostic::error(
                    removed.span,
                    format!(
                        "relationship removal '{} -/> {}' has no earlier matching relationship",
                        removed.source, removed.destination
                    ),
                )
                .with_help("define the relationship before removing it, and check its description"),
            );
        }
    }
    let mut view_keys = HashMap::new();
    for view in &workspace.views {
        validate_property_spans(&view.properties);
        if let Some(key) = &view.key {
            if let Some(original) = view_keys.insert(key.as_str(), view) {
                diagnostics.push(
                    Diagnostic::error(
                        view.key_span.unwrap_or(view.span),
                        format!("duplicate view key '{key}'"),
                    )
                    .with_help(format!(
                        "rename this view; the key was first used by a {:?} view",
                        original.kind
                    )),
                );
            }
        }
        validate_view_options(workspace, view, &mut diagnostics);
        match view.kind {
            ViewKind::SystemLandscape => {}
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
            ViewKind::Component => require_kind(
                workspace,
                view,
                ElementKind::Container,
                "component view",
                &mut diagnostics,
            ),
            ViewKind::Filtered => validate_filtered_view(workspace, view, &mut diagnostics),
            ViewKind::Dynamic => validate_dynamic_view(workspace, view, &mut diagnostics),
            ViewKind::Deployment => validate_deployment_view(workspace, view, &mut diagnostics),
            ViewKind::Custom => validate_custom_view(workspace, view, &mut diagnostics),
            ViewKind::Image => validate_image_view(workspace, view, &mut diagnostics),
            ViewKind::ArchiMate => validate_archimate_view(workspace, view, &mut diagnostics),
        }
    }
    validate_archimate_metadata(workspace, &mut diagnostics);
    if options.strict_archimate {
        diagnostics.extend(archimate_semantic_diagnostics(workspace, true));
    }
    validate_styles(workspace, &mut diagnostics);
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(render_all(&diagnostics, &workspace.source_map))
    }
}

fn validate_styles(workspace: &Workspace, diagnostics: &mut Vec<Diagnostic>) {
    const SHAPES: &[&str] = &[
        "box",
        "roundedbox",
        "circle",
        "ellipse",
        "hexagon",
        "diamond",
        "cylinder",
        "bucket",
        "pipe",
        "person",
        "robot",
        "folder",
        "webbrowser",
        "window",
        "terminal",
        "shell",
        "mobiledeviceportrait",
        "mobiledevicelandscape",
        "component",
    ];
    for style in &workspace.element_styles {
        debug_assert!(style.span.end >= style.span.start);
        validate_property_spans(&style.properties);
        for property in &style.values {
            match property.key.as_str() {
                "shape" => validate_enum(property, SHAPES, "shape", diagnostics),
                "border" => validate_enum(
                    property,
                    &["solid", "dashed", "dotted"],
                    "border",
                    diagnostics,
                ),
                "metadata" | "description" => {
                    validate_enum(property, &["true", "false"], "boolean", diagnostics)
                }
                "strokeWidth" => validate_integer(property, Some((1, 10)), diagnostics),
                "opacity" => validate_integer(property, Some((0, 100)), diagnostics),
                "width" | "height" | "fontSize" => validate_integer(property, None, diagnostics),
                _ => {}
            }
        }
    }
    for style in &workspace.relationship_styles {
        debug_assert!(style.span.end >= style.span.start);
        validate_property_spans(&style.properties);
        for property in &style.values {
            match property.key.as_str() {
                "style" => validate_enum(
                    property,
                    &["solid", "dashed", "dotted"],
                    "relationship style",
                    diagnostics,
                ),
                "routing" => validate_enum(
                    property,
                    &["direct", "orthogonal", "curved"],
                    "routing",
                    diagnostics,
                ),
                "jump" => validate_enum(property, &["true", "false"], "boolean", diagnostics),
                "position" | "opacity" => validate_integer(property, Some((0, 100)), diagnostics),
                "thickness" | "fontSize" | "width" => validate_integer(property, None, diagnostics),
                _ => {}
            }
        }
    }
    validate_property_spans(&workspace.terminology);
    if let Some(metadata) = workspace
        .terminology
        .iter()
        .rev()
        .find(|property| property.key == "metadata")
    {
        validate_enum(
            metadata,
            &["square", "round", "curly", "angle", "double-angle", "none"],
            "metadata mode",
            diagnostics,
        );
    }
}

fn validate_enum(
    property: &Property,
    allowed: &[&str],
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !allowed
        .iter()
        .any(|value| property.value.eq_ignore_ascii_case(value))
    {
        diagnostics.push(
            Diagnostic::error(
                property.span,
                format!("invalid {label} '{}'", property.value),
            )
            .with_help(format!("use one of: {}", allowed.join(", "))),
        );
    }
}

fn validate_integer(
    property: &Property,
    range: Option<(i32, i32)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match property.value.parse::<i32>() {
        Ok(value)
            if range.is_none_or(|(minimum, maximum)| (minimum..=maximum).contains(&value)) => {}
        Ok(_) => {
            let (minimum, maximum) = range.unwrap();
            diagnostics.push(
                Diagnostic::error(
                    property.span,
                    format!("{} must be between {minimum} and {maximum}", property.key),
                )
                .with_help("use an integer inside the documented range"),
            );
        }
        Err(_) => diagnostics.push(
            Diagnostic::error(
                property.span,
                format!("{} must be an integer", property.key),
            )
            .with_help("replace this value with an integer"),
        ),
    }
}

fn validate_view_options(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(layout) = &view.auto_layout {
        if !["tb", "bt", "lr", "rl"]
            .iter()
            .any(|direction| layout.direction.eq_ignore_ascii_case(direction))
        {
            diagnostics.push(
                Diagnostic::error(
                    layout.span,
                    format!("invalid autoLayout direction '{}'", layout.direction),
                )
                .with_help("use tb, bt, lr, or rl"),
            );
        }
    }
    for selector in view.includes.iter().chain(&view.excludes) {
        if selector.value == "*" || selector.value == "*?" {
            continue;
        }
        if selector.expression {
            if let Some((source, destination)) = relationship_pattern(&selector.value) {
                for identifier in [source, destination] {
                    if identifier != "*" && find(workspace, identifier).is_none() {
                        diagnostics.push(Diagnostic::error(
                            selector.span,
                            format!("view relationship expression references undefined element '{identifier}'"),
                        ));
                    }
                }
            } else if expression_candidate(&selector.value) {
                if let Err(message) = parse_view_expression(&selector.value) {
                    diagnostics.push(
                        Diagnostic::error(
                            selector.span,
                            format!("unsupported view expression: {message}"),
                        )
                        .with_help(
                            "use tag/type/property comparisons with !, &&, ||, and parentheses",
                        ),
                    );
                }
            }
            continue;
        }
        if find(workspace, &selector.value).is_none() {
            diagnostics.push(
                Diagnostic::error(
                    selector.span,
                    format!("view selector '{}' is not defined", selector.value),
                )
                .with_help("use an existing element identifier, wildcard, or supported expression"),
            );
        }
    }
    for step in &view.animations {
        debug_assert!(step.span.end >= step.span.start);
        for reference in &step.elements {
            if find(workspace, &reference.identifier).is_none() {
                diagnostics.push(Diagnostic::error(
                    reference.span,
                    format!(
                        "animation element '{}' is not defined",
                        reference.identifier
                    ),
                ));
            }
        }
    }
}

fn validate_filtered_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    let Some(filter) = &view.filter else {
        diagnostics.push(Diagnostic::error(
            view.span,
            "filtered view configuration is missing",
        ));
        return;
    };
    match workspace
        .views
        .iter()
        .find(|candidate| candidate.key.as_deref() == Some(&filter.base_key.identifier))
    {
        Some(base)
            if matches!(
                base.kind,
                ViewKind::SystemLandscape
                    | ViewKind::SystemContext
                    | ViewKind::Container
                    | ViewKind::Component
                    | ViewKind::ArchiMate
            ) => {}
        Some(base) => diagnostics.push(
            Diagnostic::error(
                filter.base_key.span,
                format!(
                    "filtered view base '{}' has unsupported type {:?}",
                    filter.base_key.identifier, base.kind
                ),
            )
            .with_help("base a filtered view on a static view"),
        ),
        None => diagnostics.push(
            Diagnostic::error(
                filter.base_key.span,
                format!(
                    "filtered view base '{}' is not defined",
                    filter.base_key.identifier
                ),
            )
            .with_help("define a keyed static view before referencing it"),
        ),
    }
}

fn validate_dynamic_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    let scope = view.scope.as_deref();
    if scope != Some("*") {
        let target = scope.and_then(|identifier| find(workspace, identifier));
        if !matches!(
            target.map(|element| &element.kind),
            Some(ElementKind::SoftwareSystem | ElementKind::Container)
        ) {
            diagnostics.push(
                Diagnostic::error(
                    view.scope_span.unwrap_or(view.span),
                    "dynamic view scope must be '*', a software system, or a container",
                )
                .with_help("use a supported dynamic view scope"),
            );
        }
    }
    for instance in &view.dynamic_relationships {
        if let Some(reference) = &instance.relationship {
            diagnostics.push(
                Diagnostic::error(
                    reference.span,
                    format!(
                        "dynamic relationship identifier '{}' cannot be resolved",
                        reference.identifier
                    ),
                )
                .with_help(
                    "relationship identifiers are not supported; use 'source -> destination'",
                ),
            );
        }
        for reference in [instance.source.as_ref(), instance.destination.as_ref()]
            .into_iter()
            .flatten()
        {
            match find(workspace, &reference.identifier) {
                Some(element) if dynamic_element_allowed(workspace, scope, element) => {}
                Some(_) => diagnostics.push(
                    Diagnostic::error(
                        reference.span,
                        format!(
                            "dynamic element '{}' is outside the view scope",
                            reference.identifier
                        ),
                    )
                    .with_help("use an element permitted by this dynamic view scope"),
                ),
                None => diagnostics.push(Diagnostic::error(
                    reference.span,
                    format!("dynamic element '{}' is not defined", reference.identifier),
                )),
            }
        }
        if let (Some(source), Some(destination)) = (&instance.source, &instance.destination) {
            let endpoints_exist = find(workspace, &source.identifier).is_some()
                && find(workspace, &destination.identifier).is_some();
            if endpoints_exist
                && !workspace.relationships.iter().any(|relationship| {
                    relationship.source == source.identifier
                        && relationship.destination == destination.identifier
                })
            {
                diagnostics.push(
                    Diagnostic::error(
                        instance.span,
                        format!(
                            "dynamic relationship '{} -> {}' is not defined in the model",
                            source.identifier, destination.identifier
                        ),
                    )
                    .with_help("define the relationship in the model before using it dynamically"),
                );
            }
        }
    }
}

fn dynamic_element_allowed(workspace: &Workspace, scope: Option<&str>, element: &Element) -> bool {
    match scope {
        Some("*") => matches!(
            element.kind,
            ElementKind::Person | ElementKind::SoftwareSystem
        ),
        Some(scope) => match find(workspace, scope).map(|item| &item.kind) {
            Some(ElementKind::SoftwareSystem) => {
                matches!(
                    element.kind,
                    ElementKind::Person | ElementKind::SoftwareSystem
                ) || (element.kind == ElementKind::Container
                    && element.parent.as_deref() == Some(scope))
            }
            Some(ElementKind::Container) => {
                matches!(
                    element.kind,
                    ElementKind::Person | ElementKind::SoftwareSystem
                ) || (element.kind == ElementKind::Component
                    && element.parent.as_deref() == Some(scope))
                    || (element.kind == ElementKind::Container
                        && software_system_of(workspace, &element.id)
                            == software_system_of(workspace, scope))
            }
            _ => false,
        },
        None => false,
    }
}

fn validate_deployment_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    if view.scope.as_deref() != Some("*") {
        require_kind(
            workspace,
            view,
            ElementKind::SoftwareSystem,
            "deployment view",
            diagnostics,
        );
    }
    let valid_environment = view.environment.as_ref().and_then(|environment| {
        workspace.elements.iter().find(|element| {
            element.kind == ElementKind::DeploymentEnvironment
                && (element.id == environment.identifier || element.name == environment.identifier)
        })
    });
    if valid_environment.is_none() {
        diagnostics.push(
            Diagnostic::error(
                view.environment
                    .as_ref()
                    .map_or(view.span, |item| item.span),
                "deployment view environment is missing or undefined",
            )
            .with_help("use an existing deployment environment identifier or name"),
        );
    }
}

fn validate_custom_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    for selector in view.includes.iter().filter(|selector| !selector.expression) {
        if let Some(element) = find(workspace, &selector.value) {
            if element.kind != ElementKind::Generic {
                diagnostics.push(
                    Diagnostic::error(
                        selector.span,
                        "custom views may include only generic elements",
                    )
                    .with_help("use elements declared with the element keyword"),
                );
            }
        }
    }
}

fn validate_image_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(scope) = view.scope.as_deref() {
        if scope != "*" && find(workspace, scope).is_none() {
            diagnostics.push(Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!("image view scope '{scope}' is not defined"),
            ));
        }
    }
}

fn validate_archimate_view(workspace: &Workspace, view: &View, diagnostics: &mut Vec<Diagnostic>) {
    let expanded = expand_view(workspace, view);
    if let Some(viewpoint) = archimate_viewpoint(view) {
        validate_enum(
            viewpoint,
            ARCHIMATE_VIEWPOINTS,
            "ArchiMate viewpoint",
            diagnostics,
        );
    }
    for (identifier, properties) in archimate_view_objects(view) {
        if !expanded.elements.contains(identifier) {
            diagnostics.push(
                Diagnostic::error(
                    properties
                        .first()
                        .map_or(view.span, |property| property.span),
                    format!("archimateView object '{identifier}' is not included in the view"),
                )
                .with_help("include the element before defining manual layout for it"),
            );
        }
        for property in properties {
            match property.key.rsplit('.').next().unwrap_or("") {
                "x" | "y" | "width" | "height" => {
                    validate_integer(property, Some((0, i32::MAX)), diagnostics)
                }
                "background" | "color" | "stroke" => validate_hex_color(property, diagnostics),
                "fontSize" => validate_integer(property, Some((0, i32::MAX)), diagnostics),
                _ => {}
            }
        }
    }
}

fn validate_archimate_metadata(workspace: &Workspace, diagnostics: &mut Vec<Diagnostic>) {
    for element in &workspace.elements {
        for property in &element.attributes {
            match property.key.as_str() {
                "background" | "color" | "stroke" => validate_hex_color(property, diagnostics),
                "fontSize" | "width" | "height" => {
                    validate_integer(property, Some((0, i32::MAX)), diagnostics)
                }
                "kind" => validate_enum(property, &["and", "or"], "junction kind", diagnostics),
                _ => {}
            }
        }
    }
    for relationship in &workspace.relationships {
        for property in &relationship.attributes {
            match property.key.as_str() {
                "type" if archimate_relationship_type(&property.value).is_none() => diagnostics.push(
                    Diagnostic::error(
                        property.span,
                        format!("unknown ArchiMate relationship type '{}'", property.value),
                    )
                    .with_help("use FlowRelationship, AccessRelationship, ServingRelationship, or another supported ArchiMate relationship type"),
                ),
                "color" => validate_hex_color(property, diagnostics),
                "thickness" => validate_integer(property, Some((0, i32::MAX)), diagnostics),
                "style" => validate_enum(property, &["solid", "dashed", "dotted"], "relationship style", diagnostics),
                "access" => validate_enum(property, &["read", "write", "readWrite", "access"], "AccessRelationship direction", diagnostics),
                _ => {}
            }
        }
    }
}

const ARCHIMATE_VIEWPOINTS: &[&str] = &[
    "introductory",
    "organization",
    "applicationCooperation",
    "applicationUsage",
    "informationStructure",
    "technology",
    "technologyUsage",
    "implementationAndDeployment",
    "motivation",
    "strategy",
    "layered",
    "physical",
    "migration",
    "implementationAndMigration",
];

fn archimate_semantic_diagnostics(workspace: &Workspace, strict: bool) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for relationship in &workspace.relationships {
        let Some(source) = find(workspace, &relationship.source) else {
            continue;
        };
        let Some(destination) = find(workspace, &relationship.destination) else {
            continue;
        };
        if !relationship_mentions_archimate(relationship, source, destination) {
            continue;
        }
        validate_archimate_relationship_semantics(
            relationship,
            source,
            destination,
            strict,
            &mut diagnostics,
        );
    }
    for view in workspace
        .views
        .iter()
        .filter(|view| view.kind == ViewKind::ArchiMate)
    {
        validate_archimate_viewpoint_semantics(workspace, view, strict, &mut diagnostics);
    }
    diagnostics
}

fn relationship_mentions_archimate(
    relationship: &Relationship,
    source: &Element,
    destination: &Element,
) -> bool {
    relationship_archimate_type(relationship).is_some()
        || matches!(source.kind, ElementKind::ArchiMate(_))
        || matches!(destination.kind, ElementKind::ArchiMate(_))
}

fn validate_archimate_relationship_semantics(
    relationship: &Relationship,
    source: &Element,
    destination: &Element,
    strict: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let kind = relationship_archimate_type(relationship).unwrap_or("AssociationRelationship");
    let source_layer = element_archimate_layer(&source.kind);
    let target_layer = element_archimate_layer(&destination.kind);
    let source_role = element_archimate_role(&source.kind);
    let target_role = element_archimate_role(&destination.kind);
    let problem = match kind {
        "AccessRelationship" if !matches!(target_role, "passive" | "junction") => Some(format!(
            "AccessRelationship from '{}' targets '{}', which is not passive structure",
            source.id, destination.id
        )),
        "AccessRelationship" if relationship_access_direction(relationship).is_none() => Some(
            "AccessRelationship has no access direction; default ArchiMate access is ambiguous"
                .to_string(),
        ),
        "AssignmentRelationship" if !matches!(source_role, "active" | "junction") => Some(format!(
            "AssignmentRelationship source '{}' is not active structure",
            source.id
        )),
        "InfluenceRelationship"
            if !matches!(source_layer, "motivation" | "strategy")
                && !matches!(target_layer, "motivation" | "strategy") =>
        {
            Some(format!(
                "InfluenceRelationship usually connects motivation or strategy elements, not '{}' to '{}'",
                source.id, destination.id
            ))
        }
        "FlowRelationship" | "TriggeringRelationship" | "ServingRelationship"
            if source_layer == "motivation" || target_layer == "motivation" =>
        {
            Some(format!(
                "{kind} between motivation and non-behavior elements is questionable"
            ))
        }
        _ => None,
    };
    if let Some(message) = problem {
        diagnostics.push(archimate_semantic_diagnostic(
            relationship.span,
            message,
            "use --strict to make ArchiMate semantic conformance issues fatal",
            strict,
        ));
    }
}

fn validate_archimate_viewpoint_semantics(
    workspace: &Workspace,
    view: &View,
    strict: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(viewpoint) = archimate_viewpoint(view) else {
        return;
    };
    let graph = expand_view(workspace, view);
    let layers = graph
        .elements
        .iter()
        .filter_map(|identifier| find(workspace, identifier))
        .map(|element| element_archimate_layer(&element.kind))
        .collect::<Vec<_>>();
    let expected = match viewpoint.value.as_str() {
        "organization" => Some("business"),
        "applicationCooperation" | "applicationUsage" | "informationStructure" => {
            Some("application")
        }
        "technology" | "technologyUsage" => Some("technology"),
        "motivation" => Some("motivation"),
        "strategy" => Some("strategy"),
        "physical" => Some("physical"),
        "implementationAndDeployment" | "implementationAndMigration" | "migration" => {
            Some("implementation_migration")
        }
        _ => None,
    };
    if let Some(layer) = expected {
        if !layers.contains(&layer) {
            diagnostics.push(archimate_semantic_diagnostic(
                viewpoint.span,
                format!(
                    "archimateView viewpoint '{}' contains no {layer} elements",
                    viewpoint.value
                ),
                "change the viewpoint or include elements from the expected ArchiMate layer",
                strict,
            ));
        }
    }
}

fn archimate_semantic_diagnostic(
    span: Span,
    message: impl Into<String>,
    help: &'static str,
    strict: bool,
) -> Diagnostic {
    if strict {
        Diagnostic::error(span, message).with_help(
            "fix the ArchiMate relationship/viewpoint or run without --strict to keep this as a warning",
        )
    } else {
        Diagnostic::warning(span, message).with_help(help)
    }
}

fn archimate_viewpoint(view: &View) -> Option<&Property> {
    view.properties
        .iter()
        .find(|property| property.key == "viewpoint")
}

fn element_archimate_layer(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "business",
        ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "application",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "technology",
        ElementKind::ArchiMate(kind) => archimate_element_layer(kind).unwrap_or("other"),
        _ => "other",
    }
}

fn element_archimate_role(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person
        | ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance
        | ElementKind::DeploymentNode
        | ElementKind::InfrastructureNode => "active",
        ElementKind::ArchiMate(kind) => archimate_element_role(kind).unwrap_or("composite"),
        _ => "composite",
    }
}

fn validate_hex_color(property: &Property, diagnostics: &mut Vec<Diagnostic>) {
    let valid = property.value.len() == 7
        && property.value.starts_with('#')
        && property.value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    if !valid {
        diagnostics.push(
            Diagnostic::error(
                property.span,
                format!("{} must be a #RRGGBB color", property.key),
            )
            .with_help("use a six-digit hexadecimal color such as #008e00"),
        );
    }
}

fn archimate_view_objects(view: &View) -> HashMap<&str, Vec<&Property>> {
    let mut objects: HashMap<&str, Vec<&Property>> = HashMap::new();
    for property in &view.properties {
        let Some(rest) = property.key.strip_prefix("object.") else {
            continue;
        };
        let Some((identifier, _)) = rest.rsplit_once('.') else {
            continue;
        };
        objects.entry(identifier).or_default().push(property);
    }
    objects
}

fn validate_property_spans(properties: &[Property]) {
    for property in properties {
        debug_assert!(property.span.end >= property.span.start);
        debug_assert!(!property.key.is_empty());
        debug_assert!(!property.value.is_empty());
    }
}

fn validate_groups(workspace: &Workspace, diagnostics: &mut Vec<Diagnostic>) {
    for (index, group) in workspace.groups.iter().enumerate() {
        let owner = group
            .owner
            .as_deref()
            .and_then(|owner| find(workspace, owner));
        let valid_owner = owner.is_none_or(|owner| {
            matches!(
                owner.kind,
                ElementKind::SoftwareSystem | ElementKind::Container
            )
        });
        let valid_parent = group
            .parent
            .is_none_or(|parent| workspace.groups[parent].owner == group.owner && parent < index);
        if !valid_owner || !valid_parent {
            diagnostics.push(
                Diagnostic::error(
                    group.span,
                    format!(
                        "group '{}' has an incompatible abstraction level",
                        group.name
                    ),
                )
                .with_help("nest groups only with groups and elements at the same C4 level"),
            );
        }
    }
}

fn validate_parent(workspace: &Workspace, element: &Element, diagnostics: &mut Vec<Diagnostic>) {
    let parent = element
        .parent
        .as_deref()
        .and_then(|identifier| find(workspace, identifier));
    let valid = match element.kind {
        ElementKind::Person
        | ElementKind::SoftwareSystem
        | ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::ArchiMate(_) => parent.is_none(),
        ElementKind::Container => {
            matches!(parent, Some(parent) if parent.kind == ElementKind::SoftwareSystem)
        }
        ElementKind::Component => {
            matches!(parent, Some(parent) if parent.kind == ElementKind::Container)
        }
        ElementKind::DeploymentGroup => {
            matches!(parent, Some(parent) if parent.kind == ElementKind::DeploymentEnvironment)
        }
        ElementKind::DeploymentNode => {
            matches!(parent, Some(parent) if matches!(parent.kind, ElementKind::DeploymentEnvironment | ElementKind::DeploymentNode))
        }
        ElementKind::InfrastructureNode
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => {
            matches!(parent, Some(parent) if parent.kind == ElementKind::DeploymentNode)
        }
    };
    if !valid {
        diagnostics.push(
            Diagnostic::error(
                element.span,
                format!(
                    "{} '{}' is not allowed in this parent",
                    element_kind_label(&element.kind),
                    element.id
                ),
            )
            .with_help(parent_help(&element.kind)),
        );
    }
}

fn validate_group(workspace: &Workspace, element: &Element, diagnostics: &mut Vec<Diagnostic>) {
    let Some(group_index) = element.group else {
        return;
    };
    let group = &workspace.groups[group_index];
    let valid = match group
        .owner
        .as_deref()
        .and_then(|owner| find(workspace, owner))
    {
        None => matches!(
            element.kind,
            ElementKind::Person
                | ElementKind::SoftwareSystem
                | ElementKind::Generic
                | ElementKind::ArchiMate(_)
        ),
        Some(owner) if owner.kind == ElementKind::SoftwareSystem => {
            element.kind == ElementKind::Container
        }
        Some(owner) if owner.kind == ElementKind::Container => {
            element.kind == ElementKind::Component
        }
        _ => false,
    };
    if !valid {
        diagnostics.push(
            Diagnostic::error(
                element.span,
                format!(
                    "element '{}' is incompatible with group '{}' abstraction level",
                    element.id, group.name
                ),
            )
            .with_help("group only elements at the same C4 abstraction level"),
        );
    }
}

fn validate_element_references(
    workspace: &Workspace,
    element: &Element,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if element.instances.is_some() && element.kind != ElementKind::DeploymentNode {
        let span = element
            .attributes
            .iter()
            .rev()
            .find(|property| property.key == "instances")
            .map_or(element.span, |property| property.span);
        diagnostics.push(
            Diagnostic::error(span, "instances is only allowed on deployment nodes")
                .with_help("move this property to a deploymentNode"),
        );
    }
    if let Some(reference) = &element.reference {
        let expected = match element.kind {
            ElementKind::SoftwareSystemInstance => Some(ElementKind::SoftwareSystem),
            ElementKind::ContainerInstance => Some(ElementKind::Container),
            _ => None,
        };
        let target = validate_prior_reference(
            workspace,
            &reference.identifier,
            reference.span,
            element.order,
            "instance reference",
            diagnostics,
        );
        if let (Some(target), Some(expected)) = (target, expected) {
            if target.kind != expected {
                diagnostics.push(
                    Diagnostic::error(
                        reference.span,
                        format!(
                            "{} must reference a {}",
                            element_kind_label(&element.kind),
                            element_kind_label(&expected)
                        ),
                    )
                    .with_help("reference a deployable element of the required kind"),
                );
            }
        }
    }
    if let Some(reference) = &element.instance_of {
        validate_prior_reference(
            workspace,
            &reference.identifier,
            reference.span,
            element.order,
            "instanceOf reference",
            diagnostics,
        );
    }
    if !element.health_checks.is_empty()
        && !matches!(
            element.kind,
            ElementKind::SoftwareSystemInstance | ElementKind::ContainerInstance
        )
    {
        diagnostics.push(
            Diagnostic::error(
                element.health_checks[0].span,
                "healthCheck is only allowed on software system or container instances",
            )
            .with_help("move the health check into an instance block"),
        );
    }
    if !element.deployment_groups.is_empty() {
        let environment = deployment_environment(workspace, element);
        for reference in &element.deployment_groups {
            let group = validate_prior_reference(
                workspace,
                &reference.identifier,
                reference.span,
                element.order,
                "deployment group",
                diagnostics,
            );
            if let Some(group) = group {
                if group.kind != ElementKind::DeploymentGroup
                    || deployment_environment(workspace, group) != environment
                {
                    diagnostics.push(
                        Diagnostic::error(
                            reference.span,
                            format!(
                                "deployment group '{}' is not in the same deployment environment",
                                reference.identifier
                            ),
                        )
                        .with_help("reference a deployment group from this environment"),
                    );
                }
            }
        }
    }
}

fn validate_prior_reference<'a>(
    workspace: &'a Workspace,
    identifier: &str,
    span: Span,
    order: usize,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a Element> {
    match find(workspace, identifier) {
        Some(element) if element.order < order => Some(element),
        Some(_) => {
            diagnostics.push(
                Diagnostic::error(
                    span,
                    format!("{label} '{identifier}' is a forward reference"),
                )
                .with_help("move the referenced element before this statement"),
            );
            None
        }
        None => {
            diagnostics.push(
                Diagnostic::error(span, format!("{label} '{identifier}' is not defined"))
                    .with_help(format!(
                        "define '{identifier}' before this statement, or check the identifier"
                    )),
            );
            None
        }
    }
}

fn deployment_environment<'a>(workspace: &'a Workspace, element: &'a Element) -> Option<&'a str> {
    let mut current = Some(element);
    while let Some(element) = current {
        if element.kind == ElementKind::DeploymentEnvironment {
            return Some(&element.id);
        }
        current = element
            .parent
            .as_deref()
            .and_then(|parent| find(workspace, parent));
    }
    None
}

fn element_kind_label(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::SoftwareSystem => "software system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Generic => "element",
        ElementKind::DeploymentEnvironment => "deployment environment",
        ElementKind::DeploymentGroup => "deployment group",
        ElementKind::DeploymentNode => "deployment node",
        ElementKind::InfrastructureNode => "infrastructure node",
        ElementKind::SoftwareSystemInstance => "software system instance",
        ElementKind::ContainerInstance => "container instance",
        ElementKind::ArchiMate(_) => "ArchiMate element",
    }
}

fn parent_help(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Container => "define containers inside a softwareSystem block",
        ElementKind::Component => "define components inside a container block",
        ElementKind::DeploymentGroup | ElementKind::DeploymentNode => {
            "define this inside a deploymentEnvironment block"
        }
        ElementKind::InfrastructureNode
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "define this inside a deploymentNode block",
        _ => "move this element to the model, enterprise, or a compatible group",
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
            "  {}{} -> {} desc={:?} tech={:?} tags={:?} archimateType={:?} access={:?}\n",
            relationship
                .id
                .as_ref()
                .map(|id| format!("{id} = "))
                .unwrap_or_default(),
            relationship.source,
            relationship.destination,
            relationship.description,
            relationship.technology,
            relationship.tags,
            relationship_archimate_type(relationship),
            relationship_access_direction(relationship)
        ));
    }
    output.push_str("views:\n");
    for view in &workspace.views {
        output.push_str(&format!(
            "  {:?} scope={:?} key={:?} desc={:?} viewpoint={:?}\n",
            view.kind,
            view.scope,
            view.key,
            view.description,
            archimate_viewpoint(view).map(|property| property.value.as_str())
        ));
    }
    if !workspace.view_properties.is_empty() || workspace.views.iter().any(has_m4_view_data) {
        output.push_str("m4 views:\n");
        output.push_str(&format!(
            "  properties={:?}\n",
            property_pairs(&workspace.view_properties)
        ));
        for view in workspace.views.iter().filter(|view| has_m4_view_data(view)) {
            output.push_str(&format!(
                "  {:?} layout={:?} default={} animations={} properties={} dynamic={} images={}\n",
                view.kind,
                view.auto_layout.as_ref().map(|layout| (
                    &layout.direction,
                    &layout.rank_separation,
                    &layout.node_separation
                )),
                view.is_default,
                view.animations.len(),
                view.properties.len(),
                view.dynamic_relationships.len(),
                view.image_sources.len()
            ));
        }
    }
    if !workspace.element_styles.is_empty()
        || !workspace.relationship_styles.is_empty()
        || !workspace.themes.is_empty()
        || workspace.branding.is_some()
        || !workspace.terminology.is_empty()
    {
        output.push_str("m5:\n");
        output.push_str(&format!(
            "  elementStyles={} relationshipStyles={} themes={:?} terminology={:?}\n",
            workspace.element_styles.len(),
            workspace.relationship_styles.len(),
            workspace
                .themes
                .iter()
                .map(|theme| (&theme.source, theme.mode, theme.span.start))
                .collect::<Vec<_>>(),
            property_pairs(&workspace.terminology)
        ));
        if let Some(branding) = &workspace.branding {
            debug_assert!(branding.span.end >= branding.span.start);
            output.push_str(&format!(
                "  branding logo={:?} fonts={:?}\n",
                branding.logo.as_ref().map(|logo| &logo.value),
                branding
                    .fonts
                    .iter()
                    .map(|font| (&font.name, &font.location, font.span.start))
                    .collect::<Vec<_>>()
            ));
        }
    }
    if !workspace.dependencies.is_empty() {
        output.push_str("m6 includes:\n");
        for dependency in &workspace.dependencies {
            output.push_str(&format!(
                "  {} -> {}\n",
                dependency.from.display(),
                dependency.to.display()
            ));
        }
    }
    if !workspace.documentation.is_empty() || !workspace.decisions.is_empty() {
        output.push_str(&format!(
            "m7 documentation: sections={} decisions={}\n",
            workspace.documentation.len(),
            workspace.decisions.len()
        ));
    }
    if workspace.extension.is_some()
        || !workspace.properties.is_empty()
        || !workspace.directives.is_empty()
        || !workspace.preserved.is_empty()
        || !workspace.groups.is_empty()
        || !workspace.removed_relationships.is_empty()
        || workspace.implied_relationships.is_some()
        || workspace.enterprise.is_some()
        || workspace.elements.iter().any(has_m3_element_data)
    {
        output.push_str("m3:\n");
        if let Some(extension) = &workspace.extension {
            output.push_str(&format!("  extends {:?}\n", extension.target));
        }
        if let Some(enterprise) = &workspace.enterprise {
            debug_assert!(enterprise.span.end >= enterprise.span.start);
            output.push_str(&format!("  enterprise {:?}\n", enterprise.name));
        }
        output.push_str(&format!(
            "  properties={:?} impliedRelationships={:?}\n",
            property_pairs(&workspace.properties),
            workspace.implied_relationships
        ));
        output.push_str(&format!(
            "  directives={:?} preserved={:?} groups={} removals={}\n",
            workspace
                .directives
                .iter()
                .map(|directive| (&directive.name, &directive.arguments))
                .collect::<Vec<_>>(),
            workspace
                .preserved
                .iter()
                .map(|preserved| (&preserved.name, &preserved.arguments))
                .collect::<Vec<_>>(),
            workspace.groups.len(),
            workspace.removed_relationships.len()
        ));
        for element in &workspace.elements {
            if has_m3_element_data(element) {
                output.push_str(&format!(
                    "  {} type={:?} url={:?} properties={:?} perspectives={:?} healthChecks={:?}\n",
                    element.id,
                    element.element_type,
                    element.url,
                    property_pairs(&element.properties),
                    property_pairs(&element.perspectives),
                    element
                        .health_checks
                        .iter()
                        .map(|check| (&check.name, &check.url, &check.interval, &check.timeout))
                        .collect::<Vec<_>>()
                ));
            }
        }
    }
    output
}

fn has_m4_view_data(view: &View) -> bool {
    !matches!(view.kind, ViewKind::SystemContext | ViewKind::Container)
        || view.is_default
        || !view.animations.is_empty()
        || !view.properties.is_empty()
        || view.filter.is_some()
        || view.environment.is_some()
        || !view.dynamic_relationships.is_empty()
        || !view.image_sources.is_empty()
        || view.auto_layout.as_ref().is_some_and(|layout| {
            layout.rank_separation.is_some() || layout.node_separation.is_some()
        })
        || view
            .includes
            .iter()
            .any(|selector| selector.value == "*?" || selector.expression)
        || view.excludes.iter().any(|selector| selector.expression)
}

fn has_m3_element_data(element: &Element) -> bool {
    element.element_type.is_some()
        || element.url.is_some()
        || !element.properties.is_empty()
        || !element.perspectives.is_empty()
        || !element.health_checks.is_empty()
        || element.instance_of.is_some()
        || element.reference.is_some()
        || !element.deployment_groups.is_empty()
        || !element.directives.is_empty()
        || element.instances.is_some()
}

fn property_pairs(properties: &[Property]) -> Vec<(&str, &str)> {
    properties
        .iter()
        .map(|property| {
            debug_assert!(property.span.end >= property.span.start);
            (property.key.as_str(), property.value.as_str())
        })
        .collect()
}

pub fn export_mermaid(workspace: &Workspace, output: &Path) -> Result<(), String> {
    for view in &workspace.views {
        let key = view
            .key
            .clone()
            .unwrap_or_else(|| default_view_key(&view.kind).into());
        fs::write(output.join(format!("{key}.mmd")), mermaid(workspace, view))
            .map_err(|error| format!("cannot write {key}.mmd: {error}"))?;
    }
    Ok(())
}

pub fn export_site(workspace: &Workspace, output: &Path) -> Result<(), String> {
    documentation::export_site(workspace, output)
}

pub fn adr_list(workspace: &Workspace) -> String {
    documentation::adr_list(workspace)
}

pub(crate) fn mermaid(workspace: &Workspace, view: &View) -> String {
    if view.kind == ViewKind::Dynamic {
        return dynamic_mermaid(workspace, view);
    }
    if view.kind == ViewKind::Custom {
        return format!(
            "%% {:?} view rendering is deferred; source metadata was preserved\n",
            view.kind
        );
    }
    if view.kind == ViewKind::Image {
        let mut output = String::from("%% Image view rendering is deferred\n");
        for source in &view.image_sources {
            debug_assert!(source.span.end >= source.span.start);
            output.push_str(&format!(
                "%% {} {}\n",
                source.kind,
                source.arguments.join(" ")
            ));
        }
        return output;
    }
    let expanded = expand_view(workspace, view);
    let mut output = String::from("flowchart LR\n");
    let mut class_definitions = Vec::new();
    for element in &workspace.elements {
        if !expanded.elements.contains(&element.id) {
            continue;
        }
        let label = format!(
            "{}\\n{}",
            escape(&element.name),
            terminology_label(workspace, &element.kind)
        );
        let style = effective_element_style(workspace, element);
        let class_name = format!("style_{}", node_id(&element.id));
        let css = element_mermaid_css(&style);
        let class = (!css.is_empty()).then(|| format!(":::{class_name}"));
        output.push_str(&format!(
            "  {}{}{}\n",
            node_id(&element.id),
            mermaid_shape(&label, style.get("shape").map(String::as_str)),
            class.as_deref().unwrap_or("")
        ));
        if !css.is_empty() {
            class_definitions.push(format!("  classDef {class_name} {}\n", css.join(",")));
        }
    }
    let mut emitted = HashSet::new();
    let mut link_styles = Vec::new();
    let mut link_index = 0usize;
    let endpoint_view = static_base_view(workspace, view).unwrap_or(view);
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        if !expanded.relationships.contains(&index) {
            continue;
        }
        let source = view_endpoint(workspace, endpoint_view, &relationship.source);
        let destination = view_endpoint(workspace, endpoint_view, &relationship.destination);
        if source == destination
            || !expanded.elements.contains(&source)
            || !expanded.elements.contains(&destination)
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
            let css =
                relationship_mermaid_css(&effective_relationship_style(workspace, relationship));
            if !css.is_empty() {
                link_styles.push(format!("  linkStyle {link_index} {}\n", css.join(",")));
            }
            link_index += 1;
        }
    }
    for definition in class_definitions {
        output.push_str(&definition);
    }
    for style in link_styles {
        output.push_str(&style);
    }
    output
}

fn effective_element_style(workspace: &Workspace, element: &Element) -> HashMap<String, String> {
    let implicit = element_style_tag(&element.kind);
    let mut effective = HashMap::new();
    for style in workspace
        .element_styles
        .iter()
        .filter(|style| style.mode == StyleMode::Default)
    {
        if style.tag == implicit || element.tags.iter().any(|tag| tag == &style.tag) {
            for property in &style.values {
                effective.insert(property.key.clone(), property.value.clone());
            }
        }
    }
    effective
}

fn effective_relationship_style(
    workspace: &Workspace,
    relationship: &Relationship,
) -> HashMap<String, String> {
    let mut effective = HashMap::new();
    for style in workspace
        .relationship_styles
        .iter()
        .filter(|style| style.mode == StyleMode::Default)
    {
        if style.tag == "Relationship" || relationship.tags.iter().any(|tag| tag == &style.tag) {
            for property in &style.values {
                effective.insert(property.key.clone(), property.value.clone());
            }
        }
    }
    effective
}

fn element_mermaid_css(style: &HashMap<String, String>) -> Vec<String> {
    let mut css = Vec::new();
    for (property, mermaid) in [
        ("background", "fill"),
        ("color", "color"),
        ("stroke", "stroke"),
    ] {
        if let Some(value) = style.get(property) {
            css.push(format!("{mermaid}:{value}"));
        }
    }
    if let Some(value) = style.get("strokeWidth") {
        css.push(format!("stroke-width:{value}px"));
    }
    match style.get("border").map(String::as_str) {
        Some(value) if value.eq_ignore_ascii_case("dashed") => {
            css.push("stroke-dasharray:5 5".into())
        }
        Some(value) if value.eq_ignore_ascii_case("dotted") => {
            css.push("stroke-dasharray:2 3".into())
        }
        _ => {}
    }
    css
}

fn relationship_mermaid_css(style: &HashMap<String, String>) -> Vec<String> {
    let mut css = Vec::new();
    if let Some(value) = style.get("color") {
        css.push(format!("stroke:{value}"));
    }
    if let Some(value) = style.get("thickness") {
        css.push(format!("stroke-width:{value}px"));
    }
    match style.get("style").map(String::as_str) {
        Some(value) if value.eq_ignore_ascii_case("dashed") => {
            css.push("stroke-dasharray:5 5".into())
        }
        Some(value) if value.eq_ignore_ascii_case("dotted") => {
            css.push("stroke-dasharray:2 3".into())
        }
        _ => {}
    }
    css
}

fn mermaid_shape(label: &str, shape: Option<&str>) -> String {
    match shape.map(str::to_ascii_lowercase).as_deref() {
        Some("roundedbox") => format!("(\"{label}\")"),
        Some("circle") => format!("((\"{label}\"))"),
        Some("hexagon") => format!("{{{{\"{label}\"}}}}"),
        Some("diamond") => format!("{{\"{label}\"}}"),
        Some("cylinder") => format!("[(\"{label}\")]"),
        _ => format!("[\"{label}\"]"),
    }
}

#[derive(Debug)]
enum ViewExpression {
    Predicate {
        target: ExpressionTarget,
        field: ExpressionField,
        equal: bool,
        value: String,
    },
    Not(Box<ViewExpression>),
    And(Box<ViewExpression>, Box<ViewExpression>),
    Or(Box<ViewExpression>, Box<ViewExpression>),
}

#[derive(Debug, PartialEq)]
enum ExpressionTarget {
    Element,
    Relationship,
}

#[derive(Debug)]
enum ExpressionField {
    Tag,
    Type,
    Property(String),
}

impl ViewExpression {
    fn matches_element(&self, element: &Element) -> bool {
        self.evaluate(|target, field, expected| {
            if *target != ExpressionTarget::Element {
                return None;
            }
            Some(match field {
                ExpressionField::Tag => element.tags.iter().any(|tag| tag == expected),
                ExpressionField::Type => element_kind_label(&element.kind)
                    .replace(' ', "")
                    .eq_ignore_ascii_case(&expected.replace(' ', "")),
                ExpressionField::Property(name) => element
                    .properties
                    .iter()
                    .any(|property| property.key == *name && property.value == expected),
            })
        })
        .unwrap_or(false)
    }

    fn matches_relationship(&self, relationship: &Relationship) -> bool {
        self.evaluate(|target, field, expected| {
            if *target != ExpressionTarget::Relationship {
                return None;
            }
            Some(match field {
                ExpressionField::Tag => relationship.tags.iter().any(|tag| tag == expected),
                ExpressionField::Property(name) => relationship
                    .properties
                    .iter()
                    .any(|property| property.key == *name && property.value == expected),
                ExpressionField::Type => false,
            })
        })
        .unwrap_or(false)
    }

    fn evaluate(
        &self,
        predicate: impl Fn(&ExpressionTarget, &ExpressionField, &str) -> Option<bool> + Copy,
    ) -> Option<bool> {
        match self {
            Self::Predicate {
                target,
                field,
                equal,
                value,
            } => predicate(target, field, value).map(|actual| actual == *equal),
            Self::Not(expression) => expression.evaluate(predicate).map(|value| !value),
            Self::And(left, right) => match (left.evaluate(predicate), right.evaluate(predicate)) {
                (Some(left), Some(right)) => Some(left && right),
                (Some(false), None) | (None, Some(false)) => Some(false),
                _ => None,
            },
            Self::Or(left, right) => match (left.evaluate(predicate), right.evaluate(predicate)) {
                (Some(left), Some(right)) => Some(left || right),
                (Some(value), None) | (None, Some(value)) => Some(value),
                (None, None) => None,
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum ExpressionToken {
    Value(String),
    Equal,
    NotEqual,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

struct ExpressionParser {
    tokens: Vec<ExpressionToken>,
    index: usize,
}

impl ExpressionParser {
    fn parse(mut self) -> Result<ViewExpression, String> {
        let expression = self.parse_or()?;
        if self.index != self.tokens.len() {
            return Err("unexpected trailing token".into());
        }
        Ok(expression)
    }

    fn parse_or(&mut self) -> Result<ViewExpression, String> {
        let mut expression = self.parse_and()?;
        while self.eat(&ExpressionToken::Or) {
            expression = ViewExpression::Or(Box::new(expression), Box::new(self.parse_and()?));
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<ViewExpression, String> {
        let mut expression = self.parse_unary()?;
        while self.eat(&ExpressionToken::And) {
            expression = ViewExpression::And(Box::new(expression), Box::new(self.parse_unary()?));
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<ViewExpression, String> {
        if self.eat(&ExpressionToken::Not) {
            return Ok(ViewExpression::Not(Box::new(self.parse_unary()?)));
        }
        if self.eat(&ExpressionToken::LeftParen) {
            let expression = self.parse_or()?;
            if !self.eat(&ExpressionToken::RightParen) {
                return Err("missing ')'".into());
            }
            return Ok(expression);
        }
        self.parse_predicate()
    }

    fn parse_predicate(&mut self) -> Result<ViewExpression, String> {
        let Some(ExpressionToken::Value(name)) = self.tokens.get(self.index) else {
            return Err("expected element or relationship predicate".into());
        };
        let name = name.clone();
        self.index += 1;
        let equal = if self.eat(&ExpressionToken::Equal) {
            true
        } else if self.eat(&ExpressionToken::NotEqual) {
            false
        } else {
            return Err("expected == or !=".into());
        };
        let Some(ExpressionToken::Value(value)) = self.tokens.get(self.index) else {
            return Err("comparison value is missing".into());
        };
        let value = value.trim_matches('"').to_string();
        self.index += 1;
        let (target, field) = expression_field(&name)?;
        Ok(ViewExpression::Predicate {
            target,
            field,
            equal,
            value,
        })
    }

    fn eat(&mut self, token: &ExpressionToken) -> bool {
        if self.tokens.get(self.index) == Some(token) {
            self.index += 1;
            true
        } else {
            false
        }
    }
}

fn parse_view_expression(value: &str) -> Result<ViewExpression, String> {
    ExpressionParser {
        tokens: expression_tokens(value)?,
        index: 0,
    }
    .parse()
}

fn expression_tokens(value: &str) -> Result<Vec<ExpressionToken>, String> {
    let mut tokens = Vec::new();
    let mut chars = value.char_indices().peekable();
    while let Some((start, character)) = chars.next() {
        if character.is_whitespace() {
            continue;
        }
        let token = match character {
            '(' => ExpressionToken::LeftParen,
            ')' => ExpressionToken::RightParen,
            '!' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                ExpressionToken::NotEqual
            }
            '!' => ExpressionToken::Not,
            '=' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                ExpressionToken::Equal
            }
            '&' if chars.peek().is_some_and(|(_, next)| *next == '&') => {
                chars.next();
                ExpressionToken::And
            }
            '|' if chars.peek().is_some_and(|(_, next)| *next == '|') => {
                chars.next();
                ExpressionToken::Or
            }
            '"' => {
                let mut end = None;
                for (index, next) in chars.by_ref() {
                    if next == '"' {
                        end = Some(index + 1);
                        break;
                    }
                }
                let end = end.ok_or_else(|| "unterminated quoted value".to_string())?;
                ExpressionToken::Value(value[start..end].into())
            }
            _ => {
                let mut end = value.len();
                while let Some((index, next)) = chars.peek() {
                    if next.is_whitespace() || matches!(next, '(' | ')' | '!' | '=' | '&' | '|') {
                        end = *index;
                        break;
                    }
                    chars.next();
                }
                ExpressionToken::Value(value[start..end].into())
            }
        };
        tokens.push(token);
    }
    Ok(tokens)
}

fn expression_field(value: &str) -> Result<(ExpressionTarget, ExpressionField), String> {
    let (target, field) = value
        .split_once('.')
        .ok_or_else(|| format!("unsupported predicate '{value}'"))?;
    let target = match target {
        "element" => ExpressionTarget::Element,
        "relationship" => ExpressionTarget::Relationship,
        _ => return Err(format!("unsupported expression target '{target}'")),
    };
    let field = match field {
        "tag" => ExpressionField::Tag,
        "type" if target == ExpressionTarget::Element => ExpressionField::Type,
        value if value.starts_with("properties.") && value.len() > 11 => {
            ExpressionField::Property(value[11..].into())
        }
        _ => return Err(format!("unsupported expression field '{field}'")),
    };
    Ok((target, field))
}

fn expression_candidate(value: &str) -> bool {
    value.contains("element.") || value.contains("relationship.")
}

#[derive(Default)]
struct ExpandedView {
    elements: HashSet<String>,
    relationships: HashSet<usize>,
}

pub(crate) struct ViewGraph {
    pub element_ids: Vec<String>,
    pub relationships: Vec<ViewGraphRelationship>,
}

pub(crate) struct ViewGraphRelationship {
    pub source: String,
    pub destination: String,
    pub description: String,
    pub relationship_index: Option<usize>,
}

pub(crate) fn view_graph(workspace: &Workspace, view: &View) -> ViewGraph {
    if view.kind == ViewKind::Dynamic {
        let mut identifiers = HashSet::new();
        let relationships = view
            .dynamic_relationships
            .iter()
            .filter_map(|relationship| {
                let (source, destination) = (&relationship.source, &relationship.destination);
                let (Some(source), Some(destination)) = (source, destination) else {
                    return None;
                };
                identifiers.insert(source.identifier.clone());
                identifiers.insert(destination.identifier.clone());
                Some(ViewGraphRelationship {
                    source: source.identifier.clone(),
                    destination: destination.identifier.clone(),
                    description: relationship.description.clone().unwrap_or_default(),
                    relationship_index: None,
                })
            })
            .collect();
        return ViewGraph {
            element_ids: workspace
                .elements
                .iter()
                .filter(|element| identifiers.contains(&element.id))
                .map(|element| element.id.clone())
                .collect(),
            relationships,
        };
    }

    let expanded = expand_view(workspace, view);
    let endpoint_view = static_base_view(workspace, view).unwrap_or(view);
    let mut emitted = HashSet::new();
    let mut relationships = Vec::new();
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        if !expanded.relationships.contains(&index) {
            continue;
        }
        let source = view_endpoint(workspace, endpoint_view, &relationship.source);
        let destination = view_endpoint(workspace, endpoint_view, &relationship.destination);
        if source == destination
            || !expanded.elements.contains(&source)
            || !expanded.elements.contains(&destination)
        {
            continue;
        }
        let description = relationship.description.clone().unwrap_or_default();
        if emitted.insert((source.clone(), destination.clone(), description.clone())) {
            relationships.push(ViewGraphRelationship {
                source,
                destination,
                description,
                relationship_index: Some(index),
            });
        }
    }
    ViewGraph {
        element_ids: workspace
            .elements
            .iter()
            .filter(|element| expanded.elements.contains(&element.id))
            .map(|element| element.id.clone())
            .collect(),
        relationships,
    }
}

fn expand_view(workspace: &Workspace, view: &View) -> ExpandedView {
    if view.kind == ViewKind::Filtered {
        return expand_filtered_view(workspace, view);
    }
    if view.kind == ViewKind::Deployment {
        return expand_deployment_view(workspace, view);
    }
    let mut expanded = ExpandedView::default();
    let wildcard = view
        .includes
        .iter()
        .find(|selector| matches!(selector.value.as_str(), "*" | "*?"));
    let reluctant = wildcard.is_some_and(|selector| selector.value == "*?");
    if wildcard.is_none() {
        apply_explicit_includes(workspace, view, &mut expanded);
        apply_excludes(workspace, view, &mut expanded);
        return expanded;
    }
    match view.kind {
        ViewKind::SystemLandscape => {
            for element in &workspace.elements {
                if matches!(
                    element.kind,
                    ElementKind::Person | ElementKind::SoftwareSystem
                ) {
                    expanded.elements.insert(element.id.clone());
                }
            }
        }
        ViewKind::SystemContext => {
            if let Some(scope) = &view.scope {
                expanded.elements.insert(scope.clone());
                for (index, relationship) in workspace.relationships.iter().enumerate() {
                    let source = if reluctant {
                        relationship.source.clone()
                    } else {
                        view_endpoint(workspace, view, &relationship.source)
                    };
                    let destination = if reluctant {
                        relationship.destination.clone()
                    } else {
                        view_endpoint(workspace, view, &relationship.destination)
                    };
                    if source == *scope {
                        expanded.elements.insert(destination.clone());
                        expanded.relationships.insert(index);
                    }
                    if destination == *scope {
                        expanded.elements.insert(source);
                        expanded.relationships.insert(index);
                    }
                }
            }
        }
        ViewKind::Container => {
            if let Some(scope) = &view.scope {
                let mut containers = HashSet::new();
                for element in &workspace.elements {
                    if element.parent.as_deref() == Some(scope) || element.id == *scope {
                        expanded.elements.insert(element.id.clone());
                        if element.kind == ElementKind::Container {
                            containers.insert(element.id.clone());
                        }
                    }
                }
                for (index, relationship) in workspace.relationships.iter().enumerate() {
                    let relevant = if reluctant {
                        containers.contains(&relationship.source)
                            || containers.contains(&relationship.destination)
                    } else {
                        expanded.elements.contains(&relationship.source)
                            || expanded.elements.contains(&relationship.destination)
                    };
                    if relevant {
                        for identifier in [&relationship.source, &relationship.destination] {
                            if container_view_element_allowed(workspace, scope, identifier) {
                                expanded.elements.insert(identifier.clone());
                            }
                        }
                    }
                    if relevant
                        && expanded.elements.contains(&relationship.source)
                        && expanded.elements.contains(&relationship.destination)
                    {
                        expanded.relationships.insert(index);
                    }
                }
            }
        }
        ViewKind::Component => {
            if let Some(scope) = &view.scope {
                expanded.elements.insert(scope.clone());
                let mut components = HashSet::new();
                for element in &workspace.elements {
                    if element.parent.as_deref() == Some(scope) {
                        components.insert(element.id.clone());
                        expanded.elements.insert(element.id.clone());
                    }
                }
                for (index, relationship) in workspace.relationships.iter().enumerate() {
                    if components.contains(&relationship.source)
                        || components.contains(&relationship.destination)
                    {
                        for identifier in [&relationship.source, &relationship.destination] {
                            if component_view_element_allowed(workspace, scope, identifier) {
                                expanded.elements.insert(identifier.clone());
                            }
                        }
                        if expanded.elements.contains(&relationship.source)
                            && expanded.elements.contains(&relationship.destination)
                        {
                            expanded.relationships.insert(index);
                        }
                    }
                }
            }
        }
        ViewKind::ArchiMate => {
            for element in &workspace.elements {
                expanded.elements.insert(element.id.clone());
            }
            for (index, relationship) in workspace.relationships.iter().enumerate() {
                if expanded.elements.contains(&relationship.source)
                    && expanded.elements.contains(&relationship.destination)
                {
                    expanded.relationships.insert(index);
                }
            }
        }
        _ => {}
    }
    if !reluctant {
        for (index, relationship) in workspace.relationships.iter().enumerate() {
            let source = view_endpoint(workspace, view, &relationship.source);
            let destination = view_endpoint(workspace, view, &relationship.destination);
            if source != destination
                && expanded.elements.contains(&source)
                && expanded.elements.contains(&destination)
            {
                expanded.relationships.insert(index);
            }
        }
    }
    apply_explicit_includes(workspace, view, &mut expanded);
    apply_excludes(workspace, view, &mut expanded);
    expanded
}

fn container_view_element_allowed(workspace: &Workspace, scope: &str, identifier: &str) -> bool {
    find(workspace, identifier).is_some_and(|element| {
        element.id == scope
            || matches!(
                element.kind,
                ElementKind::Person | ElementKind::SoftwareSystem
            )
            || (element.kind == ElementKind::Container && element.parent.as_deref() == Some(scope))
    })
}

fn component_view_element_allowed(workspace: &Workspace, scope: &str, identifier: &str) -> bool {
    let system = software_system_of(workspace, scope);
    find(workspace, identifier).is_some_and(|element| {
        element.id == scope
            || matches!(
                element.kind,
                ElementKind::Person | ElementKind::SoftwareSystem
            )
            || (element.kind == ElementKind::Component && element.parent.as_deref() == Some(scope))
            || (element.kind == ElementKind::Container
                && software_system_of(workspace, identifier) == system)
    })
}

fn apply_explicit_includes(workspace: &Workspace, view: &View, expanded: &mut ExpandedView) {
    for selector in &view.includes {
        if matches!(selector.value.as_str(), "*" | "*?") {
            continue;
        }
        if let Some((source, destination)) = relationship_pattern(&selector.value) {
            for (index, relationship) in workspace.relationships.iter().enumerate() {
                if relationship_matches(relationship, source, destination) {
                    expanded.elements.insert(relationship.source.clone());
                    expanded.elements.insert(relationship.destination.clone());
                    expanded.relationships.insert(index);
                }
            }
        } else if selector.expression {
            if let Ok(expression) = parse_view_expression(&selector.value) {
                for element in &workspace.elements {
                    if expression.matches_element(element) {
                        expanded.elements.insert(element.id.clone());
                    }
                }
                for (index, relationship) in workspace.relationships.iter().enumerate() {
                    if expression.matches_relationship(relationship) {
                        expanded.elements.insert(relationship.source.clone());
                        expanded.elements.insert(relationship.destination.clone());
                        expanded.relationships.insert(index);
                    }
                }
            }
        } else {
            expanded.elements.insert(selector.value.clone());
        }
    }
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        if expanded.elements.contains(&relationship.source)
            && expanded.elements.contains(&relationship.destination)
        {
            expanded.relationships.insert(index);
        }
    }
}

fn apply_excludes(workspace: &Workspace, view: &View, expanded: &mut ExpandedView) {
    for selector in &view.excludes {
        if let Some((source, destination)) = relationship_pattern(&selector.value) {
            expanded.relationships.retain(|index| {
                !relationship_matches(&workspace.relationships[*index], source, destination)
            });
        } else if selector.expression {
            if let Ok(expression) = parse_view_expression(&selector.value) {
                expanded.elements.retain(|identifier| {
                    find(workspace, identifier)
                        .is_none_or(|element| !expression.matches_element(element))
                });
                expanded.relationships.retain(|index| {
                    !expression.matches_relationship(&workspace.relationships[*index])
                });
            }
        } else {
            expanded.elements.remove(&selector.value);
        }
    }
    expanded.relationships.retain(|index| {
        let relationship = &workspace.relationships[*index];
        let endpoint_view = static_base_view(workspace, view).unwrap_or(view);
        expanded.elements.contains(&view_endpoint(
            workspace,
            endpoint_view,
            &relationship.source,
        )) && expanded.elements.contains(&view_endpoint(
            workspace,
            endpoint_view,
            &relationship.destination,
        ))
    });
}

fn expand_filtered_view(workspace: &Workspace, view: &View) -> ExpandedView {
    let Some(filter) = &view.filter else {
        return ExpandedView::default();
    };
    let Some(base) = workspace
        .views
        .iter()
        .find(|candidate| candidate.key.as_deref() == Some(&filter.base_key.identifier))
    else {
        return ExpandedView::default();
    };
    let mut expanded = expand_view(workspace, base);
    let element_matches = |element: &Element| {
        element
            .tags
            .iter()
            .any(|tag| filter.tags.iter().any(|filter_tag| tag == filter_tag))
    };
    let relationship_matches_tags = |relationship: &Relationship| {
        relationship
            .tags
            .iter()
            .any(|tag| filter.tags.iter().any(|filter_tag| tag == filter_tag))
    };
    match filter.mode {
        FilterMode::Include => {
            let tagged_relationships = expanded
                .relationships
                .iter()
                .copied()
                .filter(|index| relationship_matches_tags(&workspace.relationships[*index]))
                .collect::<HashSet<_>>();
            expanded.elements.retain(|identifier| {
                find(workspace, identifier).is_some_and(element_matches)
                    || tagged_relationships.iter().any(|index| {
                        let relationship = &workspace.relationships[*index];
                        view_endpoint(workspace, base, &relationship.source) == *identifier
                            || view_endpoint(workspace, base, &relationship.destination)
                                == *identifier
                    })
            });
            expanded.relationships.retain(|index| {
                let relationship = &workspace.relationships[*index];
                tagged_relationships.contains(index)
                    && expanded.elements.contains(&view_endpoint(
                        workspace,
                        base,
                        &relationship.source,
                    ))
                    && expanded.elements.contains(&view_endpoint(
                        workspace,
                        base,
                        &relationship.destination,
                    ))
            });
        }
        FilterMode::Exclude => {
            expanded.elements.retain(|identifier| {
                find(workspace, identifier).is_none_or(|element| !element_matches(element))
            });
            expanded.relationships.retain(|index| {
                let relationship = &workspace.relationships[*index];
                !relationship_matches_tags(relationship)
                    && expanded.elements.contains(&view_endpoint(
                        workspace,
                        base,
                        &relationship.source,
                    ))
                    && expanded.elements.contains(&view_endpoint(
                        workspace,
                        base,
                        &relationship.destination,
                    ))
            });
        }
    }
    apply_excludes(workspace, view, &mut expanded);
    expanded
}

fn expand_deployment_view(workspace: &Workspace, view: &View) -> ExpandedView {
    let mut expanded = ExpandedView::default();
    let Some(environment) = view.environment.as_ref().and_then(|reference| {
        workspace.elements.iter().find(|element| {
            element.kind == ElementKind::DeploymentEnvironment
                && (element.id == reference.identifier || element.name == reference.identifier)
        })
    }) else {
        return expanded;
    };
    for element in &workspace.elements {
        if deployment_environment(workspace, element) != Some(&environment.id) {
            continue;
        }
        let include = match element.kind {
            ElementKind::DeploymentNode | ElementKind::InfrastructureNode => true,
            ElementKind::ContainerInstance if view.scope.as_deref() == Some("*") => true,
            ElementKind::ContainerInstance => {
                element
                    .reference
                    .as_ref()
                    .and_then(|reference| software_system_of(workspace, &reference.identifier))
                    == view.scope.as_deref()
            }
            _ => false,
        };
        if include {
            expanded.elements.insert(element.id.clone());
        }
    }
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        if expanded.elements.contains(&relationship.source)
            && expanded.elements.contains(&relationship.destination)
        {
            expanded.relationships.insert(index);
        }
    }
    apply_excludes(workspace, view, &mut expanded);
    expanded
}

fn dynamic_mermaid(workspace: &Workspace, view: &View) -> String {
    let mut output = String::from("sequenceDiagram\n");
    for instance in &view.dynamic_relationships {
        let (Some(source), Some(destination)) = (&instance.source, &instance.destination) else {
            continue;
        };
        let source_name = find(workspace, &source.identifier)
            .map_or(source.identifier.as_str(), |element| element.name.as_str());
        let destination_name = find(workspace, &destination.identifier)
            .map_or(destination.identifier.as_str(), |element| {
                element.name.as_str()
            });
        let label = match &instance.sequence {
            Some(sequence) => format!(
                "{sequence}: {}",
                instance.description.as_deref().unwrap_or("")
            ),
            None => instance.description.clone().unwrap_or_default(),
        };
        output.push_str(&format!(
            "  {}->>{}: {}\n",
            escape(source_name),
            escape(destination_name),
            escape(&label)
        ));
    }
    output
}

fn static_base_view<'a>(workspace: &'a Workspace, view: &'a View) -> Option<&'a View> {
    view.filter.as_ref().and_then(|filter| {
        workspace
            .views
            .iter()
            .find(|candidate| candidate.key.as_deref() == Some(&filter.base_key.identifier))
    })
}

fn relationship_pattern(value: &str) -> Option<(&str, &str)> {
    let (source, destination) = value.split_once("->")?;
    Some((source.trim(), destination.trim()))
}

fn relationship_matches(relationship: &Relationship, source: &str, destination: &str) -> bool {
    (source == "*" || relationship.source == source)
        && (destination == "*" || relationship.destination == destination)
}

fn software_system_of<'a>(workspace: &'a Workspace, identifier: &str) -> Option<&'a str> {
    let mut current = find(workspace, identifier);
    while let Some(element) = current {
        if element.kind == ElementKind::SoftwareSystem {
            return Some(&element.id);
        }
        current = element
            .parent
            .as_deref()
            .and_then(|parent| find(workspace, parent));
    }
    None
}

pub(crate) fn default_view_key(kind: &ViewKind) -> &'static str {
    match kind {
        ViewKind::SystemLandscape => "system-landscape",
        ViewKind::SystemContext => "system-context",
        ViewKind::Container => "container",
        ViewKind::Component => "component",
        ViewKind::Filtered => "filtered",
        ViewKind::Dynamic => "dynamic",
        ViewKind::Deployment => "deployment",
        ViewKind::Custom => "custom",
        ViewKind::Image => "image",
        ViewKind::ArchiMate => "archimate",
    }
}

fn view_endpoint(workspace: &Workspace, view: &View, identifier: &str) -> String {
    match (&view.kind, view.scope.as_deref()) {
        (ViewKind::SystemContext, Some(scope))
            if workspace.implied_relationships.as_deref() != Some("false")
                && is_descendant_of(workspace, identifier, scope) =>
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
        Some(element) if element.order >= view.order => diagnostics.push(
            Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!("{label} scope '{}' is a forward reference", element.id),
            )
            .with_help(format!(
                "move the {} before this view",
                element_kind_label(&kind)
            )),
        ),
        Some(element) if element.kind == kind => {}
        Some(element) => diagnostics.push(
            Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!(
                    "{label} scope '{}' has wrong kind {:?}",
                    element.id, element.kind
                ),
            )
            .with_help(format!(
                "use a {} identifier as the view scope",
                element_kind_label(&kind)
            )),
        ),
        None => diagnostics.push(
            Diagnostic::error(
                view.scope_span.unwrap_or(view.span),
                format!("{label} scope is missing or undefined"),
            )
            .with_help(format!(
                "define the {} before this view",
                element_kind_label(&kind)
            )),
        ),
    }
}

fn node_id(identifier: &str) -> String {
    identifier.replace(['.', '-'], "_")
}

fn escape(value: &str) -> String {
    value.replace('"', "'").replace('|', "/")
}

fn terminology_label<'a>(workspace: &'a Workspace, kind: &ElementKind) -> &'a str {
    let key = match kind {
        ElementKind::Person => Some("person"),
        ElementKind::SoftwareSystem => Some("softwareSystem"),
        ElementKind::Container => Some("container"),
        ElementKind::Component => Some("component"),
        ElementKind::DeploymentNode => Some("deploymentNode"),
        ElementKind::InfrastructureNode => Some("infrastructureNode"),
        ElementKind::ArchiMate(kind) => return kind,
        _ => None,
    };
    key.and_then(|key| {
        workspace
            .terminology
            .iter()
            .rev()
            .find(|property| property.key == key)
            .map(|property| property.value.as_str())
    })
    .unwrap_or_else(|| default_kind_label(kind))
}

fn element_style_tag(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::SoftwareSystem => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Generic => "Element",
        ElementKind::DeploymentEnvironment => "Deployment Environment",
        ElementKind::DeploymentGroup => "Deployment Group",
        ElementKind::DeploymentNode => "Deployment Node",
        ElementKind::InfrastructureNode => "Infrastructure Node",
        ElementKind::SoftwareSystemInstance => "Software System Instance",
        ElementKind::ContainerInstance => "Container Instance",
        ElementKind::ArchiMate(kind) => kind,
    }
}

fn default_kind_label(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::SoftwareSystem => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Generic => "Element",
        ElementKind::DeploymentEnvironment => "Deployment Environment",
        ElementKind::DeploymentGroup => "Deployment Group",
        ElementKind::DeploymentNode => "Deployment Node",
        ElementKind::InfrastructureNode => "Infrastructure Node",
        ElementKind::SoftwareSystemInstance => "Software System Instance",
        ElementKind::ContainerInstance => "Container Instance",
        ElementKind::ArchiMate(kind) => kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM_CONTEXT: &str = "flowchart LR\n  customer[\"Customer\\nPerson\"]\n  bank[\"Internet Banking System\\nSoftware System\"]\n  customer -->|Uses| bank\n";
    const CONTAINER: &str = "flowchart LR\n  customer[\"Customer\\nPerson\"]\n  bank[\"Internet Banking System\\nSoftware System\"]\n  bank_web[\"Web Application\\nContainer\"]\n  bank_api[\"API Application\\nContainer\"]\n  bank_db[\"Database\\nContainer\"]\n  customer -->|Uses| bank_web\n  bank_web -->|Calls| bank_api\n  bank_api -->|Reads/writes| bank_db\n";

    fn compile_named(path: &str, source: &str) -> Result<Workspace, String> {
        let (sources, source_id) = SourceMap::from_text(path, source);
        compile_sources(&sources, source_id)
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
        assert_eq!(
            workspace.views[0]
                .auto_layout
                .as_ref()
                .map(|layout| layout.direction.as_str()),
            Some("LR")
        );
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

    #[test]
    fn parses_and_preserves_m3_core_grammar() {
        let workspace = compile_file("tests/fixtures/m3-core.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.name.as_deref(), Some("Milestone 3"));
        assert_eq!(
            workspace.description.as_deref(),
            Some("Core grammar fixture")
        );
        assert_eq!(workspace.identifiers, "hierarchical");
        assert_eq!(workspace.implied_relationships.as_deref(), Some("false"));
        assert_eq!(workspace.enterprise.as_ref().unwrap().name, "Acme");
        assert_eq!(workspace.properties[0].key, "owner");
        assert_eq!(workspace.groups.len(), 3);
        assert!(workspace
            .preserved
            .iter()
            .any(|item| item.name == "archetypes"));
        assert!(workspace
            .preserved
            .iter()
            .any(|item| item.name == "configuration"));
        for name in [
            "docs",
            "adrs",
            "extend",
            "ref",
            "element",
            "elements",
            "relationship",
            "relationships",
            "components",
        ] {
            assert!(workspace.directives.iter().any(|item| item.name == name));
        }
        let customer = find(&workspace, "customer").unwrap();
        assert_eq!(customer.description.as_deref(), Some("A customer"));
        assert_eq!(
            customer.url.as_deref(),
            Some("https://example.test/customer")
        );
        assert_eq!(customer.properties[0].value, "Retail");
        assert_eq!(customer.perspectives[0].key, "Security");
        let relationship = &workspace.relationships[0];
        assert_eq!(relationship.description.as_deref(), Some("Uses securely"));
        assert_eq!(relationship.technology.as_deref(), Some("HTTPS"));
        assert_eq!(relationship.properties[0].key, "sla");
        assert!(!workspace.warnings.is_empty());
    }

    #[test]
    fn validates_m3_deployment_and_relationship_removal() {
        let deployment = compile_file("tests/fixtures/m3-deployment.dsl").unwrap();
        validate(&deployment).unwrap();
        assert!(deployment
            .elements
            .iter()
            .any(|element| element.kind == ElementKind::DeploymentEnvironment));
        assert!(deployment
            .elements
            .iter()
            .any(|element| element.kind == ElementKind::InfrastructureNode));
        assert_eq!(
            deployment
                .elements
                .iter()
                .filter(|element| !element.health_checks.is_empty())
                .count(),
            2
        );
        let removal = compile_file("tests/fixtures/m3-remove-relationship.dsl").unwrap();
        validate(&removal).unwrap();
        assert_eq!(removal.removed_relationships.len(), 1);
    }

    #[test]
    fn supports_local_extension_and_rejects_remote_extension() {
        let workspace = compile_file("tests/fixtures/m3-extension.dsl").unwrap();
        validate(&workspace).unwrap();
        assert!(find(&workspace, "baseUser").is_some());
        assert!(find(&workspace, "derivedSystem").is_some());
        assert!(find(&workspace, "derivedSystem.api").is_some());
        assert_eq!(workspace.identifiers, "hierarchical");
        let error = compile_file("tests/fixtures/m3-url-extension.dsl").unwrap_err();
        assert!(error.contains("remote workspace extension is disabled"));
        assert!(!error.contains("no network request"));
        let opted_in = compile_file_with_options(
            "tests/fixtures/m3-url-extension.dsl",
            CompileOptions {
                allow_network: true,
                strict_safe: false,
                strict: false,
            },
        )
        .unwrap_err();
        assert!(opted_in.contains("no network request was made"));
        let json = compile_file("tests/fixtures/m3-json-extension.dsl").unwrap_err();
        assert!(json.contains("JSON workspace extension is not implemented"));
    }

    #[test]
    fn validates_identifier_modes_hierarchy_and_declaration_order() {
        let flat = compile(
            "workspace {\n  !identifiers flat\n  model {\n    a = softwareSystem A {\n      api = container API\n    }\n    b = softwareSystem B {\n      api = container API\n    }\n  }\n}\n",
        )
        .unwrap();
        assert!(validate(&flat)
            .unwrap_err()
            .contains("duplicate identifier 'api'"));

        let hierarchical = compile(
            "workspace {\n  !identifiers hierarchical\n  model {\n    s = softwareSystem S {\n      api = container One\n      api = container Two\n    }\n  }\n}\n",
        )
        .unwrap();
        assert!(validate(&hierarchical)
            .unwrap_err()
            .contains("duplicate identifier 's.api'"));

        let hierarchy =
            compile("workspace {\n  model {\n    c = container C\n    x = component X\n  }\n}\n")
                .unwrap();
        let error = validate(&hierarchy).unwrap_err();
        assert!(error.contains("container 'c' is not allowed"));
        assert!(error.contains("component 'x' is not allowed"));

        let forward = compile(
            "workspace {\n  model {\n    user -> system Uses\n    user = person User\n    system = softwareSystem System\n  }\n}\n",
        )
        .unwrap();
        assert!(validate(&forward)
            .unwrap_err()
            .contains("forward reference"));
    }

    #[test]
    fn rejects_invalid_instances_health_checks_and_removals() {
        let workspace = compile(
            "workspace {\n  model {\n    system = softwareSystem System\n    user = person User {\n      healthCheck User /health\n    }\n    thing = element Thing Type {\n      instanceOf missing\n    }\n    env = deploymentEnvironment Production {\n      node = deploymentNode Node {\n        bad = containerInstance system\n      }\n    }\n    user -/> system Uses\n  }\n}\n",
        )
        .unwrap();
        let error = validate(&workspace).unwrap_err();
        assert!(error.contains("container instance must reference a container"));
        assert!(error.contains("healthCheck is only allowed"));
        assert!(error.contains("instanceOf reference 'missing' is not defined"));
        assert!(error.contains("has no earlier matching relationship"));

        let groups = compile(
            "workspace {\n  model {\n    system = softwareSystem System {\n      api = container API\n    }\n    one = deploymentEnvironment One {\n      blue = deploymentGroup Blue\n    }\n    two = deploymentEnvironment Two {\n      node = deploymentNode Node {\n        instance = containerInstance api blue\n      }\n    }\n  }\n}\n",
        )
        .unwrap();
        assert!(validate(&groups)
            .unwrap_err()
            .contains("is not in the same deployment environment"));
    }

    #[test]
    fn safely_rejects_scripts_plugins_and_custom_implied_strategy() {
        let unsafe_error = compile_file("tests/fixtures/m3-unsafe.dsl").unwrap_err();
        assert!(unsafe_error.contains("!script is disabled and was not executed"));
        assert!(unsafe_error.contains("!plugin is disabled and was not executed"));
        let strategy = compile(
            "workspace {\n  model {\n    !impliedRelationships com.example.Custom\n  }\n}\n",
        )
        .unwrap_err();
        assert!(strategy.contains("external classes are never loaded"));
    }

    #[test]
    fn never_silently_ignores_later_milestone_features() {
        let error = compile(
            "workspace {\n  !include other.dsl\n  styles {\n    element Person\n  }\n  views {\n    dynamic system key {\n    }\n  }\n}\n",
        )
        .unwrap_err();
        assert!(error.contains("cannot include 'other.dsl'"));
    }

    #[test]
    fn parses_validates_and_expands_all_m4_view_types() {
        let workspace = compile_file("tests/fixtures/m4-views.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.views.len(), 10);
        assert_eq!(
            workspace
                .views
                .iter()
                .map(|view| &view.kind)
                .collect::<Vec<_>>(),
            vec![
                &ViewKind::SystemLandscape,
                &ViewKind::SystemLandscape,
                &ViewKind::SystemContext,
                &ViewKind::Container,
                &ViewKind::Component,
                &ViewKind::Filtered,
                &ViewKind::Filtered,
                &ViewKind::Dynamic,
                &ViewKind::Custom,
                &ViewKind::Image,
            ]
        );

        let landscape = &workspace.views[0];
        let layout = landscape.auto_layout.as_ref().unwrap();
        assert_eq!(layout.direction, "tb");
        assert_eq!(layout.rank_separation.as_deref(), Some("100"));
        assert_eq!(layout.node_separation.as_deref(), Some("200"));
        assert!(landscape.is_default);
        assert_eq!(landscape.animations.len(), 2);
        assert_eq!(landscape.title.as_deref(), Some("Landscape title"));
        assert_eq!(
            landscape.description.as_deref(),
            Some("Landscape description")
        );
        assert_eq!(landscape.properties[0].key, "owner");
        let expanded = expand_view(&workspace, landscape);
        assert!(expanded.elements.contains("user"));
        assert!(expanded.elements.contains("system"));
        assert!(expanded.elements.contains("external"));
        assert!(!expanded.elements.contains("system.web"));

        let selected = expand_view(&workspace, &workspace.views[1]);
        assert!(selected.elements.contains("user"));
        assert!(selected.elements.contains("system"));
        assert!(!selected.elements.contains("external"));
        assert_eq!(workspace.views[3].includes[0].value, "*?");
        assert!(workspace.views[2].excludes[0].expression);

        let context = expand_view(&workspace, &workspace.views[2]);
        let filtered_include = expand_view(&workspace, &workspace.views[5]);
        let filtered_exclude = expand_view(&workspace, &workspace.views[6]);
        assert!(context.elements.contains("external"));
        assert!(filtered_include.elements.contains("system"));
        assert!(!filtered_include.elements.contains("external"));
        assert!(!filtered_exclude.elements.contains("external"));
        assert!(expand_view(&workspace, &workspace.views[4])
            .elements
            .contains("system.web.controller"));

        let dynamic = &workspace.views[7];
        assert_eq!(dynamic.dynamic_relationships.len(), 2);
        assert_eq!(
            dynamic.dynamic_relationships[0].sequence.as_deref(),
            Some("2")
        );
        assert!(mermaid(&workspace, dynamic).starts_with("sequenceDiagram\n"));
        assert_eq!(workspace.views[8].includes[0].value, "note");
        assert_eq!(workspace.views[9].image_sources.len(), 4);
        assert_eq!(workspace.view_properties[0].value, "M4");
        assert!(workspace
            .warnings
            .iter()
            .any(|warning| warning.message.contains("Mermaid")));
        assert!(!workspace
            .warnings
            .iter()
            .any(|warning| warning.message.contains("element.tag==Core")));
    }

    #[test]
    fn expands_m4_deployment_views_by_environment_and_scope() {
        let workspace = compile_file("tests/fixtures/m4-deployment.dsl").unwrap();
        validate(&workspace).unwrap();
        let all = expand_view(&workspace, &workspace.views[0]);
        assert!(all.elements.contains("production.node"));
        assert!(all.elements.contains("production.node.proxy"));
        assert!(all.elements.contains("production.node.apiInstance"));
        assert!(all.elements.contains("production.node.workerInstance"));

        let system = expand_view(&workspace, &workspace.views[1]);
        assert!(system.elements.contains("production.node"));
        assert!(!system.elements.contains("production.node.proxy"));
        assert!(system.elements.contains("production.node.apiInstance"));
        assert!(!system.elements.contains("production.node.workerInstance"));
        assert!(mermaid(&workspace, &workspace.views[1]).starts_with("flowchart LR\n"));
    }

    #[test]
    fn rejects_remote_m4_image_sources_without_fetching() {
        let error = compile_file("tests/fixtures/m4-remote-image.dsl").unwrap_err();
        assert_eq!(
            error
                .matches("remote image source is disabled and was not fetched")
                .count(),
            4
        );
        for source in ["plantuml", "mermaid", "kroki", "diagram.png"] {
            assert!(error.contains(source));
        }
    }

    #[test]
    fn reports_m4_view_validation_errors() {
        let workspace = compile(
            "workspace {\n  model {\n    user = person User\n    system = softwareSystem System {\n      api = container API\n    }\n    other = softwareSystem Other\n    production = deploymentEnvironment Production\n  }\n  views {\n    systemContext system duplicate {\n      include missing\n    }\n    container system duplicate {\n      autoLayout diagonal\n    }\n    component user wrongScope {\n      include *\n    }\n    filtered absent include Tag brokenFilter {\n    }\n    dynamic * badDynamic {\n      api -> other Uses\n    }\n    dynamic system relationshipReference {\n      unknownRelationship\n    }\n    deployment user absentEnvironment badDeployment {\n      include *\n    }\n  }\n}\n",
        )
        .unwrap();
        let error = validate(&workspace).unwrap_err();
        for message in [
            "duplicate view key 'duplicate'",
            "view selector 'missing' is not defined",
            "invalid autoLayout direction 'diagonal'",
            "component view scope 'user' has wrong kind",
            "filtered view base 'absent' is not defined",
            "dynamic element 'api' is outside the view scope",
            "dynamic relationship identifier 'unknownRelationship' cannot be resolved",
            "deployment view scope 'user' has wrong kind",
            "deployment view environment is missing or undefined",
        ] {
            assert!(
                error.contains(message),
                "missing diagnostic: {message}\n{error}"
            );
        }
    }

    #[test]
    fn parses_validates_and_exports_m5_styles() {
        let workspace = compile_file("tests/fixtures/m5-styles.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.element_styles.len(), 4);
        assert_eq!(workspace.relationship_styles.len(), 1);
        assert_eq!(workspace.themes.len(), 4);
        assert_eq!(workspace.terminology.len(), 8);
        assert!(workspace
            .element_styles
            .iter()
            .any(|style| style.mode == StyleMode::Light));
        assert!(workspace
            .element_styles
            .iter()
            .any(|style| style.mode == StyleMode::Dark));
        let primary = workspace
            .element_styles
            .iter()
            .find(|style| style.tag == "Primary")
            .unwrap();
        assert!(primary
            .values
            .iter()
            .any(|property| { property.key == "color" && property.value == "#fedcba" }));
        let branding = workspace.branding.as_ref().unwrap();
        assert_eq!(branding.logo.as_ref().unwrap().value, "assets/logo.svg");
        assert_eq!(branding.fonts[0].name, "Inter");
        assert!(workspace
            .warnings
            .iter()
            .any(|warning| warning.message.contains("not rendered by Mermaid")));
        assert_eq!(
            mermaid(&workspace, &workspace.views[0]),
            "flowchart LR\n  user(\"User\\nActor\"):::style_user\n  system[\"System\\nApplication\"]\n  user -->|Uses| system\n  classDef style_user fill:#223344,color:#fedcba,stroke:#445566,stroke-width:3px,stroke-dasharray:5 5\n  linkStyle 0 stroke:#00aa00,stroke-width:4px,stroke-dasharray:2 3\n"
        );
    }

    #[test]
    fn preserves_remote_m5_metadata_without_fetching() {
        let workspace = compile_file("tests/fixtures/m5-remote.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.themes.len(), 3);
        assert_eq!(workspace.element_styles[0].values[0].key, "icon");
        let warnings = warnings(&workspace).unwrap();
        for message in [
            "remote icon URL was preserved but was not fetched",
            "remote theme URL was preserved but was not fetched",
            "remote branding logo was preserved but was not fetched",
            "remote branding font was preserved but was not fetched",
        ] {
            assert!(warnings.contains(message));
        }
    }

    #[test]
    fn reports_invalid_m5_style_values() {
        let workspace = compile_file("tests/fixtures/m5-invalid.dsl").unwrap();
        let error = validate(&workspace).unwrap_err();
        for message in [
            "invalid shape 'Blob'",
            "width must be an integer",
            "strokeWidth must be between 1 and 10",
            "invalid border 'double'",
            "opacity must be between 0 and 100",
            "invalid boolean 'perhaps'",
            "thickness must be an integer",
            "invalid relationship style 'broken'",
            "invalid routing 'ZigZag'",
            "position must be between 0 and 100",
            "invalid metadata mode 'triangle'",
        ] {
            assert!(
                error.contains(message),
                "missing diagnostic: {message}\n{error}"
            );
        }
    }

    #[test]
    fn preprocesses_m6_includes_constants_and_expressions() {
        let workspace = compile_file("tests/fixtures/m6-preprocessing.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.name.as_deref(), Some("Milestone 6 Workspace"));
        assert_eq!(workspace.elements[0].id, "user");
        assert_eq!(workspace.elements[1].id, "system");
        assert_eq!(workspace.elements[2].id, "api");
        assert_eq!(workspace.dependencies.len(), 4);
        assert!(workspace.dependencies[0].to.ends_with("01-user.dsl"));
        assert!(workspace.dependencies[1].to.ends_with("02-system.dsl"));
        assert_eq!(workspace.elements[0].tags, ["External"]);
        let diagram = mermaid(&workspace, &workspace.views[0]);
        assert!(diagram.contains("user[\"User\\nPerson\"]"));
        assert!(diagram.contains("system[\"System\\nSoftware System\"]"));
        assert!(!diagram.contains("Included container"));
    }

    #[test]
    fn substitutes_m6_constants_and_rejects_invalid_definitions() {
        let workspace = compile(
            "!constant NAME \"Constant Workspace\"\n!constant TECH Rust\nworkspace \"${NAME}\" {\n  model {\n    system = softwareSystem System {\n      api = container API Backend ${TECH}\n    }\n  }\n}\n",
        )
        .unwrap();
        assert_eq!(workspace.name.as_deref(), Some("Constant Workspace"));
        assert_eq!(workspace.elements[1].technology.as_deref(), Some("Rust"));

        for (source, message) in [
            (
                "workspace \"${MISSING}\" {\n}\n",
                "undefined constant 'MISSING'",
            ),
            (
                "!constant A one\n!constant A two\nworkspace {\n}\n",
                "duplicate constant 'A'",
            ),
            (
                "!constant A ${B}\n!constant B ${A}\nworkspace {\n}\n",
                "recursive constant detected",
            ),
        ] {
            assert!(compile(source).unwrap_err().contains(message));
        }
    }

    #[test]
    fn parses_m83_archimate_profile() {
        let workspace =
            compile(include_str!("../tests/fixtures/m83-archimate-profile.dsl")).unwrap();
        validate(&workspace).unwrap();
        assert!(workspace
            .elements
            .iter()
            .any(|element| element.id == "gateway"
                && element.kind == ElementKind::ArchiMate("ApplicationComponent")));
        assert_eq!(
            relationship_archimate_type(&workspace.relationships[0]),
            Some("FlowRelationship")
        );
        assert_eq!(workspace.views[0].kind, ViewKind::ArchiMate);
        assert!(workspace.views[0]
            .properties
            .iter()
            .any(|property| property.key == "object.gateway.x" && property.value == "300"));
    }

    #[test]
    fn rejects_invalid_m83_archimate_profile_data() {
        for (source, message) in [
            (
                "workspace {\n  model {\n    archimate {\n      a = businessActor A {\n        background red\n      }\n    }\n  }\n}\n",
                "background must be a #RRGGBB color",
            ),
            (
                "workspace {\n  model {\n    archimate {\n      a = businessActor A\n      b = applicationComponent B\n      a -> b {\n        type UnknownRelationship\n      }\n    }\n  }\n}\n",
                "unknown ArchiMate relationship type",
            ),
            (
                "workspace {\n  model {\n    archimate {\n      a = businessActor A\n    }\n  }\n  views {\n    archimateView v {\n      include *\n      object missing {\n        x 1\n      }\n    }\n  }\n}\n",
                "archimateView object 'missing' is not included",
            ),
        ] {
            let workspace = compile(source).unwrap();
            assert!(validate(&workspace).unwrap_err().contains(message));
        }
        assert!(compile(
            "workspace {\n  model {\n    archimate {\n      a = noSuchType A\n    }\n  }\n}\n"
        )
        .unwrap_err()
        .contains("expected an ArchiMate element type"));
    }

    #[test]
    fn reports_m6_include_failures_and_included_source_locations() {
        assert!(compile_file("tests/fixtures/m6-missing.dsl")
            .unwrap_err()
            .contains("cannot include"));
        assert!(compile_file("tests/fixtures/m6-cycle-a.dsl")
            .unwrap_err()
            .contains("include cycle detected"));
        let remote = compile_file("tests/fixtures/m6-remote.dsl").unwrap_err();
        assert!(remote.contains("remote include is disabled"));
        assert!(remote.contains("no network request was made"));
        let opted_in = compile_file_with_options(
            "tests/fixtures/m6-remote.dsl",
            CompileOptions {
                allow_network: true,
                strict_safe: false,
                strict: false,
            },
        )
        .unwrap_err();
        assert!(opted_in.contains("fetching is not implemented"));
        assert!(opted_in.contains("no network request was made"));

        let workspace = compile_file("tests/fixtures/m6-invalid-include.dsl").unwrap();
        let error = validate(&workspace).unwrap_err();
        assert!(error.contains("m6-invalid-part.dsl:1:"), "{error}");
        assert!(error.contains("relationship source 'missing' is not defined"));

        let directory = std::env::temp_dir().join(format!(
            "c4c-m6-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("invalid.dsl"), [0xff]).unwrap();
        fs::write(
            directory.join("workspace.dsl"),
            "workspace {\n  !include invalid.dsl\n}\n",
        )
        .unwrap();
        let non_utf8 =
            compile_file(&directory.join("workspace.dsl").to_string_lossy()).unwrap_err();
        fs::remove_dir_all(&directory).unwrap();
        assert!(non_utf8.contains("is not valid UTF-8"));
    }

    #[test]
    fn validates_m84_archimate_semantics_and_strict_mode() {
        let workspace = compile_file("tests/fixtures/m84-archimate-conformance.dsl").unwrap();
        validate(&workspace).unwrap();
        validate_with_options(
            &workspace,
            ValidationOptions {
                strict_archimate: true,
            },
        )
        .unwrap();
        assert_eq!(archimate_element_layer("Equipment"), Some("physical"));
        assert_eq!(archimate_element_role("DataObject"), Some("passive"));
        assert_eq!(
            archimate_relationship_open_group_type("AccessRelationship"),
            Some("Access")
        );
        let access = workspace
            .relationships
            .iter()
            .find(|relationship| relationship.destination == "customerRecord")
            .unwrap();
        assert_eq!(relationship_access_direction(access), Some("read"));
        assert_eq!(
            workspace.relationships[0].id.as_deref(),
            Some("rAssignment")
        );
        let junction = find(&workspace, "andJoin").unwrap();
        assert_eq!(archimate_element_keyword("Junction"), Some("junction"));
        assert!(junction
            .attributes
            .iter()
            .any(|property| property.key == "kind" && property.value == "and"));
        let inspected = inspect(&workspace);
        assert!(inspected.contains("archimateType=Some(\"AssignmentRelationship\")"));
        assert!(inspected.contains("viewpoint=Some(\"applicationCooperation\")"));

        let invalid = compile_file("tests/fixtures/m84-invalid-archimate.dsl").unwrap();
        let warnings = warnings(&invalid).unwrap();
        assert!(warnings.contains("FlowRelationship"), "{warnings}");
        let error = validate(&invalid).unwrap_err();
        assert!(error.contains("AccessRelationship direction"), "{error}");
        assert!(error.contains("ArchiMate viewpoint"), "{error}");
        let strict = validate_with_options(
            &invalid,
            ValidationOptions {
                strict_archimate: true,
            },
        )
        .unwrap_err();
        assert!(strict.contains("questionable"), "{strict}");
    }

    #[test]
    fn evaluates_m6_boolean_and_property_expressions() {
        let workspace = compile(
            "workspace {\n  model {\n    keep = person Keep {\n      tags Keep\n      properties {\n        tier one\n      }\n    }\n    skip = person Skip {\n      tags Skip\n    }\n    system = softwareSystem System\n    keep -> system Uses \"\" Async\n  }\n  views {\n    systemLandscape selected {\n      include ( element.tag==Keep && ! element.tag==Skip ) || element.properties.tier==one\n      exclude relationship.tag==Async\n    }\n  }\n}\n",
        )
        .unwrap();
        validate(&workspace).unwrap();
        let expanded = expand_view(&workspace, &workspace.views[0]);
        assert_eq!(expanded.elements, HashSet::from(["keep".into()]));
        assert!(expanded.relationships.is_empty());

        let unsupported = compile(
            "workspace {\n  model {\n    keep = person Keep\n  }\n  views {\n    systemLandscape bad {\n      include element.name==Keep\n    }\n  }\n}\n",
        )
        .unwrap();
        assert!(warnings(&unsupported).unwrap().contains("not evaluated"));
        assert!(validate(&unsupported)
            .unwrap_err()
            .contains("unsupported expression field 'name'"));
    }

    #[test]
    fn rejects_m6_executable_and_strict_unsafe_constructs() {
        let error = compile_file("tests/fixtures/m6-unsafe.dsl").unwrap_err();
        assert_eq!(error.matches("disabled and was not executed").count(), 4);
        let strict = compile_file_with_options(
            "tests/fixtures/m6-unsafe.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
                strict: false,
            },
        )
        .unwrap_err();
        assert!(strict.contains("strict-safe mode rejects scripts and plugins"));
        let assets = compile_file_with_options(
            "tests/fixtures/m5-remote.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
                strict: false,
            },
        )
        .unwrap_err();
        for message in ["theme", "icon", "branding logos", "branding fonts"] {
            assert!(assets.contains(message), "{assets}");
        }
    }

    #[test]
    fn honors_boolean_implied_relationship_behavior() {
        let source = "workspace {\n  !identifiers hierarchical\n  model {\n    user = person User\n    system = softwareSystem System {\n      api = container API\n    }\n    user -> system.api Uses\n  }\n  views {\n    systemContext system context {\n      include *\n    }\n  }\n}\n";
        let enabled = compile(source).unwrap();
        assert!(mermaid(&enabled, &enabled.views[0]).contains("user -->|Uses| system"));
        let disabled =
            compile(&source.replace("  model {", "  model {\n    !impliedRelationships false"))
                .unwrap();
        assert!(!mermaid(&disabled, &disabled.views[0]).contains("Uses"));
    }
}
