//! Architecture-facing wrapper for IDT setup.

use x86_64::structures::idt::InterruptStackFrame;

/// Initializes CPU exception handlers and loads the IDT.
pub fn init() {
    crate::arch::x86_64::idt::init();
}

/// Registers an external IRQ handler into the IDT at a given vector.
pub fn set_irq_handler(vector: u8, handler: extern "x86-interrupt" fn(InterruptStackFrame)) {
    crate::arch::x86_64::idt::set_irq_handler(vector, handler);
}
