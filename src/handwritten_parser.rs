use crate::{
    compiler::{
        AnimationStep, AutoLayout, Branding, BrandingFont, Directive, DynamicRelationship, Element,
        ElementKind, ElementStyle, FilterMode, Group, HealthCheck, ImageSource, NamedBlock,
        PreservedBlock, Property, Reference, Relationship, RelationshipStyle, RemovedRelationship,
        StyleMode, ThemeReference, View, ViewFilter, ViewKind, ViewSelector, Workspace,
        WorkspaceExtension,
    },
    diagnostic::{render_all, Diagnostic},
    lexer::{Token, TokenKind},
    source::{SourceId, SourceMap, Span},
};

pub fn parse(
    sources: &SourceMap,
    source_id: SourceId,
    tokens: Vec<Token>,
    identifiers: &str,
) -> Result<Workspace, String> {
    let mut parser = Parser::new(sources.clone(), source_id, tokens, identifiers);
    parser.parse_workspace();
    if parser.diagnostics.is_empty() {
        Ok(parser.workspace)
    } else {
        Err(render_all(&parser.diagnostics, sources))
    }
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    order: usize,
    diagnostics: Vec<Diagnostic>,
    workspace: Workspace,
    parent_stack: Vec<String>,
    group_stack: Vec<usize>,
}

impl Parser {
    fn new(sources: SourceMap, source_id: SourceId, tokens: Vec<Token>, identifiers: &str) -> Self {
        let mut workspace = Workspace::new(
            sources.clone(),
            Span::new(source_id, 0, sources.get(source_id).text.len()),
        );
        workspace.identifiers = identifiers.into();
        Self {
            workspace,
            tokens,
            index: 0,
            order: 0,
            diagnostics: Vec::new(),
            parent_stack: Vec::new(),
            group_stack: Vec::new(),
        }
    }

