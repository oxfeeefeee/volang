# GoX Programming Language

GoX is a statically typed, Go-like programming language with multiple backend targets.

## Overview

GoX aims to provide familiar Go syntax while introducing explicit reference semantics through the `object` type and supporting multiple compilation backends.

### Key Features

- **Go-like syntax** - Familiar to Go programmers
- **Static typing** with local type inference
- **Explicit memory model** - Clear distinction between value types (`struct`) and object types (`object`)
- **Multiple backends** - LLVM, WebAssembly, and a custom VM
- **No generics** - Simplified type system
- **No pointers** - Reference semantics through `object` types

## Project Structure

```
gox/
├── crates/
│   ├── gox-common/        # Shared infrastructure: source management, diagnostics, symbols
│   ├── gox-syntax/        # Lexer, AST definitions, and parser
│   ├── gox-analysis/      # Semantic analysis, type checking, name resolution
│   ├── gox-codegen-llvm/  # LLVM IR code generation backend
│   ├── gox-codegen-wasm/  # WebAssembly code generation backend
│   ├── gox-codegen-vm/    # Custom VM bytecode generation
│   ├── gox-vm/            # Custom VM runtime and interpreter
│   └── gox-cli/           # Command-line interface tool
├── english/
│   └── language_spec.md   # Language specification
└── tests/                 # Integration tests and test fixtures
```

## Crate Dependencies

```
                    ┌─────────────┐
                    │  gox-cli    │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
┌─────────────────┐ ┌─────────────┐ ┌─────────────────┐
│ gox-codegen-llvm│ │gox-codegen- │ │ gox-codegen-vm  │
└────────┬────────┘ │    wasm     │ └────────┬────────┘
         │          └──────┬──────┘          │
         │                 │                 │
         └─────────────────┼─────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │gox-analysis │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ gox-syntax  │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ gox-common  │
                    └─────────────┘

                    ┌─────────────┐
                    │   gox-vm    │ (standalone runtime)
                    └─────────────┘
```

## Building

```bash
cargo build --release
```

## Usage

```bash
# Compile a GoX program
gox build program.gox

# Run a GoX program (using VM backend)
gox run program.gox

# Compile to specific backend
gox build --target=llvm program.gox
gox build --target=wasm program.gox
gox build --target=vm program.gox
```

## Language Example

```gox
package main

type User struct {
    name string
    age  int
}

type UserRef object {
    name string
    age  int
}

interface Greeter {
    Greet() string
}

func (u User) Greet() string {
    return "Hello, " + u.name
}

func main() int {
    user := User{name: "Alice", age: 30}
    println(user.Greet())
    
    var ref UserRef = UserRef{name: "Bob", age: 25}
    ref.name = "Charlie"  // modifies the object
    
    numbers := []int{1, 2, 3}
    for i, v := range numbers {
        println(i, v)
    }
    
    return 0
}
```

## License

MIT License - see [LICENSE](LICENSE) for details.
