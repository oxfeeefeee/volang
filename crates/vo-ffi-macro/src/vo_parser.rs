//! Simple Vo function signature parser for extern function validation.
//!
//! This is a minimal parser that only extracts function signatures from .vo files.
//! It does not attempt to parse the full Vo syntax.

use std::path::Path;

/// A parsed Vo function signature.
#[derive(Debug, Clone)]
pub struct VoFuncSig {
    pub name: String,
    pub params: Vec<VoParam>,
    pub results: Vec<VoType>,
    pub is_extern: bool,
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
    Named(String),
    Pointer(Box<VoType>),
    Slice(Box<VoType>),
    Array(usize, Box<VoType>),
}

impl VoType {
    /// Parse a type string.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        match s {
            "int" => Some(VoType::Int),
            "int8" => Some(VoType::Int8),
            "int16" => Some(VoType::Int16),
            "int32" => Some(VoType::Int32),
            "int64" => Some(VoType::Int64),
            "uint" => Some(VoType::Uint),
            "uint8" | "byte" => Some(VoType::Uint8),
            "uint16" => Some(VoType::Uint16),
            "uint32" => Some(VoType::Uint32),
            "uint64" => Some(VoType::Uint64),
            "float32" => Some(VoType::Float32),
            "float64" => Some(VoType::Float64),
            "bool" => Some(VoType::Bool),
            "string" => Some(VoType::String),
            "any" => Some(VoType::Any),
            _ if s.starts_with('*') => {
                VoType::parse(&s[1..]).map(|t| VoType::Pointer(Box::new(t)))
            }
            _ if s.starts_with("[]") => {
                VoType::parse(&s[2..]).map(|t| VoType::Slice(Box::new(t)))
            }
            _ => Some(VoType::Named(s.to_string())),
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
            VoType::Slice(_) => "GcRef",
            VoType::Array(_, _) => "GcRef",
            VoType::Named(_) => "GcRef",
        }
    }
}

/// Find and parse extern functions from a package directory.
pub fn find_extern_func(pkg_dir: &Path, func_name: &str) -> Result<VoFuncSig, String> {
    // Find all .vo files in the directory
    let entries = std::fs::read_dir(pkg_dir)
        .map_err(|e| format!("cannot read directory {:?}: {}", pkg_dir, e))?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "vo").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(sig) = parse_extern_func(&content, func_name) {
                    return Ok(sig);
                }
            }
        }
    }
    
    Err(format!("extern function '{}' not found in {:?}", func_name, pkg_dir))
}

/// Parse a single extern function from Vo source code.
fn parse_extern_func(source: &str, func_name: &str) -> Option<VoFuncSig> {
    // Simple line-by-line search for function declarations
    let lines: Vec<&str> = source.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Look for "func Name(" pattern
        if !trimmed.starts_with("func ") {
            continue;
        }
        
        // Extract function name
        let rest = &trimmed[5..].trim_start();
        let name_end = rest.find('(').unwrap_or(rest.len());
        let name = rest[..name_end].trim();
        
        if name != func_name {
            continue;
        }
        
        // Check if this is an extern function (no body)
        // Look for '{' on this line or following lines
        let mut has_body = false;
        let mut full_sig = trimmed.to_string();
        
        // Collect full signature (may span multiple lines)
        let mut j = i;
        while j < lines.len() {
            let l = lines[j];
            if l.contains('{') {
                has_body = true;
                break;
            }
            if j > i {
                full_sig.push_str(l.trim());
            }
            // Check if signature is complete (ends with ')' or has return type)
            if l.trim().ends_with(')') || l.contains(')') {
                // Check next non-empty line
                let mut k = j + 1;
                while k < lines.len() && lines[k].trim().is_empty() {
                    k += 1;
                }
                if k < lines.len() {
                    let next = lines[k].trim();
                    if next.starts_with('{') {
                        has_body = true;
                    } else if next.starts_with("func ") || next.starts_with("type ") || 
                              next.starts_with("var ") || next.starts_with("const ") ||
                              next.starts_with("//") || next.is_empty() {
                        // Next declaration, no body
                        break;
                    }
                }
                break;
            }
            j += 1;
        }
        
        if has_body {
            continue; // Not an extern function
        }
        
        // Parse the signature
        return parse_func_signature(&full_sig, name);
    }
    
    None
}

/// Parse a function signature string.
fn parse_func_signature(sig: &str, name: &str) -> Option<VoFuncSig> {
    // Format: "func Name(params) returns" or "func Name(params)"
    let rest = sig.strip_prefix("func ")?.trim_start();
    let rest = rest.strip_prefix(name)?.trim_start();
    
    // Find params between ( and )
    let params_start = rest.find('(')?;
    let params_end = rest.find(')')?;
    let params_str = &rest[params_start + 1..params_end];
    
    let params = parse_params(params_str);
    
    // Parse return types (after ')')
    let returns_str = rest[params_end + 1..].trim();
    let results = parse_returns(returns_str);
    
    Some(VoFuncSig {
        name: name.to_string(),
        params,
        results,
        is_extern: true,
    })
}

/// Parse parameter list.
fn parse_params(s: &str) -> Vec<VoParam> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }
    
    let mut params = Vec::new();
    
    // Split by comma (simple, doesn't handle nested types with commas)
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        
        // Format: "name type" or "name1, name2 type" (shared type)
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.len() >= 2 {
            let ty_str = tokens.last().unwrap();
            let ty = VoType::parse(ty_str).unwrap_or(VoType::Any);
            
            for &name in &tokens[..tokens.len() - 1] {
                params.push(VoParam {
                    name: name.trim_end_matches(',').to_string(),
                    ty: ty.clone(),
                });
            }
        } else if tokens.len() == 1 {
            // Just a type (unnamed parameter)
            let ty = VoType::parse(tokens[0]).unwrap_or(VoType::Any);
            params.push(VoParam {
                name: String::new(),
                ty,
            });
        }
    }
    
    params
}

/// Parse return type list.
fn parse_returns(s: &str) -> Vec<VoType> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }
    
    // Check for parenthesized returns: (int, bool)
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        return inner.split(',')
            .filter_map(|t| {
                let t = t.trim();
                // Handle named returns: "err error" -> just get type
                let ty_str = t.split_whitespace().last()?;
                VoType::parse(ty_str)
            })
            .collect();
    }
    
    // Single return type
    if let Some(ty) = VoType::parse(s) {
        vec![ty]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type() {
        assert!(matches!(VoType::parse("int"), Some(VoType::Int)));
        assert!(matches!(VoType::parse("string"), Some(VoType::String)));
        assert!(matches!(VoType::parse("[]byte"), Some(VoType::Slice(_))));
        assert!(matches!(VoType::parse("*int"), Some(VoType::Pointer(_))));
    }

    #[test]
    fn test_parse_extern_func() {
        let source = r#"
package fmt

func Println(s string) int

func helper() {
    // has body
}
"#;
        let sig = parse_extern_func(source, "Println").unwrap();
        assert_eq!(sig.name, "Println");
        assert_eq!(sig.params.len(), 1);
        assert!(matches!(sig.params[0].ty, VoType::String));
        assert_eq!(sig.results.len(), 1);
        assert!(matches!(sig.results[0], VoType::Int));
    }

    #[test]
    fn test_parse_multi_return() {
        let source = "func Divmod(a, b int) (int, int)";
        let sig = parse_func_signature(source, "Divmod").unwrap();
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.results.len(), 2);
    }
}
