//! Expression compilation.

use vo_syntax::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use vo_vm::instruction::Opcode;

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::func::FuncBuilder;
use crate::type_info::TypeInfoWrapper;

/// Compile expression, return result slot.
pub fn compile_expr(
    expr: &Expr,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<u16, CodegenError> {
    let dst = func.alloc_temp(1); // TODO: multi-slot types
    compile_expr_to(expr, dst, ctx, func, info)?;
    Ok(dst)
}

/// Compile expression to specified slot.
pub fn compile_expr_to(
    expr: &Expr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    match &expr.kind {
        // === Literals (use constant values from type checking) ===
        ExprKind::IntLit(_) | ExprKind::RuneLit(_) => {
            let val = get_const_int(expr.id, info).unwrap_or(0);
            if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
                // Small int - inline
                let (b, c) = encode_i32(val as i32);
                func.emit_op(Opcode::LoadInt, dst, b, c);
            } else {
                // Large int - from constant pool
                let idx = ctx.const_int(val);
                func.emit_op(Opcode::LoadConst, dst, idx, 0);
            }
        }

        ExprKind::FloatLit(_) => {
            let val = get_const_float(expr.id, info).unwrap_or(0.0);
            let idx = ctx.const_float(val);
            func.emit_op(Opcode::LoadConst, dst, idx, 0);
        }

        ExprKind::StringLit(_) => {
            let val = get_const_string(expr.id, info).unwrap_or_default();
            let idx = ctx.const_string(&val);
            func.emit_op(Opcode::StrNew, dst, idx, 0);
        }

        // === Identifier ===
        ExprKind::Ident(ident) => {
            if let Some(local) = func.lookup_local(ident.symbol) {
                if local.is_heap {
                    // Escaped variable: slot contains GcRef to heap object
                    // Check if this is a value type (struct/array) - need deep copy
                    let obj_key = info.get_def(ident);
                    let type_key = obj_key.and_then(|o| info.obj_type(o));
                    let is_value_type = type_key.map(|t| {
                        info.is_struct(t) || info.is_array(t)
                    }).unwrap_or(false);
                    
                    if is_value_type {
                        // Escaped struct/array: use PtrClone for value semantics (deep copy)
                        func.emit_op(Opcode::PtrClone, dst, local.slot, 0);
                    } else {
                        // Escaped primitive or reference type: just copy the GcRef
                        func.emit_op(Opcode::Copy, dst, local.slot, 0);
                    }
                } else {
                    // Stack variable: direct copy
                    if local.slots == 1 {
                        func.emit_op(Opcode::Copy, dst, local.slot, 0);
                    } else {
                        func.emit_with_flags(Opcode::CopyN, local.slots as u8, dst, local.slot, local.slots);
                    }
                }
            } else if let Some(capture) = func.lookup_capture(ident.symbol) {
                // Closure capture: use ClosureGet
                // ClosureGet: a=dst, b=capture_index (closure implicit in r0)
                func.emit_op(Opcode::ClosureGet, dst, capture.index, 0);
            } else if let Some(global_idx) = ctx.get_global_index(ident.symbol) {
                func.emit_op(Opcode::GlobalGet, dst, global_idx as u16, 0);
            } else {
                // Could be a function name, package, etc. - handle later
                return Err(CodegenError::VariableNotFound(format!("{:?}", ident.symbol)));
            }
        }

        // === Binary operations ===
        ExprKind::Binary(bin) => {
            let left_reg = compile_expr(&bin.left, ctx, func, info)?;
            let right_reg = compile_expr(&bin.right, ctx, func, info)?;

            // Get type to determine int/float operation
            let type_key = info.expr_type(expr.id);
            let is_float = type_key.map(|t| is_float_type(t, info)).unwrap_or(false);
            let is_string = type_key.map(|t| is_string_type(t, info)).unwrap_or(false);

            let opcode = match (&bin.op, is_float, is_string) {
                (BinaryOp::Add, false, false) => Opcode::AddI,
                (BinaryOp::Add, true, false) => Opcode::AddF,
                (BinaryOp::Add, _, true) => Opcode::StrConcat,
                (BinaryOp::Sub, false, _) => Opcode::SubI,
                (BinaryOp::Sub, true, _) => Opcode::SubF,
                (BinaryOp::Mul, false, _) => Opcode::MulI,
                (BinaryOp::Mul, true, _) => Opcode::MulF,
                (BinaryOp::Div, false, _) => Opcode::DivI,
                (BinaryOp::Div, true, _) => Opcode::DivF,
                (BinaryOp::Rem, _, _) => Opcode::ModI,

                // Comparison
                (BinaryOp::Eq, false, false) => Opcode::EqI,
                (BinaryOp::Eq, true, false) => Opcode::EqF,
                (BinaryOp::Eq, _, true) => Opcode::StrEq,
                (BinaryOp::NotEq, false, false) => Opcode::NeI,
                (BinaryOp::NotEq, true, false) => Opcode::NeF,
                (BinaryOp::NotEq, _, true) => Opcode::StrNe,
                (BinaryOp::Lt, false, false) => Opcode::LtI,
                (BinaryOp::Lt, true, false) => Opcode::LtF,
                (BinaryOp::Lt, _, true) => Opcode::StrLt,
                (BinaryOp::LtEq, false, false) => Opcode::LeI,
                (BinaryOp::LtEq, true, false) => Opcode::LeF,
                (BinaryOp::LtEq, _, true) => Opcode::StrLe,
                (BinaryOp::Gt, false, false) => Opcode::GtI,
                (BinaryOp::Gt, true, false) => Opcode::GtF,
                (BinaryOp::Gt, _, true) => Opcode::StrGt,
                (BinaryOp::GtEq, false, false) => Opcode::GeI,
                (BinaryOp::GtEq, true, false) => Opcode::GeF,
                (BinaryOp::GtEq, _, true) => Opcode::StrGe,

                // Bitwise
                (BinaryOp::And, _, _) => Opcode::And,
                (BinaryOp::Or, _, _) => Opcode::Or,
                (BinaryOp::Xor, _, _) => Opcode::Xor,
                (BinaryOp::Shl, _, _) => Opcode::Shl,
                (BinaryOp::Shr, _, _) => Opcode::ShrS, // TODO: unsigned shift

                // Logical (short-circuit handled separately)
                (BinaryOp::LogAnd, _, _) | (BinaryOp::LogOr, _, _) => {
                    return compile_short_circuit(expr, &bin.op, &bin.left, &bin.right, dst, ctx, func, info);
                }

                _ => return Err(CodegenError::UnsupportedExpr(format!("binary op {:?}", bin.op))),
            };

            func.emit_op(opcode, dst, left_reg, right_reg);
        }

        // === Unary operations ===
        ExprKind::Unary(unary) => {
            match unary.op {
                UnaryOp::Addr => {
                    compile_addr_of(&unary.operand, dst, ctx, func, info)?;
                }
                UnaryOp::Deref => {
                    compile_deref(&unary.operand, dst, ctx, func, info)?;
                }
                UnaryOp::Neg => {
                    let operand = compile_expr(&unary.operand, ctx, func, info)?;
                    let type_key = info.expr_type(expr.id);
                    let is_float = type_key.map(|t| is_float_type(t, info)).unwrap_or(false);
                    let opcode = if is_float { Opcode::NegF } else { Opcode::NegI };
                    func.emit_op(opcode, dst, operand, 0);
                }
                UnaryOp::Not => {
                    let operand = compile_expr(&unary.operand, ctx, func, info)?;
                    func.emit_op(Opcode::BoolNot, dst, operand, 0);
                }
                UnaryOp::BitNot => {
                    let operand = compile_expr(&unary.operand, ctx, func, info)?;
                    func.emit_op(Opcode::Not, dst, operand, 0);
                }
                UnaryOp::Pos => {
                    // +x is a no-op
                    compile_expr_to(&unary.operand, dst, ctx, func, info)?;
                }
            }
        }

        // === Parentheses ===
        ExprKind::Paren(inner) => {
            compile_expr_to(inner, dst, ctx, func, info)?;
        }

        // === Selector (field access) ===
        ExprKind::Selector(sel) => {
            compile_selector(expr, sel, dst, ctx, func, info)?;
        }

        // === Index (array/slice access) ===
        ExprKind::Index(idx) => {
            compile_index(expr, idx, dst, ctx, func, info)?;
        }

        // === Composite literal ===
        ExprKind::CompositeLit(lit) => {
            compile_composite_lit(expr, lit, dst, ctx, func, info)?;
        }

        // === Call expression ===
        ExprKind::Call(call) => {
            compile_call(expr, call, dst, ctx, func, info)?;
        }

        // === Function literal (closure) ===
        ExprKind::FuncLit(func_lit) => {
            compile_func_lit(expr, func_lit, dst, ctx, func, info)?;
        }

        // TODO: more expression kinds
        _ => {
            return Err(CodegenError::UnsupportedExpr(format!("{:?}", expr.kind)));
        }
    }

    Ok(())
}

