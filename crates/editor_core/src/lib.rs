//! Core text editing engine for RustIDE.
//!
//! This crate provides the fundamental text editing primitives:
//! - [`Buffer`]: Rope-backed text storage with efficient insert/delete
//! - [`Cursor`] and [`Selection`]: Cursor positioning and text selection
//! - [`Document`]: Complete document model with file I/O
//! - [`History`]: Undo/redo transaction management
//! - [`Edit`]: Atomic edit operations

pub mod buffer;
pub mod cursor;
pub mod document;
pub mod edit;
pub mod history;

pub use buffer::Buffer;
pub use cursor::{Cursor, Point, Selection};
pub use document::Document;
pub use edit::{Edit, EditKind};
pub use history::{History, Transaction};

use thiserror::Error;

/// Errors that can occur during editor operations.
#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Position out of bounds: line {line}, column {column}")]
    PositionOutOfBounds { line: usize, column: usize },

    #[error("Byte offset out of bounds: {offset} (buffer length: {length})")]
    OffsetOutOfBounds { offset: usize, length: usize },

    #[error("Invalid range: start {start} > end {end}")]
    InvalidRange { start: usize, end: usize },

    #[error("Line index out of bounds: {line} (total lines: {total})")]
    LineOutOfBounds { line: usize, total: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 encoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("No undo history available")]
    NoUndoHistory,

    #[error("No redo history available")]
    NoRedoHistory,
}

/// Result type for editor operations.
pub type Result<T> = std::result::Result<T, EditorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_basic_operations() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "Hello, World!").unwrap();
        assert_eq!(buffer.text(), "Hello, World!");
        assert_eq!(buffer.len_chars(), 13);
        assert_eq!(buffer.len_lines(), 1);
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert!(!doc.is_dirty());
        assert!(doc.path().is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let cursor = Cursor::new(Point::new(0, 0));
        assert_eq!(cursor.position().line, 0);
        assert_eq!(cursor.position().column, 0);
    }
}
