# Backend P2: gox-vm

**Parent**: [2025-12-23-backend-rewrite-plan.md](2025-12-23-backend-rewrite-plan.md)  
**Status**: Not Started  
**Est. Modules**: 36  
**Depends On**: P1 (runtime-core)

## Overview

定义 bytecode 格式和 VM 解释器。这是 GoX 的核心执行引擎。

**核心原则**：
- 8 字节固定指令格式
- 寄存器式 VM（slot = 栈上 8 字节）
- 协作式调度（Fiber）

## 模块清单

### 1. 指令格式 (instruction.rs)

```rust
/// 8 字节固定格式
#[repr(C)]
pub struct Instruction {
    pub op: u8,      // Opcode
    pub flags: u8,   // 标志/变体
    pub a: u16,      // 目标寄存器 或 操作数
    pub b: u16,      // 源操作数 0
    pub c: u16,      // 源操作数 1
}

impl Instruction {
    pub const fn new(op: Opcode, a: u16, b: u16, c: u16) -> Self;
    pub const fn with_flags(op: Opcode, flags: u8, a: u16, b: u16, c: u16) -> Self;
    
    /// 从 b, c 组合 32 位立即数
    pub fn imm32(&self) -> i32 {
        ((self.b as u32) | ((self.c as u32) << 16)) as i32
    }
}
```

### 2. Opcode 枚举 (instruction.rs)

```rust
#[repr(u8)]
pub enum Opcode {
    // === 加载/存储 (0-9) ===
    Nop = 0,
    LoadNil,      // a = nil
    LoadTrue,     // a = true
    LoadFalse,    // a = false
    LoadInt,      // a = sign_extend(b|c)
    LoadConst,    // a = constants[b]
    Copy,         // a = b
    CopyN,        // copy c slots: a..a+c = b..b+c
    
    // === 全局变量 (10-14) ===
    GetGlobal = 10,  // a = globals[b]
    SetGlobal,       // globals[a] = b
    
    // === 算术 i64 (15-24) ===
    AddI64 = 15,
    SubI64,
    MulI64,
    DivI64,       // 有符号除法
    DivU64,       // 无符号除法
    ModI64,
    ModU64,
    NegI64,
    
    // === 算术 f64 (25-34) ===
    AddF64 = 25,
    SubF64,
    MulF64,
    DivF64,
    NegF64,
    
    // === 比较 i64 (35-44) ===
    EqI64 = 35,
    NeI64,
    LtI64,        // 有符号
    LeI64,
    GtI64,
    GeI64,
    LtU64,        // 无符号
    LeU64,
    GtU64,
    GeU64,
    
    // === 比较 f64 (45-54) ===
    EqF64 = 45,
    NeF64,
    LtF64,
    LeF64,
    GtF64,
    GeF64,
    
    // === 引用比较 (55-59) ===
    EqRef = 55,
    NeRef,
    IsNil,
    
    // === 位运算 (60-69) ===
    Band = 60,
    Bor,
    Bxor,
    Bnot,
    Shl,
    ShrS,         // 算术右移 (有符号)
    ShrU,         // 逻辑右移 (无符号)
    
    // === 逻辑 (70-74) ===
    BoolNot = 70,
    
    // === 控制流 (75-79) ===
    Jump = 75,       // pc += imm32(b,c)
    JumpIf,          // if a: pc += imm32(b,c)
    JumpIfNot,       // if !a: pc += imm32(b,c)
    
    // === 函数调用 (80-84) ===
    Call = 80,           // call func[a], args at b, c=arg_count, flags=ret_count
    CallExtern,          // call extern[a], args at b, c=arg_count, flags=ret_count
    CallClosure,         // call closure a, args at b, c=arg_count, flags=ret_count
    CallInterface,       // call iface a method b, args at c, flags=arg|ret
    Return,              // return values at a, count=b
    
    // === 堆内存 (85-94) ===
    PtrNew = 85,         // a = alloc(type_id=b|c<<16, slots=flags)
    PtrClone,            // a = deep_copy(b)
    PtrGet,              // a = b.slot[c]
    PtrSet,              // a.slot[b] = c
    PtrGetN,             // copy c slots: a = b.slot[flags..]
    PtrSetN,             // copy c slots: a.slot[flags..] = b
    
    // === 栈内存 (95-99) ===
    // 用于栈上 struct/array 的字段访问（扁平化后的）
    // 实际上就是 Copy/CopyN，但语义更明确
    
    // === Array/Slice (100-114) ===
    ArrayNew = 100,
    ArrayGet,
    ArraySet,
    ArrayLen,
    SliceNew,
    SliceGet,
    SliceSet,
    SliceLen,
    SliceCap,
    SliceSlice,
    SliceAppend,
    
    // === String (115-129) ===
    StrNew = 115,
    StrConcat,
    StrLen,
    StrIndex,
    StrSlice,
    StrEq,
    StrNe,
    StrLt,
    StrLe,
    StrGt,
    StrGe,
    
    // === Map (130-139) ===
    MapNew = 130,
    MapGet,           // a, ok = map[b], key at c
    MapSet,
    MapDelete,
    MapLen,
    
    // === Interface (140-149) ===
    IfaceBox = 140,   // a = box(value_kind=flags, type_id=b, data=c)
    IfaceUnbox,       // a = unbox(iface=b, expected_type=c), panic if mismatch
    IfaceType,        // a = typeof(iface=b)
    
    // === Closure (150-159) ===
    ClosureNew = 150, // a = new_closure(func_id=b, upval_count=c)
    ClosureGet,       // a = closure.upval[b]
    ClosureSet,       // closure[a].upval[b] = c
    UpvalNew,         // a = new_upval_box()
    UpvalGet,         // a = upval_box[b].value
    UpvalSet,         // upval_box[a].value = b
    
    // === Goroutine (160-169) ===
    Go = 160,         // go func[a](args at b, count=c)
    Yield,
    
    // === Channel (170-179) ===
    ChanNew = 170,    // a = make(chan, cap=b)
    ChanSend,         // a <- b
    ChanRecv,         // a = <-b
    ChanClose,        // close(a)
    
    // === Select (180-189) ===
    SelectBegin = 180,
    SelectSend,
    SelectRecv,
    SelectDefault,
    SelectEnd,
    
    // === Defer/Panic (190-199) ===
    DeferPush = 190,  // defer func[a](args at b, count=c)
    DeferPop,
    Panic,            // panic(a)
    Recover,          // a = recover()
    
    // === Iterator (200-209) ===
    IterBegin = 200,  // begin iteration on container a, type=flags
    IterNext,         // a, b = next key, value; jump if done
    IterEnd,
    
    // === Debug (210-219) ===
    Print = 210,
    Assert,
    
    // === 类型转换 (220-239) ===
    ConvI2F = 220,    // a = float64(b)
    ConvF2I,          // a = int64(b)
    ConvI32,          // a = int32(b) (truncate)
    ConvU32,          // a = uint32(b) (truncate)
    // ... 其他转换
}
```

