//! Native implementations for the errors package.

use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};

pub fn register(registry: &mut NativeRegistry) {
    registry.register("errors.New", native_new);
}

fn native_new(ctx: &mut NativeCtx) -> NativeResult {
    let text = ctx.arg_str(0).to_string();
    // For now, we return the string as the error
    // In a full implementation, we'd create an error object
    ctx.ret_string(0, &text);
    NativeResult::Ok(1)
}

