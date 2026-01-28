//! Diagnostics storage and management.
//!
//! This module provides storage and querying capabilities for LSP diagnostics,
//! organizing them by file and providing filtering by severity and range.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Uri};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

/// Storage for diagnostics across all files.
pub struct DiagnosticStore {
    /// Diagnostics keyed by file URI.
    diagnostics: RwLock<HashMap<Uri, Vec<Diagnostic>>>,
    /// Callback for diagnostic updates.
    on_update: RwLock<Option<Arc<dyn Fn(&Uri, &[Diagnostic]) + Send + Sync>>>,
}

impl fmt::Debug for DiagnosticStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiagnosticStore")
            .field("diagnostics", &self.diagnostics)
            .field("on_update", &"<callback>")
            .finish()
    }
}

impl Default for DiagnosticStore {
    fn default() -> Self {
        Self {
            diagnostics: RwLock::new(HashMap::new()),
            on_update: RwLock::new(None),
        }
    }
}

impl DiagnosticStore {
    /// Create a new diagnostic store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a callback to be notified when diagnostics are updated.
    pub fn set_update_callback<F>(&self, callback: F)
    where
        F: Fn(&Uri, &[Diagnostic]) + Send + Sync + 'static,
    {
        *self.on_update.write() = Some(Arc::new(callback));
    }

    /// Update diagnostics for a file.
    pub fn set_diagnostics(&self, uri: Uri, diagnostics: Vec<Diagnostic>) {
        let callback = self.on_update.read().clone();

        self.diagnostics.write().insert(uri.clone(), diagnostics.clone());

        if let Some(callback) = callback {
            callback(&uri, &diagnostics);
        }
    }

