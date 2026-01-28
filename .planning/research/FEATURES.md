# Features Research: Modern Code Editor Table Stakes

**Research Date:** 2026-01-27
**Research Type:** Features Dimension
**Target:** Foundation Complete State for Rustide

---

## Executive Summary

Modern code editors (Zed, VS Code, Sublime Text) share a common baseline of "table stakes" features that users expect to work flawlessly. Beyond these, editors differentiate through performance, extensibility, or novel features. This document categorizes features for Rustide's "foundation complete" milestone.

**Key Finding:** Rustide has significant scaffolding already built (LSP, DAP, syntax highlighting, rope buffers). The primary gaps are in **wiring these components together** and implementing **fundamental text editing behaviors** users take for granted.

---

## Table Stakes (Must Have for Foundation)

These features are non-negotiable. Users will not consider an editor usable without them.

### 1. Text Editing Fundamentals

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Text input (typing characters) | Low | Partial | `editor_core::Buffer` - needs UI wiring |
| Backspace/Delete | Low | No | Buffer ops exist, needs key binding |
| Enter (newline insertion) | Low | No | Buffer insert, needs auto-indent |
| Tab/Shift-Tab (indent) | Medium | No | Needs indent detection logic |
| Cursor movement (arrow keys) | Low | Partial | `Cursor` exists, needs key bindings |
| Page Up/Page Down | Low | No | Needs scroll + cursor sync |
| Home/End (line start/end) | Low | No | Cursor ops + key bindings |
| Cmd+Home/End (document start/end) | Low | No | Cursor ops + key bindings |
| Select text (Shift+arrows) | Medium | Partial | `Selection` exists, needs wiring |
| Select all (Cmd+A) | Low | No | Simple selection op |
| Word-wise movement (Option+arrows) | Medium | Partial | `word_at()` exists in buffer |
| Click to position cursor | Medium | No | Needs hit-testing in render |
| Click+drag selection | Medium | No | Needs mouse event handling |
| Double-click select word | Medium | No | `word_at()` + mouse events |
| Triple-click select line | Low | No | Line selection + mouse events |

**Dependencies:** All text editing features depend on the editor pane properly wiring keyboard/mouse events to `editor_core` operations.

### 2. Clipboard Operations

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Copy (Cmd+C) | Low | No | System clipboard access |
| Cut (Cmd+X) | Low | No | Copy + delete selection |
| Paste (Cmd+V) | Low | No | System clipboard + insert |
| Paste without formatting | Low | No | Same as paste (plain text editor) |

**Dependencies:** Requires platform clipboard API (available through GPUI).

### 3. Undo/Redo (User-Facing)

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Undo (Cmd+Z) | Low | Yes* | `History` exists, needs key binding |
| Redo (Cmd+Shift+Z) | Low | Yes* | `History` exists, needs key binding |
| Proper transaction grouping | Medium | Yes | Grouping exists in `History` |

*Scaffolding exists but not wired to UI.

### 4. File Operations

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Open file (Cmd+O) | Low | Partial | Dialog + `EditorPane::open_file` |
| Save file (Cmd+S) | Medium | No | Needs `Document::save()` wiring |
| Save As (Cmd+Shift+S) | Medium | No | Save dialog + save logic |
| Close tab (Cmd+W) | Low | Partial | `close_tab()` exists |
| Modified indicator (dot) | Low | Partial | `is_dirty()` exists, rendered |
| Prompt save on close | Medium | No | Needs modal dialog + logic |
| File reload on external change | Medium | No | Needs file watcher |

**Dependencies:** File operations depend on proper dirty state tracking and user confirmation dialogs.

### 5. Search & Replace (In-File)

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Find (Cmd+F) | Medium | No | Search UI + buffer search |
| Find next/previous | Low | No | Search state + navigation |
| Replace (Cmd+H) | Medium | No | Find + replace UI |
| Replace all | Medium | No | Batch replace operation |
| Case sensitivity toggle | Low | No | Search options |
| Regex search | Medium | No | Regex engine (available in Rust) |
| Match whole word | Low | No | Word boundary detection |

**Complexity Note:** Search UI is a new component; buffer searching is straightforward with ropey.

### 6. LSP Integration (Core Features)

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Start LSP on file open | Medium | Partial | `ServerManager` exists, needs trigger |
| Diagnostics display (squiggles) | High | Partial | `DiagnosticStore` exists, needs rendering |
| Error/warning gutter icons | Medium | No | Gutter rendering + diagnostics |
| Go to definition (F12/Cmd+click) | Medium | Yes* | Client method exists, needs UI trigger |
| Hover information | Medium | Yes* | Client method exists, needs popup |
| Autocomplete popup | High | Yes* | Completions exist, needs popup UI |
| Signature help | Medium | Yes* | Client capability, needs popup |
| Document sync (didOpen/didChange) | High | No | Must sync edits to LSP |

