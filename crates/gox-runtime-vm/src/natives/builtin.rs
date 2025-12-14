//! Built-in native functions (len, cap, panic, etc.)

use gox_vm::{NativeCtx, NativeRegistry, GcRef};
use gox_vm::ffi::GoxValue;
use gox_vm::objects::{array, slice, string, map, channel};

/// Register builtin functions.
pub fn register(registry: &mut NativeRegistry) {
    registry.register("len", native_len);
    registry.register("cap", native_cap);
    registry.register("panic", native_panic);
}

fn native_len(_ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let val = match args.first() {
        Some(GoxValue::String(ptr)) | Some(GoxValue::Slice(ptr)) | Some(GoxValue::Map(ptr)) => *ptr,
        _ => return vec![GoxValue::Int(0)],
    };
    
    if val.is_null() {
        return vec![GoxValue::Int(0)];
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
    
    vec![GoxValue::Int(len as i64)]
}

fn native_cap(_ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let val = match args.first() {
        Some(GoxValue::Slice(ptr)) => *ptr,
        _ => return vec![GoxValue::Int(0)],
    };
    
    if val.is_null() {
        return vec![GoxValue::Int(0)];
    }
    
    let header = unsafe { &(*val).header };
    let cap = match header.type_id {
        15 => array::len(val),  // ARRAY - cap = len
        16 => slice::cap(val),  // SLICE
        18 => channel::capacity(val), // CHANNEL
        _ => 0,
    };
    
    vec![GoxValue::Int(cap as i64)]
}

fn native_panic(ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let msg = match args.first() {
        Some(GoxValue::String(ptr)) if !ptr.is_null() => ctx.get_string(*ptr).to_string(),
        _ => "panic".to_string(),
    };
    panic!("{}", msg);
}
