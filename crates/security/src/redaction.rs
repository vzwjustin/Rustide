//! Secret redaction utilities

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

/// Type of secret
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretType {
    /// API key
    ApiKey,
    /// Password
    Password,
    /// Bearer token
    BearerToken,
    /// AWS credentials
    AwsCredential,
    /// GitHub token
    GitHubToken,
    /// Private key
    PrivateKey,
    /// Connection string
    ConnectionString,
    /// Generic secret
    Generic,
    /// Custom type
    Custom(String),
}

impl Default for SecretType {
    fn default() -> Self {
        Self::Generic
    }
}

impl std::fmt::Display for SecretType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretType::ApiKey => write!(f, "API_KEY"),
            SecretType::Password => write!(f, "PASSWORD"),
            SecretType::BearerToken => write!(f, "BEARER_TOKEN"),
            SecretType::AwsCredential => write!(f, "AWS_CREDENTIAL"),
            SecretType::GitHubToken => write!(f, "GITHUB_TOKEN"),
            SecretType::PrivateKey => write!(f, "PRIVATE_KEY"),
            SecretType::ConnectionString => write!(f, "CONNECTION_STRING"),
            SecretType::Generic => write!(f, "SECRET"),
            SecretType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A pattern for redacting secrets
#[derive(Debug, Clone)]
pub struct RedactionPattern {
    /// Pattern name
    pub name: String,
    /// Secret type
    pub secret_type: SecretType,
    /// Regex pattern
    pattern: Regex,
    /// Replacement format
    pub replacement: String,
    /// Capture group to redact (0 for whole match)
    pub capture_group: usize,
}

impl RedactionPattern {
    /// Create a new pattern
    pub fn new(
        name: &str,
        secret_type: SecretType,
        pattern: &str,
        replacement: &str,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            name: name.to_string(),
            secret_type,
            pattern: Regex::new(pattern)?,
            replacement: replacement.to_string(),
            capture_group: 0,
        })
    }

    /// Set capture group to redact
    pub fn with_capture_group(mut self, group: usize) -> Self {
        self.capture_group = group;
        self
    }

    /// Check if text matches this pattern
    pub fn matches(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }

    /// Find all matches in text
    pub fn find_matches(&self, text: &str) -> Vec<(usize, usize)> {
        self.pattern
            .find_iter(text)
            .map(|m| (m.start(), m.end()))
            .collect()
    }

    /// Redact text using this pattern
    pub fn redact<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.pattern.replace_all(text, &self.replacement)
    }
}

/// The main redactor
pub struct Redactor {
    /// Patterns to apply
    patterns: Vec<RedactionPattern>,
    /// Custom literal secrets to redact
    literals: HashMap<String, String>,
    /// Placeholder format
    placeholder_format: String,
    /// Whether to preserve partial content
    preserve_partial: bool,
}

