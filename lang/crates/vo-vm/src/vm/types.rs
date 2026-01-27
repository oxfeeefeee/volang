//! VM types and state definitions.

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use vo_runtime::gc::{Gc, GcRef};
use vo_runtime::SentinelErrorCache;

use crate::exec::ExternRegistry;
use vo_runtime::itab::ItabCache;

#[cfg(feature = "std")]
use std::sync::mpsc::{Sender, Receiver};
#[cfg(feature = "std")]
use std::thread::JoinHandle;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use vo_runtime::island::IslandCommand;

/// Shared registry of island command senders.
#[cfg(feature = "std")]
pub type IslandRegistry = Arc<Mutex<HashMap<u32, Sender<IslandCommand>>>>;

/// Time slice: number of instructions before forced yield check.
pub const TIME_SLICE: u32 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecResult {
    Continue,
    Return,
    Yield,
    Block,  // Channel blocking - don't re-queue, wait for wake
    Panic,
    Done,
    /// OSR request: (func_id, backedge_pc, loop_header_pc)
    Osr(u32, usize, usize),
}

/// Runtime error location for debug info lookup.
#[derive(Debug, Clone, Copy)]
pub struct ErrorLocation {
    pub func_id: u32,
    pub pc: u32,
}

#[derive(Debug)]
pub enum VmError {
    NoEntryFunction,
    InvalidFunctionId(u32),
    StackOverflow,
    StackUnderflow,
    InvalidOpcode(u8),
    DivisionByZero(Option<ErrorLocation>),
    IndexOutOfBounds(Option<ErrorLocation>),
    NilPointerDereference(Option<ErrorLocation>),
    TypeAssertionFailed(Option<ErrorLocation>),
    PanicUnwound { msg: Option<String>, loc: Option<ErrorLocation> },
    SendOnClosedChannel(Option<ErrorLocation>),
}

/// Active island thread info.
#[cfg(feature = "std")]
pub struct IslandThread {
    pub handle: GcRef,
    pub command_tx: Sender<IslandCommand>,
    pub join_handle: Option<JoinHandle<()>>,
}

/// VM mutable state that can be borrowed independently from scheduler.
pub struct VmState {
    pub gc: Gc,
    pub globals: Vec<u64>,
    pub itab_cache: ItabCache,
    pub extern_registry: ExternRegistry,
    pub program_args: Vec<String>,
    /// Per-VM sentinel error cache (reset on each module load).
    pub sentinel_errors: SentinelErrorCache,
    /// Next island ID to assign
    pub next_island_id: u32,
    /// Active island threads (index = island_id - 1, since main island is 0)
    #[cfg(feature = "std")]
    pub island_threads: Vec<IslandThread>,
    /// Shared registry for cross-island wake (used by island VMs)
    #[cfg(feature = "std")]
    pub island_registry: Option<IslandRegistry>,
    /// Current island ID (0 for main island)
    #[cfg(feature = "std")]
    pub current_island_id: u32,
    /// Main island's command receiver (for wake commands from other islands)
    #[cfg(feature = "std")]
    pub main_cmd_rx: Option<Receiver<IslandCommand>>,
}

impl VmState {
    pub fn new() -> Self {
        Self {
            gc: Gc::new(),
            globals: Vec::new(),
            itab_cache: ItabCache::new(),
            extern_registry: ExternRegistry::new(),
            program_args: Vec::new(),
            sentinel_errors: SentinelErrorCache::new(),
            next_island_id: 1, // 0 is main island
            #[cfg(feature = "std")]
            island_threads: Vec::new(),
            #[cfg(feature = "std")]
            island_registry: None,
            #[cfg(feature = "std")]
            current_island_id: 0,
            #[cfg(feature = "std")]
            main_cmd_rx: None,
        }
    }
    
    /// Send wake command to an island via shared registry.
    /// All islands (including main) are registered in the registry.
    #[cfg(feature = "std")]
    pub fn send_wake_to_island(&self, island_id: u32, fiber_id: u32) -> bool {
        if let Some(ref registry) = self.island_registry {
            if let Ok(guard) = registry.lock() {
                if let Some(tx) = guard.get(&island_id) {
                    let _ = tx.send(IslandCommand::WakeFiber { fiber_id });
                    return true;
                }
            }
        }
        false
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl Drop for VmState {
    fn drop(&mut self) {
        // Shutdown all island threads and wait for them to complete
        for island in &mut self.island_threads {
            // Send shutdown command
            let _ = island.command_tx.send(IslandCommand::Shutdown);
        }
        
        // Wait for all threads to finish
        for island in &mut self.island_threads {
            if let Some(handle) = island.join_handle.take() {
                let _ = handle.join();
            }
        }
    }
}
