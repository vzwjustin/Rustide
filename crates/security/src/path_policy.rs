//! Path policy enforcement

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Path policy errors
#[derive(Error, Debug)]
pub enum PathPolicyError {
    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Path outside workspace: {0}")]
    OutsideWorkspace(String),
}

/// Permission type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
    /// Delete access
    Delete,
    /// Full access
    All,
}

impl Permission {
    /// Check if this permission implies another
    pub fn implies(&self, other: Permission) -> bool {
        match self {
            Permission::All => true,
            Permission::Read => matches!(other, Permission::Read),
            Permission::Write => matches!(other, Permission::Write | Permission::Read),
            Permission::Execute => matches!(other, Permission::Execute),
            Permission::Delete => matches!(other, Permission::Delete | Permission::Write),
        }
    }
}

/// Rule type (allow or deny)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    Allow,
    Deny,
}

/// A path rule
#[derive(Debug, Clone)]
pub struct PathRule {
    /// Rule type
    pub rule_type: RuleType,
    /// Path pattern (glob-like)
    pub pattern: String,
    /// Compiled regex
    regex: Regex,
    /// Permissions this rule applies to
    pub permissions: Vec<Permission>,
    /// Priority (higher = evaluated first)
    pub priority: i32,
    /// Description
    pub description: Option<String>,
}

impl PathRule {
    /// Create a new allow rule
    pub fn allow(pattern: &str) -> Result<Self, regex::Error> {
        Self::new(RuleType::Allow, pattern)
    }

    /// Create a new deny rule
    pub fn deny(pattern: &str) -> Result<Self, regex::Error> {
        Self::new(RuleType::Deny, pattern)
    }

    /// Create a new rule
    fn new(rule_type: RuleType, pattern: &str) -> Result<Self, regex::Error> {
        let regex_pattern = glob_to_regex(pattern);
        let regex = Regex::new(&regex_pattern)?;

        Ok(Self {
            rule_type,
            pattern: pattern.to_string(),
            regex,
            permissions: vec![Permission::All],
            priority: 0,
            description: None,
        })
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Check if path matches this rule
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.regex.is_match(&path_str)
    }

    /// Check if this rule applies to a permission
    pub fn applies_to(&self, permission: Permission) -> bool {
        self.permissions.iter().any(|p| p.implies(permission))
    }
}

/// Convert glob pattern to regex
fn glob_to_regex(pattern: &str) -> String {
    let mut result = String::from("^");

    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        result.push_str("(.*/)?");
                    } else {
                        result.push_str(".*");
                    }
                } else {
                    result.push_str("[^/]*");
                }
            }
            '?' => result.push_str("[^/]"),
            '.' => result.push_str("\\."),
            '/' => result.push('/'),
            '[' => {
                result.push('[');
                while let Some(c) = chars.next() {
                    if c == ']' {
                        result.push(']');
                        break;
                    }
                    result.push(c);
                }
            }
            c => result.push(c),
        }
    }

    result.push('$');
    result
}

/// Path policy configuration
#[derive(Debug, Clone, Default)]
pub struct PathPolicy {
    /// Rules (evaluated in priority order, then insertion order)
    rules: Vec<PathRule>,
    /// Workspace root (all paths must be within)
    workspace_root: Option<PathBuf>,
    /// Default behavior when no rules match
    default_allow: bool,
    /// Whether to follow symlinks
    follow_symlinks: bool,
}

