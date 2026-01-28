//! Rope-backed text buffer with efficient insert/delete operations.

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

use crate::cursor::Point;
use crate::edit::{Edit, EditKind};
use crate::history::History;
use crate::{EditorError, Result};

/// A text buffer backed by a rope data structure for efficient editing.
///
/// The buffer provides O(log n) insert and delete operations, making it
/// suitable for large files. It also maintains undo/redo history.
#[derive(Debug, Clone)]
pub struct Buffer {
    /// The rope storing the text content.
    rope: Rope,
    /// Undo/redo history for this buffer.
    history: History,
    /// Whether the buffer has been modified since last save.
    dirty: bool,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    /// Creates a new empty buffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            history: History::new(),
            dirty: false,
        }
    }

    /// Creates a buffer from the given text.
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            history: History::new(),
            dirty: false,
        }
    }

    /// Returns the entire text content as a string.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Returns the text in the given byte range.
    pub fn text_range(&self, start: usize, end: usize) -> Result<String> {
        self.validate_range(start, end)?;
        let start_char = self.rope.byte_to_char(start);
        let end_char = self.rope.byte_to_char(end);
        Ok(self.rope.slice(start_char..end_char).to_string())
    }

    /// Returns the number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the number of bytes in the buffer.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Returns the number of lines in the buffer.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Returns true if the buffer has been modified since last save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the buffer as clean (saved).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Marks the buffer as dirty (modified).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns the text of a specific line (0-indexed).
    pub fn line(&self, line_idx: usize) -> Result<String> {
        if line_idx >= self.rope.len_lines() {
            return Err(EditorError::LineOutOfBounds {
                line: line_idx,
                total: self.rope.len_lines(),
            });
        }
        Ok(self.rope.line(line_idx).to_string())
    }

    /// Returns the length of a line in characters (excluding newline).
    pub fn line_len(&self, line_idx: usize) -> Result<usize> {
        if line_idx >= self.rope.len_lines() {
            return Err(EditorError::LineOutOfBounds {
                line: line_idx,
                total: self.rope.len_lines(),
            });
        }
        let line = self.rope.line(line_idx);
        let len = line.len_chars();
        // Subtract newline character if present
        if len > 0 && line.char(len - 1) == '\n' {
            Ok(len - 1)
        } else {
            Ok(len)
        }
    }

    /// Converts a (line, column) point to a byte offset.
    pub fn point_to_offset(&self, point: Point) -> Result<usize> {
        if point.line >= self.rope.len_lines() {
            return Err(EditorError::PositionOutOfBounds {
                line: point.line,
                column: point.column,
            });
        }

        let line_start = self.rope.line_to_char(point.line);
        let line_len = self.line_len(point.line)?;

        // Allow column to be at end of line (for cursor positioning)
        if point.column > line_len {
            return Err(EditorError::PositionOutOfBounds {
                line: point.line,
                column: point.column,
            });
        }

        let char_idx = line_start + point.column;
        Ok(self.rope.char_to_byte(char_idx))
    }

    /// Converts a byte offset to a (line, column) point.
    pub fn offset_to_point(&self, offset: usize) -> Result<Point> {
        if offset > self.rope.len_bytes() {
            return Err(EditorError::OffsetOutOfBounds {
                offset,
                length: self.rope.len_bytes(),
            });
        }

        let char_idx = self.rope.byte_to_char(offset);
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let column = char_idx - line_start;

        Ok(Point::new(line, column))
    }

    /// Converts a character offset to a byte offset.
    pub fn char_to_byte(&self, char_idx: usize) -> Result<usize> {
        if char_idx > self.rope.len_chars() {
            return Err(EditorError::OffsetOutOfBounds {
                offset: char_idx,
                length: self.rope.len_chars(),
            });
        }
        Ok(self.rope.char_to_byte(char_idx))
    }

    /// Converts a byte offset to a character offset.
    pub fn byte_to_char(&self, byte_idx: usize) -> Result<usize> {
        if byte_idx > self.rope.len_bytes() {
            return Err(EditorError::OffsetOutOfBounds {
                offset: byte_idx,
                length: self.rope.len_bytes(),
            });
        }
        Ok(self.rope.byte_to_char(byte_idx))
    }

    /// Inserts text at the given byte offset.
    ///
    /// Returns the edit that was applied, which can be used for undo.
    pub fn insert(&mut self, offset: usize, text: &str) -> Result<Edit> {
        if offset > self.rope.len_bytes() {
            return Err(EditorError::OffsetOutOfBounds {
                offset,
                length: self.rope.len_bytes(),
            });
        }

        let char_idx = self.rope.byte_to_char(offset);
        self.rope.insert(char_idx, text);
        self.dirty = true;

        let edit = Edit {
            kind: EditKind::Insert,
            offset,
            old_text: String::new(),
            new_text: text.to_string(),
        };

        self.history.push_edit(edit.clone());
        Ok(edit)
    }

    /// Deletes text in the given byte range.
    ///
    /// Returns the edit that was applied, which can be used for undo.
    pub fn delete(&mut self, start: usize, end: usize) -> Result<Edit> {
        self.validate_range(start, end)?;

        let start_char = self.rope.byte_to_char(start);
        let end_char = self.rope.byte_to_char(end);
        let old_text = self.rope.slice(start_char..end_char).to_string();

        self.rope.remove(start_char..end_char);
        self.dirty = true;

        let edit = Edit {
            kind: EditKind::Delete,
            offset: start,
            old_text,
            new_text: String::new(),
        };

        self.history.push_edit(edit.clone());
        Ok(edit)
    }

    /// Replaces text in the given byte range with new text.
    ///
    /// Returns the edit that was applied, which can be used for undo.
    pub fn replace(&mut self, start: usize, end: usize, text: &str) -> Result<Edit> {
        self.validate_range(start, end)?;

        let start_char = self.rope.byte_to_char(start);
        let end_char = self.rope.byte_to_char(end);
        let old_text = self.rope.slice(start_char..end_char).to_string();

        self.rope.remove(start_char..end_char);
        self.rope.insert(start_char, text);
        self.dirty = true;

        let edit = Edit {
            kind: EditKind::Replace,
            offset: start,
            old_text,
            new_text: text.to_string(),
        };

        self.history.push_edit(edit.clone());
        Ok(edit)
    }

    /// Applies an edit to the buffer without recording it in history.
    ///
    /// This is used for undo/redo operations.
    pub fn apply_edit(&mut self, edit: &Edit) -> Result<()> {
        match edit.kind {
            EditKind::Insert => {
                let char_idx = self.rope.byte_to_char(edit.offset);
                self.rope.insert(char_idx, &edit.new_text);
            }
            EditKind::Delete => {
                let start_char = self.rope.byte_to_char(edit.offset);
                let end_char = start_char + edit.old_text.chars().count();
                self.rope.remove(start_char..end_char);
            }
            EditKind::Replace => {
                let start_char = self.rope.byte_to_char(edit.offset);
                let end_char = start_char + edit.old_text.chars().count();
                self.rope.remove(start_char..end_char);
                self.rope.insert(start_char, &edit.new_text);
            }
        }
        self.dirty = true;
        Ok(())
    }

    /// Applies the inverse of an edit to the buffer.
    ///
    /// This is used for undo operations.
    pub fn apply_edit_inverse(&mut self, edit: &Edit) -> Result<()> {
        let inverse = edit.inverse();
        self.apply_edit(&inverse)
    }

    /// Undoes the last edit or transaction.
    ///
    /// Returns the edits that were undone.
    pub fn undo(&mut self) -> Result<Vec<Edit>> {
        let edits = self.history.undo()?;
        for edit in edits.iter().rev() {
            self.apply_edit_inverse(edit)?;
        }
        Ok(edits)
    }

    /// Redoes the last undone edit or transaction.
    ///
    /// Returns the edits that were redone.
    pub fn redo(&mut self) -> Result<Vec<Edit>> {
        let edits = self.history.redo()?;
        for edit in &edits {
            self.apply_edit(edit)?;
        }
        Ok(edits)
    }

    /// Returns true if there are edits that can be undone.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Returns true if there are edits that can be redone.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Begins a new transaction for grouping edits.
    pub fn begin_transaction(&mut self) {
        self.history.begin_transaction();
    }

    /// Ends the current transaction.
    pub fn end_transaction(&mut self) {
        self.history.end_transaction();
    }

    /// Clears the undo/redo history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Returns the word at the given byte offset.
    pub fn word_at(&self, offset: usize) -> Result<Option<(usize, usize, String)>> {
        if offset > self.rope.len_bytes() {
            return Err(EditorError::OffsetOutOfBounds {
                offset,
                length: self.rope.len_bytes(),
            });
        }

        let char_idx = self.rope.byte_to_char(offset);
        let line_idx = self.rope.char_to_line(char_idx);
        let line = self.rope.line(line_idx);
        let line_start = self.rope.line_to_char(line_idx);
        let col = char_idx - line_start;

        let line_str = line.to_string();

        for (idx, word) in line_str.unicode_word_indices() {
            let word_start_col = line_str[..idx].chars().count();
            let word_end_col = word_start_col + word.chars().count();

            if col >= word_start_col && col <= word_end_col {
                let word_start = line_start + word_start_col;
                let word_end = line_start + word_end_col;

                let start_byte = self.rope.char_to_byte(word_start);
                let end_byte = self.rope.char_to_byte(word_end);
                return Ok(Some((start_byte, end_byte, word.to_string())));
            }
        }

        Ok(None)
    }

    /// Validates that a byte range is within bounds and properly ordered.
    fn validate_range(&self, start: usize, end: usize) -> Result<()> {
        if start > end {
            return Err(EditorError::InvalidRange { start, end });
        }
        if end > self.rope.len_bytes() {
            return Err(EditorError::OffsetOutOfBounds {
                offset: end,
                length: self.rope.len_bytes(),
            });
        }
        Ok(())
    }

    /// Returns a reference to the underlying rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Returns a mutable reference to the history.
    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = Buffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len_chars(), 0);
        assert_eq!(buffer.len_bytes(), 0);
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn test_from_text() {
        let buffer = Buffer::from_text("Hello, World!");
        assert_eq!(buffer.text(), "Hello, World!");
        assert_eq!(buffer.len_chars(), 13);
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn test_insert() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "Hello").unwrap();
        assert_eq!(buffer.text(), "Hello");
        assert!(buffer.is_dirty());

        buffer.insert(5, ", World!").unwrap();
        assert_eq!(buffer.text(), "Hello, World!");
    }

    #[test]
    fn test_delete() {
        let mut buffer = Buffer::from_text("Hello, World!");
        buffer.delete(5, 7).unwrap();
        assert_eq!(buffer.text(), "HelloWorld!");
    }

    #[test]
    fn test_replace() {
        let mut buffer = Buffer::from_text("Hello, World!");
        buffer.replace(7, 12, "Rust").unwrap();
        assert_eq!(buffer.text(), "Hello, Rust!");
    }

    #[test]
    fn test_line_operations() {
        let buffer = Buffer::from_text("Line 1\nLine 2\nLine 3");
        assert_eq!(buffer.len_lines(), 3);
        assert_eq!(buffer.line(0).unwrap(), "Line 1\n");
        assert_eq!(buffer.line(1).unwrap(), "Line 2\n");
        assert_eq!(buffer.line(2).unwrap(), "Line 3");
        assert_eq!(buffer.line_len(0).unwrap(), 6);
    }

    #[test]
    fn test_point_conversion() {
        let buffer = Buffer::from_text("Hello\nWorld");

        let offset = buffer.point_to_offset(Point::new(0, 0)).unwrap();
        assert_eq!(offset, 0);

        let offset = buffer.point_to_offset(Point::new(1, 0)).unwrap();
        assert_eq!(offset, 6);

        let point = buffer.offset_to_point(6).unwrap();
        assert_eq!(point, Point::new(1, 0));
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "Hello").unwrap();
        assert_eq!(buffer.text(), "Hello");

        buffer.undo().unwrap();
        assert_eq!(buffer.text(), "");

        buffer.redo().unwrap();
        assert_eq!(buffer.text(), "Hello");
    }

    #[test]
    fn test_transaction() {
        let mut buffer = Buffer::new();

        buffer.begin_transaction();
        buffer.insert(0, "Hello").unwrap();
        buffer.insert(5, " World").unwrap();
        buffer.end_transaction();

        assert_eq!(buffer.text(), "Hello World");

        // Undo should undo both inserts
        buffer.undo().unwrap();
        assert_eq!(buffer.text(), "");
    }
}
