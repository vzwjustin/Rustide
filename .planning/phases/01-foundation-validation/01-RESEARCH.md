# Phase 1: Foundation Validation - Research

**Researched:** 2026-01-27
**Domain:** GPUI Entity System, Async File I/O, Virtual Scrolling
**Confidence:** MEDIUM-HIGH

## Summary

This research investigates how to implement the foundation phase for RustIDE: wiring EditorPane to Document objects, implementing async file loading, and adding virtual scrolling for large files.

The current codebase uses GPUI 0.2, stores file content as raw `String` in `EditorTab`, and renders all lines synchronously. The editor_core crate already has a robust `Document` type backed by `ropey::Rope` that should be used. The key architectural changes needed are:

1. **Entity-based state management**: Replace `EditorTab.content: String` with `Entity<Document>` from GPUI's entity system
2. **Async file loading**: Use GPUI's `cx.spawn` with `background_executor` to load files without blocking the UI
3. **Virtual scrolling**: Use GPUI's built-in `uniform_list` element which only renders visible items

**Primary recommendation:** Use GPUI's native patterns (Entity, cx.spawn, uniform_list) rather than external libraries; the framework provides everything needed.

## Standard Stack

The established libraries/tools for this domain:

### Core (Already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| gpui | 0.2 | UI framework with Entity system, async executor, uniform_list | Powers Zed editor, GPU-accelerated, handles 200K+ lines |
| ropey | 1.6 | Rope data structure for text buffer | O(log n) operations, already used by Buffer |
| tokio | 1.43 | Async file I/O (read_to_string) | Workspace dependency, used by lsp_bridge, agents |

### Supporting (Already Available)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| parking_lot | 0.12 | RwLock for SharedDocument | Thread-safe document access |
| anyhow | 1.0 | Error handling | Async operation errors |
| tracing | 0.1 | Logging | Debug file loading, rendering |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio fs | std::fs + spawn_blocking | tokio already in deps, provides async-native API |
| uniform_list | gpui-component VirtualList | uniform_list is built-in, VirtualList adds external dep |
| Entity<Document> | Arc<RwLock<Document>> | Entity integrates with GPUI's notify/observe system |

**No new dependencies required** - all needed functionality exists in current stack.

## Architecture Patterns

### Recommended State Structure

```
Workspace
  |
  +-- Entity<EditorPane>
        |
        +-- tabs: Vec<EditorTab>
        |     |
        |     +-- document: Entity<Document>  // NOT String content
        |     +-- path: PathBuf
        |     +-- name: String
        |     +-- is_loading: bool  // NEW: async loading state
        |
        +-- scroll_handle: UniformListScrollHandle
        +-- viewport: Viewport  // NEW: visible line range
```

### Pattern 1: Entity-Based Document Storage
**What:** Store `Entity<Document>` instead of raw `String` content in EditorTab
**When to use:** Always - this enables GPUI's reactivity and efficient updates

```rust
// Source: GPUI ownership blog + current codebase analysis
pub struct EditorTab {
    pub document: Entity<Document>,  // Was: pub content: String
    pub path: PathBuf,
    pub name: String,
    pub is_modified: bool,
    pub cursor: (usize, usize),
    pub is_loading: bool,  // NEW
}

impl EditorPane {
    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Create document entity
        let document = cx.new(|_| Document::new());

        let tab = EditorTab {
            document: document.clone(),
            path: path.clone(),
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            is_modified: false,
            cursor: (1, 1),
            is_loading: true,
        };

        self.tabs.push(tab);
        self.active_tab = Some(self.tabs.len() - 1);

        // Spawn async load
        self.load_file_async(document, path, cx);
        cx.notify();
    }
}
```

### Pattern 2: Async File Loading with cx.spawn
**What:** Load files in background, update Entity when complete
**When to use:** Any file operation that could block (files > 1KB)

