//! Codegen context - manages package-level compilation state.

use std::collections::HashMap;

use gox_analysis::{BasicType, TypeKey};
use gox_common::Symbol;
use gox_common_core::RuntimeTypeId;
use gox_vm::bytecode::{Constant, Module};

fn basic_to_runtime_id(b: BasicType) -> u32 {
    match b {
        BasicType::Bool => RuntimeTypeId::Bool as u32,
        BasicType::Int => RuntimeTypeId::Int as u32,
        BasicType::Int8 => RuntimeTypeId::Int8 as u32,
        BasicType::Int16 => RuntimeTypeId::Int16 as u32,
        BasicType::Int32 | BasicType::Rune => RuntimeTypeId::Int32 as u32,
        BasicType::Int64 => RuntimeTypeId::Int64 as u32,
        BasicType::Uint => RuntimeTypeId::Uint as u32,
        BasicType::Uint8 | BasicType::Byte => RuntimeTypeId::Uint8 as u32,
        BasicType::Uint16 => RuntimeTypeId::Uint16 as u32,
        BasicType::Uint32 => RuntimeTypeId::Uint32 as u32,
        BasicType::Uint64 => RuntimeTypeId::Uint64 as u32,
        BasicType::Uintptr => RuntimeTypeId::Uint as u32,
        BasicType::Float32 => RuntimeTypeId::Float32 as u32,
        BasicType::Float64 => RuntimeTypeId::Float64 as u32,
        BasicType::Str => RuntimeTypeId::String as u32,
        _ => RuntimeTypeId::Nil as u32,
    }
}

/// Package-level codegen context.
pub struct CodegenContext {
    pub module: Module,
    func_indices: HashMap<Symbol, u32>,
    next_func_idx: u32,
    extern_indices: HashMap<Symbol, u32>,
    global_indices: HashMap<Symbol, u32>,
    const_indices: HashMap<ConstKey, u16>,
    // Type ID registry for structs and interfaces
    struct_type_ids: HashMap<TypeKey, u32>,
    interface_type_ids: HashMap<TypeKey, u32>,
    next_struct_id: u32,
    next_interface_id: u32,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ConstKey {
    Int(i64),
    Float(u64),
    String(String),
    Bool(bool),
}

impl CodegenContext {
    pub fn new(name: &str) -> Self {
        Self {
            module: Module::new(name),
            func_indices: HashMap::new(),
            next_func_idx: 0,
            extern_indices: HashMap::new(),
            global_indices: HashMap::new(),
            const_indices: HashMap::new(),
            struct_type_ids: HashMap::new(),
            interface_type_ids: HashMap::new(),
            next_struct_id: RuntimeTypeId::FirstStruct as u32,
            next_interface_id: RuntimeTypeId::FirstInterface as u32,
        }
    }

    // === Type ID management ===

    pub fn register_struct_type(&mut self, type_key: TypeKey) -> u32 {
        if let Some(&id) = self.struct_type_ids.get(&type_key) {
            return id;
        }
        let id = self.next_struct_id;
        self.struct_type_ids.insert(type_key, id);
        self.next_struct_id += 1;
        id
    }

    pub fn get_struct_type_id(&self, type_key: TypeKey) -> Option<u32> {
        self.struct_type_ids.get(&type_key).copied()
    }

    pub fn register_interface_type(&mut self, type_key: TypeKey) -> u32 {
        if let Some(&id) = self.interface_type_ids.get(&type_key) {
            return id;
        }
        let id = self.next_interface_id;
        self.interface_type_ids.insert(type_key, id);
        self.next_interface_id += 1;
        id
    }

    pub fn get_interface_type_id(&self, type_key: TypeKey) -> Option<u32> {
        self.interface_type_ids.get(&type_key).copied()
    }

