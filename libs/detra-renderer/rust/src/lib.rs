//! Detra Renderer - egui-based rendering engine for Detra UI trees.
//! 
//! Uses RenderTree from layout engine - all positions are pre-calculated.

mod render_tree;

use std::cell::RefCell;
use std::collections::HashMap;

use eframe::egui;
use linkme::distributed_slice;
use vo_runtime::ffi::{ExternCallContext, ExternEntryWithContext, ExternResult, EXTERN_TABLE_WITH_CONTEXT};
use vo_runtime::gc::GcRef;
use vo_runtime::objects::string;

use detra_renderable::{RenderTree, Value, ActionCall};

type DetraLayout = unsafe extern "C" fn(usize, f32, f32) -> *const RenderTree;

fn get_detra_layout() -> Option<DetraLayout> {
    #[cfg(unix)]
    unsafe {
        let handle = libc::dlsym(libc::RTLD_DEFAULT, b"detra_layout\0".as_ptr() as *const _);
        if handle.is_null() {
            None
        } else {
            Some(std::mem::transmute::<*mut libc::c_void, DetraLayout>(handle))
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

thread_local! {
    static PENDING_ACTIONS: RefCell<Vec<ActionCall>> = const { RefCell::new(Vec::new()) };
    static CURRENT_ACTION_NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static CURRENT_ACTION_ARGS: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
}

mod theme {
    use eframe::egui::Color32;
    pub const BG_DARK: Color32 = Color32::from_rgb(30, 30, 30);
}

struct DetraApp {
    on_action_closure: GcRef,
    vm: *mut std::ffi::c_void,
    fiber: *mut std::ffi::c_void,
    call_closure_fn: vo_runtime::ffi::ClosureCallFn,
}

impl eframe::App for DetraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        let screen = ctx.screen_rect();
        
        // Get layout from detra (uses CURRENT_TREE internally)
        if let Some(layout_fn) = get_detra_layout() {
            let render_tree = unsafe { layout_fn(0, screen.width(), screen.height()) };
            if !render_tree.is_null() {
                let tree = unsafe { &*render_tree };
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(theme::BG_DARK))
                    .show(ctx, |_ui| {
                        render_tree::render(ctx, tree);
                    });
            } else {
                egui::CentralPanel::default().show(ctx, |ui| { ui.label("No tree available"); });
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| { ui.label("Layout engine not available"); });
        }

        let actions: Vec<ActionCall> = PENDING_ACTIONS.with(|cell| std::mem::take(&mut *cell.borrow_mut()));
        for action in actions {
            self.dispatch_action(&action);
        }
        ctx.request_repaint();
    }
}

impl DetraApp {
    fn dispatch_action(&self, action: &ActionCall) {
        CURRENT_ACTION_NAME.with(|cell| { *cell.borrow_mut() = action.name.clone(); });
        CURRENT_ACTION_ARGS.with(|cell| { *cell.borrow_mut() = action.args.clone(); });
        let args: [u64; 0] = [];
        let mut ret: [u64; 0] = [];
        let _ = (self.call_closure_fn)(self.vm, self.fiber, self.on_action_closure as u64, args.as_ptr(), 0, ret.as_mut_ptr(), 0);
    }
}

fn detra_renderer_run(ctx: &mut ExternCallContext) -> ExternResult {
    let title_ref = ctx.arg_ref(0);
    let title = string::as_str(title_ref).to_string();
    let width = ctx.arg_i64(1) as u32;
    let height = ctx.arg_i64(2) as u32;
    let _resizable = ctx.arg_i64(3) != 0;
    let _vsync = ctx.arg_i64(4) != 0;
    let on_action_closure = ctx.arg_ref(5);
    let vm = ctx.vm_ptr();
    let fiber = ctx.fiber_ptr();
    let call_closure_fn = ctx.closure_call_fn().unwrap();

    let app = DetraApp { on_action_closure, vm, fiber, call_closure_fn };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width as f32, height as f32])
            .with_title(&title),
        ..Default::default()
    };
    let _ = eframe::run_native(&title, options, Box::new(|_cc| Ok(Box::new(app))));
    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_RUN: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer_Run",
    func: detra_renderer_run,
};

fn detra_renderer_pending_action(ctx: &mut ExternCallContext) -> ExternResult {
    let name = CURRENT_ACTION_NAME.with(|cell| cell.borrow().clone());
    ctx.ret_str(0, &name);
    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_PENDING_ACTION: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer_PendingAction",
    func: detra_renderer_pending_action,
};

fn detra_renderer_pending_action_arg(ctx: &mut ExternCallContext) -> ExternResult {
    let key_ref = ctx.arg_ref(0);
    let key = string::as_str(key_ref);
    let value = CURRENT_ACTION_ARGS.with(|cell| {
        let args = cell.borrow();
        match args.get(key) {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    });
    ctx.ret_str(0, &value);
    ExternResult::Ok
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_DETRA_RENDERER_PENDING_ACTION_ARG: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._libs_detra-renderer_PendingActionArg",
    func: detra_renderer_pending_action_arg,
};

fn system_read_file_content(ctx: &mut ExternCallContext) -> ExternResult {
    let path_ref = ctx.arg_ref(0);
    let path = string::as_str(path_ref);
    match std::fs::read_to_string(path) {
        Ok(content) => { ctx.ret_str(0, &content); ExternResult::Ok }
        Err(e) => ExternResult::Panic(format!("failed to read file {}: {}", path, e))
    }
}

#[distributed_slice(EXTERN_TABLE_WITH_CONTEXT)]
static __VO_SYSTEM_READ_FILE_CONTENT: ExternEntryWithContext = ExternEntryWithContext {
    name: ".._vibe-studio_system_ReadFileContent",
    func: system_read_file_content,
};

pub fn link_detra_renderer_externs() {
    let _ = &__VO_DETRA_RENDERER_RUN;
    let _ = &__VO_DETRA_RENDERER_PENDING_ACTION;
    let _ = &__VO_DETRA_RENDERER_PENDING_ACTION_ARG;
    let _ = &__VO_SYSTEM_READ_FILE_CONTENT;
}

vo_ext::export_extensions!();
