#![allow(dead_code)] // Native metadata is preserved for sidecar compatibility and exercised by tests.

use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlElement {
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub text: String,
    pub children: Vec<XmlElement>,
}

#[derive(Clone, Debug)]
pub struct ArchiNativeModel {
    pub name: String,
    pub id: String,
    pub version: String,
    pub purpose: Option<String>,
    pub attributes: BTreeMap<String, String>,
    pub folders: Vec<ArchiFolder>,
}

#[derive(Clone, Debug)]
pub struct ArchiFolder {
    pub name: String,
    pub id: String,
    pub folder_type: String,
    pub attributes: BTreeMap<String, String>,
    pub elements: Vec<ArchiElement>,
    pub relationships: Vec<ArchiRelationship>,
    pub diagrams: Vec<ArchiDiagram>,
}

#[derive(Clone, Debug)]
pub struct ArchiElement {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub native_type: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ArchiRelationship {
    pub id: String,
    pub name: String,
    pub native_type: String,
    pub source: String,
    pub target: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ArchiDiagram {
    pub id: String,
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<ArchiDiagramChild>,
}

#[derive(Clone, Debug)]
pub enum ArchiDiagramChild {
    Object(ArchiDiagramObject),
    Group(ArchiGroup),
}

#[derive(Clone, Debug)]
pub struct ArchiDiagramObject {
    pub id: String,
    pub element: String,
    pub bounds: Option<ArchiBounds>,
    pub target_connections: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub connections: Vec<ArchiConnection>,
    pub children: Vec<ArchiDiagramChild>,
}

#[derive(Clone, Debug)]
pub struct ArchiGroup {
    pub id: String,
    pub name: String,
    pub bounds: Option<ArchiBounds>,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<ArchiDiagramChild>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArchiBounds {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Clone, Debug)]
pub struct ArchiConnection {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relationship: String,
    pub attributes: BTreeMap<String, String>,
    pub bendpoints: Vec<BTreeMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ArchiSidecar {
    pub format_version: u32,
    pub projection_hash: String,
    pub source_xml: String,
}

pub fn parse(input: &str) -> Result<ArchiNativeModel, String> {
    model_from_xml(&parse_xml(input)?)
}

pub fn import(input: &Path, output: &Path, sidecar: &Path) -> Result<(), String> {
    let source_xml = read_utf8(input)?;
    let model = parse(&source_xml)?;
    for diagnostic in validate_connections(&model) {
        eprintln!("warning: {diagnostic}");
    }
    let dsl = project_to_dsl(&model);
    write(output, &dsl)?;
    let metadata = ArchiSidecar {
        format_version: 1,
        projection_hash: hash(&dsl),
        source_xml,
    };
    let json = serde_json::to_string_pretty(&metadata)
        .map_err(|error| format!("cannot serialize Archi sidecar: {error}"))?;
    write(sidecar, &format!("{json}\n"))?;
    Ok(())
}

pub fn sidecar_xml(sidecar: &Path, projection: &Path) -> Result<Option<String>, String> {
    let json = read_utf8(sidecar)?;
    let sidecar: ArchiSidecar = serde_json::from_str(&json)
        .map_err(|error| format!("invalid Archi sidecar {}: {error}", sidecar.display()))?;
    if sidecar.format_version != 1 {
        return Err(format!(
            "unsupported Archi sidecar version {}",
            sidecar.format_version
        ));
    }
    if hash(&read_utf8(projection)?) != sidecar.projection_hash {
        return Ok(None);
    }
    let model = parse(&sidecar.source_xml)?;
    if let Some(diagnostic) = validate_connections(&model).first() {
        return Err(format!("invalid Archi sidecar connection: {diagnostic}"));
    }
    Ok(Some(sidecar.source_xml))
}

pub fn diff_files(a: &Path, b: &Path) -> Result<(), String> {
    let a_xml = read_utf8(a)?;
    let b_xml = read_utf8(b)?;
    for (path, model) in [(a, parse(&a_xml)?), (b, parse(&b_xml)?)] {
        if let Some(diagnostic) = validate_connections(&model).first() {
            return Err(format!("{}: {diagnostic}", path.display()));
        }
    }
    canonical_diff(&parse_xml(&a_xml)?, &parse_xml(&b_xml)?).map_or(Ok(()), Err)
}

pub fn semantic_diff_files(a: &Path, b: &Path) -> Result<(), String> {
    let a = parse(&read_utf8(a)?)?;
    let b = parse(&read_utf8(b)?)?;
    for (path, model) in [(a.name.as_str(), &a), (b.name.as_str(), &b)] {
        if let Some(diagnostic) = validate_connections(model).first() {
            return Err(format!("{path}: {diagnostic}"));
        }
    }
    let (a, b) = (semantic_signature(&a)?, semantic_signature(&b)?);
    if a == b {
        return Ok(());
    }
    let index = a
        .iter()
        .zip(&b)
        .position(|(a, b)| a != b)
        .unwrap_or(a.len().min(b.len()));
    Err(format!(
        "semantic model differs at item {index}: {:?} vs {:?}",
        a.get(index),
        b.get(index)
    ))
}

pub fn canonical_diff(a: &XmlElement, b: &XmlElement) -> Option<String> {
    compare_xml(a, b, &format!("/{}", a.name))
}

fn semantic_signature(model: &ArchiNativeModel) -> Result<Vec<String>, String> {
    let mut signature = vec![
        format!("model|{}", model.name),
        format!("purpose|{}", model.purpose.as_deref().unwrap_or("")),
    ];
    let mut elements = HashMap::new();
    let mut relationships = HashMap::new();
    for folder in &model.folders {
        signature.push(format!("folder|{}|{}", folder.folder_type, folder.name));
        for element in &folder.elements {
            let value = format!(
                "{}|{}|{}",
                element.native_type,
                element.name,
                element.description.as_deref().unwrap_or("")
            );
            elements.insert(element.id.as_str(), value.clone());
            signature.push(format!("element|{value}"));
        }
    }
    for relationship in model
        .folders
        .iter()
        .flat_map(|folder| &folder.relationships)
    {
        let source = elements
            .get(relationship.source.as_str())
            .ok_or_else(|| format!("relationship {} has unknown source", relationship.id))?;
        let target = elements
            .get(relationship.target.as_str())
            .ok_or_else(|| format!("relationship {} has unknown target", relationship.id))?;
        let value = format!(
            "{}|{}|{}|{}",
            relationship.native_type, relationship.name, source, target
        );
        relationships.insert(relationship.id.as_str(), value.clone());
        signature.push(format!("relationship|{value}"));
    }
    for diagram in model.folders.iter().flat_map(|folder| &folder.diagrams) {
        signature.push(format!("view|{}", diagram.name));
        let mut objects = HashMap::new();
        collect_objects(&diagram.children, &mut objects);
        collect_semantic_view_children(
            &diagram.name,
            &diagram.children,
            &objects,
            &elements,
            &relationships,
            &mut signature,
        )?;
    }
    signature.sort();
    Ok(signature)
}

fn collect_semantic_view_children(
    diagram: &str,
    children: &[ArchiDiagramChild],
    objects: &HashMap<&str, &ArchiDiagramObject>,
    elements: &HashMap<&str, String>,
    relationships: &HashMap<&str, String>,
    signature: &mut Vec<String>,
) -> Result<(), String> {
    for child in children {
        match child {
            ArchiDiagramChild::Object(object) => {
                let element = elements
                    .get(object.element.as_str())
                    .ok_or_else(|| format!("diagram object {} has unknown element", object.id))?;
                signature.push(format!("view|{diagram}|object|{element}"));
                for connection in &object.connections {
                    let source = objects
                        .get(connection.source.as_str())
                        .and_then(|object| elements.get(object.element.as_str()))
                        .ok_or_else(|| {
                            format!("connection {} has unknown source", connection.id)
                        })?;
                    let target = objects
                        .get(connection.target.as_str())
                        .and_then(|object| elements.get(object.element.as_str()))
                        .ok_or_else(|| {
                            format!("connection {} has unknown target", connection.id)
                        })?;
                    let relationship = relationships
                        .get(connection.relationship.as_str())
                        .ok_or_else(|| {
                            format!("connection {} has unknown relationship", connection.id)
                        })?;
                    signature.push(format!(
                        "view|{diagram}|connection|{source}|{target}|{relationship}"
                    ));
                }
                collect_semantic_view_children(
                    diagram,
                    &object.children,
                    objects,
                    elements,
                    relationships,
                    signature,
                )?;
            }
            ArchiDiagramChild::Group(group) => {
                let mut members = Vec::new();
                collect_semantic_group_members(&group.children, elements, &mut members)?;
                members.sort();
                signature.push(format!(
                    "view|{diagram}|group|{}|{}",
                    group.name,
                    members.join(",")
                ));
                collect_semantic_view_children(
                    diagram,
                    &group.children,
                    objects,
                    elements,
                    relationships,
                    signature,
                )?;
            }
        }
    }
    Ok(())
}

fn collect_semantic_group_members(
    children: &[ArchiDiagramChild],
    elements: &HashMap<&str, String>,
    members: &mut Vec<String>,
) -> Result<(), String> {
    for child in children {
        match child {
            ArchiDiagramChild::Object(object) => {
                members.push(
                    elements
                        .get(object.element.as_str())
                        .ok_or_else(|| format!("diagram object {} has unknown element", object.id))?
                        .clone(),
                );
                collect_semantic_group_members(&object.children, elements, members)?;
            }
            ArchiDiagramChild::Group(group) => {
                collect_semantic_group_members(&group.children, elements, members)?;
            }
        }
    }
    Ok(())
}

pub fn project_to_dsl(model: &ArchiNativeModel) -> String {
    let elements = model
        .folders
        .iter()
        .flat_map(|folder| &folder.elements)
        .collect::<Vec<_>>();
    let mut used_identifiers = HashSet::new();
    let identifiers = elements
        .iter()
        .map(|element| {
            let base = dsl_identifier(&element.name);
            let mut identifier = base.clone();
            let mut suffix = 2;
            while !used_identifiers.insert(identifier.clone()) {
                identifier = format!("{base}_{suffix}");
                suffix += 1;
            }
            (element.id.as_str(), identifier)
        })
        .collect::<HashMap<_, _>>();
    let groups = projection_groups(model);
    let grouped = groups
        .iter()
        .flat_map(|(_, members)| members)
        .collect::<HashSet<_>>();
    let mut used_view_keys = HashSet::new();
    let diagrams = model
        .folders
        .iter()
        .flat_map(|folder| &folder.diagrams)
        .map(|diagram| {
            let base = dsl_identifier(&diagram.name);
            let mut key = base.clone();
            let mut suffix = 2;
            while !used_view_keys.insert(key.clone()) {
                key = format!("{base}_{suffix}");
                suffix += 1;
            }
            (diagram, key.clone(), format!("archi_view_{key}"))
        })
        .collect::<Vec<_>>();
    let mut output = format!("workspace {}", dsl_string(&model.name));
    if let Some(purpose) = &model.purpose {
        output.push_str(&format!(" {}", dsl_string(purpose)));
    }
    output.push_str(" {\n  model {\n");
    let ungrouped = elements
        .iter()
        .filter(|element| !grouped.contains(&element.id))
        .copied()
        .collect::<Vec<_>>();
    append_projected_archimate_block(&mut output, &ungrouped, &identifiers, 4);
    for (name, members) in &groups {
        output.push_str(&format!("    group {} {{\n", dsl_string(name)));
        let group_elements = elements
            .iter()
            .filter(|element| members.contains(&element.id))
            .copied()
            .collect::<Vec<_>>();
        append_projected_archimate_block(&mut output, &group_elements, &identifiers, 6);
        output.push_str("    }\n");
    }
    for relationship in model
        .folders
        .iter()
        .flat_map(|folder| &folder.relationships)
    {
        let (Some(source), Some(target)) = (
            identifiers.get(relationship.source.as_str()),
            identifiers.get(relationship.target.as_str()),
        ) else {
            continue;
        };
        output.push_str(&format!("    {source} -> {target}"));
        if !relationship.name.is_empty() {
            output.push_str(&format!(" {}", dsl_string(&relationship.name)));
        }
        let tags = diagrams
            .iter()
            .filter(|(diagram, _, _)| diagram_has_relationship(diagram, &relationship.id))
            .map(|(_, _, tag)| tag.clone())
            .collect::<Vec<_>>();
        output.push_str(&format!(" {{\n      type {}\n", relationship.native_type));
        if let Some(access) = relationship_access_direction(relationship) {
            output.push_str(&format!("      access {access}\n"));
        }
        if !tags.is_empty() {
            output.push_str(&format!("      tags {}\n", dsl_string(&tags.join(","))));
        }
        output.push_str("    }\n");
    }
    output.push_str("  }\n");
    if !diagrams.is_empty() {
        output.push_str("  views {\n");
        for (diagram, key, tag) in diagrams {
            let mut visible = Vec::new();
            collect_diagram_element_ids(&diagram.children, &mut visible);
            let included = visible
                .iter()
                .filter_map(|id| identifiers.get(id.as_str()))
                .map(String::as_str)
                .collect::<Vec<_>>();
            if included.is_empty() {
                continue;
            }
            output.push_str(&format!(
                "    archimateView {key} {{\n      include {}\n      exclude relationship.tag!={tag}\n      title {}\n",
                included.join(" "),
                dsl_string(&diagram.name)
            ));
            if let Some(viewpoint) = diagram.attributes.get("viewpoint") {
                output.push_str(&format!("      viewpoint {viewpoint}\n"));
            }
            let mut objects = Vec::new();
            collect_diagram_objects(&diagram.children, &mut objects);
            for object in objects {
                let Some(identifier) = identifiers.get(object.element.as_str()) else {
                    continue;
                };
                let Some(bounds) = object.bounds else {
                    continue;
                };
                output.push_str(&format!(
                    "      object {identifier} {{\n        x {}\n        y {}\n        width {}\n        height {}\n",
                    bounds.x, bounds.y, bounds.width, bounds.height
                ));
                if let Some(color) = object.attributes.get("fillColor") {
                    output.push_str(&format!("        background {color}\n"));
                }
                output.push_str("      }\n");
            }
            output.push_str("    }\n");
        }
        output.push_str("  }\n");
    }
    output.push_str("}\n");
    output
}

fn append_projected_archimate_block(
    output: &mut String,
    elements: &[&ArchiElement],
    identifiers: &HashMap<&str, String>,
    indent: usize,
) {
    if elements.is_empty() {
        return;
    }
    let spaces = " ".repeat(indent);
    output.push_str(&format!("{spaces}archimate {{\n"));
    for element in elements {
        append_projected_element(
            output,
            element,
            &identifiers[element.id.as_str()],
            indent + 2,
        );
    }
    output.push_str(&format!("{spaces}}}\n"));
}

fn collect_diagram_objects<'a>(
    children: &'a [ArchiDiagramChild],
    objects: &mut Vec<&'a ArchiDiagramObject>,
) {
    for child in children {
        match child {
            ArchiDiagramChild::Object(object) => {
                objects.push(object);
                collect_diagram_objects(&object.children, objects);
            }
            ArchiDiagramChild::Group(group) => collect_diagram_objects(&group.children, objects),
        }
    }
}

fn append_projected_element(
    output: &mut String,
    element: &ArchiElement,
    identifier: &str,
    indent: usize,
) {
    let spaces = " ".repeat(indent);
    let name = dsl_string(&projected_element_name(element));
    let description = element
        .description
        .as_ref()
        .map(|description| format!(" {}", dsl_string(description)))
        .unwrap_or_default();
    let keyword = projected_element_keyword(element);
    output.push_str(&format!(
        "{spaces}{identifier} = {keyword} {name}{description}\n"
    ));
}

fn projected_element_keyword(element: &ArchiElement) -> &'static str {
    if element.native_type == "Junction" {
        return match element
            .attributes
            .get("kind")
            .or_else(|| element.attributes.get("type"))
            .map(|value| value.to_ascii_lowercase())
            .as_deref()
        {
            Some("and") => "andJunction",
            Some("or") => "orJunction",
            _ => "junction",
        };
    }
    crate::compiler::archimate_element_keyword(&element.native_type).unwrap_or("grouping")
}

fn projected_element_name(element: &ArchiElement) -> String {
    let trimmed = element.name.trim();
    if trimmed.is_empty() {
        format!("{} {}", element.native_type, dsl_identifier(&element.id))
    } else {
        trimmed.to_string()
    }
}

fn relationship_access_direction(relationship: &ArchiRelationship) -> Option<&'static str> {
    let value = relationship
        .attributes
        .get("accessType")
        .or_else(|| relationship.attributes.get("access"))?;
    match value.to_ascii_lowercase().as_str() {
        "read" => Some("read"),
        "write" => Some("write"),
        "readwrite" | "read_write" | "read-write" => Some("readWrite"),
        "access" => Some("access"),
        _ => None,
    }
}

