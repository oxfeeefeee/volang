# RuntimeTypeId Design

## 1. ID Range Allocation

```
┌─────────────────────────────────────────────────────┐
│ 0-13      │ Basic value types (Nil, Bool, Int...)   │
│ 14-22     │ Built-in ref types (String, Slice...)   │
│ 100+      │ User-defined structs (FirstStruct)      │
│ 2^31+     │ User-defined interfaces (FirstInterface)│
└─────────────────────────────────────────────────────┘
```

## 2. Struct Type ID Allocation

- **Each struct definition** gets a unique ID (no deduplication)
- Named struct (`type MyStruct struct{...}`) → `FirstStruct + idx`
- Anonymous struct (`struct{x int}`) → `FirstStruct + idx`
- Each definition registers a `TypeMeta` (including `slot_types` for GC scanning)

## 3. Interface Type ID Allocation

- **Each interface definition** gets a unique ID (no deduplication)
- Named interface (`type MyInterface interface{...}`) → `FirstInterface + idx`
- Anonymous interface → `FirstInterface + idx`

## 4. Codegen Flow

```
Pass 1: Register Types (in CodegenContext)
├── Traverse all types via query.iter_types()
├── Type::Struct → ctx.register_struct_type(type_key)
├── Type::Interface → ctx.register_interface_type(type_key)
├── Type::Named wrapping struct/interface → register with named type's key
└── Store in struct_type_ids / interface_type_ids HashMaps

Pass 2: Generate Code
├── ctx.runtime_type_id(ty, type_key) returns registered ID
├── Opcode::Alloc uses the allocated type_id
└── Interface variable slot[0] stores the actual value's type ID
```

## 5. Implementation Location

**CodegenContext** (`gox-codegen-vm/src/context.rs`):
```rust
pub struct CodegenContext {
    // ...
    struct_type_ids: HashMap<TypeKey, u32>,
    interface_type_ids: HashMap<TypeKey, u32>,
    next_struct_id: u32,      // starts at FirstStruct (100)
    next_interface_id: u32,   // starts at FirstInterface (2^31)
}

impl CodegenContext {
    pub fn register_struct_type(&mut self, type_key: TypeKey) -> u32;
    pub fn register_interface_type(&mut self, type_key: TypeKey) -> u32;
    pub fn runtime_type_id(&self, ty: &Type, type_key: Option<TypeKey>) -> u32;
}
```

## 6. Data Structures

**GlobalDef**:
```rust
pub struct GlobalDef {
    pub name: String,
    pub slots: u16,
    pub type_id: u32,  // RuntimeTypeId value
}
```

**TypeMeta**:
```rust
pub struct TypeMeta {
    pub type_id: u32,
    pub size_slots: usize,
    pub slot_types: Vec<SlotType>,  // For GC scanning
    // ...
}
```

## 7. GC Scanning

**Global Variables**:
- Interface → 2 slots, dynamically check slot[1]
- Other GC types → mark slot[0]

**Heap Objects**:
- Use object header's `type_id` to lookup `slot_types`
- Precisely scan fields according to `slot_types`
