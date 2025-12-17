//! GoX WebAssembly Compiler
//!
//! This crate provides compilation of GoX bytecode to WebAssembly
//! using Cranelift.
//!
//! ## Architecture
//!
//! ```text
//! VM Bytecode → Cranelift IR → WebAssembly (.wasm)
//! ```
//!
//! ## Features
//!
//! - Standalone WASM modules
//! - WASI support for system access
//! - Browser-compatible output

/// WASM output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmFormat {
    /// Standalone WASM module
    Standalone,
    /// WASI-compatible module
    Wasi,
}

/// WASM compiler context
pub struct WasmCompiler {
    format: WasmFormat,
}

impl WasmCompiler {
    /// Create a new WASM compiler
    pub fn new(format: WasmFormat) -> Self {
        Self { format }
    }

    /// Create a standalone WASM compiler
    pub fn standalone() -> Self {
        Self::new(WasmFormat::Standalone)
    }

    /// Create a WASI-compatible WASM compiler
    pub fn wasi() -> Self {
        Self::new(WasmFormat::Wasi)
    }

    /// Get the output format
    pub fn format(&self) -> WasmFormat {
        self.format
    }
}

// TODO: Implement Cranelift-based WASM compilation
// - compile_module(bytecode: &Module) -> Vec<u8>  // WASM bytes
// - Bytecode → Cranelift IR translation
// - WASM emission
// - WASI bindings for system calls
