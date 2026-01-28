# Debugging in RustIDE

## Overview

RustIDE provides a powerful debugging experience built on the **Debug Adapter Protocol (DAP)**, enhanced with Zed-style UI innovations. This integration allows developers to debug applications written in multiple languages using a consistent, intuitive interface while leveraging language-specific debug adapters for optimal debugging capabilities.

### Key Features

- **Multi-language support** via DAP-compliant adapters
- **Rich breakpoint system** including conditional breakpoints and logpoints
- **Integrated debug UI** with call stack, variables, and watch expressions
- **Launch and attach modes** for flexible debugging workflows
- **Inline values display** (planned) for at-a-glance variable inspection
- **Debug console** with REPL capabilities

---

## DAP Integration

### What is Debug Adapter Protocol?

The **Debug Adapter Protocol (DAP)** is a standardized protocol developed by Microsoft that defines how development tools communicate with debuggers. Instead of implementing debugger integrations from scratch for each language, RustIDE uses DAP to communicate with language-specific debug adapters.

```
┌─────────────┐         DAP Messages          ┌─────────────────┐
│   RustIDE   │ ◄──────────────────────────► │  Debug Adapter  │
│   (Client)  │    JSON over stdio/socket     │  (codelldb,     │
└─────────────┘                               │   debugpy, etc) │
                                              └────────┬────────┘
                                                       │
                                                       ▼
                                              ┌─────────────────┐
                                              │    Debuggee     │
                                              │  (Your Program) │
                                              └─────────────────┘
```

### How RustIDE Implements DAP

RustIDE's debugging architecture consists of several key components:

1. **DAP Client**: Manages communication with debug adapters using JSON-RPC over stdio or sockets
2. **Session Manager**: Handles debug session lifecycle, including launch, attach, and termination
3. **Breakpoint Manager**: Synchronizes breakpoints between the editor and debug adapter
4. **UI Components**: Renders debug state in panels and editor overlays

```rust
// Core debugging module structure
pub mod debugging {
    pub mod dap_client;      // DAP protocol implementation
    pub mod session;         // Debug session management
    pub mod breakpoints;     // Breakpoint types and management
    pub mod ui;              // Debug UI components
    pub mod adapters;        // Adapter configurations
}
```

### Supported Debug Adapters

| Language | Adapter | Installation |
|----------|---------|--------------|
| Rust | codelldb, lldb-dap | Via extension or system package |
| Python | debugpy | `pip install debugpy` |
| JavaScript/TypeScript | vscode-js-debug | Built-in with Node.js |
| Go | delve (dlv) | `go install github.com/go-delve/delve/cmd/dlv@latest` |
| C/C++ | lldb-dap, gdb-dap | Via LLVM or GDB packages |

---

## Debug UI Components

### Breakpoint Gutter

The breakpoint gutter appears in the left margin of the editor, allowing users to set and manage breakpoints visually.

<!-- Screenshot: breakpoint_gutter.png - Shows the editor gutter with various breakpoint indicators -->

**Breakpoint Indicators:**
- 🔴 **Red circle**: Active line breakpoint
- 🟡 **Yellow circle**: Conditional breakpoint
- 💬 **Blue diamond**: Logpoint
- ⭕ **Hollow circle**: Disabled breakpoint
- ❌ **Red X**: Invalid/unverified breakpoint

**Interactions:**
- **Click**: Toggle line breakpoint
- **Right-click**: Open breakpoint context menu
- **Shift+Click**: Add conditional breakpoint
- **Ctrl+Click**: Add logpoint

### Call Stack Panel

The call stack panel displays the current execution context when the debugger is paused.

<!-- Screenshot: call_stack_panel.png - Shows the call stack with multiple frames -->

```
┌─────────────────────────────────────────┐
│ CALL STACK                          ▼   │
├─────────────────────────────────────────┤
│ ► Thread 1 (main)                       │
│   ├─ calculate_sum      main.rs:42      │
│   ├─ process_data       utils.rs:128    │
│   └─ main               main.rs:15      │
│ ► Thread 2 (worker)                     │
│   └─ wait_for_input     io.rs:67        │
└─────────────────────────────────────────┘
```

