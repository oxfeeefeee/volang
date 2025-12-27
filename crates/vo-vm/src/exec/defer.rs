//! Defer instructions: DeferPush, ErrDeferPush, Panic, Recover

use vo_runtime_core::gc::{Gc, GcRef};
use vo_common_core::types::{ValueKind, ValueMeta};

use crate::fiber::{DeferEntry, Fiber};
use crate::instruction::Instruction;
use crate::vm::ExecResult;

/// DeferPush instruction format:
/// - a: func_id (if flags bit 0 = 0) or closure_reg (if flags bit 0 = 1)
/// - b: arg_start
/// - c: arg_slots
/// - flags bit 0: is_closure
#[inline]
pub fn exec_defer_push(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc) {
    push_defer_entry(fiber, inst, gc, false);
}

#[inline]
pub fn exec_err_defer_push(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc) {
    push_defer_entry(fiber, inst, gc, true);
}

fn push_defer_entry(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc, is_errdefer: bool) {
    let is_closure = (inst.flags & 1) != 0;
    let arg_start = inst.b;
    let arg_slots = inst.c;
    let frame_depth = fiber.frames.len();

    let (func_id, closure) = if is_closure {
        let closure_ref = fiber.read_reg(inst.a) as GcRef;
        (0, closure_ref)
    } else {
        let func_id = inst.a as u32 | ((inst.flags as u32 >> 1) << 16);
        (func_id, core::ptr::null_mut())
    };

    let args = if arg_slots > 0 {
        let args_ref = gc.alloc(ValueMeta::new(0, ValueKind::Array), arg_slots);
        for i in 0..arg_slots {
            let val = fiber.read_reg(arg_start + i);
            unsafe { Gc::write_slot(args_ref, i as usize, val) };
        }
        args_ref
    } else {
        core::ptr::null_mut()
    };

    fiber.defer_stack.push(DeferEntry {
        frame_depth,
        func_id,
        closure,
        args,
        arg_slots,
        is_closure,
        is_errdefer,
    });
}

#[inline]
pub fn exec_panic(fiber: &mut Fiber, inst: &Instruction) -> ExecResult {
    let val = fiber.read_reg(inst.a) as GcRef;
    fiber.panic_value = Some(val);
    ExecResult::Panic
}

#[inline]
pub fn exec_recover(fiber: &mut Fiber, inst: &Instruction) {
    let val = fiber.panic_value.take().map(|v| v as u64).unwrap_or(0);
    fiber.write_reg(inst.a, val);
}
