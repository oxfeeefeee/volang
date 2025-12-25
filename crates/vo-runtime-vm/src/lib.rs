//! Runtime support for Vo VM interpreter.
//!
//! Provides external function registry and VM creation helpers.

pub mod extern_fn;

use vo_vm::bytecode::Module;
use vo_vm::vm::Vm;

/// Result of VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmResult {
    Ok,
    Done,
    Yield,
    Panic(String),
}

/// Wrapper around Vm that provides a simpler interface.
pub struct RuntimeVm {
    vm: Vm,
}

impl RuntimeVm {
    pub fn new() -> Self {
        Self { vm: Vm::new() }
    }

    pub fn load_module(&mut self, module: Module) {
        // Register stdlib extern functions based on module's extern definitions
        extern_fn::register_stdlib(&mut self.vm.state.extern_registry, &module);
        self.vm.load(module);
    }

    pub fn run(&mut self) -> VmResult {
        match self.vm.run() {
            Ok(()) => VmResult::Done,
            Err(e) => VmResult::Panic(format!("{:?}", e)),
        }
    }
}

impl Default for RuntimeVm {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new VM with default settings.
pub fn create_vm() -> RuntimeVm {
    RuntimeVm::new()
}

/// Create a new VM with specified stdlib mode.
pub fn create_vm_with_mode(_mode: extern_fn::StdMode) -> RuntimeVm {
    RuntimeVm::new()
}
