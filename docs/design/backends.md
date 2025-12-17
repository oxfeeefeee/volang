# GoX Compilation Backends

This document describes the compilation backend architecture for GoX.

## Overview

GoX supports multiple compilation backends to target different execution environments:

```
                     ┌─────────────────┐
                     │   gox-syntax    │
                     │   gox-analysis  │
                     └────────┬────────┘
                              │
                     ┌────────▼────────┐
                     │  gox-codegen-vm │  ← Bytecode generation
                     └────────┬────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
   ┌─────────┐        ┌─────────────┐       ┌─────────────┐
   │ gox-vm  │        │   gox-jit   │       │ gox-aot  │
   │ (interp)│        │ (Cranelift) │       │ (Cranelift) │
   └─────────┘        └─────────────┘       └──────┬──────┘
        │                     │                    │
        ▼                     ▼              ┌─────┴─────┐
    Interpret            JIT Native          ▼           ▼
                                          Mach-O       WASM
```

## Backend Comparison

| Backend | Use Case | Startup | Peak Perf | Distribution |
|---------|----------|---------|-----------|--------------|
| `gox-vm` | Development, debugging | Fast | Low | Bytecode |
| `gox-jit` | Long-running apps | Medium | High | Bytecode |
| `gox-aot` | Production deployment | Instant | High | Executable |
| `gox-wasm` | Web, sandboxed envs | Fast | Medium | .wasm file |

## 1. VM Interpreter (`gox-vm`)

Pure bytecode interpreter. No native code generation.

**Advantages:**
- Fastest startup (no compilation)
- Full debugging support
- Platform independent

**Usage:**
```bash
gox run main.gox
```

## 2. JIT Compiler (`gox-jit`)

Transparent JIT compilation using Cranelift. Hot functions are compiled to native code at runtime.

**How it works:**
1. Start with VM interpretation
2. Count function calls
3. When threshold reached (default: 1000), compile to native
4. Replace interpreted call with native call

**Advantages:**
- Balances startup time and peak performance
- Adaptive optimization based on runtime behavior

**Usage:**
```bash
gox run --jit main.gox
```

## 3. Native Compiler (`gox-aot`)

AOT (Ahead-Of-Time) compilation to standalone executables.

**Supported targets:**
- macOS (Mach-O): x86_64, ARM64
- Linux (ELF): x86_64, ARM64 (planned)
- Windows (PE): x86_64 (planned)

**Output:**
- Object files (.o)
- Linked with `gox-runtime-core` to produce executable

**Usage:**
```bash
gox build main.gox -o main
./main
```

## 4. WebAssembly Compiler (`gox-wasm`)

Compilation to WebAssembly for browser or WASI environments.

**Formats:**
- Standalone WASM (for browsers)
- WASI-compatible (for server-side/CLI)

**Usage:**
```bash
gox build --wasm main.gox -o main.wasm
```

## Implementation Status

| Backend | Status | Notes |
|---------|--------|-------|
| `gox-vm` | ✅ Complete | Production ready |
| `gox-jit` | 🚧 Scaffold | Cranelift integration pending |
| `gox-aot` | 🚧 Scaffold | Cranelift integration pending |
| `gox-wasm` | 🚧 Scaffold | Cranelift integration pending |

## Cranelift Integration

All native backends use [Cranelift](https://cranelift.dev/) for code generation:

```toml
# Shared dependencies
cranelift-codegen = "0.113"
cranelift-frontend = "0.113"

# JIT-specific
cranelift-jit = "0.113"

# AOT-specific  
cranelift-object = "0.113"

# WASM-specific
cranelift-wasm = "0.113"
```

### Bytecode → Cranelift IR Translation

Each VM opcode maps to Cranelift IR instructions:

| VM Opcode | Cranelift IR |
|-----------|--------------|
| `Add` | `iadd` |
| `Sub` | `isub` |
| `Mul` | `imul` |
| `Call` | `call` |
| `Jump` | `jump` |
| `JumpIf` | `brif` |
| `Return` | `return` |

## Runtime Requirements

All backends require the GoX runtime for:
- Garbage collection
- Goroutine scheduling  
- Built-in functions (`println`, `make`, etc.)
- Channel operations

For native/WASM backends, the runtime is statically linked into the output.
