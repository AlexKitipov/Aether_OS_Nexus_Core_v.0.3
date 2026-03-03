//! Architecture-facing wrapper for GDT setup.

/// Initializes the Global Descriptor Table and TSS.
pub fn init() {
    crate::arch::x86_64::gdt::init();
}
