//! Vo function signature parser for extern function validation.
//!
//! Uses vo-syntax for proper parsing of .vo files.

use std::path::Path;
use vo_syntax::{self, ast, TypeExprKind};
use vo_common::symbol::SymbolInterner;

/// A parsed Vo function signature.
#[derive(Debug, Clone)]
pub struct VoFuncSig {
    pub name: String,
    pub params: Vec<VoParam>,
    pub results: Vec<VoType>,
}

impl VoFuncSig {
    /// Create a placeholder signature from a Rust function.
    /// Used when Vo signature parsing fails (e.g., variadic functions).
    pub fn from_rust_fn(func: &syn::ItemFn) -> Self {
        let params = func.sig.inputs.iter().filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                let name = if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    ident.ident.to_string()
                } else {
                    String::new()
                };
                let ty = rust_type_to_vo(&pat_type.ty);
                Some(VoParam { name, ty })
            } else {
                None
            }
        }).collect();

        let results = match &func.sig.output {
            syn::ReturnType::Default => Vec::new(),
            syn::ReturnType::Type(_, ty) => {
                if let syn::Type::Tuple(tuple) = &**ty {
                    tuple.elems.iter().map(rust_type_to_vo).collect()
                } else {
                    vec![rust_type_to_vo(ty)]
                }
            }
        };

        Self {
            name: func.sig.ident.to_string(),
            params,
            results,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoImport {
    pub alias: Option<String>,
    pub path: String,
}

fn rust_type_to_vo(ty: &syn::Type) -> VoType {
    match ty {
        syn::Type::Path(p) => {
            let ident = p.path.segments.last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            match ident.as_str() {
                "i64" | "isize" => VoType::Int64,
                "i32" => VoType::Int32,
                "i16" => VoType::Int16,
                "i8" => VoType::Int8,
                "u64" | "usize" => VoType::Uint64,
                "u32" => VoType::Uint32,
                "u16" => VoType::Uint16,
                "u8" => VoType::Uint8,
                "f64" => VoType::Float64,
                "f32" => VoType::Float32,
                "bool" => VoType::Bool,
                "String" => VoType::String,
                _ => VoType::Any,
            }
        }
        syn::Type::Reference(r) => {
            if let syn::Type::Path(p) = &*r.elem {
                let ident = p.path.segments.last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                if ident == "str" {
                    return VoType::String;
                }
            }
            VoType::Any
        }
        _ => VoType::Any,
    }
}

/// A Vo function parameter.
#[derive(Debug, Clone)]
pub struct VoParam {
    pub name: String,
    pub ty: VoType,
}

/// A Vo type.
#[derive(Debug, Clone)]
pub enum VoType {
    // Primitive types
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    Bool,
    String,
    Any,
    // Composite types
    Named(String),
    Pointer(Box<VoType>),
    Slice(Box<VoType>),
    Array(usize, Box<VoType>),
    Map(Box<VoType>, Box<VoType>),
    Chan(ChanDir, Box<VoType>),
    Func(Vec<VoType>, Vec<VoType>),
    /// Variadic parameter: ...T (e.g., ...interface{})
    Variadic(Box<VoType>),
    /// Struct type with field types
    Struct(Vec<VoType>),
}

/// Channel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanDir {
    /// Bidirectional: chan T
    Both,
    /// Send-only: chan<- T
    Send,
    /// Receive-only: <-chan T
    Recv,
}

