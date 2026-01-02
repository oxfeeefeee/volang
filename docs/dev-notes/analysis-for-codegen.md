# Vo Analysis Crate: Complete Reference for Codegen

This document provides a comprehensive reference of all information collected by `vo-analysis` during type checking, intended for codegen developers to understand what data is available and avoid redundant computation.

---

## Table of Contents

1. [Entry Points](#1-entry-points)
2. [Core Data Structures](#2-core-data-structures)
3. [TCObjects - The Central Container](#3-tcobjects---the-central-container)
4. [TypeInfo - Type Checking Results](#4-typeinfo---type-checking-results)
5. [Type System](#5-type-system)
6. [Language Objects (LangObj)](#6-language-objects-langobj)
7. [Scopes](#7-scopes)
8. [Packages](#8-packages)
9. [Selections](#9-selections)
10. [Operands](#10-operands)
11. [Constant Values](#11-constant-values)
12. [Escape Analysis Results](#12-escape-analysis-results)
13. [Type Layout Utility Functions](#13-type-layout-utility-functions)
14. [Lookup Functions](#14-lookup-functions)
15. [Key Type Mappings for Codegen](#15-key-type-mappings-for-codegen)

---

## 1. Entry Points

### `analyze_project`
```rust
pub fn analyze_project(files: FileSet, vfs: &Vfs) -> Result<Project, AnalysisError>
```
Main entry point for analyzing a multi-file project with import resolution.

### `analyze_single_file`
```rust
pub fn analyze_single_file(file: File, interner: SymbolInterner) -> Result<Project, AnalysisError>
```
Simplified API for single file (no imports).

### `Project` struct
```rust
pub struct Project {
    pub tc_objs: TCObjects,           // All type-checked objects
    pub interner: SymbolInterner,     // Symbol table
    pub packages: Vec<PackageKey>,    // Packages in dependency order
    pub main_package: PackageKey,     // Main package key
    pub type_info: TypeInfo,          // Type checking results for main package
    pub files: Vec<File>,             // Parsed AST files
    pub imported_files: HashMap<String, Vec<File>>,        // Package path -> files
    pub imported_type_infos: HashMap<String, TypeInfo>,    // Package path -> type_info
}
```

---

## 2. Core Data Structures

### Keys (Arena Indices)
All major objects are stored in arenas and referenced by typed keys:

```rust
pub struct ObjKey;      // Language objects (vars, funcs, types, etc.)
pub struct TypeKey;     // Types
pub struct ScopeKey;    // Scopes
pub struct PackageKey;  // Packages
pub struct DeclInfoKey; // Declaration info
```

Key operations:
- `key.as_usize()` - Get raw index
- `key.is_null()` - Check if null key
- `ObjKey::from_usize(n)` - Create from index

---

## 3. TCObjects - The Central Container

```rust
pub struct TCObjects {
    pub lobjs: Arena<ObjKey, LangObj>,       // All language objects
    pub types: Arena<TypeKey, Type>,         // All types
    pub scopes: Arena<ScopeKey, Scope>,      // All scopes
    pub pkgs: Arena<PackageKey, Package>,    // All packages
    pub decls: Arena<DeclInfoKey, DeclInfo>, // Declaration info
    pub universe: Option<Universe>,          // Predefined types/funcs
    pub fmt_qualifier: FmtQualifier,         // Formatting helper
}
```

### Access Patterns
```rust
// Get a language object
let obj = &tc_objs.lobjs[okey];

// Get a type
let typ = &tc_objs.types[tkey];

// Get a scope
let scope = &tc_objs.scopes[skey];

// Get universe
let univ = tc_objs.universe();
```

### Factory Methods
```rust
tc_objs.new_package(path)
tc_objs.new_scope(parent, pos, end, comment, is_func)
tc_objs.new_var(pos, pkg, name, typ)
tc_objs.new_const(pos, pkg, name, typ, val)
tc_objs.new_func(pos, pkg, name, typ)
tc_objs.new_type_name(pos, pkg, name, typ)
tc_objs.new_t_array(elem, len)
tc_objs.new_t_slice(elem)
tc_objs.new_t_struct(fields, tags)
tc_objs.new_t_pointer(base)
tc_objs.new_t_signature(scope, recv, params, results, variadic)
tc_objs.new_t_interface(methods, embeddeds)
tc_objs.new_t_map(key, elem)
tc_objs.new_t_chan(dir, elem)
tc_objs.new_t_named(obj, underlying, methods)
```

---

## 4. TypeInfo - Type Checking Results

```rust
pub struct TypeInfo {
    // Expression types and values (for constants)
    pub types: HashMap<ExprId, TypeAndValue>,
    
    // Type expression -> resolved TypeKey
    pub type_exprs: HashMap<TypeExprId, TypeKey>,
    
    // Identifier definitions: IdentId -> defining ObjKey (or None for _)
    pub defs: HashMap<IdentId, Option<ObjKey>>,
    
    // Identifier uses: IdentId -> referenced ObjKey
    pub uses: HashMap<IdentId, ObjKey>,
    
    // Implicit objects (e.g., implicit import names)
    pub implicits: HashMap<Span, ObjKey>,
    
    // Selector expressions -> Selection info
    pub selections: HashMap<ExprId, Selection>,
    
    // AST spans -> scopes they define
    pub scopes: HashMap<Span, ScopeKey>,
    
    // Package-level var initializers in execution order
    pub init_order: Vec<Initializer>,
    
    // Variables that escape to heap (from escape analysis)
    pub escaped_vars: HashSet<ObjKey>,
    
    // Closure captures: FuncLit ExprId -> captured vars
    pub closure_captures: HashMap<ExprId, Vec<ObjKey>>,
}
```

### TypeAndValue
```rust
pub struct TypeAndValue {
    pub mode: OperandMode,  // constant, variable, value, etc.
    pub typ: TypeKey,       // The type
}
```

### Initializer
```rust
pub struct Initializer {
    pub lhs: Vec<ObjKey>,   // Variables being initialized
    pub rhs: Expr,          // The initialization expression
}
```

### Key Lookups
```rust
// Get type of an expression
type_info.expr_type(expr_id) -> Option<TypeKey>

// Get mode of an expression
type_info.expr_mode(expr_id) -> Option<&OperandMode>

// Get object for definition
type_info.get_def(ident) -> Option<ObjKey>

// Get object for use
type_info.get_use(ident) -> Option<ObjKey>

// Check if variable escapes
type_info.is_escaped(obj) -> bool
```

---

## 5. Type System

### Type enum
```rust
pub enum Type {
    Basic(BasicDetail),
    Array(ArrayDetail),
    Slice(SliceDetail),
    Struct(StructDetail),
    Pointer(PointerDetail),
    Tuple(TupleDetail),
    Signature(SignatureDetail),
    Interface(InterfaceDetail),
    Map(MapDetail),
    Chan(ChanDetail),
    Named(NamedDetail),
}
```

### Type Details

#### BasicDetail
```rust
pub struct BasicDetail {
    typ: BasicType,      // Bool, Int, Int8, ..., Str, Untyped*
    info: BasicInfo,     // IsBoolean, IsInteger, IsFloat, IsString
    name: &'static str,
}
```

#### ArrayDetail
```rust
pub struct ArrayDetail {
    len: Option<u64>,   // Array length
    elem: TypeKey,      // Element type
}
```

#### SliceDetail
```rust
pub struct SliceDetail {
    elem: TypeKey,      // Element type
}
```

#### StructDetail
```rust
pub struct StructDetail {
    fields: Vec<ObjKey>,               // Field objects
    tags: Option<Vec<Option<String>>>, // Struct tags
}
```

#### PointerDetail
```rust
pub struct PointerDetail {
    base: TypeKey,  // Base type (in Vo, only structs can be pointed to)
}
```

#### TupleDetail
```rust
pub struct TupleDetail {
    vars: Vec<ObjKey>,  // Tuple elements (used for params/results)
}
```

#### SignatureDetail
```rust
pub struct SignatureDetail {
    scope: Option<ScopeKey>,  // Function scope
    recv: Option<ObjKey>,     // Receiver for methods
    params: TypeKey,          // Params tuple
    results: TypeKey,         // Results tuple
    variadic: bool,
}
```

#### InterfaceDetail
```rust
pub struct InterfaceDetail {
    methods: Vec<ObjKey>,                              // Explicitly declared methods
    embeddeds: Vec<TypeKey>,                           // Embedded interfaces
    all_methods: Rc<RefCell<Option<Vec<ObjKey>>>>,     // All methods (including embedded)
}
```

#### MapDetail
```rust
pub struct MapDetail {
    key: TypeKey,
    elem: TypeKey,
}
```

#### ChanDetail
```rust
pub struct ChanDetail {
    dir: ChanDir,    // SendRecv, SendOnly, RecvOnly
    elem: TypeKey,
}
```

#### NamedDetail
```rust
pub struct NamedDetail {
    obj: Option<ObjKey>,        // Type name object
    underlying: Option<TypeKey>, // Underlying type
    methods: Vec<ObjKey>,       // Methods
}
```

### Type Utility Functions
```rust
// Get underlying type (unwrap Named)
typ::underlying_type(t, tc_objs) -> TypeKey

// Get deep underlying type (follow chain)
typ::deep_underlying_type(t, tc_objs) -> TypeKey

// Type predicates
typ::is_boolean(t, tc_objs) -> bool
typ::is_integer(t, tc_objs) -> bool
typ::is_unsigned(t, tc_objs) -> bool
typ::is_float(t, tc_objs) -> bool
typ::is_numeric(t, tc_objs) -> bool
typ::is_string(t, tc_objs) -> bool
typ::is_typed(t, tc_objs) -> bool
typ::is_untyped(t, tc_objs) -> bool
typ::is_interface(t, tc_objs) -> bool
typ::has_nil(t, tc_objs) -> bool
typ::comparable(t, tc_objs) -> bool

// Type identity
typ::identical(x, y, tc_objs) -> bool
typ::identical_ignore_tags(x, y, tc_objs) -> bool

// Default type for untyped
typ::untyped_default_type(t, tc_objs) -> TypeKey
```

### BasicType enum
```rust
pub enum BasicType {
    Invalid,
    Bool, Int, Int8, Int16, Int32, Int64,
    Uint, Uint8, Uint16, Uint32, Uint64, Uintptr,
    Float32, Float64,
    Str,
    // Untyped (for constants)
    UntypedBool, UntypedInt, UntypedRune, UntypedFloat, UntypedString, UntypedNil,
    // Aliases
    Byte,  // = Uint8
    Rune,  // = Int32
}
```

---

## 6. Language Objects (LangObj)

```rust
pub struct LangObj {
    entity_type: EntityType,     // Kind of object
    parent: Option<ScopeKey>,    // Parent scope
    pos: Pos,                    // Source position
    pkg: Option<PackageKey>,     // Package
    name: String,                // Name
    typ: Option<TypeKey>,        // Type
    order: u32,                  // Declaration order
    color: ObjColor,             // For cycle detection
    scope_pos: Pos,              // When visible in scope
}
```

### EntityType enum
```rust
pub enum EntityType {
    PkgName { imported: PackageKey, used: bool },
    Const { val: ConstValue },
    TypeName,
    Var(VarProperty),
    Func { has_ptr_recv: bool },
    Label { used: bool },
    Builtin(Builtin),
    Nil,
}
```

### VarProperty
```rust
pub struct VarProperty {
    pub embedded: bool,   // Is embedded field
    pub is_field: bool,   // Is struct field
    pub used: bool,       // Was used
}
```

### LangObj Methods
```rust
obj.entity_type() -> &EntityType
obj.name() -> &str
obj.typ() -> Option<TypeKey>
obj.pkg() -> Option<PackageKey>
obj.pos() -> Pos
obj.order() -> u32
obj.parent() -> Option<ScopeKey>
obj.exported() -> bool

// For specific entity types
obj.const_val() -> &ConstValue        // For Const
obj.var_embedded() -> bool            // For Var
obj.var_is_field() -> bool            // For Var
obj.func_has_ptr_recv() -> bool       // For Func (method with *T receiver)
obj.pkg_name_imported() -> PackageKey // For PkgName
```

### Builtin enum
```rust
pub enum Builtin {
    Append, Cap, Close, Copy, Delete, Len, Make, New,
    Panic, Print, Println, Recover, Assert,
}
```

---

## 7. Scopes

```rust
pub struct Scope {
    parent: Option<ScopeKey>,
    children: Vec<ScopeKey>,
    elems: HashMap<String, ObjKey>,
    pos: Pos,
    end: Pos,
    comment: String,
    is_func: bool,
}
```

### Scope Methods
```rust
scope.parent() -> Option<ScopeKey>
scope.children() -> &[ScopeKey]
scope.lookup(name) -> Option<ObjKey>
scope.names() -> impl Iterator<Item = &String>
scope.objects() -> impl Iterator<Item = ObjKey>
scope.elems() -> &HashMap<String, ObjKey>
scope.is_func() -> bool
scope.contains(pos) -> bool
```

### Scope Lookup Functions
```rust
// Lookup with parent chain
scope::lookup_parent(start, name, tc_objs) -> Option<(ScopeKey, ObjKey)>

// Position-aware lookup
scope::lookup_parent_at(start, name, pos, tc_objs) -> Option<(ScopeKey, ObjKey)>
```

---

## 8. Packages

```rust
pub struct Package {
    path: String,                // Import path
    name: Option<String>,        // Package name
    scope: ScopeKey,             // Package scope
    complete: bool,              // Type checking complete
    imports: Vec<PackageKey>,    // Imported packages
    fake: bool,                  // Created for missing import
}
```

### Package Methods
```rust
pkg.path() -> &str
pkg.name() -> &Option<String>
pkg.scope() -> &ScopeKey
pkg.complete() -> bool
pkg.imports() -> &Vec<PackageKey>
```

---

## 9. Selections

Describes a selector expression `x.f`:

```rust
pub struct Selection {
    kind: SelectionKind,        // FieldVal, MethodVal, MethodExpr
    recv: Option<TypeKey>,      // Receiver type
    obj: ObjKey,                // Selected field/method
    indices: Vec<usize>,        // Path through embedded fields
    indirect: bool,             // Required pointer indirection
    typ: Option<TypeKey>,       // Type of selection
    id: String,                 // Unique id
}
```

### SelectionKind
```rust
pub enum SelectionKind {
    FieldVal,    // x.f is a struct field
    MethodVal,   // x.f is a method value
    MethodExpr,  // T.f is a method expression
}
```

### Selection Methods
```rust
sel.kind() -> &SelectionKind
sel.obj() -> ObjKey
sel.recv() -> Option<TypeKey>
sel.typ() -> TypeKey
sel.indices() -> &[usize]       // Path from receiver to field/method
sel.indirect() -> bool          // Pointer indirection needed
sel.field_index() -> Option<usize>  // Last index (field index)
```

**Important for codegen**: The `indices` path shows how to navigate through embedded fields to reach the target.

---

## 10. Operands

```rust
pub struct Operand {
    pub mode: OperandMode,
    pub expr: Option<ExprRef>,
    pub typ: Option<TypeKey>,
}
```

### OperandMode
```rust
pub enum OperandMode {
    Invalid,
    NoValue,              // No value (void func result)
    Builtin(Builtin),     // Built-in function
    TypeExpr,             // Is a type
    Constant(ConstValue), // Compile-time constant
    Variable,             // Addressable variable
    MapIndex,             // Map index (comma-ok)
    Value,                // Computed value
    CommaOk,              // Can be used in comma-ok
}
```

---

## 11. Constant Values

```rust
pub enum Value {
    Unknown,
    Bool(bool),
    Str(String),
    Int64(i64),
    IntBig(BigInt),
    Rat(BigRational),
    Float(f64),
}
```

### Constant Functions
```rust
// Accessors
constant::bool_val(v) -> bool
constant::string_val(v) -> &str
constant::int64_val(v) -> (i64, bool)     // (value, exact)
constant::uint64_val(v) -> (u64, bool)
constant::float64_val(v) -> (f64, bool)
constant::sign(v) -> i32                  // -1, 0, or 1
constant::bit_len(v) -> usize

// Conversions
constant::to_int(v) -> Value
constant::to_float(v) -> Value

// Operations
constant::unary_op(op, v, prec) -> Value
constant::binary_op(x, op, y) -> Value
constant::shift(x, op, s) -> Value
constant::compare(x, op, y) -> bool
```

---

## 12. Escape Analysis Results

Available in `TypeInfo` after analysis:

```rust
// Variables that escape to heap
pub escaped_vars: HashSet<ObjKey>

// Closure captures: FuncLit ExprId -> captured variable ObjKeys
pub closure_captures: HashMap<ExprId, Vec<ObjKey>>
```

### Escape Conditions (for reference)
A variable escapes if:
- **Address taken**: `&s` (for structs)
- **Closure capture**: Referenced in a closure
- **Interface assignment**: Assigned to interface (for struct/array)
- **Slice operation**: Array is sliced `arr[:]`
- **Pointer receiver method call**: Value type calls `*T` method
- **Large type**: > 256 slots

### Usage
```rust
// Check if a variable escapes
type_info.is_escaped(obj_key) -> bool

// Get closure captures for a FuncLit
type_info.closure_captures.get(&func_lit_expr_id) -> Option<&Vec<ObjKey>>
```

---

## 13. Type Layout Utility Functions

**Located in `type_info.rs`** - These are designed for codegen!

### Slot Count
```rust
pub fn type_slot_count(type_key: TypeKey, tc_objs: &TCObjects) -> u16
```
Returns number of slots a type occupies:
- Basic types: 1
- Pointer/Slice/Map/Chan/Signature: 1 (GcRef)
- Interface: 2
- Struct: sum of field slots (min 1)
- Array: elem_slots * len

### Slot Types
```rust
pub fn type_slot_types(type_key: TypeKey, tc_objs: &TCObjects) -> Vec<SlotType>
```
Returns slot type pattern:
- `SlotType::Value` - primitive values
- `SlotType::GcRef` - heap references
- `SlotType::Interface0`, `SlotType::Interface1` - interface slots

### Value Kind
```rust
pub fn type_value_kind(type_key: TypeKey, tc_objs: &TCObjects) -> ValueKind
```
Maps type to runtime `ValueKind`:
- `ValueKind::Int`, `Int8`, `Int16`, ..., `Float64`
- `ValueKind::String`, `Bool`
- `ValueKind::Pointer`, `Slice`, `Map`, `Channel`
- `ValueKind::Struct`, `Array`, `Interface`, `Closure`

### Element Bytes (for heap arrays)
```rust
pub fn elem_bytes_for_heap(elem_type: TypeKey, tc_objs: &TCObjects) -> usize
```
For packed types (bool, int8-32, float32): actual byte size.
For others: slot_count * 8.

### Type Predicates
```rust
pub fn is_interface(type_key, tc_objs) -> bool
pub fn is_pointer(type_key, tc_objs) -> bool
pub fn is_struct(type_key, tc_objs) -> bool
pub fn is_array(type_key, tc_objs) -> bool
pub fn is_slice(type_key, tc_objs) -> bool
pub fn is_map(type_key, tc_objs) -> bool
pub fn is_chan(type_key, tc_objs) -> bool
pub fn is_value_type(type_key, tc_objs) -> bool  // struct or array
pub fn is_named_type(type_key, tc_objs) -> bool
pub fn is_int(type_key, tc_objs) -> bool
pub fn is_float(type_key, tc_objs) -> bool
pub fn is_unsigned(type_key, tc_objs) -> bool
pub fn is_string(type_key, tc_objs) -> bool
pub fn int_bits(type_key, tc_objs) -> u8  // 8, 16, 32, 64
```

### Struct Field Access
```rust
// By name
pub fn struct_field_offset(type_key, field_name, tc_objs) -> (u16, u16)  // (offset, slots)
pub fn struct_field_type(type_key, field_name, tc_objs) -> TypeKey

// By index
pub fn struct_field_offset_by_index(type_key, field_index, tc_objs) -> (u16, u16)
pub fn struct_field_type_by_index(type_key, field_index, tc_objs) -> TypeKey

// Using selection indices (for embedded field access)
pub fn compute_field_offset_from_indices(base_type, indices, tc_objs) -> (u16, u16)
```

### Runtime Type Conversion
```rust
pub fn type_to_runtime_type(type_key, tc_objs, named_type_id_fn) -> RuntimeType
pub fn signature_to_runtime_type(sig_type, tc_objs, named_type_id_fn) -> RuntimeType
```

---

## 14. Lookup Functions

### Field/Method Lookup
```rust
pub fn lookup_field_or_method(
    tkey: TypeKey,
    addressable: bool,
    pkg: Option<PackageKey>,
    name: &str,
    tc_objs: &TCObjects,
) -> LookupResult
```

### LookupResult
```rust
pub enum LookupResult {
    Entry(ObjKey, Vec<usize>, bool),  // (obj, indices, indirect)
    Ambiguous(Vec<usize>),
    BadMethodReceiver,
    NotFound,
}
```

### Method Set

`MethodSet` computes **all methods callable on a type**, including:
- Methods declared directly on the type
- Methods from embedded fields (promoted)
- For pointers `*T`: methods with both value and pointer receivers
- For values `T`: only methods with value receivers (unless addressable)

```rust
pub struct MethodSet {
    list: Vec<Selection>,  // Sorted by method id for binary search
}

impl MethodSet {
    /// Creates method set for type T.
    /// Handles: Named types, structs with embedded fields, interfaces.
    /// Pointer types (*T) get all methods; value types get value-receiver methods only.
    pub fn new(t: &TypeKey, tc_objs: &mut TCObjects) -> MethodSet
    
    /// All methods as Selection list (sorted by id)
    pub fn list(&self) -> &[Selection]
    
    /// Binary search lookup by package and name
    pub fn lookup(&self, pkg: &PackageKey, name: &str, tc_objs: &TCObjects) -> Option<&Selection>
    
    pub fn is_empty(&self) -> bool
}
```

### MethodSet Algorithm

1. **Dereference pointer**: `*T` → `T`, mark `is_ptr = true`
2. **Named type**: Collect declared methods, then look at underlying
3. **Struct**: For each embedded field, recursively collect (with updated indices)
4. **Interface**: Use `all_methods()` (includes inherited from embedded interfaces)
5. **Collision handling**: If field and method have same name, mark as collision (excluded)
6. **Consolidation**: Merge methods at same depth, handle multiples (ambiguity)

### Selection in MethodSet

Each `Selection` contains:
- `kind`: Always `MethodVal` for method set entries
- `obj`: The method `ObjKey`
- `indices`: Path through embedded fields to reach the method's receiver type
- `indirect`: Whether pointer indirection is needed
- `typ`: Method signature type

### Usage for Codegen

```rust
// Get all methods callable on type T
let mset = MethodSet::new(&type_key, &mut tc_objs);

// Iterate all methods
for sel in mset.list() {
    let method_obj = &tc_objs.lobjs[sel.obj()];
    let method_name = method_obj.name();
    let method_sig = method_obj.typ().unwrap();
    let indices = sel.indices();  // Path for receiver access
    let indirect = sel.indirect(); // Need pointer indirection?
}

// Lookup specific method
if let Some(sel) = mset.lookup(&pkg_key, "MethodName", &tc_objs) {
    // Use selection for dispatch
}
```

### Key Points for Interface Dispatch

1. **Interface method set** = `interface.all_methods()` (from InterfaceDetail)
2. **Concrete type method set** = `MethodSet::new(&concrete_type, tc_objs)`
3. **Implementation check**: For each interface method, find matching concrete method
4. Use `missing_method()` helper for implementation verification

### Interface Implementation Check
```rust
pub fn missing_method(
    t: TypeKey,
    intf: TypeKey,
    static_: bool,
    checker: &mut Checker,
) -> Option<(ObjKey, bool)>  // (missing_method, wrong_type)
```

---

## 15. Key Type Mappings for Codegen

### Expression → Type
```rust
type_info.types.get(&expr_id) -> Option<&TypeAndValue>
type_info.expr_type(expr_id) -> Option<TypeKey>
```

### Identifier → Object
```rust
// Definition site
type_info.defs.get(&ident_id) -> Option<&Option<ObjKey>>

// Use site
type_info.uses.get(&ident_id) -> Option<&ObjKey>
```

### Selector → Selection
```rust
type_info.selections.get(&selector_expr_id) -> Option<&Selection>
```

### Type Expression → Type
```rust
type_info.type_exprs.get(&type_expr_id) -> Option<&TypeKey>
```

### AST Span → Scope
```rust
type_info.scopes.get(&span) -> Option<&ScopeKey>
```

### Closure → Captures
```rust
type_info.closure_captures.get(&func_lit_expr_id) -> Option<&Vec<ObjKey>>
```

### Variable → Escapes
```rust
type_info.escaped_vars.contains(&obj_key) -> bool
```

---

## Universe (Predefined Types/Funcs)

```rust
let univ = tc_objs.universe();

// Get predefined types
univ.types() -> &HashMap<BasicType, TypeKey>
univ.lookup_type(BasicType::Int) -> Option<TypeKey>
univ.lookup_type_by_name("int") -> Option<TypeKey>

// Special types
univ.byte() -> TypeKey
univ.rune() -> TypeKey
univ.error_type() -> TypeKey
univ.slice_of_bytes() -> TypeKey
univ.no_value_tuple() -> TypeKey  // Empty tuple

// Scope
univ.scope() -> ScopeKey
```

---

## DeclInfo (Declaration Information)

For package-level declarations:

```rust
pub enum DeclInfo {
    Const(DeclInfoConst),
    Var(DeclInfoVar),
    Type(DeclInfoType),
    Func(DeclInfoFunc),
}
```

### DeclInfoConst
```rust
pub struct DeclInfoConst {
    pub file_scope: ScopeKey,
    pub typ: Option<TypeExpr>,
    pub init: Option<Expr>,
    pub deps: HashSet<ObjKey>,
}
```

### DeclInfoVar
```rust
pub struct DeclInfoVar {
    pub file_scope: ScopeKey,
    pub lhs: Option<Vec<ObjKey>>,  // For N-to-1 assignments
    pub typ: Option<TypeExpr>,
    pub init: Option<Expr>,
    pub deps: HashSet<ObjKey>,
}
```

### DeclInfoType
```rust
pub struct DeclInfoType {
    pub file_scope: ScopeKey,
    pub typ: TypeExpr,
    pub alias: bool,
}
```

### DeclInfoFunc
```rust
pub struct DeclInfoFunc {
    pub file_scope: ScopeKey,
    pub fdecl: FuncDecl,
    pub deps: HashSet<ObjKey>,
}
```

---

## Summary: What Codegen Can Use Directly

### From `Project`
- `tc_objs`: All types, objects, scopes, packages
- `type_info`: Expression types, definitions, uses, selections
- `interner`: Symbol resolution
- `files`: AST for code generation
- `imported_files`, `imported_type_infos`: For multi-package

### From `TypeInfo`
- **Expression types**: `types[expr_id].typ`
- **Constant values**: `types[expr_id].mode` (if `Constant(val)`)
- **Definitions/Uses**: `defs`, `uses`
- **Selections**: `selections` - field/method access paths
- **Scopes**: `scopes` - for local variable lookup
- **Init order**: `init_order` - package var initialization sequence
- **Escape analysis**: `escaped_vars`, `closure_captures`

### From `type_info.rs` utility functions
- Slot counts, slot types
- Value kinds
- Struct field offsets
- Type predicates

### From `lookup.rs`
- Field/method lookup
- Method set computation
- Interface implementation checking

---

## Notes for Codegen Developers

1. **Always use `underlying_type`** when checking type structure - Named types wrap their underlying types.

2. **Selection indices** are crucial for embedded field access - they show the path through nested structs.

3. **Escape analysis results** determine stack vs heap allocation:
   - Check `type_info.is_escaped(obj)` for variables
   - Check `type_info.closure_captures` for closure upvalues

4. **Type layout functions** in `type_info.rs` are designed for codegen - use them instead of reimplementing.

5. **Init order** in `type_info.init_order` gives the correct execution order for package-level var initialization.

6. **OperandMode::Constant** contains compile-time values - can be folded at codegen time.

7. **Signature.variadic** indicates the last parameter is variadic (slice).

8. **Interface.all_methods()** includes inherited methods from embedded interfaces - use for interface dispatch.
