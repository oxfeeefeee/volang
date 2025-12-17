//! Native implementations for the bytes package.

use gox_vm::gc::{Gc, GcRef};
use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};
use gox_vm::objects::{array, slice};
use gox_vm::types::builtin;

/// Register bytes native functions.
/// GoX implementations: Equal, Compare, HasPrefix, HasSuffix, Contains, TrimPrefix, TrimSuffix
pub fn register(registry: &mut NativeRegistry) {
    // Search (native: search algorithms)
    registry.register("bytes.Index", native_index);
    registry.register("bytes.LastIndex", native_last_index);
    registry.register("bytes.Count", native_count);
    registry.register("bytes.IndexByte", native_index_byte);
    registry.register("bytes.LastIndexByte", native_last_index_byte);
    
    // Transformation (native: Unicode + allocation)
    registry.register("bytes.ToLower", native_to_lower);
    registry.register("bytes.ToUpper", native_to_upper);
    registry.register("bytes.TrimSpace", native_trim_space);
    registry.register("bytes.Trim", native_trim);
    
    // Construction (native: allocation)
    registry.register("bytes.Repeat", native_repeat);
    registry.register("bytes.Join", native_join);
    registry.register("bytes.Split", native_split);
}

// Helper: read slice as bytes
fn read_bytes(slice_ref: GcRef) -> Vec<u8> {
    if slice_ref.is_null() {
        return Vec::new();
    }
    let len = slice::len(slice_ref);
    (0..len).map(|i| slice::get(slice_ref, i) as u8).collect()
}

// Helper: create byte slice from Vec<u8>
fn create_byte_slice(gc: &mut Gc, data: &[u8]) -> GcRef {
    let arr = array::create(gc, builtin::ARRAY, builtin::UINT8, 1, data.len());
    for (i, &b) in data.iter().enumerate() {
        array::set(arr, i, b as u64);
    }
    slice::from_array(gc, builtin::SLICE, arr)
}

// ============ Search ============

fn native_index(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let sep = read_bytes(ctx.arg_ref(1));
    
    if sep.is_empty() {
        ctx.ret_i64(0, 0);
        return NativeResult::Ok(1);
    }
    
    let result = b.windows(sep.len())
        .position(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1);
    ctx.ret_i64(0, result);
    NativeResult::Ok(1)
}

fn native_last_index(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let sep = read_bytes(ctx.arg_ref(1));
    
    if sep.is_empty() {
        ctx.ret_i64(0, b.len() as i64);
        return NativeResult::Ok(1);
    }
    
    let result = b.windows(sep.len())
        .rposition(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1);
    ctx.ret_i64(0, result);
    NativeResult::Ok(1)
}

fn native_count(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let sep = read_bytes(ctx.arg_ref(1));
    
    if sep.is_empty() {
        ctx.ret_i64(0, (b.len() + 1) as i64);
        return NativeResult::Ok(1);
    }
    
    let count = b.windows(sep.len())
        .filter(|w| *w == sep)
        .count();
    ctx.ret_i64(0, count as i64);
    NativeResult::Ok(1)
}

fn native_index_byte(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let c = ctx.arg_i64(1) as u8;
    let result = b.iter().position(|&x| x == c).map(|i| i as i64).unwrap_or(-1);
    ctx.ret_i64(0, result);
    NativeResult::Ok(1)
}

fn native_last_index_byte(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let c = ctx.arg_i64(1) as u8;
    let result = b.iter().rposition(|&x| x == c).map(|i| i as i64).unwrap_or(-1);
    ctx.ret_i64(0, result);
    NativeResult::Ok(1)
}

// ============ Transformation ============