impl std::fmt::Display for VoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoType::Int => write!(f, "int"),
            VoType::Int8 => write!(f, "int8"),
            VoType::Int16 => write!(f, "int16"),
            VoType::Int32 => write!(f, "int32"),
            VoType::Int64 => write!(f, "int64"),
            VoType::Uint => write!(f, "uint"),
            VoType::Uint8 => write!(f, "uint8"),
            VoType::Uint16 => write!(f, "uint16"),
            VoType::Uint32 => write!(f, "uint32"),
            VoType::Uint64 => write!(f, "uint64"),
            VoType::Float32 => write!(f, "float32"),
            VoType::Float64 => write!(f, "float64"),
            VoType::Bool => write!(f, "bool"),
            VoType::String => write!(f, "string"),
            VoType::Any => write!(f, "any"),
            VoType::Named(name) => write!(f, "{}", name),
            VoType::Pointer(inner) => write!(f, "*{}", inner),
            VoType::Slice(inner) => write!(f, "[]{}", inner),
            VoType::Array(len, inner) => write!(f, "[{}]{}", len, inner),
            VoType::Map(k, v) => write!(f, "map[{}]{}", k, v),
            VoType::Chan(dir, inner) => {
                match dir {
                    ChanDir::Both => write!(f, "chan {}", inner),
                    ChanDir::Send => write!(f, "chan<- {}", inner),
                    ChanDir::Recv => write!(f, "<-chan {}", inner),
                }
            }
            VoType::Func(params, results) => {
                write!(f, "func(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ")")?;
                if !results.is_empty() {
                    if results.len() == 1 {
                        write!(f, " {}", results[0])?;
                    } else {
                        write!(f, " (")?;
                        for (i, r) in results.iter().enumerate() {
                            if i > 0 { write!(f, ", ")?; }
                            write!(f, "{}", r)?;
                        }
                        write!(f, ")")?;
                    }
                }
                Ok(())
            }
            VoType::Variadic(inner) => write!(f, "...{}", inner),
            VoType::Struct(fields) => write!(f, "struct{{{} fields}}", fields.len()),
        }
    }
}

impl VoType {
    /// Get the number of stack slots this type occupies.
    pub fn slot_count(&self, type_aliases: &std::collections::HashMap<String, VoType>) -> u16 {
        match self {
            // Primitive types: 1 slot each
            VoType::Int | VoType::Int8 | VoType::Int16 | VoType::Int32 | VoType::Int64 |
            VoType::Uint | VoType::Uint8 | VoType::Uint16 | VoType::Uint32 | VoType::Uint64 |
            VoType::Float32 | VoType::Float64 | VoType::Bool | VoType::String => 1,
            
            // Any/interface: 2 slots (slot0=metadata, slot1=data)
            VoType::Any => 2,
            
            // Reference types: 1 slot (GcRef)
            VoType::Pointer(_) | VoType::Slice(_) | VoType::Map(_, _) | 
            VoType::Chan(_, _) | VoType::Func(_, _) => 1,
            
            // Array: elem_slots * length
            VoType::Array(len, elem) => {
                let elem_slots = elem.slot_count(type_aliases);
                elem_slots * (*len as u16)
            }
            
            // Named type: resolve alias
            VoType::Named(name) => {
                // Check for well-known types
                match name.as_str() {
                    "error" => 2, // error is an interface
                    _ => {
                        // Try to resolve type alias
                        if let Some(underlying) = type_aliases.get(name) {
                            underlying.slot_count(type_aliases)
                        } else {
                            // Unknown named type, assume it's a reference type (1 slot)
                            // This handles struct types passed by reference
                            1
                        }
                    }
                }
            }
            
            // Variadic: treated as slice (1 slot)
            VoType::Variadic(_) => 1,
            
            // Struct: sum of all field slots
            VoType::Struct(fields) => {
                fields.iter().map(|f| f.slot_count(type_aliases)).sum()
            }
        }
    }

    /// Get the expected Rust type for this Vo type.
    pub fn expected_rust_type(&self) -> &'static str {
        match self {
            VoType::Int | VoType::Int64 => "i64",
            VoType::Int8 => "i8",
            VoType::Int16 => "i16",
            VoType::Int32 => "i32",
            VoType::Uint | VoType::Uint64 => "u64",
            VoType::Uint8 => "u8",
            VoType::Uint16 => "u16",
            VoType::Uint32 => "u32",
            VoType::Float32 => "f32",
            VoType::Float64 => "f64",
            VoType::Bool => "bool",
            VoType::String => "&str",
            VoType::Any => "any",
            VoType::Pointer(_) => "GcRef",
            VoType::Slice(inner) => match inner.as_ref() {
                VoType::Uint8 => "&[u8]",
                _ => "GcRef",
            },
            VoType::Array(_, _) => "GcRef",
            VoType::Map(_, _) => "GcRef",
            VoType::Chan(_, _) => "GcRef",
            VoType::Func(_, _) => "GcRef",
            VoType::Named(_) => "GcRef",
            VoType::Variadic(_) => "variadic",
            VoType::Struct(_) => "struct",
        }
    }

}

