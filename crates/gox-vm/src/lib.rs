//! Custom virtual machine runtime for GoX.
//!
//! This crate provides a register-based bytecode VM with:
//! - Incremental-ready garbage collector (Phase 1: stop-the-world)
//! - Fiber-based concurrency (goroutines)
//! - Full Go-like type system support
//! - Native function interface

pub mod gc;
pub mod types;
pub mod instruction;
pub mod fiber;
pub mod objects;
pub mod bytecode;
pub mod vm;

pub use gc::{Gc, GcRef, GcHeader, GcColor, NULL_REF};
pub use types::{TypeId, TypeMeta, TypeTable, TypeKind, builtin};
pub use instruction::{Instruction, Opcode};
pub use fiber::{Fiber, FiberId, FiberStatus, CallFrame, Scheduler};
pub use bytecode::{Module, FunctionDef, Constant, BytecodeError};
pub use vm::{Vm, VmResult, NativeFn, NativeCtx, NativeRegistry};
