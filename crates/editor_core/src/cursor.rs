//! Cursor and selection handling for text editing.

use serde::{Deserialize, Serialize};
use std::cmp::{max, min, Ordering};

/// A point in the text buffer, represented as line and column.
///
/// Both line and column are 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    /// The line number (0-indexed).
    pub line: usize,
    /// The column number (0-indexed, in characters).
    pub column: usize,
}

impl Point {
    /// Creates a new point at the given line and column.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Creates a point at the origin (0, 0).
    pub fn origin() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::origin()
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.line.cmp(&other.line) {
            Ordering::Equal => self.column.cmp(&other.column),
            other => other,
        }
    }
}

/// A selection in the text buffer, defined by an anchor and a head.
///
/// The anchor is where the selection started, and the head is the current
/// cursor position. The selection can be forwards (head > anchor) or
/// backwards (head < anchor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Selection {
    /// The starting point of the selection.
    pub anchor: Point,
    /// The current cursor position (end of selection).
    pub head: Point,
}

impl Selection {
    /// Creates a new selection with the given anchor and head.
    pub fn new(anchor: Point, head: Point) -> Self {
        Self { anchor, head }
    }

    /// Creates a collapsed selection (cursor) at the given point.
    pub fn cursor(point: Point) -> Self {
        Self {
            anchor: point,
            head: point,
        }
    }

    /// Creates a selection at the origin.
    pub fn origin() -> Self {
        Self::cursor(Point::origin())
    }

    /// Returns true if the selection is collapsed (anchor == head).
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns true if the selection is empty (same as collapsed).
    pub fn is_empty(&self) -> bool {
        self.is_collapsed()
    }

    /// Returns true if the selection is forwards (head >= anchor).
    pub fn is_forwards(&self) -> bool {
        self.head >= self.anchor
    }

    /// Returns true if the selection is backwards (head < anchor).
    pub fn is_backwards(&self) -> bool {
        self.head < self.anchor
    }

    /// Returns the start point of the selection (min of anchor and head).
    pub fn start(&self) -> Point {
        min(self.anchor, self.head)
    }

    /// Returns the end point of the selection (max of anchor and head).
    pub fn end(&self) -> Point {
        max(self.anchor, self.head)
    }

    /// Returns the selection in normalized form (start, end).
    pub fn normalized(&self) -> (Point, Point) {
        (self.start(), self.end())
    }

    /// Collapses the selection to the head position.
    pub fn collapse_to_head(&mut self) {
        self.anchor = self.head;
    }

    /// Collapses the selection to the anchor position.
    pub fn collapse_to_anchor(&mut self) {
        self.head = self.anchor;
    }

    /// Collapses the selection to the start position.
    pub fn collapse_to_start(&mut self) {
        let start = self.start();
        self.anchor = start;
        self.head = start;
    }

    /// Collapses the selection to the end position.
    pub fn collapse_to_end(&mut self) {
        let end = self.end();
        self.anchor = end;
        self.head = end;
    }

    /// Extends the selection to include the given point.
    pub fn extend_to(&mut self, point: Point) {
        self.head = point;
    }

    /// Returns true if the selection contains the given point.
    pub fn contains(&self, point: Point) -> bool {
        let (start, end) = self.normalized();
        point >= start && point <= end
    }

    /// Returns true if this selection overlaps with another.
    pub fn overlaps(&self, other: &Selection) -> bool {
        let (s1, e1) = self.normalized();
        let (s2, e2) = other.normalized();
        !(e1 < s2 || e2 < s1)
    }

    /// Merges this selection with another, returning the combined selection.
    pub fn merge(&self, other: &Selection) -> Selection {
        let start = min(self.start(), other.start());
        let end = max(self.end(), other.end());
        Selection::new(start, end)
    }

    /// Reverses the direction of the selection.
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.anchor, &mut self.head);
    }

    /// Returns a reversed copy of the selection.
    pub fn reversed(&self) -> Selection {
        Selection::new(self.head, self.anchor)
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::origin()
    }
}

/// A cursor with optional selection, supporting multi-cursor editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// The primary selection for this cursor.
    selection: Selection,
    /// The preferred column for vertical movement.
    /// This is used to maintain column position when moving up/down
    /// through lines of varying lengths.
    preferred_column: Option<usize>,
}