/// Parsed struct field information.
#[derive(Debug, Clone)]
pub struct VoStructField {
    pub name: String,
    pub ty: VoType,
}

/// Parsed struct definition.
#[derive(Debug, Clone)]
pub struct VoStructDef {
    #[allow(dead_code)]
    pub name: String,
    pub fields: Vec<VoStructField>,
}

impl VoStructDef {
    /// Calculate field offsets based on type slot counts.
    pub fn field_offsets(&self, type_aliases: &std::collections::HashMap<String, VoType>) -> Vec<u16> {
        let mut offsets = Vec::new();
        let mut current_offset: u16 = 0;
        for field in &self.fields {
            offsets.push(current_offset);
            current_offset += field.ty.slot_count(type_aliases);
        }
        offsets
    }
    
    /// Get total slot count for this struct.
    pub fn total_slots(&self, type_aliases: &std::collections::HashMap<String, VoType>) -> u16 {
        self.fields.iter().map(|f| f.ty.slot_count(type_aliases)).sum()
    }
}

/// Find and parse a struct definition from a package directory using vo-syntax parser.
pub fn find_struct_def(pkg_dir: &Path, struct_name: &str) -> Result<VoStructDef, String> {
    let entries = std::fs::read_dir(pkg_dir)
        .map_err(|e| format!("cannot read directory {:?}: {}", pkg_dir, e))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (file, _diagnostics, interner) = vo_syntax::parse(&content, 0);
                
                for decl in &file.decls {
                    if let ast::Decl::Type(type_decl) = decl {
                        let name = interner.resolve(type_decl.name.symbol).unwrap_or("");
                        if name == struct_name {
                            if let TypeExprKind::Struct(struct_type) = &type_decl.ty.kind {
                                let fields = struct_type.fields.iter()
                                    .flat_map(|field| {
                                        let ty = type_expr_to_vo_type(&field.ty, &interner);
                                        if field.names.is_empty() {
                                            // Embedded field
                                            vec![VoStructField {
                                                name: String::new(),
                                                ty,
                                            }]
                                        } else {
                                            field.names.iter().map(|n| VoStructField {
                                                name: interner.resolve(n.symbol).unwrap_or("").to_string(),
                                                ty: ty.clone(),
                                            }).collect()
                                        }
                                    })
                                    .collect();
                                
                                return Ok(VoStructDef {
                                    name: struct_name.to_string(),
                                    fields,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err(format!("struct '{}' not found in {:?}", struct_name, pkg_dir))
}

/// Parse all type aliases from a package directory.
/// Returns a map from type name to underlying type.
pub fn parse_type_aliases(pkg_dir: &Path) -> std::collections::HashMap<String, VoType> {
    let mut aliases = std::collections::HashMap::new();
    
    let entries = match std::fs::read_dir(pkg_dir) {
        Ok(e) => e,
        Err(_) => return aliases,
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (file, _diagnostics, interner) = vo_syntax::parse(&content, 0);
                parse_type_aliases_from_ast(&file, &mut aliases, &interner);
            }
        }
    }
    
    aliases
}

/// Parse type aliases from AST using vo-syntax parser.
fn parse_type_aliases_from_ast(file: &ast::File, aliases: &mut std::collections::HashMap<String, VoType>, interner: &SymbolInterner) {
    for decl in &file.decls {
        if let ast::Decl::Type(type_decl) = decl {
            if let Some(name) = interner.resolve(type_decl.name.symbol) {
                let ty = type_expr_to_vo_type(&type_decl.ty, interner);
                aliases.insert(name.to_string(), ty);
            }
        }
    }
}

/// Convert vo-syntax TypeExpr to VoType.
fn type_expr_to_vo_type(type_expr: &ast::TypeExpr, interner: &SymbolInterner) -> VoType {
    match &type_expr.kind {
        TypeExprKind::Ident(ident) => {
            let name = interner.resolve(ident.symbol).unwrap_or("");
            match name {
                "int" => VoType::Int,
                "int8" => VoType::Int8,
                "int16" => VoType::Int16,
                "int32" | "rune" => VoType::Int32,
                "int64" => VoType::Int64,
                "uint" => VoType::Uint,
                "uint8" | "byte" => VoType::Uint8,
                "uint16" => VoType::Uint16,
                "uint32" => VoType::Uint32,
                "uint64" => VoType::Uint64,
                "float32" => VoType::Float32,
                "float64" => VoType::Float64,
                "bool" => VoType::Bool,
                "string" => VoType::String,
                "any" => VoType::Any,
                "error" => VoType::Any, // error is interface, 2 slots
                _ => VoType::Named(name.to_string()),
            }
        }
        TypeExprKind::Selector(sel) => {
            let pkg = interner.resolve(sel.pkg.symbol).unwrap_or("");
            let name = interner.resolve(sel.sel.symbol).unwrap_or("");
            VoType::Named(format!("{}.{}", pkg, name))
        }
        TypeExprKind::Array(arr) => {
            // Try to evaluate array length from literal
            let len = match &arr.len.kind {
                ast::ExprKind::IntLit(n) => {
                    // Parse the raw int literal string to get the value
                    interner.resolve(n.raw).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0)
                }
                _ => 0,
            };
            let elem = type_expr_to_vo_type(&arr.elem, interner);
            VoType::Array(len, Box::new(elem))
        }
        TypeExprKind::Slice(elem) => {
            let elem_type = type_expr_to_vo_type(elem, interner);
            VoType::Slice(Box::new(elem_type))
        }
        TypeExprKind::Map(map) => {
            let key = type_expr_to_vo_type(&map.key, interner);
            let value = type_expr_to_vo_type(&map.value, interner);
            VoType::Map(Box::new(key), Box::new(value))
        }
        TypeExprKind::Chan(chan) => {
            let elem = type_expr_to_vo_type(&chan.elem, interner);
            let dir = match chan.dir {
                ast::ChanDir::Both => ChanDir::Both,
                ast::ChanDir::Send => ChanDir::Send,
                ast::ChanDir::Recv => ChanDir::Recv,
            };
            VoType::Chan(dir, Box::new(elem))
        }
        TypeExprKind::Func(func) => {
            let params: Vec<VoType> = func.params.iter()
                .flat_map(|p| {
                    let ty = type_expr_to_vo_type(&p.ty, interner);
                    // Each named param gets its own slot
                    let count = p.names.len().max(1);
                    std::iter::repeat(ty).take(count)
                })
                .collect();
            let results: Vec<VoType> = func.results.iter()
                .map(|r| type_expr_to_vo_type(&r.ty, interner))
                .collect();
            VoType::Func(params, results)
        }
        TypeExprKind::Struct(struct_type) => {
            let field_types: Vec<VoType> = struct_type.fields.iter()
                .flat_map(|field| {
                    let ty = type_expr_to_vo_type(&field.ty, interner);
                    // Each named field gets its own slot, embedded field is 1
                    let count = field.names.len().max(1);
                    std::iter::repeat(ty).take(count)
                })
                .collect();
            VoType::Struct(field_types)
        }
        TypeExprKind::Pointer(inner) => {
            let inner_type = type_expr_to_vo_type(inner, interner);
            VoType::Pointer(Box::new(inner_type))
        }
        TypeExprKind::Interface(_) => VoType::Any, // interface is 2 slots like any
    }
}

/// Find and parse extern functions from a package directory using vo-syntax parser.
pub fn find_extern_func(pkg_dir: &Path, func_name: &str) -> Result<VoFuncSig, String> {
    let entries = std::fs::read_dir(pkg_dir)
        .map_err(|e| format!("cannot read directory {:?}: {}", pkg_dir, e))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (file, _diagnostics, interner) = vo_syntax::parse(&content, 0);
                
                for decl in &file.decls {
                    if let ast::Decl::Func(func_decl) = decl {
                        let name = interner.resolve(func_decl.name.symbol).unwrap_or("");
                        if name == func_name && func_decl.is_extern() {
                            return Ok(func_decl_to_vo_sig(func_decl, &interner));
                        }
                    }
                }
            }
        }
    }
    
    Err(format!("extern function '{}' not found in {:?}", func_name, pkg_dir))
}

