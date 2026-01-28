# IDE Development Pitfalls

Critical mistakes to avoid when building Rustide. Each pitfall includes warning signs for early detection, prevention strategies, and the phase where it should be addressed.

---

## Category: LSP Integration

### P1: Unhandled Server Requests

**Description:** Language servers send requests to clients (not just notifications), such as `window/showMessageRequest`, `workspace/applyEdit`, and `client/registerCapability`. Ignoring these breaks core functionality.

**Warning Signs:**
- Log messages showing "Received server request" but no handler
- Features like refactoring or code actions silently fail
- Server capabilities not dynamically registering
- "Workspace edit failed" without visible error to user

**Prevention:**
1. Implement request handlers for all server-initiated requests in the LSP spec
2. Return proper error responses (not silence) for unsupported requests
3. Test with servers that heavily use client requests (rust-analyzer uses many)
4. Log unhandled requests at warning level, not debug

**Phase:** LSP Stabilization - this is currently a known issue in `lsp_bridge/src/client.rs:358` (TODO comment)

**Current Status:** The codebase has `// TODO: Handle server requests` at line 358-359

---

### P2: Missing Request Timeouts

**Description:** LSP requests without timeouts can hang indefinitely, blocking the UI and consuming resources. A crashed or slow server becomes unrecoverable.

**Warning Signs:**
- IDE becomes unresponsive after opening certain files
- Memory/CPU grows unboundedly when server is slow
- Cannot cancel in-flight requests
- No user feedback during long operations

**Prevention:**
1. Implement configurable timeouts for all LSP requests (default: 30s for heavy ops, 5s for quick ops)
2. Track pending requests with timestamps in `pending_requests` map
3. Implement cancellation via `$/cancelRequest` notification
4. Show progress indicators for long-running requests
5. Allow users to force-cancel stuck operations

**Phase:** LSP Stabilization - identified as known issue in PROJECT.md

**Implementation Notes:**
```rust
// In LspClient::request() - add timeout wrapper
let result = tokio::time::timeout(
    Duration::from_secs(timeout_secs),
    self.send_and_wait(request)
).await??;
```

---

### P3: No Graceful Server Recovery

**Description:** When an LSP server crashes, the IDE should automatically restart it and restore state (open documents, pending requests). Without this, users must manually restart.

**Warning Signs:**
- Server crash leaves IDE in broken state
- "Server not running" errors with no recovery path
- Lost diagnostics after server restart
- Users must close/reopen files to restore LSP features

**Prevention:**
1. Detect server death via process exit or broken pipe
2. Implement exponential backoff restart (1s, 2s, 4s, max 30s)
3. Re-send `didOpen` for all tracked documents after restart
4. Queue requests during restart, replay after initialization
5. Limit restart attempts (e.g., 5 times in 5 minutes)
6. Notify user when restart limit exceeded

**Phase:** LSP Stabilization - identified as known issue

**Current State:** `ServerManager` has basic restart logic but doesn't track document state for recovery

---

### P4: Document Synchronization Drift

**Description:** When buffer edits and LSP document versions desynchronize, all language features break (wrong positions, stale diagnostics, failed requests).

**Warning Signs:**
- Diagnostics appear at wrong line numbers
- Go-to-definition jumps to incorrect locations
- Completions suggest invalid items
- Server rejects requests with version mismatch errors

**Prevention:**
1. Atomic version increment on every buffer mutation
2. Include version in ALL document-related LSP requests
3. Validate diagnostic versions before display (drop stale)
4. Full document resync on detected drift
5. Unit tests that verify version consistency across edit sequences

**Phase:** Editor Integration

**Current Status:** `Document` has version tracking but `EditorPane` doesn't sync with LSP

---

## Category: Text Editing

### P5: Blocking Main Thread on File I/O

**Description:** Reading/writing large files on the main thread freezes the UI. This is especially bad for files over a few MB.