impl Cursor {
    /// Creates a new cursor at the given point.
    pub fn new(point: Point) -> Self {
        Self {
            selection: Selection::cursor(point),
            preferred_column: None,
        }
    }

    /// Creates a cursor with the given selection.
    pub fn with_selection(selection: Selection) -> Self {
        Self {
            selection,
            preferred_column: None,
        }
    }

    /// Returns the current cursor position (selection head).
    pub fn position(&self) -> Point {
        self.selection.head
    }

    /// Returns the selection.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Returns a mutable reference to the selection.
    pub fn selection_mut(&mut self) -> &mut Selection {
        &mut self.selection
    }

    /// Returns true if there is an active selection.
    pub fn has_selection(&self) -> bool {
        !self.selection.is_collapsed()
    }

    /// Moves the cursor to the given point, collapsing any selection.
    pub fn move_to(&mut self, point: Point) {
        self.selection = Selection::cursor(point);
        self.preferred_column = None;
    }

    /// Extends the selection to the given point.
    pub fn extend_to(&mut self, point: Point) {
        self.selection.extend_to(point);
        self.preferred_column = None;
    }

    /// Sets the selection.
    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        self.preferred_column = None;
    }

    /// Clears the selection, moving cursor to head.
    pub fn clear_selection(&mut self) {
        self.selection.collapse_to_head();
    }

    /// Returns the preferred column for vertical movement.
    pub fn preferred_column(&self) -> Option<usize> {
        self.preferred_column
    }

    /// Sets the preferred column for vertical movement.
    pub fn set_preferred_column(&mut self, column: usize) {
        self.preferred_column = Some(column);
    }

    /// Clears the preferred column.
    pub fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }

    /// Returns the effective column for vertical movement.
    /// Uses preferred column if set, otherwise the current column.
    pub fn effective_column(&self) -> usize {
        self.preferred_column.unwrap_or(self.position().column)
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(Point::origin())
    }
}

/// A collection of cursors for multi-cursor editing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CursorSet {
    /// The list of cursors, always non-empty after initialization.
    cursors: Vec<Cursor>,
    /// The index of the primary cursor.
    primary: usize,
}

