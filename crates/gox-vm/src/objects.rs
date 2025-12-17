//! Heap object operations: String, Array, Slice, Map, Channel, Closure.
//!
//! Most operations are re-exported from gox-runtime-core.
//! Only Channel remains here as it depends on VM-specific FiberId.

use alloc::{boxed::Box, collections::VecDeque};
use crate::gc::{Gc, GcRef};
use crate::types::TypeId;

// Re-export all object operations from runtime-core
pub use gox_runtime_core::objects::{string, array, slice, map, closure, interface, struct_hash};

// =============================================================================
// Channel - VM-specific (depends on FiberId for goroutine scheduling)
// =============================================================================

/// Channel object layout (after GcHeader):
/// - slot 0: Box pointer to channel state
/// - slot 1: elem_type
/// - slot 2: capacity
pub mod channel {
    use super::*;
    use crate::fiber::FiberId;
    
    const CHAN_PTR_SLOT: usize = 0;
    const ELEM_TYPE_SLOT: usize = 1;
    const CAP_SLOT: usize = 2;
    pub const SIZE_SLOTS: usize = 3;
    
    #[derive(Default)]
    pub struct ChannelState {
        pub buffer: VecDeque<u64>,
        pub closed: bool,
        pub waiting_senders: VecDeque<(FiberId, u64)>,
        pub waiting_receivers: VecDeque<FiberId>,
    }
    
    pub fn create(gc: &mut Gc, type_id: TypeId, elem_type: TypeId, capacity: usize) -> GcRef {
        let chan = gc.alloc(type_id, SIZE_SLOTS);
        let state = Box::new(ChannelState {
            buffer: VecDeque::with_capacity(capacity),
            closed: false,
            waiting_senders: VecDeque::new(),
            waiting_receivers: VecDeque::new(),
        });
        Gc::write_slot(chan, CHAN_PTR_SLOT, Box::into_raw(state) as u64);
        Gc::write_slot(chan, ELEM_TYPE_SLOT, elem_type as u64);
        Gc::write_slot(chan, CAP_SLOT, capacity as u64);
        chan
    }
    
    pub fn get_state(chan: GcRef) -> &'static mut ChannelState {
        let ptr = Gc::read_slot(chan, CHAN_PTR_SLOT) as *mut ChannelState;
        unsafe { &mut *ptr }
    }
    
    pub fn elem_type(chan: GcRef) -> TypeId {
        Gc::read_slot(chan, ELEM_TYPE_SLOT) as TypeId
    }
    
    pub fn capacity(chan: GcRef) -> usize {
        Gc::read_slot(chan, CAP_SLOT) as usize
    }
    
    pub fn len(chan: GcRef) -> usize {
        get_state(chan).buffer.len()
    }
    
    pub fn is_closed(chan: GcRef) -> bool {
        get_state(chan).closed
    }
    
    pub fn close(chan: GcRef) {
        get_state(chan).closed = true;
    }
    
    pub fn try_send(chan: GcRef, val: u64) -> Result<Option<FiberId>, u64> {
        let state = get_state(chan);
        let cap = capacity(chan);
        
        if state.closed {
            panic!("send on closed channel");
        }
        
        if let Some(receiver_id) = state.waiting_receivers.pop_front() {
            state.buffer.push_back(val);
            return Ok(Some(receiver_id));
        }
        
        if state.buffer.len() < cap {
            state.buffer.push_back(val);
            return Ok(None);
        }
        
        Err(val)
    }
    
    pub fn try_recv(chan: GcRef) -> Result<Option<u64>, ()> {
        let state = get_state(chan);
        
        if let Some(val) = state.buffer.pop_front() {
            if let Some((_, sender_val)) = state.waiting_senders.pop_front() {
                state.buffer.push_back(sender_val);
            }
            return Ok(Some(val));
        }
        
        if let Some((_, val)) = state.waiting_senders.pop_front() {
            return Ok(Some(val));
        }
        
        if state.closed {
            return Ok(None);
        }
        
        Err(())
    }
    
    pub unsafe fn drop_inner(chan: GcRef) {
        let ptr = Gc::read_slot(chan, CHAN_PTR_SLOT) as *mut ChannelState;
        if !ptr.is_null() {
            drop(Box::from_raw(ptr));
            Gc::write_slot(chan, CHAN_PTR_SLOT, 0);
        }
    }
}
