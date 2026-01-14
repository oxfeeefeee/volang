# Vo Hot Mode Design

Hot Mode enables hot reload capabilities for Vo programs, allowing code changes to take effect without restarting the program.

## Overview

| Mode | Binding | Reload | Use Case |
|------|---------|--------|----------|
| **Default** | Static (compile-time) | ❌ | Production |
| **Hot** | Dynamic (runtime) | ✅ | Development |

## CLI Usage

```bash
# Default mode (static, no hot reload)
vo run main.vo

# Hot mode (dynamic, supports reload)
vo run --hot main.vo
vo run -H main.vo

# Watch mode (auto-reload on file change)
vo watch main.vo
```

## Architecture

### Core Idea

**Unified bytecode + External symbol table** (like debug info):

1. Bytecode uses numeric IDs/offsets (same as production)
2. Symbol table maps IDs/offsets to names (attached when hot mode enabled)
3. At runtime, resolver translates old IDs to current IDs via names
4. On reload, migrate heap objects if struct layout changed

### Key Design Decisions

- **Single vo-vm crate** with `#[cfg(feature = "hot")]` branches
- **Single bytecode format** - no dual codegen
- **GcHeader gains type_id** - enables object migration
- **Symbol table** - similar to debug symbols in traditional compilers

### Crate Structure

```
crates/
├── vo-vm/
│   ├── Cargo.toml          # features = ["hot"]
│   └── src/
│       ├── vm/mod.rs       # #[cfg(feature = "hot")] branches
│       └── hot/            # Only compiled with "hot" feature
│           ├── mod.rs
│           ├── info.rs     # HotInfo (symbol table)
│           ├── resolver.rs # HotResolver
│           ├── types.rs    # TypeRegistry
│           └── reload.rs   # Reload + object migration
│
├── vo-runtime/
│   └── src/
│       └── gc/
│           └── header.rs   # GcHeader + type_id
│
└── vo-cli/
    └── src/commands/
        └── watch.rs        # File watcher
```

## What Can Be Hot Reloaded

| Change Type | Supported | Mechanism |
|-------------|-----------|-----------|
| Function body | ✅ | New calls use new code |
| Function add/remove | ✅ | Symbol-based lookup |
| Function signature change | ✅ | New calls use new signature |
| Global variable add/remove | ✅ | Migrate by name |
| Struct field add/remove | ✅ | Object migration |
| Struct field reorder | ✅ | Copy by field name |
| Closure hot update | ✅ | Update func_id in heap |
| Defer hot update | ✅ | Update func_id in stack |
| Interface changes | ✅ | Rebuild itab cache |

### Inherent Limitations

| Scenario | Reason |
|----------|--------|
| Mid-function code switch | PC points to old bytecode, cannot map to new |
| Incompatible type change | `int` → `string`, data cannot convert |

These are **logically impossible** - no language can do this.

## Symbol Table (HotInfo)

```rust
/// Compile-time generated, attached to Module
pub struct HotInfo {
    /// func_id -> "pkg.Type.Method"
    pub func_names: Vec<String>,
    
    /// extern_id -> "pkg.externName"  
    pub extern_names: Vec<String>,
    
    /// global_slot -> ("varName", slots)
    pub global_names: Vec<(String, u16)>,
    
    /// type_id -> TypeDef
    pub type_defs: Vec<TypeDef>,
    
    /// (func_id, pc) -> (type_id, field_name) for each PtrGet/PtrSet
    pub field_symbols: HashMap<(u32, usize), (u32, String)>,
}

pub struct TypeDef {
    pub name: String,
    pub fields: HashMap<String, FieldDef>,
    pub total_slots: u16,
}

pub struct FieldDef {
    pub offset: u16,
    pub slots: u16,
}
```

## GcHeader Extension

```rust
pub struct GcHeader {
    pub mark: u8,
    pub kind: u8,
    pub slots: u16,
    pub type_id: u32,  // NEW: identifies struct type for migration
}
```

## Runtime Resolver

