//! Call instructions: Call, CallExtern, CallClosure, CallIface, Return

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use vo_runtime::gc::{Gc, GcRef};
use vo_runtime::objects::closure;

use crate::bytecode::{FunctionDef, Module};
use crate::fiber::{CallFrame, DeferEntry, DeferState};
use crate::instruction::Instruction;
use crate::vm::ExecResult;
use vo_runtime::itab::ItabCache;

pub fn exec_call(
    stack: &mut Vec<u64>,
    frames: &mut Vec<CallFrame>,
    inst: &Instruction,
    module: &Module,
) -> ExecResult {
    let func_id = (inst.a as u32) | ((inst.flags as u32) << 16);
    let arg_start = inst.b as usize;
    let arg_slots = (inst.c >> 8) as usize;
    let ret_slots = (inst.c & 0xFF) as usize;

    let func = &module.functions[func_id as usize];

    // Get caller's bp before pushing new frame
    let caller_bp = frames.last().map_or(0, |f| f.bp);
    
    // Ensure caller's stack is large enough for return value write-back
    // This must happen BEFORE computing new_bp
    let ret_write_end = caller_bp + arg_start + ret_slots;
    if stack.len() < ret_write_end {
        stack.resize(ret_write_end, 0);
    }
    
    // New frame's bp is current stack top (now guaranteed >= ret_write_end)
    let new_bp = stack.len();
    
    // Extend stack for new frame
    stack.resize(new_bp + func.local_slots as usize, 0);
    
    // Copy args directly from caller's frame to new frame (no Vec allocation)
    // SAFETY: source and dest don't overlap since new_bp >= caller_bp + arg_start + arg_slots
    for i in 0..arg_slots {
        stack[new_bp + i] = stack[caller_bp + arg_start + i];
    }
    
    // Push frame
    frames.push(CallFrame {
        func_id,
        pc: 0,
        bp: new_bp,
        ret_reg: inst.b,
        ret_count: ret_slots as u16,
    });

    // Return because frames changed
    ExecResult::Return
}

pub fn exec_call_closure(
    stack: &mut Vec<u64>,
    frames: &mut Vec<CallFrame>,
    inst: &Instruction,
    module: &Module,
) -> ExecResult {
    let caller_bp = frames.last().map_or(0, |f| f.bp);
    let closure_ref = stack[caller_bp + inst.a as usize] as GcRef;
    let func_id = closure::func_id(closure_ref);
    let arg_start = inst.b as usize;
    let arg_slots = (inst.c >> 8) as usize;
    // Dynamic ret_slots handling:
    // - flags == 0: ret_slots from c & 0xFF (static)
    // - flags == 1: ret_slots from stack[caller_bp + arg_start - 1] (dynamic)
    let ret_slots = if inst.flags == 0 {
        (inst.c & 0xFF) as u16
    } else {
        // Dynamic mode: ret_slots stored at arg_start - 1
        debug_assert_eq!(inst.flags, 1, "CallClosure: unexpected flags value {}", inst.flags);
        stack[caller_bp + arg_start - 1] as u16
    };

    let func = &module.functions[func_id as usize];
    let recv_slots = func.recv_slots as usize;

    // New frame's bp is current stack top
    let new_bp = stack.len();
    
    // Extend stack for new frame
    stack.resize(new_bp + func.local_slots as usize, 0);
    
    // For method closures (recv_slots > 0), receiver is in captures[0]
    // For regular closures, slot 0 is closure ref
    if recv_slots > 0 && closure::capture_count(closure_ref) > 0 {
        // Method closure: copy receiver from captures to slot 0
        stack[new_bp] = closure::get_capture(closure_ref, 0);
        // Copy args after receiver
        for i in 0..arg_slots {
            stack[new_bp + recv_slots + i] = stack[caller_bp + arg_start + i];
        }
    } else {
        // Regular closure: slot 0 is closure ref
        stack[new_bp] = closure_ref as u64;
        // Copy args directly
        for i in 0..arg_slots {
            stack[new_bp + 1 + i] = stack[caller_bp + arg_start + i];
        }
    }
    
    // Push frame
    frames.push(CallFrame {
        func_id,
        pc: 0,
        bp: new_bp,
        ret_reg: inst.b,
        ret_count: ret_slots,
    });

    // Return because frames changed
    ExecResult::Return
}

