# Backend P6: gox-aot + gox-jit

**Parent**: [2025-12-23-backend-rewrite-plan.md](2025-12-23-backend-rewrite-plan.md)  
**Status**: Not Started  
**Est. Modules**: 9  
**Depends On**: P5 (codegen-cranelift)

## Overview

编译器入口，组装前面各模块完成编译。

- **gox-aot**: 生成 .o 目标文件
- **gox-jit**: 内存编译并执行

## gox-aot

```rust
/// AOT 编译器
pub struct AotCompiler {
    module: ObjectModule,
    ctx: Context,
    call_conv: CallConv,
}

impl AotCompiler {
    pub fn new() -> Result<Self>;
    pub fn compile_module(&mut self, bytecode: &BytecodeModule) -> Result<()>;
    pub fn finish(self) -> Result<ObjectOutput>;
}
```

## gox-jit

```rust
/// JIT 编译器
pub struct JitCompiler {
    module: JITModule,
    ctx: Context,
    call_conv: CallConv,
}

impl JitCompiler {
    pub fn new() -> Result<Self>;
    pub fn compile_module(&mut self, bytecode: &BytecodeModule) -> Result<()>;
    pub fn run(&self, bytecode: &BytecodeModule) -> Result<()>;
}
```

## ⚠️ 关键注意事项

### 1. Runtime 符号必须先注册
```rust
let symbols = RuntimeSymbols::new();
for sym in symbols.iter() {
    builder.symbol(sym.name, sym.ptr as *const u8);
}
```

### 2. 函数指针表顺序
```rust
self.module.define_function(func_id, ...)?;  // 1. 定义
self.module.finalize_definitions()?;          // 2. 生成机器码
let ptr = self.module.get_finalized_function(func_id);  // 3. 获取指针
set_func_ptr(idx, ptr);                       // 4. 填充表
```

## Tasks Checklist

### gox-aot
- [ ] AotCompiler::new()
- [ ] compile_module()
- [ ] finish() → .o 文件

### gox-jit
- [ ] JitCompiler::new() + 符号注册
- [ ] compile_module()
- [ ] init_runtime()
- [ ] run()
