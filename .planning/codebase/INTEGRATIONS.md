# External Integrations

**Analysis Date:** 2026-01-27

## APIs & External Services

**Language Servers (LSP):**
- rust-analyzer - Rust language intelligence
  - SDK/Client: lsp-types 0.97, custom client in `crates/lsp_bridge/`
  - Auth: None (local processes)
  - Spawned via: `tokio::process::Command` with environment variables

- typescript-language-server - TypeScript/JavaScript support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

- pyright - Python language support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

- gopls - Go language support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

- clangd - C/C++ language support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

- vscode-json-language-server - JSON support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

- marksman - Markdown support
  - SDK/Client: lsp-types 0.97
  - Auth: None (local processes)

**Debuggers (DAP - Debug Adapter Protocol):**
- codelldb - Rust debugger
  - SDK/Client: dap 0.4.1-alpha1, custom client in `crates/dap_bridge/`
  - Auth: None (local processes)
  - Process spawning: `tokio::process::Command` with stdio piping

- node (debugger) - JavaScript/TypeScript debugging
  - SDK/Client: dap 0.4.1-alpha1
  - Auth: None (local processes)

- debugpy - Python debugger
  - SDK/Client: dap 0.4.1-alpha1
  - Auth: None (local processes)

- delve - Go debugger
  - SDK/Client: dap 0.4.1-alpha1
  - Auth: None (local processes)

- gdb/lldb - C/C++ debuggers
  - SDK/Client: dap 0.4.1-alpha1
  - Auth: None (local processes)

## Data Storage

**Databases:**
- Not integrated - No persistent database used
- File-based storage: Project files and workspace state only
- Git repository as source of truth for version control

**File Storage:**
- Local filesystem only
- File watching via notify crate (7.0)
- File traversal via walkdir (2.5)
- Pattern matching via globset (0.4) for .gitignore-style exclusions
- No cloud storage integration

**Caching:**
- In-memory caching only (no persistent cache)
- Uses once_cell (1.20) for lazy initialization and static caching
- Parking_lot mutexes for thread-safe cache access

## Authentication & Identity

**Auth Provider:**
- Custom internal authentication (if agents require external APIs)
- No OAuth or third-party auth detected
- Secret redaction via `crates/security/` for sensitive data protection

**Security Model:**
- All core IDE functionality is local-first and offline
- Agent system (optional) handles external API calls
- Secret types detected and redacted:
  - API Keys
  - Passwords
  - Bearer tokens
  - AWS credentials
  - GitHub tokens
  - Private keys (PEM format)
  - JWT tokens
  - Generic secrets via regex patterns

## Monitoring & Observability

**Error Tracking:**
- None detected - Local error handling only
- Error types defined in individual crates (LspClientError, DapError, etc.)

**Logging:**
- Framework: tracing 0.1 with tracing-subscriber 0.3
- Environment-based configuration via RUST_LOG env var
- Structured logging with levels: trace, debug, info, warn, error
- Log output: stderr by default
- No external log aggregation service

**Diagnostics:**
- LSP diagnostics integration in `crates/lsp_bridge/src/diagnostics.rs`
- Diagnostics collected from language servers
- Displayed in-editor via GPUI UI shell

## Git Integration

**Version Control:**
- git2 0.19 (libgit2 bindings)
- Location: `crates/gitx/`
- Capabilities:
  - Repository status checking
  - Diff generation
  - Commit operations
  - File history

**CI/CD & Deployment:**
- Not detected - No CI/CD integration
- Local-only task runner in `crates/tasks/`
- Supports custom build and test task execution

## Process Management

**Spawned Processes:**
- LSP servers: Managed via `crates/lsp_bridge/src/client.rs`
  - Command: User-configured LSP server executable
  - Communication: stdin/stdout JSON-RPC
  - Lifecycle: Spawned on workspace open, shutdown on workspace close
  - Environment variables: Configurable per server

- DAP servers: Managed via `crates/dap_bridge/src/client.rs` and `session.rs`
  - Command: User-configured debugger executable
  - Communication: stdin/stdout JSON-RPC
  - Lifecycle: Spawned on debug session start, shutdown on end
  - Environment variables: Configurable per session

- Task runner: Managed via `crates/tasks/src/runner.rs`
  - Command: Build/test commands (cargo, npm, python, etc.)
  - Communication: Captures stdout/stderr
  - Environment variables: Workspace-specific

## Environment Configuration

**Required env vars:**
- RUST_LOG - Logging level (optional, defaults to info)
  - Format: `RUST_LOG=debug` or `RUST_LOG=ui_shell=debug,lsp_bridge=trace`

**LSP Server Configuration:**
- Stored in `crates/languages/src/config.rs`
- Per-server configuration:
  - command: Executable path (e.g., "rust-analyzer")
  - args: Command-line arguments
  - env: Environment variables for server process
  - working_dir: Server process working directory
  - root_uri: Workspace root path

**DAP Server Configuration:**
- Stored in `crates/dap_bridge/src/config.rs`
- Per-debugger configuration:
  - adapter_path: Path to debugger executable
  - args: Command-line arguments
  - env: Environment variables for debugger process

**Secrets location:**
- Handled by `crates/security/` redaction system
- Secrets detected but never transmitted without redaction
- No .env file or secrets manager integration

## Webhooks & Callbacks

**Incoming:**
- File system watchers: notify crate monitors file changes in workspace
- LSP notifications: Language servers send diagnostics, hover info, etc.
- DAP events: Debugger sends breakpoint hits, state changes, etc.

**Outgoing:**
- Task execution callbacks: Problems are reported back to UI
- LSP requests: Requests sent to language servers
- DAP commands: Commands sent to debuggers

## External Service Dependencies

**None Required for Core IDE:**
- All language servers and debuggers must be installed locally
- Fully functional offline for all core editor features
- Agent system is opt-in and sandboxed

**Optional (Agent System):**
- External LLM APIs (when agents are enabled)
- User-configurable API endpoints
- Automatic secret redaction before any external transmission
- Context capsules limit what data is sent

## Platform-Specific Integration

**macOS:**
- Apple Silicon optimization via GPUI
- Native keybindings support
- Metal GPU acceleration potential

**Linux:**
- Vulkan GPU support
- Wayland protocol support via GPUI

---

*Integration audit: 2026-01-27*
