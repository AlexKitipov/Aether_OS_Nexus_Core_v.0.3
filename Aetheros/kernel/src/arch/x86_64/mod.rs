// kernel/src/arch/x86_64/mod.rs

pub mod boot; // Ensure boot module is declared
pub mod dma;
pub mod gdt; // Isolated GDT/TSS setup
pub mod idt;
pub mod irq;
pub mod paging;

pub fn init() {
    gdt::init();
    idt::init();
    paging::init();
    // long_mode_init() from boot module would be called here in a real system
    // boot::long_mode_init(); // Conceptual call for boot mode setup
    // Initialize other architecture-specific components here
}
