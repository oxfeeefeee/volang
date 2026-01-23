//! Defer and go statement compilation.

use vo_runtime::SlotType;
use vo_vm::instruction::Opcode;

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::func::FuncBuilder;
use crate::type_info::TypeInfoWrapper;

/// Compile defer statement
/// DeferPush instruction format:
/// - a: func_id (flags bit 0 = 0) or closure_reg (flags bit 0 = 1)
/// - b: arg_start
/// - c: arg_slots
/// - flags bit 0: is_closure
pub(crate) fn compile_defer(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    compile_defer_impl(call, ctx, func, info, false)
}

/// Compile errdefer statement
pub(crate) fn compile_errdefer(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    compile_defer_impl(call, ctx, func, info, true)
}

fn compile_defer_impl(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
    is_errdefer: bool,
) -> Result<(), CodegenError> {
    use vo_syntax::ast::ExprKind;
    
    let opcode = if is_errdefer { Opcode::ErrDeferPush } else { Opcode::DeferPush };
    
    let ExprKind::Call(call_expr) = &call.kind else {
        return Err(CodegenError::UnsupportedStmt("defer requires a call expression".to_string()));
    };
    
    // Method call (e.g., res.close())
    if let ExprKind::Selector(sel) = &call_expr.func.kind {
        return compile_defer_method_call(call_expr, sel, opcode, ctx, func, info);
    }
    
    // Regular function call
    if let ExprKind::Ident(ident) = &call_expr.func.kind {
        // Use ObjKey for consistency
        let obj_key = info.get_use(ident);
        if let Some(func_idx) = ctx.get_func_by_objkey(obj_key) {
            let (args_start, total_arg_slots) = compile_defer_args_with_types(call_expr, ctx, func, info)?;
            emit_defer_func(opcode, func_idx, args_start, total_arg_slots, func);
            return Ok(());
        }
    }
    
    // Closure call (local variable or generic expression)
    // Use compile_defer_args_with_types to properly handle interface conversion
    let closure_reg = crate::expr::compile_expr(&call_expr.func, ctx, func, info)?;
    let (args_start, total_arg_slots) = compile_defer_args_with_types(call_expr, ctx, func, info)?;
    emit_defer_closure(opcode, closure_reg, args_start, total_arg_slots, func);
    Ok(())
}

