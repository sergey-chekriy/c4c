use crate::{
    diagnostic::Diagnostic,
    source::{SourceId, SourceMap, SourceSegment, Span},
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct IncludeDependency {
    pub from: PathBuf,
    pub to: PathBuf,
}

pub struct Preprocessed {
    pub source_id: SourceId,
    pub dependencies: Vec<IncludeDependency>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Clone)]
struct Constant {
    value: String,
    span: Span,
}

pub fn file(
    path: &Path,
    sources: &mut SourceMap,
    allow_network: bool,
    strict_safe: bool,
) -> Result<Preprocessed, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let bytes = fs::read(&canonical)
        .map_err(|error| format!("cannot read {}: {error}", canonical.display()))?;
    let value = String::from_utf8(bytes)
        .map_err(|_| format!("{} is not valid UTF-8", canonical.display()))?;
    let source_id = sources.add_text(&canonical, value);
    let mut context = Context::new(sources, allow_network, strict_safe);
    context.stack.push(canonical.clone());
    if let Err(diagnostic) = context.process_source(&canonical, source_id) {
        return Err(diagnostic.render(context.sources));
    }
    context.stack.pop();
    context.finish(&canonical)
}

#[cfg(test)]
pub fn text(
    path: &Path,
    value: &str,
    sources: &mut SourceMap,
    strict_safe: bool,
) -> Result<Preprocessed, String> {
    let source_id = sources.add_text(path, value);
    let mut context = Context::new(sources, false, strict_safe);
    if let Err(diagnostic) = context.process_source(path, source_id) {
        return Err(diagnostic.render(context.sources));
    }
    context.finish(path)
}

struct Context<'a> {
    sources: &'a mut SourceMap,
    allow_network: bool,
    strict_safe: bool,
    constants: HashMap<String, Constant>,
    constant_order: Vec<String>,
    stack: Vec<PathBuf>,
    dependencies: Vec<IncludeDependency>,
    warnings: Vec<Diagnostic>,
    output: String,
    mappings: Vec<SourceSegment>,
}

impl<'a> Context<'a> {
    fn new(sources: &'a mut SourceMap, allow_network: bool, strict_safe: bool) -> Self {
        Self {
            sources,
            allow_network,
            strict_safe,
            constants: HashMap::new(),
            constant_order: Vec::new(),
            stack: Vec::new(),
            dependencies: Vec::new(),
            warnings: Vec::new(),
            output: String::new(),
            mappings: Vec::new(),
        }
    }

    fn finish(self, path: &Path) -> Result<Preprocessed, String> {
        for name in &self.constant_order {
            let span = self.constants[name].span;
            if let Err(diagnostic) = self.resolve_constant(name, span, &mut Vec::new()) {
                return Err(diagnostic.render(self.sources));
            }
        }
        let source_id = self.sources.add_generated(
            format!("{}#preprocessed", path.display()),
            self.output,
            self.mappings,
        );
        Ok(Preprocessed {
            source_id,
            dependencies: self.dependencies,
            warnings: self.warnings,
        })
    }

    fn process_file(&mut self, path: &Path, include_span: Option<Span>) -> Result<(), Diagnostic> {
        let canonical = fs::canonicalize(path).map_err(|error| {
            self.include_error(
                include_span,
                format!("cannot include '{}': {error}", path.display()),
                "check that the local include path exists",
            )
        })?;
        if self.stack.contains(&canonical) {
            return Err(self.include_error(
                include_span,
                format!("include cycle detected at {}", canonical.display()),
                "remove one include from the cycle",
            ));
        }
        let bytes = fs::read(&canonical).map_err(|error| {
            self.include_error(
                include_span,
                format!("cannot read include '{}': {error}", canonical.display()),
                "check file permissions and the include path",
            )
        })?;
        let value = String::from_utf8(bytes).map_err(|_| {
            self.include_error(
                include_span,
                format!("include '{}' is not valid UTF-8", canonical.display()),
                "convert the included DSL file to UTF-8",
            )
        })?;
        let source_id = self.sources.add_text(&canonical, value);
        self.stack.push(canonical.clone());
        let result = self.process_source(&canonical, source_id);
        self.stack.pop();
        result
    }

