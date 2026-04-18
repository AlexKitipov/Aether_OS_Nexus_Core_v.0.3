#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use conquer_once::spin::Once;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags,
    PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

use crate::kprintln;
use crate::memory::frame_allocator::GlobalFrameAllocator;

const FRAME_SIZE: u64 = 4096;
pub const KERNEL_VIRT_OFFSET: u64 = 0xFFFF_8000_0000_0000;
pub const EARLY_IDENTITY_LIMIT: u64 = 1024 * 1024 * 1024;
const MAX_ADDRESS_SPACES: usize = 64;
const KERNEL_PML4_ENTRY_START: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    User,
}

#[derive(Clone, Copy, Debug)]
pub struct AddressSpace {
    pub asid: u16,
    pub kind: AddressSpaceKind,
    pub root_frame: PhysFrame<Size4KiB>,
}

#[derive(Debug)]
pub enum PagingError {
    MissingPhysicalMemoryOffset,
    AddressSpaceLimitReached,
    FrameAllocationFailed,
    MapFailed,
    UnmapFailed,
}

pub struct PageTableManager {
    physical_memory_offset: VirtAddr,
    next_asid: u16,
    spaces: [Option<AddressSpace>; MAX_ADDRESS_SPACES],
}

impl PageTableManager {
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        let active_root = Cr3::read().0;
        let kernel_space = AddressSpace {
            asid: 0,
            kind: AddressSpaceKind::Kernel,
            root_frame: active_root,
        };
        let mut spaces = [None; MAX_ADDRESS_SPACES];
        spaces[0] = Some(kernel_space);

        Self {
            physical_memory_offset,
            next_asid: 1,
            spaces,
        }
    }

    pub fn create_user_address_space(&mut self) -> Result<AddressSpace, PagingError> {
        let asid = self.allocate_asid()?;
        let mut frame_alloc = GlobalFrameAllocator;
        let new_root = frame_alloc
            .allocate_frame()
            .ok_or(PagingError::FrameAllocationFailed)?;

        unsafe {
            let new_table = self.table_ptr_mut(new_root);
            core::ptr::write_bytes(new_table as *mut PageTable as *mut u8, 0, core::mem::size_of::<PageTable>());

            let kernel_root = self.table_ptr(Cr3::read().0);
            for idx in KERNEL_PML4_ENTRY_START..512 {
                (*new_table)[idx] = (*kernel_root)[idx].clone();
            }
        }

        let space = AddressSpace {
            asid,
            kind: AddressSpaceKind::User,
            root_frame: new_root,
        };
        self.store_space(space)?;
        Ok(space)
    }

    pub fn map_in_kernel(
        &mut self,
        page: Page,
        frame: PhysFrame,
        mut flags: PageTableFlags,
    ) -> Result<(), PagingError> {
        flags |= PageTableFlags::GLOBAL;
        let mut frame_alloc = GlobalFrameAllocator;
        let flush = unsafe {
            self.kernel_mapper()
                .map_to(page, frame, flags, &mut frame_alloc)
                .map_err(map_to_err)?
        };
        flush.flush();
        Ok(())
    }

    pub fn map_in_user(
        &mut self,
        space: AddressSpace,
        page: Page,
        frame: PhysFrame,
        mut flags: PageTableFlags,
    ) -> Result<(), PagingError> {
        flags |= PageTableFlags::USER_ACCESSIBLE;
        let mut frame_alloc = GlobalFrameAllocator;
        let flush = unsafe {
            self.mapper_for_root(space.root_frame)
                .map_to(page, frame, flags, &mut frame_alloc)
                .map_err(map_to_err)?
        };
        flush.flush();
        Ok(())
    }

    pub fn unmap_kernel(&mut self, page: Page) -> Result<PhysFrame, PagingError> {
        let (frame, flush) = unsafe {
            self.kernel_mapper()
                .unmap(page)
                .map_err(|_| PagingError::UnmapFailed)?
        };
        flush.flush();
        Ok(frame)
    }

    unsafe fn kernel_mapper(&mut self) -> OffsetPageTable<'static> {
        OffsetPageTable::new(self.table_ptr_mut(Cr3::read().0), self.physical_memory_offset)
    }

    unsafe fn mapper_for_root(&mut self, root: PhysFrame) -> OffsetPageTable<'static> {
        OffsetPageTable::new(self.table_ptr_mut(root), self.physical_memory_offset)
    }

    unsafe fn table_ptr(&self, frame: PhysFrame) -> *const PageTable {
        let virt = self.physical_memory_offset + frame.start_address().as_u64();
        virt.as_ptr::<PageTable>()
    }

    unsafe fn table_ptr_mut(&self, frame: PhysFrame) -> &'static mut PageTable {
        let virt = self.physical_memory_offset + frame.start_address().as_u64();
        &mut *virt.as_mut_ptr::<PageTable>()
    }

    fn allocate_asid(&mut self) -> Result<u16, PagingError> {
        let asid = self.next_asid;
        if asid as usize >= MAX_ADDRESS_SPACES {
            return Err(PagingError::AddressSpaceLimitReached);
        }
        self.next_asid = self.next_asid.saturating_add(1);
        Ok(asid)
    }

    fn store_space(&mut self, space: AddressSpace) -> Result<(), PagingError> {
        if let Some(slot) = self.spaces.iter_mut().find(|entry| entry.is_none()) {
            *slot = Some(space);
            Ok(())
        } else {
            Err(PagingError::AddressSpaceLimitReached)
        }
    }
}

