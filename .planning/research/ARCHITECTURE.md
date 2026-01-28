# Architecture Research: Modern IDE Patterns

**Research Date:** 2026-01-27
**Dimension:** Architecture
**Question:** How are modern IDEs like Zed structured? What architectural patterns are we missing or should strengthen?

---

## Executive Summary

Rustide's 9-crate workspace provides a solid foundation but has **critical gaps in the glue layer** between crates. Modern IDEs like Zed use sophisticated patterns for:
1. **Entity-based state management** - unified identity across components
2. **Observable data flow** - reactive updates without polling
3. **Project-level coordination** - single source of truth for workspace state
4. **Background work isolation** - non-blocking UI regardless of computation

Rustide has the layered separation but lacks the **connective tissue** that makes components work together seamlessly.

---

## Component Analysis

### Current Rustide Architecture

```
                    +-----------------+
                    |    ui_shell     |  (Presentation)
                    |  GPUI + Layout  |
                    +--------+--------+
                             |
         +-------------------+-------------------+
         |         |         |         |         |
    +----v----+ +--v---+ +--v----+ +--v---+ +---v---+
    | editor  | | lsp  | | dap   | |agents| | gitx  |
    | _core   | |bridge| |bridge | |      | |       |
    +----+----+ +--+---+ +---+---+ +--+---+ +---+---+
         |         |         |        |         |
    +----v---------v---------v--------v---------v----+
    |              languages / security              |
    |            (Tree-sitter, Policies)             |
    +------------------------------------------------+
```

**Strength:** Clean layered separation with bottom-up dependencies.
**Weakness:** No horizontal coordination layer; each crate is an island.

### Zed's Architecture Pattern

Zed introduces a **Project abstraction** that coordinates state across concerns:

```
                    +-----------------+
                    |   Workspace     |  (View Layer)
                    +--------+--------+
                             |
                    +--------v--------+
                    |     Project     |  <-- Missing in Rustide
                    | (State + Coord) |
                    +--------+--------+
         +-------------------+-------------------+
         |         |         |         |         |
    +----v----+ +--v---+ +--v----+ +--v---+ +---v---+
    | Buffer  | | LSP  | | DAP   | | AI   | | Git   |
    +----+----+ +--+---+ +---+---+ +--+---+ +---+---+
```

**Key Patterns from Zed:**

| Pattern | Purpose | Zed Implementation |
|---------|---------|-------------------|
| **Entity System** | Unified identity for all state objects | `Entity<T>` with global ID |
| **Model/View Separation** | State independent of rendering | `Model<Project>` holds state |
| **Observable Properties** | Reactive updates | `observe()` subscriptions |
| **Background Executor** | Non-blocking computation | `background_executor` pool |
| **Context Propagation** | Thread-safe state access | `&AppContext` / `&mut ModelContext` |

---

## Gaps in Current Architecture

### 1. No Project Coordination Layer

**Current State:**
- `Workspace` (ui_shell) directly holds `FileTree`, `EditorPane`, etc.
- Each component manages its own state independently
- No unified model of "the project" that spans UI and domain

**What's Missing:**
- Single source of truth for open files, dirty state, workspace root
- Coordination for cross-cutting operations (e.g., rename symbol across files)
- Lifecycle management for project-scoped resources (LSP servers, watchers)

**Recommendation:** Add `project` crate between ui_shell and feature crates.

### 2. No Observable Data Flow

**Current State:**
- Components communicate via direct method calls
- UI re-renders via `cx.notify()` - manual invalidation
- No subscription mechanism for state changes

**What's Missing:**
- `editor_core::Document` changes should notify `lsp_bridge` automatically
- `lsp_bridge::DiagnosticStore` changes should notify UI automatically
- File system changes should propagate to all interested parties

**Recommendation:** Implement observer pattern via channels or callback registration.

### 3. Weak Buffer-to-Server Binding

**Current State:**
- `EditorPane` has `tabs: Vec<EditorTab>` with String content
- No connection between editor buffer and `editor_core::Document`
- No connection between document and LSP servers

**What's Missing:**
- Opening a file should: create Document -> notify LSP -> start highlighting
- Editing should: update Document -> send didChange -> update diagnostics
- Closing should: close Document -> notify LSP -> cleanup state

**Recommendation:** Wire `editor_core::Document` into editor panes; bridge to LSP.

### 4. No Background Work System

