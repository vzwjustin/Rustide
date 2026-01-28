//! Git diff operations

use git2::{DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::Path;
use crate::repo::GitError;

/// Type of line change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineChange {
    /// Line was added
    Addition,
    /// Line was deleted
    Deletion,
    /// Line is context (unchanged)
    Context,
    /// Header line
    Header,
}

/// A line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    /// Type of change
    pub change: LineChange,
    /// Line content
    pub content: String,
    /// Old line number (for context and deletions)
    pub old_line: Option<u32>,
    /// New line number (for context and additions)
    pub new_line: Option<u32>,
}

impl DiffLine {
    /// Create a new diff line
    pub fn new(change: LineChange, content: &str) -> Self {
        Self {
            change,
            content: content.to_string(),
            old_line: None,
            new_line: None,
        }
    }

    /// Set old line number
    pub fn with_old_line(mut self, line: u32) -> Self {
        self.old_line = Some(line);
        self
    }

    /// Set new line number
    pub fn with_new_line(mut self, line: u32) -> Self {
        self.new_line = Some(line);
        self
    }
}

/// A hunk in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Starting line in old file
    pub old_start: u32,
    /// Number of lines in old file
    pub old_lines: u32,
    /// Starting line in new file
    pub new_start: u32,
    /// Number of lines in new file
    pub new_lines: u32,
    /// Header text
    pub header: String,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    /// Create a new hunk
    pub fn new(old_start: u32, old_lines: u32, new_start: u32, new_lines: u32) -> Self {
        Self {
            old_start,
            old_lines,
            new_start,
            new_lines,
            header: format!("@@ -{},{} +{},{} @@", old_start, old_lines, new_start, new_lines),
            lines: Vec::new(),
        }
    }

    /// Add a line to the hunk
    pub fn add_line(&mut self, line: DiffLine) {
        self.lines.push(line);
    }

    /// Get additions count
    pub fn additions(&self) -> usize {
        self.lines.iter().filter(|l| l.change == LineChange::Addition).count()
    }

    /// Get deletions count
    pub fn deletions(&self) -> usize {
        self.lines.iter().filter(|l| l.change == LineChange::Deletion).count()
    }
}

/// Diff for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Old file path
    pub old_path: Option<String>,
    /// New file path
    pub new_path: Option<String>,
    /// Whether file is binary
    pub is_binary: bool,
    /// Hunks in this diff
    pub hunks: Vec<DiffHunk>,
    /// Total additions
    pub additions: usize,
    /// Total deletions
    pub deletions: usize,
}

impl FileDiff {
    /// Create a new file diff
    pub fn new(old_path: Option<&str>, new_path: Option<&str>) -> Self {
        Self {
            old_path: old_path.map(|s| s.to_string()),
            new_path: new_path.map(|s| s.to_string()),
            is_binary: false,
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
        }
    }

    /// Get the file path (prefers new path)
    pub fn path(&self) -> Option<&str> {
        self.new_path.as_deref().or(self.old_path.as_deref())
    }

    /// Add a hunk
    pub fn add_hunk(&mut self, hunk: DiffHunk) {
        self.additions += hunk.additions();
        self.deletions += hunk.deletions();
        self.hunks.push(hunk);
    }

    /// Get diff for a specific file
    pub fn for_file(repo: &Repository, path: &Path) -> Result<Self, GitError> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        // Get diff between index and workdir
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;

        let file_diff = RefCell::new(FileDiff::new(None, path.to_str()));

        diff.foreach(
            &mut |delta, _| {
                let mut fd = file_diff.borrow_mut();
                fd.old_path = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
                fd.new_path = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
                fd.is_binary = delta.old_file().is_binary() || delta.new_file().is_binary();
                true
            },
            Some(&mut |_, _| true),
            Some(&mut |_, hunk| {
                let diff_hunk = DiffHunk::new(
                    hunk.old_start(),
                    hunk.old_lines(),
                    hunk.new_start(),
                    hunk.new_lines(),
                );
                file_diff.borrow_mut().hunks.push(diff_hunk);
                true
            }),
            Some(&mut |_, _, line| {
                let mut fd = file_diff.borrow_mut();
                if let Some(hunk) = fd.hunks.last_mut() {
                    let change = match line.origin() {
                        '+' => LineChange::Addition,
                        '-' => LineChange::Deletion,
                        ' ' => LineChange::Context,
                        _ => LineChange::Header,
                    };

                    let content = std::str::from_utf8(line.content())
                        .unwrap_or("")
                        .trim_end_matches('\n')
                        .to_string();

                    let mut diff_line = DiffLine::new(change, &content);

                    if let Some(old) = line.old_lineno() {
                        diff_line = diff_line.with_old_line(old);
                    }
                    if let Some(new) = line.new_lineno() {
                        diff_line = diff_line.with_new_line(new);
                    }

                    hunk.add_line(diff_line);
                }
                true
            }),
        )?;

        // Recalculate totals
        let mut file_diff = file_diff.into_inner();
        file_diff.additions = file_diff.hunks.iter().map(|h| h.additions()).sum();
        file_diff.deletions = file_diff.hunks.iter().map(|h| h.deletions()).sum();

