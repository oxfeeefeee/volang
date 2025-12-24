# Backend P3: vo-codegen-vm

**Parent**: [2025-12-23-backend-rewrite-plan.md](2025-12-23-backend-rewrite-plan.md)  
**Status**: Planning  
**Depends On**: P1 (runtime-core), P2 (vo-vm), vo-analysis (类型检查+逃逸分析)

## Overview

将类型检查后的 AST 编译为 VM bytecode。

**输入**：
- `vo_analysis::Project` (含 `TypeInfo`, `TCObjects`, `escaped_vars`, `files`)

**输出**：
- `vo_vm::bytecode::Module`

**核心原则**：
- 查询逃逸分析结果决定栈/堆分配
- 为每个函数生成正确的 `slot_types`
- 正确处理嵌套 struct 扁平化

## Module Structure

```
vo-codegen-vm/
├── lib.rs           # compile_project 入口
├── context.rs       # CodegenContext
├── func.rs          # FuncBuilder
├── type_info.rs     # TypeInfo 包装 (slot 布局计算)
├── expr.rs          # 表达式编译
├── stmt.rs          # 语句编译
└── error.rs         # CodegenError
```

---

## 1. CodegenContext (context.rs)

包级编译上下文，管理函数/全局变量/常量池/类型元数据。

```rust
pub struct CodegenContext {
    module: Module,
    
    // 函数索引: (receiver_type, name) -> func_id
    func_indices: HashMap<(Option<TypeKey>, Symbol), u32>,
    
    // 外部函数索引: name -> extern_id
    extern_indices: HashMap<Symbol, u32>,
    
    // 全局变量索引: name -> global_idx
    global_indices: HashMap<Symbol, u32>,
    
    // 常量池: value -> const_idx
    const_int: HashMap<i64, u16>,
    const_float: HashMap<u64, u16>,   // f64 as bits
    const_string: HashMap<String, u16>,
    
    // 类型 meta_id 分配
    struct_meta_ids: HashMap<TypeKey, u16>,
    interface_meta_ids: HashMap<TypeKey, u16>,
    
    // ObjKey -> func_id (用于 StructMeta.methods)
    objkey_to_func: HashMap<ObjKey, u32>,
    
    // init 函数列表 (按声明顺序)
    init_functions: Vec<u32>,
}
```

### 关键接口

```rust
impl CodegenContext {
    pub fn new(name: &str) -> Self;
    
    // === 类型 meta_id 注册 ===
    pub fn register_struct_type(&mut self, type_key: TypeKey) -> u16;
    pub fn register_interface_type(&mut self, type_key: TypeKey) -> u16;
    pub fn get_struct_meta_id(&self, type_key: TypeKey) -> Option<u16>;
    pub fn get_interface_meta_id(&self, type_key: TypeKey) -> Option<u16>;
    
    // === 函数注册 ===
    pub fn register_func(&mut self, recv: Option<TypeKey>, name: Symbol) -> u32;
    pub fn get_func_index(&self, recv: Option<TypeKey>, name: Symbol) -> Option<u32>;
    pub fn register_extern(&mut self, name: Symbol, def: ExternDef) -> u32;
    pub fn get_extern_index(&self, name: Symbol) -> Option<u32>;
    
    // === 全局变量 ===
    pub fn register_global(&mut self, name: Symbol, def: GlobalDef) -> u32;
    pub fn get_global_index(&self, name: Symbol) -> Option<u32>;
    
    // === 常量池 ===
    pub fn const_int(&mut self, val: i64) -> u16;
    pub fn const_float(&mut self, val: f64) -> u16;
    pub fn const_string(&mut self, val: &str) -> u16;
    pub fn const_value_meta(&mut self, vk: ValueKind, meta_id: u32) -> u16;
    
    // === ObjKey 映射 ===
    pub fn register_objkey_func(&mut self, obj: ObjKey, func_id: u32);
    pub fn get_func_by_objkey(&self, obj: ObjKey) -> Option<u32>;
    
    // === init 函数 ===
    pub fn register_init_function(&mut self, func_id: u32);
    
    // === 完成 ===
    pub fn add_function(&mut self, func: FunctionDef) -> u32;
    pub fn finish(self) -> Module;
}
```

---

## 2. FuncBuilder (func.rs)

单个函数的构建器，管理局部变量/slot分配/指令发射。