/// Convert vo-syntax FuncDecl to VoFuncSig.
fn func_decl_to_vo_sig(func_decl: &ast::FuncDecl, interner: &SymbolInterner) -> VoFuncSig {
    let name = interner.resolve(func_decl.name.symbol).unwrap_or("").to_string();
    
    let params: Vec<VoParam> = func_decl.sig.params.iter()
        .flat_map(|param| {
            let ty = type_expr_to_vo_type(&param.ty, interner);
            if param.names.is_empty() {
                vec![VoParam { name: String::new(), ty }]
            } else {
                param.names.iter().map(|n| VoParam {
                    name: interner.resolve(n.symbol).unwrap_or("").to_string(),
                    ty: ty.clone(),
                }).collect()
            }
        })
        .collect();
    
    // Handle variadic: wrap last param type in Variadic
    let params = if func_decl.sig.variadic && !params.is_empty() {
        let mut params = params;
        if let Some(last) = params.last_mut() {
            last.ty = VoType::Variadic(Box::new(last.ty.clone()));
        }
        params
    } else {
        params
    };
    
    let results: Vec<VoType> = func_decl.sig.results.iter()
        .map(|r| type_expr_to_vo_type(&r.ty, interner))
        .collect();
    
    VoFuncSig { name, params, results }
}

