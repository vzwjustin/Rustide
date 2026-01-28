# Rustide

## What This Is

A Zed-inspired Rust IDE built with GPUI for GPU-accelerated rendering, LSP/DAP protocol support, and tree-sitter syntax highlighting. The goal is to complete the foundational architecture so all scaffolding is in place for future feature polish — performance, extensibility, Zed parity, and stability are all must-haves.

## Core Value

The IDE must be architecturally sound and extensible — clean crate boundaries, async-first design, and GPU-accelerated rendering that can handle real codebases without lag.

## Requirements

### Validated

<!-- Existing capabilities from codebase analysis -->

- ✓ Rope-backed text buffers with O(log n) insert/delete — existing (`editor_core::Buffer`)
- ✓ Multi-cursor and selection support — existing (`editor_core::Cursor`, `Selection`)
- ✓ Undo/redo transaction history — existing (`editor_core::History`)
- ✓ Tree-sitter syntax highlighting for 7 languages — existing (`languages::Highlighter`)
- ✓ LSP client infrastructure (spawn, JSON-RPC, diagnostics) — existing (`lsp_bridge`)
- ✓ DAP client infrastructure (spawn, breakpoints, sessions) — existing (`dap_bridge`)
- ✓ GPUI-based UI shell structure — existing (`ui_shell`)
- ✓ Command palette with fuzzy search — existing (`ui_shell::command_palette`)
- ✓ File tree component — existing (`ui_shell::file_tree`)
- ✓ Workspace with pane management — existing (`ui_shell::workspace`)
- ✓ Agent system with context capsules — existing (`agents`)
- ✓ Tool registry for agent IDE operations — existing (`agents::tools`)
- ✓ Git integration via libgit2 — existing (`gitx`)
- ✓ Secret redaction and path policies — existing (`security`)
- ✓ Task runner for build/test commands — existing (`tasks`)

### Active

<!-- Foundation completion — what needs to work end-to-end -->

- [ ] Application compiles and launches on macOS
- [ ] Editor pane renders text with syntax highlighting
- [ ] User can open files from file tree
- [ ] User can edit text with visible cursor
- [ ] User can undo/redo edits
- [ ] User can save files
- [ ] LSP server spawns and connects for Rust files
- [ ] Diagnostics display in editor (error squiggles)
- [ ] Go-to-definition works for Rust code
- [ ] Hover shows type information
- [ ] Completions appear while typing
- [ ] Command palette opens and dispatches commands
- [ ] Multiple panes/splits work
- [ ] Theme applies consistently across UI
- [ ] File tree shows workspace structure
- [ ] Status bar shows file info and diagnostics count

### Out of Scope

- Linux/Windows support — macOS first, cross-platform later
- Plugin/extension system — focus on core IDE first
- Collaborative editing — single-user foundation first
- AI agent features — existing scaffolding sufficient for foundation
- Git UI beyond status — Git operations are terminal-level for now
- Debugger UI — DAP infrastructure exists but UI can wait

## Context

**Brownfield project:** 9-crate workspace with significant scaffolding already built. The architecture follows Zed patterns — GPUI for rendering, tree-sitter for parsing, LSP/DAP protocols for language intelligence and debugging.

**Current state unknown:** The codebase has not been run yet. First phase must validate what actually works vs what's just scaffolding.

**Known issues from codebase analysis:**
- TOML and Markdown grammars disabled due to `cc` crate version conflicts
- LSP server requests (e.g., `window/showMessage`) are logged but not handled
- Problem matcher has issues with multi-line compiler output
- No request timeouts for pending LSP requests
- No graceful server restart when LSP crashes

## Constraints

- **UI Framework**: Must use GPUI — committed to Zed's rendering approach
- **Platform**: macOS first — Apple Silicon and Intel, macOS 12+
- **Language**: Pure Rust — no native dependencies beyond what's already in Cargo.toml
- **Performance**: GPU-accelerated rendering, async-first, no blocking on main thread

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GPUI for UI | Zed-like performance and rendering model | — Pending validation |
| Layered crate architecture | Clear separation of concerns, testable in isolation | — Pending validation |
| Tree-sitter for parsing | Incremental parsing, proven in Zed | — Pending validation |
| LSP/DAP protocols | Standard protocols, broad language/debugger support | — Pending validation |

---
*Last updated: 2026-01-27 after initialization*
