# Codebase Structure

**Analysis Date:** 2026-01-27

## Directory Layout

```
Rustide/
├── Cargo.toml                 # Workspace manifest (9 member crates)
├── Cargo.lock                 # Locked dependency versions
├── rust-toolchain.toml        # Rust version constraint
├── README.md                  # Project overview and quick start
├── LICENSE                    # MIT license
├── .gitignore                 # Git exclusions
├── .git/                      # Git repository
├── .planning/                 # Planning & analysis documents
│   └── codebase/              # Generated architecture/structure docs
├── docs/                      # Developer documentation
│   ├── ARCHITECTURE.md        # System design (user-facing)
│   ├── AGENTS.md              # Context Capsules and agent system
│   ├── DEBUGGING.md           # DAP debugging setup
│   ├── LANGUAGES.md           # Multi-language support
│   └── SECURITY.md            # Privacy and redaction
└── crates/                    # Workspace member crates
    ├── ui_shell/              # GPUI application shell (entry point)
    ├── editor_core/           # Rope-backed text editing
    ├── languages/             # Tree-sitter syntax + highlighting
    ├── lsp_bridge/            # Language Server Protocol client
    ├── dap_bridge/            # Debug Adapter Protocol client
    ├── tasks/                 # Task runner (build, test, etc.)
    ├── gitx/                  # Git integration via libgit2
    ├── agents/                # Context Capsules + agent orchestration
    └── security/              # Secret redaction + path policies
```

## Directory Purposes

**Root Level:**
- Purpose: Workspace root; Rust project metadata and CI configuration
- Contains: Cargo workspace manifest, LICENSE, README, docs folder
- Key files: `Cargo.toml` (workspace members, shared dependencies), `rust-toolchain.toml` (Rust 1.75+), `.gitignore`