```rust
/// 局部变量信息
pub struct LocalVar {
    pub symbol: Symbol,
    pub slot: u16,           // 起始 slot
    pub slots: u16,          // 占用 slot 数
    pub is_heap: bool,       // true = 堆分配(GcRef), false = 栈分配
}

/// 循环上下文 (for break/continue)
struct LoopContext {
    continue_pc: usize,      // continue 跳转目标
    break_patches: Vec<usize>, // break 待 patch 位置
    label: Option<Symbol>,   // 可选标签
}

/// 函数构建器
pub struct FuncBuilder {
    name: String,
    param_count: u16,
    param_slots: u16,
    ret_slots: u16,
    next_slot: u16,
    locals: HashMap<Symbol, LocalVar>,
    slot_types: Vec<SlotType>,
    code: Vec<Instruction>,
    loop_stack: Vec<LoopContext>,
}
```

### 关键接口

```rust
impl FuncBuilder {
    pub fn new(name: &str) -> Self;
    
    // === 参数定义 ===
    pub fn define_param(&mut self, sym: Symbol, slots: u16, slot_types: &[SlotType]) -> u16;
    
    // === 局部变量定义 ===
    /// 栈分配 (非逃逸)
    pub fn define_local_stack(&mut self, sym: Symbol, slots: u16, slot_types: &[SlotType]) -> u16;
    /// 堆分配 (逃逸) - 1 slot GcRef
    pub fn define_local_heap(&mut self, sym: Symbol) -> u16;
    
    // === 查询 ===
    pub fn lookup_local(&self, sym: Symbol) -> Option<&LocalVar>;
    pub fn is_heap_local(&self, sym: Symbol) -> bool;
    
    // === 临时变量 ===
    pub fn alloc_temp(&mut self, slots: u16) -> u16;
    pub fn alloc_temp_typed(&mut self, slot_types: &[SlotType]) -> u16;
    
    // === 指令发射 ===
    pub fn emit(&mut self, inst: Instruction);
    pub fn emit_op(&mut self, op: Opcode, a: u16, b: u16, c: u16);
    pub fn emit_with_flags(&mut self, op: Opcode, flags: u8, a: u16, b: u16, c: u16);
    
    // === 跳转 ===
    pub fn current_pc(&self) -> usize;
    pub fn emit_jump(&mut self, op: Opcode, cond_reg: u16) -> usize;  // 返回待 patch 位置
    pub fn emit_jump_to(&mut self, op: Opcode, cond_reg: u16, target: usize);
    pub fn patch_jump(&mut self, pc: usize, target: usize);
    
    // === 循环 ===
    pub fn enter_loop(&mut self, continue_pc: usize, label: Option<Symbol>);
    pub fn exit_loop(&mut self) -> Vec<usize>;  // 返回 break patches
    pub fn emit_break(&mut self, label: Option<Symbol>);
    pub fn emit_continue(&mut self, label: Option<Symbol>);
    
    // === 完成 ===
    pub fn set_ret_slots(&mut self, slots: u16);
    pub fn build(self) -> FunctionDef;
}
```

---

## 3. TypeInfo Wrapper (type_info.rs)

封装 vo-analysis 的类型查询，提供 VM slot 布局计算。

```rust
pub struct TypeInfoWrapper<'a> {
    pub project: &'a Project,
}

impl<'a> TypeInfoWrapper<'a> {
    // === 表达式类型查询 ===
    pub fn expr_type(&self, expr: &Expr) -> Option<TypeKey>;
    pub fn expr_mode(&self, expr: &Expr) -> Option<&OperandMode>;
    pub fn expr_const_value(&self, expr: &Expr) -> Option<&ConstValue>;
    
    // === 类型表达式查询 ===
    pub fn type_expr_type(&self, type_expr: &TypeExpr) -> Option<TypeKey>;
    
    // === 定义/使用查询 ===
    pub fn get_def(&self, ident: &Ident) -> Option<ObjKey>;
    pub fn get_use(&self, ident: &Ident) -> Option<ObjKey>;
    
    // === 逃逸查询 ===
    pub fn is_escaped(&self, obj: ObjKey) -> bool;
    
    // === Selection 查询 (字段/方法选择) ===
    pub fn get_selection(&self, expr_id: ExprId) -> Option<&Selection>;
    
    // === Slot 布局计算 ===
    pub fn type_slot_count(&self, type_key: TypeKey) -> u16;
    pub fn type_slot_types(&self, type_key: TypeKey) -> Vec<SlotType>;
    pub fn type_value_kind(&self, type_key: TypeKey) -> ValueKind;
    
    // === Struct 布局 ===
    pub fn struct_field_offset(&self, type_key: TypeKey, field: &str) -> Option<(u16, u16)>; // (slot_offset, slot_count)
    pub fn struct_field_slot_types(&self, type_key: TypeKey) -> Vec<SlotType>;
    
    // === Interface ===
    pub fn is_interface(&self, type_key: TypeKey) -> bool;
    pub fn interface_method_index(&self, iface_key: TypeKey, method: &str) -> Option<usize>;
    
    // === 方法查询 ===
    pub fn lookup_method(&self, recv_type: TypeKey, method: &str) -> Option<ObjKey>;
}
```

