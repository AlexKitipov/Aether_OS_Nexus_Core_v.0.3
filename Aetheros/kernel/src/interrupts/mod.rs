//! Common IRQ subsystem (separate from CPU exception handling).

pub mod keyboard;
pub mod pic;
pub mod timer;

use crate::{idt, kprintln};

pub(crate) const IRQ_TIMER: u8 = 0;
pub(crate) const IRQ_KEYBOARD: u8 = 1;

const fn irq_vector(irq: u8) -> u8 {
    pic::PIC_1_OFFSET + irq
}

unsafe fn unmask_irq(irq: u8) {
    use x86_64::instructions::port::Port;

    let (port, bit) = if irq < 8 {
        (pic::PIC_1_DATA, irq)
    } else {
        (pic::PIC_2_DATA, irq - 8)
    };

    let mut data: Port<u8> = Port::new(port);
    let current = data.read();
    data.write(current & !(1 << bit));
}

/// Initializes hardware IRQ handling:
/// - remaps/initializes PIC
/// - installs IRQ handlers in the IDT
/// - unmasks required IRQ lines
pub fn init() {
    unsafe {
        pic::remap();
    }

    idt::set_irq_handler(irq_vector(IRQ_TIMER), timer::handler);
    idt::set_irq_handler(irq_vector(IRQ_KEYBOARD), keyboard::handler);

    unsafe {
        unmask_irq(IRQ_TIMER);
        unmask_irq(IRQ_KEYBOARD);
    }

    kprintln!("[kernel] interrupts: IRQ subsystem initialized.");
}