/// Compile short-circuit logical operations (&&, ||).
fn compile_short_circuit(
    _expr: &Expr,
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    compile_expr_to(left, dst, ctx, func, info)?;

    let skip_jump = match op {
        BinaryOp::LogAnd => func.emit_jump(Opcode::JumpIfNot, dst), // if false, skip
        BinaryOp::LogOr => func.emit_jump(Opcode::JumpIf, dst),     // if true, skip
        _ => unreachable!(),
    };

    compile_expr_to(right, dst, ctx, func, info)?;

    func.patch_jump(skip_jump, func.current_pc());
    Ok(())
}

// === Selector (field access) ===

fn compile_selector(
    expr: &Expr,
    sel: &vo_syntax::ast::SelectorExpr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Get receiver type
    let recv_type = info.expr_type(sel.expr.id)
        .ok_or_else(|| CodegenError::Internal("selector receiver has no type".to_string()))?;
    
    let field_name = info.project.interner.resolve(sel.sel.symbol)
        .ok_or_else(|| CodegenError::Internal("cannot resolve field name".to_string()))?;
    
    // Check if receiver is pointer - need to dereference
    let is_ptr = info.is_pointer(recv_type);
    
    if is_ptr {
        // Pointer receiver: load ptr, then PtrGetN
        let ptr_reg = compile_expr(&sel.expr, ctx, func, info)?;
        let (offset, slots) = info.struct_field_offset_from_ptr(recv_type, field_name)
            .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
        
        if slots == 1 {
            func.emit_op(Opcode::PtrGet, dst, ptr_reg, offset);
        } else {
            func.emit_with_flags(Opcode::PtrGetN, slots as u8, dst, ptr_reg, offset);
        }
    } else {
        // Value receiver: check if on stack or heap
        let root_var = find_root_var(&sel.expr, func);
        
        if let Some((local_slot, is_heap)) = root_var {
            if is_heap {
                // Heap variable: load GcRef, then PtrGetN
                let (offset, slots) = info.struct_field_offset(recv_type, field_name)
                    .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
                
                if slots == 1 {
                    func.emit_op(Opcode::PtrGet, dst, local_slot, offset);
                } else {
                    func.emit_with_flags(Opcode::PtrGetN, slots as u8, dst, local_slot, offset);
                }
            } else {
                // Stack variable: direct slot access
                let (offset, slots) = info.struct_field_offset(recv_type, field_name)
                    .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
                
                let src_slot = local_slot + offset;
                if slots == 1 {
                    func.emit_op(Opcode::Copy, dst, src_slot, 0);
                } else {
                    func.emit_with_flags(Opcode::CopyN, slots as u8, dst, src_slot, slots);
                }
            }
        } else {
            // Temporary value - compile receiver first
            let recv_reg = compile_expr(&sel.expr, ctx, func, info)?;
            let (offset, slots) = info.struct_field_offset(recv_type, field_name)
                .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
            
            let src_slot = recv_reg + offset;
            if slots == 1 {
                func.emit_op(Opcode::Copy, dst, src_slot, 0);
            } else {
                func.emit_with_flags(Opcode::CopyN, slots as u8, dst, src_slot, slots);
            }
        }
    }
    
    Ok(())
}

