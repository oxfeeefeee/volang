//! Statement compilation.

use vo_syntax::ast::{Block, Stmt, StmtKind};
use vo_vm::instruction::Opcode;

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::expr::compile_expr_to;
use crate::func::FuncBuilder;
use crate::type_info::TypeInfoWrapper;

/// Compile a statement.
pub fn compile_stmt(
    stmt: &Stmt,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    match &stmt.kind {
        // === Variable declaration ===
        StmtKind::Var(var_decl) => {
            for spec in &var_decl.specs {
                for (i, name) in spec.names.iter().enumerate() {
                    // Get type
                    let type_key = if let Some(ty) = &spec.ty {
                        info.project.type_info.type_exprs.get(&ty.id).copied()
                    } else if i < spec.values.len() {
                        info.expr_type(spec.values[i].id)
                    } else {
                        None
                    };

                    let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
                    let slot_types = type_key
                        .map(|t| info.type_slot_types(t))
                        .unwrap_or_else(|| vec![vo_common_core::types::SlotType::Value]);

                    // Check escape
                    let obj_key = info.get_def(name);
                    let escapes = obj_key.map(|k| info.is_escaped(k)).unwrap_or(false);

                    if escapes {
                        // Heap allocation for escaped variable (any type)
                        let slot = func.define_local_heap(name.symbol);
                        
                        // Get ValueMeta index for PtrNew
                        let meta_idx = ctx.get_or_create_value_meta(type_key, slots, &slot_types);
                        
                        // PtrNew: a=dst, b=meta_idx, c=slots
                        func.emit_op(Opcode::PtrNew, slot, meta_idx, slots);
                        
                        // Initialize value
                        if i < spec.values.len() {
                            // Compile value to temp, then PtrSet
                            let tmp = func.alloc_temp(slots);
                            compile_expr_to(&spec.values[i], tmp, ctx, func, info)?;
                            if slots == 1 {
                                func.emit_op(Opcode::PtrSet, slot, 0, tmp);
                            } else {
                                func.emit_with_flags(Opcode::PtrSetN, slots as u8, slot, 0, tmp);
                            }
                        }
                        // else: PtrNew already zero-initializes
                    } else {
                        // Stack allocation
                        let slot = func.define_local_stack(name.symbol, slots, &slot_types);

                        // Initialize
                        if i < spec.values.len() {
                            compile_expr_to(&spec.values[i], slot, ctx, func, info)?;
                        } else {
                            // Zero initialize
                            for j in 0..slots {
                                func.emit_op(Opcode::LoadNil, slot + j, 0, 0);
                            }
                        }
                    }
                }
            }
        }

        // === Short variable declaration ===
        StmtKind::ShortVar(short_var) => {
            for (i, name) in short_var.names.iter().enumerate() {
                // Skip blank identifier
                if info.project.interner.resolve(name.symbol) == Some("_") {
                    continue;
                }

                let type_key = if i < short_var.values.len() {
                    info.expr_type(short_var.values[i].id)
                } else {
                    None
                };

                let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
                let slot_types = type_key
                    .map(|t| info.type_slot_types(t))
                    .unwrap_or_else(|| vec![vo_common_core::types::SlotType::Value]);

                // Check if this is a new definition or reassignment
                let is_def = info.project.type_info.defs.contains_key(name);

                if is_def {
                    // New variable
                    let obj_key = info.get_def(name);
                    let escapes = obj_key.map(|k| info.is_escaped(k)).unwrap_or(false);

                    if escapes {
                        // Heap allocation for escaped variable
                        let slot = func.define_local_heap(name.symbol);
                        
                        // Get ValueMeta index for PtrNew
                        let meta_idx = ctx.get_or_create_value_meta(type_key, slots, &slot_types);
                        
                        // PtrNew: a=dst, b=meta_idx, c=slots
                        func.emit_op(Opcode::PtrNew, slot, meta_idx, slots);
                        
                        // Initialize value
                        if i < short_var.values.len() {
                            let tmp = func.alloc_temp(slots);
                            compile_expr_to(&short_var.values[i], tmp, ctx, func, info)?;
                            if slots == 1 {
                                func.emit_op(Opcode::PtrSet, slot, 0, tmp);
                            } else {
                                func.emit_with_flags(Opcode::PtrSetN, slots as u8, slot, 0, tmp);
                            }
                        }
                    } else {
                        let slot = func.define_local_stack(name.symbol, slots, &slot_types);
                        if i < short_var.values.len() {
                            compile_expr_to(&short_var.values[i], slot, ctx, func, info)?;
                        }
                    }
                } else {
                    // Reassignment to existing variable
                    if let Some(local) = func.lookup_local(name.symbol) {
                        let slot = local.slot;
                        if i < short_var.values.len() {
                            compile_expr_to(&short_var.values[i], slot, ctx, func, info)?;
                        }
                    }
                }
            }
        }

        // === Assignment ===
        StmtKind::Assign(assign) => {
            for (lhs, rhs) in assign.lhs.iter().zip(assign.rhs.iter()) {
                compile_assign(lhs, rhs, ctx, func, info)?;
            }
        }

        // === Expression statement ===
        StmtKind::Expr(expr) => {
            let _ = crate::expr::compile_expr(expr, ctx, func, info)?;
        }

        // === Return ===
        StmtKind::Return(ret) => {
            if ret.values.is_empty() {
                func.emit_op(Opcode::Return, 0, 0, 0);
            } else {
                // Calculate total return slots needed
                let mut total_ret_slots = 0u16;
                for result in &ret.values {
                    let type_key = info.expr_type(result.id);
                    let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
                    total_ret_slots += slots;
                }
                
                // Allocate space for return values
                let ret_start = func.alloc_temp(total_ret_slots);
                
                // Compile return values
                let mut offset = 0u16;
                for result in &ret.values {
                    let type_key = info.expr_type(result.id);
                    let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
                    compile_expr_to(result, ret_start + offset, ctx, func, info)?;
                    offset += slots;
                }
                func.emit_op(Opcode::Return, ret_start, total_ret_slots, 0);
            }
        }

        // === If statement ===
        StmtKind::If(if_stmt) => {
            // Init statement
            if let Some(init) = &if_stmt.init {
                compile_stmt(init, ctx, func, info)?;
            }

            // Condition
            let cond_reg = crate::expr::compile_expr(&if_stmt.cond, ctx, func, info)?;
            let else_jump = func.emit_jump(Opcode::JumpIfNot, cond_reg);

            // Then branch
            compile_block(&if_stmt.then, ctx, func, info)?;

            if let Some(else_body) = &if_stmt.else_ {
                let end_jump = func.emit_jump(Opcode::Jump, 0);
                func.patch_jump(else_jump, func.current_pc());
                compile_stmt(else_body, ctx, func, info)?;
                func.patch_jump(end_jump, func.current_pc());
            } else {
                func.patch_jump(else_jump, func.current_pc());
            }
        }

        // === For statement ===
        StmtKind::For(for_stmt) => {
            use vo_syntax::ast::ForClause;

            match &for_stmt.clause {
                ForClause::Cond(cond_opt) => {
                    // while-style: for cond { } or infinite: for { }
                    let loop_start = func.current_pc();
                    func.enter_loop(loop_start, None);

                    let end_jump = if let Some(cond) = cond_opt {
                        let cond_reg = crate::expr::compile_expr(cond, ctx, func, info)?;
                        Some(func.emit_jump(Opcode::JumpIfNot, cond_reg))
                    } else {
                        None
                    };

                    compile_block(&for_stmt.body, ctx, func, info)?;
                    func.emit_jump_to(Opcode::Jump, 0, loop_start);

                    if let Some(j) = end_jump {
                        func.patch_jump(j, func.current_pc());
                    }
                    let break_patches = func.exit_loop();
                    for pc in break_patches {
                        func.patch_jump(pc, func.current_pc());
                    }
                }

                ForClause::Three { init, cond, post } => {
                    // C-style: for init; cond; post { }
                    if let Some(init) = init {
                        compile_stmt(init, ctx, func, info)?;
                    }

                    let loop_start = func.current_pc();
                    let post_pc = loop_start; // continue goes to post (will adjust)

                    let end_jump = if let Some(cond) = cond {
                        let cond_reg = crate::expr::compile_expr(cond, ctx, func, info)?;
                        Some(func.emit_jump(Opcode::JumpIfNot, cond_reg))
                    } else {
                        None
                    };

                    func.enter_loop(post_pc, None);
                    compile_block(&for_stmt.body, ctx, func, info)?;

                    // Post statement
                    let actual_post_pc = func.current_pc();
                    if let Some(post) = post {
                        compile_stmt(post, ctx, func, info)?;
                    }

                    func.emit_jump_to(Opcode::Jump, 0, loop_start);

                    if let Some(j) = end_jump {
                        func.patch_jump(j, func.current_pc());
                    }

                    let break_patches = func.exit_loop();
                    for pc in break_patches {
                        func.patch_jump(pc, func.current_pc());
                    }

                    // Fix continue jumps to post
                    let _ = actual_post_pc;
                }

                ForClause::Range { .. } => {
                    // TODO: for-range
                    return Err(CodegenError::UnsupportedStmt("for-range".to_string()));
                }
            }
        }

        // === Block ===
        StmtKind::Block(block) => {
            compile_block(block, ctx, func, info)?;
        }

        // === Break ===
        StmtKind::Break(brk) => {
            func.emit_break(brk.label.as_ref().map(|l| l.symbol));
        }

        // === Continue ===
        StmtKind::Continue(cont) => {
            func.emit_continue(cont.label.as_ref().map(|l| l.symbol));
        }

        // === Empty ===
        StmtKind::Empty => {}

        // === Defer ===
        StmtKind::Defer(defer_stmt) => {
            // Defer is implemented as pushing a closure to defer stack
            // The call expression becomes a closure that will be called on function exit
            compile_defer(&defer_stmt.call, ctx, func, info)?;
        }

        // === Go ===
        StmtKind::Go(go_stmt) => {
            compile_go(&go_stmt.call, ctx, func, info)?;
        }

        // === Send (channel send) ===
        StmtKind::Send(send_stmt) => {
            let chan_reg = crate::expr::compile_expr(&send_stmt.chan, ctx, func, info)?;
            let val_reg = crate::expr::compile_expr(&send_stmt.value, ctx, func, info)?;
            func.emit_op(Opcode::ChanSend, chan_reg, val_reg, 0);
        }

        // TODO: more statement kinds
        _ => {
            return Err(CodegenError::UnsupportedStmt(format!("{:?}", stmt.kind)));
        }
    }

    Ok(())
}

