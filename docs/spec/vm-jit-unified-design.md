# Synchronous JIT Design

## 1. Overview

### Goals

1. Implement JIT compilation for hot functions using Cranelift
2. Keep JIT simple by only supporting synchronous operations
3. Minimal changes to existing VM

### Key Decisions

- **Synchronous JIT**: JIT functions execute synchronously within VM Fiber context
- **No separate goroutine model**: No corosensei, no stackful coroutines
- **Async operations excluded**: Functions with defer/recover/go/channel/select are not JIT-compiled
- **Compile-time iterator expansion**: All for-range loops expanded at compile time

## 2. JIT Support Matrix

### Supported

| Feature | Notes |
|---------|-------|
| Arithmetic, comparison, bitwise ops | Direct Cranelift mapping |
| Branching, jumps | Direct mapping |
| Function calls | Both VM and JIT targets |
| Stack/heap variable access | Register + offset |
| Heap allocation | Call out to `vo_gc_alloc` |
| for-range array/slice/int | Compile-time expansion |
| for-range map/string | Compile-time expansion + runtime helper |
| panic() | Set flag, return to VM |
| Method calls, interface dispatch | Call out to runtime |
| Write barrier | Must inline |

### Not Supported

| Feature | Reason |
|---------|--------|
| defer | Unwind semantics require VM management |
| recover() | Panic state managed by VM |
| go statement | Goroutine creation is VM-only |
| channel send/recv/close | May block, requires yield |
| select | Channel multiplexing requires scheduler |
| for-range channel | Blocking iteration |

### Function-Level Decision

```rust
fn can_jit(func: &FunctionDef) -> bool {
    !func.has_defer
    && !func.has_recover
    && !func.has_go_stmt
    && !func.has_channel_op  // send, recv, close
    && !func.has_select
}
```

## 3. Architecture

```
vo-runtime-core (shared)
├── gc.rs              # GC implementation
├── objects/           # Channel, Closure, Map, etc.
└── jit_api.rs         # extern "C" functions for JIT

vo-vm (unchanged except iterator removal)
├── bytecode.rs
├── instruction.rs
├── fiber.rs           # Remove iter_stack
├── scheduler.rs
└── exec/

vo-codegen-vm (main changes)
└── lower/             # Expand all for-range at compile time

vo-jit (new)
├── translate.rs       # bytecode → Cranelift IR
├── compile.rs         # Cranelift → native code
├── cache.rs           # JIT code cache
└── trampoline.rs      # VM ↔ JIT bridge
```

## 4. For-Range Compile-Time Expansion

All for-range loops are expanded at **codegen** time (vo-codegen-vm). Both VM and JIT execute the same expanded bytecode. No runtime iterator state, no special iterator instructions.

This simplifies the VM significantly:
- No `Iterator` enum or `iter_stack`
- No `IterBegin/IterNext/IterEnd` opcodes
- For-range is just regular `Jump` + `JumpIf` + `ArrayGet`/`ChanRecv`/etc.

### Array/Slice

```go
// Source
for i, v := range arr { body }

// Expanded bytecode
__len := len(arr)
__idx := 0
loop:
    if __idx >= __len { goto end }
    i := __idx
    v := arr[__idx]
    body
    __idx++
    goto loop
end:
```

### Int Range

```go
// Source
for i := 0; i < n; i++ { body }

// Already a simple loop, no expansion needed
```

### Map

```go
// Source
for k, v := range m { body }

// Expanded bytecode
__cursor := 0  // local variable
loop:
    __k, __v, __ok := vo_map_iter_next(m, &__cursor)
    if !__ok { goto end }
    k := __k
    v := __v
    body
    goto loop
end:
```

Uses IndexMap for O(1) index-based access. Cursor is just `0..len`.

### String

```go
// Source
for i, r := range s { body }

// Expanded bytecode
__pos := 0  // byte position, local variable
loop:
    if __pos >= len(s) { goto end }
    i := __pos
    r, __width := vo_decode_rune(s, __pos)
    body
    __pos += __width
    goto loop
end:
```

### Channel

```go
// Source
for v := range ch { body }

// Expanded bytecode (uses existing ChanRecv)
loop:
    v, ok := <-ch  // ChanRecv with ok flag
    if !ok { goto end }
    body
    goto loop
end:
```

Function containing channel range will not be JIT-compiled (due to blocking ChanRecv).

## 5. VM Changes

### Remove Entirely

| Item | Location |
|------|----------|
| `Iterator` enum | fiber.rs |
| `Fiber.iter_stack` | fiber.rs |
| `Opcode::IterBegin` | instruction.rs |
| `Opcode::IterNext` | instruction.rs |
| `Opcode::IterEnd` | instruction.rs |
| `exec_iter_*` functions | exec/iter.rs |

All for-range loops are now compile-time expanded. No runtime iterator state needed.

## 6. Runtime API (vo-runtime-core/src/jit_api.rs)

```rust
/// JIT function signature
pub type JitFunc = extern "C" fn(
    ctx: *mut JitContext,
    args: *const u64,
    ret: *mut u64,
) -> JitResult;

#[repr(C)]
pub struct JitContext {
    pub gc: *mut Gc,
    pub globals: *mut u64,
    pub panic_flag: *mut bool,
}

#[repr(C)]
pub enum JitResult {
    Ok,
    Panic,
}

// GC
#[no_mangle]
pub extern "C" fn vo_gc_alloc(gc: *mut Gc, slots: u32, meta: u32) -> u64;

#[no_mangle]
pub extern "C" fn vo_gc_write_barrier(gc: *mut Gc, field: *mut u64, val: u64);

// Map iteration (cursor is local variable, not runtime state)
#[no_mangle]
pub extern "C" fn vo_map_iter_next(
    map: u64,
    cursor: *mut u64,
    key: *mut u64,
    val: *mut u64,
) -> bool;

// String iteration
#[no_mangle]
pub extern "C" fn vo_decode_rune(s: u64, pos: u64, rune: *mut i32) -> u64; // returns next_pos
```

