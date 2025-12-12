# GoX Type Checker Design Document

## Overview

The `gox-analysis` crate implements semantic analysis (type checking) for GoX. It takes an AST from `gox-syntax` and produces:

1. A validated, type-annotated view of the program
2. Diagnostics for semantic errors
3. Symbol tables for code generation

This document provides the technical specification for implementation.

---

## 0. High-Level Design

### 0.1 Core Problem

The type checker answers two fundamental questions:

1. **Is this program semantically valid?** (e.g., no type mismatches, no undefined variables)
2. **What is the type of each expression/variable?** (needed for code generation)

### 0.2 Multi-Phase Architecture

Type checking cannot be done in a single pass because of forward references. Consider:

```gox
func foo() User { return User{}; }  // Uses User before it's defined
type User struct { name string; };
```

The checker uses a **multi-phase approach**:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Phase 1: Collect                         │
│  Scan all declarations, create placeholder objects in scope     │
│  - VarDecl → Var object (type TBD)                             │
│  - TypeDecl → TypeName object (underlying TBD)                 │
│  - FuncDecl → Func object (signature TBD)                      │
│  - InterfaceDecl → Interface type (methods TBD)                │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                     Phase 2: Resolve Types                      │
│  For each type declaration, resolve its underlying type        │
│  - struct fields get their types resolved                      │
│  - interface methods get their signatures resolved             │
│  - Named types link to their underlying types                  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 3: Check Bodies                        │
│  Type check function bodies and variable initializers          │
│  - Each expression gets a type assigned                        │
│  - Type mismatches are reported as errors                      │
│  - Scopes are created for blocks, functions                    │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                  Phase 4: Check Implements                      │
│  Verify that `implements` declarations are satisfied           │
│  - Type must have all methods required by interface            │
│  - Method signatures must match exactly                        │
└─────────────────────────────────────────────────────────────────┘
```

### 0.3 How Types Are Determined

#### For Variables

```gox
var x int;           // Type is explicit: int
var y = 42;          // Type inferred from initializer: int
x := someFunc();     // Type inferred from function return type
```

The checker:
1. If explicit type given → resolve the type expression
2. If initializer given → check the expression, use its type
3. Store the resolved `TypeKey` in the variable's `LangObj.typ`

#### For Expressions

Every expression is checked recursively, returning an `Operand` with:
- `typ`: The resolved `TypeKey`
- `mode`: Whether it's a value, variable, constant, or type

```
check_expr(a + b):
  left = check_expr(a)   → Operand { typ: int, mode: Variable }
  right = check_expr(b)  → Operand { typ: int, mode: Constant(3) }
  verify: left.typ == right.typ (or compatible)
  return Operand { typ: int, mode: Value }
```

#### For Function Calls

```gox
result := foo(x, y);
```

1. Check `foo` → get its type (must be `Signature`)
2. Check each argument → get their types
3. Verify: argument types match parameter types
4. Return type = function's result type

### 0.4 Scope and Name Resolution

Names are resolved through a **scope chain**:

```
Universe Scope (bool, int, string, len, append, ...)
    ↓
Package Scope (top-level declarations)
    ↓
Function Scope (parameters, local vars)
    ↓
Block Scope (if/for/switch bodies)
    ↓
Nested Block Scope ...
```

When looking up a name:
1. Search current scope
2. If not found, search parent scope
3. Continue until Universe scope
4. If still not found → "undefined" error

### 0.5 Type Identity and Assignability

**Type Identity**: Two types are identical if:
- Same basic type (int == int)
- Same named type (same `ObjKey`)
- Same structure (array: same length and element type)

**Assignability** (can `src` be assigned to `dst`?):
1. If identical → yes
2. If `src` is `nil` and `dst` is object type → yes
3. If `dst` is interface and `src` implements it → yes
4. Otherwise → no

### 0.6 Error Recovery

When an error is found:
1. Report the diagnostic with span information
2. Return an `Invalid` type or `Operand::invalid()`
3. Continue checking (don't stop at first error)
4. Skip cascading errors when operand is already invalid

### 0.7 Output: TypeInfo

The checker produces a `TypeInfo` structure containing:

| Field | Purpose |
|-------|---------|
| `types: Map<Span, TypeAndValue>` | Type of each expression |
| `defs: Map<Span, ObjKey>` | Where each name is defined |
| `uses: Map<Span, ObjKey>` | What object each name refers to |
| `scopes: Map<Span, ScopeKey>` | Scope at each position |

This information is used by:
- **IDE features**: hover for type, go-to-definition
- **Code generation**: know the type of each value

---

## 1. Project Structure

```
crates/gox-analysis/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── objects.rs          # Arena storage
│   ├── obj.rs              # Language objects
│   ├── scope.rs            # Scope management
│   ├── typ.rs              # Type representation
│   ├── universe.rs         # Predefined types/functions
│   ├── operand.rs          # Expression checking intermediate
│   └── check/
│       ├── mod.rs          # Checker main struct
│       ├── resolver.rs     # Declaration collection
│       ├── decl.rs         # Declaration checking
│       ├── stmt.rs         # Statement checking
│       ├── expr.rs         # Expression checking
│       ├── typexpr.rs      # Type expression resolution
│       ├── call.rs         # Call expression checking
│       ├── assignment.rs   # Assignment checking
│       ├── interface.rs    # Interface/implements checking
│       └── builtin.rs      # Builtin function handling
└── tests/
    └── integration.rs
```

---

## 2. Public API

File: `lib.rs`

```rust
pub use check::TypeInfo;
pub use objects::TCObjects;
pub use obj::{LangObj, ObjKey, ObjKind};
pub use scope::{Scope, ScopeKey};
pub use typ::{Type, TypeKey};

/// Analysis result
pub struct Analysis {
    pub info: TypeInfo,
    pub objs: TCObjects,
    pub diagnostics: gox_common::diagnostic::DiagnosticBag,
}

