//! External function definitions for the VM.
//!
//! Registers stdlib extern functions by name.

use vo_vm::exec::{ExternRegistry, ExternCallResult, ExternFn};
use vo_vm::bytecode::Module;

/// Standard library mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdMode {
    /// Core mode: no OS dependencies (for WASM, embedded)
    Core,
    /// Full mode: complete standard library
    #[default]
    Full,
}

/// Register stdlib extern functions based on module's extern definitions.
pub fn register_stdlib(registry: &mut ExternRegistry, module: &Module) {
    for (id, def) in module.externs.iter().enumerate() {
        if let Some(func) = get_extern_fn(&def.name) {
            registry.register(id as u32, func);
        }
    }
}

/// Get extern function by name.
fn get_extern_fn(name: &str) -> Option<ExternFn> {
    match name {
        "vo_print" => Some(vo_print),
        "vo_println" => Some(vo_println),
        "vo_copy" => Some(vo_copy),
        _ => None,
    }
}

/// Print without newline.
fn vo_print(ret: &mut [u64], args: &[u64]) -> ExternCallResult {
    // args[0] is string pointer, args[1] is length (or just pointer for GcRef string)
    // For now, just print a placeholder
    if !args.is_empty() {
        let ptr = args[0] as *const u8;
        if !ptr.is_null() {
            // Try to read as GcRef string
            use vo_runtime_core::objects::string;
            let gc_ref = args[0] as vo_runtime_core::gc::GcRef;
            let s = string::as_str(gc_ref);
            print!("{}", s);
            if !ret.is_empty() {
                ret[0] = s.len() as u64;
            }
        }
    }
    ExternCallResult::Ok
}

/// Print with newline.
fn vo_println(ret: &mut [u64], args: &[u64]) -> ExternCallResult {
    if !args.is_empty() {
        let gc_ref = args[0] as vo_runtime_core::gc::GcRef;
        if !gc_ref.is_null() {
            use vo_runtime_core::objects::string;
            let s = string::as_str(gc_ref);
            println!("{}", s);
            if !ret.is_empty() {
                ret[0] = (s.len() + 1) as u64;
            }
        } else {
            println!();
            if !ret.is_empty() {
                ret[0] = 1;
            }
        }
    } else {
        println!();
        if !ret.is_empty() {
            ret[0] = 1;
        }
    }
    ExternCallResult::Ok
}

/// Copy slice.
fn vo_copy(ret: &mut [u64], _args: &[u64]) -> ExternCallResult {
    // TODO: Implement slice copy
    if !ret.is_empty() {
        ret[0] = 0;
    }
    ExternCallResult::Ok
}
