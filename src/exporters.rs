use crate::compiler::{
    self, DocumentationOwner, Element, ElementKind, Property, Relationship, StyleMode, View,
    ViewKind, Workspace,
};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub struct ExportOptions {
    pub strict_safe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Mermaid,
    D2,
    PlantUml,
    C4PlantUml,
    Dot,
    DrawIo,
    ArchiMate,
    ArchiNative,
    Html,
    Svg,
    Png,
}

struct ExportArtifact {
    path: PathBuf,
    content: Vec<u8>,
}

struct ArchiNativeConnection {
    id: String,
    source_element: String,
    destination_element: String,
    source_object: String,
    destination_object: String,
    relationship_id: String,
    description: String,
    synthetic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArchiNativeBounds {
    x: isize,
    y: isize,
    width: isize,
    height: isize,
}

struct ArchiNativeLayout {
    objects: HashMap<String, ArchiNativeBounds>,
    groups: HashMap<usize, ArchiNativeBounds>,
}

impl Deref for ArchiNativeLayout {
    type Target = HashMap<String, ArchiNativeBounds>;

    fn deref(&self) -> &Self::Target {
        &self.objects
    }
}

pub fn export(
    workspace: &Workspace,
    format: &str,
    output: &Path,
    options: ExportOptions,
) -> Result<(), String> {
    let format = ExportFormat::parse(format)?;
    fs::create_dir_all(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    match format {
        ExportFormat::Mermaid => return compiler::export_mermaid(workspace, output),
        ExportFormat::Html => return compiler::export_site(workspace, output),
        ExportFormat::Svg => return render_graphviz(workspace, output, "svg", options.strict_safe),
        ExportFormat::Png => return render_graphviz(workspace, output, "png", options.strict_safe),
        _ => {}
    }
    let artifacts = match format {
        ExportFormat::Json => vec![artifact("workspace.json", json(workspace))],
        ExportFormat::D2 => view_artifacts(workspace, "d2", d2),
        ExportFormat::PlantUml => view_artifacts(workspace, "puml", plantuml),
        ExportFormat::C4PlantUml => view_artifacts(workspace, "puml", c4plantuml),
        ExportFormat::Dot => view_artifacts(workspace, "dot", dot),
        ExportFormat::DrawIo => view_artifacts(workspace, "drawio", drawio),
        ExportFormat::ArchiMate => vec![artifact("workspace.archimate.xml", archimate(workspace))],
        ExportFormat::ArchiNative => vec![artifact("workspace.archimate", archi_native(workspace))],
        _ => unreachable!(),
    };
    write_artifacts(output, artifacts)
}

pub fn export_with_archi_sidecar(
    workspace: &Workspace,
    format: &str,
    output: &Path,
    _options: ExportOptions,
    sidecar: &Path,
    projection: &Path,
) -> Result<(), String> {
    if ExportFormat::parse(format)? != ExportFormat::ArchiNative {
        return Err("--archi-sidecar is only valid with --format archi".into());
    }
    fs::create_dir_all(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    let content = match crate::archi_native::sidecar_xml(sidecar, projection)? {
        Some(xml) => xml,
        None => {
            eprintln!(
                "warning: Archi sidecar does not match the DSL projection; generated native IDs and layout will be used"
            );
            archi_native(workspace)
        }
    };
    write_artifacts(output, vec![artifact("workspace.archimate", content)])
}

impl ExportFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "json" | "structurizr-json" => Ok(Self::Json),
            "mermaid" | "mmd" => Ok(Self::Mermaid),
            "d2" => Ok(Self::D2),
            "plantuml" | "puml" => Ok(Self::PlantUml),
            "c4plantuml" | "c4-plantuml" => Ok(Self::C4PlantUml),
            "dot" | "graphviz" => Ok(Self::Dot),
            "drawio" | "draw.io" => Ok(Self::DrawIo),
            "archimate" | "archimate-xml" | "opengroup-archimate" => Ok(Self::ArchiMate),
            "archi" | "archi-native" | "archimate-native" => Ok(Self::ArchiNative),
            "html" | "site" => Ok(Self::Html),
            "svg" => Ok(Self::Svg),
            "png" => Ok(Self::Png),
            _ => Err(format!(
                "unsupported export format '{value}'; supported: json, mermaid, d2, plantuml, c4plantuml, dot, drawio, archimate, archi, html, svg, png"
            )),
        }
    }
}

fn artifact(path: &str, content: String) -> ExportArtifact {
    ExportArtifact {
        path: PathBuf::from(path),
        content: content.into_bytes(),
    }
}

fn view_artifacts(
    workspace: &Workspace,
    extension: &str,
    render: fn(&Workspace, &View) -> String,
) -> Vec<ExportArtifact> {
    workspace
        .views
        .iter()
        .map(|view| {
            artifact(
                &format!("{}.{}", safe_name(view_key(view)), extension),
                render(workspace, view),
            )
        })
        .collect()
}

fn write_artifacts(output: &Path, artifacts: Vec<ExportArtifact>) -> Result<(), String> {
    for artifact in artifacts {
        let path = output.join(&artifact.path);
        fs::write(&path, artifact.content)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn view_key(view: &View) -> &str {
    view.key
        .as_deref()
        .unwrap_or_else(|| compiler::default_view_key(&view.kind))
}

fn safe_name(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            output.push(character);
        } else {
            output.push('_');
            output.push_str(&format!("{:x}", character as u32));
        }
    }
    if output.is_empty() {
        "view".into()
    } else {
        output
    }
}

fn d2(workspace: &Workspace, view: &View) -> String {
    let graph = compiler::view_graph(workspace, view);
    let mut output = format!(
        "# Generated locally by c4c; no network renderer is required\n# {} view: {}\ndirection: {}\n",
        view_kind_name(&view.kind),
        quoted(view.title.as_deref().unwrap_or(view_key(view))),
        d2_direction(view)
    );
    append_style_note(workspace, &mut output, "#");
    for identifier in &graph.element_ids {
        if let Some(element) = find(workspace, identifier) {
            output.push_str(&format!(
                "{}: {}\n",
                alias(identifier),
                quoted(&format!("{}\n{}", element.name, kind_name(&element.kind)))
            ));
        }
    }
    for relationship in &graph.relationships {
        output.push_str(&format!(
            "{} -> {}: {}\n",
            alias(&relationship.source),
            alias(&relationship.destination),
            quoted(&relationship.description)
        ));
    }
    output
}

fn d2_direction(view: &View) -> &'static str {
    match view
        .auto_layout
        .as_ref()
        .map(|layout| layout.direction.to_ascii_lowercase())
        .as_deref()
    {
        Some("tb" | "bt") => "down",
        _ => "right",
    }
}

fn plantuml(workspace: &Workspace, view: &View) -> String {
    let graph = compiler::view_graph(workspace, view);
    let mut output = String::from("@startuml\n' Generated locally by c4c; no remote includes\n");
    output.push_str(&format!(
        "title {}\n",
        quoted(view.title.as_deref().unwrap_or(view_key(view)))
    ));
    append_style_note(workspace, &mut output, "'");
    if view.kind != ViewKind::Dynamic {
        output.push_str("left to right direction\n");
    }
    for identifier in &graph.element_ids {
        let Some(element) = find(workspace, identifier) else {
            continue;
        };
        let declaration = if view.kind == ViewKind::Dynamic {
            if element.kind == ElementKind::Person {
                "actor"
            } else {
                "participant"
            }
        } else {
            match element.kind {
                ElementKind::Person => "actor",
                ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "node",
                _ => "rectangle",
            }
        };
        output.push_str(&format!(
            "{declaration} {} as {}\n",
            quoted(&format!("{}\n{}", element.name, kind_name(&element.kind))),
            alias(identifier)
        ));
    }
    let arrow = if view.kind == ViewKind::Dynamic {
        "->"
    } else {
        "-->"
    };
    for relationship in &graph.relationships {
        output.push_str(&format!(
            "{} {arrow} {} : {}\n",
            alias(&relationship.source),
            alias(&relationship.destination),
            plantuml_label(&relationship.description)
        ));
    }
    output.push_str("@enduml\n");
    output
}

fn c4plantuml(workspace: &Workspace, view: &View) -> String {
    let graph = compiler::view_graph(workspace, view);
    let mut output = String::from(
        "@startuml\n' Local macro subset generated by c4c; no remote includes or network dependency\n!define Person(alias,label,description) actor label as alias <<Person>>\n!define System(alias,label,description) rectangle label as alias <<Software System>>\n!define Container(alias,label,technology,description) rectangle label as alias <<Container>>\n!define Component(alias,label,technology,description) rectangle label as alias <<Component>>\n!define Element(alias,label,description) rectangle label as alias <<Element>>\n!define Rel(source,destination,label) source --> destination : label\nleft to right direction\n",
    );
    output.push_str(&format!(
        "title {}\n",
        quoted(view.title.as_deref().unwrap_or(view_key(view)))
    ));
    append_style_note(workspace, &mut output, "'");
    for identifier in &graph.element_ids {
        let Some(element) = find(workspace, identifier) else {
            continue;
        };
        let name = quoted(&element.name);
        let description = quoted(element.description.as_deref().unwrap_or(""));
        let line = match element.kind {
            ElementKind::Person => format!("Person({}, {name}, {description})", alias(identifier)),
            ElementKind::SoftwareSystem => {
                format!("System({}, {name}, {description})", alias(identifier))
            }
            ElementKind::Container => format!(
                "Container({}, {name}, {}, {description})",
                alias(identifier),
                quoted(element.technology.as_deref().unwrap_or(""))
            ),
            ElementKind::Component => format!(
                "Component({}, {name}, {}, {description})",
                alias(identifier),
                quoted(element.technology.as_deref().unwrap_or(""))
            ),
            _ => format!("Element({}, {name}, {description})", alias(identifier)),
        };
        output.push_str(&line);
        output.push('\n');
    }
    for relationship in &graph.relationships {
        output.push_str(&format!(
            "Rel({}, {}, {})\n",
            alias(&relationship.source),
            alias(&relationship.destination),
            quoted(&relationship.description)
        ));
    }
    output.push_str("@enduml\n");
    output
}

fn dot(workspace: &Workspace, view: &View) -> String {
    let graph = compiler::view_graph(workspace, view);
    let mut output = format!(
        "digraph {} {{\n  rankdir={};\n  graph [label={}, labelloc=t];\n",
        quoted(view_key(view)),
        dot_direction(view),
        quoted(view.title.as_deref().unwrap_or(view_key(view)))
    );
    if has_styles(workspace) {
        output.push_str("  // c4c: Mermaid supports M5 styles; this DOT exporter uses deterministic defaults.\n");
    }
    for identifier in &graph.element_ids {
        if let Some(element) = find(workspace, identifier) {
            output.push_str(&format!(
                "  {} [label={}, shape={}];\n",
                quoted(identifier),
                quoted(&format!("{}\n{}", element.name, kind_name(&element.kind))),
                dot_shape(&element.kind)
            ));
        }
    }
    for relationship in &graph.relationships {
        output.push_str(&format!(
            "  {} -> {} [label={}];\n",
            quoted(&relationship.source),
            quoted(&relationship.destination),
            quoted(&relationship.description)
        ));
    }
    output.push_str("}\n");
    output
}

fn dot_direction(view: &View) -> &'static str {
    match view
        .auto_layout
        .as_ref()
        .map(|layout| layout.direction.to_ascii_lowercase())
        .as_deref()
    {
        Some("tb") => "TB",
        Some("bt") => "BT",
        Some("rl") => "RL",
        _ => "LR",
    }
}

fn dot_shape(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "oval",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "box3d",
        ElementKind::ArchiMate("Node" | "Device" | "SystemSoftware") => "box3d",
        _ => "box",
    }
}

fn drawio(workspace: &Workspace, view: &View) -> String {
    let graph = compiler::view_graph(workspace, view);
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<mxfile host=\"c4c\" version=\"M8\"><diagram id=\"{}\" name=\"{}\"><mxGraphModel dx=\"1200\" dy=\"800\" grid=\"1\" gridSize=\"10\" page=\"1\" pageWidth=\"1169\" pageHeight=\"827\"><root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>\n",
        xml_attr(&safe_name(view_key(view))),
        xml_attr(view.title.as_deref().unwrap_or(view_key(view)))
    );
    if has_styles(workspace) {
        output.push_str(
            "<!-- c4c: Mermaid supports M5 styles; Draw.io uses deterministic defaults. -->\n",
        );
    }
    for (position, identifier) in graph.element_ids.iter().enumerate() {
        let Some(element) = find(workspace, identifier) else {
            continue;
        };
        let x = 40 + (position % 4) * 220;
        let y = 40 + (position / 4) * 140;
        output.push_str(&format!(
            "<mxCell id=\"{}\" value=\"{}&#10;{}\" style=\"rounded=1;whiteSpace=wrap;html=0;\" vertex=\"1\" parent=\"1\"><mxGeometry x=\"{x}\" y=\"{y}\" width=\"180\" height=\"80\" as=\"geometry\"/></mxCell>\n",
            xml_attr(&cell_id(workspace, identifier)),
            xml_attr(&element.name),
            xml_attr(kind_name(&element.kind))
        ));
    }
    for (position, relationship) in graph.relationships.iter().enumerate() {
        output.push_str(&format!(
            "<mxCell id=\"edge-{}\" value=\"{}\" style=\"edgeStyle=orthogonalEdgeStyle;rounded=0;html=0;endArrow=block;\" edge=\"1\" parent=\"1\" source=\"{}\" target=\"{}\"><mxGeometry relative=\"1\" as=\"geometry\"/></mxCell>\n",
            relationship
                .relationship_index
                .map_or_else(|| format!("dynamic-{}", position + 1), |index| (index + 1).to_string()),
            xml_attr(&relationship.description),
            xml_attr(&cell_id(workspace, &relationship.source)),
            xml_attr(&cell_id(workspace, &relationship.destination))
        ));
    }
    output.push_str("</root></mxGraphModel></diagram></mxfile>\n");
    output
}

