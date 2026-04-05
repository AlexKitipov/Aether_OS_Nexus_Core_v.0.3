// kernel/src/arch/x86_64/mod.rs

use bootloader_api::BootInfo;

use crate::{
    drivers::vga_text,
    kprintln,
    memory::{self, frame_allocator},
};

pub mod boot; // Ensure boot module is declared
pub mod dma;
pub mod gdt; // Isolated GDT/TSS setup
pub mod idt;
pub mod irq;
pub mod paging;

pub fn init(boot_info: &'static mut BootInfo) {
    // 1) Text output (for early bring-up logs)
    vga_text::WRITER.lock().clear_screen();
    kprintln!("[kernel] x86_64: entering architecture initialization...");

    // 2) Segmentation and protection (GDT)
    gdt::init();

    // 3) Interrupt descriptor table (IDT)
    idt::init();

    // 4) Physical memory allocator from BootInfo map
    let _frame_allocator = unsafe {
        frame_allocator::BootInfoFrameAllocator::init(&boot_info.memory_regions)
    };

    // Keep the global memory subsystem in sync for later dynamic allocation paths.
    memory::init(&boot_info.memory_regions);
    let physical_memory_offset = boot_info
        .physical_memory_offset
        .into_option()
        .expect("[kernel] x86_64: missing physical_memory_offset in BootInfo");
    paging::configure_physical_memory_offset(physical_memory_offset);
    memory::init_virtual_memory_bootstrap();

    // 5) Virtual memory bootstrap + direct-map mapper (when provided)
    paging::init();
    unsafe {
        let _ = paging::init_mapper(x86_64::VirtAddr::new(physical_memory_offset));
    }

    // 6) IRQ/PIC wiring
    // NOTE: Global interrupt enable is intentionally deferred to the top-level
    // kernel init sequence after runtime subsystems are ready.
    irq::init();

    kprintln!("[kernel] x86_64: architecture initialization complete.");
}
