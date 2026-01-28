//! Document model combining buffer, cursors, and file management.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::buffer::Buffer;
use crate::cursor::{CursorSet, Point};
use crate::edit::{Edit, EditBatch, TextChange};
use crate::{EditorError, Result};

/// Line ending style for the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix-style line endings (LF, \n).
    #[default]
    Lf,
    /// Windows-style line endings (CRLF, \r\n).
    Crlf,
    /// Old Mac-style line endings (CR, \r).
    Cr,
}

impl LineEnding {
    /// Returns the string representation of the line ending.
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }

    /// Detects the line ending style from text.
    pub fn detect(text: &str) -> Self {
        if text.contains("\r\n") {
            LineEnding::Crlf
        } else if text.contains('\r') {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        }
    }

    /// Normalizes text to use LF line endings.
    pub fn normalize_to_lf(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }

    /// Converts text from LF to this line ending style.
    pub fn from_lf(&self, text: &str) -> String {
        match self {
            LineEnding::Lf => text.to_string(),
            LineEnding::Crlf => text.replace('\n', "\r\n"),
            LineEnding::Cr => text.replace('\n', "\r"),
        }
    }
}

/// Encoding for the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Encoding {
    /// UTF-8 encoding (default).
    #[default]
    Utf8,
    /// UTF-8 with BOM.
    Utf8Bom,
    /// UTF-16 Little Endian.
    Utf16Le,
    /// UTF-16 Big Endian.
    Utf16Be,
    /// Latin-1 (ISO-8859-1).
    Latin1,
}

impl Encoding {
    /// Returns the name of the encoding.
    pub fn name(&self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 with BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
            Encoding::Latin1 => "ISO-8859-1",
        }
    }
}

/// Document metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// The file path, if the document is associated with a file.
    pub path: Option<PathBuf>,
    /// The detected or configured line ending style.
    pub line_ending: LineEnding,
    /// The detected or configured encoding.
    pub encoding: Encoding,
    /// The language ID for syntax highlighting.
    pub language_id: Option<String>,
    /// Whether the document is read-only.
    pub readonly: bool,
}

