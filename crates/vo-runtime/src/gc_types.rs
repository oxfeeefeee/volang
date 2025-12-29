//! GC object scanning by type.

use crate::gc::{Gc, GcRef};
use crate::objects::{array, channel, closure, interface, map, slice, string};
use vo_common_core::bytecode::StructMeta;
use vo_common_core::types::{SlotType, ValueKind};


/// Scan a GC object and mark its children.
pub fn scan_object(gc: &mut Gc, obj: GcRef, struct_metas: &[StructMeta]) {
    let gc_header = Gc::header(obj);
    
    match gc_header.kind() {
        ValueKind::Array => scan_array(gc, obj),
        ValueKind::String => {
            let arr = string::array_ref(obj);
            if !arr.is_null() { gc.mark_gray(arr); }
        }

        ValueKind::Slice => {
            let arr = slice::array_ref(obj);
            if !arr.is_null() { gc.mark_gray(arr); }
        }

        ValueKind::Struct | ValueKind::Pointer => {
            scan_struct(gc, obj, gc_header.meta_id() as usize, struct_metas);
        }

        ValueKind::Closure => {
            for i in 0..closure::capture_count(obj) {
                let cap = closure::get_capture(obj, i);
                if cap != 0 { gc.mark_gray(cap as GcRef); }
            }
        }

        ValueKind::Map => {
            let scan_key = map::key_kind(obj).may_contain_gc_refs();
            let scan_val = map::val_kind(obj).may_contain_gc_refs();
            if scan_key || scan_val {
                for i in 0..map::len(obj) {
                    if let Some((k, v)) = map::iter_at(obj, i) {
                        if scan_key {
                            for &slot in k {
                                if slot != 0 { gc.mark_gray(slot as GcRef); }
                            }
                        }
                        if scan_val {
                            for &slot in v {
                                if slot != 0 { gc.mark_gray(slot as GcRef); }
                            }
                        }
                    }
                }
            }
        }

        ValueKind::Channel => {
            if channel::elem_kind(obj).may_contain_gc_refs() {
                let state = channel::get_state(obj);
                for elem in state.iter_buffer() {
                    for &slot in elem {
                        if slot != 0 { gc.mark_gray(slot as GcRef); }
                    }
                }
                for elem in state.iter_waiting_values() {
                    for &slot in elem {
                        if slot != 0 { gc.mark_gray(slot as GcRef); }
                    }
                }
            }
        }

        _ => {}
    }
}

fn scan_array(gc: &mut Gc, obj: GcRef) {
    let elem_kind = array::elem_kind(obj);
    // Packed types (bool, int8-32, float32) don't contain GcRefs
    if !elem_kind.may_contain_gc_refs() { return; }
    
    // For types that may contain GcRefs, elem_bytes is always multiple of 8
    let len = array::len(obj);
    let elem_bytes = array::elem_bytes(obj);
    let elem_slots = elem_bytes / 8;
    
    for idx in 0..len {
        for slot in 0..elem_slots {
            // Read slot at byte offset: idx * elem_bytes + slot * 8
            let byte_off = idx * elem_bytes + slot * 8;
            let ptr = unsafe { (obj as *const u8).add(array::HEADER_SLOTS * 8 + byte_off) as *const u64 };
            let child = unsafe { *ptr };
            if child != 0 { gc.mark_gray(child as GcRef); }
        }
    }
}

fn scan_struct(gc: &mut Gc, obj: GcRef, meta_id: usize, struct_metas: &[StructMeta]) {
    if meta_id >= struct_metas.len() { return; }
    
    let meta = &struct_metas[meta_id];
    let mut i = 0;
    while i < meta.slot_types.len() {
        let st = meta.slot_types[i];
        if st == SlotType::GcRef {
            let child = unsafe { Gc::read_slot(obj, i) };
            if child != 0 { gc.mark_gray(child as GcRef); }
        } else if st == SlotType::Interface0 {
            let header_slot = unsafe { Gc::read_slot(obj, i) };
            if interface::data_is_gc_ref(header_slot) {
                let child = unsafe { Gc::read_slot(obj, i + 1) };
                if child != 0 { gc.mark_gray(child as GcRef); }
            }
            i += 1;
        }
        i += 1;
    }
}

/// Finalize a GC object before deallocation.
/// Releases native resources (Box, etc.) not managed by GC.
pub fn finalize_object(obj: GcRef) {
    let header = Gc::header(obj);
    match header.kind() {
        ValueKind::Channel => unsafe { channel::drop_inner(obj); }
        ValueKind::Map => unsafe { map::drop_inner(obj); }
        _ => {}
    }
}