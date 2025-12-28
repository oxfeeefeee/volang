//! JIT compiler for Vo bytecode using Cranelift.
//!
//! # Architecture
//!
//! - `JitCompiler`: Main entry point, owns Cranelift JITModule
//! - `JitCache`: Maps func_id -> CompiledFunction
//! - `FunctionCompiler`: Compiles a single function (bytecode -> Cranelift IR)
//! - `GcRefTracker`: Tracks GcRef variables for stack map generation
//!
//! # JIT Function Signature
//!
//! All JIT functions use the same C ABI signature:
//! ```ignore
//! extern "C" fn(ctx: *mut JitContext, args: *mut u64, ret: *mut u64) -> JitResult
//! ```
//!
//! - `ctx`: Runtime context (GC, globals, panic flag, etc.)
//! - `args`: Pointer to argument slots (directly points to VM stack)
//! - `ret`: Pointer to return value slots (same location as args for in-place return)

mod compiler;
mod gc_tracking;
mod translate;

pub use compiler::FunctionCompiler;
pub use gc_tracking::{GcRefTracker, StackMap};

use std::collections::HashMap;

use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Module;
use cranelift_codegen::settings::{self, Configurable};

use vo_runtime::bytecode::{FunctionDef, Module as VoModule};
use vo_runtime::instruction::Opcode;
use vo_runtime::jit_api::{JitContext, JitResult};

// =============================================================================
// JitError
// =============================================================================

#[derive(Debug)]
pub enum JitError {
    /// Cranelift module error
    Module(cranelift_module::ModuleError),
    /// Cranelift codegen error
    Codegen(cranelift_codegen::CodegenError),
    /// Function not found
    FunctionNotFound(u32),
    /// Function cannot be JIT compiled (has defer/channel/go/select)
    NotJittable(u32),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for JitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitError::Module(e) => write!(f, "Cranelift module error: {}", e),
            JitError::Codegen(e) => write!(f, "Cranelift codegen error: {}", e),
            JitError::FunctionNotFound(id) => write!(f, "function not found: {}", id),
            JitError::NotJittable(id) => write!(f, "function {} cannot be JIT compiled", id),
            JitError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for JitError {}

impl From<cranelift_module::ModuleError> for JitError {
    fn from(e: cranelift_module::ModuleError) -> Self {
        JitError::Module(e)
    }
}

impl From<cranelift_codegen::CodegenError> for JitError {
    fn from(e: cranelift_codegen::CodegenError) -> Self {
        JitError::Codegen(e)
    }
}

// =============================================================================
// CompiledFunction
// =============================================================================

/// A compiled JIT function.
pub struct CompiledFunction {
    /// Pointer to the compiled native code.
    pub code_ptr: *const u8,
    /// Size of the compiled code in bytes.
    pub code_size: usize,
    /// Stack map for GC scanning.
    pub stack_map: StackMap,
    /// Number of parameter slots (for validation).
    pub param_slots: u16,
    /// Number of return value slots (for validation).
    pub ret_slots: u16,
}

// SAFETY: The code_ptr points to executable memory managed by Cranelift.
// The pointer remains valid as long as JitCompiler is alive.
unsafe impl Send for CompiledFunction {}
unsafe impl Sync for CompiledFunction {}

/// JIT function pointer type.
///
/// # Arguments
/// - `ctx`: Runtime context
/// - `args`: Pointer to argument slots on VM stack
/// - `ret`: Pointer to return value slots on VM stack
///
/// # Returns
/// - `JitResult::Ok` on success
/// - `JitResult::Panic` if a panic occurred
pub type JitFunc = extern "C" fn(
    ctx: *mut JitContext,
    args: *mut u64,
    ret: *mut u64,
) -> JitResult;

// =============================================================================
// JitCache
// =============================================================================

/// Cache of compiled JIT functions.
pub struct JitCache {
    functions: HashMap<u32, CompiledFunction>,
}

impl JitCache {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn get(&self, func_id: u32) -> Option<&CompiledFunction> {
        self.functions.get(&func_id)
    }

