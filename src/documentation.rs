use crate::{
    compiler::{
        self, DecisionFormat, DecisionRecord, Directive, DocumentationFormat, DocumentationOwner,
        DocumentationSection, ElementKind, Workspace,
    },
    diagnostic::{render_all, Diagnostic},
    source::SourceMap,
};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone)]
struct ImportRequest {
    owner: DocumentationOwner,
    directive: Directive,
}

pub fn import(workspace: &mut Workspace, strict_safe: bool) -> Result<(), String> {
    let mut requests = workspace
        .directives
        .iter()
        .filter(|directive| matches!(directive.name.as_str(), "docs" | "adrs"))
        .cloned()
        .map(|directive| ImportRequest {
            owner: DocumentationOwner::Workspace,
            directive,
        })
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    for element in &workspace.elements {
        for directive in element
            .directives
            .iter()
            .filter(|directive| matches!(directive.name.as_str(), "docs" | "adrs"))
        {
            if matches!(
                element.kind,
                ElementKind::SoftwareSystem | ElementKind::Container
            ) {
                requests.push(ImportRequest {
                    owner: DocumentationOwner::Element(element.id.clone()),
                    directive: directive.clone(),
                });
            } else {
                warnings.push(
                    Diagnostic::warning(
                        directive.span,
                        format!(
                            "!{} is not supported for {} elements; metadata was preserved",
                            directive.name,
                            element_kind(&element.kind)
                        ),
                    )
                    .with_help(
                        "attach documentation to the workspace, a software system, or a container",
                    ),
                );
            }
        }
    }

    let sources = workspace.source_map.clone();
    let mut diagnostics = Vec::new();
    let mut documentation = Vec::new();
    let mut decisions = Vec::new();
    for request in requests {
        if request.directive.arguments.is_empty() || request.directive.arguments.len() > 2 {
            diagnostics.push(
                Diagnostic::error(
                    request.directive.span,
                    format!(
                        "!{} requires a path and optional importer",
                        request.directive.name
                    ),
                )
                .with_help(format!(
                    "use '!{} <path> [importer]'",
                    request.directive.name
                )),
            );
            continue;
        }
        let path = unquote(&request.directive.arguments[0]);
        if request.directive.name == "docs" {
            if let Some(importer) = request.directive.arguments.get(1) {
                let message = format!(
                    "custom documentation importer '{}' was preserved but not loaded",
                    unquote(importer)
                );
                if strict_safe {
                    diagnostics.push(Diagnostic::error(request.directive.span, message));
                    continue;
                }
                warnings.push(Diagnostic::warning(request.directive.span, message));
            }
            match import_docs(&sources, &request, path, documentation.len()) {
                Ok((mut imported, mut imported_warnings)) => {
                    documentation.append(&mut imported);
                    warnings.append(&mut imported_warnings);
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        } else {
            let importer = request
                .directive
                .arguments
                .get(1)
                .map(|value| unquote(value))
                .unwrap_or("adrtools");
            let format = match importer.to_ascii_lowercase().as_str() {
                "adrtools" => DecisionFormat::AdrTools,
                "madr" => {
                    warnings.push(Diagnostic::warning(
                        request.directive.span,
                        "MADR metadata import is partial; Markdown content was imported safely",
                    ));
                    DecisionFormat::Madr
                }
                "log4brains" => {
                    warnings.push(Diagnostic::warning(
                        request.directive.span,
                        "Log4brains metadata import is partial; Markdown content was imported safely",
                    ));
                    DecisionFormat::Log4brains
                }
                custom if custom.contains('.') => {
                    let message = format!(
                        "custom ADR importer '{}' was preserved but not loaded",
                        importer
                    );
                    if strict_safe {
                        diagnostics.push(Diagnostic::error(request.directive.span, message));
                        continue;
                    }
                    warnings.push(Diagnostic::warning(request.directive.span, message));
                    DecisionFormat::MarkdownGeneric
                }
                _ => {
                    diagnostics.push(
                        Diagnostic::error(
                            request.directive.span,
                            format!("unsupported ADR importer '{importer}'"),
                        )
                        .with_help("use adrtools, madr, log4brains, or omit the importer"),
                    );
                    continue;
                }
            };
            match import_adrs(&sources, &request, path, format, decisions.len()) {
                Ok((mut imported, warning)) => {
                    decisions.append(&mut imported);
                    if let Some(warning) = warning {
                        warnings.push(warning);
                    }
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
    }
    workspace.warnings.extend(warnings);
    workspace.documentation = documentation;
    workspace.decisions = decisions;
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(render_all(&diagnostics, &workspace.source_map))
    }
}

fn import_docs(
    sources: &SourceMap,
    request: &ImportRequest,
    value: &str,
    order: usize,
) -> Result<(Vec<DocumentationSection>, Vec<Diagnostic>), Diagnostic> {
    let (target, display) = resolve_path(sources, &request.directive, value, "documentation")?;
    let files = document_files(&target, &request.directive)?;
    let directory = target.is_dir();
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    if files.is_empty() {
        warnings.push(Diagnostic::warning(
            request.directive.span,
            "documentation directory contains no supported files",
        ));
    }
    for file in files {
        let content = read_utf8(&file, &request.directive, "documentation")?;
        let format = documentation_format(&file).ok_or_else(|| {
            Diagnostic::error(
                request.directive.span,
                format!(
                    "unsupported documentation file '{}'; expected Markdown or AsciiDoc",
                    file.display()
                ),
            )
        })?;
        if format == DocumentationFormat::AsciiDoc {
            warnings.push(Diagnostic::warning(
                request.directive.span,
                format!(
                    "AsciiDoc '{}' is preserved and rendered as escaped text",
                    file.display()
                ),
            ));
        }
        sections.push(DocumentationSection {
            owner: request.owner.clone(),
            source_path: display_path(&display, &file, directory),
            title: document_title(&content, &file, &format),
            format,
            order: order + sections.len(),
            content,
        });
    }
    Ok((sections, warnings))
}

fn import_adrs(
    sources: &SourceMap,
    request: &ImportRequest,
    value: &str,
    format: DecisionFormat,
    order: usize,
) -> Result<(Vec<DecisionRecord>, Option<Diagnostic>), Diagnostic> {
    let (target, display) = resolve_path(sources, &request.directive, value, "ADR")?;
    let files = markdown_files(&target, &request.directive, "ADR")?;
    let directory = target.is_dir();
    let warning = files.is_empty().then(|| {
        Diagnostic::warning(
            request.directive.span,
            "ADR directory contains no Markdown files",
        )
    });
    let mut records = Vec::new();
    for file in files {
        let content = read_utf8(&file, &request.directive, "ADR")?;
        records.push(DecisionRecord {
            owner: request.owner.clone(),
            id: decision_id(&file),
            title: first_heading(&content, "# ").unwrap_or_else(|| fallback_title(&file)),
            status: section_value(&content, "Status")
                .or_else(|| labeled_value(&content, "Status:")),
            date: labeled_value(&content, "Date:").or_else(|| section_value(&content, "Date")),
            source_path: display_path(&display, &file, directory),
            content,
            format: format.clone(),
            order: order + records.len(),
        });
    }
    Ok((records, warning))
}

fn resolve_path(
    sources: &SourceMap,
    directive: &Directive,
    value: &str,
    label: &str,
) -> Result<(PathBuf, PathBuf), Diagnostic> {
    if is_url(value) {
        return Err(Diagnostic::error(
            directive.span,
            format!("remote {label} import is disabled; no network request was made"),
        ));
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(Diagnostic::error(
            directive.span,
            format!("absolute {label} paths are not allowed"),
        ));
    }
    if relative
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(Diagnostic::error(
            directive.span,
            format!("{label} path may not escape the DSL directory with '..'"),
        ));
    }
    let (source, _) = sources.resolve(directive.span);
    let base = source.path.parent().unwrap_or(Path::new("."));
    let canonical_base = fs::canonicalize(base).map_err(|error| {
        Diagnostic::error(
            directive.span,
            format!("cannot resolve DSL directory '{}': {error}", base.display()),
        )
    })?;
    let target = fs::canonicalize(base.join(relative)).map_err(|error| {
        Diagnostic::error(
            directive.span,
            format!("{label} path '{}' cannot be read: {error}", value),
        )
        .with_help("use an existing local path below the DSL file directory")
    })?;
    if !target.starts_with(&canonical_base) {
        return Err(Diagnostic::error(
            directive.span,
            format!("{label} path escapes the DSL file directory"),
        ));
    }
    Ok((target, relative.to_path_buf()))
}

fn document_files(target: &Path, directive: &Directive) -> Result<Vec<PathBuf>, Diagnostic> {
    if target.is_file() {
        if documentation_format(target).is_none() {
            return Err(Diagnostic::error(
                directive.span,
                format!(
                    "unsupported documentation extension for '{}'",
                    target.display()
                ),
            ));
        }
        return Ok(vec![target.to_path_buf()]);
    }
    directory_files(target, directive, |path| {
        documentation_format(path).is_some()
    })
}

fn markdown_files(
    target: &Path,
    directive: &Directive,
    label: &str,
) -> Result<Vec<PathBuf>, Diagnostic> {
    if target.is_file() {
        if !is_markdown(target) {
            return Err(Diagnostic::error(
                directive.span,
                format!("unsupported {label} extension for '{}'", target.display()),
            ));
        }
        return Ok(vec![target.to_path_buf()]);
    }
    directory_files(target, directive, is_markdown)
}

fn directory_files(
    target: &Path,
    directive: &Directive,
    include: impl Fn(&Path) -> bool,
) -> Result<Vec<PathBuf>, Diagnostic> {
    if !target.is_dir() {
        return Err(Diagnostic::error(
            directive.span,
            format!(
                "import path '{}' is neither a file nor directory",
                target.display()
            ),
        ));
    }
    let entries = fs::read_dir(target).map_err(|error| {
        Diagnostic::error(
            directive.span,
            format!("cannot read directory '{}': {error}", target.display()),
        )
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| Diagnostic::error(directive.span, error.to_string()))?
            .path();
        if path.is_file() && include(&path) {
            files.push(path);
        }
    }
    files.sort_by_key(|path| normalized(path));
    Ok(files)
}

fn read_utf8(path: &Path, directive: &Directive, label: &str) -> Result<String, Diagnostic> {
    let bytes = fs::read(path).map_err(|error| {
        Diagnostic::error(
            directive.span,
            format!("cannot read {label} file '{}': {error}", path.display()),
        )
    })?;
    String::from_utf8(bytes).map_err(|_| {
        Diagnostic::error(
            directive.span,
            format!("{label} file '{}' is not valid UTF-8", path.display()),
        )
    })
}

fn documentation_format(path: &Path) -> Option<DocumentationFormat> {
    match extension(path).as_str() {
        "md" | "markdown" => Some(DocumentationFormat::Markdown),
        "adoc" | "asciidoc" => Some(DocumentationFormat::AsciiDoc),
        _ => None,
    }
}

fn is_markdown(path: &Path) -> bool {
    matches!(extension(path).as_str(), "md" | "markdown")
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn document_title(content: &str, path: &Path, format: &DocumentationFormat) -> String {
    let prefix = if *format == DocumentationFormat::Markdown {
        "# "
    } else {
        "= "
    };
    first_heading(content, prefix).unwrap_or_else(|| fallback_title(path))
}

fn first_heading(content: &str, prefix: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
}

fn fallback_title(path: &Path) -> String {
    let value = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Document")
        .replace(['-', '_'], " ");
    let mut characters = value.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
        .unwrap_or_else(|| "Document".into())
}

fn section_value(content: &str, heading: &str) -> Option<String> {
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        if line
            .trim()
            .trim_start_matches('#')
            .trim()
            .eq_ignore_ascii_case(heading)
        {
            return lines
                .by_ref()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with('#'))
                .map(clean_metadata);
        }
    }
    None
}

fn labeled_value(content: &str, label: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix(label)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(clean_metadata)
    })
}