fn cell_id(workspace: &Workspace, identifier: &str) -> String {
    workspace
        .elements
        .iter()
        .position(|element| element.id == identifier)
        .map(|index| format!("element-{}", index + 1))
        .unwrap_or_else(|| format!("element-{}", alias(identifier)))
}

fn archimate(workspace: &Workspace) -> String {
    const DEFINITIONS: &[(&str, &str)] = &[
        ("id", "c4c.id"),
        ("kind", "c4c.kind"),
        ("parent", "c4c.parent"),
        ("technology", "c4c.technology"),
        ("tags", "c4c.tags"),
        ("description", "c4c.description"),
        ("source", "c4c.source"),
    ];
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model xmlns=\"http://www.opengroup.org/xsd/archimate/3.0/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" identifier=\"id-c4c-workspace\">\n",
    );
    output.push_str(&format!(
        "  <name xml:lang=\"en\">{}</name>\n",
        xml_text(workspace.name.as_deref().unwrap_or("Workspace"))
    ));
    if let Some(description) = &workspace.description {
        output.push_str(&format!(
            "  <documentation xml:lang=\"en\">{}</documentation>\n",
            xml_text(description)
        ));
    }
    output.push_str("  <elements>\n");
    for (index, element) in workspace.elements.iter().enumerate() {
        output.push_str(&format!(
            "    <element identifier=\"{}\" xsi:type=\"{}\">\n      <name xml:lang=\"en\">{}</name>\n",
            archimate_element_id(index, element),
            archimate_type(&element.kind),
            xml_text(&element.name)
        ));
        if let Some(description) = &element.description {
            output.push_str(&format!(
                "      <documentation xml:lang=\"en\">{}</documentation>\n",
                xml_text(description)
            ));
        }
        let (source, _) = workspace.source_map.resolve(element.span);
        let tags = (!element.tags.is_empty()).then(|| element.tags.join(","));
        let mut properties = vec![
            ("id", Some(element.id.as_str())),
            ("kind", Some(kind_name(&element.kind))),
            ("parent", element.parent.as_deref()),
            ("technology", element.technology.as_deref()),
            ("tags", tags.as_deref()),
            ("description", element.description.as_deref()),
            ("source", source.path.to_str()),
        ];
        for key in [
            "background",
            "color",
            "stroke",
            "fontSize",
            "width",
            "height",
            "kind",
        ] {
            properties.push((key, property_value(&element.attributes, key)));
        }
        append_archimate_properties(&mut output, &properties, 6);
        output.push_str("    </element>\n");
    }
    output.push_str("  </elements>\n  <relationships>\n");
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        let Some(source_index) = element_index(workspace, &relationship.source) else {
            continue;
        };
        let Some(destination_index) = element_index(workspace, &relationship.destination) else {
            continue;
        };
        let relationship_id = archimate_relationship_id(index, relationship);
        let access_type = (open_group_relationship_type(relationship) == "Access")
            .then(|| compiler::relationship_access_direction(relationship))
            .flatten()
            .map(open_group_access_type);
        output.push_str(&format!(
            "    <relationship identifier=\"{}\" source=\"{}\" target=\"{}\" xsi:type=\"{}\"{}>\n",
            relationship_id,
            archimate_element_id(source_index, &workspace.elements[source_index]),
            archimate_element_id(destination_index, &workspace.elements[destination_index]),
            open_group_relationship_type(relationship),
            access_type
                .map(|value| format!(" accessType=\"{}\"", xml_attr(value)))
                .unwrap_or_default()
        ));
        if let Some(description) = &relationship.description {
            output.push_str(&format!(
                "      <name xml:lang=\"en\">{}</name>\n",
                xml_text(description)
            ));
        }
        let (source_file, _) = workspace.source_map.resolve(relationship.span);
        let relationship_property_id = relationship
            .id
            .clone()
            .unwrap_or_else(|| format!("relationship-{}", index + 1));
        let properties = [
            ("id", Some(relationship_property_id)),
            ("kind", Some("Relationship".into())),
            ("parent", None),
            ("technology", relationship.technology.clone()),
            (
                "tags",
                (!relationship.tags.is_empty()).then(|| relationship.tags.join(",")),
            ),
            ("description", relationship.description.clone()),
            (
                "source",
                source_file
                    .path
                    .to_str()
                    .map(std::string::ToString::to_string),
            ),
            (
                "type",
                compiler::relationship_archimate_type(relationship).map(str::to_string),
            ),
            (
                "access",
                compiler::relationship_access_direction(relationship).map(str::to_string),
            ),
            (
                "color",
                property_value(&relationship.attributes, "color").map(str::to_string),
            ),
            (
                "thickness",
                property_value(&relationship.attributes, "thickness").map(str::to_string),
            ),
            (
                "style",
                property_value(&relationship.attributes, "style").map(str::to_string),
            ),
        ];
        let borrowed = properties
            .iter()
            .map(|(key, value)| (*key, value.as_deref()))
            .collect::<Vec<_>>();
        append_archimate_properties(&mut output, &borrowed, 6);
        output.push_str("    </relationship>\n");
    }
    output.push_str("  </relationships>\n  <propertyDefinitions>\n");
    let mut definitions = DEFINITIONS.to_vec();
    for (id, name) in archimate_extra_property_definitions(workspace) {
        definitions.push((id, name));
    }
    for (id, name) in definitions {
        output.push_str(&format!(
            "    <propertyDefinition identifier=\"property-{id}\" type=\"string\"><name xml:lang=\"en\">{name}</name></propertyDefinition>\n"
        ));
    }
    output.push_str("  </propertyDefinitions>\n</model>\n");
    output
}

fn archimate_extra_property_definitions(
    workspace: &Workspace,
) -> Vec<(&'static str, &'static str)> {
    let mut keys = Vec::new();
    let mut add = |key: &'static str, present: bool| {
        if present && !keys.iter().any(|(candidate, _)| candidate == &key) {
            keys.push((
                key,
                match key {
                    "type" => "c4c.archimate.type",
                    "background" => "c4c.archimate.background",
                    "color" => "c4c.archimate.color",
                    "stroke" => "c4c.archimate.stroke",
                    "fontSize" => "c4c.archimate.fontSize",
                    "width" => "c4c.archimate.width",
                    "height" => "c4c.archimate.height",
                    "thickness" => "c4c.archimate.thickness",
                    "style" => "c4c.archimate.style",
                    "access" => "c4c.archimate.access",
                    "kind" => "c4c.archimate.kind",
                    _ => "c4c.archimate",
                },
            ));
        }
    };
    for element in &workspace.elements {
        for key in [
            "background",
            "color",
            "stroke",
            "fontSize",
            "width",
            "height",
            "kind",
        ] {
            add(key, property_value(&element.attributes, key).is_some());
        }
    }
    for relationship in &workspace.relationships {
        add(
            "type",
            compiler::relationship_archimate_type(relationship).is_some(),
        );
        for key in ["access", "color", "thickness", "style"] {
            add(key, property_value(&relationship.attributes, key).is_some());
        }
    }
    keys
}

fn append_archimate_properties(
    output: &mut String,
    properties: &[(&str, Option<&str>)],
    indent: usize,
) {
    let properties = properties
        .iter()
        .filter_map(|(key, value)| {
            value
                .filter(|value| !value.is_empty())
                .map(|value| (*key, value))
        })
        .collect::<Vec<_>>();
    if properties.is_empty() {
        return;
    }
    let spaces = " ".repeat(indent);
    output.push_str(&format!("{spaces}<properties>\n"));
    for (key, value) in properties {
        output.push_str(&format!(
            "{spaces}  <property propertyDefinitionRef=\"property-{key}\"><value xml:lang=\"en\">{}</value></property>\n",
            xml_text(value)
        ));
    }
    output.push_str(&format!("{spaces}</properties>\n"));
}

fn archimate_element_id(index: usize, element: &Element) -> String {
    format!("id-element-{}-{}", index + 1, xml_identifier(&element.id))
}

fn archimate_relationship_id(index: usize, relationship: &Relationship) -> String {
    relationship.id.as_ref().map_or_else(
        || format!("id-relationship-{}", index + 1),
        |id| format!("id-relationship-{}", xml_identifier(id)),
    )
}

fn open_group_relationship_type(relationship: &Relationship) -> &str {
    compiler::relationship_archimate_type(relationship)
        .and_then(compiler::archimate_relationship_open_group_type)
        .unwrap_or("Association")
}

fn open_group_access_type(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "read" => "Read",
        "write" => "Write",
        "readwrite" => "ReadWrite",
        _ => "Access",
    }
}

fn archimate_type(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "BusinessActor",
        ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup => "Grouping",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "Node",
        ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "ApplicationComponent",
        ElementKind::ArchiMate(kind) => kind,
    }
}

fn archi_native(workspace: &Workspace) -> String {
    const FOLDERS: &[(&str, &str)] = &[
        ("Strategy", "strategy"),
        ("Business", "business"),
        ("Application", "application"),
        ("Technology & Physical", "technology"),
        ("Motivation", "motivation"),
        ("Implementation & Migration", "implementation_migration"),
        ("Other", "other"),
        ("Relations", "relations"),
        ("Views", "diagrams"),
    ];
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<archimate:model xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:archimate=\"http://www.archimatetool.com/archimate\" name=\"{}\" id=\"id-c4c-model\" version=\"5.0.0\">\n",
        xml_attr(workspace.name.as_deref().unwrap_or("Workspace"))
    );
    for (name, folder_type) in FOLDERS {
        output.push_str(&format!(
            "  <folder name=\"{}\" id=\"id-c4c-folder-{folder_type}\" type=\"{folder_type}\">\n",
            xml_attr(name)
        ));
        for element in workspace.elements.iter().filter(|element| {
            archi_native_folder(&element.kind) == *folder_type
                && !element.tags.iter().any(|tag| tag == "c4c_archi_synthetic")
        }) {
            output.push_str(&format!(
                "    <element xsi:type=\"archimate:{}\" name=\"{}\" id=\"{}\"",
                archi_native_type(&element.kind),
                xml_attr(&archi_native_element_name(element)),
                archi_native_element_id(&element.id)
            ));
            if let Some(description) = &element.description {
                output.push_str(&format!(
                    ">\n      <documentation>{}</documentation>\n    </element>\n",
                    xml_text(description)
                ));
            } else {
                output.push_str("/>\n");
            }
        }
        if *folder_type == "relations" {
            append_archi_native_relationships(&mut output, workspace);
        } else if *folder_type == "diagrams" {
            append_archi_native_views(&mut output, workspace);
        }
        output.push_str("  </folder>\n");
    }
    if let Some(description) = &workspace.description {
        output.push_str(&format!("  <purpose>{}</purpose>\n", xml_text(description)));
    }
    output.push_str("</archimate:model>\n");
    output
}

fn append_archi_native_relationships(output: &mut String, workspace: &Workspace) {
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        append_archi_native_relationship(
            output,
            &archi_native_relationship_id(index, relationship),
            &relationship.source,
            &relationship.destination,
            relationship.description.as_deref(),
            archi_native_relationship_type(relationship),
        );
    }
    for view in &workspace.views {
        let graph = compiler::view_graph(workspace, view);
        let objects = archi_native_object_map(workspace, view, &graph);
        for connection in archi_native_connections(workspace, view, &graph, &objects) {
            if connection.synthetic {
                append_archi_native_relationship(
                    output,
                    &connection.relationship_id,
                    &connection.source_element,
                    &connection.destination_element,
                    Some(&connection.description),
                    "AssociationRelationship",
                );
            }
        }
    }
}

fn append_archi_native_relationship(
    output: &mut String,
    id: &str,
    source: &str,
    destination: &str,
    description: Option<&str>,
    native_type: &str,
) {
    output.push_str(&format!(
        "    <element xsi:type=\"archimate:{native_type}\" id=\"{}\" source=\"{}\" target=\"{}\"",
        xml_attr(id),
        archi_native_element_id(source),
        archi_native_element_id(destination)
    ));
    if let Some(description) = description.filter(|description| !description.is_empty()) {
        output.push_str(&format!(" name=\"{}\"", xml_attr(description)));
    }
    output.push_str("/>\n");
}

fn archi_native_relationship_type(relationship: &Relationship) -> &str {
    compiler::relationship_archimate_type(relationship).unwrap_or("AssociationRelationship")
}

fn archi_native_element_name(element: &Element) -> String {
    let trimmed = element.name.trim();
    if trimmed.is_empty() {
        format!("{} {}", archi_native_type(&element.kind), element.id)
    } else {
        trimmed.to_string()
    }
}

fn archi_native_view_name(view: &View) -> String {
    let name = view.title.as_deref().unwrap_or(view_key(view)).trim();
    if name.is_empty() {
        view_key(view).to_string()
    } else {
        name.to_string()
    }
}

