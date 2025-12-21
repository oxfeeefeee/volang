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
    
    // Use expr_types and type_expr_types from Project
    let info = TypeInfo::new(query, project.expr_types(), project.type_expr_types());

    // Register all struct and interface types (Pass 1)
    register_types(project, &mut ctx);

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

/// Register all struct and interface types to allocate unique type IDs.
fn register_types(project: &Project, ctx: &mut CodegenContext) {
    use gox_analysis::Type;
    
    let query = project.query();
    
    // Iterate through all types in the project
    for (type_key, ty) in query.iter_types() {
        match ty {
            Type::Struct(_) => {
                ctx.register_struct_type(type_key);
            }
            Type::Interface(_) => {
                ctx.register_interface_type(type_key);
            }
            Type::Named(n) => {
                // Also register named types that wrap structs/interfaces
                if let Some(underlying_key) = n.try_underlying() {
                    let underlying = query.get_type(underlying_key);
                    match underlying {
                        Type::Struct(_) => {
                            ctx.register_struct_type(type_key);
                        }
                        Type::Interface(_) => {
                            ctx.register_interface_type(type_key);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
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

/// Compile a single file to a Module using a Project.
pub fn compile_single_file(project: &Project) -> Result<Module> {
    let file = project.files.first().expect("project must have at least one file");
    let query = project.query();
    let mut ctx = CodegenContext::new("main");
    let info = TypeInfo::new(query, project.expr_types(), project.type_expr_types());

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
                        let value_kind = gox_common_core::ValueKind::Nil as u8;
                        let type_id = 0u16;
                        let slots = 1;
                        ctx.register_global(name.symbol, name_str, value_kind, type_id, slots);
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

    // Define receiver parameter first (for methods)
    // Get receiver type from the function's signature (computed during analysis)
    if let Some(recv) = &func.receiver {
        if let Some(recv_type) = info.func_recv_type(func.name.symbol) {
            let slot_types = info.type_slot_types(recv_type);
            let slots = slot_types.len() as u16;
            builder.define_param(recv.name.symbol, slots, &slot_types);
        } else {
            // Fallback: pointer receiver is always 1 slot (GcRef)
            if recv.is_pointer {
                builder.define_param(recv.name.symbol, 1, &[SlotType::GcRef]);
            } else {
                // Value receiver - look up the base type
                let type_key = info.lookup_type_key(recv.ty.symbol)
                    .expect("receiver type must resolve");
                let recv_type = info.query.get_type(type_key);
                let slot_types = info.type_slot_types(recv_type);
                let slots = slot_types.len() as u16;
                builder.define_param(recv.name.symbol, slots, &slot_types);
            }
        }
    }

    // Define parameters with proper type handling
    // Use analysis-recorded TypeExpr types (handles all types including literals)
    for param in &func.sig.params {
        let param_type = info.resolve_type_expr(&param.ty)
            .expect("param type must be recorded by analysis");
        let is_interface = info.is_interface(param_type);
        
        for pname in &param.names {
            if is_interface {
                // Interface parameter: 2 slots + InitInterface
                let type_key = info.type_expr_type_key(&param.ty).expect("interface type must have TypeKey");
                let iface_type_id = ctx.type_id_for_interface(type_key);
                builder.define_param_interface(pname.symbol, iface_type_id as u32);
            } else {
                let slot_types = info.type_slot_types(param_type);
                let slots = slot_types.len() as u16;
                builder.define_param(pname.symbol, slots, &slot_types);
            }
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