/// Parse imports from a package directory using vo-syntax parser.
pub fn parse_imports(pkg_dir: &Path) -> Vec<VoImport> {
    let mut imports = Vec::new();
    let entries = match std::fs::read_dir(pkg_dir) {
        Ok(e) => e,
        Err(_) => return imports,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (file, _diagnostics, interner) = vo_syntax::parse(&content, 0);
                for import in &file.imports {
                    // StringLit has .value field with the parsed string value
                    let path_str = import.path.value.clone();
                    let alias = import.alias.as_ref().map(|a| interner.resolve(a.symbol).unwrap_or("").to_string());
                    imports.push(VoImport { alias, path: path_str });
                }
            }
        }
    }

    imports
}

/// Find package name from a package directory using vo-syntax parser.
pub fn find_package_name(pkg_dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(pkg_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (file, _diagnostics, interner) = vo_syntax::parse(&content, 0);
                if let Some(pkg) = &file.package {
                    if let Some(name) = interner.resolve(pkg.symbol) {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}

// =============================================================================
// Constant Parsing
// =============================================================================

/// Find and parse a constant value from a package directory.
pub fn find_const(pkg_dir: &Path, const_name: &str) -> Result<i64, String> {
    let entries = std::fs::read_dir(pkg_dir)
        .map_err(|e| format!("cannot read directory {:?}: {}", pkg_dir, e))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(val) = parse_const_value(&content, const_name) {
                    return Ok(val);
                }
            }
        }
    }
    
    Err(format!("const '{}' not found in {:?}", const_name, pkg_dir))
}

/// Parse a constant value from Vo source code.
fn parse_const_value(source: &str, const_name: &str) -> Option<i64> {
    let lines: Vec<&str> = source.lines().collect();
    let mut in_const_block = false;
    
    for line in &lines {
        let trimmed = line.trim();
        
        // Start of const block: "const ("
        if trimmed == "const (" {
            in_const_block = true;
            continue;
        }
        
        // End of const block: ")"
        if in_const_block && trimmed == ")" {
            in_const_block = false;
            continue;
        }
        
        // Single const: "const Name = value" or "const Name type = value"
        if let Some(rest) = trimmed.strip_prefix("const ") {
            if !rest.starts_with('(') {
                if let Some(val) = parse_const_assignment(rest, const_name) {
                    return Some(val);
                }
            }
            continue;
        }
        
        // Inside const block: "Name = value" or "Name type = value"
        if in_const_block {
            if let Some(val) = parse_const_assignment(trimmed, const_name) {
                return Some(val);
            }
        }
    }
    
    None
}

