# MM Phase 2: Codegen Changes

**Parent**: [2025-12-22-mm-memory-model-plan.md](2025-12-22-mm-memory-model-plan.md)  
**Status**: Not Started  
**Est. Lines**: ~1100  
**Depends On**: P1 (Escape Analysis)

## Overview

Modify codegen to generate different code based on escape analysis results:
- Non-escaping struct/array → stack allocation (inline slots)
- Escaping struct/array → heap allocation (GcRef)

## Current State

All structs are heap-allocated:

```rust
// Current: always ALLOC
let dst = func.alloc_temp_typed(&[SlotType::GcRef]);
func.emit_with_flags(Opcode::Alloc, field_count as u8, dst, type_id, 0);
```

## Target State

```rust
// New: check escape status
if info.is_escaped(var_symbol) {
    // Escaping: heap allocation
    let dst = func.alloc_temp_typed(&[SlotType::GcRef]);
    func.emit_with_flags(Opcode::Alloc, field_count as u8, dst, type_id, 0);
} else {
    // Non-escaping: stack allocation (inline slots)
    let slots = info.slots_for_type(ty);
    let slot_types = info.type_slot_types(ty);
    let dst = func.alloc_temp_typed(&slot_types);
    // No ALLOC needed, fields are consecutive registers
}
```

## Modifications

### 2.1 gox-codegen-vm/src/type_info.rs

| Task | Description |
|------|-------------|
| Add `is_escaped()` | Query escape status from analysis |
| Add `slots_for_type()` | Get stack slot count for type |

### 2.2 gox-codegen-vm/src/stmt.rs

| Task | Description |
|------|-------------|
| Modify `var` declaration | Choose stack/heap based on escape |
| Modify `:=` declaration | Same as above |

**Key Change** (around L136-160):

```rust
if is_struct {
    if info.is_escaped(name.symbol) {
        // Escaping: heap allocation
        let src = alloc_empty_struct(ty, ctx, func, info)?;
        define_local_with_init(name.symbol, &[SlotType::GcRef], Some(src), func);
    } else {
        // Non-escaping: stack allocation
        let slot_types = info.type_slot_types(ty);
        define_local_with_init(name.symbol, &slot_types, None, func);
    }
}
```

### 2.3 gox-codegen-vm/src/expr.rs

| Task | Description |
|------|-------------|
| Modify `compile_composite_lit` | Stack/heap struct literal |
| Add `compile_field_access_stack` | Direct register offset for stack struct |
| Modify `compile_selector` | Dispatch to stack/heap field access |
| Modify array literal | Stack/heap array |
| Modify array index | Stack/heap access |
| Add `compile_address_of` | For `&x` (already escaped, return GcRef) |

### 2.4 gox-codegen-vm/src/func.rs

| Task | Description |
|------|-------------|
| Update `LocalVar` | Track whether variable is on stack or heap |

### 2.5 gox-codegen-cranelift/src/translate.rs

| Task | Description |
|------|-------------|
| Modify `Opcode::Alloc` | Only for escaping values |
| Add stack struct handling | Multiple Cranelift variables |
| Add stack field access | Load/store from variable offset |

## Field Access Comparison

### Stack struct (non-escaping)

```asm
# s.field1 = 42  (s starts at r0, field1 is offset 1)
MOV r1, 42       # Direct register access
```

### Heap struct (escaping)

```asm
# s.field1 = 42  (s is GcRef at r0)
SET_FIELD r0, 1, 42  # Heap object access
```

## Tasks Checklist

### type_info.rs
- [ ] Add `is_escaped(symbol) -> bool`
- [ ] Add `slots_for_type(ty) -> u16`
- [ ] Add `is_stack_struct(symbol) -> bool` helper

### stmt.rs
- [ ] Modify var declaration for struct
- [ ] Modify var declaration for array
- [ ] Modify short declaration `:=`

### expr.rs
- [ ] Modify `compile_composite_lit` for struct
- [ ] Modify `compile_composite_lit` for array
- [ ] Add `compile_field_access_stack`
- [ ] Modify `compile_selector` dispatch
- [ ] Modify array index for stack
- [ ] Modify array index for heap

### func.rs
- [ ] Track stack/heap status in LocalVar

### translate.rs (Cranelift)
- [ ] Update Alloc opcode handling
- [ ] Add stack struct variable allocation
- [ ] Add stack field access translation

## Testing

After P2, existing tests should still pass with the new allocation strategy.
