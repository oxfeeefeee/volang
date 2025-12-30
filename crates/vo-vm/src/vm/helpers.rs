//! Stack and memory access helpers.

use vo_runtime::gc::GcRef;
use vo_runtime::objects::{array, slice, string};

const ARRAY_DATA_OFFSET: usize = array::HEADER_SLOTS;
const SLICE_FIELD_DATA_PTR: usize = slice::FIELD_DATA_PTR;
const SLICE_FIELD_LEN: usize = slice::FIELD_LEN;
const SLICE_FIELD_CAP: usize = slice::FIELD_CAP;
const STRING_FIELD_ARRAY: usize = string::FIELD_ARRAY;
const STRING_FIELD_START: usize = string::FIELD_START;
const STRING_FIELD_LEN: usize = string::FIELD_LEN;

// =============================================================================
// Stack access helpers
// =============================================================================

/// Unchecked stack read - SAFETY: caller ensures idx is within bounds
#[inline(always)]
pub fn stack_get(stack: &[u64], idx: usize) -> u64 {
    unsafe { *stack.get_unchecked(idx) }
}

/// Unchecked stack write - SAFETY: caller ensures idx is within bounds
#[inline(always)]
pub fn stack_set(stack: &mut [u64], idx: usize, val: u64) {
    unsafe { *stack.get_unchecked_mut(idx) = val }
}

// =============================================================================
// Slice/String field access
// =============================================================================

#[inline(always)]
pub fn slice_data_ptr(s: GcRef) -> *mut u8 {
    unsafe { *((s as *const u64).add(SLICE_FIELD_DATA_PTR)) as *mut u8 }
}

#[inline(always)]
pub fn slice_len(s: GcRef) -> usize {
    unsafe { *((s as *const u64).add(SLICE_FIELD_LEN)) as usize }
}

#[inline(always)]
pub fn slice_cap(s: GcRef) -> usize {
    unsafe { *((s as *const u64).add(SLICE_FIELD_CAP)) as usize }
}

#[inline(always)]
pub fn string_len(s: GcRef) -> usize {
    unsafe { *((s as *const u32).add(STRING_FIELD_LEN)) as usize }
}

#[inline(always)]
pub fn string_index(s: GcRef, idx: usize) -> u8 {
    let arr = unsafe { *((s as *const u64).add(STRING_FIELD_ARRAY) as *const GcRef) };
    let start = unsafe { *((s as *const u32).add(STRING_FIELD_START)) as usize };
    unsafe { *((arr.add(ARRAY_DATA_OFFSET) as *const u8).add(start + idx)) }
}
