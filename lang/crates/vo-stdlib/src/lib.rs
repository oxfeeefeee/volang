//! Vo Standard Library
//!
//! This crate provides:
//! 1. Embedded Vo source files for the standard library
//! 2. Platform-specific extern implementations (native/wasm)

mod embedded;

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use embedded::{EmbeddedStdlib, StdlibFs};

use vo_runtime::bytecode::ExternDef;
use vo_runtime::ffi::ExternRegistry;

/// Register all stdlib extern functions.
pub fn register_externs(registry: &mut ExternRegistry, externs: &[ExternDef]) {
    #[cfg(not(target_arch = "wasm32"))]
    native::register_externs(registry, externs);
    
    #[cfg(target_arch = "wasm32")]
    wasm::register_externs(registry, externs);
}
