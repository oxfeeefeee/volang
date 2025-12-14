//! Built-in native functions (len, cap, panic, etc.)

use gox_vm::{NativeCtx, NativeRegistry, GcRef};
use gox_vm::objects::{array, slice, string, map, channel};

/// Register builtin functions.
pub fn register(registry: &mut NativeRegistry) {
    registry.register("len", native_len);
    registry.register("cap", native_cap);
    registry.register("panic", native_panic);
}

fn native_len(ctx: &mut NativeCtx) -> Vec<u64> {
    let val = ctx.arg(0) as GcRef;
    if val.is_null() {
        return vec![0];
    }
    
    // Determine type and get length
    let header = unsafe { &(*val).header };
    let len = match header.type_id {
        14 => string::len(val), // STRING
        15 => array::len(val),  // ARRAY
        16 => slice::len(val),  // SLICE
        17 => map::len(val),    // MAP
        _ => 0,
    };
    
    vec![len as u64]
}

fn native_cap(ctx: &mut NativeCtx) -> Vec<u64> {
    let val = ctx.arg(0) as GcRef;
    if val.is_null() {
        return vec![0];
    }
    
    let header = unsafe { &(*val).header };
    let cap = match header.type_id {
        15 => array::len(val),  // ARRAY - cap = len
        16 => slice::cap(val),  // SLICE
        18 => channel::capacity(val), // CHANNEL
        _ => 0,
    };
    
    vec![cap as u64]
}

fn native_panic(ctx: &mut NativeCtx) -> Vec<u64> {
    let msg_ref = ctx.arg(0) as GcRef;
    let msg = if msg_ref.is_null() {
        "panic".to_string()
    } else {
        ctx.get_string(msg_ref).to_string()
    };
    panic!("{}", msg);
}
