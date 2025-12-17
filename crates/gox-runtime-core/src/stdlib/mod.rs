//! Standard library core implementations.
//!
//! This module contains the pure business logic for stdlib functions,
//! independent of calling convention. Different execution engines
//! (VM, Cranelift, WASM) provide their own bindings to these functions.
//!
//! # Architecture
//!
//! ```text
//! stdlib/           <- Core logic (this module)
//!    │
//!    ├── VM Binding (gox-runtime-vm/stdlib, NativeCtx)
//!    ├── C ABI Binding (gox-runtime-core/ffi.rs, extern "C")
//!    └── WASI Binding (future)
//! ```

#[cfg(feature = "std")]
pub mod builtin;

#[cfg(feature = "std")]
pub mod strings;

#[cfg(feature = "std")]
pub mod fmt;
