//! Vo Launcher - Compile and run Vo programs.

mod compile;
mod run;
mod stdlib;

pub use compile::{compile_source, compile_file, compile_string, CompileError, CompileOutput};
pub use run::{run_module, run_module_with_extensions, run_file, run_file_with_mode, RunMode, RunError, RuntimeError, RuntimeErrorKind};
pub use stdlib::{EmbeddedStdlib, create_resolver};

pub use vo_vm::bytecode::Module;
pub use vo_common_core::debug_info::SourceLoc;
