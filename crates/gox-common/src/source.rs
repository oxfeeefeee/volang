//! Source file management.
//!
//! This module provides types for managing source files, including loading,
//! storing, and accessing source text efficiently.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::span::{BytePos, Span};

/// A unique identifier for a source file within a `SourceMap`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(u32);

impl FileId {
    /// Creates a new file ID.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// A dummy file ID for generated code or unknown sources.
    pub const DUMMY: FileId = FileId(u32::MAX);

    /// Returns true if this is a dummy file ID.
    #[inline]
    pub const fn is_dummy(self) -> bool {
        self.0 == u32::MAX
    }
}

impl fmt::Debug for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dummy() {
            write!(f, "FileId(DUMMY)")
        } else {
            write!(f, "FileId({})", self.0)
        }
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Line and column information for a position in a source file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LineCol {
    /// 1-indexed line number.
    pub line: u32,
    /// 1-indexed column number (in UTF-8 bytes).
    pub column: u32,
}

impl LineCol {
    /// Creates a new line/column position.
    #[inline]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A source file with its content and metadata.
#[derive(Clone)]
pub struct SourceFile {
    /// The file ID.
    id: FileId,
    /// The file name or path.
    name: Arc<str>,
    /// The absolute path to the file, if available.
    path: Option<PathBuf>,
    /// The source text content.
    source: Arc<str>,
    /// Byte offsets of line starts (0-indexed).
    /// The first element is always 0.
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Creates a new source file.
    pub fn new(id: FileId, name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        let name = name.into();
        let source = source.into();
        let line_starts = Self::compute_line_starts(&source);
        
        Self {
            id,
            name,
            path: None,
            source,
            line_starts,
        }
    }

    /// Creates a new source file with a path.
    pub fn with_path(
        id: FileId,
        name: impl Into<Arc<str>>,
        path: impl Into<PathBuf>,
        source: impl Into<Arc<str>>,
    ) -> Self {
        let mut file = Self::new(id, name, source);
        file.path = Some(path.into());
        file
    }

    /// Computes the byte offsets of line starts.
    fn compute_line_starts(source: &str) -> Vec<u32> {
        let mut starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                starts.push((i + 1) as u32);
            }
        }
        starts
    }

    /// Returns the file ID.
    #[inline]
    pub const fn id(&self) -> FileId {
        self.id
    }

    /// Returns the file name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the file path, if available.
    #[inline]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the source text.
    #[inline]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the length of the source in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Returns true if the source is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Returns the number of lines in the file.
    #[inline]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the byte offset of the start of the given line (0-indexed).
    #[inline]
    pub fn line_start(&self, line: usize) -> Option<u32> {
        self.line_starts.get(line).copied()
    }

    /// Returns the byte offset of the end of the given line (0-indexed).
    /// This is the offset of the newline character or end of file.
    pub fn line_end(&self, line: usize) -> Option<u32> {
        if line >= self.line_starts.len() {
            return None;
        }
        
        if line + 1 < self.line_starts.len() {
            // End is start of next line minus the newline character
            Some(self.line_starts[line + 1] - 1)
        } else {
            // Last line ends at end of file
            Some(self.source.len() as u32)
        }
    }

    /// Returns the content of the given line (0-indexed), without the trailing newline.
    pub fn line_content(&self, line: usize) -> Option<&str> {
        let start = self.line_start(line)? as usize;
        let end = self.line_end(line)? as usize;
        Some(&self.source[start..end])
    }

    /// Converts a byte position to line/column.
    pub fn line_col(&self, pos: BytePos) -> LineCol {
        let offset = pos.to_u32();
        
        // Binary search for the line containing this offset
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };
        
        let line_start = self.line_starts[line];
        let column = offset.saturating_sub(line_start) + 1;
        
        LineCol {
            line: (line + 1) as u32,
            column,
        }
    }

    /// Converts a span to start and end line/column positions.
    pub fn span_line_col(&self, span: Span) -> (LineCol, LineCol) {
        (self.line_col(span.start), self.line_col(span.end))
    }

    /// Returns the source text for a given span.
    pub fn span_text(&self, span: Span) -> &str {
        let start = span.start.to_usize().min(self.source.len());
        let end = span.end.to_usize().min(self.source.len());
        &self.source[start..end]
    }

    /// Returns a span covering the entire file.
    pub fn full_span(&self) -> Span {
        Span::from_u32(0, self.source.len() as u32)
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceFile")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("len", &self.source.len())
            .field("lines", &self.line_starts.len())
            .finish()
    }
}

