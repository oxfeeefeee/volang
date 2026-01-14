# Bug Analysis - 2026-01-11

## 4 个 Bug 分析

### Bug 1: method_value

**问题**：`t.M` 作为 method value 时编译不正确。

**根本原因**：`compile_selector` 没有处理 `SelectionKind::MethodVal`，直接当成字段访问。

**正确方案**：
1. 在 `get_expr_source` 中检测 `MethodVal`，返回 `NeedsCompile`
2. 在 `compile_selector` 中添加 `MethodVal` 分支，调用 `compile_method_value`
3. `compile_method_value` 实现：
   - 指针接收者：直接创建闭包，捕获指针
   - 值接收者：需要 wrapper 函数，因为闭包捕获的是 boxed value，需要解包后调用原方法
4. Wrapper 函数生成放在 `context.rs` 的 `get_or_create_method_value_wrapper`

**关键修改文件**：
- `crates/vo-codegen/src/expr/mod.rs`
- `crates/vo-codegen/src/context.rs`

**注意**：Call 指令格式是 `emit_with_flags(Call, func_id_high, func_id_low, args_start, c)`，不是 `emit_op`。

---

### Bug 2: switch_break

**问题**：switch 内的 break 会错误退出外层 loop。

**根本原因**：switch 没有自己的 break context，break 直接作用到外层 loop。

**正确方案**：
1. 在 `LoopContext` 添加 `is_switch: bool` 字段
2. 添加 `enter_switch()` / `exit_switch()` 方法
3. `emit_continue` 需要跳过 switch context（continue 不应作用于 switch）

**关键修改文件**：
- `crates/vo-codegen/src/func.rs`
- `crates/vo-codegen/src/stmt.rs`（switch 编译调用 enter_switch/exit_switch）

---

### Bug 3: defer_capture (已修复)

**问题**：defer 闭包看到的是变量的原始值，而不是修改后的值。

```go
s := "hello"
defer func() { println(s) }()
s = "world"
// 应该打印 "world"，实际打印 "hello"
```

**之前的错误理解**：
> "让 escaped reference types 也被 box 是概念上的错误：reference types 本身就是引用，不应该再 box"

**正确分析**：
这个说法是错的。Box 的目的是让闭包和外部共享**变量的存储位置**，与变量是 value type 还是 reference type 无关。

- **变量**：一个存储位置（slot）
- **值**：存储位置里的内容

对于 reference type 变量 `s`：
- 变量 `s` 是栈上的一个 slot
- 值是一个 GcRef 指向 string 数据
- 当 `s = "world"` 时，修改的是**栈上 slot 的内容**（换成新的 GcRef）

**正确方案**：所有被闭包捕获的变量都需要 box，包括 reference types。

Box 后的内存布局：
```
stack slot -> GcRef(box) -> [GcRef(string)]
```

**修改内容**：
1. `stmt.rs alloc_storage`: 先检查 escapes，再检查 is_reference_type
2. `literal.rs compile_func_lit`: 移除 `!info.is_reference_type(type_key)` 条件
3. `expr/mod.rs`: 统一所有捕获变量的读取逻辑，都用 PtrGet

---

### Bug 4: short_var_redecl

**问题**：`p, q := p+1, p+2` 中，RHS 应该先全部求值，再赋给 LHS。

**根本原因**：当前实现逐个处理 name-value pair，导致 `p+2` 用的是已修改的 p。

**正确方案**：
1. Phase 1：遍历所有 RHS，求值到临时 slot
2. Phase 2：遍历所有 LHS，从临时 slot 赋值/定义

**关键修改文件**：
- `crates/vo-codegen/src/stmt.rs`（ShortVar 处理）

---

## 其他发现

### Call 指令格式
```rust
// 正确
emit_with_flags(Opcode::Call, func_id_high, func_id_low, args_start, c)

// 错误
emit_op(Opcode::Call, args_start, func_id, c)
```

### PtrSet vs PtrGet 参数顺序
- PtrSet: `a=ptr, b=offset, c=val`
- PtrGet: `a=dst, b=ptr, c=offset`

### is_reference_type 包括
- string, slice, map, channel, closure, pointer

### escape.rs 注释（已过时）
```rust
// Reference types (slice, map, chan, closure, pointer) are already GcRef, no escape concept.
```
这个注释的理解是错误的。Reference types 被闭包捕获时也需要 box，以共享存储位置。

---

## 测试状态

- switch_break: 已修复
- short_var: 已修复  
- method_value: 已修复
- defer_capture: 已修复

### 已知的独立 bug（非本次修复范围）

1. **`defer func(val int){...}(c.value)` 返回 0**
   - 闭包字面量作为 defer 目标，带参数时，参数值传递错误
   - 影响 `defer_eval_timing.vo` 中的 Test 5

2. **`escaped_array_init.vo` 数组切片初始化**
   - `arr := [5]int{10,20,30}; return arr[1:4]` 返回全零
   - 预先存在的 bug
