# Unified Diagnostic System

## Goal

Consistent, beautiful, non-redundant error output across all compilation and runtime stages.

## Architecture

### Source File Path Design

**Problem**: Runtime errors need to display source code, but:
1. `DebugInfo` stores relative paths (e.g., `bitwise_ops.vo`)
2. Runtime doesn't know where the source files are

**Solution**: CLI provides source root at runtime

```
Bytecode (portable):
  debug_info.file: "bitwise_ops.vo"  # relative path only

CLI runtime:
  source_root: PathBuf from compile_source()
  RealFs::with_root(source_root)  # resolves relative paths

Display:
  bitwise_ops.vo:23:2  # relative path in output
```

**Key principle**: Bytecode contains no absolute paths - it's portable.
The CLI provides the source root at runtime for pretty print.

### URI Scheme (RFC 8089)

| Scheme | Usage | Example |
|--------|-------|---------|
| `file://` | Local filesystem | `file:///Users/wuhao/test.vo` |
| `zip://` | Zip archive (future) | `zip://archive.zip!/src/main.vo` |
| `mem://` | In-memory (future) | `mem://test.vo` |

### Output Tags (with error source)

```
[VO:OK]                           # Success
[VO:PARSE:loc: msg]               # Parse error
[VO:CHECK:loc: msg]               # Type check error
[VO:CODEGEN:loc: msg]             # Code generation error
[VO:PANIC:loc: msg]               # Runtime panic
[VO:IO:msg]                       # IO error
```

### Command Return Type

```rust
pub fn run(...) -> bool    // true=success, false=failed (error already reported)

// main.rs - simple exit handling
if !success { process::exit(1); }
```

## Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `Diagnostic` | vo-common/diagnostics.rs | Error representation |
| `DiagnosticEmitter` | vo-common/diagnostics.rs | Pretty print (codespan-reporting) |
| `SourceMap` | vo-common/source.rs | Compile-time source management |
| `SourceProvider` | vo-common-core/source_provider.rs | On-demand source reading |
| `RealFs` | vo-common/vfs.rs | File system with root path |
| `SourceLoc` | vo-common-core/debug_info.rs | Location info (file, line, col, len) |
| `DebugInfo` | vo-common-core/debug_info.rs | Runtime debug info |
| `ErrorKind` | vo-cli/output.rs | Error source categorization |

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     Compile Time                                 │
├─────────────────────────────────────────────────────────────────┤
│  FileSet.root = "/Users/.../test_data"                          │
│       ↓                                                          │
│  codegen: debug_info.file = "bitwise_ops.vo" (relative)         │
│       ↓                                                          │
│  compile_source() returns (Module, source_root: PathBuf)        │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                     Runtime Error                                │
├─────────────────────────────────────────────────────────────────┤
│  VmError → DebugInfo.lookup() → SourceLoc { file: "bitwise.vo" }│
│       ↓                                                          │
│  RealFs::with_root(source_root).read_source("bitwise.vo")       │
│       = read_to_string("/Users/.../test_data/bitwise_ops.vo")   │
│       ↓                                                          │
│  Pretty print with source snippet                                │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                         Output                                   │
├─────────────────────────────────────────────────────────────────┤
│  error: index out of bounds                                      │
│    ┌─ bitwise_ops.vo:23:2                                       │
│    │                                                             │
│ 23 │     arr[100]                                                │
│    │     ^^^^^^^^                                                │
│  [VO:PANIC:bitwise_ops.vo:23:2: index out of bounds]            │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation

### 1. RealFs with root path support

```rust
// vo-common/src/vfs.rs
pub struct RealFs {
    root: Option<PathBuf>,  // Root for resolving relative paths
}

impl RealFs {
    pub fn new() -> Self { Self { root: None } }
    pub fn with_root(root: impl Into<PathBuf>) -> Self { 
        Self { root: Some(root.into()) } 
    }
}

impl SourceProvider for RealFs {
    fn read_source(&self, path: &str) -> Option<String> {
        let full_path = match &self.root {
            Some(root) => root.join(path),
            None => PathBuf::from(path),
        };
        std::fs::read_to_string(&full_path).ok()
    }
}
```

