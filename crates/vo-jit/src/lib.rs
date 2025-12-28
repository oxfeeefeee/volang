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
        builder.symbol("vo_call_closure", vo_runtime::jit_api::vo_call_closure as *const u8);
        builder.symbol("vo_call_iface", vo_runtime::jit_api::vo_call_iface as *const u8);
        builder.symbol("vo_str_new", vo_runtime::jit_api::vo_str_new as *const u8);
        builder.symbol("vo_str_len", vo_runtime::jit_api::vo_str_len as *const u8);
        builder.symbol("vo_str_index", vo_runtime::jit_api::vo_str_index as *const u8);
        builder.symbol("vo_str_concat", vo_runtime::jit_api::vo_str_concat as *const u8);
        builder.symbol("vo_str_slice", vo_runtime::jit_api::vo_str_slice as *const u8);
        builder.symbol("vo_str_eq", vo_runtime::jit_api::vo_str_eq as *const u8);
        builder.symbol("vo_str_cmp", vo_runtime::jit_api::vo_str_cmp as *const u8);
        builder.symbol("vo_str_decode_rune", vo_runtime::jit_api::vo_str_decode_rune as *const u8);
        builder.symbol("vo_map_new", vo_runtime::jit_api::vo_map_new as *const u8);
        builder.symbol("vo_map_len", vo_runtime::jit_api::vo_map_len as *const u8);
        builder.symbol("vo_map_get", vo_runtime::jit_api::vo_map_get as *const u8);
        builder.symbol("vo_map_set", vo_runtime::jit_api::vo_map_set as *const u8);
        builder.symbol("vo_map_delete", vo_runtime::jit_api::vo_map_delete as *const u8);
        builder.symbol("vo_ptr_clone", vo_runtime::jit_api::vo_ptr_clone as *const u8);

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
        
        // vo_gc_alloc(gc, meta, slots) -> GcRef (u64)
        let gc_alloc_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // gc: *mut Gc
            sig.params.push(AbiParam::new(types::I32)); // meta: u32
            sig.params.push(AbiParam::new(types::I32)); // slots: u32
            sig.returns.push(AbiParam::new(types::I64)); // GcRef
            sig
        };
        let gc_alloc_id = self.module.declare_function(
            "vo_gc_alloc",
            cranelift_module::Linkage::Import,
            &gc_alloc_sig,
        )?;
        
        // vo_call_closure(ctx, closure_ref, args, arg_count, ret, ret_count) -> JitResult
        let call_closure_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // ctx
            sig.params.push(AbiParam::new(types::I64)); // closure_ref
            sig.params.push(AbiParam::new(ptr_type)); // args
            sig.params.push(AbiParam::new(types::I32)); // arg_count
            sig.params.push(AbiParam::new(ptr_type)); // ret
            sig.params.push(AbiParam::new(types::I32)); // ret_count
            sig.returns.push(AbiParam::new(types::I32)); // JitResult
            sig
        };
        let call_closure_id = self.module.declare_function(
            "vo_call_closure",
            cranelift_module::Linkage::Import,
            &call_closure_sig,
        )?;
        
        // vo_call_iface(ctx, slot0, slot1, method_idx, args, arg_count, ret, ret_count, func_id) -> JitResult
        let call_iface_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // ctx
            sig.params.push(AbiParam::new(types::I64)); // iface_slot0
            sig.params.push(AbiParam::new(types::I64)); // iface_slot1
            sig.params.push(AbiParam::new(types::I32)); // method_idx
            sig.params.push(AbiParam::new(ptr_type)); // args
            sig.params.push(AbiParam::new(types::I32)); // arg_count
            sig.params.push(AbiParam::new(ptr_type)); // ret
            sig.params.push(AbiParam::new(types::I32)); // ret_count
            sig.params.push(AbiParam::new(types::I32)); // func_id (pre-resolved)
            sig.returns.push(AbiParam::new(types::I32)); // JitResult
            sig
        };
        let call_iface_id = self.module.declare_function(
            "vo_call_iface",
            cranelift_module::Linkage::Import,
            &call_iface_sig,
        )?;
        
        // Declare string helper functions
        let str_new_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // gc
            sig.params.push(AbiParam::new(ptr_type)); // data
            sig.params.push(AbiParam::new(types::I64)); // len
            sig.returns.push(AbiParam::new(types::I64)); // GcRef
            sig
        };
        let str_new_id = self.module.declare_function("vo_str_new", cranelift_module::Linkage::Import, &str_new_sig)?;
        
        let str_len_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(types::I64)); // s
            sig.returns.push(AbiParam::new(types::I64)); // len
            sig
        };
        let str_len_id = self.module.declare_function("vo_str_len", cranelift_module::Linkage::Import, &str_len_sig)?;
        
        let str_index_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(types::I64)); // s
            sig.params.push(AbiParam::new(types::I64)); // idx
            sig.returns.push(AbiParam::new(types::I64)); // byte
            sig
        };
        let str_index_id = self.module.declare_function("vo_str_index", cranelift_module::Linkage::Import, &str_index_sig)?;
        
        let str_concat_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // gc
            sig.params.push(AbiParam::new(types::I64)); // a
            sig.params.push(AbiParam::new(types::I64)); // b
            sig.returns.push(AbiParam::new(types::I64)); // result
            sig
        };
        let str_concat_id = self.module.declare_function("vo_str_concat", cranelift_module::Linkage::Import, &str_concat_sig)?;
        
        let str_slice_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type)); // gc
            sig.params.push(AbiParam::new(types::I64)); // s
            sig.params.push(AbiParam::new(types::I64)); // lo
            sig.params.push(AbiParam::new(types::I64)); // hi
            sig.returns.push(AbiParam::new(types::I64)); // result
            sig
        };
        let str_slice_id = self.module.declare_function("vo_str_slice", cranelift_module::Linkage::Import, &str_slice_sig)?;
        
        let str_eq_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(types::I64)); // a
            sig.params.push(AbiParam::new(types::I64)); // b
            sig.returns.push(AbiParam::new(types::I64)); // result
            sig
        };
        let str_eq_id = self.module.declare_function("vo_str_eq", cranelift_module::Linkage::Import, &str_eq_sig)?;
        
        let str_cmp_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(types::I64)); // a
            sig.params.push(AbiParam::new(types::I64)); // b
            sig.returns.push(AbiParam::new(types::I32)); // result
            sig
        };
        let str_cmp_id = self.module.declare_function("vo_str_cmp", cranelift_module::Linkage::Import, &str_cmp_sig)?;
        
        let str_decode_rune_sig = {
            let mut sig = Signature::new(self.module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(types::I64)); // s
            sig.params.push(AbiParam::new(types::I64)); // pos
            sig.returns.push(AbiParam::new(types::I64)); // (rune << 32) | width
            sig
        };
        let str_decode_rune_id = self.module.declare_function("vo_str_decode_rune", cranelift_module::Linkage::Import, &str_decode_rune_sig)?;
        
        // 5. Build function IR using FunctionCompiler
        let mut func_ctx = FunctionBuilderContext::new();
        {
            let safepoint_func = self.module.declare_func_in_func(safepoint_id, &mut self.ctx.func);
            let call_vm_func = self.module.declare_func_in_func(call_vm_id, &mut self.ctx.func);
            let gc_alloc_func = self.module.declare_func_in_func(gc_alloc_id, &mut self.ctx.func);
            let call_closure_func = self.module.declare_func_in_func(call_closure_id, &mut self.ctx.func);
            let call_iface_func = self.module.declare_func_in_func(call_iface_id, &mut self.ctx.func);
            
            let str_funcs = crate::compiler::StringFuncs {
                str_new: Some(self.module.declare_func_in_func(str_new_id, &mut self.ctx.func)),
                str_len: Some(self.module.declare_func_in_func(str_len_id, &mut self.ctx.func)),
                str_index: Some(self.module.declare_func_in_func(str_index_id, &mut self.ctx.func)),
                str_concat: Some(self.module.declare_func_in_func(str_concat_id, &mut self.ctx.func)),
                str_slice: Some(self.module.declare_func_in_func(str_slice_id, &mut self.ctx.func)),
                str_eq: Some(self.module.declare_func_in_func(str_eq_id, &mut self.ctx.func)),
                str_cmp: Some(self.module.declare_func_in_func(str_cmp_id, &mut self.ctx.func)),
                str_decode_rune: Some(self.module.declare_func_in_func(str_decode_rune_id, &mut self.ctx.func)),
            };
            
            let map_funcs = crate::compiler::MapFuncs::default();
            let misc_funcs = crate::compiler::MiscFuncs::default();
            
            let compiler = FunctionCompiler::new(
                &mut self.ctx.func,
                &mut func_ctx,
                func,
                vo_module,
                Some(safepoint_func),
                Some(call_vm_func),
                Some(gc_alloc_func),
                Some(call_closure_func),
                Some(call_iface_func),
                str_funcs,
                map_funcs,
                misc_funcs,
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
