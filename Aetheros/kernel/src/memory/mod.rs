pub mod frame_allocator;
pub mod page_allocator;

use bootloader_api::BootInfo;

use crate::arch::x86_64::paging;
use crate::kprintln;

/// Initializes the memory management modules.
/// This function is called early in the kernel's boot process.
pub fn init(boot_info: &'static BootInfo) {
    kprintln!("[kernel] memory: Initializing memory modules...");

    frame_allocator::init_global(&boot_info.memory_regions)
        .expect("Failed to initialize frame allocator");
    kprintln!("[kernel] memory: Frame allocator initialized.");

    let phys_offset = boot_info.physical_memory_offset.into_option();
    paging::init(phys_offset).expect("Failed to initialize page table manager");
    kprintln!("[kernel] memory: Page table manager initialized.");

    page_allocator::PageAllocator::init();
    kprintln!("[kernel] memory: PageAllocator initialized.");

    kprintln!("[kernel] memory: All memory modules initialized.");
}