**⚠️ 注意**：
- Opcode 值经过精心安排，同类指令连续
- flags 字段用途因指令而异（返回值数量、slot 数量等）
- imm32 用于跳转偏移，由 b 和 c 组合

### 3. Bytecode 模块 (bytecode.rs)

```rust
/// 常量
pub enum Constant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// 函数定义
pub struct FunctionDef {
    pub name: String,
    pub param_count: u16,    // 参数个数
    pub param_slots: u16,    // 参数占用的 slot 数
    pub local_slots: u16,    // 局部变量 slot 数
    pub ret_slots: u16,      // 返回值 slot 数
    pub code: Vec<Instruction>,
    pub slot_types: Vec<SlotType>,  // GC 扫描用
}

/// 外部函数
pub struct ExternDef {
    pub name: String,
    pub param_slots: u16,
    pub ret_slots: u16,
}

/// 全局变量
pub struct GlobalDef {
    pub name: String,
    pub slots: u16,
    pub value_kind: u8,
    pub type_id: u16,
}

/// Interface 方法分派
pub struct IfaceDispatchEntry {
    pub concrete_type_id: u16,
    pub iface_type_id: u16,
    pub method_funcs: Vec<u32>,  // 每个方法的 func_id
}

/// Bytecode 模块
pub struct Module {
    pub name: String,
    pub struct_types: Vec<TypeMeta>,
    pub interface_types: Vec<TypeMeta>,
    pub constants: Vec<Constant>,
    pub globals: Vec<GlobalDef>,
    pub functions: Vec<FunctionDef>,
    pub externs: Vec<ExternDef>,
    pub iface_dispatch: Vec<IfaceDispatchEntry>,
    pub entry_func: u32,
}
```

**⚠️ 关键**：
- `slot_types` 必须与 `local_slots` 长度一致
- `iface_dispatch` 在 codegen 时填充，VM 执行时查询

