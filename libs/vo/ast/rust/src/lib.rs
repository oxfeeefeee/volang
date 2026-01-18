//! Vo AST - Parse and print Vo ASTs.
//!
//! This crate provides Rust API for parsing Vo source code and printing ASTs.

mod printer;

#[cfg(feature = "ffi")]
mod ffi;

pub use printer::AstPrinter;

use vo_syntax::parser;

/// Error type for AST operations.
#[derive(Debug)]
pub enum AstError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for AstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstError::Io(e) => write!(f, "IO error: {}", e),
            AstError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for AstError {}

impl From<std::io::Error> for AstError {
    fn from(e: std::io::Error) -> Self {
        AstError::Io(e)
    }
}

/// Opaque AST node handle.
pub struct AstNode {
    pub(crate) file: vo_syntax::ast::File,
    pub(crate) interner: vo_common::symbol::SymbolInterner,
}

/// Parse Vo source code from a string.
pub fn parse(code: &str) -> Result<AstNode, AstError> {
    let (file, diag, interner) = parser::parse(code, 0);
    
    if diag.has_errors() {
        let errors: Vec<String> = diag.iter().map(|d| d.message.clone()).collect();
        return Err(AstError::Parse(errors.join("\n")));
    }
    
    Ok(AstNode { file, interner })
}

/// Parse a Vo source file.
pub fn parse_file(path: &str) -> Result<AstNode, AstError> {
    let content = std::fs::read_to_string(path)?;
    parse(&content)
}

/// Print an AST as formatted text.
pub fn print(node: &AstNode) -> String {
    let mut printer = AstPrinter::new(&node.interner);
    printer.print_file(&node.file)
}
