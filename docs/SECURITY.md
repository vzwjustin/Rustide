# Security Documentation

This document describes the security architecture, features, and best practices for Rustide. Security is a foundational concern, and Rustide is designed with defense-in-depth principles throughout.

---

## Table of Contents

1. [Overview](#overview)
2. [Local-First Design](#local-first-design)
3. [Secret Redaction](#secret-redaction)
4. [Path Policies](#path-policies)
5. [Agent Sandboxing](#agent-sandboxing)
6. [Data Flow Security](#data-flow-security)
7. [Sensitive Paths](#sensitive-paths)
8. [Configuration](#configuration)
9. [Best Practices](#best-practices)

---

## Overview

Rustide's security model is built on three core principles:

### Local-First
All core IDE functionality works entirely offline. Your code never leaves your machine unless you explicitly enable agent features that require external communication.

### Opt-In Agents
AI agents are completely optional. When enabled, they operate under strict permission controls and cannot access resources without explicit grants.

### Sandboxed Execution
Agents operate within sandboxed environments with restricted capabilities. Each agent has a defined set of tools it can use and paths it can access.

```
+------------------+     +------------------+     +------------------+
|   Local IDE      |     |  Agent Sandbox   |     |  External APIs   |
|                  |     |                  |     |                  |
|  - Full offline  |     |  - Limited tools |     |  - Opt-in only   |
|  - No telemetry  | --> |  - Path policies | --> |  - Redacted data |
|  - Local storage |     |  - Audit logging |     |  - User consent  |
+------------------+     +------------------+     +------------------+
```

---

## Local-First Design

### IDE Works Fully Offline

Rustide is designed to function completely without network access:

- **Code editing**: Full functionality without any network calls
- **Syntax highlighting**: Language support loaded locally
- **File operations**: Direct filesystem access only
- **Project management**: All metadata stored locally

### Agents Are Optional

Agent features must be explicitly enabled:

```rust
// Agents are disabled by default
let orchestrator = AgentOrchestrator::default();

// Agents must be explicitly registered
let agent = Agent::new("code_assistant", "Assists with code tasks")
    .with_tool("read_file")
    .with_permission("file:read");

orchestrator.register_agent(agent);
```

### No Telemetry Without Consent

**WARNING**: Rustide collects no telemetry by default. Any analytics or usage data collection requires explicit user opt-in through configuration.

- No crash reporting without consent
- No usage analytics without consent
- No feature tracking without consent
- Configuration changes are logged locally in the audit log

---

## Secret Redaction

The security crate provides comprehensive secret detection and redaction to prevent accidental exposure of sensitive data.

### What Is Detected

The redaction system detects the following secret types:

| Secret Type | Description | Example Pattern |
|-------------|-------------|-----------------|
| `ApiKey` | API keys and tokens | `api_key=sk_live_abc123...` |
| `Password` | Passwords in various formats | `password=secret123` |
| `BearerToken` | OAuth bearer tokens | `Bearer eyJhbG...` |
| `AwsCredential` | AWS access keys and secrets | `AKIA1234567890123456` |
| `GitHubToken` | GitHub personal access tokens | `ghp_xxxx...`, `github_pat_...` |
| `PrivateKey` | PEM-encoded private keys | `-----BEGIN RSA PRIVATE KEY-----` |
| `ConnectionString` | Database connection strings | `postgresql://user:pass@host/db` |
| `Generic` | Other detected secrets | Various patterns |

### Pattern Matching Approach

Secrets are detected using regex-based pattern matching:

```rust
use security::{Redactor, SecretType, RedactionPattern};

// Create a redactor with standard patterns
let redactor = Redactor::standard();

// Check if text contains secrets
if redactor.contains_secrets(text) {
    // Redact before processing
    let safe_text = redactor.redact(text);
}
```

#### Built-in Patterns

```rust
// API Keys (generic format)
r"(?i)(api[_-]?key|apikey)[=:]\s*['\"]?([a-zA-Z0-9_-]{20,})['\"]?"

// Bearer tokens
r"(?i)bearer\s+([a-zA-Z0-9_.-]+)"

// AWS Access Key
r"(?i)(AKIA[0-9A-Z]{16})"

// GitHub tokens
r"(ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})"

// Private keys
r"-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----"

// JWT tokens
r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*"

// Connection strings
r"(?i)(mongodb|postgresql|mysql|redis|amqp)://[^\s]+"
```

### Redaction Reports

The redactor can identify and report all detected secrets:

```rust
let redactor = Redactor::standard();
let text = "api_key=sk_live_abc123 and token ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

// Find all secrets with their positions
let secrets = redactor.find_secrets(text);

for secret in secrets {
    println!(
        "Found {} at position {}-{}: pattern '{}'",
        secret.secret_type,
        secret.start,
        secret.end,
        secret.pattern_name
    );
}

// Output:
// Found API_KEY at position 0-22: pattern 'api_key'
// Found GITHUB_TOKEN at position 33-73: pattern 'github_token'
```

### Custom Patterns

Add custom patterns for organization-specific secrets:

```rust
let mut redactor = Redactor::new();

// Add a custom pattern
redactor.add_pattern(RedactionPattern::new(
    "internal_token",
    SecretType::Custom("INTERNAL_TOKEN".to_string()),
    r"INT_[A-Z0-9]{32}",
    "[REDACTED:INTERNAL_TOKEN]",
)?);

// Add literal secrets to always redact
redactor.add_literal("my-specific-secret-value", "[REDACTED:SPECIFIC]");
```

### Redaction Modes

```rust
// Standard redaction - replaces with type indicator
let redacted = redactor.redact(text);
// "api_key=sk_live_abc123" -> "[REDACTED:API_KEY]"

// Preserved-length redaction - maintains visual alignment
let preserved = redactor.redact_preserved(text);
// "api_key=sk_live_abc123" -> "api_key=********************"
```

---

## Path Policies

Path policies control which files and directories agents can access.

### Allow/Deny Rules

Rules can either allow or deny access:

```rust
use security::{PathPolicy, PathPolicyBuilder, PathRule, Permission};

let policy = PathPolicyBuilder::restrictive()
    // Allow source files
    .allow("**/*.rs")
    .allow("**/*.toml")
    // Deny sensitive files
    .deny("**/.env")
    .deny("**/secrets*")
    .build();
```

### Permission Types

Four permission types control different operations:

| Permission | Description | Implies |
|------------|-------------|---------|
| `Read` | Read file contents | - |
| `Write` | Modify file contents | Read |
| `Execute` | Execute files | - |
| `Delete` | Delete files | Write |
| `All` | Full access | All permissions |

```rust
// Check specific permissions
if policy.can_read(path) { /* ... */ }
if policy.can_write(path) { /* ... */ }
if policy.can_execute(path) { /* ... */ }
if policy.can_delete(path) { /* ... */ }

// Or check with specific permission
policy.check(path, Permission::Write)?;
```

### Glob Pattern Matching

Path patterns use glob syntax:

| Pattern | Matches |
|---------|---------|
| `*.rs` | Rust files in current directory |
| `**/*.rs` | Rust files in any subdirectory |
| `src/*.rs` | Rust files directly in src/ |
| `src/**/*.rs` | Rust files anywhere under src/ |
| `[abc].txt` | a.txt, b.txt, or c.txt |
| `?.txt` | Single character .txt files |

```rust
// Pattern examples
PathRule::allow("**/*.rs")?           // All Rust files
PathRule::deny("**/target/**")?       // Build output
PathRule::allow("src/**/*")?          // Everything in src
PathRule::deny("**/.git/**")?         // Git internals
```

### Workspace Boundaries

**WARNING**: Paths outside the workspace root are automatically denied.

```rust
let policy = PathPolicyBuilder::restrictive()
    .workspace(PathBuf::from("/home/user/project"))
    .allow("**/*")
    .build();

// This will fail - outside workspace
let result = policy.check(Path::new("/etc/passwd"), Permission::Read);
// Err(PathPolicyError::OutsideWorkspace("/etc/passwd"))
```

### Default Policies

Two built-in policy presets:

```rust
// Restrictive: deny by default, must explicitly allow
let restrictive = PathPolicy::restrictive();

// Permissive: allow by default, must explicitly deny
let permissive = PathPolicy::permissive();
```

#### Standard Development Policy

A pre-configured policy suitable for most development workflows:

```rust
use security::standard_dev_policy;

let policy = standard_dev_policy(PathBuf::from("/workspace"));

// Allows: *.rs, *.toml, *.md, *.txt, *.json, *.yaml, *.yml
// Denies: .env, .env.*, credentials*, secrets*, *.pem, *.key, id_rsa*
// Denies: /etc/**, /root/**, /home/*/.ssh/**
```

### Rule Priority

Rules are evaluated by priority (higher first), then insertion order:

```rust
let policy = PathPolicyBuilder::restrictive()
    // Priority 0 - evaluated second
    .rule(PathRule::allow("**/*")?.with_priority(0))
    // Priority 10 - evaluated first
    .rule(PathRule::deny("**/.env")?.with_priority(10))
    .build();

// .env files are denied despite the allow-all rule
```

---

## Agent Sandboxing

Agents operate within strictly controlled sandboxes.

### Tool Permissions

Each tool can require specific permissions:

```rust
use agents::{Tool, ToolParameter};

// Define a tool with required permissions
let write_tool = Tool::new("write_file", "Write contents to a file", handler)
    .param(ToolParameter::new("path", "string", "File path").required())
    .param(ToolParameter::new("content", "string", "Content").required())
    .requires_permission("file:write");  // Required permission
```

Agents must have matching permissions to use tools:

```rust
let agent = Agent::new("editor", "Code editor agent")
    .with_tool("read_file")
    .with_tool("write_file")
    .with_permission("file:read")
    .with_permission("file:write");

// Permission check happens at tool execution
if !agent.can_use_tool("write_file") {
    return Err("Permission denied");
}
```

### Context Restrictions

Agents receive only the context they need through Context Capsules:

```rust
use agents::{ContextCapsule, CapsuleBuilder, CapsuleField};

// Create a restricted context for the agent
let context = CapsuleBuilder::new("editor_context")
    .string("current_file", "/project/src/main.rs")
    .string("language", "rust")
    // Don't include sensitive project-wide settings
    .build();

let agent = Agent::new("editor", "Editor agent")
    .with_context(context.id);
```

### Evidence-Lock Mode

All agent actions are logged for audit and review:

```rust
use agents::{AuditLog, AuditEntry, AuditLevel};

let audit = AuditLog::new();

// All tool executions are logged
audit.log(AuditEntry::new(
    AuditLevel::Info,
    "tool_executed",
    "Tool read_file executed by agent editor"
).with_agent(agent_id));

// Query audit history
let agent_actions = audit.entries_for_agent(agent_id);
let recent = audit.recent(100);
let errors = audit.entries_by_level(AuditLevel::Error);
```

### No Network Without Explicit Permission

**WARNING**: Agents cannot make network requests unless explicitly granted network permissions.

```rust
// Network tools require explicit permission
let agent = Agent::new("fetcher", "Data fetcher")
    .with_tool("http_get")
    .with_permission("network:read");  // Must be explicitly granted

// Without this permission, network tools will fail
```

---

## Data Flow Security

### What Never Leaves the Machine

The following data is strictly local:

- Source code (unless explicitly shared with agents)
- File system structure
- Git history and credentials
- SSH keys and certificates
- Environment variables
- IDE configuration and preferences
- Undo/redo history
- Search history
- Clipboard contents

### What Can Be Sent (With Redaction)

When agents are enabled and configured to use external services:

| Data Type | Conditions | Redaction |
|-----------|------------|-----------|
| Code snippets | User-selected context | Secrets redacted |
| Error messages | User-initiated | Paths anonymized |
| File names | If context needed | Sensitive names filtered |
| Project structure | Opt-in only | Filtered by path policy |

```rust
// Example: Preparing data for external agent
let redactor = Redactor::standard();
let policy = standard_dev_policy(workspace);

fn prepare_for_agent(content: &str, path: &Path) -> Option<String> {
    // Check path policy first
    if !policy.can_read(path) {
        return None;
    }

    // Redact any secrets
    Some(redactor.redact(content))
}
```

### User Consent Requirements

Data transmission requires explicit user consent:

1. **Initial Setup**: User must enable agent features
2. **Per-Session**: Consent may be required per session (configurable)
3. **Per-Action**: Sensitive operations prompt for confirmation
4. **Audit Trail**: All transmissions are logged locally

---

## Sensitive Paths

Certain paths receive additional protection regardless of policy configuration.

### OS Keychains

System keychain locations are always denied:

| OS | Protected Paths |
|----|-----------------|
| Linux | `~/.local/share/keyrings/`, `/etc/pki/` |
| macOS | `~/Library/Keychains/`, `/Library/Keychains/` |
| Windows | `%APPDATA%\Microsoft\Credentials\` |

### SSH Keys

SSH-related files are protected by default:

```rust
// Always denied in standard_dev_policy:
.deny("/home/*/.ssh/**")
.deny("**/id_rsa*")
.deny("**/*.pem")
.deny("**/*.key")
```

**WARNING**: Never manually allow these paths unless absolutely necessary and you understand the security implications.

### Home Directory Restrictions

Sensitive home directory locations:

```rust
// Protected by default
~/.ssh/           // SSH keys and config
~/.gnupg/         // GPG keys
~/.aws/           // AWS credentials
~/.config/        // Application configs (may contain tokens)
~/.netrc          // Network credentials
~/.docker/        // Docker credentials
```

### System Directories

System directories are denied access:

```rust
.deny("/etc/**")      // System configuration
.deny("/root/**")     // Root home directory
.deny("/var/log/**")  // System logs
```

---

## Configuration

### Security Settings

Security can be configured through the security crate:

```rust
use security::{PathPolicyBuilder, Redactor};

// Configure path policy
let policy = PathPolicyBuilder::restrictive()
    .workspace(workspace_path)
    .follow_symlinks(false)  // Don't follow symlinks (safer)
    .default_allow(false)    // Deny by default
    .allow("src/**/*.rs")
    .deny("**/.env*")
    .build();

// Configure redaction
let mut redactor = Redactor::standard();
redactor.preserve_partial(true);  // Keep partial content for context
redactor.set_placeholder_format("[HIDDEN:{type}]");
```

### Custom Rules

Add organization-specific security rules:

```rust
// Custom path rules
policy.add_rule(
    PathRule::deny("**/proprietary/**")?
        .with_priority(100)
        .with_description("Block access to proprietary code")
);

// Custom secret patterns
redactor.add_pattern(RedactionPattern::new(
    "company_api_key",
    SecretType::Custom("COMPANY_KEY".to_string()),
    r"COMPANY_[A-Z0-9]{40}",
    "[REDACTED:COMPANY_KEY]",
)?);
```

### Audit Configuration

Configure audit logging behavior:

```rust
use agents::{AuditLog, AuditConfig, AuditLevel};

let config = AuditConfig {
    max_entries: 10000,           // Maximum log entries to retain
    min_level: AuditLevel::Info,  // Minimum level to log
    include_debug: false,         // Exclude debug entries
};

let audit = AuditLog::with_config(config);
```

---

## Best Practices

### For Users

1. **Review Agent Permissions**
   - Only grant necessary permissions to agents
   - Regularly audit agent activity logs
   - Disable unused agents

2. **Protect Sensitive Files**
   - Keep secrets in `.env` files (automatically denied)
   - Use workspace boundaries to limit scope
   - Review path policies before enabling agents

3. **Monitor Audit Logs**
   ```rust
   // Regularly review agent activity
   let recent_errors = audit.entries_by_level(AuditLevel::Error);
   let tool_usage = audit.entries_by_type("tool_executed");
   ```

4. **Keep Secrets Out of Code**
   - Use environment variables for secrets
   - Never commit credentials to version control
   - Use the redaction system when sharing code snippets

### For Contributors

1. **Follow Secure Coding Practices**
   - Always use the Redactor before logging user content
   - Check path policies before file operations
   - Validate all agent inputs

2. **Add Security Tests**
   ```rust
   #[test]
   fn test_sensitive_path_denied() {
       let policy = standard_dev_policy(workspace);
       assert!(!policy.can_read(Path::new("/etc/passwd")));
       assert!(!policy.can_read(Path::new("~/.ssh/id_rsa")));
   }

   #[test]
   fn test_secrets_redacted() {
       let redactor = Redactor::standard();
       let text = "api_key=sk_live_secret123";
       let redacted = redactor.redact(text);
       assert!(!redacted.contains("secret123"));
   }
   ```

3. **Document Security Implications**
   - Comment any code that handles sensitive data
   - Document permission requirements for new tools
   - Add warnings for potentially dangerous operations

4. **Use the Audit System**
   ```rust
   // Log security-relevant events
   audit.log(AuditEntry::new(
       AuditLevel::Warning,
       "sensitive_access",
       "Attempted access to sensitive path"
   ).with_data(serde_json::json!({
       "path": path.display().to_string(),
       "denied": true
   })));
   ```

5. **Principle of Least Privilege**
   - Request only needed permissions
   - Scope access to minimum required paths
   - Use restrictive defaults, allow explicitly

---

## Security Contacts

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public issue
2. Email security concerns to the maintainers directly
3. Allow reasonable time for a fix before disclosure

---

## Changelog

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial security documentation |

---

*This document should be reviewed and updated whenever security-related changes are made to the codebase.*
