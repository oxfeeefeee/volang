//! Instruction translation: individual bytecode -> Cranelift IR.
//!
//! This module contains the translation methods for each bytecode instruction.
//! All methods are implemented on FunctionCompiler.

use cranelift_codegen::ir::{types, InstBuilder, Value};
use cranelift_codegen::ir::condcodes::{IntCC, FloatCC};

use vo_runtime::instruction::Instruction;

use crate::compiler::FunctionCompiler;

impl FunctionCompiler<'_> {
    // =========================================================================
    // Load instructions
    // =========================================================================

    pub(crate) fn translate_load_int(&mut self, inst: &Instruction) {
        let val = self.builder.ins().iconst(types::I64, inst.imm32() as i64);
        self.write_var(inst.a, val);
    }

    pub(crate) fn translate_load_const(&mut self, inst: &Instruction) {
        // TODO: Load constant from module.constants
        // For now, just load 0 as placeholder
        let val = self.builder.ins().iconst(types::I64, 0);
        self.write_var(inst.a, val);
    }

    // =========================================================================
    // Copy instructions
    // =========================================================================

    pub(crate) fn translate_copy(&mut self, inst: &Instruction) {
        let val = self.read_var(inst.b);
        self.write_var(inst.a, val);
    }

    pub(crate) fn translate_copy_n(&mut self, inst: &Instruction) {
        // Copy N slots from b to a
        let n = inst.c as usize;
        for i in 0..n {
            let val = self.read_var(inst.b + i as u16);
            self.write_var(inst.a + i as u16, val);
        }
    }

    // =========================================================================
    // Integer arithmetic
    // =========================================================================

    pub(crate) fn translate_add_i(&mut self, inst: &Instruction) {
        // AddI dst, src1, src2: dst = src1 + src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let result = self.builder.ins().iadd(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_sub_i(&mut self, inst: &Instruction) {
        // SubI dst, src1, src2: dst = src1 - src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let result = self.builder.ins().isub(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_mul_i(&mut self, inst: &Instruction) {
        // MulI dst, src1, src2: dst = src1 * src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let result = self.builder.ins().imul(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_div_i(&mut self, inst: &Instruction) {
        // DivI dst, src1, src2: dst = src1 / src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        // TODO: Add division by zero check?
        let result = self.builder.ins().sdiv(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_mod_i(&mut self, inst: &Instruction) {
        // ModI dst, src1, src2: dst = src1 % src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let result = self.builder.ins().srem(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_neg_i(&mut self, inst: &Instruction) {
        // NegI dst, src: dst = -src
        let a = self.read_var(inst.b);
        let result = self.builder.ins().ineg(a);
        self.write_var(inst.a, result);
    }

    // =========================================================================
    // Float arithmetic
    // =========================================================================

    pub(crate) fn translate_add_f(&mut self, inst: &Instruction) {
        // AddF dst, src1, src2: dst = src1 + src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        // Reinterpret i64 as f64
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let b_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), b);
        let result = self.builder.ins().fadd(a_f, b_f);
        let result_i = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), result);
        self.write_var(inst.a, result_i);
    }

    pub(crate) fn translate_sub_f(&mut self, inst: &Instruction) {
        // SubF dst, src1, src2: dst = src1 - src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let b_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), b);
        let result = self.builder.ins().fsub(a_f, b_f);
        let result_i = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), result);
        self.write_var(inst.a, result_i);
    }

    pub(crate) fn translate_mul_f(&mut self, inst: &Instruction) {
        // MulF dst, src1, src2: dst = src1 * src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let b_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), b);
        let result = self.builder.ins().fmul(a_f, b_f);
        let result_i = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), result);
        self.write_var(inst.a, result_i);
    }

    pub(crate) fn translate_div_f(&mut self, inst: &Instruction) {
        // DivF dst, src1, src2: dst = src1 / src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let b_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), b);
        let result = self.builder.ins().fdiv(a_f, b_f);
        let result_i = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), result);
        self.write_var(inst.a, result_i);
    }

    pub(crate) fn translate_neg_f(&mut self, inst: &Instruction) {
        // NegF dst, src: dst = -src
        let a = self.read_var(inst.b);
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let result = self.builder.ins().fneg(a_f);
        let result_i = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), result);
        self.write_var(inst.a, result_i);
    }

    // =========================================================================
    // Integer comparison
    // =========================================================================

    pub(crate) fn translate_cmp_i(&mut self, inst: &Instruction, cc: IntCC) {
        // LeI dst, src1, src2: dst = src1 <= src2
        // inst.a = dst, inst.b = src1, inst.c = src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let cmp = self.builder.ins().icmp(cc, a, b);
        // Convert bool to i64 (0 or 1)
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    // =========================================================================
    // Float comparison
    // =========================================================================

    pub(crate) fn translate_cmp_f(&mut self, inst: &Instruction, cc: FloatCC) {
        // LeF dst, src1, src2: dst = src1 <= src2
        // inst.a = dst, inst.b = src1, inst.c = src2
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let a_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let b_f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), b);
        let cmp = self.builder.ins().fcmp(cc, a_f, b_f);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    // =========================================================================
    // Bitwise operations
    // =========================================================================

    pub(crate) fn translate_and(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().band(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_or(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().bor(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_xor(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().bxor(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_and_not(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let not_b = self.builder.ins().bnot(b);
        let result = self.builder.ins().band(a, not_b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_not(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let result = self.builder.ins().bnot(a);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_shl(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().ishl(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_shr_s(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().sshr(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_shr_u(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let b = self.read_var(inst.b);
        let result = self.builder.ins().ushr(a, b);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_bool_not(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let cmp = self.builder.ins().icmp(IntCC::Equal, a, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    // =========================================================================
    // Control flow
    // =========================================================================

    pub(crate) fn translate_jump(&mut self, inst: &Instruction) {
        // Jump offset is relative to current PC
        let offset = inst.imm32();
        let target_pc = (self.current_pc as i32 + offset) as usize;
        
        // Check if this is a back-edge (loop)
        if target_pc <= self.current_pc {
            self.emit_safepoint();
        }
        
        let target_block = self.blocks[&target_pc];
        self.builder.ins().jump(target_block, &[]);
    }

    pub(crate) fn translate_jump_if(&mut self, inst: &Instruction) {
        let cond = self.read_var(inst.a);
        // Jump offset is relative to current PC
        let offset = inst.imm32();
        let target_pc = (self.current_pc as i32 + offset) as usize;
        
        let target_block = self.blocks[&target_pc];
        let fallthrough_block = self.builder.create_block();
        
        // If cond != 0, jump to target
        let zero = self.builder.ins().iconst(types::I64, 0);
        let cmp = self.builder.ins().icmp(IntCC::NotEqual, cond, zero);
        self.builder.ins().brif(cmp, target_block, &[], fallthrough_block, &[]);
        
        self.builder.switch_to_block(fallthrough_block);
        self.builder.seal_block(fallthrough_block);
    }

    pub(crate) fn translate_jump_if_not(&mut self, inst: &Instruction) {
        let cond = self.read_var(inst.a);
        // Jump offset is relative to current PC
        let offset = inst.imm32();
        let target_pc = (self.current_pc as i32 + offset) as usize;
        
        let target_block = self.blocks[&target_pc];
        let fallthrough_block = self.builder.create_block();
        
        // If cond == 0, jump to target
        let zero = self.builder.ins().iconst(types::I64, 0);
        let cmp = self.builder.ins().icmp(IntCC::Equal, cond, zero);
        self.builder.ins().brif(cmp, target_block, &[], fallthrough_block, &[]);
        
        self.builder.switch_to_block(fallthrough_block);
        self.builder.seal_block(fallthrough_block);
    }

    // =========================================================================
    // Function calls
    // =========================================================================

    pub(crate) fn translate_call(&mut self, inst: &Instruction) {
        // Call via vo_call_vm trampoline
        // All calls go through VM for now (simplifies JIT<->VM interop)
        
        let call_vm_func = match self.call_vm_func {
            Some(f) => f,
            None => return, // No call_vm function registered
        };
        
        self.emit_safepoint();
        
        // Decode instruction fields
        let func_id = (inst.a as u32) | ((inst.flags as u32) << 16);
        let arg_start = inst.b as usize;
        let arg_slots = (inst.c >> 8) as usize;
        let ret_slots = (inst.c & 0xFF) as usize;
        
        // Allocate stack space for args and ret buffers
        let arg_slot = self.builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            (arg_slots * 8) as u32,
            0,
        ));
        let ret_slot = self.builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            (ret_slots * 8) as u32,
            0,
        ));
        
        // Copy args to stack buffer
        for i in 0..arg_slots {
            let val = self.read_var((arg_start + i) as u16);
            self.builder.ins().stack_store(val, arg_slot, (i * 8) as i32);
        }
        
        // Get pointers to buffers
        let arg_ptr = self.builder.ins().stack_addr(types::I64, arg_slot, 0);
        let ret_ptr = self.builder.ins().stack_addr(types::I64, ret_slot, 0);
        
        // Call vo_call_vm(ctx, func_id, args, arg_count, ret, ret_count)
        let ctx = self.get_ctx_param();
        let func_id_val = self.builder.ins().iconst(types::I32, func_id as i64);
        let arg_count_val = self.builder.ins().iconst(types::I32, arg_slots as i64);
        let ret_count_val = self.builder.ins().iconst(types::I32, ret_slots as i64);
        
        let call = self.builder.ins().call(
            call_vm_func,
            &[ctx, func_id_val, arg_ptr, arg_count_val, ret_ptr, ret_count_val],
        );
        let result = self.builder.inst_results(call)[0];
        
        // Check for panic (result != 0)
        let panic_block = self.builder.create_block();
        let continue_block = self.builder.create_block();
        
        self.builder.ins().brif(result, panic_block, &[], continue_block, &[]);
        
        // Panic block: return JitResult::Panic
        self.builder.switch_to_block(panic_block);
        self.builder.seal_block(panic_block);
        let panic_result = self.builder.ins().iconst(types::I32, 1); // JitResult::Panic
        self.builder.ins().return_(&[panic_result]);
        
        // Continue block: copy return values
        self.builder.switch_to_block(continue_block);
        self.builder.seal_block(continue_block);
        
        // Copy return values from buffer to local slots
        for i in 0..ret_slots {
            let val = self.builder.ins().stack_load(types::I64, ret_slot, (i * 8) as i32);
            self.write_var((arg_start + i) as u16, val);
        }
    }

    pub(crate) fn translate_call_extern(&mut self, inst: &Instruction) {
        // TODO: Call external function
        self.emit_safepoint();
        let _ = inst;
    }

    pub(crate) fn translate_call_closure(&mut self, inst: &Instruction) {
        // TODO: Call closure via vo_call_vm
        self.emit_safepoint();
        let _ = inst;
    }

    pub(crate) fn translate_call_iface(&mut self, inst: &Instruction) {
        // TODO: Interface method call via vo_call_vm
        self.emit_safepoint();
        let _ = inst;
    }

    pub(crate) fn translate_return(&mut self, inst: &Instruction) {
        // Copy return values to ret pointer
        let ret_ptr = self.get_ret_param();
        let ret_slots = self.func_def.ret_slots as usize;
        let ret_reg = inst.a as usize;
        
        for i in 0..ret_slots {
            let val = self.read_var((ret_reg + i) as u16);
            let offset = (i * 8) as i32;
            self.builder.ins().store(
                cranelift_codegen::ir::MemFlags::trusted(),
                val,
                ret_ptr,
                offset,
            );
        }
        
        // Return JitResult::Ok (0)
        let ok = self.builder.ins().iconst(types::I32, 0);
        self.builder.ins().return_(&[ok]);
    }

    // =========================================================================
    // Global variables
    // =========================================================================

    pub(crate) fn translate_global_get(&mut self, inst: &Instruction) {
        // TODO: Load from globals array via JitContext
        let _ = inst;
    }

    pub(crate) fn translate_global_get_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_global_set(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_global_set_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Pointer operations (heap access)
    // =========================================================================

    pub(crate) fn translate_ptr_new(&mut self, inst: &Instruction) {
        // TODO: Call vo_gc_alloc
        let _ = inst;
    }

    pub(crate) fn translate_ptr_get(&mut self, inst: &Instruction) {
        // Load from GcRef
        let ptr = self.read_var(inst.b);
        let offset = (inst.c as i32) * 8;
        let val = self.builder.ins().load(
            types::I64,
            cranelift_codegen::ir::MemFlags::trusted(),
            ptr,
            offset,
        );
        self.write_var(inst.a, val);
    }

    pub(crate) fn translate_ptr_set(&mut self, inst: &Instruction) {
        // Store to GcRef
        let ptr = self.read_var(inst.a);
        let val = self.read_var(inst.b);
        let offset = (inst.c as i32) * 8;
        
        // NOTE: Write barrier integration
        // When storing a GcRef into a heap object during GC marking phase,
        // we need to call vo_gc_write_barrier to maintain tri-color invariant.
        // The check would be:
        //   if gc.is_marking && slot_types[inst.b] == GcRef {
        //       vo_gc_write_barrier(gc, ptr, offset, val)
        //   }
        // For now, we skip the barrier since:
        // 1. vo_gc_write_barrier is not yet implemented
        // 2. We need slot_types info passed to FunctionCompiler
        // TODO: Implement write barrier when GC marking is integrated
        
        self.builder.ins().store(
            cranelift_codegen::ir::MemFlags::trusted(),
            val,
            ptr,
            offset,
        );
    }

    pub(crate) fn translate_ptr_get_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_ptr_set_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Stack slot dynamic access
    // =========================================================================

    pub(crate) fn translate_slot_get(&mut self, inst: &Instruction) {
        // TODO: Dynamic slot access
        let _ = inst;
    }

    pub(crate) fn translate_slot_set(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slot_get_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slot_set_n(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // String operations
    // =========================================================================

    pub(crate) fn translate_str_new(&mut self, inst: &Instruction) {
        // TODO: Call runtime helper
        let _ = inst;
    }

    pub(crate) fn translate_str_len(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_index(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_concat(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_slice(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_eq(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_ne(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_lt(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_le(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_gt(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_ge(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_str_decode_rune(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Array operations
    // =========================================================================

    pub(crate) fn translate_array_new(&mut self, inst: &Instruction) {
        // TODO: Call vo_gc_alloc for array
        let _ = inst;
    }

    pub(crate) fn translate_array_get(&mut self, inst: &Instruction) {
        // Similar to ptr_get but with bounds check
        let _ = inst;
    }

    pub(crate) fn translate_array_set(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Slice operations
    // =========================================================================

    pub(crate) fn translate_slice_new(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_get(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_set(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_len(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_cap(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_slice(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_slice_append(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Map operations
    // =========================================================================

    pub(crate) fn translate_map_new(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_map_get(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_map_set(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_map_delete(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_map_len(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_map_iter_get(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Channel operations (only ChanNew is JIT-able)
    // =========================================================================

    pub(crate) fn translate_chan_new(&mut self, inst: &Instruction) {
        // TODO: Call vo_chan_new
        let _ = inst;
    }

    // =========================================================================
    // Closure operations
    // =========================================================================

    pub(crate) fn translate_closure_new(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_closure_get(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Interface operations
    // =========================================================================

    pub(crate) fn translate_iface_assign(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    pub(crate) fn translate_iface_assert(&mut self, inst: &Instruction) {
        let _ = inst;
    }

    // =========================================================================
    // Type conversion
    // =========================================================================

    pub(crate) fn translate_conv_i2f(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let f = self.builder.ins().fcvt_from_sint(types::F64, a);
        let result = self.builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), f);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_conv_f2i(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        let f = self.builder.ins().bitcast(types::F64, cranelift_codegen::ir::MemFlags::new(), a);
        let result = self.builder.ins().fcvt_to_sint(types::I64, f);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_conv_i32_i64(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        // Sign extend 32-bit to 64-bit
        let truncated = self.builder.ins().ireduce(types::I32, a);
        let result = self.builder.ins().sextend(types::I64, truncated);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_conv_i64_i32(&mut self, inst: &Instruction) {
        let a = self.read_var(inst.a);
        // Truncate 64-bit to 32-bit, then zero-extend back
        let truncated = self.builder.ins().ireduce(types::I32, a);
        let result = self.builder.ins().uextend(types::I64, truncated);
        self.write_var(inst.a, result);
    }

    // =========================================================================
    // Panic
    // =========================================================================

    pub(crate) fn translate_panic(&mut self, inst: &Instruction) {
        // TODO: Call vo_panic(ctx, msg) and return JitResult::Panic
        let _ = inst;
        
        // For now, just return Panic
        let panic = self.builder.ins().iconst(types::I32, 1);
        self.builder.ins().return_(&[panic]);
    }
}
