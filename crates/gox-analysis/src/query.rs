//! Type query interface for code generation.
//!
//! This module provides a clean API for querying type information,
//! hiding the internal complexity of TCObjects and keys.

use crate::obj::{Builtin, ConstValue, EntityType};
use crate::objects::{ObjKey, TCObjects, TypeKey};
use crate::typ::{
    ArrayDetail, BasicType, ChanDetail, MapDetail, NamedDetail,
    SignatureDetail, SliceDetail, StructDetail, Type,
};
use gox_common::symbol::Symbol;
use gox_common::SymbolInterner;
use gox_common_core::SlotType;

/// Unified type information query interface for code generation.
///
/// This provides a clean API that hides the internal TCObjects/key complexity.
pub struct TypeQuery<'a> {
    pub(crate) objs: &'a TCObjects,
    pub(crate) interner: &'a SymbolInterner,
    pub(crate) pkg_scope: Option<crate::objects::ScopeKey>,
}

impl<'a> TypeQuery<'a> {
    /// Creates a new TypeQuery.
    pub fn new(
        objs: &'a TCObjects,
        interner: &'a SymbolInterner,
        pkg_scope: Option<crate::objects::ScopeKey>,
    ) -> Self {
        Self { objs, interner, pkg_scope }
    }

    // =========================================================================
    // Symbol resolution
    // =========================================================================

    /// Resolves a Symbol to its string representation.
    pub fn symbol_str(&self, sym: Symbol) -> &str {
        self.interner.resolve(sym).unwrap_or("")
    }

