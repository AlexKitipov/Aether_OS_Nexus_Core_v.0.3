//! Architecture-facing wrapper for GDT setup.

pub use crate::arch::x86_64::gdt::{
    state,
    GdtState,
    DOUBLE_FAULT_IST_INDEX,
};

/// Initializes the Global Descriptor Table and TSS.
pub fn init() {
    crate::arch::x86_64::gdt::init();
}
