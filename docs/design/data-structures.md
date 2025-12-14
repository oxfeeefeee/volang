# GoX VM Data Structures

This document defines the memory layout of all runtime data structures in the GoX VM.

## 1. Memory Model Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Stack                                    │
│  ┌───────┬───────┬───────┬───────┬───────┐                      │
│  │ slot0 │ slot1 │ slot2 │ slot3 │ ...   │  8 bytes each        │
│  │ i64   │GcRef  │ bool  │GcRef  │       │                      │
│  └───┬───┴───┬───┴───────┴───┬───┴───────┘                      │
└──────│───────│───────────────│──────────────────────────────────┘
       │       │               │
       ↓       ↓               ↓
     (value)  Heap            Heap
              
┌─────────────────────────────────────────────────────────────────┐
│                          Heap                                    │
│                                                                  │
│  ┌─────────────────┐    ┌─────────────────┐                     │
│  │   GcObject      │    │   GcObject      │                     │
│  │  ┌───────────┐  │    │  ┌───────────┐  │                     │
│  │  │ GcHeader  │  │    │  │ GcHeader  │  │                     │
│  │  ├───────────┤  │    │  ├───────────┤  │                     │
│  │  │   data    │  │    │  │   data    │  │                     │
│  │  └───────────┘  │    │  └───────────┘  │                     │
│  └─────────────────┘    └─────────────────┘                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 2. Fundamental Units

### 2.1 Slot

The basic storage unit for all values.

```
Size: 8 bytes = 64 bits
```

### 2.2 GcRef

A pointer to a heap-allocated GcObject.

```rust
type GcRef = *mut GcObject;  // 8 bytes pointer
```

Stack stores GcRef; actual objects reside on the heap.

### 2.3 GcObject

Common structure for all heap objects:

```
┌─────────────────────────────────┐  ← GcRef points here
│         GcHeader (8 bytes)      │
├─────────────────────────────────┤
│         Data (N bytes)          │
└─────────────────────────────────┘
```

### 2.4 GcHeader

Object header stored at the beginning of every heap object:

```rust
struct GcHeader {
    mark: u8,        // GC mark (white/gray/black)
    flags: u8,       // Object flags
    _pad: [u8; 2],   // Alignment padding
    type_id: u32,    // Type ID → TypeMeta
}
// Total: 8 bytes
```

### 2.5 TypeMeta

Generated at compile time, stored in bytecode metadata:

```rust
struct TypeMeta {
    kind: TypeKind,           // Type category
    is_value_type: bool,      // true=struct(value), false=object(reference)
    size_slots: u16,          // Data area size in slots
    ptr_bitmap: Vec<bool>,    // Which slots are GcRef (for GC scanning)
    // ...type-specific fields
}

enum TypeKind {
    Struct, Object, Array, Slice, String, 
    Map, Channel, Closure, Interface,
}
```

## 3. Primitive Types

Stored directly in slots, **not heap-allocated**:

| Type | Slots | Storage |
|------|-------|---------|
| `bool` | 1 | 0 or 1 (as u64) |
| `i8/i16/i32/i64` | 1 | Sign-extended to 64-bit |
| `u8/u16/u32/u64` | 1 | Zero-extended to 64-bit |
| `f32` | 1 | Lower 32 bits |
| `f64` | 1 | Full 64 bits |

```
On stack:
┌─────────┐
│   42    │  ← i64 stored directly
├─────────┤
│  3.14   │  ← f64 stored directly
└─────────┘
```

## 4. Struct vs Object

### Semantic Differences

| | **struct** | **object** |
|---|-----------|------------|
| Semantics | Value type | Reference type |
| Assignment | Deep copy | Copy reference |
| Comparison | By field values | By address |
| Map key | ✅ Allowed | ❌ Not allowed |

### Memory Layout (Identical)

```
On heap:
┌─────────────────────────────────┐
│  GcHeader                       │
│    type_id → TypeMeta           │
├─────────────────────────────────┤
│  slot[0]: field1                │
│  slot[1]: field2                │
│  slot[2]: field3                │
│  ...                            │
└─────────────────────────────────┘
```

### Difference is in Instructions

```asm
# struct assignment (value semantics): allocate new object + copy all slots
ALLOC       r_new, TYPE_FOO
COPY_SLOTS  r_new, r_old, 3

# object assignment (reference semantics): copy pointer only
MOV         r_new, r_old
```

### Example

```go
struct Point { x: int, y: int }      // Value type
object Node { value: int, next: Node }  // Reference type
```

```
Point assignment:
p2 = p1  →  Create new object, copy x and y

Node assignment:
n2 = n1  →  n2 and n1 point to the same object
```

## 5. Array

Fixed-length, elements stored contiguously.

```rust
struct GcArray {
    // GcHeader (provided by outer wrapper)
    elem_type: TypeId,    // Element type
    elem_size: u16,       // Slots per element
    len: u32,             // Number of elements
    data: Vec<u64>,       // Contiguous slot array
}
```

```
[3]Point layout (Point uses 2 slots):

┌─────────────────────────────────┐
│  GcHeader                       │
├─────────────────────────────────┤
│  elem_type=Point, elem_size=2   │
│  len=3                          │
├─────────────────────────────────┤
│  arr[0].x, arr[0].y             │  slots 0-1
│  arr[1].x, arr[1].y             │  slots 2-3
│  arr[2].x, arr[2].y             │  slots 4-5
└─────────────────────────────────┘
```