### Slot 布局规则

| 类型 | Slots | SlotTypes |
|------|-------|-----------|
| `int`, `float64`, `bool` | 1 | `[Value]` |
| `string` | 1 | `[GcRef]` |
| `*T` | 1 | `[GcRef]` |
| `[]T`, `map`, `chan`, `func` | 1 | `[GcRef]` |
| `interface{}` | 2 | `[Interface0, Interface1]` |
| `struct{a int; b string}` | 2 | `[Value, GcRef]` |
| `[3]int` | 3 | `[Value, Value, Value]` |

**嵌套 struct 扁平化**:
```vo
type Inner struct { a int; b string }  // [Value, GcRef]
type Outer struct { inner Inner; c int }  // [Value, GcRef, Value]
```

---

## 4. Expression Compilation (expr.rs)

```rust
/// 编译表达式，返回结果所在 slot
pub fn compile_expr(
    expr: &Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<u16, CodegenError>;

/// 编译表达式到指定 slot
pub fn compile_expr_to(
    expr: &Expr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError>;
```

### 4.1 字面量

```rust
ExprKind::IntLit(lit) => {
    let val = parse_int(lit);
    if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
        // 小整数内联
        func.emit_op(Opcode::LoadInt, dst, val as u16, (val >> 16) as u16);
    } else {
        // 大整数从常量池
        let idx = ctx.const_int(val);
        func.emit_op(Opcode::LoadConst, dst, idx, 0);
    }
}

ExprKind::FloatLit(lit) => {
    let idx = ctx.const_float(parse_float(lit));
    func.emit_op(Opcode::LoadConst, dst, idx, 0);
}

ExprKind::StringLit(lit) => {
    let idx = ctx.const_string(&lit.value);
    func.emit_op(Opcode::StrNew, dst, idx, 0);
}
```

### 4.2 标识符

```rust
ExprKind::Ident(ident) => {
    if let Some(local) = func.lookup_local(ident.symbol) {
        if local.slots == 1 {
            func.emit_op(Opcode::Copy, dst, local.slot, 0);
        } else {
            func.emit_with_flags(Opcode::CopyN, local.slots as u8, dst, local.slot, local.slots);
        }
    } else if let Some(global_idx) = ctx.get_global_index(ident.symbol) {
        let global = &ctx.module.globals[global_idx as usize];
        if global.slots == 1 {
            func.emit_op(Opcode::GlobalGet, dst, global_idx as u16, 0);
        } else {
            func.emit_with_flags(Opcode::GlobalGetN, global.slots as u8, dst, global_idx as u16, 0);
        }
    }
}
```

### 4.3 二元运算

```rust
ExprKind::Binary(bin) => {
    let left_reg = compile_expr(&bin.left, ctx, func, info)?;
    let right_reg = compile_expr(&bin.right, ctx, func, info)?;
    
    let type_key = info.expr_type(expr).unwrap();
    let vk = info.type_value_kind(type_key);
    
    let opcode = match (bin.op, vk) {
        (BinaryOp::Add, ValueKind::Int) => Opcode::AddI,
        (BinaryOp::Add, ValueKind::Float64) => Opcode::AddF,
        (BinaryOp::Add, ValueKind::String) => Opcode::StrConcat,
        (BinaryOp::Sub, ValueKind::Int) => Opcode::SubI,
        // ...
    };
    
    func.emit_op(opcode, dst, left_reg, right_reg);
}
```

### 4.4 字段访问 (Selector) - ⚠️ 栈/堆分支

