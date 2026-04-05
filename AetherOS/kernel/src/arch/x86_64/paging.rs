// kernel/src/arch/x86_64/paging.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::kprintln;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

const FRAME_SIZE: u64 = 4096;
const DMA_PHYS_START: u64 = 0x0010_0000;
pub const KERNEL_VIRT_OFFSET: u64 = 0xFFFF_8000_0000_0000;
pub const EARLY_IDENTITY_LIMIT: u64 = 1024 * 1024 * 1024;

/// Synthetic frame allocator state for early bootstrap components that need
/// stable "physical" addresses before the full MMU path is implemented.
static NEXT_FREE_FRAME: AtomicU64 = AtomicU64::new(DMA_PHYS_START);
static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Bootstrap virtual->physical page mappings used by DMA buffers.
static BOOTSTRAP_TRANSLATIONS: Mutex<BTreeMap<u64, u64>> = Mutex::new(BTreeMap::new());

/// Initializes the paging system.
/// This includes setting up the initial page tables for the kernel's address space
/// (e.g., identity mapping for lower memory, higher-half mapping for kernel code/data).
pub fn init() {
    kprintln!("[kernel] paging: Initializing hardware paging...");

    // TODO: In a real implementation:
    // 1. Get the current physical frame allocator.
    // 2. Create a new recursive page table (or modify the bootloader-provided one).
    // 3. Map the kernel's physical memory to its higher-half virtual address.
    // 4. Identity map essential hardware registers (e.g., APIC, MMIO).
    // 5. Load the new page table base address into the CR3 register.
    // 6. Enable the PAE (Physical Address Extension) and PGE (Page Global Enable) bits in CR4 (if applicable).
    // 7. Enable paging by setting the PG bit in CR0.

    kprintln!("[kernel] paging: Higher-half kernel setup simulated.");
    kprintln!("[kernel] paging: Paging initialized (bootstrap stage).");
}

/// Validates that a virtual address is canonical in x86_64 mode.
pub fn validate_canonical_virt(addr: u64) {
    let sign = (addr >> 47) & 0x1;
    let high = addr >> 48;
    let is_canonical = if sign == 0 { high == 0 } else { high == 0xFFFF };
    assert!(is_canonical, "non-canonical virtual address: {addr:#x}");
}

/// Returns the higher-half virtual address corresponding to a physical address.
pub fn phys_to_higher_half(phys: u64) -> VirtAddr {
    let virt = KERNEL_VIRT_OFFSET + phys;
    validate_canonical_virt(virt);
    VirtAddr::new(virt)
}

/// Converts a higher-half virtual address back to physical.
pub fn higher_half_to_phys(virt: u64) -> Option<u64> {
    if virt < KERNEL_VIRT_OFFSET {
        return None;
    }
    Some(virt - KERNEL_VIRT_OFFSET)
}

/// Installs bootstrap software translations for identity + higher-half mapping.
pub fn init_bootstrap_mappings(identity_limit: u64) {
    let capped_limit = identity_limit.min(EARLY_IDENTITY_LIMIT);
    register_virt_mapping(0, 0, capped_limit as usize);
    if let Some(offset) = physical_memory_offset() {
        register_virt_mapping(offset, 0, capped_limit as usize);
    } else {
        register_virt_mapping(KERNEL_VIRT_OFFSET, 0, capped_limit as usize);
    }
    kprintln!(
        "[kernel] paging: bootstrap mappings installed for identity [0..{:#x}) and offset-based direct map.",
        capped_limit,
    );
}

/// Persists and validates the physical-memory direct-map offset from BootInfo.
pub fn configure_physical_memory_offset(offset: u64) {
    assert!(offset != 0, "physical memory offset must not be zero");
    validate_canonical_virt(offset);
    assert!(
        offset % FRAME_SIZE == 0,
        "physical memory offset must be 4KiB-aligned"
    );
    PHYSICAL_MEMORY_OFFSET.store(offset, Ordering::Release);
    kprintln!(
        "[kernel] paging: configured physical memory offset at {:#x}.",
        offset
    );
}

/// Returns the configured physical-memory direct-map offset, if available.
pub fn physical_memory_offset() -> Option<u64> {
    let offset = PHYSICAL_MEMORY_OFFSET.load(Ordering::Acquire);
    if offset == 0 {
        None
    } else {
        Some(offset)
    }
}