/// Main entry point
pub fn check(file: &gox_syntax::ast::SourceFile, file_id: FileId) -> Analysis {
    let mut checker = check::Checker::new(file, file_id);
    checker.run();
    Analysis {
        info: checker.info,
        objs: checker.objs,
        diagnostics: checker.diagnostics,
    }
}
```

---

## 3. Module: objects.rs

### Purpose

Central arena storage for all type checking objects. Using arenas instead of `Rc<RefCell<>>` avoids reference cycles and makes equality checks cheap (compare indices).

### Data Structures

```rust
use std::ops::{Index, IndexMut};

/// Typed index into an arena
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Idx<T>(u32, std::marker::PhantomData<T>);

/// Simple arena
pub struct Arena<T> {
    items: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    pub fn alloc(&mut self, item: T) -> Idx<T> {
        let idx = self.items.len() as u32;
        self.items.push(item);
        Idx(idx, std::marker::PhantomData)
    }
}

impl<T> Index<Idx<T>> for Arena<T> {
    type Output = T;
    fn index(&self, idx: Idx<T>) -> &T {
        &self.items[idx.0 as usize]
    }
}

impl<T> IndexMut<Idx<T>> for Arena<T> {
    fn index_mut(&mut self, idx: Idx<T>) -> &mut T {
        &mut self.items[idx.0 as usize]
    }
}

/// All type checking objects
pub struct TCObjects {
    pub lobjs: Arena<LangObj>,
    pub types: Arena<Type>,
    pub scopes: Arena<Scope>,
}

impl TCObjects {
    pub fn new() -> Self {
        Self {
            lobjs: Arena::new(),
            types: Arena::new(),
            scopes: Arena::new(),
        }
    }
}
```

### Type Aliases

```rust
pub type ObjKey = Idx<LangObj>;
pub type TypeKey = Idx<Type>;
pub type ScopeKey = Idx<Scope>;
```

---

## 4. Module: obj.rs

### Purpose

Represents named language entities: variables, constants, functions, types, etc.

### Data Structures

```rust
use gox_common::Span;
use crate::objects::{ObjKey, TypeKey, ScopeKey};

/// A named language entity
pub struct LangObj {
    pub name: String,
    pub kind: ObjKind,
    pub typ: Option<TypeKey>,
    pub decl_span: Span,
    pub parent_scope: Option<ScopeKey>,
}

impl LangObj {
    pub fn new(name: String, kind: ObjKind, span: Span) -> Self {
        Self {
            name,
            kind,
            typ: None,
            decl_span: span,
            parent_scope: None,
        }
    }
    
    pub fn is_var(&self) -> bool {
        matches!(self.kind, ObjKind::Var { .. })
    }
    
    pub fn is_const(&self) -> bool {
        matches!(self.kind, ObjKind::Const { .. })
    }
    
    pub fn is_func(&self) -> bool {
        matches!(self.kind, ObjKind::Func { .. })
    }
    
    pub fn is_type(&self) -> bool {
        matches!(self.kind, ObjKind::TypeName)
    }
}

pub enum ObjKind {
    /// Variable (local, parameter, field)
    Var {
        is_field: bool,
        is_param: bool,
    },
    
    /// Constant with compile-time value
    Const {
        value: ConstValue,
    },
    
    /// Type name (from type declaration)
    TypeName,
    
    /// Function or method
    Func {
        is_method: bool,
    },
    
    /// Builtin function (len, cap, etc.)
    Builtin(BuiltinKind),
}

#[derive(Clone)]
pub enum ConstValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Clone, Copy)]
pub enum BuiltinKind {
    Len,
    Cap,
    Append,
    Make,
    Print,
    Println,
}
```

---

## 5. Module: scope.rs

### Purpose

Manages the scope tree for name resolution. Scopes form a tree: universe → package → file → function → blocks.

### Data Structures

```rust
use std::collections::HashMap;
use gox_common::Span;
use crate::objects::{ObjKey, ScopeKey, TCObjects};

pub struct Scope {
    pub parent: Option<ScopeKey>,
    pub children: Vec<ScopeKey>,
    pub elems: HashMap<String, ObjKey>,
    pub span: Span,
    pub kind: ScopeKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Universe,   // Predefined identifiers
    Package,    // Package-level declarations
    File,       // File-level (for imports)
    Func,       // Function body
    Block,      // Block statement
}

impl Scope {
    pub fn new(parent: Option<ScopeKey>, kind: ScopeKind, span: Span) -> Self {
        Self {
            parent,
            children: Vec::new(),
            elems: HashMap::new(),
            span,
            kind,
        }
    }
    
    /// Look up a name in this scope only
    pub fn lookup_local(&self, name: &str) -> Option<ObjKey> {
        self.elems.get(name).copied()
    }
    
    /// Insert an object. Returns the old object if name already exists.
    pub fn insert(&mut self, name: String, obj: ObjKey) -> Option<ObjKey> {
        self.elems.insert(name, obj)
    }
}

/// Look up a name, searching parent scopes
pub fn lookup(
    scope_key: ScopeKey,
    name: &str,
    objs: &TCObjects,
) -> Option<(ScopeKey, ObjKey)> {
    let mut current = Some(scope_key);
    
    while let Some(key) = current {
        let scope = &objs.scopes[key];
        if let Some(obj) = scope.lookup_local(name) {
            return Some((key, obj));
        }
        current = scope.parent;
    }
    
    None
}
```

---

## 6. Module: typ.rs

### Purpose

Semantic type representation. Distinct from AST types (`gox_syntax::ast::Type`) which are syntactic.

### GoX Type Categories

| Category | Types | Zero Value | nil? |
|----------|-------|------------|------|
| Value | `int`, `float`, `bool`, `string`, `byte`, `[N]T` | type-specific | No |
| Object | `struct`, `interface`, `[]T`, `map[K]V`, `func` | `nil` | Yes |

### Data Structures

```rust
use crate::objects::{ObjKey, TypeKey, TCObjects};

