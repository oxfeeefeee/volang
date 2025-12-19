//! GC type information and unified object scanning.
//!
//! This module provides:
//! - Static struct ptr_bitmap table (initialized once at module load)
//! - Unified scan_object function shared by VM and JIT

use alloc::boxed::Box;
use alloc::vec::Vec;
use once_cell::sync::OnceCell;
use gox_common_core::{ValueKind, FIRST_USER_TYPE_ID, type_needs_gc};

use crate::gc::{Gc, GcRef};

#[cfg(feature = "std")]
use crate::objects::map;

// =============================================================================
// Static Struct Bitmap Table
// =============================================================================

/// Struct ptr_bitmaps, indexed by (type_id - FIRST_USER_TYPE_ID).
/// Each bitmap indicates which slots contain GC references.
static STRUCT_BITMAPS: OnceCell<Box<[Box<[bool]>]>> = OnceCell::new();

/// Initialize the struct bitmap table.
/// Called once at module load time by VM or JIT.
pub fn init_struct_bitmaps(bitmaps: Vec<Vec<bool>>) {
    let boxed: Box<[Box<[bool]>]> = bitmaps
        .into_iter()
        .map(|v| v.into_boxed_slice())
        .collect();
    let _ = STRUCT_BITMAPS.set(boxed);
}

/// Get ptr_bitmap for a user-defined struct type.
/// Returns None if type_id < FIRST_USER_TYPE_ID or not registered.
#[inline]
pub fn get_struct_bitmap(type_id: u32) -> Option<&'static [bool]> {
    if type_id < FIRST_USER_TYPE_ID {
        return None;
    }
    let idx = (type_id - FIRST_USER_TYPE_ID) as usize;
    STRUCT_BITMAPS.get()?.get(idx).map(|b| b.as_ref())
}

// =============================================================================
// Unified Object Scanning
// =============================================================================

/// Scan a GC object for internal references.
/// This is the unified scan function used by both VM and JIT.
pub fn scan_object(gc: &mut Gc, obj: GcRef) {
    if obj.is_null() {
        return;
    }
    
    let type_id = unsafe { (*obj).header.type_id };
    
    // User-defined struct: use ptr_bitmap
    if type_id >= FIRST_USER_TYPE_ID {
        if let Some(bitmap) = get_struct_bitmap(type_id) {
            for (i, &is_ptr) in bitmap.iter().enumerate() {
                if is_ptr {
                    let val = Gc::read_slot(obj, i);
                    if val != 0 {
                        gc.mark_gray(val as GcRef);
                    }
                }
            }
        }
        return;
    }
    
    // Built-in types: fixed layouts
    let kind = ValueKind::from_u8(type_id as u8);
    match kind {
        ValueKind::String | ValueKind::Slice => {
            // String: [array_ref, start, len]
            // Slice: [array_ref, start, len, cap]
            let val = Gc::read_slot(obj, 0);
            if val != 0 {
                gc.mark_gray(val as GcRef);
            }
        }
        ValueKind::Array => scan_array(gc, obj),
        #[cfg(feature = "std")]
        ValueKind::Map => scan_map(gc, obj),
        ValueKind::Channel => scan_channel(gc, obj),
        ValueKind::Closure => scan_closure(gc, obj),
        _ => {}
    }
}

/// Scan array elements.
fn scan_array(gc: &mut Gc, obj: GcRef) {
    // Array: [elem_type, elem_bytes, len, data...]
    let elem_type = Gc::read_slot(obj, 0) as u32;
    let len = Gc::read_slot(obj, 2) as usize;
    
    if !type_needs_gc(elem_type) {
        return;
    }
    
    if elem_type >= FIRST_USER_TYPE_ID {
        // Struct elements: use ptr_bitmap for each element
        if let Some(bitmap) = get_struct_bitmap(elem_type) {
            let slots_per_elem = bitmap.len().max(1);
            for i in 0..len {
                let base = 3 + i * slots_per_elem;
                for (j, &is_ptr) in bitmap.iter().enumerate() {
                    if is_ptr {
                        let val = Gc::read_slot(obj, base + j);
                        if val != 0 {
                            gc.mark_gray(val as GcRef);
                        }
                    }
                }
            }
        }
    } else {
        // Built-in reference elements: each element is a GcRef
        for i in 0..len {
            let val = Gc::read_slot(obj, 3 + i);
            if val != 0 {
                gc.mark_gray(val as GcRef);
            }
        }
    }
}

/// Scan map entries.
/// Note: Map keys must be comparable types (no slices, maps, funcs), so no GC refs.
#[cfg(feature = "std")]
fn scan_map(gc: &mut Gc, obj: GcRef) {
    // Map: [map_ptr, key_type, val_type]
    let val_type = Gc::read_slot(obj, 2) as u32;
    
    if !type_needs_gc(val_type) {
        return;
    }
    
    // Only scan values (keys are comparable types, no GC refs)
    let len = map::len(obj);
    for idx in 0..len {
        if let Some((_, val)) = map::iter_at(obj, idx) {
            if val != 0 {
                gc.mark_gray(val as GcRef);
            }
        }
    }
}

/// Scan channel buffer and waiting senders.
#[cfg(feature = "std")]
fn scan_channel(gc: &mut Gc, obj: GcRef) {
    use crate::objects::channel;
    
    // Channel: [chan_ptr, elem_type, cap]
    let elem_type = Gc::read_slot(obj, 1) as u32;
    
    if !type_needs_gc(elem_type) {
        return;
    }
    
    let state = channel::get_state(obj);
    
    // Scan buffer
    for &val in &state.buffer {
        if val != 0 {
            gc.mark_gray(val as GcRef);
        }
    }
    
    // Scan waiting_senders values
    for &(_, val) in &state.waiting_senders {
        if val != 0 {
            gc.mark_gray(val as GcRef);
        }
    }
}

#[cfg(not(feature = "std"))]
fn scan_channel(_gc: &mut Gc, _obj: GcRef) {
    // Channel not supported in no_std
}

/// Scan closure upvalues.
fn scan_closure(gc: &mut Gc, obj: GcRef) {
    // Closure: [func_id, count, upval0, upval1, ...]
    let count = Gc::read_slot(obj, 1) as usize;
    for i in 0..count.min(256) {
        let val = Gc::read_slot(obj, 2 + i);
        if val != 0 {
            gc.mark_gray(val as GcRef);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_init_and_get_bitmap() {
        // Note: OnceCell can only be set once, so this test may conflict with others
        let bitmaps = vec![
            vec![true, false, true],  // type_id 32
            vec![false, true],        // type_id 33
        ];
        init_struct_bitmaps(bitmaps);
        
        // Should return None for built-in types
        assert!(get_struct_bitmap(0).is_none());
        assert!(get_struct_bitmap(14).is_none());
        assert!(get_struct_bitmap(31).is_none());
        
        // Should return bitmap for user types
        assert_eq!(get_struct_bitmap(32), Some(&[true, false, true][..]));
        assert_eq!(get_struct_bitmap(33), Some(&[false, true][..]));
        assert!(get_struct_bitmap(34).is_none());
    }
}
