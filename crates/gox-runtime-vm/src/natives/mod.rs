//! Native function implementations for the VM runtime.

pub mod builtin;
pub mod bytes;
pub mod errors;
pub mod fmt;
pub mod math;
pub mod sort;
pub mod strconv;
pub mod strings;

use gox_vm::NativeRegistry;

/// Register all native functions.
pub fn register_all(registry: &mut NativeRegistry) {
    builtin::register(registry);
    bytes::register(registry);
    errors::register(registry);
    fmt::register(registry);
    math::register(registry);
    sort::register(registry);
    strconv::register(registry);
    strings::register(registry);
}