impl Redactor {
    /// Create a new redactor
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            literals: HashMap::new(),
            placeholder_format: "[REDACTED:{type}]".to_string(),
            preserve_partial: false,
        }
    }

    /// Create with standard patterns
    pub fn standard() -> Self {
        let mut redactor = Self::new();
        redactor.add_standard_patterns();
        redactor
    }

    /// Add standard redaction patterns
    pub fn add_standard_patterns(&mut self) {
        // API Keys (generic format)
        self.add_pattern(RedactionPattern::new(
            "api_key",
            SecretType::ApiKey,
            r#"(?i)(api[_-]?key|apikey)[=:]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#,
            "[REDACTED:API_KEY]",
        ).unwrap());

        // Bearer tokens
        self.add_pattern(RedactionPattern::new(
            "bearer_token",
            SecretType::BearerToken,
            r"(?i)bearer\s+([a-zA-Z0-9_.-]+)",
            "Bearer [REDACTED:TOKEN]",
        ).unwrap());

        // Password in URLs
        self.add_pattern(RedactionPattern::new(
            "url_password",
            SecretType::Password,
            r"://([^:]+):([^@]+)@",
            "://$1:[REDACTED:PASSWORD]@",
        ).unwrap());

        // AWS Access Key
        self.add_pattern(RedactionPattern::new(
            "aws_access_key",
            SecretType::AwsCredential,
            r"(?i)(AKIA[0-9A-Z]{16})",
            "[REDACTED:AWS_ACCESS_KEY]",
        ).unwrap());

        // AWS Secret Key
        self.add_pattern(RedactionPattern::new(
            "aws_secret_key",
            SecretType::AwsCredential,
            r#"(?i)(aws[_-]?secret[_-]?access[_-]?key)[=:]\s*['"]?([a-zA-Z0-9/+=]{40})['"]?"#,
            "[REDACTED:AWS_SECRET_KEY]",
        ).unwrap());

        // GitHub tokens
        self.add_pattern(RedactionPattern::new(
            "github_token",
            SecretType::GitHubToken,
            r"(ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|ghu_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})",
            "[REDACTED:GITHUB_TOKEN]",
        ).unwrap());

        // Private keys
        self.add_pattern(RedactionPattern::new(
            "private_key",
            SecretType::PrivateKey,
            r"-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----",
            "[REDACTED:PRIVATE_KEY]",
        ).unwrap());

        // Generic password patterns
        self.add_pattern(RedactionPattern::new(
            "password",
            SecretType::Password,
            r#"(?i)(password|passwd|pwd|secret)[=:]\s*['\"]?([^\s'"]+)['\"]?"#,
            "$1=[REDACTED:PASSWORD]",
        ).unwrap());

        // Connection strings
        self.add_pattern(RedactionPattern::new(
            "connection_string",
            SecretType::ConnectionString,
            r"(?i)(mongodb|postgresql|mysql|redis|amqp)://[^\s]+",
            "[REDACTED:CONNECTION_STRING]",
        ).unwrap());

        // JWT tokens
        self.add_pattern(RedactionPattern::new(
            "jwt",
            SecretType::BearerToken,
            r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*",
            "[REDACTED:JWT]",
        ).unwrap());
    }

    /// Add a pattern
    pub fn add_pattern(&mut self, pattern: RedactionPattern) {
        self.patterns.push(pattern);
    }

    /// Add a literal secret to redact
    pub fn add_literal(&mut self, secret: &str, replacement: &str) {
        self.literals.insert(secret.to_string(), replacement.to_string());
    }

    /// Set placeholder format
    pub fn set_placeholder_format(&mut self, format: &str) {
        self.placeholder_format = format.to_string();
    }

    /// Enable partial preservation
    pub fn preserve_partial(&mut self, preserve: bool) {
        self.preserve_partial = preserve;
    }

    /// Redact secrets from text
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Apply patterns
        for pattern in &self.patterns {
            result = pattern.redact(&result).into_owned();
        }

        // Apply literals
        for (secret, replacement) in &self.literals {
            result = result.replace(secret, replacement);
        }

        result
    }

    /// Check if text contains any secrets
    pub fn contains_secrets(&self, text: &str) -> bool {
        for pattern in &self.patterns {
            if pattern.matches(text) {
                return true;
            }
        }

        for secret in self.literals.keys() {
            if text.contains(secret) {
                return true;
            }
        }

        false
    }

    /// Find all secrets in text
    pub fn find_secrets(&self, text: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            for (start, end) in pattern.find_matches(text) {
                matches.push(SecretMatch {
                    secret_type: pattern.secret_type.clone(),
                    start,
                    end,
                    pattern_name: pattern.name.clone(),
                });
            }
        }

        // Sort by position
        matches.sort_by_key(|m| m.start);
        matches
    }

    /// Redact with preserved length (for display)
    pub fn redact_preserved(&self, text: &str) -> String {
        let secrets = self.find_secrets(text);
        if secrets.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut last_end = 0;

        for secret in secrets {
            result.push_str(&text[last_end..secret.start]);

            // Preserve length with asterisks
            let len = secret.end - secret.start;
            let asterisks = "*".repeat(len.min(20));
            result.push_str(&asterisks);

            last_end = secret.end;
        }

        result.push_str(&text[last_end..]);
        result
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::standard()
    }
}

