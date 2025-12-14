//! Bytecode module format and function definitions.

use crate::instruction::Instruction;
use crate::types::TypeMeta;

/// Magic bytes for bytecode files.
pub const MAGIC: [u8; 4] = *b"GOXB";

/// Bytecode version.
pub const VERSION: u32 = 1;

/// Constant value.
#[derive(Clone, Debug)]
pub enum Constant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Constant {
    pub fn as_i64(&self) -> i64 {
        match self {
            Constant::Int(v) => *v,
            Constant::Bool(b) => if *b { 1 } else { 0 },
            _ => panic!("not an int constant"),
        }
    }
    
    pub fn as_f64(&self) -> f64 {
        match self {
            Constant::Float(v) => *v,
            Constant::Int(v) => *v as f64,
            _ => panic!("not a float constant"),
        }
    }
    
    pub fn as_bool(&self) -> bool {
        match self {
            Constant::Bool(v) => *v,
            Constant::Int(v) => *v != 0,
            _ => panic!("not a bool constant"),
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            Constant::String(s) => s,
            _ => panic!("not a string constant"),
        }
    }
}

/// Function definition.
#[derive(Clone, Debug)]
pub struct FunctionDef {
    pub name: String,
    pub param_count: u16,
    pub param_slots: u16,
    pub local_slots: u16,
    pub ret_slots: u16,
    pub code: Vec<Instruction>,
}

impl FunctionDef {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            param_count: 0,
            param_slots: 0,
            local_slots: 0,
            ret_slots: 0,
            code: Vec::new(),
        }
    }
}

/// Native function definition.
#[derive(Clone, Debug)]
pub struct NativeDef {
    pub name: String,
    pub param_slots: u16,
    pub ret_slots: u16,
}

/// Bytecode module.
#[derive(Clone, Debug, Default)]
pub struct Module {
    pub name: String,
    pub types: Vec<TypeMeta>,
    pub constants: Vec<Constant>,
    pub functions: Vec<FunctionDef>,
    pub natives: Vec<NativeDef>,
    pub entry_func: u32,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            types: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
            natives: Vec::new(),
            entry_func: 0,
        }
    }
    
    /// Add a constant and return its index.
    pub fn add_constant(&mut self, c: Constant) -> u16 {
        let idx = self.constants.len();
        self.constants.push(c);
        idx as u16
    }
    
    /// Add a function and return its index.
    pub fn add_function(&mut self, f: FunctionDef) -> u32 {
        let idx = self.functions.len();
        self.functions.push(f);
        idx as u32
    }
    
    /// Add a native function reference and return its index.
    pub fn add_native(&mut self, name: &str, param_slots: u16, ret_slots: u16) -> u32 {
        let idx = self.natives.len();
        self.natives.push(NativeDef {
            name: name.to_string(),
            param_slots,
            ret_slots,
        });
        idx as u32
    }
    
    /// Get function by ID.
    pub fn get_function(&self, id: u32) -> Option<&FunctionDef> {
        self.functions.get(id as usize)
    }
    
    /// Get native by ID.
    pub fn get_native(&self, id: u32) -> Option<&NativeDef> {
        self.natives.get(id as usize)
    }
    
    /// Find function by name.
    pub fn find_function(&self, name: &str) -> Option<u32> {
        self.functions.iter().position(|f| f.name == name).map(|i| i as u32)
    }
    
    /// Find native by name.
    pub fn find_native(&self, name: &str) -> Option<u32> {
        self.natives.iter().position(|n| n.name == name).map(|i| i as u32)
    }
}
