//! fmt package native functions.

use gox_vm::{NativeCtx, NativeRegistry, GcRef};
use gox_vm::ffi::TypeTag;
use gox_vm::objects::string;

/// Register fmt functions.
pub fn register(registry: &mut NativeRegistry) {
    registry.register("fmt.Println", native_println);
    registry.register("fmt.Print", native_print);
    registry.register("fmt.Sprint", native_sprint);
}

fn native_println(ctx: &mut NativeCtx) -> Vec<u64> {
    let output = format_args_with_types(ctx);
    println!("{}", output);
    vec![output.len() as u64]
}

fn native_print(ctx: &mut NativeCtx) -> Vec<u64> {
    let output = format_args_with_types(ctx);
    print!("{}", output);
    vec![output.len() as u64]
}

fn native_sprint(ctx: &mut NativeCtx) -> Vec<u64> {
    let output = format_args_with_types(ctx);
    let str_ref = ctx.new_string(&output);
    vec![str_ref as u64]
}

/// Format arguments using type tags.
/// Args layout: [type0, val0, type1, val1, ...]
fn format_args_with_types(ctx: &NativeCtx) -> String {
    let mut output = String::new();
    let arg_count = ctx.arg_count();
    
    // Each argument is a (type, value) pair
    let pair_count = arg_count / 2;
    
    for i in 0..pair_count {
        if i > 0 {
            output.push(' ');
        }
        let type_tag = TypeTag::from_u8(ctx.arg(i * 2) as u8);
        let value = ctx.arg(i * 2 + 1);
        format_typed_value(&mut output, type_tag, value);
    }
    
    output
}

/// Format a value according to its type tag.
fn format_typed_value(output: &mut String, type_tag: TypeTag, val: u64) {
    match type_tag {
        TypeTag::Nil => output.push_str("nil"),
        TypeTag::Bool => output.push_str(if val != 0 { "true" } else { "false" }),
        TypeTag::Int | TypeTag::Int8 | TypeTag::Int16 | TypeTag::Int32 | TypeTag::Int64 => {
            output.push_str(&format!("{}", val as i64));
        }
        TypeTag::Uint | TypeTag::Uint8 | TypeTag::Uint16 | TypeTag::Uint32 | TypeTag::Uint64 => {
            output.push_str(&format!("{}", val));
        }
        TypeTag::Float32 => {
            let f = f32::from_bits(val as u32);
            output.push_str(&format!("{}", f));
        }
        TypeTag::Float64 => {
            let f = f64::from_bits(val);
            output.push_str(&format!("{}", f));
        }
        TypeTag::String => {
            let ptr = val as GcRef;
            if ptr.is_null() {
                output.push_str("");
            } else {
                output.push_str(string::as_str(ptr));
            }
        }
        TypeTag::Slice => output.push_str("[...]"),
        TypeTag::Map => output.push_str("map[...]"),
        TypeTag::Struct => output.push_str("{...}"),
        TypeTag::Pointer => {
            if val == 0 {
                output.push_str("nil");
            } else {
                output.push_str(&format!("0x{:x}", val));
            }
        }
        TypeTag::Interface => output.push_str("<interface>"),
    }
}