**Warning Signs:**
- UI freezes when opening large files
- Typing lag after saving
- Blank editor during file load
- macOS spinning beach ball

**Prevention:**
1. All file I/O must happen on tokio task pool
2. Stream large files in chunks with progress feedback
3. Set reasonable file size limits (warn at 10MB, refuse at 100MB)
4. Use background thread for syntax highlighting of large files
5. Show loading skeleton while file loads

**Phase:** Foundation Validation

**Current Status:** `Document::open()` uses synchronous `fs::read_to_string()` - needs async

---

### P6: Incorrect Unicode Handling

**Description:** Treating bytes as characters breaks multi-byte UTF-8, emoji, and combining characters. Cursor movement and editing become wrong.

**Warning Signs:**
- Cursor jumps multiple visual characters at once
- Deleting emoji leaves partial bytes
- Column numbers don't match visual position
- Crashes on non-ASCII input

**Prevention:**
1. Use grapheme cluster iteration for cursor movement (already using `unicode_segmentation`)
2. Convert between byte offsets and char offsets explicitly
3. Test with emoji, CJK, RTL text, combining diacritics
4. Use rope's proper char/byte conversion (already in `Buffer`)
5. Document coordinate system (bytes vs chars vs graphemes)

**Phase:** Editor Core Hardening

**Current Status:** `Buffer` correctly distinguishes bytes/chars but `word_at()` may have edge cases

---

### P7: Undo/Redo State Corruption

**Description:** Transaction boundaries, cursor positions, and selection state must be properly captured. Incorrect grouping makes undo unpredictable.

**Warning Signs:**
- Undo removes too much or too little
- Cursor position wrong after undo
- Selection lost after undo
- Undo stack grows unboundedly

**Prevention:**
1. Group related edits into transactions (typing batch, refactoring)
2. Store cursor/selection state with each undo entry
3. Limit undo stack size (e.g., 1000 entries or 10MB)
4. Coalesce rapid character insertions (typing)
5. Test undo across all edit operations

**Phase:** Editor Core Hardening

**Current Status:** `History` has transaction support but cursor state not preserved

---

## Category: Tree-sitter Integration

### P8: Grammar Version Conflicts

**Description:** Different tree-sitter grammars may depend on conflicting versions of build dependencies (cc, regex), causing compile failures.

**Warning Signs:**
- Build fails with cc version conflicts
- Grammar crates have incompatible semver requirements
- Some grammars commented out (current state: TOML, Markdown)

**Prevention:**
1. Pin tree-sitter grammar versions carefully in Cargo.toml
2. Use workspace-level dependency resolution
3. Consider vendoring problematic grammars
4. Build grammars as separate dylibs if conflicts persist
5. Regular dependency audit

**Phase:** Language Support - currently blocking TOML/Markdown grammars

**Current Status:** Lines 189-202 in `grammar.rs` show TOML/Markdown commented out

---

### P9: Incremental Parse Invalidation

**Description:** Failing to properly inform tree-sitter of edits results in full reparses (slow) or incorrect parse trees.

**Warning Signs:**
- Syntax highlighting flickers after typing
- Large files lag during editing
- Highlighting becomes incorrect after edits
- Memory usage grows (tree accumulation)

**Prevention:**
1. Pass old tree and edit ranges to `parse()` method
2. Convert buffer edits to tree-sitter `InputEdit` format
3. Validate parse tree after edits (check for error nodes)
4. Profile incremental vs full parse times
5. Clear old trees to prevent memory leaks

**Phase:** Syntax Highlighting

**Current Status:** `Grammar::parse()` accepts old_tree but `Highlighter` may not track it properly

---

### P10: Query Performance at Scale

**Description:** Running syntax highlight queries on every keystroke in large files causes lag. Queries must be optimized and limited in scope.

**Warning Signs:**
- Typing lag in files over 1000 lines
- CPU spikes during editing
- Syntax colors update slowly after scrolling