fn projection_groups(model: &ArchiNativeModel) -> Vec<(String, Vec<String>)> {
    let mut groups = Vec::new();
    let mut assigned = HashSet::new();
    for diagram in model.folders.iter().flat_map(|folder| &folder.diagrams) {
        collect_projection_groups(&diagram.children, &mut assigned, &mut groups);
    }
    groups.sort_by_key(|(y, _, _)| *y);
    groups
        .into_iter()
        .map(|(_, name, members)| (name, members))
        .collect()
}

fn collect_projection_groups(
    children: &[ArchiDiagramChild],
    assigned: &mut HashSet<String>,
    groups: &mut Vec<(i64, String, Vec<String>)>,
) {
    for child in children {
        let ArchiDiagramChild::Group(group) = child else {
            continue;
        };
        let members = group
            .children
            .iter()
            .filter_map(|child| match child {
                ArchiDiagramChild::Object(object) if assigned.insert(object.element.clone()) => {
                    Some(object.element.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if !members.is_empty() {
            groups.push((
                group.bounds.map_or(i64::MAX, |bounds| bounds.y),
                group.name.clone(),
                members,
            ));
        }
        collect_projection_groups(&group.children, assigned, groups);
    }
}

fn collect_diagram_element_ids(children: &[ArchiDiagramChild], identifiers: &mut Vec<String>) {
    // ponytail: views are small; the linear check preserves native object order.
    for child in children {
        match child {
            ArchiDiagramChild::Object(object) => {
                if !identifiers.contains(&object.element) {
                    identifiers.push(object.element.clone());
                }
                collect_diagram_element_ids(&object.children, identifiers);
            }
            ArchiDiagramChild::Group(group) => {
                collect_diagram_element_ids(&group.children, identifiers);
            }
        }
    }
}

fn diagram_has_relationship(diagram: &ArchiDiagram, relationship: &str) -> bool {
    diagram_children_have_relationship(&diagram.children, relationship)
}

fn diagram_children_have_relationship(children: &[ArchiDiagramChild], relationship: &str) -> bool {
    children.iter().any(|child| match child {
        ArchiDiagramChild::Object(object) => {
            object
                .connections
                .iter()
                .any(|connection| connection.relationship == relationship)
                || diagram_children_have_relationship(&object.children, relationship)
        }
        ArchiDiagramChild::Group(group) => {
            diagram_children_have_relationship(&group.children, relationship)
        }
    })
}

pub fn validate_connections(model: &ArchiNativeModel) -> Vec<String> {
    let relationships = model
        .folders
        .iter()
        .flat_map(|folder| &folder.relationships)
        .map(|relationship| relationship.id.as_str())
        .collect::<HashSet<_>>();
    let mut diagnostics = Vec::new();
    for diagram in model.folders.iter().flat_map(|folder| &folder.diagrams) {
        let mut objects = HashMap::new();
        collect_objects(&diagram.children, &mut objects);
        for child in &diagram.children {
            validate_child_connections(child, &objects, &relationships, diagram, &mut diagnostics);
        }
    }
    diagnostics
}

fn parse_xml(input: &str) -> Result<XmlElement, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::new();
    let mut root = None;
    loop {
        match reader
            .read_event()
            .map_err(|error| format!("invalid XML at byte {}: {error}", reader.error_position()))?
        {
            Event::Start(event) => stack.push(xml_element(&reader, &event)?),
            Event::Empty(event) => {
                append_xml(&mut stack, &mut root, xml_element(&reader, &event)?)?
            }
            Event::End(_) => {
                let element = stack.pop().ok_or("unexpected XML closing element")?;
                append_xml(&mut stack, &mut root, element)?;
            }
            Event::Text(event) => {
                let decoded = event
                    .decode()
                    .map_err(|error| format!("invalid XML text: {error}"))?;
                let text = quick_xml::escape::unescape(&decoded)
                    .map_err(|error| format!("invalid XML entity: {error}"))?;
                if let Some(parent) = stack.last_mut() {
                    append_text(&mut parent.text, &text);
                }
            }
            Event::CData(event) => {
                let text = event
                    .decode()
                    .map_err(|error| format!("invalid XML CDATA: {error}"))?;
                if let Some(parent) = stack.last_mut() {
                    append_text(&mut parent.text, &text);
                }
            }
            Event::DocType(_) => return Err("XML DOCTYPE declarations are not allowed".into()),
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::Comment(_) | Event::GeneralRef(_) => {}
        }
    }
    if !stack.is_empty() {
        return Err("unclosed XML element".into());
    }
    root.ok_or_else(|| "XML document has no root element".into())
}

fn xml_element(
    reader: &Reader<&[u8]>,
    event: &quick_xml::events::BytesStart<'_>,
) -> Result<XmlElement, String> {
    let mut attributes = BTreeMap::new();
    for attribute in event.attributes() {
        let attribute = attribute.map_err(|error| format!("invalid XML attribute: {error}"))?;
        let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| format!("invalid XML attribute '{name}': {error}"))?
            .into_owned();
        attributes.insert(name, value);
    }
    Ok(XmlElement {
        name: String::from_utf8_lossy(event.name().as_ref()).into_owned(),
        attributes,
        text: String::new(),
        children: Vec::new(),
    })
}

fn append_xml(
    stack: &mut [XmlElement],
    root: &mut Option<XmlElement>,
    element: XmlElement,
) -> Result<(), String> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(element);
    } else if root.replace(element).is_some() {
        return Err("XML document has multiple root elements".into());
    }
    Ok(())
}

