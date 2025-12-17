# GoX Standard Library Implementation

This document describes the implementation architecture and design principles of the GoX standard library.

## Architecture Overview

GoX stdlib functions are implemented in GoX source code (`stdlib/*/xxx.gox`). 
Each function falls into one of three categories:

```
                         GoX Source Code
                       (stdlib/*/xxx.gox)
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  Pure GoX     │    │ GoX + Native  │    │  Pure Native  │
│               │    │               │    │  (no body)    │
│ func F() {    │    │ func F() {    │    │               │
│   // GoX only │    │   // GoX code │    │ func F()      │
│ }             │    │   native()    │    │               │
│               │    │ }             │    │               │
│ e.g.          │    │ e.g.          │    │ e.g.          │
│ HasPrefix     │    │ Contains      │    │ Index         │
│ Compare       │    │ ReplaceAll    │    │ ToLower       │
└───────────────┘    └───────────────┘    └───────────────┘
                              │                     │
                              └──────────┬──────────┘
                                         ▼
                        ┌────────────────────────────────┐
                        │     Native Layer (Rust)        │
                        │  ┌──────────────────────────┐  │
                        │  │       Core Layer         │  │
                        │  │  gox-runtime-core/stdlib │  │
                        │  │      Pure logic          │  │
                        │  └────────────┬─────────────┘  │
                        │   ┌───────────┴───────────┐    │
                        │   ▼                       ▼    │
                        │ ┌─────────┐     ┌─────────┐    │
                        │ │VM Bind  │     │ C ABI   │    │
                        │ │NativeCtx│     │Cranelift│    │
                        │ └─────────┘     └─────────┘    │
                        └────────────────────────────────┘
```

## Design Principles

### 1. GoX-First Principle

Prefer implementing functions in GoX source code whenever possible:

```gox
// stdlib/strings/strings.gox
func HasPrefix(s, prefix string) bool {
    return len(s) >= len(prefix) && s[:len(prefix)] == prefix
}

func Contains(s, substr string) bool {
    return Index(s, substr) >= 0  // calls native Index
}
```

### 2. Native Function Categories

Only two categories require Native (Rust) implementation:

| Category | Reason | Examples |
|----------|--------|----------|
| **Cannot implement in GoX** | Requires runtime/system capabilities | `time.Now`, `os.ReadFile`, Unicode tables |
| **Performance critical** | GoX implementation would be slow | String search algorithms, regex, sorting |

### 3. Core Layer Responsibilities

Core layer (`gox-runtime-core/src/stdlib/`) contains:
- Pure business logic
- No `NativeCtx` or `GcRef` dependencies
- Reusable by both VM and Cranelift backends

```rust
// gox-runtime-core/src/stdlib/strings.rs
pub fn index(s: &str, substr: &str) -> i64 {
    s.find(substr).map(|i| i as i64).unwrap_or(-1)
}
```

### 4. VM Binding Layer Responsibilities

VM binding layer (`gox-runtime-vm/src/stdlib/`) only does:
1. Read arguments from `NativeCtx`
2. Call Core layer function
3. Write result to `NativeCtx`

```rust
// gox-runtime-vm/src/stdlib/core/strings.rs
fn native_index(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    let substr = ctx.arg_str(1);
    ctx.ret_i64(0, gox_runtime_core::stdlib::strings::index(s, substr));
    NativeResult::Ok(1)
}
```

## Module Listing

### Core Layer Modules (`gox-runtime-core/src/stdlib/`)

| Module | Functionality | Function Count |
|--------|---------------|----------------|
| `strings.rs` | String search/transform | 10 |
| `bytes.rs` | Byte slice operations | 12 |
| `strconv.rs` | String/number conversion | 7 |
| `time.rs` | Time operations | 6 |
| `rand.rs` | Random number generation | 8 |
| `hex.rs` | Hex encoding/decoding | 4 |
| `base64.rs` | Base64 encoding/decoding | 4 |
| `unicode.rs` | Unicode character classification | 14 |
| `json.rs` | JSON validation/escaping | 4 |
| `fmt.rs` | Formatted output | - |
| `builtin.rs` | Built-in functions | - |

### VM Binding Layer (`gox-runtime-vm/src/stdlib/`)

