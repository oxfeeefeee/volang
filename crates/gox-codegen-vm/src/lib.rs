//! Custom VM bytecode generation for GoX.
//!
//! This crate compiles type-checked GoX AST to VM bytecode.
//!
//! # Architecture
//!
//! ```text
//! gox-syntax::ast::File + gox-analysis::TypeCheckResult
//!                          │
//!                          ▼
//!                    ┌───────────┐
//!                    │  Codegen  │
//!                    └───────────┘
//!                          │
//!                          ▼
//!                   gox-vm::Module
//! ```
//!
//! # Multi-package compilation
//!
//! For multi-package projects:
//! 1. Packages are compiled in dependency order (dependencies first)
//! 2. Each package's init() functions are called in that order
//! 3. Cross-package calls use qualified names (pkg.Func)

mod types;
mod context;
mod expr;
mod stmt;

use std::collections::HashMap;
use gox_analysis::{Project, TypeCheckResult, TypedPackage};
use gox_common::{Symbol, SymbolInterner};
use gox_syntax::ast::{Decl, File};
use gox_vm::bytecode::{FunctionDef, Module};
use gox_vm::instruction::Opcode;

pub use context::CodegenContext;

/// Compile a type-checked file to bytecode.
pub fn compile(
    file: &File,
    result: &TypeCheckResult,
    interner: &SymbolInterner,
) -> Result<Module, CodegenError> {
    let mut ctx = CodegenContext::new(file, result, interner);
    ctx.compile()
}

/// Compile a multi-package project to bytecode.
/// 
/// Packages are compiled in init order (dependencies first).
/// A module-level $init function is generated that calls all package inits.
pub fn compile_project(project: &Project) -> Result<Module, CodegenError> {
    let mut module = Module::new(&project.main_package);
    
    // Build cross-package function index: "pkg.Func" -> func_idx
    let mut cross_pkg_funcs: HashMap<String, u32> = HashMap::new();
    
    // First pass: collect all function declarations from ALL packages
    // This allows cross-package function resolution
    let mut pkg_func_indices: Vec<HashMap<Symbol, u32>> = Vec::new();
    
    for pkg in &project.packages {
        let mut func_indices: HashMap<Symbol, u32> = HashMap::new();
        
        for file in &pkg.files {
            for decl in &file.decls {
                if let Decl::Func(func) = decl {
                    let func_name = pkg.interner.resolve(func.name.symbol).unwrap_or("");
                    let idx = module.functions.len() as u32;
                    
                    func_indices.insert(func.name.symbol, idx);
                    
                    // Register cross-package name for exported functions
                    if func_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        let qualified_name = format!("{}.{}", pkg.name, func_name);
                        cross_pkg_funcs.insert(qualified_name, idx);
                    }
                    
                    // Also register init functions specially
                    if func_name == "init" {
                        let init_name = format!("{}.$init", pkg.name);
                        cross_pkg_funcs.insert(init_name, idx);
                    }
                    
                    module.functions.push(FunctionDef::new(func_name));
                }
            }
        }
        
        pkg_func_indices.push(func_indices);
    }
    
    // Second pass: compile all function bodies
    for (pkg_idx, pkg) in project.packages.iter().enumerate() {
        let func_indices = &pkg_func_indices[pkg_idx];
        
        for file in &pkg.files {
            for decl in &file.decls {
                if let Decl::Func(func) = decl {
                    let mut ctx = context::CodegenContext::new(file, &pkg.types, &pkg.interner);
                    ctx.func_indices = func_indices.clone();
                    ctx.cross_pkg_funcs = cross_pkg_funcs.clone();
                    
                    let func_def = ctx.compile_func_body(func)?;
                    let idx = func_indices[&func.name.symbol] as usize;
                    module.functions[idx] = func_def;
                }
            }
        }
    }
    
    // Generate module $init function that calls package inits in order
    let module_init_idx = module.functions.len() as u32;
    let module_init = generate_module_init(project, &cross_pkg_funcs);
    module.functions.push(module_init);
    
    // Find main function
    let main_idx = module.functions.iter()
        .position(|f| f.name == "main")
        .ok_or_else(|| CodegenError::Internal("no main function found".to_string()))?;
    
    // Generate entry point that calls $init then main
    let entry_idx = module.functions.len() as u32;
    let entry = generate_entry_point(module_init_idx, main_idx as u32);
    module.functions.push(entry);
    
    module.entry_func = entry_idx;
    
    Ok(module)
}