fn append_archi_native_views(output: &mut String, workspace: &Workspace) {
    for view in &workspace.views {
        let graph = compiler::view_graph(workspace, view);
        let key = safe_name(view_key(view));
        let objects = archi_native_object_map(workspace, view, &graph);
        let connections = archi_native_connections(workspace, view, &graph, &objects);
        let layout = archi_native_layout(workspace, view, &graph, &connections);
        let scope = (view.kind == ViewKind::Container)
            .then_some(view.scope.as_deref())
            .flatten();
        let nested = graph
            .element_ids
            .iter()
            .filter(|identifier| {
                find(workspace, identifier).is_some_and(|element| {
                    element.kind == ElementKind::Container && element.parent.as_deref() == scope
                })
            })
            .collect::<Vec<_>>();
        let groups = workspace
            .groups
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                graph.element_ids.iter().any(|identifier| {
                    find(workspace, identifier).is_some_and(|element| element.group == Some(*index))
                })
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        output.push_str(&format!(
            "    <element xsi:type=\"archimate:ArchimateDiagramModel\" name=\"{}\" id=\"id-c4c-view-{key}\" connectionRouterType=\"2\">\n",
            xml_attr(&archi_native_view_name(view))
        ));
        let use_generated_bendpoints = !archi_native_has_manual_group_bounds(view);
        let object_xml = |identifier: &str,
                          bounds: ArchiNativeBounds,
                          indent: usize,
                          children: &str|
         -> Option<String> {
            let element = find(workspace, identifier)?;
            let object_id = &objects[identifier];
            let target_connections = connections
                .iter()
                .filter(|connection| connection.destination_element == identifier)
                .map(|connection| connection.id.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let spaces = " ".repeat(indent);
            let mut xml = format!(
                "{spaces}<child xsi:type=\"archimate:DiagramObject\" id=\"{object_id}\" archimateElement=\"{}\" type=\"1\" fillColor=\"{}\"{}{}{}>\n",
                archi_native_element_id(identifier),
                archi_native_object_fill(workspace, view, identifier, &element.kind),
                archi_native_view_object_property(view, identifier, "color")
                    .or_else(|| property_value(&element.attributes, "color"))
                    .map(|value| format!(" fontColor=\"{}\"", xml_attr(value)))
                    .unwrap_or_default(),
                archi_native_view_object_property(view, identifier, "stroke")
                    .or_else(|| property_value(&element.attributes, "stroke"))
                    .map(|value| format!(" lineColor=\"{}\"", xml_attr(value)))
                    .unwrap_or_default(),
                if target_connections.is_empty() {
                    String::new()
                } else {
                    format!(" targetConnections=\"{target_connections}\"")
                }
            );
            xml.push_str(&format!(
                "{spaces}  <bounds x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>\n",
                bounds.x, bounds.y, bounds.width, bounds.height
            ));
            for (route, connection) in connections
                .iter()
                .enumerate()
                .filter(|(_, connection)| connection.source_element == identifier)
            {
                xml.push_str(&format!(
                    "{spaces}  <sourceConnection xsi:type=\"archimate:Connection\" id=\"{}\" source=\"{}\" target=\"{}\" archimateRelationship=\"{}\"{}>\n",
                    connection.id,
                    connection.source_object,
                    connection.destination_object,
                    connection.relationship_id,
                    archi_native_connection_style(workspace, &connection.relationship_id)
                ));
                if let Some([first, second]) = use_generated_bendpoints
                    .then(|| archi_native_bendpoints(workspace, view, &layout, connection, route))
                    .flatten()
                {
                    for (start_x, start_y, end_x, end_y) in [first, second] {
                        xml.push_str(&format!(
                            "{spaces}    <bendpoint startX=\"{start_x}\" startY=\"{start_y}\" endX=\"{end_x}\" endY=\"{end_y}\"/>\n"
                        ));
                    }
                }
                xml.push_str(&format!("{spaces}  </sourceConnection>\n"));
            }
            xml.push_str(children);
            xml.push_str(&format!("{spaces}</child>\n"));
            Some(xml)
        };
        for identifier in &graph.element_ids {
            if nested.contains(&identifier)
                || find(workspace, identifier)
                    .and_then(|element| element.group)
                    .is_some_and(|group| groups.contains(&group))
            {
                continue;
            }
            let Some(bounds) = layout.get(identifier).copied() else {
                continue;
            };
            let mut children = String::new();
            if Some(identifier.as_str()) == scope {
                for child in &nested {
                    let child_bounds = layout[*child];
                    let relative = ArchiNativeBounds {
                        x: child_bounds.x - bounds.x,
                        y: child_bounds.y - bounds.y,
                        ..child_bounds
                    };
                    if let Some(xml) = object_xml(child, relative, 8, "") {
                        children.push_str(&xml);
                    }
                }
            }
            if let Some(xml) = object_xml(identifier, bounds, 6, &children) {
                output.push_str(&xml);
            }
        }
        for group_index in groups {
            let members = graph
                .element_ids
                .iter()
                .filter(|identifier| {
                    find(workspace, identifier)
                        .is_some_and(|element| element.group == Some(group_index))
                })
                .collect::<Vec<_>>();
            let Some(bounds) = layout.groups.get(&group_index).copied() else {
                continue;
            };
            let group_fill = archi_native_view_group_property(
                view,
                &workspace.groups[group_index].name,
                "background",
            )
            .map(|value| format!(" fillColor=\"{}\"", xml_attr(value)))
            .unwrap_or_default();
            output.push_str(&format!(
                "      <child xsi:type=\"archimate:Group\" id=\"id-c4c-viewgroup-{key}-{}\" name=\"{}\"{}>\n        <bounds x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>\n",
                group_index + 1,
                xml_attr(&workspace.groups[group_index].name),
                group_fill,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height
            ));
            for identifier in members {
                let member = layout[identifier];
                let relative = ArchiNativeBounds {
                    x: member.x - bounds.x,
                    y: member.y - bounds.y,
                    ..member
                };
                if let Some(xml) = object_xml(identifier, relative, 8, "") {
                    output.push_str(&xml);
                }
            }
            output.push_str("      </child>\n");
        }
        output.push_str("    </element>\n");
    }
}

fn archi_native_object_map(
    workspace: &Workspace,
    view: &View,
    graph: &compiler::ViewGraph,
) -> HashMap<String, String> {
    let key = safe_name(view_key(view));
    graph
        .element_ids
        .iter()
        .filter(|identifier| find(workspace, identifier).is_some())
        .map(|identifier| (identifier.clone(), archi_native_object_id(&key, identifier)))
        .collect()
}

fn archi_native_connections(
    workspace: &Workspace,
    view: &View,
    graph: &compiler::ViewGraph,
    objects: &HashMap<String, String>,
) -> Vec<ArchiNativeConnection> {
    let key = safe_name(view_key(view));
    graph
        .relationships
        .iter()
        .enumerate()
        .filter_map(|(position, relationship)| {
            let source_object = objects.get(&relationship.source)?;
            let destination_object = objects.get(&relationship.destination)?;
            let (relationship_id, synthetic) =
                archi_native_view_relationship_id(workspace, view, relationship, position);
            Some(ArchiNativeConnection {
                id: format!("id-c4c-connection-{key}-{}", position + 1),
                source_element: relationship.source.clone(),
                destination_element: relationship.destination.clone(),
                source_object: source_object.clone(),
                destination_object: destination_object.clone(),
                relationship_id,
                description: relationship.description.clone(),
                synthetic,
            })
        })
        .collect()
}

fn archi_native_view_relationship_id(
    workspace: &Workspace,
    view: &View,
    relationship: &compiler::ViewGraphRelationship,
    position: usize,
) -> (String, bool) {
    let exact = relationship.relationship_index.filter(|index| {
        let model = &workspace.relationships[*index];
        model.source == relationship.source && model.destination == relationship.destination
    });
    let exact = exact.or_else(|| {
        relationship.relationship_index.and_then(|_| {
            workspace.relationships.iter().position(|model| {
                model.source == relationship.source && model.destination == relationship.destination
            })
        })
    });
    exact.map_or_else(
        || {
            (
                format!(
                    "id-c4c-relationship-view-{}-{}",
                    safe_name(view_key(view)),
                    position + 1
                ),
                true,
            )
        },
        |index| {
            (
                archi_native_relationship_id(index, &workspace.relationships[index]),
                false,
            )
        },
    )
}

fn archi_native_internal_hub<'a>(
    members: &[&'a String],
    connections: &[ArchiNativeConnection],
) -> Option<&'a String> {
    if members.len() < 3 {
        return None;
    }
    members
        .iter()
        .copied()
        .filter_map(|candidate| {
            let connected = members
                .iter()
                .copied()
                .filter(|other| {
                    *other != candidate
                        && connections.iter().any(|connection| {
                            (connection.source_element.as_str() == candidate.as_str()
                                && connection.destination_element.as_str() == other.as_str())
                                || (connection.destination_element.as_str() == candidate.as_str()
                                    && connection.source_element.as_str() == other.as_str())
                        })
                })
                .count();
            (connected * 2 > members.len() - 1).then_some((candidate, connected))
        })
        .max_by_key(|(_, connected)| *connected)
        .map(|(candidate, _)| candidate)
}

fn archi_native_external_score(
    workspace: &Workspace,
    connections: &[ArchiNativeConnection],
    identifier: &str,
    own_group: usize,
) -> (usize, usize) {
    let (mut groups, mut elements) = (Vec::new(), Vec::new());
    for connection in connections.iter().filter(|connection| {
        connection.source_element == identifier || connection.destination_element == identifier
    }) {
        let other = if connection.source_element == identifier {
            &connection.destination_element
        } else {
            &connection.source_element
        };
        let group = find(workspace, other).and_then(|element| element.group);
        if group == Some(own_group) {
            continue;
        }
        if !elements.contains(other) {
            elements.push(other.clone());
        }
        if let Some(group) = group.filter(|group| !groups.contains(group)) {
            groups.push(group);
        }
    }
    (groups.len(), elements.len())
}

fn archi_native_has_ungrouped_neighbor(
    workspace: &Workspace,
    connections: &[ArchiNativeConnection],
    identifier: &str,
) -> bool {
    connections.iter().any(|connection| {
        let other = if connection.source_element == identifier {
            Some(&connection.destination_element)
        } else if connection.destination_element == identifier {
            Some(&connection.source_element)
        } else {
            None
        };
        other.is_some_and(|other| {
            find(workspace, other).is_some_and(|element| element.group.is_none())
        })
    })
}

fn archi_native_crossing_count(
    layout: &HashMap<String, ArchiNativeBounds>,
    connections: &[ArchiNativeConnection],
) -> usize {
    connections
        .iter()
        .filter_map(|connection| {
            Some((
                layout.get(&connection.source_element)?,
                layout.get(&connection.destination_element)?,
                connection,
            ))
        })
        .map(|(source, target, connection)| {
            let (source_x, source_y) = (source.x + source.width / 2, source.y + source.height / 2);
            let (target_x, target_y) = (target.x + target.width / 2, target.y + target.height / 2);
            layout
                .iter()
                .filter(|(identifier, _)| {
                    *identifier != &connection.source_element
                        && *identifier != &connection.destination_element
                })
                .filter(|(_, bounds)| {
                    let clearance = ArchiNativeBounds {
                        x: bounds.x - 10,
                        y: bounds.y - 10,
                        width: bounds.width + 20,
                        height: bounds.height + 20,
                    };
                    segment_crosses_bounds(source_x, source_y, target_x, target_y, clearance)
                })
                .count()
        })
        .sum()
}

fn archi_native_reduce_group_crossings(
    workspace: &Workspace,
    graph: &compiler::ViewGraph,
    connections: &[ArchiNativeConnection],
    groups: &HashMap<usize, ArchiNativeBounds>,
    layout: &mut HashMap<String, ArchiNativeBounds>,
) {
    for _ in 0..2 {
        let mut improved = false;
        for identifier in &graph.element_ids {
            let Some(group) = find(workspace, identifier).and_then(|element| element.group) else {
                continue;
            };
            let Some(frame) = groups.get(&group) else {
                continue;
            };
            let current = layout[identifier];
            if current.width != 180
                || archi_native_has_ungrouped_neighbor(workspace, connections, identifier)
            {
                continue;
            }
            let current_score = archi_native_crossing_count(layout, connections);
            let mut best = (current_score, 0, current);
            let mut x = frame.x + 20;
            while x <= frame.x + frame.width - current.width - 20 {
                let candidate = ArchiNativeBounds { x, ..current };
                let free = layout.iter().all(|(other, bounds)| {
                    other == identifier || !bounds_overlap(candidate, *bounds)
                });
                if free {
                    layout.insert(identifier.clone(), candidate);
                    let score = archi_native_crossing_count(layout, connections);
                    let choice = (score, (x - current.x).abs(), candidate);
                    if (choice.0, choice.1) < (best.0, best.1) {
                        best = choice;
                    }
                }
                x += 20;
            }
            layout.insert(
                identifier.clone(),
                if best.0 < current_score {
                    best.2
                } else {
                    current
                },
            );
            improved |= best.0 < current_score;
        }
        if !improved {
            break;
        }
    }
}

