# Rustide Stack Research

**Date:** 2025-01-27
**Context:** Brownfield analysis for completing IDE foundation
**Existing Stack:** GPUI 0.2, tree-sitter 0.22, tokio 1.43, ropey 1.6, lsp-types 0.97

---

## Executive Summary

Rustide has solid foundations but is missing critical patterns for a production code editor. The main gaps are:
1. **Text layout engine** - No glyph shaping, font fallback, or bidirectional text
2. **Input handling** - Basic key bindings exist but missing IME, dead keys, compose sequences
3. **Virtual scrolling** - Current editor renders all lines (will not scale)
4. **Incremental rendering** - No damage tracking or partial repaints
5. **Text metrics** - No line height calculation, character width measurement

---

## Recommended Additions

### 1. Text Layout & Shaping

**Current state:** GPUI provides basic text rendering but the editor_pane.rs shows direct line-by-line rendering without proper text shaping.

#### Option A: cosmic-text (Recommended)
- **Version:** 0.12.x (latest stable)
- **Confidence:** HIGH
- **Why:**
  - Battle-tested in cosmic-term and other Linux desktop apps
  - Pure Rust, no system dependencies
  - Built-in font fallback, shaping via swash, BiDi support
  - Designed for editor use cases (cacheable layout)
- **Integration:** Wraps swash internally, provides high-level API
- **Trade-off:** Adds ~2MB to binary, but worth it for correctness

```toml
cosmic-text = "0.12"
```

#### Option B: swash (Lower-level alternative)
- **Version:** 0.1.x
- **Confidence:** MEDIUM-HIGH
- **Why:** What Zed uses internally for text shaping
- **When to use:** If you need finer control or cosmic-text doesn't integrate well with GPUI
- **Trade-off:** More code to write, but more control

```toml
swash = "0.1"
```

**Recommendation:** Start with cosmic-text. If GPUI integration proves difficult, drop to swash.

---

### 2. Input Method Editor (IME) Support

**Current state:** Key bindings in app.rs use GPUI's `KeyBinding` but no IME handling visible.

**What's needed:**
- GPUI 0.2 should have IME support via platform layer
- Verify IME events are propagated to editor
- Handle composition state (underline, cursor during input)

**No additional crate needed** - this is a GPUI integration pattern, not a library gap.

**Confidence:** HIGH (GPUI handles platform IME)

**Pattern to implement:**
```rust
// In editor component
fn handle_ime_composition(&mut self, text: &str, range: Option<Range<usize>>) {
    // Show composition preview inline
    // Handle commit when composition ends
}
```

---

### 3. Virtual Scrolling / Viewport Management

**Current state:** `editor_pane.rs` iterates all lines:
```rust
.children(lines.iter().enumerate().map(|(i, line)| { ... }))
```

This will crash or freeze on large files (100K+ lines).

**Solution:** Implement viewport-aware rendering

**No crate needed** - this is an architectural pattern:

```rust
struct ViewportState {
    first_visible_line: usize,
    visible_line_count: usize,
    scroll_offset_px: f32,
    line_height: f32,
}

// Only render visible lines + small buffer
fn visible_lines(&self, viewport: &ViewportState) -> impl Iterator<Item = (usize, &str)> {
    let buffer_lines = 5; // overscan for smooth scrolling
    let start = viewport.first_visible_line.saturating_sub(buffer_lines);
    let end = (viewport.first_visible_line + viewport.visible_line_count + buffer_lines)
        .min(self.line_count());
    // ...
}
```

**Confidence:** HIGH (standard pattern, no external deps)

---

### 4. Sum Tree / Interval Tree for Syntax + Diagnostics

**Current state:** Languages crate has highlighting but no efficient range querying.

**What's needed:** Fast range queries for:
- Syntax highlighting spans
- Diagnostic underlines
- Git diff markers
- Fold regions

#### Recommended: Custom sum tree (like Zed's)
- **Confidence:** MEDIUM
- **Why:** Generic B-tree with monoidal annotations
- **Trade-off:** Significant implementation effort (~500-1000 LOC)

#### Alternative: intervaltree
- **Version:** 0.2.x
- **Confidence:** MEDIUM
- **Why:** Simpler, less performant, but works
- **Trade-off:** O(n + k) vs O(log n + k) for queries

```toml
intervaltree = "0.2"  # Only if custom sum tree is too much
```

**Recommendation:** Start with Vec + binary search for MVP, plan sum tree for v0.2.

---

### 5. Async File System Operations

**Current state:** `editor_pane.rs` uses sync `std::fs::read_to_string`.

