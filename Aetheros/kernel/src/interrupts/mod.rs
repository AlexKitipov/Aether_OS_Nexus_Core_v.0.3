//! Common IRQ subsystem (separate from CPU exception handling).

pub mod keyboard;
pub mod pic;
pub mod timer;

use crate::{arch::x86_64::idt, kprintln};

/// Initializes hardware IRQ handling:
/// - remaps/initializes PIC
/// - installs IRQ handlers in the IDT
/// - unmasks required IRQ lines
pub fn init() {
    unsafe {
        pic::init();
    }

    idt::set_irq_handler(pic::timer_vector(), timer::handler);
    idt::set_irq_handler(pic::keyboard_vector(), keyboard::handler);

    unsafe {
        pic::unmask_irq(pic::IRQ_TIMER);
        pic::unmask_irq(pic::IRQ_KEYBOARD);
    }

    kprintln!("[kernel] interrupts: IRQ subsystem initialized.");
}