fn append_text(target: &mut String, text: &str) {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(&text);
}

fn model_from_xml(root: &XmlElement) -> Result<ArchiNativeModel, String> {
    if root.name != "archimate:model" {
        return Err(format!(
            "expected archimate:model root, found {}",
            root.name
        ));
    }
    let mut folders = Vec::new();
    for folder in &root.children {
        if folder.name != "folder" {
            continue;
        }
        let mut parsed = ArchiFolder {
            name: attr(folder, "name").to_string(),
            id: required_attr(folder, "id")?.to_string(),
            folder_type: required_attr(folder, "type")?.to_string(),
            attributes: folder.attributes.clone(),
            elements: Vec::new(),
            relationships: Vec::new(),
            diagrams: Vec::new(),
        };
        for element in &folder.children {
            if element.name != "element" {
                continue;
            }
            let native_type = native_type(element);
            if native_type == "ArchimateDiagramModel" {
                parsed.diagrams.push(parse_diagram(element)?);
            } else if native_type.ends_with("Relationship") {
                parsed.relationships.push(ArchiRelationship {
                    id: required_attr(element, "id")?.to_string(),
                    name: attr(element, "name").to_string(),
                    native_type,
                    source: required_attr(element, "source")?.to_string(),
                    target: required_attr(element, "target")?.to_string(),
                    attributes: element.attributes.clone(),
                });
            } else {
                parsed.elements.push(ArchiElement {
                    id: required_attr(element, "id")?.to_string(),
                    name: attr(element, "name").to_string(),
                    description: child_text(element, "documentation"),
                    native_type,
                    attributes: element.attributes.clone(),
                });
            }
        }
        folders.push(parsed);
    }
    Ok(ArchiNativeModel {
        name: attr(root, "name").to_string(),
        id: required_attr(root, "id")?.to_string(),
        version: required_attr(root, "version")?.to_string(),
        purpose: child_text(root, "purpose"),
        attributes: root.attributes.clone(),
        folders,
    })
}