/// Find root variable for selector chain
fn find_root_var(expr: &Expr, func: &FuncBuilder) -> Option<(u16, bool)> {
    match &expr.kind {
        ExprKind::Ident(ident) => {
            func.lookup_local(ident.symbol).map(|l| (l.slot, l.is_heap))
        }
        ExprKind::Selector(sel) => find_root_var(&sel.expr, func),
        ExprKind::Paren(inner) => find_root_var(inner, func),
        _ => None,
    }
}

// === Index (array/slice access) ===

fn compile_index(
    expr: &Expr,
    idx: &vo_syntax::ast::IndexExpr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    let container_type = info.expr_type(idx.expr.id)
        .ok_or_else(|| CodegenError::Internal("index container has no type".to_string()))?;
    
    // Compile container
    let container_reg = compile_expr(&idx.expr, ctx, func, info)?;
    
    // Compile index
    let index_reg = compile_expr(&idx.index, ctx, func, info)?;
    
    // Check if array or slice
    if info.is_array(container_type) {
        // Array: ArrayGet (flags = elem_slots if > 1)
        let elem_slots = info.array_elem_slots(container_type).unwrap_or(1);
        func.emit_with_flags(Opcode::ArrayGet, elem_slots as u8, dst, container_reg, index_reg);
    } else {
        // Slice: SliceGet (flags = elem_slots if > 1)
        let elem_slots = info.slice_elem_slots(container_type).unwrap_or(1);
        func.emit_with_flags(Opcode::SliceGet, elem_slots as u8, dst, container_reg, index_reg);
    }
    
    Ok(())
}