    pub fn insert(&mut self, func_id: u32, func: CompiledFunction) {
        self.functions.insert(func_id, func);
    }

    pub fn contains(&self, func_id: u32) -> bool {
        self.functions.contains_key(&func_id)
    }

    /// Get the JIT function pointer for a compiled function.
    ///
    /// # Safety
    /// The returned function pointer must only be called with valid JitContext.
    pub unsafe fn get_func_ptr(&self, func_id: u32) -> Option<JitFunc> {
        self.functions.get(&func_id).map(|f| {
            std::mem::transmute(f.code_ptr)
        })
    }
}

impl Default for JitCache {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// JitCompiler
// =============================================================================

/// JIT compiler using Cranelift.
pub struct JitCompiler {
    /// Cranelift JIT module.
    module: JITModule,
    /// Cranelift codegen context (reused across compilations).
    ctx: cranelift_codegen::Context,
    /// Cache of compiled functions.
    cache: JitCache,
}

impl JitCompiler {
    /// Create a new JIT compiler.
    pub fn new() -> Result<Self, JitError> {
        let mut flag_builder = settings::builder();
        // Use speed optimization (not size)
        flag_builder.set("opt_level", "speed").unwrap();
        
        let isa_builder = cranelift_native::builder()
            .map_err(|e| JitError::Internal(e.to_string()))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| JitError::Internal(e.to_string()))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        
        // Register runtime helper symbols
        builder.symbol("vo_gc_safepoint", vo_runtime::jit_api::vo_gc_safepoint as *const u8);
        builder.symbol("vo_gc_alloc", vo_runtime::jit_api::vo_gc_alloc as *const u8);
        builder.symbol("vo_gc_write_barrier", vo_runtime::jit_api::vo_gc_write_barrier as *const u8);
        builder.symbol("vo_call_vm", vo_runtime::jit_api::vo_call_vm as *const u8);

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Ok(Self {
            module,
            ctx,
            cache: JitCache::new(),
        })
    }

    /// Check if a function can be JIT compiled.
    ///
    /// Functions with defer/recover/go/channel/select cannot be JIT compiled
    /// because they require VM scheduler support.
    ///
    /// TODO: This currently scans bytecode. Consider adding flags to FunctionDef
    /// during codegen for O(1) check.
    pub fn can_jit(&self, func: &FunctionDef, _module: &VoModule) -> bool {
        for inst in &func.code {
            match inst.opcode() {
                // Async operations - not supported
                Opcode::DeferPush
                | Opcode::ErrDeferPush
                | Opcode::Recover
                | Opcode::GoStart
                | Opcode::ChanSend
                | Opcode::ChanRecv
                | Opcode::ChanClose
                | Opcode::SelectBegin
                | Opcode::SelectSend
                | Opcode::SelectRecv
                | Opcode::SelectExec => return false,
                _ => {}
            }
        }
        true
    }

    /// Compile a function to native code.
    ///
    /// Returns `Ok(())` if compilation succeeds. The compiled function is stored
    /// in the internal cache and can be retrieved via `get()`.
    pub fn compile(
        &mut self,
        func_id: u32,
        func: &FunctionDef,
        vo_module: &VoModule,
    ) -> Result<(), JitError> {
        use cranelift_codegen::ir::{types, AbiParam, Signature};
        use cranelift_frontend::FunctionBuilderContext;
        
        if !self.can_jit(func, vo_module) {
            return Err(JitError::NotJittable(func_id));
        }

        if self.cache.contains(func_id) {
            return Ok(()); // Already compiled
        }

        // 1. Create function signature: (ctx: *mut, args: *mut, ret: *mut) -> i32
        let ptr_type = self.module.target_config().pointer_type();
        let mut sig = Signature::new(self.module.target_config().default_call_conv);
        sig.params.push(AbiParam::new(ptr_type)); // ctx
        sig.params.push(AbiParam::new(ptr_type)); // args
        sig.params.push(AbiParam::new(ptr_type)); // ret
        sig.returns.push(AbiParam::new(types::I32)); // JitResult

        // 2. Declare function in module
        let func_name = format!("vo_jit_{}", func_id);
        let func_id_cl = self.module.declare_function(&func_name, cranelift_module::Linkage::Local, &sig)?;

        // 3. Clear context and set signature
        self.ctx.func.signature = sig;
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id);

