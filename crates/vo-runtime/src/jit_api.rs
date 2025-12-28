//! JIT runtime API.
//!
//! This module defines the C ABI interface between JIT-compiled code and the
//! Vo runtime. All functions here are `extern "C"` and can be called from
//! JIT-generated native code.
//!
//! # Architecture
//!
//! ```text
//! JIT Code                    Runtime
//! --------                    -------
//!    |                           |
//!    |-- vo_gc_alloc() --------->|  Allocate GC object
//!    |-- vo_gc_write_barrier() ->|  Write barrier for GC
//!    |-- vo_gc_safepoint() ----->|  GC safepoint check
//!    |-- vo_call_vm() ---------->|  Call VM-interpreted function
//!    |-- vo_panic() ------------>|  Trigger panic
//!    |                           |
//! ```

use std::ffi::c_void;

use crate::gc::Gc;

// =============================================================================
// JitContext
// =============================================================================

/// Runtime context passed to JIT functions.
///
/// This struct is passed as the first argument to all JIT functions.
/// It provides access to runtime resources needed during execution.
///
/// # Memory Layout
///
/// This struct uses `#[repr(C)]` to ensure predictable field layout for
/// access from JIT-generated code.
/// Function pointer type for VM call trampoline.
/// This allows vo_call_vm to call back into the VM without circular dependency.
pub type VmCallFn = extern "C" fn(
    vm: *mut c_void,
    fiber: *mut c_void,
    func_id: u32,
    args: *const u64,
    arg_count: u32,
    ret: *mut u64,
    ret_count: u32,
) -> JitResult;

#[repr(C)]
pub struct JitContext {
    /// Pointer to the GC instance.
    pub gc: *mut Gc,
    
    /// Pointer to the global variables array.
    pub globals: *mut u64,
    
    /// Pointer to safepoint flag (read by JIT to check if GC wants to run).
    /// When this is true, JIT should call vo_gc_safepoint().
    pub safepoint_flag: *const bool,
    
    /// Pointer to panic flag (set by JIT when panic occurs).
    pub panic_flag: *mut bool,
    
    /// Opaque pointer to VM instance.
    /// Used by vo_call_vm to execute VM functions.
    /// Cast to `*mut Vm` in trampoline code.
    pub vm: *mut c_void,
    
    /// Opaque pointer to current Fiber.
    /// Used by vo_call_vm for stack management.
    /// Cast to `*mut Fiber` in trampoline code.
    pub fiber: *mut c_void,
    
    /// Callback to execute a function in VM.
    /// Set by VM when creating JitContext.
    pub call_vm_fn: Option<VmCallFn>,
}

// =============================================================================
// JitResult
// =============================================================================

/// Result of JIT function execution.
///
/// JIT functions return this to indicate success or failure.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitResult {
    /// Function completed successfully.
    Ok = 0,
    /// Function panicked.
    Panic = 1,
}

// =============================================================================
// Runtime Helper Functions
// =============================================================================

// NOTE: These functions are declared but not yet implemented.
// They will be implemented when we integrate JIT with the VM.
//
// The JIT compiler registers these symbols with Cranelift's JITBuilder,
// so JIT-generated code can call them directly.

/// Allocate a new GC object.
///
/// # Arguments
/// - `gc`: Pointer to GC instance
/// - `meta`: ValueMeta for the object (packed meta_id + value_kind)
/// - `slots`: Number of 64-bit slots to allocate
///
/// # Returns
/// GcRef (pointer to allocated object data)
///
/// # Safety
/// - `gc` must be a valid pointer to a Gc instance
#[no_mangle]
pub extern "C" fn vo_gc_alloc(_gc: *mut Gc, _meta: u32, _slots: u32) -> u64 {
    // TODO: Implement
    // unsafe {
    //     let gc = &mut *gc;
    //     let value_meta = ValueMeta::from_raw(meta);
    //     gc.alloc(value_meta, slots as u16) as u64
    // }
    todo!("vo_gc_alloc not yet implemented")
}

/// Write barrier for GC.
///
/// Called when storing a GcRef into a heap object. This is the slow path;
/// JIT generates inline fast path that checks `is_marking` first.
///
/// # Arguments
/// - `gc`: Pointer to GC instance
/// - `obj`: The object being written to (GcRef)
/// - `offset`: Slot offset within the object
/// - `val`: The value being stored (may be GcRef)
///
/// # Safety
/// - `gc` must be a valid pointer to a Gc instance
/// - `obj` must be a valid GcRef
#[no_mangle]
pub extern "C" fn vo_gc_write_barrier(_gc: *mut Gc, _obj: u64, _offset: u32, _val: u64) {
    // TODO: Implement
    // This is called when is_marking is true.
    // Need to mark the old value gray to preserve tri-color invariant.
}