**Current State:**
- LSP client spawns reader thread
- DAP client uses tokio tasks
- No unified approach to background work

**What's Missing:**
- Consistent way to run compute (highlighting, parsing) off main thread
- Progress reporting for long operations
- Cancellation for superseded work

**Recommendation:** Implement background executor pattern with task priorities.

### 5. Missing Settings Infrastructure

**Current State:**
- Theme hardcoded in `ui_shell::theme`
- No user settings, no workspace settings
- LSP server configs are compile-time

**What's Missing:**
- User preferences that affect behavior (indent, line ending)
- Workspace-local settings (.rustide/ directory)
- Runtime-configurable language server paths

**Recommendation:** Add settings crate with layered configuration.

---

## Data Flow Patterns to Implement

### Pattern 1: Document Lifecycle

```
User Opens File
      |
      v
+---------------+    +------------------+    +----------------+
| Workspace     |--->| Project          |--->| Document       |
| open_file()   |    | get_or_create()  |    | from_path()    |
+---------------+    +--------+---------+    +--------+-------+
                              |                       |
                              v                       v
                     +-----------------+     +------------------+
                     | ServerManager   |     | Highlighter      |
                     | did_open()      |     | parse()          |
                     +-----------------+     +------------------+
```

**Required Bindings:**
1. `ui_shell` -> `project` (new): File open request
2. `project` -> `editor_core`: Document creation/caching
3. `project` -> `lsp_bridge`: LSP notifications
4. `project` -> `languages`: Syntax highlighting

### Pattern 2: Edit Flow with LSP Sync

```
User Types Character
      |
      v
+---------------+    +------------------+    +----------------+
| EditorPane    |--->| Document         |--->| Buffer         |
| handle_input()|    | insert()         |    | insert()       |
+-------+-------+    +--------+---------+    +--------+-------+
        |                     |                       |
        v                     v                       v
+---------------+    +------------------+    +----------------+
| GPUI notify() |    | Version++        |    | History record |
+---------------+    +--------+---------+    +----------------+
                              |
                              v
                     +-----------------+
                     | ServerManager   |
                     | did_change()    |
                     +--------+--------+
                              |
                              v
                     +-----------------+
                     | DiagnosticStore |
                     | (async update)  |
                     +--------+--------+
                              |
                              v
                     +-----------------+
                     | UI subscribe    |
                     | re-render       |
                     +-----------------+
```

**Required Bindings:**
1. `EditorPane` must use actual `Document`, not raw String
2. `Document` must trigger LSP sync on mutation
3. `DiagnosticStore` must notify UI on changes

### Pattern 3: Diagnostic Display

```
LSP Server Sends publishDiagnostics
      |
      v
+-------------------+    +--------------------+    +------------------+
| LspClient         |--->| DiagnosticStore    |--->| Observers        |
| reader thread     |    | set_diagnostics()  |    | (UI, StatusBar)  |
+-------------------+    +--------+-----------+    +------------------+
                                  |
                                  v
                         +------------------+
                         | Query API        |
                         | for_file(uri)    |
                         | count()          |
                         +------------------+
```

**Current State:** Notification callback exists; store exists; no observer mechanism.

**Required:** Add observer registration to `DiagnosticStore`.

---

## Build Order for Getting Things Working

### Phase A: Core Loop (Minimal Viable IDE)

**Goal:** Open file, see text, edit text, save file.

| Order | Component | Work Required | Deps |
|-------|-----------|---------------|------|
| A1 | `ui_shell` | Fix GPUI window creation | None |
| A2 | `editor_core` | Verify Document/Buffer works | None |
| A3 | `ui_shell::EditorPane` | Wire to real Document (not String) | A2 |
| A4 | `ui_shell` | File open dialog -> Document | A3 |
| A5 | `ui_shell` | Save command -> Document::save() | A3 |

**Validation:** Can open, edit, save a file. No highlighting. No LSP.

### Phase B: Syntax Highlighting

**Goal:** Text is colorized based on language.

| Order | Component | Work Required | Deps |
|-------|-----------|---------------|------|
| B1 | `languages` | Test grammar loading for Rust | None |
| B2 | `languages::Highlighter` | Test parse + highlight_spans | B1 |
| B3 | `editor_pane` | Render with highlight spans | B2, A3 |
| B4 | `project` (new or inline) | Language detection on file open | B3 |

**Validation:** Opening .rs file shows Rust syntax colors.

