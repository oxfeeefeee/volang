//! Interface operations.
//!
//! Interface is a value type (2 slots on stack):
//! - Slot 0: [itab_id:32 | value_meta:32]  (value_meta = meta_id:24 | value_kind:8)
//! - Slot 1: data = immediate value or GcRef
//!
//! nil check: value_kind == Void (same as Go: typed nil is NOT nil interface)

use vo_common_core::types::{ValueKind, ValueMeta};

pub const SLOT_COUNT: usize = 2;

/// Pack slot0 from itab_id and value_meta
#[inline]
pub fn pack_slot0(itab_id: u32, value_meta: ValueMeta) -> u64 {
    ((itab_id as u64) << 32) | (value_meta.to_raw() as u64)
}

/// Extract itab_id from slot0 (high 32 bits)
#[inline]
pub fn unpack_itab_id(slot0: u64) -> u32 {
    (slot0 >> 32) as u32
}

/// Extract value_meta from slot0 (low 32 bits)
#[inline]
pub fn unpack_value_meta(slot0: u64) -> ValueMeta {
    ValueMeta::from_raw(slot0 as u32)
}

/// Extract value_kind from slot0
#[inline]
pub fn unpack_value_kind(slot0: u64) -> ValueKind {
    unpack_value_meta(slot0).value_kind()
}

/// Check if interface is nil (value_kind == Void)
/// Note: typed nil (e.g. (*T)(nil)) is NOT nil interface (same as Go)
#[inline]
pub fn is_nil(slot0: u64) -> bool {
    unpack_value_kind(slot0) == ValueKind::Void
}

/// Check if slot1 data is a GC reference
#[inline]
pub fn data_is_gc_ref(slot0: u64) -> bool {
    let vk = unpack_value_kind(slot0);
    vk.may_contain_gc_refs()
}