        Ok(file_diff)
    }

    /// Get all diffs in the repository
    pub fn all(repo: &Repository) -> Result<Vec<Self>, GitError> {
        let mut opts = DiffOptions::new();

        // Get diff between index and workdir
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;

        let diffs = RefCell::new(Vec::new());
        let current_diff: RefCell<Option<FileDiff>> = RefCell::new(None);

        diff.foreach(
            &mut |delta, _| {
                // Save previous diff if any
                if let Some(d) = current_diff.borrow_mut().take() {
                    diffs.borrow_mut().push(d);
                }

                let mut d = FileDiff::new(
                    delta.old_file().path().map(|p| p.to_str().unwrap_or("")),
                    delta.new_file().path().map(|p| p.to_str().unwrap_or("")),
                );
                d.is_binary = delta.old_file().is_binary() || delta.new_file().is_binary();
                *current_diff.borrow_mut() = Some(d);
                true
            },
            Some(&mut |_, _| true),
            Some(&mut |_, hunk| {
                if let Some(ref mut d) = *current_diff.borrow_mut() {
                    let diff_hunk = DiffHunk::new(
                        hunk.old_start(),
                        hunk.old_lines(),
                        hunk.new_start(),
                        hunk.new_lines(),
                    );
                    d.hunks.push(diff_hunk);
                }
                true
            }),
            Some(&mut |_, _, line| {
                if let Some(ref mut d) = *current_diff.borrow_mut() {
                    if let Some(hunk) = d.hunks.last_mut() {
                        let change = match line.origin() {
                            '+' => LineChange::Addition,
                            '-' => LineChange::Deletion,
                            ' ' => LineChange::Context,
                            _ => LineChange::Header,
                        };

                        let content = std::str::from_utf8(line.content())
                            .unwrap_or("")
                            .trim_end_matches('\n')
                            .to_string();

                        let mut diff_line = DiffLine::new(change, &content);

                        if let Some(old) = line.old_lineno() {
                            diff_line = diff_line.with_old_line(old);
                        }
                        if let Some(new) = line.new_lineno() {
                            diff_line = diff_line.with_new_line(new);
                        }

                        hunk.add_line(diff_line);
                    }
                }
                true
            }),
        )?;

        // Don't forget the last one
        if let Some(d) = current_diff.into_inner() {
            diffs.borrow_mut().push(d);
        }

        // Recalculate totals
        let mut diffs = diffs.into_inner();
        for d in &mut diffs {
            d.additions = d.hunks.iter().map(|h| h.additions()).sum();
            d.deletions = d.hunks.iter().map(|h| h.deletions()).sum();
        }

        Ok(diffs)
    }

    /// Format as unified diff string
    pub fn to_unified(&self) -> String {
        let mut output = String::new();

        // File header
        let old_path = self.old_path.as_deref().unwrap_or("/dev/null");
        let new_path = self.new_path.as_deref().unwrap_or("/dev/null");
        output.push_str(&format!("--- a/{}\n", old_path));
        output.push_str(&format!("+++ b/{}\n", new_path));

        // Hunks
        for hunk in &self.hunks {
            output.push_str(&hunk.header);
            output.push('\n');

            for line in &hunk.lines {
                let prefix = match line.change {
                    LineChange::Addition => '+',
                    LineChange::Deletion => '-',
                    LineChange::Context => ' ',
                    LineChange::Header => ' ',
                };
                output.push(prefix);
                output.push_str(&line.content);
                output.push('\n');
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_line() {
        let line = DiffLine::new(LineChange::Addition, "new line")
            .with_new_line(10);

        assert_eq!(line.change, LineChange::Addition);
        assert_eq!(line.content, "new line");
        assert_eq!(line.new_line, Some(10));
        assert_eq!(line.old_line, None);
    }

    #[test]
    fn test_diff_hunk() {
        let mut hunk = DiffHunk::new(10, 5, 10, 7);

        hunk.add_line(DiffLine::new(LineChange::Context, "context"));
        hunk.add_line(DiffLine::new(LineChange::Deletion, "old"));
        hunk.add_line(DiffLine::new(LineChange::Addition, "new1"));
        hunk.add_line(DiffLine::new(LineChange::Addition, "new2"));

        assert_eq!(hunk.additions(), 2);
        assert_eq!(hunk.deletions(), 1);
    }

    #[test]
    fn test_file_diff() {
        let mut diff = FileDiff::new(Some("old.rs"), Some("new.rs"));

        let mut hunk = DiffHunk::new(1, 3, 1, 4);
        hunk.add_line(DiffLine::new(LineChange::Addition, "added"));
        diff.add_hunk(hunk);

        assert_eq!(diff.path(), Some("new.rs"));
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 0);
    }

    #[test]
    fn test_unified_format() {
        let mut diff = FileDiff::new(Some("test.rs"), Some("test.rs"));

        let mut hunk = DiffHunk::new(1, 2, 1, 3);
        hunk.add_line(DiffLine::new(LineChange::Context, "line1"));
        hunk.add_line(DiffLine::new(LineChange::Deletion, "old"));
        hunk.add_line(DiffLine::new(LineChange::Addition, "new1"));
        hunk.add_line(DiffLine::new(LineChange::Addition, "new2"));
        diff.add_hunk(hunk);

        let unified = diff.to_unified();
        assert!(unified.contains("--- a/test.rs"));
        assert!(unified.contains("+++ b/test.rs"));
        assert!(unified.contains("+new1"));
        assert!(unified.contains("-old"));
    }
}