impl CursorSet {
    /// Creates a new cursor set with a single cursor at the origin.
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::default()],
            primary: 0,
        }
    }

    /// Creates a cursor set with a single cursor at the given point.
    pub fn at(point: Point) -> Self {
        Self {
            cursors: vec![Cursor::new(point)],
            primary: 0,
        }
    }

    /// Returns the number of cursors.
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Returns true if there are no cursors (should never happen).
    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// Returns a reference to all cursors.
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns a mutable reference to all cursors.
    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Returns the primary cursor.
    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    /// Returns a mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    /// Returns the primary cursor index.
    pub fn primary_index(&self) -> usize {
        self.primary
    }

    /// Sets the primary cursor index.
    pub fn set_primary(&mut self, index: usize) {
        if index < self.cursors.len() {
            self.primary = index;
        }
    }

    /// Adds a new cursor at the given point.
    pub fn add_cursor(&mut self, point: Point) {
        self.cursors.push(Cursor::new(point));
    }

    /// Adds a new cursor with the given selection.
    pub fn add_cursor_with_selection(&mut self, selection: Selection) {
        self.cursors.push(Cursor::with_selection(selection));
    }

    /// Removes all cursors except the primary.
    pub fn clear_secondary(&mut self) {
        let primary = self.cursors.remove(self.primary);
        self.cursors.clear();
        self.cursors.push(primary);
        self.primary = 0;
    }

    /// Removes the cursor at the given index.
    /// If it's the last cursor, does nothing.
    pub fn remove_cursor(&mut self, index: usize) {
        if self.cursors.len() > 1 && index < self.cursors.len() {
            self.cursors.remove(index);
            if self.primary >= self.cursors.len() {
                self.primary = self.cursors.len() - 1;
            } else if self.primary > index {
                self.primary -= 1;
            }
        }
    }

    /// Merges overlapping selections and removes duplicate cursors.
    pub fn merge_overlapping(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        // Sort cursors by position
        self.cursors.sort_by(|a, b| {
            let (s1, _) = a.selection().normalized();
            let (s2, _) = b.selection().normalized();
            s1.cmp(&s2)
        });

        // Merge overlapping selections
        let mut merged = vec![self.cursors[0].clone()];
        for cursor in self.cursors.iter().skip(1) {
            let last = merged.last_mut().unwrap();
            if last.selection().overlaps(cursor.selection()) {
                let merged_selection = last.selection().merge(cursor.selection());
                last.set_selection(merged_selection);
            } else {
                merged.push(cursor.clone());
            }
        }

        self.cursors = merged;
        if self.primary >= self.cursors.len() {
            self.primary = self.cursors.len() - 1;
        }
    }

    /// Applies a function to all cursors.
    pub fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Cursor),
    {
        for cursor in &mut self.cursors {
            f(cursor);
        }
    }

    /// Moves all cursors to collapse selections.
    pub fn clear_all_selections(&mut self) {
        for cursor in &mut self.cursors {
            cursor.clear_selection();
        }
    }

    /// Returns an iterator over all selections.
    pub fn selections(&self) -> impl Iterator<Item = &Selection> {
        self.cursors.iter().map(|c| c.selection())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_ordering() {
        let p1 = Point::new(0, 5);
        let p2 = Point::new(1, 0);
        let p3 = Point::new(0, 10);

        assert!(p1 < p2);
        assert!(p1 < p3);
        assert!(p3 < p2);
    }

    #[test]
    fn test_selection_collapsed() {
        let sel = Selection::cursor(Point::new(1, 5));
        assert!(sel.is_collapsed());
        assert!(sel.is_empty());
        assert_eq!(sel.start(), sel.end());
    }

    #[test]
    fn test_selection_direction() {
        let forward = Selection::new(Point::new(0, 0), Point::new(1, 5));
        assert!(forward.is_forwards());
        assert!(!forward.is_backwards());

        let backward = Selection::new(Point::new(1, 5), Point::new(0, 0));
        assert!(!backward.is_forwards());
        assert!(backward.is_backwards());
    }

    #[test]
    fn test_selection_contains() {
        let sel = Selection::new(Point::new(1, 0), Point::new(3, 10));

        assert!(sel.contains(Point::new(2, 5)));
        assert!(sel.contains(Point::new(1, 0)));
        assert!(sel.contains(Point::new(3, 10)));
        assert!(!sel.contains(Point::new(0, 5)));
        assert!(!sel.contains(Point::new(4, 0)));
    }

    #[test]
    fn test_selection_overlaps() {
        let s1 = Selection::new(Point::new(0, 0), Point::new(2, 0));
        let s2 = Selection::new(Point::new(1, 0), Point::new(3, 0));
        let s3 = Selection::new(Point::new(5, 0), Point::new(6, 0));

        assert!(s1.overlaps(&s2));
        assert!(s2.overlaps(&s1));
        assert!(!s1.overlaps(&s3));
    }

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::new(Point::new(0, 0));

        cursor.move_to(Point::new(5, 10));
        assert_eq!(cursor.position(), Point::new(5, 10));
        assert!(!cursor.has_selection());

        cursor.extend_to(Point::new(7, 5));
        assert!(cursor.has_selection());
        assert_eq!(cursor.selection().anchor, Point::new(5, 10));
        assert_eq!(cursor.selection().head, Point::new(7, 5));
    }

    #[test]
    fn test_cursor_set_operations() {
        let mut set = CursorSet::new();
        assert_eq!(set.len(), 1);

        set.add_cursor(Point::new(5, 0));
        set.add_cursor(Point::new(10, 0));
        assert_eq!(set.len(), 3);

        set.clear_secondary();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_cursor_set_merge_overlapping() {
        let mut set = CursorSet::new();
        set.primary_mut().set_selection(Selection::new(
            Point::new(0, 0),
            Point::new(2, 0),
        ));
        set.add_cursor_with_selection(Selection::new(
            Point::new(1, 0),
            Point::new(3, 0),
        ));
        set.add_cursor_with_selection(Selection::new(
            Point::new(10, 0),
            Point::new(12, 0),
        ));

        assert_eq!(set.len(), 3);
        set.merge_overlapping();
        assert_eq!(set.len(), 2);
    }
}
