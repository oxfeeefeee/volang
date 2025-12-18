//! math package extern functions.

use crate::extern_dispatch::ExternDispatchFn;

fn f64_from_u64(v: u64) -> f64 { f64::from_bits(v) }
fn u64_from_f64(v: f64) -> u64 { v.to_bits() }

pub fn register(reg: &mut dyn FnMut(&str, ExternDispatchFn)) {
    // Basic
    reg("math.Mod", native_mod);
    reg("math.Remainder", native_remainder);
    // Power/root
    reg("math.Sqrt", native_sqrt);
    reg("math.Cbrt", native_cbrt);
    reg("math.Pow", native_pow);
    reg("math.Pow10", native_pow10);
    reg("math.Exp", native_exp);
    reg("math.Exp2", native_exp2);
    reg("math.Log", native_log);
    reg("math.Log10", native_log10);
    reg("math.Log2", native_log2);
    // Trig
    reg("math.Sin", native_sin);
    reg("math.Cos", native_cos);
    reg("math.Tan", native_tan);
    reg("math.Asin", native_asin);
    reg("math.Acos", native_acos);
    reg("math.Atan", native_atan);
    reg("math.Atan2", native_atan2);
    reg("math.Sinh", native_sinh);
    reg("math.Cosh", native_cosh);
    reg("math.Tanh", native_tanh);
    // Rounding
    reg("math.Ceil", native_ceil);
    reg("math.Floor", native_floor);
    reg("math.Trunc", native_trunc);
    reg("math.Round", native_round);
    // Special
    reg("math.Hypot", native_hypot);
    reg("math.Inf", native_inf);
    reg("math.IsInf", native_is_inf);
    reg("math.IsNaN", native_is_nan);
    reg("math.NaN", native_nan);
    reg("math.Signbit", native_signbit);
    reg("math.Copysign", native_copysign);
}

fn native_mod(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let y = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(x % y);
    Ok(())
}

fn native_remainder(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let y = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(x - (x / y).round() * y);
    Ok(())
}

fn native_sqrt(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).sqrt());
    Ok(())
}

fn native_cbrt(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).cbrt());
    Ok(())
}

fn native_pow(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let y = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(x.powf(y));
    Ok(())
}

fn native_pow10(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let n = args[0] as i64;
    rets[0] = u64_from_f64(10.0_f64.powi(n as i32));
    Ok(())
}

fn native_exp(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).exp());
    Ok(())
}

fn native_exp2(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).exp2());
    Ok(())
}

fn native_log(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).ln());
    Ok(())
}

fn native_log10(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).log10());
    Ok(())
}

fn native_log2(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).log2());
    Ok(())
}

fn native_sin(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).sin());
    Ok(())
}

fn native_cos(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).cos());
    Ok(())
}

fn native_tan(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).tan());
    Ok(())
}

fn native_asin(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).asin());
    Ok(())
}

fn native_acos(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).acos());
    Ok(())
}

fn native_atan(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).atan());
    Ok(())
}

fn native_atan2(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let y = f64_from_u64(args[0]);
    let x = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(y.atan2(x));
    Ok(())
}

fn native_sinh(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).sinh());
    Ok(())
}

fn native_cosh(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).cosh());
    Ok(())
}

fn native_tanh(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).tanh());
    Ok(())
}

fn native_ceil(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).ceil());
    Ok(())
}

fn native_floor(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).floor());
    Ok(())
}

fn native_trunc(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).trunc());
    Ok(())
}

fn native_round(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64_from_u64(args[0]).round());
    Ok(())
}

fn native_hypot(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let y = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(x.hypot(y));
    Ok(())
}

fn native_inf(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let sign = args[0] as i64;
    let inf = if sign >= 0 { f64::INFINITY } else { f64::NEG_INFINITY };
    rets[0] = u64_from_f64(inf);
    Ok(())
}

fn native_is_inf(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let sign = args[1] as i64;
    let result = match sign {
        0 => x.is_infinite(),
        s if s > 0 => x == f64::INFINITY,
        _ => x == f64::NEG_INFINITY,
    };
    rets[0] = result as u64;
    Ok(())
}

fn native_is_nan(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = f64_from_u64(args[0]).is_nan() as u64;
    Ok(())
}

fn native_nan(_args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = u64_from_f64(f64::NAN);
    Ok(())
}

fn native_signbit(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    rets[0] = f64_from_u64(args[0]).is_sign_negative() as u64;
    Ok(())
}

fn native_copysign(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let x = f64_from_u64(args[0]);
    let y = f64_from_u64(args[1]);
    rets[0] = u64_from_f64(x.copysign(y));
    Ok(())
}