**Features:**
- Thread selection and navigation
- Frame selection updates variables and source view
- Collapse/expand threads
- Copy stack trace to clipboard

### Variables Panel

The variables panel shows local variables, arguments, and watch expressions for the selected stack frame.

<!-- Screenshot: variables_panel.png - Shows expanded variable tree with nested structures -->

```
┌─────────────────────────────────────────┐
│ VARIABLES                           ▼   │
├─────────────────────────────────────────┤
│ ▼ Locals                                │
│   ├─ counter: i32 = 42                  │
│   ├─ name: &str = "RustIDE"             │
│   ▼ config: Config                      │
│     ├─ debug: bool = true               │
│     └─ level: u8 = 3                    │
├─────────────────────────────────────────┤
│ ▼ Arguments                             │
│   └─ args: Vec<String> [3 items]        │
├─────────────────────────────────────────┤
│ ▼ Watch                                 │
│   ├─ counter * 2 = 84                   │
│   └─ config.level > 2 = true            │
└─────────────────────────────────────────┘
```

**Features:**
- Expandable nested structures
- Inline value editing (where supported)
- Add/remove watch expressions
- Copy variable values
- Set value (modify during debugging)

### Debug Console

The debug console provides a REPL interface for evaluating expressions and viewing debug output.

<!-- Screenshot: debug_console.png - Shows debug console with evaluated expressions -->

```
┌─────────────────────────────────────────────────────────────┐
│ DEBUG CONSOLE                                           ▼   │
├─────────────────────────────────────────────────────────────┤
│ [14:23:01] Debugger attached to process 12345               │
│ [14:23:02] Breakpoint hit at main.rs:42                     │
│ > counter + 10                                              │
│ 52                                                          │
│ > config                                                    │
│ Config { debug: true, level: 3 }                            │
│ > _                                                         │
└─────────────────────────────────────────────────────────────┘
```

**Capabilities:**
- Expression evaluation in current context
- Command history (up/down arrows)
- Syntax highlighting for output
- Filter by message type (info, warning, error)
- Clear console output

### Inline Values (Future)

Inline values will display variable values directly in the editor next to their declarations and usages.

<!-- Screenshot: inline_values_mockup.png - Shows variable values displayed inline in editor -->

```rust
fn calculate_sum(numbers: &[i32]) -> i32 {  // numbers = [1, 2, 3, 4, 5]
    let mut sum = 0;                         // sum = 0
    for num in numbers {                     // num = 3 (current iteration)
        sum += num;                          // sum = 6
    }
    sum                                      // sum = 15
}
```

---

## Breakpoint Types

### Line Breakpoints

The simplest form of breakpoint, pausing execution when a specific line is reached.

**Setting a Line Breakpoint:**
1. Click in the gutter next to the target line
2. Use keyboard shortcut `F9`
3. Use Command Palette: "Debug: Toggle Breakpoint"

```rust
fn main() {
    let x = 10;
    let y = 20;
    let result = x + y;  // ← Line breakpoint here
    println!("{}", result);
}
```

### Conditional Breakpoints

Breakpoints that only trigger when a specified condition evaluates to true.

**Setting a Conditional Breakpoint:**
1. Right-click in the gutter → "Add Conditional Breakpoint"
2. Enter a boolean expression

```rust
for i in 0..1000 {
    process_item(i);  // ← Conditional breakpoint: i == 500
}
```

**Condition Examples:**
```
i == 500                    // Exact value match
user.name == "admin"        // String comparison
buffer.len() > 1024         // Method call
count > 10 && flag == true  // Complex expression
```

### Logpoints

Logpoints print messages to the debug console without pausing execution, useful for non-intrusive debugging.

**Setting a Logpoint:**
1. Right-click in the gutter → "Add Logpoint"
2. Enter the message template

**Message Template Syntax:**
```
Processing item {i} of {total}
User {user.name} logged in at {timestamp}
Value changed: old={old_value}, new={new_value}
```

Expressions in `{}` are evaluated and interpolated into the message.

### Exception Breakpoints