#[derive(Clone)]
pub enum Type {
    /// Error placeholder for recovery
    Invalid,
    
    /// Basic types: int, float, bool, string, byte
    Basic(BasicType),
    
    /// Array: [N]T
    Array {
        len: i64,
        elem: TypeKey,
    },
    
    /// Slice: []T
    Slice {
        elem: TypeKey,
    },
    
    /// Map: map[K]V
    Map {
        key: TypeKey,
        value: TypeKey,
    },
    
    /// Channel: chan T
    Chan {
        elem: TypeKey,
    },
    
    /// Struct type
    Struct(StructType),
    
    /// Function signature
    Signature(Signature),
    
    /// Interface type
    Interface(InterfaceType),
    
    /// Named type (from type declaration)
    Named(NamedType),
}

#[derive(Clone, Copy, PartialEq)]
pub enum BasicType {
    Bool,
    Int,
    Float,
    String,
    Byte,
    /// Type of untyped nil
    UntypedNil,
}

#[derive(Clone)]
pub struct StructType {
    /// Fields (each is a Var-kind LangObj)
    pub fields: Vec<ObjKey>,
}

#[derive(Clone)]
pub struct Signature {
    /// Receiver for methods, None for functions
    pub recv: Option<ObjKey>,
    /// Parameters
    pub params: Vec<ObjKey>,
    /// Result types
    pub results: Vec<TypeKey>,
}

#[derive(Clone)]
pub struct InterfaceType {
    /// Methods (each is a Func-kind LangObj with Signature type)
    pub methods: Vec<ObjKey>,
    /// Embedded interfaces
    pub embeddeds: Vec<TypeKey>,
}

#[derive(Clone)]
pub struct NamedType {
    /// The TypeName object this refers to
    pub obj: ObjKey,
    /// Underlying type
    pub underlying: TypeKey,
    /// Methods defined on this type
    pub methods: Vec<ObjKey>,
}
```

### Type Helper Methods

```rust
impl Type {
    /// Get underlying type (unwrap Named)
    pub fn underlying<'a>(&'a self, objs: &'a TCObjects) -> &'a Type {
        match self {
            Type::Named(n) => objs.types[n.underlying].underlying(objs),
            _ => self,
        }
    }
    
    /// Is this an object type (can be nil)?
    pub fn is_object(&self, objs: &TCObjects) -> bool {
        match self.underlying(objs) {
            Type::Struct(_) | Type::Interface(_) |
            Type::Slice { .. } | Type::Map { .. } |
            Type::Chan { .. } | Type::Signature(_) => true,
            _ => false,
        }
    }
    
    /// Is this a value type?
    pub fn is_value(&self, objs: &TCObjects) -> bool {
        !self.is_object(objs)
    }
    
    /// Is this a numeric type?
    pub fn is_numeric(&self, objs: &TCObjects) -> bool {
        match self.underlying(objs) {
            Type::Basic(b) => matches!(b, BasicType::Int | BasicType::Float | BasicType::Byte),
            _ => false,
        }
    }
    
    /// Can values of this type be compared with == / !=?
    pub fn is_comparable(&self, objs: &TCObjects) -> bool {
        match self.underlying(objs) {
            Type::Basic(b) => *b != BasicType::UntypedNil,
            Type::Array { elem, .. } => objs.types[*elem].is_comparable(objs),
            // Object types: only nil comparison allowed (handled separately)
            _ => false,
        }
    }
    
    /// Can values be ordered with < <= > >=?
    pub fn is_ordered(&self, objs: &TCObjects) -> bool {
        match self.underlying(objs) {
            Type::Basic(b) => matches!(b, 
                BasicType::Int | BasicType::Float | BasicType::Byte | BasicType::String),
            _ => false,
        }
    }
}

/// Check if two types are identical
pub fn identical(a: TypeKey, b: TypeKey, objs: &TCObjects) -> bool {
    if a == b {
        return true;
    }
    
    let ta = &objs.types[a];
    let tb = &objs.types[b];
    
    match (ta, tb) {
        (Type::Basic(a), Type::Basic(b)) => a == b,
        (Type::Array { len: la, elem: ea }, Type::Array { len: lb, elem: eb }) => {
            la == lb && identical(*ea, *eb, objs)
        }
        (Type::Slice { elem: ea }, Type::Slice { elem: eb }) => {
            identical(*ea, *eb, objs)
        }
        (Type::Map { key: ka, value: va }, Type::Map { key: kb, value: vb }) => {
            identical(*ka, *kb, objs) && identical(*va, *vb, objs)
        }
        // Named types are identical only if same ObjKey
        (Type::Named(a), Type::Named(b)) => a.obj == b.obj,
        _ => false,
    }
}

/// Check if src is assignable to dst
pub fn assignable_to(src: TypeKey, dst: TypeKey, objs: &TCObjects) -> bool {
    // Identical types are always assignable
    if identical(src, dst, objs) {
        return true;
    }
    
    let ts = &objs.types[src];
    let td = &objs.types[dst];
    
    // nil is assignable to any object type
    if matches!(ts, Type::Basic(BasicType::UntypedNil)) {
        return td.is_object(objs);
    }
    
    // Check underlying types (for named types)
    let us = ts.underlying(objs);
    let ud = td.underlying(objs);
    
    // If both are unnamed and identical underlying
    if !matches!(ts, Type::Named(_)) && !matches!(td, Type::Named(_)) {
        return identical_underlying(us, ud, objs);
    }
    
    // Interface assignment: src implements dst interface
    if let Type::Interface(iface) = ud {
        return implements_interface(src, iface, objs);
    }
    
    false
}
```

---

## 7. Module: universe.rs

### Purpose

Initialize predefined types and builtin functions.

### Implementation

```rust
use crate::objects::{TCObjects, ObjKey, TypeKey, ScopeKey};
use crate::obj::{LangObj, ObjKind, BuiltinKind};
use crate::typ::{Type, BasicType};
use crate::scope::{Scope, ScopeKind};
use gox_common::Span;

