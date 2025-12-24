# Design Q&A

This document records design decisions and their rationale.

---

## Q: Why is `IfaceInit` instruction not needed?

**Context**: In the original design, interface initialization required two steps:
1. `IfaceInit` - Initialize nil interface, set `iface_meta_id` in slot0
2. `IfaceAssign` - Assign value, preserve `iface_meta_id` from dst.slot0

**Answer**: `IfaceAssign` now takes `iface_meta_id` as an instruction parameter (compile-time known):

```
IfaceAssign dst, src, iface_meta_id, vk
```

This eliminates the need for `IfaceInit` because:

1. **Compile-time knowledge**: When compiling `var r Reader = x`, the compiler knows the target interface type is `Reader`, so it can encode `Reader_meta_id` directly in the `IfaceAssign` instruction.

2. **Zero-value initialization**: For `var r Reader` without assignment, slots are naturally zero-initialized:
   - `slot0 = 0` → `itab_id=0`, `value_kind=Void`
   - `value_kind == Void` means nil interface (Go semantics)
   - No special instruction needed

3. **Simpler instruction set**: One less opcode to implement and maintain.

**Related**: Interface nil check uses `value_kind == Void`, not `itab_id == 0`. This follows Go semantics where typed nil (e.g., `(*T)(nil)` assigned to interface) is NOT nil interface.

---

## Q: Why does Itab only contain `methods`, not `meta_id` or `iface_meta_id`?

**Context**: Original Itab design stored both type IDs:
```rust
struct Itab {
    iface_meta_id: u32,
    concrete_meta_id: u32,
    methods: Vec<u32>,
}
```

**Answer**: These fields are redundant:

1. **`concrete_meta_id`**: Already available in `slot0.value_meta.meta_id()`. No need to duplicate.

2. **`iface_meta_id`**: The `itab_cache` key is `(meta_id, iface_meta_id)`, so can be reverse-looked-up if needed (rare, mainly for debugging).

3. **For `IfaceAssert`**: Type assertion checks `slot0.value_meta.meta_id() == target_meta_id`, no need to access Itab at all.

**Simplified Itab**:
```rust
struct Itab {
    methods: Vec<u32>,  // method_idx -> func_id
}
```

---

## Q: Why use `value_kind == Void` for nil interface check instead of `itab_id == 0`?

**Answer**: This follows Go's interface nil semantics:

```go
var i interface{} = nil     // i == nil → true
var p *int = nil
var j interface{} = p       // j == nil → false! (typed nil)
```

In Go, an interface is nil only when **both** type and value are nil. If type is set but value is nil (typed nil), the interface is NOT nil.

In Vo's representation:
- `value_kind == Void` → no type assigned → nil interface
- `value_kind != Void && data == 0` → typed nil → NOT nil interface

Using `itab_id == 0` would be wrong because a valid type with no methods could legitimately have `itab_id == 0`.

---

## Q: Why remove `concrete_meta_id` terminology?

**Answer**: It's redundant with `value_meta.meta_id()`:

- `value_meta` = `[meta_id:24 | value_kind:8]`
- `meta_id` in interface context IS the concrete type's meta_id
- No need for a separate name that describes the same thing

Using consistent terminology (`meta_id`) reduces confusion.

---

## Q: Why rename `upval` to `capture` in closures?

**Answer**: `upval` (upvalue) is Lua terminology. `capture` is more self-explanatory:

- **capture**: Variables captured from enclosing scope
- **upval**: Requires knowing Lua's implementation details

Also, Vo closures are simpler than Lua upvalues:
- Escaped variables are heap-allocated directly (by escape analysis)
- Closures store GcRef to these variables, no indirection box needed
- `ClosureGet`/`ClosureSet` directly read/write the heap location

```rust
struct ClosureHeader {
    func_id: u32,
    capture_count: u32,
}
// Followed by capture_count GcRef slots
```

---

## Q: Why distinguish `HeapArray` vs `StackArray` in Iterator?

**Answer**: For `for i, v := range arr` where `arr` is a stack-allocated array:

- **HeapArray**: `arr` is a GcRef, iterator holds the reference
- **StackArray**: `arr` lives in stack slots, iterator holds `(bp, base_slot)`

If we only had HeapArray and used GcRef for stack arrays, we'd need to:
1. Box the stack array to heap just for iteration (wasteful)
2. Or use unsafe pointer to stack (invalid after function returns)

StackArray iterator is safe because:
- `bp` (base pointer) ties it to a specific call frame
- When frame pops, iterator is also popped (iter_stack per Fiber)

---

## Q: Why encode `elem_slots` in instructions instead of runtime lookup?

**Context**: Container creation instructions (`ArrayNew`, `SliceNew`, `ChanNew`, `MapNew`) now encode `elem_slots` in instruction operands.

**Answer**: Avoids runtime struct_metas lookup:

**Option A** (runtime lookup):
```rust
// ArrayNew: a=dst, b=meta_reg, c=len_reg
let elem_meta = slots[b] as ValueMeta;
let elem_slots = match elem_meta.value_kind() {
    Struct => module.struct_metas[elem_meta.meta_id()].size_slots,
    Interface => 2,
    _ => 1,
};
```

**Option B** (instruction encoding, chosen):
```rust
// ArrayNew: a=dst, b=meta_reg, c=len_reg, flags=elem_slots
let elem_slots = flags as usize;  // O(1), no lookup
```

Benefits of Option B:
1. **Simpler VM**: No conditional logic or table lookup in hot path
2. **Compile-time known**: `elem_slots` is always known at compile time
3. **No struct_metas access**: VM execution doesn't need module reference for container ops

---

## Q: How is `interface{}` (empty interface) handled for `iface_meta_id`?

**Answer**: Empty interface `interface{}` has a concrete `iface_meta_id` in `interface_metas[]`, just with `method_names: []`. No special case needed—itab will have an empty `methods` Vec.
