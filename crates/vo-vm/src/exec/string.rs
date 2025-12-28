//! String instructions: StrNew, StrConcat, StrSlice

use vo_runtime::gc::{Gc, GcRef};
use vo_runtime::objects::string;

use crate::bytecode::Constant;
use crate::fiber::Fiber;
use crate::instruction::Instruction;

#[inline]
pub fn exec_str_new(fiber: &mut Fiber, inst: &Instruction, constants: &[Constant], gc: &mut Gc) {
    if let Constant::String(s) = &constants[inst.b as usize] {
        let str_ref = string::from_rust_str(gc, s);
        fiber.write_reg(inst.a, str_ref as u64);
    } else {
        fiber.write_reg(inst.a, 0);
    }
}

#[inline]
pub fn exec_str_concat(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc) {
    let a = fiber.read_reg(inst.b) as GcRef;
    let b = fiber.read_reg(inst.c) as GcRef;
    let result = string::concat(gc, a, b);
    fiber.write_reg(inst.a, result as u64);
}

#[inline]
pub fn exec_str_slice(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc) {
    let s = fiber.read_reg(inst.b) as GcRef;
    let lo = fiber.read_reg(inst.c) as usize;
    let hi = fiber.read_reg(inst.c + 1) as usize;
    let result = string::slice_of(gc, s, lo, hi);
    fiber.write_reg(inst.a, result as u64);
}
