# GC Object Scanning Design

## Overview

GC traverses all reachable objects and marks internal references. This document describes a unified `scan_object` function shared by VM and JIT.

## Core Principle

- **Only user-defined structs require ptr_bitmap lookup**
- **All built-in types have fixed internal layouts**
- **内联型需要遍历内部结构，指针型直接 mark**

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
| 0 | Nil | 值类型 | ❌ | 不扫描 |
| 1 | Bool | 值类型 | ❌ | 不扫描 |
| 2-6 | Int* | 值类型 | ❌ | 不扫描 |
| 7-11 | Uint* | 值类型 | ❌ | 不扫描 |
| 12-13 | Float* | 值类型 | ❌ | 不扫描 |
| **14** | **String** | 内联型 | ✅ | mark slot 0 |
| **15** | **Slice** | 内联型 | ✅ | mark slot 0 |
| **16** | **Map** | 内联型 | ✅ | 遍历 value |
| 17 | Struct | - | ❌ 不出现 | 见 32+ |
| **18** | **Pointer** | 指针型 | ❌ 值是指针 | 直接 mark |
| 19 | Interface | 条件指针 | ❌ 不出现 | 2 slots，见下方说明 |
| **20** | **Array** | 内联型 | ✅ | 遍历元素 |
| **21** | **Channel** | 内联型 | ✅ | 遍历 buffer |
| **22** | **Closure** | 内联型 | ✅ | mark upvalues |
| **32+** | **User Struct** | 内联型 | ✅ | ptr_bitmap |

### Types NOT appearing as GcObject.header.type_id

| type_id | Type | Reason |
|---------|------|--------|
| 17 | Struct | 用户 struct 使用 32+ |
| 18 | Pointer | 值本身就是 GcRef，指向的对象有自己的 type_id |
| 19 | Interface | 存储为 2 slots `[packed_types, data]`，不独立分配 |

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
Note:    key 必须是可比较类型，不含 GC 引用
```

### Array (20)
```
Layout: [elem_type, elem_bytes, len, data...]
Scan:
  if elem_type >= 32 (user struct):
    # 元素内联存储，用 ptr_bitmap 扫描每个元素
    for i in 0..len:
      for (j, is_ptr) in bitmap:
        if is_ptr: mark data[i*slots + j]
  elif type_needs_gc(elem_type):
    # 内置引用类型，每个元素是指针
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
Note:   upvalue 总是可能包含 GC 引用
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

### Interface (值类型，非 GcObject)

```
Layout: [packed_types, data]  ← 2 slots
  slot 0: (iface_type_id << 32) | value_type_id
  slot 1: data (值或 GcRef)

Read:
  let packed = slot0;
  let iface_type = (packed >> 32) as u32;
  let value_type = packed as u32;

GC Scan:
  if type_needs_gc(value_type): mark slot1
```

### Interface Field in Struct

```go
type Container struct {
    data interface{}   // 2 slots: [packed_types, data]
}
// ptr_bitmap = [false, true]  ← slot 1 保守 mark
```

## ptr_bitmap Generation Rules

| Field Type | Slots | ptr_bitmap |
|------------|-------|------------|
| `int`, `float`, `bool` | 1 | `[false]` |
| `string`, `*T`, `[]T`, `map`, `chan`, `func` | 1 | `[true]` |
| `interface{}` | 2 | `[false, true]` |
| `MyStruct` (embedded) | N | 递归生成 |

## Static Type Table

```rust
use once_cell::sync::OnceCell;

/// Struct ptr_bitmaps, indexed by (type_id - FIRST_USER_TYPE_ID)
static STRUCT_BITMAPS: OnceCell<Box<[Box<[bool]>]>> = OnceCell::new();

pub fn init_struct_bitmaps(bitmaps: Vec<Vec<bool>>) { ... }
pub fn get_struct_bitmap(type_id: u32) -> Option<&'static [bool]> { ... }
```

## Unified scan_object

```rust
pub fn scan_object(gc: &mut Gc, obj: GcRef) {
    let type_id = unsafe { (*obj).header.type_id };
    
    // User-defined struct: use ptr_bitmap
    if type_id >= FIRST_USER_TYPE_ID {
        if let Some(bitmap) = get_struct_bitmap(type_id) {
            for (i, &is_ptr) in bitmap.iter().enumerate() {
                if is_ptr {
                    let val = Gc::read_slot(obj, i);
                    if val != 0 { gc.mark_gray(val as GcRef); }
                }
            }
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
```

## Data Flow

```
Compile Time (codegen)
    │
    ▼
For each struct definition:
  - Analyze field types
  - Generate ptr_bitmap (考虑 interface 占 3 slots)
  - Store in bytecode.types[].ptr_bitmap
    │
    ▼
Load Time (VM/JIT)
    │
    ▼
init_struct_bitmaps(bytecode.types.map(|t| t.ptr_bitmap))
    │
    ▼
GC Time
    │
    ▼
gc.collect(|gc, obj| gc_types::scan_object(gc, obj))
```

## Implementation Location

- `gox-runtime-core/src/gc_types.rs`: scan_object, STRUCT_BITMAPS
- `gox-common-core/src/types.rs`: type_needs_gc
- `gox-codegen-vm/src/types.rs`: ptr_bitmap generation (TODO)

## Properties

- **Unified**: Same scan_object for VM and JIT
- **Simple**: Only structs need table lookup
- **Lock-free**: Static data, read-only after init
- **Conservative**: Interface slot always marked (safe)