/// GC safepoint.
///
/// Called at loop back-edges and before function calls when safepoint_flag
/// is set. May trigger garbage collection.
///
/// # Arguments
/// - `ctx`: JIT context
///
/// # Safety
/// - `ctx` must be a valid pointer to JitContext
#[no_mangle]
pub extern "C" fn vo_gc_safepoint(_ctx: *mut JitContext) {
    // TODO: Implement
    // 1. Check if GC wants to run
    // 2. If so, scan JIT stack frames using stack maps
    // 3. Run GC
    // 4. Clear safepoint_flag
}

/// Call a VM-interpreted function from JIT code.
///
/// This is the trampoline from JIT to VM. All function calls from JIT
/// go through this (for simplicity - we don't inline JIT->JIT calls yet).
///
/// # Arguments
/// - `ctx`: JIT context
/// - `func_id`: Function ID to call
/// - `args`: Pointer to argument slots
/// - `arg_count`: Number of argument slots
/// - `ret`: Pointer to return value slots
/// - `ret_count`: Number of return value slots
///
/// # Returns
/// - `JitResult::Ok` if function completed normally
/// - `JitResult::Panic` if function panicked
///
/// # Safety
/// - `ctx` must be a valid pointer to JitContext
/// - `args` must point to at least `arg_count` u64 values
/// - `ret` must point to space for at least `ret_count` u64 values
#[no_mangle]
pub extern "C" fn vo_call_vm(
    ctx: *mut JitContext,
    func_id: u32,
    args: *const u64,
    arg_count: u32,
    ret: *mut u64,
    ret_count: u32,
) -> JitResult {
    // Safety: ctx must be valid
    let ctx = unsafe { &*ctx };
    
    // Get the VM call callback
    let call_fn = match ctx.call_vm_fn {
        Some(f) => f,
        None => return JitResult::Panic, // No callback registered
    };
    
    // Call back into VM
    call_fn(ctx.vm, ctx.fiber, func_id, args, arg_count, ret, ret_count)
}

/// Trigger a panic.
///
/// # Arguments
/// - `ctx`: JIT context
/// - `msg`: Panic message (GcRef to string, or 0 for no message)
///
/// # Safety
/// - `ctx` must be a valid pointer to JitContext
#[no_mangle]
pub extern "C" fn vo_panic(_ctx: *mut JitContext, _msg: u64) {
    // TODO: Implement
    // 1. Set panic_flag to true
    // 2. Store panic message in fiber
}

// =============================================================================
// Map/String Iteration Helpers
// =============================================================================

/// Get next key-value pair from map iteration.
///
/// # Arguments
/// - `map`: Map GcRef
/// - `cursor`: Pointer to cursor (updated by this function)
/// - `key`: Pointer to store key
/// - `val`: Pointer to store value
///
/// # Returns
/// - `true` if there was a next element
/// - `false` if iteration is complete
///
/// # Safety
/// - All pointers must be valid
#[no_mangle]
pub extern "C" fn vo_map_iter_next(
    _map: u64,
    _cursor: *mut u64,
    _key: *mut u64,
    _val: *mut u64,
) -> bool {
    // TODO: Implement
    todo!("vo_map_iter_next not yet implemented")
}

/// Decode a UTF-8 rune from a string.
///
/// # Arguments
/// - `s`: String GcRef
/// - `pos`: Byte position in string
///
/// # Returns
/// Packed value: `(rune << 32) | width`
/// - `rune`: The decoded Unicode code point (or replacement char on error)
/// - `width`: Number of bytes consumed (1-4)
///
/// # Safety
/// - `s` must be a valid string GcRef
/// - `pos` must be within string bounds
#[no_mangle]
pub extern "C" fn vo_str_decode_rune(_s: u64, _pos: u64) -> u64 {
    // TODO: Implement
    todo!("vo_str_decode_rune not yet implemented")
}

// =============================================================================
// Symbol Registration
// =============================================================================

/// Get all runtime symbols for JIT registration.
///
/// Returns a slice of (name, function_pointer) pairs that should be
/// registered with Cranelift's JITBuilder.
pub fn get_runtime_symbols() -> &'static [(&'static str, *const u8)] {
    &[
        ("vo_gc_alloc", vo_gc_alloc as *const u8),
        ("vo_gc_write_barrier", vo_gc_write_barrier as *const u8),
        ("vo_gc_safepoint", vo_gc_safepoint as *const u8),
        ("vo_call_vm", vo_call_vm as *const u8),
        ("vo_panic", vo_panic as *const u8),
        ("vo_map_iter_next", vo_map_iter_next as *const u8),
        ("vo_str_decode_rune", vo_str_decode_rune as *const u8),
    ]
}