impl PathPolicy {
    /// Create a new policy
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            workspace_root: None,
            default_allow: false,
            follow_symlinks: false,
        }
    }

    /// Create a permissive policy (allow by default)
    pub fn permissive() -> Self {
        Self {
            rules: Vec::new(),
            workspace_root: None,
            default_allow: true,
            follow_symlinks: true,
        }
    }

    /// Create a restrictive policy (deny by default)
    pub fn restrictive() -> Self {
        Self {
            rules: Vec::new(),
            workspace_root: None,
            default_allow: false,
            follow_symlinks: false,
        }
    }

    /// Set workspace root
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: PathRule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Check if a path is allowed for a permission
    pub fn check(&self, path: &Path, permission: Permission) -> Result<(), PathPolicyError> {
        // Normalize path
        let path = self.normalize_path(path)?;

        // Check workspace boundary
        if let Some(ref root) = self.workspace_root {
            if !path.starts_with(root) {
                return Err(PathPolicyError::OutsideWorkspace(
                    path.display().to_string(),
                ));
            }
        }

        // Check rules
        for rule in &self.rules {
            if rule.matches(&path) && rule.applies_to(permission) {
                match rule.rule_type {
                    RuleType::Allow => return Ok(()),
                    RuleType::Deny => {
                        return Err(PathPolicyError::AccessDenied(format!(
                            "Path '{}' denied by rule: {}",
                            path.display(),
                            rule.pattern
                        )));
                    }
                }
            }
        }

        // Default behavior
        if self.default_allow {
            Ok(())
        } else {
            Err(PathPolicyError::AccessDenied(format!(
                "Path '{}' not allowed (no matching rule)",
                path.display()
            )))
        }
    }

    /// Check if path is allowed for reading
    pub fn can_read(&self, path: &Path) -> bool {
        self.check(path, Permission::Read).is_ok()
    }

    /// Check if path is allowed for writing
    pub fn can_write(&self, path: &Path) -> bool {
        self.check(path, Permission::Write).is_ok()
    }

    /// Check if path is allowed for execution
    pub fn can_execute(&self, path: &Path) -> bool {
        self.check(path, Permission::Execute).is_ok()
    }

    /// Check if path is allowed for deletion
    pub fn can_delete(&self, path: &Path) -> bool {
        self.check(path, Permission::Delete).is_ok()
    }

    /// Normalize a path
    fn normalize_path(&self, path: &Path) -> Result<PathBuf, PathPolicyError> {
        // If path is relative and we have a workspace root, make it absolute
        let path = if path.is_relative() {
            if let Some(ref root) = self.workspace_root {
                root.join(path)
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        };

        // Canonicalize if following symlinks
        if self.follow_symlinks && path.exists() {
            path.canonicalize()
                .map_err(|e| PathPolicyError::InvalidPath(e.to_string()))
        } else {
            // Simple normalization without resolving symlinks
            Ok(normalize_path_components(&path))
        }
    }

    /// Get all rules
    pub fn rules(&self) -> &[PathRule] {
        &self.rules
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }
}

/// Normalize path components (resolve . and ..)
fn normalize_path_components(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                components.pop();
            }
            c => components.push(c),
        }
    }

    components.iter().collect()
}

/// Builder for path policies
pub struct PathPolicyBuilder {
    policy: PathPolicy,
}

impl PathPolicyBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            policy: PathPolicy::new(),
        }
    }

    /// Start with permissive defaults
    pub fn permissive() -> Self {
        Self {
            policy: PathPolicy::permissive(),
        }
    }

    /// Start with restrictive defaults
    pub fn restrictive() -> Self {
        Self {
            policy: PathPolicy::restrictive(),
        }
    }

    /// Set workspace root
    pub fn workspace(mut self, root: PathBuf) -> Self {
        self.policy.set_workspace_root(root);
        self
    }

    /// Allow a pattern
    pub fn allow(mut self, pattern: &str) -> Self {
        if let Ok(rule) = PathRule::allow(pattern) {
            self.policy.add_rule(rule);
        }
        self
    }

    /// Deny a pattern
    pub fn deny(mut self, pattern: &str) -> Self {
        if let Ok(rule) = PathRule::deny(pattern) {
            self.policy.add_rule(rule);
        }
        self
    }

    /// Allow a pattern with specific permissions
    pub fn allow_with_permissions(mut self, pattern: &str, permissions: Vec<Permission>) -> Self {
        if let Ok(rule) = PathRule::allow(pattern) {
            self.policy.add_rule(rule.with_permissions(permissions));
        }
        self
    }

    /// Deny a pattern with specific permissions
    pub fn deny_with_permissions(mut self, pattern: &str, permissions: Vec<Permission>) -> Self {
        if let Ok(rule) = PathRule::deny(pattern) {
            self.policy.add_rule(rule.with_permissions(permissions));
        }
        self
    }

    /// Add a rule
    pub fn rule(mut self, rule: PathRule) -> Self {
        self.policy.add_rule(rule);
        self
    }

    /// Set whether to follow symlinks
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.policy.follow_symlinks = follow;
        self
    }

    /// Set default allow behavior
    pub fn default_allow(mut self, allow: bool) -> Self {
        self.policy.default_allow = allow;
        self
    }

    /// Build the policy
    pub fn build(self) -> PathPolicy {
        self.policy
    }
}