/// Returns the physical base address of the currently active kernel PML4 table.
pub fn get_kernel_pml4() -> u64 {
    let (level_4_table_frame, _) = Cr3::read();
    let pml4 = level_4_table_frame.start_address().as_u64();
    kprintln!("[kernel] paging: Active kernel PML4 at physical {:#x}.", pml4);
    pml4
}

/// Initializes a hardware-backed view of the currently active paging tables.
///
/// # Safety
/// The caller must guarantee that `physical_memory_offset` is a valid direct
/// mapping base for all physical memory and remains stable for the returned
/// `OffsetPageTable` lifetime.
pub unsafe fn init_active_paging(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    assert!(
        physical_memory_offset.as_u64() != 0,
        "physical_memory_offset must not be zero for active paging init"
    );
    validate_canonical_virt(physical_memory_offset.as_u64());
    configure_physical_memory_offset(physical_memory_offset.as_u64());
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    let level_4_table = &mut *page_table_ptr;
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Initializes a direct-map backed page-table mapper from the currently active
/// CR3 root table.
///
/// # Safety
/// The caller must provide a valid direct physical memory mapping base.
pub unsafe fn init_mapper(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    init_active_paging(physical_memory_offset)
}

/// Maps a virtual heap region onto newly allocated physical frames.
pub fn map_heap_region(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    size: usize,
) -> Result<(), &'static str> {
    validate_canonical_virt(start_addr.as_u64());

    let page_range = {
        let heap_start_page = Page::containing_address(start_addr);
        let heap_end = start_addr + (size as u64) - 1u64;
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("No more physical frames for heap")?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|_| "Failed to map heap page")?
                .flush();
        }
    }

    Ok(())
}

/// Allocates a contiguous run of synthetic physical frames.
///
/// This is a bootstrap helper and not a replacement for a real physical frame
/// allocator wired to the bootloader memory map.
pub fn alloc_frame_range(size_bytes: usize) -> u64 {
    let needed_frames = if size_bytes == 0 {
        1
    } else {
        ((size_bytes as u64) + (FRAME_SIZE - 1)) / FRAME_SIZE
    };
    let allocation_size = needed_frames * FRAME_SIZE;

    let start = NEXT_FREE_FRAME.fetch_add(allocation_size, Ordering::SeqCst);
    kprintln!(
        "[kernel] paging: reserved {} frame(s) [{} bytes] at phys {:#x}.",
        needed_frames,
        allocation_size,
        start
    );
    start
}

/// Registers bootstrap translations for a virtually contiguous memory region.
pub fn register_virt_mapping(virt_addr: u64, phys_addr: u64, size_bytes: usize) {
    let page_count = if size_bytes == 0 {
        1
    } else {
        ((size_bytes as u64) + (FRAME_SIZE - 1)) / FRAME_SIZE
    };

    let virt_base = virt_addr & !(FRAME_SIZE - 1);
    let phys_base = phys_addr & !(FRAME_SIZE - 1);

    let mut mappings = BOOTSTRAP_TRANSLATIONS.lock();
    for page in 0..page_count {
        mappings.insert(virt_base + page * FRAME_SIZE, phys_base + page * FRAME_SIZE);
    }
}

/// Removes previously registered bootstrap translations.
pub fn unregister_virt_mapping(virt_addr: u64, size_bytes: usize) {
    let page_count = if size_bytes == 0 {
        1
    } else {
        ((size_bytes as u64) + (FRAME_SIZE - 1)) / FRAME_SIZE
    };
    let virt_base = virt_addr & !(FRAME_SIZE - 1);

    let mut mappings = BOOTSTRAP_TRANSLATIONS.lock();
    for page in 0..page_count {
        mappings.remove(&(virt_base + page * FRAME_SIZE));
    }
}

/// Best-effort virtual-to-physical translation for bootstrap paths.
///
/// At this stage, we still use identity/direct-map semantics as a fallback.
/// Once full page-table walking is implemented, this function should traverse
/// PML4/PDPT/PD/PT entries and return the resolved physical address.
pub fn virt_to_phys(virt_addr: u64) -> u64 {
    let page_base = virt_addr & !(FRAME_SIZE - 1);
    let page_offset = virt_addr & (FRAME_SIZE - 1);

    let mappings = BOOTSTRAP_TRANSLATIONS.lock();
    if let Some(phys_base) = mappings.get(&page_base) {
        return *phys_base + page_offset;
    }

    if let Some(offset) = physical_memory_offset() {
        if virt_addr >= offset {
            return virt_addr - offset;
        }
    }

    virt_addr
}

