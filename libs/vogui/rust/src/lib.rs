//! VoGUI - GUI library for Vo, built on top of vo-web.
//!
//! This crate provides initGuiApp and handleGuiEvent APIs for browser integration.
//!
//! Design: Vo registers an event handler callback. Rust calls it directly via execute_closure_sync.

use std::cell::RefCell;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;

use vo_runtime::gc::GcRef;
use vo_runtime::objects::{closure, string};
use vo_runtime::ffi::{ExternCall, ExternResult, ExternRegistry};
use vo_vm::vm::Vm;
use vo_vm::bytecode::{Module, ExternDef};

// Embed gui.vo at compile time
static GUI_VO: &str = include_str!("../../gui.vo");

// =============================================================================
// Global State
// =============================================================================

pub struct GuiAppState {
    pub vm: Vm,
    pub event_handler: GcRef,  // Closure registered by Vo
}

thread_local! {
    pub static GUI_STATE: RefCell<Option<GuiAppState>> = RefCell::new(None);
    pub static PENDING_HANDLER: RefCell<Option<GcRef>> = RefCell::new(None);
}

// =============================================================================
// Result Type
// =============================================================================

pub struct GuiResult {
    pub status: String,
    pub render_json: String,
    pub error: String,
}

impl GuiResult {
    pub fn ok(render_json: String) -> Self {
        Self { status: "ok".into(), render_json, error: String::new() }
    }
    
    pub fn error(msg: impl Into<String>) -> Self {
        Self { status: "error".into(), render_json: String::new(), error: msg.into() }
    }
    
    pub fn compile_error(msg: impl Into<String>) -> Self {
        Self { status: "compile_error".into(), render_json: String::new(), error: msg.into() }
    }
}

// =============================================================================
// WASM API (exported to JS)
// =============================================================================

#[wasm_bindgen]
pub struct WasmGuiResult {
    status: String,
    render_json: String,
    error: String,
}

#[wasm_bindgen]
impl WasmGuiResult {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String { self.status.clone() }
    
    #[wasm_bindgen(getter, js_name = "renderJson")]
    pub fn render_json(&self) -> String { self.render_json.clone() }
    
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> String { self.error.clone() }
}

impl From<GuiResult> for WasmGuiResult {
    fn from(r: GuiResult) -> Self {
        Self { status: r.status, render_json: r.render_json, error: r.error }
    }
}

/// Initialize a GUI app from source code.
#[wasm_bindgen(js_name = "initGuiApp")]
pub fn init_gui_app(source: &str, filename: Option<String>) -> WasmGuiResult {
    let filename = filename.unwrap_or_else(|| "main.vo".to_string());
    
    // Build stdlib with gui package added
    let mut std_fs = vo_web::build_stdlib_fs();
    std_fs.add_file(PathBuf::from("gui/gui.vo"), GUI_VO.to_string());
    
    // Compile with custom stdlib
    let bytecode = match vo_web::compile_source_with_std_fs(source, &filename, std_fs) {
        Ok(b) => b,
        Err(e) => return GuiResult::compile_error(e).into(),
    };
    
    let module = match Module::deserialize(&bytecode) {
        Ok(m) => m,
        Err(e) => return GuiResult::error(format!("Failed to load bytecode: {:?}", e)).into(),
    };
    
    run_gui_module(module).into()
}

/// Handle a GUI event.
#[wasm_bindgen(js_name = "handleGuiEvent")]
pub fn handle_gui_event(handler_id: i32, payload: &str) -> WasmGuiResult {
    handle_event(handler_id, payload).into()
}

// =============================================================================
// Core API
// =============================================================================

