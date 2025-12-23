# Backend P4: gox-runtime-native

**Parent**: [2025-12-23-backend-rewrite-plan.md](2025-12-23-backend-rewrite-plan.md)  
**Status**: Not Started  
**Est. Modules**: 35  
**Depends On**: P1 (runtime-core)

## Overview

为 JIT/AOT 提供 runtime 支持：全局 GC 管理、符号注册、goroutine 调度、stack map。

**核心原则**：
- 所有需要 GC 的操作通过全局 GC 实例
- extern "C" 接口供 Cranelift 生成的代码调用
- goroutine 使用 corosensei (stackful coroutine)

## 模块清单

### 1. 全局 GC (gc_global.rs)

```rust
use gox_runtime_core::gc::{Gc, GcRef};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// 全局 GC 实例
static GLOBAL_GC: Lazy<Mutex<Gc>> = Lazy::new(|| Mutex::new(Gc::new()));

/// 全局变量存储
static GLOBALS: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// 全局变量是否是 GC 引用
static GLOBALS_IS_REF: Mutex<Vec<bool>> = Mutex::new(Vec::new());

/// 函数指针表
static FUNC_TABLE: RwLock<Vec<*const u8>> = RwLock::new(Vec::new());

/// 初始化
pub fn init_gc() {
    *GLOBAL_GC.lock() = Gc::new();
}

pub fn init_globals(size: usize, is_ref: Vec<bool>) {
    let mut globals = GLOBALS.lock();
    globals.clear();
    globals.resize(size, 0);
    *GLOBALS_IS_REF.lock() = is_ref;
}

pub fn init_func_table(size: usize) {
    let mut table = FUNC_TABLE.write();
    table.clear();
    table.resize(size, std::ptr::null());
}

pub fn set_func_ptr(func_id: u32, ptr: *const u8) {
    FUNC_TABLE.write()[func_id as usize] = ptr;
}
```

**extern "C" 接口**：

```rust
#[no_mangle]
pub extern "C" fn gox_rt_alloc(value_kind: u8, type_id: u16, slots: u16) -> GcRef {
    GLOBAL_GC.lock().alloc(value_kind, type_id, slots)
}

#[no_mangle]
pub extern "C" fn gox_gc_read_slot(obj: GcRef, idx: usize) -> u64 {
    Gc::read_slot(obj, idx)
}

#[no_mangle]
pub extern "C" fn gox_gc_write_slot(obj: GcRef, idx: usize, val: u64) {
    Gc::write_slot(obj, idx, val)
}

#[no_mangle]
pub extern "C" fn gox_rt_write_barrier(parent: GcRef, child: GcRef) {
    GLOBAL_GC.lock().write_barrier(parent, child)
}

#[no_mangle]
pub extern "C" fn gox_rt_get_global(idx: usize) -> u64 {
    GLOBALS.lock()[idx]
}

#[no_mangle]
pub extern "C" fn gox_rt_set_global(idx: usize, val: u64) {
    GLOBALS.lock()[idx] = val;
}

#[no_mangle]
pub extern "C" fn gox_func_table_ptr() -> *const *const u8 {
    FUNC_TABLE.read().as_ptr()
}
```

**⚠️ 注意**：
- `gox_gc_read_slot` / `gox_gc_write_slot` 是静态方法，不需要锁
- 但 `gox_rt_alloc` 需要锁 GC

### 2. 符号注册 (symbols.rs)

