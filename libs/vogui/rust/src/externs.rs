//! GUI extern functions implemented with vo-ext macros.

use vo_ext::prelude::*;
use vo_runtime::objects::string;

use crate::{PENDING_HANDLER, start_js_timeout, clear_js_timeout, start_js_interval, clear_js_interval, js_navigate, js_get_current_path};

// =============================================================================
// App Externs
// =============================================================================

#[vo_extern_ctx("gui", "registerEventHandler")]
pub fn register_event_handler(ctx: &mut ExternCallContext) -> ExternResult {
    let handler = ctx.arg_ref(slots::ARG_HANDLER);
    PENDING_HANDLER.with(|s| *s.borrow_mut() = Some(handler));
    ExternResult::Ok
}

#[vo_extern_ctx("gui", "emitRender")]
pub fn emit_render(ctx: &mut ExternCallContext) -> ExternResult {
    let json_ref = ctx.arg_ref(slots::ARG_JSON);
    let json = if json_ref.is_null() { "" } else { string::as_str(json_ref) };
    
    vo_runtime::output::write("__VOGUI__");
    vo_runtime::output::writeln(json);
    
    ExternResult::Ok
}

// =============================================================================
// Timer Externs
// =============================================================================

#[vo_extern_ctx("gui", "startTimeout")]
pub fn start_timeout(ctx: &mut ExternCallContext) -> ExternResult {
    let id = ctx.arg_i64(slots::ARG_ID) as i32;
    let ms = ctx.arg_i64(slots::ARG_MS) as i32;
    start_js_timeout(id, ms);
    ExternResult::Ok
}

#[vo_extern_ctx("gui", "clearTimeout")]
pub fn clear_timeout(ctx: &mut ExternCallContext) -> ExternResult {
    let id = ctx.arg_i64(slots::ARG_ID) as i32;
    clear_js_timeout(id);
    ExternResult::Ok
}

#[vo_extern_ctx("gui", "startInterval")]
pub fn start_interval(ctx: &mut ExternCallContext) -> ExternResult {
    let id = ctx.arg_i64(slots::ARG_ID) as i32;
    let ms = ctx.arg_i64(slots::ARG_MS) as i32;
    start_js_interval(id, ms);
    ExternResult::Ok
}

#[vo_extern_ctx("gui", "clearInterval")]
pub fn clear_interval(ctx: &mut ExternCallContext) -> ExternResult {
    let id = ctx.arg_i64(slots::ARG_ID) as i32;
    clear_js_interval(id);
    ExternResult::Ok
}

// =============================================================================
// Router Externs
// =============================================================================

#[vo_extern_ctx("gui", "navigate")]
pub fn navigate(ctx: &mut ExternCallContext) -> ExternResult {
    let path_ref = ctx.arg_ref(slots::ARG_PATH);
    let path = if path_ref.is_null() { "" } else { string::as_str(path_ref) };
    js_navigate(path);
    ExternResult::Ok
}

#[vo_extern_ctx("gui", "getCurrentPath")]
pub fn get_current_path(ctx: &mut ExternCallContext) -> ExternResult {
    let path = js_get_current_path();
    let gc_ref = string::from_rust_str(ctx.gc(), &path);
    ctx.ret_ref(slots::RET_0, gc_ref);
    ExternResult::Ok
}

// =============================================================================
// Export all entries for registration
// =============================================================================

// Native: use linkme auto-registration (no explicit export needed)
#[cfg(not(target_arch = "wasm32"))]
vo_ext::export_extensions!();

// WASM: use explicit entry list
#[cfg(target_arch = "wasm32")]
vo_ext::export_extensions!(
    __STDLIB_gui_registerEventHandler,
    __STDLIB_gui_emitRender,
    __STDLIB_gui_startTimeout,
    __STDLIB_gui_clearTimeout,
    __STDLIB_gui_startInterval,
    __STDLIB_gui_clearInterval,
    __STDLIB_gui_navigate,
    __STDLIB_gui_getCurrentPath
);

// =============================================================================
// Registration function for vogui
// =============================================================================

use vo_runtime::ffi::ExternRegistry;
use vo_vm::bytecode::ExternDef;

/// Register all GUI extern functions into the provided registry.
/// Native: uses linkme tables (already registered via distributed_slice)
/// WASM: uses explicit entry list
pub fn vo_ext_register(registry: &mut ExternRegistry, externs: &[ExternDef]) {
    // Native: functions are auto-registered via linkme, just need to map IDs
    #[cfg(not(target_arch = "wasm32"))]
    {
        use vo_runtime::ffi::{EXTERN_TABLE, EXTERN_TABLE_WITH_CONTEXT};
        
        for entry in EXTERN_TABLE.iter() {
            for (id, def) in externs.iter().enumerate() {
                if def.name == entry.name {
                    registry.register(id as u32, entry.func);
                    break;
                }
            }
        }
        
        for entry in EXTERN_TABLE_WITH_CONTEXT.iter() {
            for (id, def) in externs.iter().enumerate() {
                if def.name == entry.name {
                    registry.register_with_context(id as u32, entry.func);
                    break;
                }
            }
        }
    }
    
    // WASM: use generated VO_EXT_ENTRIES
    #[cfg(target_arch = "wasm32")]
    {
        for entry in VO_EXT_ENTRIES {
            for (id, def) in externs.iter().enumerate() {
                if def.name == entry.name() {
                    entry.register(registry, id as u32);
                    break;
                }
            }
        }
    }
}