pub struct Universe {
    pub scope: ScopeKey,
    
    // Predefined types
    pub bool_type: TypeKey,
    pub int_type: TypeKey,
    pub float_type: TypeKey,
    pub string_type: TypeKey,
    pub byte_type: TypeKey,
    pub nil_type: TypeKey,
}

impl Universe {
    pub fn new(objs: &mut TCObjects) -> Self {
        // Create universe scope
        let scope_key = objs.scopes.alloc(Scope::new(
            None,
            ScopeKind::Universe,
            Span::new(0, 0),
        ));
        
        // Create basic types
        let bool_type = objs.types.alloc(Type::Basic(BasicType::Bool));
        let int_type = objs.types.alloc(Type::Basic(BasicType::Int));
        let float_type = objs.types.alloc(Type::Basic(BasicType::Float));
        let string_type = objs.types.alloc(Type::Basic(BasicType::String));
        let byte_type = objs.types.alloc(Type::Basic(BasicType::Byte));
        let nil_type = objs.types.alloc(Type::Basic(BasicType::UntypedNil));
        
        // Create type name objects and add to universe
        let span = Span::new(0, 0);
        
        add_type_name(objs, scope_key, "bool", bool_type, span);
        add_type_name(objs, scope_key, "int", int_type, span);
        add_type_name(objs, scope_key, "float", float_type, span);
        add_type_name(objs, scope_key, "string", string_type, span);
        add_type_name(objs, scope_key, "byte", byte_type, span);
        
        // Add builtin functions
        add_builtin(objs, scope_key, "len", BuiltinKind::Len, span);
        add_builtin(objs, scope_key, "cap", BuiltinKind::Cap, span);
        add_builtin(objs, scope_key, "append", BuiltinKind::Append, span);
        add_builtin(objs, scope_key, "make", BuiltinKind::Make, span);
        add_builtin(objs, scope_key, "print", BuiltinKind::Print, span);
        add_builtin(objs, scope_key, "println", BuiltinKind::Println, span);
        
        // Add true, false, nil
        add_const(objs, scope_key, "true", bool_type, ConstValue::Bool(true), span);
        add_const(objs, scope_key, "false", bool_type, ConstValue::Bool(false), span);
        // nil has special nil_type
        
        Self {
            scope: scope_key,
            bool_type,
            int_type,
            float_type,
            string_type,
            byte_type,
            nil_type,
        }
    }
}

fn add_type_name(objs: &mut TCObjects, scope: ScopeKey, name: &str, typ: TypeKey, span: Span) {
    let mut obj = LangObj::new(name.to_string(), ObjKind::TypeName, span);
    obj.typ = Some(typ);
    let key = objs.lobjs.alloc(obj);
    objs.scopes[scope].insert(name.to_string(), key);
}

fn add_builtin(objs: &mut TCObjects, scope: ScopeKey, name: &str, kind: BuiltinKind, span: Span) {
    let obj = LangObj::new(name.to_string(), ObjKind::Builtin(kind), span);
    let key = objs.lobjs.alloc(obj);
    objs.scopes[scope].insert(name.to_string(), key);
}

fn add_const(objs: &mut TCObjects, scope: ScopeKey, name: &str, typ: TypeKey, val: ConstValue, span: Span) {
    let mut obj = LangObj::new(name.to_string(), ObjKind::Const { value: val }, span);
    obj.typ = Some(typ);
    let key = objs.lobjs.alloc(obj);
    objs.scopes[scope].insert(name.to_string(), key);
}
```

---

## 8. Module: operand.rs

### Purpose

Represents the result of checking an expression. Carries type, mode (value/variable/constant), and span.

### Data Structures

```rust
use gox_common::Span;
use crate::objects::TypeKey;
use crate::obj::ConstValue;

/// Result of expression type checking
pub struct Operand {
    pub mode: OperandMode,
    pub typ: TypeKey,
    pub span: Span,
}

#[derive(Clone)]
pub enum OperandMode {
    /// Invalid expression (error occurred)
    Invalid,
    /// A value (rvalue)
    Value,
    /// A variable (lvalue, addressable)
    Variable,
    /// A constant with known value
    Constant(ConstValue),
    /// A type expression (not a value)
    TypeExpr,
    /// A builtin function (needs special call handling)
    Builtin,
}

impl Operand {
    pub fn invalid(span: Span, nil_type: TypeKey) -> Self {
        Self {
            mode: OperandMode::Invalid,
            typ: nil_type, // Use a placeholder
            span,
        }
    }
    
    pub fn value(typ: TypeKey, span: Span) -> Self {
        Self { mode: OperandMode::Value, typ, span }
    }
    
    pub fn variable(typ: TypeKey, span: Span) -> Self {
        Self { mode: OperandMode::Variable, typ, span }
    }
    
    pub fn constant(typ: TypeKey, value: ConstValue, span: Span) -> Self {
        Self { mode: OperandMode::Constant(value), typ, span }
    }
    
    pub fn is_invalid(&self) -> bool {
        matches!(self.mode, OperandMode::Invalid)
    }
    
    pub fn is_addressable(&self) -> bool {
        matches!(self.mode, OperandMode::Variable)
    }
}
```

---

## 9. Module: check/mod.rs

### Purpose

Main `Checker` struct that orchestrates type checking.

### Data Structures

```rust
use gox_common::{Span, FileId};
use gox_common::diagnostic::DiagnosticBag;
use gox_syntax::ast;
use crate::objects::{TCObjects, ObjKey, TypeKey, ScopeKey};
use crate::scope::{Scope, ScopeKind};
use crate::universe::Universe;
use std::collections::HashMap;