pub fn exec_call_iface(
    stack: &mut Vec<u64>,
    frames: &mut Vec<CallFrame>,
    inst: &Instruction,
    module: &Module,
    itab_cache: &ItabCache,
) -> ExecResult {
    let arg_slots = (inst.c >> 8) as usize;
    let ret_slots = (inst.c & 0xFF) as usize;
    let method_idx = inst.flags as usize;

    let caller_bp = frames.last().map_or(0, |f| f.bp);
    let slot0 = stack[caller_bp + inst.a as usize];
    let slot1 = stack[caller_bp + inst.a as usize + 1];

    let itab_id = (slot0 >> 32) as u32;
    let func_id = itab_cache.lookup_method(itab_id, method_idx);

    let func = &module.functions[func_id as usize];
    let recv_slots = func.recv_slots as usize;

    // Ensure caller's stack is large enough for return value write-back
    // ret_reg is inst.b (args_start), return values written to caller_bp + ret_reg
    let ret_write_end = caller_bp + inst.b as usize + ret_slots;
    if stack.len() < ret_write_end {
        stack.resize(ret_write_end, 0);
    }
    
    // New frame's bp is current stack top (now guaranteed >= ret_write_end)
    let new_bp = stack.len();
    
    // Extend stack for new frame
    stack.resize(new_bp + func.local_slots as usize, 0);
    
    // Pass slot1 directly as receiver (1 slot: GcRef or primitive)
    stack[new_bp] = slot1;
    
    // Copy args directly (no Vec allocation)
    for i in 0..arg_slots {
        stack[new_bp + recv_slots + i] = stack[caller_bp + inst.b as usize + i];
    }
    
    // Push frame
    frames.push(CallFrame {
        func_id,
        pc: 0,
        bp: new_bp,
        ret_reg: inst.b,
        ret_count: ret_slots as u16,
    });

    // Return because frames changed
    ExecResult::Return
}

/// Collect defers for current frame in LIFO order, filtering out errdefers if not error return.
#[inline]
fn collect_pending_defers(
    defer_stack: &mut Vec<DeferEntry>,
    frame_depth: usize,
    is_error_return: bool,
) -> Vec<DeferEntry> {
    let mut pending = Vec::new();
    while let Some(entry) = defer_stack.last() {
        if entry.frame_depth != frame_depth {
            break;
        }
        let entry = defer_stack.pop().unwrap();
        if entry.is_errdefer && !is_error_return {
            continue;
        }
        pending.push(entry);
    }
    pending
}

/// Read values from heap GcRefs (for escaped named returns).
#[inline]
fn read_heap_gcrefs(heap_gcrefs: &[u64], value_slots_per_ref: usize) -> Vec<u64> {
    let mut vals = Vec::with_capacity(heap_gcrefs.len() * value_slots_per_ref);
    for &gcref_raw in heap_gcrefs {
        let gcref: GcRef = gcref_raw as GcRef;
        for offset in 0..value_slots_per_ref {
            // SAFETY: gcref points to valid heap allocation with at least value_slots_per_ref slots
            vals.push(unsafe { *gcref.add(offset) });
        }
    }
    vals
}

