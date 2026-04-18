pub mod frame_allocator;
pub mod page_allocator;

use crate::arch::x86_64::paging;
use crate::kprintln;
use bootloader_api::info::MemoryRegions;
use core::sync::atomic::{AtomicUsize, Ordering};
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::PhysAddr;

static TOTAL_MEMORY_BYTES: AtomicUsize = AtomicUsize::new(0);
static USED_MEMORY_ESTIMATE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Initializes the memory management modules from bootloader regions.
pub fn init(memory_regions: &'static MemoryRegions) {
    kprintln!("[kernel] memory: Initializing memory modules...");

    frame_allocator::init_global(memory_regions)
        .expect("failed to initialize bitmap-backed global frame allocator");

    let total_usable = memory_regions
        .iter()
        .filter(|r| r.kind == bootloader_api::info::MemoryRegionKind::Usable && r.end > r.start)
        .fold(0usize, |acc, r| {
            acc.saturating_add((r.end - r.start) as usize)
        });

    TOTAL_MEMORY_BYTES.store(total_usable, Ordering::Release);
    USED_MEMORY_ESTIMATE_BYTES.store(0, Ordering::Release);

    kprintln!("[kernel] memory: Frame allocator initialized.");
}

/// Finalize virtual-memory bootstrap using known direct-map offset.
pub fn init_virtual_memory_bootstrap() {
    let offset = paging::physical_memory_offset();
    paging::init(offset).expect("failed to initialize page table manager");
    kprintln!("[kernel] memory: page table manager initialized.");
}

/// Finalizes allocators once paging is active.
pub fn finalize_allocator_init() {
    page_allocator::PageAllocator::init();
    kprintln!("[kernel] memory: PageAllocator initialized.");
}

pub fn alloc_frame() -> Option<PhysFrame<Size4KiB>> {
    frame_allocator::with_allocator(|alloc| alloc.allocate())
}

pub fn alloc_frame_addr() -> Option<PhysAddr> {
    let frame = alloc_frame()?;
    USED_MEMORY_ESTIMATE_BYTES.fetch_add(4096, Ordering::AcqRel);
    Some(frame.start_address())
}

pub fn with_frame_allocator<R>(f: impl FnOnce(&mut frame_allocator::BootInfoFrameAllocator) -> R) -> Option<R> {
    Some(frame_allocator::with_allocator(f))
}

pub fn virt_to_phys(virtual_address: u64) -> u64 {
    paging::virt_to_phys(virtual_address)
}

pub fn total_memory() -> usize {
    TOTAL_MEMORY_BYTES.load(Ordering::Acquire)
}

pub fn free_memory() -> usize {
    total_memory().saturating_sub(USED_MEMORY_ESTIMATE_BYTES.load(Ordering::Acquire))
}
