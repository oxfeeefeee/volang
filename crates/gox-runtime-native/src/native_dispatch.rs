//! Native function dispatch for JIT/AOT.
//!
//! Provides a uniform C ABI interface for calling native functions from JIT-compiled code.
//! Instead of requiring individual C ABI wrappers for each native function, this module
//! provides a single dispatch function that routes calls to the appropriate implementation.

use std::collections::HashMap;
use std::sync::OnceLock;
use gox_runtime_core::gc::GcRef;

/// Native function type for JIT dispatch.
/// Takes args array and writes to rets array.
pub type NativeDispatchFn = fn(args: &[u64], rets: &mut [u64]) -> Result<(), String>;

/// Registry of native functions for JIT dispatch.
struct NativeDispatchRegistry {
    funcs: HashMap<String, NativeDispatchFn>,
    id_to_name: Vec<String>,
}

impl NativeDispatchRegistry {
    fn new() -> Self {
        Self {
            funcs: HashMap::new(),
            id_to_name: Vec::new(),
        }
    }
    
    fn register(&mut self, name: &str, func: NativeDispatchFn) {
        self.funcs.insert(name.to_string(), func);
    }
    
    fn get(&self, name: &str) -> Option<NativeDispatchFn> {
        self.funcs.get(name).copied()
    }
}

static REGISTRY: OnceLock<NativeDispatchRegistry> = OnceLock::new();

fn get_registry() -> &'static NativeDispatchRegistry {
    REGISTRY.get_or_init(|| {
        let mut registry = NativeDispatchRegistry::new();
        register_natives(&mut registry);
        registry
    })
}

/// Initialize native ID to name mapping from bytecode module.
/// Must be called before any native calls.
pub fn init_native_names(names: Vec<String>) {
    // For now, we just verify natives exist
    let registry = get_registry();
    for name in &names {
        if registry.get(name).is_none() {
            eprintln!("WARNING: native function not registered for JIT: {}", name);
        }
    }
}

/// Dispatch a native function call.
/// 
/// # Safety
/// Caller must ensure args and rets point to valid memory with correct sizes.
#[no_mangle]
pub unsafe extern "C" fn gox_native_call(
    native_name_ptr: *const u8,
    native_name_len: usize,
    args: *const u64,
    arg_count: usize,
    rets: *mut u64,
    ret_count: usize,
) -> i32 {
    // Convert name from C string
    let name_slice = std::slice::from_raw_parts(native_name_ptr, native_name_len);
    let name = match std::str::from_utf8(name_slice) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let registry = get_registry();
    let func = match registry.get(name) {
        Some(f) => f,
        None => {
            eprintln!("Native function not found: {}", name);
            return -2;
        }
    };
    
    let args_slice = std::slice::from_raw_parts(args, arg_count);
    let rets_slice = std::slice::from_raw_parts_mut(rets, ret_count);
    
    match func(args_slice, rets_slice) {
        Ok(()) => 0,
        Err(msg) => {
            eprintln!("Native function error: {}", msg);
            -3
        }
    }
}

/// Register all native functions.
fn register_natives(registry: &mut NativeDispatchRegistry) {
    // === strings package ===
    registry.register("strings.Index", native_strings_index);
    registry.register("strings.LastIndex", native_strings_last_index);
    registry.register("strings.Count", native_strings_count);
    registry.register("strings.ToLower", native_strings_to_lower);
    registry.register("strings.ToUpper", native_strings_to_upper);
    registry.register("strings.TrimSpace", native_strings_trim_space);
    registry.register("strings.Contains", native_strings_contains);
    registry.register("strings.HasPrefix", native_strings_has_prefix);
    registry.register("strings.HasSuffix", native_strings_has_suffix);
    
    // Add more as needed...
}

// === strings implementations ===

fn native_strings_index(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.Index: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    
    let result = s.find(substr).map(|i| i as i64).unwrap_or(-1);
    rets[0] = result as u64;
    Ok(())
}

fn native_strings_last_index(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.LastIndex: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    
    let result = s.rfind(substr).map(|i| i as i64).unwrap_or(-1);
    rets[0] = result as u64;
    Ok(())
}

fn native_strings_count(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.Count: wrong arg count".to_string());
    }
    
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

fn native_strings_to_lower(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.is_empty() || rets.is_empty() {
        return Err("strings.ToLower: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let lower = s.to_lowercase();
    
    // Allocate new string using global GC (type_id 1 = STRING)
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &lower)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_strings_to_upper(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.is_empty() || rets.is_empty() {
        return Err("strings.ToUpper: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let upper = s.to_uppercase();
    
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &upper)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_strings_trim_space(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.is_empty() || rets.is_empty() {
        return Err("strings.TrimSpace: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let trimmed = s.trim();
    
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, trimmed)
    });
    rets[0] = result as u64;
    Ok(())
}

fn native_strings_contains(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.Contains: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let substr = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    
    rets[0] = s.contains(substr) as u64;
    Ok(())
}

fn native_strings_has_prefix(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.HasPrefix: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let prefix = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    
    rets[0] = s.starts_with(prefix) as u64;
    Ok(())
}

fn native_strings_has_suffix(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    if args.len() < 2 || rets.is_empty() {
        return Err("strings.HasSuffix: wrong arg count".to_string());
    }
    
    let s = gox_runtime_core::objects::string::as_str(args[0] as GcRef);
    let suffix = gox_runtime_core::objects::string::as_str(args[1] as GcRef);
    
    rets[0] = s.ends_with(suffix) as u64;
    Ok(())
}
