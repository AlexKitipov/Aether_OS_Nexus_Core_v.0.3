pub mod frame_allocator;
pub mod page_allocator;

use crate::kprintln;
use bootloader_api::info::MemoryRegions;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

static FRAME_ALLOCATOR: Mutex<Option<frame_allocator::BootInfoFrameAllocator>> = Mutex::new(None);
static TOTAL_MEMORY_BYTES: AtomicUsize = AtomicUsize::new(0);
static USED_MEMORY_ESTIMATE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Initializes the memory management modules.
/// This function is called early in the kernel's boot process.
///
/// # Parameters
/// - `memory_regions`: Bootloader-provided physical memory map used to seed
///   the frame allocator.
pub fn init(memory_regions: &'static MemoryRegions) {
    kprintln!("[kernel] memory: Initializing memory modules...");
    let total = memory_regions
        .iter()
        .map(|region| (region.end.saturating_sub(region.start)) as usize)
        .sum::<usize>();
    TOTAL_MEMORY_BYTES.store(total, Ordering::Release);
    USED_MEMORY_ESTIMATE_BYTES.store(0, Ordering::Release);

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

/// Finalize virtual-memory bootstrap once bootloader handoff information
/// (direct-map offset, current CR3 tables) is available.
pub fn init_virtual_memory_bootstrap() {
    crate::arch::x86_64::paging::init_bootstrap_mappings(
        crate::arch::x86_64::paging::EARLY_IDENTITY_LIMIT,
    );
    kprintln!(
        "[kernel] memory: Identity + higher-half bootstrap mappings synchronized."
    );
}

/// Allocates one physical frame from the global bootstrap frame allocator.
pub fn alloc_frame() -> Option<PhysFrame<Size4KiB>> {
    let mut slot = FRAME_ALLOCATOR.lock();
    slot.as_mut()?.allocate_frame()
}

/// Convenience helper that returns the physical address of an allocated frame.
pub fn alloc_frame_addr() -> Option<PhysAddr> {
    let frame = alloc_frame()?;
    USED_MEMORY_ESTIMATE_BYTES.fetch_add(4096, Ordering::AcqRel);
    Some(frame.start_address())
}

/// Provides mutable access to the global frame allocator.
pub fn with_frame_allocator<R>(
    f: impl FnOnce(&mut frame_allocator::BootInfoFrameAllocator) -> R,
) -> Option<R> {
    let mut slot = FRAME_ALLOCATOR.lock();
    let allocator = slot.as_mut()?;
    Some(f(allocator))
}

/// Conceptually translates a virtual address to a physical address.
///
/// At this stage of the project, paging is still mostly simulated, so we use
/// identity translation as a predictable fallback.
/// Once full page-table walking is available, this function should read the
/// active page tables and return the mapped physical address.
pub fn virt_to_phys(virtual_address: u64) -> u64 {
    crate::arch::x86_64::paging::virt_to_phys(virtual_address)
}

/// Returns total known physical memory size from the bootloader map.
pub fn total_memory() -> usize {
    TOTAL_MEMORY_BYTES.load(Ordering::Acquire)
}

/// Returns a conservative free-memory estimate for runtime metrics.
pub fn free_memory() -> usize {
    total_memory().saturating_sub(USED_MEMORY_ESTIMATE_BYTES.load(Ordering::Acquire))
}