```rust
ExprKind::Selector(sel) => {
    let base_type = info.expr_type(&sel.expr).unwrap();
    
    // 检查 base 是否是栈分配的 struct
    if let ExprKind::Ident(ident) = &sel.expr.kind {
        if let Some(local) = func.lookup_local(ident.symbol) {
            if !local.is_heap {
                // ⚠️ 栈 struct: 直接用 slot 偏移
                let (offset, slots) = info.struct_field_offset(base_type, &sel.sel.name)?;
                let field_slot = local.slot + offset;
                
                if slots == 1 {
                    func.emit_op(Opcode::Copy, dst, field_slot, 0);
                } else {
                    func.emit_with_flags(Opcode::CopyN, slots as u8, dst, field_slot, slots);
                }
                return Ok(dst);
            }
        }
    }
    
    // ⚠️ 堆 struct: 用 PtrGet
    let base_reg = compile_expr(&sel.expr, ctx, func, info)?;
    let (offset, slots) = info.struct_field_offset(base_type, &sel.sel.name)?;
    
    if slots == 1 {
        func.emit_op(Opcode::PtrGet, dst, base_reg, offset);
    } else {
        func.emit_with_flags(Opcode::PtrGetN, slots as u8, dst, base_reg, offset);
    }
}
```

### 4.5 函数调用

```rust
ExprKind::Call(call) => {
    // 1. 编译参数
    let arg_start = func.next_slot;
    let mut arg_slots = 0u16;
    for arg in &call.args {
        let arg_type = info.expr_type(arg).unwrap();
        let slots = info.type_slot_count(arg_type);
        compile_expr_to(arg, arg_start + arg_slots, ctx, func, info)?;
        arg_slots += slots;
    }
    
    // 2. 确定调用类型
    match &call.func.kind {
        ExprKind::Ident(ident) => {
            // 普通函数调用
            if let Some(func_idx) = ctx.get_func_index(None, ident.symbol) {
                let ret_slots = /* from function signature */;
                let c = ((arg_slots as u16) << 8) | (ret_slots as u16);
                func.emit_with_flags(Opcode::Call, (func_idx >> 16) as u8, 
                                     (func_idx & 0xFFFF) as u16, arg_start, c);
            } else if let Some(extern_idx) = ctx.get_extern_index(ident.symbol) {
                // CallExtern
            }
        }
        ExprKind::Selector(sel) => {
            // 方法调用 - 检查是否是 interface
            let recv_type = info.expr_type(&sel.expr).unwrap();
            if info.is_interface(recv_type) {
                // CallIface
            } else {
                // 普通方法调用
            }
        }
        _ => {
            // 闭包调用
            let closure_reg = compile_expr(&call.func, ctx, func, info)?;
            // CallClosure
        }
    }
}
```

### 4.6 闭包 (FuncLit)

```rust
ExprKind::FuncLit(func_lit) => {
    // 1. 分析捕获变量 (TODO: 从 analysis 获取，或在此重新分析)
    let captures = analyze_captures(func_lit, func, info);
    
    // 2. 编译闭包函数体
    let inner_func_id = compile_closure_func(func_lit, &captures, ctx, info)?;
    
    if captures.is_empty() {
        // 无捕获 - 只需函数指针
        func.emit_op(Opcode::LoadInt, dst, inner_func_id as u16, (inner_func_id >> 16) as u16);
    } else {
        // 有捕获 - 创建 closure 对象
        func.emit_with_flags(
            Opcode::ClosureNew,
            (inner_func_id >> 16) as u8,
            dst,
            (inner_func_id & 0xFFFF) as u16,
            captures.len() as u16
        );
        
        // 填充捕获变量 (都是 GcRef，因为已逃逸)
        for (i, cap) in captures.iter().enumerate() {
            let local = func.lookup_local(cap.symbol).unwrap();
            func.emit_op(Opcode::ClosureSet, i as u16, local.slot, 0);
        }
    }
}
```

---

## 5. Statement Compilation (stmt.rs)

```rust
pub fn compile_stmt(
    stmt: &Stmt,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError>;
```

### 5.1 变量声明 - ⚠️ 逃逸决策

