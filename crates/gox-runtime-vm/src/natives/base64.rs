//! Native implementations for the encoding/base64 package.

use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};
use gox_vm::objects::{array, slice};
use gox_vm::types::builtin;

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn register(registry: &mut NativeRegistry) {
    registry.register("base64.EncodeToString", native_encode_to_string);
    registry.register("base64.DecodeString", native_decode_string);
    registry.register("base64.EncodedLen", native_encoded_len);
    registry.register("base64.DecodedLen", native_decoded_len);
}

fn native_encode_to_string(ctx: &mut NativeCtx) -> NativeResult {
    let src_ref = ctx.arg_ref(0);
    
    if src_ref.is_null() {
        ctx.ret_string(0, "");
        return NativeResult::Ok(1);
    }
    
    let len = slice::len(src_ref);
    let src: Vec<u8> = (0..len).map(|i| slice::get(src_ref, i) as u8).collect();
    
    let encoded = base64_encode(&src);
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
    
    let decoded = base64_decode(&s);
    
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
    let n = ctx.arg_i64(0) as usize;
    let len = (n + 2) / 3 * 4;
    ctx.ret_i64(0, len as i64);
    NativeResult::Ok(1)
}

fn native_decoded_len(ctx: &mut NativeCtx) -> NativeResult {
    let n = ctx.arg_i64(0) as usize;
    let len = n / 4 * 3;
    ctx.ret_i64(0, len as i64);
    NativeResult::Ok(1)
}

fn base64_encode(src: &[u8]) -> String {
    let mut result = String::with_capacity((src.len() + 2) / 3 * 4);
    
    for chunk in src.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        
        result.push(BASE64_CHARS[b0 >> 2] as char);
        result.push(BASE64_CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

fn base64_decode(s: &str) -> Vec<u8> {
    let s = s.trim_end_matches('=');
    let mut result = Vec::with_capacity(s.len() * 3 / 4);
    
    let chars: Vec<u8> = s.bytes()
        .filter_map(|c| decode_char(c))
        .collect();
    
    for chunk in chars.chunks(4) {
        if chunk.len() >= 2 {
            result.push((chunk[0] << 2) | (chunk[1] >> 4));
        }
        if chunk.len() >= 3 {
            result.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if chunk.len() >= 4 {
            result.push((chunk[2] << 6) | chunk[3]);
        }
    }
    
    result
}

fn decode_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