**Prevention:**
1. Only query visible line range plus buffer
2. Debounce query execution (16ms typical)
3. Cache highlight results by tree version
4. Use tree-sitter's lazy iteration, don't collect all matches
5. Profile with large real-world files (10K+ lines)

**Phase:** Syntax Highlighting

---

## Category: UI/Rendering

### P11: Layout Thrashing

**Description:** Recalculating layout on every frame or minor state change causes dropped frames. GPUI is fast but not magic.

**Warning Signs:**
- Scroll stuttering
- Cursor movement feels laggy
- Resize operations janky
- High CPU during idle

**Prevention:**
1. Memoize expensive layout calculations
2. Only notify when visible state actually changes
3. Batch multiple state changes into single render
4. Use dirty tracking for partial relayout
5. Profile with GPUI's built-in metrics

**Phase:** UI Polish

**Current Status:** `EditorPane` re-renders entire line list on any change

---

### P12: Not Virtualizing Long Lists

**Description:** Rendering all lines/items in DOM for large files destroys performance. Only visible items should be rendered.

**Warning Signs:**
- Opening large file freezes UI
- Scroll is choppy in long files
- Memory usage proportional to file size
- File tree with many items is slow

**Prevention:**
1. Implement virtual scrolling for editor content
2. Calculate visible line range from scroll position
3. Render only visible lines plus small overscan buffer
4. Use fixed line height for predictable positioning
5. Same approach for file tree, search results

**Phase:** Foundation Validation - critical for usability

**Current Status:** `EditorPane::render_editor_content()` renders ALL lines (line 184-215)

---

### P13: Memory Leaks from Retained Entities

**Description:** GPUI entities (Entity<T>) that aren't properly dropped accumulate, causing memory growth and potential crashes.

**Warning Signs:**
- Memory grows when opening/closing files
- Closed tabs still trigger updates
- Entity count grows in profiler
- Eventually out-of-memory crash

**Prevention:**
1. Explicitly drop entities when views close
2. Use weak references for cross-entity communication
3. Implement Drop traits that clean up subscriptions
4. Monitor entity count during development
5. Test open/close cycles for memory leaks

**Phase:** Memory Management

---

## Category: Problem Matching

### P14: Multi-line Pattern Matching Failure

