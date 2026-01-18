//! Vo Bytecode - Text format parsing and formatting.
//!
//! This crate provides Rust API for working with Vo bytecode
//! in text and binary formats.

mod format;

#[cfg(feature = "ffi")]
mod ffi;

pub use format::{format_text, parse_text};

// Re-export Module for convenience
pub use vo_vm::bytecode::Module;

/// Error type for bytecode operations.
#[derive(Debug)]
pub enum BytecodeError {
    Io(std::io::Error),
    Parse(String),
    Serialize(String),
}

impl std::fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BytecodeError::Io(e) => write!(f, "IO error: {}", e),
            BytecodeError::Parse(e) => write!(f, "Parse error: {}", e),
            BytecodeError::Serialize(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for BytecodeError {}

impl From<std::io::Error> for BytecodeError {
    fn from(e: std::io::Error) -> Self {
        BytecodeError::Io(e)
    }
}

/// Serialize a module to binary format.
pub fn serialize(module: &Module) -> Vec<u8> {
    module.serialize()
}

/// Deserialize a module from binary format.
pub fn deserialize(data: &[u8]) -> Result<Module, BytecodeError> {
    Module::deserialize(data)
        .map_err(|e| BytecodeError::Serialize(format!("{:?}", e)))
}

/// Save a module to a text file (.vot).
pub fn save_text(module: &Module, path: &str) -> Result<(), BytecodeError> {
    let text = format_text(module);
    std::fs::write(path, text)?;
    Ok(())
}

/// Load a module from a text file (.vot).
pub fn load_text(path: &str) -> Result<Module, BytecodeError> {
    let content = std::fs::read_to_string(path)?;
    parse_text(&content).map_err(BytecodeError::Parse)
}

/// Save a module to a binary file (.vob).
pub fn save_binary(module: &Module, path: &str) -> Result<(), BytecodeError> {
    let bytes = serialize(module);
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Load a module from a binary file (.vob).
pub fn load_binary(path: &str) -> Result<Module, BytecodeError> {
    let bytes = std::fs::read(path)?;
    deserialize(&bytes)
}
