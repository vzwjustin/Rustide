//! Edit operations for text manipulation.

use serde::{Deserialize, Serialize};

use crate::buffer::Buffer;
use crate::cursor::Point;
use crate::Result;

/// The kind of edit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditKind {
    /// Insert text at a position.
    Insert,
    /// Delete text from a range.
    Delete,
    /// Replace text in a range with new text.
    Replace,
}

/// An atomic edit operation.
///
/// Represents a single change to the text buffer that can be undone/redone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    /// The kind of edit operation.
    pub kind: EditKind,
    /// The byte offset where the edit occurs.
    pub offset: usize,
    /// The old text that was replaced/deleted (empty for inserts).
    pub old_text: String,
    /// The new text that was inserted/replaced (empty for deletes).
    pub new_text: String,
}

impl Edit {
    /// Creates an insert edit.
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Insert,
            offset,
            old_text: String::new(),
            new_text: text.into(),
        }
    }

    /// Creates a delete edit.
    pub fn delete(offset: usize, text: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Delete,
            offset,
            old_text: text.into(),
            new_text: String::new(),
        }
    }

    /// Creates a replace edit.
    pub fn replace(offset: usize, old_text: impl Into<String>, new_text: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Replace,
            offset,
            old_text: old_text.into(),
            new_text: new_text.into(),
        }
    }

    /// Returns the inverse of this edit (for undo).
    pub fn inverse(&self) -> Edit {
        match self.kind {
            EditKind::Insert => Edit {
                kind: EditKind::Delete,
                offset: self.offset,
                old_text: self.new_text.clone(),
                new_text: String::new(),
            },
            EditKind::Delete => Edit {
                kind: EditKind::Insert,
                offset: self.offset,
                old_text: String::new(),
                new_text: self.old_text.clone(),
            },
            EditKind::Replace => Edit {
                kind: EditKind::Replace,
                offset: self.offset,
                old_text: self.new_text.clone(),
                new_text: self.old_text.clone(),
            },
        }
    }

    /// Returns the byte offset where the edit starts.
    pub fn start(&self) -> usize {
        self.offset
    }

    /// Returns the byte offset where the edit ends (after applying).
    pub fn end(&self) -> usize {
        self.offset + self.new_text.len()
    }

    /// Returns the length of text that was removed.
    pub fn deleted_len(&self) -> usize {
        self.old_text.len()
    }

    /// Returns the length of text that was inserted.
    pub fn inserted_len(&self) -> usize {
        self.new_text.len()
    }

    /// Returns the net change in buffer length.
    pub fn delta(&self) -> isize {
        self.inserted_len() as isize - self.deleted_len() as isize
    }

    /// Returns true if this edit is a no-op.
    pub fn is_noop(&self) -> bool {
        self.old_text == self.new_text
    }
}

/// A batch of edits to be applied atomically.
///
/// Edits are applied in order, with offsets adjusted for previous edits.
#[derive(Debug, Clone, Default)]
pub struct EditBatch {
    /// The edits in this batch.
    edits: Vec<Edit>,
}

