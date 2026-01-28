//! Git status tracking

use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::repo::GitError;

/// File status in the working tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// File is unmodified
    Current,
    /// File is new in index
    New,
    /// File is modified
    Modified,
    /// File is deleted
    Deleted,
    /// File is renamed
    Renamed,
    /// File is copied
    Copied,
    /// File is ignored
    Ignored,
    /// File has conflicts
    Conflicted,
    /// File is untracked
    Untracked,
    /// File type changed
    TypeChange,
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::Current
    }
}

impl From<Status> for FileStatus {
    fn from(status: Status) -> Self {
        if status.is_conflicted() {
            FileStatus::Conflicted
        } else if status.is_wt_new() || status.is_index_new() {
            FileStatus::New
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            FileStatus::Deleted
        } else if status.is_wt_renamed() || status.is_index_renamed() {
            FileStatus::Renamed
        } else if status.is_wt_modified() || status.is_index_modified() {
            FileStatus::Modified
        } else if status.is_wt_typechange() || status.is_index_typechange() {
            FileStatus::TypeChange
        } else if status.is_ignored() {
            FileStatus::Ignored
        } else {
            FileStatus::Current
        }
    }
}

/// Entry in the status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEntry {
    /// File path relative to repository root
    pub path: PathBuf,
    /// Status in the index (staged)
    pub index_status: FileStatus,
    /// Status in the working tree
    pub worktree_status: FileStatus,
    /// Original path (for renames)
    pub original_path: Option<PathBuf>,
}

impl StatusEntry {
    /// Check if file is staged
    pub fn is_staged(&self) -> bool {
        !matches!(self.index_status, FileStatus::Current | FileStatus::Untracked)
    }

    /// Check if file has unstaged changes
    pub fn has_unstaged_changes(&self) -> bool {
        !matches!(self.worktree_status, FileStatus::Current)
    }

    /// Check if file is untracked
    pub fn is_untracked(&self) -> bool {
        matches!(self.worktree_status, FileStatus::New) && 
        matches!(self.index_status, FileStatus::Current)
    }

    /// Check if file has conflicts
    pub fn is_conflicted(&self) -> bool {
        matches!(self.index_status, FileStatus::Conflicted) ||
        matches!(self.worktree_status, FileStatus::Conflicted)
    }
}

/// Complete worktree status
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeStatus {
    /// All status entries
    pub entries: Vec<StatusEntry>,
    /// Staged files count
    pub staged_count: usize,
    /// Unstaged files count
    pub unstaged_count: usize,
    /// Untracked files count
    pub untracked_count: usize,
    /// Conflicted files count
    pub conflicted_count: usize,
}

impl WorktreeStatus {
    /// Create status from a repository
    pub fn from_repo(repo: &Repository) -> Result<Self, GitError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut entries = Vec::new();
        let mut staged_count = 0;
        let mut unstaged_count = 0;
        let mut untracked_count = 0;
        let mut conflicted_count = 0;

        for entry in statuses.iter() {
            let status = entry.status();
            let path = PathBuf::from(entry.path().unwrap_or(""));

            // Determine index status
            let index_status = if status.is_index_new() {
                FileStatus::New
            } else if status.is_index_modified() {
                FileStatus::Modified
            } else if status.is_index_deleted() {
                FileStatus::Deleted
            } else if status.is_index_renamed() {
                FileStatus::Renamed
            } else if status.is_index_typechange() {
                FileStatus::TypeChange
            } else {
                FileStatus::Current
            };

            // Determine worktree status
            let worktree_status = if status.is_conflicted() {
                FileStatus::Conflicted
            } else if status.is_wt_new() {
                FileStatus::New
            } else if status.is_wt_modified() {
                FileStatus::Modified
            } else if status.is_wt_deleted() {
                FileStatus::Deleted
            } else if status.is_wt_renamed() {
                FileStatus::Renamed
            } else if status.is_wt_typechange() {
                FileStatus::TypeChange
            } else {
                FileStatus::Current
            };

            let entry = StatusEntry {
                path,
                index_status,
                worktree_status,
                original_path: entry.head_to_index()
                    .and_then(|d| d.old_file().path())
                    .map(|p| p.to_path_buf()),
            };

            // Update counts
            if entry.is_conflicted() {
                conflicted_count += 1;
            } else if entry.is_untracked() {
                untracked_count += 1;
            } else {
                if entry.is_staged() {
                    staged_count += 1;
                }
                if entry.has_unstaged_changes() {
                    unstaged_count += 1;
                }
            }

            entries.push(entry);
        }

        Ok(Self {
            entries,
            staged_count,
            unstaged_count,
            untracked_count,
            conflicted_count,
        })
    }

    /// Check if working tree is clean
    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get staged files
    pub fn staged(&self) -> Vec<&StatusEntry> {
        self.entries.iter().filter(|e| e.is_staged()).collect()
    }

    /// Get unstaged files
    pub fn unstaged(&self) -> Vec<&StatusEntry> {
        self.entries.iter().filter(|e| e.has_unstaged_changes()).collect()
    }

    /// Get untracked files
    pub fn untracked(&self) -> Vec<&StatusEntry> {
        self.entries.iter().filter(|e| e.is_untracked()).collect()
    }

    /// Get conflicted files
    pub fn conflicted(&self) -> Vec<&StatusEntry> {
        self.entries.iter().filter(|e| e.is_conflicted()).collect()
    }

    /// Get status for a specific file
    pub fn get(&self, path: &PathBuf) -> Option<&StatusEntry> {
        self.entries.iter().find(|e| &e.path == path)
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if self.staged_count > 0 {
            parts.push(format!("{} staged", self.staged_count));
        }
        if self.unstaged_count > 0 {
            parts.push(format!("{} modified", self.unstaged_count));
        }
        if self.untracked_count > 0 {
            parts.push(format!("{} untracked", self.untracked_count));
        }
        if self.conflicted_count > 0 {
            parts.push(format!("{} conflicted", self.conflicted_count));
        }

        if parts.is_empty() {
            "Clean".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_default() {
        let status = FileStatus::default();
        assert_eq!(status, FileStatus::Current);
    }

    #[test]
    fn test_status_entry_untracked() {
        let entry = StatusEntry {
            path: PathBuf::from("new_file.txt"),
            index_status: FileStatus::Current,
            worktree_status: FileStatus::New,
            original_path: None,
        };

        assert!(entry.is_untracked());
        assert!(!entry.is_staged());
    }

    #[test]
    fn test_status_entry_staged() {
        let entry = StatusEntry {
            path: PathBuf::from("staged_file.txt"),
            index_status: FileStatus::Modified,
            worktree_status: FileStatus::Current,
            original_path: None,
        };

        assert!(entry.is_staged());
        assert!(!entry.has_unstaged_changes());
    }

    #[test]
    fn test_worktree_status_summary() {
        let status = WorktreeStatus {
            entries: Vec::new(),
            staged_count: 2,
            unstaged_count: 1,
            untracked_count: 3,
            conflicted_count: 0,
        };

        let summary = status.summary();
        assert!(summary.contains("2 staged"));
        assert!(summary.contains("1 modified"));
        assert!(summary.contains("3 untracked"));
    }

    #[test]
    fn test_worktree_status_clean() {
        let status = WorktreeStatus::default();
        assert!(status.is_clean());
        assert_eq!(status.summary(), "Clean");
    }
}
