//! GC type information and unified object scanning.
//!
//! This module provides:
//! - Static struct slot_types table (initialized once at module load)
//! - Unified scan_object function shared by VM and JIT

use alloc::boxed::Box;
use alloc::vec::Vec;
use once_cell::sync::OnceCell;
use gox_common_core::{ValueKind, SlotType, FIRST_USER_TYPE_ID, type_needs_gc};

use crate::gc::{Gc, GcRef};

#[cfg(feature = "std")]
use crate::objects::map;

// =============================================================================
// Static Struct SlotType Table
// =============================================================================

/// Struct slot_types, indexed by (type_id - FIRST_USER_TYPE_ID).
/// Each entry describes how GC should scan each slot.
static STRUCT_SLOT_TYPES: OnceCell<Box<[Box<[SlotType]>]>> = OnceCell::new();

/// Initialize the struct slot_types table.
/// Called once at module load time by VM or JIT.
pub fn init_struct_slot_types(slot_types: Vec<Vec<SlotType>>) {
    let boxed: Box<[Box<[SlotType]>]> = slot_types
        .into_iter()
        .map(|v| v.into_boxed_slice())
        .collect();
    let _ = STRUCT_SLOT_TYPES.set(boxed);
}

/// Get slot_types for a user-defined struct type.
/// Returns None if type_id < FIRST_USER_TYPE_ID or not registered.
#[inline]
pub fn get_struct_slot_types(type_id: u32) -> Option<&'static [SlotType]> {
    if type_id < FIRST_USER_TYPE_ID {
        return None;
    }
    let idx = (type_id - FIRST_USER_TYPE_ID) as usize;
    STRUCT_SLOT_TYPES.get()?.get(idx).map(|b| b.as_ref())
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
    
    // User-defined struct: use slot_types for dynamic scanning
    if type_id >= FIRST_USER_TYPE_ID {
        if let Some(slot_types) = get_struct_slot_types(type_id) {
            scan_with_slot_types(gc, obj, slot_types, 0);
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
        // Struct elements: use slot_types for each element
        if let Some(slot_types) = get_struct_slot_types(elem_type) {
            let slots_per_elem = slot_types.len().max(1);
            for i in 0..len {
                let base = 3 + i * slots_per_elem;
                scan_with_slot_types(gc, obj, slot_types, base);
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

/// Scan slots using slot_types for dynamic interface handling.
/// This is the core function that handles Interface1 slots dynamically.
fn scan_with_slot_types(gc: &mut Gc, obj: GcRef, slot_types: &[SlotType], base_offset: usize) {
    let mut i = 0;
    while i < slot_types.len() {
        match slot_types[i] {
            SlotType::Value => {
                // Non-pointer, skip
            }
            SlotType::GcRef => {
                let val = Gc::read_slot(obj, base_offset + i);
                if val != 0 {
                    gc.mark_gray(val as GcRef);
                }
            }
            SlotType::Interface0 => {
                // Interface first slot is type_id, not a pointer
                // Next slot (Interface1) needs dynamic check
            }
            SlotType::Interface1 => {
                // Dynamic check: look at the type_id in previous slot
                if i > 0 {
                    let type_id = Gc::read_slot(obj, base_offset + i - 1) as u32;
                    // If type_id indicates a reference type, scan the data slot
                    if type_needs_gc(type_id) {
                        let val = Gc::read_slot(obj, base_offset + i);
                        if val != 0 {
                            gc.mark_gray(val as GcRef);
                        }
                    }
                }
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_init_and_get_slot_types() {
        // Note: OnceCell can only be set once, so this test may conflict with others
        let slot_types = vec![
            vec![SlotType::GcRef, SlotType::Value, SlotType::GcRef],  // type_id 32
            vec![SlotType::Value, SlotType::GcRef],                   // type_id 33
        ];
        init_struct_slot_types(slot_types);
        
        // Should return None for built-in types
        assert!(get_struct_slot_types(0).is_none());
        assert!(get_struct_slot_types(14).is_none());
        assert!(get_struct_slot_types(31).is_none());
        
        // Should return slot_types for user types
        assert_eq!(get_struct_slot_types(32), Some(&[SlotType::GcRef, SlotType::Value, SlotType::GcRef][..]));
        assert_eq!(get_struct_slot_types(33), Some(&[SlotType::Value, SlotType::GcRef][..]));
        assert!(get_struct_slot_types(34).is_none());
    }
}