/// Compile a block.
pub fn compile_block(
    block: &Block,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    for stmt in &block.stmts {
        compile_stmt(stmt, ctx, func, info)?;
    }
    Ok(())
}

/// Compile defer statement
fn compile_defer(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Compile the call expression to get closure/function
    // For simplicity, compile the entire call and use Defer instruction
    let call_reg = crate::expr::compile_expr(call, ctx, func, info)?;
    
    // DeferPush instruction: push closure to defer stack
    func.emit_op(Opcode::DeferPush, call_reg, 0, 0);
    
    Ok(())
}

/// Compile go statement
fn compile_go(
    call: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Compile the call expression
    if let vo_syntax::ast::ExprKind::Call(call_expr) = &call.kind {
        // Compile function/closure
        let func_reg = crate::expr::compile_expr(&call_expr.func, ctx, func, info)?;
        
        // Compile arguments
        let args_start = func.alloc_temp(call_expr.args.len() as u16);
        for (i, arg) in call_expr.args.iter().enumerate() {
            crate::expr::compile_expr_to(arg, args_start + (i as u16), ctx, func, info)?;
        }
        
        // GoCall: a=func, b=args_start, c=arg_count
        func.emit_op(Opcode::GoCall, func_reg, args_start, call_expr.args.len() as u16);
    } else {
        return Err(CodegenError::UnsupportedStmt("go with non-call".to_string()));
    }
    
    Ok(())
}

