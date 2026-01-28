//! Git commit operations

use git2::{Commit, Oid, Repository, Signature, Time};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::repo::GitError;

/// Information about a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit SHA
    pub id: String,
    /// Short SHA (first 7 characters)
    pub short_id: String,
    /// Commit message (first line)
    pub summary: String,
    /// Full commit message
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp (unix seconds)
    pub timestamp: i64,
    /// Parent commit IDs
    pub parents: Vec<String>,
}

impl CommitInfo {
    /// Create from a git2 Commit
    pub fn from_commit(commit: &Commit) -> Self {
        let id = commit.id().to_string();
        let short_id = id[..7.min(id.len())].to_string();

        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();

        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();

        let timestamp = commit.time().seconds();

        let parents = commit.parents()
            .map(|p| p.id().to_string())
            .collect();

        Self {
            id,
            short_id,
            summary,
            message,
            author_name,
            author_email,
            timestamp,
            parents,
        }
    }

    /// Get formatted date
    pub fn date(&self) -> String {
        let datetime = UNIX_EPOCH + Duration::from_secs(self.timestamp as u64);
        // Simple ISO-like format
        format!("{:?}", datetime)
    }

    /// Get relative time (e.g., "2 hours ago")
    pub fn relative_time(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let diff = now - self.timestamp;

        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            let mins = diff / 60;
            format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
        } else if diff < 86400 {
            let hours = diff / 3600;
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else if diff < 604800 {
            let days = diff / 86400;
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else if diff < 2592000 {
            let weeks = diff / 604800;
            format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
        } else if diff < 31536000 {
            let months = diff / 2592000;
            format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
        } else {
            let years = diff / 31536000;
            format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
        }
    }

    /// Check if this is a merge commit
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}

/// Builder for creating commits
pub struct CommitBuilder {
    message: String,
    author_name: Option<String>,
    author_email: Option<String>,
    amend: bool,
}

impl CommitBuilder {
    /// Create a new commit builder
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            author_name: None,
            author_email: None,
            amend: false,
        }
    }

    /// Set custom author
    pub fn author(mut self, name: &str, email: &str) -> Self {
        self.author_name = Some(name.to_string());
        self.author_email = Some(email.to_string());
        self
    }

    /// Mark as amend
    pub fn amend(mut self) -> Self {
        self.amend = true;
        self
    }

    /// Create the commit
    pub fn commit(self, repo: &Repository) -> Result<CommitInfo, GitError> {
        // Get the signature
        let signature = if let (Some(name), Some(email)) = (&self.author_name, &self.author_email) {
            Signature::now(name, email)?
        } else {
            repo.signature()?
        };

        // Get the index and create tree
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get parent commits
        let parent_commit = if repo.is_empty()? {
            None
        } else {
            Some(repo.head()?.peel_to_commit()?)
        };

        let parents: Vec<&Commit> = parent_commit.iter().collect();

        // Create the commit
        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &self.message,
            &tree,
            &parents,
        )?;

        let commit = repo.find_commit(oid)?;
        Ok(CommitInfo::from_commit(&commit))
    }
}

/// Parse conventional commit message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConventionalCommit {
    /// Type (feat, fix, docs, etc.)
    pub commit_type: String,
    /// Scope (optional)
    pub scope: Option<String>,
    /// Description
    pub description: String,
    /// Body (optional)
    pub body: Option<String>,
    /// Breaking change
    pub breaking: bool,
    /// Footer entries
    pub footers: Vec<(String, String)>,
}

impl ConventionalCommit {
    /// Parse a commit message
    pub fn parse(message: &str) -> Option<Self> {
        let lines: Vec<&str> = message.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];

        // Parse type and optional scope
        let (type_scope, description) = first_line.split_once(": ")?;

        let (commit_type, scope, breaking_prefix) = if let Some(idx) = type_scope.find('(') {
            let t = &type_scope[..idx];
            let breaking = t.ends_with('!');
            let t = t.trim_end_matches('!');

            let scope_end = type_scope.find(')')?;
            let s = &type_scope[idx + 1..scope_end];

            (t.to_string(), Some(s.to_string()), breaking)
        } else {
            let breaking = type_scope.ends_with('!');
            let t = type_scope.trim_end_matches('!');
            (t.to_string(), None, breaking)
        };