### 4. 序列化 (bytecode.rs 续)

```rust
impl Module {
    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8>;
    
    /// 从字节反序列化
    pub fn deserialize(bytes: &[u8]) -> Result<Self, BytecodeError>;
}

/// 文件格式
/// Magic: "GOXB" (4 bytes)
/// Version: u32
/// struct_types: [TypeMeta]
/// interface_types: [TypeMeta]
/// constants: [Constant]
/// globals: [GlobalDef]
/// functions: [FunctionDef]
/// externs: [ExternDef]
/// iface_dispatch: [IfaceDispatchEntry]
/// entry_func: u32
```

### 5. VM 结构 (vm.rs)

```rust
/// 调用帧
pub struct CallFrame {
    pub func_id: u32,
    pub pc: usize,
    pub bp: usize,       // base pointer (栈底)
    pub ret_reg: u16,    // 返回值存放位置
    pub ret_count: u16,
}

/// Defer 条目
pub struct DeferEntry {
    pub frame_depth: usize,
    pub func_id: u32,
    pub arg_count: u8,
    pub args: [u64; 8],  // 最多 8 个参数
}

/// 迭代器
pub enum Iterator {
    Slice { arr: GcRef, pos: usize, len: usize },
    Map { map: GcRef, pos: usize },
    String { s: GcRef, byte_pos: usize },
    IntRange { cur: i64, end: i64, step: i64 },
}

/// Fiber (协程)
pub struct Fiber {
    pub id: u32,
    pub status: FiberStatus,
    pub stack: Vec<u64>,
    pub frames: Vec<CallFrame>,
    pub defer_stack: Vec<DeferEntry>,
    pub iter_stack: Vec<Iterator>,
    pub panic_value: Option<GcRef>,
}

pub enum FiberStatus {
    Running,
    Suspended,
    Dead,
}

/// 调度器
pub struct Scheduler {
    pub fibers: Vec<Fiber>,
    pub ready_queue: VecDeque<u32>,
    pub current: Option<u32>,
}

/// VM 主结构
pub struct Vm {
    pub module: Option<Module>,
    pub gc: Gc,
    pub scheduler: Scheduler,
    pub globals: Vec<u64>,
}
```

### 6. VM 执行 (vm.rs 续)

```rust
impl Vm {
    pub fn new() -> Self;
    
    /// 加载模块
    pub fn load(&mut self, module: Module);
    
    /// 运行入口函数
    pub fn run(&mut self) -> Result<(), VmError>;
    
    /// 执行单条指令
    fn exec_instruction(&mut self, fiber_id: u32) -> ExecResult;
    
    /// 读写寄存器
    fn read_reg(&self, fiber_id: u32, reg: u16) -> u64;
    fn write_reg(&mut self, fiber_id: u32, reg: u16, val: u64);
}

enum ExecResult {
    Continue,
    Return,
    Yield,
    Panic(GcRef),
}
```

### 7. 指令执行详解

#### 7.1 函数调用 (Call)

```rust
Opcode::Call => {
    // a=func_id, b=arg_start, c=arg_count, flags=ret_count
    let func = &module.functions[a as usize];
    
    // 复制参数（避免帧切换后覆盖）
    let args: Vec<u64> = (0..c).map(|i| self.read_reg(fiber_id, b + i)).collect();
    
    // 推入新帧
    fiber.push_frame(a, func.local_slots, b, flags);
    
    // 写入参数到新帧
    for (i, arg) in args.into_iter().enumerate() {
        fiber.write_reg(i as u16, arg);
    }
}
```

#### 7.2 Closure 调用 (CallClosure)

```rust
Opcode::CallClosure => {
    // a=closure_reg, b=arg_start, c=arg_count, flags=ret_count
    let closure = self.read_reg(fiber_id, a) as GcRef;
    let func_id = closure::func_id(closure);
    let func = &module.functions[func_id as usize];
    
    // 复制参数
    let args: Vec<u64> = (0..c).map(|i| self.read_reg(fiber_id, b + i)).collect();
    
    // 推入新帧
    fiber.push_frame(func_id, func.local_slots, b, flags);
    
    // ⚠️ 关键：closure 作为 r0
    fiber.write_reg(0, closure as u64);
    
    // 参数从 r1 开始
    for (i, arg) in args.into_iter().enumerate() {
        fiber.write_reg((i + 1) as u16, arg);
    }
}
```

#### 7.3 Interface 调用 (CallInterface)