    /// Get runtime type ID, using registered IDs for struct/interface types.
    pub fn runtime_type_id(&self, ty: &gox_analysis::Type, type_key: Option<TypeKey>) -> u32 {
        use gox_analysis::Type;
        use gox_common_core::RuntimeTypeId;

        match ty {
            Type::Struct(_) => {
                let key = type_key.expect("struct type must have TypeKey");
                self.get_struct_type_id(key).expect("struct type must be registered")
            }
            Type::Interface(_) => {
                let key = type_key.expect("interface type must have TypeKey");
                self.get_interface_type_id(key).expect("interface type must be registered")
            }
            Type::Named(n) => {
                let key = type_key.expect("named type must have TypeKey");
                // Check if this named type wraps a struct or interface
                if let Some(&id) = self.struct_type_ids.get(&key) {
                    return id;
                }
                if let Some(&id) = self.interface_type_ids.get(&key) {
                    return id;
                }
                // Try underlying key
                if let Some(underlying_key) = n.try_underlying() {
                    if let Some(&id) = self.struct_type_ids.get(&underlying_key) {
                        return id;
                    }
                    if let Some(&id) = self.interface_type_ids.get(&underlying_key) {
                        return id;
                    }
                }
                panic!("named type {:?} must be registered as struct or interface", key)
            }
            Type::Basic(b) => basic_to_runtime_id(b.typ()),
            Type::Slice(_) => RuntimeTypeId::Slice as u32,
            Type::Map(_) => RuntimeTypeId::Map as u32,
            Type::Array(_) => RuntimeTypeId::Array as u32,
            Type::Chan(_) => RuntimeTypeId::Channel as u32,
            Type::Signature(_) => RuntimeTypeId::Closure as u32,
            Type::Pointer(_) => {
                let key = type_key.expect("pointer type must have TypeKey");
                self.get_struct_type_id(key).expect("pointer base type must be registered struct")
            }
            Type::Tuple(_) => RuntimeTypeId::Nil as u32,
        }
    }

    // === Function management ===

    pub fn register_func(&mut self, symbol: Symbol) -> u32 {
        let idx = self.next_func_idx;
        self.func_indices.insert(symbol, idx);
        self.next_func_idx += 1;
        idx
    }

    pub fn get_func_index(&self, symbol: Symbol) -> Option<u32> {
        self.func_indices.get(&symbol).copied()
    }

    // === Extern management ===

    pub fn register_extern(&mut self, symbol: Symbol, name: &str, param_slots: u16, ret_slots: u16) -> u32 {
        if let Some(&idx) = self.extern_indices.get(&symbol) {
            return idx;
        }
        let idx = self.module.add_extern(name, param_slots, ret_slots);
        self.extern_indices.insert(symbol, idx);
        idx
    }

    pub fn get_extern_index(&self, symbol: Symbol) -> Option<u32> {
        self.extern_indices.get(&symbol).copied()
    }

    // === Global management ===

    pub fn register_global(&mut self, symbol: Symbol, name: &str, type_id: u32, slots: u16) -> u32 {
        if let Some(&idx) = self.global_indices.get(&symbol) {
            return idx;
        }
        let idx = self.module.add_global(name, type_id, slots);
        self.global_indices.insert(symbol, idx);
        idx
    }

    pub fn get_global_index(&self, symbol: Symbol) -> Option<u32> {
        self.global_indices.get(&symbol).copied()
    }

    // === Constant management ===

    pub fn const_int(&mut self, value: i64) -> u16 {
        let key = ConstKey::Int(value);
        if let Some(&idx) = self.const_indices.get(&key) {
            return idx;
        }
        let idx = self.module.add_constant(Constant::Int(value));
        self.const_indices.insert(key, idx);
        idx
    }

    pub fn const_float(&mut self, value: f64) -> u16 {
        let key = ConstKey::Float(value.to_bits());
        if let Some(&idx) = self.const_indices.get(&key) {
            return idx;
        }
        let idx = self.module.add_constant(Constant::Float(value));
        self.const_indices.insert(key, idx);
        idx
    }

    pub fn const_string(&mut self, value: &str) -> u16 {
        let key = ConstKey::String(value.to_string());
        if let Some(&idx) = self.const_indices.get(&key) {
            return idx;
        }
        let idx = self.module.add_constant(Constant::String(value.to_string()));
        self.const_indices.insert(key, idx);
        idx
    }

    pub fn const_bool(&mut self, value: bool) -> u16 {
        let key = ConstKey::Bool(value);
        if let Some(&idx) = self.const_indices.get(&key) {
            return idx;
        }
        let idx = self.module.add_constant(Constant::Bool(value));
        self.const_indices.insert(key, idx);
        idx
    }

    // === Build ===

    pub fn finish(self) -> Module {
        self.module
    }
}
