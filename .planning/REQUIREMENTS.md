# Requirements

**Project:** Rustide IDE
**Version:** v1.0 Foundation
**Created:** 2026-01-27

## v1 Requirements (Foundation)

### Text Editing

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| EDIT-01 | Text input with visible cursor and basic navigation (arrows, home/end, page up/down) | Must Have | Research: FEATURES.md |
| EDIT-02 | Selection via keyboard (Shift+arrows) and mouse (click-drag, double-click word, triple-click line) | Must Have | Research: FEATURES.md |
| EDIT-03 | Clipboard operations: copy (Cmd+C), cut (Cmd+X), paste (Cmd+V) | Must Have | Research: FEATURES.md |
| EDIT-04 | Undo (Cmd+Z) and redo (Cmd+Shift+Z) with transaction grouping | Must Have | Research: FEATURES.md |

### File Operations

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| FILE-01 | Open file from file tree with async loading (no UI freeze on large files) | Must Have | Research: FEATURES.md, PITFALLS.md P5 |
| FILE-02 | Save file (Cmd+S) with dirty state indicator in tab | Must Have | Research: FEATURES.md |
| FILE-03 | Close tab with unsaved changes confirmation dialog | Must Have | Research: FEATURES.md |
| FILE-04 | Tab switching with keyboard (Cmd+1-9) and mouse click | Must Have | Research: FEATURES.md |

### LSP Integration

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| LSP-01 | Document synchronization: didOpen on file open, didChange on edit, didClose on close | Must Have | Research: FEATURES.md, ARCHITECTURE.md |
| LSP-02 | Diagnostics display: inline squiggles, gutter icons, status bar count | Must Have | Research: FEATURES.md |
| LSP-03 | Go-to-definition (F12 / Cmd+click) with navigation stack | Must Have | Research: FEATURES.md |
| LSP-04 | Hover information popup on mouse hover with type/doc info | Must Have | Research: FEATURES.md |
| LSP-05 | Autocomplete popup with LSP suggestions while typing | Must Have | Research: FEATURES.md |

### Visual Features

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| VIS-01 | Syntax highlighting applied to rendered text using tree-sitter + theme colors | Must Have | Research: FEATURES.md |
| VIS-02 | Virtual scrolling: only render visible lines + buffer (prevent freeze on 100K+ line files) | Must Have | Research: PITFALLS.md P12 |
| VIS-03 | Current line highlight with subtle background color | Should Have | Research: FEATURES.md |
| VIS-04 | Status bar showing file path, cursor position, language, diagnostics count | Must Have | Research: FEATURES.md |

### UI Components

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| UI-01 | File tree showing workspace directory structure with file/folder icons | Must Have | PROJECT.md Active |
| UI-02 | Command palette (Cmd+Shift+P) with fuzzy search and action dispatch | Must Have | PROJECT.md Active |
| UI-03 | Multiple panes/splits for side-by-side editing | Should Have | PROJECT.md Active |
| UI-04 | Theme consistency across all UI components | Should Have | PROJECT.md Active |

### Infrastructure

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| INFRA-01 | Application compiles and launches on macOS (Apple Silicon + Intel) | Must Have | PROJECT.md Active |
| INFRA-02 | LSP server spawns automatically for recognized file types (rust-analyzer for .rs) | Must Have | PROJECT.md Active |
| INFRA-03 | Request timeouts for LSP operations (30s heavy, 5s quick) | Must Have | Research: PITFALLS.md P2 |
| INFRA-04 | Graceful LSP server restart on crash | Should Have | Research: PITFALLS.md P3 |

## v2 Requirements (Polish)

### Advanced Editing

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| EDIT-10 | Multi-cursor editing (Cmd+click, Cmd+D select next occurrence) | Nice to Have | Research: FEATURES.md |
| EDIT-11 | Find in file (Cmd+F) with regex support | Nice to Have | Research: FEATURES.md |
| EDIT-12 | Replace and replace all | Nice to Have | Research: FEATURES.md |
| EDIT-13 | Go-to-line (Cmd+G) | Nice to Have | Research: FEATURES.md |

### Advanced Navigation

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| NAV-01 | Fuzzy file finder (Cmd+P) | Nice to Have | Research: FEATURES.md |
| NAV-02 | Go-to-symbol (Cmd+Shift+O) | Nice to Have | Research: FEATURES.md |
| NAV-03 | Project-wide search (Cmd+Shift+F) | Nice to Have | Research: FEATURES.md |
| NAV-04 | Breadcrumb navigation | Nice to Have | Research: FEATURES.md |

### Advanced LSP

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| LSP-10 | Signature help popup | Nice to Have | Research: FEATURES.md |
| LSP-11 | Code actions (quick fixes) | Nice to Have | Research: FEATURES.md |
| LSP-12 | Rename symbol | Nice to Have | Research: FEATURES.md |
| LSP-13 | Find all references | Nice to Have | Research: FEATURES.md |

### Git Integration

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| GIT-01 | Gutter diff markers (added/modified/deleted lines) | Nice to Have | Research: FEATURES.md |
| GIT-02 | Changed file indicators in file tree | Nice to Have | Research: FEATURES.md |

### AI Features

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| AI-01 | API provider configuration for AI assistants | Nice to Have | User request |
| AI-02 | Agent context capsule integration | Nice to Have | Existing: agents crate |

## Out of Scope

| Item | Rationale |
|------|-----------|
| Linux/Windows support | macOS first; cross-platform after foundation stable |
| Plugin/extension system | Core IDE must be stable before extensibility |
| Collaborative editing | Single-user foundation first |
| Debugger UI | DAP infrastructure exists but UI complexity deferred |
| Remote development | Significant scope addition |
| Vim/Emacs modes | Can be added as v2+ feature |

## Traceability

*Filled by roadmap: maps requirements to phases*

| Requirement | Phase | Plan |
|-------------|-------|------|
| EDIT-01 | — | — |
| EDIT-02 | — | — |
| EDIT-03 | — | — |
| EDIT-04 | — | — |
| FILE-01 | — | — |
| FILE-02 | — | — |
| FILE-03 | — | — |
| FILE-04 | — | — |
| LSP-01 | — | — |
| LSP-02 | — | — |
| LSP-03 | — | — |
| LSP-04 | — | — |
| LSP-05 | — | — |
| VIS-01 | — | — |
| VIS-02 | — | — |
| VIS-03 | — | — |
| VIS-04 | — | — |
| UI-01 | — | — |
| UI-02 | — | — |
| UI-03 | — | — |
| UI-04 | — | — |
| INFRA-01 | — | — |
| INFRA-02 | — | — |
| INFRA-03 | — | — |
| INFRA-04 | — | — |

---
*Requirements defined: 2026-01-27*
