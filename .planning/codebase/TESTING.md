# Testing Patterns

**Analysis Date:** 2026-01-27

## Test Framework

**Runner:**
- Rust built-in test framework (no external test runner)
- Compile with: `cargo test`
- Test discovery: Rust compiler automatically finds `#[test]` functions

**Assertion Library:**
- Rust standard library `assert!`, `assert_eq!`, `assert_ne!` macros
- No external assertion library (standard Rust assertions used throughout)

**Run Commands:**
```bash
cargo test                    # Run all tests
cargo test -- --test-threads=1   # Run tests serially
cargo test -- --nocapture    # Show println! output during tests
cargo test crate_name         # Run tests for specific crate
cargo test test_function_name # Run specific test function
```

## Test File Organization

**Location:**
- Tests are co-located with implementation code in the same file
- Pattern: Each module has `#[cfg(test)] mod tests { ... }` at bottom

**Naming:**
- Test functions follow pattern: `test_<feature_being_tested>()`
- Example test names: `test_buffer_basic_operations()`, `test_rust_error_match()`, `test_add_breakpoint()`

**Structure:**
```
crates/
├── editor_core/src/
│   ├── lib.rs              # Contains tests at bottom
│   ├── buffer.rs           # Implementation only
│   ├── cursor.rs           # Implementation only
│   ├── document.rs         # Implementation only
│   └── ...
├── dap_bridge/src/
│   ├── lib.rs
│   ├── breakpoints.rs      # Contains tests at bottom
│   ├── client.rs           # Contains tests at bottom
│   └── session.rs          # Contains tests at bottom
├── tasks/src/
│   ├── problem_matcher.rs  # Contains tests at bottom
│   └── runner.rs           # Contains tests at bottom
└── ...
```

## Test Structure

**Suite Organization:**

Tests are organized in a `#[cfg(test)] mod tests` block at the end of each file. The block imports the module's public interface via `use super::*;`

```rust
// From crates/editor_core/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_basic_operations() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "Hello, World!").unwrap();
        assert_eq!(buffer.text(), "Hello, World!");
        assert_eq!(buffer.len_chars(), 13);
        assert_eq!(buffer.len_lines(), 1);
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert!(!doc.is_dirty());
        assert!(doc.path().is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let cursor = Cursor::new(Point::new(0, 0));
        assert_eq!(cursor.position().line, 0);
        assert_eq!(cursor.position().column, 0);
    }
}
```

**Patterns:**

1. **Setup Pattern:** Direct instantiation of objects being tested
   ```rust
   let mut buffer = Buffer::new();
   let manager = BreakpointManager::new();
   let doc = Document::new();
   ```

2. **Teardown Pattern:** Rust automatic cleanup via scope/drop; no explicit teardown needed

3. **Assertion Pattern:** Standard Rust assertions with equality checks
   ```rust
   assert_eq!(buffer.text(), "Hello, World!");
   assert!(matches!(bp.state, BreakpointState::Pending));
   assert!(problem.is_error());
   ```

## Test Examples

**Simple Unit Test:**

From `crates/dap_bridge/src/breakpoints.rs`:
```rust
#[test]
fn test_add_breakpoint() {
    let manager = BreakpointManager::new();
    let bp = manager.add(PathBuf::from("/test/file.rs"), 10);

    assert_eq!(bp.line, 10);
    assert_eq!(bp.file, PathBuf::from("/test/file.rs"));
    assert!(matches!(bp.state, BreakpointState::Pending));
}
```

**Stateful Test with Multiple Operations:**

From `crates/dap_bridge/src/breakpoints.rs`:
```rust
#[test]
fn test_toggle_breakpoint() {
    let manager = BreakpointManager::new();
    let file = PathBuf::from("/test/file.rs");

    // Add
    let bp = manager.toggle(file.clone(), 10);
    assert!(bp.is_some());

    // Toggle to remove
    let bp = manager.toggle(file.clone(), 10);
    assert!(bp.is_none());
}
```

**Regex-based Pattern Matching Test:**

From `crates/tasks/src/problem_matcher.rs`:
```rust
#[test]
fn test_rust_error_match() {
    let matcher = RustProblemMatcher::new();
    let line = "error[E0382]: borrow of moved value: `x`";

    let problem = matcher.match_line(line).unwrap();
    assert!(problem.is_error());
    assert_eq!(problem.code, Some("E0382".to_string()));
    assert!(problem.message.contains("borrow of moved value"));
}
```

**Builder Pattern Test:**