```rust
pub struct HotResolver {
    /// old_func_id -> new_func_id
    func_map: Vec<u32>,
    
    /// old_type_id -> new_type_id  
    type_map: Vec<u32>,
    
    /// old_global_slot -> new_global_slot
    global_map: Vec<u32>,
    
    /// type_id -> current TypeDef
    type_layouts: HashMap<u32, TypeDef>,
    
    /// (func_id, pc) -> (type_id, field_name)
    field_symbols: HashMap<(u32, usize), (u32, String)>,
}

impl HotResolver {
    pub fn resolve_func(&self, old_id: u32) -> u32 {
        self.func_map.get(old_id as usize).copied().unwrap_or(old_id)
    }
    
    pub fn resolve_field(&self, func_id: u32, pc: usize, _old_offset: u16) -> u16 {
        let (type_id, field_name) = &self.field_symbols[&(func_id, pc)];
        self.type_layouts[type_id].fields[field_name].offset
    }
}
```

## Execution with Hot Mode

```rust
// vo-vm/src/vm/mod.rs

impl Vm {
    fn exec_call(&mut self, inst: &Instruction) {
        let raw_func_id = extract_func_id(inst);
        
        #[cfg(feature = "hot")]
        let func_id = if self.hot_enabled {
            self.resolver.resolve_func(raw_func_id)
        } else {
            raw_func_id
        };
        
        #[cfg(not(feature = "hot"))]
        let func_id = raw_func_id;
        
        let func = &self.module.functions[func_id as usize];
        // ...
    }
    
    fn exec_ptr_get(&mut self, inst: &Instruction) {
        let ptr = self.stack[self.bp + inst.b as usize] as GcRef;
        
        #[cfg(feature = "hot")]
        let offset = if self.hot_enabled {
            self.resolver.resolve_field(self.current_func_id, self.pc, inst.c)
        } else {
            inst.c
        };
        
        #[cfg(not(feature = "hot"))]
        let offset = inst.c;
        
        let val = Gc::read_slot(ptr, offset as usize);
        self.stack[self.bp + inst.a as usize] = val;
    }
}
```

## Reload Flow

```
User saves file
    ↓
Watcher detects change
    ↓
Recompile → new_module + new_hot_info
    ↓
vm.reload(new_module, new_hot_info)
```

```rust
fn reload(&mut self, new_module: Module, new_info: HotInfo) {
    let old_info = self.hot_info.take().unwrap();
    
    // Step 1: Build mapping tables
    let func_map = build_func_map(&old_info, &new_info);
    let type_map = build_type_map(&old_info, &new_info);
    let changed_types = find_layout_changes(&old_info, &new_info);
    
    // Step 2: Migrate heap objects (if layout changed)
    if !changed_types.is_empty() {
        self.migrate_objects(&old_info, &new_info, &changed_types, &type_map);
    }
    
    // Step 3: Update Closures (func_id)
    self.gc.for_each_closure(|closure| {
        let old_id = closure.func_id();
        closure.set_func_id(func_map[old_id as usize]);
    });
    
    // Step 4: Update DeferEntry (func_id)
    for fiber in self.scheduler.all_fibers_mut() {
        for entry in &mut fiber.defer_stack {
            entry.func_id = func_map[entry.func_id as usize];
        }
    }
    
    // Step 5: Migrate global variables
    self.migrate_globals(&old_info, &new_info);
    
    // Step 6: Rebuild resolver
    self.resolver.rebuild(&new_info, &func_map, &type_map);
    
    // Step 7: Clear caches
    self.itab_cache.clear();
    
    // Step 8: Replace module
    self.module = new_module;
    self.hot_info = Some(new_info);
    
    println!("[hot] Reloaded");
}
```

## Object Migration

When struct layout changes, migrate existing heap objects:

```rust
fn migrate_objects(&mut self, old_info: &HotInfo, new_info: &HotInfo, 
                   changed_types: &HashSet<u32>, type_map: &[u32]) {
    // Pass 1: Allocate new objects, copy data by field name
    self.gc.for_each_object_mut(|obj| {
        let old_type_id = obj.header.type_id;
        
        if !changed_types.contains(&old_type_id) {
            // Layout unchanged, just update type_id mapping
            obj.header.type_id = type_map[old_type_id as usize];
            return;
        }
        
        // Layout changed, need migration
        let old_layout = old_info.get_type_layout(old_type_id);
        let new_type_id = type_map[old_type_id as usize];
        let new_layout = new_info.get_type_layout(new_type_id);
        
        // Allocate new object
        let new_obj = self.gc.alloc_raw(new_layout.total_slots);
        new_obj.header.type_id = new_type_id;
        
        // Copy by field name
        for (field_name, new_field) in &new_layout.fields {
            if let Some(old_field) = old_layout.fields.get(field_name) {
                for i in 0..new_field.slots.min(old_field.slots) {
                    let val = Gc::read_slot(obj, old_field.offset + i);
                    Gc::write_slot(new_obj, new_field.offset + i, val);
                }
            }
            // New fields keep zero value
        }
        
        // Set forwarding pointer
        obj.set_forwarding(new_obj);
    });
    
    // Pass 2: Update all references (GC already has this capability)
    self.gc.update_references();
    
    // Update stack references too
    for fiber in self.scheduler.all_fibers() {
        for slot in &mut fiber.stack {
            if is_gc_ref(*slot) {
                let ptr = *slot as GcRef;
                if let Some(forwarded) = ptr.get_forwarding() {
                    *slot = forwarded as u64;
                }
            }
        }
    }
}
```

