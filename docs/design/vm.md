# GoX VM Design

This document describes the GoX VM architecture, instruction set, and execution model.

## 1. Overview

GoX VM is a register-based bytecode interpreter.

### Design Principles

1. **Static typing**: Types determined at compile time, values carry no type tags
2. **Type-specialized instructions**: Each numeric type has dedicated instructions
3. **8-byte fixed instructions**: Simple, uniform, supports large register numbers
4. **Multi-slot values**: struct/interface can occupy multiple consecutive registers
5. **Coroutine support**: Fiber implements goroutine
6. **Incremental GC**: Tri-color mark-sweep

## 2. Instruction Format

### 8-Byte Fixed Format

```
┌────────┬────────┬────────────────┬────────────────┬────────────────┐
│ op (8) │flag(8) │    d (16)      │    s0 (16)     │    s1 (16)     │
└────────┴────────┴────────────────┴────────────────┴────────────────┘
                         8 bytes
```

```rust
struct Instruction {
    op: u8,       // Opcode
    flags: u8,    // Flags/variant
    d: u16,       // Destination register
    s0: u16,      // Source operand 0
    s1: u16,      // Source operand 1
}
```

### Registers

- 16-bit numbering, up to 65536 registers
- Register = stack slot, 8 bytes each
- `rN` = `stack[bp + N]`

## 3. Execution Model

### Fiber (Coroutine)

```rust
struct Fiber {
    id: FiberId,
    status: FiberStatus,      // Running, Suspended, Dead
    
    // Execution state
    stack: Vec<u64>,          // Value stack
    frames: Vec<CallFrame>,   // Call frame stack
    
    // Iterator stack (for range loops)
    iter_stack: Vec<Iterator>,
    
    // Defer stack
    defer_stack: Vec<DeferEntry>,
    
    // Panic state
    panic_value: Option<GcRef>,
}

struct CallFrame {
    func_id: FuncId,
    pc: usize,           // Program counter
    bp: usize,           // Base pointer
    ret_reg: u16,        // Return value destination
    ret_count: u16,      // Number of return values
}

enum FiberStatus {
    Running,
    Suspended,
    Dead,
}
```

### Scheduler

```rust
struct Scheduler {
    fibers: Vec<Fiber>,
    ready_queue: VecDeque<FiberId>,
    current: Option<FiberId>,
}
```

Cooperative scheduling. Fiber yields on:
- Channel blocking
- `yield` instruction
- `runtime.Gosched()`

### Iterator (for range loops)

Stored in Fiber's iter_stack (zero allocation):

```rust
enum Iterator {
    Slice { ref_: GcRef, pos: usize },
    Map { ref_: GcRef, pos: usize },
    String { ref_: GcRef, byte_pos: usize },
    IntRange { cur: i64, end: i64, step: i64 },
}
```

### Defer Entry

Stored in Fiber's defer_stack (zero allocation):

```rust
struct DeferEntry {
    frame_depth: usize,
    func_id: FuncId,
    arg_count: u8,
    args: [u64; 8],    // Fixed size, covers most cases
}
```

## 4. Instruction Set

### 4.1 Load/Store

```asm
LOAD_NIL      d              # d = nil
LOAD_BOOL     d, imm         # d = true/false
LOAD_INT      d, imm         # d = immediate (16-bit)
LOAD_CONST    d, idx         # d = constant_pool[idx]
MOV           d, s           # d = s (single slot)
MOV_N         d, s, n        # Copy n slots
```

### 4.2 Arithmetic (Type-Specialized)

```asm
ADD_I64       d, s0, s1
SUB_I64       d, s0, s1
MUL_I64       d, s0, s1
DIV_I64       d, s0, s1
MOD_I64       d, s0, s1
NEG_I64       d, s

ADD_F64       d, s0, s1
SUB_F64       d, s0, s1
MUL_F64       d, s0, s1
DIV_F64       d, s0, s1
NEG_F64       d, s

# Similar for: i32, u32, u64, f32
```

### 4.3 Comparison

```asm
EQ_I64        d, s0, s1      # d = (s0 == s1)
NE_I64        d, s0, s1
LT_I64        d, s0, s1
LE_I64        d, s0, s1
GT_I64        d, s0, s1
GE_I64        d, s0, s1

EQ_REF        d, s0, s1      # Reference equality (address comparison)
```

### 4.4 Bitwise

```asm
BAND          d, s0, s1
BOR           d, s0, s1
BXOR          d, s0, s1
BNOT          d, s
SHL           d, s0, s1
SHR           d, s0, s1      # Arithmetic right shift
USHR          d, s0, s1      # Logical right shift
```

### 4.5 Control Flow

```asm
JUMP          offset         # PC += offset
JUMP_IF       s, offset      # if s then jump
JUMP_IF_NOT   s, offset      # if !s then jump
```

### 4.6 Function Call

Unified CALL instruction handles both GoX functions and native functions:

```asm
CALL          callable, arg_start, arg_count, ret_start
CALL_METHOD   recv, method_idx, arg_start, arg_count, ret_start
RETURN        ret_start, ret_count
```

Callable types (determined at runtime):

```rust
enum Callable {
    GoxFunc { func_id: FuncId },
    NativeFunc { native_fn: NativeFn },
    Closure { func_id: FuncId, upvalues: Vec<GcRef> },
}

// Native function signature
type NativeFn = fn(&mut VmContext, args: &[u64], ret: &mut [u64]);
```

