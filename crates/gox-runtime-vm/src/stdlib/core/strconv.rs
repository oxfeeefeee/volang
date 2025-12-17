//! strconv package native functions.
//!
//! Provides string conversion functions using zero-copy native API.

use gox_vm::{NativeCtx, NativeRegistry, NativeResult};

/// Register strconv functions.
/// GoX implementations: FormatBool, ParseBool (in stdlib/strconv/strconv.gox)
pub fn register(registry: &mut NativeRegistry) {
    registry.register("strconv.Atoi", native_atoi);
    registry.register("strconv.Itoa", native_itoa);
    registry.register("strconv.ParseInt", native_parse_int);
    registry.register("strconv.ParseFloat", native_parse_float);
    registry.register("strconv.FormatInt", native_format_int);
    registry.register("strconv.FormatFloat", native_format_float);
    registry.register("strconv.Quote", native_quote);
}

/// strconv.Atoi(s string) (int, error)
fn native_atoi(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    match s.trim().parse::<i64>() {
        Ok(v) => {
            ctx.ret_i64(0, v);
            ctx.ret_nil(1); // no error
            NativeResult::Ok(2)
        }
        Err(_) => {
            ctx.ret_i64(0, 0);
            // TODO: proper error value
            ctx.ret_i64(1, 1); // error indicator
            NativeResult::Ok(2)
        }
    }
}

/// strconv.Itoa(i int) string
fn native_itoa(ctx: &mut NativeCtx) -> NativeResult {
    let i = ctx.arg_i64(0);
    let result = i.to_string();
    ctx.ret_string(0, &result);
    NativeResult::Ok(1)
}

/// strconv.ParseInt(s string, base, bitSize int) (int, error)
fn native_parse_int(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    let base = ctx.arg_i64(1) as u32;
    let _bit_size = ctx.arg_i64(2);
    
    // Handle base 0 (auto-detect)
    let (actual_base, num_str) = if base == 0 {
        if s.starts_with("0x") || s.starts_with("0X") {
            (16, &s[2..])
        } else if s.starts_with("0o") || s.starts_with("0O") {
            (8, &s[2..])
        } else if s.starts_with("0b") || s.starts_with("0B") {
            (2, &s[2..])
        } else if s.starts_with('0') && s.len() > 1 {
            (8, &s[1..])
        } else {
            (10, s)
        }
    } else {
        (base, s)
    };
    
    match i64::from_str_radix(num_str.trim(), actual_base) {
        Ok(v) => {
            ctx.ret_i64(0, v);
            ctx.ret_nil(1);
            NativeResult::Ok(2)
        }
        Err(_) => {
            ctx.ret_i64(0, 0);
            ctx.ret_i64(1, 1); // error
            NativeResult::Ok(2)
        }
    }
}

/// strconv.ParseFloat(s string, bitSize int) (float64, error)
fn native_parse_float(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    let _bit_size = ctx.arg_i64(1);
    
    match s.trim().parse::<f64>() {
        Ok(v) => {
            ctx.ret_f64(0, v);
            ctx.ret_nil(1);
            NativeResult::Ok(2)
        }
        Err(_) => {
            ctx.ret_f64(0, 0.0);
            ctx.ret_i64(1, 1); // error
            NativeResult::Ok(2)
        }
    }
}

/// strconv.FormatInt(i, base int) string
fn native_format_int(ctx: &mut NativeCtx) -> NativeResult {
    let i = ctx.arg_i64(0);
    let base = ctx.arg_i64(1) as u32;
    
    let result = match base {
        2 => format!("{:b}", i),
        8 => format!("{:o}", i),
        10 => format!("{}", i),
        16 => format!("{:x}", i),
        _ => format!("{}", i), // fallback to base 10
    };
    
    ctx.ret_string(0, &result);
    NativeResult::Ok(1)
}

/// strconv.FormatFloat(f float64, fmt byte, prec, bitSize int) string
fn native_format_float(ctx: &mut NativeCtx) -> NativeResult {
    let f = ctx.arg_f64(0);
    let fmt = ctx.arg_i64(1) as u8 as char;
    let prec = ctx.arg_i64(2);
    let _bit_size = ctx.arg_i64(3);
    
    let result = match fmt {
        'e' | 'E' => {
            if prec < 0 {
                format!("{:e}", f)
            } else {
                format!("{:.prec$e}", f, prec = prec as usize)
            }
        }
        'f' | 'F' => {
            if prec < 0 {
                format!("{}", f)
            } else {
                format!("{:.prec$}", f, prec = prec as usize)
            }
        }
        'g' | 'G' => {
            // General format: use exponential for large/small, decimal otherwise
            if prec < 0 {
                format!("{}", f)
            } else {
                format!("{:.prec$}", f, prec = prec as usize)
            }
        }
        _ => format!("{}", f),
    };
    
    ctx.ret_string(0, &result);
    NativeResult::Ok(1)
}

/// strconv.Quote(s string) string
fn native_quote(ctx: &mut NativeCtx) -> NativeResult {
    let s = ctx.arg_str(0);
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_ascii_graphic() || c == ' ' => result.push(c),
            c => {
                // Unicode escape
                if c as u32 <= 0xFFFF {
                    result.push_str(&format!("\\u{:04x}", c as u32));
                } else {
                    result.push_str(&format!("\\U{:08x}", c as u32));
                }
            }
        }
    }
    
    result.push('"');
    ctx.ret_string(0, &result);
    NativeResult::Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register() {
        let mut registry = NativeRegistry::new();
        register(&mut registry);
        
        assert!(registry.get("strconv.Atoi").is_some());
        assert!(registry.get("strconv.Itoa").is_some());
        assert!(registry.get("strconv.ParseInt").is_some());
    }
}