    /// Looks up a name in the package scope.
    pub fn lookup(&self, name: &str) -> Option<EntityRef<'a>> {
        let scope_key = self.pkg_scope?;
        let obj_key = self.objs.scopes[scope_key].lookup(name)?;
        Some(self.entity_ref(obj_key))
    }

    /// Looks up a Symbol in the package scope.
    pub fn lookup_symbol(&self, sym: Symbol) -> Option<EntityRef<'a>> {
        let name = self.symbol_str(sym);
        self.lookup(name)
    }

    /// Checks if a symbol refers to a builtin function.
    pub fn is_builtin(&self, sym: Symbol) -> Option<Builtin> {
        let name = self.symbol_str(sym);
        // Check universe scope for builtins
        let universe = self.objs.universe();
        let obj_key = self.objs.scopes[universe.scope()].lookup(name)?;
        let obj = &self.objs.lobjs[obj_key];
        match obj.entity_type() {
            EntityType::Builtin(b) => Some(*b),
            _ => None,
        }
    }

    // =========================================================================
    // Type resolution
    // =========================================================================

    /// Gets the type for a TypeKey.
    pub fn get_type(&self, key: TypeKey) -> &'a Type {
        &self.objs.types[key]
    }

    /// Gets the underlying type for a named type.
    pub fn underlying(&self, key: TypeKey) -> &'a Type {
        self.objs.types[key].underlying_val(self.objs)
    }

    /// Gets type details for slice types.
    pub fn slice_elem(&self, slice: &SliceDetail) -> &'a Type {
        &self.objs.types[slice.elem()]
    }

    /// Gets type details for array types.
    pub fn array_elem(&self, arr: &ArrayDetail) -> &'a Type {
        &self.objs.types[arr.elem()]
    }

    /// Gets type details for map types.
    pub fn map_key(&self, map: &MapDetail) -> &'a Type {
        &self.objs.types[map.key()]
    }

    /// Gets type details for map types.
    pub fn map_elem(&self, map: &MapDetail) -> &'a Type {
        &self.objs.types[map.elem()]
    }

    /// Gets type details for chan types.
    pub fn chan_elem(&self, chan: &ChanDetail) -> &'a Type {
        &self.objs.types[chan.elem()]
    }

    /// Gets type details for pointer types.
    pub fn pointer_base(&self, ptr: &crate::typ::PointerDetail) -> &'a Type {
        &self.objs.types[ptr.base()]
    }

    /// Gets the underlying type for a named type.
    pub fn named_underlying(&self, named: &NamedDetail) -> Option<&'a Type> {
        named.try_underlying().map(|k| &self.objs.types[k])
    }

    /// Gets the name of a named type.
    pub fn named_name(&self, named: &NamedDetail) -> Option<&'a str> {
        named.obj().map(|k| self.objs.lobjs[k].name())
    }

    // =========================================================================
    // Struct field access
    // =========================================================================

    /// Gets the fields of a struct type.
    pub fn struct_fields(&self, s: &'a StructDetail) -> Vec<FieldInfo<'a>> {
        s.fields()
            .iter()
            .enumerate()
            .map(|(i, &okey)| {
                let obj = &self.objs.lobjs[okey];
                FieldInfo {
                    name: obj.name(),
                    typ: obj.typ().map(|t| &self.objs.types[t]),
                    tag: s.tag(i).map(|s| s.as_str()),
                    embedded: obj.var_embedded(),
                    index: i,
                }
            })
            .collect()
    }

    /// Looks up a field by name in a struct.
    pub fn struct_field_index(&self, s: &StructDetail, name: Symbol) -> Option<usize> {
        let name_str = self.symbol_str(name);
        s.fields().iter().enumerate().find_map(|(i, &okey)| {
            let obj = &self.objs.lobjs[okey];
            if obj.name() == name_str {
                Some(i)
            } else {
                None
            }
        })
    }

    // =========================================================================
    // Signature access
    // =========================================================================

    /// Gets parameter types for a signature.
    pub fn signature_params(&self, sig: &SignatureDetail) -> Vec<&'a Type> {
        let tuple = self.objs.types[sig.params()].try_as_tuple().unwrap();
        tuple
            .vars()
            .iter()
            .filter_map(|&okey| {
                self.objs.lobjs[okey].typ().map(|t| &self.objs.types[t])
            })
            .collect()
    }

    /// Gets result types for a signature.
    pub fn signature_results(&self, sig: &SignatureDetail) -> Vec<&'a Type> {
        let tuple = self.objs.types[sig.results()].try_as_tuple().unwrap();
        tuple
            .vars()
            .iter()
            .filter_map(|&okey| {
                self.objs.lobjs[okey].typ().map(|t| &self.objs.types[t])
            })
            .collect()
    }

    // =========================================================================
    // Type properties for codegen
    // =========================================================================

    /// Computes the runtime type ID for a type.
    pub fn runtime_type_id(&self, ty: &Type) -> u32 {
        use gox_common_core::RuntimeTypeId;
        match ty {
            Type::Basic(b) => basic_to_runtime_id(b.typ()),
            Type::Slice(_) => RuntimeTypeId::Slice as u32,
            Type::Map(_) => RuntimeTypeId::Map as u32,
            Type::Array(_) => RuntimeTypeId::Array as u32,
            Type::Chan(_) => RuntimeTypeId::Channel as u32,
            Type::Signature(_) => RuntimeTypeId::Closure as u32,
            Type::Pointer(p) => self.runtime_type_id(&self.objs.types[p.base()]),
            Type::Struct(_) => RuntimeTypeId::FirstStruct as u32,
            Type::Interface(_) => RuntimeTypeId::FirstInterface as u32,
            Type::Named(n) => {
                if let Some(u) = n.try_underlying() {
                    self.runtime_type_id(&self.objs.types[u])
                } else {
                    RuntimeTypeId::Nil as u32
                }
            }
            Type::Tuple(_) => RuntimeTypeId::Nil as u32,
        }
    }

    /// Computes the number of slots a type occupies.
    pub fn type_slots(&self, ty: &Type) -> u16 {
        match ty {
            Type::Basic(_) => 1,
            Type::Slice(_) | Type::Map(_) | Type::Chan(_) | Type::Signature(_) | Type::Pointer(_) => 1,
            Type::Array(arr) => {
                let len = arr.len().unwrap_or(0) as u16;
                len * self.type_slots(&self.objs.types[arr.elem()])
            }
            Type::Struct(s) => {
                s.fields()
                    .iter()
                    .map(|&okey| {
                        self.objs.lobjs[okey]
                            .typ()
                            .map(|t| self.type_slots(&self.objs.types[t]))
                            .unwrap_or(1)
                    })
                    .sum()
            }
            Type::Interface(_) => 2,
            Type::Named(n) => {
                if let Some(u) = n.try_underlying() {
                    self.type_slots(&self.objs.types[u])
                } else {
                    1
                }
            }
            Type::Tuple(_) => 1,
        }
    }

    /// Computes the SlotType list for GC scanning.
    pub fn type_slot_types(&self, ty: &Type) -> Vec<SlotType> {
        match ty {
            Type::Basic(b) if b.typ() == BasicType::Str => vec![SlotType::GcRef],
            Type::Basic(_) => vec![SlotType::Value],
            Type::Slice(_) | Type::Map(_) | Type::Chan(_) | Type::Signature(_) | Type::Pointer(_) => {
                vec![SlotType::GcRef]
            }
            Type::Array(arr) => {
                let elem = self.type_slot_types(&self.objs.types[arr.elem()]);
                let mut result = Vec::new();
                for _ in 0..arr.len().unwrap_or(0) {
                    result.extend(elem.iter().copied());
                }
                result
            }
            Type::Struct(s) => {
                let mut result = Vec::new();
                for &okey in s.fields() {
                    if let Some(t) = self.objs.lobjs[okey].typ() {
                        result.extend(self.type_slot_types(&self.objs.types[t]));
                    }
                }
                result
            }
            Type::Interface(_) => vec![SlotType::Interface0, SlotType::Interface1],
            Type::Named(n) => {
                if let Some(u) = n.try_underlying() {
                    self.type_slot_types(&self.objs.types[u])
                } else {
                    vec![SlotType::Value]
                }
            }
            Type::Tuple(_) => vec![SlotType::Value],
        }
    }

    /// Returns true if the type is a reference type (pointer, slice, map, etc.).
    pub fn is_ref_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Basic(b) => b.typ() == BasicType::Str,
            Type::Slice(_) | Type::Map(_) | Type::Chan(_) | Type::Signature(_) | Type::Pointer(_) => true,
            Type::Array(_) | Type::Struct(_) | Type::Tuple(_) => false,
            Type::Interface(_) => true,
            Type::Named(n) => {
                if let Some(u) = n.try_underlying() {
                    self.is_ref_type(&self.objs.types[u])
                } else {
                    false
                }
            }
        }
    }

    /// Returns true if the type is an interface type.
    pub fn is_interface(&self, ty: &Type) -> bool {
        match ty {
            Type::Interface(_) => true,
            Type::Named(n) => {
                if let Some(u) = n.try_underlying() {
                    matches!(&self.objs.types[u], Type::Interface(_))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    fn entity_ref(&self, okey: ObjKey) -> EntityRef<'a> {
        let obj = &self.objs.lobjs[okey];
        let typ = obj.typ().map(|t| &self.objs.types[t]);
        
        match obj.entity_type() {
            EntityType::Var(_) => EntityRef::Var {
                name: obj.name(),
                typ,
            },
            EntityType::Func { .. } => EntityRef::Func {
                name: obj.name(),
                sig: typ.and_then(|t| t.try_as_signature()),
            },
            EntityType::TypeName => EntityRef::Type {
                name: obj.name(),
                underlying: typ.map(|t| t.underlying_val(self.objs)),
            },
            EntityType::Const { val } => EntityRef::Const {
                name: obj.name(),
                typ,
                value: val,
            },
            EntityType::Builtin(b) => EntityRef::Builtin(*b),
            EntityType::PkgName { .. } => EntityRef::Package { name: obj.name() },
            EntityType::Nil => EntityRef::Nil,
            EntityType::Label { .. } => EntityRef::Label { name: obj.name() },
        }
    }
}

/// A simplified reference to a language entity.
#[derive(Debug)]
pub enum EntityRef<'a> {
    Var {
        name: &'a str,
        typ: Option<&'a Type>,
    },
    Func {
        name: &'a str,
        sig: Option<&'a SignatureDetail>,
    },
    Type {
        name: &'a str,
        underlying: Option<&'a Type>,
    },
    Const {
        name: &'a str,
        typ: Option<&'a Type>,
        value: &'a ConstValue,
    },
    Builtin(Builtin),
    Package {
        name: &'a str,
    },
    Nil,
    Label {
        name: &'a str,
    },
}

/// Information about a struct field.
#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub name: &'a str,
    pub typ: Option<&'a Type>,
    pub tag: Option<&'a str>,
    pub embedded: bool,
    pub index: usize,
}

// Helper function
fn basic_to_runtime_id(b: BasicType) -> u32 {
    use gox_common_core::RuntimeTypeId;
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