/// Direct-map helper variant used by DMA paths that already have a known
/// physical memory offset.
pub fn virt_to_phys_with_offset(virt: u64, offset: u64) -> PhysAddr {
    assert!(offset != 0, "physical memory offset must not be zero");
    assert!(
        virt >= offset,
        "virtual address {virt:#x} is below physical memory offset {offset:#x}"
    );
    PhysAddr::new(virt - offset)
}

/// Strict bootstrap virtual-to-physical translation.
///
/// Unlike `virt_to_phys`, this variant does not fallback to identity mapping
/// and returns `None` when no explicit translation exists.
pub fn try_virt_to_phys(virt_addr: u64) -> Option<u64> {
    let page_base = virt_addr & !(FRAME_SIZE - 1);
    let page_offset = virt_addr & (FRAME_SIZE - 1);

    let mappings = BOOTSTRAP_TRANSLATIONS.lock();
    mappings.get(&page_base).map(|phys_base| *phys_base + page_offset)
}

#[inline]
fn pml4_index(virt: u64) -> u16 {
    ((virt >> 39) & 0x1FF) as u16
}

#[inline]
fn pdpt_index(virt: u64) -> u16 {
    ((virt >> 30) & 0x1FF) as u16
}

#[inline]
fn pd_index(virt: u64) -> u16 {
    ((virt >> 21) & 0x1FF) as u16
}

#[inline]
fn pt_index(virt: u64) -> u16 {
    ((virt >> 12) & 0x1FF) as u16
}

/// Bootstrap-realistic page mapper for early kernel bring-up.
///
/// This function tracks synthetic page-table allocation events for missing
/// paging levels and records virtual->physical translations in the bootstrap
/// software map. It does not yet mutate hardware page tables directly.
pub fn map_page_real(phys: u64, virt: u64, flags: u64) {
    validate_canonical_virt(virt);

    let virt_page = virt & !(FRAME_SIZE - 1);
    let phys_page = phys & !(FRAME_SIZE - 1);

    let pml4 = get_kernel_pml4();
    let pml4_i = pml4_index(virt_page);
    let pdpt_i = pdpt_index(virt_page);
    let pd_i = pd_index(virt_page);
    let pt_i = pt_index(virt_page);

    // Synthetic allocation of intermediate tables for observability during
    // early bring-up (when a full FrameAllocator-backed walker is pending).
    let synthetic_pdpt = alloc_frame_range(FRAME_SIZE as usize);
    let synthetic_pd = alloc_frame_range(FRAME_SIZE as usize);
    let synthetic_pt = alloc_frame_range(FRAME_SIZE as usize);

    register_virt_mapping(virt_page, phys_page, FRAME_SIZE as usize);

    kprintln!(
        "[kernel] paging: map_page_real pml4={:#x} idx [{}, {}, {}, {}], tables [{:#x}, {:#x}, {:#x}], map v={:#x} -> p={:#x}, flags={:#x}.",
        pml4,
        pml4_i,
        pdpt_i,
        pd_i,
        pt_i,
        synthetic_pdpt,
        synthetic_pd,
        synthetic_pt,
        virt_page,
        phys_page,
        flags
    );
}

/// Conceptually maps a virtual address to a physical address.
/// In a real system, this would involve modifying page table entries.
pub fn map(physical_address: usize, virtual_address: usize, flags: u64) {
    map_page_real(physical_address as u64, virtual_address as u64, flags);
}

/// Conceptually unmaps a virtual address.
/// In a real system, this would involve modifying page table entries.
pub fn unmap(virtual_address: usize) {
    unregister_virt_mapping(virtual_address as u64, FRAME_SIZE as usize);
    kprintln!("[kernel] paging: Unmapped virtual {:#x} (bootstrap real path).", virtual_address);
}

/// Backward-compatible conceptual alias.
pub fn map_page(physical_address: usize, virtual_address: usize, flags: u64) {
    map(physical_address, virtual_address, flags);
}

/// Backward-compatible conceptual alias.
pub fn unmap_page(virtual_address: usize) {
    unmap(virtual_address);
}
