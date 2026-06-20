use crate::compiler::{
    self, DocumentationOwner, Element, ElementKind, Property, Relationship, StyleMode, View,
    ViewKind, Workspace,
};
use std::{
    fs,
    io::Write,
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
        let properties = [
            ("id", Some(element.id.as_str())),
            ("kind", Some(kind_name(&element.kind))),
            ("parent", element.parent.as_deref()),
            ("technology", element.technology.as_deref()),
            ("tags", tags.as_deref()),
            ("description", element.description.as_deref()),
            ("source", source.path.to_str()),
        ];
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
        output.push_str(&format!(
            "    <relationship identifier=\"id-relationship-{}\" source=\"{}\" target=\"{}\" xsi:type=\"Association\">\n",
            index + 1,
            archimate_element_id(source_index, &workspace.elements[source_index]),
            archimate_element_id(destination_index, &workspace.elements[destination_index])
        ));
        if let Some(description) = &relationship.description {
            output.push_str(&format!(
                "      <name xml:lang=\"en\">{}</name>\n",
                xml_text(description)
            ));
        }
        let (source_file, _) = workspace.source_map.resolve(relationship.span);
        let properties = [
            ("id", Some(format!("relationship-{}", index + 1))),
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
        ];
        let borrowed = properties
            .iter()
            .map(|(key, value)| (*key, value.as_deref()))
            .collect::<Vec<_>>();
        append_archimate_properties(&mut output, &borrowed, 6);
        output.push_str("    </relationship>\n");
    }
    output.push_str("  </relationships>\n  <propertyDefinitions>\n");
    for (id, name) in DEFINITIONS {
        output.push_str(&format!(
            "    <propertyDefinition identifier=\"property-{id}\" type=\"string\"><name xml:lang=\"en\">{name}</name></propertyDefinition>\n"
        ));
    }
    output.push_str("  </propertyDefinitions>\n</model>\n");
    output
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
        for element in workspace
            .elements
            .iter()
            .filter(|element| archi_native_folder(&element.kind) == *folder_type)
        {
            output.push_str(&format!(
                "    <element xsi:type=\"archimate:{}\" name=\"{}\" id=\"{}\"",
                archi_native_type(&element.kind),
                xml_attr(&element.name),
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
    output.push_str("</archimate:model>\n");
    output
}

fn append_archi_native_relationships(output: &mut String, workspace: &Workspace) {
    for (index, relationship) in workspace.relationships.iter().enumerate() {
        append_archi_native_relationship(
            output,
            &archi_native_relationship_id(index),
            &relationship.source,
            &relationship.destination,
            relationship.description.as_deref(),
        );
    }
    for view in &workspace.views {
        let graph = compiler::view_graph(workspace, view);
        for (position, relationship) in graph.relationships.iter().enumerate() {
            let (id, synthetic) =
                archi_native_view_relationship_id(workspace, view, relationship, position);
            if synthetic {
                append_archi_native_relationship(
                    output,
                    &id,
                    &relationship.source,
                    &relationship.destination,
                    Some(&relationship.description),
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
) {
    output.push_str(&format!(
        "    <element xsi:type=\"archimate:TriggeringRelationship\" id=\"{}\" source=\"{}\" target=\"{}\"",
        xml_attr(id),
        archi_native_element_id(source),
        archi_native_element_id(destination)
    ));
    if let Some(description) = description.filter(|description| !description.is_empty()) {
        output.push_str(&format!(" name=\"{}\"", xml_attr(description)));
    }
    output.push_str("/>\n");
}

fn append_archi_native_views(output: &mut String, workspace: &Workspace) {
    for view in &workspace.views {
        let graph = compiler::view_graph(workspace, view);
        let key = safe_name(view_key(view));
        output.push_str(&format!(
            "    <element xsi:type=\"archimate:ArchimateDiagramModel\" name=\"{}\" id=\"id-c4c-view-{key}\" connectionRouterType=\"2\">\n",
            xml_attr(view.title.as_deref().unwrap_or(view_key(view)))
        ));
        for (position, identifier) in graph.element_ids.iter().enumerate() {
            let Some(element) = find(workspace, identifier) else {
                continue;
            };
            let object_id = archi_native_object_id(&key, identifier);
            output.push_str(&format!(
                "      <child xsi:type=\"archimate:DiagramObject\" id=\"{object_id}\" archimateElement=\"{}\" type=\"1\" fillColor=\"{}\">\n",
                archi_native_element_id(identifier),
                archi_native_fill(&element.kind)
            ));
            let (x, y) = archi_native_position(view, position);
            output.push_str(&format!(
                "        <bounds x=\"{x}\" y=\"{y}\" width=\"180\" height=\"80\"/>\n"
            ));
            for (relationship_position, relationship) in graph
                .relationships
                .iter()
                .enumerate()
                .filter(|(_, relationship)| relationship.source == *identifier)
            {
                let (relationship_id, _) = archi_native_view_relationship_id(
                    workspace,
                    view,
                    relationship,
                    relationship_position,
                );
                output.push_str(&format!(
                    "        <sourceConnection xsi:type=\"archimate:Connection\" id=\"id-c4c-connection-{key}-{}\" source=\"{object_id}\" target=\"{}\" archimateRelationship=\"{relationship_id}\"/>\n",
                    relationship_position + 1,
                    archi_native_object_id(&key, &relationship.destination)
                ));
            }
            output.push_str("      </child>\n");
        }
        output.push_str("    </element>\n");
    }
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
        workspace.relationships.iter().position(|model| {
            model.source == relationship.source && model.destination == relationship.destination
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
        |index| (archi_native_relationship_id(index), false),
    )
}

fn archi_native_position(view: &View, position: usize) -> (usize, usize) {
    // ponytail: stable grid first; add boundary-aware layout only when manual use demands it.
    let vertical = view.auto_layout.as_ref().is_some_and(|layout| {
        matches!(layout.direction.to_ascii_lowercase().as_str(), "tb" | "bt")
    });
    let (column, row) = if vertical {
        (position / 4, position % 4)
    } else {
        (position % 4, position / 4)
    };
    (40 + column * 260, 40 + row * 160)
}

fn archi_native_element_id(identifier: &str) -> String {
    format!("id-c4c-element-{}", safe_name(identifier))
}

fn archi_native_relationship_id(index: usize) -> String {
    format!("id-c4c-relationship-{}", index + 1)
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
    }
}

fn archi_native_fill(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "#ffffb5",
        ElementKind::DeploymentNode | ElementKind::InfrastructureNode => "#c9d9ff",
        ElementKind::Generic
        | ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup => "#eeeeee",
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
            "TriggeringRelationship",
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
        assert!(native.contains("<bounds x=\"40\" y=\"40\" width=\"180\" height=\"80\"/>"));
        assert!(native.contains("<sourceConnection xsi:type=\"archimate:Connection\""));
        assert!(native.contains("Milestone 8 &lt;Exporters&gt; &amp; Exchange"));
        assert!(native.contains("User &lt;Admin&gt; &amp; Owner"));

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
        assert!(open_group.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model xmlns=\"http://www.opengroup.org/xsd/archimate/3.0/\""));
        assert!(!open_group.contains("ArchimateDiagramModel"));
        fs::remove_dir_all(output).unwrap();
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