### Phase C: LSP Integration

**Goal:** Diagnostics appear; go-to-definition works.

| Order | Component | Work Required | Deps |
|-------|-----------|---------------|------|
| C1 | `lsp_bridge` | Test LspClient::start() with rust-analyzer | None |
| C2 | `lsp_bridge::ServerManager` | Start server on file open | C1 |
| C3 | `lsp_bridge` | Wire did_open/did_change to Document | C2, A3 |
| C4 | `lsp_bridge::DiagnosticStore` | Add observer mechanism | C2 |
| C5 | `editor_pane` | Render diagnostic markers | C4 |
| C6 | `lsp_bridge` | Implement goto_definition request | C2 |
| C7 | `editor_pane` | Wire goto-definition command | C6 |

**Validation:** Red squiggles on errors; Ctrl+click goes to definition.

### Phase D: Project Layer (Optional Strengthening)

**Goal:** Unified state management for multi-file operations.

| Order | Component | Work Required | Deps |
|-------|-----------|---------------|------|
| D1 | `project` crate | Create with document cache | A-C |
| D2 | `project` | Manage server lifecycle | D1, C2 |
| D3 | `project` | Observable document state | D1 |
| D4 | Refactor `ui_shell` | Use Project instead of direct crate calls | D3 |

**Validation:** Architecture supports rename-across-files, find-all-references.

---

## Component Boundaries to Clarify

### 1. Who Owns the Document?

**Current ambiguity:**
- `EditorPane` has `EditorTab` with `content: String`
- `editor_core` has `Document` with `Buffer`
- No connection between them

**Resolution:**
- `editor_core::Document` is the canonical document state
- `EditorPane` holds `Entity<Document>` (or `Arc<RwLock<Document>>`)
- UI renders from Document; edits go to Document

### 2. Who Starts LSP Servers?

**Current ambiguity:**
- `ServerManager` exists but nothing calls `start_server()`
- No trigger for "file opened that needs LSP"

**Resolution:**
- Project layer (or Workspace) detects language on file open
- Project calls `ServerManager::start_server(language_id)` if needed
- Project tracks which servers are active per language

### 3. Who Routes Diagnostics to UI?

**Current ambiguity:**
- `DiagnosticStore::set_diagnostics()` is called from notification callback
- No path from DiagnosticStore to EditorPane

**Resolution:**
- `DiagnosticStore` emits events (channel or observer)
- EditorPane subscribes to diagnostics for its document URI
- EditorPane re-renders on diagnostic updates

### 4. Who Manages Background Tasks?

**Current ambiguity:**
- LSP uses std::thread
- DAP uses tokio tasks
- No unified approach

**Resolution:**
- All async work uses tokio runtime
- Create background executor for compute-heavy tasks (highlighting)
- Ensure UI thread never blocks

---

## Recommendations Summary

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| P0 | Wire EditorPane to Document | 1 day | Enables real editing |
| P0 | File open creates Document | 0.5 day | Core flow |
| P0 | Test GPUI window works | 0.5 day | Validate foundation |
| P1 | Language detection + highlighting | 1 day | Visual feedback |
| P1 | Start rust-analyzer on .rs open | 1 day | LSP foundation |
| P1 | Wire did_open/did_change | 1 day | LSP sync |
| P1 | Diagnostic rendering | 1 day | Error visibility |
| P2 | Project coordination layer | 2-3 days | Clean architecture |
| P2 | Observable DiagnosticStore | 0.5 day | Reactive updates |
| P2 | Request timeouts | 0.5 day | Reliability |
| P3 | Settings infrastructure | 2 days | User config |
| P3 | Background executor | 1 day | Performance |

---

## Quality Gate Checklist

- [x] Components clearly defined with boundaries
  - Document: canonical text state owner
  - Project: coordination layer (to add)
  - ServerManager: LSP server lifecycle
  - Workspace: UI layout and routing

- [x] Data flow direction explicit
  - User input -> EditorPane -> Document -> LSP
  - LSP notification -> DiagnosticStore -> UI observers
  - File open -> Project -> Document + LSP + Highlighter

- [x] Build order implications noted
  - Phase A: Core edit loop (no dependencies)
  - Phase B: Highlighting (requires Phase A)
  - Phase C: LSP (requires Phase A, can parallel with B)
  - Phase D: Project layer (strengthening after C)

---

*Research completed: 2026-01-27*