// === Function literal (closure) ===

fn compile_func_lit(
    expr: &Expr,
    func_lit: &vo_syntax::ast::FuncLit,
    dst: u16,
    ctx: &mut CodegenContext,
    parent_func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Get closure captures from type info
    let captures = info.project.type_info.closure_captures.get(&expr.id)
        .cloned()
        .unwrap_or_default();
    
    // Generate a unique name for the closure function
    let closure_name = format!("closure_{}", ctx.next_closure_id());
    
    // Create new FuncBuilder for the closure body (slot 0 reserved for closure ref)
    let mut closure_builder = FuncBuilder::new_closure(&closure_name);
    
    // Register captures in closure builder so it can access them via ClosureGet
    for (i, obj_key) in captures.iter().enumerate() {
        let var_name = info.obj_name(*obj_key);
        if let Some(sym) = info.project.interner.get(var_name) {
            closure_builder.define_capture(sym, i as u16);
        }
    }
    
    // Define parameters (starting after slot 0 which is closure ref)
    for param in &func_lit.sig.params {
        let type_key = info.project.type_info.type_exprs.get(&param.ty.id).copied();
        let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
        let slot_types = type_key
            .map(|t| info.type_slot_types(t))
            .unwrap_or_else(|| vec![vo_common_core::types::SlotType::Value]);
        for name in &param.names {
            closure_builder.define_param(name.symbol, slots, &slot_types);
        }
    }
    
    // Set return slots
    let mut ret_slots = 0u16;
    for result in &func_lit.sig.results {
        let type_key = info.project.type_info.type_exprs.get(&result.ty.id).copied();
        let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
        ret_slots += slots;
    }
    closure_builder.set_ret_slots(ret_slots);
    
    // Compile closure body
    crate::stmt::compile_block(&func_lit.body, ctx, &mut closure_builder, info)?;
    
    // Add return if not present
    closure_builder.emit_op(Opcode::Return, 0, 0, 0);
    
    // Build and add closure function to module
    let closure_func = closure_builder.build();
    let func_id = ctx.add_function(closure_func);
    
    // Emit ClosureNew instruction
    // ClosureNew: a=dst, b=func_id, c=capture_count
    let capture_count = captures.len() as u16;
    parent_func.emit_op(Opcode::ClosureNew, dst, func_id as u16, capture_count);
    
    // Set captures (copy GcRefs from escaped variables)
    // Closure layout: ClosureHeader (1 slot) + captures[]
    // So capture[i] is at offset (1 + i)
    for (i, obj_key) in captures.iter().enumerate() {
        let var_name = info.obj_name(*obj_key);
        if let Some(sym) = info.project.interner.get(var_name) {
            if let Some(local) = parent_func.lookup_local(sym) {
                // Use PtrSet to write directly to closure's capture slot
                // PtrSet: heap[slots[a]].offset[b] = slots[c]
                // offset = 1 (ClosureHeader) + capture_index
                let offset = 1 + i as u16;
                parent_func.emit_op(Opcode::PtrSet, dst, offset, local.slot);
            }
        }
    }
    
    Ok(())
}

// === Address-of ===

fn compile_addr_of(
    operand: &Expr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // &x where x is escaped -> x's slot already holds GcRef, just copy
    if let ExprKind::Ident(ident) = &operand.kind {
        if let Some(local) = func.lookup_local(ident.symbol) {
            if local.is_heap {
                // x is already heap-allocated, its slot holds the GcRef
                func.emit_op(Opcode::Copy, dst, local.slot, 0);
                return Ok(());
            }
        }
    }
    
    // TODO: handle &stack_var (need to allocate on heap)
    Err(CodegenError::UnsupportedExpr("address-of non-escaped".to_string()))
}