#### core/ (Core packages, no OS dependency)
| Module | Description |
|--------|-------------|
| `strings.rs` | Calls Core layer strings |
| `bytes.rs` | Calls Core layer bytes |
| `strconv.rs` | Calls Core layer strconv |
| `math.rs` | Inline FPU operations |
| `unicode.rs` | Calls Core layer unicode |
| `hex.rs` | Calls Core layer hex |
| `base64.rs` | Calls Core layer base64 |
| `json.rs` | Calls Core layer json |
| `sort.rs` | Direct GC slice operations |
| `errors.rs` | Minimal error creation |

#### std/ (Standard packages, requires OS support)
| Module | Description |
|--------|-------------|
| `time.rs` | Calls Core layer time |
| `rand.rs` | Calls Core layer rand |
| `os.rs` | System calls |
| `path.rs` | Path operations |
| `regexp.rs` | Regular expressions |
| `fmt.rs` | Formatted output |

### GoX Implementations (`stdlib/`)

| Package | File | GoX-implemented Functions |
|---------|------|---------------------------|
| `strings` | `strings.gox` | HasPrefix, HasSuffix, Contains, Compare, Repeat, TrimPrefix, TrimSuffix, ReplaceAll |
| `bytes` | `bytes.gox` | Equal, Compare, HasPrefix, HasSuffix, Contains, TrimPrefix, TrimSuffix |
| `strconv` | `strconv.gox` | FormatBool, ParseBool |
| `math` | `math.gox` | Abs, Max, Min, Dim |

## Special Cases

### 1. Inline Functions (math)

Simple FPU operations don't need Core layer, call Rust f64 methods directly in VM binding:

```rust
fn native_sqrt(ctx: &mut NativeCtx) -> NativeResult {
    ctx.ret_f64(0, ctx.arg_f64(0).sqrt());
    NativeResult::Ok(1)
}
```

### 2. GC Object Operations (sort)

Functions that need direct GC object access stay in VM layer:

```rust
fn native_sort_ints(ctx: &mut NativeCtx) -> NativeResult {
    let slice_ref = ctx.arg_ref(0);
    // read -> sort -> write back
    let mut values: Vec<i64> = (0..slice::len(slice_ref))
        .map(|i| slice::get(slice_ref, i) as i64)
        .collect();
    values.sort();
    for (i, &v) in values.iter().enumerate() {
        slice::set(slice_ref, i, v as u64);
    }
    NativeResult::Ok(0)
}
```

### 3. System Calls (os, time)

OS-dependent functions must be in Native layer:

```rust
// Core layer
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos() as i64
}
```

## C ABI Export (Cranelift)

`gox-runtime-core/src/ffi.rs` provides C ABI for Cranelift backend:

```rust
#[no_mangle]
pub unsafe extern "C" fn gox_strings_index(s: GcRef, substr: GcRef) -> i64 {
    use crate::objects::string;
    crate::stdlib::strings::index(string::as_str(s), string::as_str(substr))
}
```

## Adding New Functions Guide

### 1. Determine Implementation Approach

```
Can it be implemented in GoX?
  ├─ Yes → Implement in stdlib/pkg/pkg.gox
  └─ No  → Does it need system calls or GC object access?
            ├─ Yes → Implement in VM layer
            └─ No  → Implement in Core layer + VM binding
```

### 2. GoX Implementation

```gox
// stdlib/strings/strings.gox
func NewFunc(args...) RetType {
    // Can call native functions in the same package
    return nativeHelper(...)
}
```

### 3. Core Layer Implementation

```rust
// gox-runtime-core/src/stdlib/pkg.rs
pub fn new_func(args...) -> RetType {
    // Pure logic, no GC dependency
}
```

### 4. VM Binding

```rust
// gox-runtime-vm/src/stdlib/core/pkg.rs
fn native_new_func(ctx: &mut NativeCtx) -> NativeResult {
    let arg = ctx.arg_xxx(0);
    let result = gox_runtime_core::stdlib::pkg::new_func(arg);
    ctx.ret_xxx(0, result);
    NativeResult::Ok(1)
}

// Add to register()
registry.register("pkg.NewFunc", native_new_func);
```

### 5. C ABI (Optional, for Cranelift)

```rust
// gox-runtime-core/src/ffi.rs
#[no_mangle]
pub unsafe extern "C" fn gox_pkg_new_func(args...) -> RetType {
    crate::stdlib::pkg::new_func(...)
}
```