```rust
StmtKind::Var(var_decl) => {
    for spec in &var_decl.specs {
        let ty = info.type_expr_type(&spec.ty)?;
        
        for (i, name) in spec.names.iter().enumerate() {
            let obj_key = info.get_def(name);
            let escapes = obj_key.map(|k| info.is_escaped(k)).unwrap_or(true);
            
            let vk = info.type_value_kind(ty);
            
            if vk == ValueKind::Struct || vk == ValueKind::Array {
                if escapes {
                    // ⚠️ 逃逸: 堆分配
                    let slot = func.define_local_heap(name.symbol);
                    let meta_id = ctx.get_struct_meta_id(ty).unwrap();
                    let meta_const = ctx.const_value_meta(vk, meta_id as u32);
                    let size_slots = info.type_slot_count(ty);
                    
                    let tmp = func.alloc_temp(1);
                    func.emit_op(Opcode::LoadConst, tmp, meta_const, 0);
                    func.emit_with_flags(Opcode::PtrNew, size_slots as u8, slot, tmp, 0);
                } else {
                    // ⚠️ 不逃逸: 栈分配 (扁平化)
                    let slot_types = info.type_slot_types(ty);
                    let slot = func.define_local_stack(name.symbol, slot_types.len() as u16, &slot_types);
                    // 零初始化
                    for j in 0..slot_types.len() {
                        func.emit_op(Opcode::LoadNil, slot + j as u16, 0, 0);
                    }
                }
            } else {
                // 基础类型/引用类型
                let slot_types = info.type_slot_types(ty);
                func.define_local_stack(name.symbol, slot_types.len() as u16, &slot_types);
            }
            
            // 初始化
            if i < spec.values.len() {
                let local = func.lookup_local(name.symbol).unwrap();
                compile_expr_to(&spec.values[i], local.slot, ctx, func, info)?;
            }
        }
    }
}
```

### 5.2 赋值

```rust
StmtKind::Assign(assign) => {
    for (lhs, rhs) in assign.lhs.iter().zip(assign.rhs.iter()) {
        compile_assign_single(lhs, rhs, ctx, func, info)?;
    }
}

fn compile_assign_single(lhs: &Expr, rhs: &Expr, ...) -> Result<(), CodegenError> {
    match &lhs.kind {
        ExprKind::Ident(ident) => {
            let local = func.lookup_local(ident.symbol).unwrap();
            compile_expr_to(rhs, local.slot, ctx, func, info)?;
        }
        ExprKind::Selector(sel) => {
            // 字段赋值 - 检查栈/堆
            // ...
        }
        ExprKind::Index(idx) => {
            // 数组/slice/map 索引赋值
            // ...
        }
        _ => return Err(CodegenError::InvalidLHS),
    }
    Ok(())
}
```

### 5.3 控制流

```rust
// if 语句
StmtKind::If(if_stmt) => {
    if let Some(init) = &if_stmt.init {
        compile_stmt(init, ctx, func, info)?;
    }
    
    let cond_reg = compile_expr(&if_stmt.cond, ctx, func, info)?;
    let else_jump = func.emit_jump(Opcode::JumpIfNot, cond_reg);
    
    compile_block(&if_stmt.then, ctx, func, info)?;
    
    if let Some(else_body) = &if_stmt.else_ {
        let end_jump = func.emit_jump(Opcode::Jump, 0);
        func.patch_jump(else_jump, func.current_pc());
        compile_stmt(else_body, ctx, func, info)?;
        func.patch_jump(end_jump, func.current_pc());
    } else {
        func.patch_jump(else_jump, func.current_pc());
    }
}

// for 语句
StmtKind::For(for_stmt) => {
    match &for_stmt.clause {
        ForClause::Three { init, cond, post } => {
            if let Some(init) = init {
                compile_stmt(init, ctx, func, info)?;
            }
            
            let loop_start = func.current_pc();
            
            let end_jump = if let Some(cond) = cond {
                let cond_reg = compile_expr(cond, ctx, func, info)?;
                Some(func.emit_jump(Opcode::JumpIfNot, cond_reg))
            } else {
                None
            };
            
            let post_pc = func.current_pc();  // continue 跳到 post 之前
            func.enter_loop(post_pc, None);
            
            compile_block(&for_stmt.body, ctx, func, info)?;
            
            if let Some(post) = post {
                compile_stmt(post, ctx, func, info)?;
            }
            
            func.emit_jump_to(Opcode::Jump, 0, loop_start);
            
            if let Some(j) = end_jump {
                func.patch_jump(j, func.current_pc());
            }
            
            let break_patches = func.exit_loop();
            for pc in break_patches {
                func.patch_jump(pc, func.current_pc());
            }
        }
        ForClause::Range { key, value, define, expr } => {
            // for-range 编译
            // ...
        }
        _ => {}
    }
}
```

