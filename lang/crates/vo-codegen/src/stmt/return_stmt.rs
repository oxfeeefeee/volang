//! Return and fail statement compilation.

use vo_runtime::SlotType;
use vo_vm::instruction::Opcode;

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::expr::{compile_expr_to, get_expr_source};
use crate::func::{ExprSource, FuncBuilder, StorageKind};
use crate::type_info::TypeInfoWrapper;

/// Emit Return with heap_returns flag for escaped named returns.
/// VM reads per-ref slot counts from FunctionDef.heap_ret_slots.
fn emit_heap_returns(func: &mut FuncBuilder, named_return_slots: &[(u16, u16, bool)]) {
    use vo_common_core::bytecode::RETURN_FLAG_HEAP_RETURNS;
    let gcref_count = named_return_slots.len() as u16;
    let gcref_start = named_return_slots[0].0;
    func.emit_with_flags(Opcode::Return, RETURN_FLAG_HEAP_RETURNS, gcref_start, gcref_count, 0);
}

/// Compile return statement
pub(super) fn compile_return(
    ret: &vo_syntax::ast::ReturnStmt,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    if ret.values.is_empty() {
        // Bare return - use pre-recorded slot info (not affected by shadow variables)
        let named_return_slots: Vec<_> = func.named_return_slots().to_vec();
        if named_return_slots.is_empty() {
            func.emit_op(Opcode::Return, 0, 0, 0);
        } else {
            // Check if ALL named returns are escaped (for defer named return semantics)
            // When all are escaped, we pass GcRefs and let VM read after defer
            let all_escaped = named_return_slots.iter().all(|(_, _, escaped)| *escaped);
            
            if all_escaped {
                emit_heap_returns(func, &named_return_slots);
            } else {
                // Mixed or non-escaped: copy to return area as before
                let total_ret_slots: u16 = named_return_slots.iter().map(|(_, s, _)| *s).sum();
                // Build slot types from named return variables
                let mut ret_slot_types = Vec::new();
                for (_, slots, _) in &named_return_slots {
                    for _ in 0..*slots {
                        ret_slot_types.push(SlotType::Value);  // Conservative: named returns are already properly typed at definition
                    }
                }
                let ret_start = func.alloc_temp_typed(&ret_slot_types);
                let mut offset = 0u16;
                for &(slot, slots, escaped) in &named_return_slots {
                    if escaped {
                        // Escaped variable: slot is GcRef, need PtrGet to read value
                        if slots == 1 {
                            func.emit_op(Opcode::PtrGet, ret_start + offset, slot, 0);
                        } else {
                            func.emit_with_flags(Opcode::PtrGetN, slots as u8, ret_start + offset, slot, 0);
                        }
                    } else {
                        func.emit_copy(ret_start + offset, slot, slots);
                    }
                    offset += slots;
                }
                func.emit_op(Opcode::Return, ret_start, total_ret_slots, 0);
            }
        }
    } else {
        // Check if we have escaped named returns - if so, we need special handling
        // to ensure defer can modify the return values
        let named_return_slots: Vec<_> = func.named_return_slots().to_vec();
        let all_escaped = !named_return_slots.is_empty() 
            && named_return_slots.iter().all(|(_, _, escaped)| *escaped);
        
        if all_escaped {
            // For escaped named returns with explicit return values:
            // 1. Store return values into the heap-allocated named return variables
            // 2. Use heap_returns mode so VM reads from heap after defer
            let ret_types: Vec<_> = func.return_types().to_vec();
            
            // Store each return value into the corresponding named return variable
            for (i, result) in ret.values.iter().enumerate() {
                let (gcref_slot, slots, _) = named_return_slots[i];
                let ret_type = ret_types.get(i).copied();
                
                // Compile value to temp, then store to heap
                let temp_slot_types = ret_type.map(|rt| info.type_slot_types(rt)).unwrap_or_else(|| vec![SlotType::Value; slots as usize]);
                let temp = func.alloc_temp_typed(&temp_slot_types);
                if let Some(rt) = ret_type {
                    crate::assign::emit_assign(temp, crate::assign::AssignSource::Expr(result), rt, ctx, func, info)?;
                } else {
                    compile_expr_to(result, temp, ctx, func, info)?;
                }
                
                // Store to heap: PtrSet gcref[0..slots] = temp
                if slots == 1 {
                    func.emit_op(Opcode::PtrSet, gcref_slot, 0, temp);
                } else {
                    func.emit_with_flags(Opcode::PtrSetN, slots as u8, gcref_slot, 0, temp);
                }
            }
            
            emit_heap_returns(func, &named_return_slots);
        } else {
            // Get function's return types (clone to avoid borrow issues)
            let ret_types: Vec<_> = func.return_types().to_vec();
            
            // Calculate total return slots needed (use declared return types)
            let mut total_ret_slots = 0u16;
            for ret_type in &ret_types {
                total_ret_slots += info.type_slot_count(*ret_type);
            }
            
            // Optimization: single return value that's already in a usable slot
            let optimized = if ret.values.len() == 1 && ret_types.len() == 1 {
                let result = &ret.values[0];
                let ret_type = ret_types[0];
                let expr_type = info.expr_type(result.id);
                
                // Only optimize if types match (no interface conversion needed)
                if expr_type == ret_type {
                    if let ExprSource::Location(StorageKind::StackValue { slot, slots }) = 
                        get_expr_source(result, ctx, func, info) 
                    {
                        // Direct return from existing slot
                        func.emit_op(Opcode::Return, slot, slots, 0);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            
            if !optimized {
                // Standard path: allocate space and compile return values
                let mut ret_slot_types = Vec::new();
                for ret_type in &ret_types {
                    ret_slot_types.extend(info.type_slot_types(*ret_type));
                }
                let ret_start = func.alloc_temp_typed(&ret_slot_types);
                
                // Check for multi-value case: return f() where f() returns a tuple
                let is_multi_value = ret.values.len() == 1 
                    && ret_types.len() >= 2
                    && info.is_tuple(info.expr_type(ret.values[0].id));
                
                if is_multi_value {
                    // return f() where f() returns tuple: compile once, convert each element
                    let tuple = crate::expr::CompiledTuple::compile(&ret.values[0], ctx, func, info)?;
                    let tuple_type = info.expr_type(ret.values[0].id);
                    
                    let mut src_offset = 0u16;
                    let mut dst_offset = 0u16;
                    for i in 0..info.tuple_len(tuple_type) {
                        let elem_type = info.tuple_elem_type(tuple_type, i);
                        let rt = ret_types[i];
                        crate::assign::emit_assign(ret_start + dst_offset, crate::assign::AssignSource::Slot { slot: tuple.base + src_offset, type_key: elem_type }, rt, ctx, func, info)?;
                        src_offset += info.type_slot_count(elem_type);
                        dst_offset += info.type_slot_count(rt);
                    }
                } else {
                    // return a, b, ...: compile each expression with type conversion
                    let mut offset = 0u16;
                    for (i, result) in ret.values.iter().enumerate() {
                        let rt = ret_types[i];
                        crate::assign::emit_assign(ret_start + offset, crate::assign::AssignSource::Expr(result), rt, ctx, func, info)?;
                        offset += info.type_slot_count(rt);
                    }
                }
                func.emit_op(Opcode::Return, ret_start, total_ret_slots, 0);
            }
        }
    }
    Ok(())
}

/// Compile fail statement (return zero values + error)
pub(super) fn compile_fail(
    fail_stmt: &vo_syntax::ast::FailStmt,
    ctx: &mut CodegenContext,
    func: &mut FuncBuilder,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Fail returns zero values for all non-error returns, plus the error value
    // This is equivalent to: return <zero-values>, err
    
    // Get function's return types
    let ret_types: Vec<_> = func.return_types().to_vec();
    
    // Calculate total return slots needed
    let mut total_ret_slots = 0u16;
    for ret_type in &ret_types {
        total_ret_slots += info.type_slot_count(*ret_type);
    }
    
    // Allocate space for return values
    let mut fail_ret_slot_types = Vec::new();
    for ret_type in &ret_types {
        fail_ret_slot_types.extend(info.type_slot_types(*ret_type));
    }
    let ret_start = func.alloc_temp_typed(&fail_ret_slot_types);
    
    // Initialize all slots to zero/nil first
    for i in 0..total_ret_slots {
        func.emit_op(Opcode::LoadInt, ret_start + i, 0, 0);
    }
    
    // Compile the error expression into the last return slot(s)
    // The error is the last return value
    if !ret_types.is_empty() {
        let error_type = *ret_types.last().unwrap();
        let error_slots = info.type_slot_count(error_type);
        let error_start = ret_start + total_ret_slots - error_slots;
        crate::assign::emit_assign(error_start, crate::assign::AssignSource::Expr(&fail_stmt.error), error_type, ctx, func, info)?;
    }
    
    // flags bit 0 = 1 indicates error return (for errdefer)
    func.emit_with_flags(Opcode::Return, 1, ret_start, total_ret_slots, 0);
    Ok(())
}
