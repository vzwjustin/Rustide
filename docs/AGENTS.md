# RustIDE Agents Documentation

This document provides comprehensive documentation for RustIDE's agent system, covering context capsules, tools, orchestration, governance, and configuration.

## Table of Contents

1. [Overview](#overview)
2. [Context Capsules](#context-capsules)
3. [Context Building Pipeline](#context-building-pipeline)
4. [Agent Tools](#agent-tools)
5. [Agent Roles](#agent-roles)
6. [Governance](#governance)
7. [Handoff Bundles](#handoff-bundles)
8. [Audit System](#audit-system)
9. [Configuration](#configuration)

---

## Overview

### What Makes RustIDE's Agents Different

RustIDE agents are fundamentally different from traditional AI coding assistants because they are **always context-aware** through the use of **Context Capsules**. Rather than relying on ad-hoc context gathering or user-provided snippets, every agent operation begins with a deterministically-built capsule that captures the complete state of the development environment.

```
┌─────────────────────────────────────────────────────────────────┐
│                     Traditional Approach                        │
├─────────────────────────────────────────────────────────────────┤
│   User Query  ───►  Agent  ───►  Ad-hoc Context  ───►  Response │
│                       │                                         │
│                       └── Context may be incomplete or stale    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     RustIDE Approach                            │
├─────────────────────────────────────────────────────────────────┤
│   User Query  ───►  Context Capsule  ───►  Agent  ───►  Response│
│                           │                                     │
│                           ├── Goal                              │
│                           ├── Workspace Map                     │
│                           ├── Ranked Files                      │
│                           ├── Editor State                      │
│                           ├── Git Diff                          │
│                           ├── Diagnostics                       │
│                           ├── Evidence                          │
│                           └── Token Budget                      │
└─────────────────────────────────────────────────────────────────┘
```

**Key Benefits:**

- **Deterministic Context**: The same workspace state always produces the same capsule
- **Complete Awareness**: Agents see diagnostics, git changes, and editor state
- **Token Efficiency**: Hierarchical summarization keeps context within budget
- **Auditability**: Every capsule can be inspected and reproduced
- **Security**: Sensitive data is redacted before external calls

---

## Context Capsules

### Purpose and Design

Context Capsules are immutable, versioned snapshots of everything an agent needs to understand and operate on a codebase. They serve as the single source of truth for agent operations, ensuring consistency and reproducibility.

```rust
// From crates/agents/src/capsule.rs
pub struct ContextCapsule {
    /// Unique identifier
    pub id: Uuid,
    /// Capsule name/type
    pub name: String,
    /// Version for optimistic concurrency
    pub version: u64,
    /// Fields in this capsule
    pub fields: HashMap<String, CapsuleField>,
    /// Parent capsule ID (for inheritance)
    pub parent_id: Option<Uuid>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}
```

### Capsule Fields

Each capsule contains structured fields with type information and update timestamps:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `goal` | string | Yes | The user's intent or task description |
| `workspace_map` | object | Yes | Hierarchical representation of the project structure |
| `ranked_files` | array | Yes | Files ranked by relevance to the current task |
| `editor_state` | object | No | Current cursor position, selections, visible range |
| `git_diff` | string | No | Uncommitted changes in the workspace |
| `diagnostics` | array | No | Compiler errors, warnings, and lints |
| `evidence` | array | No | Facts gathered during agent reasoning |
| `token_budget` | number | Yes | Maximum tokens allowed for this operation |
| `redaction_report` | object | No | Summary of redacted sensitive data |

#### Field Definition Example

```rust
pub struct CapsuleField {
    /// Field name
    pub name: String,
    /// Field value
    pub value: Value,
    /// Field type description
    pub field_type: String,
    /// Whether this field is required
    pub required: bool,
    /// Description of the field
    pub description: Option<String>,
    /// When this field was last updated
    pub updated_at: DateTime<Utc>,
}
```

### Building Capsules Deterministically

Capsules are built using the `CapsuleBuilder` pattern, ensuring consistent construction:

```rust
let capsule = CapsuleBuilder::new("task_context")
    .string("goal", "Implement error handling for database operations")
    .field(CapsuleField::array("ranked_files", vec![
        json!({"path": "src/db.rs", "score": 0.95}),
        json!({"path": "src/error.rs", "score": 0.87}),
        json!({"path": "src/lib.rs", "score": 0.72}),
    ]).required())
    .number("token_budget", 8000.0)
    .boolean("has_diagnostics", true)
    .tag("database")
    .tag("error-handling")
    .metadata("source", json!("user_request"))
    .build();
```

The build process is deterministic:
1. Same inputs always produce functionally equivalent capsules
2. UUIDs are generated but can be seeded for testing
3. Timestamps are captured at creation time

### Viewing Capsules in the UI

Capsules can be inspected through multiple interfaces:

**Command Palette:**
```
Ctrl+Shift+P → "Agent: Show Current Context Capsule"
```

**Sidebar Panel:**
- Navigate to the Agent panel in the activity bar
- Expand "Context Capsules" section
- Click on any capsule to view its contents

**JSON Export:**
```rust
let json = capsule.to_json();
// Returns full capsule as serde_json::Value
```

**Console Output:**
```
[Agent Context] Capsule ID: 550e8400-e29b-41d4-a716-446655440000
  Goal: Implement error handling for database operations
  Files: 3 ranked (top: src/db.rs @ 0.95)
  Diagnostics: 2 errors, 1 warning
  Token Budget: 8000 (used: 3247)
  Redactions: 2 secrets removed
```

---

## Context Building Pipeline

The context building pipeline transforms raw workspace state into a focused, token-efficient capsule through five stages:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        Context Building Pipeline                          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                  │
│  │   Signal    │    │  Candidate  │    │   Ranking   │                  │
│  │ Collection  │───►│ Generation  │───►│  Algorithm  │                  │
│  └─────────────┘    └─────────────┘    └─────────────┘                  │
│         │                                      │                         │
│         │                                      ▼                         │
│         │                              ┌─────────────┐                   │
│         │                              │Hierarchical │                   │
│         │                              │Summarization│                   │
│         │                              └─────────────┘                   │
│         │                                      │                         │
│         ▼                                      ▼                         │
│  ┌─────────────┐                       ┌─────────────┐                  │
│  │   Token     │◄──────────────────────│   Final     │                  │
│  │  Budgeting  │                       │   Capsule   │                  │
│  └─────────────┘                       └─────────────┘                  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 1. Signal Collection

Signals are gathered from multiple sources in the IDE:

| Signal Type | Source | Weight | Description |
|-------------|--------|--------|-------------|
| Focus | Editor | High | Currently active file and cursor position |
| Search | Search panel | Medium | Recent search queries and results |
| Diagnostics | LSP | High | Errors, warnings at specific locations |
| Diffs | Git | Medium | Modified files and hunks |
| Symbols | LSP | Variable | Referenced and defined symbols |

```rust
// Signal collection pseudocode
struct SignalCollector {
    focus_signals: Vec<FocusSignal>,
    search_signals: Vec<SearchSignal>,
    diagnostic_signals: Vec<DiagnosticSignal>,
    diff_signals: Vec<DiffSignal>,
    symbol_signals: Vec<SymbolSignal>,
}

impl SignalCollector {
    fn collect(&mut self, workspace: &Workspace) {
        // Gather focus signals from editor state
        self.focus_signals = workspace.editors()
            .filter(|e| e.is_visible())
            .map(|e| FocusSignal::from_editor(e))
            .collect();

        // Gather diagnostic signals
        self.diagnostic_signals = workspace.diagnostics()
            .map(|d| DiagnosticSignal::from_diagnostic(d))
            .collect();

        // ... other signal types
    }
}
```

### 2. Candidate Generation

From signals, candidate files and symbols are generated:

```rust
struct Candidate {
    path: PathBuf,
    symbols: Vec<Symbol>,
    signals: Vec<Signal>,
    initial_score: f64,
}

fn generate_candidates(signals: &SignalCollector) -> Vec<Candidate> {
    let mut candidates = HashMap::new();

    // Files from focus signals get highest initial score
    for signal in &signals.focus_signals {
        candidates.entry(signal.path.clone())
            .or_insert_with(|| Candidate::new(&signal.path))
            .add_signal(signal, 1.0);
    }

    // Files with diagnostics
    for signal in &signals.diagnostic_signals {
        candidates.entry(signal.path.clone())
            .or_insert_with(|| Candidate::new(&signal.path))
            .add_signal(signal, 0.8);
    }

    // Files from git diff
    for signal in &signals.diff_signals {
        candidates.entry(signal.path.clone())
            .or_insert_with(|| Candidate::new(&signal.path))
            .add_signal(signal, 0.6);
    }

    candidates.into_values().collect()
}
```

### 3. Ranking Algorithm

Candidates are ranked using a weighted multi-factor algorithm:

```
Score = w₁(Recency) + w₂(Proximity) + w₃(Relevance) + w₄(Dependencies)
```

| Factor | Weight | Calculation |
|--------|--------|-------------|
| Recency | 0.25 | Time decay: `e^(-λ * seconds_since_access)` |
| Proximity | 0.30 | Graph distance from focus file |
| Relevance | 0.30 | Semantic similarity to goal |
| Dependencies | 0.15 | Import/dependency relationship strength |

```rust
fn rank_candidates(candidates: &mut [Candidate], context: &RankingContext) {
    for candidate in candidates.iter_mut() {
        let recency = calculate_recency(candidate, context);
        let proximity = calculate_proximity(candidate, context);
        let relevance = calculate_relevance(candidate, context);
        let dependencies = calculate_dependencies(candidate, context);

        candidate.final_score =
            0.25 * recency +
            0.30 * proximity +
            0.30 * relevance +
            0.15 * dependencies;
    }

    candidates.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());
}
```

### 4. Hierarchical Summarization

Large files are summarized hierarchically to fit within token budgets:

```
┌─────────────────────────────────────────┐
│              Full File                  │
│  (may exceed token budget)              │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│           Level 1: Outline              │
│  - Module/class declarations            │
│  - Function signatures                  │
│  - Type definitions                     │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│       Level 2: Key Implementations      │
│  - Functions referenced by goal         │
│  - Functions with diagnostics           │
│  - Recently modified functions          │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│        Level 3: Full Content            │
│  - Complete source for top-ranked       │
│  - Focus file always included           │
└─────────────────────────────────────────┘
```

### 5. Token Budgeting

The budget is allocated across capsule sections:

```rust
struct TokenBudget {
    total: usize,
    allocations: HashMap<String, usize>,
}

impl TokenBudget {
    fn allocate(&mut self, total: usize) {
        self.total = total;

        // Fixed allocations
        self.allocations.insert("goal".into(), 200);
        self.allocations.insert("workspace_map".into(), 500);
        self.allocations.insert("diagnostics".into(), 800);
        self.allocations.insert("git_diff".into(), 1000);

        // Remaining goes to ranked files
        let fixed: usize = self.allocations.values().sum();
        self.allocations.insert("ranked_files".into(), total - fixed);
    }

    fn remaining(&self, section: &str) -> usize {
        self.allocations.get(section).copied().unwrap_or(0)
    }
}
```

---

## Agent Tools

Agents interact with the workspace through a controlled set of tools. Each tool has defined parameters, return types, and permission requirements.

### File Operations

#### `list_files`

Lists files in a directory.

```rust
Tool::new("list_files", "List files in a directory", |call| async move {
    let path = call.get_string("path").unwrap_or(".");
    // ... implementation
})
.param(ToolParameter::new("path", "string", "Directory path")
    .with_default(json!(".")))
```

**Parameters:**
| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `path` | string | No | `.` | Directory to list |

**Returns:**
```json
{
  "path": "src",
  "files": ["main.rs", "lib.rs", "utils/"]
}
```

#### `read_file`

Reads the contents of a file.

```rust
Tool::new("read_file", "Read contents of a file", |call| async move {
    let path = call.get_string("path")?;
    // ... implementation
})
.param(ToolParameter::new("path", "string", "Path to the file").required())
```

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | string | Yes | File path to read |

**Returns:**
```json
{
  "content": "fn main() {\n    println!(\"Hello\");\n}",
  "path": "src/main.rs"
}
```

#### `search`

Searches for patterns in the codebase.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | string | Yes | Search pattern (regex supported) |
| `path` | string | No | Directory scope |
| `file_pattern` | string | No | File glob pattern |
| `max_results` | number | No | Maximum results to return |

### Modification Operations

#### `apply_patch`

Applies a unified diff patch to a file.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | string | Yes | Target file |
| `patch` | string | Yes | Unified diff content |

**Permissions Required:** `file:write`

#### `create_file`

Creates a new file with specified content.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | string | Yes | File path to create |
| `content` | string | Yes | Initial content |

**Permissions Required:** `file:write`

#### `delete_file`

Deletes a file from the workspace.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | string | Yes | File path to delete |

**Permissions Required:** `file:delete`

### Task Operations

#### `run_task`

Executes a predefined task (build, test, lint).

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task` | string | Yes | Task name |
| `args` | array | No | Additional arguments |

**Permissions Required:** `task:execute`

#### `get_diagnostics`

Retrieves current diagnostics from the language server.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | string | No | Filter by file path |
| `severity` | string | No | Filter by severity level |

### Git Operations

#### `get_git_status`

Gets the current git status.

**Returns:**
```json
{
  "branch": "feature/agents",
  "modified": ["src/lib.rs"],
  "staged": ["src/main.rs"],
  "untracked": ["notes.txt"]
}
```

#### `get_git_diff`

Gets the current git diff.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `staged` | boolean | No | Show staged changes only |
| `path` | string | No | Filter by file path |

### Tool Permissions and Sandboxing

Tools are subject to permission checks and sandboxing:

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    handler: ToolHandler,
    /// Required permissions
    pub permissions: Vec<String>,
}
```

**Permission Levels:**

| Permission | Description |
|------------|-------------|
| `file:read` | Read file contents |
| `file:write` | Modify or create files |
| `file:delete` | Delete files |
| `task:execute` | Run build/test tasks |
| `git:read` | Read git state |
| `git:write` | Perform git operations |
| `*` | Full access (admin) |

**Sandboxing Enforcement:**

```rust
// Path policy enforcement
let policy = PathPolicyBuilder::restrictive()
    .workspace(workspace_root)
    .allow("**/*.rs")
    .allow("**/*.toml")
    .deny("**/.env")
    .deny("**/secrets*")
    .build();

// Check before tool execution
if !policy.can_write(&path) {
    return ToolResult::failure("Access denied by path policy");
}
```

---

## Agent Roles

RustIDE supports specialized agent roles, each with distinct behaviors and tool access:

### Planner

**Purpose:** Analyzes tasks and creates implementation plans.

**Behaviors:**
- Examines workspace structure and dependencies
- Breaks complex tasks into subtasks
- Identifies affected files and potential risks
- Does NOT modify files directly

**Tools Available:**
- `list_files`, `read_file`, `search`
- `get_diagnostics`, `get_git_status`

```rust
let planner = Agent::new("planner", "Creates implementation plans")
    .with_tool("list_files")
    .with_tool("read_file")
    .with_tool("search")
    .with_tool("get_diagnostics")
    .with_permission("file:read");
```

### Implementer

**Purpose:** Executes code changes based on plans.

**Behaviors:**
- Applies patches and creates files
- Follows plans from Planner agent
- Handles merge conflicts
- Runs incremental builds

**Tools Available:**
- All read tools
- `apply_patch`, `create_file`, `delete_file`
- `run_task`

```rust
let implementer = Agent::new("implementer", "Executes code changes")
    .with_tool("*")  // All tools
    .with_permission("file:read")
    .with_permission("file:write")
    .with_permission("task:execute");
```

### Verifier

**Purpose:** Validates changes through testing and analysis.

**Behaviors:**
- Runs test suites
- Checks for regressions
- Validates type correctness
- Reports verification results

**Tools Available:**
- All read tools
- `run_task`, `get_diagnostics`

```rust
let verifier = Agent::new("verifier", "Validates changes")
    .with_tool("read_file")
    .with_tool("run_task")
    .with_tool("get_diagnostics")
    .with_permission("file:read")
    .with_permission("task:execute");
```

### Reviewer

**Purpose:** Reviews changes for quality and best practices.

**Behaviors:**
- Analyzes code quality
- Checks for security issues
- Suggests improvements
- Validates documentation

**Tools Available:**
- All read tools
- `search`, `get_git_diff`

### Documenter

**Purpose:** Creates and updates documentation.

**Behaviors:**
- Generates doc comments
- Updates README files
- Creates API documentation
- Maintains changelogs

**Tools Available:**
- All read tools
- `apply_patch`, `create_file` (limited to `.md` files)

---

## Governance

### Evidence-Lock Mode

Evidence-lock mode ensures agents only act on verified information:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Evidence-Lock Flow                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────┐    ┌──────────┐    ┌──────────┐    ┌─────────┐  │
│   │ Gather  │───►│ Validate │───►│  Lock    │───►│  Act    │  │
│   │Evidence │    │ Evidence │    │ Evidence │    │         │  │
│   └─────────┘    └──────────┘    └──────────┘    └─────────┘  │
│                        │                               │        │
│                        │ Invalid                       │        │
│                        ▼                               ▼        │
│                  ┌──────────┐                   ┌──────────┐   │
│                  │  Reject  │                   │  Audit   │   │
│                  │  Action  │                   │  Trail   │   │
│                  └──────────┘                   └──────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Rules:**
1. Every claim must have supporting evidence in the capsule
2. Evidence must be gathered through tool calls, not assumed
3. Stale evidence (older than configured threshold) is rejected
4. Actions are logged with their supporting evidence

### Safety Checks

Before any modification, safety checks are performed:

```rust
fn safety_check(action: &Action, context: &ContextCapsule) -> Result<(), SafetyError> {
    // 1. Path policy check
    if !context.path_policy.allows(&action.target_path, action.permission)? {
        return Err(SafetyError::PathDenied);
    }

    // 2. Scope check - action must relate to goal
    if !action.relates_to_goal(&context.goal)? {
        return Err(SafetyError::OutOfScope);
    }

    // 3. Destructive action check
    if action.is_destructive() && !context.allows_destructive {
        return Err(SafetyError::DestructiveNotAllowed);
    }

    // 4. Rate limit check
    if context.action_count >= context.max_actions {
        return Err(SafetyError::RateLimitExceeded);
    }

    Ok(())
}
```

### Redaction Before External Calls

All data sent to external services is redacted:

```rust
// From crates/security/src/redaction.rs
let redactor = Redactor::standard();

// Standard patterns redacted:
// - API keys: api_key=sk_live_xxx → [REDACTED:API_KEY]
// - Bearer tokens: Bearer eyJ... → Bearer [REDACTED:TOKEN]
// - URL passwords: user:pass@host → user:[REDACTED:PASSWORD]@host
// - AWS credentials: AKIA... → [REDACTED:AWS_ACCESS_KEY]
// - GitHub tokens: ghp_... → [REDACTED:GITHUB_TOKEN]
// - Private keys: -----BEGIN RSA PRIVATE KEY----- → [REDACTED:PRIVATE_KEY]
// - JWT tokens: eyJ...eyJ...xxx → [REDACTED:JWT]
// - Connection strings: postgres://... → [REDACTED:CONNECTION_STRING]

// Before sending to external API:
let safe_content = redactor.redact(&capsule_content);
```

**Redaction Report:**
```json
{
  "total_redactions": 3,
  "by_type": {
    "API_KEY": 1,
    "PASSWORD": 1,
    "GITHUB_TOKEN": 1
  },
  "files_affected": ["src/config.rs", ".env.example"]
}
```

---

## Handoff Bundles

Handoff bundles enable seamless transitions between agents or sessions.

### Contents

A handoff bundle contains:

```rust
struct HandoffBundle {
    /// The context capsule at handoff time
    capsule: ContextCapsule,
    /// Completed task history
    completed_tasks: Vec<AgentTask>,
    /// Pending tasks for the next agent
    pending_tasks: Vec<AgentTask>,
    /// Evidence gathered
    evidence: Vec<Evidence>,
    /// Tool call history
    tool_history: Vec<ToolCall>,
    /// Intermediate artifacts
    artifacts: HashMap<String, Value>,
    /// Handoff metadata
    metadata: HandoffMetadata,
}

struct HandoffMetadata {
    from_agent: Uuid,
    to_agent: Option<Uuid>,
    reason: String,
    timestamp: DateTime<Utc>,
    priority: i32,
}
```

### When Used

Handoff bundles are created in these scenarios:

1. **Role Transition**: Planner completes and hands off to Implementer
2. **Session Persistence**: User closes IDE with pending work
3. **Error Recovery**: Agent fails and hands off for retry
4. **Parallel Work**: Task is split across multiple agents
5. **Human-in-the-Loop**: Agent pauses for user approval

### Example Handoff

```rust
// Planner completing and handing off to Implementer
let bundle = HandoffBundle {
    capsule: current_capsule.clone(),
    completed_tasks: vec![
        AgentTask::completed("Analyze codebase structure", json!({"files": 47})),
        AgentTask::completed("Identify affected modules", json!({"modules": ["db", "api"]})),
    ],
    pending_tasks: vec![
        AgentTask::new("Implement error handling in db.rs", json!({
            "file": "src/db.rs",
            "changes": [
                {"line": 45, "action": "wrap_in_result"},
                {"line": 67, "action": "add_error_type"},
            ]
        })),
    ],
    evidence: vec![
        Evidence::new("db.rs has 3 unwrap() calls that need handling"),
        Evidence::new("Error type already defined in src/error.rs"),
    ],
    tool_history: tool_calls.clone(),
    artifacts: HashMap::new(),
    metadata: HandoffMetadata {
        from_agent: planner.id,
        to_agent: Some(implementer.id),
        reason: "Planning complete, ready for implementation".into(),
        timestamp: Utc::now(),
        priority: 10,
    },
};
```

---

## Audit System

The audit system provides comprehensive logging of all agent activities.

### Audit Levels

```rust
pub enum AuditLevel {
    Debug,     // Detailed debugging information
    Info,      // Normal operational events
    Warning,   // Potential issues
    Error,     // Operation failures
    Critical,  // System-level failures
}
```

### Audit Entries

Each entry captures:

```rust
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub level: AuditLevel,
    pub event_type: String,
    pub message: String,
    pub agent_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub data: Option<Value>,
    pub source: Option<String>,
}
```

### Logging Operations

```rust
// Creating entries
let entry = AuditEntry::info("task_started", "Beginning implementation")
    .with_agent(agent.id)
    .with_task(task.id)
    .with_data(json!({"file": "src/main.rs"}));

audit_log.log(entry);

// Using the macro with source location
audit!(log, AuditLevel::Info, "tool_call", "Executing read_file");
```

### Viewing Audit Logs

**Filter by Level:**
```rust
let errors = audit_log.entries_by_level(AuditLevel::Error);
```

**Filter by Agent:**
```rust
let agent_activity = audit_log.entries_for_agent(agent_id);
```

**Filter by Task:**
```rust
let task_history = audit_log.entries_for_task(task_id);
```

**Filter by Time Range:**
```rust
let recent = audit_log.entries_in_range(start_time, end_time);
```

**Search:**
```rust
let results = audit_log.search("database");
```

### Exporting Audit Logs

**JSON Export:**
```rust
let json = audit_log.export_json();
// Returns: [{"id": "...", "level": "INFO", ...}, ...]
```

**Text Export:**
```rust
let text = audit_log.export_text();
// Returns:
// 2024-01-15 10:23:45.123 [INFO] [task_started] Beginning implementation agent=xxx
// 2024-01-15 10:23:46.456 [INFO] [tool_call] Executing read_file task=yyy
```

---

## Configuration

### Enabling Agents

Agents can be enabled through the configuration file or settings UI:

**settings.json:**
```json
{
  "rustide.agents.enabled": true,
  "rustide.agents.defaultRole": "planner",
  "rustide.agents.autoAssign": true
}
```

### Agent Configuration

```json
{
  "rustide.agents": {
    "enabled": true,
    "roles": {
      "planner": {
        "enabled": true,
        "maxConcurrentTasks": 3,
        "tokenBudget": 8000
      },
      "implementer": {
        "enabled": true,
        "requireApproval": true,
        "maxFilesPerTask": 10
      },
      "verifier": {
        "enabled": true,
        "autoRun": true,
        "testTimeout": 300
      }
    }
  }
}
```

### Context Capsule Configuration

```json
{
  "rustide.context": {
    "tokenBudget": 16000,
    "maxRankedFiles": 20,
    "summarizationLevel": "balanced",
    "signals": {
      "focus": { "weight": 0.3 },
      "diagnostics": { "weight": 0.25 },
      "gitDiff": { "weight": 0.2 },
      "search": { "weight": 0.15 },
      "symbols": { "weight": 0.1 }
    }
  }
}
```

### Security Configuration

```json
{
  "rustide.security": {
    "redaction": {
      "enabled": true,
      "patterns": ["standard"],
      "customPatterns": [
        {
          "name": "internal_token",
          "pattern": "INT_[A-Z0-9]{32}",
          "replacement": "[REDACTED:INTERNAL]"
        }
      ]
    },
    "pathPolicy": {
      "defaultAllow": false,
      "rules": [
        { "pattern": "**/*.rs", "allow": true },
        { "pattern": "**/.env*", "allow": false },
        { "pattern": "**/secrets/**", "allow": false }
      ]
    },
    "sandbox": {
      "enabled": true,
      "workspaceOnly": true,
      "followSymlinks": false
    }
  }
}
```

### Audit Configuration

```json
{
  "rustide.audit": {
    "enabled": true,
    "minLevel": "info",
    "maxEntries": 10000,
    "includeDebug": false,
    "persistToFile": true,
    "filePath": ".rustide/audit.log",
    "rotateSize": "10MB"
  }
}
```

### Programmatic Configuration

```rust
use rustide_agents::{AgentOrchestrator, Agent, ToolRegistry, CapsuleStore};

// Create orchestrator with custom configuration
let tools = Arc::new(ToolRegistry::new());
let capsules = Arc::new(CapsuleStore::new());
let orchestrator = AgentOrchestrator::new(tools, capsules);

// Register custom agent
let custom_agent = Agent::new("custom", "Custom agent for specific tasks")
    .with_tool("read_file")
    .with_tool("search")
    .with_permission("file:read")
    .with_context(context_capsule.id);

orchestrator.register_agent(custom_agent);

// Configure audit
let audit_config = AuditConfig {
    max_entries: 5000,
    min_level: AuditLevel::Info,
    include_debug: false,
};
orchestrator.audit_log().configure(audit_config);
```

---

## API Reference

For detailed API documentation, see:
- `crates/agents/src/capsule.rs` - Context Capsule types
- `crates/agents/src/tools.rs` - Tool definitions
- `crates/agents/src/orchestrator.rs` - Agent orchestration
- `crates/agents/src/audit.rs` - Audit logging
- `crates/security/src/redaction.rs` - Secret redaction
- `crates/security/src/path_policy.rs` - Path access control