        // 4. Declare runtime helper functions
        let safepoint_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // ctx
            sig
        };
        let safepoint_id = self.module.declare_function(
            "vo_gc_safepoint",
            cranelift_module::Linkage::Import,
            &safepoint_sig,
        )?;
        
        // vo_call_vm(ctx, func_id, args, arg_count, ret, ret_count) -> JitResult
        let call_vm_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // ctx
            sig.params.push(AbiParam::new(types::I32)); // func_id
            sig.params.push(AbiParam::new(ptr_type)); // args
            sig.params.push(AbiParam::new(types::I32)); // arg_count
            sig.params.push(AbiParam::new(ptr_type)); // ret
            sig.params.push(AbiParam::new(types::I32)); // ret_count
            sig.returns.push(AbiParam::new(types::I32)); // JitResult
            sig
        };
        let call_vm_id = self.module.declare_function(
            "vo_call_vm",
            cranelift_module::Linkage::Import,
            &call_vm_sig,
        )?;
        
        // 5. Build function IR using FunctionCompiler
        let mut func_ctx = FunctionBuilderContext::new();
        {
            let safepoint_func = self.module.declare_func_in_func(safepoint_id, &mut self.ctx.func);
            let call_vm_func = self.module.declare_func_in_func(call_vm_id, &mut self.ctx.func);
            let compiler = FunctionCompiler::new(
                &mut self.ctx.func,
                &mut func_ctx,
                func,
                vo_module,
                Some(safepoint_func),
                Some(call_vm_func),
            );
            let stack_map = compiler.compile()?;
            
            // 5. Compile to machine code
            self.module.define_function(func_id_cl, &mut self.ctx)?;
            self.module.clear_context(&mut self.ctx);
            
            // 6. Finalize and get code pointer
            self.module.finalize_definitions()?;
            let code_ptr = self.module.get_finalized_function(func_id_cl);
            let code_size = 0; // TODO: Get actual code size
            
            // 7. Store in cache
            let compiled = CompiledFunction {
                code_ptr,
                code_size,
                stack_map,
                param_slots: func.param_slots,
                ret_slots: func.ret_slots,
            };
            self.cache.insert(func_id, compiled);
        }

        Ok(())
    }

    /// Get a compiled function by ID.
    pub fn get(&self, func_id: u32) -> Option<&CompiledFunction> {
        self.cache.get(func_id)
    }

    /// Get the JIT function pointer for a compiled function.
    ///
    /// # Safety
    /// The returned function pointer must only be called with valid JitContext.
    pub unsafe fn get_func_ptr(&self, func_id: u32) -> Option<JitFunc> {
        self.cache.get_func_ptr(func_id)
    }

    /// Get the internal cache (for VM integration).
    pub fn cache(&self) -> &JitCache {
        &self.cache
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new().expect("failed to create JIT compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vo_runtime::bytecode::{FunctionDef, Module as VoModule};
    use vo_runtime::instruction::{Instruction, Opcode};
    use vo_runtime::SlotType;
    use vo_runtime::jit_api::JitContext;

    fn create_simple_module() -> VoModule {
        VoModule::new("test".into())
    }
    
    /// Test context holder to keep flags alive.
    struct TestContext {
        safepoint_flag: bool,
        panic_flag: bool,
    }
    
    impl TestContext {
        fn new() -> Self {
            Self {
                safepoint_flag: false,
                panic_flag: false,
            }
        }
        
        fn as_jit_ctx(&mut self) -> JitContext {
            JitContext {
                gc: std::ptr::null_mut(),
                globals: std::ptr::null_mut(),
                safepoint_flag: &self.safepoint_flag as *const bool,
                panic_flag: &mut self.panic_flag as *mut bool,
                vm: std::ptr::null_mut(),
                fiber: std::ptr::null_mut(),
                call_vm_fn: None, // No VM callback in unit tests
            }
        }
    }

    fn create_return_42_func() -> FunctionDef {
        // Function that returns 42:
        // slot[0] = 42
        // return slot[0]
        FunctionDef {
            name: "return_42".into(),
            param_count: 0,
            param_slots: 0,
            local_slots: 1,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::LoadInt, 0, 42, 0), // slot[0] = 42
                Instruction::new(Opcode::Return, 0, 0, 0),   // return slot[0]
            ],
            slot_types: vec![SlotType::Value],
        }
    }

    #[test]
    fn test_compile_return_42() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_return_42_func();

        // Compile the function
        jit.compile(0, &func, &module).expect("compilation failed");

        // Verify it's in the cache
        assert!(jit.cache.contains(0));
        let compiled = jit.get(0).expect("function not in cache");
        assert_eq!(compiled.param_slots, 0);
        assert_eq!(compiled.ret_slots, 1);
    }

    #[test]
    fn test_run_return_42() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_return_42_func();

        jit.compile(0, &func, &module).expect("compilation failed");

        // Get the function pointer
        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        // Prepare context and call
        let mut ret: u64 = 0;
        let ctx = std::ptr::null_mut::<JitContext>();
        let args = std::ptr::null_mut::<u64>();
        let ret_ptr = &mut ret as *mut u64;

        let result = func_ptr(ctx, args, ret_ptr);

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 42);
    }

    fn create_add_func() -> FunctionDef {
        // Function: return 10 + 32
        // slot[0] = 10
        // slot[1] = 32
        // slot[0] = slot[0] + slot[1]
        // return slot[0]
        FunctionDef {
            name: "add".into(),
            param_count: 0,
            param_slots: 0,
            local_slots: 2,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::LoadInt, 0, 10, 0),  // slot[0] = 10
                Instruction::new(Opcode::LoadInt, 1, 32, 0),  // slot[1] = 32
                Instruction::new(Opcode::AddI, 0, 1, 0),      // slot[0] += slot[1]
                Instruction::new(Opcode::Return, 0, 0, 0),    // return slot[0]
            ],
            slot_types: vec![SlotType::Value, SlotType::Value],
        }
    }

    #[test]
    fn test_run_add() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_add_func();

        jit.compile(0, &func, &module).expect("compilation failed");

        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        let mut ret: u64 = 0;
        let result = func_ptr(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut ret as *mut u64,
        );

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 42); // 10 + 32 = 42
    }

    fn create_arithmetic_func() -> FunctionDef {
        // Function: return ((100 - 50) * 2) / 5
        // Expected: (50 * 2) / 5 = 100 / 5 = 20
        FunctionDef {
            name: "arithmetic".into(),
            param_count: 0,
            param_slots: 0,
            local_slots: 2,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::LoadInt, 0, 100, 0), // slot[0] = 100
                Instruction::new(Opcode::LoadInt, 1, 50, 0),  // slot[1] = 50
                Instruction::new(Opcode::SubI, 0, 1, 0),      // slot[0] -= slot[1] -> 50
                Instruction::new(Opcode::LoadInt, 1, 2, 0),   // slot[1] = 2
                Instruction::new(Opcode::MulI, 0, 1, 0),      // slot[0] *= slot[1] -> 100
                Instruction::new(Opcode::LoadInt, 1, 5, 0),   // slot[1] = 5
                Instruction::new(Opcode::DivI, 0, 1, 0),      // slot[0] /= slot[1] -> 20
                Instruction::new(Opcode::Return, 0, 0, 0),    // return slot[0]
            ],
            slot_types: vec![SlotType::Value, SlotType::Value],
        }
    }

    #[test]
    fn test_run_arithmetic() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_arithmetic_func();

        jit.compile(0, &func, &module).expect("compilation failed");

        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        let mut ret: u64 = 0;
        let result = func_ptr(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut ret as *mut u64,
        );

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 20);
    }

    fn create_branch_func() -> FunctionDef {
        // Function: if 10 > 5 { return 1 } else { return 0 }
        // PC 0: slot[0] = 10
        // PC 1: slot[1] = 5
        // PC 2: slot[0] = slot[0] > slot[1]  (GtI)
        // PC 3: jumpif slot[0], PC 5
        // PC 4: slot[0] = 0; return  (else branch)
        // PC 5: slot[0] = 1; return  (then branch)
        FunctionDef {
            name: "branch".into(),
            param_count: 0,
            param_slots: 0,
            local_slots: 2,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::LoadInt, 0, 10, 0),      // PC 0: slot[0] = 10
                Instruction::new(Opcode::LoadInt, 1, 5, 0),       // PC 1: slot[1] = 5
                Instruction::new(Opcode::GtI, 0, 1, 0),           // PC 2: slot[0] = slot[0] > slot[1]
                Instruction::with_flags(Opcode::JumpIf, 0, 0, 5, 0), // PC 3: if slot[0] goto PC 5
                Instruction::new(Opcode::LoadInt, 0, 0, 0),       // PC 4: slot[0] = 0
                Instruction::new(Opcode::Return, 0, 0, 0),        // PC 5: return (else)
                Instruction::new(Opcode::LoadInt, 0, 1, 0),       // PC 6: slot[0] = 1
                Instruction::new(Opcode::Return, 0, 0, 0),        // PC 7: return (then)
            ],
            slot_types: vec![SlotType::Value, SlotType::Value],
        }
    }

    #[test]
    fn test_run_branch() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_branch_func();

        jit.compile(0, &func, &module).expect("compilation failed");

        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        let mut ret: u64 = 0;
        let result = func_ptr(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut ret as *mut u64,
        );

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 1); // 10 > 5 is true, so return 1
    }

    fn create_loop_func() -> FunctionDef {
        // Function: sum 1..5 = 1+2+3+4+5 = 15
        // slot[0] = sum, slot[1] = i
        // sum = 0; i = 5
        // loop: if i == 0 goto end
        //       sum += i
        //       i -= 1
        //       goto loop
        // end: return sum
        FunctionDef {
            name: "loop".into(),
            param_count: 0,
            param_slots: 0,
            local_slots: 3,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::LoadInt, 0, 0, 0),       // PC 0: sum = 0
                Instruction::new(Opcode::LoadInt, 1, 5, 0),       // PC 1: i = 5
                // loop:
                Instruction::new(Opcode::LoadInt, 2, 0, 0),       // PC 2: slot[2] = 0
                Instruction::new(Opcode::EqI, 2, 1, 0),           // PC 3: slot[2] = (i == 0)
                Instruction::with_flags(Opcode::JumpIf, 0, 2, 9, 0), // PC 4: if slot[2] goto PC 9 (end)
                Instruction::new(Opcode::AddI, 0, 1, 0),          // PC 5: sum += i
                Instruction::new(Opcode::LoadInt, 2, 1, 0),       // PC 6: slot[2] = 1
                Instruction::new(Opcode::SubI, 1, 2, 0),          // PC 7: i -= 1
                Instruction::with_flags(Opcode::Jump, 0, 0, 2, 0), // PC 8: goto PC 2 (loop)
                // end:
                Instruction::new(Opcode::Return, 0, 0, 0),        // PC 9: return sum
            ],
            slot_types: vec![SlotType::Value, SlotType::Value, SlotType::Value],
        }
    }

    #[test]
    fn test_run_loop() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_loop_func();

        jit.compile(0, &func, &module).expect("compilation failed");

        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        let mut test_ctx = TestContext::new();
        let mut ctx = test_ctx.as_jit_ctx();
        let mut ret: u64 = 0;
        let result = func_ptr(
            &mut ctx,
            std::ptr::null_mut(),
            &mut ret as *mut u64,
        );

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 15); // 1+2+3+4+5 = 15
    }

    fn create_ptr_read_write_func() -> FunctionDef {
        // Function that takes a pointer as arg, reads slot[0], adds 10, writes back
        // args[0] = pointer to heap object
        // val = ptr[0]
        // val += 10
        // ptr[0] = val
        // return val
        FunctionDef {
            name: "ptr_rw".into(),
            param_count: 1,
            param_slots: 1,
            local_slots: 2,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                // slot[0] = args[0] (pointer), slot[1] = temp
                Instruction::new(Opcode::PtrGet, 1, 0, 0),    // slot[1] = ptr[0]
                Instruction::new(Opcode::LoadInt, 0, 10, 0),  // slot[0] = 10  (reuse slot[0])
                Instruction::new(Opcode::AddI, 1, 0, 0),      // slot[1] += 10
                Instruction::new(Opcode::Copy, 0, 1, 0),      // slot[0] = slot[1] (for PtrSet)
                // Need to reload ptr from args since we overwrote slot[0]
                // Actually let's redesign: use 3 slots
            ],
            slot_types: vec![SlotType::GcRef, SlotType::Value],
        }
    }

    fn create_ptr_read_write_func_v2() -> FunctionDef {
        // slot[0] = ptr (from args)
        // slot[1] = val
        // slot[2] = temp
        // val = ptr[0]
        // val += 10
        // ptr[0] = val
        // return val
        FunctionDef {
            name: "ptr_rw".into(),
            param_count: 1,
            param_slots: 1,  // ptr in slot[0]
            local_slots: 3,
            ret_slots: 1,
            recv_slots: 0,
            code: vec![
                Instruction::new(Opcode::PtrGet, 1, 0, 0),    // slot[1] = ptr[0]
                Instruction::new(Opcode::LoadInt, 2, 10, 0),  // slot[2] = 10
                Instruction::new(Opcode::AddI, 1, 2, 0),      // slot[1] += 10
                Instruction::new(Opcode::PtrSet, 0, 1, 0),    // ptr[0] = slot[1]
                Instruction::new(Opcode::Copy, 0, 1, 0),      // slot[0] = slot[1] for return
                Instruction::new(Opcode::Return, 0, 0, 0),    // return slot[0]
            ],
            slot_types: vec![SlotType::GcRef, SlotType::Value, SlotType::Value],
        }
    }

    #[test]
    fn test_run_ptr_read_write() {
        let mut jit = JitCompiler::new().expect("failed to create JIT");
        let module = create_simple_module();
        let func = create_ptr_read_write_func_v2();

        jit.compile(0, &func, &module).expect("compilation failed");

        let func_ptr = unsafe { jit.get_func_ptr(0) }.expect("function not found");

        // Create a fake heap object (just a u64 on the stack for testing)
        let mut heap_val: u64 = 32;
        let heap_ptr = &mut heap_val as *mut u64;

        // args[0] = pointer to heap
        let mut args: [u64; 1] = [heap_ptr as u64];
        let mut ret: u64 = 0;

        let result = func_ptr(
            std::ptr::null_mut(),
            args.as_mut_ptr(),
            &mut ret as *mut u64,
        );

        assert_eq!(result, JitResult::Ok);
        assert_eq!(ret, 42);      // 32 + 10 = 42
        assert_eq!(heap_val, 42); // heap was modified
    }
}
