//! FFI bindings for the bytecode package.

use vo_ext::prelude::*;
use vo_vm::bytecode::Module;
use vo_runtime::stdlib::error_helper::{write_error_to, write_nil_error};

use crate::{format_text, parse_text, serialize, deserialize, save_text, load_text, save_binary, load_binary};

const CODE_IO: isize = 2000;

// Note: bytecode uses runner's module storage, so we share the same handle IDs.
// The module ID passed in is the same as used in runner package.

// We need access to runner's module storage
use std::sync::Mutex;
static MODULES: Mutex<Vec<Option<Module>>> = Mutex::new(Vec::new());

fn store_module(module: Module) -> i64 {
    let mut modules = MODULES.lock().unwrap();
    for (i, slot) in modules.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(module);
            return i as i64;
        }
    }
    let id = modules.len();
    modules.push(Some(module));
    id as i64
}

fn get_module(id: i64) -> Option<Module> {
    let modules = MODULES.lock().unwrap();
    let idx = id as usize;
    if idx < modules.len() {
        modules[idx].clone()
    } else {
        None
    }
}

#[vo_extern_ctx("bytecode", "Format")]
fn bytecode_format(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    
    let text = match get_module(module_id) {
        Some(m) => format_text(&m),
        None => String::new(),
    };
    
    ctx.ret_str(slots::RET_0, &text);
    ExternResult::Ok
}

#[vo_extern_ctx("bytecode", "Parse")]
fn bytecode_parse(ctx: &mut ExternCallContext) -> ExternResult {
    let text = ctx.arg_str(slots::ARG_TEXT).to_string();
    
    match parse_text(&text) {
        Ok(module) => {
            let id = store_module(module);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            write_nil_error(ctx, slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, CODE_IO, &e);
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("bytecode", "SaveText")]
fn bytecode_save_text(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    let module = match get_module(module_id) {
        Some(m) => m,
        None => {
            write_error_to(ctx, slots::RET_0, CODE_IO, "invalid module handle");
            return ExternResult::Ok;
        }
    };
    
    match save_text(&module, &path) {
        Ok(()) => write_nil_error(ctx, slots::RET_0),
        Err(e) => write_error_to(ctx, slots::RET_0, CODE_IO, &e.to_string()),
    }
    ExternResult::Ok
}

#[vo_extern_ctx("bytecode", "LoadText")]
fn bytecode_load_text(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match load_text(&path) {
        Ok(module) => {
            let id = store_module(module);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            write_nil_error(ctx, slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, CODE_IO, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("bytecode", "SaveBinary")]
fn bytecode_save_binary(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    let module = match get_module(module_id) {
        Some(m) => m,
        None => {
            write_error_to(ctx, slots::RET_0, CODE_IO, "invalid module handle");
            return ExternResult::Ok;
        }
    };
    
    match save_binary(&module, &path) {
        Ok(()) => write_nil_error(ctx, slots::RET_0),
        Err(e) => write_error_to(ctx, slots::RET_0, CODE_IO, &e.to_string()),
    }
    ExternResult::Ok
}

#[vo_extern_ctx("bytecode", "LoadBinary")]
fn bytecode_load_binary(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match load_binary(&path) {
        Ok(module) => {
            let id = store_module(module);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            write_nil_error(ctx, slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, CODE_IO, &e.to_string());
        }
    }
    ExternResult::Ok
}

vo_ext::export_extensions!();
