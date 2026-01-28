//! GitX - Git integration for Rust IDE
//!
//! This crate provides Git repository operations including status,
//! diff, commit, and other common Git operations.

pub mod commit;
pub mod diff;
pub mod repo;
pub mod status;

pub use commit::{CommitBuilder, CommitInfo};
pub use diff::{DiffHunk, DiffLine, FileDiff, LineChange};
pub use repo::{GitError, Repository};
pub use status::{FileStatus, StatusEntry, WorktreeStatus};
