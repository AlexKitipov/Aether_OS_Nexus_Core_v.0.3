// kernel/src/arch/x86_64/irq.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use alloc::collections::BTreeMap;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::pic::{self, PIC_1_DATA, PIC_1_OFFSET, PIC_2_DATA};
use crate::{ipc, kprintln};

/// Maps an IRQ number to an IPC channel ID, which the kernel will use
/// to notify the owning V-Node about an interrupt.
static IRQ_TO_CHANNEL_MAP: Mutex<BTreeMap<u8, ipc::ChannelId>> = Mutex::new(BTreeMap::new());

/// Initializes interrupt controller plumbing.
pub fn init() {
    kprintln!("[kernel] irq: Interrupt subsystem initialized.");
}

/// Register an interrupt handler.
/// In this microkernel model, "registering a handler" means mapping an IRQ
/// to an IPC channel and wiring the corresponding hardware vector in the IDT.
pub fn register_irq_handler(irq_number: u8, channel_id: ipc::ChannelId) {
    if irq_number >= IRQ_HANDLER_ENTRIES.len() as u8 {
        kprintln!(
            "[kernel] irq: Refusing to register unsupported IRQ {} (max {}).",
            irq_number,
            IRQ_HANDLER_ENTRIES.len() - 1
        );
        return;
    }

    let mut map = IRQ_TO_CHANNEL_MAP.lock();
    map.insert(irq_number, channel_id);

    let vector = PIC_1_OFFSET + irq_number;
    crate::arch::x86_64::idt::set_irq_handler(vector, IRQ_HANDLER_ENTRIES[irq_number as usize]);

    unsafe {
        unmask_irq(irq_number);
    }

    kprintln!(
        "[kernel] irq: Registered IRQ {} (vector {}) to IPC channel {}.",
        irq_number,
        vector,
        channel_id
    );
}

/// Acknowledges a specific IRQ by issuing an End-Of-Interrupt (EOI) to the PIC.
pub fn acknowledge_irq(irq_number: u8) {
    unsafe {
        pic::end_of_interrupt(irq_number);
    }
    kprintln!("[kernel] irq: Hardware EOI sent for IRQ {}.", irq_number);
}

/// This function is called by the actual hardware interrupt handler.
/// It dispatches an IPC message to the registered V-Node.
pub fn handle_irq(irq_number: u8) {
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
    } else {
        kprintln!("[kernel] irq: Unhandled IRQ {}.", irq_number);
    }

    acknowledge_irq(irq_number);
}

unsafe fn unmask_irq(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC_1_DATA, irq)
    } else {
        (PIC_2_DATA, irq - 8)
    };

    let mut data: Port<u8> = Port::new(port);
    let current = data.read();
    data.write(current & !(1 << bit));
}

macro_rules! define_irq_entry {
    ($name:ident, $irq:expr) => {
        extern "x86-interrupt" fn $name(_stack_frame: InterruptStackFrame) {
            handle_irq($irq);
        }
    };
}

define_irq_entry!(irq_entry_0, 0);
define_irq_entry!(irq_entry_1, 1);
define_irq_entry!(irq_entry_2, 2);
define_irq_entry!(irq_entry_3, 3);
define_irq_entry!(irq_entry_4, 4);
define_irq_entry!(irq_entry_5, 5);
define_irq_entry!(irq_entry_6, 6);
define_irq_entry!(irq_entry_7, 7);
define_irq_entry!(irq_entry_8, 8);
define_irq_entry!(irq_entry_9, 9);
define_irq_entry!(irq_entry_10, 10);
define_irq_entry!(irq_entry_11, 11);
define_irq_entry!(irq_entry_12, 12);
define_irq_entry!(irq_entry_13, 13);
define_irq_entry!(irq_entry_14, 14);
define_irq_entry!(irq_entry_15, 15);

type IrqHandler = extern "x86-interrupt" fn(InterruptStackFrame);

const IRQ_HANDLER_ENTRIES: [IrqHandler; 16] = [
    irq_entry_0,
    irq_entry_1,
    irq_entry_2,
    irq_entry_3,
    irq_entry_4,
    irq_entry_5,
    irq_entry_6,
    irq_entry_7,
    irq_entry_8,
    irq_entry_9,
    irq_entry_10,
    irq_entry_11,
    irq_entry_12,
    irq_entry_13,
    irq_entry_14,
    irq_entry_15,
];