/// Parse a single constant assignment line.
/// Formats: "Name = value", "Name type = value", "Name = value // comment"
fn parse_const_assignment(line: &str, const_name: &str) -> Option<i64> {
    // Remove trailing comment
    let line = line.split("//").next()?.trim();
    
    // Split by '='
    let (name_part, value_part) = line.split_once('=')?;
    let name_part = name_part.trim();
    let value_part = value_part.trim();
    
    // name_part could be "Name" or "Name type"
    let name = name_part.split_whitespace().next()?;
    
    if name != const_name {
        return None;
    }
    
    eval_const_expr(value_part)
}

/// Evaluate a simple constant expression.
fn eval_const_expr(expr: &str) -> Option<i64> {
    let expr = expr.trim();
    
    // Empty
    if expr.is_empty() {
        return None;
    }
    
    // Parenthesized expression
    if expr.starts_with('(') && expr.ends_with(')') {
        return eval_const_expr(&expr[1..expr.len()-1]);
    }
    
    // Bit OR: a | b
    if let Some((left, right)) = split_binary_op(expr, '|') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l | r);
    }
    
    // Bit AND: a & b
    if let Some((left, right)) = split_binary_op(expr, '&') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l & r);
    }
    
    // Bit shift left: a << b
    if let Some((left, right)) = expr.split_once("<<") {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l << r);
    }
    
    // Bit shift right: a >> b
    if let Some((left, right)) = expr.split_once(">>") {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l >> r);
    }
    
    // Addition: a + b
    if let Some((left, right)) = split_binary_op(expr, '+') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l + r);
    }
    
    // Subtraction: a - b (careful with negative numbers)
    if let Some((left, right)) = split_binary_op_careful(expr, '-') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l - r);
    }
    
    // Multiplication: a * b
    if let Some((left, right)) = split_binary_op(expr, '*') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        return Some(l * r);
    }
    
    // Division: a / b
    if let Some((left, right)) = split_binary_op(expr, '/') {
        let l = eval_const_expr(left)?;
        let r = eval_const_expr(right)?;
        if r != 0 {
            return Some(l / r);
        }
        return None;
    }
    
    // Unary minus: -x
    if let Some(rest) = expr.strip_prefix('-') {
        let val = eval_const_expr(rest)?;
        return Some(-val);
    }
    
    // Character literal: 'x' or '\n' etc
    if expr.starts_with('\'') && expr.ends_with('\'') {
        return parse_char_literal(expr);
    }
    
    // Hex literal: 0x...
    if let Some(hex) = expr.strip_prefix("0x").or_else(|| expr.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    
    // Octal literal: 0o...
    if let Some(oct) = expr.strip_prefix("0o").or_else(|| expr.strip_prefix("0O")) {
        return i64::from_str_radix(oct, 8).ok();
    }
    
    // Binary literal: 0b...
    if let Some(bin) = expr.strip_prefix("0b").or_else(|| expr.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).ok();
    }
    
    // Integer literal
    expr.parse::<i64>().ok()
}

/// Split by binary operator, but only at top level (not inside parens or quotes).
fn split_binary_op(expr: &str, op: char) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut in_char = false;
    let mut prev_backslash = false;
    
    for (i, c) in expr.char_indices() {
        if in_char {
            if c == '\'' && !prev_backslash {
                in_char = false;
            }
            prev_backslash = c == '\\' && !prev_backslash;
            continue;
        }
        
        match c {
            '\'' => in_char = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if c == op && depth == 0 => {
                return Some((&expr[..i], &expr[i+1..]));
            }
            _ => {}
        }
    }
    None
}