// === Dereference ===

fn compile_deref(
    operand: &Expr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // *p -> PtrGet
    let ptr_reg = compile_expr(operand, ctx, func, info)?;
    
    // Get element type slot count
    let ptr_type = info.expr_type(operand.id);
    let elem_slots = ptr_type
        .and_then(|t| info.pointer_elem_slots(t))
        .unwrap_or(1);
    
    if elem_slots == 1 {
        func.emit_op(Opcode::PtrGet, dst, ptr_reg, 0);
    } else {
        func.emit_with_flags(Opcode::PtrGetN, elem_slots as u8, dst, ptr_reg, 0);
    }
    
    Ok(())
}

// === Call expression ===

fn compile_call(
    expr: &Expr,
    call: &vo_syntax::ast::CallExpr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Check if method call (selector expression)
    if let ExprKind::Selector(sel) = &call.func.kind {
        return compile_method_call(expr, call, sel, dst, ctx, func, info);
    }
    
    // Check if builtin
    if let ExprKind::Ident(ident) = &call.func.kind {
        let name = info.project.interner.resolve(ident.symbol);
        if let Some(name) = name {
            if is_builtin(name) {
                return compile_builtin_call(name, call, dst, ctx, func, info);
            }
        }
    }
    
    // Check if calling a closure (local variable with Signature type)
    if let ExprKind::Ident(ident) = &call.func.kind {
        // First check if it's a known function
        if let Some(func_idx) = ctx.get_function_index(ident.symbol) {
            // Regular function call
            let args_start = func.alloc_temp(call.args.len() as u16);
            for (i, arg) in call.args.iter().enumerate() {
                compile_expr_to(arg, args_start + (i as u16), ctx, func, info)?;
            }
            func.emit_with_flags(Opcode::Call, call.args.len() as u8, dst, func_idx as u16, args_start);
            return Ok(());
        }
        
        // Check if it's a local variable (could be a closure)
        if func.lookup_local(ident.symbol).is_some() || func.lookup_capture(ident.symbol).is_some() {
            // Closure call - compile closure expression first
            let closure_reg = compile_expr(&call.func, ctx, func, info)?;
            
            // Compile arguments
            let args_start = func.alloc_temp(call.args.len() as u16);
            for (i, arg) in call.args.iter().enumerate() {
                compile_expr_to(arg, args_start + (i as u16), ctx, func, info)?;
            }
            
            // Get return slot count
            let ret_type = info.expr_type(expr.id);
            let ret_slots = ret_type.map(|t| info.type_slot_count(t)).unwrap_or(0) as u8;
            
            // CallClosure: a=closure, b=args_start, c=(arg_slots<<8|ret_slots)
            let c = ((call.args.len() as u16) << 8) | (ret_slots as u16);
            func.emit_op(Opcode::CallClosure, closure_reg, args_start, c);
            
            // Copy result to dst if needed
            if dst != closure_reg && ret_slots > 0 {
                // Result is at closure_reg after call? Actually need to check VM behavior
                // For now, assume result goes to args_start position
            }
            
            return Ok(());
        }
        
        return Err(CodegenError::Internal("function not found".to_string()));
    }
    
    // Non-ident function call (e.g., expression returning a closure)
    let closure_reg = compile_expr(&call.func, ctx, func, info)?;
    
    // Compile arguments
    let args_start = func.alloc_temp(call.args.len() as u16);
    for (i, arg) in call.args.iter().enumerate() {
        compile_expr_to(arg, args_start + (i as u16), ctx, func, info)?;
    }
    
    // Get return slot count
    let ret_type = info.expr_type(expr.id);
    let ret_slots = ret_type.map(|t| info.type_slot_count(t)).unwrap_or(0) as u8;
    
    // CallClosure: a=closure, b=args_start, c=(arg_slots<<8|ret_slots)
    let c = ((call.args.len() as u16) << 8) | (ret_slots as u16);
    func.emit_op(Opcode::CallClosure, closure_reg, args_start, c);
    
    Ok(())
}