## File Changes Summary

### vo-runtime

| File | Change |
|------|--------|
| `gc/header.rs` | Add `type_id: u32` to GcHeader |
| `gc/mod.rs` | Add `for_each_object_mut()`, `update_references()` |

### vo-vm

| File | Change |
|------|--------|
| `Cargo.toml` | Add `features = ["hot"]` |
| `vm/mod.rs` | Add `#[cfg(feature = "hot")]` branches |
| `hot/mod.rs` | New module |
| `hot/info.rs` | HotInfo struct |
| `hot/resolver.rs` | HotResolver |
| `hot/types.rs` | TypeRegistry |
| `hot/reload.rs` | Reload + object migration |

### vo-codegen

| File | Change |
|------|--------|
| `context.rs` | Add `build_hot_info()` |
| `lib.rs` | Include HotInfo in compile result |

### vo-cli

| File | Change |
|------|--------|
| `commands/watch.rs` | New: file watcher + reload loop |

## JIT Compatibility

Hot Mode is compatible with JIT. On reload, all JIT-compiled code is invalidated.

### Problem

JIT compiles bytecode to native code with hardcoded IDs/offsets:

```asm
mov rax, [module + 0x100]    ; func_id baked in
mov rbx, [rax + 16]          ; field offset baked in
```

These cannot be dynamically resolved like VM bytecode.

### Solution: Invalidate on Reload

```rust
fn reload(&mut self, new_module: Module, new_info: HotInfo) {
    // ... other steps ...
    
    // Invalidate all JIT code
    #[cfg(feature = "jit")]
    if let Some(jit) = &mut self.jit_mgr {
        jit.invalidate_all();
    }
    
    // ... replace module ...
}
```

```rust
// vo-jit/src/manager.rs

impl JitManager {
    pub fn invalidate_all(&mut self) {
        self.compiled_funcs.clear();
        self.call_counts.clear();
        self.code_memory.reset();
    }
}
```

### Behavior

1. After reload, all functions run in VM interpreter
2. Hot functions re-trigger JIT compilation (with new code)
3. JIT state gradually recovers

### Overhead

- Brief performance dip after reload
- Hot functions recompile (milliseconds)
- Acceptable for development mode

## Workload Estimate

| Task | Lines | Days |
|------|-------|------|
| GcHeader + type_id | ~50 | 0.5 |
| vo-codegen HotInfo generation | ~300 | 2 |
| HotResolver | ~200 | 1 |
| vo-vm #[cfg] branches | ~150 | 1 |
| TypeRegistry | ~150 | 1 |
| Object migration | ~200 | 1.5 |
| Closure/Defer update | ~100 | 0.5 |
| Global migration | ~80 | 0.5 |
| vo-cli watch command | ~100 | 0.5 |
| Testing | ~300 | 1.5 |
| **Total** | **~1600** | **~10 days** |

## Comparison with Other Languages

| Language | Function Body | Signature Change | Struct Layout |
|----------|---------------|------------------|---------------|
| Python | ✅ | ❌ old objects unchanged | ❌ |
| Node.js | ✅ | ❌ old objects unchanged | ❌ |
| Erlang | ✅ | ✅ requires migration code | ❌ |
| Flutter | ✅ | ⚠️ limited | ❌ |
| **Vo Hot** | ✅ | ✅ | ✅ |

## Summary

This design achieves **maximum functionality** for hot reload:

- ✅ Function body, signature, add/remove
- ✅ Global variables
- ✅ Struct field add/remove/reorder
- ✅ Closure and defer
- ✅ Interface changes

With **practical engineering**:

- Single bytecode format
- Single vo-vm crate with feature flag
- ~1600 lines of new code
- ~10 days of work

The only limitations are **logically impossible** scenarios (mid-execution code switch, incompatible type conversion).
