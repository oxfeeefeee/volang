//! Native implementations for the rand package.

use gox_vm::gc::Gc;
use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};
use gox_vm::objects::{array, slice};
use gox_vm::types::builtin;
use std::sync::atomic::{AtomicU64, Ordering};

// Simple Linear Congruential Generator
static SEED: AtomicU64 = AtomicU64::new(1);

fn next_random() -> u64 {
    let mut seed = SEED.load(Ordering::Relaxed);
    // LCG parameters from Numerical Recipes
    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    SEED.store(seed, Ordering::Relaxed);
    seed
}

pub fn register(registry: &mut NativeRegistry) {
    registry.register("rand.Seed", native_seed);
    registry.register("rand.Int", native_int);
    registry.register("rand.Int64", native_int64);
    registry.register("rand.Intn", native_intn);
    registry.register("rand.Int63n", native_int63n);
    registry.register("rand.Float64", native_float64);
    registry.register("rand.Float32", native_float32);
    registry.register("rand.Perm", native_perm);
}

fn native_seed(ctx: &mut NativeCtx) -> NativeResult {
    let seed = ctx.arg_i64(0) as u64;
    SEED.store(seed, Ordering::Relaxed);
    NativeResult::Ok(0)
}

fn native_int(ctx: &mut NativeCtx) -> NativeResult {
    let val = (next_random() >> 1) as i64;
    ctx.ret_i64(0, val);
    NativeResult::Ok(1)
}

fn native_int64(ctx: &mut NativeCtx) -> NativeResult {
    let val = (next_random() >> 1) as i64;
    ctx.ret_i64(0, val);
    NativeResult::Ok(1)
}

fn native_intn(ctx: &mut NativeCtx) -> NativeResult {
    let n = ctx.arg_i64(0);
    if n <= 0 {
        ctx.ret_i64(0, 0);
        return NativeResult::Ok(1);
    }
    let val = ((next_random() >> 1) as i64) % n;
    ctx.ret_i64(0, val);
    NativeResult::Ok(1)
}

fn native_int63n(ctx: &mut NativeCtx) -> NativeResult {
    let n = ctx.arg_i64(0);
    if n <= 0 {
        ctx.ret_i64(0, 0);
        return NativeResult::Ok(1);
    }
    let val = ((next_random() >> 1) as i64) % n;
    ctx.ret_i64(0, val);
    NativeResult::Ok(1)
}

fn native_float64(ctx: &mut NativeCtx) -> NativeResult {
    let val = (next_random() >> 11) as f64 / (1u64 << 53) as f64;
    ctx.ret_f64(0, val);
    NativeResult::Ok(1)
}

fn native_float32(ctx: &mut NativeCtx) -> NativeResult {
    let val = (next_random() >> 40) as f32 / (1u64 << 24) as f32;
    ctx.ret_f64(0, val as f64); // Return as f64, GoX will handle conversion
    NativeResult::Ok(1)
}

fn native_perm(ctx: &mut NativeCtx) -> NativeResult {
    let n = ctx.arg_i64(0) as usize;
    
    // Create array [0, 1, 2, ..., n-1]
    let gc = ctx.gc();
    let arr = array::create(gc, builtin::ARRAY, builtin::INT, 1, n);
    for i in 0..n {
        array::set(arr, i, i as u64);
    }
    
    // Fisher-Yates shuffle
    for i in (1..n).rev() {
        let j = ((next_random() >> 1) as usize) % (i + 1);
        let a = array::get(arr, i);
        let b = array::get(arr, j);
        array::set(arr, i, b);
        array::set(arr, j, a);
    }
    
    let result = slice::from_array(gc, builtin::SLICE, arr);
    ctx.ret_ref(0, result);
    NativeResult::Ok(1)
}

