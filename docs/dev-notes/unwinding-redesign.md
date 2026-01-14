# Unwinding System Redesign

## Problem Analysis

The current panic/recover implementation has accumulated multiple bugs because:

1. **Scattered State**: `panic_state` and `unwinding` are managed independently
2. **Confusing Entry Points**: `exec_return` and `exec_panic_unwind` have interleaved logic
3. **Duplicated Code**: `continue_panic_unwind`, `handle_panic_inside_defer`, `start_panic_unwind` share logic
4. **Complex Conditions**: Frame depth checks scattered across functions

## First Principles: Go Semantics

### Key Rules
1. `panic(v)` - Start unwinding, save value v
2. `recover()` - Only works when called **directly** from defer, returns panic value
3. `defer` - Execute in LIFO order, even during panic
4. Panic in defer - New panic replaces old, continue executing remaining defers

### State Machine

```
                    ┌─────────────────────────────────────┐
                    │           NORMAL                     │
                    │      (no unwinding)                  │
                    └──────────┬──────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │ return         │                │ panic
              ▼                │                ▼
┌─────────────────────────┐    │    ┌─────────────────────────┐
│      RETURN MODE        │    │    │      PANIC MODE         │
│   (executing defers)    │    │    │   (executing defers)    │
├─────────────────────────┤    │    ├─────────────────────────┤
│ • Execute defers LIFO   │    │    │ • Execute defers LIFO   │
│ • On completion: return │    │    │ • Check recover() after │
│ • On panic: → PANIC     │    │    │   each defer returns    │
└─────────────────────────┘    │    │ • On recover: → RETURN  │
                               │    │ • On panic: replace old │
                               │    │ • No defers: unwind up  │
                               │    └─────────────────────────┘
```

## New Design

### Core Principle: Single Entry Point

All unwinding events flow through ONE function that implements the state machine.

### Events That Trigger Unwinding Logic

1. **ReturnInstr** - Return instruction executed
2. **PanicInstr** - Panic instruction executed  
3. **DeferReturned** - A defer function returned (via Return instruction at defer boundary)

### Unified State

```rust
/// Fiber's unwinding state (replaces both unwinding and panic_state)
pub struct UnwindState {
    /// Current mode
    pub mode: UnwindMode,
    /// Pending defers (LIFO, first = next to execute)
    pub pending: Vec<DeferEntry>,
    /// Frame depth after the original function was popped
    pub target_depth: usize,
    /// Return value info (for both modes)
    pub return_info: ReturnInfo,
}

pub enum UnwindMode {
    /// Normal return - executing defers before returning
    Return,
    /// Panic - executing defers, checking for recover
    Panic { value: (u64, u64) },  // interface{} (slot0, slot1)
}

pub struct ReturnInfo {
    pub kind: ReturnValueKind,
    pub caller_ret_reg: u16,
    pub caller_ret_count: usize,
}
```

### Key Insight: Defer Boundary Detection

A defer returns when: `frames.len() == unwind_state.target_depth + 1` AND executing Return instruction.

This is the ONLY place where we need to "continue" unwinding logic.

### Simplified Flow

```rust
// In VM loop for Return instruction:
if let Some(state) = &fiber.unwind_state {
    if frames.len() == state.target_depth + 1 {
        // Defer just returned - handle based on mode
        return handle_defer_returned(fiber, stack, module);
    }
}
// Otherwise: normal return (may start unwinding if has defers)
exec_return(...)

// In VM loop for Panic instruction:
// Always goes to handle_panic()
handle_panic(fiber, stack, module, panic_value)
```

### handle_defer_returned()

```rust
fn handle_defer_returned(fiber, stack, module) -> ExecResult {
    let state = fiber.unwind_state.as_mut().unwrap();
    
    // Collect any defers from the defer function itself
    collect_nested_defers(&mut state.pending, ...);
    pop_frame(stack, frames);
    
    match &mut state.mode {
        UnwindMode::Return => {
            // Just execute next defer or complete return
            execute_next_or_complete_return(state, ...)
        }
        UnwindMode::Panic { value } => {
            // Check if recover() was called (panic_value is None)
            if fiber.panic_value.is_none() {
                // Recovered! Switch to Return mode
                state.mode = UnwindMode::Return;
                execute_next_or_complete_return(state, ...)
            } else {
                // Still panicking
                execute_next_or_unwind_parent(state, ...)
            }
        }
    }
}
```

### handle_panic()

```rust
fn handle_panic(fiber, stack, module, value) -> ExecResult {
    fiber.panic_value = Some(value);
    
    if let Some(state) = &mut fiber.unwind_state {
        // Panic during unwinding - unwind to defer boundary and continue
        unwind_to_boundary(state, stack, frames);
        pop_defer_frame(state, stack, frames);
        state.mode = UnwindMode::Panic { value };
        execute_next_or_unwind_parent(state, ...)
    } else {
        // Fresh panic - start unwinding
        start_panic_unwind(fiber, stack, module)
    }
}
```

## Benefits of New Design

1. **Single Source of Truth**: All state in one place
2. **Clear Entry Points**: Each event type has one handler
3. **No Duplicated Logic**: Common operations factored out
4. **Explicit State Machine**: Easy to reason about transitions
5. **Testable**: Can unit test each transition independently
