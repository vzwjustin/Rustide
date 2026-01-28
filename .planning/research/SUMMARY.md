# Project Research Summary

**Project:** Rustide IDE
**Domain:** Code Editor / IDE Development
**Researched:** 2026-01-27
**Confidence:** HIGH

## Executive Summary

Rustide is a brownfield IDE project with strong foundational infrastructure (GPUI framework, tree-sitter, LSP/DAP clients, rope-based text buffer) but lacks critical integration and rendering patterns. The codebase has 9 well-separated crates but is missing the "glue layer" that wires components together into a functional editor. The primary challenge is not building new components, but completing the integration of existing scaffolding.

The recommended approach is to focus on **vertical completion** rather than horizontal expansion. Each integration phase should wire one complete user workflow (e.g., "open file → edit → save" or "type code → see diagnostics → goto definition"). This approach mirrors how Zed evolved: they built a project coordination layer first, then connected components through observable data flow rather than direct coupling.

Key risks include LSP synchronization drift, unvirtualized rendering breaking on large files, and missing background work isolation causing UI freezes. All are preventable through established patterns documented in the research. The path to foundation completion is clear: wire existing components through a project coordination layer, implement viewport-aware rendering, and add text shaping for correctness.

## Key Findings

### Recommended Stack

Rustide's existing stack is solid and needs minimal additions. The main gaps are in text rendering (no shaping or layout engine), input handling (no IME support), and virtual scrolling (currently renders all lines). Adding cosmic-text 0.12 for text layout, implementing virtual scrolling as an architectural pattern, and using tokio for async file I/O addresses the critical gaps. The existing dependencies (GPUI 0.2, tree-sitter 0.22, ropey 1.6, tokio 1.43) require no changes.

**Core technologies to add:**
- **cosmic-text 0.12**: Text layout and shaping — handles complex scripts, font fallback, and bidirectional text correctly
- **arboard 3.4**: Clipboard integration — cross-platform clipboard access for copy/paste operations
- **Virtual scrolling (pattern)**: Only render visible lines — prevents UI freeze on large files (100K+ lines)

**Critical patterns (no new crates):**
- Async file I/O using existing tokio — prevent UI blocking on large file loads
- Viewport-aware rendering — calculate visible line range, render only those + buffer
- IME event handling through GPUI — support international text input

**Keep as-is:**
- GPUI 0.2, tree-sitter 0.22, ropey 1.6, lsp-types 0.97, tokio 1.43, notify 7.0 — all current and appropriate

**Do NOT add:**
- xi-rope (abandoned), druid/iced (framework conflict), syntect (duplicate of tree-sitter), mlua (premature complexity)

### Expected Features

Research identified 10 categories of table stakes features and 10 categories of polish features. The foundation requires completing basic text editing, file operations, LSP core features, and syntax highlighting. Advanced features like multi-cursor, workspace splitting, and debugger UI can wait.

**Must have (table stakes):**
- Text editing fundamentals — typing, cursor movement, selection (keyboard + mouse), clipboard (copy/cut/paste)
- File operations — open, save, close, dirty indicators, save confirmation dialogs
- LSP integration core — document sync (didOpen/didChange), diagnostics display, go-to-definition, hover
- Syntax highlighting — apply tree-sitter highlighting in render with theme colors
- Basic UI — functional tab switching, file tree interaction, status bar diagnostics count

**Should have (competitive):**
- Advanced navigation — go-to-line, go-to-symbol, fuzzy file finder (command palette)
- Search & replace — in-file find with regex support, project-wide search
- Multi-cursor editing — Cmd+click, select all occurrences
- Git integration UI — gutter diff markers, changed file indicators

**Defer (v2+):**
- Extension/plugin system — foundation must be stable first
- AI features — agent scaffolding exists but wiring can wait
- Debugger UI — DAP infrastructure exists but complex UI can follow LSP completion
- Remote development, Vim/Emacs modes, collaborative editing — significant scope additions

**Feature dependency flow:**
Text Input → Selection → Clipboard → Undo/Redo → Save File → LSP Document Sync → Diagnostics/Go-to-Definition/Hover/Autocomplete

### Architecture Approach