fn native_to_lower(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let result: Vec<u8> = b.iter().map(|&c| c.to_ascii_lowercase()).collect();
    let slice_ref = create_byte_slice(ctx.gc(), &result);
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

fn native_to_upper(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let result: Vec<u8> = b.iter().map(|&c| c.to_ascii_uppercase()).collect();
    let slice_ref = create_byte_slice(ctx.gc(), &result);
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

fn native_trim_space(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let trimmed = String::from_utf8_lossy(&b);
    let result = trimmed.trim().as_bytes();
    let slice_ref = create_byte_slice(ctx.gc(), result);
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

fn native_trim(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let cutset = ctx.arg_str(1);
    
    let s = String::from_utf8_lossy(&b);
    let trimmed = s.trim_matches(|c: char| cutset.contains(c));
    let slice_ref = create_byte_slice(ctx.gc(), trimmed.as_bytes());
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

// ============ Construction ============

fn native_repeat(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let count = ctx.arg_i64(1) as usize;
    
    let result: Vec<u8> = b.iter().cloned().cycle().take(b.len() * count).collect();
    let slice_ref = create_byte_slice(ctx.gc(), &result);
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

fn native_join(ctx: &mut NativeCtx) -> NativeResult {
    let s_slice = ctx.arg_ref(0);
    let sep = read_bytes(ctx.arg_ref(1));
    
    if s_slice.is_null() {
        let slice_ref = create_byte_slice(ctx.gc(), &[]);
        ctx.ret_ref(0, slice_ref);
        return NativeResult::Ok(1);
    }
    
    let s_len = slice::len(s_slice);
    if s_len == 0 {
        let slice_ref = create_byte_slice(ctx.gc(), &[]);
        ctx.ret_ref(0, slice_ref);
        return NativeResult::Ok(1);
    }
    
    // Read all byte slices
    let mut parts: Vec<Vec<u8>> = Vec::with_capacity(s_len);
    for i in 0..s_len {
        let part_ref = slice::get(s_slice, i) as GcRef;
        parts.push(read_bytes(part_ref));
    }
    
    // Join with separator
    let total_len = parts.iter().map(|p| p.len()).sum::<usize>() + sep.len() * (s_len - 1);
    let mut result = Vec::with_capacity(total_len);
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            result.extend_from_slice(&sep);
        }
        result.extend_from_slice(part);
    }
    
    let slice_ref = create_byte_slice(ctx.gc(), &result);
    ctx.ret_ref(0, slice_ref);
    NativeResult::Ok(1)
}

// ============ Splitting ============

fn native_split(ctx: &mut NativeCtx) -> NativeResult {
    let b = read_bytes(ctx.arg_ref(0));
    let sep = read_bytes(ctx.arg_ref(1));
    
    if sep.is_empty() {
        // Split into individual bytes
        let gc = ctx.gc();
        let parts: Vec<GcRef> = b.iter()
            .map(|&byte| create_byte_slice(gc, &[byte]))
            .collect();
        
        // Create result slice
        let arr = array::create(gc, builtin::ARRAY, builtin::SLICE, 1, parts.len());
        for (i, &part) in parts.iter().enumerate() {
            array::set(arr, i, part as u64);
        }
        let result = slice::from_array(gc, builtin::SLICE, arr);
        ctx.ret_ref(0, result);
        return NativeResult::Ok(1);
    }
    
    // Split by separator
    let mut parts: Vec<&[u8]> = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i + sep.len() <= b.len() {
        if &b[i..i + sep.len()] == sep.as_slice() {
            parts.push(&b[start..i]);
            start = i + sep.len();
            i = start;
        } else {
            i += 1;
        }
    }
    parts.push(&b[start..]);
    
    // Create result
    let gc = ctx.gc();
    let arr = array::create(gc, builtin::ARRAY, builtin::SLICE, 1, parts.len());
    for (i, part) in parts.iter().enumerate() {
        let part_slice = create_byte_slice(gc, part);
        array::set(arr, i, part_slice as u64);
    }
    let result = slice::from_array(gc, builtin::SLICE, arr);
    ctx.ret_ref(0, result);
    NativeResult::Ok(1)
}

