//! strconv package extern functions.

use gox_runtime_core::gc::GcRef;
use crate::extern_dispatch::ExternDispatchFn;

fn f64_from_u64(v: u64) -> f64 { f64::from_bits(v) }
fn u64_from_f64(v: f64) -> u64 { v.to_bits() }

pub fn register(reg: &mut dyn FnMut(&str, ExternDispatchFn)) {
    reg("strconv.Atoi", native_atoi);
    reg("strconv.Itoa", native_itoa);
    reg("strconv.ParseInt", native_parse_int);
    reg("strconv.ParseFloat", native_parse_float);
    reg("strconv.FormatInt", native_format_int);
    reg("strconv.FormatFloat", native_format_float);
}

fn native_atoi(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    match s.parse::<i64>() {
        Ok(n) => {
            rets[0] = n as u64;
            rets[1] = 0; // nil error
        }
        Err(_) => {
            rets[0] = 0;
            rets[1] = 1; // error (non-nil placeholder)
        }
    }
    Ok(())
}

fn native_itoa(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let n = args[0] as i64;
    let s = n.to_string();
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &s)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_parse_int(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let base = args[1] as i64;
    let _bitsize = args[2] as i64;
    
    let result = if base == 0 {
        // Auto-detect base
        if s.starts_with("0x") || s.starts_with("0X") {
            i64::from_str_radix(&s[2..], 16)
        } else if s.starts_with("0b") || s.starts_with("0B") {
            i64::from_str_radix(&s[2..], 2)
        } else if s.starts_with("0o") || s.starts_with("0O") {
            i64::from_str_radix(&s[2..], 8)
        } else if s.starts_with('0') && s.len() > 1 {
            i64::from_str_radix(&s[1..], 8)
        } else {
            s.parse::<i64>()
        }
    } else {
        i64::from_str_radix(s, base as u32)
    };
    
    match result {
        Ok(n) => {
            rets[0] = n as u64;
            rets[1] = 0;
        }
        Err(_) => {
            rets[0] = 0;
            rets[1] = 1;
        }
    }
    Ok(())
}

fn native_parse_float(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let _bitsize = args[1] as i64;
    
    match s.parse::<f64>() {
        Ok(f) => {
            rets[0] = u64_from_f64(f);
            rets[1] = 0;
        }
        Err(_) => {
            rets[0] = 0;
            rets[1] = 1;
        }
    }
    Ok(())
}

fn native_format_int(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let n = args[0] as i64;
    let base = args[1] as i64;
    
    let s = match base {
        2 => format!("{:b}", n),
        8 => format!("{:o}", n),
        10 => format!("{}", n),
        16 => format!("{:x}", n),
        _ => format!("{}", n),
    };
    
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &s)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_format_float(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let f = f64_from_u64(args[0]);
    let fmt_char = args[1] as u8 as char;
    let prec = args[2] as i64;
    let _bitsize = args[3] as i64;
    
    let s = match fmt_char {
        'e' | 'E' => format!("{:.prec$e}", f, prec = prec as usize),
        'f' | 'F' => format!("{:.prec$}", f, prec = prec as usize),
        'g' | 'G' => {
            if prec < 0 {
                format!("{}", f)
            } else {
                format!("{:.prec$}", f, prec = prec as usize)
            }
        }
        _ => format!("{}", f),
    };
    
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &s)
    });
    rets[0] = result as u64;
    Ok(())
}