pub fn exec_return(
    stack: &mut Vec<u64>,
    frames: &mut Vec<CallFrame>,
    defer_stack: &mut Vec<DeferEntry>,
    defer_state: &mut Option<DeferState>,
    inst: &Instruction,
    func: &FunctionDef,
    module: &Module,
    is_error_return: bool,
) -> ExecResult {
    let current_frame_depth = frames.len();

    // Check if we're continuing defer execution (a defer just returned)
    if let Some(ref mut state) = defer_state {
        // A defer just finished, check if more to execute
        // pending is in LIFO order (first element = next to run)
        if !state.pending.is_empty() {
            let entry = state.pending.remove(0);
            pop_frame(stack, frames);
            return call_defer_entry(stack, frames, &entry, module);
        } else {
            // All defers done, complete the original return
            let ret_vals = if !state.heap_gcrefs.is_empty() {
                read_heap_gcrefs(&state.heap_gcrefs, state.value_slots_per_ref)
            } else {
                core::mem::take(&mut state.ret_vals)
            };
            let caller_ret_reg = state.caller_ret_reg;
            let caller_ret_count = state.caller_ret_count;
            *defer_state = None;

            pop_frame(stack, frames);
            return write_return_values(stack, frames, &ret_vals, caller_ret_reg, caller_ret_count);
        }
    }

    // Normal return - check for defers
    let heap_returns = (inst.flags & 0x02) != 0;
    let ret_start = inst.a as usize;
    let ret_count = inst.b as usize;

    // Check if there are any defers for current frame
    let has_defers = defer_stack.last()
        .map_or(false, |e| e.frame_depth == current_frame_depth);

    // For heap_returns, we need special handling
    if heap_returns {
        // inst.a = gcref_start, inst.b = gcref_count, inst.c = value_slots_per_ref
        let gcref_start = inst.a as usize;
        let gcref_count = inst.b as usize;
        let value_slots_per_ref = inst.c as usize;
        let current_bp = frames.last().unwrap().bp;
        
        // Collect GcRefs before popping frame
        let heap_gcrefs: Vec<u64> = (0..gcref_count)
            .map(|i| stack[current_bp + gcref_start + i])
            .collect();
        
        let pending_defers = collect_pending_defers(defer_stack, current_frame_depth, is_error_return);
        
        let frame = pop_frame(stack, frames);
        if frame.is_none() {
            return ExecResult::Done;
        }
        let frame = frame.unwrap();
        
        if !pending_defers.is_empty() {
            // Has defers - save GcRefs and dereference after defers complete
            let mut pending = pending_defers;
            let first_defer = pending.remove(0);
            *defer_state = Some(DeferState {
                pending,
                ret_vals: Vec::new(),
                ret_slot_types: Vec::new(),
                caller_ret_reg: frame.ret_reg,
                caller_ret_count: frame.ret_count as usize,
                is_error_return,
                heap_gcrefs,
                value_slots_per_ref,
            });
            return call_defer_entry(stack, frames, &first_defer, module);
        } else {
            // No defers - dereference GcRefs immediately
            let ret_vals = read_heap_gcrefs(&heap_gcrefs, value_slots_per_ref);
            return write_return_values(stack, frames, &ret_vals, frame.ret_reg, frame.ret_count as usize);
        }
    }

    if !has_defers && ret_count <= 4 {
        // Fast path: no defers AND small return count - use fixed buffer
        let frame = frames.last().unwrap();
        let current_bp = frame.bp;
        let ret_reg = frame.ret_reg;
        let ret_slots = frame.ret_count;
        
        let write_count = (ret_slots as usize).min(ret_count);
        
        // Read return values before pop_frame truncates stack
        let mut ret_buf = [0u64; 4];
        for i in 0..write_count {
            ret_buf[i] = stack[current_bp + ret_start + i];
        }
        
        // Pop frame (truncates stack)
        pop_frame(stack, frames);
        
        return write_return_values(stack, frames, &ret_buf[..write_count], ret_reg, ret_slots as usize);
    }

    // Slow path: has defers - need to save return values in Vec
    let current_bp = frames.last().unwrap().bp;
    let ret_vals: Vec<u64> = (0..ret_count)
        .map(|i| stack[current_bp + ret_start + i])
        .collect();
    
    // Get slot types for return values (for GC scanning)
    let ret_slot_types: Vec<vo_runtime::SlotType> = func.slot_types
        .get(ret_start..ret_start + ret_count)
        .map(|s| s.to_vec())
        .unwrap_or_default();

    let pending_defers = collect_pending_defers(defer_stack, current_frame_depth, is_error_return);

    let frame = pop_frame(stack, frames);
    if frame.is_none() {
        return ExecResult::Done;
    }
    let frame = frame.unwrap();

    if !pending_defers.is_empty() {
        // Has defers to execute - save state and call first defer
        let mut pending = pending_defers;
        let first_defer = pending.remove(0);
        *defer_state = Some(DeferState {
            pending,
            ret_vals,
            ret_slot_types,
            caller_ret_reg: frame.ret_reg,
            caller_ret_count: frame.ret_count as usize,
            is_error_return,
            heap_gcrefs: Vec::new(),
            value_slots_per_ref: 0,
        });
        return call_defer_entry(stack, frames, &first_defer, module);
    }

    // No defers after filtering - normal return
    write_return_values(stack, frames, &ret_vals, frame.ret_reg, frame.ret_count as usize)
}

