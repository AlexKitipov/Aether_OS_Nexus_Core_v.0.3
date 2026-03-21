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

    // 5) Virtual memory bootstrap + direct-map mapper (when provided)
    paging::init();
    if let Some(physical_memory_offset) = boot_info.physical_memory_offset.into_option() {
        unsafe {
            let _ = paging::init_mapper(x86_64::VirtAddr::new(physical_memory_offset));
        }
    }

    // 6) IRQ/PIC wiring and hardware interrupt enable
    irq::init();
    unsafe { x86_64::instructions::interrupts::enable() };

    kprintln!("[kernel] x86_64: architecture initialization complete.");
}