Modern IDEs use a project coordination layer (like Zed's `Project` abstraction) that sits between UI and domain crates. This layer provides unified document identity, observable state changes, and lifecycle management for project-scoped resources (LSP servers, file watchers). Rustide currently lacks this layer, causing components to be isolated islands.

**Major components needed:**
1. **Project coordination layer** — single source of truth for workspace state, manages document cache and LSP server lifecycle
2. **Observable data flow** — components subscribe to state changes (DiagnosticStore updates → UI re-renders) rather than polling
3. **Document-centric wiring** — EditorPane holds Entity<Document> instead of raw String; Document drives LSP sync automatically
4. **Background executor** — unified approach for compute-heavy tasks (highlighting, parsing) off main thread

**Current architecture strength:** 9-crate layered separation with clean bottom-up dependencies

**Critical gap:** No horizontal coordination; EditorPane directly holds String content with no connection to editor_core::Document or LSP servers

**Data flow to implement:**
- User opens file → Project creates/retrieves Document → triggers LSP didOpen + highlighting
- User edits text → Document buffer updates → auto-increments version → sends LSP didChange → stores edit in History
- LSP sends diagnostics → DiagnosticStore updates → notifies observers → UI re-renders with squiggles

### Critical Pitfalls

Research identified 20 pitfalls across 7 categories. The most critical are listed below with prevention strategies.

1. **No virtual scrolling (P12)** — Current editor renders ALL lines; will freeze on large files. Implement viewport calculation: only render visible lines + small overscan buffer. Use fixed line height for predictable positioning.

2. **LSP document sync drift (P4)** — When buffer edits and LSP versions desynchronize, all language features break. Atomic version increment on every mutation; include version in all LSP requests; validate diagnostic versions before display.

3. **Unhandled LSP server requests (P1)** — Language servers send requests to clients (window/showMessageRequest, workspace/applyEdit). Currently has TODO at line 358 in client.rs. Implement all server-initiated request handlers or return proper error responses.

4. **Blocking main thread on file I/O (P5)** — Synchronous fs::read_to_string in Document::open() freezes UI on large files. Use tokio::fs for all file operations; stream large files in chunks; show loading skeleton.

5. **No request timeouts (P2)** — LSP requests can hang indefinitely. Implement configurable timeouts (30s for heavy ops, 5s for quick ops); track pending requests with timestamps; allow cancellation via $/cancelRequest.

**Phase mapping for pitfall avoidance:**
- Foundation Validation: Address P5, P12 (blocking I/O, virtual scrolling)
- LSP Stabilization: Address P1, P2, P3 (server requests, timeouts, recovery)
- Editor Integration: Address P4, P11 (sync drift, layout thrashing)

## Implications for Roadmap

Based on combined research, the roadmap should follow vertical integration phases that complete end-to-end workflows. Each phase wires existing components rather than building new ones. This approach ensures continuous validation and avoids the "integration hell" phase at the end.

### Suggested Phase Structure

#### Phase 1: Foundation Validation
**Rationale:** Prove the GPUI + editor_core integration works before adding complexity. Wire EditorPane to real Document objects instead of raw Strings. This unblocks all subsequent phases.

**Delivers:**
- Functional text editing loop (type, cursor movement, selection)
- Real Document objects driving editor panes
- Async file loading (no UI freeze)
- Virtual scrolling pattern (renders only visible lines)

**Addresses from FEATURES.md:**
- Text input, backspace/delete, cursor movement, selection
- Page up/down, home/end navigation
- Basic file open

**Avoids from PITFALLS.md:**
- P5 (blocking I/O), P12 (no virtualization), P6 (Unicode handling)

**Technology from STACK.md:**
- Uses existing ropey for text buffer
- Implements virtual scrolling pattern (no new crate)
- Adds async file I/O with tokio

**Research flag:** Standard patterns, skip deep research

---

#### Phase 2: Core Editing Loop
**Rationale:** Complete the basic editing experience users expect. Clipboard, undo/redo, and save must work before adding language features.

**Delivers:**
- Clipboard integration (copy/cut/paste)
- Undo/redo wired to UI
- Save file with dirty state tracking
- Tab management (switch, close with confirmation)

**Addresses from FEATURES.md:**
- Clipboard operations
- Undo/Redo user-facing
- Save file, save-as, close tab
- Modified indicator, prompt save on close

**Avoids from PITFALLS.md:**
- P7 (undo state corruption)

**Technology from STACK.md:**
- Adds arboard 3.4 for clipboard
- Uses existing History in editor_core

**Research flag:** Standard patterns, skip deep research

---

#### Phase 3: Syntax Highlighting
**Rationale:** Visual feedback makes the editor feel alive. Tree-sitter infrastructure exists; needs wiring to render layer.

**Delivers:**
- Syntax coloring applied to rendered text
- Incremental re-highlighting on edits
- Theme colors properly mapped
- Current line highlight

**Addresses from FEATURES.md:**
- Syntax highlighting visual application
- Theme color integration

**Avoids from PITFALLS.md:**
- P9 (incremental parse invalidation), P10 (query performance at scale)

**Technology from STACK.md:**
- Uses existing tree-sitter 0.22 and Highlighter
- May need cosmic-text for proper text layout if special characters cause issues

**Research flag:** Standard patterns, but may need `/gsd:research-phase` if cosmic-text integration proves complex

---

#### Phase 4: LSP Foundation
**Rationale:** Language features are the core value of an IDE. Start LSP servers on file open and sync document changes. This is the most complex integration.

**Delivers:**
- LSP servers start on file open (rust-analyzer for .rs)
- Document synchronization (didOpen/didChange/didClose)
- Diagnostics display (squiggles and gutter icons)
- Status bar diagnostics count

**Addresses from FEATURES.md:**
- LSP integration core features
- Start LSP on file open, document sync
- Diagnostics display, error/warning gutter icons
- Status bar updates

**Avoids from PITFALLS.md:**
- P1 (unhandled server requests), P2 (missing timeouts), P3 (no server recovery), P4 (sync drift)

**Technology from STACK.md:**
- Uses existing lsp_bridge infrastructure
- Implements observable DiagnosticStore pattern

**Research flag:** NEEDS `/gsd:research-phase` — LSP synchronization is complex; need to research document sync patterns, request lifecycle, and error recovery

---

#### Phase 5: LSP Interaction
**Rationale:** Build on LSP foundation with user-triggered features. Hover, go-to-definition, and autocomplete complete the core IDE experience.

**Delivers:**
- Go-to-definition (F12 / Cmd+click)
- Hover information popup
- Autocomplete popup with LSP suggestions
- Signature help

**Addresses from FEATURES.md:**
- LSP interactive features
- Go to definition, hover, autocomplete, signature help

**Avoids from PITFALLS.md:**
- P18 (missing cancellation — cancel hover on mouse move)

**Technology from STACK.md:**
- Uses existing LspClient methods
- Needs popup UI components

**Research flag:** Standard patterns once Phase 4 complete

---

#### Phase 6: Project Coordination Layer (Architecture Strengthening)
**Rationale:** Refactor to introduce Project abstraction for cleaner state management. This isn't user-facing but enables multi-file operations and cleaner architecture.

**Delivers:**
- Project crate coordinating workspace state
- Document cache managed at project level
- LSP server lifecycle tied to project
- Observable properties for reactive updates

**Addresses from ARCHITECTURE.md:**
- Project coordination layer
- Observable data flow
- Background work isolation
- Document-centric wiring

**Avoids from PITFALLS.md:**
- P16 (circular dependencies), P17 (global mutable state)

**Technology from STACK.md:**
- Pattern-based, no new crates
- Architectural refactoring

**Research flag:** May need `/gsd:research-phase` if observable pattern implementation unclear

---

#### Phase 7: Search & Navigation
**Rationale:** Users expect to find things quickly. In-file search unlocks productivity.

**Delivers:**
- Find in file (Cmd+F)
- Find next/previous navigation
- Replace and replace all
- Case sensitivity, regex, whole word options

**Addresses from FEATURES.md:**
- Search & replace in-file

**Avoids from PITFALLS.md:**
- (None specific, standard UI pattern)

**Technology from STACK.md:**
- Rust regex crate (standard library quality)
- Search UI component

**Research flag:** Standard patterns, skip deep research

---

#### Phase 8: Advanced Features
**Rationale:** Polish and differentiators. Multi-cursor, git gutter, advanced navigation.

**Delivers:**
- Multi-cursor editing
- Git gutter diff markers
- Go-to-symbol, go-to-line
- Command palette, fuzzy file finder

**Addresses from FEATURES.md:**
- Nice-to-have features for polish

**Avoids from PITFALLS.md:**
- (Various, depends on features selected)

**Technology from STACK.md:**
- Uses existing gitx crate
- Fuzzy-matcher already in dependencies

**Research flag:** Feature-dependent; likely standard patterns

---

### Phase Ordering Rationale

1. **Foundation first:** Phases 1-2 establish the core edit loop. Nothing else matters if typing and saving don't work.

2. **Visual feedback early:** Phase 3 (syntax highlighting) makes the editor feel professional and provides quick validation that parsing works.

3. **LSP as cornerstone:** Phases 4-5 are the most complex and highest value. Split into foundation (sync, diagnostics) and interaction (hover, autocomplete) for manageable scope.

4. **Architecture strengthening optional:** Phase 6 can be deferred if Phases 1-5 work acceptably without it. Include it when multi-file operations (rename, find-all-references) become important.

5. **Polish last:** Phases 7-8 add productivity features but aren't blocking for basic IDE functionality.

**Dependency flow:**
- Phase 2 depends on Phase 1 (needs Document objects)
- Phase 3 independent (can parallel with Phase 2)
- Phase 4 depends on Phase 1 (needs Document version tracking)
- Phase 5 depends on Phase 4 (needs LSP servers running)
- Phase 6 refactors Phases 1-5 (must come after)
- Phases 7-8 can happen in any order after Phase 2

### Research Flags

**Phases needing `/gsd:research-phase` during planning:**
- **Phase 4 (LSP Foundation):** Complex synchronization patterns, error recovery, notification handling — research LSP document lifecycle and Zed's implementation
- **Phase 6 (Project Layer):** Observable pattern implementation in Rust/GPUI context — research Entity subscription patterns

**Phases with well-documented patterns (skip research):**
- **Phase 1 (Foundation Validation):** Virtual scrolling is universal pattern; async file I/O is standard tokio
- **Phase 2 (Core Editing Loop):** Clipboard, undo/redo are established patterns
- **Phase 3 (Syntax Highlighting):** Tree-sitter integration is straightforward
- **Phase 5 (LSP Interaction):** Request/response pattern established in Phase 4
- **Phase 7 (Search & Navigation):** Standard UI patterns
- **Phase 8 (Advanced Features):** Feature-dependent but generally standard

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Existing dependencies are appropriate; gaps (cosmic-text, arboard) are standard solutions |
| Features | HIGH | Clear table stakes vs nice-to-have separation; feature dependencies well-understood |
| Architecture | HIGH | Gap analysis is precise; Zed patterns well-documented and applicable |
| Pitfalls | HIGH | 20 pitfalls identified from codebase analysis and IDE domain knowledge; prevention strategies concrete |

**Overall confidence:** HIGH

### Gaps to Address

While research confidence is high, several areas need validation during implementation:

- **GPUI 0.2 IME support:** Verify GPUI's platform layer exposes IME events properly; if not, text input for CJK languages won't work
- **cosmic-text GPUI integration:** Test whether cosmic-text's layout APIs integrate cleanly with GPUI's rendering; may need adapter layer
- **Observable pattern in GPUI:** Research assumes GPUI's Entity<T> supports observer pattern; verify context.observe() or build custom
- **Virtual scrolling with GPUI layout:** Confirm GPUI's layout system allows absolute positioning for virtual scrolling; may need custom scroll container
- **Tree-sitter grammar version conflicts:** TOML/Markdown grammars currently disabled due to cc version conflicts; need build system solution or vendor grammars

**Mitigation strategy:** Each phase includes validation tasks for its assumptions. If GPUI limitations discovered, fall back to manual implementations (e.g., custom observable via channels if Entity doesn't support it).

## Sources

### Primary (HIGH confidence)
- **Rustide codebase analysis** — Direct inspection of 9 crates revealed architecture, existing capabilities, and gaps
- **Zed source code patterns** — Zed is open-source and uses same tech (GPUI, tree-sitter, rust-analyzer); architecture patterns directly applicable
- **LSP specification** — Official Language Server Protocol spec from Microsoft; defines client responsibilities
- **DAP specification** — Official Debug Adapter Protocol spec; defines client requirements
- **Tree-sitter documentation** — Official docs for incremental parsing and query system
- **GPUI 0.2 documentation** — Framework capabilities and limitations

### Secondary (MEDIUM confidence)
- **Ropey documentation** — Rope data structure capabilities for text editing
- **cosmic-text documentation** — Text layout and shaping library capabilities
- **VS Code architecture** — Public talks and documentation about extension architecture (less directly applicable due to Electron vs native)

### Tertiary (LOW confidence)
- **Sublime Text patterns** — Inferred from behavior; source code not available
- **Editor comparison blog posts** — Community consensus on table stakes features; needs validation with user testing

---
*Research completed: 2026-01-27*
*Ready for roadmap: yes*
