# MM Phase 1: Escape Analysis

**Parent**: [2025-12-22-mm-memory-model-plan.md](2025-12-22-mm-memory-model-plan.md)  
**Status**: Not Started  
**Est. Lines**: ~500

## Overview

Implement static escape analysis in `gox-analysis` to determine which variables need heap allocation.

## Escape Rules

### Primitives (int, float, bool)
- Escape when: captured by closure

### struct
- Escape when:
  1. Address taken `&s`
  2. Captured by closure
  3. Assigned to interface

### array
- Escape when:
  1. Captured by closure
  2. Assigned to interface
  3. Sliced `arr[:]` or `arr[i:j]`

### Size Threshold
- struct/array > 256 slots → always escape (direct heap allocation)

### Transitivity
- Nested struct/array fields escape with parent

## Implementation

### Option A: Integrate into Checker (Recommended)

Add escape tracking to existing type checking:

```rust
// checker.rs
pub struct Checker {
    // ... existing fields
    pub escaped_vars: HashSet<ObjKey>,
}

impl Checker {
    pub fn mark_escaped(&mut self, obj: ObjKey) {
        self.escaped_vars.insert(obj);
    }
    
    pub fn is_escaped(&self, obj: ObjKey) -> bool {
        self.escaped_vars.contains(&obj)
    }
}
```

### Modification Points

| File | Location | Change |
|------|----------|--------|
| `checker.rs` | struct Checker | Add `escaped_vars: HashSet<ObjKey>` |
| `expr.rs` | `UnaryOp::Addr` (~L124) | Call `mark_escaped()` on operand |
| `expr.rs` | `ExprKind::FuncLit` (~L1224) | Mark captured variables as escaped |
| `expr.rs` | interface assignment | Mark struct/array as escaped |
| `expr.rs` | slice operation | Mark array as escaped |
| `type_info.rs` | export | Add `is_escaped(symbol)` query |

### Closure Capture Detection

Need to track which variables are captured by closures:

```rust
// In FuncLit handling
fn check_func_lit(&mut self, func: &FuncLit) {
    let outer_scope = self.current_scope();
    
    // Check function body
    self.check_func_body(&func.body);
    
    // Any variable from outer_scope used in body is captured
    for captured in self.find_captured_vars(outer_scope, &func.body) {
        self.mark_escaped(captured);
    }
}
```

## Tasks

- [ ] Add `escaped_vars` field to `Checker`
- [ ] Implement `mark_escaped()` and `is_escaped()`
- [ ] Add escape marking for address-taken (`&s`)
- [ ] Add escape marking for closure capture
- [ ] Add escape marking for interface assignment
- [ ] Add escape marking for slice operation
- [ ] Add size threshold check (>256 slots)
- [ ] Expose `is_escaped()` in `TypeInfo`
- [ ] Write unit tests

## Testing

```gox
func test_no_escape() {
    var s Point  // should NOT escape
    s.x = 1
}

func test_address_escape() {
    var s Point
    p := &s      // s should escape
}

func test_closure_escape() {
    x := 42
    f := func() { println(x) }  // x should escape
    f()
}

func test_interface_escape() {
    var s Point
    var i interface{} = s  // s should escape
}

func test_slice_escape() {
    var arr [5]int
    s := arr[:]  // arr should escape
}
```

## Deliverables

1. `Checker.escaped_vars` field
2. Escape marking at all trigger points
3. `TypeInfo.is_escaped()` query
4. Unit tests for all escape scenarios