fn child_text(element: &XmlElement, name: &str) -> Option<String> {
    element
        .children
        .iter()
        .find(|child| child.name == name && !child.text.is_empty())
        .map(|child| child.text.clone())
}

fn parse_diagram(element: &XmlElement) -> Result<ArchiDiagram, String> {
    Ok(ArchiDiagram {
        id: required_attr(element, "id")?.to_string(),
        name: attr(element, "name").to_string(),
        attributes: element.attributes.clone(),
        children: element
            .children
            .iter()
            .filter(|child| is_supported_diagram_child(child))
            .map(parse_diagram_child)
            .collect::<Result<_, _>>()?,
    })
}

fn parse_diagram_child(element: &XmlElement) -> Result<ArchiDiagramChild, String> {
    let children = element
        .children
        .iter()
        .filter(|child| is_supported_diagram_child(child))
        .map(parse_diagram_child)
        .collect::<Result<Vec<_>, _>>()?;
    if native_type(element) == "Group" {
        return Ok(ArchiDiagramChild::Group(ArchiGroup {
            id: required_attr(element, "id")?.to_string(),
            name: attr(element, "name").to_string(),
            bounds: parse_bounds(element)?,
            attributes: element.attributes.clone(),
            children,
        }));
    }
    Ok(ArchiDiagramChild::Object(ArchiDiagramObject {
        id: required_attr(element, "id")?.to_string(),
        element: required_attr(element, "archimateElement")?.to_string(),
        bounds: parse_bounds(element)?,
        target_connections: attr(element, "targetConnections")
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        attributes: element.attributes.clone(),
        connections: element
            .children
            .iter()
            .filter(|child| child.name == "sourceConnection")
            .map(parse_connection)
            .collect::<Result<_, _>>()?,
        children,
    }))
}

