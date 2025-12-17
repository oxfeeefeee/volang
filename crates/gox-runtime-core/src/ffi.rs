//! FFI exports for Cranelift and external code.
//!
//! This module re-exports all `extern "C"` functions that can be called
//! by Cranelift-generated code or external native code.
//!
//! # Usage from Cranelift
//!
//! ```ignore
//! // Import runtime function
//! let func_ref = module.declare_function("gox_gc_alloc", ...);
//! // Call it
//! builder.ins().call(func_ref, &[gc_ptr, type_id, slots]);
//! ```

// Re-export core types
pub use crate::gc::{Gc, GcRef, GcHeader, TypeId, NULL_REF};

// =============================================================================
// GC C ABI (5 functions)
// =============================================================================
pub use crate::gc::{
    gox_gc_alloc,
    gox_gc_read_slot,
    gox_gc_write_slot,
    gox_gc_write_barrier,
    gox_gc_mark_gray,
};

// =============================================================================
// String C ABI (5 functions)
// =============================================================================
pub use crate::objects::{
    gox_string_len,
    gox_string_index,
    gox_string_concat,
    gox_string_eq,
    gox_string_ne,
};

// =============================================================================
// Array C ABI (4 functions)
// =============================================================================
pub use crate::objects::{
    gox_array_create,
    gox_array_len,
    gox_array_get,
    gox_array_set,
};

// =============================================================================
// Slice C ABI (7 functions)
// =============================================================================
pub use crate::objects::{
    gox_slice_create,
    gox_slice_len,
    gox_slice_cap,
    gox_slice_get,
    gox_slice_set,
    gox_slice_append,
    gox_slice_slice,
};

// =============================================================================
// Struct Hash C ABI (1 function)
// =============================================================================
pub use crate::objects::gox_struct_hash;

// =============================================================================
// Map C ABI (6 functions, requires std feature)
// =============================================================================
#[cfg(feature = "std")]
pub use crate::objects::{
    gox_map_create,
    gox_map_len,
    gox_map_get,
    gox_map_set,
    gox_map_delete,
    gox_map_contains,
};

// =============================================================================
// Closure C ABI (8 functions)
// =============================================================================
pub use crate::objects::{
    gox_closure_create,
    gox_closure_func_id,
    gox_closure_upvalue_count,
    gox_closure_get_upvalue,
    gox_closure_set_upvalue,
    gox_upval_box_create,
    gox_upval_box_get,
    gox_upval_box_set,
};

// =============================================================================
// Interface C ABI (3 functions)
// =============================================================================
pub use crate::objects::{
    gox_interface_unbox_type,
    gox_interface_unbox_data,
    gox_interface_is_nil,
};

// =============================================================================
// Summary: Total 39 C ABI functions
// =============================================================================
//
// GC:        5 (alloc, read_slot, write_slot, write_barrier, mark_gray)
// String:    5 (len, index, concat, eq, ne)
// Array:     4 (create, len, get, set)
// Slice:     7 (create, len, cap, get, set, append, slice)
// Struct:    1 (hash)
// Map:       6 (create, len, get, set, delete, contains) [std only]
// Closure:   8 (create, func_id, upvalue_count, get/set_upvalue, upval_box_*)
// Interface: 3 (unbox_type, unbox_data, is_nil)
//
