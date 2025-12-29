//! Slice object operations.
//!
//! Layout: GcHeader + SliceData
//! Slice references an underlying array with start offset.

use crate::gc::{Gc, GcRef};
use crate::objects::array;
use vo_common_core::types::{ValueKind, ValueMeta};

#[repr(C)]
pub struct SliceData {
    pub array: GcRef,
    pub start: usize,
    pub len: usize,
    pub cap: usize,
}

pub const DATA_SLOTS: u16 = 4;
const _: () = assert!(core::mem::size_of::<SliceData>() == DATA_SLOTS as usize * 8);

pub const FIELD_ARRAY: usize = 0;
pub const FIELD_START: usize = 1;
pub const FIELD_LEN: usize = 2;
pub const FIELD_CAP: usize = 3;

impl SliceData {
    #[inline]
    fn as_ref(s: GcRef) -> &'static Self {
        unsafe { &*(s as *const Self) }
    }

    #[inline]
    fn as_mut(s: GcRef) -> &'static mut Self {
        unsafe { &mut *(s as *mut Self) }
    }
}

/// Create a new slice with packed element storage.
/// elem_bytes: actual byte size per element (1/2/4/8 for packed, slots*8 for slot-based)
pub fn create(gc: &mut Gc, elem_meta: ValueMeta, elem_bytes: usize, length: usize, capacity: usize) -> GcRef {
    let arr = array::create(gc, elem_meta, elem_bytes, capacity);
    from_array_range(gc, arr, 0, length, capacity)
}

pub fn from_array_range(gc: &mut Gc, arr: GcRef, start_off: usize, length: usize, capacity: usize) -> GcRef {
    let s = gc.alloc(ValueMeta::new(0, ValueKind::Slice), DATA_SLOTS);
    let data = SliceData::as_mut(s);
    data.array = arr;
    data.start = start_off;
    data.len = length;
    data.cap = capacity;
    s
}

pub fn from_array(gc: &mut Gc, arr: GcRef) -> GcRef {
    let length = array::len(arr);
    from_array_range(gc, arr, 0, length, length)
}

#[inline]
pub fn array_ref(s: GcRef) -> GcRef { SliceData::as_ref(s).array }
#[inline]
pub fn start(s: GcRef) -> usize { SliceData::as_ref(s).start }
#[inline]
pub fn len(s: GcRef) -> usize { SliceData::as_ref(s).len }
#[inline]
pub fn cap(s: GcRef) -> usize { SliceData::as_ref(s).cap }
#[inline]
pub fn elem_kind(s: GcRef) -> ValueKind { array::elem_kind(array_ref(s)) }
#[inline]
pub fn elem_meta_id(s: GcRef) -> u32 { array::elem_meta_id(array_ref(s)) }
#[inline]
pub fn elem_meta(s: GcRef) -> ValueMeta { array::elem_meta(array_ref(s)) }

#[inline]
pub fn get(s: GcRef, idx: usize, elem_bytes: usize) -> u64 {
    // start is element index
    array::get(array_ref(s), start(s) + idx, elem_bytes)
}
#[inline]
pub fn get_auto(arr: GcRef, start: usize, idx: usize, elem_bytes: usize) -> u64 {
    array::get_auto(arr, start + idx, elem_bytes)
}
#[inline]
pub fn set(s: GcRef, idx: usize, val: u64, elem_bytes: usize) {
    array::set(array_ref(s), start(s) + idx, val, elem_bytes);
}
#[inline]
pub fn set_auto(arr: GcRef, start: usize, idx: usize, val: u64, elem_bytes: usize) {
    array::set_auto(arr, start + idx, val, elem_bytes);
}

pub fn get_n(s: GcRef, idx: usize, dest: &mut [u64], elem_bytes: usize) {
    array::get_n(array_ref(s), start(s) + idx, dest, elem_bytes);
}
pub fn set_n(s: GcRef, idx: usize, src: &[u64], elem_bytes: usize) {
    array::set_n(array_ref(s), start(s) + idx, src, elem_bytes);
}

/// Two-index slice: s[new_start:new_end] - capacity extends to original cap
pub fn slice_of(gc: &mut Gc, s: GcRef, new_start: usize, new_end: usize) -> GcRef {
    let data = SliceData::as_ref(s);
    from_array_range(gc, data.array, data.start + new_start, new_end - new_start, data.cap - new_start)
}

/// Three-index slice: s[new_start:new_end:max] - capacity = max - new_start
pub fn slice_of_with_cap(gc: &mut Gc, s: GcRef, new_start: usize, new_end: usize, max: usize) -> GcRef {
    let data = SliceData::as_ref(s);
    from_array_range(gc, data.array, data.start + new_start, new_end - new_start, max - new_start)
}

/// Append single element to slice.
/// elem_bytes: actual byte size per element
pub fn append(gc: &mut Gc, em: ValueMeta, elem_bytes: usize, s: GcRef, val: &[u64]) -> GcRef {
    if s.is_null() {
        let new_arr = array::create(gc, em, elem_bytes, 4);
        array::set_n(new_arr, 0, val, elem_bytes);
        return from_array_range(gc, new_arr, 0, 1, 4);
    }
    let data = SliceData::as_ref(s);
    let cur_len = data.len;
    let cur_cap = data.cap;
    if cur_len < cur_cap {
        array::set_n(data.array, data.start + cur_len, val, elem_bytes);
        SliceData::as_mut(s).len = cur_len + 1;
        s
    } else {
        let new_cap = if cur_cap == 0 { 4 } else { cur_cap * 2 };
        let aem = elem_meta(s);
        let new_arr = array::create(gc, aem, elem_bytes, new_cap);
        array::copy_range(data.array, data.start, new_arr, 0, cur_len, elem_bytes);
        array::set_n(new_arr, cur_len, val, elem_bytes);
        from_array_range(gc, new_arr, 0, cur_len + 1, new_cap)
    }
}