    fn parse_workspace(&mut self) {
        self.skip_newlines();
        let Some(start) = self.eat_keyword("workspace") else {
            self.error(
                self.current().span,
                "expected keyword 'workspace'",
                Some("start the file with 'workspace {'"),
            );
            return;
        };
        if self.eat_keyword("extends").is_some() {
            if let Some((target, span)) = self.take_value() {
                self.workspace.extension = Some(WorkspaceExtension { target, span });
            } else {
                self.error(
                    self.current().span,
                    "workspace extension target is missing",
                    Some("add a local .dsl path after 'extends'"),
                );
            }
        } else {
            let arguments = self.take_values();
            self.limit_arguments(
                &arguments,
                2,
                "workspace accepts at most a name and description",
            );
            self.workspace.attributes = argument_properties(&arguments, &["name", "description"]);
            self.workspace.name = optional_argument(arguments.first());
            self.workspace.description = optional_argument(arguments.get(1));
        }
        if !self.open_block("workspace") {
            return;
        }

        let end = loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                break self.close_block("workspace");
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("workspace");
                break None;
            }
            self.parse_workspace_statement();
        };
        if let Some(end) = end {
            self.workspace.span = start.span.merge(end.span);
        }
        self.skip_newlines();
        if !self.at(TokenTag::Eof) {
            self.error(
                self.current().span,
                "unexpected content after workspace",
                Some("remove content after the workspace closing brace"),
            );
        }
    }

    fn parse_workspace_statement(&mut self) {
        if self.at_bang("identifiers") {
            self.parse_identifiers();
        } else if self.at_bang("docs") || self.at_bang("adrs") {
            let directive = self.parse_directive();
            self.workspace.directives.push(directive);
        } else if self.at_bang("script") || self.at_bang("plugin") {
            self.parse_unsafe_directive();
        } else if self.at_keyword("model") {
            self.parse_model();
        } else if self.at_keyword("views") {
            self.parse_views();
        } else if self.at_keyword("name") {
            if let Some(property) = self.parse_single_property("name") {
                self.workspace.name = Some(property.value.clone());
                self.workspace.attributes.push(property);
            }
        } else if self.at_keyword("description") {
            if let Some(property) = self.parse_single_property("description") {
                self.workspace.description = Some(property.value.clone());
                self.workspace.attributes.push(property);
            }
        } else if self.at_keyword("properties") {
            let properties = self.parse_property_block("properties");
            self.workspace.properties.extend(properties);
        } else if self.at_keyword("configuration") {
            let preserved = self.parse_preserved("configuration");
            self.warn_preserved(&preserved, "configuration semantics are deferred");
            self.workspace.preserved.push(preserved);
        } else {
            self.unknown_statement("workspace");
        }
    }

    fn parse_identifiers(&mut self) {
        let keyword = self.advance();
        self.workspace.identifiers_explicit = true;
        let value = self.take_value();
        match value {
            Some((value, _)) if value.eq_ignore_ascii_case("flat") => {
                self.workspace.identifiers = "flat".into();
            }
            Some((value, _)) if value.eq_ignore_ascii_case("hierarchical") => {
                self.workspace.identifiers = "hierarchical".into();
            }
            Some((value, span)) => self.error(
                span,
                format!("unsupported identifier mode '{value}'"),
                Some("use '!identifiers flat' or '!identifiers hierarchical'"),
            ),
            None => self.error(
                keyword.span,
                "missing identifier mode",
                Some("use '!identifiers flat' or '!identifiers hierarchical'"),
            ),
        }
        self.finish_statement("identifier mode");
    }

    fn parse_model(&mut self) {
        self.advance();
        if !self.open_block("model") {
            return;
        }
        self.parse_model_block("model");
    }

    fn parse_model_block(&mut self, label: &str) -> Option<Token> {
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                return self.close_block(label);
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace(label);
                return None;
            }
            self.parse_model_statement();
        }
    }

    fn parse_model_statement(&mut self) {
        if self.relationship_ahead(TokenTag::Arrow) {
            self.parse_relationship();
            return;
        }
        if self.relationship_ahead(TokenTag::RemoveArrow) {
            self.parse_removed_relationship();
            return;
        }
        if self.at_bang("identifiers") {
            self.parse_identifiers();
            return;
        }
        if self.at_bang("impliedRelationships") {
            self.parse_implied_relationships();
            return;
        }
        if self.at_bang("docs") || self.at_bang("adrs") {
            let directive = self.parse_directive();
            self.error(
                directive.span,
                format!("!{} is not allowed directly in the model", directive.name),
                Some("attach documentation to the workspace, a software system, or a container"),
            );
            self.workspace.directives.push(directive);
            return;
        }
        if self.at_bang("script") || self.at_bang("plugin") {
            self.parse_unsafe_directive();
            return;
        }
        if self.at_any_bang(&[
            "extend",
            "ref",
            "element",
            "elements",
            "relationship",
            "relationships",
            "components",
        ]) {
            let directive = self.parse_directive();
            self.warn_deferred(
                &directive,
                "directive semantics are deferred; syntax was preserved",
            );
            self.workspace.directives.push(directive);
            return;
        }
        if self.at_keyword("archetypes") {
            let preserved = self.parse_preserved("archetypes");
            self.warn_preserved(
                &preserved,
                "archetype expansion is deferred; syntax was preserved",
            );
            self.workspace.preserved.push(preserved);
            return;
        }
        if self.at_keyword("enterprise") {
            self.parse_enterprise();
            return;
        }
        if self.at_keyword("group") {
            self.parse_group();
            return;
        }
        if self.at_keyword("archimate") {
            self.parse_archimate_block();
            return;
        }

        let statement_start = self.current().span;
        let assigned = if self.identifier_ahead() && self.nth_at(1, TokenTag::Equals) {
            let identifier = self.take_identifier().unwrap();
            self.advance();
            Some(identifier)
        } else {
            None
        };
        if let Some(kind) = self.current_element_kind() {
            self.parse_element(statement_start, assigned, kind);
        } else if assigned.is_some() {
            self.error(
                self.current().span,
                "expected an M3 element type after '='",
                Some("check the core model element keyword"),
            );
            self.skip_statement();
        } else {
            self.unknown_statement("model");
        }
    }

    fn parse_archimate_block(&mut self) {
        self.advance();
        if !self.open_block("archimate") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("archimate");
                return;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("archimate");
                return;
            }
            if self.relationship_ahead(TokenTag::Arrow) {
                self.parse_relationship();
                continue;
            }
            let statement_start = self.current().span;
            let assigned = if self.identifier_ahead() && self.nth_at(1, TokenTag::Equals) {
                let identifier = self.take_identifier().unwrap();
                self.advance();
                Some(identifier)
            } else {
                None
            };
            let junction_kind = self.current_archimate_junction_kind();
            if let Some(kind) = self.current_archimate_element_kind() {
                let before = self.workspace.elements.len();
                self.parse_element(statement_start, assigned, kind);
                self.add_implicit_junction_kind(before, statement_start, junction_kind);
            } else if assigned.is_some() {
                self.error(
                    self.current().span,
                    "expected an ArchiMate element type after '='",
                    Some("use businessActor, applicationComponent, node, or another supported ArchiMate keyword"),
                );
                self.skip_statement();
            } else {
                self.unknown_statement("archimate");
            }
        }
    }

    fn parse_implied_relationships(&mut self) {
        let keyword = self.advance();
        let value = self
            .take_value()
            .map(|value| value.0)
            .unwrap_or_else(|| "true".into());
        if !value.eq_ignore_ascii_case("true") && !value.eq_ignore_ascii_case("false") {
            self.error(
                keyword.span,
                format!("custom implied relationship strategy '{value}' is not supported"),
                Some("use true or false; external classes are never loaded"),
            );
        }
        self.workspace.implied_relationships = Some(value);
        self.finish_statement("implied relationships");
    }

    fn parse_enterprise(&mut self) {
        let start = self.advance();
        let Some((name, _)) = self.take_value() else {
            self.error(self.current().span, "enterprise name is missing", None);
            self.skip_statement();
            return;
        };
        if !self.open_block("enterprise") {
            return;
        }
        let end = self.parse_model_block("enterprise");
        self.workspace.enterprise = Some(NamedBlock {
            name,
            span: end.map_or(start.span, |end| start.span.merge(end.span)),
        });
    }

    fn parse_group(&mut self) {
        let start = self.advance();
        let Some((name, _)) = self.take_value() else {
            self.error(self.current().span, "group name is missing", None);
            self.skip_statement();
            return;
        };
        if !self.open_block("group") {
            return;
        }
        let index = self.workspace.groups.len();
        let owner = self.parent_stack.last().cloned();
        self.workspace.groups.push(Group {
            name,
            parent: self
                .group_stack
                .last()
                .copied()
                .filter(|group| self.workspace.groups[*group].owner == owner),
            owner,
            span: start.span,
        });
        self.group_stack.push(index);
        if let Some(end) = self.parse_model_block("group") {
            self.workspace.groups[index].span = start.span.merge(end.span);
        }
        self.group_stack.pop();
    }

    fn current_element_kind(&self) -> Option<ElementKind> {
        [
            ("person", ElementKind::Person),
            ("softwareSystem", ElementKind::SoftwareSystem),
            ("container", ElementKind::Container),
            ("component", ElementKind::Component),
            ("element", ElementKind::Generic),
            ("deploymentEnvironment", ElementKind::DeploymentEnvironment),
            ("deploymentGroup", ElementKind::DeploymentGroup),
            ("deploymentNode", ElementKind::DeploymentNode),
            ("infrastructureNode", ElementKind::InfrastructureNode),
            (
                "softwareSystemInstance",
                ElementKind::SoftwareSystemInstance,
            ),
            ("containerInstance", ElementKind::ContainerInstance),
        ]
        .into_iter()
        .find_map(|(keyword, kind)| self.at_keyword(keyword).then_some(kind))
    }

    fn current_archimate_element_kind(&self) -> Option<ElementKind> {
        if let TokenKind::Identifier(value) = &self.current().kind {
            crate::compiler::archimate_element_kind(value)
        } else {
            None
        }
    }

    fn current_archimate_junction_kind(&self) -> Option<&'static str> {
        if self.at_keyword("andJunction") {
            Some("and")
        } else if self.at_keyword("orJunction") {
            Some("or")
        } else {
            None
        }
    }

    fn add_implicit_junction_kind(
        &mut self,
        before: usize,
        span: Span,
        junction_kind: Option<&'static str>,
    ) {
        if let Some(value) = junction_kind.filter(|_| self.workspace.elements.len() > before) {
            self.workspace.elements[before].attributes.push(Property {
                key: "kind".into(),
                value: value.into(),
                span,
            });
        }
    }

    fn parse_element(
        &mut self,
        statement_start: Span,
        assigned: Option<(String, Span)>,
        kind: ElementKind,
    ) {
        let keyword = self.advance();
        let arguments = self.take_values();
        let Some((first, first_span)) = arguments.first().cloned() else {
            self.error(
                self.current().span,
                format!("{} value is missing", element_label(&kind)),
                Some("add the required name or referenced identifier"),
            );
            self.finish_statement("element");
            return;
        };
        if first.is_empty() {
            self.error(first_span, "required element value cannot be empty", None);
        }
        let maximum = maximum_arguments(&kind);
        self.limit_arguments(
            &arguments,
            maximum,
            &format!("too many properties for {}", element_label(&kind)),
        );

        let instance = matches!(
            kind,
            ElementKind::SoftwareSystemInstance | ElementKind::ContainerInstance
        );
        let reference = instance.then(|| Reference {
            identifier: first.clone(),
            span: first_span,
        });
        let inline = inline_properties(&kind, &arguments);
        let (base_id, id_span) = assigned.unwrap_or_else(|| (slug(&first), first_span));
        let id = self.qualify_identifier(base_id);
        let parent = self.parent_stack.last().cloned();
        let mut span = statement_start.merge(
            arguments
                .last()
                .map(|argument| argument.1)
                .unwrap_or(keyword.span),
        );
        let index = self.workspace.elements.len();
        let order = self.next_order();
        let attributes = element_argument_properties(&kind, &arguments);
        self.workspace.elements.push(Element {
            id: id.clone(),
            kind,
            name: first,
            description: inline.description,
            technology: inline.technology,
            parent,
            group: self.group_stack.last().copied().filter(|group| {
                self.workspace.groups[*group].owner.as_ref() == self.parent_stack.last()
            }),
            tags: inline.tags,
            url: None,
            attributes,
            properties: Vec::new(),
            perspectives: Vec::new(),
            instances: inline.instances,
            instance_of: None,
            reference,
            deployment_groups: inline.deployment_groups,
            health_checks: Vec::new(),
            directives: Vec::new(),
            element_type: inline.element_type,
            order,
            span,
            id_span,
        });

        if self.at(TokenTag::LeftBrace) {
            if !self.open_block("element") {
                return;
            }
            self.parent_stack.push(id);
            loop {
                self.skip_newlines();
                if self.at(TokenTag::RightBrace) {
                    if let Some(end) = self.close_block("element") {
                        span = span.merge(end.span);
                        self.workspace.elements[index].span = span;
                    }
                    break;
                }
                if self.at(TokenTag::Eof) {
                    self.missing_closing_brace("element");
                    break;
                }
                if !self.parse_element_property(index) {
                    self.parse_model_statement();
                }
            }
            self.parent_stack.pop();
        } else {
            self.finish_statement("element");
        }
    }

    fn parse_element_property(&mut self, index: usize) -> bool {
        if self.at_keyword("description") {
            if let Some(property) = self.parse_single_property("description") {
                self.workspace.elements[index].description = Some(property.value.clone());
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("technology") {
            if let Some(property) = self.parse_single_property("technology") {
                self.workspace.elements[index].technology = Some(property.value.clone());
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("tag") {
            if let Some(property) = self.parse_single_property("tag") {
                self.workspace.elements[index]
                    .tags
                    .push(property.value.clone());
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("tags") {
            if let Some(property) = self.parse_single_property("tags") {
                self.workspace.elements[index]
                    .tags
                    .extend(split_tags(&property.value));
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("url") {
            if let Some(property) = self.parse_single_property("url") {
                self.workspace.elements[index].url = Some(property.value.clone());
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("instances") {
            if let Some(property) = self.parse_single_property("instances") {
                self.workspace.elements[index].instances = Some(property.value.clone());
                self.workspace.elements[index].attributes.push(property);
            }
        } else if self.at_keyword("instanceOf") {
            let start = self.advance();
            self.workspace.elements[index].instance_of = self
                .take_identifier()
                .map(|(identifier, span)| Reference { identifier, span });
            if self.workspace.elements[index].instance_of.is_none() {
                self.error(start.span, "instanceOf reference is missing", None);
            }
            self.finish_statement("instanceOf");
        } else if self.at_keyword("properties") {
            let properties = self.parse_property_block("properties");
            self.workspace.elements[index].properties.extend(properties);
        } else if self.at_keyword("perspectives") {
            let perspectives = self.parse_property_block("perspectives");
            self.workspace.elements[index]
                .perspectives
                .extend(perspectives);
        } else if self.at_keyword("healthCheck") {
            self.parse_health_check(index);
        } else if self.at_bang("docs") || self.at_bang("adrs") {
            let directive = self.parse_directive();
            self.workspace.elements[index].directives.push(directive);
        } else if self.at_bang("components") {
            let directive = self.parse_directive();
            self.warn_deferred(
                &directive,
                "component discovery is deferred; syntax was preserved",
            );
            self.workspace.elements[index].directives.push(directive);
        } else if let Some(name) = self.archimate_element_property() {
            if let Some(property) = self.parse_style_property(name) {
                self.workspace.elements[index].attributes.push(property);
            }
        } else {
            return false;
        }
        true
    }

    fn archimate_element_property(&self) -> Option<&'static str> {
        [
            "background",
            "color",
            "colour",
            "stroke",
            "fontSize",
            "width",
            "height",
            "kind",
        ]
        .into_iter()
        .find(|name| self.at_keyword(name))
    }

    fn parse_health_check(&mut self, index: usize) {
        let start = self.advance();
        let arguments = self.take_values();
        if arguments.len() < 2 {
            self.error(
                start.span,
                "healthCheck requires a name and URL",
                Some("use healthCheck <name> <url> [interval] [timeout]"),
            );
        } else {
            self.limit_arguments(&arguments, 4, "healthCheck accepts at most four values");
            let end = arguments.last().unwrap().1;
            self.workspace.elements[index]
                .health_checks
                .push(HealthCheck {
                    name: arguments[0].0.clone(),
                    url: arguments[1].0.clone(),
                    interval: optional_argument(arguments.get(2)),
                    timeout: optional_argument(arguments.get(3)),
                    span: start.span.merge(end),
                });
        }
        self.finish_statement("healthCheck");
    }

    fn parse_relationship(&mut self) {
        let (id, id_span) = if self.identifier_ahead() && self.nth_at(1, TokenTag::Equals) {
            let (identifier, span) = self.take_identifier().unwrap();
            self.advance();
            (Some(identifier), Some(span))
        } else {
            (None, None)
        };
        let (source, source_span) = self.take_identifier().unwrap();
        self.advance();
        let Some((destination, destination_span)) = self.take_identifier() else {
            self.error(
                self.current().span,
                "relationship destination is missing",
                Some("add an element identifier after '->'"),
            );
            self.finish_statement("relationship");
            return;
        };
        let arguments = self.take_values();
        self.limit_arguments(
            &arguments,
            3,
            "relationship accepts at most description, technology, and tags",
        );
        let end = arguments
            .last()
            .map(|argument| argument.1)
            .unwrap_or(destination_span);
        let index = self.workspace.relationships.len();
        let order = self.next_order();
        self.workspace.relationships.push(Relationship {
            id,
            source,
            destination,
            description: optional_argument(arguments.first()),
            technology: optional_argument(arguments.get(1)),
            tags: optional_argument(arguments.get(2))
                .map(|tags| split_tags(&tags))
                .unwrap_or_default(),
            url: None,
            attributes: argument_properties(&arguments, &["description", "technology", "tags"]),
            properties: Vec::new(),
            perspectives: Vec::new(),
            order,
            span: id_span.unwrap_or(source_span).merge(end),
            id_span,
            source_span,
            destination_span,
        });
        if self.at(TokenTag::LeftBrace) {
            if !self.open_block("relationship") {
                return;
            }
            loop {
                self.skip_newlines();
                if self.at(TokenTag::RightBrace) {
                    if let Some(end) = self.close_block("relationship") {
                        self.workspace.relationships[index].span =
                            id_span.unwrap_or(source_span).merge(end.span);
                    }
                    break;
                }
                if self.at(TokenTag::Eof) {
                    self.missing_closing_brace("relationship");
                    break;
                }
                self.parse_relationship_property(index);
            }
        } else {
            self.finish_statement("relationship");
        }
    }

    fn parse_relationship_property(&mut self, index: usize) {
        if self.at_keyword("description") {
            if let Some(property) = self.parse_single_property("description") {
                self.workspace.relationships[index].description = Some(property.value.clone());
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("technology") {
            if let Some(property) = self.parse_single_property("technology") {
                self.workspace.relationships[index].technology = Some(property.value.clone());
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("tag") {
            if let Some(property) = self.parse_single_property("tag") {
                self.workspace.relationships[index]
                    .tags
                    .push(property.value.clone());
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("tags") {
            if let Some(property) = self.parse_single_property("tags") {
                self.workspace.relationships[index]
                    .tags
                    .extend(split_tags(&property.value));
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("url") {
            if let Some(property) = self.parse_single_property("url") {
                self.workspace.relationships[index].url = Some(property.value.clone());
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("properties") {
            let properties = self.parse_property_block("properties");
            self.workspace.relationships[index]
                .properties
                .extend(properties);
        } else if self.at_keyword("perspectives") {
            let perspectives = self.parse_property_block("perspectives");
            self.workspace.relationships[index]
                .perspectives
                .extend(perspectives);
        } else if self.at_keyword("type") {
            if let Some(mut property) = self.parse_single_property("type") {
                if let Some(native_type) =
                    crate::compiler::archimate_relationship_type(&property.value)
                {
                    property.value = native_type.into();
                }
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if self.at_keyword("access") {
            if let Some(property) = self.parse_single_property("access") {
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else if let Some(name) = self.relationship_style_property() {
            if let Some(property) = self.parse_style_property(name) {
                self.workspace.relationships[index]
                    .attributes
                    .push(property);
            }
        } else {
            self.unknown_statement("relationship");
        }
    }

    fn parse_removed_relationship(&mut self) {
        let (source, source_span) = self.take_identifier().unwrap();
        self.advance();
        let Some((destination, destination_span)) = self.take_identifier() else {
            self.error(
                self.current().span,
                "removed relationship destination is missing",
                Some("add an element identifier after '-/>'"),
            );
            self.finish_statement("removed relationship");
            return;
        };
        let description = self.take_value();
        let end = description
            .as_ref()
            .map_or(destination_span, |value| value.1);
        let order = self.next_order();
        self.workspace
            .removed_relationships
            .push(RemovedRelationship {
                source,
                destination,
                description: description
                    .map(|value| value.0)
                    .filter(|value| !value.is_empty()),
                span: source_span.merge(end),
                source_span,
                destination_span,
                order,
            });
        self.finish_statement("removed relationship");
    }

    fn parse_property_block(&mut self, label: &str) -> Vec<Property> {
        self.advance();
        if !self.open_block(label) {
            return Vec::new();
        }
        let mut properties = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block(label);
                return properties;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace(label);
                return properties;
            }
            let Some((key, key_span)) = self.take_value() else {
                self.error(self.current().span, format!("{label} key is missing"), None);
                self.skip_statement();
                continue;
            };
            let Some((value, value_span)) = self.take_value() else {
                self.error(
                    key_span,
                    format!("{label} value for '{key}' is missing"),
                    None,
                );
                self.finish_statement(label);
                continue;
            };
            properties.push(Property {
                key,
                value,
                span: key_span.merge(value_span),
            });
            self.finish_statement(label);
        }
    }

    fn parse_single_property(&mut self, label: &str) -> Option<Property> {
        let start = self.advance();
        let value = self.take_value();
        if value.is_none() {
            self.error(start.span, format!("{label} value is missing"), None);
        }
        self.finish_statement(label);
        value.and_then(|(value, span)| {
            (!value.is_empty()).then(|| Property {
                key: label.into(),
                value,
                span: start.span.merge(span),
            })
        })
    }

    fn parse_directive(&mut self) -> Directive {
        let start = self.advance();
        let name = match &start.kind {
            TokenKind::Bang(name) => name.clone(),
            _ => unreachable!(),
        };
        let arguments = self.take_raw_arguments();
        let mut span = start.span;
        if self.at(TokenTag::LeftBrace) {
            if let Some(end) = self.skip_preserved_block(&name) {
                span = span.merge(end.span);
            }
        } else {
            if let Some(previous) = self.tokens.get(self.index.saturating_sub(1)) {
                span = span.merge(previous.span);
            }
            self.finish_statement(&name);
        }
        Directive {
            name,
            arguments,
            span,
        }
    }

    fn parse_unsafe_directive(&mut self) {
        let directive = self.parse_directive();
        self.error(
            directive.span,
            format!("!{} is disabled and was not executed", directive.name),
            Some("remove executable directives; M6 parses them but never executes code"),
        );
        self.workspace.directives.push(directive);
    }

    fn parse_preserved(&mut self, name: &str) -> PreservedBlock {
        let start = self.advance();
        let arguments = self.take_raw_arguments();
        let span = if self.at(TokenTag::LeftBrace) {
            self.skip_preserved_block(name)
                .map_or(start.span, |end| start.span.merge(end.span))
        } else {
            self.finish_statement(name);
            start.span
        };
        PreservedBlock {
            name: name.into(),
            arguments,
            span,
        }
    }

    fn skip_preserved_block(&mut self, label: &str) -> Option<Token> {
        if !self.open_block(label) {
            return None;
        }
        let mut depth = 1usize;
        while !self.at(TokenTag::Eof) {
            if self.at(TokenTag::LeftBrace) {
                depth += 1;
                self.advance();
            } else if self.at(TokenTag::RightBrace) {
                if depth == 1 {
                    return self.close_block(label);
                }
                depth -= 1;
                self.advance();
            } else {
                self.advance();
            }
        }
        self.missing_closing_brace(label);
        None
    }

    fn warn_deferred(&mut self, directive: &Directive, message: &str) {
        self.workspace.warnings.push(
            Diagnostic::warning(directive.span, format!("!{}: {message}", directive.name))
                .with_help(
                    "the source construct was preserved without executing deferred behavior",
                ),
        );
    }

    fn warn_preserved(&mut self, preserved: &PreservedBlock, message: &str) {
        self.workspace.warnings.push(
            Diagnostic::warning(preserved.span, format!("{}: {message}", preserved.name))
                .with_help("the source block was preserved for a later milestone"),
        );
    }

    fn parse_views(&mut self) {
        self.advance();
        if !self.open_block("views") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("views");
                return;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("views");
                return;
            }
            self.parse_view_statement();
        }
    }

    fn parse_view_statement(&mut self) {
        let kind = if self.at_keyword("systemLandscape") {
            Some(ViewKind::SystemLandscape)
        } else if self.at_keyword("systemContext") {
            Some(ViewKind::SystemContext)
        } else if self.at_keyword("container") {
            Some(ViewKind::Container)
        } else if self.at_keyword("component") {
            Some(ViewKind::Component)
        } else if self.at_keyword("filtered") {
            Some(ViewKind::Filtered)
        } else if self.at_keyword("dynamic") {
            Some(ViewKind::Dynamic)
        } else if self.at_keyword("deployment") {
            Some(ViewKind::Deployment)
        } else if self.at_keyword("custom") {
            Some(ViewKind::Custom)
        } else if self.at_keyword("image") {
            Some(ViewKind::Image)
        } else if self.at_keyword("archimateView") {
            Some(ViewKind::ArchiMate)
        } else {
            None
        };
        if let Some(kind) = kind {
            self.parse_view(kind);
        } else if self.at_keyword("properties") {
            let properties = self.parse_property_block("properties");
            self.workspace.view_properties.extend(properties);
        } else if self.at_keyword("styles") {
            self.parse_styles();
        } else if self.at_keyword("theme") || self.at_keyword("themes") {
            self.parse_themes(StyleMode::Default);
        } else if self.at_keyword("branding") {
            self.parse_branding();
        } else if self.at_keyword("terminology") {
            self.parse_terminology();
        } else {
            self.unknown_statement("views");
        }
    }

    fn parse_styles(&mut self) {
        self.advance();
        if !self.open_block("styles") {
            return;
        }
        self.parse_style_block("styles", StyleMode::Default);
    }

    fn parse_style_block(&mut self, label: &str, mode: StyleMode) {
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block(label);
                return;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace(label);
                return;
            }
            if self.at_keyword("element") {
                self.parse_element_style(mode);
            } else if self.at_keyword("relationship") {
                self.parse_relationship_style(mode);
            } else if self.at_keyword("light") || self.at_keyword("dark") {
                let variant = if self.at_keyword("light") {
                    StyleMode::Light
                } else {
                    StyleMode::Dark
                };
                let start = self.advance();
                if self.open_block("style variant") {
                    self.workspace.warnings.push(
                        Diagnostic::warning(
                            start.span,
                            "light/dark style variants are preserved but Mermaid uses default styles",
                        )
                        .with_help("a future exporter can select the requested style variant"),
                    );
                    self.parse_style_block("style variant", variant);
                }
            } else if self.at_keyword("theme") || self.at_keyword("themes") {
                self.parse_themes(mode);
            } else {
                self.unknown_statement(label);
            }
        }
    }

    fn parse_element_style(&mut self, mode: StyleMode) {
        let start = self.advance();
        let Some((tag, _)) = self.take_value() else {
            self.error(start.span, "element style tag is missing", None);
            self.skip_statement();
            return;
        };
        if !self.open_block("element style") {
            return;
        }
        let mut style = ElementStyle {
            tag,
            mode,
            values: Vec::new(),
            properties: Vec::new(),
            span: start.span,
        };
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                if let Some(end) = self.close_block("element style") {
                    style.span = start.span.merge(end.span);
                }
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("element style");
                break;
            }
            if self.at_keyword("properties") {
                style
                    .properties
                    .extend(self.parse_property_block("properties"));
                self.warn_style(
                    start.span,
                    "element style properties are not rendered by Mermaid",
                );
            } else if let Some(name) = self.element_style_property() {
                if let Some(property) = self.parse_style_property(name) {
                    self.warn_element_style_property(&property);
                    style.values.push(property);
                }
            } else {
                self.unknown_statement("element style");
            }
        }
        self.workspace.element_styles.push(style);
    }

    fn parse_relationship_style(&mut self, mode: StyleMode) {
        let start = self.advance();
        let Some((tag, _)) = self.take_value() else {
            self.error(start.span, "relationship style tag is missing", None);
            self.skip_statement();
            return;
        };
        if !self.open_block("relationship style") {
            return;
        }
        let mut style = RelationshipStyle {
            tag,
            mode,
            values: Vec::new(),
            properties: Vec::new(),
            span: start.span,
        };
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                if let Some(end) = self.close_block("relationship style") {
                    style.span = start.span.merge(end.span);
                }
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("relationship style");
                break;
            }
            if self.at_keyword("properties") {
                style
                    .properties
                    .extend(self.parse_property_block("properties"));
                self.warn_style(
                    start.span,
                    "relationship style properties are not rendered by Mermaid",
                );
            } else if let Some(name) = self.relationship_style_property() {
                if let Some(property) = self.parse_style_property(name) {
                    self.warn_relationship_style_property(&property);
                    style.values.push(property);
                }
            } else {
                self.unknown_statement("relationship style");
            }
        }
        self.workspace.relationship_styles.push(style);
    }

    fn element_style_property(&self) -> Option<&'static str> {
        [
            "shape",
            "icon",
            "width",
            "height",
            "background",
            "color",
            "colour",
            "stroke",
            "strokeWidth",
            "fontSize",
            "border",
            "opacity",
            "metadata",
            "description",
        ]
        .into_iter()
        .find(|name| self.at_keyword(name))
    }

    fn relationship_style_property(&self) -> Option<&'static str> {
        [
            "thickness",
            "color",
            "colour",
            "style",
            "routing",
            "jump",
            "fontSize",
            "width",
            "position",
            "opacity",
        ]
        .into_iter()
        .find(|name| self.at_keyword(name))
    }

    fn parse_style_property(&mut self, name: &str) -> Option<Property> {
        let start = self.advance();
        let value = self.take_value();
        if value.is_none() {
            self.error(start.span, format!("{name} value is missing"), None);
        }
        self.finish_statement(name);
        value.and_then(|(value, span)| {
            (!value.is_empty()).then(|| Property {
                key: if name.eq_ignore_ascii_case("colour") {
                    "color".into()
                } else {
                    name.into()
                },
                value,
                span: start.span.merge(span),
            })
        })
    }

    fn warn_element_style_property(&mut self, property: &Property) {
        let unsupported = match property.key.as_str() {
            "icon" | "width" | "height" | "fontSize" | "opacity" | "metadata" | "description" => {
                true
            }
            "shape" => ![
                "box",
                "roundedbox",
                "circle",
                "hexagon",
                "diamond",
                "cylinder",
            ]
            .iter()
            .any(|shape| property.value.eq_ignore_ascii_case(shape)),
            _ => false,
        };
        if unsupported {
            self.warn_style(
                property.span,
                &format!(
                    "element style property '{}' is preserved but not rendered by Mermaid",
                    property.key
                ),
            );
        }
        if property.key == "icon" && is_remote_url(&property.value) {
            self.warn_style(
                property.span,
                "remote icon URL was preserved but was not fetched",
            );
        }
    }

    fn warn_relationship_style_property(&mut self, property: &Property) {
        if matches!(
            property.key.as_str(),
            "routing" | "jump" | "fontSize" | "width" | "position" | "opacity"
        ) {
            self.warn_style(
                property.span,
                &format!(
                    "relationship style property '{}' is preserved but not rendered by Mermaid",
                    property.key
                ),
            );
        }
    }

    fn parse_themes(&mut self, mode: StyleMode) {
        let start = self.advance();
        let plural = matches!(&start.kind, TokenKind::Identifier(name) if name.eq_ignore_ascii_case("themes"));
        let values = self.take_values();
        if values.is_empty() {
            self.error(start.span, "theme reference is missing", None);
        }
        if !plural {
            self.limit_arguments(&values, 1, "theme accepts one reference");
        }
        for (source, span) in values {
            let message = if is_remote_url(&source) {
                "remote theme URL was preserved but was not fetched"
            } else {
                "theme reference was preserved but was not loaded"
            };
            self.warn_style(span, message);
            self.workspace
                .themes
                .push(ThemeReference { source, mode, span });
        }
        self.finish_statement("theme");
    }

    fn parse_branding(&mut self) {
        let start = self.advance();
        if !self.open_block("branding") {
            return;
        }
        let mut branding = Branding {
            logo: None,
            fonts: Vec::new(),
            span: start.span,
        };
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                if let Some(end) = self.close_block("branding") {
                    branding.span = start.span.merge(end.span);
                }
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("branding");
                break;
            }
            if self.eat_keyword("logo").is_some() {
                if let Some((value, span)) = self.take_value() {
                    if is_remote_url(&value) {
                        self.warn_style(
                            span,
                            "remote branding logo was preserved but was not fetched",
                        );
                    }
                    branding.logo = Some(Property {
                        key: "logo".into(),
                        value,
                        span,
                    });
                } else {
                    self.error(self.current().span, "branding logo is missing", None);
                }
                self.finish_statement("branding logo");
            } else if self.eat_keyword("font").is_some() {
                let values = self.take_values();
                self.limit_arguments(&values, 2, "branding font accepts a name and optional URL");
                if let Some((name, span)) = values.first() {
                    let location = optional_argument(values.get(1));
                    if location.as_deref().is_some_and(is_remote_url) {
                        self.warn_style(
                            values[1].1,
                            "remote branding font was preserved but was not fetched",
                        );
                    }
                    branding.fonts.push(BrandingFont {
                        name: name.clone(),
                        location,
                        span: *span,
                    });
                } else {
                    self.error(self.current().span, "branding font name is missing", None);
                }
                self.finish_statement("branding font");
            } else {
                self.unknown_statement("branding");
            }
        }
        self.warn_style(
            branding.span,
            "branding metadata is preserved but not rendered by Mermaid",
        );
        self.workspace.branding = Some(branding);
    }

    fn parse_terminology(&mut self) {
        self.advance();
        if !self.open_block("terminology") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("terminology");
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("terminology");
                break;
            }
            let Some(name) = [
                "person",
                "softwareSystem",
                "container",
                "component",
                "deploymentNode",
                "infrastructureNode",
                "relationship",
                "metadata",
            ]
            .into_iter()
            .find(|name| self.at_keyword(name)) else {
                self.unknown_statement("terminology");
                continue;
            };
            if let Some(property) = self.parse_style_property(name) {
                if matches!(name, "relationship" | "metadata") {
                    self.warn_style(
                        property.span,
                        &format!(
                            "terminology '{}' is preserved but not rendered by Mermaid",
                            name
                        ),
                    );
                }
                self.workspace.terminology.push(property);
            }
        }
    }

    fn warn_style(&mut self, span: Span, message: &str) {
        self.workspace.warnings.push(
            Diagnostic::warning(span, message)
                .with_help("the value remains available to future local exporters"),
        );
    }

    fn parse_view(&mut self, kind: ViewKind) {
        let start = self.advance();
        let mut scope = None;
        let mut filter = None;
        let mut environment = None;
        let arguments = match kind {
            ViewKind::SystemLandscape | ViewKind::Custom | ViewKind::ArchiMate => {
                self.take_values()
            }
            ViewKind::Filtered => {
                let base = self.take_identifier();
                let mode = self.take_identifier();
                let tags = self.take_value();
                if let (Some((base_key, base_span)), Some((mode, mode_span)), Some((tags, _))) =
                    (base, mode, tags)
                {
                    let mode = if mode.eq_ignore_ascii_case("include") {
                        Some(FilterMode::Include)
                    } else if mode.eq_ignore_ascii_case("exclude") {
                        Some(FilterMode::Exclude)
                    } else {
                        self.error(
                            mode_span,
                            format!("invalid filtered view mode '{mode}'"),
                            Some("use include or exclude"),
                        );
                        None
                    };
                    filter = mode.map(|mode| ViewFilter {
                        base_key: Reference {
                            identifier: base_key,
                            span: base_span,
                        },
                        mode,
                        tags: split_tags(&tags),
                    });
                } else {
                    self.error(
                        self.current().span,
                        "filtered view requires a base key, mode, and tags",
                        Some("use 'filtered <baseKey> <include|exclude> <tags>'"),
                    );
                }
                self.take_values()
            }
            ViewKind::Deployment => {
                scope = self.take_view_scope();
                environment = self
                    .take_value()
                    .map(|(identifier, span)| Reference { identifier, span });
                if environment.is_none() {
                    self.error(
                        self.current().span,
                        "deployment view environment is missing",
                        Some("add an environment identifier or name"),
                    );
                }
                self.take_values()
            }
            ViewKind::Image => {
                scope = self.take_view_scope();
                self.take_values()
            }
            _ => {
                scope = self.take_view_scope();
                self.take_values()
            }
        };
        let maximum = match kind {
            ViewKind::Custom => 3,
            ViewKind::Image | ViewKind::ArchiMate => 1,
            _ => 2,
        };
        self.limit_arguments(&arguments, maximum, "too many arguments for view");
        if requires_scope(&kind) && scope.is_none() {
            self.error(
                self.current().span,
                format!("{} view scope is missing", view_label(&kind)),
                Some("add an element identifier or '*' before the view key"),
            );
        }
        if !self.open_block("view") {
            return;
        }
        let order = self.next_order();
        let key_argument = arguments.first();
        let mut view = View {
            kind: kind.clone(),
            scope: scope.as_ref().map(|scope| scope.0.clone()),
            key: optional_argument(key_argument),
            description: optional_argument(arguments.get(if kind == ViewKind::Custom {
                2
            } else {
                1
            })),
            includes: Vec::new(),
            excludes: Vec::new(),
            auto_layout: None,
            title: (kind == ViewKind::Custom)
                .then(|| optional_argument(arguments.get(1)))
                .flatten(),
            is_default: false,
            animations: Vec::new(),
            properties: Vec::new(),
            filter,
            environment,
            dynamic_relationships: Vec::new(),
            image_sources: Vec::new(),
            order,
            span: start.span,
            scope_span: scope.as_ref().map(|scope| scope.1),
            key_span: key_argument.map(|argument| argument.1),
        };
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                if let Some(end) = self.close_block("view") {
                    view.span = start.span.merge(end.span);
                }
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("view");
                break;
            }
            self.parse_view_property(&mut view);
        }
        if matches!(view.kind, ViewKind::Custom | ViewKind::Image) {
            self.workspace.warnings.push(
                Diagnostic::warning(
                    view.span,
                    format!("{} view rendering is deferred", view_label(&view.kind)),
                )
                .with_help("the view grammar and metadata were preserved without remote rendering"),
            );
        }
        self.workspace.views.push(view);
    }

    fn parse_view_property(&mut self, view: &mut View) {
        if self.eat_keyword("include").is_some() {
            let selectors = self.take_view_selectors();
            if selectors.is_empty() {
                self.error(
                    self.current().span,
                    "include reference is missing",
                    Some("use 'include *' or an element identifier"),
                );
            }
            self.warn_unsupported_selectors(&selectors);
            view.includes.extend(selectors);
            self.finish_statement("include");
        } else if self.eat_keyword("exclude").is_some() {
            let selectors = self.take_view_selectors();
            if selectors.is_empty() {
                self.error(
                    self.current().span,
                    "exclude reference is missing",
                    Some("add an element identifier"),
                );
            }
            self.warn_unsupported_selectors(&selectors);
            view.excludes.extend(selectors);
            self.finish_statement("exclude");
        } else if self.eat_keyword("autolayout").is_some() {
            let start = self.tokens[self.index - 1].span;
            let arguments = self.take_values();
            self.limit_arguments(
                &arguments,
                3,
                "autoLayout accepts direction, rank, and node separation",
            );
            view.auto_layout = Some(AutoLayout {
                direction: arguments
                    .first()
                    .map_or_else(|| "tb".into(), |item| item.0.clone()),
                rank_separation: optional_argument(arguments.get(1)),
                node_separation: optional_argument(arguments.get(2)),
                span: start,
            });
            self.finish_statement("autolayout");
        } else if self.eat_keyword("default").is_some() {
            view.is_default = true;
            self.finish_statement("default");
        } else if self.at_keyword("animation") {
            self.parse_animation(view);
        } else if self.eat_keyword("title").is_some() {
            view.title = self.take_value().map(|value| value.0);
            if view.title.is_none() {
                self.error(
                    self.current().span,
                    "view title is missing",
                    Some("add a quoted or unquoted title"),
                );
            }
            self.finish_statement("title");
        } else if self.eat_keyword("description").is_some() {
            view.description = self.take_value().map(|value| value.0);
            if view.description.is_none() {
                self.error(self.current().span, "view description is missing", None);
            }
            self.finish_statement("description");
        } else if view.kind == ViewKind::ArchiMate && self.at_keyword("viewpoint") {
            if let Some(property) = self.parse_single_property("viewpoint") {
                view.properties.push(property);
            }
        } else if self.at_keyword("properties") {
            view.properties
                .extend(self.parse_property_block("properties"));
        } else if view.kind == ViewKind::ArchiMate && self.at_keyword("object") {
            self.parse_archimate_view_object(view);
        } else if view.kind == ViewKind::Dynamic {
            self.parse_dynamic_relationship(view);
        } else if view.kind == ViewKind::Image
            && ["plantuml", "mermaid", "kroki", "image"]
                .iter()
                .any(|name| self.at_keyword(name))
        {
            self.parse_image_source(view);
        } else {
            self.unknown_statement("view");
        }
    }

    fn parse_archimate_view_object(&mut self, view: &mut View) {
        let start = self.advance();
        let Some((identifier, identifier_span)) = self.take_identifier() else {
            self.error(
                start.span,
                "archimateView object identifier is missing",
                None,
            );
            self.skip_statement();
            return;
        };
        if !self.open_block("archimateView object") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("archimateView object");
                return;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("archimateView object");
                return;
            }
            let Some(name) = [
                "x",
                "y",
                "width",
                "height",
                "background",
                "color",
                "colour",
                "stroke",
                "fontSize",
            ]
            .into_iter()
            .find(|name| self.at_keyword(name)) else {
                self.unknown_statement("archimateView object");
                continue;
            };
            if let Some(mut property) = self.parse_style_property(name) {
                let key = if property.key == "colour" {
                    "color".into()
                } else {
                    property.key.clone()
                };
                property.key = format!("object.{identifier}.{key}");
                property.span = identifier_span.merge(property.span);
                view.properties.push(property);
            }
        }
    }

    fn parse_animation(&mut self, view: &mut View) {
        let start = self.advance();
        if !self.open_block("animation") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("animation");
                break;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("animation");
                break;
            }
            let step_start = self.current().span;
            let elements = self
                .take_values()
                .into_iter()
                .map(|(identifier, span)| Reference { identifier, span })
                .collect::<Vec<_>>();
            if elements.is_empty() {
                self.error(
                    step_start,
                    "animation step is empty",
                    Some("add one or more element identifiers"),
                );
            } else {
                let end = elements.last().map_or(step_start, |item| item.span);
                view.animations.push(AnimationStep {
                    elements,
                    span: step_start.merge(end),
                });
            }
            self.finish_statement("animation");
        }
        if view.animations.is_empty() {
            self.workspace.warnings.push(
                Diagnostic::warning(start.span, "animation has no steps")
                    .with_help("add identifiers inside the animation block"),
            );
        }
    }

    fn parse_dynamic_relationship(&mut self, view: &mut View) {
        let start = self.current().span;
        let mut sequence = None;
        if let TokenKind::Identifier(value) = &self.current().kind {
            if value.ends_with(':') {
                sequence = Some(value.trim_end_matches(':').to_string());
                self.advance();
            }
        }
        let first = self.take_identifier();
        let Some((identifier, identifier_span)) = first else {
            self.unknown_statement("dynamic view");
            return;
        };
        let mut relationship = DynamicRelationship {
            sequence,
            source: None,
            destination: None,
            relationship: None,
            description: None,
            technology: None,
            span: start,
        };
        if self.at(TokenTag::Arrow) {
            self.advance();
            relationship.source = Some(Reference {
                identifier,
                span: identifier_span,
            });
            relationship.destination = self
                .take_identifier()
                .map(|(identifier, span)| Reference { identifier, span });
            if relationship.destination.is_none() {
                self.error(
                    self.current().span,
                    "dynamic relationship destination is missing",
                    None,
                );
            }
            let arguments = self.take_values();
            self.limit_arguments(
                &arguments,
                2,
                "dynamic relationship accepts description and technology",
            );
            relationship.description = optional_argument(arguments.first());
            relationship.technology = optional_argument(arguments.get(1));
        } else {
            relationship.relationship = Some(Reference {
                identifier,
                span: identifier_span,
            });
            let arguments = self.take_values();
            self.limit_arguments(
                &arguments,
                1,
                "relationship reference accepts one description override",
            );
            relationship.description = optional_argument(arguments.first());
        }
        if let Some(previous) = self.tokens.get(self.index.saturating_sub(1)) {
            relationship.span = start.merge(previous.span);
        }
        self.finish_statement("dynamic relationship");
        view.dynamic_relationships.push(relationship);
    }

    fn parse_image_source(&mut self, view: &mut View) {
        let start = self.advance();
        let kind = match &start.kind {
            TokenKind::Identifier(value) => value.clone(),
            _ => unreachable!(),
        };
        let arguments = self.take_values();
        let expected = if kind.eq_ignore_ascii_case("kroki") {
            2
        } else {
            1
        };
        if arguments.len() != expected {
            self.error(
                start.span,
                format!("{kind} image source expects {expected} argument(s)"),
                None,
            );
        }
        if let Some((_, span)) = arguments.iter().find(|(value, _)| is_remote_url(value)) {
            self.error(
                *span,
                "remote image source is disabled and was not fetched",
                Some("use a local file or view key; remote rendering is not allowed"),
            );
        }
        let span = arguments
            .last()
            .map_or(start.span, |item| start.span.merge(item.1));
        view.image_sources.push(ImageSource {
            kind,
            arguments: arguments.into_iter().map(|item| item.0).collect(),
            span,
        });
        self.finish_statement("image source");
    }

    fn take_view_scope(&mut self) -> Option<(String, Span)> {
        if self.at(TokenTag::Star) {
            let span = self.advance().span;
            Some(("*".into(), span))
        } else {
            self.take_identifier()
        }
    }

    fn take_view_selectors(&mut self) -> Vec<ViewSelector> {
        let mut tokens = Vec::new();
        while !self.at(TokenTag::Newline)
            && !self.at(TokenTag::RightBrace)
            && !self.at(TokenTag::Eof)
        {
            tokens.push(self.advance());
        }
        if tokens.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::Arrow
                    | TokenKind::Equals
                    | TokenKind::DoubleEquals
                    | TokenKind::NotEquals
                    | TokenKind::And
                    | TokenKind::Or
                    | TokenKind::Not
                    | TokenKind::LeftParen
                    | TokenKind::RightParen
            )
        }) {
            let span = tokens
                .first()
                .unwrap()
                .span
                .merge(tokens.last().unwrap().span);
            return vec![ViewSelector {
                value: tokens
                    .iter()
                    .map(|token| token_text(&token.kind))
                    .collect::<Vec<_>>()
                    .join(" "),
                span,
                expression: true,
            }];
        }
        tokens
            .into_iter()
            .map(|token| {
                let value = match &token.kind {
                    TokenKind::Star => "*".into(),
                    TokenKind::ReluctantStar => "*?".into(),
                    TokenKind::Identifier(value) | TokenKind::String(value) => value.clone(),
                    _ => token_text(&token.kind),
                };
                let expression = value.contains("->")
                    || value.contains(' ')
                    || value.contains("==")
                    || value.contains("!=");
                ViewSelector {
                    value,
                    span: token.span,
                    expression,
                }
            })
            .collect()
    }

    fn warn_unsupported_selectors(&mut self, selectors: &[ViewSelector]) {
        for selector in selectors {
            if selector.expression
                && !selector.value.contains("->")
                && !supported_view_expression(&selector.value)
            {
                self.workspace.warnings.push(
                    Diagnostic::warning(
                        selector.span,
                        format!(
                            "view expression '{}' was preserved but is not evaluated",
                            selector.value
                        ),
                    )
                    .with_help(
                        "use identifiers, wildcards, or a simple source -> destination pattern",
                    ),
                );
            }
        }
    }

    fn take_values(&mut self) -> Vec<(String, Span)> {
        let mut values = Vec::new();
        while let Some(value) = self.take_value() {
            values.push(value);
        }
        values
    }

    fn take_value(&mut self) -> Option<(String, Span)> {
        match &self.current().kind {
            TokenKind::Identifier(value) | TokenKind::String(value) => {
                let value = value.clone();
                let span = self.advance().span;
                Some((value, span))
            }
            _ => None,
        }
    }

    fn take_identifier(&mut self) -> Option<(String, Span)> {
        match &self.current().kind {
            TokenKind::Identifier(value) => {
                let value = value.clone();
                let span = self.advance().span;
                Some((value, span))
            }
            _ => None,
        }
    }

    fn take_raw_arguments(&mut self) -> Vec<String> {
        let mut arguments = Vec::new();
        while !self.at(TokenTag::LeftBrace)
            && !self.at(TokenTag::RightBrace)
            && !self.at(TokenTag::Newline)
            && !self.at(TokenTag::Eof)
        {
            arguments.push(token_text(&self.advance().kind));
        }
        arguments
    }

    fn qualify_identifier(&self, identifier: String) -> String {
        if self.workspace.identifiers == "hierarchical" {
            self.parent_stack
                .last()
                .map(|parent| format!("{parent}.{identifier}"))
                .unwrap_or(identifier)
        } else {
            identifier
        }
    }

    fn next_order(&mut self) -> usize {
        let order = self.order;
        self.order += 1;
        order
    }

    fn limit_arguments(&mut self, arguments: &[(String, Span)], maximum: usize, message: &str) {
        if arguments.len() > maximum {
            self.error(arguments[maximum].1, message, None);
        }
    }

    fn open_block(&mut self, label: &str) -> bool {
        if self.at(TokenTag::LeftBrace) {
            self.advance();
        } else if self.at(TokenTag::Newline) {
            let span = self.current().span;
            self.error(
                span,
                format!("opening '{{' for {label} must be on the same line"),
                Some("move '{' to the end of the preceding statement"),
            );
            self.skip_newlines();
            if self.at(TokenTag::LeftBrace) {
                self.advance();
            } else {
                return false;
            }
        } else {
            self.error(
                self.current().span,
                format!("expected '{{' to open {label}"),
                Some("add '{' on this line"),
            );
            self.skip_statement();
            return false;
        }
        if self.at(TokenTag::Newline) {
            self.advance();
        } else if !self.at(TokenTag::Eof) {
            self.error(
                self.current().span,
                format!("content inside {label} must start on a new line"),
                None,
            );
            self.skip_statement();
        }
        true
    }

    fn close_block(&mut self, label: &str) -> Option<Token> {
        let closing = self.current().clone();
        let starts_line = self.index == 0 || self.nth_at_back(1, TokenTag::Newline);
        let ends_line = self.nth_at(1, TokenTag::Newline) || self.nth_at(1, TokenTag::Eof);
        if !starts_line || !ends_line {
            self.error(
                closing.span,
                format!("closing '}}' for {label} must be on a line of its own"),
                Some("move '}' to a separate line"),
            );
        }
        self.advance();
        if !ends_line {
            self.skip_statement();
        }
        Some(closing)
    }

    fn missing_closing_brace(&mut self, label: &str) {
        self.error(
            self.current().span,
            format!("missing closing '}}' for {label}"),
            Some("add '}' on a line of its own"),
        );
    }

    fn finish_statement(&mut self, label: &str) {
        if self.at(TokenTag::Newline) {
            self.advance();
        } else if !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            self.error(
                self.current().span,
                format!("unexpected token in {label} statement"),
                Some("put each DSL statement on its own line"),
            );
            self.skip_statement();
        }
    }

    fn unknown_statement(&mut self, context: &str) {
        let token = self.current().clone();
        self.error(
            token.span,
            format!("unknown {context} statement '{}'", token_text(&token.kind)),
            Some("this feature is not part of the implemented M4 grammar"),
        );
        self.skip_statement();
    }

    fn skip_statement(&mut self) {
        let mut depth = 0usize;
        while !self.at(TokenTag::Eof) {
            match self.current().kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace if depth > 0 => depth -= 1,
                TokenKind::Newline if depth == 0 => {
                    self.advance();
                    return;
                }
                _ => {}
            }
            self.advance();
        }
    }

    fn relationship_ahead(&self, arrow: TokenTag) -> bool {
        (self.identifier_ahead() && self.nth_at(1, arrow))
            || (self.identifier_ahead()
                && self.nth_at(1, TokenTag::Equals)
                && self.nth_at(2, TokenTag::Identifier)
                && self.nth_at(3, arrow))
    }

    fn identifier_ahead(&self) -> bool {
        self.at(TokenTag::Identifier)
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(value) if value.eq_ignore_ascii_case(keyword))
    }

    fn eat_keyword(&mut self, keyword: &str) -> Option<Token> {
        self.at_keyword(keyword).then(|| self.advance())
    }

    fn at_bang(&self, keyword: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Bang(value) if value.eq_ignore_ascii_case(keyword))
    }

    fn at_any_bang(&self, keywords: &[&str]) -> bool {
        keywords.iter().any(|keyword| self.at_bang(keyword))
    }

    fn at(&self, tag: TokenTag) -> bool {
        token_is(&self.current().kind, tag)
    }

    fn nth_at(&self, offset: usize, tag: TokenTag) -> bool {
        self.tokens
            .get(self.index + offset)
            .is_some_and(|token| token_is(&token.kind, tag))
    }

    fn nth_at_back(&self, offset: usize, tag: TokenTag) -> bool {
        self.index
            .checked_sub(offset)
            .and_then(|index| self.tokens.get(index))
            .is_some_and(|token| token_is(&token.kind, tag))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.index += 1;
        }
        token
    }

    fn skip_newlines(&mut self) {
        while self.at(TokenTag::Newline) {
            self.advance();
        }
    }

    fn error(&mut self, span: Span, message: impl Into<String>, help: Option<&str>) {
        let mut diagnostic = Diagnostic::error(span, message);
        if let Some(help) = help {
            diagnostic = diagnostic.with_help(help);
        }
        self.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Copy)]
