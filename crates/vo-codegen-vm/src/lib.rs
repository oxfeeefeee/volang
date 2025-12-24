//! Vo bytecode code generation.
//!
//! This crate compiles type-checked AST to VM bytecode.

mod context;
mod error;
mod expr;
mod func;
mod stmt;
mod type_info;

pub use context::CodegenContext;
pub use error::CodegenError;
pub use func::FuncBuilder;
pub use type_info::TypeInfoWrapper;

use vo_analysis::Project;
use vo_syntax::ast::Decl;
use vo_vm::bytecode::Module;

/// Compile a type-checked project to VM bytecode.
pub fn compile_project(project: &Project) -> Result<Module, CodegenError> {
    let info = TypeInfoWrapper::new(project);
    let pkg_name = "main"; // TODO: get from project
    let mut ctx = CodegenContext::new(pkg_name);
    
    // 1. Register types (StructMeta, InterfaceMeta)
    register_types(project, &mut ctx, &info)?;
    
    // 2. Collect declarations (functions, globals, externs)
    collect_declarations(project, &mut ctx, &info)?;
    
    // 3. Compile functions
    compile_functions(project, &mut ctx, &info)?;
    
    // 4. Generate __init__ and __entry__
    compile_init_and_entry(project, &mut ctx, &info)?;
    
    Ok(ctx.finish())
}

fn register_types(
    _project: &Project,
    _ctx: &mut CodegenContext,
    _info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // TODO: iterate all types, register StructMeta and InterfaceMeta
    Ok(())
}

fn collect_declarations(
    project: &Project,
    ctx: &mut CodegenContext,
    _info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Register all function names first (so calls can find them)
    for file in &project.files {
        for decl in &file.decls {
            if let Decl::Func(func_decl) = decl {
                // Register function (None = no receiver)
                ctx.register_func(None, func_decl.name.symbol);
            }
        }
    }
    Ok(())
}

fn compile_functions(
    project: &Project,
    ctx: &mut CodegenContext,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // Iterate all files and compile function declarations
    for file in &project.files {
        for decl in &file.decls {
            if let Decl::Func(func_decl) = decl {
                compile_func_decl(func_decl, ctx, info)?;
            }
        }
    }
    Ok(())
}

fn compile_func_decl(
    func_decl: &vo_syntax::ast::FuncDecl,
    ctx: &mut CodegenContext,
    info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    let name = info.project.interner.resolve(func_decl.name.symbol)
        .unwrap_or("unknown");
    
    let mut builder = FuncBuilder::new(name);
    
    // Define parameters
    for param in &func_decl.sig.params {
        let type_key = info.project.type_info.type_exprs.get(&param.ty.id).copied();
        let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
        let slot_types = type_key
            .map(|t| info.type_slot_types(t))
            .unwrap_or_else(|| vec![vo_common_core::types::SlotType::Value]);
        for name in &param.names {
            builder.define_param(name.symbol, slots, &slot_types);
        }
    }
    
    // Set return slots
    let mut ret_slots = 0u16;
    for result in &func_decl.sig.results {
        let type_key = info.project.type_info.type_exprs.get(&result.ty.id).copied();
        let slots = type_key.map(|t| info.type_slot_count(t)).unwrap_or(1);
        ret_slots += slots;
    }
    builder.set_ret_slots(ret_slots);
    
    // Compile function body
    if let Some(body) = &func_decl.body {
        stmt::compile_block(body, ctx, &mut builder, info)?;
    }
    
    // Add return if not present at end
    builder.emit_op(vo_vm::instruction::Opcode::Return, 0, 0, 0);
    
    // Build and add to module
    let func_def = builder.build();
    ctx.add_function(func_def);
    
    Ok(())
}

fn compile_init_and_entry(
    _project: &Project,
    _ctx: &mut CodegenContext,
    _info: &TypeInfoWrapper,
) -> Result<(), CodegenError> {
    // TODO: generate __init__ for global var initialization
    // TODO: generate __entry__ that calls init funcs then main
    Ok(())
}