fn archi_native_viewpoint_layout(
    workspace: &Workspace,
    view: &View,
    graph: &compiler::ViewGraph,
    connections: &[ArchiNativeConnection],
) -> ArchiNativeLayout {
    let viewpoint = archi_native_viewpoint(view).unwrap_or("fallback");
    let horizontal = matches!(
        viewpoint,
        "applicationCooperation" | "applicationUsage" | "informationStructure" | "fallback"
    );
    let (base_x, base_y, rank_gap, row_gap, width, height) = if horizontal {
        (160, 160, 420, 170, 210, 90)
    } else {
        (160, 160, 220, 230, 210, 90)
    };
    let mut layout = HashMap::new();
    let mut ranked = graph
        .element_ids
        .iter()
        .filter(|identifier| find(workspace, identifier).is_some())
        .map(|identifier| {
            (
                archi_native_viewpoint_rank(workspace, viewpoint, connections, identifier),
                identifier,
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(rank, identifier)| (*rank, identifier.as_str()));
    let mut rank_counts = HashMap::<usize, usize>::new();
    for (rank, identifier) in ranked {
        let index = *rank_counts.get(&rank).unwrap_or(&0) as isize;
        let default = if horizontal {
            ArchiNativeBounds {
                x: base_x + rank as isize * rank_gap,
                y: base_y + index * row_gap,
                width,
                height,
            }
        } else {
            ArchiNativeBounds {
                x: base_x + index * row_gap,
                y: base_y + rank as isize * rank_gap,
                width,
                height,
            }
        };
        let manual = archi_native_manual_bounds(view, identifier, default);
        let mut bounds = manual.unwrap_or(default);
        if manual.is_none() {
            while layout.values().any(|existing| {
                bounds_overlap(
                    archi_native_padded(bounds, 30),
                    archi_native_padded(*existing, 30),
                )
            }) {
                if horizontal {
                    bounds.y += row_gap;
                } else {
                    bounds.x += row_gap;
                }
            }
            *rank_counts.entry(rank).or_default() += 1;
        }
        layout.insert(identifier.clone(), bounds);
    }
    ArchiNativeLayout {
        objects: layout,
        groups: HashMap::new(),
    }
}

fn archi_native_viewpoint(view: &View) -> Option<&str> {
    property_value(&view.properties, "viewpoint")
}

fn archi_native_viewpoint_rank(
    workspace: &Workspace,
    viewpoint: &str,
    connections: &[ArchiNativeConnection],
    identifier: &str,
) -> usize {
    let Some(element) = find(workspace, identifier) else {
        return 9;
    };
    if archi_native_type(&element.kind) == "Junction" {
        let mut ranks = connections
            .iter()
            .filter_map(|connection| {
                let other = if connection.source_element == identifier {
                    &connection.destination_element
                } else if connection.destination_element == identifier {
                    &connection.source_element
                } else {
                    return None;
                };
                find(workspace, other)
                    .map(|element| archi_native_base_rank(viewpoint, &element.kind))
            })
            .collect::<Vec<_>>();
        ranks.sort_unstable();
        return ranks.get(ranks.len() / 2).copied().unwrap_or(2);
    }
    archi_native_base_rank(viewpoint, &element.kind)
}

fn archi_native_base_rank(viewpoint: &str, kind: &ElementKind) -> usize {
    let native = archi_native_type(kind);
    let layer = archi_native_layer(kind);
    let role = archi_native_role(kind);
    match viewpoint {
        "applicationCooperation" | "applicationUsage" | "informationStructure" => {
            match (layer, role) {
                ("business", "active") => 0,
                ("business", _) => 1,
                ("application", _)
                    if native.ends_with("Service") || native.ends_with("Interface") =>
                {
                    1
                }
                ("application", "active") => 2,
                ("application", "passive") => 4,
                ("application", _) => 3,
                ("technology" | "physical" | "implementation_migration", _) => 4,
                ("motivation" | "strategy", _) => 0,
                _ => 4,
            }
        }
        "motivation" | "strategy" => match native {
            "Stakeholder" | "Driver" | "Assessment" => 0,
            "Goal" | "Outcome" | "Principle" => 1,
            "Requirement" | "Constraint" => 2,
            _ if layer == "strategy" => 3,
            _ if matches!(layer, "application" | "technology" | "physical") => 4,
            _ => 3,
        },
        "technology" | "technologyUsage" | "implementationAndDeployment" => match native {
            _ if native.ends_with("Service") || native.ends_with("Interface") => 0,
            "Node" | "Device" | "SystemSoftware" | "CommunicationNetwork" | "Path" => 1,
            "Artifact" => 2,
            _ if layer == "physical" => 3,
            _ if layer == "implementation_migration" => 4,
            _ => 1,
        },
        "layered" => match layer {
            "business" => 0,
            "application" => 1,
            "technology" => 2,
            "physical" => 3,
            "implementation_migration" => 4,
            "motivation" | "strategy" => 0,
            _ => 5,
        },
        _ => match layer {
            "business" | "motivation" | "strategy" => 0,
            "application" => 1,
            "technology" => 2,
            "physical" => 3,
            "implementation_migration" => 4,
            _ => 5,
        },
    }
}

fn archi_native_layer(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "business",
        ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "application",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "technology",
        ElementKind::ArchiMate(kind) => compiler::archimate_element_layer(kind).unwrap_or("other"),
        _ => "other",
    }
}

fn archi_native_role(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person
        | ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance
        | ElementKind::DeploymentNode
        | ElementKind::InfrastructureNode => "active",
        ElementKind::ArchiMate(kind) => {
            compiler::archimate_element_role(kind).unwrap_or("composite")
        }
        _ => "composite",
    }
}

fn archi_native_manual_bounds(
    view: &View,
    identifier: &str,
    default: ArchiNativeBounds,
) -> Option<ArchiNativeBounds> {
    let has_manual = ["x", "y", "width", "height"]
        .into_iter()
        .any(|name| archi_native_view_object_property(view, identifier, name).is_some());
    has_manual.then(|| ArchiNativeBounds {
        x: archi_native_view_object_property(view, identifier, "x")
            .and_then(|value| value.parse::<isize>().ok())
            .unwrap_or(default.x),
        y: archi_native_view_object_property(view, identifier, "y")
            .and_then(|value| value.parse::<isize>().ok())
            .unwrap_or(default.y),
        width: archi_native_view_object_property(view, identifier, "width")
            .and_then(|value| value.parse::<isize>().ok())
            .unwrap_or(default.width)
            .max(1),
        height: archi_native_view_object_property(view, identifier, "height")
            .and_then(|value| value.parse::<isize>().ok())
            .unwrap_or(default.height)
            .max(1),
    })
}

fn archi_native_padded(bounds: ArchiNativeBounds, padding: isize) -> ArchiNativeBounds {
    ArchiNativeBounds {
        x: bounds.x - padding,
        y: bounds.y - padding,
        width: bounds.width + padding * 2,
        height: bounds.height + padding * 2,
    }
}

fn archi_native_complete_manual_group_layout(
    workspace: &Workspace,
    view: &View,
    graph: &compiler::ViewGraph,
    visible_groups: &[usize],
) -> Option<ArchiNativeLayout> {
    let mut objects = HashMap::new();
    for identifier in &graph.element_ids {
        if !["x", "y", "width", "height"]
            .iter()
            .all(|name| archi_native_view_object_property(view, identifier, name).is_some())
        {
            return None;
        }
        objects.insert(
            identifier.clone(),
            archi_native_manual_bounds(
                view,
                identifier,
                ArchiNativeBounds {
                    x: 0,
                    y: 0,
                    width: 180,
                    height: 80,
                },
            )?,
        );
    }
    let mut groups = HashMap::new();
    for group in visible_groups {
        groups.insert(
            *group,
            archi_native_manual_group_bounds(view, &workspace.groups[*group].name)?,
        );
    }
    Some(ArchiNativeLayout { objects, groups })
}

fn archi_native_manual_group_bounds(view: &View, name: &str) -> Option<ArchiNativeBounds> {
    Some(ArchiNativeBounds {
        x: archi_native_view_group_property(view, name, "x")?
            .parse()
            .ok()?,
        y: archi_native_view_group_property(view, name, "y")?
            .parse()
            .ok()?,
        width: archi_native_view_group_property(view, name, "width")?
            .parse::<isize>()
            .ok()?
            .max(1),
        height: archi_native_view_group_property(view, name, "height")?
            .parse::<isize>()
            .ok()?
            .max(1),
    })
}

fn archi_native_has_manual_group_bounds(view: &View) -> bool {
    view.properties
        .iter()
        .any(|property| property.key.starts_with("group.") && property.key.ends_with(".x"))
}

