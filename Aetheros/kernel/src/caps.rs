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
    // Add more capabilities as the system grows
}

impl Capability {
    /// A placeholder for a more sophisticated capability checking mechanism.
    /// In a real system, this would involve checking a V-Node's capability table.
    pub fn check(&self, _task_id: u64) -> bool {
        // For the current alpha stub, we'll implement simple checks.
        // In a production system, this would consult the actual capability store
        // associated with the task/V-Node making the syscall.
        match self {
            Capability::LogWrite => true, // Logging is generally permitted for V-Nodes for debugging
            Capability::TimeRead => true, // Reading time is generally permitted
            Capability::NetworkAccess => true, // Temporarily granted for network V-Nodes development
            Capability::IrqRegister(_) => true, // Temporarily granted for driver V-Nodes
            Capability::DmaAlloc => true, // Temporarily granted for driver V-Nodes
            Capability::DmaAccess => true, // Temporarily granted for driver V-Nodes
            Capability::IrqAck(_) => true, // Temporarily granted for driver V-Nodes
            Capability::IpcManage => true, // Temporarily granted for general IPC usage
            Capability::StorageAccess => false, // Deny by default until VFS is fully robust
            // _ => {
            //     kprintln!("[kernel] caps: Capability {:?} not explicitly granted.", self);
            //     false
            // }
        }
    }
}