```rust
// Source: Zed blog "Async Rust" + GPUI docs
impl EditorPane {
    fn load_file_async(
        &mut self,
        document: Entity<Document>,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let tab_index = self.tabs.len() - 1;

        cx.spawn(|this, mut cx| async move {
            // Run file I/O on background executor
            let content = cx.background_executor()
                .spawn(async move {
                    tokio::fs::read_to_string(&path).await
                })
                .await;

            // Update document on main thread
            this.update(&mut cx, |pane, cx| {
                match content {
                    Ok(text) => {
                        document.update(cx, |doc, _| {
                            *doc = Document::from_text(&text);
                        });
                        if let Some(tab) = pane.tabs.get_mut(tab_index) {
                            tab.is_loading = false;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load file: {}", e);
                        // Handle error state
                    }
                }
                cx.notify();
            }).ok();
        }).detach();
    }
}
```

### Pattern 3: Virtual Scrolling with uniform_list
**What:** Only render visible lines using GPUI's uniform_list
**When to use:** Text content with more than ~50 lines

```rust
// Source: GPUI docs.rs + uniform_list example
impl EditorPane {
    fn render_editor_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        if let Some(tab) = self.active_tab() {
            let line_count = tab.document.read(cx).line_count();
            let document = tab.document.clone();

            uniform_list(
                cx.entity().clone(),
                "editor-lines",
                line_count,
                move |_this, visible_range, _window, cx| {
                    let doc = document.read(cx);
                    visible_range
                        .map(|line_idx| {
                            let line_text = doc.line(line_idx)
                                .unwrap_or_default();
                            render_line(line_idx, &line_text, &theme)
                        })
                        .collect()
                },
            )
            .track_scroll(&self.scroll_handle)
            .flex_1()
            .into_any_element()
        } else {
            // Empty state
            div().child("No file open").into_any_element()
        }
    }
}

fn render_line(line_idx: usize, text: &str, theme: &Theme) -> impl IntoElement {
    div()
        .h(px(LINE_HEIGHT))  // Must be uniform!
        .flex()
        .child(
            div()
                .w(px(40.0))
                .text_right()
                .pr(px(12.0))
                .text_color(theme.text.line_number)
                .child(format!("{}", line_idx + 1))
        )
        .child(
            div()
                .flex_1()
                .text_color(theme.text.primary)
                .child(if text.is_empty() { " " } else { text })
        )
}
```

### Anti-Patterns to Avoid
- **Storing String content directly**: Loses reactivity, duplicates buffer
- **Synchronous file I/O in open_file()**: Freezes UI, the exact pitfall P5 warns against
- **Rendering all lines**: Current `.children(lines.iter().enumerate().map(...))` pattern
- **Using std::fs::read_to_string without spawn**: Blocks main thread

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Virtual scrolling | Custom viewport tracking | `uniform_list` | Handles scroll position, visible range, item recycling |
| Async UI updates | Manual channel/mutex | `cx.spawn` + `entity.update` | Integrates with GPUI's update cycle, thread-safe |
| Text storage | Vec<String> or String | `Document` with `Buffer` (ropey) | O(log n) edits, line indexing, undo history |
| Entity reactivity | Manual RefCell/Rc | `Entity<T>` + `cx.notify()` | Framework handles observer notifications |
| File I/O | Custom thread pool | `cx.background_executor()` | Integrated with platform event loop |

**Key insight:** GPUI provides a complete async-to-UI bridge. Using external patterns (channels, Arc<Mutex>) bypasses the framework's optimizations and complicates state management.

## Common Pitfalls

### Pitfall 1: Blocking the Main Thread (P5)
**What goes wrong:** UI freezes when opening large files
**Why it happens:** `std::fs::read_to_string` in `open_file()` runs synchronously on main thread
**How to avoid:** Always use `cx.spawn` + `background_executor` for file I/O
**Warning signs:** UI becomes unresponsive during file operations; 100K+ line files take visible time to open

### Pitfall 2: Rendering All Lines (P12)
**What goes wrong:** Slow/frozen UI when scrolling large files
**Why it happens:** Current `lines.iter().enumerate().map(...)` creates element for every line
**How to avoid:** Use `uniform_list` which only renders visible range
**Warning signs:** Frame drops when scrolling; memory usage proportional to file size