```rust
Opcode::CallInterface => {
    // a=iface_reg, b=method_idx, c=arg_start
    // flags 低 4 位 = arg_count, 高 4 位 = ret_count
    let (slot0, slot1) = self.read_interface(fiber_id, a);
    let concrete_type_id = extract_concrete_type(slot0);
    let iface_type_id = extract_iface_type(slot0);
    
    // 查 dispatch table
    let entry = module.find_dispatch(concrete_type_id, iface_type_id);
    let func_id = entry.method_funcs[b as usize];
    
    // 像普通函数一样调用，但 receiver 是 slot1
    // ...
}
```

#### 7.4 GC 根扫描

```rust
impl Vm {
    /// 扫描所有 GC 根
    pub fn scan_roots(&self, gc: &mut Gc) {
        // 1. 扫描全局变量
        for (i, &val) in self.globals.iter().enumerate() {
            let def = &self.module.as_ref().unwrap().globals[i];
            if needs_gc(def.value_kind) && val != 0 {
                gc.mark_gray(val as GcRef);
            }
        }
        
        // 2. 扫描所有 Fiber 的栈
        for fiber in &self.scheduler.fibers {
            for frame in &fiber.frames {
                let func = &module.functions[frame.func_id as usize];
                for (i, &st) in func.slot_types.iter().enumerate() {
                    let slot_idx = frame.bp + i;
                    match st {
                        SlotType::GcRef => {
                            let val = fiber.stack[slot_idx];
                            if val != 0 { gc.mark_gray(val as GcRef); }
                        }
                        SlotType::Interface1 => {
                            // 动态检查前一个 slot
                            let header = fiber.stack[slot_idx - 1];
                            if needs_gc(extract_value_kind(header)) {
                                let val = fiber.stack[slot_idx];
                                if val != 0 { gc.mark_gray(val as GcRef); }
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            // 3. 扫描 defer_stack
            // 4. 扫描 iter_stack
        }
    }
}
```

## Tasks Checklist

### instruction.rs
- [ ] Instruction 结构
- [ ] Opcode 枚举（完整 ~100 个）
- [ ] 辅助方法 (imm32, opcode)

### bytecode.rs
- [ ] Constant 枚举
- [ ] FunctionDef
- [ ] ExternDef
- [ ] GlobalDef
- [ ] IfaceDispatchEntry
- [ ] TypeMeta (从 runtime-core 引入或重定义)
- [ ] Module
- [ ] Module::serialize
- [ ] Module::deserialize

### vm.rs - 结构
- [ ] CallFrame
- [ ] DeferEntry
- [ ] Iterator 枚举
- [ ] Fiber
- [ ] FiberStatus
- [ ] Scheduler
- [ ] Vm

### vm.rs - 执行
- [ ] exec 加载/存储 (Nop..CopyN)
- [ ] exec 全局变量 (GetGlobal, SetGlobal)
- [ ] exec 算术 i64
- [ ] exec 算术 f64
- [ ] exec 比较
- [ ] exec 位运算
- [ ] exec 控制流 (Jump, JumpIf, JumpIfNot)
- [ ] exec 函数调用 (Call, CallExtern, Return)
- [ ] exec CallClosure
- [ ] exec CallInterface
- [ ] exec 堆内存 (PtrNew, PtrGet, PtrSet, ...)
- [ ] exec Array/Slice
- [ ] exec String
- [ ] exec Map
- [ ] exec Interface (IfaceBox, IfaceUnbox)
- [ ] exec Closure (ClosureNew, ClosureGet, ClosureSet, Upval*)
- [ ] exec Goroutine (Go, Yield)
- [ ] exec Channel
- [ ] exec Select
- [ ] exec Defer/Panic/Recover
- [ ] exec Iterator
- [ ] exec 类型转换
- [ ] exec Debug (Print, Assert)

### vm.rs - 调度
- [ ] Scheduler 实现
- [ ] Fiber 切换
- [ ] Channel 阻塞/唤醒

### vm.rs - GC 集成
- [ ] scan_roots 实现
- [ ] 调用 gc.collect 的时机

## 单元测试

```rust
#[test]
fn test_arithmetic() {
    let mut vm = Vm::new();
    // 构造简单的加法 bytecode
    // 执行并验证结果
}

#[test]
fn test_function_call() {
    // 测试函数调用和返回
}

#[test]
fn test_closure_call() {
    // 测试闭包调用，验证 closure 作为 r0
}

#[test]
fn test_gc_roots() {
    // 测试 GC 根扫描正确性
}
```
