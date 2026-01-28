# Architecture

**Analysis Date:** 2026-01-27

## Pattern Overview

**Overall:** Layered micro-crate architecture with clear separation of concerns. The IDE is structured as a Rust workspace with 9 semi-independent crates that communicate through well-defined public APIs. The design follows a presentation → application → domain → infrastructure pattern.

**Key Characteristics:**
- **Workspace-based modularization**: Each feature domain (UI, editing, LSP, DAP, agents) is its own crate with independent dependencies
- **Bottom-up dependency**: Lower crates have no knowledge of UI; UI shell depends on everything else
- **Protocol-driven integration**: Crates communicate via structured protocols (JSON-RPC for LSP, DAP types for debugging)
- **Async-first runtime**: Tokio-backed async throughout; real-time responsiveness critical
- **GPU-accelerated rendering**: GPUI framework for immediate-mode rendering like a videogame engine

## Layers

**Presentation (UI):**
- Purpose: Render application UI, handle user input, dispatch commands
- Location: `crates/ui_shell/`
- Contains: GPUI components (app, workspace layout, panes, command palette, file tree, theme)
- Depends on: All lower crates (editor_core, languages, lsp_bridge, dap_bridge, agents, tasks, gitx)
- Used by: User interaction (keyboard, mouse); entry point is `ui_shell::RustideApp`

**Application (Feature Layer):**
- Purpose: Coordinate features across multiple domains; implement IDE workflows
- Locations:
  - Editing workflows: `crates/editor_core/`
  - Language features: `crates/lsp_bridge/`, `crates/dap_bridge/`
  - Integrations: `crates/tasks/`, `crates/gitx/`
  - AI/Agents: `crates/agents/`
- Depends on: Domain models; protocols
- Used by: UI shell to implement IDE commands

**Domain (Core Abstraction Layer):**
- Purpose: Core business logic; language-aware editing and analysis
- Locations:
  - Text editing model: `crates/editor_core/` (Buffer, Cursor, Document, History)
  - Language support: `crates/languages/` (Tree-sitter grammars, syntax highlighting, language registry)
  - Protocol clients: `crates/lsp_bridge/` (LSP client, server manager), `crates/dap_bridge/` (DAP client, debug sessions)
- Depends on: External libraries (ropey, tree-sitter, lsp-types, dap); infrastructure (tokio)
- Used by: Application layer for all feature workflows

**Infrastructure (Foundation Layer):**
- Purpose: Cross-cutting concerns; security, logging, protocol communication
- Locations:
  - Security: `crates/security/` (secret redaction, path policies)
  - (Logging/tracing: Distributed via `tracing` crate, used by all)
- Depends on: Standard library; serde, chrono for serialization/time
- Used by: All higher layers

## Data Flow

**Edit Operation Flow:**
1. User types in `editor_pane` (UI)
2. Input routed to active `Document` in `editor_core`
3. `Document` modifies `Buffer` (rope-backed)
4. `Buffer` records `Edit` transaction to `History`
5. UI re-renders affected regions via GPUI
6. On save: `Document::save()` writes file; propagates to LSP servers via `lsp_bridge::ServerManager`

**LSP Feature Flow:**
1. File change detected in `editor_core::Document`
2. LSP notification sent to servers via `lsp_bridge::LspClient::send_notification()`
3. Server responds with diagnostics/completions
4. `lsp_bridge::DiagnosticStore` accumulates results
5. `lsp_bridge::ServerManager` broadcasts to all consuming crates
6. UI queries `DiagnosticStore` and renders error markers

**Debug Session Flow:**
1. User sets breakpoint in `editor_pane`
2. `dap_bridge::BreakpointManager` stores breakpoint
3. `dap_bridge::DapClient::set_breakpoint()` sends to debugger
4. Program stops; `dap_bridge::DebugSession` receives stopped event
5. `DapClient::stack_trace()` retrieves frames
6. UI displays current line, stack, variables

**Agent Execution Flow:**
1. User invokes agent command via command palette
2. `agents::Agent` receives task
3. `agents::CapsuleBuilder` collects context from workspace state
4. `ContextCapsule` ranks/summarizes files by relevance
5. `security::Redactor` removes secrets before transmission
6. Agent runs with structured context and `ToolRegistry` for IDE operations
7. Results displayed in UI; `AuditLog` records execution

**State Management:**
- **Buffer state**: Owned by `editor_core::Document`; mutations via `Edit` transactions tracked in `History`
- **LSP state**: Centralized in `lsp_bridge::ServerManager`; `DiagnosticStore` is query-able cache
- **Debug state**: Centralized in `dap_bridge::DebugSession`; breakpoints in `BreakpointManager`
- **Workspace state**: Lightweight state in UI shell; crates are largely stateless
- **Synchronization**: `parking_lot::RwLock` for shared state; tokio channels for async communication

## Key Abstractions

**Buffer (Rope-backed Text Store):**
- Purpose: Efficient incremental text editing on large files
- Examples: `crates/editor_core/src/buffer.rs`, `crates/editor_core/src/document.rs`
- Pattern: Ropey rope data structure; O(log n) insert/delete; dirty flag for unsaved changes

**Cursor & Selection:**
- Purpose: Represent editing positions; multi-cursor support
- Examples: `crates/editor_core/src/cursor.rs` (Point, Cursor, Selection types)
- Pattern: Immutable cursor positions; cursor collections managed by Document

**Undo/Redo Transactions:**
- Purpose: Atomic grouping of edits for user-facing undo/redo
- Examples: `crates/editor_core/src/history.rs`, `crates/editor_core/src/edit.rs`
- Pattern: Edit transactions timestamped; History maintains undo/redo stacks; edits are atomic

