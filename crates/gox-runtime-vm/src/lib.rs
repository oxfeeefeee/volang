//! VM runtime implementation for GoX.
//!
//! This crate provides the runtime for the custom VM backend,
//! including native function implementations and Value/FFI conversion.

pub mod context;
pub mod natives;

use gox_vm::{Vm, NativeRegistry};

/// Create a VM with all native functions registered.
pub fn create_vm() -> Vm {
    let mut registry = NativeRegistry::new();
    natives::register_all(&mut registry);
    Vm::with_natives(registry)
}