### 2. compile_source returns source_root

```rust
// vo-cli/src/commands/run.rs
fn compile_source(file: &str, std_mode: StdMode) -> Option<(Module, PathBuf)> {
    // ...
    let source_root = file_set.root.clone();
    // ...
    Some((module, source_root))
}
```

### 3. Runtime error with source_root

```rust
// vo-cli/src/commands/run.rs
fn report_vm_error(vm: &Vm, e: &VmError, source_root: &Path) -> bool {
    let loc = debug_info.lookup(func_id, pc)?;
    let fs = RealFs::with_root(source_root);
    render_error(loc.as_ref(), "panic", msg, &fs);
    println!("{}", format_tag(ErrorKind::Panic, loc.as_ref(), msg));
    false
}

fn run_vm(module: Module, source_root: &Path) -> bool {
    // ...
    Err(e) => report_vm_error(&vm, &e, source_root),
}
```

### 4. ErrorKind enum

```rust
// vo-cli/src/output.rs
pub enum ErrorKind {
    Parse,    // [VO:PARSE:...]
    Check,    // [VO:CHECK:...]
    Codegen,  // [VO:CODEGEN:...]
    Panic,    // [VO:PANIC:...]
    Io,       // [VO:IO:...]
}
```

## Expected Output

```bash
# Parse error
$ vo run test.vo
error[E1101]: expected ';', found IDENT
   ┌─ test.vo:26:5
   │
26 │     as sert(x == 1)
   │        ^^^^
[VO:PARSE:parse failed: 1 error(s)]

# Type check error
$ vo run test.vo
error[E2001]: type mismatch
   ┌─ test.vo:10:5
   │
10 │     x := "hello"
   │     ^^^^^^^^^^^^ expected int, found string
[VO:CHECK:type check failed: 1 error(s)]

# Runtime panic (with source)
$ vo run test.vo
error: assertion failed: & basic
   ┌─ bitwise_ops.vo:23:2
   │
23 │     assert(c == 0b10100, "& basic")
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
[VO:PANIC:bitwise_ops.vo:23:2: assertion failed: & basic]

# Runtime panic (no source - bytecode only)
$ vo run test.vob
test.vo:23:2: panic: index out of bounds
[VO:PANIC:test.vo:23:2: index out of bounds]
```

## Benefits

1. **Portable bytecode** - `debug_info` stores relative paths only
2. **Runtime source resolution** - CLI provides source root at runtime
3. **Extensible** - ZipFs implemented, future support for more SourceProviders
4. **Clean error flow** - Commands return `bool`, errors reported at source
5. **Categorized errors** - Tags show error type (PARSE/CHECK/CODEGEN/PANIC/IO)
6. **Pretty print** - Source code snippets shown when source available

## ZipFs Support

### Implementation

```rust
// vo-common/src/vfs.rs
pub struct ZipFs {
    files: HashMap<PathBuf, String>,  // relative path -> content
}

impl ZipFs {
    pub fn from_path(path: &Path) -> io::Result<Self>;
    pub fn from_reader<R: Read + Seek>(reader: R) -> io::Result<Self>;
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self>;
}

// Implements both FileSystem and SourceProvider with consistent behavior
impl FileSystem for ZipFs { ... }
impl SourceProvider for ZipFs { ... }
```

### Usage

```bash
# Run single-file project from zip
vo run project.zip
```

### Multi-file Zip Support

Multi-file projects with imports are now fully supported:

```bash
# Run multi-file project from zip root
vo run project.zip

# Run project from subdirectory within zip
vo run "project.zip:src/"
```

The `PackageResolver::with_fs(zip_fs)` creates a resolver where all three package sources (StdSource, LocalSource, ModSource) share the same ZipFs instance.
