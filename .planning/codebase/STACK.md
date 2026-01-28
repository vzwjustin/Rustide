# Technology Stack

**Analysis Date:** 2026-01-27

## Languages

**Primary:**
- Rust 1.75+ (stable channel) - All application code, pure Rust IDE built for macOS and Linux

## Runtime

**Environment:**
- Rust stable toolchain with rustfmt, clippy, and rust-analyzer components
- Cargo workspace with 9 crates organized under `crates/` directory

**Package Manager:**
- Cargo (Rust package manager)
- Lockfile: Not present in repository (uses workspace dependency resolution)

## Frameworks

**Core:**
- GPUI 0.2 - GPU-accelerated UI framework for rendering (Zed-like responsiveness)
  - Used in: `crates/ui_shell/`
  - Provides: Workspace layout, command palette, file tree, panes

**Syntax & Parsing:**
- Tree-sitter 0.22 - Incremental parser for syntax trees
- Tree-sitter-highlight 0.22 - Syntax highlighting engine
- Tree-sitter language bindings (0.21):
  - tree-sitter-rust
  - tree-sitter-javascript
  - tree-sitter-python
  - tree-sitter-c
  - tree-sitter-go
  - tree-sitter-json
  - tree-sitter-md (markdown)

**Language Server Protocol (LSP):**
- lsp-types 0.97 - LSP type definitions and protocol
- Custom LSP client implementation in `crates/lsp_bridge/`

**Debug Adapter Protocol (DAP):**
- dap 0.4.1-alpha1 - Debug Adapter Protocol client library
- Custom DAP client implementation in `crates/dap_bridge/`

**Testing:**
- tokio-test 0.4 - Testing utilities for async code (dev dependency)
- tempfile 3.14 - Temporary file handling for tests
- Standard Rust test framework (built-in)

**Build/Dev:**
- Cargo (build system)
- Profile optimizations:
  - Release: opt-level 3, thin LTO, single codegen unit
  - Dev: opt-level 0 with per-dependency opt-level 3

## Key Dependencies

**Critical:**
- tokio 1.43 (with "full" features) - Async runtime for concurrent operations
  - Location: Used in `crates/lsp_bridge/`, `crates/dap_bridge/`, `crates/agents/`, `crates/tasks/`
  - Why it matters: Handles LSP/DAP communication, task execution, and agent orchestration

- ropey 1.6 - Rope data structure for efficient text buffer operations
  - Location: `crates/editor_core/`
  - Why it matters: Core text editing performance for multi-cursor support

- lsp-types 0.97 - LSP protocol definitions
  - Location: `crates/lsp_bridge/` and `crates/agents/`
  - Why it matters: Communication with external language servers

- git2 0.19 - libgit2 bindings for Git operations
  - Location: `crates/gitx/`
  - Why it matters: Git integration (status, diff, commit)

**Async/Concurrency:**
- async-trait 0.1 - Async trait implementations
- futures 0.3 - Async utilities and combinators
- crossbeam-channel 0.5 - Multi-producer multi-consumer channels
- parking_lot 0.12 - Faster synchronization primitives than std
- once_cell 1.20 - Lazy statics and initialization

**Text Handling:**
- unicode-segmentation 1.12 - Unicode grapheme cluster handling
- regex 1.11 - Regular expression engine
- fuzzy-matcher 0.3 - Fuzzy string matching for search/completion

**Serialization:**
- serde 1.0 (with derive) - Serialization framework
- serde_json 1.0 - JSON serialization
- toml 0.8 - TOML parsing for configuration files

**File System:**
- notify 7.0 - File system event notifications
- walkdir 2.5 - Recursive directory iteration
- globset 0.4 - .gitignore-style pattern matching

**Logging:**
- tracing 0.1 - Structured logging and diagnostics
- tracing-subscriber 0.3 (with env-filter) - Logging subscriber with environment filtering

**Error Handling:**
- anyhow 1.0 - Error context and result handling
- thiserror 2.0 - Derive macros for error types

**Utilities:**
- uuid 1.11 (with v4, serde) - UUID generation
- chrono 0.4 (with serde) - Date and time handling
- serde_json 1.0 - JSON data manipulation

## Configuration

**Environment:**
- RUST_LOG - Controls tracing/logging verbosity level
  - Example: `RUST_LOG=debug cargo run -p ui_shell`

**Build Configuration:**
- Workspace-level Cargo.toml defines shared dependencies and versions
- Individual crate Cargo.toml files inherit from workspace.dependencies
- Three profiles configured:
  - release: Production optimizations
  - dev: Development with per-dependency optimization
  - (implicit test profile)

**No Config Files:**
- No .env files detected
- No environment secrets management (handled by security crate internally)
- No build.rs custom build scripts detected

## Platform Requirements

**Development:**
- Rust 1.75+ (stable)
- macOS 12+ (primary platform, Apple Silicon optimized)
- Linux with Vulkan support (alternative platform)
- Linux dependencies: `libxkbcommon-dev libwayland-dev pkg-config`

**Production:**
- Deployment target: macOS 12+ (Intel and Apple Silicon) and Linux
- No Docker or containerization detected
- Local-first architecture - no cloud service dependencies required

## Runtime Behavior

**Process Management:**
- LSP servers: Spawned as child processes, managed via `tokio::process::Command`
- DAP servers: Spawned as child processes for debugging
- Workspace task runners: Execute local build/test commands

**Communication Patterns:**
- JSON-RPC 2.0 protocol with LSP servers over stdin/stdout
- JSON-RPC 2.0 protocol with DAP servers over stdin/stdout
- Internal async message passing via tokio channels

**Memory & Performance:**
- No garbage collection (Rust memory safety)
- GPU-accelerated rendering via GPUI
- Profile optimizations enabled for release builds
- LTO (Link Time Optimization) enabled for smaller binaries

---

*Stack analysis: 2026-01-27*