Pause execution when exceptions or panics occur.

**Configuration:**
```json
{
  "exceptionBreakpoints": [
    {
      "filter": "all",
      "enabled": true
    },
    {
      "filter": "uncaught",
      "enabled": true
    }
  ]
}
```

**Available Filters (varies by adapter):**
- `all`: Break on all exceptions
- `uncaught`: Break only on uncaught exceptions
- `raised`: Break when exception is raised (Python)
- `panic`: Break on Rust panics

---

## Session Lifecycle

### Launch vs Attach

RustIDE supports two primary debugging modes:

**Launch Mode:**
- RustIDE starts the debuggee process
- Full control over program arguments and environment
- Process terminates when debugging ends

**Attach Mode:**
- Connect to an already-running process
- Useful for debugging servers, long-running processes, or remote debugging
- Process continues after debugging session ends

```
┌──────────────────────────────────────────────────────────────┐
│                     SESSION LIFECYCLE                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  [Initialize] ──► [Launch/Attach] ──► [Running] ◄──┐         │
│                                           │        │         │
│                                           ▼        │         │
│                                      [Paused] ─────┘         │
│                                           │                  │
│                                           ▼                  │
│                                    [Terminated]              │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Configuration Files

Debug configurations are stored in `.rustide/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug Binary",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/myapp",
      "args": ["--verbose"],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_BACKTRACE": "1"
      }
    }
  ]
}
```

**Configuration File Locations:**
- `.rustide/launch.json` - Project-specific configurations
- `~/.config/rustide/launch.json` - User-wide defaults
- `.vscode/launch.json` - VS Code compatibility (read-only)

### Session States

| State | Description |
|-------|-------------|
| `Inactive` | No debug session active |
| `Initializing` | Adapter starting, setting up |
| `Running` | Debuggee executing |
| `Paused` | Execution stopped at breakpoint/step |
| `Stopping` | Session terminating |
| `Terminated` | Session ended |

---

## Per-Language Setup

### Rust (codelldb / lldb-dap)

**Recommended Adapter:** codelldb (provides better Rust-specific features)

**Installation:**
```bash
# Option 1: Install codelldb extension
rustide --install-extension vadimcn.vscode-lldb

# Option 2: Use system LLDB
# macOS: Included with Xcode
# Linux: sudo apt install lldb
# Windows: Install LLVM from llvm.org
```

**Configuration:**
```json
{
  "name": "Debug Rust Binary",
  "type": "lldb",
  "request": "launch",
  "cargo": {
    "args": ["build", "--bin=myapp"],
    "filter": {
      "kind": "bin"
    }
  },
  "args": [],
  "cwd": "${workspaceFolder}",
  "env": {
    "RUST_BACKTRACE": "1"
  },
  "sourceLanguages": ["rust"]
}
```

**Tips:**
- Enable `RUST_BACKTRACE=1` for better stack traces
- Use `cargo build` before debugging for faster startup
- Configure pretty printers for standard library types

### Python (debugpy)

**Installation:**
```bash
pip install debugpy
# or
pip3 install debugpy
```

**Configuration:**
```json
{
  "name": "Debug Python",
  "type": "debugpy",
  "request": "launch",
  "program": "${file}",
  "console": "integratedTerminal",
  "justMyCode": true,
  "env": {
    "PYTHONPATH": "${workspaceFolder}"
  }
}
```

**Django Configuration:**
```json
{
  "name": "Django",
  "type": "debugpy",
  "request": "launch",
  "program": "${workspaceFolder}/manage.py",
  "args": ["runserver", "--noreload"],
  "django": true
}
```

**Flask Configuration:**
```json
{
  "name": "Flask",
  "type": "debugpy",
  "request": "launch",
  "module": "flask",
  "args": ["run", "--no-debugger"],
  "env": {
    "FLASK_APP": "app.py",
    "FLASK_ENV": "development"
  }
}
```

### JavaScript / TypeScript (Node Debugger)

**Built-in Support:** No additional installation required for Node.js debugging.

**Node.js Configuration:**
```json
{
  "name": "Debug Node.js",
  "type": "node",
  "request": "launch",
  "program": "${workspaceFolder}/src/index.js",
  "cwd": "${workspaceFolder}",
  "runtimeExecutable": "node",
  "runtimeArgs": ["--inspect"],
  "console": "integratedTerminal"
}
```

**TypeScript Configuration:**
```json
{
  "name": "Debug TypeScript",
  "type": "node",
  "request": "launch",
  "program": "${workspaceFolder}/src/index.ts",
  "preLaunchTask": "tsc: build",
  "outFiles": ["${workspaceFolder}/dist/**/*.js"],
  "sourceMaps": true
}
```

**Attach to Running Process:**
```json
{
  "name": "Attach to Node",
  "type": "node",
  "request": "attach",
  "port": 9229,
  "restart": true,
  "localRoot": "${workspaceFolder}",
  "remoteRoot": "/app"
}
```

### Go (Delve)

**Installation:**
```bash
go install github.com/go-delve/delve/cmd/dlv@latest
```

**Configuration:**
```json
{
  "name": "Debug Go",
  "type": "go",
  "request": "launch",
  "mode": "auto",
  "program": "${workspaceFolder}",
  "args": [],
  "env": {},
  "showLog": true
}
```

**Debug Test:**
```json
{
  "name": "Debug Go Test",
  "type": "go",
  "request": "launch",
  "mode": "test",
  "program": "${workspaceFolder}/pkg/mypackage",
  "args": ["-test.v", "-test.run", "TestMyFunction"]
}
```

**Remote Debugging:**
```json
{
  "name": "Attach to Remote",
  "type": "go",
  "request": "attach",
  "mode": "remote",
  "remotePath": "/app",
  "port": 2345,
  "host": "127.0.0.1"
}
```

### C/C++ (GDB / LLDB)

**Installation:**
```bash
# GDB (Linux)
sudo apt install gdb

