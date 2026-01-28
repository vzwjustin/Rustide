//! Problem matching for build output

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Severity of a problem
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Default for ProblemSeverity {
    fn default() -> Self {
        Self::Error
    }
}

/// A problem/diagnostic found in build output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    /// Severity of the problem
    pub severity: ProblemSeverity,
    /// Problem message
    pub message: String,
    /// Source file
    pub file: Option<PathBuf>,
    /// Line number (1-indexed)
    pub line: Option<u32>,
    /// Column number (1-indexed)
    pub column: Option<u32>,
    /// End line (for ranges)
    pub end_line: Option<u32>,
    /// End column (for ranges)
    pub end_column: Option<u32>,
    /// Error code
    pub code: Option<String>,
    /// Source of the problem (e.g., "rustc", "clippy")
    pub source: Option<String>,
}

impl Problem {
    /// Create a new error
    pub fn error(message: &str) -> Self {
        Self {
            severity: ProblemSeverity::Error,
            message: message.to_string(),
            file: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            code: None,
            source: None,
        }
    }

    /// Create a new warning
    pub fn warning(message: &str) -> Self {
        Self {
            severity: ProblemSeverity::Warning,
            message: message.to_string(),
            file: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            code: None,
            source: None,
        }
    }

    /// Set file location
    pub fn with_location(mut self, file: PathBuf, line: u32, column: Option<u32>) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self.column = column;
        self
    }

    /// Set error code
    pub fn with_code(mut self, code: &str) -> Self {
        self.code = Some(code.to_string());
        self
    }

    /// Set source
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    /// Check if this is an error
    pub fn is_error(&self) -> bool {
        matches!(self.severity, ProblemSeverity::Error)
    }

    /// Check if this is a warning
    pub fn is_warning(&self) -> bool {
        matches!(self.severity, ProblemSeverity::Warning)
    }
}

/// Trait for problem matchers
pub trait ProblemMatcher: Send + Sync {
    /// Match a line of output, returning a problem if found
    fn match_line(&self, line: &str) -> Option<Problem>;

    /// Reset the matcher state
    fn reset(&mut self);
}

/// Problem matcher for Rust compiler output
pub struct RustProblemMatcher {
    /// Pattern for error/warning lines
    error_pattern: Regex,
    /// Pattern for note/help lines
    note_pattern: Regex,
    /// Pattern for location lines
    location_pattern: Regex,
    /// Current problem being built
    current_problem: Option<Problem>,
}

impl RustProblemMatcher {
    /// Create a new Rust problem matcher
    pub fn new() -> Self {
        // Match: error[E0382]: borrow of moved value
        // Match: warning: unused variable
        let error_pattern = Regex::new(
            r"^(error|warning)(\[E\d+\])?:\s+(.+)$"
        ).expect("Invalid error pattern");

        // Match: note: ...
        // Match: help: ...
        let note_pattern = Regex::new(
            r"^\s+(note|help):\s+(.+)$"
        ).expect("Invalid note pattern");

        // Match: --> src/main.rs:10:5
        let location_pattern = Regex::new(
            r"^\s*-->\s+(.+):(\d+):(\d+)$"
        ).expect("Invalid location pattern");

        Self {
            error_pattern,
            note_pattern,
            location_pattern,
            current_problem: None,
        }
    }
}

impl Default for RustProblemMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProblemMatcher for RustProblemMatcher {
    fn match_line(&self, line: &str) -> Option<Problem> {
        // Try to match error/warning
        if let Some(caps) = self.error_pattern.captures(line) {
            let severity = match caps.get(1).map(|m| m.as_str()) {
                Some("error") => ProblemSeverity::Error,
                Some("warning") => ProblemSeverity::Warning,
                _ => ProblemSeverity::Error,
            };

            let code = caps.get(2).map(|m| {
                m.as_str().trim_start_matches('[').trim_end_matches(']').to_string()
            });

            let message = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            let mut problem = Problem {
                severity,
                message,
                file: None,
                line: None,
                column: None,
                end_line: None,
                end_column: None,
                code,
                source: Some("rustc".to_string()),
            };

            return Some(problem);
        }

        // Try to match location
        if let Some(caps) = self.location_pattern.captures(line) {
            let file = caps.get(1).map(|m| PathBuf::from(m.as_str()));
            let line_num: Option<u32> = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col: Option<u32> = caps.get(3).and_then(|m| m.as_str().parse().ok());

            if let (Some(file), Some(line_num)) = (file, line_num) {
                // This is location info - would need state to associate with problem
                // For simplicity, we create a placeholder problem
                return None;
            }
        }

        None
    }

    fn reset(&mut self) {
        self.current_problem = None;
    }
}

/// Problem matcher for GCC/Clang output
pub struct GccProblemMatcher {
    /// Pattern for error/warning lines
    pattern: Regex,
}

