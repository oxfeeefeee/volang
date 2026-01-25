//! SDK for creating Vo native extensions.
//!
//! This crate provides the API for implementing Vo functions in Rust.
//!
//! # Platform Support
//!
//! - **Native** (default): Extensions are compiled as dynamic libraries (.so/.dylib/.dll)
//!   and loaded by the Vo runtime at startup via dlopen.
//! - **WASM**: Extensions are statically linked and registered at initialization.
//!
//! # Example
//!
//! ```ignore
//! use vo_ext::prelude::*;
//!
//! #[vo_extern("mylib/math", "FastAdd")]
//! fn fast_add(a: i64, b: i64) -> i64 {
//!     a + b
//! }
//!
//! // Required: export the extension table
//! vo_ext::export_extensions!();
//! ```
//!
//! # Extension Structure
//!
//! A typical extension project:
//!
//! ```text
//! myext/
//! ├── vo.ext.toml          # Extension manifest
//! ├── math/
//! │   └── math.vo          # Vo interface (extern declarations)
//! └── native/
//!     ├── Cargo.toml
//!     └── src/
//!         └── lib.rs       # Native implementation
//! ```

pub use vo_ffi_macro::vo_extern;
pub use vo_ffi_macro::vo_extern_ctx;
pub use vo_ffi_macro::vo_struct;
pub use vo_runtime::ffi::{
    ExternCall, ExternCallContext, ExternFn, ExternFnWithContext, 
    ExternResult, StdlibEntry, ExternRegistry,
    // Type-safe slot wrapper for interface types
    InterfaceSlot,
    // Container accessors
    VoSlice, VoSliceCursor, VoMap, VoMapCursor,
    VoArray, VoArrayCursor, VoString, VoBytes,
    VoElem, VoStringElem,
    // Pointer and closure accessors
    VoPtr, VoClosure,
};
pub use vo_runtime::gc::GcRef;

// Native platform: re-export linkme types for auto-registration
#[cfg(feature = "native")]
pub use vo_runtime::ffi::{ExternEntry, ExternEntryWithContext, EXTERN_TABLE, EXTERN_TABLE_WITH_CONTEXT};
#[cfg(feature = "native")]
pub use vo_runtime::distributed_slice;

/// ABI version for extension compatibility checking.
pub const ABI_VERSION: u32 = 1;

/// Extension table returned by `vo_ext_get_entries` (native platform only).
#[cfg(feature = "native")]
#[repr(C)]
pub struct ExtensionTable {
    /// ABI version - must match runtime's ABI_VERSION.
    pub version: u32,
    /// Number of entries in the basic extern table.
    pub entry_count: usize,
    /// Pointer to extern entries array.
    pub entries: *const ExternEntry,
    /// Number of entries in the context extern table.
    pub entry_with_context_count: usize,
    /// Pointer to extern entries with context array.
    pub entries_with_context: *const ExternEntryWithContext,
}

#[cfg(feature = "native")]
unsafe impl Send for ExtensionTable {}
#[cfg(feature = "native")]
unsafe impl Sync for ExtensionTable {}

/// Export the extension entry point.
///
/// # Native Platform (default)
/// Call with no arguments. Generates `vo_ext_get_entries` function for dlopen.
/// ```ignore
/// vo_ext::export_extensions!();
/// ```
///
/// # WASM Platform
/// Call with list of StdlibEntry constants. Generates `vo_ext_register` function.
/// ```ignore
/// vo_ext::export_extensions!(__STDLIB_gui_emitRender, __STDLIB_gui_navigate);
/// ```
#[macro_export]
macro_rules! export_extensions {
    // Native: no arguments, auto-collect from linkme tables
    () => {
        #[no_mangle]
        pub extern "C" fn vo_ext_get_entries() -> $crate::ExtensionTable {
            $crate::ExtensionTable {
                version: $crate::ABI_VERSION,
                entry_count: $crate::EXTERN_TABLE.len(),
                entries: $crate::EXTERN_TABLE.as_ptr(),
                entry_with_context_count: $crate::EXTERN_TABLE_WITH_CONTEXT.len(),
                entries_with_context: $crate::EXTERN_TABLE_WITH_CONTEXT.as_ptr(),
            }
        }
    };
    
    // WASM: explicit list of entries (caller must implement registration)
    ($($entry:ident),+ $(,)?) => {
        /// All extern entries for this extension.
        pub static VO_EXT_ENTRIES: &[$crate::StdlibEntry] = &[$($entry),*];
    };
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::export_extensions;
    pub use crate::vo_extern;
    pub use crate::vo_extern_ctx;
    pub use crate::vo_struct;
    pub use crate::ExternResult;
    pub use crate::ExternCallContext;
    pub use crate::InterfaceSlot;
    pub use crate::GcRef;
    // Container accessors
    pub use crate::{VoSlice, VoSliceCursor, VoMap, VoMapCursor};
    pub use crate::{VoArray, VoArrayCursor, VoString, VoBytes};
    pub use crate::{VoElem, VoStringElem};
    // Pointer and closure accessors
    pub use crate::{VoPtr, VoClosure};
}