**Description:** Compiler output often spans multiple lines (Rust's error format is 5+ lines). Line-by-line matching misses context and location info.

**Warning Signs:**
- File/line info missing from problems
- Only first line of error captured
- Related notes/suggestions lost
- Can't click to navigate from error

**Prevention:**
1. Use stateful pattern matching (current problem context)
2. Accumulate lines until complete diagnostic detected
3. Parse structured output formats (JSON where available)
4. Use `cargo`'s `--message-format=json` for Rust
5. Test with real multi-line compiler output

**Phase:** Task Integration - identified as known issue

**Current Status:** `RustProblemMatcher` is stateless, doesn't accumulate lines

---

## Category: DAP Integration

### P15: Race Conditions in Debug Events

**Description:** Debug adapter events (stopped, continued, terminated) can arrive out of order or overlap. State machine must handle all transitions.

**Warning Signs:**
- Debug state shows "running" when actually stopped
- Breakpoints hit but UI doesn't update
- Can't step after hitting breakpoint
- Zombie debug sessions

**Prevention:**
1. Implement proper state machine for debug states
2. Queue events and process sequentially
3. Validate state transitions (some combos invalid)
4. Handle "stopped" event before "continued" response
5. Timeout stuck states

**Phase:** Debugger Integration

**Current Status:** `DebugSession` has state enum but races possible between events and commands

---

## Category: Architecture

### P16: Circular Dependencies Between Crates

**Description:** As crates grow, natural dependencies can become circular. This prevents compilation and indicates architectural problems.

**Warning Signs:**
- Build fails with cycle errors
- Feature additions require touching many crates
- Can't unit test crates in isolation
- Layering violations creep in

**Prevention:**
1. Define clear dependency direction (ui -> core -> common)
2. Use traits/interfaces at boundaries
3. Event-based communication for reverse direction
4. Regular dependency graph visualization
5. CI check for new cycles

**Phase:** Ongoing - architectural discipline

**Current Architecture:** 9 crates with apparent layering, validate during Foundation phase

---

### P17: Global Mutable State

**Description:** Globals make testing hard, concurrency unsafe, and code unpredictable. GPUI's Entity system should be used instead.

**Warning Signs:**
- `lazy_static!` or `static mut` in codebase
- Tests interfere with each other
- Hard to reason about state flow
- Race conditions in tests

**Prevention:**
1. Use GPUI Context for shared state
2. Pass dependencies explicitly
3. Use Entity<T> for stateful components
4. Document any unavoidable globals
5. Review PRs for new globals

**Phase:** Ongoing

**Current Status:** `current_theme()` in theme.rs uses global - acceptable for theming

---

### P18: Missing Cancellation Propagation

**Description:** When user switches files or closes views, in-flight operations (LSP requests, file loads, searches) should cancel to avoid wasted work and stale results.

**Warning Signs:**
- Results appear for previous file
- Old diagnostics flash on new file
- CPU busy after switching away
- Memory accumulates for abandoned work

**Prevention:**
1. Use CancellationToken pattern for async operations
2. Pass tokens through entire call chain
3. Cancel on view destruction/file switch
4. Check cancellation in tight loops
5. Clean up pending requests on cancel

**Phase:** Performance Optimization

---

## Category: Testing

### P19: No Integration Tests with Real Servers

**Description:** Unit tests with mocked LSP/DAP miss protocol edge cases. Real servers behave differently from spec assumptions.

**Warning Signs:**
- Tests pass but features broken
- Works with one server, fails with another
- Protocol version mismatches in production
- Missing capabilities not detected

**Prevention:**
1. Integration tests with actual rust-analyzer
2. Test against multiple server versions
3. Record/replay server responses for deterministic tests
4. Test error paths (server crash, timeout)
5. CI with real server subprocess

**Phase:** Testing Infrastructure

---

### P20: Untested Error Paths

**Description:** Happy path works, but errors crash or misbehave. File not found, permission denied, network timeout, malformed input - all need handling.

**Warning Signs:**
- Panics in production
- Silent failures with no feedback
- Partial state after error
- Unrecoverable error states

**Prevention:**
1. Use Result<T, E> throughout, avoid panics
2. Test error conditions explicitly
3. Fuzzy test inputs (proptest)
4. Simulate file/network errors in tests
5. Error message review for user-friendliness

**Phase:** Quality Assurance

---

## Summary: Phase Mapping

| Phase | Pitfalls to Address |
|-------|---------------------|
| Foundation Validation | P5, P12 |
| LSP Stabilization | P1, P2, P3 |
| Editor Integration | P4, P11 |
| Syntax Highlighting | P9, P10 |
| Editor Core Hardening | P6, P7 |
| Language Support | P8 |
| Task Integration | P14 |
| Debugger Integration | P15 |
| Performance Optimization | P18 |
| Memory Management | P13 |
| UI Polish | P11 |
| Quality Assurance | P19, P20 |
| Ongoing | P16, P17 |

---

## Known Issues Already in Codebase

From PROJECT.md and codebase analysis:

1. **TOML/Markdown grammars disabled** (P8) - `cc` crate version conflicts
2. **LSP server requests not handled** (P1) - TODO in client.rs:358
3. **No request timeouts** (P2) - identified risk
4. **No graceful server restart** (P3) - identified risk
5. **Problem matcher multi-line issues** (P14) - stateless matcher design

---

*Document generated for Rustide project planning*
*Last updated: 2026-01-27*