/// Type checking result for expressions
pub struct TypeAndValue {
    pub mode: OperandMode,
    pub typ: TypeKey,
}

/// Complete type information for a file
pub struct TypeInfo {
    /// Expression types
    pub types: HashMap<Span, TypeAndValue>,
    /// Identifier definitions
    pub defs: HashMap<Span, Option<ObjKey>>,
    /// Identifier uses
    pub uses: HashMap<Span, ObjKey>,
    /// Scopes
    pub scopes: HashMap<Span, ScopeKey>,
}

pub struct Checker<'a> {
    // Input
    pub ast: &'a ast::SourceFile,
    pub file_id: FileId,
    
    // Storage
    pub objs: TCObjects,
    pub universe: Universe,
    
    // Scopes
    pub pkg_scope: ScopeKey,
    pub scope: ScopeKey,  // Current scope
    
    // Function context (for return checking)
    pub func_results: Option<Vec<TypeKey>>,
    
    // Output
    pub info: TypeInfo,
    pub diagnostics: DiagnosticBag,
}

impl<'a> Checker<'a> {
    pub fn new(ast: &'a ast::SourceFile, file_id: FileId) -> Self {
        let mut objs = TCObjects::new();
        let universe = Universe::new(&mut objs);
        
        // Create package scope
        let pkg_scope = objs.scopes.alloc(Scope::new(
            Some(universe.scope),
            ScopeKind::Package,
            ast.span,
        ));
        
        Self {
            ast,
            file_id,
            objs,
            universe,
            pkg_scope,
            scope: pkg_scope,
            func_results: None,
            info: TypeInfo::new(),
            diagnostics: DiagnosticBag::new(),
        }
    }
    
    pub fn run(&mut self) {
        // Phase 1: Collect declarations
        self.resolve_decls();
        
        // Phase 2: Check type bodies (struct fields, interface methods)
        self.check_type_bodies();
        
        // Phase 3: Check function bodies and initializers
        self.check_bodies();
        
        // Phase 4: Check implements declarations
        self.check_implements();
    }
    
