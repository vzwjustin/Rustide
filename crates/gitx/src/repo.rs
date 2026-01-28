//! Git repository operations

use crate::commit::{CommitBuilder, CommitInfo};
use crate::diff::FileDiff;
use crate::status::{StatusEntry, WorktreeStatus};
use git2::{
    BranchType, Cred, CredentialType, ErrorCode, FetchOptions, Oid, PushOptions,
    RemoteCallbacks, Repository as Git2Repository, ResetType, Signature,
};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur in Git operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a Git repository: {0}")]
    NotARepository(String),

    #[error("Git operation failed: {0}")]
    OperationFailed(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Remote not found: {0}")]
    RemoteNotFound(String),

    #[error("Merge conflict")]
    MergeConflict,

    #[error("Working tree is dirty")]
    DirtyWorkTree,

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        match err.code() {
            ErrorCode::NotFound => {
                if err.message().contains("remote") {
                    GitError::RemoteNotFound(err.message().to_string())
                } else {
                    GitError::BranchNotFound(err.message().to_string())
                }
            }
            ErrorCode::Auth => GitError::AuthenticationFailed,
            ErrorCode::Conflict => GitError::MergeConflict,
            _ => GitError::OperationFailed(err.message().to_string()),
        }
    }
}

/// Git repository wrapper
pub struct Repository {
    repo: Git2Repository,
    path: PathBuf,
}

impl Repository {
    /// Open an existing repository
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        let path = path.as_ref().to_path_buf();
        let repo = Git2Repository::open(&path).map_err(|e| {
            if e.code() == ErrorCode::NotFound {
                GitError::NotARepository(path.display().to_string())
            } else {
                GitError::from(e)
            }
        })?;