fn archi_native_layout(
    workspace: &Workspace,
    view: &View,
    graph: &compiler::ViewGraph,
    connections: &[ArchiNativeConnection],
) -> ArchiNativeLayout {
    let vertical = view.kind == ViewKind::ArchiMate
        || view.auto_layout.as_ref().is_some_and(|layout| {
            matches!(layout.direction.to_ascii_lowercase().as_str(), "tb" | "bt")
        });
    let route_margin = 120 + connections.len() as isize * 30;
    let scope = (view.kind == ViewKind::Container)
        .then_some(view.scope.as_deref())
        .flatten();
    let nested = graph
        .element_ids
        .iter()
        .filter(|identifier| {
            find(workspace, identifier).is_some_and(|element| {
                element.kind == ElementKind::Container && element.parent.as_deref() == scope
            })
        })
        .collect::<Vec<_>>();
    let mut layout: HashMap<String, ArchiNativeBounds> = HashMap::new();
    let visible_groups = workspace
        .groups
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            graph.element_ids.iter().any(|identifier| {
                find(workspace, identifier).is_some_and(|element| element.group == Some(*index))
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if view.kind == ViewKind::ArchiMate && visible_groups.is_empty() {
        return archi_native_viewpoint_layout(workspace, view, graph, connections);
    }
    if vertical && !visible_groups.is_empty() {
        if let Some(layout) =
            archi_native_complete_manual_group_layout(workspace, view, graph, &visible_groups)
        {
            return layout;
        }
        let widest_group = visible_groups
            .iter()
            .map(|group| {
                let members = graph
                    .element_ids
                    .iter()
                    .filter(|identifier| {
                        find(workspace, identifier)
                            .is_some_and(|element| element.group == Some(*group))
                    })
                    .collect::<Vec<_>>();
                let columns = if archi_native_internal_hub(&members, connections).is_some() {
                    members.len().saturating_sub(1)
                } else {
                    members.len()
                };
                columns as isize * 220 + 40
            })
            .max()
            .unwrap_or(700);
        let common_group_width = widest_group.max(700);
        let mut cursor = 40;
        let mut group_bounds = HashMap::new();
        for group_index in &visible_groups {
            let members = graph
                .element_ids
                .iter()
                .filter(|identifier| {
                    find(workspace, identifier)
                        .is_some_and(|element| element.group == Some(*group_index))
                })
                .collect::<Vec<_>>();
            if members.len() < 2 {
                let frame = ArchiNativeBounds {
                    x: route_margin,
                    y: cursor,
                    width: common_group_width,
                    height: 140,
                };
                group_bounds.insert(*group_index, frame);
                for identifier in members {
                    let x = connections
                        .iter()
                        .filter_map(|connection| {
                            let other = if connection.source_element == *identifier {
                                &connection.destination_element
                            } else if connection.destination_element == *identifier {
                                &connection.source_element
                            } else {
                                return None;
                            };
                            layout
                                .get(other)
                                .map(|bounds| bounds.x + bounds.width / 2 - 90)
                        })
                        .next()
                        .unwrap_or(frame.x + (frame.width - 180) / 2)
                        .clamp(frame.x + 20, frame.x + frame.width - 200);
                    layout.insert(
                        identifier.clone(),
                        ArchiNativeBounds {
                            x,
                            y: cursor + 40,
                            width: 180,
                            height: 80,
                        },
                    );
                }
                cursor += 200;
                continue;
            }
            let Some(hub) = archi_native_internal_hub(&members, connections) else {
                let frame = ArchiNativeBounds {
                    x: route_margin,
                    y: cursor,
                    width: common_group_width,
                    height: 140,
                };
                group_bounds.insert(*group_index, frame);
                let anchor = *members
                    .iter()
                    .filter(|identifier| {
                        !archi_native_has_ungrouped_neighbor(workspace, connections, identifier)
                    })
                    .max_by_key(|identifier| {
                        archi_native_external_score(
                            workspace,
                            connections,
                            identifier,
                            *group_index,
                        )
                    })
                    .unwrap_or(&members[0]);
                let mut edges = members
                    .iter()
                    .copied()
                    .filter(|identifier| {
                        archi_native_has_ungrouped_neighbor(workspace, connections, identifier)
                    })
                    .collect::<Vec<_>>();
                edges.sort();
                let left_edge = edges.first().copied();
                let right_edge = edges.get(1).copied();
                let mut middle = members
                    .into_iter()
                    .filter(|identifier| {
                        Some(*identifier) != left_edge && Some(*identifier) != right_edge
                    })
                    .collect::<Vec<_>>();
                middle.sort_by_key(|identifier| {
                    std::cmp::Reverse(archi_native_external_score(
                        workspace,
                        connections,
                        identifier,
                        *group_index,
                    ))
                });
                if let Some(position) = middle.iter().position(|identifier| *identifier == anchor) {
                    let anchor = middle.remove(position);
                    middle.insert(0, anchor);
                }
                let mut ordered = Vec::new();
                if let Some(identifier) = left_edge {
                    ordered.push(identifier);
                }
                ordered.extend(middle);
                if let Some(identifier) = right_edge {
                    ordered.push(identifier);
                }
                ordered.extend(edges.into_iter().skip(2));
                let anchor_position = ordered
                    .iter()
                    .position(|identifier| *identifier == anchor)
                    .unwrap_or(ordered.len() / 2);
                let target_center = connections.iter().find_map(|connection| {
                    let other = if connection.source_element == *anchor {
                        &connection.destination_element
                    } else if connection.destination_element == *anchor {
                        &connection.source_element
                    } else {
                        return None;
                    };
                    let other_group = find(workspace, other).and_then(|element| element.group)?;
                    (other_group != *group_index)
                        .then(|| layout.get(other))
                        .flatten()
                        .map(|bounds| bounds.x + bounds.width / 2)
                });
                let minimum_left = frame.x + 20;
                let maximum_left = frame.x + frame.width - ordered.len() as isize * 200;
                let centered_left =
                    frame.x + (frame.width - (ordered.len() as isize * 200 - 20)) / 2;
                let left = target_center
                    .map(|center| center - anchor_position as isize * 200 - 90)
                    .unwrap_or(centered_left)
                    .clamp(minimum_left, maximum_left.max(minimum_left));
                for (position, identifier) in ordered.into_iter().enumerate() {
                    layout.insert(
                        identifier.clone(),
                        ArchiNativeBounds {
                            x: left + position as isize * 200,
                            y: cursor + 40,
                            width: 180,
                            height: 80,
                        },
                    );
                }
                cursor += 240;
                continue;
            };
            let peers = members
                .into_iter()
                .filter(|identifier| *identifier != hub)
                .collect::<Vec<_>>();
            let rank = visible_groups
                .iter()
                .position(|candidate| candidate == group_index)
                .unwrap();
            let (mut top, mut bottom) = (Vec::new(), Vec::new());
            for identifier in peers {
                let (mut above, mut below) = (0, 0);
                for connection in connections.iter().filter(|connection| {
                    connection.source_element == *identifier
                        || connection.destination_element == *identifier
                }) {
                    let other = if connection.source_element == *identifier {
                        &connection.destination_element
                    } else {
                        &connection.source_element
                    };
                    let Some(other_group) =
                        find(workspace, other).and_then(|element| element.group)
                    else {
                        continue;
                    };
                    let Some(other_rank) = visible_groups
                        .iter()
                        .position(|candidate| *candidate == other_group)
                    else {
                        continue;
                    };
                    above += usize::from(other_rank < rank);
                    below += usize::from(other_rank > rank);
                }
                if below > 0 && below >= above {
                    bottom.push(identifier);
                } else if above > 0 || top.len() <= bottom.len() {
                    top.push(identifier);
                } else {
                    bottom.push(identifier);
                }
            }
            let group_width = common_group_width;
            group_bounds.insert(
                *group_index,
                ArchiNativeBounds {
                    x: route_margin,
                    y: cursor,
                    width: group_width,
                    height: 800,
                },
            );
            let hub_width = group_width * 4 / 5;
            layout.insert(
                hub.clone(),
                ArchiNativeBounds {
                    x: route_margin + (group_width - hub_width) / 2,
                    y: cursor + 360,
                    width: hub_width,
                    height: 80,
                },
            );
            let (mut hub_above, mut hub_below) = (false, false);
            for connection in connections.iter().filter(|connection| {
                connection.source_element == *hub || connection.destination_element == *hub
            }) {
                let other = if connection.source_element == *hub {
                    &connection.destination_element
                } else {
                    &connection.source_element
                };
                let Some(other_group) = find(workspace, other).and_then(|element| element.group)
                else {
                    continue;
                };
                let Some(other_rank) = visible_groups
                    .iter()
                    .position(|candidate| *candidate == other_group)
                else {
                    continue;
                };
                hub_above |= other_rank < rank;
                hub_below |= other_rank > rank;
            }
            for (row, y) in [(&top, cursor + 40), (&bottom, cursor + 700)] {
                let reserve_center = if y < cursor + 360 {
                    hub_above
                } else {
                    hub_below
                };
                let center = route_margin + group_width / 2;
                let mut row_bounds = Vec::new();
                for (position, identifier) in row.iter().enumerate() {
                    let available = group_width - 40 - 180;
                    let default_x = route_margin
                        + 20
                        + if row.len() > 1 {
                            position as isize * available / (row.len() - 1) as isize
                        } else {
                            available / 2
                        };
                    let external_neighbors = connections
                        .iter()
                        .filter_map(|connection| {
                            let other = if connection.source_element.as_str() == identifier.as_str()
                            {
                                &connection.destination_element
                            } else if connection.destination_element.as_str() == identifier.as_str()
                            {
                                &connection.source_element
                            } else {
                                return None;
                            };
                            let other_group =
                                find(workspace, other).and_then(|element| element.group)?;
                            (other_group != *group_index)
                                .then(|| layout.get(other))
                                .flatten()
                                .map(|bounds| (other, *bounds))
                        })
                        .collect::<Vec<_>>();
                    let desired = external_neighbors
                        .first()
                        .map(|(_, bounds)| bounds.x + bounds.width / 2 - 90)
                        .unwrap_or(default_x)
                        .clamp(route_margin + 20, route_margin + group_width - 200);
                    let x = [
                        desired,
                        default_x,
                        center - 200,
                        center + 20,
                        route_margin + 20,
                        route_margin + (group_width - 180) / 2,
                        route_margin + group_width - 200,
                    ]
                    .into_iter()
                    .filter(|x| !reserve_center || center < *x || center > *x + 180)
                    .filter(|x| {
                        let candidate = ArchiNativeBounds {
                            x: *x,
                            y,
                            width: 180,
                            height: 80,
                        };
                        row_bounds
                            .iter()
                            .all(|bounds| !bounds_overlap(candidate, *bounds))
                    })
                    .min_by_key(|x| {
                        let source = (*x + 90, y + 40);
                        let crossings = external_neighbors
                            .iter()
                            .map(|(neighbor, target)| {
                                let target =
                                    (target.x + target.width / 2, target.y + target.height / 2);
                                layout
                                    .iter()
                                    .filter(|(other, bounds)| {
                                        other.as_str() != neighbor.as_str()
                                            && other.as_str() != identifier.as_str()
                                            && segment_crosses_bounds(
                                                source.0, source.1, target.0, target.1, **bounds,
                                            )
                                    })
                                    .count()
                            })
                            .sum::<usize>();
                        (crossings, (*x - desired).abs())
                    })
                    .unwrap_or(default_x);
                    let bounds = ArchiNativeBounds {
                        x,
                        y,
                        width: 180,
                        height: 80,
                    };
                    layout.insert((*identifier).clone(), bounds);
                    row_bounds.push(bounds);
                }
            }
            cursor += 900;
        }
        archi_native_reduce_group_crossings(
            workspace,
            graph,
            connections,
            &group_bounds,
            &mut layout,
        );
        let all_group_bounds = group_bounds.values().copied().collect::<Vec<_>>();
        let mut ungrouped_y = 80;
        for identifier in &graph.element_ids {
            if layout.contains_key(identifier) {
                continue;
            }
            let neighbors = connections
                .iter()
                .filter_map(|connection| {
                    let other = if connection.source_element == *identifier {
                        &connection.destination_element
                    } else if connection.destination_element == *identifier {
                        &connection.source_element
                    } else {
                        return None;
                    };
                    find(workspace, other)
                        .and_then(|element| element.group)
                        .filter(|group| group_bounds.contains_key(group))?;
                    Some(other)
                })
                .collect::<Vec<_>>();
            let mut candidates = Vec::new();
            for neighbor in &neighbors {
                let neighbor_bounds = layout[*neighbor];
                let group = find(workspace, neighbor)
                    .and_then(|element| element.group)
                    .unwrap();
                let bounds = group_bounds[&group];
                candidates.extend([
                    ArchiNativeBounds {
                        x: bounds.x - 240,
                        y: neighbor_bounds.y,
                        width: 180,
                        height: 80,
                    },
                    ArchiNativeBounds {
                        x: bounds.x + bounds.width + 60,
                        y: neighbor_bounds.y,
                        width: 180,
                        height: 80,
                    },
                    ArchiNativeBounds {
                        x: neighbor_bounds.x + (neighbor_bounds.width - 180) / 2,
                        y: bounds.y - 140,
                        width: 180,
                        height: 80,
                    },
                    ArchiNativeBounds {
                        x: neighbor_bounds.x + (neighbor_bounds.width - 180) / 2,
                        y: bounds.y + bounds.height + 60,
                        width: 180,
                        height: 80,
                    },
                ]);
            }
            let chosen = candidates
                .into_iter()
                .enumerate()
                .min_by_key(|(order, candidate)| {
                    let score = archi_native_position_score(
                        &layout,
                        &all_group_bounds,
                        connections,
                        identifier,
                        &neighbors,
                        *candidate,
                    );
                    (
                        score.0,
                        score.1,
                        score.2,
                        usize::from(order % 4 >= 2),
                        score.3,
                        *order,
                    )
                })
                .map(|(_, candidate)| candidate)
                .unwrap_or_else(|| {
                    let mut candidate = ArchiNativeBounds {
                        x: route_margin + 60 + common_group_width,
                        y: ungrouped_y,
                        width: 180,
                        height: 80,
                    };
                    while layout
                        .values()
                        .any(|bounds| bounds_overlap(candidate, *bounds))
                    {
                        candidate.y += 140;
                    }
                    candidate
                });
            layout.insert(identifier.clone(), chosen);
            ungrouped_y += 140;
        }
        apply_archi_native_manual_layout(view, &mut layout);
        return ArchiNativeLayout {
            objects: layout,
            groups: group_bounds,
        };
    }
    let mut cursor = 40;
    for identifier in &graph.element_ids {
        if nested.contains(&identifier) {
            continue;
        }
        if Some(identifier.as_str()) == scope && !nested.is_empty() {
            let size = 40 + nested.len() as isize * 220;
            let bounds = if vertical {
                ArchiNativeBounds {
                    x: route_margin,
                    y: cursor,
                    width: 220,
                    height: size,
                }
            } else {
                ArchiNativeBounds {
                    x: cursor,
                    y: route_margin,
                    width: size,
                    height: 160,
                }
            };
            layout.insert(identifier.clone(), bounds);
            for (position, child) in nested.iter().enumerate() {
                layout.insert(
                    (*child).clone(),
                    if vertical {
                        ArchiNativeBounds {
                            x: bounds.x + 20,
                            y: bounds.y + 40 + position as isize * 220,
                            width: 180,
                            height: 80,
                        }
                    } else {
                        ArchiNativeBounds {
                            x: bounds.x + 20 + position as isize * 220,
                            y: bounds.y + 60,
                            width: 180,
                            height: 80,
                        }
                    },
                );
            }
            cursor += size + 80;
        } else {
            layout.insert(
                identifier.clone(),
                if vertical {
                    ArchiNativeBounds {
                        x: route_margin + 20,
                        y: cursor,
                        width: 180,
                        height: 80,
                    }
                } else {
                    ArchiNativeBounds {
                        x: cursor,
                        y: route_margin + 60,
                        width: 180,
                        height: 80,
                    }
                },
            );
            cursor += 260;
        }
    }
    apply_archi_native_manual_layout(view, &mut layout);
    ArchiNativeLayout {
        objects: layout,
        groups: HashMap::new(),
    }
}

fn apply_archi_native_manual_layout(view: &View, layout: &mut HashMap<String, ArchiNativeBounds>) {
    if view.kind != ViewKind::ArchiMate {
        return;
    }
    let identifiers = layout.keys().cloned().collect::<Vec<_>>();
    for identifier in identifiers {
        let current = layout[&identifier];
        if let Some(bounds) = archi_native_manual_bounds(view, &identifier, current) {
            layout.insert(identifier, bounds);
        }
    }
}

fn archi_native_view_object_property<'a>(
    view: &'a View,
    identifier: &str,
    name: &str,
) -> Option<&'a str> {
    let key = format!("object.{identifier}.{name}");
    property_value(&view.properties, &key)
}

fn archi_native_view_group_property<'a>(
    view: &'a View,
    group_name: &str,
    name: &str,
) -> Option<&'a str> {
    let key = format!("group.{group_name}.{name}");
    property_value(&view.properties, &key)
}

fn archi_native_object_fill(
    workspace: &Workspace,
    view: &View,
    identifier: &str,
    kind: &ElementKind,
) -> String {
    archi_native_view_object_property(view, identifier, "background")
        .or_else(|| {
            find(workspace, identifier)
                .and_then(|element| property_value(&element.attributes, "background"))
        })
        .unwrap_or_else(|| archi_native_fill(kind))
        .into()
}

fn archi_native_connection_style(workspace: &Workspace, relationship_id: &str) -> String {
    let Some(relationship) = workspace
        .relationships
        .iter()
        .enumerate()
        .find(|(index, relationship)| {
            archi_native_relationship_id(*index, relationship) == relationship_id
        })
        .map(|(_, relationship)| relationship)
    else {
        return String::new();
    };
    let mut output = String::new();
    if let Some(color) = property_value(&relationship.attributes, "color") {
        output.push_str(&format!(" lineColor=\"{}\"", xml_attr(color)));
    }
    if let Some(width) = property_value(&relationship.attributes, "thickness") {
        output.push_str(&format!(" lineWidth=\"{}\"", xml_attr(width)));
    }
    output
}

fn archi_native_position_score(
    layout: &HashMap<String, ArchiNativeBounds>,
    groups: &[ArchiNativeBounds],
    connections: &[ArchiNativeConnection],
    identifier: &str,
    neighbors: &[&String],
    candidate: ArchiNativeBounds,
) -> (usize, usize, usize, isize) {
    let source = (
        candidate.x + candidate.width / 2,
        candidate.y + candidate.height / 2,
    );
    let overlaps = layout
        .values()
        .filter(|bounds| bounds_overlap(candidate, **bounds))
        .count()
        + groups
            .iter()
            .filter(|bounds| bounds_overlap(candidate, **bounds))
            .count();
    let mut object_crossings = 0;
    let mut line_crossings = 0;
    let mut length = 0;
    for neighbor in neighbors {
        let target_bounds = layout[*neighbor];
        let target = (
            target_bounds.x + target_bounds.width / 2,
            target_bounds.y + target_bounds.height / 2,
        );
        object_crossings += layout
            .iter()
            .filter(|(other, bounds)| {
                other.as_str() != neighbor.as_str()
                    && segment_crosses_bounds(source.0, source.1, target.0, target.1, **bounds)
            })
            .count();
        line_crossings += connections
            .iter()
            .filter(|connection| {
                connection.source_element != identifier
                    && connection.destination_element != identifier
                    && connection.source_element.as_str() != neighbor.as_str()
                    && connection.destination_element.as_str() != neighbor.as_str()
            })
            .filter_map(|connection| {
                let start = layout.get(&connection.source_element)?;
                let end = layout.get(&connection.destination_element)?;
                Some((
                    (start.x + start.width / 2, start.y + start.height / 2),
                    (end.x + end.width / 2, end.y + end.height / 2),
                ))
            })
            .filter(|(start, end)| segments_cross(source, target, *start, *end))
            .count();
        length += (source.0 - target.0).abs() + (source.1 - target.1).abs();
    }
    (overlaps, object_crossings, line_crossings, length)
}

fn bounds_overlap(left: ArchiNativeBounds, right: ArchiNativeBounds) -> bool {
    left.x < right.x + right.width
        && left.x + left.width > right.x
        && left.y < right.y + right.height
        && left.y + left.height > right.y
}

