//! Architecture-facing wrapper for IRQ/PIC setup and IRQ-to-IPC routing.

pub use crate::arch::x86_64::irq::{acknowledge_irq, register_irq_handler};

/// Initializes architecture IRQ plumbing (PIC + IDT IRQ vectors).
pub fn init() {
    crate::arch::x86_64::irq::init();
}

