//! Load instructions: Nop, LoadInt, LoadConst

use crate::bytecode::Constant;
use crate::fiber::Fiber;
use crate::instruction::Instruction;

#[inline]
pub fn exec_nop(_fiber: &mut Fiber, _inst: &Instruction) {}

#[inline]
pub fn exec_load_int(fiber: &mut Fiber, inst: &Instruction) {
    let val = inst.imm32() as i64 as u64;
    fiber.write_reg(inst.a, val);
}

#[inline]
pub fn exec_load_const(fiber: &mut Fiber, inst: &Instruction, constants: &[Constant]) {
    let val = match &constants[inst.b as usize] {
        Constant::Nil => 0,
        Constant::Bool(b) => *b as u64,
        Constant::Int(i) => *i as u64,
        Constant::Float(f) => f.to_bits(),
        Constant::String(_) => 0, // String handled separately via StrNew
    };
    fiber.write_reg(inst.a, val);
}