# LLDB (macOS - included with Xcode)
xcode-select --install

# LLDB (Linux)
sudo apt install lldb
```

**GDB Configuration:**
```json
{
  "name": "Debug C++ (GDB)",
  "type": "cppdbg",
  "request": "launch",
  "program": "${workspaceFolder}/build/myapp",
  "args": [],
  "cwd": "${workspaceFolder}",
  "environment": [],
  "MIMode": "gdb",
  "setupCommands": [
    {
      "text": "-enable-pretty-printing",
      "ignoreFailures": true
    }
  ]
}
```

**LLDB Configuration:**
```json
{
  "name": "Debug C++ (LLDB)",
  "type": "lldb",
  "request": "launch",
  "program": "${workspaceFolder}/build/myapp",
  "args": [],
  "cwd": "${workspaceFolder}",
  "initCommands": [
    "settings set target.x86-disassembly-flavor intel"
  ]
}
```

**CMake Project:**
```json
{
  "name": "Debug CMake Target",
  "type": "lldb",
  "request": "launch",
  "program": "${command:cmake.launchTargetPath}",
  "args": [],
  "cwd": "${workspaceFolder}",
  "preLaunchTask": "cmake: build"
}
```

---

## Launch Configurations

### Format and Fields

Launch configurations use a JSON format with the following common fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Display name for the configuration |
| `type` | string | Debug adapter type (lldb, debugpy, node, etc.) |
| `request` | string | "launch" or "attach" |
| `program` | string | Path to the executable or script |
| `args` | array | Command-line arguments |
| `cwd` | string | Working directory |
| `env` | object | Environment variables |
| `preLaunchTask` | string | Task to run before debugging |
| `postDebugTask` | string | Task to run after debugging |

### Variable Substitution

Configurations support variable substitution:

| Variable | Description |
|----------|-------------|
| `${workspaceFolder}` | Root folder of the workspace |
| `${file}` | Current open file |
| `${fileBasename}` | Current file name without path |
| `${fileDirname}` | Directory of current file |
| `${fileExtname}` | Extension of current file |
| `${env:VAR_NAME}` | Environment variable value |
| `${command:...}` | Result of a command |

### Complete Examples

**Full Rust Example:**
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug Rust Binary",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["build", "--bin=myapp", "--package=myapp"],
        "filter": {
          "name": "myapp",
          "kind": "bin"
        }
      },
      "args": ["--config", "dev.toml"],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_BACKTRACE": "1",
        "RUST_LOG": "debug"
      },
      "sourceLanguages": ["rust"],
      "preLaunchTask": "cargo: build"
    },
    {
      "name": "Debug Rust Tests",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["test", "--no-run", "--lib"],
        "filter": {
          "kind": "lib"
        }
      },
      "args": ["test_name"],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**Full Python Example:**
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Python: Current File",
      "type": "debugpy",
      "request": "launch",
      "program": "${file}",
      "console": "integratedTerminal",
      "justMyCode": false,
      "env": {
        "PYTHONPATH": "${workspaceFolder}/src"
      }
    },
    {
      "name": "Python: FastAPI",
      "type": "debugpy",
      "request": "launch",
      "module": "uvicorn",
      "args": ["main:app", "--reload", "--port", "8000"],
      "jinja": true,
      "env": {
        "DATABASE_URL": "postgresql://localhost/dev"
      }
    },
    {
      "name": "Python: Attach Remote",
      "type": "debugpy",
      "request": "attach",
      "connect": {
        "host": "localhost",
        "port": 5678
      },
      "pathMappings": [
        {
          "localRoot": "${workspaceFolder}",
          "remoteRoot": "/app"
        }
      ]
    }
  ]
}
```