fn clean_metadata(value: &str) -> String {
    value.trim_matches(['*', '_', '`']).trim().to_string()
}

fn decision_id(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("ADR");
    let lower = stem.to_ascii_lowercase();
    let candidate = lower.strip_prefix("adr-").unwrap_or(&lower);
    let digits = candidate
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        stem.to_ascii_uppercase()
    } else {
        format!("ADR-{digits}")
    }
}

fn display_path(base: &Path, file: &Path, directory: bool) -> PathBuf {
    if directory {
        base.join(file.file_name().unwrap_or_default())
    } else {
        base.to_path_buf()
    }
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

fn element_kind(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::Component => "component",
        ElementKind::Generic => "generic",
        ElementKind::DeploymentEnvironment
        | ElementKind::DeploymentGroup
        | ElementKind::DeploymentNode
        | ElementKind::InfrastructureNode
        | ElementKind::SoftwareSystemInstance
        | ElementKind::ContainerInstance => "deployment",
        ElementKind::ArchiMate(_) => "archimate",
        ElementKind::SoftwareSystem => "software system",
        ElementKind::Container => "container",
    }
}

pub fn export_site(workspace: &Workspace, output: &Path) -> Result<(), String> {
    let assets = output.join("assets");
    let diagrams = output.join("diagrams");
    let docs = output.join("docs");
    let decisions = output.join("decisions");
    for directory in [&assets, &diagrams, &docs, &decisions] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    }
    write(
        &assets.join("style.css"),
        "body{font:16px system-ui,sans-serif;max-width:960px;margin:auto;padding:2rem;color:#222}nav a{margin-right:1rem}pre{overflow:auto;padding:1rem;background:#f4f4f4}code{white-space:pre-wrap}table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:.4rem}a{color:#075ea8}\n",
    )?;

    let mut diagram_html = String::new();
    for view in &workspace.views {
        let key = view
            .key
            .as_deref()
            .unwrap_or_else(|| compiler::default_view_key(&view.kind));
        let file = format!("{}.mmd", safe_name(key));
        let source = compiler::mermaid(workspace, view);
        write(&diagrams.join(&file), &source)?;
        diagram_html.push_str(&format!(
            "<section><h2>{}</h2><p><a href=\"diagrams/{file}\">Raw Mermaid source</a></p><pre><code>{}</code></pre></section>",
            html_escape(key),
            html_escape(&source)
        ));
    }

    let docs_index = documentation_pages(workspace, &docs)?;
    let decisions_index = decision_pages(workspace, &decisions)?;
    let mut elements = String::new();
    for element in &workspace.elements {
        elements.push_str(&format!(
            "<li><code>{}</code> — {}</li>",
            html_escape(&element.id),
            html_escape(&element.name)
        ));
    }
    let body = format!(
        "<nav><a href=\"docs/index.html\">Documentation</a><a href=\"decisions/index.html\">ADRs</a></nav><h1>{}</h1><p>{}</p><h2>Model summary</h2><p>{} elements, {} relationships, {} views.</p><ul>{elements}</ul><h2>Diagrams</h2>{diagram_html}<p>{} documentation sections; {} decisions.</p>",
        html_escape(workspace.name.as_deref().unwrap_or("Workspace")),
        html_escape(workspace.description.as_deref().unwrap_or("")),
        workspace.elements.len(),
        workspace.relationships.len(),
        workspace.views.len(),
        workspace.documentation.len(),
        workspace.decisions.len()
    );
    write(
        &output.join("index.html"),
        &page("Workspace", "assets/style.css", &body),
    )?;
    write(
        &docs.join("index.html"),
        &page("Documentation", "../assets/style.css", &docs_index),
    )?;
    write(
        &decisions.join("index.html"),
        &page(
            "Architecture decisions",
            "../assets/style.css",
            &decisions_index,
        ),
    )?;
    Ok(())
}