**crates/**
- Purpose: Container for all workspace member crates; allows independent versioning and dependencies (all share workspace version)
- Contains: 9 crates organized by domain concern
- Key pattern: Each crate is autonomous; no crate-to-crate circular dependencies

**crates/ui_shell/**
- Purpose: Main application binary and UI layer; GPUI-based immediate-mode rendering
- Contains: Application state, GPUI components, command handlers, event routing
- Key files:
  - `src/main.rs` - Binary entry point; logging initialization
  - `src/app.rs` - `RustideApp` struct; app initialization; global state (`AppState`)
  - `src/workspace.rs` - Pane layout management; multi-pane support
  - `src/editor_pane.rs` - Individual editor view; rope rendering + syntax colors
  - `src/file_tree.rs` - Workspace file browser; directory tree with file icons
  - `src/command_palette.rs` - Searchable command list; fuzzy matching
  - `src/bottom_panel.rs` - Task output, diagnostics, debug console
  - `src/theme.rs` - Color scheme definitions; light/dark theme support

**crates/editor_core/**
- Purpose: Text editing engine; buffer model, cursors, undo/redo
- Contains: Core data structures for document editing
- Key files:
  - `src/lib.rs` - Public API; error types (`EditorError`, `Result`)
  - `src/buffer.rs` - `Buffer` struct wrapping ropey::Rope; insert/delete/text operations
  - `src/document.rs` - `Document` struct; file I/O, dirty flag, line endings
  - `src/cursor.rs` - `Point` (line, column), `Cursor`, `Selection` (ranges)
  - `src/edit.rs` - `Edit` struct representing atomic text change; `EditKind` (Insert, Delete, Replace)
  - `src/history.rs` - `History` and `Transaction` for undo/redo; transaction-based grouping

**crates/languages/**
- Purpose: Language support infrastructure; Tree-sitter parsing and syntax highlighting
- Contains: Grammar loading, theme-aware highlighting, language configuration
- Key files:
  - `src/lib.rs` - Public API; `init_language_registry()` entry point
  - `src/builtin.rs` - Built-in language definitions (Rust, TypeScript, Python, Go, C, JSON)
  - `src/grammar.rs` - `Grammar` struct wrapping tree-sitter; `GrammarLoader` for .so loading
  - `src/config.rs` - `LanguageConfig` with LSP/DAP/comment/bracket settings
  - `src/registry.rs` - `LanguageRegistry` (singleton); lookup by ID, extension, or filename
  - `src/highlighter.rs` - `Highlighter` and `HighlightIterator` for syntax colors; `Theme` and `ThemeStyle`

**crates/lsp_bridge/**
- Purpose: Language Server Protocol client and multiplexer
- Contains: Server process spawning, JSON-RPC communication, capability tracking
- Key files:
  - `src/lib.rs` - Public API; exported types for re-export
  - `src/client.rs` - `LspClient` (per-server); spawns process; sends/receives JSON-RPC messages
  - `src/manager.rs` - `ServerManager` multiplexes multiple servers; `ServerManagerBuilder` for config
  - `src/protocol.rs` - `JsonRpcMessage`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`
  - `src/diagnostics.rs` - `DiagnosticStore` (URI → Diagnostic[]), `DiagnosticSummary` for quick access
  - `src/capabilities.rs` - `ServerCapabilityInfo` parsed from LSP initialize response

**crates/dap_bridge/**
- Purpose: Debug Adapter Protocol client; debug session management and breakpoints
- Contains: Debugger process spawning, debug session state, breakpoint storage
- Key files:
  - `src/lib.rs` - Public API
  - `src/client.rs` - `DapClient` (per-session); spawns debugger; sends DAP messages
  - `src/session.rs` - `DebugSession` (session state); `SessionState` (idle, running, stopped)
  - `src/breakpoints.rs` - `Breakpoint` struct; `BreakpointManager` for breakpoint storage/sync

**crates/tasks/**
- Purpose: Task runner for build, test, and custom commands; output parsing
- Contains: Task execution, problem matching, diagnostic extraction
- Key files:
  - `src/lib.rs` - Public API; `TaskRunner`, `TaskStatus`
  - `src/runner.rs` - `Task` struct; `TaskRunner` spawns and streams output
  - `src/problem_matcher.rs` - `ProblemMatcher` parses compiler/test output; extracts diagnostics
  - `src/output.rs` - `OutputLine`, `OutputType` (stderr/stdout); line-by-line streaming

**crates/gitx/**
- Purpose: Git integration via libgit2; repository operations
- Contains: Repository status, diff, commit, clone operations
- Key files:
  - `src/lib.rs` - Public API
  - `src/repo.rs` - `Repository` struct wrapping git2::Repository
  - `src/status.rs` - `FileStatus`, `WorktreeStatus`, `StatusEntry` (staged/unstaged/untracked)
  - `src/diff.rs` - `FileDiff`, `DiffHunk`, `DiffLine` (line-by-line diffs with +/- markers)
  - `src/commit.rs` - `CommitInfo`, `CommitBuilder` for creating commits

**crates/agents/**
- Purpose: Context-aware agent system; AI assistant orchestration and tooling
- Contains: Context Capsules, tool registry, agent orchestration, audit logging
- Key files:
  - `src/lib.rs` - Public API
  - `src/capsule.rs` - `ContextCapsule` (ranked/summarized workspace context); `CapsuleBuilder` for construction
  - `src/tools.rs` - `Tool` interface; `ToolRegistry` for tool discovery; `ToolResult` with execution time
  - `src/orchestrator.rs` - `Agent`, `AgentOrchestrator` for multi-agent coordination; `AgentState`, `AgentTask`
  - `src/audit.rs` - `AuditLog`, `AuditEntry`, `AuditLevel` (Info, Warn, Error) for execution tracking

**crates/security/**
- Purpose: Security-critical functionality; secret redaction, path policies
- Contains: Redaction patterns, path-based access control
- Key files:
  - `src/lib.rs` - Public API
  - `src/redaction.rs` - `Redactor` with patterns (API_KEY, PASSWORD, etc.); `RedactionPattern` for custom patterns
  - `src/path_policy.rs` - `PathPolicy` with `PathRule` (allow/deny); `Permission` enum

**docs/**
- Purpose: User-facing architecture and integration documentation
- Contains: Design explanations, setup guides, feature descriptions
- Key files:
  - `ARCHITECTURE.md` - System design overview (similar to this but user-focused)
  - `AGENTS.md` - Context Capsule system explained
  - `DEBUGGING.md` - DAP setup for different debuggers
  - `LANGUAGES.md` - How to add new language support
  - `SECURITY.md` - Privacy model and redaction details

## Key File Locations

**Entry Points:**
- `crates/ui_shell/src/main.rs` - Binary entry; logging setup; app launch
- `crates/ui_shell/src/app.rs::RustideApp::run()` - GPUI application initialization
- `crates/languages/src/lib.rs::init_language_registry()` - Language support initialization
- `crates/lsp_bridge/src/manager.rs::ServerManagerBuilder::build()` - LSP server setup

**Configuration:**
- `Cargo.toml` - Workspace dependencies, members, profiles, version constraints
- `rust-toolchain.toml` - Rust version requirement (stable 1.75+)
- `crates/*/Cargo.toml` - Per-crate dependencies (all version "workspace")

**Core Logic:**
- Text editing: `crates/editor_core/src/buffer.rs` (rope operations), `src/history.rs` (undo/redo)
- Syntax highlighting: `crates/languages/src/highlighter.rs` (Tree-sitter query execution)
- LSP communication: `crates/lsp_bridge/src/client.rs` (JSON-RPC), `src/manager.rs` (multiplexing)
- Debug sessions: `crates/dap_bridge/src/session.rs` (state machine), `src/breakpoints.rs` (storage)
- Agent context: `crates/agents/src/capsule.rs` (ranking/summarization), `src/tools.rs` (tool dispatch)

**Testing:**
- Unit tests co-located with source: `#[cfg(test)]` modules in each `.rs` file
- Test modules in: `crates/editor_core/src/lib.rs`, `crates/languages/src/lib.rs`, `crates/lsp_bridge/src/lib.rs`
- No separate `tests/` directories; integration tests inline
- Run via `cargo test --workspace`

