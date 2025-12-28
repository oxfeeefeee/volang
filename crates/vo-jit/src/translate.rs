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
        use vo_runtime::bytecode::Constant;
        
        let const_idx = inst.b as usize;
        let val = match &self.vo_module.constants[const_idx] {
            Constant::Nil => self.builder.ins().iconst(types::I64, 0),
            Constant::Bool(b) => self.builder.ins().iconst(types::I64, *b as i64),
            Constant::Int(i) => self.builder.ins().iconst(types::I64, *i),
            Constant::Float(f) => {
                let bits = f.to_bits() as i64;
                self.builder.ins().iconst(types::I64, bits)
            }
            Constant::String(_) => {
                // String constants are handled by StrNew, LoadConst just returns 0
                self.builder.ins().iconst(types::I64, 0)
            }
        };
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
        // CallClosure: a = closure slot, b = arg start, c = (arg_slots << 8) | ret_slots
        let call_closure_func = match self.call_closure_func {
            Some(f) => f,
            None => return,
        };
        
        self.emit_safepoint();
        
        let closure_ref = self.read_var(inst.a);
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
        
        // Copy args to stack buffer (excluding closure)
        for i in 0..arg_slots {
            let val = self.read_var((arg_start + i) as u16);
            self.builder.ins().stack_store(val, arg_slot, (i * 8) as i32);
        }
        
        // Get pointers to buffers
        let arg_ptr = self.builder.ins().stack_addr(types::I64, arg_slot, 0);
        let ret_ptr = self.builder.ins().stack_addr(types::I64, ret_slot, 0);
        
        // Call vo_call_closure(ctx, closure_ref, args, arg_count, ret, ret_count)
        let ctx = self.get_ctx_param();
        let arg_count_val = self.builder.ins().iconst(types::I32, arg_slots as i64);
        let ret_count_val = self.builder.ins().iconst(types::I32, ret_slots as i64);
        
        let call = self.builder.ins().call(
            call_closure_func,
            &[ctx, closure_ref, arg_ptr, arg_count_val, ret_ptr, ret_count_val],
        );
        let result = self.builder.inst_results(call)[0];
        
        // Check for panic
        let panic_block = self.builder.create_block();
        let continue_block = self.builder.create_block();
        
        self.builder.ins().brif(result, panic_block, &[], continue_block, &[]);
        
        self.builder.switch_to_block(panic_block);
        self.builder.seal_block(panic_block);
        let panic_result = self.builder.ins().iconst(types::I32, 1);
        self.builder.ins().return_(&[panic_result]);
        
        self.builder.switch_to_block(continue_block);
        self.builder.seal_block(continue_block);
        
        // Copy return values
        for i in 0..ret_slots {
            let val = self.builder.ins().stack_load(types::I64, ret_slot, (i * 8) as i32);
            self.write_var((arg_start + i) as u16, val);
        }
    }

    pub(crate) fn translate_call_iface(&mut self, inst: &Instruction) {
        // CallIface: a = iface slot (2 slots), b = arg start, c = (arg_slots << 8) | ret_slots, flags = method_idx
        let call_iface_func = match self.call_iface_func {
            Some(f) => f,
            None => return,
        };
        
        self.emit_safepoint();
        
        let iface_slot0 = self.read_var(inst.a);
        let iface_slot1 = self.read_var(inst.a + 1);
        let method_idx = inst.flags as u32;
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
        
        // Extract itab_id from slot0 and lookup func_id
        // For now, we pass 0 as func_id and let runtime resolve it
        // TODO: Access itab table from JitContext for O(1) method lookup
        let itab_id = self.builder.ins().ushr_imm(iface_slot0, 32);
        let itab_id_i32 = self.builder.ins().ireduce(types::I32, itab_id);
        
        // Call vo_call_iface(ctx, slot0, slot1, method_idx, args, arg_count, ret, ret_count, func_id)
        let ctx = self.get_ctx_param();
        let method_idx_val = self.builder.ins().iconst(types::I32, method_idx as i64);
        let arg_count_val = self.builder.ins().iconst(types::I32, arg_slots as i64);
        let ret_count_val = self.builder.ins().iconst(types::I32, ret_slots as i64);
        // func_id = 0 means runtime should resolve it
        let func_id_val = self.builder.ins().iconst(types::I32, 0);
        
        let call = self.builder.ins().call(
            call_iface_func,
            &[ctx, iface_slot0, iface_slot1, method_idx_val, arg_ptr, arg_count_val, ret_ptr, ret_count_val, func_id_val],
        );
        let result = self.builder.inst_results(call)[0];
        
        // Check for panic
        let panic_block = self.builder.create_block();
        let continue_block = self.builder.create_block();
        
        self.builder.ins().brif(result, panic_block, &[], continue_block, &[]);
        
        self.builder.switch_to_block(panic_block);
        self.builder.seal_block(panic_block);
        let panic_result = self.builder.ins().iconst(types::I32, 1);
        self.builder.ins().return_(&[panic_result]);
        
        self.builder.switch_to_block(continue_block);
        self.builder.seal_block(continue_block);
        
        // Copy return values
        for i in 0..ret_slots {
            let val = self.builder.ins().stack_load(types::I64, ret_slot, (i * 8) as i32);
            self.write_var((arg_start + i) as u16, val);
        }
        
        let _ = itab_id_i32; // Will be used for O(1) lookup later
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
        // GlobalGet: a = dst slot, b = global index
        let globals_ptr = self.load_globals_ptr();
        let offset = (inst.b as i32) * 8;
        let val = self.builder.ins().load(
            types::I64,
            cranelift_codegen::ir::MemFlags::trusted(),
            globals_ptr,
            offset,
        );
        self.write_var(inst.a, val);
    }

    pub(crate) fn translate_global_get_n(&mut self, inst: &Instruction) {
        // GlobalGetN: a = dst slot start, b = global index start, flags = count
        let globals_ptr = self.load_globals_ptr();
        let count = inst.flags as usize;
        for i in 0..count {
            let offset = ((inst.b as usize + i) * 8) as i32;
            let val = self.builder.ins().load(
                types::I64,
                cranelift_codegen::ir::MemFlags::trusted(),
                globals_ptr,
                offset,
            );
            self.write_var(inst.a + i as u16, val);
        }
    }

    pub(crate) fn translate_global_set(&mut self, inst: &Instruction) {
        // GlobalSet: a = global index, b = src slot
        let globals_ptr = self.load_globals_ptr();
        let val = self.read_var(inst.b);
        let offset = (inst.a as i32) * 8;
        self.builder.ins().store(
            cranelift_codegen::ir::MemFlags::trusted(),
            val,
            globals_ptr,
            offset,
        );
    }

    pub(crate) fn translate_global_set_n(&mut self, inst: &Instruction) {
        // GlobalSetN: a = global index start, b = src slot start, flags = count
        let globals_ptr = self.load_globals_ptr();
        let count = inst.flags as usize;
        for i in 0..count {
            let val = self.read_var(inst.b + i as u16);
            let offset = ((inst.a as usize + i) * 8) as i32;
            self.builder.ins().store(
                cranelift_codegen::ir::MemFlags::trusted(),
                val,
                globals_ptr,
                offset,
            );
        }
    }

    // =========================================================================
    // Pointer operations (heap access)
    // =========================================================================

    pub(crate) fn translate_ptr_new(&mut self, inst: &Instruction) {
        // PtrNew: a = dst slot, b = meta_raw slot, flags = slots count
        // Call vo_gc_alloc(gc, meta, slots) -> GcRef
        let gc_alloc_func = match self.gc_alloc_func {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let meta_raw = self.read_var(inst.b);
        let meta_i32 = self.builder.ins().ireduce(types::I32, meta_raw);
        let slots = self.builder.ins().iconst(types::I32, inst.flags as i64);
        
        let call = self.builder.ins().call(gc_alloc_func, &[gc_ptr, meta_i32, slots]);
        let gc_ref = self.builder.inst_results(call)[0];
        
        self.write_var(inst.a, gc_ref);
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
        // PtrGetN: a = dst slot start, b = ptr slot, c = offset, flags = count
        let ptr = self.read_var(inst.b);
        let count = inst.flags as usize;
        for i in 0..count {
            let offset = ((inst.c as usize + i) * 8) as i32;
            let val = self.builder.ins().load(
                types::I64,
                cranelift_codegen::ir::MemFlags::trusted(),
                ptr,
                offset,
            );
            self.write_var(inst.a + i as u16, val);
        }
    }

    pub(crate) fn translate_ptr_set_n(&mut self, inst: &Instruction) {
        // PtrSetN: a = ptr slot, b = offset, c = src slot start, flags = count
        let ptr = self.read_var(inst.a);
        let count = inst.flags as usize;
        for i in 0..count {
            let val = self.read_var(inst.c + i as u16);
            let offset = ((inst.b as usize + i) * 8) as i32;
            self.builder.ins().store(
                cranelift_codegen::ir::MemFlags::trusted(),
                val,
                ptr,
                offset,
            );
        }
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
        // StrNew: a = dst, b = const_idx
        // Get string constant and create GcRef via vo_str_new
        use vo_runtime::bytecode::Constant;
        
        let str_new_func = match self.str_funcs.str_new {
            Some(f) => f,
            None => return,
        };
        
        let const_idx = inst.b as usize;
        if let Constant::String(s) = &self.vo_module.constants[const_idx] {
            // Create a global data section for the string bytes
            // For now, we'll embed the string bytes as a series of iconst + stores
            // This is inefficient but works for small strings
            // TODO: Use Cranelift's data sections for constant strings
            
            let gc_ptr = self.load_gc_ptr();
            let len = s.len();
            
            if len == 0 {
                // Empty string is null
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.write_var(inst.a, zero);
            } else {
                // Allocate stack space for string bytes
                let stack_slot = self.builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    len as u32,
                    0,
                ));
                
                // Copy bytes to stack
                let bytes = s.as_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    let byte_val = self.builder.ins().iconst(types::I8, b as i64);
                    self.builder.ins().stack_store(byte_val, stack_slot, i as i32);
                }
                
                // Get pointer to stack data
                let data_ptr = self.builder.ins().stack_addr(types::I64, stack_slot, 0);
                let len_val = self.builder.ins().iconst(types::I64, len as i64);
                
                // Call vo_str_new(gc, data, len)
                let call = self.builder.ins().call(str_new_func, &[gc_ptr, data_ptr, len_val]);
                let result = self.builder.inst_results(call)[0];
                self.write_var(inst.a, result);
            }
        } else {
            let zero = self.builder.ins().iconst(types::I64, 0);
            self.write_var(inst.a, zero);
        }
    }

    pub(crate) fn translate_str_len(&mut self, inst: &Instruction) {
        // StrLen: a = dst, b = str
        let str_len_func = match self.str_funcs.str_len {
            Some(f) => f,
            None => return,
        };
        
        let s = self.read_var(inst.b);
        let call = self.builder.ins().call(str_len_func, &[s]);
        let result = self.builder.inst_results(call)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_index(&mut self, inst: &Instruction) {
        // StrIndex: a = dst, b = str, c = idx
        let str_index_func = match self.str_funcs.str_index {
            Some(f) => f,
            None => return,
        };
        
        let s = self.read_var(inst.b);
        let idx = self.read_var(inst.c);
        let call = self.builder.ins().call(str_index_func, &[s, idx]);
        let result = self.builder.inst_results(call)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_concat(&mut self, inst: &Instruction) {
        // StrConcat: a = dst, b = str1, c = str2
        let str_concat_func = match self.str_funcs.str_concat {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_concat_func, &[gc_ptr, a, b]);
        let result = self.builder.inst_results(call)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_slice(&mut self, inst: &Instruction) {
        // StrSlice: a = dst, b = str, c = lo_slot (hi is c+1)
        let str_slice_func = match self.str_funcs.str_slice {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let s = self.read_var(inst.b);
        let lo = self.read_var(inst.c);
        let hi = self.read_var(inst.c + 1);
        let call = self.builder.ins().call(str_slice_func, &[gc_ptr, s, lo, hi]);
        let result = self.builder.inst_results(call)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_eq(&mut self, inst: &Instruction) {
        // StrEq: a = dst, b = str1, c = str2
        let str_eq_func = match self.str_funcs.str_eq {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_eq_func, &[a, b]);
        let result = self.builder.inst_results(call)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_ne(&mut self, inst: &Instruction) {
        // StrNe: a = dst, b = str1, c = str2
        let str_eq_func = match self.str_funcs.str_eq {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_eq_func, &[a, b]);
        let eq_result = self.builder.inst_results(call)[0];
        // Negate: result = eq_result == 0 ? 1 : 0
        let zero = self.builder.ins().iconst(types::I64, 0);
        let cmp = self.builder.ins().icmp(IntCC::Equal, eq_result, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_lt(&mut self, inst: &Instruction) {
        // StrLt: a = dst, b = str1, c = str2
        let str_cmp_func = match self.str_funcs.str_cmp {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_cmp_func, &[a, b]);
        let cmp_result = self.builder.inst_results(call)[0];
        // result = cmp_result < 0 ? 1 : 0
        let zero = self.builder.ins().iconst(types::I32, 0);
        let cmp = self.builder.ins().icmp(IntCC::SignedLessThan, cmp_result, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_le(&mut self, inst: &Instruction) {
        let str_cmp_func = match self.str_funcs.str_cmp {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_cmp_func, &[a, b]);
        let cmp_result = self.builder.inst_results(call)[0];
        let zero = self.builder.ins().iconst(types::I32, 0);
        let cmp = self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, cmp_result, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_gt(&mut self, inst: &Instruction) {
        let str_cmp_func = match self.str_funcs.str_cmp {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_cmp_func, &[a, b]);
        let cmp_result = self.builder.inst_results(call)[0];
        let zero = self.builder.ins().iconst(types::I32, 0);
        let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThan, cmp_result, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_ge(&mut self, inst: &Instruction) {
        let str_cmp_func = match self.str_funcs.str_cmp {
            Some(f) => f,
            None => return,
        };
        
        let a = self.read_var(inst.b);
        let b = self.read_var(inst.c);
        let call = self.builder.ins().call(str_cmp_func, &[a, b]);
        let cmp_result = self.builder.inst_results(call)[0];
        let zero = self.builder.ins().iconst(types::I32, 0);
        let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, cmp_result, zero);
        let result = self.builder.ins().uextend(types::I64, cmp);
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_str_decode_rune(&mut self, inst: &Instruction) {
        // StrDecodeRune: a = rune_slot, b = str, c = pos
        // Writes: rune at a, width at a+1
        let str_decode_rune_func = match self.str_funcs.str_decode_rune {
            Some(f) => f,
            None => return,
        };
        
        let s = self.read_var(inst.b);
        let pos = self.read_var(inst.c);
        let call = self.builder.ins().call(str_decode_rune_func, &[s, pos]);
        let packed = self.builder.inst_results(call)[0];
        
        // Unpack: rune = packed >> 32, width = packed & 0xFFFFFFFF
        let rune = self.builder.ins().ushr_imm(packed, 32);
        let width = self.builder.ins().band_imm(packed, 0xFFFFFFFF);
        
        self.write_var(inst.a, rune);
        self.write_var(inst.a + 1, width);
    }

    // =========================================================================
    // Array operations
    // =========================================================================

    pub(crate) fn translate_array_new(&mut self, inst: &Instruction) {
        // ArrayNew: a = dst, b = elem_meta_slot, c = len_slot, flags = elem_slots
        // For now, use vo_gc_alloc with array layout
        // TODO: Add vo_array_new helper for proper ArrayHeader setup
        let gc_alloc_func = match self.gc_alloc_func {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let meta_raw = self.read_var(inst.b);
        let meta_i32 = self.builder.ins().ireduce(types::I32, meta_raw);
        let len = self.read_var(inst.c);
        let elem_slots = inst.flags as i64;
        
        // Total slots = ArrayHeader (2 slots) + len * elem_slots
        // ArrayHeader: len (usize), elem_meta+elem_bytes packed
        let header_slots = self.builder.ins().iconst(types::I64, 2);
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots);
        let data_slots = self.builder.ins().imul(len, elem_slots_val);
        let total_slots = self.builder.ins().iadd(header_slots, data_slots);
        let total_slots_i32 = self.builder.ins().ireduce(types::I32, total_slots);
        
        let call = self.builder.ins().call(gc_alloc_func, &[gc_ptr, meta_i32, total_slots_i32]);
        let arr_ref = self.builder.inst_results(call)[0];
        
        // Initialize ArrayHeader: store len at offset 0
        self.builder.ins().store(
            cranelift_codegen::ir::MemFlags::trusted(),
            len,
            arr_ref,
            0,
        );
        
        self.write_var(inst.a, arr_ref);
    }

    pub(crate) fn translate_array_get(&mut self, inst: &Instruction) {
        // ArrayGet: a = dst, b = arr, c = idx, flags = elem_slots
        // Read elem_slots values from arr at offset = ArrayHeader(2) + idx * elem_slots
        let arr = self.read_var(inst.b);
        let idx = self.read_var(inst.c);
        let elem_slots = inst.flags as usize;
        
        // Calculate offset: (2 + idx * elem_slots) * 8
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots as i64);
        let slot_offset = self.builder.ins().imul(idx, elem_slots_val);
        let header_offset = self.builder.ins().iconst(types::I64, 2); // ArrayHeader is 2 slots
        let total_offset = self.builder.ins().iadd(header_offset, slot_offset);
        let byte_offset = self.builder.ins().imul_imm(total_offset, 8);
        
        // Load each element slot
        for i in 0..elem_slots {
            let slot_byte_offset = self.builder.ins().iadd_imm(byte_offset, (i * 8) as i64);
            let addr = self.builder.ins().iadd(arr, slot_byte_offset);
            let val = self.builder.ins().load(
                types::I64,
                cranelift_codegen::ir::MemFlags::trusted(),
                addr,
                0,
            );
            self.write_var(inst.a + i as u16, val);
        }
    }

    pub(crate) fn translate_array_set(&mut self, inst: &Instruction) {
        // ArraySet: a = arr, b = idx, c = src, flags = elem_slots
        let arr = self.read_var(inst.a);
        let idx = self.read_var(inst.b);
        let elem_slots = inst.flags as usize;
        
        // Calculate offset: (2 + idx * elem_slots) * 8
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots as i64);
        let slot_offset = self.builder.ins().imul(idx, elem_slots_val);
        let header_offset = self.builder.ins().iconst(types::I64, 2);
        let total_offset = self.builder.ins().iadd(header_offset, slot_offset);
        let byte_offset = self.builder.ins().imul_imm(total_offset, 8);
        
        // Store each element slot
        for i in 0..elem_slots {
            let val = self.read_var(inst.c + i as u16);
            let slot_byte_offset = self.builder.ins().iadd_imm(byte_offset, (i * 8) as i64);
            let addr = self.builder.ins().iadd(arr, slot_byte_offset);
            self.builder.ins().store(
                cranelift_codegen::ir::MemFlags::trusted(),
                val,
                addr,
                0,
            );
        }
    }

    // =========================================================================
    // Slice operations
    // =========================================================================

    pub(crate) fn translate_slice_new(&mut self, inst: &Instruction) {
        // SliceNew: a = dst, b = elem_meta_slot, c = len_slot (cap at c+1), flags = elem_slots
        // Similar to ArrayNew but creates a SliceData structure
        let gc_alloc_func = match self.gc_alloc_func {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let meta_raw = self.read_var(inst.b);
        let meta_i32 = self.builder.ins().ireduce(types::I32, meta_raw);
        let len = self.read_var(inst.c);
        let cap = self.read_var(inst.c + 1);
        let elem_slots = inst.flags as i64;
        
        // Allocate underlying array: ArrayHeader (2 slots) + cap * elem_slots
        let header_slots = self.builder.ins().iconst(types::I64, 2);
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots);
        let data_slots = self.builder.ins().imul(cap, elem_slots_val);
        let total_arr_slots = self.builder.ins().iadd(header_slots, data_slots);
        let total_arr_slots_i32 = self.builder.ins().ireduce(types::I32, total_arr_slots);
        
        let call = self.builder.ins().call(gc_alloc_func, &[gc_ptr, meta_i32, total_arr_slots_i32]);
        let arr_ref = self.builder.inst_results(call)[0];
        
        // Initialize ArrayHeader len
        self.builder.ins().store(
            cranelift_codegen::ir::MemFlags::trusted(),
            cap,
            arr_ref,
            0,
        );
        
        // Allocate SliceData (3 slots: array, start, len+cap packed)
        let slice_meta = self.builder.ins().iconst(types::I32, 0); // TODO: proper slice meta
        let slice_slots = self.builder.ins().iconst(types::I32, 3);
        let call2 = self.builder.ins().call(gc_alloc_func, &[gc_ptr, slice_meta, slice_slots]);
        let slice_ref = self.builder.inst_results(call2)[0];
        
        // Store SliceData fields
        // slot 0: array ref
        self.builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), arr_ref, slice_ref, 0);
        // slot 1: start (0)
        let zero = self.builder.ins().iconst(types::I64, 0);
        self.builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), zero, slice_ref, 8);
        // slot 2: len (low 32 bits) + cap (high 32 bits)
        let len_i32 = self.builder.ins().ireduce(types::I32, len);
        let cap_i32 = self.builder.ins().ireduce(types::I32, cap);
        let cap_ext = self.builder.ins().uextend(types::I64, cap_i32);
        let cap_shifted = self.builder.ins().ishl_imm(cap_ext, 32);
        let len_ext = self.builder.ins().uextend(types::I64, len_i32);
        let len_cap = self.builder.ins().bor(len_ext, cap_shifted);
        self.builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), len_cap, slice_ref, 16);
        
        self.write_var(inst.a, slice_ref);
    }

    pub(crate) fn translate_slice_get(&mut self, inst: &Instruction) {
        // SliceGet: a = dst, b = slice, c = idx, flags = elem_slots
        // SliceData: array(0), start(8), len_cap(16)
        let s = self.read_var(inst.b);
        let idx = self.read_var(inst.c);
        let elem_slots = inst.flags as usize;
        
        // Load array ref and start from SliceData
        let arr = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 0);
        let start = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 8);
        
        // Calculate offset in array: (start + idx) * elem_slots + ArrayHeader(2)
        let total_idx = self.builder.ins().iadd(start, idx);
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots as i64);
        let slot_offset = self.builder.ins().imul(total_idx, elem_slots_val);
        let header_offset = self.builder.ins().iconst(types::I64, 2);
        let final_offset = self.builder.ins().iadd(header_offset, slot_offset);
        let byte_offset = self.builder.ins().imul_imm(final_offset, 8);
        
        // Load each element slot
        for i in 0..elem_slots {
            let slot_byte_offset = self.builder.ins().iadd_imm(byte_offset, (i * 8) as i64);
            let addr = self.builder.ins().iadd(arr, slot_byte_offset);
            let val = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), addr, 0);
            self.write_var(inst.a + i as u16, val);
        }
    }

    pub(crate) fn translate_slice_set(&mut self, inst: &Instruction) {
        // SliceSet: a = slice, b = idx, c = src, flags = elem_slots
        let s = self.read_var(inst.a);
        let idx = self.read_var(inst.b);
        let elem_slots = inst.flags as usize;
        
        // Load array ref and start from SliceData
        let arr = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 0);
        let start = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 8);
        
        // Calculate offset
        let total_idx = self.builder.ins().iadd(start, idx);
        let elem_slots_val = self.builder.ins().iconst(types::I64, elem_slots as i64);
        let slot_offset = self.builder.ins().imul(total_idx, elem_slots_val);
        let header_offset = self.builder.ins().iconst(types::I64, 2);
        let final_offset = self.builder.ins().iadd(header_offset, slot_offset);
        let byte_offset = self.builder.ins().imul_imm(final_offset, 8);
        
        // Store each element slot
        for i in 0..elem_slots {
            let val = self.read_var(inst.c + i as u16);
            let slot_byte_offset = self.builder.ins().iadd_imm(byte_offset, (i * 8) as i64);
            let addr = self.builder.ins().iadd(arr, slot_byte_offset);
            self.builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val, addr, 0);
        }
    }

    pub(crate) fn translate_slice_len(&mut self, inst: &Instruction) {
        // SliceLen: a = dst, b = slice
        let s = self.read_var(inst.b);
        
        // Check for null slice
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_null = self.builder.ins().icmp(IntCC::Equal, s, zero);
        
        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        self.builder.append_block_param(merge_block, types::I64);
        
        self.builder.ins().brif(is_null, then_block, &[], else_block, &[]);
        
        // Null case: return 0
        self.builder.switch_to_block(then_block);
        self.builder.seal_block(then_block);
        self.builder.ins().jump(merge_block, &[zero]);
        
        // Non-null: load len from SliceData
        self.builder.switch_to_block(else_block);
        self.builder.seal_block(else_block);
        let len_cap = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 16);
        let len = self.builder.ins().band_imm(len_cap, 0xFFFFFFFF);
        self.builder.ins().jump(merge_block, &[len]);
        
        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(merge_block);
        let result = self.builder.block_params(merge_block)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_slice_cap(&mut self, inst: &Instruction) {
        // SliceCap: a = dst, b = slice
        let s = self.read_var(inst.b);
        
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_null = self.builder.ins().icmp(IntCC::Equal, s, zero);
        
        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        self.builder.append_block_param(merge_block, types::I64);
        
        self.builder.ins().brif(is_null, then_block, &[], else_block, &[]);
        
        self.builder.switch_to_block(then_block);
        self.builder.seal_block(then_block);
        self.builder.ins().jump(merge_block, &[zero]);
        
        self.builder.switch_to_block(else_block);
        self.builder.seal_block(else_block);
        let len_cap = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), s, 16);
        let cap = self.builder.ins().ushr_imm(len_cap, 32);
        self.builder.ins().jump(merge_block, &[cap]);
        
        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(merge_block);
        let result = self.builder.block_params(merge_block)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_slice_slice(&mut self, inst: &Instruction) {
        // SliceSlice: a = dst, b = slice/array, c = lo (hi at c+1, max at c+2 if three-index)
        // flags: bit0 = is_array, bit1 = has_max
        // TODO: Implement with runtime helper for complexity
        // For now, mark as not implemented - slice operations through VM
        let _ = inst;
    }

    pub(crate) fn translate_slice_append(&mut self, inst: &Instruction) {
        // SliceAppend: a = dst, b = slice, c = val_src, flags = elem_slots
        // TODO: Implement with runtime helper (needs potential reallocation)
        let _ = inst;
    }

    // =========================================================================
    // Map operations
    // =========================================================================

    pub(crate) fn translate_map_new(&mut self, inst: &Instruction) {
        // MapNew: a = dst, b = packed_meta_slot, c = (key_slots << 8) | val_slots
        // packed_meta = (key_meta << 32) | val_meta
        // TODO: Add vo_map_new FuncRef and call it
        // For now, mark as not implemented
        let _ = inst;
    }

    pub(crate) fn translate_map_get(&mut self, inst: &Instruction) {
        // MapGet: complex - needs runtime helper
        // TODO: Implement with vo_map_get
        let _ = inst;
    }

    pub(crate) fn translate_map_set(&mut self, inst: &Instruction) {
        // MapSet: complex - needs runtime helper
        // TODO: Implement with vo_map_set
        let _ = inst;
    }

    pub(crate) fn translate_map_delete(&mut self, inst: &Instruction) {
        // MapDelete: needs runtime helper
        // TODO: Implement with vo_map_delete
        let _ = inst;
    }

    pub(crate) fn translate_map_len(&mut self, inst: &Instruction) {
        // MapLen: a = dst, b = map
        // Simple - can inline null check + load
        let m = self.read_var(inst.b);
        
        let zero = self.builder.ins().iconst(types::I64, 0);
        let is_null = self.builder.ins().icmp(IntCC::Equal, m, zero);
        
        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        self.builder.append_block_param(merge_block, types::I64);
        
        self.builder.ins().brif(is_null, then_block, &[], else_block, &[]);
        
        // Null: return 0
        self.builder.switch_to_block(then_block);
        self.builder.seal_block(then_block);
        self.builder.ins().jump(merge_block, &[zero]);
        
        // Non-null: load len from MapHeader (offset 0, low 32 bits)
        self.builder.switch_to_block(else_block);
        self.builder.seal_block(else_block);
        let header = self.builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), m, 0);
        let len = self.builder.ins().band_imm(header, 0xFFFFFFFF);
        self.builder.ins().jump(merge_block, &[len]);
        
        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(merge_block);
        let result = self.builder.block_params(merge_block)[0];
        self.write_var(inst.a, result);
    }

    pub(crate) fn translate_map_iter_get(&mut self, inst: &Instruction) {
        // MapIterGet: needs runtime helper for iteration
        // TODO: Implement with vo_map_iter_get
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
        // ClosureNew: a = dst, b = func_id_low, c = capture_count, flags = func_id_high
        // Layout: ClosureHeader (1 slot: func_id + capture_count) + captures
        let gc_alloc_func = match self.gc_alloc_func {
            Some(f) => f,
            None => return,
        };
        
        let gc_ptr = self.load_gc_ptr();
        let func_id = ((inst.flags as u32) << 16) | (inst.b as u32);
        let capture_count = inst.c as usize;
        
        // Total slots = header (1) + capture_count
        let total_slots = 1 + capture_count;
        
        // ValueKind::Closure = 9
        let closure_meta = self.builder.ins().iconst(types::I32, 9);
        let slots_val = self.builder.ins().iconst(types::I32, total_slots as i64);
        
        let call = self.builder.ins().call(gc_alloc_func, &[gc_ptr, closure_meta, slots_val]);
        let closure_ref = self.builder.inst_results(call)[0];
        
        // Initialize ClosureHeader: func_id (low 32) + capture_count (high 32)
        let func_id_val = self.builder.ins().iconst(types::I64, func_id as i64);
        let cap_count_val = self.builder.ins().iconst(types::I64, (capture_count as i64) << 32);
        let header = self.builder.ins().bor(func_id_val, cap_count_val);
        self.builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), header, closure_ref, 0);
        
        self.write_var(inst.a, closure_ref);
    }

    pub(crate) fn translate_closure_get(&mut self, inst: &Instruction) {
        // ClosureGet: a = dst, b = capture_idx
        // Closure is always in slot 0 of current function
        // Layout: [ClosureHeader (1 slot)][captures...]
        let closure = self.read_var(0);
        let capture_idx = inst.b as usize;
        
        // Offset = (1 + capture_idx) * 8 bytes
        let offset = ((1 + capture_idx) * 8) as i32;
        let val = self.builder.ins().load(
            types::I64,
            cranelift_codegen::ir::MemFlags::trusted(),
            closure,
            offset,
        );
        self.write_var(inst.a, val);
    }

    pub(crate) fn translate_closure_set(&mut self, inst: &Instruction) {
        // ClosureSet: a = capture_idx, b = src
        let closure = self.read_var(0);
        let capture_idx = inst.a as usize;
        let val = self.read_var(inst.b);
        
        let offset = ((1 + capture_idx) * 8) as i32;
        self.builder.ins().store(
            cranelift_codegen::ir::MemFlags::trusted(),
            val,
            closure,
            offset,
        );
    }

    // =========================================================================
    // Interface operations
    // =========================================================================

    pub(crate) fn translate_iface_assign(&mut self, inst: &Instruction) {
        // IfaceAssign: a=dst (2 slots), b=src, c=const_idx, flags=value_kind
        // For concrete type -> interface (simple case):
        // slot0 = pack(itab_id, rttid, vk), slot1 = src or clone(src)
        //
        // Complex cases (interface->interface) need runtime itab lookup
        // TODO: Add vo_iface_assign runtime helper for complex cases
        
        use vo_runtime::bytecode::Constant;
        
        let vk = inst.flags;
        let src = self.read_var(inst.b);
        
        // Read packed constant: (rttid << 32) | itab_id
        let const_idx = inst.c as usize;
        let (rttid, itab_id) = if let Constant::Int(packed) = &self.vo_module.constants[const_idx] {
            let rttid = (*packed >> 32) as u32;
            let itab_id = (*packed & 0xFFFFFFFF) as u32;
            (rttid, itab_id)
        } else {
            (0, 0)
        };
        
        // Pack slot0: (itab_id << 32) | (rttid << 8) | vk
        let itab_shifted = (itab_id as u64) << 32;
        let rttid_shifted = (rttid as u64) << 8;
        let slot0_val = itab_shifted | rttid_shifted | (vk as u64);
        let slot0 = self.builder.ins().iconst(types::I64, slot0_val as i64);
        
        // slot1 depends on value kind
        // ValueKind::Struct = 7, Array = 8 need ptr_clone
        // ValueKind::Interface = 11 is complex (skip for now)
        let slot1 = if vk == 7 || vk == 8 {
            // Struct/Array: need deep copy
            // TODO: Call vo_ptr_clone
            // For now, just copy the pointer (incorrect but allows compilation)
            src
        } else if vk == 11 {
            // Interface -> Interface: need runtime support
            // For now, just copy slot1 from source
            self.read_var(inst.b + 1)
        } else {
            // Primitive types: direct copy
            src
        };
        
        self.write_var(inst.a, slot0);
        self.write_var(inst.a + 1, slot1);
    }

    pub(crate) fn translate_iface_assert(&mut self, inst: &Instruction) {
        // IfaceAssert is complex - needs runtime support for type checking
        // TODO: Implement with vo_iface_assert runtime helper
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