/// Run a compiled GUI module and return initial render.
/// VM runs until Run() returns (after registering event handler).
fn run_gui_module(module: Module) -> GuiResult {
    // Clear previous state
    GUI_STATE.with(|s| *s.borrow_mut() = None);
    PENDING_HANDLER.with(|s| *s.borrow_mut() = None);
    
    // Clear output buffer
    vo_runtime::output::clear_output();
    
    // Create VM
    let mut vm = Vm::new();
    
    // Get extern defs before loading
    let externs = module.externs.clone();
    
    vm.load(module);
    
    // Register GUI extern functions
    register_gui_externs(&mut vm.state.extern_registry, &externs);
    
    // Run until completion (Run() returns after registering handler)
    if let Err(e) = vm.run() {
        return GuiResult::error(format!("{:?}", e));
    }
    
    // Extract render output
    let stdout = vo_runtime::output::take_output();
    let render_json = extract_render_json(&stdout);
    
    // Fail fast
    if render_json.is_empty() {
        return GuiResult::error(format!("No render output. stdout: {}", stdout));
    }
    
    // Get event handler from registerEventHandler
    let event_handler = PENDING_HANDLER.with(|s| s.borrow_mut().take());
    let event_handler = match event_handler {
        Some(h) => h,
        None => return GuiResult::error("registerEventHandler not called"),
    };
    
    // Store state for subsequent events
    GUI_STATE.with(|s| {
        *s.borrow_mut() = Some(GuiAppState { vm, event_handler });
    });
    
    GuiResult::ok(render_json)
}

/// Handle a GUI event and return new render.
/// 1. Calls the registered Vo callback (sends event to channel)
/// 2. Runs VM to let eventLoop goroutine process the event
pub fn handle_event(handler_id: i32, payload: &str) -> GuiResult {
    GUI_STATE.with(|s| {
        let mut state_ref = s.borrow_mut();
        let state = match state_ref.as_mut() {
            Some(st) => st,
            None => return GuiResult::error("GUI app not initialized"),
        };
        
        vo_runtime::output::clear_output();
        
        // Allocate payload string
        let payload_ref = string::from_rust_str(&mut state.vm.state.gc, payload);
        
        // Get func_id and build full args (like closure_call_trampoline does)
        let closure_ref = state.event_handler;
        let func_id = closure::func_id(closure_ref);
        
        let module = state.vm.module().expect("module not set");
        let func_def = &module.functions[func_id as usize];
        
        // Build args: closure_ref (if needed) + handler_id + payload
        let user_args = [handler_id as u64, payload_ref as u64];
        let full_args = vo_vm::vm::helpers::build_closure_args(
            closure_ref as u64,
            closure_ref,
            func_def,
            user_args.as_ptr(),
            user_args.len() as u32,
        );
        
        let mut ret: [u64; 0] = [];
        let success = state.vm.execute_closure_sync(func_id, &full_args, ret.as_mut_ptr(), 0);
        if !success {
            return GuiResult::error("Event handler panicked");
        }
        
        // Run scheduled fibers (eventLoop will process the event)
        if let Err(e) = state.vm.run_scheduled() {
            return GuiResult::error(format!("{:?}", e));
        }
        
        let stdout = vo_runtime::output::take_output();
        let render_json = extract_render_json(&stdout);
        
        GuiResult::ok(render_json)
    })
}

// =============================================================================
// Extern Functions
// =============================================================================

pub fn register_gui_externs(registry: &mut ExternRegistry, externs: &[ExternDef]) {
    for (id, def) in externs.iter().enumerate() {
        match def.name.as_str() {
            "gui_registerEventHandler" => registry.register(id as u32, extern_register_event_handler),
            "gui_emitRender" => registry.register(id as u32, extern_emit_render),
            _ => {}
        }
    }
}

fn extern_register_event_handler(call: &mut ExternCall) -> ExternResult {
    let handler = call.arg_ref(0);
    PENDING_HANDLER.with(|s| *s.borrow_mut() = Some(handler));
    ExternResult::Ok
}

fn extern_emit_render(call: &mut ExternCall) -> ExternResult {
    let json_ref = call.arg_ref(0);
    let json = if json_ref.is_null() { "" } else { string::as_str(json_ref) };
    
    vo_runtime::output::write("__VOGUI__");
    vo_runtime::output::writeln(json);
    
    ExternResult::Ok
}

// =============================================================================
// Helpers
// =============================================================================

fn extract_render_json(stdout: &str) -> String {
    for line in stdout.lines() {
        if let Some(json) = line.strip_prefix("__VOGUI__") {
            return json.to_string();
        }
    }
    String::new()
}
