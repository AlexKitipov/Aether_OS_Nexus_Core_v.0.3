// kernel/src/caps.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;

/// Represents a fine-grained capability that can be granted to a V-Node.
/// Capabilities enforce the principle of least privilege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Allows writing messages to the kernel log.
    LogWrite,
    /// Allows reading the kernel's monotonic timer.
    TimeRead,
    /// Allows basic network operations (e.g., registering IRQs, allocating DMA for networking).
    NetworkAccess,
    /// Allows access to persistent storage.
    StorageAccess,
    /// Allows a V-Node to register an interrupt handler for a specific IRQ line.
    /// The u8 specifies the IRQ number.
    IrqRegister(u8),
    /// Allows a V-Node to allocate DMA-capable memory buffers.
    DmaAlloc,
    /// Allows a V-Node to get a pointer to a DMA buffer and set its length.
    DmaAccess,
    /// Allows a V-Node to acknowledge a specific IRQ.
    IrqAck(u8),
    /// Allows a V-Node to create and manage IPC channels.
    IpcManage,
    /// Allows reading global runtime metrics.
    ReadMetrics,
    /// Allows writing to centralized runtime logs.
    WriteLogs,
    /// Allows requesting runtime restart/recovery actions for V-Nodes.
    RestartVNode,
    /// Allows participating in snapshot synchronization flows.
    SyncSnapshots,
    /// Allows reading only the calling V-Node metrics.
    ReadOwnMetrics,
    // Add more capabilities as the system grows
}

impl Capability {
    /// Checks whether the provided task currently holds this capability.
    #[inline]
    pub fn check(&self, task_id: u64) -> bool {
        has_capability(task_id, *self)
    }

    /// Checks whether the currently scheduled task holds this capability.
    #[inline]
    pub fn check_current(&self) -> bool {
        has_current_task_capability(*self)
    }
}

/// Returns true if the currently scheduled task holds `cap`.
pub fn has_current_task_capability(cap: Capability) -> bool {
    let current_task_id = crate::task::scheduler::get_current_task_id();
    has_capability(current_task_id, cap)
}

/// Returns true if a given task currently holds `cap`.
pub fn has_capability(task_id: u64, cap: Capability) -> bool {
    crate::task::scheduler::task_has_capability(task_id, cap)
}

/// Grants `cap` from one task to another if the source task already has it.
/// Returns `true` when the destination task gained the capability.
pub fn transfer_capability(from_task_id: u64, to_task_id: u64, cap: Capability) -> bool {
    if !has_capability(from_task_id, cap) {
        return false;
    }
    crate::task::scheduler::grant_capability(to_task_id, cap)
}


/// Initializes the capability subsystem.
pub fn init() {
    kprintln!("[kernel] caps: Initialized.");
}