From `crates/tasks/src/problem_matcher.rs`:
```rust
#[test]
fn test_problem_builder() {
    let problem = Problem::error("test error")
        .with_location(PathBuf::from("test.rs"), 10, Some(5))
        .with_code("E0001")
        .with_source("rustc");

    assert!(problem.is_error());
    assert_eq!(problem.file, Some(PathBuf::from("test.rs")));
    assert_eq!(problem.line, Some(10));
    assert_eq!(problem.code, Some("E0001".to_string()));
}
```

## Mocking

**Approach:** No external mocking framework used; instead, tests use concrete implementations or simple test doubles

**Test Doubles Strategy:**
- Create minimal test objects directly: `BreakpointManager::new()`, `RustProblemMatcher::new()`
- Use temporary file paths: `PathBuf::from("/test/file.rs")`
- Tests work with real implementations, not mocks

**What to Mock:**
- File I/O operations: Create temp files if needed or use in-memory representations
- Network operations: Not visible in current test suite
- Time-dependent behavior: Not visible in current test suite

**What NOT to Mock:**
- Core business logic (buffer operations, breakpoint tracking, problem matching)
- Data structures (use real PathBuf, String, HashMap, etc.)
- Enums and configuration types

## Fixtures and Factories

**Test Data:**

Tests create their own data directly rather than using factories:

```rust
// From crates/dap_bridge/src/breakpoints.rs
let file = PathBuf::from("/test/file.rs");
let bp = manager.add(file.clone(), 10);
```

```rust
// From crates/tasks/src/problem_matcher.rs
let problem = Problem::error("test error")
    .with_location(PathBuf::from("test.rs"), 10, Some(5))
    .with_code("E0001")
    .with_source("rustc");
```

**Location:**
- No separate fixtures directory
- Test data created inline in test functions
- Builders used for complex object construction

## Coverage

**Requirements:** No coverage requirements enforced

**View Coverage:**
```bash
cargo tarpaulin --out Html  # If tarpaulin is installed
```

Coverage not currently configured in workspace.

## Test Types

**Unit Tests:**
- Scope: Individual structs, functions, and methods
- Approach: Direct function calls with assertions
- Files: Same file as implementation (in `#[cfg(test)]` module)
- Examples:
  - `test_buffer_basic_operations()` - tests Buffer struct operations
  - `test_add_breakpoint()` - tests BreakpointManager::add() method
  - `test_rust_error_match()` - tests RustProblemMatcher::match_line() regex matching

**Integration Tests:**
- Not currently used
- Could be added as `tests/` directory at crate root if needed

**E2E Tests:**
- Not used in this codebase
- UI tests would require integration with GPUI framework

## Common Patterns

**Result Handling in Tests:**

Tests return Result types from library code and handle `unwrap()`:

```rust
#[test]
fn test_buffer_basic_operations() {
    let mut buffer = Buffer::new();
    buffer.insert(0, "Hello, World!").unwrap();  // Unwrap propagates panic on error
    assert_eq!(buffer.text(), "Hello, World!");
}
```

**Option Handling in Tests:**

Tests check for Some/None values:

```rust
#[test]
fn test_toggle_breakpoint() {
    let bp = manager.toggle(file.clone(), 10);
    assert!(bp.is_some());  // Or use unwrap() if panic desired
}
```

**String Assertions:**

Tests verify string content with contains() or exact equality:

```rust
#[test]
fn test_rust_error_match() {
    let problem = matcher.match_line(line).unwrap();
    assert!(problem.message.contains("borrow of moved value"));
    assert_eq!(problem.code, Some("E0382".to_string()));
}
```

**Trait Object Testing:**

Implementations of traits are tested via concrete structs:

```rust
// From crates/tasks/src/problem_matcher.rs
pub trait ProblemMatcher: Send + Sync {
    fn match_line(&self, line: &str) -> Option<Problem>;
}

#[test]
fn test_rust_error_match() {
    let matcher = RustProblemMatcher::new();  // Concrete impl of trait
    let problem = matcher.match_line(line).unwrap();
    assert!(problem.is_error());
}
```

## Test Statistics

**Files with Tests:**
- `crates/editor_core/src/lib.rs` - 3 tests
- `crates/dap_bridge/src/breakpoints.rs` - 7+ tests
- `crates/dap_bridge/src/client.rs` - 2+ tests
- `crates/dap_bridge/src/session.rs` - 2+ tests
- `crates/tasks/src/problem_matcher.rs` - 6 tests
- `crates/tasks/src/runner.rs` - 4+ tests

**Total visible tests:** 25+ unit tests across codebase

## Running Tests

**From workspace root:**
```bash
cargo test                           # Test all crates
cargo test --package editor_core     # Test specific crate
cargo test test_buffer_basic_operations  # Run specific test
cargo test -- --test-threads=1     # Single-threaded execution
```

**Watch Mode (requires cargo-watch):**
```bash
cargo watch -x test
```

---

*Testing analysis: 2026-01-27*