fn map_to_err(error: MapToError<Size4KiB>) -> PagingError {
    match error {
        MapToError::FrameAllocationFailed => PagingError::FrameAllocationFailed,
        _ => PagingError::MapFailed,
    }
}

static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new();
static PAGE_TABLE_MANAGER: Once<Mutex<PageTableManager>> = Once::new();
static BOOTSTRAP_TRANSLATIONS: Mutex<BTreeMap<u64, u64>> = Mutex::new(BTreeMap::new());

pub fn init(physical_memory_offset: Option<u64>) -> Result<(), PagingError> {
    let offset = physical_memory_offset.ok_or(PagingError::MissingPhysicalMemoryOffset)?;
    configure_physical_memory_offset(offset);

    PAGE_TABLE_MANAGER.call_once(|| Mutex::new(PageTableManager::new(VirtAddr::new(offset))));

    init_bootstrap_mappings(EARLY_IDENTITY_LIMIT);

    kprintln!(
        "[kernel] paging: initialized with physical_memory_offset={:#x}",
        offset
    );
    Ok(())
}

pub fn with_manager<R>(f: impl FnOnce(&mut PageTableManager) -> R) -> R {
    let manager = PAGE_TABLE_MANAGER
        .get()
        .expect("paging manager not initialized");
    let mut guard = manager.lock();
    f(&mut guard)
}

pub fn create_user_address_space() -> Result<AddressSpace, PagingError> {
    with_manager(|mgr| mgr.create_user_address_space())
}

pub fn map_kernel_page(
    virtual_address: VirtAddr,
    physical_address: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), PagingError> {
    let virt_page = virtual_address.as_u64() & !(FRAME_SIZE - 1);
    let phys_page = physical_address.as_u64() & !(FRAME_SIZE - 1);

    with_manager(|mgr| {
        mgr.map_in_kernel(
            Page::containing_address(virtual_address),
            PhysFrame::containing_address(physical_address),
            flags,
        )
    })?;

    register_virt_mapping(virt_page, phys_page, FRAME_SIZE as usize);
    Ok(())
}

pub fn map_user_page(
    space: AddressSpace,
    virtual_address: VirtAddr,
    physical_address: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), PagingError> {
    with_manager(|mgr| {
        mgr.map_in_user(
            space,
            Page::containing_address(virtual_address),
            PhysFrame::containing_address(physical_address),
            flags,
        )
    })
}

pub fn unmap_kernel_page(virtual_address: VirtAddr) -> Result<PhysFrame, PagingError> {
    let virt_page = virtual_address.as_u64() & !(FRAME_SIZE - 1);
    let frame = with_manager(|mgr| mgr.unmap_kernel(Page::containing_address(virtual_address)))?;
    unregister_virt_mapping(virt_page, FRAME_SIZE as usize);
    Ok(frame)
}

