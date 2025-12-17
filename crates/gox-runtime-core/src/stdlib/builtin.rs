//! Built-in function core implementations.
//!
//! These are the pure logic implementations for len, cap, etc.
//! Execution engines provide their own bindings to call these.

use crate::gc::GcRef;
use crate::objects::{string, array, slice};

#[cfg(feature = "std")]
use crate::objects::map;

/// Get length of a value based on its type tag.
/// 
/// # Arguments
/// * `type_tag` - Type identifier (from ValueKind)
/// * `ptr` - GcRef to the object
/// 
/// # Returns
/// Length as usize, or 0 for null/unsupported types
pub fn len_impl(type_tag: u8, ptr: GcRef) -> usize {
    use gox_common_core::ValueKind;
    
    if ptr.is_null() {
        return 0;
    }
    
    match ValueKind::from_u8(type_tag) {
        ValueKind::String => string::len(ptr),
        ValueKind::Array => array::len(ptr),
        ValueKind::Slice => slice::len(ptr),
        #[cfg(feature = "std")]
        ValueKind::Map => map::len(ptr),
        _ => 0,
    }
}

/// Get capacity of a value based on its type tag.
/// 
/// # Arguments
/// * `type_tag` - Type identifier (from ValueKind)
/// * `ptr` - GcRef to the object
/// 
/// # Returns
/// Capacity as usize, or 0 for null/unsupported types
pub fn cap_impl(type_tag: u8, ptr: GcRef) -> usize {
    use gox_common_core::ValueKind;
    
    if ptr.is_null() {
        return 0;
    }
    
    match ValueKind::from_u8(type_tag) {
        ValueKind::Array => array::len(ptr), // array cap = len
        ValueKind::Slice => slice::cap(ptr),
        _ => 0,
    }
}
