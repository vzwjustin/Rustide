# Roadmap: Rustide IDE Foundation

## Overview

Rustide is a brownfield IDE with strong scaffolding (9-crate workspace, GPUI, tree-sitter, LSP/DAP clients) but missing the integration layer that wires components into a functional editor. This roadmap completes the foundation through 8 vertical phases, each delivering an end-to-end user workflow. The approach prioritizes critical pitfall prevention (virtual scrolling, async I/O, LSP sync) in early phases, then builds editing, file management, syntax highlighting, and full LSP integration as successive layers.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation Validation** - Wire EditorPane to real Documents with async loading and virtual scrolling
- [ ] **Phase 2: Core Text Editing** - Text input, cursor navigation, selection, and current line highlight
- [ ] **Phase 3: Edit Operations** - Clipboard integration and undo/redo UI wiring
- [ ] **Phase 4: File Management** - Save, close, tab switching with dirty state handling
- [ ] **Phase 5: Syntax Highlighting** - Tree-sitter coloring in render layer with theme integration
- [ ] **Phase 6: LSP Foundation** - Server spawning, document sync, diagnostics display, and error recovery
- [ ] **Phase 7: LSP Interaction** - Go-to-definition, hover popups, and autocomplete
- [ ] **Phase 8: UI Completion** - File tree, command palette, pane splitting, and theme consistency

## Phase Details

### Phase 1: Foundation Validation
**Goal**: Application launches with EditorPane wired to real Document objects, async file loading prevents UI freeze, and virtual scrolling handles large files
**Depends on**: Nothing (first phase)
**Requirements**: INFRA-01, VIS-02, FILE-01 (partial: async loading only)
**Success Criteria** (what must be TRUE):
  1. Application compiles and launches on macOS (Apple Silicon and Intel)
  2. Opening a 100K+ line file does not freeze the UI (async loading works)
  3. Scrolling through a 100K+ line file renders smoothly (virtual scrolling renders only visible lines)
  4. EditorPane displays actual file content from Document objects (not placeholder strings)
**Plans**: 5 plans in 4 waves

Plans:
- [ ] 01-01-PLAN.md - Build validation and macOS launch (Wave 1)
- [ ] 01-02-PLAN.md - Wire EditorPane to editor_core Document (Wave 2)
- [ ] 01-03-PLAN.md - Implement async file loading with cx.spawn (Wave 3)
- [ ] 01-04-PLAN.md - Implement virtual scrolling with uniform_list (Wave 3)
- [ ] 01-05-PLAN.md - Large file integration testing (Wave 4)

**Addresses Pitfalls:** P5 (blocking I/O), P12 (no virtual scrolling)

---

### Phase 2: Core Text Editing
**Goal**: User can type, navigate with keyboard/mouse, and select text with visual feedback
**Depends on**: Phase 1
**Requirements**: EDIT-01, EDIT-02, VIS-03
**Success Criteria** (what must be TRUE):
  1. User can type text at cursor position with visible cursor blinking
  2. User can navigate with arrow keys, Home/End, Page Up/Down
  3. User can select text via Shift+arrows (keyboard) and click-drag, double-click word, triple-click line (mouse)
  4. Current line has subtle background highlight distinguishing it from other lines
**Plans**: TBD

Plans:
- [ ] 02-01: Text input and cursor rendering
- [ ] 02-02: Keyboard navigation (arrows, home/end, page up/down)
- [ ] 02-03: Mouse input and click positioning
- [ ] 02-04: Selection rendering and keyboard selection
- [ ] 02-05: Mouse selection (drag, double-click, triple-click)
- [ ] 02-06: Current line highlight

---

### Phase 3: Edit Operations
**Goal**: Clipboard operations and undo/redo work with proper transaction grouping
**Depends on**: Phase 2
**Requirements**: EDIT-03, EDIT-04
**Success Criteria** (what must be TRUE):
  1. User can copy selected text with Cmd+C (text available in system clipboard)
  2. User can cut selected text with Cmd+X (text removed and in clipboard)
  3. User can paste from system clipboard with Cmd+V at cursor position
  4. User can undo last edit with Cmd+Z (multiple edits grouped as single undo)
  5. User can redo undone edit with Cmd+Shift+Z
**Plans**: TBD

Plans:
- [ ] 03-01: Clipboard integration with arboard
- [ ] 03-02: Undo/redo UI wiring to History
- [ ] 03-03: Transaction grouping for multi-edit operations

