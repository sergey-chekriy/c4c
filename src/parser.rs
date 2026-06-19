use crate::{
    compiler::{Element, ElementKind, Relationship, View, ViewKind, Workspace},
    diagnostic::{render_all, Diagnostic},
    lexer::{Token, TokenKind},
    source::{SourceId, SourceMap, Span},
};

pub fn parse(
    sources: SourceMap,
    source_id: SourceId,
    tokens: Vec<Token>,
) -> Result<Workspace, String> {
    let mut parser = Parser::new(sources, source_id, tokens);
    parser.parse_workspace();
    if parser.diagnostics.is_empty() {
        Ok(parser.workspace)
    } else {
        Err(render_all(&parser.diagnostics, &parser.sources))
    }
}

struct Parser {
    sources: SourceMap,
    tokens: Vec<Token>,
    index: usize,
    diagnostics: Vec<Diagnostic>,
    workspace: Workspace,
    id_stack: Vec<String>,
}

impl Parser {
    fn new(sources: SourceMap, source_id: SourceId, tokens: Vec<Token>) -> Self {
        Self {
            workspace: Workspace::new(
                sources.clone(),
                Span::new(source_id, 0, sources.get(source_id).text.len()),
            ),
            sources,
            tokens,
            index: 0,
            diagnostics: Vec::new(),
            id_stack: Vec::new(),
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
        let arguments = self.take_values();
        if arguments.len() > 2 {
            self.error(
                arguments[2].1,
                "workspace accepts at most a name and description",
                None,
            );
        }
        self.workspace.name = optional_argument(arguments.first());
        self.workspace.description = optional_argument(arguments.get(1));
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
            if self.at_bang("identifiers") {
                self.parse_identifiers();
            } else if self.at_keyword("model") {
                self.parse_model();
            } else if self.at_keyword("views") {
                self.parse_views();
            } else {
                self.unknown_statement("workspace");
            }
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

    fn parse_identifiers(&mut self) {
        let keyword = self.advance();
        let value = self.take_value();
        match value {
            Some((value, _)) if value.eq_ignore_ascii_case("hierarchical") => {
                self.workspace.identifiers = "hierarchical".into();
            }
            Some((value, span)) => self.error(
                span,
                format!("unsupported identifier mode '{value}' in M2"),
                Some("use '!identifiers hierarchical'"),
            ),
            None => self.error(
                keyword.span,
                "missing identifier mode",
                Some("use '!identifiers hierarchical'"),
            ),
        }
        self.finish_statement("identifier mode");
    }

    fn parse_model(&mut self) {
        self.advance();
        if !self.open_block("model") {
            return;
        }
        loop {
            self.skip_newlines();
            if self.at(TokenTag::RightBrace) {
                self.close_block("model");
                return;
            }
            if self.at(TokenTag::Eof) {
                self.missing_closing_brace("model");
                return;
            }
            self.parse_model_statement();
        }
    }

    fn parse_model_statement(&mut self) {
        if self.relationship_ahead() {
            self.parse_relationship();
            return;
        }
        let statement_start = self.current().span;
        let assigned = if self.identifier_ahead() && self.nth_at(1, TokenTag::Equals) {
            let (identifier, span) = self.take_identifier().unwrap();
            self.advance();
            Some((identifier, span))
        } else {
            None
        };

        let kind = if self.at_keyword("person") {
            Some((ElementKind::Person, false))
        } else if self.at_keyword("softwareSystem") {
            Some((ElementKind::SoftwareSystem, true))
        } else if self.at_keyword("container") {
            Some((ElementKind::Container, true))
        } else if self.at_keyword("component") {
            Some((ElementKind::Component, false))
        } else {
            None
        };
        if let Some((kind, may_have_children)) = kind {
            self.parse_element(statement_start, assigned, kind, may_have_children);
        } else if assigned.is_some() {
            self.error(
                self.current().span,
                "expected an M1 element type after '='",
                Some(
                    "supported element types are person, softwareSystem, container, and component",
                ),
            );
            self.skip_statement();
        } else {
            self.unknown_statement("model");
        }
    }

    fn parse_element(
        &mut self,
        statement_start: Span,
        assigned: Option<(String, Span)>,
        kind: ElementKind,
        may_have_children: bool,
    ) {
        let keyword = self.advance();
        let arguments = self.take_values();
        let Some((name, _)) = arguments.first().cloned() else {
            self.error(
                self.current().span,
                format!("{} name is missing", element_label(&kind)),
                Some("add a quoted or unquoted name on this line"),
            );
            self.finish_statement("element");
            return;
        };
        if name.is_empty() {
            self.error(
                arguments[0].1,
                format!("{} name cannot be empty", element_label(&kind)),
                None,
            );
        }

        let maximum_arguments = if matches!(kind, ElementKind::Container | ElementKind::Component) {
            4
        } else {
            3
        };
        if arguments.len() > maximum_arguments {
            self.error(
                arguments[maximum_arguments].1,
                format!("too many properties for {}", element_label(&kind)),
                None,
            );
        }
        let description = optional_argument(arguments.get(1));
        let (technology, tags_index) =
            if matches!(kind, ElementKind::Container | ElementKind::Component) {
                (optional_argument(arguments.get(2)), 3)
            } else {
                (None, 2)
            };
        let tags = optional_argument(arguments.get(tags_index))
            .map(|tags| split_tags(&tags))
            .unwrap_or_default();
        let (base_id, id_span) = assigned.unwrap_or_else(|| (slug(&name), arguments[0].1));
        let id = if self.workspace.identifiers == "hierarchical" {
            self.id_stack
                .last()
                .map(|parent| format!("{parent}.{base_id}"))
                .unwrap_or(base_id)
        } else {
            base_id
        };
        let parent = self.id_stack.last().cloned();
        let mut span = statement_start.merge(
            arguments
                .last()
                .map(|argument| argument.1)
                .unwrap_or(keyword.span),
        );
        let element_index = self.workspace.elements.len();
        self.workspace.elements.push(Element {
            id: id.clone(),
            kind,
            name,
            description,
            technology,
            parent,
            tags,
            span,
            id_span,
        });

        if self.at(TokenTag::LeftBrace) {
            if !may_have_children {
                self.error(
                    self.current().span,
                    format!(
                        "{} child blocks are not supported in M2",
                        element_label(&self.workspace.elements[element_index].kind)
                    ),
                    None,
                );
                self.skip_statement();
                return;
            }
            if !self.open_block("element") {
                return;
            }
            self.id_stack.push(id);
            loop {
                self.skip_newlines();
                if self.at(TokenTag::RightBrace) {
                    if let Some(end) = self.close_block("element") {
                        span = span.merge(end.span);
                        self.workspace.elements[element_index].span = span;
                    }
                    break;
                }
                if self.at(TokenTag::Eof) {
                    self.missing_closing_brace("element");
                    break;
                }
                self.parse_model_statement();
            }
            self.id_stack.pop();
        } else {
            self.finish_statement("element");
        }
    }

    fn parse_relationship(&mut self) {
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
        if arguments.len() > 3 {
            self.error(
                arguments[3].1,
                "relationship accepts at most description, technology, and tags",
                None,
            );
        }
        let end = arguments
            .last()
            .map(|argument| argument.1)
            .unwrap_or(destination_span);
        self.workspace.relationships.push(Relationship {
            source,
            destination,
            description: optional_argument(arguments.first()),
            technology: optional_argument(arguments.get(1)),
            tags: optional_argument(arguments.get(2))
                .map(|tags| split_tags(&tags))
                .unwrap_or_default(),
            span: source_span.merge(end),
            source_span,
            destination_span,
        });
        self.finish_statement("relationship");
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
        if self.at_keyword("systemContext") {
            self.parse_view(ViewKind::SystemContext);
        } else if self.at_keyword("container") {
            self.parse_view(ViewKind::Container);
        } else {
            self.unknown_statement("views");
        }
    }

    fn parse_view(&mut self, kind: ViewKind) {
        let start = self.advance();
        let scope = self.take_identifier();
        if scope.is_none() {
            self.error(
                self.current().span,
                format!("{} view scope is missing", view_label(&kind)),
                Some("add a software system identifier before the view key"),
            );
        }
        let arguments = self.take_values();
        if arguments.len() > 2 {
            self.error(
                arguments[2].1,
                "view accepts at most a key and description after its scope",
                None,
            );
        }
        if !self.open_block("view") {
            return;
        }
        let mut view = View {
            kind,
            scope: scope.as_ref().map(|scope| scope.0.clone()),
            key: optional_argument(arguments.first()),
            description: optional_argument(arguments.get(1)),
            includes: Vec::new(),
            excludes: Vec::new(),
            auto_layout: None,
            title: None,
            span: start.span,
            scope_span: scope.as_ref().map(|scope| scope.1),
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
        self.workspace.views.push(view);
    }

    fn parse_view_property(&mut self, view: &mut View) {
        if self.eat_keyword("include").is_some() {
            if let Some(reference) = self.take_reference() {
                view.includes.push(reference);
            } else {
                self.error(
                    self.current().span,
                    "include reference is missing",
                    Some("use 'include *' or an element identifier"),
                );
            }
            self.finish_statement("include");
        } else if self.eat_keyword("exclude").is_some() {
            if let Some(reference) = self.take_reference() {
                view.excludes.push(reference);
            } else {
                self.error(
                    self.current().span,
                    "exclude reference is missing",
                    Some("add an element identifier"),
                );
            }
            self.finish_statement("exclude");
        } else if self.eat_keyword("autolayout").is_some() {
            view.auto_layout = self
                .take_value()
                .map(|value| value.0)
                .or_else(|| Some("tb".into()));
            self.finish_statement("autolayout");
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
        } else {
            self.unknown_statement("view");
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

    fn take_reference(&mut self) -> Option<String> {
        if self.at(TokenTag::Star) {
            self.advance();
            Some("*".into())
        } else {
            self.take_identifier().map(|reference| reference.0)
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
            Some("M2 accepts only the Milestone 1 language subset"),
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

    fn relationship_ahead(&self) -> bool {
        self.identifier_ahead() && self.nth_at(1, TokenTag::Arrow)
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
    }
}

fn view_label(kind: &ViewKind) -> &'static str {
    match kind {
        ViewKind::SystemContext => "systemContext",
        ViewKind::Container => "container",
    }
}

fn token_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(value) => value.clone(),
        TokenKind::Bang(value) => format!("!{value}"),
        TokenKind::String(value) => format!("\"{value}\""),
        TokenKind::LeftBrace => "{".into(),
        TokenKind::RightBrace => "}".into(),
        TokenKind::Equals => "=".into(),
        TokenKind::Arrow => "->".into(),
        TokenKind::Star => "*".into(),
        TokenKind::Newline => "newline".into(),
        TokenKind::Eof => "end of file".into(),
    }
}
