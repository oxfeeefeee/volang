//! strings package extern functions.

use gox_runtime_core::gc::GcRef;
use crate::extern_dispatch::ExternDispatchFn;

pub fn register(reg: &mut dyn FnMut(&str, ExternDispatchFn)) {
    reg("strings.Index", native_index);
    reg("strings.LastIndex", native_last_index);
    reg("strings.Count", native_count);
    reg("strings.ToLower", native_to_lower);
    reg("strings.ToUpper", native_to_upper);
    reg("strings.TrimSpace", native_trim_space);
    reg("strings.Contains", native_contains);
    reg("strings.HasPrefix", native_has_prefix);
    reg("strings.HasSuffix", native_has_suffix);
}

fn native_index(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    rets[0] = s.find(substr).map(|i| i as i64).unwrap_or(-1) as u64;
    Ok(())
}

fn native_last_index(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    rets[0] = s.rfind(substr).map(|i| i as i64).unwrap_or(-1) as u64;
    Ok(())
}

fn native_count(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    let result = if substr.is_empty() {
        s.chars().count() + 1
    } else {
        s.matches(substr).count()
    };
    rets[0] = result as u64;
    Ok(())
}

fn native_to_lower(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let lower = s.to_lowercase();
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &lower)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_to_upper(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let upper = s.to_uppercase();
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &upper)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_trim_space(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let trimmed = s.trim();
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, trimmed)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_contains(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    rets[0] = s.contains(substr) as u64;
    Ok(())
}

fn native_has_prefix(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let prefix = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    rets[0] = s.starts_with(prefix) as u64;
    Ok(())
}

fn native_has_suffix(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let suffix = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    rets[0] = s.ends_with(suffix) as u64;
    Ok(())
}
