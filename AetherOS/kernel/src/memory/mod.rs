pub mod frame_allocator;
pub mod page_allocator;

use crate::kprintln;
use bootloader::info::{MemoryRegionKind, MemoryRegions};
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
    let validation = validate_boot_memory_regions(memory_regions);
    assert!(
        validation.usable_region_count > 0 && validation.usable_bytes >= 4096,
        "[kernel] memory: boot memory map has no usable RAM"
    );

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

    kprintln!("[kernel] memory: All memory modules initialized.");
}

/// Finalize virtual-memory bootstrap once bootloader handoff information
/// (direct-map offset, current CR3 tables) is available.
pub fn init_virtual_memory_bootstrap() {
    if crate::arch::x86_64::paging::physical_memory_offset().is_none() {
        kprintln!(
            "[kernel] memory WARNING: physical memory offset not configured; using compatibility higher-half bootstrap map."
        );
    }
    crate::arch::x86_64::paging::init_bootstrap_mappings(
        crate::arch::x86_64::paging::EARLY_IDENTITY_LIMIT,
    );
    kprintln!(
        "[kernel] memory: Identity + higher-half bootstrap mappings synchronized."
    );
}

/// Finalizes bootstrap allocators after direct-map paging is confirmed active.
pub fn finalize_allocator_init() {
    let mut slot = FRAME_ALLOCATOR.lock();
    let frame_allocator = slot
        .as_mut()
        .expect("frame allocator must be initialized before page allocator");
    page_allocator::PageAllocator::init(frame_allocator);
    kprintln!("[kernel] memory: PageAllocator initialized.");
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

struct MemoryValidation {
    usable_region_count: usize,
    usable_bytes: usize,
}

fn validate_boot_memory_regions(memory_regions: &'static MemoryRegions) -> MemoryValidation {
    let mut usable_region_count = 0usize;
    let mut usable_bytes = 0usize;
    let mut last_end = 0u64;
    let mut unknown_region_count = 0usize;

    for (index, region) in memory_regions.iter().enumerate() {
        if region.end <= region.start {
            kprintln!(
                "[kernel] memory WARNING: region #{} has invalid bounds {:#x}..{:#x} ({:?}).",
                index,
                region.start,
                region.end,
                region.kind
            );
            continue;
        }

        if index > 0 && region.start < last_end {
            kprintln!(
                "[kernel] memory WARNING: out-of-order/overlapping region #{} start={:#x} < prev_end={:#x}.",
                index,
                region.start,
                last_end
            );
        }
        last_end = region.end.max(last_end);

        match region.kind {
            MemoryRegionKind::Usable => {
                usable_region_count += 1;
                usable_bytes = usable_bytes.saturating_add((region.end - region.start) as usize);
            }
            MemoryRegionKind::UnknownUefi(_) | MemoryRegionKind::UnknownBios(_) => {
                unknown_region_count += 1;
                kprintln!(
                    "[kernel] memory WARNING: unknown region type at #{}, {:#x}..{:#x}; treating as reserved.",
                    index,
                    region.start,
                    region.end
                );
            }
            _ => {}
        }
    }

    kprintln!(
        "[kernel] memory: validated map (usable regions: {}, usable bytes: {}).",
        usable_region_count,
        usable_bytes
    );
    if unknown_region_count > 0 {
        kprintln!(
            "[kernel] memory: {} unknown firmware region(s) excluded from allocation.",
            unknown_region_count
        );
    }

    MemoryValidation {
        usable_region_count,
        usable_bytes,
    }
}
