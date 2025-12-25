#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod types;
pub mod gc;
pub mod gc_types;
pub mod objects;
pub mod ffi;
pub mod builtins;
pub mod stdlib;

pub use vo_common_core::types::{MetaId, SlotType, ValueKind, ValueMeta};
pub use ffi::{
    ExternCall, ExternCallWithGc, ExternFn, ExternFnWithGc, ExternRegistry, ExternResult,
    ExternEntry, ExternEntryWithGc, EXTERN_TABLE, EXTERN_TABLE_WITH_GC,
    lookup_extern, lookup_extern_with_gc,
};
pub use linkme::distributed_slice;
