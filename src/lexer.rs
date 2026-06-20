use crate::{
    diagnostic::Diagnostic,
    source::{SourceFile, Span},
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Bang(String),
    String(String),
    LeftBrace,
    RightBrace,
    Equals,
    DoubleEquals,
    NotEquals,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    Arrow,
    RemoveArrow,
    Star,
    ReluctantStar,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn lex(source: &SourceFile) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut lexer = Lexer {
        source,
        offset: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
    };
    lexer.run();
    if lexer.diagnostics.is_empty() {
        Ok(lexer.tokens)
    } else {
        Err(lexer.diagnostics)
    }
}

struct Lexer<'a> {
    source: &'a SourceFile,
    offset: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl Lexer<'_> {
    fn run(&mut self) {
        while self.offset < self.source.text.len() {
            let start = self.offset;
            match self.current() {
                ' ' | '\t' | '\r' => self.advance(),
                '\n' => {
                    self.advance();
                    self.push(TokenKind::Newline, start, self.offset);
                }
                '\\' if self.rest().starts_with("\\\n") => self.offset += 2,
                '\\' if self.rest().starts_with("\\\r\n") => self.offset += 3,
                '/' if self.rest().starts_with("//") => self.skip_comment(),
                '#' if self.color() => {}
                '#' => self.skip_comment(),
                '-' if self.rest().starts_with("-/>") => {
                    self.offset += 3;
                    self.push(TokenKind::RemoveArrow, start, self.offset);
                }
                '-' if self.rest().starts_with("->") => {
                    self.offset += 2;
                    self.push(TokenKind::Arrow, start, self.offset);
                }
                '{' => self.single(TokenKind::LeftBrace, start),
                '}' => self.single(TokenKind::RightBrace, start),
                '=' if self.rest().starts_with("==") => {
                    self.offset += 2;
                    self.push(TokenKind::DoubleEquals, start, self.offset);
                }
                '=' => self.single(TokenKind::Equals, start),
                '!' if self.rest().starts_with("!=") => {
                    self.offset += 2;
                    self.push(TokenKind::NotEquals, start, self.offset);
                }
                '&' if self.rest().starts_with("&&") => {
                    self.offset += 2;
                    self.push(TokenKind::And, start, self.offset);
                }
                '|' if self.rest().starts_with("||") => {
                    self.offset += 2;
                    self.push(TokenKind::Or, start, self.offset);
                }
                '(' => self.single(TokenKind::LeftParen, start),
                ')' => self.single(TokenKind::RightParen, start),
                '*' if self.rest().starts_with("*?") => {
                    self.offset += 2;
                    self.push(TokenKind::ReluctantStar, start, self.offset);
                }
                '*' => self.single(TokenKind::Star, start),
                '"' => {
                    if !self.string(start) {
                        break;
                    }
                }
                '!' if self.directive_position() => self.bang(start),
                '!' => self.single(TokenKind::Not, start),
                character if is_identifier(character) => self.identifier(start),
                character => {
                    self.advance();
                    self.diagnostics.push(
                        Diagnostic::error(
                            Span::new(self.source.id, start, self.offset),
                            format!("unexpected character '{character}'"),
                        )
                        .with_help(
                            "remove this character or separate valid DSL tokens with whitespace",
                        ),
                    );
                }
            }
        }
        let end = self.source.text.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.source.id, end, end),
        });
    }

    fn string(&mut self, start: usize) -> bool {
        self.advance();
        let mut value = String::new();
        while self.offset < self.source.text.len() {
            match self.current() {
                '"' => {
                    self.advance();
                    self.push(TokenKind::String(value), start, self.offset);
                    return true;
                }
                '\\' => {
                    self.advance();
                    if self.offset == self.source.text.len() {
                        break;
                    }
                    value.push(self.current());
                    self.advance();
                }
                character => {
                    value.push(character);
                    self.advance();
                }
            }
        }
        self.diagnostics.push(
            Diagnostic::error(
                Span::new(self.source.id, start, self.source.text.len()),
                "unterminated string",
            )
            .with_help("add a closing double quote"),
        );
        false
    }

    fn bang(&mut self, start: usize) {
        self.advance();
        let value_start = self.offset;
        while self.offset < self.source.text.len() && is_identifier(self.current()) {
            self.advance();
        }
        if value_start == self.offset {
            self.diagnostics.push(Diagnostic::error(
                Span::new(self.source.id, start, self.offset),
                "expected a keyword after '!'",
            ));
        } else {
            self.push(
                TokenKind::Bang(self.source.text[value_start..self.offset].to_string()),
                start,
                self.offset,
            );
        }
    }

    fn identifier(&mut self, start: usize) {
        while self.offset < self.source.text.len() && is_identifier(self.current()) {
            self.advance();
        }
        self.push(
            TokenKind::Identifier(self.source.text[start..self.offset].to_string()),
            start,
            self.offset,
        );
    }

    fn single(&mut self, kind: TokenKind, start: usize) {
        self.advance();
        self.push(kind, start, self.offset);
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        if let Some(previous) = self.tokens.last() {
            if !matches!(previous.kind, TokenKind::Newline)
                && !matches!(kind, TokenKind::Newline)
                && previous.span.end == start
                && !expression_token(&previous.kind)
                && !expression_token(&kind)
            {
                self.diagnostics.push(
                    Diagnostic::error(
                        Span::new(self.source.id, start, end),
                        "DSL tokens must be separated by whitespace",
                    )
                    .with_help("insert a space between these tokens"),
                );
            }
        }
        self.tokens.push(Token {
            kind,
            span: Span::new(self.source.id, start, end),
        });
    }

    fn skip_comment(&mut self) {
        while self.offset < self.source.text.len() && self.current() != '\n' {
            self.advance();
        }
    }

    fn color(&mut self) -> bool {
        if !matches!(
            self.tokens.last().map(|token| &token.kind),
            Some(TokenKind::Identifier(name))
                if ["background", "color", "colour", "stroke"]
                    .iter()
                    .any(|keyword| name.eq_ignore_ascii_case(keyword))
        ) {
            return false;
        }
        let start = self.offset;
        let Some(value) = self.rest().get(1..7) else {
            return false;
        };
        if value.len() != 6 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
            return false;
        }
        self.offset += 7;
        self.push(
            TokenKind::Identifier(self.source.text[start..self.offset].to_string()),
            start,
            self.offset,
        );
        true
    }

    fn current(&self) -> char {
        self.rest().chars().next().unwrap()
    }

    fn rest(&self) -> &str {
        &self.source.text[self.offset..]
    }

    fn advance(&mut self) {
        self.offset += self.current().len_utf8();
    }

    fn directive_position(&self) -> bool {
        self.tokens
            .last()
            .is_none_or(|token| matches!(token.kind, TokenKind::Newline))
    }
}

fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | '/' | ':' | ',')
}

fn expression_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::DoubleEquals
            | TokenKind::NotEquals
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::LeftParen
            | TokenKind::RightParen
    )
}
