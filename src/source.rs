use std::{fs, path::PathBuf};

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

    pub fn add_file(&mut self, path: &str) -> Result<SourceId, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        Ok(self.add_text(path, text))
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
}
