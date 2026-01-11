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

impl CodegenError {
    /// Get the error message without location.
    pub fn message(&self) -> String {
        match self {
            CodegenError::TypeNotFound(name) => format!("type not found: {}", name),
            CodegenError::FunctionNotFound(name) => format!("function not found: {}", name),
            CodegenError::VariableNotFound(name) => format!("variable not found: {}", name),
            CodegenError::InvalidLHS => "invalid left-hand side in assignment".to_string(),
            CodegenError::UnsupportedExpr(msg) => format!("unsupported expression: {}", msg),
            CodegenError::UnsupportedStmt(msg) => format!("unsupported statement: {}", msg),
            CodegenError::Internal(msg) => format!("internal error: {}", msg),
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CodegenError {}