```rust
/// Runtime 符号
pub struct RuntimeSymbol {
    pub name: &'static str,
    pub ptr: *const u8,
}

/// 符号表
pub struct RuntimeSymbols {
    pub symbols: Vec<RuntimeSymbol>,
}

impl RuntimeSymbols {
    pub fn new() -> Self {
        let symbols = vec![
            // GC (5)
            RuntimeSymbol { name: "gox_rt_alloc", ptr: gox_rt_alloc as *const u8 },
            RuntimeSymbol { name: "gox_gc_read_slot", ptr: gox_gc_read_slot as *const u8 },
            RuntimeSymbol { name: "gox_gc_write_slot", ptr: gox_gc_write_slot as *const u8 },
            RuntimeSymbol { name: "gox_rt_write_barrier", ptr: gox_rt_write_barrier as *const u8 },
            RuntimeSymbol { name: "gox_rt_mark_gray", ptr: gox_rt_mark_gray as *const u8 },
            
            // Globals (2)
            RuntimeSymbol { name: "gox_rt_get_global", ptr: gox_rt_get_global as *const u8 },
            RuntimeSymbol { name: "gox_rt_set_global", ptr: gox_rt_set_global as *const u8 },
            
            // String (11)
            RuntimeSymbol { name: "gox_string_len", ptr: ffi::gox_string_len as *const u8 },
            RuntimeSymbol { name: "gox_string_index", ptr: ffi::gox_string_index as *const u8 },
            RuntimeSymbol { name: "gox_rt_string_concat", ptr: gox_rt_string_concat as *const u8 },
            RuntimeSymbol { name: "gox_rt_string_slice", ptr: gox_rt_string_slice as *const u8 },
            RuntimeSymbol { name: "gox_string_eq", ptr: ffi::gox_string_eq as *const u8 },
            // ... 更多 string 函数
            
            // Array (4)
            RuntimeSymbol { name: "gox_rt_array_create", ptr: gox_rt_array_create as *const u8 },
            RuntimeSymbol { name: "gox_array_len", ptr: ffi::gox_array_len as *const u8 },
            RuntimeSymbol { name: "gox_array_get", ptr: ffi::gox_array_get as *const u8 },
            RuntimeSymbol { name: "gox_array_set", ptr: ffi::gox_array_set as *const u8 },
            
            // Slice (7)
            RuntimeSymbol { name: "gox_rt_slice_create", ptr: gox_rt_slice_create as *const u8 },
            RuntimeSymbol { name: "gox_slice_len", ptr: ffi::gox_slice_len as *const u8 },
            RuntimeSymbol { name: "gox_slice_cap", ptr: ffi::gox_slice_cap as *const u8 },
            RuntimeSymbol { name: "gox_slice_get", ptr: ffi::gox_slice_get as *const u8 },
            RuntimeSymbol { name: "gox_slice_set", ptr: ffi::gox_slice_set as *const u8 },
            RuntimeSymbol { name: "gox_rt_slice_append", ptr: gox_rt_slice_append as *const u8 },
            RuntimeSymbol { name: "gox_rt_slice_slice", ptr: gox_rt_slice_slice as *const u8 },
            
            // Closure (6)
            RuntimeSymbol { name: "gox_rt_closure_create", ptr: gox_rt_closure_create as *const u8 },
            RuntimeSymbol { name: "gox_closure_func_id", ptr: ffi::gox_closure_func_id as *const u8 },
            RuntimeSymbol { name: "gox_closure_get_upvalue", ptr: ffi::gox_closure_get_upvalue as *const u8 },
            RuntimeSymbol { name: "gox_closure_set_upvalue", ptr: ffi::gox_closure_set_upvalue as *const u8 },
            RuntimeSymbol { name: "gox_rt_upval_box_create", ptr: gox_rt_upval_box_create as *const u8 },
            RuntimeSymbol { name: "gox_upval_box_get", ptr: ffi::gox_upval_box_get as *const u8 },
            RuntimeSymbol { name: "gox_upval_box_set", ptr: ffi::gox_upval_box_set as *const u8 },
            
            // Interface (3)
            RuntimeSymbol { name: "gox_interface_unbox_type", ptr: ffi::gox_interface_unbox_type as *const u8 },
            RuntimeSymbol { name: "gox_interface_unbox_data", ptr: ffi::gox_interface_unbox_data as *const u8 },
            RuntimeSymbol { name: "gox_interface_is_nil", ptr: ffi::gox_interface_is_nil as *const u8 },
            
            // Function table
            RuntimeSymbol { name: "gox_func_table_ptr", ptr: gox_func_table_ptr as *const u8 },
            
            // Extern dispatch
            RuntimeSymbol { name: "gox_extern_call", ptr: extern_dispatch::gox_extern_call as *const u8 },
            
            // Goroutine (6)
            RuntimeSymbol { name: "gox_go_spawn", ptr: goroutine::gox_go_spawn as *const u8 },
            RuntimeSymbol { name: "gox_yield", ptr: goroutine::gox_yield as *const u8 },
            RuntimeSymbol { name: "gox_chan_new", ptr: goroutine::gox_chan_new as *const u8 },
            RuntimeSymbol { name: "gox_chan_send", ptr: goroutine::gox_chan_send as *const u8 },
            RuntimeSymbol { name: "gox_chan_recv", ptr: goroutine::gox_chan_recv as *const u8 },
            RuntimeSymbol { name: "gox_chan_close", ptr: goroutine::gox_chan_close as *const u8 },
            
            // Defer/Panic (4)
            RuntimeSymbol { name: "gox_defer_push", ptr: goroutine::gox_defer_push as *const u8 },
            RuntimeSymbol { name: "gox_defer_pop", ptr: goroutine::gox_defer_pop as *const u8 },
            RuntimeSymbol { name: "gox_panic", ptr: goroutine::gox_panic as *const u8 },
            RuntimeSymbol { name: "gox_recover", ptr: goroutine::gox_recover as *const u8 },
            
            // Select (4)
            RuntimeSymbol { name: "gox_select_start", ptr: goroutine::gox_select_start as *const u8 },
            RuntimeSymbol { name: "gox_select_add_send", ptr: goroutine::gox_select_add_send as *const u8 },
            RuntimeSymbol { name: "gox_select_add_recv", ptr: goroutine::gox_select_add_recv as *const u8 },
            RuntimeSymbol { name: "gox_select_exec", ptr: goroutine::gox_select_exec as *const u8 },
            
            // Iterator (3)
            RuntimeSymbol { name: "gox_iter_begin", ptr: goroutine::gox_iter_begin as *const u8 },
            RuntimeSymbol { name: "gox_iter_next", ptr: goroutine::gox_iter_next as *const u8 },
            RuntimeSymbol { name: "gox_iter_end", ptr: goroutine::gox_iter_end as *const u8 },
            
            // Debug (4)
            RuntimeSymbol { name: "gox_debug_print", ptr: debug::gox_debug_print as *const u8 },
            RuntimeSymbol { name: "gox_assert_begin", ptr: debug::gox_assert_begin as *const u8 },
            RuntimeSymbol { name: "gox_assert_arg", ptr: debug::gox_assert_arg as *const u8 },
            RuntimeSymbol { name: "gox_assert_end", ptr: debug::gox_assert_end as *const u8 },
        ];
        
        Self { symbols }
    }
    
    pub fn get(&self, name: &str) -> Option<*const u8> {
        self.symbols.iter().find(|s| s.name == name).map(|s| s.ptr)
    }
}
```