**Addresses Pitfalls:** P7 (undo state corruption)

---

### Phase 4: File Management
**Goal**: User can save files, manage tabs, and handle unsaved changes safely
**Depends on**: Phase 3
**Requirements**: FILE-01 (complete: file tree integration), FILE-02, FILE-03, FILE-04
**Success Criteria** (what must be TRUE):
  1. User can open file from file tree (click opens in editor pane)
  2. User can save file with Cmd+S (dirty indicator clears after save)
  3. Tab shows dirty indicator (dot/asterisk) when file has unsaved changes
  4. Closing tab with unsaved changes shows confirmation dialog (Save/Don't Save/Cancel)
  5. User can switch tabs with Cmd+1-9 and mouse click
**Plans**: TBD

Plans:
- [ ] 04-01: File tree to editor pane wiring
- [ ] 04-02: Save file implementation with dirty state
- [ ] 04-03: Tab dirty indicator UI
- [ ] 04-04: Close confirmation dialog
- [ ] 04-05: Tab switching (keyboard and mouse)

---

### Phase 5: Syntax Highlighting
**Goal**: Code is colorized using tree-sitter with theme colors, status bar shows file info
**Depends on**: Phase 1 (needs Document), can parallel with Phases 2-4
**Requirements**: VIS-01, VIS-04
**Success Criteria** (what must be TRUE):
  1. Rust code displays with syntax colors (keywords, types, strings, comments differentiated)
  2. Editing code triggers incremental re-highlighting (changes colorize within 100ms)
  3. Theme colors apply consistently to syntax tokens
  4. Status bar shows: file path, cursor line:column, language name, diagnostics count (0 until Phase 6)
**Plans**: TBD

Plans:
- [ ] 05-01: Wire Highlighter to EditorPane render
- [ ] 05-02: Incremental highlighting on edit
- [ ] 05-03: Theme color mapping
- [ ] 05-04: Status bar implementation

**Addresses Pitfalls:** P9 (incremental parse invalidation), P10 (query performance)

---

### Phase 6: LSP Foundation
**Goal**: LSP servers start automatically, documents sync correctly, and diagnostics display with proper error handling
**Depends on**: Phase 4 (needs file open/save workflow), Phase 5 (needs status bar)
**Requirements**: LSP-01, LSP-02, INFRA-02, INFRA-03, INFRA-04
**Success Criteria** (what must be TRUE):
  1. Opening a .rs file spawns rust-analyzer automatically
  2. LSP receives didOpen when file opens, didChange on edit, didClose on close
  3. Diagnostics appear as inline squiggles on error lines
  4. Gutter shows error/warning icons for lines with diagnostics
  5. Status bar shows diagnostics count (e.g., "2 errors, 1 warning")
  6. LSP requests timeout after configured duration (30s heavy, 5s quick)
  7. LSP server crash triggers graceful restart without losing editor state
**Plans**: TBD

Plans:
- [ ] 06-01: LSP server auto-spawn on file open
- [ ] 06-02: Document synchronization (didOpen/didChange/didClose)
- [ ] 06-03: Diagnostics rendering (squiggles and gutter icons)
- [ ] 06-04: Status bar diagnostics count
- [ ] 06-05: Request timeout implementation
- [ ] 06-06: Server crash recovery and restart
- [ ] 06-07: Handle server-initiated requests (window/showMessage, etc.)

**Addresses Pitfalls:** P1 (unhandled server requests), P2 (no timeouts), P3 (no recovery), P4 (sync drift)

**Research Flag:** LSP synchronization patterns - may need `/gsd:research-phase` for document lifecycle and error recovery

---

### Phase 7: LSP Interaction
**Goal**: User can navigate code with go-to-definition, view type info on hover, and get autocomplete suggestions
**Depends on**: Phase 6
**Requirements**: LSP-03, LSP-04, LSP-05
**Success Criteria** (what must be TRUE):
  1. F12 or Cmd+click on symbol jumps to definition (opens file and positions cursor)
  2. Navigation maintains a stack (can go back to previous location)
  3. Hovering over symbol shows popup with type information and documentation
  4. Typing triggers autocomplete popup with LSP suggestions
  5. Selecting autocomplete item inserts the completion at cursor
**Plans**: TBD

Plans:
- [ ] 07-01: Go-to-definition request and navigation
- [ ] 07-02: Navigation stack (back/forward)
- [ ] 07-03: Hover popup UI and request
- [ ] 07-04: Autocomplete popup UI
- [ ] 07-05: Autocomplete trigger and insertion

**Addresses Pitfalls:** P18 (missing cancellation - cancel hover on mouse move)

---

### Phase 8: UI Completion
**Goal**: All UI components work together - file tree navigation, command palette, pane splits, and consistent theming
**Depends on**: Phase 4 (file management), Phase 7 (LSP features complete)
**Requirements**: UI-01, UI-02, UI-03, UI-04
**Success Criteria** (what must be TRUE):
  1. File tree shows workspace directory structure with file/folder icons
  2. File tree updates when files are created/deleted/renamed externally
  3. Command palette opens with Cmd+Shift+P and shows searchable command list
  4. Selecting command in palette executes the action
  5. User can split editor into multiple panes (horizontal/vertical)
  6. All UI components (file tree, tabs, status bar, popups) use consistent theme colors
**Plans**: TBD

Plans:
- [ ] 08-01: File tree directory structure and icons
- [ ] 08-02: File tree external change watching
- [ ] 08-03: Command palette search and dispatch
- [ ] 08-04: Pane splitting implementation
- [ ] 08-05: Theme consistency audit and fixes

---

## Requirement Coverage

| Requirement | Phase | Description |
|-------------|-------|-------------|
| INFRA-01 | Phase 1 | Application compiles and launches on macOS |
| VIS-02 | Phase 1 | Virtual scrolling for large files |
| EDIT-01 | Phase 2 | Text input with cursor and navigation |
| EDIT-02 | Phase 2 | Selection via keyboard and mouse |
| VIS-03 | Phase 2 | Current line highlight |
| EDIT-03 | Phase 3 | Clipboard operations |
| EDIT-04 | Phase 3 | Undo/redo with transaction grouping |
| FILE-01 | Phase 1+4 | Open file (async loading Phase 1, tree integration Phase 4) |
| FILE-02 | Phase 4 | Save file with dirty indicator |
| FILE-03 | Phase 4 | Close tab with confirmation |
| FILE-04 | Phase 4 | Tab switching |
| VIS-01 | Phase 5 | Syntax highlighting |
| VIS-04 | Phase 5 | Status bar |
| LSP-01 | Phase 6 | Document synchronization |
| LSP-02 | Phase 6 | Diagnostics display |
| INFRA-02 | Phase 6 | LSP server auto-spawn |
| INFRA-03 | Phase 6 | Request timeouts |
| INFRA-04 | Phase 6 | Graceful server restart |
| LSP-03 | Phase 7 | Go-to-definition |
| LSP-04 | Phase 7 | Hover information |
| LSP-05 | Phase 7 | Autocomplete |
| UI-01 | Phase 8 | File tree |
| UI-02 | Phase 8 | Command palette |
| UI-03 | Phase 8 | Multiple panes/splits |
| UI-04 | Phase 8 | Theme consistency |

**Coverage:** 25/25 v1 requirements mapped (100%)

---

## Dependencies Graph

```
Phase 1 (Foundation)
    |
    +---> Phase 2 (Text Editing) ---> Phase 3 (Edit Ops) ---> Phase 4 (File Mgmt)
    |                                                              |
    +---> Phase 5 (Syntax) ----------------------------------------+
                                                                   |
                                                                   v
                                                         Phase 6 (LSP Foundation)
                                                                   |
                                                                   v
                                                         Phase 7 (LSP Interaction)
                                                                   |
                                                                   v
                                                         Phase 8 (UI Completion)
```

**Parallelization Note:** Phase 5 (Syntax Highlighting) can run in parallel with Phases 2-4 since it only depends on Phase 1's Document wiring.

---

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation Validation | 5/5 | ✓ Complete | 2026-01-28 |
| 2. Core Text Editing | 0/6 | Not started | - |
| 3. Edit Operations | 0/3 | Not started | - |
| 4. File Management | 0/5 | Not started | - |
| 5. Syntax Highlighting | 0/4 | Not started | - |
| 6. LSP Foundation | 0/7 | Not started | - |
| 7. LSP Interaction | 0/5 | Not started | - |
| 8. UI Completion | 0/5 | Not started | - |

**Total Plans:** 40 estimated
**Total Requirements:** 25 mapped

---
*Roadmap created: 2026-01-27*
*Phase 1 planned: 2026-01-27*
*Depth: Comprehensive (8 phases, 40 plans)*