fn documentation_pages(workspace: &Workspace, root: &Path) -> Result<String, String> {
    let mut index = String::from(
        "<nav><a href=\"../index.html\">Workspace</a></nav><h1>Documentation</h1><ul>",
    );
    for section in &workspace.documentation {
        let (directory, relative, css) = match &section.owner {
            DocumentationOwner::Workspace => (
                root.join("workspace"),
                "workspace".to_string(),
                "../../assets/style.css",
            ),
            DocumentationOwner::Element(identifier) => {
                let relative = format!("elements/{}", safe_name(identifier));
                (root.join(&relative), relative, "../../../assets/style.css")
            }
        };
        fs::create_dir_all(&directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        let file = format!(
            "{:04}-{}.html",
            section.order + 1,
            safe_name(&section.title)
        );
        let owner = owner_label(&section.owner);
        let content = render_document(&section.content, &section.format);
        let body = format!(
            "<nav><a href=\"../{}index.html\">Documentation</a></nav><h1>{}</h1><p>Owner: {}; source: <code>{}</code></p>{content}",
            if matches!(section.owner, DocumentationOwner::Workspace) { "" } else { "../" },
            html_escape(&section.title),
            html_escape(&owner),
            html_escape(&normalized(&section.source_path))
        );
        write(&directory.join(&file), &page(&section.title, css, &body))?;
        index.push_str(&format!(
            "<li><a href=\"{relative}/{file}\">{}</a> — {}</li>",
            html_escape(&section.title),
            html_escape(&owner)
        ));
    }
    index.push_str("</ul>");
    Ok(index)
}

fn decision_pages(workspace: &Workspace, root: &Path) -> Result<String, String> {
    let mut index = String::from(
        "<nav><a href=\"../index.html\">Workspace</a></nav><h1>Architecture decisions</h1><ul>",
    );
    for decision in &workspace.decisions {
        let file = format!("{:04}-{}.html", decision.order + 1, safe_name(&decision.id));
        let owner = owner_label(&decision.owner);
        let body = format!(
            "<nav><a href=\"index.html\">Architecture decisions</a></nav><h1>{}: {}</h1><p>Status: {}; date: {}; owner: {}; format: {:?}; source: <code>{}</code></p>{}",
            html_escape(&decision.id),
            html_escape(&decision.title),
            html_escape(decision.status.as_deref().unwrap_or("-")),
            html_escape(decision.date.as_deref().unwrap_or("-")),
            html_escape(&owner),
            decision.format,
            html_escape(&normalized(&decision.source_path)),
            render_markdown(&decision.content)
        );
        write(
            &root.join(&file),
            &page(&decision.title, "../assets/style.css", &body),
        )?;
        index.push_str(&format!(
            "<li><a href=\"{file}\">{}: {}</a> — {} — {}</li>",
            html_escape(&decision.id),
            html_escape(&decision.title),
            html_escape(decision.status.as_deref().unwrap_or("-")),
            html_escape(&owner)
        ));
    }
    index.push_str("</ul>");
    Ok(index)
}

pub fn adr_list(workspace: &Workspace) -> String {
    if workspace.decisions.is_empty() {
        return "No ADRs found.\n".into();
    }
    let mut output = String::new();
    for decision in &workspace.decisions {
        output.push_str(&format!(
            "{} | {} | {} | {} | {} | {}\n",
            decision.id,
            decision.status.as_deref().unwrap_or("-"),
            decision.date.as_deref().unwrap_or("-"),
            decision.title,
            owner_label(&decision.owner),
            normalized(&decision.source_path)
        ));
    }
    output
}

fn render_document(content: &str, format: &DocumentationFormat) -> String {
    match format {
        DocumentationFormat::Markdown => render_markdown(content),
        DocumentationFormat::AsciiDoc => {
            format!("<pre><code>{}</code></pre>", html_escape(content))
        }
    }
}

fn render_markdown(content: &str) -> String {
    let mut output = String::new();
    let mut code = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(language) = trimmed.strip_prefix("```") {
            if code {
                output.push_str("</code></pre>");
            } else {
                output.push_str(&format!(
                    "<pre data-language=\"{}\"><code>",
                    html_escape(language)
                ));
            }
            code = !code;
        } else if code {
            output.push_str(&html_escape(line));
            output.push('\n');
        } else if let Some(title) = trimmed.strip_prefix("### ") {
            output.push_str(&format!("<h3>{}</h3>", html_escape(title)));
        } else if let Some(title) = trimmed.strip_prefix("## ") {
            output.push_str(&format!("<h2>{}</h2>", html_escape(title)));
        } else if let Some(title) = trimmed.strip_prefix("# ") {
            output.push_str(&format!("<h1>{}</h1>", html_escape(title)));
        } else if !trimmed.is_empty() {
            output.push_str(&format!("<p>{}</p>", html_escape(trimmed)));
        }
    }
    if code {
        output.push_str("</code></pre>");
    }
    output
}

