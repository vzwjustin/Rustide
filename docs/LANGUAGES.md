# Language Support in Rustide

This document provides comprehensive documentation for Rustide's multi-language support system, which leverages Tree-sitter for syntax parsing, Language Server Protocol (LSP) for intelligent code features, and Debug Adapter Protocol (DAP) for debugging capabilities.

## Table of Contents

1. [Overview](#overview)
2. [Language Registry](#language-registry)
3. [Tree-sitter Integration](#tree-sitter-integration)
4. [LSP Integration](#lsp-integration)
5. [DAP Integration](#dap-integration)
6. [Built-in Languages](#built-in-languages)
7. [Adding New Languages](#adding-new-languages)

---

## Overview

Rustide provides multi-language support through three complementary systems:

| System | Purpose | Protocol/Technology |
|--------|---------|---------------------|
| **Tree-sitter** | Syntax highlighting, parsing, code folding | Incremental parsing with S-expression queries |
| **LSP** | Code intelligence (completion, diagnostics, navigation) | Language Server Protocol (JSON-RPC) |
| **DAP** | Debugging (breakpoints, stepping, variables) | Debug Adapter Protocol (JSON-RPC) |

### Architecture Diagram

```
                          +------------------+
                          |  Language        |
                          |  Registry        |
                          +--------+---------+
                                   |
          +------------------------+------------------------+
          |                        |                        |
+---------v---------+  +-----------v-----------+  +---------v---------+
|   Tree-sitter     |  |      LSP Bridge       |  |    DAP Bridge     |
|   - Grammar       |  |   - ServerManager     |  |   - DebugSession  |
|   - Highlighter   |  |   - LspClient         |  |   - DapClient     |
|   - Queries       |  |   - Capabilities      |  |   - Breakpoints   |
+-------------------+  +-----------------------+  +-------------------+
```

---

## Language Registry

The `LanguageRegistry` is the central hub for managing all language definitions. It provides thread-safe storage and lookup for language configurations.

### LanguageDefinition Structure

Each language is defined by a `LanguageDefinition` struct containing:

```rust
pub struct LanguageDefinition {
    /// Unique identifier (e.g., "rust", "python")
    pub id: LanguageId,

    /// Human-readable name (e.g., "Rust", "Python")
    pub display_name: String,

    /// File extensions (without dot): ["rs"], ["py", "pyi"]
    pub extensions: Vec<String>,

    /// Exact file names: ["Makefile", "Dockerfile"]
    pub file_names: Vec<String>,

    /// Tree-sitter grammar for parsing
    pub grammar: Grammar,

    /// Language-specific configuration (LSP, DAP, etc.)
    pub config: LanguageConfig,

    /// MIME types: ["text/x-rust"]
    pub mime_types: Vec<String>,

    /// First-line patterns for detection (shebangs)
    pub first_line_patterns: Vec<String>,
}
```

### LanguageConfig Fields

The `LanguageConfig` structure contains all language-specific settings:

| Field | Type | Description |
|-------|------|-------------|
| `lsp` | `Option<LspConfig>` | LSP server configuration |
| `dap` | `Option<DapConfig>` | Debug adapter configuration |
| `comments` | `CommentConfig` | Line/block comment syntax |
| `brackets` | `Vec<BracketPair>` | Bracket pairs for matching |
| `auto_pairs` | `Vec<AutoPair>` | Auto-closing pairs (quotes) |
| `indentation` | `IndentationConfig` | Tab/space settings |
| `formatter` | `Option<FormatterConfig>` | Code formatter command |
| `run` | `Option<RunConfig>` | Run/build commands |
| `icon` | `Option<String>` | Icon identifier |

### Language Lookup

The registry provides multiple lookup methods:

```rust
// Initialize with built-in languages
let registry = init_language_registry();

// Lookup by ID
let rust = registry.get_by_id(&LanguageId::new("rust"));

// Lookup by file extension
let lang = registry.get_by_extension("rs");

// Lookup by exact filename
let makefile = registry.get_by_filename("Makefile");

// Auto-detect from path and content
let detected = registry.detect_language(
    Path::new("/src/main.rs"),
    Some("#!/usr/bin/env rust-script"),
);
```

### Language Detection Priority

When detecting a language, the registry follows this priority:

1. **Exact file name match** - For files like `Makefile`, `Dockerfile`
2. **File extension match** - Case-insensitive extension lookup
3. **First-line pattern match** - Shebang lines like `#!/usr/bin/env python`

---

## Tree-sitter Integration

Tree-sitter provides fast, incremental parsing for syntax highlighting and structural understanding of code.

### Grammar Loading

Grammars are loaded via the `GrammarLoader` which wraps Tree-sitter language bindings:

```rust
// Load a grammar with highlight queries
let grammar = GrammarLoader::rust()?;
let grammar = GrammarLoader::python()?;
let grammar = GrammarLoader::javascript()?;
```

### Grammar Structure

```rust
pub struct Grammar {
    /// The Tree-sitter language definition
    language: Language,

    /// Highlight query for syntax coloring
    highlight_query: Option<HighlightQuery>,

    /// Injection query for embedded languages
    injection_query: Option<HighlightQuery>,

    /// Locals query for variable tracking
    locals_query: Option<HighlightQuery>,
}
```

### Highlight Queries (.scm Files)

Highlight queries use Tree-sitter's S-expression query language to map syntax nodes to highlight groups:

```scheme
; Keywords
"fn" @keyword.function
"let" @keyword
"if" @keyword
"else" @keyword

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Functions
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)

; Strings and literals
(string_literal) @string
(integer_literal) @number
(float_literal) @number

; Comments
(line_comment) @comment
(block_comment) @comment

; Operators
"+" @operator
"-" @operator
"*" @operator
```

### Available Highlight Groups

| Group | Description | Example |
|-------|-------------|---------|
| `keyword` | Language keywords | `fn`, `if`, `class` |
| `keyword.function` | Function definition keywords | `fn`, `def`, `function` |
| `type` | Type identifiers | `MyStruct`, `Vec` |
| `type.builtin` | Built-in types | `i32`, `str`, `int` |
| `function` | Function names | `main`, `calculate` |
| `function.method` | Method names | `.push()`, `.len()` |
| `function.macro` | Macro invocations | `println!`, `vec!` |
| `variable` | Variable names | `x`, `count` |
| `variable.parameter` | Function parameters | `fn foo(x: i32)` |
| `variable.builtin` | Built-in variables | `self`, `this` |
| `string` | String literals | `"hello"` |
| `string.escape` | Escape sequences | `\n`, `\t` |
| `number` | Numeric literals | `42`, `3.14` |
| `comment` | Comments | `// comment` |
| `operator` | Operators | `+`, `-`, `&&` |
| `punctuation.bracket` | Brackets | `()`, `[]`, `{}` |
| `punctuation.delimiter` | Delimiters | `,`, `;`, `:` |

### Incremental Parsing

Tree-sitter supports incremental parsing for efficient updates:

```rust
let highlighter = Highlighter::new(grammar)?;

// Initial parse
highlighter.parse(source)?;

// After an edit, re-parse incrementally
let edit = tree_sitter::InputEdit {
    start_byte,
    old_end_byte,
    new_end_byte,
    start_position,
    old_end_position,
    new_end_position,
};
highlighter.edit(&edit, new_source)?;
```

### Injection Support

Injection queries allow highlighting embedded languages (e.g., SQL in Python strings):

```scheme
; Inject SQL in strings tagged with 'sql'
((string_literal) @injection.content
  (#match? @injection.content "^\"SELECT")
  (#set! injection.language "sql"))
```

---

## LSP Integration

The LSP Bridge provides communication with Language Server Protocol servers for intelligent code features.

### Server Configuration

LSP servers are configured via `LspConfig`:

```rust
pub struct LspConfig {
    /// Command to start the server (e.g., "rust-analyzer")
    pub command: String,

    /// Command arguments (e.g., ["--stdio"])
    pub args: Vec<String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Working directory
    pub cwd: Option<String>,

    /// Initialization options (server-specific)
    pub initialization_options: Option<serde_json::Value>,

    /// Server settings
    pub settings: HashMap<String, serde_json::Value>,

    /// Root detection patterns (e.g., ["Cargo.toml"])
    pub root_patterns: Vec<String>,
}
```

### Lifecycle Management

The `ServerManager` handles the complete lifecycle of language servers:

```rust
// Create manager for a workspace
let manager = ServerManagerBuilder::new("/path/to/workspace")
    .with_rust_analyzer()
    .with_python_lsp()
    .with_typescript_lsp()
    .build();

// Start a server
manager.start_server("rust").await?;

// Check server status
if manager.is_server_running("rust") {
    // Server is ready
}

// Stop a server
manager.stop_server("rust").await?;

// Restart a server
manager.restart_server("rust").await?;
```

### Client State Machine

The LSP client transitions through these states:

```
Disconnected -> Starting -> Connected -> Initializing -> Ready -> ShuttingDown -> Shutdown
```

| State | Description |
|-------|-------------|
| `Disconnected` | Client not connected |
| `Starting` | Spawning server process |
| `Connected` | Process started, not initialized |
| `Initializing` | Sending initialize request |
| `Ready` | Server ready for requests |
| `ShuttingDown` | Shutdown in progress |
| `Shutdown` | Server terminated |

### Capability Detection

After initialization, server capabilities are parsed into `ServerCapabilityInfo`:

```rust
let caps = client.capabilities()?;

// Check supported features
if caps.supports_completion() {
    // Completion is available
}

if caps.supports_definition() {
    // Go to definition is available
}

// Get feature summary
let summary = caps.feature_summary();
println!("Supported: {}/{}", summary.supported_count(), summary.total_count());
```

### Supported LSP Features

| Feature | Method | Description |
|---------|--------|-------------|
| Completion | `textDocument/completion` | Code completion suggestions |
| Hover | `textDocument/hover` | Documentation on hover |
| Signature Help | `textDocument/signatureHelp` | Function signature info |
| Go to Definition | `textDocument/definition` | Navigate to definition |
| Go to Type Definition | `textDocument/typeDefinition` | Navigate to type definition |
| Go to Implementation | `textDocument/implementation` | Navigate to implementations |
| Find References | `textDocument/references` | Find all references |
| Document Highlight | `textDocument/documentHighlight` | Highlight occurrences |
| Document Symbols | `textDocument/documentSymbol` | Symbol outline |
| Workspace Symbols | `workspace/symbol` | Search symbols across workspace |
| Code Actions | `textDocument/codeAction` | Quick fixes, refactorings |
| Code Lens | `textDocument/codeLens` | Inline actions/info |
| Formatting | `textDocument/formatting` | Format document |
| Range Formatting | `textDocument/rangeFormatting` | Format selection |
| Rename | `textDocument/rename` | Rename symbol |
| Folding Range | `textDocument/foldingRange` | Code folding regions |
| Semantic Tokens | `textDocument/semanticTokens` | Semantic highlighting |
| Inlay Hints | `textDocument/inlayHint` | Inline type hints |
| Call Hierarchy | `textDocument/prepareCallHierarchy` | Call hierarchy view |

### Document Synchronization

The manager handles document lifecycle notifications:

```rust
// Document opened
manager.did_open("rust", uri, text, version)?;

// Document changed
manager.did_change("rust", uri, version, changes)?;

// Document saved
manager.did_save("rust", uri, Some(text))?;

// Document closed
manager.did_close("rust", uri)?;
```

### Diagnostics

Diagnostics are collected via `publishDiagnostics` notifications:

```rust
let diagnostics = manager.diagnostics();

// Get diagnostics for a file
let file_diags = diagnostics.get_diagnostics(&uri);

// Get summary
let summary = diagnostics.summary();
println!("Errors: {}, Warnings: {}", summary.error_count, summary.warning_count);
```

---

## DAP Integration

The DAP Bridge provides debugging capabilities through the Debug Adapter Protocol.

### Debug Adapter Configuration

Debug adapters are configured via `DapConfig`:

```rust
pub struct DapConfig {
    /// Adapter identifier (e.g., "codelldb")
    pub adapter_id: String,

    /// Command to start the adapter
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Port for socket-based adapters
    pub port: Option<u16>,

    /// Launch configuration templates
    pub launch_templates: Vec<DapLaunchTemplate>,

    /// Attach configuration templates
    pub attach_templates: Vec<DapLaunchTemplate>,
}
```

### Launch/Attach Configurations

Launch templates define how to start debugging:

```rust
// Launch configuration
DapLaunchTemplate::launch("Launch Debug")
    .with_config("type", json!("lldb"))
    .with_config("request", json!("launch"))
    .with_config("program", json!("${workspaceFolder}/target/debug/myapp"))
    .with_config("cwd", json!("${workspaceFolder}"))
    .with_config("args", json!([]))
    .with_config("stopOnEntry", json!(false))

// Attach configuration
DapLaunchTemplate::attach("Attach to Process")
    .with_config("type", json!("lldb"))
    .with_config("request", json!("attach"))
    .with_config("pid", json!("${command:pickProcess}"))
```

### Debug Session Lifecycle

```rust
// Create a debug session
let config = DapClientConfig {
    adapter_path: PathBuf::from("codelldb"),
    adapter_args: vec!["--port".into(), "13000".into()],
    timeout_ms: 10000,
    ..Default::default()
};

let mut session = DebugSession::new(config);

// Start the session
session.start().await?;

// Launch the program
session.launch("/path/to/program", &["arg1", "arg2"], Some("/working/dir")).await?;

// Or attach to a running process
session.attach(12345).await?;
```

### Session States

| State | Description |
|-------|-------------|
| `Inactive` | Session not started |
| `Initializing` | Debug adapter initializing |
| `Ready` | Ready to launch/attach |
| `Running` | Program is executing |
| `Stopped(reason)` | Execution paused (breakpoint, step, etc.) |
| `Terminated` | Debug session ended |

### Stopped Reasons

When execution stops, the reason is provided:

```rust
pub enum StoppedReason {
    Step,                // Step completed
    Breakpoint,          // Hit a breakpoint
    Exception,           // Exception thrown
    Pause,               // User-requested pause
    Entry,               // Program entry point
    Goto,                // Goto target reached
    FunctionBreakpoint,  // Function breakpoint hit
    DataBreakpoint,      // Data breakpoint triggered
    InstructionBreakpoint, // Instruction breakpoint
}
```

### Breakpoint Management

The `BreakpointManager` handles all breakpoint operations:

```rust
let manager = BreakpointManager::new();

// Add a simple breakpoint
let bp = manager.add(PathBuf::from("/src/main.rs"), 42);

// Add a conditional breakpoint
let bp = manager.add_conditional(
    PathBuf::from("/src/main.rs"),
    50,
    "counter > 10"
);

// Add a logpoint (logs instead of breaking)
let bp = manager.add_logpoint(
    PathBuf::from("/src/main.rs"),
    60,
    "Value: {x}"
);

// Toggle breakpoint at location
manager.toggle(PathBuf::from("/src/main.rs"), 42);

// Enable/disable breakpoints
manager.enable(bp.id);
manager.disable(bp.id);

// Remove breakpoint
manager.remove(bp.id);
```

### Breakpoint States

```rust
pub enum BreakpointState {
    Pending,           // Awaiting verification
    Verified,          // Confirmed by adapter
    Disabled,          // Manually disabled
    Failed(String),    // Failed to set
}
```

### Debugging Operations

```rust
// Execution control
session.continue_execution().await?;
session.step_over().await?;
session.step_into().await?;
session.step_out().await?;
session.pause().await?;

// Inspect threads
let threads = session.get_threads().await?;

// Get stack trace
let frames = session.get_stack_trace(thread_id).await?;

// Get scopes for a frame
let scopes = session.get_scopes(frame_id).await?;

// Get variables in a scope
let variables = session.get_variables(scope.variables_reference).await?;

// Evaluate expression
let result = session.evaluate("myVar.field", Some(frame_id)).await?;
```

### Variable Inspection

```rust
pub struct Variable {
    pub name: String,              // Variable name
    pub value: String,             // Display value
    pub var_type: Option<String>,  // Type name
    pub variables_reference: i64,  // Reference for expansion (0 = leaf)
}

// Expand complex variables
if variable.variables_reference > 0 {
    let children = session.get_variables(variable.variables_reference).await?;
}
```

---

## Built-in Languages

Rustide includes built-in support for the following languages:

| Language | Extensions | LSP Server | DAP Adapter | Formatter |
|----------|------------|------------|-------------|-----------|
| **Rust** | `.rs` | rust-analyzer | codelldb | rustfmt |
| **JavaScript** | `.js`, `.mjs`, `.cjs`, `.jsx` | typescript-language-server | node --inspect | prettier |
| **TypeScript** | `.ts`, `.mts`, `.cts`, `.tsx` | typescript-language-server | ts-node | prettier |
| **Python** | `.py`, `.pyi`, `.pyw` | pyright-langserver | debugpy | black |
| **Go** | `.go` | gopls | delve (dlv) | gofmt |
| **C** | `.c`, `.h` | clangd | gdb (cppdbg) | clang-format |
| **JSON** | `.json`, `.jsonc`, `.json5` | vscode-json-language-server | - | prettier |
| **TOML** | `.toml` | taplo | - | taplo |
| **Markdown** | `.md`, `.markdown`, `.mdown` | marksman | - | prettier |

### Language-Specific Details

#### Rust

```rust
LspConfig::new("rust-analyzer")
    .with_root_patterns(["Cargo.toml", "rust-project.json"])

DapConfig::new("codelldb", "codelldb")
    .with_args(["--port", "13000"])
    .with_port(13000)
    .with_launch_template(
        DapLaunchTemplate::launch("Launch")
            .with_config("type", json!("lldb"))
            .with_config("program", json!("${workspaceFolder}/target/debug/${workspaceFolderBasename}"))
    )

FormatterConfig::new("rustfmt")
    .with_args(["--edition", "2021"])
    .format_on_save(true)
```

#### Python

```rust
LspConfig::new("pyright-langserver")
    .with_args(["--stdio"])
    .with_root_patterns(["pyproject.toml", "setup.py", "requirements.txt"])

DapConfig::new("debugpy", "python")
    .with_args(["-m", "debugpy.adapter"])
    .with_launch_template(
        DapLaunchTemplate::launch("Launch")
            .with_config("type", json!("python"))
            .with_config("program", json!("${file}"))
    )

FormatterConfig::new("black")
    .with_args(["-"])
    .format_on_save(true)
```

#### Go

```rust
LspConfig::new("gopls")
    .with_root_patterns(["go.mod", "go.sum"])

DapConfig::new("delve", "dlv")
    .with_args(["dap"])
    .with_launch_template(
        DapLaunchTemplate::launch("Launch")
            .with_config("type", json!("go"))
            .with_config("mode", json!("debug"))
            .with_config("program", json!("${workspaceFolder}"))
    )

FormatterConfig::new("gofmt")
    .format_on_save(true)
```

---

## Adding New Languages

This section provides a step-by-step guide for adding support for a new language.

### Step 1: Add Tree-sitter Grammar

First, add the Tree-sitter grammar dependency to `Cargo.toml`:

```toml
[dependencies]
tree-sitter-mylang = "0.1"
```

Then add a loader method in `/crates/languages/src/grammar.rs`:

```rust
impl GrammarLoader {
    /// Load the MyLang grammar with highlight queries.
    pub fn mylang() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_mylang::LANGUAGE.into())
            .with_highlight_query(MYLANG_HIGHLIGHTS)?;
        Ok(grammar)
    }
}
```

### Step 2: Create Highlight Queries

Add highlight queries for your language:

```rust
pub const MYLANG_HIGHLIGHTS: &str = r#"
; Keywords
"func" @keyword.function
"var" @keyword
"if" @keyword
"else" @keyword
"for" @keyword
"return" @keyword

; Types
(type_identifier) @type

; Functions
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)

; Variables
(identifier) @variable

; Literals
(string_literal) @string
(number_literal) @number

; Comments
(comment) @comment

; Operators
"+" @operator
"-" @operator
"=" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"," @punctuation.delimiter
";" @punctuation.delimiter
"#"#;
```

### Step 3: Configure LSP

Create the LSP configuration in `/crates/languages/src/builtin.rs`:

```rust
pub fn get_mylang_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::mylang().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("mylang-lsp")
                .with_args(["--stdio"])
                .with_root_patterns(["mylang.config", "project.mylang"]),
        )
        // ... other config
        .build();

    Some(
        LanguageDefinition::builder("mylang", "MyLang")
            .extensions(["ml", "mylang"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}
```

### Step 4: Configure DAP (Optional)

If the language has a debug adapter:

```rust
.dap(
    DapConfig::new("mylang-debug", "mylang-debug-adapter")
        .with_args(["--mode", "dap"])
        .with_launch_template(
            DapLaunchTemplate::launch("Launch")
                .with_config("type", json!("mylang"))
                .with_config("request", json!("launch"))
                .with_config("program", json!("${file}")),
        )
        .with_attach_template(
            DapLaunchTemplate::attach("Attach")
                .with_config("type", json!("mylang"))
                .with_config("request", json!("attach"))
                .with_config("port", json!(9229)),
        ),
)
```

### Step 5: Configure Additional Features

```rust
.comments(
    CommentConfig::new()
        .with_line("//")
        .with_block("/*", "*/")
        .with_doc_line("///")
)
.brackets(vec![
    BracketPair::new("(", ")"),
    BracketPair::new("[", "]"),
    BracketPair::new("{", "}").is_scope(true),
])
.auto_pairs(vec![
    AutoPair::symmetric("\"").not_before(["\\", "\""]),
    AutoPair::symmetric("'").not_before(["\\", "'"]),
])
.indentation(IndentationConfig::new().with_spaces(4))
.formatter(
    FormatterConfig::new("mylang-fmt")
        .with_args(["--stdin"])
        .format_on_save(true),
)
.run(
    RunConfig::new("mylang")
        .with_args(["run", "${file}"]),
)
.icon("mylang")
```

### Step 6: Register the Language

Add to `get_builtin_languages()` in `/crates/languages/src/builtin.rs`:

```rust
pub fn get_builtin_languages() -> Vec<LanguageDefinition> {
    let mut languages = Vec::new();

    // ... existing languages ...

    if let Some(lang) = get_mylang_language() {
        languages.push(lang);
    }

    languages
}
```

### Step 7: Export the Function

Add export in `/crates/languages/src/lib.rs`:

```rust
pub use builtin::{
    // ... existing exports ...
    get_mylang_language,
};
```

### Step 8: Testing

Create tests for your language support:

```rust
#[test]
fn test_mylang_language() {
    let lang = get_mylang_language();
    assert!(lang.is_some());

    let lang = lang.unwrap();
    assert_eq!(lang.id.as_str(), "mylang");
    assert!(lang.matches_extension("ml"));
    assert!(lang.config.lsp.is_some());
}

#[test]
fn test_mylang_grammar() {
    let grammar = GrammarLoader::mylang().unwrap();
    let source = "func main() { print(\"hello\") }";
    let tree = grammar.parse(source, None).unwrap();
    assert_eq!(tree.root_node().kind(), "source_file");
}

#[test]
fn test_mylang_highlighting() {
    let grammar = GrammarLoader::mylang().unwrap();
    let highlighter = Highlighter::new(grammar).unwrap();

    let source = "func main() {}";
    highlighter.parse(source).unwrap();
    let spans = highlighter.highlight_spans(source).unwrap();

    // Verify keywords are highlighted
    let groups: Vec<_> = spans.iter().filter_map(|(_, g)| g.clone()).collect();
    assert!(groups.iter().any(|g| g.contains("keyword")));
}
```

### Complete Example

Here's a complete example adding support for a hypothetical language:

```rust
// In grammar.rs
pub const EXAMPLE_HIGHLIGHTS: &str = r#"
"func" @keyword.function
"let" @keyword
"return" @keyword
(identifier) @variable
(string) @string
(number) @number
(comment) @comment
"#;

impl GrammarLoader {
    pub fn example() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_example::LANGUAGE.into())
            .with_highlight_query(EXAMPLE_HIGHLIGHTS)?;
        Ok(grammar)
    }
}

// In builtin.rs
pub fn get_example_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::example().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("example-lsp")
                .with_args(["--stdio"])
                .with_root_patterns(["example.json"]),
        )
        .dap(
            DapConfig::new("example-debug", "example-debugger")
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", json!("example"))
                        .with_config("request", json!("launch"))
                        .with_config("program", json!("${file}")),
                ),
        )
        .comments(CommentConfig::c_style())
        .brackets(BracketPair::standard())
        .auto_pairs(AutoPair::quotes())
        .indentation(IndentationConfig::new().with_spaces(2))
        .icon("example")
        .formatter(
            FormatterConfig::new("example-fmt")
                .format_on_save(true),
        )
        .run(
            RunConfig::new("example")
                .with_args(["run", "${file}"]),
        )
        .build();

    Some(
        LanguageDefinition::builder("example", "Example Language")
            .extensions(["ex", "example"])
            .file_names(["example.json"])
            .mime_types(["text/x-example"])
            .first_line_patterns(["#!/usr/bin/env example"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}
```

---

## Troubleshooting

### LSP Server Not Starting

1. Verify the server command is in PATH:
   ```bash
   which rust-analyzer
   ```

2. Check server logs in the output panel

3. Verify root patterns match your project structure

### Highlighting Not Working

1. Ensure the grammar crate is properly linked
2. Check for syntax errors in highlight queries
3. Verify capture names match expected patterns

### DAP Connection Issues

1. Check if the debug adapter is installed
2. Verify port availability for socket-based adapters
3. Review adapter-specific documentation for configuration

---

## References

- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