**⚠️ 注意**：
- 区分 `gox_rt_*` (需要 GC) 和 `gox_*` (不需要 GC)
- `gox_rt_*` 函数在本模块实现，`gox_*` 从 `runtime-core/ffi.rs` 导入

### 3. Stack Map (stack_map.rs)

```rust
/// Stack map 条目
#[derive(Debug, Clone, Default)]
pub struct StackMapEntry {
    /// 栈指针偏移，指向 GcRef
    pub sp_offsets: Vec<i32>,
}

/// 全局 stack map 表
static STACK_MAPS: Lazy<RwLock<HashMap<usize, StackMapEntry>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

/// 注册单个 stack map
pub fn register_stack_map(return_addr: usize, entry: StackMapEntry) {
    if entry.is_empty() { return; }
    STACK_MAPS.write().insert(return_addr, entry);
}

/// 批量注册 (AOT 用)
pub fn register_stack_maps_batch(base_addr: usize, maps: &[(u32, StackMapEntry)]) {
    let mut table = STACK_MAPS.write();
    for (offset, entry) in maps {
        if !entry.is_empty() {
            table.insert(base_addr + *offset as usize, entry.clone());
        }
    }
}

/// 查找
pub fn lookup_stack_map(return_addr: usize) -> Option<StackMapEntry> {
    STACK_MAPS.read().get(&return_addr).cloned()
}

/// 扫描原生栈
pub fn scan_native_stack(gc: &mut Gc) {
    backtrace::trace(|frame| {
        let ip = frame.ip() as usize;
        
        if let Some(entry) = lookup_stack_map(ip) {
            let sp = frame.sp() as usize;
            
            if sp != 0 {
                for &offset in &entry.sp_offsets {
                    let slot_addr = if offset >= 0 {
                        sp.wrapping_add(offset as usize)
                    } else {
                        sp.wrapping_sub((-offset) as usize)
                    };
                    
                    let gc_ref = unsafe { *(slot_addr as *const u64) };
                    if gc_ref != 0 {
                        gc.mark_gray(gc_ref as GcRef);
                    }
                }
            }
        }
        
        true // 继续遍历
    });
}
```

**⚠️ 关键**：
- JIT 编译后必须调用 `register_stack_map` 注册每个 safepoint
- `scan_native_stack` 在 GC collect 时被调用

### 4. Goroutine (goroutine.rs)

