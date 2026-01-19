//! WebAssembly bindings for Vo Playground.
//!
//! TODO: Rewrite this crate to implement the playground RFC.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in console.
#[cfg(feature = "std")]
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Get version information.
#[wasm_bindgen]
pub fn version() -> alloc::string::String {
    "Vo Web 0.1.0 (stub)".into()
}
