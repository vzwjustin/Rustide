//! Undo/redo history management with transaction support.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::edit::Edit;
use crate::{EditorError, Result};

/// A transaction is a group of edits that should be undone/redone together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// The edits in this transaction, in order of application.
    edits: Vec<Edit>,
    /// An optional description of this transaction.
    description: Option<String>,
    /// Timestamp when the transaction was created (as duration since UNIX epoch).
    #[serde(skip)]
    timestamp: Option<Instant>,
}

impl Transaction {
    /// Creates a new empty transaction.
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            description: None,
            timestamp: Some(Instant::now()),
        }
    }

    /// Creates a transaction with a description.
    pub fn with_description(description: impl Into<String>) -> Self {
        Self {
            edits: Vec::new(),
            description: Some(description.into()),
            timestamp: Some(Instant::now()),
        }
    }

    /// Creates a transaction from a single edit.
    pub fn from_edit(edit: Edit) -> Self {
        Self {
            edits: vec![edit],
            description: None,
            timestamp: Some(Instant::now()),
        }
    }

    /// Adds an edit to this transaction.
    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    /// Returns the edits in this transaction.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Returns the number of edits in this transaction.
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Returns true if this transaction has no edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Returns the description of this transaction.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the description of this transaction.
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    /// Returns the timestamp when this transaction was created.
    pub fn timestamp(&self) -> Option<Instant> {
        self.timestamp
    }

    /// Returns the inverse of this transaction (for undo).
    pub fn inverse(&self) -> Transaction {
        Transaction {
            edits: self.edits.iter().rev().map(|e| e.inverse()).collect(),
            description: self.description.clone().map(|d| format!("Undo: {}", d)),
            timestamp: Some(Instant::now()),
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// History manager for undo/redo operations.
///
/// Maintains a stack of transactions for undo and a separate stack for redo.
/// Supports transaction grouping for combining multiple edits into a single
/// undo/redo operation.
#[derive(Debug, Clone, Default)]
pub struct History {
    /// Stack of transactions that can be undone.
    undo_stack: Vec<Transaction>,
    /// Stack of transactions that can be redone.
    redo_stack: Vec<Transaction>,
    /// Current transaction being built (for grouping).
    current_transaction: Option<Transaction>,
    /// Maximum number of transactions to keep.
    max_history: usize,
    /// Time threshold for auto-grouping edits (in milliseconds).
    group_timeout: Duration,
    /// Last edit timestamp for auto-grouping.
    last_edit_time: Option<Instant>,
}

impl History {
    /// Creates a new history manager.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: None,
            max_history: 1000,
            group_timeout: Duration::from_millis(500),
            last_edit_time: None,
        }
    }

    /// Creates a history manager with a custom maximum size.
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            max_history,
            ..Self::new()
        }
    }

    /// Returns true if there are transactions that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() || self.current_transaction.is_some()
    }

    /// Returns true if there are transactions that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the number of undo transactions available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len() + if self.current_transaction.is_some() { 1 } else { 0 }
    }

    /// Returns the number of redo transactions available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Begins a new transaction for grouping edits.
    pub fn begin_transaction(&mut self) {
        // If there's a current transaction, commit it first
        if let Some(transaction) = self.current_transaction.take() {
            if !transaction.is_empty() {
                self.push_transaction(transaction);
            }
        }
        self.current_transaction = Some(Transaction::new());
    }

    /// Begins a transaction with a description.
    pub fn begin_transaction_with_description(&mut self, description: impl Into<String>) {
        if let Some(transaction) = self.current_transaction.take() {
            if !transaction.is_empty() {
                self.push_transaction(transaction);
            }
        }
        self.current_transaction = Some(Transaction::with_description(description));
    }

    /// Ends the current transaction.
    pub fn end_transaction(&mut self) {
        if let Some(transaction) = self.current_transaction.take() {
            if !transaction.is_empty() {
                self.push_transaction(transaction);
            }
        }
    }

    /// Returns true if currently in a transaction.
    pub fn in_transaction(&self) -> bool {
        self.current_transaction.is_some()
    }

    /// Pushes an edit to the history.
    ///
    /// If in a transaction, adds to the current transaction.
    /// Otherwise, creates a new transaction or auto-groups with recent edits.
    pub fn push_edit(&mut self, edit: Edit) {
        // Clear redo stack when new edits are made
        self.redo_stack.clear();

        if let Some(ref mut transaction) = self.current_transaction {
            // Add to current transaction
            transaction.push(edit);
        } else {
            // Check if we should auto-group with the last transaction
            let should_group = self.should_auto_group();

            if should_group {
                if let Some(last) = self.undo_stack.last_mut() {
                    last.push(edit);
                    self.last_edit_time = Some(Instant::now());
                    return;
                }
            }

            // Create a new transaction
            let transaction = Transaction::from_edit(edit);
            self.push_transaction(transaction);
        }

        self.last_edit_time = Some(Instant::now());
    }

    /// Checks if a new edit should be auto-grouped with the previous transaction.
    fn should_auto_group(&self) -> bool {
        if let Some(last_time) = self.last_edit_time {
            let elapsed = last_time.elapsed();
            return elapsed < self.group_timeout;
        }
        false
    }

    /// Pushes a transaction to the undo stack.
    fn push_transaction(&mut self, transaction: Transaction) {
        self.undo_stack.push(transaction);

        // Trim history if it exceeds the maximum
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undoes the last transaction.
    ///
    /// Returns the edits that were undone.
    pub fn undo(&mut self) -> Result<Vec<Edit>> {
        // First, commit any current transaction
        if let Some(transaction) = self.current_transaction.take() {
            if !transaction.is_empty() {
                self.push_transaction(transaction);
            }
        }

        if let Some(transaction) = self.undo_stack.pop() {
            let edits = transaction.edits().to_vec();
            self.redo_stack.push(transaction);
            Ok(edits)
        } else {
            Err(EditorError::NoUndoHistory)
        }
    }

    /// Redoes the last undone transaction.
    ///
    /// Returns the edits that were redone.
    pub fn redo(&mut self) -> Result<Vec<Edit>> {
        if let Some(transaction) = self.redo_stack.pop() {
            let edits = transaction.edits().to_vec();
            self.undo_stack.push(transaction);
            Ok(edits)
        } else {
            Err(EditorError::NoRedoHistory)
        }
    }

    /// Clears all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_transaction = None;
        self.last_edit_time = None;
    }

    /// Sets the maximum number of transactions to keep.
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max;
        while self.undo_stack.len() > max {
            self.undo_stack.remove(0);
        }
    }

    /// Sets the timeout for auto-grouping edits.
    pub fn set_group_timeout(&mut self, timeout: Duration) {
        self.group_timeout = timeout;
    }

    /// Returns the last transaction in the undo stack (for inspection).
    pub fn last_transaction(&self) -> Option<&Transaction> {
        if let Some(ref current) = self.current_transaction {
            if !current.is_empty() {
                return Some(current);
            }
        }
        self.undo_stack.last()
    }

    /// Merges the last N transactions into one.
    pub fn merge_last(&mut self, count: usize) {
        if count <= 1 || self.undo_stack.len() < count {
            return;
        }

        let mut merged = Transaction::new();
        for _ in 0..count {
            if let Some(transaction) = self.undo_stack.pop() {
                for edit in transaction.edits().iter().rev() {
                    merged.edits.insert(0, edit.clone());
                }
            }
        }

        if !merged.is_empty() {
            self.undo_stack.push(merged);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::EditKind;

    fn make_insert(offset: usize, text: &str) -> Edit {
        Edit {
            kind: EditKind::Insert,
            offset,
            old_text: String::new(),
            new_text: text.to_string(),
        }
    }

    #[allow(dead_code)]
    fn make_delete(offset: usize, text: &str) -> Edit {
        Edit {
            kind: EditKind::Delete,
            offset,
            old_text: text.to_string(),
            new_text: String::new(),
        }
    }

    #[test]
    fn test_empty_history() {
        let history = History::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_push_and_undo() {
        let mut history = History::new();

        history.push_edit(make_insert(0, "Hello"));
        assert!(history.can_undo());

        // Wait to avoid auto-grouping
        std::thread::sleep(Duration::from_millis(600));

        history.push_edit(make_insert(5, " World"));
        assert_eq!(history.undo_count(), 2);

        let edits = history.undo().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, " World");

        assert!(history.can_redo());
    }

    #[test]
    fn test_redo() {
        let mut history = History::new();

        history.push_edit(make_insert(0, "Hello"));
        history.undo().unwrap();

        assert!(history.can_redo());
        let edits = history.redo().unwrap();
        assert_eq!(edits[0].new_text, "Hello");
        assert!(!history.can_redo());
    }

    #[test]
    fn test_redo_cleared_on_edit() {
        let mut history = History::new();

        history.push_edit(make_insert(0, "Hello"));
        history.undo().unwrap();
        assert!(history.can_redo());

        history.push_edit(make_insert(0, "World"));
        assert!(!history.can_redo());
    }

    #[test]
    fn test_transaction_grouping() {
        let mut history = History::new();

        history.begin_transaction();
        history.push_edit(make_insert(0, "Hello"));
        history.push_edit(make_insert(5, " World"));
        history.end_transaction();

        assert_eq!(history.undo_count(), 1);

        let edits = history.undo().unwrap();
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_nested_transactions() {
        let mut history = History::new();

        history.begin_transaction();
        history.push_edit(make_insert(0, "A"));
        history.begin_transaction(); // This should commit the first transaction
        history.push_edit(make_insert(1, "B"));
        history.end_transaction();

        assert_eq!(history.undo_count(), 2);
    }

    #[test]
    fn test_max_history() {
        let mut history = History::with_max_history(3);

        for i in 0..5 {
            // Use sleep to avoid auto-grouping
            std::thread::sleep(Duration::from_millis(600));
            history.push_edit(make_insert(i, &i.to_string()));
        }

        // Should only keep the last 3 transactions
        assert!(history.undo_count() <= 3);
    }

    #[test]
    fn test_transaction_inverse() {
        let mut transaction = Transaction::new();
        transaction.push(make_insert(0, "Hello"));
        transaction.push(make_insert(5, " World"));

        let inverse = transaction.inverse();
        assert_eq!(inverse.len(), 2);

        // Inverse edits should be in reverse order
        assert_eq!(inverse.edits()[0].offset, 5);
        assert_eq!(inverse.edits()[1].offset, 0);
    }

    #[test]
    fn test_clear_history() {
        let mut history = History::new();
        history.push_edit(make_insert(0, "Hello"));

        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
