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
// Stdlib: Builtin C ABI (2 functions)
// =============================================================================
#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_builtin_len(type_tag: u8, ptr: GcRef) -> usize {
    crate::stdlib::builtin::len_impl(type_tag, ptr)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_builtin_cap(type_tag: u8, ptr: GcRef) -> usize {
    crate::stdlib::builtin::cap_impl(type_tag, ptr)
}

// =============================================================================
// Stdlib: Strings C ABI (14 functions)
// =============================================================================
#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_contains(s: GcRef, substr: GcRef) -> bool {
    use crate::objects::string;
    crate::stdlib::strings::contains(string::as_str(s), string::as_str(substr))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_has_prefix(s: GcRef, prefix: GcRef) -> bool {
    use crate::objects::string;
    crate::stdlib::strings::has_prefix(string::as_str(s), string::as_str(prefix))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_has_suffix(s: GcRef, suffix: GcRef) -> bool {
    use crate::objects::string;
    crate::stdlib::strings::has_suffix(string::as_str(s), string::as_str(suffix))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_index(s: GcRef, substr: GcRef) -> i64 {
    use crate::objects::string;
    crate::stdlib::strings::index(string::as_str(s), string::as_str(substr))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_count(s: GcRef, substr: GcRef) -> usize {
    use crate::objects::string;
    crate::stdlib::strings::count(string::as_str(s), string::as_str(substr))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_to_lower(gc: *mut Gc, s: GcRef, type_id: TypeId) -> GcRef {
    use crate::objects::string;
    let result = crate::stdlib::strings::to_lower(string::as_str(s));
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_to_upper(gc: *mut Gc, s: GcRef, type_id: TypeId) -> GcRef {
    use crate::objects::string;
    let result = crate::stdlib::strings::to_upper(string::as_str(s));
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_replace_all(
    gc: *mut Gc, s: GcRef, old: GcRef, new: GcRef, type_id: TypeId
) -> GcRef {
    use crate::objects::string;
    let result = crate::stdlib::strings::replace_all(
        string::as_str(s), string::as_str(old), string::as_str(new)
    );
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_repeat(
    gc: *mut Gc, s: GcRef, n: usize, type_id: TypeId
) -> GcRef {
    use crate::objects::string;
    let result = crate::stdlib::strings::repeat(string::as_str(s), n);
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_compare(s: GcRef, t: GcRef) -> i64 {
    use crate::objects::string;
    crate::stdlib::strings::compare(string::as_str(s), string::as_str(t))
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_strings_equal_fold(s: GcRef, t: GcRef) -> bool {
    use crate::objects::string;
    crate::stdlib::strings::equal_fold(string::as_str(s), string::as_str(t))
}

// =============================================================================
// Stdlib: Fmt C ABI (2 functions)
// =============================================================================
#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_fmt_format_value(
    gc: *mut Gc, val: u64, type_tag: u8, type_id: TypeId
) -> GcRef {
    use crate::objects::string;
    let result = crate::stdlib::fmt::format_value(val, type_tag);
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn gox_fmt_println(args: *const u64, tags: *const u8, argc: usize) -> usize {
    use alloc::vec::Vec;
    let args_slice = core::slice::from_raw_parts(args, argc);
    let tags_slice = core::slice::from_raw_parts(tags, argc);
    let pairs: Vec<(u64, u8)> = args_slice.iter().zip(tags_slice.iter())
        .map(|(&v, &t)| (v, t)).collect();
    let output = crate::stdlib::fmt::format_args(&pairs);
    #[cfg(feature = "std")]
    println!("{}", output);
    output.len() + 1
}

// =============================================================================
// Summary: Total 57 C ABI functions
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
// Builtin:   2 (len, cap) [std only]
// Strings:  11 (contains, has_prefix, has_suffix, index, count, to_lower, 
//              to_upper, replace_all, repeat, compare, equal_fold) [std only]
// Fmt:       2 (format_value, println) [std only]
//
