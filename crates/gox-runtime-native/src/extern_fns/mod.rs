//! Extern function implementations for JIT/AOT.
//!
//! Each stdlib package has its own module with extern function implementations.

pub mod strings;
pub mod math;
pub mod strconv;
pub mod fmt;

use crate::extern_dispatch::ExternDispatchFn;

/// Register all extern functions from all packages.
pub fn register_all(register: &mut dyn FnMut(&str, ExternDispatchFn)) {
    strings::register(register);
    math::register(register);
    strconv::register(register);
    fmt::register(register);
}