pub fn configure_physical_memory_offset(offset: u64) {
    assert!(offset != 0, "physical memory offset must not be zero");
    validate_canonical_virt(offset);
    assert!(offset % FRAME_SIZE == 0, "physical memory offset must be 4KiB-aligned");
    PHYSICAL_MEMORY_OFFSET.init_once(|| offset);
}

pub fn physical_memory_offset() -> Option<u64> {
    PHYSICAL_MEMORY_OFFSET.get().copied()
}

pub fn get_kernel_pml4() -> u64 {
    Cr3::read().0.start_address().as_u64()
}

pub unsafe fn init_active_paging(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    let level_4_table = &mut *page_table_ptr;
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

pub unsafe fn init_mapper(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    init_active_paging(physical_memory_offset)
}

pub fn map_heap_region(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    size: usize,
) -> Result<(), &'static str> {
    validate_canonical_virt(start_addr.as_u64());

    let heap_start_page = Page::containing_address(start_addr);
    let heap_end = start_addr + (size as u64) - 1;
    let heap_end_page = Page::containing_address(heap_end);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
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

pub fn validate_canonical_virt(addr: u64) {
    let sign = (addr >> 47) & 0x1;
    let high = addr >> 48;
    let is_canonical = if sign == 0 { high == 0 } else { high == 0xFFFF };
    assert!(is_canonical, "non-canonical virtual address: {addr:#x}");
}

pub fn phys_to_higher_half(phys: u64) -> VirtAddr {
    let virt = KERNEL_VIRT_OFFSET + phys;
    validate_canonical_virt(virt);
    VirtAddr::new(virt)
}

pub fn higher_half_to_phys(virt: u64) -> Option<u64> {
    if virt < KERNEL_VIRT_OFFSET {
        return None;
    }
    Some(virt - KERNEL_VIRT_OFFSET)
}

pub fn init_bootstrap_mappings(identity_limit: u64) {
    let capped_limit = identity_limit.min(EARLY_IDENTITY_LIMIT);
    if capped_limit == 0 {
        return;
    }

    register_virt_mapping(0, 0, capped_limit as usize);
    if let Some(offset) = physical_memory_offset() {
        register_virt_mapping(offset, 0, capped_limit as usize);
    } else {
        register_virt_mapping(KERNEL_VIRT_OFFSET, 0, capped_limit as usize);
    }
}

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

pub fn try_virt_to_phys(virt_addr: u64) -> Option<u64> {
    let page_base = virt_addr & !(FRAME_SIZE - 1);
    let page_offset = virt_addr & (FRAME_SIZE - 1);

    let mappings = BOOTSTRAP_TRANSLATIONS.lock();
    mappings.get(&page_base).map(|phys_base| *phys_base + page_offset)
}

pub fn virt_to_phys_with_offset(virt: u64, offset: u64) -> PhysAddr {
    assert!(offset != 0, "physical memory offset must not be zero");
    validate_canonical_virt(offset);
    assert!(offset % FRAME_SIZE == 0, "physical memory offset must be 4KiB-aligned");
    assert!(
        virt >= offset,
        "virtual address {virt:#x} is below physical memory offset {offset:#x}"
    );
    PhysAddr::new(virt - offset)
}

/// Compatibility wrapper used by existing bootstrap paths.
pub fn map_page_real(phys: u64, virt: u64, flags: u64) {
    let mut table_flags = PageTableFlags::from_bits_truncate(flags);
    table_flags |= PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let _ = map_kernel_page(VirtAddr::new(virt), PhysAddr::new(phys), table_flags);
}

pub fn map(physical_address: usize, virtual_address: usize, flags: u64) {
    map_page_real(physical_address as u64, virtual_address as u64, flags);
}

pub fn unmap(virtual_address: usize) {
    let _ = unmap_kernel_page(VirtAddr::new(virtual_address as u64));
}

pub fn map_page(physical_address: usize, virtual_address: usize, flags: u64) {
    map(physical_address, virtual_address, flags);
}

pub fn unmap_page(virtual_address: usize) {
    unmap(virtual_address);
}
