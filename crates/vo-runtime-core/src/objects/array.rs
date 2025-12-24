//! Array object operations.
//!
//! Layout: GcHeader + ArrayHeader + [elements...]
//! - GcHeader: kind=Array, meta_id=0 (Array doesn't need its own meta_id)
//! - ArrayHeader: len (usize), elem_meta (ValueMeta), elem_bytes (u32) - 2 slots
//! - Elements: len * elem_slots slots

use crate::gc::{Gc, GcRef};
use vo_common_core::types::{ValueKind, ValueMeta};

#[repr(C)]
pub struct ArrayHeader {
    pub len: usize,
    pub elem_meta: ValueMeta,
    pub elem_bytes: u32,
}

pub const HEADER_SLOTS: usize = 2;
const _: () = assert!(core::mem::size_of::<ArrayHeader>() == HEADER_SLOTS * 8);

impl ArrayHeader {
    #[inline]
    fn as_ref(arr: GcRef) -> &'static Self {
        unsafe { &*(arr as *const Self) }
    }

    #[inline]
    fn as_mut(arr: GcRef) -> &'static mut Self {
        unsafe { &mut *(arr as *mut Self) }
    }
}

pub fn create(gc: &mut Gc, elem_meta: ValueMeta, elem_slots: usize, length: usize) -> GcRef {
    let total_slots = HEADER_SLOTS + length * elem_slots;
    let array_meta = ValueMeta::new(0, ValueKind::Array);
    let arr = gc.alloc(array_meta, total_slots as u16);
    let header = ArrayHeader::as_mut(arr);
    header.len = length;
    header.elem_meta = elem_meta;
    header.elem_bytes = (elem_slots * 8) as u32;
    arr
}

#[inline]
pub fn len(arr: GcRef) -> usize {
    ArrayHeader::as_ref(arr).len
}

#[inline]
pub fn elem_meta(arr: GcRef) -> ValueMeta {
    ArrayHeader::as_ref(arr).elem_meta
}

#[inline]
pub fn elem_kind(arr: GcRef) -> ValueKind {
    elem_meta(arr).value_kind()
}

#[inline]
pub fn elem_meta_id(arr: GcRef) -> u32 {
    elem_meta(arr).meta_id()
}

#[inline]
pub fn elem_bytes(arr: GcRef) -> usize {
    ArrayHeader::as_ref(arr).elem_bytes as usize
}

#[inline]
fn data_ptr(arr: GcRef) -> *mut u64 {
    unsafe { arr.add(HEADER_SLOTS) }
}

#[inline]
pub fn get(arr: GcRef, offset: usize) -> u64 {
    unsafe { *data_ptr(arr).add(offset) }
}

#[inline]
pub fn set(arr: GcRef, offset: usize, val: u64) {
    unsafe { *data_ptr(arr).add(offset) = val }
}

pub fn get_n(arr: GcRef, offset: usize, dest: &mut [u64]) {
    let ptr = unsafe { data_ptr(arr).add(offset) };
    for (i, d) in dest.iter_mut().enumerate() {
        *d = unsafe { *ptr.add(i) };
    }
}

pub fn set_n(arr: GcRef, offset: usize, src: &[u64]) {
    let ptr = unsafe { data_ptr(arr).add(offset) };
    for (i, &s) in src.iter().enumerate() {
        unsafe { *ptr.add(i) = s };
    }
}

pub fn copy_range(src: GcRef, src_offset: usize, dst: GcRef, dst_offset: usize, slot_count: usize) {
    let src_ptr = unsafe { data_ptr(src).add(src_offset) };
    let dst_ptr = unsafe { data_ptr(dst).add(dst_offset) };
    unsafe {
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, slot_count);
    }
}

pub fn as_bytes(arr: GcRef) -> *const u8 {
    data_ptr(arr) as *const u8
}

pub fn as_bytes_mut(arr: GcRef) -> *mut u8 {
    data_ptr(arr) as *mut u8
}
