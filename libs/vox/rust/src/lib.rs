//! Vox - compile, run, and inspect Vo programs.
//!
//! This crate provides:
//! - Re-exports of vo-engine (compile, run, etc.)
//! - AST parsing and printing
//! - Bytecode formatting
//! - FFI bindings for the vox package in Vo

mod ffi;
mod printer;
mod format;

// Re-export vo-engine
pub use vo_engine::{compile, compile_with_cache, compile_string, CompileError, CompileOutput};
pub use vo_engine::{run, RunMode, RunError, RuntimeError, RuntimeErrorKind};
pub use vo_engine::Module;

pub use printer::AstPrinter;
pub use format::{format_text, parse_text};
