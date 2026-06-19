use crate::source::{SourceMap, Span};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub help: Option<String>,
    severity: Severity,
}

#[derive(Debug, Clone, Copy)]
enum Severity {
    Error,
    Warning,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            help: None,
            severity: Severity::Error,
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            help: None,
            severity: Severity::Warning,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn render(&self, sources: &SourceMap) -> String {
        let source = sources.get(self.span.source_id);
        let (line, column) = source.line_column(self.span.start);
        let text = source.line(line);
        let line_start = source.line_start(line);
        let marker_start = source.text[line_start..self.span.start.min(line_start + text.len())]
            .chars()
            .count();
        let marker_end = self.span.end.min(line_start + text.len());
        let marker_width = source.text[self.span.start.min(marker_end)..marker_end]
            .chars()
            .count()
            .max(1);
        let number_width = line.to_string().len();
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!(
            "{}:{line}:{column}: {severity}: {}\n  {line:>number_width$} | {text}\n  {:number_width$} | {}{}",
            source.path.display(),
            self.message,
            "",
            " ".repeat(marker_start),
            "^".repeat(marker_width),
        );
        if let Some(help) = &self.help {
            out.push_str(&format!("\nhelp: {help}"));
        }
        out
    }
}

pub fn render_all(diagnostics: &[Diagnostic], sources: &SourceMap) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(sources))
        .collect::<Vec<_>>()
        .join("\n\n")
}
