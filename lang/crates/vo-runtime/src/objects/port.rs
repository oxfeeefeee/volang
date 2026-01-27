//! Port object operations for cross-island communication.
//!
//! Layout: GcHeader + PortData
//!
//! Port is similar to Channel but with:
//! 1. Thread-safe inner state (Arc<Mutex<>>)
//! 2. Pack before send, unpack after recv (values are deep-copied)
//! 3. Cross-island fiber wake mechanism

#[cfg(not(feature = "std"))]
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

#[cfg(feature = "std")]
use std::{collections::VecDeque, sync::Arc, vec::Vec};

use crate::gc::{Gc, GcRef};
use crate::pack::PackedValue;
use crate::slot::{ptr_to_slot, slot_to_ptr, slot_to_usize, Slot, SLOT_BYTES};
use vo_common_core::types::{ValueKind, ValueMeta};

// Use parking_lot for std, spin for no_std (if available)
// For now, only support std mode for Port (requires thread-safe Mutex)
#[cfg(feature = "std")]
use std::sync::Mutex;

#[repr(C)]
pub struct PortData {
    pub state: Slot,
    pub cap: Slot,
    pub elem_meta: ValueMeta,
    pub elem_slots: u16,
    _pad: u16,
}

pub const DATA_SLOTS: u16 = 3;
const _: () = assert!(core::mem::size_of::<PortData>() == DATA_SLOTS as usize * SLOT_BYTES);

impl_gc_object!(PortData);

