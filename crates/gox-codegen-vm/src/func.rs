//! Function compilation utilities.

use gox_syntax::ast::FuncDecl;
use gox_vm::FunctionDef;

use crate::{CodegenContext, CodegenError};
use crate::context::FuncContext;

/// Get the number of parameter slots for a function.
pub fn param_slots(func: &FuncDecl) -> u16 {
    let mut slots = 0u16;
    if let Some(ref sig) = func.sig {
        for param in &sig.params {
            slots += param.names.len() as u16;
        }
    }
    slots
}

/// Get the number of return slots for a function.
pub fn return_slots(func: &FuncDecl) -> u16 {
    let mut slots = 0u16;
    if let Some(ref sig) = func.sig {
        slots = sig.results.len() as u16;
        if slots == 0 {
            // Check for named returns
            for result in &sig.results {
                slots += result.names.len().max(1) as u16;
            }
        }
    }
    slots
}
