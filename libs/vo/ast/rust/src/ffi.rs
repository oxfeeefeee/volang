//! FFI bindings for the ast package.

use std::sync::Mutex;
use vo_ext::prelude::*;
use vo_common::symbol::SymbolInterner;
use vo_runtime::stdlib::error_helper::{write_error_to, write_nil_error};
use vo_syntax::parser;
use vo_syntax::ast::File;

use crate::printer::AstPrinter;

struct ParsedAst {
    file: File,
    interner: SymbolInterner,
}

static NODES: Mutex<Vec<Option<ParsedAst>>> = Mutex::new(Vec::new());

fn store_node(ast: ParsedAst) -> i64 {
    let mut nodes = NODES.lock().unwrap();
    for (i, slot) in nodes.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ast);
            return i as i64;
        }
    }
    let id = nodes.len();
    nodes.push(Some(ast));
    id as i64
}

fn get_node(id: i64) -> Option<std::sync::MutexGuard<'static, Vec<Option<ParsedAst>>>> {
    let nodes = NODES.lock().unwrap();
    let idx = id as usize;
    if idx < nodes.len() && nodes[idx].is_some() {
        Some(nodes)
    } else {
        None
    }
}

fn free_node(id: i64) {
    let mut nodes = NODES.lock().unwrap();
    let idx = id as usize;
    if idx < nodes.len() {
        nodes[idx] = None;
    }
}

const CODE_IO: isize = 2000;

#[vo_extern_ctx("ast", "ParseFile")]
fn ast_parse_file(ctx: &mut ExternCallContext) -> ExternResult {
    let path = ctx.arg_str(slots::ARG_PATH).to_string();
    
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            ctx.ret_any(slots::RET_0, AnySlot::nil());
            write_error_to(ctx, slots::RET_1, CODE_IO, &e.to_string());
            return ExternResult::Ok;
        }
    };
    
    let (file, diag, interner) = parser::parse(&content, 0);
    
    if diag.has_errors() {
        let msg = diag.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join("; ");
        ctx.ret_any(slots::RET_0, AnySlot::nil());
        write_error_to(ctx, slots::RET_1, CODE_IO, &msg);
        return ExternResult::Ok;
    }
    
    let id = store_node(ParsedAst { file, interner });
    ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
    write_nil_error(ctx, slots::RET_1);
    ExternResult::Ok
}

#[vo_extern_ctx("ast", "ParseString")]
fn ast_parse_string(ctx: &mut ExternCallContext) -> ExternResult {
    let code = ctx.arg_str(slots::ARG_CODE).to_string();
    
    let (file, diag, interner) = parser::parse(&code, 0);
    
    if diag.has_errors() {
        let msg = diag.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join("; ");
        ctx.ret_any(slots::RET_0, AnySlot::nil());
        write_error_to(ctx, slots::RET_1, CODE_IO, &msg);
        return ExternResult::Ok;
    }
    
    let id = store_node(ParsedAst { file, interner });
    ctx.ret_any(slots::RET_0, AnySlot::from_i64(id));
    write_nil_error(ctx, slots::RET_1);
    ExternResult::Ok
}

#[vo_extern_ctx("ast", "Print")]
fn ast_print(ctx: &mut ExternCallContext) -> ExternResult {
    let node_id = ctx.arg_any_as_i64(slots::ARG_NODE);
    
    let result = {
        let nodes = NODES.lock().unwrap();
        let idx = node_id as usize;
        if idx < nodes.len() {
            if let Some(ast) = &nodes[idx] {
                let mut printer = AstPrinter::new(&ast.interner);
                Some(printer.print_file(&ast.file))
            } else {
                None
            }
        } else {
            None
        }
    };
    
    match result {
        Some(text) => ctx.ret_str(slots::RET_0, &text),
        None => ctx.ret_str(slots::RET_0, ""),
    }
    ExternResult::Ok
}

#[vo_extern_ctx("ast", "Free")]
fn ast_free(ctx: &mut ExternCallContext) -> ExternResult {
    let node_id = ctx.arg_any_as_i64(slots::ARG_NODE);
    free_node(node_id);
    ExternResult::Ok
}

vo_ext::export_extensions!();
