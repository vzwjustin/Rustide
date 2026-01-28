# RustIDE

A Zed-like, Mac-first, pure-Rust IDE with always-context-aware agents.

## Overview

RustIDE is a high-performance, GPU-accelerated code editor built entirely in Rust. It combines the responsiveness of Zed with a unique agent system that provides deterministic, auditable context awareness for AI-assisted development.

### Key Features

- **Pure Rust + GPUI** - GPU-accelerated UI using Zed's GPUI framework for "render like a videogame" responsiveness
- **Multi-Language Support** - Tree-sitter for syntax, LSP for semantics, DAP for debugging
- **Context-Aware Agents** - Deterministic "Context Capsules" with ranked, summarized, evidence-linked, token-budgeted context
- **Local-First** - Fully functional without cloud services; agents are opt-in and sandboxed
- **Mac-First Polish** - Apple Silicon optimized, native keybindings, smooth scrolling

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         ui_shell                            │
│  (GPUI app, workspace, panes, command palette, file tree)   │
├─────────────┬─────────────┬─────────────┬───────────────────┤
│ editor_core │  languages  │  lsp_bridge │    dap_bridge     │
│ (buffer,    │ (tree-sitter│ (LSP client │  (DAP client,     │
│  cursors,   │  grammars,  │  manager,   │   debug session,  │
│  undo/redo) │  highlight) │  diagnostics│   breakpoints)    │
├─────────────┴─────────────┼─────────────┴───────────────────┤
│         tasks             │            gitx                  │
│ (task runner, problems)   │ (git status, diff, commit)       │
├───────────────────────────┴─────────────────────────────────┤
│                          agents                              │
│    (Context Capsules, tools, orchestration, audit logs)      │
├─────────────────────────────────────────────────────────────┤
│                         security                             │
│          (secret redaction, path policies)                   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust stable (1.75+)
- macOS 12+ or Linux with Vulkan support
- For Linux: `libxkbcommon-dev libwayland-dev pkg-config`

### Build

```bash
# Clone the repository
git clone https://github.com/vzwjustin/Rustide.git
cd Rustide

# Build the project
cargo build --release

# Run the IDE
cargo run --release -p ui_shell
```

### Development

```bash
# Run in development mode with logging
RUST_LOG=debug cargo run -p ui_shell

# Run tests
cargo test --workspace

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `ui_shell` | GPUI application shell, workspace layout, command palette |
| `editor_core` | Rope-backed text buffer, multi-cursor editing, undo/redo |
| `languages` | Tree-sitter grammar loading, syntax highlighting, language registry |
| `lsp_bridge` | LSP client, server lifecycle management, diagnostics |
| `dap_bridge` | DAP client, debug sessions, breakpoint management |
| `tasks` | Task runner (build/test), problem matchers |
| `gitx` | Git integration via libgit2 |
| `agents` | Context Capsules, agent tools, orchestration, audit |
| `security` | Secret redaction, path access policies |

## Language Support

RustIDE supports these languages out of the box:

| Language | Extensions | LSP Server | Debugger |
|----------|------------|------------|----------|
| Rust | `.rs` | rust-analyzer | codelldb |
| TypeScript/JavaScript | `.ts`, `.js`, `.tsx`, `.jsx` | typescript-language-server | node |
| Python | `.py` | pyright | debugpy |
| Go | `.go` | gopls | delve |
| C/C++ | `.c`, `.cpp`, `.h` | clangd | gdb/lldb |
| JSON | `.json` | vscode-json-language-server | - |
| TOML | `.toml` | taplo | - |
| Markdown | `.md` | marksman | - |

## Agent System

RustIDE's differentiating feature is its **Context Capsule** system for agents:

```
┌─────────────────────────────────────────┐
│           Context Capsule               │
├─────────────────────────────────────────┤
│ • Goal + constraints                    │
│ • Workspace map                         │
│ • Ranked relevant files with summaries  │
│ • Active editor state                   │
│ • Git diff summary                      │
│ • Diagnostics snapshot                  │
│ • Evidence snippets (path + lines)      │
│ • Token budget plan                     │
│ • Redaction report                      │
└─────────────────────────────────────────┘
```

Every agent invocation receives a deterministically-built capsule that:
- Ranks context by relevance (recency, proximity, diagnostics, dependencies)
- Summarizes code hierarchically based on importance
- Respects token budgets with transparent inclusion/exclusion
- Redacts secrets before any external transmission
- Provides evidence links for all included code

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and crate structure
- [Languages](docs/LANGUAGES.md) - Multi-language support guide
- [Debugging](docs/DEBUGGING.md) - DAP integration and setup
- [Agents](docs/AGENTS.md) - Context Capsules and agent system
- [Security](docs/SECURITY.md) - Privacy, redaction, and sandboxing

## Milestones

- [x] **Milestone 0** - App Shell + GPUI (workspace, file tree, basic editor)
- [x] **Milestone 1** - Editor Core + Tree-sitter (rope buffer, syntax highlighting)
- [x] **Milestone 2** - LSP Bridge (diagnostics, completion, go-to-def)
- [ ] **Milestone 3** - Tasks + Git + Terminal
- [ ] **Milestone 4** - DAP Debugger
- [ ] **Milestone 5** - Agents (Context Capsules + Tooling + Audit UI)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure your code:
- Compiles without warnings (`cargo build`)
- Passes all tests (`cargo test`)
- Is formatted (`cargo fmt`)
- Passes clippy (`cargo clippy`)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Zed](https://zed.dev) - Inspiration for the UI/UX model and GPUI framework
- [Tree-sitter](https://tree-sitter.github.io) - Incremental parsing
- [rust-analyzer](https://rust-analyzer.github.io) - Rust language server