fn segments_cross(
    first_start: (isize, isize),
    first_end: (isize, isize),
    second_start: (isize, isize),
    second_end: (isize, isize),
) -> bool {
    let side = |start: (isize, isize), end: (isize, isize), point: (isize, isize)| {
        (end.0 - start.0) as i128 * (point.1 - start.1) as i128
            - (end.1 - start.1) as i128 * (point.0 - start.0) as i128
    };
    let (a, b) = (
        side(first_start, first_end, second_start),
        side(first_start, first_end, second_end),
    );
    let (c, d) = (
        side(second_start, second_end, first_start),
        side(second_start, second_end, first_end),
    );
    (a > 0) != (b > 0) && (c > 0) != (d > 0)
}

fn archi_native_bendpoints(
    workspace: &Workspace,
    view: &View,
    layout: &ArchiNativeLayout,
    connection: &ArchiNativeConnection,
    route: usize,
) -> Option<[(isize, isize, isize, isize); 2]> {
    let source = layout.get(&connection.source_element)?;
    let target = layout.get(&connection.destination_element)?;
    let (source_x, source_y) = (source.x + source.width / 2, source.y + source.height / 2);
    let (target_x, target_y) = (target.x + target.width / 2, target.y + target.height / 2);
    let blocked = layout.iter().any(|(identifier, bounds)| {
        identifier != &connection.source_element
            && identifier != &connection.destination_element
            && segment_crosses_bounds(source_x, source_y, target_x, target_y, *bounds)
    });
    if !blocked {
        return None;
    }
    let group = find(workspace, &connection.source_element)
        .and_then(|element| element.group)
        .filter(|group| {
            find(workspace, &connection.destination_element)
                .is_some_and(|element| element.group == Some(*group))
        });
    if let Some(group) = group {
        let bounds = *layout.groups.get(&group)?;
        let offset = 10 + route as isize % 3 * 10;
        let candidates = [
            ((bounds.x + offset, source_y), (bounds.x + offset, target_y)),
            (
                (bounds.x + bounds.width - offset, source_y),
                (bounds.x + bounds.width - offset, target_y),
            ),
            ((source_x, bounds.y + offset), (target_x, bounds.y + offset)),
            (
                (source_x, bounds.y + bounds.height - offset),
                (target_x, bounds.y + bounds.height - offset),
            ),
        ];
        // ponytail: four internal lanes cover this layout; use a graph router if layouts become free-form.
        if let Some((first, second)) = candidates.into_iter().find(|(first, second)| {
            archi_native_path_clear(
                layout,
                connection,
                (source_x, source_y),
                *first,
                *second,
                (target_x, target_y),
            )
        }) {
            return Some([
                (
                    first.0 - source_x,
                    first.1 - source_y,
                    first.0 - target_x,
                    first.1 - target_y,
                ),
                (
                    second.0 - source_x,
                    second.1 - source_y,
                    second.0 - target_x,
                    second.1 - target_y,
                ),
            ]);
        }
        return None;
    }
    let route_vertical = archi_native_route_vertical(view);
    let lane = if route_vertical {
        layout.values().map(|bounds| bounds.x).min().unwrap_or(60) - 40
    } else {
        layout.values().map(|bounds| bounds.y).min().unwrap_or(60) - 40
    };
    let (first_x, first_y, second_x, second_y) = if route_vertical {
        (lane, source_y, lane, target_y)
    } else {
        (source_x, lane, target_x, lane)
    };
    Some([
        (
            first_x - source_x,
            first_y - source_y,
            first_x - target_x,
            first_y - target_y,
        ),
        (
            second_x - source_x,
            second_y - source_y,
            second_x - target_x,
            second_y - target_y,
        ),
    ])
}

fn archi_native_route_vertical(view: &View) -> bool {
    view.auto_layout
        .as_ref()
        .is_some_and(|layout| matches!(layout.direction.to_ascii_lowercase().as_str(), "tb" | "bt"))
        || archi_native_viewpoint(view).is_some_and(|viewpoint| {
            matches!(
                viewpoint,
                "motivation"
                    | "strategy"
                    | "technology"
                    | "technologyUsage"
                    | "implementationAndDeployment"
                    | "implementationAndMigration"
                    | "migration"
                    | "layered"
            )
        })
}

fn archi_native_path_clear(
    layout: &HashMap<String, ArchiNativeBounds>,
    connection: &ArchiNativeConnection,
    source: (isize, isize),
    first: (isize, isize),
    second: (isize, isize),
    target: (isize, isize),
) -> bool {
    [(source, first), (first, second), (second, target)]
        .into_iter()
        .all(|(start, end)| {
            layout.iter().all(|(identifier, bounds)| {
                identifier == &connection.source_element
                    || identifier == &connection.destination_element
                    || !segment_crosses_bounds(start.0, start.1, end.0, end.1, *bounds)
            })
        })
}

fn segment_crosses_bounds(
    source_x: isize,
    source_y: isize,
    target_x: isize,
    target_y: isize,
    bounds: ArchiNativeBounds,
) -> bool {
    let mut near: f64 = 0.0;
    let mut far: f64 = 1.0;
    for (start, delta, low, high) in [
        (
            source_x as f64,
            (target_x - source_x) as f64,
            bounds.x as f64,
            (bounds.x + bounds.width) as f64,
        ),
        (
            source_y as f64,
            (target_y - source_y) as f64,
            bounds.y as f64,
            (bounds.y + bounds.height) as f64,
        ),
    ] {
        if delta == 0.0 {
            if start < low || start > high {
                return false;
            }
            continue;
        }
        let (entry, exit) = ((low - start) / delta, (high - start) / delta);
        near = near.max(entry.min(exit));
        far = far.min(entry.max(exit));
        if near > far {
            return false;
        }
    }
    true
}

fn archi_native_element_id(identifier: &str) -> String {
    format!("id-c4c-element-{}", safe_name(identifier))
}

fn archi_native_relationship_id(index: usize, relationship: &Relationship) -> String {
    relationship.id.as_ref().map_or_else(
        || format!("id-c4c-relationship-{}", index + 1),
        |id| format!("id-c4c-relationship-{}", safe_name(id)),
    )
}

fn archi_native_object_id(view_key: &str, identifier: &str) -> String {
    format!(
        "id-c4c-viewobject-{}-{}",
        safe_name(view_key),
        safe_name(identifier)
    )
}

fn archi_native_folder(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "business",
        ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "application",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "technology",
        ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup => "other",
        ElementKind::ArchiMate(kind) => compiler::archimate_element_folder(kind).unwrap_or("other"),
    }
}

fn archi_native_type(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "BusinessActor",
        ElementKind::SoftwareSystem
        | ElementKind::Container
        | ElementKind::Component
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "ApplicationComponent",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "Node",
        ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup => "Grouping",
        ElementKind::ArchiMate(kind) => kind,
    }
}

fn archi_native_fill(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "#ffffb5",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "#c9d9ff",
        ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup => "#eeeeee",
        ElementKind::ArchiMate(kind) => {
            compiler::archimate_element_default_color(kind).unwrap_or("#b5ffff")
        }
        _ => "#b5ffff",
    }
}

fn xml_identifier(value: &str) -> String {
    let value = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if value.is_empty() {
        "item".into()
    } else {
        value
    }
}

fn render_graphviz(
    workspace: &Workspace,
    output: &Path,
    format: &str,
    strict_safe: bool,
) -> Result<(), String> {
    if strict_safe {
        return Err(format!(
            "--strict-safe rejects external local renderer execution for {format}. Export --format dot or --format mermaid instead. No renderer was executed and no network request was made."
        ));
    }
    for view in &workspace.views {
        let mut child = match Command::new("dot")
            .arg(format!("-T{format}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(missing_renderer(format))
            }
            Err(error) => {
                return Err(format!(
                "cannot start local Graphviz renderer 'dot': {error}. No network request was made."
            ))
            }
        };
        child
            .stdin
            .as_mut()
            .ok_or_else(|| "cannot open Graphviz renderer input".to_string())?
            .write_all(dot(workspace, view).as_bytes())
            .map_err(|error| format!("cannot write to local Graphviz renderer: {error}"))?;
        let rendered = child
            .wait_with_output()
            .map_err(|error| format!("local Graphviz renderer failed: {error}"))?;
        if !rendered.status.success() {
            return Err(format!(
                "local Graphviz renderer failed: {}. No network request was made.",
                String::from_utf8_lossy(&rendered.stderr).trim()
            ));
        }
        let path = output.join(format!("{}.{}", safe_name(view_key(view)), format));
        fs::write(&path, rendered.stdout)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn missing_renderer(format: &str) -> String {
    format!(
        "{} export requires a local renderer, but Graphviz 'dot' was not found. Install Graphviz (`brew install graphviz` or your platform equivalent), or export --format dot or --format mermaid. No network request was made.",
        format.to_ascii_uppercase()
    )
}

fn json(workspace: &Workspace) -> String {
    let people = workspace
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::Person)
        .map(element_json)
        .collect();
    let systems = workspace
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::SoftwareSystem)
        .map(|system| software_system_json(workspace, system))
        .collect();
    let deployment = workspace
        .elements
        .iter()
        .filter(|element| {
            matches!(
                element.kind,
                ElementKind::DeploymentEnvironment
                    | ElementKind::DeploymentGroup
                    | ElementKind::DeploymentNode
                    | ElementKind::InfrastructureNode
                    | ElementKind::SoftwareSystemInstance
                    | ElementKind::ContainerInstance
            )
        })
        .map(element_json)
        .collect();
    let relationships = workspace
        .relationships
        .iter()
        .enumerate()
        .map(|(index, relationship)| relationship_json(index, relationship))
        .collect();
    let model = Json::object(vec![
        ("people", Json::Array(people)),
        ("softwareSystems", Json::Array(systems)),
        ("deploymentElements", Json::Array(deployment)),
        ("relationships", Json::Array(relationships)),
    ]);
    let views = Json::object(
        [
            ("systemLandscapeViews", ViewKind::SystemLandscape),
            ("systemContextViews", ViewKind::SystemContext),
            ("containerViews", ViewKind::Container),
            ("componentViews", ViewKind::Component),
            ("filteredViews", ViewKind::Filtered),
            ("dynamicViews", ViewKind::Dynamic),
            ("deploymentViews", ViewKind::Deployment),
            ("customViews", ViewKind::Custom),
            ("imageViews", ViewKind::Image),
            ("archimateViews", ViewKind::ArchiMate),
        ]
        .into_iter()
        .map(|(name, kind)| {
            (
                name,
                Json::Array(
                    workspace
                        .views
                        .iter()
                        .filter(|view| view.kind == kind)
                        .map(|view| view_json(workspace, view))
                        .collect(),
                ),
            )
        })
        .collect(),
    );
    let c4c = Json::object(vec![
        ("formatVersion", Json::string("8")),
        ("identifiers", Json::string(&workspace.identifiers)),
        (
            "elements",
            Json::Array(workspace.elements.iter().map(element_json).collect()),
        ),
        ("styles", styles_json(workspace)),
        ("documentation", documentation_json(workspace)),
    ]);
    let root = Json::object(vec![
        ("name", optional_json(workspace.name.as_deref())),
        (
            "description",
            optional_json(workspace.description.as_deref()),
        ),
        ("model", model),
        ("views", views),
        ("c4c", c4c),
    ]);
    format!("{}\n", root.render(0))
}

fn element_json(element: &Element) -> Json {
    Json::object(vec![
        ("id", Json::string(&element.id)),
        ("name", Json::string(&element.name)),
        ("description", optional_json(element.description.as_deref())),
        ("technology", optional_json(element.technology.as_deref())),
        ("parentId", optional_json(element.parent.as_deref())),
        ("tags", Json::string(&element.tags.join(","))),
        ("kind", Json::string(kind_name(&element.kind))),
        ("properties", properties_json(&element.properties)),
    ])
}

fn software_system_json(workspace: &Workspace, system: &Element) -> Json {
    let mut fields = match element_json(system) {
        Json::Object(fields) => fields,
        _ => unreachable!(),
    };
    fields.push((
        "containers",
        Json::Array(
            workspace
                .elements
                .iter()
                .filter(|element| {
                    element.kind == ElementKind::Container
                        && element.parent.as_deref() == Some(&system.id)
                })
                .map(|container| container_json(workspace, container))
                .collect(),
        ),
    ));
    Json::Object(fields)
}

fn container_json(workspace: &Workspace, container: &Element) -> Json {
    let mut fields = match element_json(container) {
        Json::Object(fields) => fields,
        _ => unreachable!(),
    };
    fields.push((
        "components",
        Json::Array(
            workspace
                .elements
                .iter()
                .filter(|element| {
                    element.kind == ElementKind::Component
                        && element.parent.as_deref() == Some(&container.id)
                })
                .map(element_json)
                .collect(),
        ),
    ));
    Json::Object(fields)
}

fn relationship_json(index: usize, relationship: &Relationship) -> Json {
    Json::object(vec![
        ("id", Json::string(&(index + 1).to_string())),
        ("sourceId", Json::string(&relationship.source)),
        ("destinationId", Json::string(&relationship.destination)),
        (
            "description",
            optional_json(relationship.description.as_deref()),
        ),
        (
            "technology",
            optional_json(relationship.technology.as_deref()),
        ),
        ("tags", Json::string(&relationship.tags.join(","))),
        ("properties", properties_json(&relationship.properties)),
    ])
}