fn compile_method_call(
    _expr: &Expr,
    call: &vo_syntax::ast::CallExpr,
    sel: &vo_syntax::ast::SelectorExpr,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Get receiver type
    let recv_type = info.expr_type(sel.expr.id)
        .ok_or_else(|| CodegenError::Internal("method receiver has no type".to_string()))?;
    
    // Compile receiver first
    let recv_reg = compile_expr(&sel.expr, ctx, func, info)?;
    
    // Get method name
    let method_name = info.project.interner.resolve(sel.sel.symbol)
        .ok_or_else(|| CodegenError::Internal("cannot resolve method name".to_string()))?;
    
    // Check if interface method call
    if info.is_interface(recv_type) {
        // Interface method call: use CallIface
        // Compile arguments
        let args_start = func.alloc_temp(call.args.len() as u16);
        for (i, arg) in call.args.iter().enumerate() {
            compile_expr_to(arg, args_start + (i as u16), ctx, func, info)?;
        }
        
        // Get method index in interface
        let method_idx = info.get_interface_method_index(recv_type, method_name).unwrap_or(0);
        
        // CallIface: a=dst, b=iface_slot, c=method_idx, flags=arg_count
        func.emit_with_flags(Opcode::CallIface, call.args.len() as u8, dst, recv_reg, method_idx);
    } else {
        // Concrete method call: find function and use Call
        // Compile arguments (receiver + args)
        let total_args = 1 + call.args.len();
        let args_start = func.alloc_temp(total_args as u16);
        
        // Copy receiver to args (slot 0)
        let recv_slots = info.type_slot_count(recv_type);
        if recv_slots == 1 {
            func.emit_op(Opcode::Copy, args_start, recv_reg, 0);
        } else {
            func.emit_with_flags(Opcode::CopyN, recv_slots as u8, args_start, recv_reg, recv_slots);
        }
        
        // Compile other arguments
        let mut arg_offset = recv_slots;
        for arg in &call.args {
            compile_expr_to(arg, args_start + arg_offset, ctx, func, info)?;
            let arg_type = info.expr_type(arg.id);
            arg_offset += arg_type.map(|t| info.type_slot_count(t)).unwrap_or(1);
        }
        
        // Look up method function
        if let Some(method_sym) = info.project.interner.get(method_name) {
            if let Some(func_idx) = ctx.get_func_index(Some(recv_type), method_sym) {
                // Call: a=dst, b=func_idx, c=args_start, flags=arg_slots
                func.emit_with_flags(Opcode::Call, arg_offset as u8, dst, func_idx as u16, args_start);
                return Ok(());
            }
        }
        
        // Method not found - could be on underlying type
        return Err(CodegenError::UnsupportedExpr(format!("method {} not found", method_name)));
    }
    
    Ok(())
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "len" | "cap" | "make" | "new" | "append" | "copy" | "delete" | "panic" | "print" | "println")
}