CALL execution:

```rust
fn exec_call(vm: &mut Vm, callable: &Callable, args: &[u64], ret: &mut [u64]) {
    match callable {
        GoxFunc { func_id } => {
            // Push new call frame, execute bytecode
        }
        NativeFunc { native_fn } => {
            // Pause GC, call native function directly
            vm.gc.pause();
            native_fn(&mut vm.ctx, args, ret);
            vm.gc.resume();
        }
        Closure { func_id, upvalues } => {
            // Set up upvalues, then execute like GoxFunc
        }
    }
}
```

### 4.7 Object Operations

```asm
ALLOC         d, type_id     # Allocate heap object
GET_FIELD     d, obj, idx    # d = obj.fields[idx]
SET_FIELD     obj, idx, s    # obj.fields[idx] = s
COPY_SLOTS    d, s, n        # Deep copy n slots (struct assignment)
```

### 4.8 Array/Slice

```asm
ARRAY_GET     d, arr, idx
ARRAY_SET     arr, idx, s
ARRAY_LEN     d, arr

SLICE_GET     d, slice, idx
SLICE_SET     slice, idx, s
SLICE_LEN     d, slice
SLICE_CAP     d, slice
SLICE_MAKE    d, type_id, len, cap
SLICE_SLICE   d, s, lo, hi
SLICE_APPEND  d, slice, elem
```

### 4.9 String

```asm
STR_CONCAT    d, s0, s1
STR_LEN       d, s
STR_INDEX     d, s, idx
```

### 4.10 Map

```asm
MAP_MAKE      d, type_id, cap
MAP_GET       d, ok, map, key_start
MAP_SET       map, key_start, val_start
MAP_DELETE    map, key_start
MAP_LEN       d, map
```

### 4.11 Channel

```asm
CHAN_MAKE     d, type_id, cap
CHAN_SEND     chan, val_start    # May block
CHAN_RECV     d, chan            # May block
CHAN_CLOSE    chan
```

### 4.12 Range Iteration

```asm
ITER_PUSH     container, type    # Push iterator onto stack
ITER_NEXT     key, val           # Get next key-value pair
ITER_POP                         # Pop iterator from stack
JUMP_DONE     offset             # Jump if iteration complete
```

### 4.13 Goroutine

```asm
GO            func_id, arg_start, arg_count
YIELD
```

### 4.14 Defer/Panic/Recover

```asm
DEFER_PUSH    func_id, arg_start, arg_count
PANIC         val
RECOVER       d
```

### 4.15 Interface

```asm
IFACE_BOX     d, s, type_id      # Box: d = [type_id, value/ref]
IFACE_UNBOX   d, iface, type_id  # Unbox: type assertion
IFACE_TYPE    d, iface           # Get type_id
```

## 5. Garbage Collection

### Incremental Tri-Color Mark-Sweep

```
White: Not visited (potentially garbage)
Gray:  Visited, children not scanned
Black: Visited, children scanned
```

### Root Set

- All Fiber stacks
- All Fiber iter_stacks (container references in iterators)
- All Fiber defer_stacks (references in defer arguments)
- Global variables

### Scanning

```rust
fn scan(obj: &GcObject) {
    let meta = &TYPE_TABLE[obj.header.type_id];
    
    match meta.kind {
        Interface => {
            // Special handling: check type_id to determine if data is pointer
            let actual_type = obj.slots[0];
            let data = obj.slots[1];
            if TYPE_TABLE[actual_type].is_reference_type() {
                mark_grey(data as GcRef);
            }
        }
        _ => {
            // Normal types: scan by ptr_bitmap
            for (i, is_ptr) in meta.ptr_bitmap.iter().enumerate() {
                if *is_ptr {
                    mark_grey(obj.slots[i] as GcRef);
                }
            }
        }
    }
}
```

## 6. Bytecode File Format

```
┌─────────────────────────────────────────┐
│  Magic: "GOXB" (4 bytes)                │
│  Version (4 bytes)                      │
├─────────────────────────────────────────┤
│  Type Table                             │
│    count: u32                           │
│    TypeMeta[]                           │
├─────────────────────────────────────────┤
│  Constant Pool                          │
│    count: u32                           │
│    Constant[]                           │
├─────────────────────────────────────────┤
│  Function Table                         │
│    count: u32                           │
│    FunctionDef[]                        │
├─────────────────────────────────────────┤
│  Entry Point (main func_id)             │
└─────────────────────────────────────────┘

FunctionDef:
  - name_idx: u32        (index into constant pool)
  - param_slots: u16     (total slots for parameters)
  - local_slots: u16     (total slots for locals)
  - code_len: u32
  - code: [Instruction]
```

## 7. Summary

| Component | Design |
|-----------|--------|
| Instruction format | 8-byte fixed |
| Instruction dispatch | Type-specialized |
| Registers | 16-bit numbering, stack slots |
| struct | Multi-register, value semantics |
| object | GcRef, reference semantics |
| interface | 2-slot inline [type_id, data] |
| Iterator | Fiber internal stack |
| defer | Fiber internal stack |
| Goroutine | Fiber + cooperative scheduling |
| GC | Incremental tri-color mark-sweep |