## 7. JIT Execution Model

### VM Calls JIT

```rust
impl Vm {
    fn exec_call(&mut self, fiber: &mut Fiber, func_id: u32, ...) -> ExecResult {
        if let Some(jit_func) = self.jit_cache.get(func_id) {
            let ctx = JitContext {
                gc: &mut self.state.gc,
                globals: self.state.globals.as_mut_ptr(),
                panic_flag: &mut fiber.panic_flag,
            };
            match jit_func(&ctx, args_ptr, ret_ptr) {
                JitResult::Ok => ExecResult::Continue,
                JitResult::Panic => ExecResult::Panic,
            }
        } else {
            // Normal VM interpretation
            self.push_frame(func_id, ...);
            ExecResult::Continue
        }
    }
}
```

### JIT Calls VM (via trampoline)

```rust
#[no_mangle]
pub extern "C" fn vo_call_vm(
    ctx: *mut JitContext,
    func_id: u32,
    args: *const u64,
    ret: *mut u64,
) -> JitResult;
```

### Panic Handling

JIT functions check panic flag after calls:

```rust
// JIT-generated code (pseudo)
fn jit_function(ctx: *mut JitContext, args: *const u64, ret: *mut u64) -> JitResult {
    // ... computation ...
    
    // After any call that may panic:
    vo_call_vm(ctx, some_func_id, call_args, call_ret);
    if *(*ctx).panic_flag {
        return JitResult::Panic;
    }
    
    // ... continue ...
    JitResult::Ok
}
```

## 8. GC Interaction

### During JIT Execution

- GC does **not** scan JIT native stack
- JIT functions are "atomic" from GC perspective

### At Call-Out Points

Before calling runtime functions, JIT flushes live GcRefs to known location:

```rust
#[repr(C)]
pub struct JitContext {
    // ...
    pub gc_roots: *mut [u64; 16],  // Spill area for GcRefs
    pub gc_root_count: *mut u8,
}
```

JIT compiler tracks which values are GcRefs and generates spill code before calls.

### Write Barrier

All pointer writes must use barrier:

```rust
// SATB barrier (mark old value)
#[no_mangle]
pub extern "C" fn vo_gc_write_barrier(gc: *mut Gc, field: *mut u64, val: u64) {
    unsafe {
        if (*gc).is_marking() {
            let old = *field;
            if old != 0 {
                (*gc).mark_gray(old as GcRef);
            }
        }
        *field = val;
    }
}
```

## 9. JIT Compiler Structure

```rust
pub struct JitCompiler {
    module: cranelift_jit::JITModule,
    cache: HashMap<u32, *const u8>,  // func_id -> native code
}

impl JitCompiler {
    pub fn compile(&mut self, func_id: u32, func: &FunctionDef) -> *const u8 {
        let mut ctx = self.module.make_context();
        let mut builder = FunctionBuilder::new(&mut ctx.func, ...);
        
        // Translate each bytecode instruction
        for inst in &func.code {
            self.translate_inst(&mut builder, inst);
        }
        
        builder.finalize();
        let id = self.module.declare_function(...)?;
        self.module.define_function(id, &mut ctx)?;
        self.module.finalize_definitions();
        
        self.module.get_finalized_function(id)
    }
    
    fn translate_inst(&self, builder: &mut FunctionBuilder, inst: &Instruction) {
        match inst.opcode() {
            Opcode::AddI => {
                let a = builder.use_var(inst.a());
                let b = builder.use_var(inst.b());
                let result = builder.ins().iadd(a, b);
                builder.def_var(inst.a(), result);
            }
            Opcode::Call => {
                // Generate call + panic check
            }
            // ... ~50 opcodes to translate
        }
    }
}
```

## 10. Implementation Steps

| Order | Task | Scope |
|-------|------|-------|
| 1 | vo-codegen-vm: Expand array/slice/int for-range | codegen |
| 2 | vo-codegen-vm: Expand map/string for-range | codegen |
| 3 | vo-vm: Remove Iterator enum (except Channel) | fiber.rs |
| 4 | vo-vm: Remove IterBegin/IterNext/IterEnd opcodes | instruction.rs, exec |
| 5 | vo-runtime-core: Add extern "C" API | new jit_api.rs |
| 6 | vo-jit: Create crate, implement bytecode → Cranelift | new crate |
| 7 | vo-jit: Implement VM ↔ JIT trampoline | trampoline.rs |
| 8 | vo-vm: Add JIT dispatch in call instruction | exec/call.rs |

## 11. File Changes

| File | Action |
|------|--------|
| `vo-codegen-vm/src/lower/for_range.rs` | Add: expand all for-range |
| `vo-runtime-core/src/jit_api.rs` | Add: extern "C" runtime API |
| `vo-vm/src/fiber.rs` | Modify: remove Iterator enum, iter_stack |
| `vo-vm/src/instruction.rs` | Modify: remove IterBegin/IterNext/IterEnd opcodes |
| `vo-vm/src/exec/iter.rs` | Remove entirely |
| `vo-vm/src/vm.rs` | Modify: remove iter opcode handling |
| `vo-jit/` | Add: new crate |