fn is_supported_diagram_child(element: &XmlElement) -> bool {
    element.name == "child" && matches!(native_type(element).as_str(), "DiagramObject" | "Group")
}

fn parse_bounds(element: &XmlElement) -> Result<Option<ArchiBounds>, String> {
    element
        .children
        .iter()
        .find(|child| child.name == "bounds")
        .map(|bounds| {
            Ok(ArchiBounds {
                x: number(bounds, "x")?,
                y: number(bounds, "y")?,
                width: number(bounds, "width")?,
                height: number(bounds, "height")?,
            })
        })
        .transpose()
}

fn parse_connection(element: &XmlElement) -> Result<ArchiConnection, String> {
    Ok(ArchiConnection {
        id: required_attr(element, "id")?.to_string(),
        source: required_attr(element, "source")?.to_string(),
        target: required_attr(element, "target")?.to_string(),
        relationship: required_attr(element, "archimateRelationship")?.to_string(),
        attributes: element.attributes.clone(),
        bendpoints: element
            .children
            .iter()
            .filter(|child| child.name == "bendpoint")
            .map(|child| child.attributes.clone())
            .collect(),
    })
}

fn compare_xml(a: &XmlElement, b: &XmlElement, path: &str) -> Option<String> {
    if a.name != b.name {
        return Some(format!("{path}: element differs: {} vs {}", a.name, b.name));
    }
    let a_attributes = canonical_attributes(&a.attributes);
    let b_attributes = canonical_attributes(&b.attributes);
    if a_attributes != b_attributes {
        for key in a_attributes.keys().chain(b_attributes.keys()) {
            if a_attributes.get(key) != b_attributes.get(key) {
                return Some(format!(
                    "{path}.@{key} differs: {:?} vs {:?}",
                    a_attributes.get(key),
                    b_attributes.get(key)
                ));
            }
        }
    }
    if a.text != b.text {
        return Some(format!("{path}.text differs: {:?} vs {:?}", a.text, b.text));
    }
    if a.children.len() != b.children.len() {
        return Some(format!(
            "{path}: child count differs: {} vs {}",
            a.children.len(),
            b.children.len()
        ));
    }
    for (index, (a, b)) in a.children.iter().zip(&b.children).enumerate() {
        let id = a.attributes.get("id").map_or_else(
            || format!("{}[{index}]", a.name),
            |id| format!("{}[@id='{id}']", a.name),
        );
        if let Some(diff) = compare_xml(a, b, &format!("{path}/{id}")) {
            return Some(diff);
        }
    }
    None
}

