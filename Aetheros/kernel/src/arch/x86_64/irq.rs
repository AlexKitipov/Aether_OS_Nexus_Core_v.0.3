// kernel/src/arch/x86_64/irq.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use alloc::collections::BTreeMap;
use spin::Mutex;

use crate::{ipc, kprintln};

/// Maps an IRQ number to an IPC channel ID, which the kernel will use
/// to notify the owning V-Node about an interrupt.
static IRQ_TO_CHANNEL_MAP: Mutex<BTreeMap<u8, ipc::ChannelId>> = Mutex::new(BTreeMap::new());

/// Initializes legacy PIC / interrupt routing scaffolding.
pub unsafe fn init_pic() {
    // Placeholder wiring point for real PIC/APIC init.
    // Keeping as a dedicated function allows `lib::init` to install this
    // before enabling interrupts.
    kprintln!("[kernel] irq: PIC/APIC initialized (scaffold).");
}

/// Register an interrupt handler.
pub fn register_irq_handler(irq_number: u8, channel_id: ipc::ChannelId) {
    let mut map = IRQ_TO_CHANNEL_MAP.lock();
    map.insert(irq_number, channel_id);
    kprintln!(
        "[kernel] irq: Registered IRQ {} to IPC channel {}.",
        irq_number,
        channel_id
    );
}

/// Acknowledges a specific IRQ.
pub fn acknowledge_irq(irq_number: u8) {
    // In a real x86_64 system, this would send an EOI to the PIC/APIC.
    kprintln!("[kernel] irq: Acknowledged IRQ {}.", irq_number);
}

/// This function is called by the actual hardware interrupt handler.
/// It dispatches timer ticks and device IRQ notifications.
pub fn handle_irq(irq_number: u8) {
    if irq_number == crate::timer::TIMER_IRQ {
        crate::timer::tick();
    }

    let channel_id = {
        let map = IRQ_TO_CHANNEL_MAP.lock();
        map.get(&irq_number).cloned()
    };

    if let Some(id) = channel_id {
        kprintln!(
            "[kernel] irq: IRQ {} received, sending IPC to channel {}.",
            irq_number,
            id
        );
        let irq_msg_data = alloc::vec![irq_number];
        let _ = ipc::kernel_send(id, 0, &irq_msg_data);
    }

    acknowledge_irq(irq_number);
}