### Pitfall 3: Non-Uniform Line Heights with uniform_list
**What goes wrong:** Lines overlap or have gaps; scroll position jumps
**Why it happens:** uniform_list assumes all items have identical height
**How to avoid:** Set explicit `.h(px(LINE_HEIGHT))` on every line; use fixed line height (e.g., 20.0)
**Warning signs:** Visual glitches when scrolling; items not aligned to grid

### Pitfall 4: Stale Entity References in Async Closures
**What goes wrong:** Entity was dropped before async task completes
**Why it happens:** Entity handle captured in closure, but tab closed before load finishes
**How to avoid:** Use `this.update(&mut cx, ...)` pattern which returns `Result`; handle `Err` case
**Warning signs:** Panic on `.unwrap()` after closing tab quickly; "entity not found" errors

### Pitfall 5: Forgetting cx.notify() After Updates
**What goes wrong:** UI doesn't reflect new state after async operation
**Why it happens:** GPUI only re-renders when notified of changes
**How to avoid:** Always call `cx.notify()` after modifying state in update closures
**Warning signs:** File appears empty after loading; must resize window to see content

## Code Examples

### Complete Async File Loading Pattern
```rust
// Source: Verified from GPUI docs + Zed blog patterns

impl EditorPane {
    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // 1. Check if already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(idx);
            cx.notify();
            return;
        }

        // 2. Create placeholder document
        let document = cx.new(|_| Document::new());

        // 3. Create tab in loading state
        let tab = EditorTab {
            document: document.clone(),
            path: path.clone(),
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "untitled".to_string()),
            is_modified: false,
            cursor: (1, 1),
            is_loading: true,
        };

        self.tabs.push(tab);
        self.active_tab = Some(self.tabs.len() - 1);
        cx.notify();  // Show loading state immediately

        // 4. Spawn async loading task
        let tab_index = self.tabs.len() - 1;
        cx.spawn(|this, mut cx| async move {
            // Read file on background thread
            let result = cx.background_executor()
                .spawn(async move {
                    tokio::fs::read_to_string(&path).await
                })
                .await;

            // Update on main thread
            let _ = this.update(&mut cx, |pane, cx| {
                match result {
                    Ok(content) => {
                        // Update document with loaded content
                        document.update(cx, |doc, _| {
                            *doc = Document::from_text(&content);
                        });

                        // Mark loading complete
                        if let Some(tab) = pane.tabs.get_mut(tab_index) {
                            tab.is_loading = false;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load file: {}", e);
                        // Could set error state on tab here
                    }
                }
                cx.notify();
            });
        }).detach();
    }
}
```

### Virtual Scrolling Editor Content
```rust
// Source: GPUI uniform_list docs + verified patterns

const LINE_HEIGHT: f32 = 20.0;  // Must be constant for uniform_list

impl EditorPane {
    fn render_editor_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        match self.active_tab() {
            Some(tab) if !tab.is_loading => {
                let doc = tab.document.clone();
                let line_count = doc.read(cx).line_count();

                div()
                    .id("editor-content")
                    .flex_1()
                    .overflow_hidden()  // Required for uniform_list
                    .bg(theme.background.editor)
                    .font_family(theme.fonts.mono_family.clone())
                    .text_size(px(theme.fonts.mono_size))
                    .child(
                        uniform_list(
                            cx.entity().clone(),
                            "editor-lines",
                            line_count,
                            move |_view, visible_range, _window, cx| {
                                let document = doc.read(cx);
                                visible_range
                                    .map(|idx| Self::render_line(idx, &document, &theme))
                                    .collect()
                            },
                        )
                        .track_scroll(&self.scroll_handle)
                    )
            }
            Some(tab) if tab.is_loading => {
                div()
                    .id("loading")
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.background.editor)
                    .child("Loading...")
            }
            _ => {
                div()
                    .id("empty")
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.background.editor)
                    .child("No file open")
            }
        }
    }

    fn render_line(idx: usize, doc: &Document, theme: &Theme) -> impl IntoElement {
        let line_text = doc.line(idx).unwrap_or_default();

        div()
            .h(px(LINE_HEIGHT))  // CRITICAL: uniform height
            .w_full()
            .flex()
            .child(
                // Line number gutter
                div()
                    .w(px(50.0))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pr(px(12.0))
                    .text_color(theme.text.line_number)
                    .child(SharedString::from(format!("{}", idx + 1)))
            )
            .child(
                // Line content
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .text_color(theme.text.primary)
                    .child(SharedString::from(
                        if line_text.trim_end_matches('\n').is_empty() {
                            " ".to_string()
                        } else {
                            line_text.trim_end_matches('\n').to_string()
                        }
                    ))
            )
    }
}
```