**Access**: `arr[i]` = `data[i * elem_size]`

## 6. Slice

A view into an array, sharing the underlying storage.

```rust
struct GcSlice {
    // GcHeader
    array: GcRef,    // → GcArray
    begin: u32,      // Start index
    end: u32,        // End index
    cap: u32,        // Capacity
}
```

```
┌─────────────┐           ┌─────────────────┐
│  GcSlice    │           │    GcArray      │
│  array ─────┼──────────→│  [0][1][2][3].. │
│  begin: 1   │           └─────────────────┘
│  end: 3     │
│  cap: 5     │
└─────────────┘

Slice represents arr[1:3], sharing underlying array
```

## 7. String

Immutable byte slice.

```rust
struct GcString {
    // GcHeader
    array: GcRef,    // → GcArray<u8>
    begin: u32,
    end: u32,
    // No cap (immutable)
}
```

Substrings share the underlying byte array:

```go
s := "hello world"
sub := s[0:5]  // "hello", shares underlying array
```

## 8. Map

Uses `indexmap` crate, preserving insertion order.

```rust
struct GcMap {
    // GcHeader
    key_type: TypeId,
    value_type: TypeId,
    key_size: u16,        // Slots per key
    value_size: u16,      // Slots per value
    inner: IndexMap<MapKey, MapValue>,
}

// Key/Value are slot arrays
struct MapKey(Vec<u64>);
struct MapValue(Vec<u64>);
```

### Hash/Eq Implementation

```rust
impl Hash for MapKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);  // Hash by slots
    }
}

impl Eq for MapKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0    // Compare by slots
    }
}
```

### Range Iteration

```rust
struct MapIterator {
    map_ref: GcRef,
    position: usize,  // indexmap index, supports pause/resume
}
```

## 9. Channel

```rust
struct GcChannel {
    // GcHeader
    elem_type: TypeId,
    elem_size: u16,
    cap: u32,                       // 0 = unbuffered
    closed: bool,
    buffer: VecDeque<Vec<u64>>,
    send_waiters: Vec<FiberId>,
    recv_waiters: Vec<FiberId>,
}
```

## 10. Closure

```rust
struct GcClosure {
    // GcHeader
    func_id: FuncId,
    upvalues: Vec<GcRef>,  // Captured variables
}
```

## 11. Interface

Dynamic type, stored inline as 2 slots (not a heap object).

```
interface{} layout (2 slots):
┌─────────────────┬─────────────────┐
│  type_id (u64)  │  data (u64)     │
└─────────────────┴─────────────────┘
     slot 0            slot 1
```

### Data Slot Contents

| Actual Type | data slot contains |
|-------------|--------------------|
| Primitives (int, float, bool) | Value directly |
| Reference types (string, slice, map, object) | GcRef |
| struct (value type) | GcRef → heap copy |

### Examples

```go
var x interface{} = 42
// [TYPE_INT, 42]  ← value stored directly, no heap allocation

var y interface{} = "hello"
// [TYPE_STRING, GcRef]  ← pointer to existing string

var z interface{} = Point{1, 2}
// [TYPE_POINT, GcRef]  ← pointer to heap copy of struct
```

### Interface as Struct Field

```go
struct Foo {
    a: int,
    b: interface{},
    c: bool,
}
```

```
Foo layout on heap (4 slots):
┌─────────────────────────────────┐
│  GcHeader                       │
├─────────────────────────────────┤
│  slot[0]: a (int)               │
│  slot[1]: b.type_id             │  ← interface occupies 2 slots
│  slot[2]: b.data                │
│  slot[3]: c (bool)              │
└─────────────────────────────────┘
```

### GC Scanning for Interface

GC must check `type_id` to determine if `data` slot is a pointer:

```rust
fn scan_interface(type_id: u64, data: u64) {
    let meta = &TYPE_TABLE[type_id];
    if meta.is_reference_type() {
        mark_grey(data as GcRef);
    }
    // Primitive values: no action needed
}
```

## 12. GC Scanning

1. Start from root set (stack, globals)
2. For each GcRef, read GcHeader.type_id
3. Look up TypeMeta.ptr_bitmap to find pointer slots
4. Recursively mark all reachable objects

```rust
fn scan(obj: &GcObject) {
    let meta = &TYPE_TABLE[obj.header.type_id];
    for (i, is_ptr) in meta.ptr_bitmap.iter().enumerate() {
        if *is_ptr {
            mark_grey(obj.data[i] as GcRef);
        }
    }
}
```

## 13. Summary

| Type | Stack Storage | Heap Structure | GC Managed |
|------|--------------|----------------|------------|
| Primitives | Value (8 bytes) | None | No |
| struct | GcRef | GcHeader + slots | Yes |
| object | GcRef | GcHeader + slots | Yes |
| Array | GcRef | GcHeader + GcArray | Yes |
| Slice | GcRef | GcHeader + GcSlice | Yes |
| String | GcRef | GcHeader + GcString | Yes |
| Map | GcRef | GcHeader + GcMap | Yes |
| Channel | GcRef | GcHeader + GcChannel | Yes |
| Closure | GcRef | GcHeader + GcClosure | Yes |
| Interface | 2 slots [type_id, data] | None (inline) | data slot if reference |