/// Generate the module $init function that calls all package inits in order.
fn generate_module_init(
    project: &Project,
    cross_pkg_funcs: &HashMap<String, u32>,
) -> FunctionDef {
    use gox_vm::instruction::Instruction;
    
    let mut func = FunctionDef::new("$init");
    func.local_slots = 0;
    
    // Call each package's init in dependency order
    for pkg in &project.packages {
        // Check if package has init() function
        let init_name = format!("{}.$init", pkg.name);
        if let Some(&init_idx) = cross_pkg_funcs.get(&init_name) {
            // Call init: Call func_id=init_idx, arg_start=0, arg_count=0, ret_count=0
            func.code.push(Instruction::with_flags(Opcode::Call, 0, init_idx as u16, 0, 0));
        }
    }
    
    // Return
    func.code.push(Instruction::new(Opcode::Return, 0, 0, 0));
    
    func
}

/// Generate entry point that calls $init then main.
fn generate_entry_point(init_idx: u32, main_idx: u32) -> FunctionDef {
    use gox_vm::instruction::Instruction;
    
    let mut func = FunctionDef::new("$entry");
    func.local_slots = 0;
    
    // Call $init
    func.code.push(Instruction::with_flags(Opcode::Call, 0, init_idx as u16, 0, 0));
    
    // Call main
    func.code.push(Instruction::with_flags(Opcode::Call, 0, main_idx as u16, 0, 0));
    
    // Return
    func.code.push(Instruction::new(Opcode::Return, 0, 0, 0));
    
    func
}

/// Codegen error.
#[derive(Debug)]
pub enum CodegenError {
    /// Unsupported feature.
    Unsupported(String),
    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            CodegenError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use gox_common::{DiagnosticSink, FileId};
    use gox_syntax::parse;
    use gox_analysis::typecheck_file;
    use gox_vm::VmResult;

    fn compile_and_run(source: &str) -> VmResult {
        let file_id = FileId::new(0);
        let (file, _parse_diag, interner) = parse(file_id, source);
        let mut diag = DiagnosticSink::new();
        let result = typecheck_file(&file, &interner, &mut diag);
        
        if diag.has_errors() {
            panic!("Type check errors");
        }
        
        let module = compile(&file, &result, &interner).expect("Compilation failed");
        
        let mut vm = gox_runtime_vm::create_vm();
        vm.load_module(module);
        vm.run()
    }

    #[test]
    fn test_empty_main() {
        let result = compile_and_run(r#"
package main

func main() {
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_simple_arithmetic() {
        let result = compile_and_run(r#"
package main

func main() {
    x := 1 + 2 * 3
    _ = x
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_variable_assignment() {
        let result = compile_and_run(r#"
package main

func main() {
    x := 10
    y := 20
    z := x + y
    _ = z
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_println() {
        let result = compile_and_run(r#"
package main

func main() {
    println(42)
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_if_simple() {
        let result = compile_and_run(r#"
package main

func main() {
    x := 10
    if x > 5 {
        println(1)
    }
    println(2)
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_if_else() {
        let result = compile_and_run(r#"
package main

func main() {
    x := 3
    if x > 5 {
        println(1)
    } else {
        println(2)
    }
    println(3)
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_for_loop() {
        let result = compile_and_run(r#"
package main

func main() {
    for i := 0; i < 3; i = i + 1 {
        println(i)
    }
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }

    #[test]
    fn test_function_call() {
        let result = compile_and_run(r#"
package main

func add(a int, b int) int {
    return a + b
}

func main() {
    x := add(3, 4)
    println(x)
}
"#);
        assert!(matches!(result, VmResult::Done | VmResult::Ok));
    }
}
