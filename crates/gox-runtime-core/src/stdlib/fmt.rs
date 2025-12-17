//! Formatting core implementations.
//!
//! Pure logic for fmt package functions.

use alloc::string::String;
use alloc::format;
use crate::gc::GcRef;
use crate::objects::string as gox_string;
use gox_common_core::ValueKind;

/// Format a single value to string based on its type.
pub fn format_value(val: u64, type_tag: u8) -> String {
    match ValueKind::from_u8(type_tag) {
        ValueKind::Nil => "nil".into(),
        ValueKind::Bool => {
            if val != 0 { "true".into() } else { "false".into() }
        }
        ValueKind::Int | ValueKind::Int8 | ValueKind::Int16 | 
        ValueKind::Int32 | ValueKind::Int64 => {
            format!("{}", val as i64)
        }
        ValueKind::Uint | ValueKind::Uint8 | ValueKind::Uint16 |
        ValueKind::Uint32 | ValueKind::Uint64 => {
            format!("{}", val)
        }
        ValueKind::Float32 => {
            let f = f32::from_bits(val as u32);
            format!("{}", f)
        }
        ValueKind::Float64 => {
            let f = f64::from_bits(val);
            if f.abs() >= 1e10 || (f != 0.0 && f.abs() < 1e-4) {
                format!("{:e}", f)
            } else {
                format!("{}", f)
            }
        }
        ValueKind::String => {
            let ptr = val as GcRef;
            if ptr.is_null() {
                String::new()
            } else {
                gox_string::as_str(ptr).into()
            }
        }
        ValueKind::Slice => "[...]".into(),
        ValueKind::Array => "[...]".into(),
        ValueKind::Map => "map[...]".into(),
        ValueKind::Struct => "{...}".into(),
        ValueKind::Pointer => "*{...}".into(),
        ValueKind::Interface => "<interface>".into(),
        ValueKind::Channel => "<chan>".into(),
        ValueKind::Closure => "<func>".into(),
    }
}

/// Format multiple values with space separator (like fmt.Println).
pub fn format_args(args: &[(u64, u8)]) -> String {
    let mut output = String::new();
    for (i, (val, tag)) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&format_value(*val, *tag));
    }
    output
}

/// Sprint implementation - format to string.
pub fn sprint(args: &[(u64, u8)]) -> String {
    format_args(args)
}

/// Sprintln implementation - format to string with newline.
pub fn sprintln(args: &[(u64, u8)]) -> String {
    let mut s = format_args(args);
    s.push('\n');
    s
}
