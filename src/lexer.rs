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
    Arrow,
    Star,
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
                '#' => self.skip_comment(),
                '-' if self.rest().starts_with("->") => {
                    self.offset += 2;
                    self.push(TokenKind::Arrow, start, self.offset);
                }
                '{' => self.single(TokenKind::LeftBrace, start),
                '}' => self.single(TokenKind::RightBrace, start),
                '=' => self.single(TokenKind::Equals, start),
                '*' => self.single(TokenKind::Star, start),
                '"' => {
                    if !self.string(start) {
                        break;
                    }
                }
                '!' => self.bang(start),
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

    fn current(&self) -> char {
        self.rest().chars().next().unwrap()
    }

    fn rest(&self) -> &str {
        &self.source.text[self.offset..]
    }

    fn advance(&mut self) {
        self.offset += self.current().len_utf8();
    }
}

fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | '/')
}