/// A central registry for all source files.
///
/// The `SourceMap` owns all source files and provides efficient access
/// to them via `FileId`.
#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates a new empty source map.
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Adds a source file to the map and returns its ID.
    pub fn add_file(&mut self, name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> FileId {
        let id = FileId::new(self.files.len() as u32);
        let file = SourceFile::new(id, name, source);
        self.files.push(file);
        id
    }

    /// Adds a source file with a path to the map and returns its ID.
    pub fn add_file_with_path(
        &mut self,
        name: impl Into<Arc<str>>,
        path: impl Into<PathBuf>,
        source: impl Into<Arc<str>>,
    ) -> FileId {
        let id = FileId::new(self.files.len() as u32);
        let file = SourceFile::with_path(id, name, path, source);
        self.files.push(file);
        id
    }

    /// Loads a file from disk and adds it to the map.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<FileId> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)?;
        let name = path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        
        Ok(self.add_file_with_path(name, path.to_path_buf(), source))
    }

    /// Returns the source file for the given ID.
    pub fn get_file(&self, id: FileId) -> Option<&SourceFile> {
        if id.is_dummy() {
            return None;
        }
        self.files.get(id.0 as usize)
    }

    /// Returns the number of files in the map.
    #[inline]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns an iterator over all files.
    pub fn files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }

    /// Returns the source text for a file.
    pub fn source(&self, id: FileId) -> Option<&str> {
        self.get_file(id).map(|f| f.source())
    }

    /// Returns the file name for a file.
    pub fn file_name(&self, id: FileId) -> Option<&str> {
        self.get_file(id).map(|f| f.name())
    }

    /// Converts a position to line/column in a file.
    pub fn line_col(&self, id: FileId, pos: BytePos) -> Option<LineCol> {
        self.get_file(id).map(|f| f.line_col(pos))
    }

    /// Returns the source text for a span in a file.
    pub fn span_text(&self, id: FileId, span: Span) -> Option<&str> {
        self.get_file(id).map(|f| f.span_text(span))
    }
}

impl fmt::Debug for SourceMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceMap")
            .field("file_count", &self.files.len())
            .finish()
    }
}

/// A location in a source file, combining file ID and span.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// The file containing this location.
    pub file: FileId,
    /// The span within the file.
    pub span: Span,
}

impl SourceLocation {
    /// Creates a new source location.
    #[inline]
    pub const fn new(file: FileId, span: Span) -> Self {
        Self { file, span }
    }

    /// Creates a dummy source location.
    #[inline]
    pub const fn dummy() -> Self {
        Self {
            file: FileId::DUMMY,
            span: Span::dummy(),
        }
    }

    /// Returns true if this is a dummy location.
    #[inline]
    pub const fn is_dummy(&self) -> bool {
        self.file.is_dummy()
    }

    /// Merges two locations, assuming they are in the same file.
    pub fn merge(self, other: SourceLocation) -> SourceLocation {
        debug_assert_eq!(self.file, other.file, "Cannot merge locations from different files");
        SourceLocation {
            file: self.file,
            span: self.span.merge(other.span),
        }
    }
}