### Updated EditorTab Structure
```rust
// Source: Current codebase + GPUI Entity patterns

use gpui::Entity;
use editor_core::Document;

#[derive(Clone)]
pub struct EditorTab {
    /// Document entity (replaces String content)
    pub document: Entity<Document>,
    /// File path
    pub path: PathBuf,
    /// Display name (filename)
    pub name: String,
    /// Whether the file has unsaved changes
    pub is_modified: bool,
    /// Cursor position (line, column)
    pub cursor: (usize, usize),
    /// Whether file is currently being loaded
    pub is_loading: bool,
}

impl EditorTab {
    pub fn new(path: PathBuf, document: Entity<Document>) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        Self {
            document,
            path,
            name,
            is_modified: false,
            cursor: (1, 1),
            is_loading: true,
        }
    }
}

pub struct EditorPane {
    focus_handle: FocusHandle,
    tabs: Vec<EditorTab>,
    active_tab: Option<usize>,
    scroll_handle: UniformListScrollHandle,  // NEW
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| String content storage | Entity<Document> | GPUI 0.2+ | Reactive updates, shared state |
| std::fs sync I/O | cx.spawn + background_executor | GPUI design | Non-blocking UI |
| Render all items | uniform_list | GPUI design | O(visible) vs O(total) |
| Arc<Mutex<T>> for state | Entity<T> | GPUI design | Framework-managed lifecycle |

**Deprecated/outdated:**
- `Arc<RwLock<Document>>` pattern: Still works but bypasses GPUI's observer system
- Storing content as `String`: Duplicates buffer, loses undo history
- Custom scroll virtualization: Framework handles it better

## Open Questions

1. **tokio runtime coexistence with GPUI**
   - What we know: GPUI has its own async executor; Zed uses platform GCD, not tokio
   - What's unclear: Whether calling `tokio::fs` from background_executor has issues
   - Recommendation: Test with simple file reads first; fall back to `std::fs` + spawn_blocking if needed

2. **uniform_list item recycling**
   - What we know: uniform_list only renders visible items
   - What's unclear: Whether item elements are recycled or recreated on scroll
   - Recommendation: Keep render_line lightweight; measure performance with 100K lines

3. **Document entity lifecycle when closing tabs**
   - What we know: Entity dropped when all handles released
   - What's unclear: If async load task holds handle, does it keep document alive?
   - Recommendation: Handle `this.update()` returning `Err` gracefully

## Sources

### Primary (HIGH confidence)
- [GPUI docs.rs](https://docs.rs/gpui) - uniform_list, Entity<T>, Context types
- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) - Framework overview
- [Zed Blog: GPUI Ownership](https://zed.dev/blog/gpui-ownership) - Entity/Context patterns
- [Zed Blog: Async Rust](https://zed.dev/blog/zed-decoded-async-rust) - cx.spawn patterns

### Secondary (MEDIUM confidence)
- [GPUI Component VirtualList](https://longbridge.github.io/gpui-component/docs/components/virtual-list) - Virtual scrolling patterns
- [gpui.rs examples](https://www.gpui.rs/) - Basic usage patterns
- Existing RustIDE codebase - Current implementation context

### Tertiary (LOW confidence)
- [Tokio bridging docs](https://tokio.rs/tokio/topics/bridging) - GUI + tokio patterns (not GPUI-specific)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Using existing deps, no new additions needed
- Entity/async patterns: HIGH - Verified from Zed blog posts and GPUI docs
- Virtual scrolling: MEDIUM-HIGH - uniform_list documented but examples sparse
- tokio integration: MEDIUM - Not officially documented with GPUI

**Research date:** 2026-01-27
**Valid until:** 2026-02-27 (GPUI is relatively stable at 0.2)