        // Parse body and footers
        let mut body = None;
        let mut footers = Vec::new();
        let mut in_body = false;
        let mut body_lines = Vec::new();

        for line in lines.iter().skip(1) {
            if line.is_empty() {
                in_body = true;
                continue;
            }

            if in_body {
                // Check for footer
                if let Some((key, value)) = line.split_once(": ") {
                    if key.chars().all(|c| c.is_alphanumeric() || c == '-') {
                        footers.push((key.to_string(), value.to_string()));
                        continue;
                    }
                }
                body_lines.push(*line);
            }
        }

        if !body_lines.is_empty() {
            body = Some(body_lines.join("\n"));
        }

        let breaking = breaking_prefix || footers.iter().any(|(k, _)| k == "BREAKING CHANGE");

        Some(Self {
            commit_type,
            scope,
            description: description.to_string(),
            body,
            breaking,
            footers,
        })
    }

    /// Format as a commit message
    pub fn format(&self) -> String {
        let mut msg = String::new();

        // Type and scope
        msg.push_str(&self.commit_type);
        if let Some(ref scope) = self.scope {
            msg.push('(');
            msg.push_str(scope);
            msg.push(')');
        }
        if self.breaking {
            msg.push('!');
        }
        msg.push_str(": ");
        msg.push_str(&self.description);

        // Body
        if let Some(ref body) = self.body {
            msg.push_str("\n\n");
            msg.push_str(body);
        }

        // Footers
        if !self.footers.is_empty() {
            msg.push_str("\n");
            for (key, value) in &self.footers {
                msg.push('\n');
                msg.push_str(key);
                msg.push_str(": ");
                msg.push_str(value);
            }
        }

        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_info_relative_time() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let info = CommitInfo {
            id: "abc123".to_string(),
            short_id: "abc123".to_string(),
            summary: "Test".to_string(),
            message: "Test".to_string(),
            author_name: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            timestamp: now - 3600, // 1 hour ago
            parents: Vec::new(),
        };

        assert_eq!(info.relative_time(), "1 hour ago");
    }

    #[test]
    fn test_conventional_commit_parse() {
        let msg = "feat(parser): add conventional commit parsing";
        let cc = ConventionalCommit::parse(msg).unwrap();

        assert_eq!(cc.commit_type, "feat");
        assert_eq!(cc.scope, Some("parser".to_string()));
        assert_eq!(cc.description, "add conventional commit parsing");
        assert!(!cc.breaking);
    }

    #[test]
    fn test_conventional_commit_breaking() {
        let msg = "feat!: breaking change";
        let cc = ConventionalCommit::parse(msg).unwrap();

        assert!(cc.breaking);
    }

    #[test]
    fn test_conventional_commit_with_body() {
        let msg = "fix: bug fix\n\nThis is the body of the commit.\n\nFixes: #123";
        let cc = ConventionalCommit::parse(msg).unwrap();

        assert_eq!(cc.commit_type, "fix");
        assert!(cc.body.is_some());
        assert!(cc.footers.iter().any(|(k, v)| k == "Fixes" && v == "#123"));
    }

    #[test]
    fn test_conventional_commit_format() {
        let cc = ConventionalCommit {
            commit_type: "feat".to_string(),
            scope: Some("api".to_string()),
            description: "add new endpoint".to_string(),
            body: None,
            breaking: false,
            footers: Vec::new(),
        };

        assert_eq!(cc.format(), "feat(api): add new endpoint");
    }

    #[test]
    fn test_is_merge() {
        let info = CommitInfo {
            id: "abc".to_string(),
            short_id: "abc".to_string(),
            summary: "Merge".to_string(),
            message: "Merge".to_string(),
            author_name: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            timestamp: 0,
            parents: vec!["parent1".to_string(), "parent2".to_string()],
        };

        assert!(info.is_merge());
    }
}