fn canonical_attributes(attributes: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    attributes
        .iter()
        .map(|(name, value)| {
            let value = if name == "targetConnections" {
                let mut values = value.split_whitespace().collect::<Vec<_>>();
                values.sort_unstable();
                values.join(" ")
            } else {
                value.clone()
            };
            (name.clone(), value)
        })
        .collect()
}

fn collect_objects<'a>(
    children: &'a [ArchiDiagramChild],
    objects: &mut HashMap<&'a str, &'a ArchiDiagramObject>,
) {
    for child in children {
        match child {
            ArchiDiagramChild::Object(object) => {
                objects.insert(&object.id, object);
                collect_objects(&object.children, objects);
            }
            ArchiDiagramChild::Group(group) => collect_objects(&group.children, objects),
        }
    }
}

fn validate_child_connections(
    child: &ArchiDiagramChild,
    objects: &HashMap<&str, &ArchiDiagramObject>,
    relationships: &HashSet<&str>,
    diagram: &ArchiDiagram,
    diagnostics: &mut Vec<String>,
) {
    match child {
        ArchiDiagramChild::Object(object) => {
            for connection in &object.connections {
                let prefix = format!("diagram '{}' connection '{}'", diagram.name, connection.id);
                if !objects.contains_key(connection.source.as_str()) {
                    diagnostics.push(format!("{prefix} has missing source {}", connection.source));
                }
                let Some(target) = objects.get(connection.target.as_str()) else {
                    diagnostics.push(format!("{prefix} has missing target {}", connection.target));
                    continue;
                };
                if !relationships.contains(connection.relationship.as_str()) {
                    diagnostics.push(format!(
                        "{prefix} has missing relationship {}",
                        connection.relationship
                    ));
                }
                if !target.target_connections.contains(&connection.id) {
                    diagnostics.push(format!(
                        "{prefix} is absent from targetConnections on {}",
                        target.id
                    ));
                }
            }
            for child in &object.children {
                validate_child_connections(child, objects, relationships, diagram, diagnostics);
            }
        }
        ArchiDiagramChild::Group(group) => {
            for child in &group.children {
                validate_child_connections(child, objects, relationships, diagram, diagnostics);
            }
        }
    }
}

