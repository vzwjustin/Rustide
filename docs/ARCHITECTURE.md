# RustIDE Architecture

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Crate Structure](#crate-structure)
4. [Data Flow](#data-flow)
5. [Threading Model](#threading-model)
6. [Performance Considerations](#performance-considerations)
7. [Extension Points](#extension-points)

---

## Overview

RustIDE is a **Zed-like, Mac-first, pure-Rust integrated development environment** designed for high performance and developer productivity. It combines a GPU-accelerated user interface with context-aware AI agents to deliver a modern editing experience.

### Key Characteristics

- **Pure Rust Implementation**: The entire codebase is written in Rust, ensuring memory safety, performance, and a unified development experience.
- **GPU-Accelerated UI**: Built on GPUI, providing smooth 60+ FPS rendering with native macOS integration.
- **Standards-Based**: Leverages industry-standard protocols (LSP, DAP) and tools (Tree-sitter) for broad language support.
- **Local-First Architecture**: All processing happens locally with no mandatory cloud dependencies.
- **Context-Aware Agents**: Integrated AI agent system with structured context capsules for intelligent assistance.

### High-Level Architecture

```
+------------------------------------------------------------------+
|                           RustIDE                                 |
+------------------------------------------------------------------+
|                                                                   |
|  +-------------------+  +-------------------+  +----------------+ |
|  |     ui_shell      |  |    editor_core    |  |   languages    | |
|  |  (GPUI Frontend)  |  |  (Text Engine)    |  | (Tree-sitter)  | |
|  +-------------------+  +-------------------+  +----------------+ |
|           |                     |                     |           |
|  +-------------------+  +-------------------+  +----------------+ |
|  |    lsp_bridge     |  |    dap_bridge     |  |     tasks      | |
|  |   (LSP Client)    |  |   (DAP Client)    |  | (Task Runner)  | |
|  +-------------------+  +-------------------+  +----------------+ |
|           |                     |                     |           |
|  +-------------------+  +-------------------+  +----------------+ |
|  |       gitx        |  |      agents       |  |    security    | |
|  |  (Git via git2)   |  |(Context Capsules) |  |  (Redaction)   | |
|  +-------------------+  +-------------------+  +----------------+ |
|                                                                   |
+------------------------------------------------------------------+
```

---

## Design Principles

### 1. Pure Rust

The entire codebase is implemented in Rust without external language bindings (except for system libraries). This provides:

- **Memory Safety**: Rust's ownership system prevents common bugs like null pointer dereferences, buffer overflows, and data races.
- **Performance**: Zero-cost abstractions enable high-level code that compiles to efficient machine code.
- **Unified Tooling**: `cargo` manages building, testing, and dependency management uniformly.
- **Type Safety**: Strong static typing catches errors at compile time.

### 2. GPU-Accelerated GPUI

RustIDE uses [GPUI](https://github.com/zed-industries/zed) as its UI framework:

- **Metal Rendering**: Native macOS GPU acceleration via Metal for smooth animations.
- **Immediate Mode**: Efficient rendering model that only updates what changes.
- **Declarative Components**: React-like component model with `View`, `Render`, and state management.
- **Native Integration**: Proper macOS menu bars, keyboard shortcuts (Cmd+key), and window management.

```rust
impl Render for Workspace {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .id("workspace")
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_file_tree(cx))
            .child(self.render_editor_area(cx))
    }
}
```

### 3. Standards-Based Integration

RustIDE embraces established standards rather than reinventing them:

| Standard | Purpose | Implementation |
|----------|---------|----------------|
| **Tree-sitter** | Syntax parsing & highlighting | `languages` crate |
| **LSP** | Language intelligence (completion, diagnostics) | `lsp_bridge` crate |
| **DAP** | Debugging protocol | `dap_bridge` crate |
| **libgit2** | Git operations | `gitx` crate |

### 4. Local-First

All computation happens on the user's machine:

- **No Telemetry**: No data sent to external servers.
- **Offline Capable**: Full functionality without network access.
- **Privacy**: Source code never leaves the local system.
- **Low Latency**: No network round-trips for core operations.

---

## Crate Structure

The project is organized as a Cargo workspace with nine specialized crates:

```
Rustide/
+-- Cargo.toml          # Workspace root
+-- crates/
    +-- ui_shell/       # GPUI application shell
    +-- editor_core/    # Text editing engine
    +-- languages/      # Tree-sitter grammars
    +-- lsp_bridge/     # LSP client
    +-- dap_bridge/     # DAP client
    +-- tasks/          # Task runner
    +-- gitx/           # Git integration
    +-- agents/         # AI agent system
    +-- security/       # Security utilities
```

### ui_shell

**Purpose**: GPUI application shell, workspace management, panes, and command palette.

**Key Components**:

| Module | Description |
|--------|-------------|
| `app.rs` | Main application struct, global state, key bindings, menu setup |
| `workspace.rs` | Main container view orchestrating file tree, editor panes, bottom panel |
| `command_palette.rs` | Fuzzy search command palette with keyboard navigation |
| `editor_pane.rs` | Editor tab management and split views |
| `file_tree.rs` | Hierarchical file browser |
| `bottom_panel.rs` | Terminal, problems, and output panels |
| `theme.rs` | Theme definitions and color schemes |

**Key Types**:

```rust
pub struct RustideApp {
    state: Arc<RwLock<AppState>>,
}

pub struct Workspace {
    focus_handle: FocusHandle,
    file_tree: View<FileTree>,
    editor_pane: View<EditorPane>,
    bottom_panel: View<BottomPanel>,
    command_palette: Option<View<CommandPalette>>,
}
```

**Responsibilities**:
- Application lifecycle management
- Window creation and configuration
- Global keyboard shortcuts (Cmd+N, Cmd+O, Cmd+S, etc.)
- View composition and layout
- Theme application

---

### editor_core

**Purpose**: Rope-backed text buffer, multi-cursor editing, undo/redo, and document model.

**Key Components**:

| Module | Description |
|--------|-------------|
| `buffer.rs` | Rope-backed text storage with O(log n) operations |
| `cursor.rs` | Point, Selection, Cursor, and CursorSet for multi-cursor |
| `document.rs` | Complete document model with file I/O and metadata |
| `edit.rs` | Atomic edit operations (Insert, Delete, Replace) |
| `history.rs` | Transaction-based undo/redo stack |

**Data Structures**:

```rust
pub struct Buffer {
    rope: Rope,          // ropey::Rope for text storage
    history: History,    // Undo/redo history
    dirty: bool,         // Modified since last save
}

pub struct Cursor {
    selection: Selection,          // Anchor and head positions
    preferred_column: Option<usize>, // For vertical movement
}

pub struct CursorSet {
    cursors: Vec<Cursor>,
    primary: usize,      // Index of primary cursor
}
```

**Buffer Operations**:

```
Operation     | Complexity | Description
--------------+------------+----------------------------------
insert()      | O(log n)   | Insert text at byte offset
delete()      | O(log n)   | Delete text in byte range
replace()     | O(log n)   | Replace text in byte range
undo()        | O(k)       | Undo last k edits in transaction
redo()        | O(k)       | Redo last k undone edits
point_to_offset() | O(log n) | Convert (line, col) to byte offset
```

---

### languages

**Purpose**: Tree-sitter grammars, language registry, and syntax highlighting.

**Key Components**:

| Module | Description |
|--------|-------------|
| `registry.rs` | Language registry with lookup by ID, extension, filename |
| `grammar.rs` | Grammar wrapper with highlight queries |
| `highlighter.rs` | Incremental syntax highlighter |
| `config.rs` | Language configuration (LSP, DAP, comments, brackets) |
| `builtin.rs` | Built-in language definitions |

**Supported Languages**:

- Rust (`.rs`)
- JavaScript/TypeScript (`.js`, `.ts`, `.jsx`, `.tsx`)
- Python (`.py`)
- C (`.c`, `.h`)
- Go (`.go`)
- JSON (`.json`)
- TOML (`.toml`)
- Markdown (`.md`)

**Language Definition**:

```rust
pub struct LanguageDefinition {
    pub id: LanguageId,
    pub display_name: String,
    pub extensions: Vec<String>,
    pub file_names: Vec<String>,
    pub grammar: Grammar,
    pub config: LanguageConfig,
}

pub struct LanguageConfig {
    pub lsp: Option<LspConfig>,
    pub dap: Option<DapConfig>,
    pub comments: CommentConfig,
    pub brackets: Vec<BracketPair>,
    pub auto_pairs: Vec<AutoPair>,
}
```

**Highlighting Architecture**:

```
Source Code
     |
     v
+-------------+
| Tree-sitter |  Parse source into AST
|   Parser    |
+-------------+
     |
     v
+-------------+
|   Query     |  Match highlight patterns
|   Engine    |
+-------------+
     |
     v
+-------------+
|   Theme     |  Map captures to colors
|   Mapping   |
+-------------+
     |
     v
Styled Spans
```

---

### lsp_bridge

**Purpose**: LSP client, server lifecycle management, and diagnostics.

**Key Components**:

| Module | Description |
|--------|-------------|
| `client.rs` | Core LSP client with JSON-RPC over stdio |
| `manager.rs` | Server lifecycle management for multiple languages |
| `protocol.rs` | JSON-RPC message types |
| `capabilities.rs` | Server capability parsing |
| `diagnostics.rs` | Diagnostic storage and querying |

**Client State Machine**:

```
Disconnected --> Starting --> Connected --> Initializing --> Ready
                                                              |
                                              <---------------+
                                              |
                                         ShuttingDown --> Shutdown
```

**Server Manager**:

```rust
pub struct ServerManager {
    workspace_root: PathBuf,
    configs: RwLock<HashMap<String, LanguageServerConfig>>,
    servers: RwLock<HashMap<String, ManagedServer>>,
    diagnostics: Arc<DiagnosticStore>,
}
```

**Supported LSP Features**:

- Text synchronization (open, change, save, close)
- Completion with snippets
- Hover documentation
- Go to definition/declaration/implementation
- Find references
- Document symbols
- Code actions
- Formatting
- Rename
- Diagnostics
- Semantic tokens
- Inlay hints

---

### dap_bridge

**Purpose**: DAP client, debug sessions, and breakpoint management.

**Key Components**:

| Module | Description |
|--------|-------------|
| `client.rs` | Core DAP client with JSON message protocol |
| `session.rs` | Debug session state machine |
| `breakpoints.rs` | Breakpoint management |

**Session States**:

```
Inactive --> Initializing --> Ready --> Running <--> Stopped
                                           |
                                           v
                                      Terminated
```

**Debug Capabilities**:

```rust
pub struct DebugSession {
    client: DapClient,
    state: Arc<RwLock<SessionState>>,
    threads: Arc<RwLock<HashMap<i64, ThreadInfo>>>,
    breakpoint_manager: Arc<BreakpointManager>,
}

// Available operations
session.launch(program, args, cwd)
session.attach(pid)
session.continue_execution()
session.step_over()
session.step_into()
session.step_out()
session.pause()
session.get_stack_trace(thread_id)
session.get_variables(scope_ref)
session.evaluate(expression, frame_id)
```

---

### tasks

**Purpose**: Task runner, problem matchers, and build/test execution.

**Key Components**:

| Module | Description |
|--------|-------------|
| `runner.rs` | Task execution engine |
| `problem_matcher.rs` | Parse compiler output for errors/warnings |
| `output.rs` | Output line parsing and classification |

**Task Types**:

```rust
pub enum TaskType {
    Build,    // cargo build
    Test,     // cargo test
    Run,      // cargo run
    Check,    // cargo check
    Clean,    // cargo clean
    Custom,   // User-defined
}

pub struct TaskConfig {
    pub name: String,
    pub task_type: TaskType,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub problem_matcher: Option<String>,
}
```

**Problem Matcher Output**:

```
Compiler Output:
  error[E0382]: borrow of moved value: `x`
    --> src/main.rs:10:5
     |
  10 |     println!("{}", x);
     |                    ^ value borrowed here after move

Parsed Problem:
  Problem {
    severity: Error,
    file: "src/main.rs",
    line: 10,
    column: 5,
    message: "borrow of moved value: `x`",
    code: Some("E0382"),
  }
```

---

### gitx

**Purpose**: Git integration using libgit2.

**Key Components**:

| Module | Description |
|--------|-------------|
| `repo.rs` | Repository operations (open, init, clone) |
| `status.rs` | Working tree status and file states |
| `diff.rs` | Diff computation and hunks |
| `commit.rs` | Commit creation and history |

**File Status Types**:

```rust
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

pub struct StatusEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged: bool,
}
```

**Diff Representation**:

```rust
pub struct FileDiff {
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
    pub hunks: Vec<DiffHunk>,
}

pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}
```

---

### agents

**Purpose**: Context Capsules, tools, orchestration, and audit logging.

**Key Components**:

| Module | Description |
|--------|-------------|
| `capsule.rs` | Structured context containers for agents |
| `tools.rs` | Function registry for agent actions |
| `orchestrator.rs` | Multi-agent coordination |
| `audit.rs` | Action logging and tracking |

**Context Capsule**:

```rust
pub struct ContextCapsule {
    pub id: Uuid,
    pub name: String,
    pub version: u64,
    pub fields: HashMap<String, CapsuleField>,
    pub parent_id: Option<Uuid>,
    pub tags: Vec<String>,
}

// Builder pattern
let capsule = CapsuleBuilder::new("code_context")
    .string("file_path", "/src/main.rs")
    .string("language", "rust")
    .number("cursor_line", 42.0)
    .tag("active")
    .build();
```

**Tool System**:

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    handler: ToolHandler,
    pub permissions: Vec<String>,
}

// Built-in tools
- read_file: Read contents of a file
- write_file: Write contents to a file (requires file:write)
- list_files: List files in a directory
```

**Agent Orchestration**:

```
                    +------------------+
                    |   Orchestrator   |
                    +------------------+
                           |
         +-----------------+-----------------+
         |                 |                 |
    +----v----+       +----v----+       +----v----+
    | Agent 1 |       | Agent 2 |       | Agent N |
    +---------+       +---------+       +---------+
         |                 |                 |
         v                 v                 v
    +----+----+       +----+----+       +----+----+
    | Capsule |       | Capsule |       | Capsule |
    +---------+       +---------+       +---------+
```

---

### security

**Purpose**: Secret redaction and path policies.

**Key Components**:

| Module | Description |
|--------|-------------|
| `redaction.rs` | Pattern-based secret detection and masking |
| `path_policy.rs` | File access control policies |

**Secret Types Detected**:

- API Keys
- Bearer Tokens
- AWS Credentials
- GitHub Tokens
- Private Keys
- Connection Strings
- JWT Tokens
- Passwords in URLs

**Redaction Example**:

```rust
let redactor = Redactor::standard();

// Input
"api_key=sk_live_abc123def456 and token=ghp_1234567890abcdef"

// Output
"[REDACTED:API_KEY] and [REDACTED:GITHUB_TOKEN]"
```

**Path Policy**:

```rust
let policy = PathPolicyBuilder::restrictive()
    .workspace(PathBuf::from("/project"))
    .allow("**/*.rs")
    .allow("**/*.toml")
    .deny("**/.env")
    .deny("**/secrets*")
    .build();

policy.can_read(Path::new("/project/src/main.rs"))  // true
policy.can_read(Path::new("/project/.env"))          // false
policy.can_read(Path::new("/etc/passwd"))            // false (outside workspace)
```

---

## Data Flow

### Document Editing Flow

```
User Input
    |
    v
+----------+     Edit Event     +------------+
| UI Layer | ----------------> | EditorPane |
+----------+                   +------------+
                                     |
                                     v
                              +------------+
                              |  Document  |
                              +------------+
                                     |
          +--------------------------+--------------------------+
          |                          |                          |
          v                          v                          v
    +----------+              +------------+             +----------+
    |  Buffer  |              |  History   |             | Cursors  |
    | (Rope)   |              | (Undo/Redo)|             |          |
    +----------+              +------------+             +----------+
          |
          v
    +----------+
    |Highlighter|
    +----------+
          |
          v
    Rendered View
```

### LSP Integration Flow

```
Document Change
       |
       v
+---------------+        didChange         +---------------+
| ServerManager | -----------------------> |  LSP Server   |
+---------------+                          +---------------+
                                                  |
                                                  v
                                           +------------+
                                           | Analysis   |
                                           +------------+
                                                  |
       +------------------------------------------+
       |
       v
+---------------+    publishDiagnostics    +---------------+
| DiagnosticStore| <---------------------- |  LSP Server   |
+---------------+                          +---------------+
       |
       v
+---------------+
|  UI Update    |
+---------------+
```

### Agent Execution Flow

```
User Request
      |
      v
+-------------+       Task        +-------------+
| Orchestrator| ----------------> |    Agent    |
+-------------+                   +-------------+
                                        |
                                        v
                                 +-------------+
                                 |   Capsule   | (Context)
                                 +-------------+
                                        |
      +---------------------------------+
      |
      v
+-------------+      ToolCall     +-------------+
|   Agent     | ----------------> |    Tool     |
+-------------+                   +-------------+
      ^                                 |
      |                                 v
      |                          +-------------+
      +------------------------- |   Result    |
                                 +-------------+
                                        |
                                        v
                                 +-------------+
                                 |  Audit Log  |
                                 +-------------+
```

---

## Threading Model

### Overview

RustIDE employs a multi-threaded architecture to ensure UI responsiveness:

```
+------------------+     +------------------+     +------------------+
|    Main Thread   |     | Background Pool  |     |   LSP Threads    |
|    (UI/GPUI)     |     |   (Tokio)        |     |   (per server)   |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        |   UI Events           |   Async Tasks          |   JSON-RPC
        |   Rendering           |   File I/O             |   Communication
        |   User Input          |   Task Execution       |
        |                       |   Git Operations       |
        +------------------------+------------------------+
```

### Thread Responsibilities

| Thread | Responsibilities |
|--------|------------------|
| **Main (UI)** | GPUI rendering, user input, view updates |
| **Tokio Runtime** | Async file I/O, network, task execution |
| **LSP Reader** | Read responses from LSP server stdout |
| **DAP Handler** | Debug adapter event processing |
| **Git Worker** | Heavy Git operations (diff, status) |

### Synchronization Primitives

```rust
// Thread-safe state containers
use parking_lot::RwLock;    // Read-heavy concurrent access
use parking_lot::Mutex;      // Exclusive access
use std::sync::Arc;          // Shared ownership

// Async communication
use tokio::sync::mpsc;       // Multi-producer channels
use tokio::sync::oneshot;    // Single-use response channels
use futures::channel::oneshot; // For LSP request/response
```

### Example: LSP Communication

```rust
// Main thread: Send request
let (tx, rx) = oneshot::channel();
pending_requests.insert(id, PendingRequest { sender: tx });
send_raw(&content)?;

// Reader thread: Receive response
if let Some(request) = pending.remove(&request_id) {
    let _ = request.sender.send(result);
}

// Main thread: Await response
let result = rx.await?;
```

---

## Performance Considerations

### 1. Incremental Parsing

Tree-sitter supports incremental parsing, re-parsing only changed regions:

```rust
impl Highlighter {
    pub fn edit(&self, edit: &InputEdit, new_source: &str) -> Result<()> {
        // Apply edit to existing tree
        if let Some(ref mut tree) = *self.tree.write() {
            tree.edit(edit);
        }
        // Re-parse with edited tree (incremental)
        self.parse(new_source)
    }
}
```

**Impact**: O(log n) for typical edits vs O(n) for full re-parse.

### 2. Rope Data Structure

The text buffer uses a rope for efficient editing of large files:

```
                    Root
                   /    \
                 /        \
              Node        Node
             /    \      /    \
          Leaf  Leaf  Leaf  Leaf
          "Hel" "lo"  " Wo" "rld"
```

| Operation | Array | Rope |
|-----------|-------|------|
| Index access | O(1) | O(log n) |
| Insert at position | O(n) | O(log n) |
| Delete range | O(n) | O(log n) |
| Concatenate | O(n) | O(log n) |

### 3. Lazy Loading

Components are loaded on-demand:

- **Language grammars**: Loaded when first file of that type is opened
- **LSP servers**: Started when needed, stopped when idle
- **File contents**: Streamed for large files
- **Git status**: Computed asynchronously

### 4. Caching

Strategic caching reduces redundant computation:

```rust
// Syntax highlighting cache
pub struct Highlighter {
    tree: RwLock<Option<Tree>>,  // Cached parse tree
}

// Language registry cache
pub struct LanguageRegistry {
    extension_map: RwLock<HashMap<String, LanguageId>>,  // Fast lookup
}

// Diagnostic store
pub struct DiagnosticStore {
    diagnostics: RwLock<HashMap<Uri, Vec<Diagnostic>>>,
}
```

### 5. Efficient Rendering

GPUI optimizations:

- **Declarative diffing**: Only updates changed elements
- **GPU acceleration**: Metal-based rendering on macOS
- **Virtual scrolling**: Only renders visible lines
- **Batched updates**: Coalesces rapid changes

---

## Extension Points

### Adding a New Language

1. **Create Grammar**:

```rust
// crates/languages/src/builtin.rs
pub fn get_mylang_language() -> LanguageDefinition {
    let grammar = GrammarLoader::from_language(
        tree_sitter_mylang::LANGUAGE.into()
    ).with_highlights(include_str!("queries/mylang/highlights.scm"));

    LanguageDefinition::builder("mylang", "MyLang")
        .extensions(["ml", "myl"])
        .grammar(grammar.unwrap())
        .config(LanguageConfig {
            lsp: Some(LspConfig {
                command: "mylang-lsp".to_string(),
                args: vec![],
            }),
            comments: CommentConfig {
                line: Some("//".to_string()),
                block: Some(("/*".to_string(), "*/".to_string())),
            },
            brackets: vec![
                BracketPair::new("(", ")"),
                BracketPair::new("{", "}"),
            ],
            ..Default::default()
        })
        .build()
}
```

2. **Register in Registry**:

```rust
// crates/languages/src/builtin.rs
pub fn get_builtin_languages() -> Vec<LanguageDefinition> {
    vec![
        get_rust_language(),
        get_mylang_language(),  // Add here
        // ...
    ]
}
```

3. **Add Highlight Queries**:

Create `crates/languages/src/queries/mylang/highlights.scm`:

```scheme
(function_definition name: (identifier) @function)
(call_expression function: (identifier) @function)
(string_literal) @string
(number_literal) @number
(comment) @comment
["if" "else" "for" "while"] @keyword
```

### Adding a New Tool

```rust
// crates/agents/src/tools.rs
registry.register(
    Tool::new("my_tool", "Description of what it does", |call| async move {
        let param = match call.get_string("param") {
            Some(p) => p,
            None => return ToolResult::failure("Missing param"),
        };

        // Do something
        ToolResult::success(serde_json::json!({
            "result": "value"
        }))
    })
    .param(ToolParameter::new("param", "string", "Parameter description").required())
    .requires_permission("my:permission")
);
```

### Adding a New Theme

```rust
// crates/languages/src/highlighter.rs
impl Theme {
    pub fn my_theme() -> Self {
        let mut theme = Theme::new("MyTheme");

        theme.default_style = ThemeStyle::foreground(0xFFFFFFFF);
        theme.add_style("keyword", ThemeStyle::foreground(0xFF6B6BFF));
        theme.add_style("function", ThemeStyle::foreground(0x6BFFB8FF));
        theme.add_style("string", ThemeStyle::foreground(0xFFD93DFF));
        theme.add_style("comment", ThemeStyle::foreground(0x6B6B6BFF).with_italic());
        // ...

        theme
    }
}
```

### Adding a New Command

```rust
// crates/ui_shell/src/app.rs

// 1. Define the action
actions!(
    rustide,
    [
        // ... existing actions
        MyNewCommand,
    ]
);

// 2. Register keybinding
cx.bind_keys([
    KeyBinding::new("cmd-shift-m", MyNewCommand, None),
]);

// 3. Add to menu
MenuItem::action("My Command", MyNewCommand),

// 4. Implement handler
impl Workspace {
    pub fn handle_my_command(&mut self, _: &MyNewCommand, cx: &mut ViewContext<Self>) {
        // Implementation
    }
}
```

### Adding Custom Path Policies

```rust
// crates/security/src/path_policy.rs
pub fn my_project_policy(workspace: PathBuf) -> PathPolicy {
    PathPolicyBuilder::restrictive()
        .workspace(workspace)
        // Allow source files
        .allow("**/*.rs")
        .allow("**/*.toml")
        // Allow read-only access to vendor
        .allow_with_permissions("vendor/**", vec![Permission::Read])
        // Deny all access to secrets
        .deny("**/.env*")
        .deny("**/secrets/**")
        // Custom rules
        .rule(
            PathRule::deny("**/node_modules/**")
                .unwrap()
                .with_priority(100)
                .with_description("Block node_modules")
        )
        .build()
}
```

---

## Appendix

### Dependency Graph

```
ui_shell
    +-- editor_core
    +-- languages
    +-- lsp_bridge
    +-- dap_bridge
    +-- tasks
    +-- gitx
    +-- agents
    +-- security

editor_core
    +-- ropey
    +-- unicode-segmentation

languages
    +-- tree-sitter
    +-- tree-sitter-*

lsp_bridge
    +-- lsp-types
    +-- tokio

dap_bridge
    +-- tokio

tasks
    +-- tokio

gitx
    +-- git2

agents
    +-- tokio
    +-- chrono
    +-- uuid

security
    +-- regex
```

### Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace dependencies and profiles |
| `rust-toolchain.toml` | Rust version pinning |
| `.gitignore` | Git ignore patterns |

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `RUST_LOG` | Logging level (e.g., `debug`, `info`) |
| `RUSTIDE_WORKSPACE` | Default workspace path |

---

*Document Version: 1.0*
*Last Updated: 2026-01-26*
