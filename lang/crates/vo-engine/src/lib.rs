//! Vo Compilation and Execution Core
//!
//! This crate provides the core compile and run functionality for Vo programs.
//! It is used by both the Vo CLI launcher and the vox library.

mod compile;
mod run;

pub use compile::{compile, compile_with_cache, compile_string, CompileError, CompileOutput};
pub use run::{run, RunMode, RunError, RuntimeError, RuntimeErrorKind};

pub use vo_vm::bytecode::Module;