impl EditBatch {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        Self { edits: Vec::new() }
    }

    /// Adds an insert edit to the batch.
    pub fn insert(&mut self, offset: usize, text: impl Into<String>) -> &mut Self {
        self.edits.push(Edit::insert(offset, text));
        self
    }

    /// Adds a delete edit to the batch.
    pub fn delete(&mut self, start: usize, _end: usize, old_text: impl Into<String>) -> &mut Self {
        self.edits.push(Edit::delete(start, old_text));
        self
    }

    /// Adds a replace edit to the batch.
    pub fn replace(
        &mut self,
        offset: usize,
        old_text: impl Into<String>,
        new_text: impl Into<String>,
    ) -> &mut Self {
        self.edits.push(Edit::replace(offset, old_text, new_text));
        self
    }

    /// Adds an edit to the batch.
    pub fn push(&mut self, edit: Edit) -> &mut Self {
        self.edits.push(edit);
        self
    }

    /// Returns the edits in this batch.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Returns the number of edits in this batch.
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Returns true if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Clears all edits from the batch.
    pub fn clear(&mut self) {
        self.edits.clear();
    }

    /// Applies all edits in the batch to the buffer.
    ///
    /// Edits are applied in order, with subsequent edit offsets adjusted
    /// based on the length changes from previous edits.
    pub fn apply(&self, buffer: &mut Buffer) -> Result<Vec<Edit>> {
        let mut applied = Vec::with_capacity(self.edits.len());
        let mut offset_delta: isize = 0;

        buffer.begin_transaction();

        for edit in &self.edits {
            // Adjust offset based on previous edits
            let adjusted_offset = (edit.offset as isize + offset_delta) as usize;

            let applied_edit = match edit.kind {
                EditKind::Insert => buffer.insert(adjusted_offset, &edit.new_text)?,
                EditKind::Delete => {
                    let end = adjusted_offset + edit.old_text.len();
                    buffer.delete(adjusted_offset, end)?
                }
                EditKind::Replace => {
                    let end = adjusted_offset + edit.old_text.len();
                    buffer.replace(adjusted_offset, end, &edit.new_text)?
                }
            };

            offset_delta += applied_edit.delta();
            applied.push(applied_edit);
        }

        buffer.end_transaction();

        Ok(applied)
    }

    /// Sorts edits by offset in descending order.
    ///
    /// This is useful when you want to apply edits from end to start
    /// to avoid offset invalidation.
    pub fn sort_descending(&mut self) {
        self.edits.sort_by(|a, b| b.offset.cmp(&a.offset));
    }

    /// Sorts edits by offset in ascending order.
    pub fn sort_ascending(&mut self) {
        self.edits.sort_by(|a, b| a.offset.cmp(&b.offset));
    }
}

/// Builder for creating complex edit operations.
#[derive(Debug)]
pub struct EditBuilder<'a> {
    buffer: &'a Buffer,
    batch: EditBatch,
}

impl<'a> EditBuilder<'a> {
    /// Creates a new edit builder for the given buffer.
    pub fn new(buffer: &'a Buffer) -> Self {
        Self {
            buffer,
            batch: EditBatch::new(),
        }
    }

    /// Inserts text at the given point.
    pub fn insert_at_point(&mut self, point: Point, text: impl Into<String>) -> Result<&mut Self> {
        let offset = self.buffer.point_to_offset(point)?;
        self.batch.insert(offset, text);
        Ok(self)
    }

    /// Deletes text between two points.
    pub fn delete_range(&mut self, start: Point, end: Point) -> Result<&mut Self> {
        let start_offset = self.buffer.point_to_offset(start)?;
        let end_offset = self.buffer.point_to_offset(end)?;
        let old_text = self.buffer.text_range(start_offset, end_offset)?;
        self.batch.delete(start_offset, end_offset, old_text);
        Ok(self)
    }

    /// Replaces text between two points.
    pub fn replace_range(
        &mut self,
        start: Point,
        end: Point,
        new_text: impl Into<String>,
    ) -> Result<&mut Self> {
        let start_offset = self.buffer.point_to_offset(start)?;
        let end_offset = self.buffer.point_to_offset(end)?;
        let old_text = self.buffer.text_range(start_offset, end_offset)?;
        self.batch.replace(start_offset, old_text, new_text);
        Ok(self)
    }

    /// Deletes an entire line.
    pub fn delete_line(&mut self, line: usize) -> Result<&mut Self> {
        let start = Point::new(line, 0);
        let start_offset = self.buffer.point_to_offset(start)?;

        // Get the line including the newline
        let line_text = self.buffer.line(line)?;
        let end_offset = start_offset + line_text.len();

        self.batch.delete(start_offset, end_offset, line_text);
        Ok(self)
    }

    /// Inserts a new line at the given line number.
    pub fn insert_line(&mut self, line: usize, text: impl Into<String>) -> Result<&mut Self> {
        let offset = if line >= self.buffer.len_lines() {
            // Insert at end of buffer
            self.buffer.len_bytes()
        } else {
            self.buffer.point_to_offset(Point::new(line, 0))?
        };

        let mut text = text.into();
        if !text.ends_with('\n') {
            text.push('\n');
        }

        self.batch.insert(offset, text);
        Ok(self)
    }

    /// Returns the built batch of edits.
    pub fn build(self) -> EditBatch {
        self.batch
    }

    /// Applies the edits to a buffer.
    pub fn apply(self, buffer: &mut Buffer) -> Result<Vec<Edit>> {
        self.batch.apply(buffer)
    }
}

/// Represents a text change for LSP-style incremental updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    /// The range being replaced (in line/column coordinates).
    pub start: Point,
    pub end: Point,
    /// The new text to insert.
    pub new_text: String,
}