    // Scope management
    pub fn enter_scope(&mut self, kind: ScopeKind, span: Span) {
        let new_scope = self.objs.scopes.alloc(Scope::new(
            Some(self.scope),
            kind,
            span,
        ));
        self.objs.scopes[self.scope].children.push(new_scope);
        self.scope = new_scope;
    }
    
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.objs.scopes[self.scope].parent {
            self.scope = parent;
        }
    }
    
    // Error reporting
    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        use gox_common::diagnostic::Diagnostic;
        self.diagnostics.add(
            Diagnostic::error(msg).with_label(span, self.file_id, "")
        );
    }
    
    pub fn error_at(&mut self, span: Span, msg: impl Into<String>, note_span: Span, note: &str) {
        use gox_common::diagnostic::Diagnostic;
        self.diagnostics.add(
            Diagnostic::error(msg)
                .with_label(span, self.file_id, "")
                .with_secondary_label(note_span, self.file_id, note)
        );
    }
}
```

---

## 10. Module: check/resolver.rs

### Purpose

Phase 1: Collect all declarations and add them to scope. Does not check bodies.

### Implementation

```rust
impl Checker<'_> {
    pub fn resolve_decls(&mut self) {
        for decl in &self.ast.decls {
            match decl {
                TopDecl::Var(v) => self.declare_var(v),
                TopDecl::Const(c) => self.declare_const(c),
                TopDecl::Type(t) => self.declare_type(t),
                TopDecl::Interface(i) => self.declare_interface(i),
                TopDecl::Func(f) => self.declare_func(f),
                TopDecl::Implements(_) => {} // Phase 4
            }
        }
    }
    
    fn declare_var(&mut self, decl: &VarDecl) {
        for spec in &decl.specs {
            // Create object (type will be filled later)
            let obj = LangObj::new(
                spec.name.name.clone(),
                ObjKind::Var { is_field: false, is_param: false },
                spec.span,
            );
            let key = self.objs.lobjs.alloc(obj);
            
            // Check for redeclaration
            if let Some(old) = self.objs.scopes[self.scope].insert(spec.name.name.clone(), key) {
                let old_span = self.objs.lobjs[old].decl_span;
                self.error_at(spec.name.span, 
                    format!("{} redeclared", spec.name.name),
                    old_span, "previously declared here");
            }
            
            // Record definition
            self.info.defs.insert(spec.name.span, Some(key));
        }
    }
    
    fn declare_const(&mut self, decl: &ConstDecl) {
        // Similar to declare_var, but ObjKind::Const
    }
    
    fn declare_type(&mut self, decl: &TypeDecl) {
        let obj = LangObj::new(
            decl.name.name.clone(),
            ObjKind::TypeName,
            decl.span,
        );
        let key = self.objs.lobjs.alloc(obj);
        
        // Insert and check for redeclaration
        if let Some(old) = self.objs.scopes[self.scope].insert(decl.name.name.clone(), key) {
            // Error...
        }
        
        self.info.defs.insert(decl.name.span, Some(key));
    }
    
    fn declare_interface(&mut self, decl: &InterfaceDecl) {
        // Similar to declare_type
    }
    
    fn declare_func(&mut self, decl: &FuncDecl) {
        let is_method = decl.receiver.is_some();
        let obj = LangObj::new(
            decl.name.name.clone(),
            ObjKind::Func { is_method },
            decl.span,
        );
        let key = self.objs.lobjs.alloc(obj);
        
        if !is_method {
            // Add to package scope
            self.objs.scopes[self.scope].insert(decl.name.name.clone(), key);
        }
        // Methods are added to their receiver type later
        
        self.info.defs.insert(decl.name.span, Some(key));
    }
}
```

---

## 11. Module: check/typexpr.rs

### Purpose

Convert AST type expressions to semantic types.

### Implementation

```rust
impl Checker<'_> {
    /// Resolve an AST type to a semantic TypeKey
    pub fn resolve_type(&mut self, ty: &ast::Type) -> TypeKey {
        match ty {
            ast::Type::Named(ident) => self.resolve_type_name(ident),
            ast::Type::Array(arr) => self.resolve_array_type(arr),
            ast::Type::Slice(slice) => self.resolve_slice_type(slice),
            ast::Type::Map(map) => self.resolve_map_type(map),
            ast::Type::Chan(chan) => self.resolve_chan_type(chan),
            ast::Type::Func(func) => self.resolve_func_type(func),
            ast::Type::Struct(st) => self.resolve_struct_type(st),
        }
    }
    
    fn resolve_type_name(&mut self, ident: &ast::Ident) -> TypeKey {
        // Look up the name
        if let Some((_, obj_key)) = scope::lookup(self.scope, &ident.name, &self.objs) {
            let obj = &self.objs.lobjs[obj_key];
            if obj.is_type() {
                if let Some(typ) = obj.typ {
                    self.info.uses.insert(ident.span, obj_key);
                    return typ;
                }
            } else {
                self.error(ident.span, format!("{} is not a type", ident.name));
            }
        } else {
            self.error(ident.span, format!("undefined: {}", ident.name));
        }
        
        // Return invalid type for error recovery
        self.objs.types.alloc(Type::Invalid)
    }
    
    fn resolve_array_type(&mut self, arr: &ast::ArrayType) -> TypeKey {
        let elem = self.resolve_type(&arr.elem);
        self.objs.types.alloc(Type::Array { len: arr.len, elem })
    }
    
    fn resolve_slice_type(&mut self, slice: &ast::SliceType) -> TypeKey {
        let elem = self.resolve_type(&slice.elem);
        self.objs.types.alloc(Type::Slice { elem })
    }
    
    fn resolve_map_type(&mut self, map: &ast::MapType) -> TypeKey {
        let key = self.resolve_type(&map.key);
        let value = self.resolve_type(&map.value);
        
        // Validate: key must be comparable value type
        let key_type = &self.objs.types[key];
        if !key_type.is_comparable(&self.objs) || key_type.is_object(&self.objs) {
            self.error(map.key.span(), "invalid map key type");
        }
        
        self.objs.types.alloc(Type::Map { key, value })
    }
    
    fn resolve_struct_type(&mut self, st: &ast::StructType) -> TypeKey {
        let mut fields = Vec::new();
        
        for field in &st.fields {
            let field_type = self.resolve_type(&field.ty);
            let mut obj = LangObj::new(
                field.name.name.clone(),
                ObjKind::Var { is_field: true, is_param: false },
                field.span,
            );
            obj.typ = Some(field_type);
            let key = self.objs.lobjs.alloc(obj);
            fields.push(key);
        }
        
        self.objs.types.alloc(Type::Struct(StructType { fields }))
    }
}
```

---

## 12. Module: check/expr.rs

### Purpose

Type check expressions, return `Operand`.

### Key Functions

```rust
impl Checker<'_> {
    pub fn check_expr(&mut self, expr: &ast::Expr) -> Operand {
        let op = match expr {
            ast::Expr::Ident(id) => self.check_ident(id),
            ast::Expr::Int(v, span) => self.check_int(*v, *span),
            ast::Expr::Float(v, span) => self.check_float(*v, *span),
            ast::Expr::String(s, span) => self.check_string(s.clone(), *span),
            ast::Expr::Bool(v, span) => self.check_bool(*v, *span),
            ast::Expr::Nil(span) => self.check_nil(*span),
            ast::Expr::Binary(b) => self.check_binary(b),
            ast::Expr::Unary(u) => self.check_unary(u),
            ast::Expr::Call(c) => self.check_call(c),
            ast::Expr::Index(i) => self.check_index(i),
            ast::Expr::Slice(s) => self.check_slice(s),
            ast::Expr::Selector(s) => self.check_selector(s),
            ast::Expr::TypeAssert(ta) => self.check_type_assert(ta),
            ast::Expr::CompositeLit(c) => self.check_composite_lit(c),
            ast::Expr::FuncLit(f) => self.check_func_lit(f),
            ast::Expr::Paren(p) => self.check_expr(&p.expr),
            ast::Expr::Recv(r) => self.check_recv(r),
            ast::Expr::Make(m) => self.check_make(m),
        };
        
        // Record in TypeInfo
        self.info.types.insert(expr.span(), TypeAndValue {
            mode: op.mode.clone(),
            typ: op.typ,
        });
        
        op
    }
    
    fn check_ident(&mut self, ident: &ast::Ident) -> Operand {
        if let Some((_, obj_key)) = scope::lookup(self.scope, &ident.name, &self.objs) {
            let obj = &self.objs.lobjs[obj_key];
            self.info.uses.insert(ident.span, obj_key);
            
            match &obj.kind {
                ObjKind::Var { .. } => {
                    Operand::variable(obj.typ.unwrap(), ident.span)
                }
                ObjKind::Const { value } => {
                    Operand::constant(obj.typ.unwrap(), value.clone(), ident.span)
                }
                ObjKind::Func { .. } => {
                    Operand::value(obj.typ.unwrap(), ident.span)
                }
                ObjKind::TypeName => {
                    Operand {
                        mode: OperandMode::TypeExpr,
                        typ: obj.typ.unwrap(),
                        span: ident.span,
                    }
                }
                ObjKind::Builtin(_) => {
                    Operand {
                        mode: OperandMode::Builtin,
                        typ: self.universe.nil_type, // placeholder
                        span: ident.span,
                    }
                }
            }
        } else {
            self.error(ident.span, format!("undefined: {}", ident.name));
            Operand::invalid(ident.span, self.universe.nil_type)
        }
    }
    
    fn check_int(&mut self, v: i64, span: Span) -> Operand {
        Operand::constant(self.universe.int_type, ConstValue::Int(v), span)
    }
    
    fn check_float(&mut self, v: f64, span: Span) -> Operand {
        Operand::constant(self.universe.float_type, ConstValue::Float(v), span)
    }
    
    fn check_string(&mut self, s: String, span: Span) -> Operand {
        Operand::constant(self.universe.string_type, ConstValue::String(s), span)
    }
    
    fn check_bool(&mut self, v: bool, span: Span) -> Operand {
        Operand::constant(self.universe.bool_type, ConstValue::Bool(v), span)
    }
    
    fn check_nil(&mut self, span: Span) -> Operand {
        Operand::value(self.universe.nil_type, span)
    }
    
    fn check_binary(&mut self, expr: &ast::BinaryExpr) -> Operand {
        let left = self.check_expr(&expr.left);
        let right = self.check_expr(&expr.right);
        
        // Skip if either side is invalid
        if left.is_invalid() || right.is_invalid() {
            return Operand::invalid(expr.span, self.universe.nil_type);
        }
        
        use ast::BinaryOp::*;
        match expr.op {
            Add | Sub | Mul | Div | Mod => {
                self.check_arithmetic(left, right, expr.span)
            }
            Eq | NotEq => {
                self.check_equality(left, right, expr.span)
            }
            Lt | LtEq | Gt | GtEq => {
                self.check_ordering(left, right, expr.span)
            }
            And | Or => {
                self.check_logical(left, right, expr.span)
            }
        }
    }
    
    fn check_arithmetic(&mut self, left: Operand, right: Operand, span: Span) -> Operand {
        // String concatenation with +
        if matches!(self.objs.types[left.typ].underlying(&self.objs), Type::Basic(BasicType::String)) {
            // TODO: handle string + string
        }
        
        // Numeric operations
        if !self.objs.types[left.typ].is_numeric(&self.objs) {
            self.error(left.span, "operator requires numeric operand");
            return Operand::invalid(span, self.universe.nil_type);
        }
        if !self.objs.types[right.typ].is_numeric(&self.objs) {
            self.error(right.span, "operator requires numeric operand");
            return Operand::invalid(span, self.universe.nil_type);
        }
        
        if !identical(left.typ, right.typ, &self.objs) {
            self.error(span, "mismatched types in binary expression");
            return Operand::invalid(span, self.universe.nil_type);
        }
        
        Operand::value(left.typ, span)
    }
}
```

---

## 13. Module: check/stmt.rs

### Purpose

Type check statements.

### Key Functions

```rust
impl Checker<'_> {
    pub fn check_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Block(b) => self.check_block(b),
            ast::Stmt::Var(v) => self.check_var_stmt(v),
            ast::Stmt::Const(c) => self.check_const_stmt(c),
            ast::Stmt::ShortVar(sv) => self.check_short_var(sv),
            ast::Stmt::Assign(a) => self.check_assign(a),
            ast::Stmt::Expr(e) => { self.check_expr(&e.expr); }
            ast::Stmt::Return(r) => self.check_return(r),
            ast::Stmt::If(i) => self.check_if(i),
            ast::Stmt::For(f) => self.check_for(f),
            ast::Stmt::Switch(s) => self.check_switch(s),
            ast::Stmt::ForRange(fr) => self.check_for_range(fr),
            ast::Stmt::TypeSwitch(ts) => self.check_type_switch(ts),
            ast::Stmt::Select(s) => self.check_select(s),
            ast::Stmt::Go(g) => self.check_go(g),
            ast::Stmt::Defer(d) => self.check_defer(d),
            ast::Stmt::Send(s) => self.check_send(s),
            ast::Stmt::Goto(_) | ast::Stmt::Labeled(_) | ast::Stmt::Fallthrough(_) |
            ast::Stmt::Break(_) | ast::Stmt::Continue(_) | ast::Stmt::Empty(_) => {}
        }
    }
    
    fn check_block(&mut self, block: &ast::Block) {
        self.enter_scope(ScopeKind::Block, block.span);
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        self.exit_scope();
    }
    
    fn check_short_var(&mut self, decl: &ast::ShortVarDecl) {
        // GoX: := always declares new variables
        if decl.names.len() != decl.values.len() {
            self.error(decl.span, "assignment mismatch");
            return;
        }
        
        for (name, value) in decl.names.iter().zip(&decl.values) {
            let op = self.check_expr(value);
            
            // Cannot infer type from nil
            if matches!(self.objs.types[op.typ], Type::Basic(BasicType::UntypedNil)) {
                self.error(name.span, "cannot infer type from nil");
                continue;
            }
            
            // Create new variable
            let mut obj = LangObj::new(
                name.name.clone(),
                ObjKind::Var { is_field: false, is_param: false },
                name.span,
            );
            obj.typ = Some(op.typ);
            let key = self.objs.lobjs.alloc(obj);
            
            // Note: no redeclaration check - := always shadows
            self.objs.scopes[self.scope].insert(name.name.clone(), key);
            self.info.defs.insert(name.span, Some(key));
        }
    }
    
    fn check_return(&mut self, ret: &ast::ReturnStmt) {
        let expected = match &self.func_results {
            Some(r) => r.clone(),
            None => {
                self.error(ret.span, "return outside function");
                return;
            }
        };
        
        if ret.values.len() != expected.len() {
            self.error(ret.span, format!(
                "wrong number of return values: got {}, want {}",
                ret.values.len(), expected.len()
            ));
            return;
        }
        
        for (expr, expected_type) in ret.values.iter().zip(&expected) {
            let op = self.check_expr(expr);
            if !assignable_to(op.typ, *expected_type, &self.objs) {
                self.error(expr.span(), "cannot use value as return type");
            }
        }
    }
    
    fn check_if(&mut self, stmt: &ast::IfStmt) {
        let cond = self.check_expr(&stmt.cond);
        if !matches!(self.objs.types[cond.typ].underlying(&self.objs), Type::Basic(BasicType::Bool)) {
            self.error(stmt.cond.span(), "non-boolean condition in if statement");
        }
        
        self.check_block(&stmt.then_block);
        
        if let Some(else_clause) = &stmt.else_clause {
            match else_clause {
                ast::ElseClause::Block(b) => self.check_block(b),
                ast::ElseClause::If(elif) => self.check_if(elif),
            }
        }
    }
}
```

---

## 14. Module: check/interface.rs

### Purpose

Check interface declarations and `implements` statements.

### Key Functions

```rust
impl Checker<'_> {
    pub fn check_implements(&mut self) {
        for decl in &self.ast.decls {
            if let TopDecl::Implements(impl_decl) = decl {
                self.check_implements_decl(impl_decl);
            }
        }
    }
    
    fn check_implements_decl(&mut self, decl: &ast::ImplementsDecl) {
        // Resolve the type being checked
        let type_key = if let Some((_, obj)) = scope::lookup(self.scope, &decl.type_name.name, &self.objs) {
            if let Some(typ) = self.objs.lobjs[obj].typ {
                typ
            } else {
                self.error(decl.type_name.span, "type has no definition");
                return;
            }
        } else {
            self.error(decl.type_name.span, format!("undefined: {}", decl.type_name.name));
            return;
        };
        
        // Check against each interface
        for iface_name in &decl.interfaces {
            let iface_key = if let Some((_, obj)) = scope::lookup(self.scope, &iface_name.name, &self.objs) {
                if let Some(typ) = self.objs.lobjs[obj].typ {
                    typ
                } else {
                    continue;
                }
            } else {
                self.error(iface_name.span, format!("undefined: {}", iface_name.name));
                continue;
            };
            
            // Verify type implements interface
            if let Type::Interface(iface) = &self.objs.types[iface_key] {
                self.verify_implements(type_key, iface, decl.span);
            } else {
                self.error(iface_name.span, format!("{} is not an interface", iface_name.name));
            }
        }
    }
    
    fn verify_implements(&mut self, typ: TypeKey, iface: &InterfaceType, span: Span) {
        // Get methods of typ
        let type_methods = self.get_method_set(typ);
        
        // Check each interface method is present
        for method_key in &iface.methods {
            let method = &self.objs.lobjs[*method_key];
            
            if !type_methods.contains_key(&method.name) {
                self.error(span, format!("missing method: {}", method.name));
            } else {
                // TODO: verify signature matches
            }
        }
    }
    
    fn get_method_set(&self, typ: TypeKey) -> HashMap<String, ObjKey> {
        let mut methods = HashMap::new();
        
        if let Type::Named(named) = &self.objs.types[typ] {
            for m in &named.methods {
                let obj = &self.objs.lobjs[*m];
                methods.insert(obj.name.clone(), *m);
            }
        }
        
        methods
    }
}
```

---

## 15. Implementation Order

### Phase 1: Foundation (Week 1)
1. `objects.rs` - Arena, Idx types
2. `obj.rs` - LangObj, ObjKind
3. `scope.rs` - Scope, lookup
4. `typ.rs` - Type enum (Basic, Invalid only)
5. `universe.rs` - Basic types, builtins
6. `check/mod.rs` - Checker skeleton, empty run()

**Deliverable**: Can create checker, has predefined types

### Phase 2: Declarations (Week 2)
1. `check/resolver.rs` - resolve_decls()
2. `check/typexpr.rs` - resolve_type()
3. `check/decl.rs` - check type declarations

**Deliverable**: Can collect declarations, resolve type names

### Phase 3: Expressions (Week 3)
1. `operand.rs` - Operand struct
2. `check/expr.rs` - literals, idents, binary, unary
3. `check/call.rs` - function calls
4. `check/builtin.rs` - builtin functions

**Deliverable**: Can type check expressions

### Phase 4: Statements (Week 4)
1. `check/stmt.rs` - all statement types
2. `check/assignment.rs` - assignment validation

**Deliverable**: Can type check function bodies

### Phase 5: Interfaces (Week 5)
1. Complete `typ.rs` - InterfaceType, NamedType.methods
2. `check/interface.rs` - implements verification

**Deliverable**: Full type checker

---

## 16. Testing Strategy

### Unit Tests

Each module should have `#[cfg(test)] mod tests`.

