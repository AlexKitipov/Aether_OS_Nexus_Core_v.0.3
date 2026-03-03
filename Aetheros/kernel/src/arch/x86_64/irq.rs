// kernel/src/arch/x86_64/irq.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use spin::Mutex;
use alloc::collections::BTreeMap;
use crate::{kprintln, ipc};

/// Maps an IRQ number to an IPC channel ID, which the kernel will use
/// to notify the owning V-Node about an interrupt.
static IRQ_TO_CHANNEL_MAP: Mutex<BTreeMap<u8, ipc::ChannelId>> = Mutex::new(BTreeMap::new());

/// Register an interrupt handler.
/// In this microkernel model, "registering a handler" means mapping an IRQ
/// to an IPC channel. When an interrupt occurs, the kernel will send an
/// IPC message to the specified channel.
pub fn register_irq_handler(irq_number: u8, channel_id: ipc::ChannelId) {
    let mut map = IRQ_TO_CHANNEL_MAP.lock();
    map.insert(irq_number, channel_id);
    kprintln!("[kernel] irq: Registered IRQ {} to IPC channel {}.", irq_number, channel_id);
}

/// Acknowledges a specific IRQ.
/// This would typically involve sending an End-Of-Interrupt (EOI) to the PIC/APIC.
pub fn acknowledge_irq(irq_number: u8) {
    kprintln!("[kernel] irq: Acknowledged IRQ {}.", irq_number);
    // In a real x86_64 system, this would involve writing to the PIC/APIC
    // EOI register. For simulation, this is a no-op.
}

/// This function is called by the actual hardware interrupt handler.
/// It dispatches an IPC message to the registered V-Node.
pub fn handle_irq(irq_number: u8) {
    let channel_id = {
        let map = IRQ_TO_CHANNEL_MAP.lock();
        map.get(&irq_number).cloned()
    };

    if let Some(id) = channel_id {
        kprintln!("[kernel] irq: IRQ {} received, sending IPC to channel {}.", irq_number, id);
        // Send a dummy message to the V-Node indicating the IRQ occurred.
        // The V-Node can then poll its device.
        let irq_msg_data = alloc::vec![irq_number]; // Simple payload: just the IRQ number
        // For now, we assume kernel itself is sender (task_id 0)
        let _ = ipc::kernel_send(id, 0, &irq_msg_data);
    } else {
        kprintln!("[kernel] irq: Unhandled IRQ {}.", irq_number);
    }

    // Always acknowledge the IRQ to prevent repeated interrupts
    acknowledge_irq(irq_number);
}