    /// Get diagnostics for a file.
    pub fn get_diagnostics(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.diagnostics
            .read()
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    /// Get diagnostics for a file by path.
    pub fn get_diagnostics_for_path(&self, path: &Path) -> Vec<Diagnostic> {
        if let Some(uri) = path_to_uri(path) {
            self.get_diagnostics(&uri)
        } else {
            Vec::new()
        }
    }

    /// Clear diagnostics for a file.
    pub fn clear_diagnostics(&self, uri: &Uri) {
        let callback = self.on_update.read().clone();

        self.diagnostics.write().remove(uri);

        if let Some(callback) = callback {
            callback(uri, &[]);
        }
    }

    /// Clear all diagnostics.
    pub fn clear_all(&self) {
        self.diagnostics.write().clear();
    }

    /// Get all files with diagnostics.
    pub fn files_with_diagnostics(&self) -> Vec<Uri> {
        self.diagnostics.read().keys().cloned().collect()
    }

    /// Get diagnostic count for a file.
    pub fn diagnostic_count(&self, uri: &Uri) -> usize {
        self.diagnostics
            .read()
            .get(uri)
            .map(|d: &Vec<Diagnostic>| d.len())
            .unwrap_or(0)
    }

    /// Get total diagnostic count across all files.
    pub fn total_diagnostic_count(&self) -> usize {
        self.diagnostics.read().values().map(|d: &Vec<Diagnostic>| d.len()).sum()
    }

    /// Get diagnostics filtered by severity.
    pub fn get_by_severity(&self, uri: &Uri, severity: DiagnosticSeverity) -> Vec<Diagnostic> {
        self.get_diagnostics(uri)
            .into_iter()
            .filter(|d| d.severity == Some(severity))
            .collect()
    }

    /// Get error diagnostics for a file.
    pub fn get_errors(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.get_by_severity(uri, DiagnosticSeverity::ERROR)
    }

    /// Get warning diagnostics for a file.
    pub fn get_warnings(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.get_by_severity(uri, DiagnosticSeverity::WARNING)
    }

    /// Get info diagnostics for a file.
    pub fn get_info(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.get_by_severity(uri, DiagnosticSeverity::INFORMATION)
    }

    /// Get hint diagnostics for a file.
    pub fn get_hints(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.get_by_severity(uri, DiagnosticSeverity::HINT)
    }

    /// Get diagnostics at a specific position.
    pub fn get_at_position(&self, uri: &Uri, position: Position) -> Vec<Diagnostic> {
        self.get_diagnostics(uri)
            .into_iter()
            .filter(|d| position_in_range(position, d.range))
            .collect()
    }

    /// Get diagnostics that overlap with a range.
    pub fn get_in_range(&self, uri: &Uri, range: Range) -> Vec<Diagnostic> {
        self.get_diagnostics(uri)
            .into_iter()
            .filter(|d| ranges_overlap(d.range, range))
            .collect()
    }

    /// Get diagnostics on a specific line.
    pub fn get_on_line(&self, uri: &Uri, line: u32) -> Vec<Diagnostic> {
        self.get_diagnostics(uri)
            .into_iter()
            .filter(|d| d.range.start.line <= line && d.range.end.line >= line)
            .collect()
    }

    /// Count errors across all files.
    pub fn total_error_count(&self) -> usize {
        self.diagnostics
            .read()
            .values()
            .flat_map(|d: &Vec<Diagnostic>| d.iter())
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .count()
    }

    /// Count warnings across all files.
    pub fn total_warning_count(&self) -> usize {
        self.diagnostics
            .read()
            .values()
            .flat_map(|d: &Vec<Diagnostic>| d.iter())
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .count()
    }

    /// Get a summary of diagnostics.
    pub fn summary(&self) -> DiagnosticSummary {
        let guard = self.diagnostics.read();
        let mut summary = DiagnosticSummary::default();

        summary.file_count = guard.len();

        for diagnostics in guard.values() {
            for diagnostic in diagnostics {
                match diagnostic.severity {
                    Some(DiagnosticSeverity::ERROR) => summary.error_count += 1,
                    Some(DiagnosticSeverity::WARNING) => summary.warning_count += 1,
                    Some(DiagnosticSeverity::INFORMATION) => summary.info_count += 1,
                    Some(DiagnosticSeverity::HINT) => summary.hint_count += 1,
                    _ => summary.other_count += 1,
                }
            }
        }

        summary
    }
}

/// Summary of diagnostic counts.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticSummary {
    /// Number of files with diagnostics.
    pub file_count: usize,
    /// Number of error diagnostics.
    pub error_count: usize,
    /// Number of warning diagnostics.
    pub warning_count: usize,
    /// Number of info diagnostics.
    pub info_count: usize,
    /// Number of hint diagnostics.
    pub hint_count: usize,
    /// Number of diagnostics with unknown severity.
    pub other_count: usize,
}

impl DiagnosticSummary {
    /// Get total diagnostic count.
    pub fn total(&self) -> usize {
        self.error_count + self.warning_count + self.info_count + self.hint_count + self.other_count
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }
}

/// Check if a position is within a range.
fn position_in_range(pos: Position, range: Range) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }

    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }

    if pos.line == range.end.line && pos.character > range.end.character {
        return false;
    }

    true
}

/// Check if two ranges overlap.
fn ranges_overlap(a: Range, b: Range) -> bool {
    // a ends before b starts
    if a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
    {
        return false;
    }

    // a starts after b ends
    if a.start.line > b.end.line
        || (a.start.line == b.end.line && a.start.character > b.end.character)
    {
        return false;
    }

    true
}

/// Convert a path to a URI.
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    let path_str = path.to_str()?;

    // Create a file:// URI from the path
    #[cfg(windows)]
    let uri_string = format!("file:///{}", path_str.replace('\\', "/"));
    #[cfg(not(windows))]
    let uri_string = format!("file://{}", path_str);

    Uri::from_str(&uri_string).ok()
}

/// Convert a URI to a path.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let uri_str = uri.as_str();

    if !uri_str.starts_with("file://") {
        return None;
    }

    let path_str = uri_str.strip_prefix("file://")?;

    // Handle Windows paths (file:///C:/...)
    #[cfg(windows)]
    let path_str = path_str.strip_prefix('/').unwrap_or(path_str);

    Some(PathBuf::from(path_str))
}

