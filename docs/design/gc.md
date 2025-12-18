# GoX GC Design

This document describes the garbage collector design with a phased implementation approach.

## 1. Overview

- **Final goal**: Lua 5.4 style incremental + generational GC
- **Initial version**: Simple stop-the-world mark-sweep
- **Strategy**: Data structures support final version from day one, algorithm evolves

## 2. Data Structures

### 2.1 GC Color (Tri-color Marking)

```rust
#[repr(u8)]
pub enum GcColor {
    White = 0,   // Not visited (potentially garbage)
    Gray = 1,    // Visited, children not yet scanned
    Black = 2,   // Visited, children scanned
}
```

### 2.2 GC Generation

```rust
#[repr(u8)]
pub enum GcGen {
    Young = 0,   // Newly allocated
    Old = 1,     // Long-lived
    Touched = 2, // Old but modified (needs rescan)
}
```

### 2.3 Object Header

```rust
#[repr(C)]
pub struct GcHeader {
    pub mark: u8,       // GcColor
    pub gen: u8,        // GcGen (for generational GC)
    pub flags: u8,      // Reserved flags
    pub _pad: u8,
    pub type_id: u32,   // Type ID → TypeMeta
}
// Total: 8 bytes
```

### 2.4 GC State Machine

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum GcState {
    Pause,      // Idle
    Propagate,  // Marking gray objects
    Atomic,     // Final marking (short pause)
    Sweep,      // Freeing white objects
}
```

```
State transitions (final version):
Pause → Propagate → Atomic → Sweep → Pause
         ↑____incremental____↑
```

### 2.5 GC Main Structure

```rust
pub struct Gc {
    // === Object tracking ===
    all_objects: Vec<*mut GcObject>,
    
    // === Incremental marking ===
    gray_queue: Vec<*mut GcObject>,
    state: GcState,
    current_white: u8,  // Flip between 0 and 1 for white marking
    
    // === Generational (reserved) ===
    young_list: Vec<*mut GcObject>,
    old_list: Vec<*mut GcObject>,
    
    // === Statistics ===
    total_bytes: usize,
    threshold: usize,
    
    // === Tuning parameters (Lua style) ===
    pause: usize,       // % of memory growth before next GC (default: 200)
    stepmul: usize,     // Work multiplier per step (default: 200)
    
    // === Pause control ===
    pause_count: u32,   // >0 means GC is paused (during extern calls)
}
```

## 3. Implementation Phases

### Phase 1: Stop-the-World (Initial)

Simple complete collection:

```rust
impl Gc {
    pub fn collect(&mut self) {
        if self.pause_count > 0 {
            return;  // Paused during extern call
        }
        
        // 1. Mark phase
        self.mark_roots();
        self.mark_propagate_all();  // Process all grays at once
        
        // 2. Sweep phase
        self.sweep();
        
        // 3. Adjust threshold
        self.threshold = self.total_bytes * self.pause / 100;
    }
    
    fn mark_propagate_all(&mut self) {
        while let Some(obj) = self.gray_queue.pop() {
            self.scan_object(obj);
            unsafe { (*obj).header.mark = GcColor::Black as u8; }
        }
    }
    
    fn sweep(&mut self) {
        let white = self.current_white;
        self.all_objects.retain(|&obj| {
            unsafe {
                if (*obj).header.mark == white {
                    self.total_bytes -= object_size(obj);
                    deallocate(obj);
                    false
                } else {
                    (*obj).header.mark = white;  // Reset for next GC
                    true
                }
            }
        });
    }
}
```

### Phase 2: Incremental Marking

Process gray queue in steps:

```rust
impl Gc {
    /// Do one step of incremental GC
    pub fn step(&mut self) {
        if self.pause_count > 0 {
            return;
        }
        
        match self.state {
            GcState::Pause => {
                if self.total_bytes > self.threshold {
                    self.mark_roots();
                    self.state = GcState::Propagate;
                }
            }
            GcState::Propagate => {
                // Process N objects per step
                let work = self.stepmul;
                for _ in 0..work {
                    if let Some(obj) = self.gray_queue.pop() {
                        self.scan_object(obj);
                        unsafe { (*obj).header.mark = GcColor::Black as u8; }
                    } else {
                        self.state = GcState::Atomic;
                        break;
                    }
                }
            }
            GcState::Atomic => {
                // Short pause: final marking
                self.mark_roots();  // Re-mark roots
                while let Some(obj) = self.gray_queue.pop() {
                    self.scan_object(obj);
                    unsafe { (*obj).header.mark = GcColor::Black as u8; }
                }
                self.state = GcState::Sweep;
            }
            GcState::Sweep => {
                self.sweep();
                self.state = GcState::Pause;
                self.threshold = self.total_bytes * self.pause / 100;
            }
        }
    }
}
```

### Phase 3: Write Barriers

Required for incremental correctness:

```rust
impl Gc {
    /// Forward barrier: when writing a reference into an object
    #[inline]
    pub fn write_barrier(&mut self, parent: *mut GcObject, child: *mut GcObject) {
        if self.state != GcState::Propagate {
            return;
        }
        
        unsafe {
            // If parent is black and child is white, mark child gray
            if (*parent).header.mark == GcColor::Black as u8
               && (*child).header.mark == self.current_white
            {
                (*child).header.mark = GcColor::Gray as u8;
                self.gray_queue.push(child);
            }
        }
    }
}