fn compile_defer_args_with_types(
    call_expr: &vo_syntax::ast::CallExpr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(u16, u16), CodegenError> {
    let func_type = info.expr_type(call_expr.func.id);
    let param_types = info.func_param_types(func_type);
    let is_variadic = info.is_variadic(func_type);
    
    // Use calc_method_arg_slots to handle variadic packing (returns 1 slot for packed slice)
    let total_arg_slots = crate::expr::call::calc_method_arg_slots(call_expr, &param_types, is_variadic, info);
    let args_start = func.alloc_args(total_arg_slots);
    // Use compile_method_args to handle variadic packing
    crate::expr::call::compile_method_args(call_expr, &param_types, is_variadic, args_start, ctx, func, info)?;
    
    Ok((args_start, total_arg_slots))
}

#[inline]
fn emit_defer_func(opcode: Opcode, func_idx: u32, args_start: u16, arg_slots: u16, func: &mut FuncBuilder) {
    let (func_id_low, func_id_high) = crate::type_info::encode_func_id(func_idx);
    func.emit_with_flags(opcode, func_id_high << 1, func_id_low, args_start, arg_slots);
}

#[inline]
fn emit_defer_closure(opcode: Opcode, closure_reg: u16, args_start: u16, arg_slots: u16, func: &mut FuncBuilder) {
    func.emit_with_flags(opcode, 1, closure_reg, args_start, arg_slots);
}

fn compile_defer_method_call(
    call_expr: &vo_syntax::ast::CallExpr,
    sel: &vo_syntax::ast::SelectorExpr,
    opcode: Opcode,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    use vo_syntax::ast::ExprKind;
    use crate::embed::MethodDispatch;
    
    let recv_type = info.expr_type(sel.expr.id);
    let method_name = info.project.interner.resolve(sel.sel.symbol)
        .ok_or_else(|| CodegenError::Internal("cannot resolve method name".to_string()))?;
    let method_sym = sel.sel.symbol;
    
    let selection = info.get_selection(call_expr.func.id);
    let is_interface_recv = info.is_interface(recv_type);
    
    let call_info = crate::embed::resolve_method_call(
        recv_type,
        method_name,
        method_sym,
        selection,
        is_interface_recv,
        ctx,
        &info.project.tc_objs,
        &info.project.interner,
    ).ok_or_else(|| CodegenError::UnsupportedExpr(format!("method {} not found", method_name)))?;
    
    match &call_info.dispatch {
        MethodDispatch::Static { func_id, expects_ptr_recv } => {
            // Static dispatch - compile receiver and args, emit DeferPush
            let base_type = if call_info.recv_is_pointer { info.pointer_base(recv_type) } else { recv_type };
            let actual_recv_type = call_info.actual_recv_type(base_type);
            let recv_storage = match &sel.expr.kind {
                ExprKind::Ident(ident) => func.lookup_local(ident.symbol).map(|l| l.storage),
                _ => None,
            };
            
            let method_type = info.expr_type(call_expr.func.id);
            let param_types = info.func_param_types(method_type);
            let is_variadic = info.is_variadic(method_type);
            
            let recv_slots = if *expects_ptr_recv { 1 } else { info.type_slot_count(actual_recv_type) };
            let other_arg_slots = crate::expr::call::calc_method_arg_slots(call_expr, &param_types, is_variadic, info);
            let total_arg_slots = recv_slots + other_arg_slots;
            let args_start = func.alloc_args(total_arg_slots);
            
            crate::expr::emit_receiver(
                &sel.expr, args_start, recv_type, recv_storage,
                &call_info, actual_recv_type, ctx, func, info
            )?;
            
            crate::expr::call::compile_method_args(call_expr, &param_types, is_variadic, args_start + recv_slots, ctx, func, info)?;
            
            emit_defer_func(opcode, *func_id, args_start, total_arg_slots, func);
        }
        MethodDispatch::Interface { method_idx } => {
            // Direct interface dispatch - generate wrapper
            compile_defer_iface_call(
                call_expr, sel, opcode, recv_type, recv_type, *method_idx, method_name,
                &call_info.embed_path.steps, false, ctx, func, info
            )?;
        }
        MethodDispatch::EmbeddedInterface { iface_type, method_idx } => {
            // Embedded interface dispatch - extract interface first
            compile_defer_iface_call(
                call_expr, sel, opcode, recv_type, *iface_type, *method_idx, method_name,
                &call_info.embed_path.steps, true, ctx, func, info
            )?;
        }
    }
    Ok(())
}

/// Helper for defer on interface method call.
/// Generates a wrapper function and emits defer with interface value + args.
fn compile_defer_iface_call(
    call_expr: &vo_syntax::ast::CallExpr,
    sel: &vo_syntax::ast::SelectorExpr,
    opcode: Opcode,
    recv_type: vo_analysis::objects::TypeKey,
    iface_type: vo_analysis::objects::TypeKey,
    method_idx: u32,
    method_name: &str,
    embed_steps: &[crate::embed::EmbedStep],
    is_embedded: bool,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    let method_type = info.expr_type(call_expr.func.id);
    let param_types = info.func_param_types(method_type);
    let is_variadic = info.is_variadic(method_type);
    let arg_slots = crate::expr::call::calc_method_arg_slots(call_expr, &param_types, is_variadic, info);
    
    let wrapper_id = crate::wrapper::generate_defer_iface_wrapper(
        ctx, iface_type, method_name, method_idx as usize, arg_slots, 0
    );
    
    let total_arg_slots = 2 + arg_slots;
    let args_start = func.alloc_args(total_arg_slots);
    
    // Compile interface receiver
    let iface_reg = crate::expr::compile_expr(&sel.expr, ctx, func, info)?;
    if is_embedded {
        let recv_is_ptr = info.is_pointer(recv_type);
        let start = crate::embed::TraverseStart { reg: iface_reg, is_pointer: recv_is_ptr };
        crate::embed::emit_embed_path_traversal(func, start, embed_steps, false, 2, args_start);
    } else {
        func.emit_copy(args_start, iface_reg, 2);
    }
    
    // Compile other args
    crate::expr::call::compile_method_args(call_expr, &param_types, is_variadic, args_start + 2, ctx, func, info)?;
    
    emit_defer_func(opcode, wrapper_id, args_start, total_arg_slots, func);
    Ok(())
}

/// Compile go statement
/// GoStart: a=func_id/closure, b=args_start, c=arg_slots, flags bit0=is_closure
pub(crate) fn compile_go(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    use vo_syntax::ast::ExprKind;
    
    let ExprKind::Call(call_expr) = &call.kind else {
        return Err(CodegenError::UnsupportedStmt("go requires a call expression".to_string()));
    };
    
    // Get function signature info
    let func_type = info.expr_type(call_expr.func.id);
    let param_types = info.func_param_types(func_type);
    let is_variadic = info.is_variadic(func_type);
    let total_arg_slots = crate::expr::call::calc_method_arg_slots(call_expr, &param_types, is_variadic, info);
    
    // Helper to compile args and emit GoStart for closure
    let emit_go_closure = |closure_reg: u16, func: &mut FuncBuilder, ctx: &mut CodegenContext| -> Result<(), CodegenError> {
        let args_start = if total_arg_slots > 0 {
            func.alloc_temp_typed(&vec![SlotType::Value; total_arg_slots as usize])
        } else { 0 };
        crate::expr::call::compile_method_args(call_expr, &param_types, is_variadic, args_start, ctx, func, info)?;
        func.emit_with_flags(Opcode::GoStart, 1, closure_reg, args_start, total_arg_slots);
        Ok(())
    };
    
    // Check if it's a regular function call
    if let ExprKind::Ident(ident) = &call_expr.func.kind {
        let obj_key = info.get_use(ident);
        if let Some(func_idx) = ctx.get_func_by_objkey(obj_key) {
            // Regular function
            let args_start = if total_arg_slots > 0 {
                func.alloc_temp_typed(&vec![SlotType::Value; total_arg_slots as usize])
            } else { 0 };
            crate::expr::call::compile_method_args(call_expr, &param_types, is_variadic, args_start, ctx, func, info)?;
            let (func_id_low, func_id_high) = crate::type_info::encode_func_id(func_idx);
            func.emit_with_flags(Opcode::GoStart, func_id_high << 1, func_id_low, args_start, total_arg_slots);
            return Ok(());
        }
        
        // Local variable (closure)
        if func.lookup_local(ident.symbol).is_some() || func.lookup_capture(ident.symbol).is_some() {
            let closure_reg = crate::expr::compile_expr(&call_expr.func, ctx, func, info)?;
            return emit_go_closure(closure_reg, func, ctx);
        }
    }
    
    // Generic case: expression returning a closure
    let closure_reg = crate::expr::compile_expr(&call_expr.func, ctx, func, info)?;
    emit_go_closure(closure_reg, func, ctx)
}