```rust
use corosensei::{Coroutine, CoroutineResult};
use crossbeam_deque::{Injector, Worker, Stealer};
use parking_lot::{Mutex, Condvar};

/// Goroutine
pub struct Goroutine {
    id: u64,
    coro: Option<Coroutine<YieldReason, ResumeInput, ()>>,
}

pub enum YieldReason {
    Yield,
    ChannelSend(GcRef, u64),
    ChannelRecv(GcRef),
}

pub enum ResumeInput {
    None,
    ChannelResult(Result<u64, ChannelError>),
}

/// 调度器
pub struct Scheduler {
    next_id: AtomicU64,
    ready_queue: Injector<u64>,
    goroutines: Mutex<HashMap<u64, Goroutine>>,
    // work-stealing
    workers: Vec<Worker<u64>>,
    stealers: Vec<Stealer<u64>>,
}

impl Scheduler {
    pub fn new(num_workers: usize) -> Self;
    
    pub fn spawn(&self, func_ptr: *const u8, args: &[u64]) -> u64;
    
    pub fn yield_current(&self);
    
    pub fn run(&self);
}

/// Channel (线程安全)
pub struct Channel {
    inner: Mutex<ChannelState>,  // 复用 runtime-core 的 ChannelState
    send_cv: Condvar,
    recv_cv: Condvar,
}

impl Channel {
    pub fn new(capacity: usize) -> Self;
    pub fn send(&self, val: u64);
    pub fn recv(&self) -> u64;
    pub fn close(&self);
}
```

**extern "C" 接口**：

```rust
#[no_mangle]
pub extern "C" fn gox_go_spawn(func_ptr: *const u8, arg_ptr: *const u64, arg_count: u32) {
    let args = unsafe { std::slice::from_raw_parts(arg_ptr, arg_count as usize) };
    SCHEDULER.spawn(func_ptr, args);
}

#[no_mangle]
pub extern "C" fn gox_yield() {
    SCHEDULER.yield_current();
}

#[no_mangle]
pub extern "C" fn gox_chan_new(capacity: i64) -> GcRef {
    // 分配 Channel 对象
    // ...
}

#[no_mangle]
pub extern "C" fn gox_chan_send(ch: GcRef, val: u64) {
    // 可能阻塞
    // ...
}

#[no_mangle]
pub extern "C" fn gox_chan_recv(ch: GcRef) -> u64 {
    // 可能阻塞
    // ...
}

#[no_mangle]
pub extern "C" fn gox_chan_close(ch: GcRef) {
    // ...
}
```

**⚠️ 注意**：
- goroutine 阻塞时让出执行权，不阻塞 OS 线程
- Channel 使用 Condvar 实现等待

### 5. Defer/Panic (goroutine.rs 续)

```rust
/// Defer 栈 (每个 goroutine 有一个)
thread_local! {
    static DEFER_STACK: RefCell<Vec<DeferEntry>> = RefCell::new(Vec::new());
    static PANIC_VALUE: RefCell<Option<GcRef>> = RefCell::new(None);
}

#[no_mangle]
pub extern "C" fn gox_defer_push(func_ptr: *const u8, arg_ptr: *const u64, arg_count: u32) {
    DEFER_STACK.with(|stack| {
        let args = unsafe { std::slice::from_raw_parts(arg_ptr, arg_count as usize) };
        stack.borrow_mut().push(DeferEntry {
            func_ptr,
            args: args.to_vec(),
        });
    });
}

#[no_mangle]
pub extern "C" fn gox_defer_pop() {
    DEFER_STACK.with(|stack| {
        if let Some(entry) = stack.borrow_mut().pop() {
            // 调用 deferred 函数
            call_func(entry.func_ptr, &entry.args);
        }
    });
}

#[no_mangle]
pub extern "C" fn gox_panic(val: GcRef) {
    PANIC_VALUE.with(|pv| {
        *pv.borrow_mut() = Some(val);
    });
    
    // 执行所有 defer
    DEFER_STACK.with(|stack| {
        while let Some(entry) = stack.borrow_mut().pop() {
            call_func(entry.func_ptr, &entry.args);
        }
    });
    
    // 真正 panic
    panic!("GoX panic");
}

#[no_mangle]
pub extern "C" fn gox_recover() -> GcRef {
    PANIC_VALUE.with(|pv| {
        pv.borrow_mut().take().unwrap_or(std::ptr::null_mut())
    })
}
```

### 6. Extern Dispatch (extern_dispatch.rs)

