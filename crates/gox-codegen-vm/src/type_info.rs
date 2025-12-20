//! Type information wrapper for code generation.
//!
//! This module wraps gox_analysis::TypeQuery and adds expression type tracking.

use gox_analysis::{Builtin, ConstValue, Type, TypeAndValue, TypeKey, TypeQuery};
use gox_analysis::operand::OperandMode;
use gox_common::Symbol;
use gox_common_core::SlotType;
use gox_syntax::ast::{Expr, TypeExpr, TypeExprKind};
use std::collections::HashMap;

use gox_common_core::ExprId;

/// Type information for code generation.
///
/// Wraps TypeQuery from gox-analysis and adds expression type tracking.
pub struct TypeInfo<'a> {
    /// Type query interface from analysis.
    pub query: TypeQuery<'a>,
    /// Expression types and values recorded during type checking.
    pub expr_types: &'a HashMap<ExprId, TypeAndValue>,
}

impl<'a> TypeInfo<'a> {
    pub fn new(query: TypeQuery<'a>, expr_types: &'a HashMap<ExprId, TypeAndValue>) -> Self {
        Self { query, expr_types }
    }

    // === Expression type queries ===

    pub fn expr_type(&self, expr: &Expr) -> Option<&'a Type> {
        self.expr_types
            .get(&expr.id)
            .map(|tv| self.query.get_type(tv.typ))
    }

    pub fn expr_type_key(&self, expr: &Expr) -> Option<TypeKey> {
        self.expr_types.get(&expr.id).map(|tv| tv.typ)
    }

    /// Get constant value for an expression (if it's a constant).
    pub fn expr_const_value(&self, expr: &Expr) -> Option<&ConstValue> {
        self.expr_types.get(&expr.id).and_then(|tv| {
            match &tv.mode {
                OperandMode::Constant(v) => Some(v),
                _ => None,
            }
        })
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

    /// Lookup a symbol and get its type.
    pub fn lookup_symbol_type(&self, sym: Symbol) -> Option<&'a Type> {
        use gox_analysis::query::EntityRef;
        match self.query.lookup_symbol(sym)? {
            EntityRef::Var { typ, .. } => typ,
            _ => None,
        }
    }

    /// Lookup a type name symbol and return its TypeKey.
    pub fn lookup_type_key(&self, sym: Symbol) -> Option<TypeKey> {
        self.query.lookup_type_key(sym)
    }

    /// Resolve a TypeExpr to its Type.
    pub fn resolve_type_expr(&self, ty: &TypeExpr) -> Option<&'a Type> {
        match &ty.kind {
            TypeExprKind::Ident(ident) => {
                let type_key = self.query.lookup_type_key(ident.symbol)?;
                Some(self.query.get_type(type_key))
            }
            TypeExprKind::Interface(_) => None, // Anonymous interface
            _ => None,
        }
    }

    /// Get TypeKey from a TypeExpr (for named types).
    pub fn type_expr_key(&self, ty: &TypeExpr) -> Option<TypeKey> {
        match &ty.kind {
            TypeExprKind::Ident(ident) => self.query.lookup_type_key(ident.symbol),
            _ => None,
        }
    }
}