fn view_json(workspace: &Workspace, view: &View) -> Json {
    let graph = compiler::view_graph(workspace, view);
    Json::object(vec![
        ("key", Json::string(view_key(view))),
        ("scope", optional_json(view.scope.as_deref())),
        ("description", optional_json(view.description.as_deref())),
        ("title", optional_json(view.title.as_deref())),
        (
            "elements",
            Json::Array(
                graph
                    .element_ids
                    .iter()
                    .map(|id| Json::object(vec![("id", Json::string(id))]))
                    .collect(),
            ),
        ),
        (
            "relationships",
            Json::Array(
                graph
                    .relationships
                    .iter()
                    .enumerate()
                    .map(|(position, relationship)| {
                        Json::object(vec![(
                            "id",
                            Json::string(&relationship.relationship_index.map_or_else(
                                || format!("dynamic-{}", position + 1),
                                |index| (index + 1).to_string(),
                            )),
                        )])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn styles_json(workspace: &Workspace) -> Json {
    Json::object(vec![
        (
            "elements",
            Json::Array(
                workspace
                    .element_styles
                    .iter()
                    .map(|style| {
                        Json::object(vec![
                            ("tag", Json::string(&style.tag)),
                            ("mode", Json::string(style_mode(style.mode))),
                            ("values", properties_json(&style.values)),
                            ("properties", properties_json(&style.properties)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "relationships",
            Json::Array(
                workspace
                    .relationship_styles
                    .iter()
                    .map(|style| {
                        Json::object(vec![
                            ("tag", Json::string(&style.tag)),
                            ("mode", Json::string(style_mode(style.mode))),
                            ("values", properties_json(&style.values)),
                            ("properties", properties_json(&style.properties)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "themes",
            Json::Array(
                workspace
                    .themes
                    .iter()
                    .map(|theme| {
                        Json::object(vec![
                            ("source", Json::string(&theme.source)),
                            ("mode", Json::string(style_mode(theme.mode))),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("terminology", properties_json(&workspace.terminology)),
        (
            "branding",
            workspace.branding.as_ref().map_or(Json::Null, |branding| {
                Json::object(vec![
                    (
                        "logo",
                        optional_json(branding.logo.as_ref().map(|logo| logo.value.as_str())),
                    ),
                    (
                        "fonts",
                        Json::Array(
                            branding
                                .fonts
                                .iter()
                                .map(|font| {
                                    Json::object(vec![
                                        ("name", Json::string(&font.name)),
                                        ("location", optional_json(font.location.as_deref())),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ])
            }),
        ),
    ])
}

fn documentation_json(workspace: &Workspace) -> Json {
    Json::object(vec![
        (
            "sections",
            Json::Array(
                workspace
                    .documentation
                    .iter()
                    .map(|section| {
                        Json::object(vec![
                            ("owner", Json::string(&owner(&section.owner))),
                            ("title", Json::string(&section.title)),
                            (
                                "source",
                                Json::string(&section.source_path.to_string_lossy()),
                            ),
                            ("format", Json::string(&format!("{:?}", section.format))),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "decisions",
            Json::Array(
                workspace
                    .decisions
                    .iter()
                    .map(|decision| {
                        Json::object(vec![
                            ("id", Json::string(&decision.id)),
                            ("title", Json::string(&decision.title)),
                            ("owner", Json::string(&owner(&decision.owner))),
                            ("status", optional_json(decision.status.as_deref())),
                            ("date", optional_json(decision.date.as_deref())),
                            (
                                "source",
                                Json::string(&decision.source_path.to_string_lossy()),
                            ),
                            ("format", Json::string(&format!("{:?}", decision.format))),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn properties_json(properties: &[Property]) -> Json {
    Json::Array(
        properties
            .iter()
            .map(|property| {
                Json::object(vec![
                    ("key", Json::string(&property.key)),
                    ("value", Json::string(&property.value)),
                ])
            })
            .collect(),
    )
}

fn optional_json(value: Option<&str>) -> Json {
    value.map_or(Json::Null, Json::string)
}

fn owner(owner: &DocumentationOwner) -> String {
    match owner {
        DocumentationOwner::Workspace => "workspace".into(),
        DocumentationOwner::Element(identifier) => identifier.clone(),
    }
}

fn style_mode(mode: StyleMode) -> &'static str {
    match mode {
        StyleMode::Default => "default",
        StyleMode::Light => "light",
        StyleMode::Dark => "dark",
    }
}

enum Json {
    Null,
    String(String),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    fn string(value: &str) -> Self {
        Self::String(value.into())
    }

    fn object(fields: Vec<(&'static str, Json)>) -> Self {
        Self::Object(fields)
    }

    fn render(&self, depth: usize) -> String {
        match self {
            Self::Null => "null".into(),
            Self::String(value) => quoted_json(value),
            Self::Array(values) if values.is_empty() => "[]".into(),
            Self::Array(values) => {
                let indent = "  ".repeat(depth + 1);
                let values = values
                    .iter()
                    .map(|value| format!("{indent}{}", value.render(depth + 1)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("[\n{values}\n{}]", "  ".repeat(depth))
            }
            Self::Object(fields) if fields.is_empty() => "{}".into(),
            Self::Object(fields) => {
                let indent = "  ".repeat(depth + 1);
                let fields = fields
                    .iter()
                    .map(|(key, value)| {
                        format!("{indent}{}: {}", quoted_json(key), value.render(depth + 1))
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{fields}\n{}}}", "  ".repeat(depth))
            }
        }
    }
}

fn quoted_json(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn find<'a>(workspace: &'a Workspace, identifier: &str) -> Option<&'a Element> {
    workspace
        .elements
        .iter()
        .find(|element| element.id == identifier)
}

fn element_index(workspace: &Workspace, identifier: &str) -> Option<usize> {
    workspace
        .elements
        .iter()
        .position(|element| element.id == identifier)
}

fn property_value<'a>(properties: &'a [Property], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .rev()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn alias(identifier: &str) -> String {
    let mut output = String::from("n_");
    for character in identifier.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            output.push(character);
        } else {
            output.push('_');
            output.push_str(&format!("{:x}", character as u32));
        }
    }
    output
}

fn kind_name(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::SoftwareSystem => "SoftwareSystem",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Generic => "GenericElement",
        ElementKind::DeploymentEnvironment => "DeploymentEnvironment",
        ElementKind::DeploymentGroup => "DeploymentGroup",
        ElementKind::DeploymentNode => "DeploymentNode",
        ElementKind::InfrastructureNode => "InfrastructureNode",
        ElementKind::SoftwareSystemInstance => "SoftwareSystemInstance",
        ElementKind::ContainerInstance => "ContainerInstance",
        ElementKind::ArchiMate(kind) => kind,
    }
}

fn view_kind_name(kind: &ViewKind) -> &'static str {
    match kind {
        ViewKind::SystemLandscape => "SystemLandscape",
        ViewKind::SystemContext => "SystemContext",
        ViewKind::Container => "Container",
        ViewKind::Component => "Component",
        ViewKind::Filtered => "Filtered",
        ViewKind::Dynamic => "Dynamic",
        ViewKind::Deployment => "Deployment",
        ViewKind::Custom => "Custom",
        ViewKind::Image => "Image",
        ViewKind::ArchiMate => "ArchiMate",
    }
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", escaped(value))
}

fn escaped(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "")
        .replace('\n', "\\n")
}

fn plantuml_label(value: &str) -> String {
    escaped(value).replace(':', "\\:")
}

fn xml_text(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '\t' | '\n' | '\r' => output.push(character),
            character if character.is_control() => output.push('\u{fffd}'),
            character => output.push(character),
        }
    }
    output
}

fn xml_attr(value: &str) -> String {
    xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
}

fn has_styles(workspace: &Workspace) -> bool {
    !workspace.element_styles.is_empty()
        || !workspace.relationship_styles.is_empty()
        || !workspace.themes.is_empty()
        || workspace.branding.is_some()
}

fn append_style_note(workspace: &Workspace, output: &mut String, comment: &str) {
    if has_styles(workspace) {
        output.push_str(&format!(
            "{comment} c4c: Mermaid supports M5 styles; this exporter uses deterministic defaults.\n"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{compile_file, compile_file_with_options, validate, CompileOptions};
    use std::{
        collections::HashSet,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn exports_all_m8_text_formats_deterministically() {
        let workspace = compile_file("examples/internet-banking.dsl").unwrap();
        validate(&workspace).unwrap();
        let output = temporary_directory("text");
        for format in [
            "json",
            "mermaid",
            "d2",
            "plantuml",
            "c4plantuml",
            "dot",
            "drawio",
            "archimate",
        ] {
            export(
                &workspace,
                format,
                &output,
                ExportOptions { strict_safe: false },
            )
            .unwrap();
        }
        let expected = [
            "workspace.json",
            "system-context.mmd",
            "container.mmd",
            "system-context.d2",
            "container.d2",
            "system-context.puml",
            "container.puml",
            "system-context.dot",
            "container.dot",
            "system-context.drawio",
            "container.drawio",
            "workspace.archimate.xml",
        ];
        for file in expected {
            assert!(output.join(file).is_file(), "missing {file}");
        }
        let json_first = fs::read_to_string(output.join("workspace.json")).unwrap();
        export(
            &workspace,
            "json",
            &output,
            ExportOptions { strict_safe: false },
        )
        .unwrap();
        assert_eq!(
            json_first,
            fs::read_to_string(output.join("workspace.json")).unwrap()
        );
        assert!(json_first.contains("\"people\""));
        assert!(json_first.contains("\"softwareSystems\""));
        assert!(json_first.contains("\"containers\""));
        assert!(json_first.contains("\"relationships\""));
        assert!(json_first.contains("\"systemContextViews\""));
        let view = &workspace.views[0];
        assert!(d2(&workspace, view).contains("n_customer -> n_bank: \"Uses\""));
        let plantuml = plantuml(&workspace, view);
        assert!(plantuml.starts_with("@startuml\n"));
        assert!(plantuml.contains("n_customer --> n_bank : Uses"));
        assert!(plantuml.ends_with("@enduml\n"));
        let c4 = c4plantuml(&workspace, view);
        assert!(c4.contains("Person(n_customer"));
        assert!(c4.contains("Rel(n_customer, n_bank, \"Uses\")"));
        assert!(!c4.to_ascii_lowercase().contains("!includeurl"));
        let dot = dot(&workspace, view);
        assert!(dot.contains("digraph \"system-context\""));
        assert!(dot.contains("\"customer\" -> \"bank\" [label=\"Uses\"]"));
        let drawio = drawio(&workspace, view);
        assert!(drawio.contains("<mxfile"));
        assert!(drawio.contains("source=\"element-1\" target=\"element-2\""));
        fs::remove_dir_all(output).unwrap();
    }

    #[test]
    fn exporters_escape_content_and_never_emit_remote_plantuml_includes() {
        let workspace = compile_file("tests/fixtures/m8-exporters.dsl").unwrap();
        validate(&workspace).unwrap();
        let c4 = c4plantuml(&workspace, &workspace.views[0]);
        assert!(c4.contains("Person("));
        assert!(c4.contains("Rel("));
        assert!(!c4.to_ascii_lowercase().contains("!includeurl"));
        assert!(!plantuml(&workspace, &workspace.views[0]).contains("!include"));
        assert!(dot(&workspace, &workspace.views[0]).contains("digraph"));
        let drawio = drawio(&workspace, &workspace.views[0]);
        assert!(drawio.contains("&lt;Admin&gt; &amp; Owner"));
        assert!(drawio.contains("<mxfile"));
        let archimate = archimate(&workspace);
        assert!(archimate.contains("xsi:type=\"BusinessActor\""));
        assert!(archimate.contains("xsi:type=\"ApplicationComponent\""));
        assert!(archimate.contains("xsi:type=\"Association\""));
        assert!(archimate.contains("User &lt;Admin&gt; &amp; Owner"));
        assert!(archimate.contains("c4c.id"));
    }

    #[test]
    fn exports_deterministic_archi_native_models_with_editable_views() {
        let workspace = compile_file_with_options(
            "tests/fixtures/m8-exporters.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
                strict: false,
            },
        )
        .unwrap();
        validate(&workspace).unwrap();
        let native = archi_native(&workspace);
        assert_eq!(native, archi_native(&workspace));
        assert!(native.contains("xmlns:archimate=\"http://www.archimatetool.com/archimate\""));
        assert!(native.contains("version=\"5.0.0\""));
        for folder in [
            "strategy",
            "business",
            "application",
            "technology",
            "motivation",
            "implementation_migration",
            "other",
            "relations",
            "diagrams",
        ] {
            assert!(native.contains(&format!("type=\"{folder}\"")));
        }
        for native_type in [
            "BusinessActor",
            "ApplicationComponent",
            "Node",
            "Grouping",
            "AssociationRelationship",
            "ArchimateDiagramModel",
            "DiagramObject",
            "Connection",
        ] {
            assert!(native.contains(&format!("xsi:type=\"archimate:{native_type}\"")));
        }
        assert_eq!(
            native
                .matches("xsi:type=\"archimate:ArchimateDiagramModel\"")
                .count(),
            workspace.views.len()
        );
        assert!(native.contains("width=\"180\" height=\"80\"/>"));
        assert!(native.contains("<sourceConnection xsi:type=\"archimate:Connection\""));
        assert_eq!(native.matches("<bendpoint ").count() % 2, 0);
        assert!(
            native.matches("<bendpoint ").count()
                < native.matches("<sourceConnection ").count() * 2
        );
        assert!(native.contains("connectionRouterType=\"2\""));
        assert!(native.contains("Milestone 8 &lt;Exporters&gt; &amp; Exchange"));
        assert!(native.contains("User &lt;Admin&gt; &amp; Owner"));
        assert!(native.contains("name=\"Open orders\""));
        assert!(native.contains("name=\"Post entry\""));
        assert_archi_fill_colors_only_on_visual_children(&native);
        let container_view = native
            .split("id=\"id-c4c-view-containers\"")
            .nth(1)
            .unwrap()
            .split("    </element>")
            .next()
            .unwrap();
        let scope = container_view
            .find("id=\"id-c4c-viewobject-containers-system\"")
            .unwrap();
        let container = container_view
            .find("id=\"id-c4c-viewobject-containers-system_2eapi\"")
            .unwrap();
        let scope_end = scope + container_view[scope..].find("\n      </child>").unwrap();
        assert!(container > scope && container < scope_end);
        assert_archi_connection_integrity(&native);
        assert_archi_routes_clear_boxes(&workspace);

        let output = temporary_directory("archi-native");
        for format in ["archi", "archi-native", "archimate-native"] {
            export(
                &workspace,
                format,
                &output,
                ExportOptions { strict_safe: true },
            )
            .unwrap();
            assert_eq!(
                fs::read_to_string(output.join("workspace.archimate")).unwrap(),
                native
            );
        }
        let open_group = archimate(&workspace);
        export(
            &workspace,
            "archimate",
            &output,
            ExportOptions { strict_safe: true },
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(output.join("workspace.archimate.xml")).unwrap(),
            open_group
        );
        let normalized_open_group = open_group.replace(env!("CARGO_MANIFEST_DIR"), "$ROOT");
        assert_eq!(fnv1a(&normalized_open_group), 5_863_218_997_552_418_175);
        assert!(open_group.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model xmlns=\"http://www.opengroup.org/xsd/archimate/3.0/\""));
        assert!(!open_group.contains("ArchimateDiagramModel"));
        fs::remove_dir_all(output).unwrap();
    }

    #[test]
    fn exports_m83_explicit_archimate_types_and_layout() {
        let workspace = compile_file_with_options(
            "tests/fixtures/m83-archimate-profile.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
                strict: false,
            },
        )
        .unwrap();
        validate(&workspace).unwrap();
        let native = archi_native(&workspace);
        for value in [
            "BusinessActor",
            "ApplicationComponent",
            "Node",
            "FlowRelationship",
            "AccessRelationship",
            "ArchimateDiagramModel",
            "DiagramObject",
            "Connection",
        ] {
            assert!(native.contains(&format!("xsi:type=\"archimate:{value}\"")));
        }
        assert!(native.contains("fillColor=\"#008e00\""));
        assert!(native.contains("<bounds x=\"300\" y=\"120\" width=\"180\" height=\"80\"/>"));
        assert!(!native.contains("lineStyle="));
        assert_archi_fill_colors_only_on_visual_children(&native);
        assert_archi_connection_integrity(&native);
        let open_group = archimate(&workspace);
        assert!(open_group.contains("xsi:type=\"ApplicationComponent\""));
        assert!(open_group.contains("xsi:type=\"Flow\""));
        assert!(open_group.contains("c4c.archimate.background"));
        let output = temporary_directory("m83");
        export(
            &workspace,
            "archi",
            &output,
            ExportOptions { strict_safe: true },
        )
        .unwrap();
        assert!(output.join("workspace.archimate").is_file());
        fs::remove_dir_all(output).unwrap();
    }

    #[test]
    fn exports_m84_archimate_semantics_without_native_warnings() {
        let workspace = compile_file("tests/fixtures/m84-archimate-conformance.dsl").unwrap();
        validate(&workspace).unwrap();
        let native = archi_native(&workspace);
        assert!(!native.contains("lineStyle="));
        for value in [
            "BusinessActor",
            "ApplicationComponent",
            "Node",
            "Junction",
            "FlowRelationship",
            "AccessRelationship",
            "ArchimateDiagramModel",
            "DiagramObject",
            "Connection",
        ] {
            assert!(native.contains(&format!("xsi:type=\"archimate:{value}\"")));
        }
        assert!(native.contains("id-c4c-relationship-rAssignment"));
        assert_archi_fill_colors_only_on_visual_children(&native);
        assert_archi_connection_integrity(&native);
        let open_group = archimate(&workspace);
        assert!(open_group.contains("xsi:type=\"Access\" accessType=\"Read\""));
        assert!(open_group.contains("xsi:type=\"Flow\""));
        assert!(open_group.contains("c4c.archimate.access"));
        let output = temporary_directory("m84-archi");
        export(
            &workspace,
            "archi",
            &output,
            ExportOptions { strict_safe: true },
        )
        .unwrap();
        fs::remove_dir_all(output).unwrap();
    }

    #[test]
    fn exports_m85_readable_viewpoint_layouts() {
        let workspace = compile_file("tests/fixtures/m85-archimate-views-layout.dsl").unwrap();
        validate(&workspace).unwrap();
        let native = archi_native(&workspace);
        assert_eq!(native, archi_native(&workspace));
        assert!(!native.contains("lineStyle="));
        assert!(!native.contains("name=\"\""));
        assert_archi_fill_colors_only_on_visual_children(&native);
        for value in ["ArchimateDiagramModel", "DiagramObject", "Connection"] {
            assert!(native.contains(&format!("xsi:type=\"archimate:{value}\"")));
        }
        assert_archi_connection_integrity(&native);

        let app = workspace
            .views
            .iter()
            .find(|view| view.key.as_deref() == Some("m85-app"))
            .unwrap();
        let app_layout = archi_native_test_layout(&workspace, app);
        assert_eq!(
            app_layout["gateway"],
            ArchiNativeBounds {
                x: 1000,
                y: 320,
                width: 220,
                height: 90,
            }
        );
        assert_layout_has_no_overlaps(&app_layout);
        assert!(app_layout["operator"].x < app_layout["gateway"].x);
        assert!(app_layout["gateway"].x < app_layout["ledger"].x);
        assert!(
            app_layout
                .values()
                .map(|bounds| bounds.x)
                .collect::<HashSet<_>>()
                .len()
                > 2
        );

        let motivation = workspace
            .views
            .iter()
            .find(|view| view.key.as_deref() == Some("m85-motivation"))
            .unwrap();
        let motivation_layout = archi_native_test_layout(&workspace, motivation);
        assert_layout_has_no_overlaps(&motivation_layout);
        assert!(motivation_layout["regulation"].y < motivation_layout["traceabilityGoal"].y);
        assert!(motivation_layout["traceabilityGoal"].y < motivation_layout["auditRequirement"].y);

        let technology = workspace
            .views
            .iter()
            .find(|view| view.key.as_deref() == Some("m85-technology"))
            .unwrap();
        let technology_layout = archi_native_test_layout(&workspace, technology);
        assert_layout_has_no_overlaps(&technology_layout);
        assert_eq!(technology_layout["runtime"].y, 380);
        assert!(technology_layout["techService"].y < technology_layout["runtime"].y);
        assert!(technology_layout["runtime"].y < technology_layout["gatewayArtifact"].y);
    }

    #[test]
    fn archi_native_layout_places_connected_objects_on_clear_short_routes() {
        let workspace = crate::compiler::compile(
            r#"workspace "Routing" {
  model {
    group "Upper" {
      hub = softwareSystem "Hub"
      peer1 = softwareSystem "Peer 1"
      peer2 = softwareSystem "Peer 2"
      peer3 = softwareSystem "Peer 3"
    }
    group "Lower" {
      target = softwareSystem "Target"
    }
    group "Flat" {
      flat1 = softwareSystem "Flat 1"
      flat2 = softwareSystem "Flat 2"
    }
    outside = softwareSystem "Outside"
    hub -> peer1
    hub -> peer2
    hub -> peer3
    hub -> target
    outside -> hub
  }
  views {
    systemLandscape routing {
      include *
      autolayout tb
    }
  }
}"#,
        )
        .unwrap();
        let view = &workspace.views[0];
        let graph = compiler::view_graph(&workspace, view);
        let objects = archi_native_object_map(&workspace, view, &graph);
        let connections = archi_native_connections(&workspace, view, &graph, &objects);
        let layout = archi_native_layout(&workspace, view, &graph, &connections);
        let upper = layout.groups[&0];
        assert_eq!(upper.height, 800);
        assert!(layout["hub"].width * 5 >= upper.width * 4);
        let flat = layout.groups[&2];
        assert_eq!(flat.height, 140);
        assert_eq!(layout["flat1"].width, 180);
        assert_eq!(layout["flat2"].width, 180);
        assert!(layout
            .groups
            .values()
            .all(|bounds| bounds.x == upper.x && bounds.width == upper.width));
        assert_eq!(
            layout["hub"].x + layout["hub"].width / 2,
            layout["target"].x + layout["target"].width / 2
        );
        let outside = layout["outside"];
        assert!(
            outside.x + outside.width + 60 == upper.x || outside.x == upper.x + upper.width + 60
        );
        assert!(connections.iter().enumerate().all(|(route, connection)| {
            archi_native_bendpoints(&workspace, view, &layout, connection, route).is_none()
        }));
    }

    fn assert_archi_connection_integrity(native: &str) {
        let relationships = native
            .lines()
            .filter(|line| line.contains("Relationship\""))
            .filter_map(|line| xml_attribute(line, "id"))
            .collect::<HashSet<_>>();
        for diagram in native
            .split("<element xsi:type=\"archimate:ArchimateDiagramModel\"")
            .skip(1)
        {
            let diagram = diagram.split("    </element>").next().unwrap();
            let objects = diagram
                .lines()
                .filter(|line| line.contains("xsi:type=\"archimate:DiagramObject\""))
                .map(|line| {
                    (
                        xml_attribute(line, "id").unwrap(),
                        xml_attribute(line, "targetConnections").unwrap_or(""),
                    )
                })
                .collect::<HashMap<_, _>>();
            for line in diagram
                .lines()
                .filter(|line| line.contains("xsi:type=\"archimate:Connection\""))
            {
                let id = xml_attribute(line, "id").unwrap();
                let source = xml_attribute(line, "source").unwrap();
                let target = xml_attribute(line, "target").unwrap();
                let relationship = xml_attribute(line, "archimateRelationship").unwrap();
                assert!(
                    objects.contains_key(source),
                    "missing source object {source}"
                );
                assert!(
                    objects.contains_key(target),
                    "missing target object {target}"
                );
                assert!(
                    relationships.contains(relationship),
                    "missing relationship {relationship}"
                );
                assert!(
                    objects[target].split_whitespace().any(|value| value == id),
                    "target {target} does not reference connection {id}"
                );
            }
        }
    }

    fn assert_archi_fill_colors_only_on_visual_children(native: &str) {
        for line in native.lines().filter(|line| line.contains("fillColor=")) {
            assert!(
                line.contains("<child ")
                    && (line.contains("xsi:type=\"archimate:DiagramObject\"")
                        || line.contains("xsi:type=\"archimate:Group\"")),
                "{line}"
            );
        }
    }

    fn assert_archi_routes_clear_boxes(workspace: &Workspace) {
        for view in &workspace.views {
            let graph = compiler::view_graph(workspace, view);
            let objects = archi_native_object_map(workspace, view, &graph);
            let connections = archi_native_connections(workspace, view, &graph, &objects);
            let layout = archi_native_layout(workspace, view, &graph, &connections);
            let vertical = view.auto_layout.as_ref().is_some_and(|layout| {
                matches!(layout.direction.to_ascii_lowercase().as_str(), "tb" | "bt")
            });
            for (route, connection) in connections.iter().enumerate() {
                if let Some(bends) =
                    archi_native_bendpoints(workspace, view, &layout, connection, route)
                {
                    let source = layout[&connection.source_element];
                    if vertical {
                        let lane = source.x + source.width / 2 + bends[0].0;
                        assert!(layout.values().all(|bounds| lane < bounds.x));
                    } else {
                        let lane = source.y + source.height / 2 + bends[0].1;
                        assert!(layout.values().all(|bounds| lane < bounds.y));
                    }
                    continue;
                }
                let source = layout[&connection.source_element];
                let target = layout[&connection.destination_element];
                assert!(layout.iter().all(|(identifier, bounds)| {
                    identifier == &connection.source_element
                        || identifier == &connection.destination_element
                        || !segment_crosses_bounds(
                            source.x + source.width / 2,
                            source.y + source.height / 2,
                            target.x + target.width / 2,
                            target.y + target.height / 2,
                            *bounds,
                        )
                }));
            }
        }
    }

    fn archi_native_test_layout(
        workspace: &Workspace,
        view: &View,
    ) -> HashMap<String, ArchiNativeBounds> {
        let graph = compiler::view_graph(workspace, view);
        let objects = archi_native_object_map(workspace, view, &graph);
        let connections = archi_native_connections(workspace, view, &graph, &objects);
        archi_native_layout(workspace, view, &graph, &connections).objects
    }

    fn assert_layout_has_no_overlaps(layout: &HashMap<String, ArchiNativeBounds>) {
        let items = layout.iter().collect::<Vec<_>>();
        for (index, (left_id, left)) in items.iter().enumerate() {
            for (right_id, right) in items.iter().skip(index + 1) {
                assert!(
                    !bounds_overlap(**left, **right),
                    "{left_id} overlaps {right_id}: {left:?} {right:?}"
                );
            }
        }
    }

    fn xml_attribute<'a>(line: &'a str, name: &str) -> Option<&'a str> {
        let needle = format!("{name}=\"");
        let start = line.find(&needle)? + needle.len();
        let end = line[start..].find('"')? + start;
        Some(&line[start..end])
    }

    fn fnv1a(value: &str) -> u64 {
        value.bytes().fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
    }

    #[test]
    fn html_delegates_to_m7_and_strict_safe_rejects_renderers() {
        let workspace = compile_file("tests/fixtures/m7-docs.dsl").unwrap();
        let output = temporary_directory("html");
        export(
            &workspace,
            "site",
            &output,
            ExportOptions { strict_safe: false },
        )
        .unwrap();
        assert!(output.join("index.html").is_file());
        assert!(output.join("diagrams/context.mmd").is_file());
        let error = export(
            &workspace,
            "svg",
            &output,
            ExportOptions { strict_safe: true },
        )
        .unwrap_err();
        assert!(error.contains("strict-safe"));
        assert!(error.contains("No renderer was executed"));
        assert!(error.contains("no network request was made"));
        assert!(missing_renderer("png").contains("No network request was made"));
        fs::remove_dir_all(output).unwrap();
    }

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "c4c-m8-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
