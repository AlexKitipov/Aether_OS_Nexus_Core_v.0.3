pub mod frame_allocator;
pub mod page_allocator;

use crate::kprintln;
use bootloader_api::info::MemoryRegions;

/// Initializes the memory management modules.
/// This function is called early in the kernel's boot process.
pub fn init(memory_regions: &'static MemoryRegions) {
    kprintln!("[kernel] memory: Initializing memory modules...");

    // Initialize the frame allocator with the bootloader's memory map.
    // SAFETY: The caller must guarantee that the memory_regions are valid
    // and accurately describe the physical memory layout.
    let mut frame_allocator =
        unsafe { frame_allocator::BootInfoFrameAllocator::init(memory_regions) };
    kprintln!("[kernel] memory: BootInfoFrameAllocator initialized.");

    // Initialize the page allocator, which uses the frame allocator.
    // In a real system, the page allocator would manage kernel and user virtual address spaces.
    page_allocator::PageAllocator::init(&mut frame_allocator);
    kprintln!("[kernel] memory: PageAllocator initialized.");

    kprintln!("[kernel] memory: All memory modules initialized.");
}

