# Coding Conventions

**Analysis Date:** 2026-01-27

## Naming Patterns

**Files:**
- Snake case for module files: `cursor.rs`, `problem_matcher.rs`, `breakpoints.rs`
- Example: `crates/editor_core/src/buffer.rs`, `crates/lsp_bridge/src/client.rs`

**Functions:**
- Snake case for all function names, including public APIs
- Example: `test_buffer_basic_operations()`, `with_condition()`, `set_state()`
- Builder pattern uses `with_*` prefix for chainable setters: `with_condition()`, `with_location()`, `with_code()`

**Types/Structs:**
- PascalCase for all struct names
- Example: `Buffer`, `Cursor`, `Point`, `Selection`, `Document`, `BreakpointManager`, `RustProblemMatcher`
- Enum types in PascalCase: `BreakpointState`, `ProblemSeverity`, `SessionState`, `StoppedReason`

**Variables:**
- Snake case for all variable names and field names
- Example: `rope`, `history`, `dirty`, `error_pattern`, `current_problem`

**Constants/Enums:**
- PascalCase for enum variants: `Pending`, `Verified`, `Disabled`, `Error`, `Warning`
- Struct field visibility uses explicit `pub` keyword

## Code Style

**Formatting:**
- Rust default formatting (enforced by rustfmt)
- 4-space indentation (Rust standard)
- No line length restrictions visible (tools default to 100 chars for rustfmt)

**Linting:**
- Clippy enabled in rust-toolchain.toml
- Standard Rust warnings enforced
- No custom clippy configurations visible

## Import Organization

**Order:**
1. Standard library imports (`use std::...`)
2. Third-party crate imports
3. Crate-local imports (`use crate::...`)

**Examples from codebase:**

`crates/editor_core/src/document.rs`:
```rust
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::buffer::Buffer;
use crate::cursor::{CursorSet, Point};
use crate::edit::{Edit, EditBatch, TextChange};
use crate::{EditorError, Result};
```

`crates/dap_bridge/src/breakpoints.rs`:
```rust
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};
```

**Path Aliases:**
- No path aliases configured
- Direct relative imports from crate root using `use crate::`

## Error Handling

**Patterns:**
- Custom error types using `thiserror` crate for domain-specific errors
- `anyhow::Result<T>` for propagating errors with context
- Custom `Result<T>` type aliases at module level for convenience

**Examples:**