#[inline]
fn pop_frame(stack: &mut Vec<u64>, frames: &mut Vec<CallFrame>) -> Option<CallFrame> {
    if let Some(frame) = frames.pop() {
        stack.truncate(frame.bp);
        Some(frame)
    } else {
        None
    }
}

/// Write return values to caller's stack or stack start (for trampoline fiber).
/// Returns `ExecResult::Done` if no caller frame, `ExecResult::Return` otherwise.
#[inline]
fn write_return_values(
    stack: &mut Vec<u64>,
    frames: &[CallFrame],
    ret_vals: &[u64],
    ret_reg: u16,
    ret_count: usize,
) -> ExecResult {
    let write_count = ret_count.min(ret_vals.len());
    if frames.is_empty() {
        // Top-level return (trampoline fiber) - write to stack start
        stack.resize(write_count, 0);
        for i in 0..write_count {
            stack[i] = ret_vals[i];
        }
        ExecResult::Done
    } else {
        let caller_bp = frames.last().unwrap().bp;
        let write_end = caller_bp + ret_reg as usize + write_count;
        // Ensure stack is large enough (may have been truncated during defer execution)
        if stack.len() < write_end {
            stack.resize(write_end, 0);
        }
        for i in 0..write_count {
            stack[caller_bp + ret_reg as usize + i] = ret_vals[i];
        }
        ExecResult::Return
    }
}

fn call_defer_entry(
    stack: &mut Vec<u64>,
    frames: &mut Vec<CallFrame>,
    entry: &DeferEntry,
    module: &Module,
) -> ExecResult {
    let func_id = if entry.is_closure {
        closure::func_id(entry.closure)
    } else {
        entry.func_id
    };

    let func = &module.functions[func_id as usize];
    let arg_slots = entry.arg_slots as usize;

    // Allocate space for args (defer functions have no return value we care about)
    let args_start = stack.len();
    stack.resize(args_start + func.local_slots as usize, 0);

    // Copy args from heap to stack
    if !entry.args.is_null() {
        for i in 0..arg_slots {
            let val = unsafe { Gc::read_slot(entry.args, i) };
            stack[args_start + i] = val;
        }
    }

    // For closure call, set closure ref as first slot
    if entry.is_closure {
        stack[args_start] = entry.closure as u64;
    }

    // Push frame for defer function
    frames.push(CallFrame {
        func_id,
        pc: 0,
        bp: args_start,
        ret_reg: 0,  // defer return values are ignored
        ret_count: 0,
    });

    // Return because frames changed (need to refetch frame_ptr in vm loop)
    ExecResult::Return
}