enum TokenTag {
    Identifier,
    LeftBrace,
    RightBrace,
    Equals,
    Arrow,
    RemoveArrow,
    Star,
    Newline,
    Eof,
}

fn token_is(kind: &TokenKind, tag: TokenTag) -> bool {
    matches!(
        (kind, tag),
        (TokenKind::Identifier(_), TokenTag::Identifier)
            | (TokenKind::LeftBrace, TokenTag::LeftBrace)
            | (TokenKind::RightBrace, TokenTag::RightBrace)
            | (TokenKind::Equals, TokenTag::Equals)
            | (TokenKind::Arrow, TokenTag::Arrow)
            | (TokenKind::RemoveArrow, TokenTag::RemoveArrow)
            | (TokenKind::Star, TokenTag::Star)
            | (TokenKind::Newline, TokenTag::Newline)
            | (TokenKind::Eof, TokenTag::Eof)
    )
}

fn optional_argument(argument: Option<&(String, Span)>) -> Option<String> {
    argument
        .map(|argument| argument.0.clone())
        .filter(|argument| !argument.is_empty())
}

fn argument_properties(arguments: &[(String, Span)], names: &[&str]) -> Vec<Property> {
    arguments
        .iter()
        .zip(names)
        .filter(|((value, _), _)| !value.is_empty())
        .map(|((value, span), name)| Property {
            key: (*name).into(),
            value: value.clone(),
            span: *span,
        })
        .collect()
}