impl GccProblemMatcher {
    /// Create a new GCC problem matcher
    pub fn new() -> Self {
        // Match: file.c:10:5: error: message
        let pattern = Regex::new(
            r"^(.+):(\d+):(\d+):\s+(error|warning|note):\s+(.+)$"
        ).expect("Invalid gcc pattern");

        Self { pattern }
    }
}

impl Default for GccProblemMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProblemMatcher for GccProblemMatcher {
    fn match_line(&self, line: &str) -> Option<Problem> {
        if let Some(caps) = self.pattern.captures(line) {
            let file = caps.get(1).map(|m| PathBuf::from(m.as_str()));
            let line_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col = caps.get(3).and_then(|m| m.as_str().parse().ok());

            let severity = match caps.get(4).map(|m| m.as_str()) {
                Some("error") => ProblemSeverity::Error,
                Some("warning") => ProblemSeverity::Warning,
                Some("note") => ProblemSeverity::Info,
                _ => ProblemSeverity::Error,
            };

            let message = caps.get(5).map(|m| m.as_str()).unwrap_or("").to_string();

            return Some(Problem {
                severity,
                message,
                file,
                line: line_num,
                column: col,
                end_line: None,
                end_column: None,
                code: None,
                source: Some("gcc".to_string()),
            });
        }

        None
    }

    fn reset(&mut self) {}
}

/// Problem matcher for generic output (file:line:message)
pub struct GenericProblemMatcher {
    pattern: Regex,
}

impl GenericProblemMatcher {
    /// Create a new generic problem matcher
    pub fn new() -> Self {
        // Match: file:line: message
        // Match: file:line:col: message
        let pattern = Regex::new(
            r"^(.+):(\d+)(?::(\d+))?:\s*(.+)$"
        ).expect("Invalid generic pattern");

        Self { pattern }
    }
}

impl Default for GenericProblemMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProblemMatcher for GenericProblemMatcher {
    fn match_line(&self, line: &str) -> Option<Problem> {
        // Skip lines that don't look like problems
        let lower = line.to_lowercase();
        if !lower.contains("error") && !lower.contains("warning") && !lower.contains("fail") {
            return None;
        }

        if let Some(caps) = self.pattern.captures(line) {
            let file = caps.get(1).map(|m| PathBuf::from(m.as_str()));
            let line_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col = caps.get(3).and_then(|m| m.as_str().parse().ok());
            let message = caps.get(4).map(|m| m.as_str()).unwrap_or("").to_string();

            let severity = if lower.contains("error") || lower.contains("fail") {
                ProblemSeverity::Error
            } else {
                ProblemSeverity::Warning
            };

            return Some(Problem {
                severity,
                message,
                file,
                line: line_num,
                column: col,
                end_line: None,
                end_column: None,
                code: None,
                source: None,
            });
        }

        None
    }

    fn reset(&mut self) {}
}

/// Create a problem matcher by name
pub fn create_matcher(name: &str) -> Box<dyn ProblemMatcher> {
    match name {
        "rust" | "rustc" | "cargo" => Box::new(RustProblemMatcher::new()),
        "gcc" | "clang" | "cc" => Box::new(GccProblemMatcher::new()),
        _ => Box::new(GenericProblemMatcher::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_error_match() {
        let matcher = RustProblemMatcher::new();
        let line = "error[E0382]: borrow of moved value: `x`";

        let problem = matcher.match_line(line).unwrap();
        assert!(problem.is_error());
        assert_eq!(problem.code, Some("E0382".to_string()));
        assert!(problem.message.contains("borrow of moved value"));
    }

    #[test]
    fn test_rust_warning_match() {
        let matcher = RustProblemMatcher::new();
        let line = "warning: unused variable: `x`";

        let problem = matcher.match_line(line).unwrap();
        assert!(problem.is_warning());
        assert!(problem.message.contains("unused variable"));
    }

    #[test]
    fn test_gcc_error_match() {
        let matcher = GccProblemMatcher::new();
        let line = "main.c:10:5: error: expected ';' before 'return'";

        let problem = matcher.match_line(line).unwrap();
        assert!(problem.is_error());
        assert_eq!(problem.file, Some(PathBuf::from("main.c")));
        assert_eq!(problem.line, Some(10));
        assert_eq!(problem.column, Some(5));
    }

    #[test]
    fn test_problem_builder() {
        let problem = Problem::error("test error")
            .with_location(PathBuf::from("test.rs"), 10, Some(5))
            .with_code("E0001")
            .with_source("rustc");

        assert!(problem.is_error());
        assert_eq!(problem.file, Some(PathBuf::from("test.rs")));
        assert_eq!(problem.line, Some(10));
        assert_eq!(problem.code, Some("E0001".to_string()));
    }

    #[test]
    fn test_create_matcher() {
        let matcher = create_matcher("rust");
        let problem = matcher.match_line("error: test");
        assert!(problem.is_some());
    }
}