/// Compile assignment.
fn compile_assign(
    lhs: &vo_syntax::ast::Expr,
    rhs: &vo_syntax::ast::Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    use vo_syntax::ast::ExprKind;

    match &lhs.kind {
        ExprKind::Ident(ident) => {
            // Copy local info to avoid borrow conflict
            let local_info = func.lookup_local(ident.symbol).map(|l| (l.slot, l.slots, l.is_heap));
            
            if let Some((slot, slots, is_heap)) = local_info {
                // Check if assigning to interface variable
                let lhs_type = info.get_def(ident).and_then(|o| info.obj_type(o));
                let is_iface = lhs_type.map(|t| info.is_interface(t)).unwrap_or(false);
                
                if is_iface {
                    // Interface assignment: use IfaceAssign
                    let src_reg = crate::expr::compile_expr(rhs, ctx, func, info)?;
                    let src_type = info.expr_type(rhs.id);
                    let iface_meta_id = lhs_type.and_then(|t| ctx.get_interface_meta_id(t)).unwrap_or(0);
                    
                    // Get source value kind for flags
                    let vk = src_type.map(|t| info.value_kind(t)).unwrap_or(0);
                    
                    // IfaceAssign: a=dst, b=src, c=iface_meta_id, flags=vk
                    func.emit_with_flags(Opcode::IfaceAssign, vk, slot, src_reg, iface_meta_id);
                } else if is_heap {
                    // Escaped variable: write via PtrSet
                    let tmp = crate::expr::compile_expr(rhs, ctx, func, info)?;
                    if slots == 1 {
                        func.emit_op(Opcode::PtrSet, slot, 0, tmp);
                    } else {
                        func.emit_with_flags(Opcode::PtrSetN, slots as u8, slot, 0, tmp);
                    }
                } else {
                    // Stack variable: direct write
                    compile_expr_to(rhs, slot, ctx, func, info)?;
                }
            } else if let Some(global_idx) = ctx.get_global_index(ident.symbol) {
                let tmp = crate::expr::compile_expr(rhs, ctx, func, info)?;
                func.emit_op(Opcode::GlobalSet, global_idx as u16, tmp, 0);
            } else {
                return Err(CodegenError::VariableNotFound(format!("{:?}", ident.symbol)));
            }
        }

        // === Selector assignment (struct field) ===
        ExprKind::Selector(sel) => {
            let recv_type = info.expr_type(sel.expr.id)
                .ok_or_else(|| CodegenError::Internal("selector recv has no type".to_string()))?;
            
            let field_name = info.project.interner.resolve(sel.sel.symbol)
                .ok_or_else(|| CodegenError::Internal("cannot resolve field".to_string()))?;
            
            let (offset, slots) = info.struct_field_offset(recv_type, field_name)
                .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
            
            // Check if receiver is pointer or heap variable
            let is_ptr = info.is_pointer(recv_type);
            
            if is_ptr {
                // Pointer receiver: compile ptr, then PtrSet
                let ptr_reg = crate::expr::compile_expr(&sel.expr, ctx, func, info)?;
                let tmp = crate::expr::compile_expr(rhs, ctx, func, info)?;
                if slots == 1 {
                    func.emit_op(Opcode::PtrSet, ptr_reg, offset, tmp);
                } else {
                    func.emit_with_flags(Opcode::PtrSetN, slots as u8, ptr_reg, offset, tmp);
                }
            } else {
                // Value receiver on stack - find root variable
                if let ExprKind::Ident(ident) = &sel.expr.kind {
                    let local_info = func.lookup_local(ident.symbol)
                        .map(|l| (l.slot, l.is_heap));
                    
                    if let Some((base_slot, is_heap)) = local_info {
                        if is_heap {
                            // Heap variable: use PtrSet
                            let tmp = crate::expr::compile_expr(rhs, ctx, func, info)?;
                            if slots == 1 {
                                func.emit_op(Opcode::PtrSet, base_slot, offset, tmp);
                            } else {
                                func.emit_with_flags(Opcode::PtrSetN, slots as u8, base_slot, offset, tmp);
                            }
                        } else {
                            // Stack variable: direct slot write
                            let target_slot = base_slot + offset;
                            compile_expr_to(rhs, target_slot, ctx, func, info)?;
                        }
                    } else {
                        return Err(CodegenError::VariableNotFound(format!("{:?}", ident.symbol)));
                    }
                } else {
                    return Err(CodegenError::InvalidLHS);
                }
            }
        }

        // TODO: index assignment
        _ => {
            return Err(CodegenError::InvalidLHS);
        }
    }

    Ok(())
}
