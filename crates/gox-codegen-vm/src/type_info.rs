//! Type information wrapper for code generation.
//!
//! This module wraps gox_analysis::TypeQuery and adds expression type tracking.

use gox_analysis::{Builtin, Type, TypeKey, TypeQuery};
use gox_common::Symbol;
use gox_common_core::SlotType;
use gox_syntax::ast::Expr;
use std::collections::HashMap;

use gox_common_core::ExprId;

/// Type information for code generation.
///
/// Wraps TypeQuery from gox-analysis and adds expression type tracking.
pub struct TypeInfo<'a> {
    /// Type query interface from analysis.
    pub query: TypeQuery<'a>,
    /// Expression types recorded during type checking.
    pub expr_types: &'a HashMap<ExprId, TypeKey>,
}

impl<'a> TypeInfo<'a> {
    pub fn new(query: TypeQuery<'a>, expr_types: &'a HashMap<ExprId, TypeKey>) -> Self {
        Self { query, expr_types }
    }

    // === Expression type queries ===

    pub fn expr_type(&self, expr: &Expr) -> Option<&'a Type> {
        self.expr_types
            .get(&expr.id)
            .map(|&key| self.query.get_type(key))
    }

    pub fn expr_type_key(&self, expr: &Expr) -> Option<TypeKey> {
        self.expr_types.get(&expr.id).copied()
    }

    // === Symbol queries (delegate to TypeQuery) ===

    pub fn symbol_str(&self, sym: Symbol) -> &str {
        self.query.symbol_str(sym)
    }

    pub fn is_builtin(&self, sym: Symbol) -> Option<Builtin> {
        self.query.is_builtin(sym)
    }

    // === Type property queries (delegate to TypeQuery) ===

    pub fn runtime_type_id(&self, ty: &Type) -> u32 {
        self.query.runtime_type_id(ty)
    }

    pub fn type_slots(&self, ty: &Type) -> u16 {
        self.query.type_slots(ty)
    }

    pub fn type_slot_types(&self, ty: &Type) -> Vec<SlotType> {
        self.query.type_slot_types(ty)
    }

    pub fn is_ref_type(&self, ty: &Type) -> bool {
        self.query.is_ref_type(ty)
    }

    pub fn is_interface(&self, ty: &Type) -> bool {
        self.query.is_interface(ty)
    }
}
