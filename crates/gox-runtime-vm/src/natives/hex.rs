//! Native implementations for the encoding/hex package.

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
    
    let encoded = hex_encode(&src);
    ctx.ret_string(0, &encoded);
    NativeResult::Ok(1)
}

fn native_decode_string(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0).to_string();
    
    if s.is_empty() {
        let gc = ctx.gc();
        let arr = array::create(gc, builtin::ARRAY, builtin::UINT8, 1, 0);
        let result = slice::from_array(gc, builtin::SLICE, arr);
        ctx.ret_ref(0, result);
        return NativeResult::Ok(1);
    }
    
    let decoded = hex_decode(&s);
    
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
    let n = ctx.arg_i64(0);
    ctx.ret_i64(0, n * 2);
    NativeResult::Ok(1)
}

fn native_decoded_len(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_i64(0);
    ctx.ret_i64(0, x / 2);
    NativeResult::Ok(1)
}

fn hex_encode(src: &[u8]) -> String {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(src.len() * 2);
    
    for &b in src {
        result.push(HEX_CHARS[(b >> 4) as usize] as char);
        result.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    
    result
}

fn hex_decode(s: &str) -> Vec<u8> {
    let bytes: Vec<u8> = s.bytes().filter_map(decode_hex_char).collect();
    let mut result = Vec::with_capacity(bytes.len() / 2);
    
    for chunk in bytes.chunks(2) {
        if chunk.len() == 2 {
            result.push((chunk[0] << 4) | chunk[1]);
        }
    }
    
    result
}

fn decode_hex_char(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

