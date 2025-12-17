//! Standard package native implementations.
//! These packages require OS support.

pub mod fmt;
pub mod os;
pub mod path;
pub mod rand;
pub mod time;

use gox_vm::NativeRegistry;

/// Register all std package native functions.
pub fn register_all(registry: &mut NativeRegistry) {
    fmt::register(registry);
    os::register(registry);
    path::register(registry);
    rand::register(registry);
    time::register(registry);
}
