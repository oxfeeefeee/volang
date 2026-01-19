//! Vo Runner - compile, run, and inspect Vo programs.
//!
//! This crate provides FFI bindings for the runner package in Vo,
//! including AST parsing/printing and bytecode formatting.

mod ffi;
mod printer;
mod format;

pub use vo_launcher::*;
pub use printer::AstPrinter;
pub use format::{format_text, parse_text};
