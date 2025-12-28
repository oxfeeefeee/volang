//! Call instructions: Call, CallExtern, CallClosure, CallIface, Return

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use vo_runtime_core::gc::{Gc, GcRef};
use vo_runtime_core::objects::closure;

use crate::bytecode::{FunctionDef, Module};
use crate::fiber::{CallFrame, DeferState, Fiber};
use crate::instruction::Instruction;
use crate::itab::ItabCache;
use crate::vm::ExecResult;

pub fn exec_call(fiber: &mut Fiber, inst: &Instruction, module: &Module) -> ExecResult {
    let func_id = (inst.a as u32) | ((inst.flags as u32) << 16);
    let arg_start = inst.b;
    let arg_slots = (inst.c >> 8) as usize;
    let ret_slots = (inst.c & 0xFF) as u16;

    let func = &module.functions[func_id as usize];

    let args: Vec<u64> = (0..arg_slots)
        .map(|i| fiber.read_reg(arg_start + i as u16))
        .collect();

    fiber.push_frame(func_id, func.local_slots, arg_start, ret_slots);

    for (i, arg) in args.into_iter().enumerate() {
        fiber.write_reg(i as u16, arg);
    }

    ExecResult::Continue
}

pub fn exec_call_closure(fiber: &mut Fiber, inst: &Instruction, module: &Module) -> ExecResult {
    let closure_ref = fiber.read_reg(inst.a) as GcRef;
    let func_id = closure::func_id(closure_ref);
    let arg_start = inst.b;
    let arg_slots = (inst.c >> 8) as usize;
    let ret_slots = (inst.c & 0xFF) as u16;

    let func = &module.functions[func_id as usize];

    let args: Vec<u64> = (0..arg_slots)
        .map(|i| fiber.read_reg(arg_start + i as u16))
        .collect();

    fiber.push_frame(func_id, func.local_slots, arg_start, ret_slots);

    fiber.write_reg(0, closure_ref as u64);

    for (i, arg) in args.into_iter().enumerate() {
        fiber.write_reg((i + 1) as u16, arg);
    }

    ExecResult::Continue
}

pub fn exec_call_iface(
    fiber: &mut Fiber,
    inst: &Instruction,
    module: &Module,
    itab_cache: &ItabCache,
) -> ExecResult {
    let arg_slots = (inst.c >> 8) as usize;
    let ret_slots = (inst.c & 0xFF) as u16;
    let method_idx = inst.flags as usize;

    let slot0 = fiber.read_reg(inst.a);
    let slot1 = fiber.read_reg(inst.a + 1);

    let itab_id = (slot0 >> 32) as u32;
    let func_id = itab_cache.lookup_method(itab_id, method_idx);

    let func = &module.functions[func_id as usize];
    let recv_slots = func.recv_slots as usize;

    let args: Vec<u64> = (0..arg_slots)
        .map(|i| fiber.read_reg(inst.b + i as u16))
        .collect();

    fiber.push_frame(func_id, func.local_slots, inst.b, ret_slots);

    // Pass slot1 directly as receiver (1 slot: GcRef or primitive)
    // For value receiver methods, itab points to wrapper that dereferences
    fiber.write_reg(0, slot1);

    for (i, arg) in args.into_iter().enumerate() {
        fiber.write_reg((recv_slots + i) as u16, arg);
    }

    ExecResult::Continue
}

pub fn exec_return(
    fiber: &mut Fiber,
    inst: &Instruction,
    _func: &FunctionDef,
    module: &Module,
    is_error_return: bool,
) -> ExecResult {
    let current_frame_depth = fiber.frames.len();

    // Check if we're continuing defer execution (a defer just returned)
    if let Some(ref mut state) = fiber.defer_state {
        // A defer just finished, check if more to execute
        if let Some(entry) = state.pending.pop() {
            // Pop the current defer frame before calling next defer
            fiber.pop_frame();
            // Execute next defer
            return call_defer_entry(fiber, &entry, module);
        } else {
            // All defers done, complete the original return
            let ret_vals = core::mem::take(&mut state.ret_vals);
            let caller_ret_reg = state.caller_ret_reg;
            let caller_ret_count = state.caller_ret_count;
            fiber.defer_state = None;

            // Pop the defer frame before writing to caller's registers
            fiber.pop_frame();

            if fiber.frames.is_empty() {
                return ExecResult::Done;
            }

            let write_count = caller_ret_count.min(ret_vals.len());
            for i in 0..write_count {
                fiber.write_reg(caller_ret_reg + i as u16, ret_vals[i]);
            }
            return ExecResult::Return;
        }
    }

    // Normal return - check for defers
    let ret_start = inst.a as usize;
    let ret_count = inst.b as usize;

    let ret_vals: Vec<u64> = (0..ret_count)
        .map(|i| fiber.read_reg((ret_start + i) as u16))
        .collect();

    // Collect defers for current frame (in reverse order for LIFO)
    let mut pending_defers: Vec<_> = Vec::new();
    while let Some(entry) = fiber.defer_stack.last() {
        if entry.frame_depth != current_frame_depth {
            break;
        }
        let entry = fiber.defer_stack.pop().unwrap();
        // Skip errdefer if not error return
        if entry.is_errdefer && !is_error_return {
            continue;
        }
        pending_defers.push(entry);
    }

    let frame = fiber.pop_frame();
    if frame.is_none() {
        return ExecResult::Done;
    }
    let frame = frame.unwrap();

    if !pending_defers.is_empty() {
        // Has defers to execute - save state and call first defer
        let first_defer = pending_defers.pop().unwrap();
        fiber.defer_state = Some(DeferState {
            pending: pending_defers,
            ret_vals,
            caller_ret_reg: frame.ret_reg,
            caller_ret_count: frame.ret_count as usize,
            is_error_return,
        });
        return call_defer_entry(fiber, &first_defer, module);
    }

    // No defers - normal return
    if fiber.frames.is_empty() {
        return ExecResult::Done;
    }

    let write_count = (frame.ret_count as usize).min(ret_vals.len());
    for i in 0..write_count {
        fiber.write_reg(frame.ret_reg + i as u16, ret_vals[i]);
    }

    ExecResult::Return
}

fn call_defer_entry(fiber: &mut Fiber, entry: &crate::fiber::DeferEntry, module: &Module) -> ExecResult {
    let func_id = if entry.is_closure {
        closure::func_id(entry.closure)
    } else {
        entry.func_id
    };

    let func = &module.functions[func_id as usize];
    let arg_slots = entry.arg_slots as usize;

    // Allocate space for args (defer functions have no return value we care about)
    let args_start = fiber.stack.len();
    fiber.stack.resize(args_start + func.local_slots as usize, 0);

    // Copy args from heap to stack
    if !entry.args.is_null() {
        for i in 0..arg_slots {
            let val = unsafe { Gc::read_slot(entry.args, i) };
            fiber.stack[args_start + i] = val;
        }
    }

    // For closure call, set closure ref as first slot
    if entry.is_closure {
        fiber.stack[args_start] = entry.closure as u64;
    }

    // Push frame for defer function
    fiber.frames.push(CallFrame {
        func_id,
        pc: 0,
        bp: args_start,
        ret_reg: 0,  // defer return values are ignored
        ret_count: 0,
    });

    ExecResult::Continue
}