/// A complete document with buffer, cursors, and file management.
#[derive(Debug)]
pub struct Document {
    /// The text buffer.
    buffer: Buffer,
    /// The cursor set for this document.
    cursors: CursorSet,
    /// Document metadata.
    metadata: DocumentMetadata,
    /// Version number for LSP synchronization.
    version: u64,
    /// The last saved version (for tracking dirty state).
    saved_version: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// Creates a new empty document.
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursors: CursorSet::new(),
            metadata: DocumentMetadata::default(),
            version: 0,
            saved_version: 0,
        }
    }

    /// Creates a document from text.
    pub fn from_text(text: &str) -> Self {
        // Detect and normalize line endings
        let line_ending = LineEnding::detect(text);
        let normalized = LineEnding::normalize_to_lf(text);

        Self {
            buffer: Buffer::from_text(&normalized),
            cursors: CursorSet::new(),
            metadata: DocumentMetadata {
                line_ending,
                ..Default::default()
            },
            version: 0,
            saved_version: 0,
        }
    }

    /// Opens a document from a file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;

        let line_ending = LineEnding::detect(&content);
        let normalized = LineEnding::normalize_to_lf(&content);

        // Detect language from file extension
        let language_id = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Self::language_from_extension(ext));

        let metadata = DocumentMetadata {
            path: Some(path.to_path_buf()),
            line_ending,
            encoding: Encoding::Utf8,
            language_id,
            readonly: false,
        };

        Ok(Self {
            buffer: Buffer::from_text(&normalized),
            cursors: CursorSet::new(),
            metadata,
            version: 0,
            saved_version: 0,
        })
    }

    /// Saves the document to its associated file path.
    pub fn save(&mut self) -> Result<()> {
        let path = self
            .metadata
            .path
            .clone()
            .ok_or_else(|| EditorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Document has no associated file path",
            )))?;

        self.save_as(&path)
    }

    /// Saves the document to a specific file path.
    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Convert line endings for saving
        let text = self.buffer.text();
        let text_with_endings = self.metadata.line_ending.from_lf(&text);

        let file = fs::File::create(path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(text_with_endings.as_bytes())?;
        writer.flush()?;

        self.metadata.path = Some(path.to_path_buf());
        self.buffer.mark_clean();
        self.saved_version = self.version;

        Ok(())
    }

    /// Reloads the document from disk.
    pub fn reload(&mut self) -> Result<()> {
        let path = self
            .metadata
            .path
            .clone()
            .ok_or_else(|| EditorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Document has no associated file path",
            )))?;

        let content = fs::read_to_string(&path)?;
        let line_ending = LineEnding::detect(&content);
        let normalized = LineEnding::normalize_to_lf(&content);

        // Replace buffer content
        let len = self.buffer.len_bytes();
        if len > 0 {
            self.buffer.delete(0, len)?;
        }
        if !normalized.is_empty() {
            self.buffer.insert(0, &normalized)?;
        }

        self.metadata.line_ending = line_ending;
        self.buffer.mark_clean();
        self.buffer.clear_history();
        self.version += 1;
        self.saved_version = self.version;

        // Reset cursors
        self.cursors = CursorSet::new();

        Ok(())
    }

    /// Returns a reference to the buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Returns a mutable reference to the buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.version += 1;
        &mut self.buffer
    }

    /// Returns a reference to the cursor set.
    pub fn cursors(&self) -> &CursorSet {
        &self.cursors
    }

    /// Returns a mutable reference to the cursor set.
    pub fn cursors_mut(&mut self) -> &mut CursorSet {
        &mut self.cursors
    }

    /// Returns the file path associated with this document.
    pub fn path(&self) -> Option<&Path> {
        self.metadata.path.as_deref()
    }

    /// Returns the file name without the directory.
    pub fn file_name(&self) -> Option<&str> {
        self.metadata
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
    }

    /// Returns true if the document has been modified since last save.
    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty() || self.version != self.saved_version
    }

    /// Returns true if the document is read-only.
    pub fn is_readonly(&self) -> bool {
        self.metadata.readonly
    }

    /// Sets whether the document is read-only.
    pub fn set_readonly(&mut self, readonly: bool) {
        self.metadata.readonly = readonly;
    }

    /// Returns the document version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the line ending style.
    pub fn line_ending(&self) -> LineEnding {
        self.metadata.line_ending
    }

    /// Sets the line ending style.
    pub fn set_line_ending(&mut self, line_ending: LineEnding) {
        self.metadata.line_ending = line_ending;
    }

    /// Returns the encoding.
    pub fn encoding(&self) -> Encoding {
        self.metadata.encoding
    }

    /// Sets the encoding.
    pub fn set_encoding(&mut self, encoding: Encoding) {
        self.metadata.encoding = encoding;
    }

    /// Returns the language ID.
    pub fn language_id(&self) -> Option<&str> {
        self.metadata.language_id.as_deref()
    }

    /// Sets the language ID.
    pub fn set_language_id(&mut self, language_id: impl Into<String>) {
        self.metadata.language_id = Some(language_id.into());
    }

    /// Returns the document metadata.
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Returns a mutable reference to the document metadata.
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    // Edit operations that update both buffer and cursors

    /// Inserts text at the primary cursor position.
    pub fn insert(&mut self, text: &str) -> Result<Edit> {
        let pos = self.cursors.primary().position();
        let offset = self.buffer.point_to_offset(pos)?;
        self.version += 1;
        let edit = self.buffer.insert(offset, text)?;

        // Update cursor position
        let new_pos = self.buffer.offset_to_point(offset + text.len())?;
        self.cursors.primary_mut().move_to(new_pos);

        Ok(edit)
    }

    /// Deletes the selection or character before cursor.
    pub fn delete_backward(&mut self) -> Result<Option<Edit>> {
        let cursor = self.cursors.primary();
        let selection = cursor.selection();

        if !selection.is_collapsed() {
            // Delete selection
            let (start, end) = selection.normalized();
            let start_offset = self.buffer.point_to_offset(start)?;
            let end_offset = self.buffer.point_to_offset(end)?;

            self.version += 1;
            let edit = self.buffer.delete(start_offset, end_offset)?;
            self.cursors.primary_mut().move_to(start);

            Ok(Some(edit))
        } else {
            // Delete character before cursor
            let pos = cursor.position();
            if pos.line == 0 && pos.column == 0 {
                return Ok(None); // At start of document
            }

            let offset = self.buffer.point_to_offset(pos)?;
            if offset == 0 {
                return Ok(None);
            }

            // Find the previous character boundary
            let prev_offset = if pos.column == 0 {
                // Delete the newline from previous line
                let prev_line_len = self.buffer.line_len(pos.line - 1)?;
                self.buffer.point_to_offset(Point::new(pos.line - 1, prev_line_len))?
            } else {
                offset - 1
            };

            self.version += 1;
            let edit = self.buffer.delete(prev_offset, offset)?;
            let new_pos = self.buffer.offset_to_point(prev_offset)?;
            self.cursors.primary_mut().move_to(new_pos);

            Ok(Some(edit))
        }
    }

    /// Deletes the selection or character after cursor.
    pub fn delete_forward(&mut self) -> Result<Option<Edit>> {
        let cursor = self.cursors.primary();
        let selection = cursor.selection();

        if !selection.is_collapsed() {
            // Delete selection
            let (start, end) = selection.normalized();
            let start_offset = self.buffer.point_to_offset(start)?;
            let end_offset = self.buffer.point_to_offset(end)?;

            self.version += 1;
            let edit = self.buffer.delete(start_offset, end_offset)?;
            self.cursors.primary_mut().move_to(start);

            Ok(Some(edit))
        } else {
            // Delete character after cursor
            let pos = cursor.position();
            let offset = self.buffer.point_to_offset(pos)?;

            if offset >= self.buffer.len_bytes() {
                return Ok(None); // At end of document
            }

            // Find the next character boundary
            let line_len = self.buffer.line_len(pos.line)?;
            let next_offset = if pos.column >= line_len {
                // Delete the newline
                offset + 1
            } else {
                offset + 1
            };

            self.version += 1;
            let edit = self.buffer.delete(offset, next_offset)?;

            Ok(Some(edit))
        }
    }

    /// Applies a text change (LSP-style).
    pub fn apply_change(&mut self, change: TextChange) -> Result<Edit> {
        let edit = change.to_edit(&self.buffer)?;

        self.version += 1;
        match edit.kind {
            crate::edit::EditKind::Insert => {
                self.buffer.insert(edit.offset, &edit.new_text)?;
            }
            crate::edit::EditKind::Delete => {
                self.buffer.delete(edit.offset, edit.offset + edit.old_text.len())?;
            }
            crate::edit::EditKind::Replace => {
                self.buffer.replace(
                    edit.offset,
                    edit.offset + edit.old_text.len(),
                    &edit.new_text,
                )?;
            }
        }

        Ok(edit)
    }

    /// Applies a batch of edits.
    pub fn apply_batch(&mut self, batch: &EditBatch) -> Result<Vec<Edit>> {
        self.version += 1;
        batch.apply(&mut self.buffer)
    }

    /// Undoes the last edit.
    pub fn undo(&mut self) -> Result<Vec<Edit>> {
        self.version += 1;
        self.buffer.undo()
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> Result<Vec<Edit>> {
        self.version += 1;
        self.buffer.redo()
    }

    /// Returns the text content.
    pub fn text(&self) -> String {
        self.buffer.text()
    }

    /// Returns the number of lines.
    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Returns a specific line's text.
    pub fn line(&self, line_idx: usize) -> Result<String> {
        self.buffer.line(line_idx)
    }

    /// Detects language from file extension.
    fn language_from_extension(ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "javascriptreact",
            "tsx" => "typescriptreact",
            "py" => "python",
            "rb" => "ruby",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "java" => "java",
            "cs" => "csharp",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "scala" => "scala",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "less" => "less",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "xml" => "xml",
            "md" | "markdown" => "markdown",
            "sql" => "sql",
            "sh" | "bash" => "shellscript",
            "ps1" => "powershell",
            "dockerfile" => "dockerfile",
            _ => ext,
        }
        .to_string()
    }
}