fn compile_builtin_call(
    name: &str,
    call: &vo_syntax::ast::CallExpr,
    dst: u16,
    _ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    match name {
        "len" => {
            if call.args.len() != 1 {
                return Err(CodegenError::Internal("len expects 1 argument".to_string()));
            }
            let arg_reg = compile_expr(&call.args[0], _ctx, func, info)?;
            let arg_type = info.expr_type(call.args[0].id);
            
            // Check type: string, array, slice, map
            if let Some(type_key) = arg_type {
                if info.is_array(type_key) {
                    // Array: len is known at compile time
                    let len = info.array_len(type_key).unwrap_or(0);
                    let (b, c) = encode_i32(len as i32);
                    func.emit_op(Opcode::LoadInt, dst, b, c);
                } else if is_string_type(type_key, info) {
                    func.emit_op(Opcode::StrLen, dst, arg_reg, 0);
                } else {
                    // Slice: SliceLen
                    func.emit_op(Opcode::SliceLen, dst, arg_reg, 0);
                }
            } else {
                func.emit_op(Opcode::SliceLen, dst, arg_reg, 0);
            }
        }
        "cap" => {
            if call.args.len() != 1 {
                return Err(CodegenError::Internal("cap expects 1 argument".to_string()));
            }
            let arg_reg = compile_expr(&call.args[0], _ctx, func, info)?;
            func.emit_op(Opcode::SliceCap, dst, arg_reg, 0);
        }
        "print" | "println" => {
            // CallExtern with vo_print/vo_println
            let extern_name = if name == "println" { "vo_println" } else { "vo_print" };
            let extern_id = _ctx.get_or_register_extern(extern_name);
            
            // Compile arguments
            let args_start = func.alloc_temp(call.args.len() as u16);
            for (i, arg) in call.args.iter().enumerate() {
                compile_expr_to(arg, args_start + (i as u16), _ctx, func, info)?;
            }
            
            // CallExtern: a=dst, b=extern_id, c=args_start, flags=arg_count
            func.emit_with_flags(Opcode::CallExtern, call.args.len() as u8, dst, extern_id as u16, args_start);
        }
        "panic" => {
            // Compile panic message
            if !call.args.is_empty() {
                let msg_reg = compile_expr(&call.args[0], _ctx, func, info)?;
                func.emit_op(Opcode::Panic, msg_reg, 0, 0);
            } else {
                func.emit_op(Opcode::Panic, 0, 0, 0);
            }
        }
        "make" => {
            // make([]T, len) or make([]T, len, cap) or make(map[K]V) or make(chan T)
            let result_type = info.expr_type(call.args[0].id);
            
            if let Some(type_key) = result_type {
                if info.is_slice(type_key) {
                    // make([]T, len) or make([]T, len, cap)
                    let elem_slots = info.slice_elem_slots(type_key).unwrap_or(1);
                    let len_reg = if call.args.len() > 1 {
                        compile_expr(&call.args[1], _ctx, func, info)?
                    } else {
                        func.alloc_temp(1)
                    };
                    let cap_reg = if call.args.len() > 2 {
                        compile_expr(&call.args[2], _ctx, func, info)?
                    } else {
                        len_reg
                    };
                    // SliceNew: a=dst, b=len, c=cap, flags=elem_slots
                    func.emit_with_flags(Opcode::SliceNew, elem_slots as u8, dst, len_reg, cap_reg);
                } else if info.is_map(type_key) {
                    // make(map[K]V)
                    func.emit_op(Opcode::MapNew, dst, 0, 0);
                } else if info.is_chan(type_key) {
                    // make(chan T) or make(chan T, cap)
                    let cap_reg = if call.args.len() > 1 {
                        compile_expr(&call.args[1], _ctx, func, info)?
                    } else {
                        let tmp = func.alloc_temp(1);
                        func.emit_op(Opcode::LoadInt, tmp, 0, 0);
                        tmp
                    };
                    func.emit_op(Opcode::ChanNew, dst, cap_reg, 0);
                } else {
                    return Err(CodegenError::UnsupportedExpr("make with unsupported type".to_string()));
                }
            }
        }
        "new" => {
            // new(T) - allocate zero value of T on heap
            let result_type = info.expr_type(call.args[0].id);
            if let Some(type_key) = result_type {
                let slots = info.type_slot_count(type_key);
                // PtrNew: a=dst, b=0 (zero init), c=slots
                func.emit_op(Opcode::PtrNew, dst, 0, slots);
            }
        }
        "append" => {
            // append(slice, elem...) - variadic
            if call.args.len() < 2 {
                return Err(CodegenError::Internal("append requires at least 2 args".to_string()));
            }
            let slice_reg = compile_expr(&call.args[0], _ctx, func, info)?;
            let elem_reg = compile_expr(&call.args[1], _ctx, func, info)?;
            
            let slice_type = info.expr_type(call.args[0].id);
            let elem_slots = slice_type.and_then(|t| info.slice_elem_slots(t)).unwrap_or(1);
            
            // SliceAppend: a=dst, b=slice, c=elem, flags=elem_slots
            func.emit_with_flags(Opcode::SliceAppend, elem_slots as u8, dst, slice_reg, elem_reg);
        }
        "copy" => {
            // copy(dst, src) - use extern for now
            let extern_id = _ctx.get_or_register_extern("vo_copy");
            let args_start = func.alloc_temp(2);
            compile_expr_to(&call.args[0], args_start, _ctx, func, info)?;
            compile_expr_to(&call.args[1], args_start + 1, _ctx, func, info)?;
            func.emit_with_flags(Opcode::CallExtern, 2, dst, extern_id as u16, args_start);
        }
        "delete" => {
            // delete(map, key)
            if call.args.len() != 2 {
                return Err(CodegenError::Internal("delete requires 2 args".to_string()));
            }
            let map_reg = compile_expr(&call.args[0], _ctx, func, info)?;
            let key_reg = compile_expr(&call.args[1], _ctx, func, info)?;
            func.emit_op(Opcode::MapDelete, map_reg, key_reg, 0);
        }
        "close" => {
            // close(chan)
            if call.args.len() != 1 {
                return Err(CodegenError::Internal("close requires 1 arg".to_string()));
            }
            let chan_reg = compile_expr(&call.args[0], _ctx, func, info)?;
            func.emit_op(Opcode::ChanClose, chan_reg, 0, 0);
        }
        _ => {
            return Err(CodegenError::UnsupportedExpr(format!("builtin {}", name)));
        }
    }
    
    Ok(())
}