/// Split by '-' but avoid splitting on negative number prefix.
fn split_binary_op_careful(expr: &str, op: char) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut in_char = false;
    let mut prev_backslash = false;
    let chars: Vec<char> = expr.chars().collect();
    
    for (i, &c) in chars.iter().enumerate() {
        if in_char {
            if c == '\'' && !prev_backslash {
                in_char = false;
            }
            prev_backslash = c == '\\' && !prev_backslash;
            continue;
        }
        
        match c {
            '\'' => in_char = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if c == op && depth == 0 && i > 0 => {
                // Make sure it's not a negative prefix (preceded by operator or start)
                // Look for the last non-whitespace character before this position
                let mut prev_idx = i - 1;
                while prev_idx > 0 && chars[prev_idx].is_whitespace() {
                    prev_idx -= 1;
                }
                let prev = chars.get(prev_idx).copied();
                if let Some(p) = prev {
                    if p.is_ascii_digit() || p == ')' || p == '\'' {
                        let byte_pos: usize = expr.char_indices()
                            .nth(i)
                            .map(|(pos, _)| pos)
                            .unwrap_or(0);
                        return Some((&expr[..byte_pos], &expr[byte_pos+1..]));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse character literal like 'x', '\n', '\t', '\\', '\''
fn parse_char_literal(s: &str) -> Option<i64> {
    if s.len() < 3 || !s.starts_with('\'') || !s.ends_with('\'') {
        return None;
    }
    
    let inner = &s[1..s.len()-1];
    
    if inner.starts_with('\\') {
        // Escape sequence
        match inner {
            "\\n" => Some('\n' as i64),
            "\\t" => Some('\t' as i64),
            "\\r" => Some('\r' as i64),
            "\\\\" => Some('\\' as i64),
            "\\'" => Some('\'' as i64),
            "\\\"" => Some('"' as i64),
            "\\0" => Some(0),
            _ => None,
        }
    } else if inner.len() == 1 {
        Some(inner.chars().next()? as i64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_const_expr() {
        assert_eq!(eval_const_expr("3000"), Some(3000));
        assert_eq!(eval_const_expr("0"), Some(0));
        assert_eq!(eval_const_expr("-1"), Some(-1));
        assert_eq!(eval_const_expr("0x1F"), Some(31));
        assert_eq!(eval_const_expr("0xff"), Some(255));
        assert_eq!(eval_const_expr("1 << 3"), Some(8));
        assert_eq!(eval_const_expr("1 << 7"), Some(128));
        assert_eq!(eval_const_expr("'/'"), Some(47));
        assert_eq!(eval_const_expr("'\\n'"), Some(10));
        assert_eq!(eval_const_expr("'\\t'"), Some(9));
        assert_eq!(eval_const_expr("1 + 2"), Some(3));
        assert_eq!(eval_const_expr("10 - 3"), Some(7));
        assert_eq!(eval_const_expr("2 * 3"), Some(6));
        assert_eq!(eval_const_expr("10 / 2"), Some(5));
        assert_eq!(eval_const_expr("1 | 2"), Some(3));
        assert_eq!(eval_const_expr("3 & 1"), Some(1));
    }

    #[test]
    fn test_parse_const_value() {
        let source = r#"
package os

const (
    codeErrInvalid    = 3000 // Invalid argument
    codeErrPermission = 3001 // Permission denied
    O_RDONLY int = 0
    O_APPEND int = 1 << 3
)

const PathSeparator = '/'
"#;
        assert_eq!(parse_const_value(source, "codeErrInvalid"), Some(3000));
        assert_eq!(parse_const_value(source, "codeErrPermission"), Some(3001));
        assert_eq!(parse_const_value(source, "O_RDONLY"), Some(0));
        assert_eq!(parse_const_value(source, "O_APPEND"), Some(8));
        assert_eq!(parse_const_value(source, "PathSeparator"), Some(47));
        assert_eq!(parse_const_value(source, "NotExist"), None);
    }
}