**Full Node.js Example:**
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Node: Run Current File",
      "type": "node",
      "request": "launch",
      "program": "${file}",
      "console": "integratedTerminal",
      "skipFiles": ["<node_internals>/**"]
    },
    {
      "name": "Node: Run with npm",
      "type": "node",
      "request": "launch",
      "runtimeExecutable": "npm",
      "runtimeArgs": ["run", "dev"],
      "cwd": "${workspaceFolder}",
      "console": "integratedTerminal"
    },
    {
      "name": "Jest: Current Test File",
      "type": "node",
      "request": "launch",
      "runtimeExecutable": "npx",
      "runtimeArgs": [
        "jest",
        "${fileBasenameNoExtension}",
        "--runInBand",
        "--watchAll=false"
      ],
      "console": "integratedTerminal",
      "internalConsoleOptions": "neverOpen"
    }
  ]
}
```

---

## Inline Values (Planned)

### Overview

Inline values is a planned feature that will display variable values directly in the editor, adjacent to their occurrences in the source code. This provides immediate visibility into program state without navigating to the Variables panel.

### Tree-sitter Query Approach

RustIDE will use Tree-sitter queries to identify variable locations in source code, enabling accurate placement of inline value decorations.

**Query Example (Rust):**
```scheme
; Match variable declarations
(let_declaration
  pattern: (identifier) @variable.declaration)

; Match variable references
(identifier) @variable.reference

; Match function parameters
(parameter
  pattern: (identifier) @variable.parameter)

; Match struct field access
(field_expression
  value: (identifier) @variable.base
  field: (field_identifier) @variable.field)