### Integration Tests

```rust
// tests/integration.rs

fn check(src: &str) -> Analysis {
    let file = gox_syntax::parser::parse(src).expect("parse error");
    gox_analysis::check(&file, 0)
}

#[test]
fn test_valid_var_decl() {
    let r = check("var x int = 42;");
    assert!(!r.diagnostics.has_errors());
}

#[test]
fn test_type_mismatch() {
    let r = check("var x int = \"hello\";");
    assert!(r.diagnostics.has_errors());
}

#[test]
fn test_nil_to_value_type() {
    let r = check("var x int = nil;");
    assert!(r.diagnostics.has_errors());
}

#[test]
fn test_nil_to_struct() {
    let r = check("type User struct {}; var u User = nil;");
    assert!(!r.diagnostics.has_errors());
}

#[test]
fn test_implements() {
    let r = check(r#"
        interface Greeter { Greet() string; };
        type User struct { name string; };
        func (u User) Greet() string { return u.name; }
        implements User : Greeter;
    "#);
    assert!(!r.diagnostics.has_errors());
}

#[test]
fn test_missing_method() {
    let r = check(r#"
        interface Greeter { Greet() string; };
        type User struct {};
        implements User : Greeter;
    "#);
    assert!(r.diagnostics.has_errors());
}
```
