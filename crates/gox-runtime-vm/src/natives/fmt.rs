//! fmt package native functions.

use gox_vm::{NativeCtx, NativeRegistry, GcRef};
use gox_vm::objects::string;

/// Register fmt functions.
pub fn register(registry: &mut NativeRegistry) {
    registry.register("fmt.Println", native_println);
    registry.register("fmt.Print", native_print);
    registry.register("fmt.Sprint", native_sprint);
}

fn native_println(ctx: &mut NativeCtx) -> Vec<u64> {
    let mut output = String::new();
    
    for i in 0..ctx.arg_count() {
        if i > 0 {
            output.push(' ');
        }
        let val = ctx.arg(i);
        format_value(&mut output, val);
    }
    
    println!("{}", output);
    vec![output.len() as u64]
}

fn native_print(ctx: &mut NativeCtx) -> Vec<u64> {
    let mut output = String::new();
    
    for i in 0..ctx.arg_count() {
        if i > 0 {
            output.push(' ');
        }
        let val = ctx.arg(i);
        format_value(&mut output, val);
    }
    
    print!("{}", output);
    vec![output.len() as u64]
}

fn native_sprint(ctx: &mut NativeCtx) -> Vec<u64> {
    let mut output = String::new();
    
    for i in 0..ctx.arg_count() {
        if i > 0 {
            output.push(' ');
        }
        let val = ctx.arg(i);
        format_value(&mut output, val);
    }
    
    let str_ref = ctx.new_string(&output);
    vec![str_ref as u64]
}

fn format_value(output: &mut String, val: u64) {
    // For now, treat all values as i64 unless they look like a pointer
    // In a full implementation, we'd check the type
    if val == 0 {
        output.push_str("0");
    } else if val > 0x1000_0000_0000 {
        // Likely a pointer (GcRef)
        let ptr = val as GcRef;
        if !ptr.is_null() {
            let header = unsafe { &(*ptr).header };
            match header.type_id {
                14 => { // STRING
                    output.push_str(string::as_str(ptr));
                }
                _ => {
                    output.push_str(&format!("0x{:x}", val));
                }
            }
        } else {
            output.push_str("nil");
        }
    } else {
        // Treat as integer
        output.push_str(&format!("{}", val as i64));
    }
}
