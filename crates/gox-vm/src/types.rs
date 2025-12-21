//! Type metadata and type system.

use alloc::{string::{String, ToString}, vec, vec::Vec};
use hashbrown::HashMap;
use gox_common_core::{RuntimeTypeId, SlotType, ValueKind};

/// Type ID (index into type table).
pub type TypeId = u32;

/// Field layout info for compact struct representation.
#[derive(Clone, Debug, Default)]
pub struct FieldLayout {
    /// Byte offset from start of struct data.
    pub byte_offset: u32,
    /// Size in bytes (1, 2, 4, or 8).
    pub size: u8,
    /// Whether this field is signed (for sign extension on read).
    pub signed: bool,
}

impl FieldLayout {
    pub fn new(byte_offset: u32, size: u8, signed: bool) -> Self {
        Self { byte_offset, size, signed }
    }
    
    /// Encode size as 2-bit code: 0=1, 1=2, 2=4, 3=8 bytes.
    pub fn size_code(&self) -> u8 {
        match self.size {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            _ => 3, // Default to 8
        }
    }
    
    /// Get flags for GetField/SetField instruction.
    /// flags[1:0] = size_code, flags[2] = signed
    pub fn flags(&self) -> u8 {
        self.size_code() | (if self.signed { 0b100 } else { 0 })
    }
}

/// Type metadata.
#[derive(Clone, Debug)]
pub struct TypeMeta {
    /// ValueKind for this type.
    pub value_kind: ValueKind,
    /// Runtime type ID (only for Struct/Interface, indexes into meta tables).
    pub type_id: RuntimeTypeId,
    /// Size in 8-byte slots (for GC allocation, backward compat).
    pub size_slots: usize,
    /// Size in bytes (compact layout).
    pub size_bytes: usize,
    /// Slot types for GC scanning. Each slot describes how GC should handle it.
    pub slot_types: Vec<SlotType>,
    pub name: String,
    
    // For struct/object: field layouts (compact)
    pub field_layouts: Vec<FieldLayout>,
    
    // For array/slice/channel: element type and size
    pub elem_type: Option<TypeId>,
    pub elem_size: Option<usize>,
    
    // For map: key and value types
    pub key_type: Option<TypeId>,
    pub value_type: Option<TypeId>,
}

impl TypeMeta {
    /// Check if this is a struct type.
    pub fn is_struct(&self) -> bool {
        self.value_kind == ValueKind::Struct
    }
    
    /// Check if this is an interface type.
    pub fn is_interface(&self) -> bool {
        self.value_kind == ValueKind::Interface
    }
    
    /// Check if this type needs GC scanning.
    pub fn needs_gc(&self) -> bool {
        self.value_kind.needs_gc()
    }
    
    /// Create a builtin type.
    pub fn builtin(value_kind: ValueKind, name: &str, size_slots: usize, slot_types: Vec<SlotType>) -> Self {
        Self {
            value_kind,
            type_id: 0,
            size_slots,
            size_bytes: size_slots * 8,
            slot_types,
            name: name.to_string(),
            field_layouts: vec![],
            elem_type: None,
            elem_size: None,
            key_type: None,
            value_type: None,
        }
    }
    
    pub fn nil() -> Self {
        Self::builtin(ValueKind::Nil, "nil", 0, vec![])
    }
    
    pub fn primitive(value_kind: ValueKind, name: &str) -> Self {
        Self::builtin(value_kind, name, 1, vec![SlotType::Value])
    }
    
    pub fn struct_(type_id: RuntimeTypeId, name: &str, size_slots: usize, slot_types: Vec<SlotType>) -> Self {
        Self {
            value_kind: ValueKind::Struct,
            type_id,
            size_slots,
            size_bytes: size_slots * 8,
            slot_types,
            name: name.to_string(),
            field_layouts: vec![],
            elem_type: None,
            elem_size: None,
            key_type: None,
            value_type: None,
        }
    }
    
    pub fn object(value_kind: ValueKind, name: &str, size_slots: usize, slot_types: Vec<SlotType>) -> Self {
        Self {
            value_kind,
            type_id: 0,
            size_slots,
            size_bytes: size_slots * 8,
            slot_types,
            name: name.to_string(),
            field_layouts: vec![],
            elem_type: None,
            elem_size: None,
            key_type: None,
            value_type: None,
        }
    }
    
    pub fn is_primitive(&self) -> bool {
        (self.value_kind as u8) <= (ValueKind::FuncPtr as u8)
    }
    
    /// Get field layout by index.
    pub fn get_field_layout(&self, idx: usize) -> Option<&FieldLayout> {
        self.field_layouts.get(idx)
    }
}

/// Type table (compile-time generated, loaded into VM).
#[derive(Clone, Debug, Default)]
pub struct TypeTable {
    types: Vec<TypeMeta>,
    by_name: HashMap<String, TypeId>,
}

impl TypeTable {
    pub fn new() -> Self {
        let mut table = Self {
            types: Vec::new(),
            by_name: HashMap::new(),
        };
        table.init_builtins();
        table
    }
    
    fn init_builtins(&mut self) {
        // Reserve space for builtin types (ValueKind values 0-23)
        self.types.resize(24, TypeMeta::nil());
        
        // Helper to set builtin type at its ValueKind index
        let set_builtin = |types: &mut Vec<TypeMeta>, by_name: &mut HashMap<String, TypeId>, meta: TypeMeta| {
            let idx = meta.value_kind as usize;
            by_name.insert(meta.name.clone(), idx as TypeId);
            types[idx] = meta;
        };
        
        // Primitives
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::nil());
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Bool, "bool"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Int, "int"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Int8, "int8"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Int16, "int16"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Int32, "int32"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Int64, "int64"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Uint, "uint"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Uint8, "uint8"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Uint16, "uint16"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Uint32, "uint32"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Uint64, "uint64"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Float32, "float32"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::Float64, "float64"));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::primitive(ValueKind::FuncPtr, "funcptr"));
        
        // Reference types
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::String, "string", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Array, "array", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Slice, "slice", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Map, "map", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Channel, "channel", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Closure, "closure", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Struct, "struct", 0, vec![]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Pointer, "pointer", 1, vec![SlotType::GcRef]));
        set_builtin(&mut self.types, &mut self.by_name, TypeMeta::object(ValueKind::Interface, "interface", 2, vec![SlotType::Interface0, SlotType::Interface1]));
    }
    
    fn set(&mut self, id: TypeId, meta: TypeMeta) {
        let idx = id as usize;
        if idx >= self.types.len() {
            self.types.resize(idx + 1, TypeMeta::nil());
        }
        self.by_name.insert(meta.name.clone(), id);
        self.types[idx] = meta;
    }
    
    /// Register a new struct type.
    pub fn register_struct(&mut self, type_id: RuntimeTypeId, name: &str, size_slots: usize, slot_types: Vec<SlotType>) -> TypeId {
        let meta = TypeMeta::struct_(type_id, name, size_slots, slot_types);
        self.by_name.insert(name.to_string(), self.types.len() as TypeId);
        let id = self.types.len() as TypeId;
        self.types.push(meta);
        id
    }
    
    /// Get type metadata by ID.
    pub fn get(&self, id: TypeId) -> Option<&TypeMeta> {
        self.types.get(id as usize)
    }
    
    /// Get type metadata by ID (unchecked).
    pub fn get_unchecked(&self, id: TypeId) -> &TypeMeta {
        &self.types[id as usize]
    }
    
    /// Get type ID by name.
    pub fn get_by_name(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }
    
    /// Get number of types.
    pub fn len(&self) -> usize {
        self.types.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