fn owner_label(owner: &DocumentationOwner) -> String {
    match owner {
        DocumentationOwner::Workspace => "workspace".into(),
        DocumentationOwner::Element(identifier) => identifier.clone(),
    }
}

fn safe_name(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while output.contains("--") {
        output = output.replace("--", "-");
    }
    let output = output.trim_matches('-');
    if output.is_empty() {
        "item".into()
    } else {
        output.into()
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn page(title: &str, css: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"{css}\"></head><body>{body}</body></html>\n",
        html_escape(title)
    )
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{
        compile, compile_file, compile_file_with_options, validate, CompileOptions,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn imports_m7_documentation_and_adrs() {
        let workspace = compile_file("tests/fixtures/m7-docs.dsl").unwrap();
        validate(&workspace).unwrap();
        assert_eq!(workspace.documentation.len(), 4);
        assert_eq!(workspace.documentation[0].title, "Workspace overview");
        assert_eq!(workspace.documentation[1].title, "02 operating notes");
        assert_eq!(
            workspace.documentation[2].owner,
            DocumentationOwner::Element("bank".into())
        );
        assert_eq!(
            workspace.documentation[3].format,
            DocumentationFormat::AsciiDoc
        );
        assert_eq!(workspace.decisions.len(), 5);
        assert_eq!(workspace.decisions[0].id, "ADR-0001");
        assert_eq!(workspace.decisions[0].status.as_deref(), Some("Accepted"));
        assert_eq!(workspace.decisions[0].date.as_deref(), Some("2026-01-10"));
        assert_eq!(
            workspace.decisions[2].owner,
            DocumentationOwner::Element("bank".into())
        );
        assert_eq!(workspace.decisions[2].format, DecisionFormat::Madr);
        assert_eq!(workspace.decisions[4].format, DecisionFormat::Log4brains);
    }

    #[test]
    fn generates_deterministic_escaped_m7_site() {
        let workspace = compile_file("tests/fixtures/m7-docs.dsl").unwrap();
        let output = temporary_directory("site");
        export_site(&workspace, &output).unwrap();
        let first = fs::read_to_string(output.join("index.html")).unwrap();
        export_site(&workspace, &output).unwrap();
        assert_eq!(
            first,
            fs::read_to_string(output.join("index.html")).unwrap()
        );
        for path in [
            "assets/style.css",
            "diagrams/context.mmd",
            "diagrams/containers.mmd",
            "docs/index.html",
            "docs/workspace/0001-workspace-overview.html",
            "decisions/index.html",
            "decisions/0001-adr-0001.html",
        ] {
            assert!(output.join(path).is_file(), "missing {path}");
        }
        let page =
            fs::read_to_string(output.join("docs/workspace/0001-workspace-overview.html")).unwrap();
        assert!(page.contains("&lt;script&gt;alert(&quot;unsafe&quot;)&lt;/script&gt;"));
        assert!(!page.contains("<script>"));
        assert!(first.contains("diagrams/context.mmd"));
        fs::remove_dir_all(output).unwrap();
    }

    #[test]
    fn rejects_unsafe_m7_imports_and_custom_importers_in_strict_mode() {
        let error = compile_file("tests/fixtures/m7-invalid-paths.dsl").unwrap_err();
        for message in [
            "cannot be read",
            "may not escape",
            "absolute documentation paths",
            "remote documentation import is disabled",
            "unsupported documentation extension",
            "absolute ADR paths",
            "remote ADR import is disabled",
        ] {
            assert!(error.contains(message), "missing '{message}' in {error}");
        }

        let custom = compile_file("tests/fixtures/m7-custom-importer.dsl").unwrap();
        assert_eq!(custom.documentation.len(), 2);
        assert_eq!(custom.decisions.len(), 2);
        assert!(crate::compiler::warnings(&custom)
            .unwrap()
            .contains("preserved but not loaded"));
        let strict = compile_file_with_options(
            "tests/fixtures/m7-custom-importer.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
            },
        )
        .unwrap_err();
        assert!(strict.contains("custom documentation importer"));
        assert!(strict.contains("custom ADR importer"));
        compile_file_with_options(
            "tests/fixtures/m7-docs.dsl",
            CompileOptions {
                allow_network: false,
                strict_safe: true,
            },
        )
        .unwrap();

        let unsupported = compile("workspace {\n  model {\n    !docs docs\n  }\n}\n").unwrap_err();
        assert!(unsupported.contains("is not allowed directly in the model"));
    }

    #[test]
    fn rejects_non_utf8_m7_content_and_lists_adrs() {
        let directory = temporary_directory("utf8");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("bad.md"), [0xff]).unwrap();
        fs::write(
            directory.join("workspace.dsl"),
            "workspace {\n  !docs bad.md\n}\n",
        )
        .unwrap();
        let error = compile_file(&directory.join("workspace.dsl").to_string_lossy()).unwrap_err();
        assert!(error.contains("not valid UTF-8"));
        fs::remove_dir_all(directory).unwrap();

        let workspace = compile_file("tests/fixtures/m7-docs.dsl").unwrap();
        let list = adr_list(&workspace);
        assert!(list.starts_with(
            "ADR-0001 | Accepted | 2026-01-10 | Use PostgreSQL for audit storage | workspace"
        ));
        assert!(list.contains("ADR-0003 | Accepted | 2026-01-14 | Define the API boundary | bank"));
        assert_eq!(
            adr_list(&compile_file("examples/internet-banking.dsl").unwrap()),
            "No ADRs found.\n"
        );
    }

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "c4c-m7-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
