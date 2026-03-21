// kernel/src/arch/x86_64/mod.rs

pub mod boot; // Ensure boot module is declared
pub mod dma;
pub mod gdt; // Isolated GDT/TSS setup
pub mod idt;
pub mod irq;
pub mod paging;

pub fn init() {
    // 1) GDT/TSS
    gdt::init();
    // 2) IDT
    idt::init();
    // 3) Paging bootstrap (CR3-backed structures)
    paging::init();
    // 4) IRQ/IPC bridge wiring for hardware interrupts
    irq::init();
}