/// A secret match
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// Type of secret
    pub secret_type: SecretType,
    /// Start position
    pub start: usize,
    /// End position
    pub end: usize,
    /// Pattern name that matched
    pub pattern_name: String,
}

impl SecretMatch {
    /// Get the matched text from the original string
    pub fn text<'a>(&self, original: &'a str) -> &'a str {
        &original[self.start..self.end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_type_display() {
        assert_eq!(SecretType::ApiKey.to_string(), "API_KEY");
        assert_eq!(SecretType::Custom("MY_SECRET".to_string()).to_string(), "MY_SECRET");
    }

    #[test]
    fn test_redaction_pattern() {
        let pattern = RedactionPattern::new(
            "test",
            SecretType::ApiKey,
            r"secret=(\w+)",
            "secret=[REDACTED]",
        ).unwrap();

        assert!(pattern.matches("secret=abc123"));
        assert_eq!(pattern.redact("secret=abc123"), "secret=[REDACTED]");
    }

    #[test]
    fn test_redactor_api_key() {
        let redactor = Redactor::standard();

        // Use a test pattern that won't trigger secret scanning
        let text = "api_key=sk_test_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
        let redacted = redactor.redact(text);

        assert!(redacted.contains("[REDACTED:API_KEY]"));
        assert!(!redacted.contains("abc123"));
    }

    #[test]
    fn test_redactor_bearer_token() {
        let redactor = Redactor::standard();

        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9";
        let redacted = redactor.redact(text);

        assert!(redacted.contains("[REDACTED:TOKEN]"));
    }

    #[test]
    fn test_redactor_github_token() {
        let redactor = Redactor::standard();

        let text = "token: ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let redacted = redactor.redact(text);

        assert!(redacted.contains("[REDACTED:GITHUB_TOKEN]"));
    }

    #[test]
    fn test_redactor_url_password() {
        let redactor = Redactor::standard();

        let text = "postgres://user:secretpassword@localhost:5432/db";
        let redacted = redactor.redact(text);

        assert!(redacted.contains("[REDACTED:PASSWORD]"));
        assert!(!redacted.contains("secretpassword"));
    }

    #[test]
    fn test_redactor_private_key() {
        let redactor = Redactor::standard();

        let text = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA0Z...
-----END RSA PRIVATE KEY-----"#;

        let redacted = redactor.redact(text);
        assert!(redacted.contains("[REDACTED:PRIVATE_KEY]"));
    }

    #[test]
    fn test_redactor_literal() {
        let mut redactor = Redactor::new();
        redactor.add_literal("mysecret123", "[SECRET]");

        let text = "The password is mysecret123";
        let redacted = redactor.redact(text);

        assert_eq!(redacted, "The password is [SECRET]");
    }

    #[test]
    fn test_redactor_contains_secrets() {
        let redactor = Redactor::standard();

        assert!(redactor.contains_secrets("api_key=abc123def456ghi789jkl012mno"));
        assert!(!redactor.contains_secrets("Hello, World!"));
    }

    #[test]
    fn test_redactor_find_secrets() {
        let redactor = Redactor::standard();

        let text = "ghp_1234567890abcdefghijklmnopqrstuvwxyz and AKIA1234567890123456";
        let secrets = redactor.find_secrets(text);

        assert_eq!(secrets.len(), 2);
    }

    #[test]
    fn test_redactor_preserved() {
        let redactor = Redactor::standard();

        let text = "token=ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let redacted = redactor.redact_preserved(text);

        // Should contain asterisks
        assert!(redacted.contains("*"));
    }

    #[test]
    fn test_jwt_redaction() {
        let redactor = Redactor::standard();

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let redacted = redactor.redact(jwt);

        assert!(redacted.contains("[REDACTED:JWT]"));
    }
}