        Ok(Self { repo, path })
    }

    /// Discover repository from a path (walks up directories)
    pub fn discover<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        let path = path.as_ref();
        let repo = Git2Repository::discover(path)?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::NotARepository("Bare repository".to_string()))?
            .to_path_buf();

        Ok(Self {
            repo,
            path: workdir,
        })
    }

    /// Initialize a new repository
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        let path = path.as_ref().to_path_buf();
        let repo = Git2Repository::init(&path)?;
        Ok(Self { repo, path })
    }

    /// Clone a repository
    pub fn clone(url: &str, path: &Path) -> Result<Self, GitError> {
        info!("Cloning {} to {}", url, path.display());
        let repo = Git2Repository::clone(url, path)?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::NotARepository("Bare repository".to_string()))?
            .to_path_buf();

        Ok(Self {
            repo,
            path: workdir,
        })
    }

    /// Get repository path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        if head.is_branch() {
            let name = head
                .shorthand()
                .ok_or_else(|| GitError::Internal("Invalid branch name".to_string()))?;
            Ok(name.to_string())
        } else {
            // Detached HEAD
            let oid = head.target().ok_or_else(|| {
                GitError::Internal("HEAD has no target".to_string())
            })?;
            Ok(format!("HEAD detached at {}", &oid.to_string()[..7]))
        }
    }

    /// Get all local branches
    pub fn branches(&self) -> Result<Vec<String>, GitError> {
        let branches = self.repo.branches(Some(BranchType::Local))?;
        let mut names = Vec::new();

        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                names.push(name.to_string());
            }
        }

        Ok(names)
    }

    /// Get all remote branches
    pub fn remote_branches(&self) -> Result<Vec<String>, GitError> {
        let branches = self.repo.branches(Some(BranchType::Remote))?;
        let mut names = Vec::new();

        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                names.push(name.to_string());
            }
        }

        Ok(names)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str, from: Option<&str>) -> Result<(), GitError> {
        let commit = if let Some(from) = from {
            let obj = self.repo.revparse_single(from)?;
            obj.peel_to_commit()?
        } else {
            let head = self.repo.head()?;
            head.peel_to_commit()?
        };

        self.repo.branch(name, &commit, false)?;
        info!("Created branch: {}", name);
        Ok(())
    }

    /// Checkout a branch
    pub fn checkout(&self, name: &str) -> Result<(), GitError> {
        let (object, reference) = self.repo.revparse_ext(name)?;

        self.repo.checkout_tree(&object, None)?;

        match reference {
            Some(gref) => {
                self.repo.set_head(gref.name().unwrap())?;
            }
            None => {
                self.repo.set_head_detached(object.id())?;
            }
        }

        info!("Checked out: {}", name);
        Ok(())
    }

    /// Delete a branch
    pub fn delete_branch(&self, name: &str, force: bool) -> Result<(), GitError> {
        let mut branch = self.repo.find_branch(name, BranchType::Local)?;

        if !force && !branch.is_head() {
            // Check if branch is merged
            let head = self.repo.head()?.peel_to_commit()?;
            let branch_commit = branch.get().peel_to_commit()?;

            let merge_base = self.repo.merge_base(head.id(), branch_commit.id())?;
            if merge_base != branch_commit.id() {
                return Err(GitError::OperationFailed(
                    "Branch is not fully merged. Use force to delete.".to_string(),
                ));
            }
        }

        branch.delete()?;
        info!("Deleted branch: {}", name);
        Ok(())
    }

    /// Get worktree status
    pub fn status(&self) -> Result<WorktreeStatus, GitError> {
        WorktreeStatus::from_repo(&self.repo)
    }

    /// Stage a file
    pub fn stage_file<P: AsRef<Path>>(&self, path: P) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        let path = path.as_ref();

        // Check if file exists
        let full_path = self.path.join(path);
        if full_path.exists() {
            index.add_path(path)?;
        } else {
            index.remove_path(path)?;
        }

        index.write()?;
        debug!("Staged: {}", path.display());
        Ok(())
    }

    /// Stage all changes
    pub fn stage_all(&self) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        debug!("Staged all changes");
        Ok(())
    }

    /// Unstage a file
    pub fn unstage_file<P: AsRef<Path>>(&self, path: P) -> Result<(), GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        let head_tree = head.tree()?;

        self.repo
            .reset_default(Some(&head.into_object()), [path.as_ref()])?;

        debug!("Unstaged: {}", path.as_ref().display());
        Ok(())
    }

    /// Discard changes in a file
    pub fn discard_changes<P: AsRef<Path>>(&self, path: P) -> Result<(), GitError> {
        let path = path.as_ref();
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.path(path).force();

        self.repo.checkout_head(Some(&mut opts))?;
        debug!("Discarded changes: {}", path.display());
        Ok(())
    }

    /// Create a commit
    pub fn commit(&self, message: &str) -> Result<CommitInfo, GitError> {
        let builder = CommitBuilder::new(message);
        builder.commit(&self.repo)
    }

    /// Amend the last commit
    pub fn amend_commit(&self, message: Option<&str>) -> Result<CommitInfo, GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let message = message.unwrap_or_else(|| head.message().unwrap_or(""));

        let new_oid = head.amend(
            Some("HEAD"),
            None,
            None,
            None,
            Some(message),
            Some(&tree),
        )?;

        let new_commit = self.repo.find_commit(new_oid)?;
        Ok(CommitInfo::from_commit(&new_commit))
    }

    /// Get recent commits
    pub fn log(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_commit(&commit));
        }

        Ok(commits)
    }

    /// Get diff for a file
    pub fn diff_file<P: AsRef<Path>>(&self, path: P) -> Result<FileDiff, GitError> {
        FileDiff::for_file(&self.repo, path.as_ref())
    }

    /// Get all diffs
    pub fn diff(&self) -> Result<Vec<FileDiff>, GitError> {
        FileDiff::all(&self.repo)
    }

    /// Get remotes
    pub fn remotes(&self) -> Result<Vec<String>, GitError> {
        let remotes = self.repo.remotes()?;
        Ok(remotes.iter().filter_map(|r| r.map(|s| s.to_string())).collect())
    }

    /// Fetch from remote
    pub fn fetch(&self, remote: &str) -> Result<(), GitError> {
        info!("Fetching from {}", remote);

        let mut remote = self.repo.find_remote(remote)?;
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut opts = FetchOptions::new();
        opts.remote_callbacks(callbacks);

        remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
        Ok(())
    }

    /// Pull from remote (fetch + merge)
    pub fn pull(&self, remote: &str, branch: &str) -> Result<(), GitError> {
        self.fetch(remote)?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        let (analysis, _) = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch);
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            self.repo.set_head(&refname)?;
            self.repo.checkout_head(Some(&mut git2::build::CheckoutBuilder::new().force()))?;
            info!("Fast-forward to {}", fetch_commit.id());
        } else if analysis.is_normal() {
            // Normal merge would require more complex logic
            return Err(GitError::OperationFailed(
                "Non-fast-forward merge not implemented".to_string(),
            ));
        }

        Ok(())
    }

    /// Push to remote
    pub fn push(&self, remote: &str, branch: &str) -> Result<(), GitError> {
        info!("Pushing {} to {}", branch, remote);

        let mut remote = self.repo.find_remote(remote)?;
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut opts = PushOptions::new();
        opts.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        remote.push(&[&refspec], Some(&mut opts))?;

        Ok(())
    }

    /// Reset to a commit
    pub fn reset(&self, target: &str, hard: bool) -> Result<(), GitError> {
        let obj = self.repo.revparse_single(target)?;
        let commit = obj.peel_to_commit()?;

        let reset_type = if hard {
            ResetType::Hard
        } else {
            ResetType::Mixed
        };

        self.repo.reset(&commit.into_object(), reset_type, None)?;
        info!("Reset to {}", target);
        Ok(())
    }

    /// Stash changes
    pub fn stash(&mut self, message: Option<&str>) -> Result<Oid, GitError> {
        let signature = self.repo.signature()?;
        let oid = self.repo.stash_save(&signature, message.unwrap_or("WIP"), None)?;
        info!("Stashed changes: {}", oid);
        Ok(oid)
    }

    /// Pop stash
    pub fn stash_pop(&mut self) -> Result<(), GitError> {
        self.repo.stash_pop(0, None)?;
        info!("Popped stash");
        Ok(())
    }

    /// Get head commit
    pub fn head_commit(&self) -> Result<CommitInfo, GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        Ok(CommitInfo::from_commit(&head))
    }

    /// Check if repository is dirty
    pub fn is_dirty(&self) -> Result<bool, GitError> {
        let status = self.status()?;
        Ok(!status.is_clean())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_init_repository() {
        let (_dir, repo) = create_test_repo();
        assert!(repo.path().exists());
    }

    #[test]
    fn test_current_branch_initial() {
        let (_dir, repo) = create_test_repo();
        // Initial repo has no commits, so HEAD is unborn
        // This will error, which is expected
        assert!(repo.current_branch().is_err());
    }
}
