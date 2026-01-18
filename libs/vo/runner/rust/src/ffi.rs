//! FFI bindings for the runner package.
//!
//! Exposes compile and run functions to Vo code.

use std::sync::Mutex;
use vo_ext::prelude::*;
use vo_vm::bytecode::Module;
use vo_launcher::{compile_file, compile_source, compile_string, CompileOutput, run_module_with_extensions, run_file_with_mode, RunMode};
use vo_runtime::stdlib::error_helper::write_error_to;

struct StoredModule {
    module: Module,
    source_root: std::path::PathBuf,
    extensions: Vec<vo_runtime::ext_loader::ExtensionManifest>,
}

static MODULES: Mutex<Vec<Option<StoredModule>>> = Mutex::new(Vec::new());

fn store_module(output: CompileOutput) -> i64 {
    let mut modules = MODULES.lock().unwrap();
    // Find an empty slot or append
    for (i, slot) in modules.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(StoredModule {
                module: output.module,
                source_root: output.source_root,
                extensions: output.extensions,
            });
            return i as i64;
        }
    }
    let id = modules.len();
    modules.push(Some(StoredModule {
        module: output.module,
        source_root: output.source_root,
        extensions: output.extensions,
    }));
    id as i64
}

fn take_module(id: i64) -> Option<StoredModule> {
    let mut modules = MODULES.lock().unwrap();
    let idx = id as usize;
    if idx < modules.len() {
        modules[idx].take()
    } else {
        None
    }
}

fn get_module(id: i64) -> Option<StoredModule> {
    let modules = MODULES.lock().unwrap();
    let idx = id as usize;
    if idx < modules.len() {
        modules[idx].as_ref().map(|m| StoredModule {
            module: m.module.clone(),
            source_root: m.source_root.clone(),
            extensions: m.extensions.clone(),
        })
    } else {
        None
    }
}

fn free_module(id: i64) {
    let mut modules = MODULES.lock().unwrap();
    let idx = id as usize;
    if idx < modules.len() {
        modules[idx] = None;
    }
}

// ============ Compile Functions ============

#[vo_extern_ctx("runner", "CompileFile")]
fn runner_compile_file(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match compile_file(&path) {
        Ok(output) => {
            let id = store_module(output);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            ctx.ret_nil_error(slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("runner", "CompileDir")]
fn runner_compile_dir(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match compile_source(&path) {
        Ok(output) => {
            let id = store_module(output);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            ctx.ret_nil_error(slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("runner", "CompileString")]
fn runner_compile_string(ctx: &mut ExternCallContext) -> ExternResult {
    let code = ctx.arg_str(slots::ARG_CODE).to_string();
    
    match compile_string(&code) {
        Ok(output) => {
            let id = store_module(output);
            ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
            ctx.ret_nil_error(slots::RET_1);
        }
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

// ============ Run Functions ============

#[vo_extern_ctx("runner", "Run")]
fn runner_run(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    
    let stored = match get_module(module_id) {
        Some(m) => m,
        None => {
            write_error_to(ctx, slots::RET_0, 0, "invalid module handle");
            return ExternResult::Ok;
        }
    };

    let output = CompileOutput {
        module: stored.module,
        source_root: stored.source_root,
        extensions: stored.extensions,
    };

    match run_module_with_extensions(output, RunMode::Vm, Vec::new()) {
        Ok(()) => ctx.ret_nil_error(slots::RET_0),
        Err(e) => {
            write_error_to(ctx, slots::RET_0, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("runner", "RunJit")]
fn runner_run_jit(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    
    let stored = match get_module(module_id) {
        Some(m) => m,
        None => {
            write_error_to(ctx, slots::RET_0, 0, "invalid module handle");
            return ExternResult::Ok;
        }
    };

    let output = CompileOutput {
        module: stored.module,
        source_root: stored.source_root,
        extensions: stored.extensions,
    };

    match run_module_with_extensions(output, RunMode::Jit, Vec::new()) {
        Ok(()) => ctx.ret_nil_error(slots::RET_0),
        Err(e) => {
            write_error_to(ctx, slots::RET_0, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("runner", "RunFile")]
fn runner_run_file(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match run_file_with_mode(&path, RunMode::Vm) {
        Ok(()) => ctx.ret_nil_error(slots::RET_0),
        Err(e) => {
            write_error_to(ctx, slots::RET_0, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

#[vo_extern_ctx("runner", "RunFileJit")]
fn runner_run_file_jit(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    match run_file_with_mode(&path, RunMode::Jit) {
        Ok(()) => ctx.ret_nil_error(slots::RET_0),
        Err(e) => {
            write_error_to(ctx, slots::RET_0, 0, &e.to_string());
        }
    }
    ExternResult::Ok
}

// ============ Resource Functions ============

#[vo_extern_ctx("runner", "Free")]
fn runner_free(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    free_module(module_id);
    ExternResult::Ok
}

// ============ Info Functions ============

#[vo_extern_ctx("runner", "Name")]
fn runner_name(ctx: &mut ExternCallContext) -> ExternResult {
    let module_id = ctx.arg_any_as_i64(slots::ARG_M);
    
    let name = match get_module(module_id) {
        Some(m) => m.module.name.clone(),
        None => String::new(),
    };
    
    ctx.ret_str(slots::RET_0, &name);
    ExternResult::Ok
}

vo_ext::export_extensions!();