### 5.4 Defer / ErrDefer

```rust
StmtKind::Defer(defer_stmt) => {
    // defer foo(x, y) → 包装成 0 参数 closure
    let closure_reg = compile_defer_closure(&defer_stmt.call, ctx, func, info)?;
    func.emit_op(Opcode::DeferPush, closure_reg, 0, 0);
}

StmtKind::ErrDefer(errdefer_stmt) => {
    let closure_reg = compile_defer_closure(&errdefer_stmt.call, ctx, func, info)?;
    func.emit_op(Opcode::ErrDeferPush, closure_reg, 0, 0);
}

/// 将 defer 调用包装成 0 参数 closure
fn compile_defer_closure(
    call: &CallExpr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<u16, CodegenError> {
    // 1. 编译所有参数，作为 captures
    let mut captures = Vec::new();
    for arg in &call.args {
        let reg = compile_expr(arg, ctx, func, info)?;
        captures.push(reg);
    }
    
    // 2. 生成 wrapper 函数
    let wrapper_func_id = generate_defer_wrapper(&call.func, &captures, ctx, info)?;
    
    // 3. 创建 closure
    let closure_reg = func.alloc_temp(1);
    func.emit_with_flags(
        Opcode::ClosureNew,
        (wrapper_func_id >> 16) as u8,
        closure_reg,
        (wrapper_func_id & 0xFFFF) as u16,
        captures.len() as u16
    );
    
    // 4. 填充 captures
    for (i, &cap_reg) in captures.iter().enumerate() {
        func.emit_op(Opcode::ClosureSet, i as u16, cap_reg, 0);
    }
    
    Ok(closure_reg)
}
```

---

## 6. Entry Point (lib.rs)

```rust
/// 编译整个项目
pub fn compile_project(project: &Project) -> Result<Module, CodegenError> {
    let info = TypeInfoWrapper::new(project);
    let pkg_name = project.main_pkg().name().unwrap_or("main");
    let mut ctx = CodegenContext::new(pkg_name);
    
    // 1. 注册所有类型 (StructMeta, InterfaceMeta)
    register_types(project, &mut ctx, &info)?;
    
    // 2. 收集声明 (函数、全局变量、extern)
    for file in &project.files {
        collect_declarations(file, &mut ctx, &info)?;
    }
    
    // 3. 编译函数
    for file in &project.files {
        compile_functions(file, &mut ctx, &info)?;
    }
    
    // 4. 生成 __init__ 和 __entry__
    compile_init_and_entry(project, &mut ctx, &info)?;
    
    Ok(ctx.finish())
}
```

---

## 7. 操作指令速查表

### 基础类型

| 操作 | 栈上 | 堆上 (逃逸) |
|------|------|-------------|
| 声明 | 直接分配 slot | `PtrNew` (BoxedInt 等) |
| 读取 | `Copy` | `PtrGet` |
| 写入 | `Copy` | `PtrSet` |

### Struct

| 操作 | 栈上 (不逃逸) | 堆上 (逃逸) |
|------|---------------|-------------|
| 声明 | 分配多 slot + 零初始化 | `PtrNew` |
| 字段读 | `Copy` (slot + offset) | `PtrGet` |
| 字段写 | `Copy` | `PtrSet` |
| 多字段 | `CopyN` | `PtrGetN/SetN` |
| 整体赋值 | `CopyN` | `PtrClone` |
| 取地址 &s | ❌ (会逃逸) | 返回 GcRef |

### Interface

| 操作 | 指令 |
|------|------|
| 声明 (nil) | `LoadNil` x2 |
| 赋值 | `IfaceAssign` |
| 类型断言 | `IfaceAssert` |
| 方法调用 | `CallIface` |

### Closure

| 操作 | 指令 |
|------|------|
| 创建 | `ClosureNew` |
| 读捕获变量 | `ClosureGet` |
| 写捕获变量 | `ClosureSet` |
| 调用 | `CallClosure` |

---

## 8. 已解决问题

### 8.1 捕获变量列表获取 ✅

**问题**: 如何从 vo-analysis 获取闭包的捕获变量列表？

**决定**: 扩展 vo-analysis/escape.rs

