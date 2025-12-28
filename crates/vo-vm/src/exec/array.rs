//! Array instructions: ArrayNew

use vo_runtime::ValueMeta;
use vo_runtime::gc::Gc;
use vo_runtime::objects::array;

use crate::fiber::Fiber;
use crate::instruction::Instruction;

#[inline]
pub fn exec_array_new(fiber: &mut Fiber, inst: &Instruction, gc: &mut Gc) {
    let meta_raw = fiber.read_reg(inst.b) as u32;
    let elem_meta = ValueMeta::from_raw(meta_raw);
    let len = fiber.read_reg(inst.c) as usize;
    let elem_slots = inst.flags as usize;
    let arr = array::create(gc, elem_meta, elem_slots, len);
    fiber.write_reg(inst.a, arr as u64);
}
