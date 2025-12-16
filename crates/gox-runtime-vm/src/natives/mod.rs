//! Native function implementations for the VM runtime.

pub mod builtin;
pub mod fmt;
pub mod strings;
pub mod strconv;

use gox_vm::NativeRegistry;

/// Register all native functions.
pub fn register_all(registry: &mut NativeRegistry) {
    builtin::register(registry);
    fmt::register(registry);
    strings::register(registry);
    strconv::register(registry);
}
