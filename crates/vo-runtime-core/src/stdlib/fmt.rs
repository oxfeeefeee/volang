//! fmt package native function implementations.
//!
//! Provides print and format functions.

use vo_ffi_macro::vo_extern_std;

#[allow(unused_imports)]
use crate::gc::GcRef;
#[allow(unused_imports)]
use crate::objects::string;
#[allow(unused_imports)]
use vo_common_core::types::ValueKind;

/// fmt.Print - print values without newline
#[vo_extern_std("fmt", "Print")]
fn print(s: &str) -> i64 {
    print!("{}", s);
    s.len() as i64
}

/// fmt.Println - print values with newline
#[vo_extern_std("fmt", "Println")]
fn println(s: &str) -> i64 {
    println!("{}", s);
    s.len() as i64 + 1
}

/// fmt.Sprint - format values to string
#[vo_extern_std("fmt", "Sprint")]
fn sprint(s: &str) -> String {
    s.to_string()
}

// ==================== Helper for formatting any value ====================

/// Format a value based on its kind.
#[cfg(feature = "std")]
pub fn format_value(val: u64, kind: ValueKind) -> String {
    use alloc::string::ToString;
    
    match kind {
        ValueKind::Void => "nil".to_string(),
        ValueKind::Bool => if val != 0 { "true" } else { "false" }.to_string(),
        ValueKind::Int | ValueKind::Int64 => (val as i64).to_string(),
        ValueKind::Int8 => (val as i8).to_string(),
        ValueKind::Int16 => (val as i16).to_string(),
        ValueKind::Int32 => (val as i32).to_string(),
        ValueKind::Uint | ValueKind::Uint64 => val.to_string(),
        ValueKind::Uint8 => (val as u8).to_string(),
        ValueKind::Uint16 => (val as u16).to_string(),
        ValueKind::Uint32 => (val as u32).to_string(),
        ValueKind::Float32 => f32::from_bits(val as u32).to_string(),
        ValueKind::Float64 => f64::from_bits(val).to_string(),
        ValueKind::String => {
            let ptr = val as GcRef;
            if ptr.is_null() {
                "\"\"".to_string()
            } else {
                format!("\"{}\"", string::as_str(ptr))
            }
        }
        ValueKind::Slice => "[...]".to_string(),
        ValueKind::Array => "[...]".to_string(),
        ValueKind::Map => "map[...]".to_string(),
        ValueKind::Struct => "{...}".to_string(),
        ValueKind::Pointer => format!("0x{:x}", val),
        ValueKind::Interface => "<interface>".to_string(),
        ValueKind::Channel => "<chan>".to_string(),
        ValueKind::Closure => "<func>".to_string(),
        ValueKind::FuncPtr => "<func>".to_string(),
    }
}

#[cfg(not(feature = "std"))]
pub fn format_value(_val: u64, _kind: ValueKind) -> String {
    alloc::string::String::from("<value>")
}

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate alloc;
