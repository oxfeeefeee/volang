//! Garbage collector - re-exports from gox-runtime-core.
//!
//! This module re-exports the GC implementation from gox-runtime-core,
//! ensuring a single source of truth that both VM and Cranelift backends use.

pub use gox_runtime_core::gc::*;