From `crates/editor_core/src/lib.rs`:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Position out of bounds: line {line}, column {column}")]
    PositionOutOfBounds { line: usize, column: usize },

    #[error("Byte offset out of bounds: {offset} (buffer length: {length})")]
    OffsetOutOfBounds { offset: usize, length: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EditorError>;
```

From `crates/lsp_bridge/src/client.rs`:
```rust
#[derive(Debug, Error)]
pub enum LspClientError {
    #[error("Failed to spawn server process: {0}")]
    SpawnError(String),

    #[error("Server process exited unexpectedly")]
    ServerExited,
}
```

- Error types derive `Debug`, `Error`, and implement `Display` via `#[error(...)]` attributes
- Constructor functions return custom `Result<T>` types
- Question mark operator (`?`) used to propagate errors in async functions

## Logging

**Framework:** Tracing crate (`tracing` and `tracing-subscriber`)

**Patterns:**
- Macros: `debug!()`, `info!()`, `warn!()`, `error!()`
- Contextual logging with format strings
- Log statements at key lifecycle points

**Examples from codebase:**

From `crates/dap_bridge/src/breakpoints.rs`:
```rust
use tracing::{debug, info};

info!("Adding breakpoint {} at {}:{}", id, file.display(), line);
```

From `crates/dap_bridge/src/session.rs`:
```rust
use tracing::{debug, error, info, warn};

info!("Starting debug session");
info!("Launching program: {}", program);
info!("Attaching to process: {}", pid);
```

From `crates/lsp_bridge/src/client.rs`:
```rust
use tracing::{debug, error, info, trace, warn};
```

## Comments

**When to Comment:**
- Module-level documentation using `//!` doc comments (see examples below)
- Type-level documentation for public structs, enums, traits
- No inline code comments for obvious logic
- Comments explain "why", not "what"

**Documentation Pattern:**
- All public modules include documentation comments at top
- Each public type includes a doc comment line before definition
- Field documentation for struct fields (especially in config/data structs)

**Examples:**

From `crates/editor_core/src/lib.rs`:
```rust
//! Core text editing engine for RustIDE.
//!
//! This crate provides the fundamental text editing primitives:
//! - [`Buffer`]: Rope-backed text storage with efficient insert/delete
//! - [`Cursor`] and [`Selection`]: Cursor positioning and text selection
//! - [`Document`]: Complete document model with file I/O
//! - [`History`]: Undo/redo transaction management
//! - [`Edit`]: Atomic edit operations
```

From `crates/lsp_bridge/src/client.rs`:
```rust
//! LSP client for communicating with language servers.
//!
//! This module provides the core client implementation for spawning LSP server
//! processes and handling JSON-RPC message communication.
```

From `crates/editor_core/src/document.rs`:
```rust
/// A point in the text buffer, represented as line and column.
///
/// Both line and column are 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    /// The line number (0-indexed).
    pub line: usize,
    /// The column number (0-indexed, in characters).
    pub column: usize,
}
```

From `crates/editor_core/src/edit.rs`:
```rust
/// An atomic edit operation.
///
/// Represents a single change to the text buffer that can be undone/redone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    /// The kind of edit operation.
    pub kind: EditKind,
    /// The byte offset where the edit occurs.
    pub offset: usize,
    /// The old text that was replaced/deleted (empty for inserts).
    pub old_text: String,
    /// The new text that was inserted/replaced (empty for deletes).
    pub new_text: String,
}
```

## Function Design

**Size:** Functions are generally focused and small (10-30 lines typical)

**Parameters:**
- Owned values (`String`, `Vec<T>`) for configuration/construction
- References for temporary usage: `&str`, `&Path`, `&[T]`
- Use of builder pattern with `impl Into<String>` for flexibility

**Examples:**

Simple constructor:
```rust
// crates/dap_bridge/src/breakpoints.rs
pub fn new(id: u64, file: PathBuf, line: u32) -> Self {
    Self {
        id,
        file,
        line,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
        state: BreakpointState::Pending,
        server_id: None,
    }
}
```

Builder pattern:
```rust
// crates/dap_bridge/src/breakpoints.rs
pub fn with_condition(mut self, condition: &str) -> Self {
    self.condition = Some(condition.to_string());
    self
}
```

Query function:
```rust
// crates/editor_core/src/cursor.rs
pub fn is_empty(&self) -> bool {
    self.is_collapsed()
}
```

**Return Values:**
- Use of `Option<T>` for nullable values: `with_condition()` returns `Option<String>`
- Use of `Result<T, E>` for fallible operations
- Direct return of calculated values for simple queries

## Module Design

**Exports:**
- All public types re-exported at crate level in `lib.rs`
- Clear separation of public API from internal implementation

**Examples:**

From `crates/editor_core/src/lib.rs`:
```rust
pub mod buffer;
pub mod cursor;
pub mod document;
pub mod edit;
pub mod history;

pub use buffer::Buffer;
pub use cursor::{Cursor, Point, Selection};
pub use document::Document;
pub use edit::{Edit, EditKind};
pub use history::{History, Transaction};
```

From `crates/gitx/src/lib.rs`:
```rust
pub mod commit;
pub mod diff;
pub mod repo;
pub mod status;

pub use commit::{CommitBuilder, CommitInfo};
pub use diff::{DiffHunk, DiffLine, FileDiff, LineChange};
pub use repo::{GitError, Repository};
pub use status::{FileStatus, StatusEntry, WorktreeStatus};
```

**Barrel Files:**
- Standard barrel exports in `lib.rs` re-exporting public types
- Allows clean imports: `use editor_core::{Buffer, Document, Edit}`

## Derive Attributes

**Standard Derives:**
- `Debug` on all public types
- `Clone` for copyable data structures (with `Copy` where appropriate)
- `Serialize, Deserialize` from `serde` for data that serializes
- `PartialEq, Eq` for types that should be comparable
- `Hash` for types used as map keys

**Examples:**
```rust
// crates/dap_bridge/src/breakpoints.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakpointState {
    Pending,
    Verified,
    Disabled,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: u64,
    // ...
}

// crates/editor_core/src/cursor.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}
```

---

*Convention analysis: 2026-01-27*
