//! GoX VM bytecode code generator.
//!
//! This crate compiles type-checked GoX AST to VM bytecode.

mod context;
mod error;
mod expr;
mod func;
mod stmt;
mod type_info;

pub use context::CodegenContext;
pub use error::{CodegenError, Result};
pub use func::FuncBuilder;
pub use type_info::TypeInfo;

use gox_analysis::{Project, TypeQuery};
use gox_common_core::SlotType;
use gox_syntax::ast::{Decl, File, FuncDecl};
use gox_vm::bytecode::Module;
use gox_vm::instruction::Opcode;
use std::collections::HashMap;

use crate::expr::compile_expr;
use crate::stmt::compile_stmt;

/// Compile a project to a Module.
pub fn compile_project(project: &Project) -> Result<Module> {
    let pkg_name = project.main_pkg().name().as_deref().unwrap_or("main");
    let mut ctx = CodegenContext::new(pkg_name);

    // Create a TypeQuery for the main package
    let query = project.query();
    
    // Use expr_types from Project
    let info = TypeInfo::new(query, project.expr_types());

    // Collect declarations from all files
    for file in &project.files {
        collect_declarations(file, &info, &mut ctx)?;
    }

    // Compile functions from all files
    for file in &project.files {
        compile_functions(file, &info, &mut ctx)?;
    }

    // Generate init and entry
    compile_init_and_entry_files(&project.files, &info, &mut ctx)?;

    Ok(ctx.finish())
}

fn compile_init_and_entry_files(
    files: &[File],
    info: &TypeInfo,
    ctx: &mut CodegenContext,
) -> Result<()> {
    let mut init_builder = FuncBuilder::new("__init__");

    // Compile global var initializers from all files
    for file in files {
        for decl in &file.decls {
            if let Decl::Var(var) = decl {
                for spec in &var.specs {
                    for (i, name) in spec.names.iter().enumerate() {
                        if i < spec.values.len() {
                            let src = compile_expr(&spec.values[i], ctx, &mut init_builder, info)?;
                            if let Some(idx) = ctx.get_global_index(name.symbol) {
                                init_builder.emit_op(Opcode::SetGlobal, idx as u16, src, 0);
                            }
                        }
                    }
                }
            }
        }
    }

    init_builder.emit_op(Opcode::Return, 0, 0, 0);
    let init_def = init_builder.build();
    ctx.module.add_function(init_def);

    let mut entry_builder = FuncBuilder::new("__entry__");

    let init_idx = ctx.module.functions.len() as u16 - 1;
    entry_builder.emit_op(Opcode::Call, init_idx, 0, 0);

    if let Some(main_idx) = ctx.module.find_function("main") {
        entry_builder.emit_with_flags(Opcode::Call, 0, main_idx as u16, 0, 0);
    }

    entry_builder.emit_op(Opcode::Return, 0, 0, 0);
    let entry_def = entry_builder.build();
    let entry_idx = ctx.module.add_function(entry_def);
    ctx.module.entry_func = entry_idx;

    Ok(())
}

/// Compile a single file to a Module with a TypeQuery.
pub fn compile_file(
    file: &File,
    query: TypeQuery<'_>,
    expr_types: &HashMap<gox_common_core::ExprId, gox_analysis::TypeKey>,
) -> Result<Module> {
    let mut ctx = CodegenContext::new("main");
    let info = TypeInfo::new(query, expr_types);

    collect_declarations(file, &info, &mut ctx)?;
    compile_functions(file, &info, &mut ctx)?;
    compile_init_and_entry(file, &info, &mut ctx)?;

    Ok(ctx.finish())
}

fn collect_declarations(
    file: &File,
    info: &TypeInfo,
    ctx: &mut CodegenContext,
) -> Result<()> {
    for decl in &file.decls {
        match decl {
            Decl::Func(func) => {
                if func.body.is_some() {
                    ctx.register_func(func.name.symbol);
                } else {
                    let name = info.symbol_str(func.name.symbol);
                    let param_slots: u16 = func.sig.params.iter()
                        .map(|p| p.names.len().max(1) as u16)
                        .sum();
                    let ret_slots = func.sig.results.len() as u16;
                    ctx.register_extern(func.name.symbol, name, param_slots, ret_slots);
                }
            }
            Decl::Var(var) => {
                for spec in &var.specs {
                    for name in &spec.names {
                        let name_str = info.symbol_str(name.symbol);
                        // Without type info, use default slot count
                        // TODO: Get actual type from expr_types when available
                        let type_id = 0;
                        let slots = 1;
                        ctx.register_global(name.symbol, name_str, type_id, slots);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn compile_functions(
    file: &File,
    info: &TypeInfo,
    ctx: &mut CodegenContext,
) -> Result<()> {
    for decl in &file.decls {
        if let Decl::Func(func) = decl {
            if func.body.is_some() {
                compile_func_decl(func, info, ctx)?;
            }
        }
    }
    Ok(())
}

fn compile_func_decl(
    func: &FuncDecl,
    info: &TypeInfo,
    ctx: &mut CodegenContext,
) -> Result<()> {
    let name = info.symbol_str(func.name.symbol);
    let mut builder = FuncBuilder::new(name);

    // Define parameters - for now use default slot types
    // TODO: Get actual type from type annotations when expr_types is populated
    for param in &func.sig.params {
        let slot_types = vec![SlotType::Value];
        let slots = slot_types.len() as u16;
        for pname in &param.names {
            builder.define_param(pname.symbol, slots, &slot_types);
        }
    }

    builder.ret_slots = func.sig.results.len() as u16;

    if let Some(body) = &func.body {
        for stmt in &body.stmts {
            compile_stmt(stmt, ctx, &mut builder, info)?;
        }
    }

    if builder.code.is_empty() || builder.code.last().map(|i| i.opcode()) != Some(Opcode::Return) {
        builder.emit_op(Opcode::Return, 0, 0, 0);
    }

    let func_def = builder.build();
    ctx.module.add_function(func_def);

    Ok(())
}

fn compile_init_and_entry(
    file: &File,
    info: &TypeInfo,
    ctx: &mut CodegenContext,
) -> Result<()> {
    let mut init_builder = FuncBuilder::new("__init__");

    for decl in &file.decls {
        if let Decl::Var(var) = decl {
            for spec in &var.specs {
                for (i, name) in spec.names.iter().enumerate() {
                    if i < spec.values.len() {
                        let src = compile_expr(&spec.values[i], ctx, &mut init_builder, info)?;
                        if let Some(idx) = ctx.get_global_index(name.symbol) {
                            init_builder.emit_op(Opcode::SetGlobal, idx as u16, src, 0);
                        }
                    }
                }
            }
        }
    }

    init_builder.emit_op(Opcode::Return, 0, 0, 0);
    let init_def = init_builder.build();
    ctx.module.add_function(init_def);

    let mut entry_builder = FuncBuilder::new("__entry__");

    let init_idx = ctx.module.functions.len() as u16 - 1;
    entry_builder.emit_op(Opcode::Call, init_idx, 0, 0);

    if let Some(main_idx) = ctx.module.find_function("main") {
        entry_builder.emit_with_flags(Opcode::Call, 0, main_idx as u16, 0, 0);
    }

    entry_builder.emit_op(Opcode::Return, 0, 0, 0);
    let entry_def = entry_builder.build();
    let entry_idx = ctx.module.add_function(entry_def);
    ctx.module.entry_func = entry_idx;

    Ok(())
}