```rust
/// Extern 函数类型
pub type ExternFn = fn(&[u64], &mut [u64]) -> Result<(), String>;

/// Extern 函数注册表
static EXTERN_FNS: Lazy<RwLock<HashMap<u32, ExternFn>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn register_extern(extern_id: u32, f: ExternFn) {
    EXTERN_FNS.write().insert(extern_id, f);
}

#[no_mangle]
pub extern "C" fn gox_extern_call(
    extern_id: u32,
    args: *const u64,
    arg_count: u32,
    rets: *mut u64,
    ret_count: u32,
) {
    let args_slice = unsafe { std::slice::from_raw_parts(args, arg_count as usize) };
    let rets_slice = unsafe { std::slice::from_raw_parts_mut(rets, ret_count as usize) };
    
    if let Some(f) = EXTERN_FNS.read().get(&extern_id) {
        if let Err(e) = f(args_slice, rets_slice) {
            // panic 或设置错误
            panic!("extern call failed: {}", e);
        }
    } else {
        panic!("unknown extern function: {}", extern_id);
    }
}
```

### 7. Stdlib Extern 实现 (extern_fns/mod.rs)

```rust
pub fn register_all() {
    // math
    register_extern(MATH_SQRT, |args, rets| {
        rets[0] = f64::from_bits(args[0]).sqrt().to_bits();
        Ok(())
    });
    
    // strings
    register_extern(STRINGS_INDEX, |args, rets| {
        let s = args[0] as GcRef;
        let substr = args[1] as GcRef;
        rets[0] = strings::index(s, substr) as u64;
        Ok(())
    });
    
    // ...
}
```

### 8. Debug (debug.rs)

```rust
#[no_mangle]
pub extern "C" fn gox_debug_print(val: u64, kind: u8) {
    let vk = ValueKind::from_u8(kind);
    match vk {
        ValueKind::Int => println!("{}", val as i64),
        ValueKind::Float64 => println!("{}", f64::from_bits(val)),
        ValueKind::Bool => println!("{}", val != 0),
        ValueKind::String => {
            let s = val as GcRef;
            let bytes = string::as_bytes(s);
            println!("{}", std::str::from_utf8(bytes).unwrap_or("<invalid utf8>"));
        }
        _ => println!("<value kind={:?}>", vk),
    }
}

#[no_mangle]
pub extern "C" fn gox_assert_begin() {
    // 开始收集 assert 参数
}

#[no_mangle]
pub extern "C" fn gox_assert_arg(val: u64, kind: u8) {
    // 收集一个参数
}

#[no_mangle]
pub extern "C" fn gox_assert_end(passed: bool) {
    if !passed {
        // 打印收集的参数
        panic!("assertion failed");
    }
}
```

## Tasks Checklist

### gc_global.rs
- [ ] GLOBAL_GC
- [ ] GLOBALS / GLOBALS_IS_REF
- [ ] FUNC_TABLE
- [ ] init_* 函数
- [ ] gox_rt_alloc
- [ ] gox_gc_read_slot / gox_gc_write_slot
- [ ] gox_rt_write_barrier
- [ ] gox_rt_get_global / gox_rt_set_global
- [ ] gox_func_table_ptr
- [ ] gox_rt_string_* (需要 GC 的 string 操作)
- [ ] gox_rt_array_* / gox_rt_slice_*
- [ ] gox_rt_closure_* / gox_rt_upval_box_*
- [ ] collect_garbage

### symbols.rs
- [ ] RuntimeSymbol / RuntimeSymbols
- [ ] 所有 ~70 个符号注册

### stack_map.rs
- [ ] StackMapEntry
- [ ] register_stack_map
- [ ] register_stack_maps_batch
- [ ] lookup_stack_map
- [ ] scan_native_stack

### goroutine.rs
- [ ] Goroutine 结构
- [ ] Scheduler
- [ ] Channel (Mutex<ChannelState>)
- [ ] gox_go_spawn
- [ ] gox_yield
- [ ] gox_chan_*

### defer/panic
- [ ] DEFER_STACK
- [ ] gox_defer_push / gox_defer_pop
- [ ] gox_panic / gox_recover

### select
- [ ] SelectState
- [ ] gox_select_*

### extern_dispatch.rs
- [ ] ExternFn 类型
- [ ] register_extern
- [ ] gox_extern_call

### extern_fns/
- [ ] math 函数
- [ ] strings 函数
- [ ] 其他 stdlib

### debug.rs
- [ ] gox_debug_print
- [ ] gox_assert_*

## 单元测试

```rust
#[test]
fn test_gc_global() {
    init_gc();
    let obj = gox_rt_alloc(ValueKind::Struct as u8, 0, 2);
    assert!(!obj.is_null());
}

#[test]
fn test_stack_map() {
    clear_stack_maps();
    register_stack_map(0x1000, StackMapEntry::new(vec![8, 16]));
    assert!(lookup_stack_map(0x1000).is_some());
}

#[test]
fn test_channel() {
    let ch = Channel::new(1);
    ch.send(42);
    assert_eq!(ch.recv(), 42);
}
```