/// Create a diagnostic range from line and column positions.
pub fn make_range(
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> Range {
    Range {
        start: Position {
            line: start_line,
            character: start_col,
        },
        end: Position {
            line: end_line,
            character: end_col,
        },
    }
}

/// Create a single-line diagnostic range.
pub fn make_line_range(line: u32, start_col: u32, end_col: u32) -> Range {
    make_range(line, start_col, line, end_col)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_diagnostic(line: u32, severity: DiagnosticSeverity, message: &str) -> Diagnostic {
        Diagnostic {
            range: make_line_range(line, 0, 10),
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("test".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn make_test_uri(name: &str) -> Uri {
        Uri::from_str(&format!("file:///test/{}", name)).unwrap()
    }

    #[test]
    fn test_diagnostic_store() {
        let store = DiagnosticStore::new();
        let uri = make_test_uri("test.rs");

        let diagnostics = vec![
            make_diagnostic(0, DiagnosticSeverity::ERROR, "error 1"),
            make_diagnostic(1, DiagnosticSeverity::WARNING, "warning 1"),
        ];

        store.set_diagnostics(uri.clone(), diagnostics);

        assert_eq!(store.diagnostic_count(&uri), 2);
        assert_eq!(store.get_errors(&uri).len(), 1);
        assert_eq!(store.get_warnings(&uri).len(), 1);
    }

    #[test]
    fn test_diagnostic_on_line() {
        let store = DiagnosticStore::new();
        let uri = make_test_uri("test.rs");

        let diagnostics = vec![
            make_diagnostic(0, DiagnosticSeverity::ERROR, "line 0"),
            make_diagnostic(5, DiagnosticSeverity::WARNING, "line 5"),
            make_diagnostic(10, DiagnosticSeverity::ERROR, "line 10"),
        ];

        store.set_diagnostics(uri.clone(), diagnostics);

        assert_eq!(store.get_on_line(&uri, 0).len(), 1);
        assert_eq!(store.get_on_line(&uri, 5).len(), 1);
        assert_eq!(store.get_on_line(&uri, 3).len(), 0);
    }

    #[test]
    fn test_diagnostic_summary() {
        let store = DiagnosticStore::new();
        let uri1 = make_test_uri("test1.rs");
        let uri2 = make_test_uri("test2.rs");

        store.set_diagnostics(
            uri1,
            vec![
                make_diagnostic(0, DiagnosticSeverity::ERROR, "error"),
                make_diagnostic(1, DiagnosticSeverity::ERROR, "error"),
            ],
        );

        store.set_diagnostics(
            uri2,
            vec![make_diagnostic(0, DiagnosticSeverity::WARNING, "warning")],
        );

        let summary = store.summary();
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.error_count, 2);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.total(), 3);
    }

    #[test]
    fn test_position_in_range() {
        let range = make_range(5, 10, 5, 20);

        assert!(position_in_range(Position { line: 5, character: 15 }, range));
        assert!(position_in_range(Position { line: 5, character: 10 }, range));
        assert!(position_in_range(Position { line: 5, character: 20 }, range));
        assert!(!position_in_range(Position { line: 5, character: 9 }, range));
        assert!(!position_in_range(Position { line: 5, character: 21 }, range));
        assert!(!position_in_range(Position { line: 4, character: 15 }, range));
    }

    #[test]
    fn test_ranges_overlap() {
        let a = make_range(5, 0, 10, 0);
        let b = make_range(8, 0, 15, 0);
        let c = make_range(0, 0, 3, 0);

        assert!(ranges_overlap(a, b));
        assert!(ranges_overlap(b, a));
        assert!(!ranges_overlap(a, c));
        assert!(!ranges_overlap(c, a));
    }

    #[test]
    fn test_path_to_uri() {
        let path = Path::new("/home/user/test.rs");
        let uri = path_to_uri(path);
        assert!(uri.is_some());
        assert!(uri.unwrap().as_str().contains("test.rs"));
    }
}
