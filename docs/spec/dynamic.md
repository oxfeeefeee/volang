# Dynamic Type Specification

## Overview

The `dynamic` type is a built-in type that enables dynamic (duck-typing) operations on values whose structure is unknown at compile time. It is an **error-carrying interface** — essentially an interface that can hold an error state from failed dynamic operations.

## Type Definition

```
dynamic = error-carrying interface
```

- **Independent built-in type**, parallel to `int`, `interface`, `struct`
- **Behavior similar to interface**, but supports dynamic field access, indexing, and operators
- **May carry an error** — when operations fail, the error propagates through subsequent operations

## Syntax

### Entering Dynamic Mode

```go
// Method 1: Implicit assignment (any value → dynamic)
var d dynamic = someValue
var d dynamic = someAny

// Method 2: Type conversion (for short declarations)
d := dynamic(someValue)
d := dynamic(someAny)
```

**Note**: `.(dynamic)` is NOT supported. This is not a type assertion (not extracting a concrete type from interface).

### Operations Inside Dynamic

```go
// Field access
d.field           // → dynamic

// Indexing
d[key]            // → dynamic

// Method call (arguments must be dynamic)
d.method(d2, d3)  // → dynamic

// Operators (both operands must be dynamic)
d1 + d2           // → dynamic
d1 == d2          // → dynamic
```

**Error propagation**: If an operand has an error, the result inherits the error.

### Exiting Dynamic Mode

```go
// Type assertion (checks error + type)
s := d.(string)          // error or type mismatch → panic
s, ok := d.(string)      // error or type mismatch → ok=false
s := d.(string)?         // error or type mismatch → fail propagation

// Convert back to any
a := d.(any)             // error → panic
a, ok := d.(any)         // error → ok=false
a := d.(any)?            // error → fail propagation
```

### Type Strictness

```go
// ❌ dynamic cannot mix with other types
d + 10                   // compile error
d.field + "hello"        // compile error

// ✅ both sides must be dynamic
d + dynamic(10)          // OK
d.field + dynamic("hello")  // OK
```

## Assignment Rules

| Direction | Allowed | Syntax |
|-----------|---------|--------|
| `T → dynamic` | ✅ implicit | `var d dynamic = x` |
| `any → dynamic` | ✅ implicit | `var d dynamic = a` |
| `dynamic → dynamic` | ✅ direct | `d2 = d1` |
| `dynamic → any` | ❌ explicit required | `a = d.(any)` |
| `dynamic → T` | ❌ explicit required | `t = d.(T)` |

## Error Mechanism

### Error Generation

| Operation | Failure Condition | Error Type |
|-----------|-------------------|------------|
| `d.field` | field doesn't exist | AttributeError |
| `d.field` | value is not struct/map | TypeError |
| `d[key]` | value is not indexable | TypeError |
| `d1 + d2` | types incompatible | TypeError |
| `d.method()` | method doesn't exist | AttributeError |
| `d.method()` | argument mismatch | CallError |
| `d()` | value is not callable | CallError |

### Error Propagation

```go
d := dynamic(data)
x := d.user           // if failed, x carries error
y := x.name           // if x has error, y inherits error (actual .name not executed)
z := y + d.suffix     // error continues to propagate
```

### Error Checking

Errors are only checked when **exiting dynamic mode**:
- `d.(T)` — panic on error
- `d.(T, ok)` — ok=false on error
- `d.(T)?` — fail propagation on error

## Runtime Representation

### Layout (reuses interface layout)

```
// Normal state (same as interface)
slot0: [itab_id:32 | value_meta:32]
slot1: data

// Error state (special marker)
slot0: [error_kind:8 | reserved:24 | ERROR_MARKER:32]  // ERROR_MARKER = 0xFFFFFFFF
slot1: error_message (GcRef to string)
```

### Error Kind

```rust
enum DynamicErrorKind {
    Attribute = 1,  // field/method doesn't exist
    Type = 2,       // type incompatible
    Call = 3,       // call failed
}
```

## Implementation Details

### Type Checker Changes

```rust
// New type kind
TypeKind::Dynamic

// Selector expression
if base_type.is_dynamic() {
    // No static field lookup, directly return dynamic
    return Type::Dynamic;
}

// Binary expression
if lhs.is_dynamic() && rhs.is_dynamic() {
    return Type::Dynamic;
} else if lhs.is_dynamic() || rhs.is_dynamic() {
    // Error: dynamic cannot mix with other types
    error!("mismatched types");
}

// Type assertion
if base.is_dynamic() && target == Type::Dynamic {
    error!("cannot assert dynamic to dynamic");
}
```

### Codegen Changes

```rust
// Dynamic operations lower to runtime calls
d.field     →  dyn_get_attr(d, "field") → dynamic
d[key]      →  dyn_get_index(d, key) → dynamic
d1 + d2     →  dyn_binop(Add, d1, d2) → dynamic
d.method()  →  dyn_call_method(d, "method", args) → dynamic

// Type assertion
d.(T)       →  dyn_unwrap(d) + type_assert(T)
d.(T)?      →  dyn_unwrap_or_fail(d) + type_assert(T)
```

### Runtime Builtins

```rust
fn dyn_get_attr(d: Dynamic, name: &str) -> Dynamic {
    if d.is_error() { return d; }  // error propagation
    
    let value = d.value();
    match get_field(value, name) {
        Ok(v) => Dynamic::ok(v),
        Err(e) => Dynamic::err(DynamicErrorKind::Attribute, e.message),
    }
}

fn dyn_binop(op: BinOp, d1: Dynamic, d2: Dynamic) -> Dynamic {
    if d1.is_error() { return d1; }
    if d2.is_error() { return d2; }
    
    match apply_op(op, d1.value(), d2.value()) {
        Ok(v) => Dynamic::ok(v),
        Err(e) => Dynamic::err(DynamicErrorKind::Type, e.message),
    }
}

fn dyn_unwrap(d: Dynamic) -> any {
    if d.is_error() { panic(d.error_message()); }
    d.value()
}

fn dyn_unwrap_or_fail(d: Dynamic) -> any {
    if d.is_error() { fail(d.to_error()); }
    d.value()
}
```

## Examples

### Basic Usage

```go
func processJSON(data any) (string, error) {
    // Enter dynamic mode
    d := dynamic(data)
    
    // Chained operations, error auto-propagates
    userName := d.response.data.user.name
    userAge := d.response.data.user.age
    greeting := d.config.greeting
    
    // Complex expression
    message := greeting + dynamic(" ") + userName
    
    // Exit dynamic mode, propagate error
    return message.(string)?, nil
}
```

### Safe Access with comma-ok

```go
func safeGet(data any, field string) (any, bool) {
    d := dynamic(data)
    result := d[field]
    
    // comma-ok check
    if v, ok := result.(any); ok {
        return v, true
    }
    return nil, false
}
```

### Arithmetic Operations

```go
func calculate(data any) (int, error) {
    d := dynamic(data)
    
    // All operands must be dynamic
    sum := d.a + d.b + d.c
    
    // Exit and convert type
    return sum.(int)?, nil
}
```

## Design Summary

| Aspect | Design |
|--------|--------|
| Type Position | Independent built-in type |
| Enter Mode | Implicit assignment or `dynamic(x)` |
| Exit Mode | `.(T)` type assertion |
| Error Handling | Propagates inside, checks at boundary |
| Runtime Layout | Reuses interface's 2 slots |
| Type Strictness | dynamic cannot mix with other types |