**LSP Client & Server Manager:**
- Purpose: Spawn language servers; manage JSON-RPC message protocol; aggregate responses
- Examples: `crates/lsp_bridge/src/client.rs`, `crates/lsp_bridge/src/manager.rs`
- Pattern: `LspClient` is per-server; `ServerManager` multiplexes multiple servers; async request/response with timeouts

**Language Grammar & Highlighter:**
- Purpose: Parse source code incrementally; compute syntax colors
- Examples: `crates/languages/src/grammar.rs`, `crates/languages/src/highlighter.rs`
- Pattern: Tree-sitter queries for syntax; theme-based color mapping; HighlightIterator streams events

**Context Capsule (Agent Context):**
- Purpose: Deterministic, structured context for AI agents; token budgets and redaction
- Examples: `crates/agents/src/capsule.rs`, `crates/agents/src/orchestrator.rs`
- Pattern: Builder pattern for constructing; ranked files by relevance; evidence links (path + line numbers); token counting

**Tool Registry (Agent Tools):**
- Purpose: Safe interface for agents to interact with IDE (read files, run tasks, apply edits)
- Examples: `crates/agents/src/tools.rs`
- Pattern: Registry of callable tools; each tool has schema, handler, timeout; execution tracked in `AuditLog`

**Diagnostics Store:**
- Purpose: Centralized cache of LSP diagnostics; queryable by file/range
- Examples: `crates/lsp_bridge/src/diagnostics.rs`
- Pattern: Hashmap-backed by URI; DiagnosticSummary for quick counts

## Entry Points

**Main Application:**
- Location: `crates/ui_shell/src/main.rs`
- Triggers: `cargo run --release -p ui_shell` or binary invocation
- Responsibilities: Initialize logging via `tracing_subscriber`; create and run `RustideApp`

**RustideApp:**
- Location: `crates/ui_shell/src/app.rs`
- Triggers: Called from main.rs
- Responsibilities: GPUI app initialization; register global state; create main window; dispatch commands

**Workspace:**
- Location: `crates/ui_shell/src/workspace.rs`
- Triggers: Created by RustideApp on startup
- Responsibilities: Manage pane layout; maintain open file tabs; route editor events

**Editor Pane:**
- Location: `crates/ui_shell/src/editor_pane.rs`
- Triggers: Opened for each file
- Responsibilities: Render rope buffer with syntax highlighting; handle text input; dispatch edits to Document

**Command Palette:**
- Location: `crates/ui_shell/src/command_palette.rs`
- Triggers: Toggled via action `ToggleCommandPalette`
- Responsibilities: Display searchable command list; dispatch selected commands to app actions

**Language Registry Initialization:**
- Location: `crates/languages/src/lib.rs::init_language_registry()`
- Triggers: Called during UI shell startup
- Responsibilities: Register all built-in language grammars and configs

**Server Manager Initialization:**
- Location: `crates/lsp_bridge/src/manager.rs::ServerManagerBuilder`
- Triggers: Called during workspace initialization
- Responsibilities: Configure which LSP servers to spawn; manage server lifecycle

## Error Handling

**Strategy:** Result-based error propagation with specific error enums per crate; `anyhow::Result` for internal fallibility; `thiserror` for public error types

**Patterns:**
- **Editor errors**: `editor_core::EditorError` (PositionOutOfBounds, InvalidRange, Io, Utf8, NoUndoHistory)
- **LSP errors**: `lsp_bridge::LspClientError` (SpawnError, ServerExited, SendError, Timeout, ServerError)
- **DAP errors**: `dap_bridge::DapError` (similar to LSP)
- **Agent errors**: `agents::orchestrator::OrchestratorError` (AgentNotFound, TaskFailed, CommunicationError)
- **Language errors**: `languages::GrammarError`, `languages::RegistryError`
- **Git errors**: `gitx::GitError`

**Error Recovery:**
- Unreliable servers (LSP/DAP): Logged as warn; gracefully degrade (no completion, no debugging)
- Edit failures: Prevent save; display error in status bar
- Grammar load failures: Fall back to plaintext; log error
- Agent tool failures: Record in audit log; display error to user; continue execution

## Cross-Cutting Concerns

**Logging:**
- Framework: `tracing` crate with `tracing_subscriber`
- Configuration: RUST_LOG environment variable; default level set in main.rs
- Entry point: `crates/ui_shell/src/main.rs` sets up `fmt::layer()` + `EnvFilter`
- Pattern: All modules use `tracing::{debug, info, warn, error, trace}` macros

**Validation:**
- File paths: `security::PathPolicy` enforces allowed directories
- Edit ranges: `editor_core::Buffer` validates byte/char offsets
- LSP payloads: `lsp_types` crate provides type-safe serialization
- DAP payloads: `dap-types` validates protocol messages

**Authentication:**
- For agents: Optional external provider (future); currently sandboxed local-only
- For LSP servers: Credentials passed via environment (future feature)
- For Git: System Git credentials via libgit2

**Syntax Highlighting:**
- Initialized per document: `Document` pairs with `languages::Highlighter`
- Incremental: Tree-sitter updates on each edit via `Language::highlight_range()`
- Cached: Highlight iterators memoize results; invalidated on document change

**Concurrency Model:**
- Async runtime: Single `tokio` runtime per application instance
- Channels: `tokio::sync::mpsc` for async communication (LSP responses, DAP events)
- Shared state: `parking_lot::RwLock` for mutable state (less contention than std)
- Thread safety: Immutable data preferred; critical sections minimized

---

*Architecture analysis: 2026-01-27*
