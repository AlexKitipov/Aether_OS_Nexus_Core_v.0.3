//! Architecture-facing wrapper for IRQ/PIC setup and IRQ-to-IPC routing.

pub use crate::arch::x86_64::irq::{acknowledge_irq, register_irq_handler};

/// Initializes IRQ plumbing through the canonical architecture entry point.
///
/// The architecture layer delegates PIC remap and initial vector setup to
/// `interrupts::init()`, keeping a single ordering contract for installing and
/// loading IDT vectors before IRQ lines are unmasked.
pub fn init() {
    crate::arch::x86_64::irq::init();
}
