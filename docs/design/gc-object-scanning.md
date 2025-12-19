# GC Object Scanning Design

## Overview

GC traverses all reachable objects and marks internal references. This document describes a unified `scan_object` function shared by VM and JIT.

## Core Principle

- **Only user-defined structs require ptr_bitmap lookup**
- **All built-in types have fixed internal layouts**
- **Inline types need internal traversal, pointer types mark directly**

## Type Classification

### type_needs_gc

```rust
/// type_id >= 14 (String) needs scanning.
pub fn type_needs_gc(type_id: u32) -> bool {
    type_id >= ValueKind::String as u32  // >= 14
}
```

### Complete Type List

| type_id | Type | Category | GcObject? | Scan Method |
|---------|------|----------|-----------|-------------|
| 0 | Nil | value | ❌ | skip |
| 1 | Bool | value | ❌ | skip |
| 2-6 | Int* | value | ❌ | skip |
| 7-11 | Uint* | value | ❌ | skip |
| 12-13 | Float* | value | ❌ | skip |
| **14** | **String** | inline | ✅ | mark slot 0 |
| **15** | **Slice** | inline | ✅ | mark slot 0 |
| **16** | **Map** | inline | ✅ | iterate values |
| 17 | Struct | - | ❌ N/A | see 32+ |
| **18** | **Pointer** | pointer | ❌ value is ptr | mark directly |
| 19 | Interface | conditional | ❌ N/A | 2 slots, see below |
| **20** | **Array** | inline | ✅ | iterate elements |
| **21** | **Channel** | inline | ✅ | iterate buffer |
| **22** | **Closure** | inline | ✅ | mark upvalues |
| **32+** | **User Struct** | inline | ✅ | slot_types |

### Types NOT appearing as GcObject.header.type_id

| type_id | Type | Reason |
|---------|------|--------|
| 17 | Struct | User structs use 32+ |
| 18 | Pointer | Value itself is GcRef, pointed object has its own type_id |
| 19 | Interface | Stored as 2 slots `[packed_types, data]`, not allocated separately |

## Built-in Type Layouts

### String (14)
```
Layout: [array_ref, start, len]
Scan:   mark slot 0 (array_ref)
```

### Slice (15)
```
Layout: [array_ref, start, len, cap]
Scan:   mark slot 0 (array_ref)
```

### Map (16)
```
Layout:  [map_ptr, key_type, val_type]
Storage: IndexMap<u64, u64>
Scan:    if type_needs_gc(val_type):
           for (_, val) in entries:
             mark val
Note:    keys must be comparable types, no GC refs
```

### Array (20)
```
Layout: [elem_type, elem_bytes, len, data...]
Scan:
  if elem_type >= 32 (user struct):
    # elements stored inline, scan each with slot_types
    for i in 0..len:
      for (j, is_ptr) in bitmap:
        if is_ptr: mark data[i*slots + j]
  elif type_needs_gc(elem_type):
    # builtin ref type, each element is a pointer
    for i in 0..len:
      mark data[i]
```

### Channel (21)
```
Layout:  [chan_ptr, elem_type, cap]
Storage: ChannelState { buffer: VecDeque<u64>, waiting_senders: VecDeque<(GoId, u64)> }
Scan:    if type_needs_gc(elem_type):
           for val in buffer: mark val
           for (_, val) in waiting_senders: mark val
```

### Closure (22)
```
Layout: [func_id, count, upval0, upval1, ...]
Scan:   for i in 0..count: mark upval[i]
Note:   upvalues may contain GC refs
```

## User-Defined Struct (32+)

Structs require ptr_bitmap from codegen.

```go
type Person struct {
    name   string      // GC ref
    age    int         // value
    friend *Person     // GC ref
}
// ptr_bitmap = [true, false, true]
```

### Interface (value type, not GcObject)

```
Layout: [packed_types, data]  ← 2 slots
  slot 0: (iface_type_id << 32) | value_type_id
  slot 1: data (value or GcRef, depends on value_type)

Read:
  let packed = slot0;
  let iface_type = (packed >> 32) as u32;
  let value_type = packed as u32;

GC Scan (dynamic check):
  let value_type = slot0 as u32;  // low 32 bits
  if type_needs_gc(value_type): mark slot1
```

**Key point**: Whether interface slot 1 needs scanning depends on runtime value_type, cannot be determined statically.

### Interface Field in Struct

```go
type Container struct {
    data interface{}   // 2 slots: [packed_types, data]
}
// slot_types needs special markers for interface slots
```