*LSP client has methods, but UI integration is missing.

**Dependencies:** All LSP features depend on proper document synchronization (didOpen, didChange, didClose notifications).

### 7. Syntax Highlighting (Visual)

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Syntax coloring | Medium | Yes* | `Highlighter` exists |
| Apply theme colors in render | Medium | No | Theme + highlighter -> GPUI |
| Incremental rehighlight on edit | Medium | Yes | `Highlighter::edit()` exists |

*Highlighter is built; needs to be applied in `render_editor_content()`.

### 8. UI Shell Basics

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Tab bar with tabs | Low | Yes | Rendered in `EditorPane` |
| Tab switching (click) | Medium | No | Click handler needed |
| Tab close button | Low | No | UI + handler |
| File tree (project explorer) | Low | Yes | `FileTree` component |
| File tree expand/collapse | Low | Partial | Needs interaction |
| Open file from tree (click/Enter) | Medium | No | Event -> `open_file()` |
| Status bar | Low | Yes | `render_status_bar()` |
| Line/column in status bar | Low | Yes | Rendered |
| Diagnostics count in status bar | Low | No | Query `DiagnosticStore` |

### 9. Scrolling & Viewport

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Vertical scroll (mouse wheel) | Low | Partial | GPUI handles some |
| Scroll bar | Low | No | May need custom impl |
| Horizontal scroll for long lines | Medium | No | Needs viewport logic |
| Scroll cursor into view | Medium | No | Auto-scroll on cursor move |
| Smooth scrolling | Low | Maybe | GPUI may support |

### 10. Line Numbers

| Feature | Complexity | Existing? | Dependencies |
|---------|------------|-----------|--------------|
| Line number gutter | Low | Yes | Rendered in content |
| Current line highlight | Low | No | Track active line |
| Relative line numbers (optional) | Low | No | Config + math |

---

## Nice-to-Have (Polish Phase)

These features enhance usability but can wait until foundation is solid.

### 1. Advanced Navigation

| Feature | Complexity | Notes |
|---------|------------|-------|
| Go to line (Cmd+G) | Low | Dialog + cursor move |
| Go to symbol (Cmd+Shift+O) | Medium | LSP document symbols |
| Go to file (Cmd+P) | Medium | Fuzzy file finder |
| Breadcrumbs | Medium | Symbol hierarchy in header |
| Outline view | Medium | Document symbols sidebar |

### 2. Multi-Cursor & Bulk Editing

| Feature | Complexity | Notes |
|---------|------------|-------|
| Add cursor (Cmd+click) | Medium | Multi-cursor support exists in Cursor |
| Add cursor above/below | Medium | Cmd+Option+Up/Down |
| Select all occurrences | Medium | Find + multi-select |
| Column selection | Medium | Option+drag |

### 3. Code Intelligence (Advanced LSP)

| Feature | Complexity | Notes |
|---------|------------|-------|
| Find references | Medium | LSP request + results panel |
| Rename symbol | Medium | LSP + workspace edit |
| Code actions (quick fix) | Medium | LSP code actions |
| Format document | Low | LSP formatting |
| Format selection | Low | LSP range formatting |
| Organize imports | Low | LSP code action |

### 4. Git Integration (UI)

| Feature | Complexity | Notes |
|---------|------------|-------|
| Gutter diff markers | Medium | gitx diff + gutter |
| Changed file indicators in tree | Low | gitx status |
| Inline blame | Medium | gitx blame (would need impl) |

### 5. Workspace Features

| Feature | Complexity | Notes |
|---------|------------|-------|
| Split editor (vertical/horizontal) | Medium | Pane splitting logic |
| Multiple windows | High | GPUI window management |
| Tab groups | Medium | Grouping within workspace |
| Recent files | Low | State persistence |
| Session restore | Medium | Workspace state save/load |

### 6. Search (Project-Wide)

| Feature | Complexity | Notes |
|---------|------------|-------|
| Find in files (Cmd+Shift+F) | High | Ripgrep integration |
| Replace in files | High | Batch replace + preview |
| Search results panel | Medium | Results list component |

### 7. Terminal Integration

| Feature | Complexity | Notes |
|---------|------------|-------|
| Integrated terminal | High | PTY spawning + VT100 |
| Multiple terminals | Medium | Terminal tabs |
| Terminal split | Medium | Panel management |

### 8. Minimap