```

### How It Will Work

1. **Breakpoint Hit:** When execution pauses, RustIDE queries the debug adapter for all in-scope variables.

2. **Source Analysis:** Tree-sitter parses the visible source code to identify variable locations.

3. **Value Matching:** Variable names from the debugger are matched to source locations.

4. **Decoration Rendering:** Inline decorations are rendered showing values:

```rust
fn process_order(order: Order) -> Result<Receipt, Error> {
    //              ▲
    //              └── order = Order { id: 42, items: [...], total: 99.99 }

    let discount = calculate_discount(&order);
    //  ▲
    //  └── discount = 0.15

    let final_price = order.total * (1.0 - discount);
    //  ▲                   ▲              ▲
    //  │                   │              └── discount = 0.15
    //  │                   └── order.total = 99.99
    //  └── final_price = 84.99

    Ok(Receipt::new(order.id, final_price))
}
```

### Configuration Options (Planned)

```json
{
  "debug.inlineValues": {
    "enabled": true,
    "maxLength": 50,
    "showTypes": false,
    "showOnHover": true,
    "excludePatterns": ["password", "secret", "token"]
  }
}
```

---

## Troubleshooting

### Common Issues and Solutions

#### Debugger Fails to Start

**Symptom:** "Failed to launch debug adapter" or similar error.

**Solutions:**
1. Verify the debug adapter is installed:
   ```bash
   # For codelldb
   which lldb-vscode || which codelldb

   # For debugpy
   python -c "import debugpy; print(debugpy.__file__)"

   # For delve
   which dlv
   ```

2. Check adapter path in settings:
   ```json
   {
     "debug.adapters.lldb.path": "/usr/local/bin/codelldb"
   }
   ```

3. Review the debug console for detailed error messages.

#### Breakpoints Not Being Hit

**Symptom:** Breakpoints appear but execution doesn't pause.

**Solutions:**
1. **Check build configuration:** Ensure you're debugging a debug build, not release:
   ```bash
   # Rust
   cargo build  # Not cargo build --release
   ```

2. **Verify source mapping:** Ensure source files match the compiled binary.

3. **Check breakpoint status:** Look for unverified breakpoints (hollow circles).

4. **Rebuild the project:** Source changes require recompilation.

#### Variables Show "Optimized Out"

**Symptom:** Variables display as `<optimized out>` or `<unavailable>`.

**Solutions:**
1. Use debug build configuration:
   ```toml
   # Cargo.toml
   [profile.dev]
   opt-level = 0
   debug = true
   ```

2. Disable compiler optimizations:
   ```bash
   # GCC/Clang
   -O0 -g
   ```

3. Some variables may genuinely be optimized; step to a different line.

#### Source File Mismatch

**Symptom:** Debugger shows wrong source file or stale code.

**Solutions:**
1. Rebuild the project to ensure binary matches source.

2. Check source path mappings:
   ```json
   {
     "sourceMap": {
       "/build/path": "${workspaceFolder}"
     }
   }
   ```

3. Clear any cached debug information.

#### Remote Debugging Connection Failed

**Symptom:** Cannot connect to remote debug server.

**Solutions:**
1. Verify the remote process is listening:
   ```bash
   # Check if debug port is open
   netstat -an | grep 9229
   ```

2. Check firewall rules allow the debug port.

3. Ensure correct host and port in configuration:
   ```json
   {
     "request": "attach",
     "host": "192.168.1.100",
     "port": 9229
   }
   ```

#### Performance Issues During Debugging

**Symptom:** IDE becomes slow or unresponsive while debugging.

**Solutions:**
1. Reduce the number of watched expressions.

2. Disable "Break on All Exceptions" if enabled.

3. Limit the depth of variable expansion:
   ```json
   {
     "debug.variableExpansionDepth": 3
   }
   ```

4. Close unnecessary panels during debugging.

### Debug Adapter Logs

Enable verbose logging for troubleshooting:

```json
{
  "debug.trace": "verbose",
  "debug.logFile": "${workspaceFolder}/.rustide/debug.log"
}
```

### Getting Help

If issues persist:

1. Check the [RustIDE Issues](https://github.com/rustide/rustide/issues) for known problems
2. Include debug adapter logs when reporting issues
3. Provide your launch configuration and steps to reproduce

---

## Keyboard Shortcuts

| Action | Shortcut (Default) |
|--------|-------------------|
| Start Debugging | `F5` |
| Stop Debugging | `Shift+F5` |
| Restart Debugging | `Ctrl+Shift+F5` |
| Step Over | `F10` |
| Step Into | `F11` |
| Step Out | `Shift+F11` |
| Continue | `F5` |
| Toggle Breakpoint | `F9` |
| Conditional Breakpoint | `Ctrl+Shift+F9` |
| Run to Cursor | `Ctrl+F10` |
| Show Debug Console | `Ctrl+Shift+Y` |

---

## See Also

- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [codelldb Documentation](https://github.com/vadimcn/codelldb)
- [debugpy Documentation](https://github.com/microsoft/debugpy)
- [Delve Documentation](https://github.com/go-delve/delve)