impl fmt::Debug for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:?}", self.file, self.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id() {
        let id = FileId::new(5);
        assert_eq!(id.as_u32(), 5);
        assert!(!id.is_dummy());

        assert!(FileId::DUMMY.is_dummy());
    }

    #[test]
    fn test_line_col() {
        let lc = LineCol::new(10, 5);
        assert_eq!(lc.line, 10);
        assert_eq!(lc.column, 5);
        assert_eq!(format!("{}", lc), "10:5");
    }

    #[test]
    fn test_source_file_basic() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "hello\nworld\n");
        
        assert_eq!(file.name(), "test.gox");
        assert_eq!(file.source(), "hello\nworld\n");
        assert_eq!(file.len(), 12);
        assert!(!file.is_empty());
    }

    #[test]
    fn test_source_file_lines() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "line1\nline2\nline3");
        
        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_start(0), Some(0));
        assert_eq!(file.line_start(1), Some(6));
        assert_eq!(file.line_start(2), Some(12));
        assert_eq!(file.line_start(3), None);

        assert_eq!(file.line_content(0), Some("line1"));
        assert_eq!(file.line_content(1), Some("line2"));
        assert_eq!(file.line_content(2), Some("line3"));
        assert_eq!(file.line_content(3), None);
    }

    #[test]
    fn test_source_file_line_col() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "abc\ndefgh\nij");
        
        // First line
        assert_eq!(file.line_col(BytePos(0)), LineCol::new(1, 1));
        assert_eq!(file.line_col(BytePos(2)), LineCol::new(1, 3));
        
        // Second line
        assert_eq!(file.line_col(BytePos(4)), LineCol::new(2, 1));
        assert_eq!(file.line_col(BytePos(7)), LineCol::new(2, 4));
        
        // Third line
        assert_eq!(file.line_col(BytePos(10)), LineCol::new(3, 1));
        assert_eq!(file.line_col(BytePos(11)), LineCol::new(3, 2));
    }

    #[test]
    fn test_source_file_span_text() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "hello world");
        
        assert_eq!(file.span_text(Span::from_u32(0, 5)), "hello");
        assert_eq!(file.span_text(Span::from_u32(6, 11)), "world");
        assert_eq!(file.span_text(Span::from_u32(0, 11)), "hello world");
    }

    #[test]
    fn test_source_map() {
        let mut map = SourceMap::new();
        
        let id1 = map.add_file("file1.gox", "content1");
        let id2 = map.add_file("file2.gox", "content2");
        
        assert_eq!(map.file_count(), 2);
        assert_eq!(map.file_name(id1), Some("file1.gox"));
        assert_eq!(map.file_name(id2), Some("file2.gox"));
        assert_eq!(map.source(id1), Some("content1"));
        assert_eq!(map.source(id2), Some("content2"));
    }

    #[test]
    fn test_source_map_get_file() {
        let mut map = SourceMap::new();
        let id = map.add_file("test.gox", "test content");
        
        let file = map.get_file(id).unwrap();
        assert_eq!(file.name(), "test.gox");
        
        assert!(map.get_file(FileId::DUMMY).is_none());
        assert!(map.get_file(FileId::new(999)).is_none());
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(FileId::new(0), Span::from_u32(10, 20));
        assert!(!loc.is_dummy());
        
        let dummy = SourceLocation::dummy();
        assert!(dummy.is_dummy());
    }

    #[test]
    fn test_source_location_merge() {
        let loc1 = SourceLocation::new(FileId::new(0), Span::from_u32(10, 20));
        let loc2 = SourceLocation::new(FileId::new(0), Span::from_u32(15, 30));
        let merged = loc1.merge(loc2);
        
        assert_eq!(merged.span.start.0, 10);
        assert_eq!(merged.span.end.0, 30);
    }

    #[test]
    fn test_empty_file() {
        let file = SourceFile::new(FileId::new(0), "empty.gox", "");
        
        assert!(file.is_empty());
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_content(0), Some(""));
    }

    #[test]
    fn test_single_line_no_newline() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "single line");
        
        assert_eq!(file.line_count(), 1);
        assert_eq!(file.line_content(0), Some("single line"));
    }

    #[test]
    fn test_trailing_newline() {
        let file = SourceFile::new(FileId::new(0), "test.gox", "line1\nline2\n");
        
        assert_eq!(file.line_count(), 3);
        assert_eq!(file.line_content(0), Some("line1"));
        assert_eq!(file.line_content(1), Some("line2"));
        assert_eq!(file.line_content(2), Some(""));
    }
}
