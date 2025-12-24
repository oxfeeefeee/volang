//! Codegen errors.

use std::fmt;

#[derive(Debug)]
pub enum CodegenError {
    /// Type not found
    TypeNotFound(String),
    /// Function not found
    FunctionNotFound(String),
    /// Variable not found
    VariableNotFound(String),
    /// Invalid left-hand side in assignment
    InvalidLHS,
    /// Unsupported expression
    UnsupportedExpr(String),
    /// Unsupported statement
    UnsupportedStmt(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::TypeNotFound(name) => write!(f, "type not found: {}", name),
            CodegenError::FunctionNotFound(name) => write!(f, "function not found: {}", name),
            CodegenError::VariableNotFound(name) => write!(f, "variable not found: {}", name),
            CodegenError::InvalidLHS => write!(f, "invalid left-hand side in assignment"),
            CodegenError::UnsupportedExpr(msg) => write!(f, "unsupported expression: {}", msg),
            CodegenError::UnsupportedStmt(msg) => write!(f, "unsupported statement: {}", msg),
            CodegenError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}