| Feature | Complexity | Notes |
|---------|------------|-------|
| Code minimap | Medium | Scaled preview rendering |
| Highlighted sections in minimap | Medium | Sync with search/errors |

### 9. Code Folding

| Feature | Complexity | Notes |
|---------|------------|-------|
| Fold/unfold regions | Medium | Tree-sitter fold queries |
| Fold all/unfold all | Low | Batch operation |
| Fold level N | Low | Scope-aware folding |

### 10. Snippets

| Feature | Complexity | Notes |
|---------|------------|-------|
| LSP snippet expansion | Medium | Snippet parser + placeholders |
| Tab stop navigation | Medium | Placeholder state machine |
| User-defined snippets | Low | Config format |

---

## Anti-Features (Do NOT Build Yet)

These should be explicitly deferred to avoid scope creep.

### 1. Extension/Plugin System
- **Why Not Now:** Foundation must be stable first; extension APIs are breaking-change magnets
- **When:** After 1.0 or major stability milestone

### 2. Settings UI
- **Why Not Now:** Config files work; UI is polish
- **When:** After core editing works

### 3. Theme Editor/Marketplace
- **Why Not Now:** Ship with 1-2 good themes; customization is polish
- **When:** After extensibility story

### 4. Collaborative Editing
- **Why Not Now:** Requires CRDT/OT, networking, auth - completely separate project
- **When:** If ever (this is not a differentiator for Rustide)

### 5. AI Features (Code Completion, Chat)
- **Why Not Now:** Agent system scaffolding exists; wiring can wait
- **When:** After editor is dogfood-ready

### 6. Debugger UI
- **Why Not Now:** DAP infrastructure exists; UI is complex and can follow
- **When:** After LSP features are solid

### 7. Notebook Support (Jupyter)
- **Why Not Now:** Different editing paradigm
- **When:** Likely never for v1

### 8. Remote Development (SSH, Containers)
- **Why Not Now:** Significant infrastructure; local-first is fine
- **When:** After core is mature

### 9. Language-Specific Features
- **Why Not Now:** LSP should provide generality; special-casing fragments focus
- **When:** Only if LSP is insufficient

### 10. Vim/Emacs Keybinding Modes
- **Why Not Now:** Complex state machine; modal editing is niche
- **When:** After basic editing is flawless

---

## Feature Dependencies Graph

```
Text Input
    |
    v
Clipboard <--- Selection <--- Mouse Events
    |              |
    v              v
Undo/Redo     Multi-Cursor
    |
    v
Save File
    |
    v
LSP Document Sync
    |
    +--> Diagnostics Display
    |
    +--> Go to Definition
    |
    +--> Hover Info
    |
    +--> Autocomplete

Search In-File
    |
    v
Find in Files (project-wide)
```

---

## Priority Implementation Order

Based on dependencies and user impact:

### Phase 1: Core Editing Loop
1. Text input (typing)
2. Cursor movement (all directions)
3. Backspace/Delete
4. Selection (keyboard + mouse)
5. Clipboard (copy/cut/paste)
6. Undo/Redo wiring

### Phase 2: File Management
1. Save file
2. Save confirmation on close
3. Tab switching (click)
4. Tab close button
5. File tree click-to-open

### Phase 3: Visual Polish
1. Syntax highlighting in render
2. Current line highlight
3. Scroll cursor into view

### Phase 4: LSP Foundation
1. Document sync (didOpen/didChange/didClose)
2. Diagnostics display (squiggles)
3. Status bar diagnostics count
4. Go to definition
5. Hover information

### Phase 5: LSP Completion
1. Autocomplete popup
2. Signature help

### Phase 6: Search
1. Find in file
2. Find next/previous
3. Replace

---

## Zed Parity Checklist

Features Zed has that are table stakes for parity:

- [x] GPU-accelerated rendering (GPUI)
- [x] Tree-sitter syntax highlighting
- [x] LSP protocol support
- [ ] Fast file opening
- [ ] Instant startup (<100ms)
- [ ] Smooth 60fps editing
- [ ] Multi-cursor editing
- [ ] Command palette
- [ ] Fuzzy file finder
- [ ] Project-wide search
- [ ] Git gutter indicators
- [ ] Inline diagnostics
- [ ] Hover documentation
- [ ] Autocomplete
- [ ] Go to definition
- [ ] Split editors

---

## Complexity Legend

- **Low:** 1-2 days for experienced developer
- **Medium:** 3-5 days, may involve new abstractions
- **High:** 1-2 weeks, significant new components or integration work

---

*Research compiled from analysis of VS Code, Zed, Sublime Text, and Rustide codebase inspection.*