/// A thread-safe document wrapper.
pub type SharedDocument = Arc<RwLock<Document>>;

/// Creates a new shared document.
pub fn shared_document() -> SharedDocument {
    Arc::new(RwLock::new(Document::new()))
}

/// Creates a shared document from text.
pub fn shared_document_from_text(text: &str) -> SharedDocument {
    Arc::new(RwLock::new(Document::from_text(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document() {
        let doc = Document::new();
        assert!(!doc.is_dirty());
        assert!(doc.path().is_none());
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn test_document_from_text() {
        let doc = Document::from_text("Hello, World!");
        assert_eq!(doc.text(), "Hello, World!");
        assert_eq!(doc.line_count(), 1);
    }

    #[test]
    fn test_line_ending_detection() {
        let unix_doc = Document::from_text("line1\nline2\n");
        assert_eq!(unix_doc.line_ending(), LineEnding::Lf);

        let windows_doc = Document::from_text("line1\r\nline2\r\n");
        assert_eq!(windows_doc.line_ending(), LineEnding::Crlf);
        // Content should be normalized to LF
        assert_eq!(windows_doc.text(), "line1\nline2\n");
    }

    #[test]
    fn test_document_insert() {
        let mut doc = Document::new();
        doc.insert("Hello").unwrap();
        assert_eq!(doc.text(), "Hello");
        assert!(doc.is_dirty());
    }

    #[test]
    fn test_document_delete_backward() {
        let mut doc = Document::from_text("Hello");
        doc.cursors_mut().primary_mut().move_to(Point::new(0, 5));

        doc.delete_backward().unwrap();
        assert_eq!(doc.text(), "Hell");
    }

    #[test]
    fn test_document_undo_redo() {
        let mut doc = Document::new();
        doc.insert("Hello").unwrap();
        assert_eq!(doc.text(), "Hello");

        doc.undo().unwrap();
        assert_eq!(doc.text(), "");

        doc.redo().unwrap();
        assert_eq!(doc.text(), "Hello");
    }

    #[test]
    fn test_language_detection() {
        let _doc = Document::from_text("");

        assert_eq!(Document::language_from_extension("rs"), "rust");
        assert_eq!(Document::language_from_extension("js"), "javascript");
        assert_eq!(Document::language_from_extension("py"), "python");
        assert_eq!(Document::language_from_extension("go"), "go");
    }

    #[test]
    fn test_line_ending_conversion() {
        assert_eq!(LineEnding::Lf.from_lf("a\nb"), "a\nb");
        assert_eq!(LineEnding::Crlf.from_lf("a\nb"), "a\r\nb");
        assert_eq!(LineEnding::Cr.from_lf("a\nb"), "a\rb");
    }

    #[test]
    fn test_document_version() {
        let mut doc = Document::new();
        let v0 = doc.version();

        doc.insert("Hello").unwrap();
        let v1 = doc.version();
        assert!(v1 > v0);

        doc.undo().unwrap();
        let v2 = doc.version();
        assert!(v2 > v1);
    }
}
