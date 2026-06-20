use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub source_id: SourceId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(source_id: SourceId, start: usize, end: usize) -> Self {
        Self {
            source_id,
            start,
            end,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        debug_assert_eq!(self.source_id, other.source_id);
        Self::new(
            self.source_id,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub text: String,
    line_starts: Vec<usize>,
    mappings: Vec<SourceSegment>,
}

#[derive(Debug, Clone)]
pub struct SourceSegment {
    pub generated_start: usize,
    pub generated_end: usize,
    pub original: Span,
}

impl SourceFile {
    fn new(id: SourceId, path: impl Into<PathBuf>, text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(offset, _)| offset + 1));
        Self {
            id,
            path: path.into(),
            text,
            line_starts,
            mappings: Vec::new(),
        }
    }

    pub fn line_column(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let column = self.text[self.line_starts[line_index]..offset]
            .chars()
            .count()
            + 1;
        (line_index + 1, column)
    }

    pub fn line(&self, line: usize) -> &str {
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        self.text[start..end].trim_end_matches(['\r', '\n'])
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.line_starts[line - 1]
    }
}

#[derive(Debug, Clone)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_text(&mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> SourceId {
        let id = SourceId(self.files.len());
        self.files.push(SourceFile::new(id, path, text.into()));
        id
    }

    pub fn add_generated(
        &mut self,
        path: impl Into<PathBuf>,
        text: String,
        mappings: Vec<SourceSegment>,
    ) -> SourceId {
        let id = self.add_text(path, text);
        self.files[id.0].mappings = mappings;
        id
    }

    #[cfg(test)]
    pub fn from_text(path: impl Into<PathBuf>, text: impl Into<String>) -> (Self, SourceId) {
        let mut sources = Self::new();
        let id = sources.add_text(path, text);
        (sources, id)
    }

    pub fn get(&self, id: SourceId) -> &SourceFile {
        &self.files[id.0]
    }

    pub fn resolve(&self, span: Span) -> (&SourceFile, Span) {
        let generated = self.get(span.source_id);
        let Some(segment) = generated
            .mappings
            .iter()
            .find(|segment| {
                span.start >= segment.generated_start && span.start < segment.generated_end
            })
            .or_else(|| {
                (span.start == generated.text.len())
                    .then(|| generated.mappings.last())
                    .flatten()
            })
        else {
            return (generated, span);
        };
        let offset = span.start - segment.generated_start;
        let start = (segment.original.start + offset).min(segment.original.end.saturating_sub(1));
        let width = span.end.saturating_sub(span.start).max(1);
        let end = (start + width).min(segment.original.end).max(start + 1);
        (
            self.get(segment.original.source_id),
            Span::new(segment.original.source_id, start, end),
        )
    }
}