fn native_type(element: &XmlElement) -> String {
    attr(element, "xsi:type")
        .strip_prefix("archimate:")
        .unwrap_or_else(|| attr(element, "xsi:type"))
        .to_string()
}

fn dsl_identifier(name: &str) -> String {
    let mut identifier = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !identifier.is_empty() {
                identifier.push('_');
            }
            identifier.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if identifier.is_empty() {
        return "element".into();
    }
    if identifier.starts_with(|character: char| character.is_ascii_digit()) {
        identifier.insert_str(0, "element_");
    }
    identifier
}

fn dsl_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn hash(value: &str) -> String {
    let value = value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{value:016x}")
}

fn attr<'a>(element: &'a XmlElement, name: &str) -> &'a str {
    element.attributes.get(name).map_or("", String::as_str)
}

fn required_attr<'a>(element: &'a XmlElement, name: &str) -> Result<&'a str, String> {
    element
        .attributes
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("{} is missing required attribute '{name}'", element.name))
}

fn number(element: &XmlElement, name: &str) -> Result<i64, String> {
    required_attr(element, name)?
        .parse()
        .map_err(|error| format!("invalid {}.{name}: {error}", element.name))
}

fn read_utf8(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compiler, exporters};
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    const MINI: &str = include_str!("../tests/fixtures/m8-archi-native-mini.archimate");

    #[test]
    fn parses_native_model_and_preserves_diagram_metadata() {
        let model = parse(MINI).unwrap();
        assert_eq!(model.name, "Mini & Model");
        assert_eq!(model.id, "model-1");
        assert_eq!(model.version, "5.0.0");
        assert_eq!(model.folders.len(), 9);
        assert!(model
            .folders
            .iter()
            .flat_map(|folder| &folder.elements)
            .any(|element| element.native_type == "BusinessActor"));
        assert!(model
            .folders
            .iter()
            .flat_map(|folder| &folder.elements)
            .any(|element| element.native_type == "ApplicationComponent"));
        assert!(model
            .folders
            .iter()
            .flat_map(|folder| &folder.elements)
            .any(|element| element.native_type == "Node"));
        let relationships = model
            .folders
            .iter()
            .flat_map(|folder| &folder.relationships)
            .collect::<Vec<_>>();
        assert_eq!(relationships.len(), 2);
        assert_eq!(relationships[0].native_type, "FlowRelationship");
        let diagram = &model
            .folders
            .iter()
            .flat_map(|folder| &folder.diagrams)
            .next()
            .unwrap();
        let ArchiDiagramChild::Group(group) = &diagram.children[0] else {
            panic!("expected group");
        };
        assert_eq!(group.bounds.unwrap().width, 220);
        let ArchiDiagramChild::Object(actor) = &group.children[0] else {
            panic!("expected actor object");
        };
        assert_eq!(actor.element, "actor-1");
        assert_eq!(actor.connections[0].target, "object-app");
        assert_eq!(actor.connections[0].bendpoints.len(), 1);
        let ArchiDiagramChild::Object(app) = &diagram.children[1] else {
            panic!("expected app object");
        };
        assert_eq!(app.target_connections, ["connection-1"]);
        assert!(validate_connections(&model).is_empty());
    }

    #[test]
    fn projects_valid_c4_dsl_and_round_trips_with_sidecar() {
        let model = parse(MINI).unwrap();
        let dsl = project_to_dsl(&model);
        let workspace = compiler::compile(&dsl).unwrap();
        compiler::validate(&workspace).unwrap();
        assert_eq!(workspace.views.len(), 1);
        assert_eq!(workspace.groups.len(), 1);
        assert!(dsl.contains("businessActor \"User <Admin>\""));
        assert!(dsl.contains("applicationComponent \"Orders\""));
        assert!(dsl.contains("node \"Primary Node\""));
        assert!(dsl.contains("archimateView context"));
        assert!(dsl.contains("user_admin -> orders \"Uses\""));
        assert!(dsl.contains("boundary -> orders {"));
        assert!(dsl.contains("type CompositionRelationship"));
        assert!(dsl.contains("exclude relationship.tag!=archi_view_context"));
        assert!(dsl.contains("object orders {"));
        assert!(!dsl.contains("archi_id_"));
        assert!(!dsl.contains("actor-1"));

        let directory = temporary_directory("archi-roundtrip");
        let input = directory.join("input.archimate");
        let projection = directory.join("workspace.dsl");
        let sidecar = directory.join("workspace.archi-sidecar.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&input, MINI).unwrap();
        import(&input, &projection, &sidecar).unwrap();
        let output = sidecar_xml(&sidecar, &projection).unwrap().unwrap();
        assert!(canonical_diff(&parse_xml(MINI).unwrap(), &parse_xml(&output).unwrap()).is_none());
        let export_directory = directory.join("out");
        exporters::export_with_archi_sidecar(
            &workspace,
            "archi",
            &export_directory,
            exporters::ExportOptions { strict_safe: true },
            &sidecar,
            &projection,
        )
        .unwrap();
        diff_files(&input, &export_directory.join("workspace.archimate")).unwrap();
        let generated_directory = directory.join("generated");
        exporters::export(
            &workspace,
            "archi",
            &generated_directory,
            exporters::ExportOptions { strict_safe: true },
        )
        .unwrap();
        let generated =
            fs::read_to_string(generated_directory.join("workspace.archimate")).unwrap();
        semantic_diff_files(&input, &generated_directory.join("workspace.archimate")).unwrap();
        assert!(generated.contains("archimate:FlowRelationship"));
        assert!(generated.contains("archimate:CompositionRelationship"));
        assert_eq!(generated.matches("archimate:Group\"").count(), 1);
        assert_eq!(generated.matches("archimate:Connection\"").count(), 1);
        assert!(!generated.contains("Imported Environment"));
        fs::write(
            &projection,
            format!("{}\n", fs::read_to_string(&projection).unwrap()),
        )
        .unwrap();
        assert!(sidecar_xml(&sidecar, &projection).unwrap().is_none());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn canonical_diff_ignores_formatting_attribute_order_and_target_order() {
        let a = parse_xml(
            "<root a=\"1\"><child targetConnections=\"b a\" id=\"x\"> value </child></root>",
        )
        .unwrap();
        let b = parse_xml(
            "<root a=\"1\">\n  <child id=\"x\" targetConnections=\"a b\">value</child>\n</root>",
        )
        .unwrap();
        assert!(canonical_diff(&a, &b).is_none());
        let changed = parse_xml("<root a=\"2\"><child id=\"x\">value</child></root>").unwrap();
        assert!(canonical_diff(&a, &changed).unwrap().contains("@a differs"));
    }

    #[test]
    fn reports_dangling_native_connections() {
        let invalid = MINI.replace("target=\"object-app\"", "target=\"missing\"");
        let diagnostics = validate_connections(&parse(&invalid).unwrap());
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("missing target")));
    }

    fn temporary_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("c4c-{label}-{nonce}"))
    }
}