impl Default for PathPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a standard development policy
pub fn standard_dev_policy(workspace: PathBuf) -> PathPolicy {
    PathPolicyBuilder::restrictive()
        .workspace(workspace)
        // Allow source files
        .allow("**/*.rs")
        .allow("**/*.toml")
        .allow("**/*.md")
        .allow("**/*.txt")
        .allow("**/*.json")
        .allow("**/*.yaml")
        .allow("**/*.yml")
        // Deny sensitive files
        .deny("**/.env")
        .deny("**/.env.*")
        .deny("**/credentials*")
        .deny("**/secrets*")
        .deny("**/*.pem")
        .deny("**/*.key")
        .deny("**/id_rsa*")
        // Deny system directories
        .deny("/etc/**")
        .deny("/root/**")
        .deny("/home/*/.ssh/**")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_implies() {
        assert!(Permission::All.implies(Permission::Read));
        assert!(Permission::All.implies(Permission::Write));
        assert!(Permission::Write.implies(Permission::Read));
        assert!(!Permission::Read.implies(Permission::Write));
    }

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("*.rs"), "^[^/]*\\.rs$");
        assert_eq!(glob_to_regex("**/*.rs"), "^(.*/)?\\.rs$");
        assert_eq!(glob_to_regex("src/*.rs"), "^src/[^/]*\\.rs$");
    }

    #[test]
    fn test_path_rule_matches() {
        let rule = PathRule::allow("**/*.rs").unwrap();

        assert!(rule.matches(Path::new("src/main.rs")));
        assert!(rule.matches(Path::new("src/lib/module.rs")));
        assert!(!rule.matches(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_path_policy_allow() {
        let policy = PathPolicyBuilder::restrictive()
            .allow("**/*.rs")
            .build();

        assert!(policy.can_read(Path::new("src/main.rs")));
        assert!(!policy.can_read(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_path_policy_deny() {
        let policy = PathPolicyBuilder::permissive()
            .deny("**/.env")
            .build();

        assert!(policy.can_read(Path::new("src/main.rs")));
        assert!(!policy.can_read(Path::new(".env")));
    }

    #[test]
    fn test_path_policy_priority() {
        let policy = PathPolicyBuilder::restrictive()
            .rule(PathRule::allow("**/*").unwrap().with_priority(0))
            .rule(PathRule::deny("**/.env").unwrap().with_priority(10))
            .build();

        // Higher priority deny should win
        assert!(!policy.can_read(Path::new(".env")));
        assert!(policy.can_read(Path::new("other.txt")));
    }

    #[test]
    fn test_path_policy_workspace_boundary() {
        let policy = PathPolicyBuilder::permissive()
            .workspace(PathBuf::from("/workspace"))
            .build();

        // Path outside workspace should be denied
        let result = policy.check(Path::new("/etc/passwd"), Permission::Read);
        assert!(matches!(result, Err(PathPolicyError::OutsideWorkspace(_))));
    }

    #[test]
    fn test_path_policy_permissions() {
        let policy = PathPolicyBuilder::restrictive()
            .allow_with_permissions("**/*.rs", vec![Permission::Read])
            .build();

        assert!(policy.can_read(Path::new("src/main.rs")));
        assert!(!policy.can_write(Path::new("src/main.rs")));
    }

    #[test]
    fn test_normalize_path_components() {
        let path = PathBuf::from("/a/b/../c/./d");
        let normalized = normalize_path_components(&path);
        assert_eq!(normalized, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn test_standard_dev_policy() {
        let policy = standard_dev_policy(PathBuf::from("/workspace"));

        // Should allow source files
        assert!(policy.can_read(Path::new("/workspace/src/main.rs")));
        assert!(policy.can_read(Path::new("/workspace/Cargo.toml")));

        // Should deny sensitive files
        assert!(!policy.can_read(Path::new("/workspace/.env")));
        assert!(!policy.can_read(Path::new("/workspace/secrets.json")));
    }
}
