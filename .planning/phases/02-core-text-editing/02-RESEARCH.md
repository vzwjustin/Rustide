# Phase 2: Core Text Editing - Research

**Researched:** 2026-01-28
**Domain:** GPUI input events, cursor rendering, editor_core integration
**Confidence:** MEDIUM

## Summary

Phase 2 must wire GPUI input events into the existing editor_core Document and render
the primary cursor with a visible blink. The codebase already has text editing
primitives (Document insert/delete, CursorSet, Selection), but the UI layer does
not handle keyboard input or cursor rendering. EditorPane currently renders lines
via uniform_list and shows a status bar based on an EditorTab cursor tuple that is
not connected to Document state.

The main integration points are:
1. Capture text input and key events in EditorPane.
2. Apply edits through Document::insert, delete_backward, and delete_forward.
3. Render the cursor at the Document primary cursor position, with a blink timer.
4. Align status bar line/column with Document's 0-indexed cursor positions.

## Standard Stack

### Core (Already in Workspace)
| Library | Purpose | Evidence |
|---------|---------|----------|
| gpui | Input events, focus, rendering, timers | crates/ui_shell/src/editor_pane.rs |
| editor_core | Document, Buffer, Cursor, Selection | crates/editor_core/src/document.rs |
| ropey | Rope-based text storage | crates/editor_core/src/buffer.rs |

### Supporting (Already Available)
| Library | Purpose | Evidence |
|---------|---------|----------|
| tracing | Logging for input/edit diagnostics | crates/ui_shell/src/editor_pane.rs |

## Architecture Patterns

### Pattern 1: Document cursor is the source of truth
Use Document::cursors().primary().position() to drive rendering and status bar
display. Avoid storing a separate cursor tuple in EditorTab.

### Pattern 2: Input handlers call Document edit APIs
Keyboard text input should call Document::insert, while Backspace/Delete should
call delete_backward/delete_forward. These methods already update cursor position
internally.

### Pattern 3: Cursor rendering in render_line()
Render the cursor by splitting the line string at the cursor column and inserting
a small caret element. This keeps layout driven by text elements without requiring
manual pixel math.

### Pattern 4: Blink state toggled by timer
Maintain a cursor_visible bool in EditorPane and toggle it on a timer while the
pane is focused. The blink should restart on text input to reduce flicker.

## Anti-Patterns to Avoid

- Editing by replacing the rendered line strings instead of updating Document.
- Maintaining a UI cursor that can drift from Document cursor state.
- Forgetting cx.notify() after document edits (UI will not refresh).
- Using byte offsets for cursor placement without converting to line/column.

## Open Questions

1. GPUI input APIs: which event types provide text input vs keydown for Backspace,
   Delete, and Enter?
2. GPUI timer or animation API: preferred way to toggle cursor_visible every 500ms.
3. Cursor placement: best way to split and render text while keeping monospace
   alignment consistent with GPUI text layout.
4. IME composition support in GPUI 0.2 (CJK input) needs validation.

## Sources

- crates/ui_shell/src/editor_pane.rs (render_line, render_editor_content)
- crates/editor_core/src/document.rs (insert/delete/edit APIs, cursor set)
- crates/editor_core/src/cursor.rs (Point, Selection, CursorSet)
- crates/editor_core/src/buffer.rs (line_len, point_to_offset)
- .planning/research/FEATURES.md (text editing fundamentals)
- .planning/STATE.md (IME support concern)

## Metadata

**Confidence breakdown:**
- Document edit APIs: HIGH (verified in code)
- UI input event wiring: MEDIUM (GPUI API details not yet mapped)
- Cursor rendering approach: MEDIUM (needs GPUI layout confirmation)
- Blink timer: MEDIUM (needs GPUI timer API confirmation)

**Research date:** 2026-01-28
