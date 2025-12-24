//! Type info wrapper - provides slot layout calculation and type queries.

use vo_analysis::objects::{ObjKey, TCObjects, TypeKey};
use vo_analysis::typ::{self, Type};
use vo_analysis::Project;
use vo_common::symbol::Ident;
use vo_common_core::ExprId;
use vo_common_core::types::SlotType;

/// Wrapper around Project for codegen queries.
pub struct TypeInfoWrapper<'a> {
    pub project: &'a Project,
}

impl<'a> TypeInfoWrapper<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self { project }
    }

    fn tc_objs(&self) -> &TCObjects {
        &self.project.tc_objs
    }

    fn type_info(&self) -> &vo_analysis::check::TypeInfo {
        &self.project.type_info
    }

    // === Expression type queries ===

    pub fn expr_type(&self, expr_id: ExprId) -> Option<TypeKey> {
        self.type_info().expr_type(expr_id)
    }

    pub fn expr_mode(&self, expr_id: ExprId) -> Option<&vo_analysis::operand::OperandMode> {
        self.type_info().expr_mode(expr_id)
    }

    // === Definition/Use queries ===

    pub fn get_def(&self, ident: &Ident) -> Option<ObjKey> {
        self.type_info().get_def(ident)
    }

    pub fn get_use(&self, ident: &Ident) -> Option<ObjKey> {
        self.type_info().get_use(ident)
    }

    // === Escape queries ===

    pub fn is_escaped(&self, obj: ObjKey) -> bool {
        self.type_info().is_escaped(obj)
    }

    // === Closure captures ===

    pub fn get_closure_captures(&self, func_lit_id: ExprId) -> Option<&Vec<ObjKey>> {
        self.type_info().closure_captures.get(&func_lit_id)
    }

    // === Selection queries ===

    pub fn get_selection(&self, expr_id: ExprId) -> Option<&vo_analysis::selection::Selection> {
        self.type_info().selections.get(&expr_id)
    }

    // === Slot layout calculation ===

    pub fn type_slot_count(&self, type_key: TypeKey) -> u16 {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        match &self.tc_objs().types[underlying] {
            Type::Basic(_) => 1,
            Type::Pointer(_) => 1,
            Type::Slice(_) => 1,
            Type::Map(_) => 1,
            Type::Chan(_) => 1,
            Type::Signature(_) => 1, // closure is GcRef
            Type::Interface(_) => 2,
            Type::Struct(s) => {
                let mut total = 0u16;
                for &field_obj in s.fields() {
                    if let Some(field_type) = self.tc_objs().lobjs[field_obj].typ() {
                        total += self.type_slot_count(field_type);
                    }
                }
                total
            }
            Type::Array(a) => {
                let elem_slots = self.type_slot_count(a.elem());
                let len = a.len().unwrap_or(0) as u16;
                elem_slots * len
            }
            _ => 1,
        }
    }

    pub fn type_slot_types(&self, type_key: TypeKey) -> Vec<SlotType> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        match &self.tc_objs().types[underlying] {
            Type::Basic(_) => vec![SlotType::Value],
            Type::Pointer(_) => vec![SlotType::GcRef],
            Type::Slice(_) => vec![SlotType::GcRef],
            Type::Map(_) => vec![SlotType::GcRef],
            Type::Chan(_) => vec![SlotType::GcRef],
            Type::Signature(_) => vec![SlotType::GcRef],
            Type::Interface(_) => vec![SlotType::Interface0, SlotType::Interface1],
            Type::Struct(s) => {
                let mut types = Vec::new();
                for &field_obj in s.fields() {
                    if let Some(field_type) = self.tc_objs().lobjs[field_obj].typ() {
                        types.extend(self.type_slot_types(field_type));
                    }
                }
                types
            }
            Type::Array(a) => {
                let elem_types = self.type_slot_types(a.elem());
                let mut types = Vec::new();
                let len = a.len().unwrap_or(0) as usize;
                for _ in 0..len {
                    types.extend(elem_types.iter().cloned());
                }
                types
            }
            _ => vec![SlotType::Value],
        }
    }

    // === Struct layout ===

    pub fn struct_field_offset(
        &self,
        type_key: TypeKey,
        field_name: &str,
    ) -> Option<(u16, u16)> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        if let Type::Struct(s) = &self.tc_objs().types[underlying] {
            let mut offset = 0u16;
            for &field_obj in s.fields() {
                let obj = &self.tc_objs().lobjs[field_obj];
                let field_type = obj.typ()?;
                let field_slots = self.type_slot_count(field_type);
                if obj.name() == field_name {
                    return Some((offset, field_slots));
                }
                offset += field_slots;
            }
        }
        None
    }

    // === Type queries ===

    pub fn is_interface(&self, type_key: TypeKey) -> bool {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        self.tc_objs().types[underlying].try_as_interface().is_some()
    }

    pub fn is_pointer(&self, type_key: TypeKey) -> bool {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        self.tc_objs().types[underlying].try_as_pointer().is_some()
    }

    pub fn is_struct(&self, type_key: TypeKey) -> bool {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        self.tc_objs().types[underlying].try_as_struct().is_some()
    }

    pub fn is_array(&self, type_key: TypeKey) -> bool {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        self.tc_objs().types[underlying].try_as_array().is_some()
    }

    /// Get object's type
    pub fn obj_type(&self, obj: ObjKey) -> Option<TypeKey> {
        self.tc_objs().lobjs[obj].typ()
    }

    /// Get object's name
    pub fn obj_name(&self, obj: ObjKey) -> &str {
        self.tc_objs().lobjs[obj].name()
    }

    /// Get struct field offset from pointer type
    pub fn struct_field_offset_from_ptr(
        &self,
        ptr_type: TypeKey,
        field_name: &str,
    ) -> Option<(u16, u16)> {
        let underlying = typ::underlying_type(ptr_type, self.tc_objs());
        if let Type::Pointer(p) = &self.tc_objs().types[underlying] {
            self.struct_field_offset(p.base(), field_name)
        } else {
            None
        }
    }

    /// Get array element slot count
    pub fn array_elem_slots(&self, type_key: TypeKey) -> Option<u16> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        if let Type::Array(a) = &self.tc_objs().types[underlying] {
            Some(self.type_slot_count(a.elem()))
        } else {
            None
        }
    }

    /// Get slice element slot count
    pub fn slice_elem_slots(&self, type_key: TypeKey) -> Option<u16> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        if let Type::Slice(s) = &self.tc_objs().types[underlying] {
            Some(self.type_slot_count(s.elem()))
        } else {
            None
        }
    }

    /// Get array length
    pub fn array_len(&self, type_key: TypeKey) -> Option<u64> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        if let Type::Array(a) = &self.tc_objs().types[underlying] {
            a.len()
        } else {
            None
        }
    }

    /// Get pointer element slot count
    pub fn pointer_elem_slots(&self, type_key: TypeKey) -> Option<u16> {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        if let Type::Pointer(p) = &self.tc_objs().types[underlying] {
            Some(self.type_slot_count(p.base()))
        } else {
            None
        }
    }

    /// Get interface meta ID (for IfaceAssign)
    pub fn get_interface_meta_id(&self, _type_key: TypeKey) -> Option<u16> {
        // TODO: look up from registered interface metas
        None
    }

    /// Get value kind for a type (for IfaceAssign flags)
    pub fn value_kind(&self, type_key: TypeKey) -> u8 {
        let underlying = typ::underlying_type(type_key, self.tc_objs());
        match &self.tc_objs().types[underlying] {
            Type::Basic(_) => 0,       // Value
            Type::Pointer(_) => 1,     // GcRef
            Type::Struct(_) => 0,      // Value (multi-slot)
            Type::Array(_) => 0,       // Value (multi-slot)
            Type::Slice(_) => 1,       // GcRef
            Type::Map(_) => 1,         // GcRef
            Type::Chan(_) => 1,        // GcRef
            Type::Interface(_) => 2,   // Interface0
            _ => 0,
        }
    }

    /// Get method index in interface
    pub fn get_interface_method_index(&self, iface_type: TypeKey, method_name: &str) -> Option<u16> {
        let underlying = typ::underlying_type(iface_type, self.tc_objs());
        if let Type::Interface(iface) = &self.tc_objs().types[underlying] {
            for (idx, method) in iface.methods().iter().enumerate() {
                if self.tc_objs().lobjs[*method].name() == method_name {
                    return Some(idx as u16);
                }
            }
        }
        None
    }
}