**What's needed:**
- Async file loading (don't block UI on large files)
- File watching already uses `notify 7.0` (good)
- Need async file saving with atomic writes

**No new crate needed** - use existing tokio:

```rust
// Already have tokio, just need to use it properly
async fn load_file(path: &Path) -> Result<String> {
    tokio::fs::read_to_string(path).await
}

async fn save_file(path: &Path, content: &str) -> Result<()> {
    let temp = path.with_extension("tmp");
    tokio::fs::write(&temp, content).await?;
    tokio::fs::rename(&temp, path).await?;
    Ok(())
}
```

**Confidence:** HIGH

---

### 6. Clipboard Integration

**Current state:** No clipboard crate visible in Cargo.toml.

**Recommended:** arboard
- **Version:** 3.4.x
- **Confidence:** HIGH
- **Why:** Cross-platform, actively maintained, supports images

```toml
arboard = "3.4"
```

**Alternative:** GPUI may have built-in clipboard - verify before adding.

---

### 7. Settings/Configuration System

**Current state:** Theme is hardcoded, no user settings visible.

**What's needed:**
- User settings file (JSON/TOML)
- Schema validation
- Live reload on change
- Default settings with override

**Recommended pattern:**

```rust
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub editor: EditorSettings,
    pub ui: UiSettings,
    pub keybindings: Vec<KeybindingOverride>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub tab_size: usize,
    pub insert_spaces: bool,
    pub word_wrap: WordWrap,
}
```

**Crate needed:** schemars (for JSON schema generation)
- **Version:** 0.8.x
- **Confidence:** MEDIUM (nice-to-have, not blocking)

```toml
schemars = { version = "0.8", features = ["derive"] }
```

---

## Patterns Missing (No Crate Needed)

### 1. Damage Tracking / Incremental Repaint

**Current state:** Full repaint on every change (GPUI handles this, but editor should minimize work).

**Pattern:**
```rust
struct DamageRegion {
    lines: Range<usize>,  // Dirty line range
    full_repaint: bool,   // Scroll, resize, etc.
}

impl Editor {
    fn mark_lines_dirty(&mut self, lines: Range<usize>) {
        self.damage.extend(lines);
    }
}
```

### 2. Edit Transaction Coalescing

**Current state:** History exists but may create too many undo stops.

**Pattern:** Group rapid edits (typing) into single undo unit:
```rust
const COALESCE_TIMEOUT_MS: u64 = 300;

fn should_coalesce(&self, edit: &Edit) -> bool {
    if let Some(last) = self.pending_transaction.last() {
        let time_delta = edit.timestamp - last.timestamp;
        time_delta < Duration::from_millis(COALESCE_TIMEOUT_MS)
            && edit.is_adjacent_to(last)
    } else {
        false
    }
}
```

### 3. Line Cache with Invalidation

**Pattern for syntax highlighting:**
```rust
struct LineCache {
    lines: Vec<Option<CachedLine>>,
    version: u64,  // Increment on edit
}

struct CachedLine {
    text_hash: u64,
    highlights: Vec<Highlight>,
    computed_at_version: u64,
}
```

### 4. Async LSP Request Debouncing

**Current state:** LSP client exists but may fire too many requests.

**Pattern:**
```rust
struct DebouncedRequest<T> {
    pending: Option<(Instant, T)>,
    delay: Duration,
}

impl<T> DebouncedRequest<T> {
    fn request(&mut self, value: T) -> Option<T> {
        let now = Instant::now();
        if let Some((scheduled, _)) = &self.pending {
            if now < *scheduled {
                self.pending = Some((now + self.delay, value));
                return None;
            }
        }
        self.pending = Some((now + self.delay, value));
        // Return after delay via timer
        None
    }
}
```

---

## Keep As-Is (No Changes Needed)

| Component | Why Keep |
|-----------|----------|
| **GPUI 0.2** | Core framework, no alternative |
| **tree-sitter 0.22** | Current stable, 0.23 not mature enough |
| **ropey 1.6** | Best rope implementation for Rust |
| **lsp-types 0.97** | Matches LSP spec, no newer needed |
| **tokio 1.43** | Latest stable, full features |
| **notify 7.0** | Latest stable file watcher |
| **git2 0.19** | Stable libgit2 bindings |
| **parking_lot 0.12** | Faster than std mutexes |
| **fuzzy-matcher 0.3** | Good enough for command palette |

---

## What NOT to Add

| Library | Why Skip |
|---------|----------|
| **xi-rope** | Abandoned, ropey is better |
| **druid/iced** | Have GPUI, don't mix frameworks |
| **syntect** | Have tree-sitter, don't duplicate |
| **pulldown-cmark** | Tree-sitter-md handles markdown |
| **clipboard** (crate) | Use arboard instead (better maintained) |
| **neon/pyo3** | No scripting layer needed yet |
| **mlua** | Lua scripting adds complexity, defer to v0.3+ |
| **winit** | GPUI handles windowing |
| **wgpu** | GPUI handles GPU, don't bypass |

---

## Version Verification Notes

> **Note:** Versions above are based on knowledge as of early 2025. Before adding dependencies, verify latest stable versions on crates.io.

Key version checks needed:
- [ ] cosmic-text: Verify 0.12.x is latest stable
- [ ] arboard: Verify 3.4.x is latest stable
- [ ] schemars: Verify 0.8.x is latest stable
- [ ] intervaltree: Verify 0.2.x is still maintained

---

## Priority Order for Implementation

1. **P0 (Blocking):** Virtual scrolling - current impl won't scale
2. **P0 (Blocking):** Text layout/shaping - needed for non-ASCII text
3. **P1 (Important):** Clipboard - basic editor functionality
4. **P1 (Important):** Async file I/O - UX for large files
5. **P2 (Polish):** Settings system - user customization
6. **P2 (Polish):** Sum tree - performance at scale
7. **P3 (Future):** IME - international input support

---

## Confidence Summary

| Recommendation | Confidence | Reasoning |
|----------------|------------|-----------|
| cosmic-text for text layout | HIGH | Standard solution, battle-tested |
| Virtual scrolling pattern | HIGH | Universal editor pattern |
| arboard for clipboard | HIGH | Clear best choice |
| Async file I/O pattern | HIGH | Already have tokio |
| Settings with schemars | MEDIUM | Nice-to-have, not critical |
| Custom sum tree | MEDIUM | High effort, can defer |
| intervaltree as fallback | MEDIUM | Works but not optimal |