    fn process_source(&mut self, path: &Path, source_id: SourceId) -> Result<(), Diagnostic> {
        let text = self.sources.get(source_id).text.clone();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();
            let statement = Span::new(source_id, offset + leading, offset + line.len());
            if let Some(rest) = directive(trimmed, "!constant") {
                self.constant(rest, statement)?;
                if line.ends_with('\n') {
                    self.append(
                        "\n",
                        Span::new(source_id, offset + line.len() - 1, offset + line.len()),
                    );
                }
            } else if let Some(rest) = directive(trimmed, "!include") {
                self.include(path, rest, statement)?;
            } else {
                if self.strict_safe
                    && (directive(trimmed, "!script").is_some()
                        || directive(trimmed, "!plugin").is_some())
                {
                    return Err(Diagnostic::error(
                        statement,
                        "strict-safe mode rejects scripts and plugins",
                    )
                    .with_help("remove the executable directive; it was not executed"));
                }
                let expanded = self.substitute(line, statement)?;
                self.append(&expanded, Span::new(source_id, offset, offset + line.len()));
            }
            offset += line.len();
        }
        Ok(())
    }

    fn constant(&mut self, rest: &str, span: Span) -> Result<(), Diagnostic> {
        let rest = rest.trim();
        let split = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let name = &rest[..split];
        let value = rest[split..].trim().trim_end_matches(['\r', '\n']);
        if name.is_empty() || value.is_empty() {
            return Err(
                Diagnostic::error(span, "!constant requires a name and value")
                    .with_help("use '!constant NAME value'"),
            );
        }
        if let Some(original) = self.constants.get(name) {
            let source = self.sources.get(original.span.source_id);
            let (line, column) = source.line_column(original.span.start);
            return Err(
                Diagnostic::error(span, format!("duplicate constant '{name}'")).with_help(format!(
                    "the constant was first defined at {}:{line}:{column}",
                    source.path.display()
                )),
            );
        }
        self.constant_order.push(name.into());
        self.constants.insert(
            name.into(),
            Constant {
                value: unquote(value).into(),
                span,
            },
        );
        Ok(())
    }

    fn include(&mut self, including: &Path, rest: &str, span: Span) -> Result<(), Diagnostic> {
        let target = unquote(self.substitute(rest.trim(), span)?.trim()).to_string();
        if target.is_empty() {
            return Err(Diagnostic::error(span, "!include requires a path or URL")
                .with_help("add a local file or directory after !include"));
        }
        if is_url(&target) {
            let message = if self.allow_network {
                "remote include fetching is not implemented; no network request was made"
            } else {
                "remote include is disabled; no network request was made"
            };
            return Err(Diagnostic::error(span, message)
                .with_help("use a local include path; --allow-network does not fetch URLs in M6"));
        }
        let parent = including.parent().unwrap_or(Path::new("."));
        let target = parent.join(target);
        if target.is_dir() {
            let entries = fs::read_dir(&target).map_err(|error| {
                Diagnostic::error(
                    span,
                    format!(
                        "cannot read include directory '{}': {error}",
                        target.display()
                    ),
                )
                .with_help("check directory permissions and the include path")
            })?;
            let mut files = Vec::new();
            for entry in entries {
                let path = entry
                    .map_err(|error| {
                        Diagnostic::error(
                            span,
                            format!(
                                "cannot read entry in include directory '{}': {error}",
                                target.display()
                            ),
                        )
                    })?
                    .path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "dsl") {
                    files.push(path);
                }
            }
            files.sort_by_key(|path| path.to_string_lossy().replace('\\', "/"));
            if files.is_empty() {
                self.warnings.push(
                    Diagnostic::warning(span, "include directory contains no .dsl files")
                        .with_help("add a .dsl file or remove the empty directory include"),
                );
            }
            for file in files {
                self.include_file(including, &file, span)?;
            }
        } else {
            self.include_file(including, &target, span)?;
        }
        Ok(())
    }

    fn include_file(
        &mut self,
        including: &Path,
        target: &Path,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let canonical = fs::canonicalize(target).map_err(|error| {
            Diagnostic::error(
                span,
                format!("cannot include '{}': {error}", target.display()),
            )
            .with_help("check that the local include path exists")
        })?;
        self.dependencies.push(IncludeDependency {
            from: including.to_path_buf(),
            to: canonical.clone(),
        });
        self.process_file(&canonical, Some(span))?;
        if !self.output.ends_with('\n') {
            self.append("\n", span);
        }
        Ok(())
    }

    fn substitute(&self, value: &str, span: Span) -> Result<String, Diagnostic> {
        self.substitute_with_stack(value, span, &mut Vec::new())
    }

    fn substitute_with_stack(
        &self,
        value: &str,
        span: Span,
        stack: &mut Vec<String>,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        let mut rest = value;
        while let Some(start) = rest.find("${") {
            output.push_str(&rest[..start]);
            let Some(end) = rest[start + 2..].find('}') else {
                return Err(
                    Diagnostic::error(span, "unterminated constant substitution")
                        .with_help("close the substitution with '}'"),
                );
            };
            let name = &rest[start + 2..start + 2 + end];
            output.push_str(&self.resolve_constant(name, span, stack)?);
            rest = &rest[start + 3 + end..];
        }
        output.push_str(rest);
        Ok(output)
    }

    fn resolve_constant(
        &self,
        name: &str,
        use_span: Span,
        stack: &mut Vec<String>,
    ) -> Result<String, Diagnostic> {
        let Some(constant) = self.constants.get(name) else {
            return Err(
                Diagnostic::error(use_span, format!("undefined constant '{name}'"))
                    .with_help("define the constant before it is used"),
            );
        };
        if stack.iter().any(|item| item == name) {
            return Err(Diagnostic::error(
                constant.span,
                format!(
                    "recursive constant detected: {} -> {name}",
                    stack.join(" -> ")
                ),
            )
            .with_help("remove the cyclic constant substitution"));
        }
        stack.push(name.into());
        let result = self.substitute_with_stack(&constant.value, constant.span, stack);
        stack.pop();
        result
    }

    fn append(&mut self, value: &str, original: Span) {
        if value.is_empty() {
            return;
        }
        let start = self.output.len();
        self.output.push_str(value);
        self.mappings.push(SourceSegment {
            generated_start: start,
            generated_end: self.output.len(),
            original,
        });
    }

    fn include_error(&self, span: Option<Span>, message: String, help: &str) -> Diagnostic {
        Diagnostic::error(span.unwrap_or(Span::new(SourceId(0), 0, 1)), message).with_help(help)
    }
}

fn directive<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.strip_prefix(name)
        .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