/// Information about a waiting fiber for cross-island wake
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiterInfo {
    pub island_id: u32,
    pub fiber_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendResult {
    /// Value sent to waiting receiver (via buffer, receiver will pop it)
    DirectSend(WaiterInfo),
    /// Value buffered
    Buffered,
    /// Would block (buffer full, no receivers) - returns the value back
    WouldBlock(PackedValue),
    /// Port is closed
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecvResult {
    /// Successfully received (optionally woke a sender)
    Success(Option<WaiterInfo>),
    /// Would block (buffer empty, no senders)
    WouldBlock,
    /// Port is closed
    Closed,
}

/// Thread-safe inner state for Port
pub struct PortState {
    /// Buffered packed values
    pub buffer: VecDeque<PackedValue>,
    /// Port closed flag
    pub closed: bool,
    /// Waiting senders: (waiter_info, packed_value)
    pub waiting_senders: VecDeque<(WaiterInfo, PackedValue)>,
    /// Waiting receivers
    pub waiting_receivers: VecDeque<WaiterInfo>,
}

impl PortState {
    pub fn new(cap: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(cap),
            closed: false,
            waiting_senders: VecDeque::new(),
            waiting_receivers: VecDeque::new(),
        }
    }

    pub fn try_send(&mut self, value: PackedValue, cap: usize) -> SendResult {
        if self.closed {
            return SendResult::Closed;
        }
        // If there's a waiting receiver, buffer the value and wake receiver
        // (receiver will pop from buffer when it runs)
        if let Some(receiver) = self.waiting_receivers.pop_front() {
            self.buffer.push_back(value);
            return SendResult::DirectSend(receiver);
        }
        // Buffer if capacity allows
        if self.buffer.len() < cap {
            self.buffer.push_back(value);
            return SendResult::Buffered;
        }
        SendResult::WouldBlock(value)
    }

    pub fn try_recv(&mut self) -> (RecvResult, Option<PackedValue>) {
        if let Some(value) = self.buffer.pop_front() {
            let woke_sender = if let Some((sender, sender_value)) = self.waiting_senders.pop_front() {
                self.buffer.push_back(sender_value);
                Some(sender)
            } else {
                None
            };
            return (RecvResult::Success(woke_sender), Some(value));
        }
        if let Some((sender, value)) = self.waiting_senders.pop_front() {
            return (RecvResult::Success(Some(sender)), Some(value));
        }
        if self.closed {
            (RecvResult::Closed, None)
        } else {
            (RecvResult::WouldBlock, None)
        }
    }

    pub fn register_sender(&mut self, waiter: WaiterInfo, value: PackedValue) {
        self.waiting_senders.push_back((waiter, value));
    }

    pub fn register_receiver(&mut self, waiter: WaiterInfo) {
        self.waiting_receivers.push_back(waiter);
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn take_waiting_receivers(&mut self) -> Vec<WaiterInfo> {
        self.waiting_receivers.drain(..).collect()
    }

    pub fn take_waiting_senders(&mut self) -> Vec<(WaiterInfo, PackedValue)> {
        self.waiting_senders.drain(..).collect()
    }
}

/// Create a new port.
#[cfg(feature = "std")]
pub fn create(gc: &mut Gc, elem_meta: ValueMeta, elem_slots: u16, cap: usize) -> GcRef {
    let port = gc.alloc(ValueMeta::new(0, ValueKind::Port), DATA_SLOTS);
    let state = Arc::new(Mutex::new(PortState::new(cap)));
    let data = PortData::as_mut(port);
    data.state = ptr_to_slot(Arc::into_raw(state) as *mut u8);
    data.cap = cap as Slot;
    data.elem_meta = elem_meta;
    data.elem_slots = elem_slots;
    port
}

#[cfg(not(feature = "std"))]
pub fn create(_gc: &mut Gc, _elem_meta: ValueMeta, _elem_slots: u16, _cap: usize) -> GcRef {
    panic!("Port not supported in no_std mode")
}

/// Clone a port for cross-island transfer.
/// Creates a new GcRef pointing to a new PortData, but sharing the same Arc<Mutex<PortState>>.
#[cfg(feature = "std")]
pub fn clone_for_island(gc: &mut Gc, src_port: GcRef) -> GcRef {
    let src = PortData::as_ref(src_port);
    
    // Increment Arc refcount by cloning the Arc
    let arc_ptr = slot_to_ptr::<Mutex<PortState>>(src.state);
    let arc: Arc<Mutex<PortState>> = unsafe { Arc::from_raw(arc_ptr) };
    let arc_clone = Arc::clone(&arc);
    // Don't drop the original Arc - convert back to raw
    let _ = Arc::into_raw(arc);
    
    // Create new PortData on the target island's GC
    let port = gc.alloc(ValueMeta::new(0, ValueKind::Port), DATA_SLOTS);
    let data = PortData::as_mut(port);
    data.state = ptr_to_slot(Arc::into_raw(arc_clone) as *mut u8);
    data.cap = src.cap;
    data.elem_meta = src.elem_meta;
    data.elem_slots = src.elem_slots;
    port
}

/// Get the raw state pointer for cross-island transfer.
#[cfg(feature = "std")]
pub fn get_state_ptr(port: GcRef) -> u64 {
    PortData::as_ref(port).state
}

/// Get port metadata for cross-island transfer.
#[cfg(feature = "std")]
pub fn get_metadata(port: GcRef) -> (u64, ValueMeta, u16) {
    let data = PortData::as_ref(port);
    (data.cap, data.elem_meta, data.elem_slots)
}

/// Create a port from raw state pointer (for cross-island transfer).
#[cfg(feature = "std")]
pub fn create_from_raw(gc: &mut Gc, state_ptr: u64, cap: u64, elem_meta: ValueMeta, elem_slots: u16) -> GcRef {
    // Increment Arc refcount
    let arc_ptr = slot_to_ptr::<Mutex<PortState>>(state_ptr);
    let arc: Arc<Mutex<PortState>> = unsafe { Arc::from_raw(arc_ptr) };
    let arc_clone = Arc::clone(&arc);
    let _ = Arc::into_raw(arc); // Don't drop original
    
    let port = gc.alloc(ValueMeta::new(0, ValueKind::Port), DATA_SLOTS);
    let data = PortData::as_mut(port);
    data.state = ptr_to_slot(Arc::into_raw(arc_clone) as *mut u8);
    data.cap = cap;
    data.elem_meta = elem_meta;
    data.elem_slots = elem_slots;
    port
}

#[inline]
pub fn elem_meta(port: GcRef) -> ValueMeta {
    PortData::as_ref(port).elem_meta
}

#[inline]
pub fn elem_kind(port: GcRef) -> ValueKind {
    elem_meta(port).value_kind()
}

#[inline]
pub fn elem_slots(port: GcRef) -> u16 {
    PortData::as_ref(port).elem_slots
}

#[inline]
pub fn capacity(port: GcRef) -> usize {
    slot_to_usize(PortData::as_ref(port).cap)
}

/// Access port state via closure. Guard lifetime is bounded to the closure.
#[cfg(feature = "std")]
#[inline]
fn with_state<T, F: FnOnce(&mut PortState) -> T>(port: GcRef, f: F) -> T {
    let arc_ptr = slot_to_ptr::<Mutex<PortState>>(PortData::as_ref(port).state);
    let mut guard = unsafe { (*arc_ptr).lock().unwrap() };
    f(&mut guard)
}

#[cfg(feature = "std")]
pub fn len(port: GcRef) -> usize {
    with_state(port, |s| s.len())
}

#[cfg(feature = "std")]
pub fn is_closed(port: GcRef) -> bool {
    with_state(port, |s| s.is_closed())
}

#[cfg(feature = "std")]
pub fn close(port: GcRef) {
    with_state(port, |s| s.close());
}

/// Try to send a packed value through the port.
#[cfg(feature = "std")]
pub fn try_send(port: GcRef, value: PackedValue) -> SendResult {
    let cap = capacity(port);
    with_state(port, |s| s.try_send(value, cap))
}

/// Try to receive a packed value from the port.
#[cfg(feature = "std")]
pub fn try_recv(port: GcRef) -> (RecvResult, Option<PackedValue>) {
    with_state(port, |s| s.try_recv())
}

/// Register a sender to wait.
#[cfg(feature = "std")]
pub fn register_sender(port: GcRef, waiter: WaiterInfo, value: PackedValue) {
    with_state(port, |s| s.register_sender(waiter, value));
}

/// Register a receiver to wait.
#[cfg(feature = "std")]
pub fn register_receiver(port: GcRef, waiter: WaiterInfo) {
    with_state(port, |s| s.register_receiver(waiter));
}

/// Take all waiting receivers (for close notification).
#[cfg(feature = "std")]
pub fn take_waiting_receivers(port: GcRef) -> Vec<WaiterInfo> {
    with_state(port, |s| s.take_waiting_receivers())
}

/// Take all waiting senders (for close notification).
#[cfg(feature = "std")]
pub fn take_waiting_senders(port: GcRef) -> Vec<(WaiterInfo, PackedValue)> {
    with_state(port, |s| s.take_waiting_senders())
}

/// # Safety
/// port must be a valid Port GcRef.
#[cfg(feature = "std")]
pub unsafe fn drop_inner(port: GcRef) {
    let data = PortData::as_mut(port);
    if data.state != 0 {
        let arc_ptr = slot_to_ptr::<Mutex<PortState>>(data.state) as *const Mutex<PortState>;
        drop(Arc::from_raw(arc_ptr));
        data.state = 0;
    }
}

#[cfg(not(feature = "std"))]
pub unsafe fn drop_inner(_port: GcRef) {
    // No-op in no_std - ports not supported
}
