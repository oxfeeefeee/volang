//! Native implementations for the math package.

use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};

/// Register math native functions.
/// GoX implementations: Abs, Max, Min, Dim (in stdlib/math/math.gox)
pub fn register(registry: &mut NativeRegistry) {
    // Basic functions (native: FPU operations)
    registry.register("math.Mod", native_mod);
    registry.register("math.Remainder", native_remainder);
    
    // Power and root functions
    registry.register("math.Sqrt", native_sqrt);
    registry.register("math.Cbrt", native_cbrt);
    registry.register("math.Pow", native_pow);
    registry.register("math.Pow10", native_pow10);
    registry.register("math.Exp", native_exp);
    registry.register("math.Exp2", native_exp2);
    registry.register("math.Expm1", native_expm1);
    registry.register("math.Log", native_log);
    registry.register("math.Log10", native_log10);
    registry.register("math.Log2", native_log2);
    registry.register("math.Log1p", native_log1p);
    
    // Trigonometric functions
    registry.register("math.Sin", native_sin);
    registry.register("math.Cos", native_cos);
    registry.register("math.Tan", native_tan);
    registry.register("math.Asin", native_asin);
    registry.register("math.Acos", native_acos);
    registry.register("math.Atan", native_atan);
    registry.register("math.Atan2", native_atan2);
    registry.register("math.Sinh", native_sinh);
    registry.register("math.Cosh", native_cosh);
    registry.register("math.Tanh", native_tanh);
    
    // Rounding functions
    registry.register("math.Ceil", native_ceil);
    registry.register("math.Floor", native_floor);
    registry.register("math.Trunc", native_trunc);
    registry.register("math.Round", native_round);
    registry.register("math.RoundToEven", native_round_to_even);
    
    // Special functions
    registry.register("math.Hypot", native_hypot);
    registry.register("math.Inf", native_inf);
    registry.register("math.IsInf", native_is_inf);
    registry.register("math.IsNaN", native_is_nan);
    registry.register("math.NaN", native_nan);
    registry.register("math.Signbit", native_signbit);
    registry.register("math.Copysign", native_copysign);
}

// ============ Basic functions ============

fn native_mod(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let y = ctx.arg_f64(1);
    ctx.ret_f64(0, x % y);
    NativeResult::Ok(1)
}

fn native_remainder(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let y = ctx.arg_f64(1);
    // IEEE 754 remainder
    let n = (x / y).round();
    ctx.ret_f64(0, x - n * y);
    NativeResult::Ok(1)
}

// ============ Power and root functions ============

fn native_sqrt(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.sqrt());
    NativeResult::Ok(1)
}

fn native_cbrt(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.cbrt());
    NativeResult::Ok(1)
}

fn native_pow(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let y = ctx.arg_f64(1);
    ctx.ret_f64(0, x.powf(y));
    NativeResult::Ok(1)
}

fn native_pow10(ctx: &mut NativeCtx) -> NativeResult {
    let n = ctx.arg_i64(0);
    ctx.ret_f64(0, 10.0_f64.powi(n as i32));
    NativeResult::Ok(1)
}

fn native_exp(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.exp());
    NativeResult::Ok(1)
}

fn native_exp2(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.exp2());
    NativeResult::Ok(1)
}

fn native_expm1(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.exp_m1());
    NativeResult::Ok(1)
}

fn native_log(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.ln());
    NativeResult::Ok(1)
}

fn native_log10(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.log10());
    NativeResult::Ok(1)
}

fn native_log2(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.log2());
    NativeResult::Ok(1)
}

fn native_log1p(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.ln_1p());
    NativeResult::Ok(1)
}

// ============ Trigonometric functions ============

fn native_sin(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.sin());
    NativeResult::Ok(1)
}

fn native_cos(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.cos());
    NativeResult::Ok(1)
}

fn native_tan(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.tan());
    NativeResult::Ok(1)
}

fn native_asin(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.asin());
    NativeResult::Ok(1)
}

fn native_acos(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.acos());
    NativeResult::Ok(1)
}

fn native_atan(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.atan());
    NativeResult::Ok(1)
}

fn native_atan2(ctx: &mut NativeCtx) -> NativeResult {
    let y = ctx.arg_f64(0);
    let x = ctx.arg_f64(1);
    ctx.ret_f64(0, y.atan2(x));
    NativeResult::Ok(1)
}

fn native_sinh(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.sinh());
    NativeResult::Ok(1)
}

fn native_cosh(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.cosh());
    NativeResult::Ok(1)
}

fn native_tanh(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.tanh());
    NativeResult::Ok(1)
}

// ============ Rounding functions ============

fn native_ceil(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.ceil());
    NativeResult::Ok(1)
}

fn native_floor(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.floor());
    NativeResult::Ok(1)
}

fn native_trunc(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.trunc());
    NativeResult::Ok(1)
}

fn native_round(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_f64(0, x.round());
    NativeResult::Ok(1)
}

fn native_round_to_even(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    // Banker's rounding (round half to even)
    let rounded = x.round();
    let diff = x - rounded;
    if diff.abs() == 0.5 {
        // Round to even
        let trunc = x.trunc();
        if trunc as i64 % 2 == 0 {
            ctx.ret_f64(0, trunc);
        } else {
            ctx.ret_f64(0, trunc + x.signum());
        }
    } else {
        ctx.ret_f64(0, rounded);
    }
    NativeResult::Ok(1)
}

// ============ Special functions ============

fn native_hypot(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let y = ctx.arg_f64(1);
    ctx.ret_f64(0, x.hypot(y));
    NativeResult::Ok(1)
}

fn native_inf(ctx: &mut NativeCtx) -> NativeResult {
    let sign = ctx.arg_i64(0);
    if sign >= 0 {
        ctx.ret_f64(0, f64::INFINITY);
    } else {
        ctx.ret_f64(0, f64::NEG_INFINITY);
    }
    NativeResult::Ok(1)
}

fn native_is_inf(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let sign = ctx.arg_i64(1);
    let result = match sign {
        s if s > 0 => x == f64::INFINITY,
        s if s < 0 => x == f64::NEG_INFINITY,
        _ => x.is_infinite(),
    };
    ctx.ret_bool(0, result);
    NativeResult::Ok(1)
}

fn native_is_nan(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_bool(0, x.is_nan());
    NativeResult::Ok(1)
}

fn native_nan(ctx: &mut NativeCtx) -> NativeResult {
    ctx.ret_f64(0, f64::NAN);
    NativeResult::Ok(1)
}

fn native_signbit(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    ctx.ret_bool(0, x.is_sign_negative());
    NativeResult::Ok(1)
}

fn native_copysign(ctx: &mut NativeCtx) -> NativeResult {
    let x = ctx.arg_f64(0);
    let y = ctx.arg_f64(1);
    ctx.ret_f64(0, x.copysign(y));
    NativeResult::Ok(1)
}

