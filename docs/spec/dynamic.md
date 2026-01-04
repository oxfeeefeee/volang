# Dynamic Access Specification

## Overview

Vo supports opt-in dynamic (duck-typing) operations on values whose structure is unknown at compile time.

Dynamic access is enabled by the `~>` operator on `any`/`interface` values. The `~>` operator does not introduce a new runtime representation: the base value is still a normal interface value.

Dynamic operations return `(any, error)` and are intended to compose with Vo's existing error handling (`?`, `fail`, `errdefer`). There is no error-carrying payload.

## Syntax

### Dynamic Access Operator (`~>`)

```go
// Field access
a~>field           // → (any, error)
a~>field?          // → any (propagate error)

// Chaining
a~>b~>c             // → (any, error) (implicit short-circuiting)
a~>b~>c?            // → any

// Field assignment
a~>field = value   // statement form: fail-on-error
a~>b~>c = value    // chained assignment target

// Indexing
a~>[key]           // → (any, error)
a~>[key]?          // → any (propagate error)

// Index assignment
a~>[key] = value   // statement form: fail-on-error

// Call
a~>(args...)       // → (any, error)
a~>(args...)?      // → any (propagate error)

// Method call (syntax sugar)
a~>method(args...) // → (any, error)
a~>method(args...)?// → any (propagate error)

// Type assertion (same as interface)
v := a.(T)         // → T (panic on failure)
v, ok := a.(T)     // → (T, bool) (ok=false on failure, no panic)
```

**Note**: Optional-chaining style operators like `?.` and `?[]` are not part of Vo syntax. Use postfix `?` on the result of each dynamic operation.

**Note**: The left operand of `~>` may have static type `any/interface` or `(any, error)`. If the left operand is `(any, error)`, `~>` implicitly short-circuits: if `error != nil`, the result is `(nil, error)`; otherwise the operation continues on the `any` value.

### Dynamic Operation Whitelist

Dynamic access does not participate in normal static member lookup or overload resolution. Only a small whitelist of operations is supported when using `~>`:

- Field access: `a~>field` (returns `(any, error)`)
- Field assignment: `a~>field = x` (statement; fail-on-error)
- Indexing: `a~>[key]` (returns `(any, error)`)
- Index assignment: `a~>[key] = x` (statement; fail-on-error)
- Call: `a~>(args...)` (returns `(any, error)`)
- Method call (syntax sugar): `a~>method(args...)` (returns `(any, error)`)
- Error propagation: apply postfix `?` to unwrap and propagate the `error`

All other dynamic operations are compile errors.

## Error Mechanism

### Error Generation

| Operation | Failure Condition | Error Type |
|-----------|-------------------|------------|
| `a~>field` | field doesn't exist | AttributeError |
| `a~>field` | value is not struct/map | TypeError |
| `a~>field = x` | field doesn't exist or is not assignable | AttributeError |
| `a~>field = x` | value is not mutable (not map and not pointer-to-struct) | TypeError |
| `a~>[key]` | value is not indexable | TypeError |
| `a~>[key] = x` | value is not mutable indexable (not map/slice/array) | TypeError |
| `a~>method(args...)` | method doesn't exist | AttributeError |
| `a~>method(args...)` | call failed | CallError |
| `a~>(args...)` | value is not callable | CallError |

## Runtime Representation

### Representation

Dynamic access operates on ordinary `any`/`interface` values.

There is no error-carrying payload. Errors are reported via the explicit `error` return value of dynamic operations.

## Implementation Details

### Type Checker Changes

```rust
// Dynamic access operator
// If the base has static type any/interface, dynamic access returns (any, error).
if base_type.is_any_or_interface() {
    return Type::Tuple(Type::Any, Type::Error);
}

// If the base has static type (any, error), dynamic access also returns (any, error)
// and short-circuits on the error component.
if base_type.is_tuple_any_error() {
    return Type::Tuple(Type::Any, Type::Error);
}
```

### Codegen Changes

```rust
// Dynamic access operations lower to runtime helper calls that return (value, error).
// Postfix `?` is regular Vo error-propagation sugar.
a~>field           → dyn_get_attr(a, "field")                 // (any, error)
a~>field?          → dyn_get_attr(a, "field")?                // any
a~>field = x       → dyn_set_attr(a, "field", x)?              // statement (fail-on-error)
a~>[key]           → dyn_get_index(a, any(key))                // (any, error)
a~>[key]?          → dyn_get_index(a, any(key))?               // any
a~>[key] = x       → dyn_set_index(a, any(key), x)?             // statement (fail-on-error)
a~>(args...)       → dyn_call(a, any(args...))                 // (any, error)
a~>(args...)?      → dyn_call(a, any(args...))?                // any

// Method call sugar
a~>method(args...) → dyn_call_method(a, "method", any(args...))  // (any, error)
```

### Chained Access Codegen (Short-circuiting)

When the left operand of `~>` has type `(any, error)`, the generated code must short-circuit:

```rust
// a~>b~>c where a has type `any`
// Desugars to:
{
    let (v1, e1) = dyn_get_attr(a, "b")
    if e1 != nil {
        (nil, e1)  // short-circuit
    } else {
        dyn_get_attr(v1, "c")
    }
}

// Equivalently, using a helper:
dyn_chain(dyn_get_attr(a, "b"), |v| dyn_get_attr(v, "c"))

// where dyn_chain is:
fn dyn_chain(base: (any, error), op: fn(any) -> (any, error)) -> (any, error) {
    let (v, e) = base
    if e != nil {
        return (nil, e)
    }
    op(v)
}
```

### Difference: `a~>b~>c` vs `a~>b?~>c`

| Expression | On `a~>b` failure | Result type |
|------------|-------------------|-------------|
| `a~>b~>c` | Short-circuit, return `(nil, err)` | `(any, error)` |
| `a~>b?~>c` | `?` triggers `fail`, function exits | `(any, error)` |

The first form collects errors; the second form fails early.

### nil Interface

If the base value is a nil interface, dynamic access returns `TypeError("cannot access on nil")`.

```go
var a any = nil
v, err := a~>field   // err is TypeError
```

### Runtime Builtins

```rust
fn dyn_get_attr(d: any, name: &str) -> (any, error) {
    match get_field(d, name) {
        Ok(v) => (v, nil),
        Err(e) => (any(nil), e),
    }
}

fn dyn_get_index(d: any, key: any) -> (any, error) {
    match get_index(d, key) {
        Ok(v) => (v, nil),
        Err(e) => (any(nil), e),
    }
}

fn dyn_call(callee: any, args: any) -> (any, error) {
    // If callee returns multiple values, the result is []any.
    // If callee returns a single value, the result is that value.
    // If callee returns nothing, the result is nil.
    match call(callee, args) {
        Ok(v) => (v, nil),
        Err(e) => (any(nil), e),
    }
}

// dyn_assert not needed - uses standard interface type assertion

fn dyn_set_attr(d: any, name: &str, val: any) -> error {
    set_field(d, name, val)
}

fn dyn_set_index(d: any, key: any, val: any) -> error {
    set_index(d, key, val)
}

fn dyn_call_method(d: any, method: &str, args: any) -> (any, error) {
    match get_method(d, method) {
        Ok(m) => call(m, args),
        Err(e) => (any(nil), e),
    }
}
```

### User-facing Helper APIs

The runtime helpers used for lowering are also exposed as user-facing APIs (stdlib or compiler built-ins). They are intended for advanced usage, library authors, and cases where the caller needs to explicitly handle errors.

- `dyn_get_attr(base any, name string) -> (any, error)`
- `dyn_set_attr(base any, name string, value any) -> error`
- `dyn_get_index(base any, key any) -> (any, error)`
- `dyn_set_index(base any, key any, value any) -> error`
- `dyn_call(callee any, args []any) -> (any, error)`
- `dyn_call_method(base any, method string, args []any) -> (any, error)`

## Examples

### Basic Usage

```go
func processJSON(data any) (string, error) {
    a := data
     
    // Chained operations, each step is checked
    userName := (a~>response~>data~>user~>name?).(string)
    userAge := (a~>response~>data~>user~>age?).(int)
    greeting := (a~>config~>greeting?).(string)

    // Dynamic set (single-step). Statement form is fail-on-error.
    a~>last_user = userName
    a~>["last_age"] = userAge
    
    // Complex expression
    message := greeting + " " + userName
    
    return message, nil
}
```

### Dynamic set with explicit error handling

```go
func updateCount(data any) error {
    a := data

    // Dynamic set is statement-only.
    // Use the helper APIs when explicit error handling is needed.
    err := dyn_set_attr(a, "count", 1)
    err?

    err = dyn_set_index(a, "count", 2)
    err?

    return nil
}
```

### Chained assignment

```go
func chainedSet(a any, v any) error {
    // Chained assignment is supported.
    // Its meaning is: evaluate `a~>a` (short-circuit on error), then set `b` on the result.
    a~>a~>b = v
    return nil
}
```

**Warning**: Chained assignment does not guarantee "path write-back" if intermediate results are value copies. For example, if `a~>a` returns a struct by value, setting `b` on it will not modify the original. To ensure mutation, the intermediate value must be a pointer or a reference type (map, slice).

### Safe Access with explicit error

```go
func safeGet(data any, field string) (any, error) {
    v, err := dyn_get_attr(data, field)
    if err != nil {
        return nil, err
    }
    return v, nil
}
```

## Design Summary

| Aspect | Design |
|--------|--------|
| Entry Point | `~>` operator on `any`/`interface` |
| Result Type | Dynamic operations return `(any, error)` |
| Error Handling | No error-carrying payload; check per-step via `?` |
| Runtime Layout | Uses ordinary `any`/`interface` values |
| Semantics | Whitelist + compile-time desugaring to helper calls |