// Called by VM when setting object fields
fn set_field(gc: &mut Gc, obj: GcRef, idx: usize, val: u64, is_ptr: bool) {
    unsafe {
        (*obj).data[idx] = val;
        if is_ptr && !val.is_null() {
            gc.write_barrier(obj, val as *mut GcObject);
        }
    }
}
```

### Phase 4: Generational Collection

Separate young/old generations:

```rust
impl Gc {
    pub fn minor_collect(&mut self) {
        // Only collect young generation
        // Promote survivors to old
    }
    
    pub fn major_collect(&mut self) {
        // Collect all generations
    }
    
    /// Back barrier: when old object references young object
    pub fn write_barrier_back(&mut self, parent: *mut GcObject, child: *mut GcObject) {
        unsafe {
            if (*parent).header.gen == GcGen::Old as u8
               && (*child).header.gen == GcGen::Young as u8
            {
                (*parent).header.gen = GcGen::Touched as u8;
                // Add to remembered set
            }
        }
    }
}
```

## 4. Root Set

```rust
pub trait GcRoots {
    fn mark_roots(&self, gc: &mut Gc);
}

impl GcRoots for Vm {
    fn mark_roots(&self, gc: &mut Gc) {
        // All Fiber stacks
        for fiber in &self.fibers {
            // Value stack
            for slot in &fiber.stack {
                if is_gc_ref(*slot) {
                    gc.mark_gray(*slot as *mut GcObject);
                }
            }
            
            // Iterator stack (container refs)
            for iter in &fiber.iter_stack {
                gc.mark_gray(iter.container_ref());
            }
            
            // Defer stack (captured args)
            for defer in &fiber.defer_stack {
                for arg in &defer.args[..defer.arg_count as usize] {
                    if is_gc_ref(*arg) {
                        gc.mark_gray(*arg as *mut GcObject);
                    }
                }
            }
        }
        
        // Global variables
        for slot in &self.globals {
            if is_gc_ref(*slot) {
                gc.mark_gray(*slot as *mut GcObject);
            }
        }
    }
}
```

## 5. Object Scanning

```rust
impl Gc {
    fn scan_object(&mut self, obj: *mut GcObject) {
        let type_id = unsafe { (*obj).header.type_id };
        let meta = get_type_meta(type_id);
        
        match meta.kind {
            TypeKind::Interface => {
                // Special: check type_id to determine if data is pointer
                let actual_type = unsafe { (*obj).data[0] };
                let data = unsafe { (*obj).data[1] };
                if get_type_meta(actual_type as u32).is_reference_type() {
                    self.mark_gray(data as *mut GcObject);
                }
            }
            _ => {
                // Normal: scan by ptr_bitmap
                for (i, is_ptr) in meta.ptr_bitmap.iter().enumerate() {
                    if *is_ptr {
                        let child = unsafe { (*obj).data[i] };
                        if child != 0 {
                            self.mark_gray(child as *mut GcObject);
                        }
                    }
                }
            }
        }
    }
    
    fn mark_gray(&mut self, obj: *mut GcObject) {
        if obj.is_null() {
            return;
        }
        unsafe {
            if (*obj).header.mark == self.current_white {
                (*obj).header.mark = GcColor::Gray as u8;
                self.gray_queue.push(obj);
            }
        }
    }
}
```

## 6. API

```rust
impl Gc {
    /// Allocate a new object
    pub fn alloc(&mut self, type_id: TypeId, size_slots: usize) -> GcRef;
    
    /// Pause GC (call before extern function)
    pub fn pause(&mut self);
    
    /// Resume GC (call after extern function)
    pub fn resume(&mut self);
    
    /// Force full collection
    pub fn collect(&mut self);
    
    /// Do incremental step (Phase 2+)
    pub fn step(&mut self);
    
    /// Write barrier (Phase 3+)
    pub fn write_barrier(&mut self, parent: GcRef, child: GcRef);
}
```

## 7. Summary

| Phase | Algorithm | Features |
|-------|-----------|----------|
| 1 | Stop-the-world | Simple, complete |
| 2 | Incremental mark | Reduced pause time |
| 3 | + Write barriers | Correct incremental |
| 4 | + Generational | Fast minor collections |

**Data structures ready for Phase 4 from day one.**
