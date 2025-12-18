//! fmt package extern functions.

use crate::extern_dispatch::ExternDispatchFn;

pub fn register(reg: &mut dyn FnMut(&str, ExternDispatchFn)) {
    reg("fmt.Println", native_println);
    reg("fmt.Print", native_print);
    reg("fmt.Sprint", native_sprint);
}

fn native_println(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let mut output = String::new();
    for (i, &arg) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&format!("{}", arg));
    }
    println!("{}", output);
    rets[0] = output.len() as u64;
    rets[1] = 0;
    Ok(())
}

fn native_print(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let mut output = String::new();
    for (i, &arg) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&format!("{}", arg));
    }
    print!("{}", output);
    rets[0] = output.len() as u64;
    rets[1] = 0;
    Ok(())
}

fn native_sprint(args: &[u64], rets: &mut [u64]) -> Result<(), String> {
    let mut output = String::new();
    for (i, &arg) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&format!("{}", arg));
    }
    let result = crate::gc_global::with_gc(|gc| {
        gox_runtime_core::objects::string::from_rust_str(gc, 1, &output)
    });
    rets[0] = result as u64;
    Ok(())
}