fn element_argument_properties(kind: &ElementKind, arguments: &[(String, Span)]) -> Vec<Property> {
    let names: &[&str] = match kind {
        ElementKind::Person | ElementKind::SoftwareSystem => &["name", "description", "tags"],
        ElementKind::Container | ElementKind::Component | ElementKind::InfrastructureNode => {
            &["name", "description", "technology", "tags"]
        }
        ElementKind::Generic => &["name", "type", "description", "technology", "tags"],
        ElementKind::DeploymentNode => &["name", "description", "technology", "instances", "tags"],
        ElementKind::SoftwareSystemInstance | ElementKind::ContainerInstance => {
            &["reference", "deploymentGroups", "tags"]
        }
        ElementKind::DeploymentEnvironment | ElementKind::DeploymentGroup => &["name"],
        ElementKind::ArchiMate(_) => &["name", "description"],
    };
    argument_properties(arguments, names)
}

struct InlineProperties {
    description: Option<String>,
    technology: Option<String>,
    tags: Vec<String>,
    instances: Option<String>,
    element_type: Option<String>,
    deployment_groups: Vec<Reference>,
}

fn inline_properties(kind: &ElementKind, arguments: &[(String, Span)]) -> InlineProperties {
    let values = match kind {
        ElementKind::Person | ElementKind::SoftwareSystem => (
            optional_argument(arguments.get(1)),
            None,
            optional_argument(arguments.get(2))
                .map(|value| split_tags(&value))
                .unwrap_or_default(),
            None,
            None,
            Vec::new(),
        ),
        ElementKind::Container | ElementKind::Component | ElementKind::InfrastructureNode => (
            optional_argument(arguments.get(1)),
            optional_argument(arguments.get(2)),
            optional_argument(arguments.get(3))
                .map(|value| split_tags(&value))
                .unwrap_or_default(),
            None,
            None,
            Vec::new(),
        ),
        ElementKind::Generic => (
            optional_argument(arguments.get(2)),
            optional_argument(arguments.get(3)),
            optional_argument(arguments.get(4))
                .map(|value| split_tags(&value))
                .unwrap_or_default(),
            None,
            optional_argument(arguments.get(1)),
            Vec::new(),
        ),
        ElementKind::DeploymentNode => (
            optional_argument(arguments.get(1)),
            optional_argument(arguments.get(2)),
            optional_argument(arguments.get(4))
                .map(|value| split_tags(&value))
                .unwrap_or_default(),
            optional_argument(arguments.get(3)),
            None,
            Vec::new(),
        ),
        ElementKind::SoftwareSystemInstance | ElementKind::ContainerInstance => (
            None,
            None,
            optional_argument(arguments.get(2))
                .map(|value| split_tags(&value))
                .unwrap_or_default(),
            None,
            None,
            arguments
                .get(1)
                .map(|(groups, span)| {
                    split_tags(groups)
                        .into_iter()
                        .map(|identifier| Reference {
                            identifier,
                            span: *span,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        ),
        ElementKind::DeploymentEnvironment | ElementKind::DeploymentGroup => {
            (None, None, Vec::new(), None, None, Vec::new())
        }
        ElementKind::ArchiMate(_) => (
            optional_argument(arguments.get(1)),
            None,
            Vec::new(),
            None,
            None,
            Vec::new(),
        ),
    };
    InlineProperties {
        description: values.0,
        technology: values.1,
        tags: values.2,
        instances: values.3,
        element_type: values.4,
        deployment_groups: values.5,
    }
}

fn maximum_arguments(kind: &ElementKind) -> usize {
    match kind {
        ElementKind::Person | ElementKind::SoftwareSystem => 3,
        ElementKind::Container | ElementKind::Component | ElementKind::InfrastructureNode => 4,
        ElementKind::Generic | ElementKind::DeploymentNode => 5,
        ElementKind::SoftwareSystemInstance | ElementKind::ContainerInstance => 3,
        ElementKind::DeploymentEnvironment | ElementKind::DeploymentGroup => 1,
        ElementKind::ArchiMate(_) => 2,
    }
}

fn split_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn slug(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn element_label(kind: &ElementKind) -> &'static str {
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

fn view_label(kind: &ViewKind) -> &'static str {
    match kind {
        ViewKind::SystemLandscape => "systemLandscape",
        ViewKind::SystemContext => "systemContext",
        ViewKind::Container => "container",
        ViewKind::Component => "component",
        ViewKind::Filtered => "filtered",
        ViewKind::Dynamic => "dynamic",
        ViewKind::Deployment => "deployment",
        ViewKind::Custom => "custom",
        ViewKind::Image => "image",
        ViewKind::ArchiMate => "archimateView",
    }
}

fn requires_scope(kind: &ViewKind) -> bool {
    matches!(
        kind,
        ViewKind::SystemContext
            | ViewKind::Container
            | ViewKind::Component
            | ViewKind::Dynamic
            | ViewKind::Deployment
            | ViewKind::Image
    )
}

fn is_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn token_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(value) => value.clone(),
        TokenKind::Bang(value) => format!("!{value}"),
        TokenKind::String(value) => format!("\"{value}\""),
        TokenKind::LeftBrace => "{".into(),
        TokenKind::RightBrace => "}".into(),
        TokenKind::Equals => "=".into(),
        TokenKind::DoubleEquals => "==".into(),
        TokenKind::NotEquals => "!=".into(),
        TokenKind::And => "&&".into(),
        TokenKind::Or => "||".into(),
        TokenKind::Not => "!".into(),
        TokenKind::LeftParen => "(".into(),
        TokenKind::RightParen => ")".into(),
        TokenKind::Arrow => "->".into(),
        TokenKind::RemoveArrow => "-/>".into(),
        TokenKind::Star => "*".into(),
        TokenKind::ReluctantStar => "*?".into(),
        TokenKind::Newline => "newline".into(),
        TokenKind::Eof => "end of file".into(),
    }
}

fn supported_view_expression(value: &str) -> bool {
    value.split_whitespace().any(|token| {
        token.starts_with("element.tag")
            || token.starts_with("element.type")
            || token.starts_with("element.properties.")
            || token.starts_with("relationship.tag")
            || token.starts_with("relationship.properties.")
    })
}