For structs containing interface fields, dynamic check is required during scanning:
```rust
fn scan_struct_with_interface(gc: &mut Gc, obj: GcRef, bitmap: &[SlotType]) {
    for (i, slot_type) in bitmap.iter().enumerate() {
        match slot_type {
            SlotType::Value => { /* skip */ }
            SlotType::GcRef => {
                let val = Gc::read_slot(obj, i);
                if val != 0 { gc.mark_gray(val as GcRef); }
            }
            SlotType::Interface0 => { /* skip, this is type_id */ }
            SlotType::Interface1 => {
                // Dynamic check: read value_type from previous slot
                let packed = Gc::read_slot(obj, i - 1);
                let value_type = packed as u32;
                if type_needs_gc(value_type) {
                    let val = Gc::read_slot(obj, i);
                    if val != 0 { gc.mark_gray(val as GcRef); }
                }
            }
        }
    }
}
```

## SlotType (defined in gox-common-core)

Since interface requires dynamic checking, struct slot_types use `SlotType` instead of simple `bool`.
`SlotType` is defined in `gox-common-core/src/types.rs`, used for both stack and heap scanning:

```rust
// gox-common-core/src/types.rs
#[repr(u8)]
pub enum SlotType {
    Value = 0,       // non-pointer, skip
    GcRef = 1,       // GC pointer, must scan
    Interface0 = 2,  // interface slot 0 (type_id), skip
    Interface1 = 3,  // interface slot 1, dynamic check required
}
```

**Unified design**: Stack and heap scanning use the same `SlotType` enum.

### Generation Rules

| Field Type | Slots | SlotType |
|------------|-------|----------|
| `int`, `float`, `bool` | 1 | `[Value]` |
| `string`, `*T`, `[]T`, `map`, `chan`, `func` | 1 | `[GcRef]` |
| `interface{}` | 2 | `[Interface0, Interface1]` |
| `MyStruct` (embedded) | N | recursive |

## Static Type Table

```rust
use once_cell::sync::OnceCell;

/// Struct slot types, indexed by (type_id - FIRST_USER_TYPE_ID)
static STRUCT_SLOT_TYPES: OnceCell<Box<[Box<[SlotType]>]>> = OnceCell::new();

pub fn init_struct_slot_types(types: Vec<Vec<SlotType>>) { ... }
pub fn get_struct_slot_types(type_id: u32) -> Option<&'static [SlotType]> { ... }
```

## Unified scan_object

```rust
pub fn scan_object(gc: &mut Gc, obj: GcRef) {
    let type_id = unsafe { (*obj).header.type_id };
    
    // User-defined struct: use slot_types
    if type_id >= FIRST_USER_TYPE_ID {
        if let Some(slot_types) = get_struct_slot_types(type_id) {
            scan_with_slot_types(gc, obj, slot_types);
        }
        return;
    }
    
    // Built-in types: fixed layouts
    match ValueKind::from_u8(type_id as u8) {
        ValueKind::String | ValueKind::Slice => {
            let val = Gc::read_slot(obj, 0);
            if val != 0 { gc.mark_gray(val as GcRef); }
        }
        ValueKind::Array => scan_array(gc, obj),
        ValueKind::Map => scan_map(gc, obj),
        ValueKind::Channel => scan_channel(gc, obj),
        ValueKind::Closure => scan_closure(gc, obj),
        _ => {}
    }
}

/// Scan object using SlotType array (handles interface dynamically)
fn scan_with_slot_types(gc: &mut Gc, obj: GcRef, slot_types: &[SlotType]) {
    for (i, &slot_type) in slot_types.iter().enumerate() {
        match slot_type {
            SlotType::Value | SlotType::Interface0 => { /* skip */ }
            SlotType::GcRef => {
                let val = Gc::read_slot(obj, i);
                if val != 0 { gc.mark_gray(val as GcRef); }
            }
            SlotType::Interface1 => {
                // Dynamic check: read value_type from previous slot
                let packed = Gc::read_slot(obj, i - 1);
                let value_type = packed as u32;
                if type_needs_gc(value_type) {
                    let val = Gc::read_slot(obj, i);
                    if val != 0 { gc.mark_gray(val as GcRef); }
                }
            }
        }
    }
}
```

## Data Flow

```
Compile Time (codegen)
    │
    ▼
For each struct definition:
  - Analyze field types
  - Generate slot_types (interface → [Interface0, Interface1])
  - Store in bytecode.types[].slot_types
    │
    ▼
Load Time (VM/JIT)
    │
    ▼
init_struct_slot_types(bytecode.types.map(|t| t.slot_types))
    │
    ▼
GC Time
    │
    ▼
gc.collect(|gc, obj| gc_types::scan_object(gc, obj))
```

## Implementation Location

- `gox-common-core/src/types.rs`: `SlotType` enum definition
- `gox-runtime-core/src/gc_types.rs`: `scan_object`, `STRUCT_SLOT_TYPES`
- `gox-common-core/src/types.rs`: `type_needs_gc`
- `gox-codegen-vm/src/types.rs`: slot_types generation (TODO)

## Properties

- **Unified**: Stack and heap scanning use the same `SlotType` enum
- **Precise**: Interface scanned precisely via dynamic value_type check
- **Lock-free**: Static data, read-only after initialization
- **No false positives**: Never incorrectly marks non-pointer values