impl TextChange {
    /// Creates a new text change.
    pub fn new(start: Point, end: Point, new_text: impl Into<String>) -> Self {
        Self {
            start,
            end,
            new_text: new_text.into(),
        }
    }

    /// Converts this text change to an edit using the given buffer.
    pub fn to_edit(&self, buffer: &Buffer) -> Result<Edit> {
        let start_offset = buffer.point_to_offset(self.start)?;
        let end_offset = buffer.point_to_offset(self.end)?;
        let old_text = buffer.text_range(start_offset, end_offset)?;

        Ok(Edit::replace(start_offset, old_text, &self.new_text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_insert() {
        let edit = Edit::insert(5, "Hello");
        assert_eq!(edit.kind, EditKind::Insert);
        assert_eq!(edit.offset, 5);
        assert_eq!(edit.new_text, "Hello");
        assert!(edit.old_text.is_empty());
    }

    #[test]
    fn test_edit_delete() {
        let edit = Edit::delete(0, "Hello");
        assert_eq!(edit.kind, EditKind::Delete);
        assert_eq!(edit.offset, 0);
        assert_eq!(edit.old_text, "Hello");
        assert!(edit.new_text.is_empty());
    }

    #[test]
    fn test_edit_replace() {
        let edit = Edit::replace(0, "Hello", "World");
        assert_eq!(edit.kind, EditKind::Replace);
        assert_eq!(edit.old_text, "Hello");
        assert_eq!(edit.new_text, "World");
    }

    #[test]
    fn test_edit_inverse() {
        let insert = Edit::insert(0, "Hello");
        let inverse = insert.inverse();
        assert_eq!(inverse.kind, EditKind::Delete);
        assert_eq!(inverse.old_text, "Hello");

        let delete = Edit::delete(0, "World");
        let inverse = delete.inverse();
        assert_eq!(inverse.kind, EditKind::Insert);
        assert_eq!(inverse.new_text, "World");

        let replace = Edit::replace(0, "foo", "bar");
        let inverse = replace.inverse();
        assert_eq!(inverse.kind, EditKind::Replace);
        assert_eq!(inverse.old_text, "bar");
        assert_eq!(inverse.new_text, "foo");
    }

    #[test]
    fn test_edit_delta() {
        let insert = Edit::insert(0, "Hello");
        assert_eq!(insert.delta(), 5);

        let delete = Edit::delete(0, "Hello");
        assert_eq!(delete.delta(), -5);

        let replace = Edit::replace(0, "Hi", "Hello");
        assert_eq!(replace.delta(), 3);
    }

    #[test]
    fn test_edit_batch() {
        let mut batch = EditBatch::new();
        batch
            .insert(0, "Hello")
            .insert(5, " World");

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_edit_batch_apply() {
        let mut buffer = Buffer::new();

        // When building a batch, offsets are relative to the ORIGINAL document.
        // Since we're applying to an empty buffer, both inserts are at offset 0.
        // The apply function adjusts subsequent offsets based on previous edits.
        let mut batch = EditBatch::new();
        batch
            .insert(0, "Hello")
            .insert(0, " World");  // This is at offset 0 in original (empty) doc

        let applied = batch.apply(&mut buffer).unwrap();

        // After first insert "Hello" at 0, buffer is "Hello", delta = +5
        // Second insert at 0 (adjusted to 0+5=5) inserts " World"
        assert_eq!(buffer.text(), "Hello World");
        assert_eq!(applied.len(), 2);
    }

    #[test]
    fn test_edit_builder() {
        let buffer = Buffer::from_text("Hello World");
        let mut builder = EditBuilder::new(&buffer);

        builder.replace_range(Point::new(0, 6), Point::new(0, 11), "Rust").unwrap();

        let batch = builder.build();
        assert_eq!(batch.len(), 1);

        let mut buffer = Buffer::from_text("Hello World");
        batch.apply(&mut buffer).unwrap();
        assert_eq!(buffer.text(), "Hello Rust");
    }

    #[test]
    fn test_text_change_to_edit() {
        let buffer = Buffer::from_text("Hello World");

        let change = TextChange::new(
            Point::new(0, 0),
            Point::new(0, 5),
            "Hi",
        );

        let edit = change.to_edit(&buffer).unwrap();
        assert_eq!(edit.old_text, "Hello");
        assert_eq!(edit.new_text, "Hi");
    }
}