**实现**:
- `TypeInfo` 新增字段: `closure_captures: HashMap<ExprId, Vec<ObjKey>>`
- `escape::analyze()` 返回 `EscapeResult { escaped, closure_captures }`
- FuncLit 的 `ExprId` 作为 key，捕获的变量列表作为 value

**使用**:
```rust
// codegen 中获取闭包捕获
let captures = info.project.type_info.closure_captures.get(&func_lit_expr.id);
```

### 8.2 值类型调用指针 Receiver 方法 ✅

**问题**: `t.PointerMethod()` 如何处理？(T 调用 *T 方法)

**决定**: 扩展 escape.rs 标记 receiver 逃逸

**实现**: 在 `Call` 表达式处理中检查：
1. 是否是方法调用 (`SelectionKind::MethodVal`)
2. 方法是否有指针 receiver
3. 调用者是否是值类型

如果满足条件，标记 receiver 的根变量为逃逸。

**Codegen 处理**：
```rust
// t.PointerMethod() - t 已逃逸，local.is_heap = true
// 直接传递 t 的 GcRef 作为 receiver
```

### 8.3 Composite Literal 目标 ✅

**问题**: `&Point{1, 2}` 如何知道要堆分配？

**决定**: Codegen 阶段处理

**实现**: 在编译 `UnaryOp::Addr` 时检查 operand：
```rust
ExprKind::Unary(unary) if unary.op == UnaryOp::Addr => {
    match &unary.operand.kind {
        ExprKind::CompositeLit(lit) => {
            // &Point{1, 2} - 直接堆分配 CompositeLit
            let type_key = info.type_expr_type(&lit.ty)?;
            let meta_id = ctx.get_struct_meta_id(type_key)?;
            // PtrNew + 初始化字段
        }
        _ => {
            // &x - x 应该已经逃逸 (由 escape analysis 标记)
            // 直接返回 x 的 GcRef
        }
    }
}
```

---

## 9. 开发计划

### 测试策略

每个 Phase 完成后写集成测试，同时验证 codegen 和 VM：

```
Vo 源码 → vo-syntax (parse) → vo-analysis (check) → vo-codegen-vm → Module → vo-vm (run)
```

### Phase 0: 准备工作 ✅

- [x] 创建设计文档
- [x] 扩展 escape.rs 添加 `closure_captures`
- [x] 扩展 escape.rs 添加指针 receiver 方法调用逃逸检查

### Phase 1: 基础框架 + 简单测试

- [ ] 1.1 创建 vo-codegen-vm crate 骨架
- [ ] 1.2 `context.rs`: CodegenContext
- [ ] 1.3 `func.rs`: FuncBuilder
- [ ] 1.4 `type_info.rs`: TypeInfoWrapper (slot 布局计算)
- [ ] 1.5 `error.rs`: CodegenError
- [ ] 1.6 简单表达式 + 语句 (int 运算/var/return)
- [ ] 1.7 集成测试: codegen + VM 跑简单程序

### Phase 2: Struct/Array

- [ ] 2.1 Struct 声明/字段访问 (栈/堆分支)
- [ ] 2.2 Composite literal
- [ ] 2.3 Array 索引 (静态/动态)
- [ ] 2.4 集成测试

### Phase 3: 函数调用

- [ ] 3.1 普通函数调用
- [ ] 3.2 方法调用 (receiver 适配: 值/指针)
- [ ] 3.3 外部函数调用
- [ ] 3.4 集成测试

### Phase 4: Interface

- [ ] 4.1 IfaceAssign
- [ ] 4.2 CallIface
- [ ] 4.3 类型断言
- [ ] 4.4 集成测试

### Phase 5: Closure

- [ ] 5.1 ClosureNew + captures
- [ ] 5.2 CallClosure
- [ ] 5.3 闭包内访问捕获变量
- [ ] 5.4 集成测试

### Phase 6: 其他

- [ ] 6.1 defer / errdefer
- [ ] 6.2 go
- [ ] 6.3 channel (send/recv/close)
- [ ] 6.4 select
- [ ] 6.5 for-range
- [ ] 6.6 slice/map 操作
- [ ] 6.7 集成测试

### Phase 7: 入口点 + 完整测试

- [ ] 7.1 `compile_project` 入口
- [ ] 7.2 类型注册 (StructMeta/InterfaceMeta)
- [ ] 7.3 `__init__` 和 `__entry__` 生成
- [ ] 7.4 完整集成测试 (test_data/*.vo)
