//! Bytecode text format parser and formatter.
//!
//! TODO: Rewrite to use new vo-vm instruction format.

use vo_vm::bytecode::Module;

/// Parse bytecode text format into a Module.
pub fn parse_text(_input: &str) -> Result<Module, String> {
    Err("bytecode text format not yet implemented for new VM".into())
}

/// Format a Module as text.
pub fn format_text(module: &Module) -> String {
    format!("# Module: {}\n# (bytecode text format pending rewrite)\n", module.name)
}
