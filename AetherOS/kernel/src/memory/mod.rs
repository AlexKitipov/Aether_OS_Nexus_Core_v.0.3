pub mod frame_allocator;
pub mod page_allocator;

use crate::kprintln;
use bootloader_api::info::MemoryRegions;
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

static FRAME_ALLOCATOR: Mutex<Option<frame_allocator::BootInfoFrameAllocator>> = Mutex::new(None);

/// Initializes the memory management modules.
/// This function is called early in the kernel's boot process.
///
/// # Parameters
/// - `memory_regions`: Bootloader-provided physical memory map used to seed
///   the frame allocator.
pub fn init(memory_regions: &'static MemoryRegions) {
    kprintln!("[kernel] memory: Initializing memory modules...");

    {
        // Initialize the frame allocator with the bootloader's memory map.
        // SAFETY: The caller guarantees bootloader-provided memory regions are valid.
        let mut slot = FRAME_ALLOCATOR.lock();
        *slot = Some(unsafe { frame_allocator::BootInfoFrameAllocator::init(memory_regions) });
    }
    kprintln!("[kernel] memory: BootInfoFrameAllocator wired from BootInfo map.");

    // Initialize the page allocator with the same global frame allocator instance.
    let mut slot = FRAME_ALLOCATOR.lock();
    let frame_allocator = slot
        .as_mut()
        .expect("frame allocator must be initialized before page allocator");
    page_allocator::PageAllocator::init(frame_allocator);
    kprintln!("[kernel] memory: PageAllocator initialized.");

    kprintln!("[kernel] memory: All memory modules initialized.");
}

/// Allocates one physical frame from the global bootstrap frame allocator.
pub fn alloc_frame() -> Option<PhysFrame<Size4KiB>> {
    let mut slot = FRAME_ALLOCATOR.lock();
    slot.as_mut()?.allocate_frame()
}

/// Convenience helper that returns the physical address of an allocated frame.
pub fn alloc_frame_addr() -> Option<PhysAddr> {
    alloc_frame().map(|frame| frame.start_address())
}

/// Conceptually translates a virtual address to a physical address.
///
/// At this stage of the project, paging is still mostly simulated, so we use
/// identity translation as a predictable fallback.
/// Once full page-table walking is available, this function should read the
/// active page tables and return the mapped physical address.
pub fn virt_to_phys(virtual_address: u64) -> u64 {
    virtual_address
}