## Naming Conventions

**Files:**
- Pattern: `snake_case.rs` for modules
- Examples: `buffer.rs`, `editor_pane.rs`, `command_palette.rs`

**Directories:**
- Pattern: `snake_case` for crate names and subdirectories
- Examples: `ui_shell`, `editor_core`, `lsp_bridge`

**Modules:**
- Pattern: Public modules export via `pub use` statements
- Examples: `pub mod buffer; pub use buffer::Buffer;` in lib.rs

**Types:**
- Pattern: `PascalCase` for structs, enums, traits
- Examples: `Buffer`, `Document`, `Cursor`, `RustideApp`, `ServerManager`

**Functions:**
- Pattern: `snake_case` for all functions
- Examples: `insert()`, `delete()`, `undo()`, `start_server()`, `set_breakpoint()`

**Constants/Statics:**
- Pattern: `SCREAMING_SNAKE_CASE` for constants
- Examples: `DEFAULT_BUFFER_SIZE`, `MAX_UNDO_HISTORY`

## Where to Add New Code

**New Feature (e.g., "Add markdown preview"):**
- Primary code: `crates/languages/src/` (register markdown grammar); `crates/ui_shell/src/` (add PreviewPane component)
- Tests: Inline tests in implementation file + integration test (future: `crates/ui_shell/tests/`)
- Configuration: `crates/languages/src/builtin.rs` (markdown config)

**New Language Support:**
- Grammar: `crates/languages/src/builtin.rs::get_builtin_languages()` (register tree-sitter grammar)
- Config: `crates/languages/src/config.rs::LanguageConfig` (LSP server command, comment style)
- Registry: Automatically picked up by `init_language_registry()`

**New LSP Feature (e.g., "Add code actions"):**
- Protocol: `crates/lsp_bridge/src/protocol.rs` (new JSON-RPC message types)
- Client: `crates/lsp_bridge/src/client.rs` (send request)
- Manager: `crates/lsp_bridge/src/manager.rs` (broadcast response)
- UI: `crates/ui_shell/src/` (display in UI)

**New Agent Tool:**
- Tool definition: `crates/agents/src/tools.rs::ToolRegistry` (register new tool with schema)
- Implementation: Same file or dedicated module
- Audit: `crates/agents/src/audit.rs` (tool calls logged automatically)

**New UI Component:**
- Component: `crates/ui_shell/src/*.rs` (new file or extend existing)
- State: Incorporated into workspace or pane state
- Commands: Register actions in `crates/ui_shell/src/app.rs::actions!` macro
- Styling: Use theme from `crates/ui_shell/src/theme.rs`

**Shared Utilities:**
- Editor utilities: `crates/editor_core/src/` (extend Buffer or Document)
- String utilities: No dedicated crate; add to `editor_core` or create new utility crate
- Path utilities: `crates/security/src/path_policy.rs` (path validation)

**New Integration (external service):**
- Feature crate: Create new `crates/feature_name/` (e.g., `crates/github_api/`)
- Follow pattern: Domain crate → specific feature
- Dependency: Add to workspace members in root `Cargo.toml`
- Integration in UI: Add handlers in `crates/ui_shell/src/`

## Special Directories

**crates/*/src/:**
- Purpose: Source code directory for each crate
- Generated: No
- Committed: Yes
- Pattern: lib.rs as crate root; modules for each major feature

**target/**
- Purpose: Build artifacts and compiled binaries
- Generated: Yes (by `cargo build`)
- Committed: No (in `.gitignore`)

**.planning/codebase/**
- Purpose: Generated architecture analysis documents
- Generated: Yes (by GSD mapper)
- Committed: Yes (for team reference)
- Files: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, STACK.md, INTEGRATIONS.md, CONCERNS.md

**docs/**
- Purpose: User and developer documentation
- Generated: Manually maintained
- Committed: Yes
- Scope: Architecture overview, integration guides, debugging setup, security policy

**Cargo.lock**
- Purpose: Locked dependency versions for reproducible builds
- Generated: By `cargo` (commit to repo for binary releases)
- Committed: Yes

---

*Structure analysis: 2026-01-27*
