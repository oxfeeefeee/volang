//! External function call: CallExtern
//!
//! Uses ExternRegistry from vo-runtime-core for extern function dispatch.

use crate::bytecode::ExternDef;
use crate::fiber::Fiber;
use crate::instruction::Instruction;
use crate::vm::ExecResult;

pub use vo_runtime_core::ffi::ExternRegistry;
use vo_runtime_core::ffi::ExternResult;
use vo_runtime_core::gc::Gc;

pub fn exec_call_extern(
    fiber: &mut Fiber,
    inst: &Instruction,
    externs: &[ExternDef],
    registry: &ExternRegistry,
    gc: &mut Gc,
) -> ExecResult {
    // CallExtern: a=dst, b=extern_id, c=args_start, flags=arg_count
    let extern_id = inst.b as u32;
    let arg_start = inst.c;
    let arg_count = inst.flags as u16;

    if extern_id as usize >= externs.len() {
        return ExecResult::Panic;
    }
    let _extern_def = &externs[extern_id as usize];

    let frame = fiber.frames.last().expect("no active frame");
    let bp = frame.bp;

    // Call through ExternRegistry using ExternCall API
    let result = registry.call(
        extern_id,
        &mut fiber.stack,
        bp,
        arg_start,
        arg_count,
        arg_start, // ret_start same as arg_start (reuses argument slots)
        gc,
    );

    match result {
        ExternResult::Ok => ExecResult::Continue,
        ExternResult::Yield => ExecResult::Yield,
        ExternResult::Panic(_) => ExecResult::Panic,
    }
}
