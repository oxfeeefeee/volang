//! VM bindings for the encoding/hex package.
//!
//! All logic is in gox-runtime-core/src/stdlib/hex.rs

use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};
use gox_vm::objects::{array, slice};
use gox_vm::types::builtin;

pub fn register(registry: &mut NativeRegistry) {
    registry.register("hex.EncodeToString", native_encode_to_string);
    registry.register("hex.DecodeString", native_decode_string);
    registry.register("hex.EncodedLen", native_encoded_len);
    registry.register("hex.DecodedLen", native_decoded_len);
}

fn native_encode_to_string(ctx: &mut NativeCtx) -> NativeResult {
    let src_ref = ctx.arg_ref(0);
    if src_ref.is_null() {
        ctx.ret_string(0, "");
        return NativeResult::Ok(1);
    }
    
    let len = slice::len(src_ref);
    let src: Vec<u8> = (0..len).map(|i| slice::get(src_ref, i) as u8).collect();
    let encoded = gox_runtime_core::stdlib::hex::encode_to_string(&src);
    ctx.ret_string(0, &encoded);
    NativeResult::Ok(1)
}

fn native_decode_string(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    let decoded = gox_runtime_core::stdlib::hex::decode_string(s);
    
    let gc = ctx.gc();
    let arr = array::create(gc, builtin::ARRAY, builtin::UINT8, 1, decoded.len());
    for (i, &b) in decoded.iter().enumerate() {
        array::set(arr, i, b as u64);
    }
    let result = slice::from_array(gc, builtin::SLICE, arr);
    ctx.ret_ref(0, result);
    NativeResult::Ok(1)
}

fn native_encoded_len(ctx: &mut NativeCtx) -> NativeResult {
    ctx.ret_i64(0, gox_runtime_core::stdlib::hex::encoded_len(ctx.arg_i64(0)));
    NativeResult::Ok(1)
}

fn native_decoded_len(ctx: &mut NativeCtx) -> NativeResult {
    ctx.ret_i64(0, gox_runtime_core::stdlib::hex::decoded_len(ctx.arg_i64(0)));
    NativeResult::Ok(1)
}