// === Composite literal ===

fn compile_composite_lit(
    expr: &Expr,
    lit: &vo_syntax::ast::CompositeLit,
    dst: u16,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    let type_key = info.expr_type(expr.id)
        .ok_or_else(|| CodegenError::Internal("composite lit has no type".to_string()))?;
    
    if info.is_struct(type_key) {
        // Struct literal: initialize fields
        let total_slots = info.type_slot_count(type_key);
        
        // Zero-initialize all slots first
        for i in 0..total_slots {
            func.emit_op(Opcode::LoadNil, dst + i, 0, 0);
        }
        
        // Initialize specified fields
        for elem in &lit.elems {
            if let Some(key) = &elem.key {
                // Named field: key is field name
                if let vo_syntax::ast::CompositeLitKey::Ident(field_ident) = key {
                    let field_name = info.project.interner.resolve(field_ident.symbol)
                        .ok_or_else(|| CodegenError::Internal("cannot resolve field name".to_string()))?;
                    
                    let (offset, _slots) = info.struct_field_offset(type_key, field_name)
                        .ok_or_else(|| CodegenError::Internal(format!("field {} not found", field_name)))?;
                    
                    compile_expr_to(&elem.value, dst + offset, ctx, func, info)?;
                }
            }
        }
    } else if info.is_array(type_key) {
        // Array literal
        let elem_slots = info.array_elem_slots(type_key).unwrap_or(1);
        
        for (i, elem) in lit.elems.iter().enumerate() {
            let offset = (i as u16) * elem_slots;
            compile_expr_to(&elem.value, dst + offset, ctx, func, info)?;
        }
    } else {
        return Err(CodegenError::UnsupportedExpr("composite literal for non-struct/array".to_string()));
    }
    
    Ok(())
}

// === Helpers ===

/// Get constant int value from type info
fn get_const_int(expr_id: vo_common_core::ExprId, info: &TypeInfoWrapper) -> Option<i64> {
    let tv = info.project.type_info.types.get(&expr_id)?;
    if let vo_analysis::operand::OperandMode::Constant(val) = &tv.mode {
        val.int_val()
    } else {
        None
    }
}

/// Get constant float value from type info
fn get_const_float(expr_id: vo_common_core::ExprId, info: &TypeInfoWrapper) -> Option<f64> {
    let tv = info.project.type_info.types.get(&expr_id)?;
    if let vo_analysis::operand::OperandMode::Constant(val) = &tv.mode {
        Some(vo_analysis::constant::float64_val(val).0)
    } else {
        None
    }
}

/// Get constant string value from type info
fn get_const_string(expr_id: vo_common_core::ExprId, info: &TypeInfoWrapper) -> Option<String> {
    let tv = info.project.type_info.types.get(&expr_id)?;
    if let vo_analysis::operand::OperandMode::Constant(val) = &tv.mode {
        Some(vo_analysis::constant::string_val(val).to_string())
    } else {
        None
    }
}

fn encode_i32(val: i32) -> (u16, u16) {
    let bits = val as u32;
    ((bits & 0xFFFF) as u16, ((bits >> 16) & 0xFFFF) as u16)
}

fn is_float_type(type_key: vo_analysis::objects::TypeKey, info: &TypeInfoWrapper) -> bool {
    use vo_analysis::typ::{underlying_type, BasicType, Type};
    let underlying = underlying_type(type_key, &info.project.tc_objs);
    if let Type::Basic(b) = &info.project.tc_objs.types[underlying] {
        matches!(b.typ(), BasicType::Float32 | BasicType::Float64 | BasicType::UntypedFloat)
    } else {
        false
    }
}

fn is_string_type(type_key: vo_analysis::objects::TypeKey, info: &TypeInfoWrapper) -> bool {
    use vo_analysis::typ::{underlying_type, BasicType, Type};
    let underlying = underlying_type(type_key, &info.project.tc_objs);
    if let Type::Basic(b) = &info.project.tc_objs.types[underlying] {
        matches!(b.typ(), BasicType::Str | BasicType::UntypedString)
    } else {
        false
    }
}
