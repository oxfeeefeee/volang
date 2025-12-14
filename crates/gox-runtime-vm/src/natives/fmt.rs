//! fmt package native functions.

use gox_vm::{NativeCtx, NativeRegistry};
use gox_vm::ffi::GoxValue;

/// Register fmt functions.
pub fn register(registry: &mut NativeRegistry) {
    registry.register("fmt.Println", native_println);
    registry.register("fmt.Print", native_print);
    registry.register("fmt.Sprint", native_sprint);
}

fn native_println(_ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let output = format_args(&args);
    println!("{}", output);
    vec![GoxValue::Int(output.len() as i64)]
}

fn native_print(_ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let output = format_args(&args);
    print!("{}", output);
    vec![GoxValue::Int(output.len() as i64)]
}

fn native_sprint(ctx: &mut NativeCtx, args: Vec<GoxValue>) -> Vec<GoxValue> {
    let output = format_args(&args);
    let str_ref = ctx.new_string(&output);
    vec![GoxValue::String(str_ref)]
}

/// Format arguments with spaces between them.
fn format_args(args: &[GoxValue]) -> String {
    let mut output = String::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&arg.format());
    }
    output
}
